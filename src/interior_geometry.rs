//! General realized interior topology derived from the physical boundary.
use crate::resources::{BaseResource, Form};
use crate::structure::{OrganismStructure, Placement};
use std::collections::HashMap;
use std::f64::consts::TAU;

const EPS: f64 = 1e-8;
const NODE_TOLERANCE: f64 = 1e-7;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Point {
    x: f64,
    y: f64,
}
impl Point {
    fn sub(self, o: Self) -> Self {
        Self {
            x: self.x - o.x,
            y: self.y - o.y,
        }
    }
    fn add(self, o: Self) -> Self {
        Self {
            x: self.x + o.x,
            y: self.y + o.y,
        }
    }
    fn scale(self, f: f64) -> Self {
        Self {
            x: self.x * f,
            y: self.y * f,
        }
    }
    fn cross(self, o: Self) -> f64 {
        self.x * o.y - self.y * o.x
    }
    fn norm(self) -> f64 {
        self.x.hypot(self.y)
    }
}
#[derive(Clone, Copy, Debug)]
struct Edge {
    from: usize,
    to: usize,
}

/// A finite region enclosed by the realized structural boundary.
#[derive(Clone, Debug, PartialEq)]
pub struct EnclosedRegion {
    pub area: f64,
    pub boundary_units: Vec<usize>,
    pub sample_point: (f64, f64),
    pub(crate) boundary: Vec<(f64, f64)>,
}

impl EnclosedRegion {
    /// Whether a world-space point lies in this enclosed region.
    pub fn contains_point(&self, x: f64, y: f64) -> bool {
        let point = Point { x, y };
        let polygon: Vec<Point> = self.boundary.iter().map(|&(x, y)| Point { x, y }).collect();
        point_in_polygon(point, &polygon)
    }
}

/// Whether a structural connection endpoint lies in an accessible enclosed
/// interior region. The genome cavity is already excluded when callers supply
/// regions from `find_accessible_interior_regions`.
pub fn endpoint_in_accessible_interior(
    endpoint: crate::structure::ConnectionEndpoint,
    unit: &crate::structure::StructuralUnit,
    catalog: &[BaseResource],
    regions: &[EnclosedRegion],
) -> bool {
    let Some(point) = endpoint.world_point(unit, catalog) else {
        return false;
    };
    regions
        .iter()
        .any(|region| region.contains_point(point.x, point.y))
}

/// Find finite enclosed regions of a realized organism.
///
/// This deliberately remains separate from genome-cavity qualification. Fluid
/// forms participate in realized boundary topology only when they actually
/// contribute to the organism boundary; no synthetic interior material is
/// created here.
///
/// Find enclosed interior regions that are owned by the organism and are
/// available for ordinary physical material. The genome cavity is deliberately
/// excluded: it is inside the organism, but remains genuinely empty.
pub fn find_accessible_interior_regions(
    structure: &OrganismStructure,
    catalog: &[BaseResource],
) -> Result<Vec<EnclosedRegion>, String> {
    let regions = find_enclosed_regions(structure, catalog);
    let Some(genome) = crate::cavity::analyze_genome_cavity(structure, catalog)? else {
        return Ok(regions);
    };
    let mut genome_boundary = genome.boundary_units;
    genome_boundary.sort_unstable();
    Ok(regions
        .into_iter()
        .filter(|region| {
            let mut boundary = region.boundary_units.clone();
            boundary.sort_unstable();
            boundary != genome_boundary
        })
        .collect())
}

