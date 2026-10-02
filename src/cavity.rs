#![expect(dead_code, reason = "Staged API retained for subsystem integration")]
//! Physical genome-cavity qualification from realized rigid geometry.

use crate::resources::{BaseResource, Form};
use crate::structure::{OrganismStructure, Placement};
use std::collections::{HashMap, HashSet};
use std::f64::consts::TAU;

const EPS: f64 = 1e-8;
const NODE_TOLERANCE: f64 = 1e-7;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Point {
    x: f64,
    y: f64,
}

fn bond_connects_units(structure: &OrganismStructure, unit_a: usize, unit_b: usize) -> bool {
    let Some(expected_a) = structure.units.get(unit_a) else {
        return false;
    };
    let Some(expected_b) = structure.units.get(unit_b) else {
        return false;
    };
    if unit_a == unit_b {
        return true;
    }
    let id_a = expected_a.physical_id;
    let id_b = expected_b.physical_id;
    structure.bonds.iter().any(|bond| {
        (bond.endpoint_a.constituent_id == id_a && bond.endpoint_b.constituent_id == id_b)
            || (bond.endpoint_a.constituent_id == id_b && bond.endpoint_b.constituent_id == id_a)
    })
}

impl Point {
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }

    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }

    fn scale(self, factor: f64) -> Self {
        Self {
            x: self.x * factor,
            y: self.y * factor,
        }
    }

    fn cross(self, other: Self) -> f64 {
        self.x * other.y - self.y * other.x
    }

    fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y
    }

    fn norm(self) -> f64 {
        self.dot(self).sqrt()
    }
}

#[derive(Clone, Copy, Debug)]
struct Edge {
    from: usize,
    to: usize,
}

#[derive(Clone, Debug)]
pub struct GenomeCavity {
    pub area: f64,
    pub boundary_units: Vec<usize>,
    pub minimum_area: f64,
    boundary_segments: Vec<(Point, Point, usize, usize)>,
}

impl GenomeCavity {
    #[cfg(test)]
    pub(crate) fn test_fixture(area: f64, minimum_area: f64) -> Self {
        Self {
            area,
            boundary_units: Vec::new(),
            minimum_area,
            boundary_segments: Vec::new(),
        }
    }

    pub fn qualifies(&self) -> bool {
        self.area + EPS >= self.minimum_area
    }

    /// Expose the realized cavity boundary geometry to harmonic reception.
    /// These segments are derived from the same qualifying cavity, so harmonic
    /// sensing cannot invent a separate sensor boundary.
    pub(crate) fn boundary_segments(&self) -> Vec<((f64, f64), (f64, f64), usize, usize)> {
        self.boundary_segments
            .iter()
            .map(|&(a, b, unit_a, unit_b)| ((a.x, a.y), (b.x, b.y), unit_a, unit_b))
            .collect()
    }

    /// Return the physical bond indices that form the qualifying genome-cavity
    /// boundary. This is derived from the realized physical genome criterion;
    /// it is not a second stored genome representation.
    pub fn boundary_bond_indices(
        &self,
        structure: &OrganismStructure,
        _catalog: &[BaseResource],
    ) -> Vec<usize> {
        let mut indices = Vec::new();
        for &(_, _, unit_a, unit_b) in &self.boundary_segments {
            if unit_a == unit_b {
                continue;
            }
            let Some(id_a) = structure.units.get(unit_a).map(|unit| unit.physical_id) else {
                continue;
            };
            let Some(id_b) = structure.units.get(unit_b).map(|unit| unit.physical_id) else {
                continue;
            };
            if let Some(index) = structure
                .bonds
                .iter()
                .enumerate()
                .find_map(|(index, bond)| {
                    ((bond.endpoint_a.constituent_id == id_a
                        && bond.endpoint_b.constituent_id == id_b)
                        || (bond.endpoint_a.constituent_id == id_b
                            && bond.endpoint_b.constituent_id == id_a))
                        .then_some(index)
                })
            {
                if !indices.contains(&index) {
                    indices.push(index);
                }
            }
        }
        indices
    }
}

/// The minimum cavity is strictly larger than the area occupied by three
/// joined Carbon hexagons. Carbon supplies only the geometric reference; it is
/// not the genome.
pub fn minimum_genome_cavity_area(catalog: &[BaseResource]) -> Result<f64, String> {
    let carbon = catalog
        .iter()
        .find(|resource| resource.name == "Carbon")
        .ok_or_else(|| "catalog has no Carbon resource".to_string())?;
    let area =
        form_area(&carbon.shape.form).ok_or_else(|| "Carbon has no finite 2D area".to_string())?;
    let minimum = 3.0 * area;
    if !minimum.is_finite() || minimum <= 0.0 {
        return Err("invalid Carbon cavity reference area".into());
    }
    Ok(minimum)
}

/// Analyze the realized physical structure for a sealed cavity large enough
/// for the established three-Carbon reference. No predefined subset of units
/// is treated as the genome; the qualifying cavity itself is the physical
/// genome criterion.
pub fn analyze_genome_cavity(
    structure: &OrganismStructure,
    catalog: &[BaseResource],
) -> Result<Option<GenomeCavity>, String> {
    // The genome cavity is always re-derived from the current realized graph.
    // Persisted physical IDs are only a cache/identity aid and must never freeze
    // the cavity boundary against later structural evolution.
    let mut best = None;
    for component in structure.connected_components() {
        let candidate = analyze_genome_cavity_in_indices(structure, catalog, &component)?;
        if candidate.as_ref().is_some_and(|candidate| {
            best.as_ref()
                .is_none_or(|current: &GenomeCavity| candidate.area > current.area)
        }) {
            best = candidate;
        }
    }
    Ok(best)
}