pub fn find_enclosed_regions(
    structure: &OrganismStructure,
    catalog: &[BaseResource],
) -> Vec<EnclosedRegion> {
    let mut polygons = Vec::<(usize, Vec<Point>)>::new();
    let mut line_segments = Vec::<(usize, (Point, Point))>::new();
    let mut fluid_polygons = Vec::<(usize, Vec<Point>)>::new();
    for index in structure.structural_unit_indices(catalog) {
        let unit = &structure.units[index];
        let Some((name, _)) = unit.material.parts.first() else {
            continue;
        };
        let Some(resource) = catalog.iter().find(|resource| resource.name == *name) else {
            continue;
        };
        let Some(geometry) = unit.geometry.as_ref() else {
            continue;
        };
        if let Some(polygon) = transformed_polygon(&geometry.shape().form, unit.placement) {
            if polygon.len() < 3 {
                continue;
            }
            if resource.physical_state == crate::resources::PhysicalState::Fluid {
                fluid_polygons.push((index, polygon));
            } else {
                polygons.push((index, polygon));
            }
            continue;
        }

        // Rigid line constituents are zero-area geometry, but they still form
        // real boundaries when their endpoints stitch two area boundaries.
        // Keep them in the planar graph without allowing a free-standing line
        // to create containment by itself.
        let Form::Line { length } = geometry.shape().form else {
            continue;
        };
        if !length.is_finite() || length <= 0.0 {
            continue;
        }
        let half = length / 2.0;
        let (sin, cos) = unit.placement.rotation_radians.sin_cos();
        let transform = |x: f64, y: f64| Point {
            x: unit.placement.x + x * cos - y * sin,
            y: unit.placement.y + x * sin + y * cos,
        };
        line_segments.push((index, (transform(-half, 0.0), transform(half, 0.0))));
    }

    // A realized fluid boundary is part of organism topology only when
    // some portion of that boundary is actually exposed to the environment.
    // Do not infer this from a world-space bounding envelope: an interior
    // fluid can share the organism's envelope without participating in its
    // physical outer boundary.
    let rigid_polygons = polygons.clone();
    for index_fluid in 0..fluid_polygons.len() {
        let (index, polygon) = &fluid_polygons[index_fluid];
        if fluid_boundary_is_exposed(polygon, &rigid_polygons, &fluid_polygons, index_fluid) {
            polygons.push((*index, polygon.clone()));
        }
    }

    if polygons.is_empty() {
        return Vec::new();
    }

    // Only line constituents that physically reach an area boundary can
    // contribute to an enclosed face. Internal line constituents remain
    // ordinary structural geometry and stay out of this planar topology graph.
    line_segments.retain(|(_, (a, b))| {
        [*a, *b].iter().any(|point| {
            polygons
                .iter()
                .any(|(_, polygon)| point_on_polygon_boundary(*point, polygon))
        })
    });

    let mut points = Vec::new();
    let mut point_index = HashMap::new();
    let mut edges = Vec::new();
    let mut edge_units = Vec::new();
    for (unit, polygon) in &polygons {
        for i in 0..polygon.len() {
            let a_point = polygon[i];
            let b_point = polygon[(i + 1) % polygon.len()];
            let mut split_points = vec![a_point, b_point];
            for (_, (line_a, line_b)) in &line_segments {
                if point_on_segment(*line_a, a_point, b_point) {
                    split_points.push(*line_a);
                }
                if point_on_segment(*line_b, a_point, b_point) {
                    split_points.push(*line_b);
                }
            }
            let ab = b_point.sub(a_point);
            let length_sq = ab.x * ab.x + ab.y * ab.y;
            split_points.sort_by(|left, right| {
                let left_t = if length_sq <= EPS {
                    0.0
                } else {
                    ((left.x - a_point.x) * ab.x + (left.y - a_point.y) * ab.y) / length_sq
                };
                let right_t = if length_sq <= EPS {
                    0.0
                } else {
                    ((right.x - a_point.x) * ab.x + (right.y - a_point.y) * ab.y) / length_sq
                };
                left_t.total_cmp(&right_t)
            });
            split_points.dedup_by(|left, right| left.sub(*right).norm() <= NODE_TOLERANCE);
            for pair in split_points.windows(2) {
                let a = intern(pair[0], &mut points, &mut point_index);
                let b = intern(pair[1], &mut points, &mut point_index);
                if a != b {
                    edges.push(Edge { from: a, to: b });
                    edge_units.push(*unit);
                    edges.push(Edge { from: b, to: a });
                    edge_units.push(*unit);
                }
            }
        }
    }
    for (unit, (a_point, b_point)) in &line_segments {
        let a = intern(*a_point, &mut points, &mut point_index);
        let b = intern(*b_point, &mut points, &mut point_index);
        if a != b {
            edges.push(Edge { from: a, to: b });
            edge_units.push(*unit);
            edges.push(Edge { from: b, to: a });
            edge_units.push(*unit);
        }
    }
    if edges.is_empty() {
        return Vec::new();
    }

    let mut outgoing = vec![Vec::new(); points.len()];
    for (i, e) in edges.iter().enumerate() {
        outgoing[e.from].push(i);
    }
    let mut visited = vec![false; edges.len()];
    let mut regions = Vec::new();

    for start in 0..edges.len() {
        if visited[start] {
            continue;
        }
        let mut face = Vec::new();
        let mut current = start;
        let mut closed = false;
        for _ in 0..=edges.len() {
            if current == start && !face.is_empty() {
                closed = true;
                break;
            }
            if visited[current] {
                break;
            }
            visited[current] = true;
            face.push(current);
            let edge = edges[current];
            let incoming = points[edge.to].sub(points[edge.from]);
            let reverse_angle =
                (incoming.y.atan2(incoming.x) + std::f64::consts::PI).rem_euclid(TAU);
            let mut next = None;
            let mut best_turn = f64::INFINITY;
            for &candidate in &outgoing[edge.to] {
                if candidate == (current ^ 1) || (visited[candidate] && candidate != start) {
                    continue;
                }
                let e = edges[candidate];
                let direction = points[e.to].sub(points[e.from]);
                let angle = direction.y.atan2(direction.x).rem_euclid(TAU);
                let turn = (reverse_angle - angle).rem_euclid(TAU);
                if turn < best_turn {
                    best_turn = turn;
                    next = Some(candidate);
                }
            }
            let Some(next) = next else { break };
            current = next;
        }
        if !closed || face.len() < 3 {
            continue;
        }
        let area = face
            .iter()
            .map(|&i| points[edges[i].from].cross(points[edges[i].to]))
            .sum::<f64>()
            * 0.5;
        if area <= EPS {
            continue;
        }

        let a = points[edges[face[0]].from];
        let b = points[edges[face[0]].to];
        let tangent = b.sub(a);
        let length = tangent.norm();
        if length <= EPS {
            continue;
        }
        let sample = a.add(b).scale(0.5).add(
            Point {
                x: -tangent.y / length,
                y: tangent.x / length,
            }
            .scale(NODE_TOLERANCE * 10.0),
        );
        // Keep the geometric face even when its initial epsilon sample
        // falls inside a wall polygon. The caller can validate accessibility
        // using the complete boundary topology.
        let mut boundary_units = Vec::new();
        for &edge_index in &face {
            let unit = edge_units[edge_index];
            if !boundary_units.contains(&unit) {
                boundary_units.push(unit);
            }
        }
        if boundary_units.is_empty() {
            continue;
        }
        let boundary = face
            .iter()
            .map(|&edge_index| {
                let point = points[edges[edge_index].from];
                (point.x, point.y)
            })
            .collect();
        regions.push(EnclosedRegion {
            area,
            boundary_units,
            sample_point: (sample.x, sample.y),
            boundary,
        });
    }
    regions.sort_by(|a, b| b.area.total_cmp(&a.area));
    regions
}

fn fluid_boundary_is_exposed(
    fluid: &[Point],
    rigid_polygons: &[(usize, Vec<Point>)],
    all_fluids: &[(usize, Vec<Point>)],
    fluid_index: usize,
) -> bool {
    if fluid.len() < 3 {
        return false;
    }

    let signed_area = polygon_signed_area(fluid);
    if signed_area.abs() <= EPS {
        return false;
    }

    let outward_sign = if signed_area > 0.0 { 1.0 } else { -1.0 };
    let sample_distance = NODE_TOLERANCE * 10.0;

    for index in 0..fluid.len() {
        let a = fluid[index];
        let b = fluid[(index + 1) % fluid.len()];
        let tangent = b.sub(a);
        let length = tangent.norm();
        if length <= EPS {
            continue;
        }

        let outward = Point {
            x: outward_sign * tangent.y / length,
            y: outward_sign * -tangent.x / length,
        };
        let sample = a.add(b).scale(0.5).add(outward.scale(sample_distance));

        if rigid_polygons
            .iter()
            .any(|(_, polygon)| point_in_polygon(sample, polygon))
        {
            continue;
        }

        if all_fluids
            .iter()
            .enumerate()
            .any(|(other_index, (_, polygon))| {
                other_index != fluid_index && point_in_polygon(sample, polygon)
            })
        {
            continue;
        }

        return true;
    }

    false
}

fn polygon_signed_area(polygon: &[Point]) -> f64 {
    polygon
        .iter()
        .enumerate()
        .map(|(index, point)| point.cross(polygon[(index + 1) % polygon.len()]))
        .sum::<f64>()
        * 0.5
}