fn analyze_genome_cavity_in_indices(
    structure: &OrganismStructure,
    catalog: &[BaseResource],
    candidate_indices: &[usize],
) -> Result<Option<GenomeCavity>, String> {
    let minimum_area = minimum_genome_cavity_area(catalog)?;
    let structural_indices = candidate_indices
        .iter()
        .copied()
        .filter(|&index| structure.is_structurally_qualified(index, catalog));
    let mut polygons = Vec::<(usize, Vec<Point>)>::new();
    for index in structural_indices {
        let unit = &structure.units[index];
        let Some((name, _)) = unit.material.parts.first() else {
            continue;
        };
        let Some(resource) = catalog.iter().find(|resource| resource.name == *name) else {
            continue;
        };
        if resource.physical_state == crate::resources::PhysicalState::Fluid {
            continue;
        }
        let Some(geometry) = unit.geometry.as_ref() else {
            continue;
        };
        let Some(polygon) = transformed_polygon(&geometry.shape().form, unit.placement) else {
            continue;
        };
        if polygon.len() < 3 {
            continue;
        }
        polygons.push((index, polygon));
    }
    if polygons.is_empty() {
        return Ok(None);
    }

    let mut points = Vec::new();
    let mut point_index = HashMap::new();
    let mut edges = Vec::new();
    let mut edge_units = Vec::new();
    for (unit, polygon) in &polygons {
        for i in 0..polygon.len() {
            let a = intern(polygon[i], &mut points, &mut point_index);
            let b = intern(
                polygon[(i + 1) % polygon.len()],
                &mut points,
                &mut point_index,
            );
            if a != b {
                edges.push(Edge { from: a, to: b });
                edge_units.push(*unit);
                edges.push(Edge { from: b, to: a });
                edge_units.push(*unit);
            }
        }
    }
    if edges.is_empty() {
        return Ok(None);
    }

    let mut outgoing = vec![Vec::new(); points.len()];
    for (i, edge) in edges.iter().enumerate() {
        outgoing[edge.from].push(i);
    }
    let mut visited = vec![false; edges.len()];
    let mut best = None;

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
                let next_edge = edges[candidate];
                let direction = points[next_edge.to].sub(points[next_edge.from]);
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
        if polygons
            .iter()
            .any(|(_, polygon)| point_in_polygon(sample, polygon))
        {
            continue;
        }
        let mut boundary_units = Vec::new();
        let mut boundary_segments = Vec::new();
        let face_units = face
            .iter()
            .map(|&edge_index| edge_units[edge_index])
            .collect::<Vec<_>>();
        for (position, &edge_index) in face.iter().enumerate() {
            let a = points[edges[edge_index].from];
            let b = points[edges[edge_index].to];
            let unit_a = face_units[position];
            let unit_b = face_units[(position + 1) % face_units.len()];
            boundary_segments.push((a, b, unit_a, unit_b));
            if !boundary_units.contains(&unit_a) {
                boundary_units.push(unit_a);
            }
        }
        if boundary_units.len() < 2 || boundary_segments.len() < 3 {
            continue;
        }
        // A cavity boundary is physically sealed when every transition from
        // one boundary constituent to the next is an actual structural bond.
        // The bond belongs to the constituent interface, not necessarily to
        // the empty cavity wall itself: face-to-face bonds commonly lie on the
        // occupied side of that wall. A single constituent may own consecutive
        // boundary segments and needs no self-bond.
        let sealed = boundary_segments
            .iter()
            .all(|&(_, _, unit_a, unit_b)| bond_connects_units(structure, unit_a, unit_b));
        if !sealed {
            continue;
        }
        let candidate = GenomeCavity {
            area,
            boundary_units,
            minimum_area,
            boundary_segments,
        };
        if best
            .as_ref()
            .map_or(true, |current: &GenomeCavity| area > current.area)
        {
            best = Some(candidate);
        }
    }
    if best.is_none() {
        // The general interior-region tracer already handles the same realized
        // geometry, including contacts whose epsilon sample lies inside a wall.
        // Reuse that geometric face only as a fallback; the genome still must
        // have a sufficiently large area and a bonded closed boundary.
        let component_set: HashSet<usize> = candidate_indices.iter().copied().collect();
        for region in crate::interior_geometry::find_enclosed_regions(structure, catalog) {
            if region.area + EPS < minimum_area
                || region.boundary_units.len() < 3
                || !region
                    .boundary_units
                    .iter()
                    .all(|index| component_set.contains(index))
            {
                continue;
            }
            let segment_units = region
                .boundary
                .iter()
                .enumerate()
                .map(|(i, &(ax, ay))| {
                    let (bx, by) = region.boundary[(i + 1) % region.boundary.len()];
                    let segment_a = Point { x: ax, y: ay };
                    let segment_b = Point { x: bx, y: by };
                    region
                        .boundary_units
                        .iter()
                        .copied()
                        .filter(|unit_index| {
                            unit_boundary_matches_segment(
                                structure,
                                catalog,
                                *unit_index,
                                segment_a,
                                segment_b,
                            )
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            let closed = !segment_units.is_empty()
                && segment_units.iter().all(|units| !units.is_empty())
                && segment_units.iter().enumerate().all(|(i, current_units)| {
                    let next_units = &segment_units[(i + 1) % segment_units.len()];
                    current_units.iter().any(|&unit_a| {
                        next_units
                            .iter()
                            .any(|&unit_b| bond_connects_units(structure, unit_a, unit_b))
                    })
                });
            if closed {
                let boundary_segments = region
                    .boundary
                    .iter()
                    .enumerate()
                    .filter_map(|(i, &(ax, ay))| {
                        let (bx, by) = region.boundary[(i + 1) % region.boundary.len()];
                        let units_a = &segment_units[i];
                        let units_b = &segment_units[(i + 1) % segment_units.len()];
                        let unit_a = units_a.iter().copied().find(|&unit_a| {
                            units_b
                                .iter()
                                .copied()
                                .any(|unit_b| bond_connects_units(structure, unit_a, unit_b))
                        })?;
                        let unit_b = units_b
                            .iter()
                            .copied()
                            .find(|&unit_b| bond_connects_units(structure, unit_a, unit_b))?;
                        Some((
                            Point { x: ax, y: ay },
                            Point { x: bx, y: by },
                            unit_a,
                            unit_b,
                        ))
                    })
                    .collect::<Vec<_>>();
                if boundary_segments.len() == region.boundary.len() {
                    best = Some(GenomeCavity {
                        area: region.area,
                        boundary_units: region.boundary_units.clone(),
                        minimum_area,
                        boundary_segments,
                    });
                    break;
                }
            }
        }
    }

    Ok(best)
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

fn unit_boundary_matches_segment(
    structure: &OrganismStructure,
    catalog: &[BaseResource],
    unit_index: usize,
    segment_a: Point,
    segment_b: Point,
) -> bool {
    let Some(unit) = structure.units.get(unit_index) else {
        return false;
    };
    let Some(shape) = unit.shape(catalog) else {
        return false;
    };

    match &shape.form {
        Form::Line { length } => {
            if !length.is_finite() || *length <= 0.0 {
                return false;
            }
            let half = length / 2.0;
            let (s, c) = unit.placement.rotation_radians.sin_cos();
            let endpoints = [
                Point {
                    x: unit.placement.x - half * c,
                    y: unit.placement.y - half * s,
                },
                Point {
                    x: unit.placement.x + half * c,
                    y: unit.placement.y + half * s,
                },
            ];
            (endpoints[0].sub(segment_a).norm() <= NODE_TOLERANCE
                && endpoints[1].sub(segment_b).norm() <= NODE_TOLERANCE)
                || (endpoints[0].sub(segment_b).norm() <= NODE_TOLERANCE
                    && endpoints[1].sub(segment_a).norm() <= NODE_TOLERANCE)
        }
        _ => {
            let Some(polygon) = transformed_polygon(&shape.form, unit.placement) else {
                return false;
            };
            segment_in_polygon_boundary(segment_a, segment_b, &polygon)
        }
    }
}

fn transformed_polygon(form: &Form, placement: Placement) -> Option<Vec<Point>> {
    let vertices = form.polygon_vertices()?;
    Some(
        vertices
            .into_iter()
            .map(|(x, y)| {
                let (s, c) = placement.rotation_radians.sin_cos();
                Point {
                    x: placement.x + x * c - y * s,
                    y: placement.y + x * s + y * c,
                }
            })
            .collect(),
    )
}

fn form_area(form: &Form) -> Option<f64> {
    match form {
        Form::Circle { radius } => Some(std::f64::consts::PI * radius * radius),
        Form::Rectangle { width, height } => Some(width * height),
        Form::RegularPolygon { sides, radius } if *sides >= 3 => {
            Some(0.5 * *sides as f64 * radius * radius * (TAU / *sides as f64).sin())
        }
        Form::Polygon { vertices } if vertices.len() >= 3 => Some(
            (0..vertices.len())
                .map(|i| {
                    vertices[i].0 * vertices[(i + 1) % vertices.len()].1
                        - vertices[(i + 1) % vertices.len()].0 * vertices[i].1
                })
                .sum::<f64>()
                .abs()
                * 0.5,
        ),
        Form::Line { .. } | Form::Fluid { .. } => None,
        _ => None,
    }
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
    use crate::resources::default_catalog;

    #[test]
    fn three_carbon_reference_comes_from_carbon_geometry() {
        let catalog = default_catalog();
        let carbon = catalog.iter().find(|r| r.name == "Carbon").unwrap();
        assert!(
            (minimum_genome_cavity_area(&catalog).unwrap()
                - 3.0 * form_area(&carbon.shape.form).unwrap())
            .abs()
                < 1e-12
        );
    }

    #[test]
    fn realized_structure_cavity_qualifies_without_a_predefined_core() {
        let catalog = default_catalog();
        let blueprint = crate::juvenile::confirmed_seed_baseline(&catalog).unwrap();
        let (structure, _, _) = crate::juvenile::realize_initial(&blueprint, &catalog).unwrap();
        let cavity = analyze_genome_cavity(&structure, &catalog)
            .unwrap()
            .expect("realized structure must contain a sealed qualifying cavity");
        assert!(cavity.qualifies(), "cavity={cavity:?}");
        assert!(!cavity.boundary_units.is_empty());
    }

    #[test]
    fn unbonded_structure_is_not_a_genome() {
        let catalog = default_catalog();
        let blueprint = crate::juvenile::confirmed_seed_baseline(&default_catalog()).unwrap();
        let mut structure = blueprint.realize(&catalog).unwrap();
        structure.bonds.clear();
        assert!(analyze_genome_cavity(&structure, &catalog)
            .unwrap()
            .is_none());
    }
}