fn intern(point: Point, points: &mut Vec<Point>, index: &mut HashMap<(i64, i64), usize>) -> usize {
    let key = (
        (point.x / NODE_TOLERANCE).round() as i64,
        (point.y / NODE_TOLERANCE).round() as i64,
    );
    if let Some(&i) = index.get(&key) {
        if points[i].sub(point).norm() <= NODE_TOLERANCE {
            return i;
        }
    }
    let i = points.len();
    points.push(point);
    index.insert(key, i);
    i
}
fn transformed_polygon(form: &Form, placement: Placement) -> Option<Vec<Point>> {
    let vertices = form.polygon_vertices()?;
    let (s, c) = placement.rotation_radians.sin_cos();
    Some(
        vertices
            .into_iter()
            .map(|(x, y)| Point {
                x: placement.x + x * c - y * s,
                y: placement.y + x * s + y * c,
            })
            .collect(),
    )
}
fn point_in_polygon(point: Point, polygon: &[Point]) -> bool {
    let mut inside = false;
    for i in 0..polygon.len() {
        let a = polygon[i];
        let b = polygon[(i + 1) % polygon.len()];
        if (a.y > point.y) != (b.y > point.y)
            && point.x < (b.x - a.x) * (point.y - a.y) / (b.y - a.y) + a.x
        {
            inside = !inside;
        }
    }
    inside
}
fn point_on_segment(point: Point, a: Point, b: Point) -> bool {
    let ab = b.sub(a);
    let length_sq = ab.x * ab.x + ab.y * ab.y;
    if length_sq <= EPS {
        return point.sub(a).norm() <= NODE_TOLERANCE;
    }
    let t = ((point.x - a.x) * ab.x + (point.y - a.y) * ab.y) / length_sq;
    if !(-NODE_TOLERANCE..=1.0 + NODE_TOLERANCE).contains(&t) {
        return false;
    }
    let projection = Point {
        x: a.x + t * ab.x,
        y: a.y + t * ab.y,
    };
    projection.sub(point).norm() <= NODE_TOLERANCE
}

fn point_on_polygon_boundary(point: Point, polygon: &[Point]) -> bool {
    (0..polygon.len()).any(|i| {
        let a = polygon[i];
        let b = polygon[(i + 1) % polygon.len()];
        let ab = b.sub(a);
        let length_sq = ab.x * ab.x + ab.y * ab.y;
        if length_sq <= EPS {
            return point.sub(a).norm() <= NODE_TOLERANCE;
        }
        let t = ((point.x - a.x) * ab.x + (point.y - a.y) * ab.y) / length_sq;
        if !(0.0..=1.0).contains(&t) {
            return false;
        }
        let projection = Point {
            x: a.x + t * ab.x,
            y: a.y + t * ab.y,
        };
        projection.sub(point).norm() <= NODE_TOLERANCE
    })
}

fn segment_in_polygon_boundary(a: Point, b: Point, polygon: &[Point]) -> bool {
    (0..polygon.len()).any(|i| {
        let p = polygon[i];
        let q = polygon[(i + 1) % polygon.len()];
        (p.sub(a).norm() <= NODE_TOLERANCE && q.sub(b).norm() <= NODE_TOLERANCE)
            || (p.sub(b).norm() <= NODE_TOLERANCE && q.sub(a).norm() <= NODE_TOLERANCE)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square() -> Vec<Vec<Point>> {
        vec![vec![
            Point { x: -1.0, y: -1.0 },
            Point { x: 1.0, y: -1.0 },
            Point { x: 1.0, y: 1.0 },
            Point { x: -1.0, y: 1.0 },
        ]]
    }

    #[test]
    fn connection_endpoint_accessibility_is_point_based() {
        let catalog = crate::resources::default_catalog();
        let unit = crate::structure::StructuralUnit::new(
            "Water",
            crate::structure::Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        );
        let region = EnclosedRegion {
            area: 4.0,
            boundary_units: vec![],
            sample_point: (0.0, 0.0),
            boundary: vec![(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)],
        };
        assert!(endpoint_in_accessible_interior(
            crate::structure::ConnectionEndpoint::Fluid { x: 0.0, y: 0.0 },
            &unit,
            &catalog,
            std::slice::from_ref(&region),
        ));
    }

    fn region_count(polygons: Vec<Vec<Point>>) -> usize {
        let mut points = Vec::new();
        let mut point_index = HashMap::new();
        let mut edges = Vec::new();
        for (unit, polygon) in polygons.iter().enumerate() {
            for i in 0..polygon.len() {
                let a = intern(polygon[i], &mut points, &mut point_index);
                let b = intern(
                    polygon[(i + 1) % polygon.len()],
                    &mut points,
                    &mut point_index,
                );
                if a != b {
                    edges.push(Edge { from: a, to: b });
                    edges.push(Edge { from: b, to: a });
                }
            }
            let _ = unit;
        }
        let mut outgoing = vec![Vec::new(); points.len()];
        for (i, edge) in edges.iter().enumerate() {
            outgoing[edge.from].push(i);
        }
        let mut visited = vec![false; edges.len()];
        let mut count = 0;
        for start in 0..edges.len() {
            if visited[start] {
                continue;
            }
            let mut face = Vec::new();
            let mut current = start;
            let mut closed = false;
            for _ in 0..=edges.len() {
                if current == start && !face.is_empty() {
                    closed = true;
                    break;
                }
                if visited[current] {
                    break;
                }
                visited[current] = true;
                face.push(current);
                let edge = edges[current];
                let incoming = points[edge.to].sub(points[edge.from]);
                let reverse_angle =
                    (incoming.y.atan2(incoming.x) + std::f64::consts::PI).rem_euclid(TAU);
                let mut next = None;
                let mut best_turn = f64::INFINITY;
                for &candidate in &outgoing[edge.to] {
                    if candidate == (current ^ 1) || (visited[candidate] && candidate != start) {
                        continue;
                    }
                    let e = edges[candidate];
                    let direction = points[e.to].sub(points[e.from]);
                    let angle = direction.y.atan2(direction.x).rem_euclid(TAU);
                    let turn = (reverse_angle - angle).rem_euclid(TAU);
                    if turn < best_turn {
                        best_turn = turn;
                        next = Some(candidate);
                    }
                }
                let Some(next) = next else { break };
                current = next;
            }
            if closed && face.len() >= 3 {
                let area = face
                    .iter()
                    .map(|&i| points[edges[i].from].cross(points[edges[i].to]))
                    .sum::<f64>()
                    * 0.5;
                if area > EPS {
                    let a = points[edges[face[0]].from];
                    let b = points[edges[face[0]].to];
                    let tangent = b.sub(a);
                    let length = tangent.norm();
                    if length > EPS {
                        let sample = a.add(b).scale(0.5).add(
                            Point {
                                x: tangent.y / length,
                                y: -tangent.x / length,
                            }
                            .scale(NODE_TOLERANCE * 10.0),
                        );
                        if !polygons.iter().any(|p| point_in_polygon(sample, p)) {
                            count += 1;
                        }
                    }
                }
            }
        }
        count
    }

    #[test]
    fn fluid_boundary_exposure_is_geometric_not_envelope_based() {
        let rigid = vec![(
            0,
            vec![
                Point { x: -2.0, y: -2.0 },
                Point { x: 2.0, y: -2.0 },
                Point { x: 2.0, y: 2.0 },
                Point { x: -2.0, y: 2.0 },
            ],
        )];
        let interior_fluid = vec![(
            1,
            vec![
                Point { x: -0.5, y: -0.5 },
                Point { x: 0.5, y: -0.5 },
                Point { x: 0.5, y: 0.5 },
                Point { x: -0.5, y: 0.5 },
            ],
        )];

        assert!(!fluid_boundary_is_exposed(
            &interior_fluid[0].1,
            &rigid,
            &interior_fluid,
            0,
        ));
    }

    #[test]
    fn fluid_boundary_is_exposed_when_part_of_outer_material_surface() {
        let rigid = vec![(
            0,
            vec![
                Point { x: -1.0, y: -1.0 },
                Point { x: 0.0, y: -1.0 },
                Point { x: 0.0, y: 1.0 },
                Point { x: -1.0, y: 1.0 },
            ],
        )];
        let outer_fluid = vec![(
            1,
            vec![
                Point { x: 0.0, y: -1.0 },
                Point { x: 1.0, y: -1.0 },
                Point { x: 1.0, y: 1.0 },
                Point { x: 0.0, y: 1.0 },
            ],
        )];

        assert!(fluid_boundary_is_exposed(
            &outer_fluid[0].1,
            &rigid,
            &outer_fluid,
            0,
        ));
    }

    #[test]
    fn closed_polygon_has_an_enclosed_region() {
        assert_eq!(region_count(square()), 1);
    }

    #[test]
    fn removing_one_wall_opens_the_region() {
        // Each structural wall is itself a closed polygon, but three walls
        // arranged as a U do not form a closed cycle around the open interior.
        let wall_thickness = 0.2;
        let p = vec![
            vec![
                Point { x: -1.0, y: -1.0 },
                Point { x: 1.0, y: -1.0 },
                Point {
                    x: 1.0,
                    y: -1.0 + wall_thickness,
                },
                Point {
                    x: -1.0,
                    y: -1.0 + wall_thickness,
                },
            ],
            vec![
                Point { x: -1.0, y: -1.0 },
                Point {
                    x: -1.0 + wall_thickness,
                    y: -1.0,
                },
                Point {
                    x: -1.0 + wall_thickness,
                    y: 1.0,
                },
                Point { x: -1.0, y: 1.0 },
            ],
            vec![
                Point {
                    x: 1.0 - wall_thickness,
                    y: -1.0,
                },
                Point { x: 1.0, y: -1.0 },
                Point { x: 1.0, y: 1.0 },
                Point {
                    x: 1.0 - wall_thickness,
                    y: 1.0,
                },
            ],
        ];
        assert_eq!(region_count(p), 0);
    }
}
