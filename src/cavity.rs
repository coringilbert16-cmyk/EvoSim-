//! Physical genome-cavity analysis for realized juvenile structure.
//!
//! The genome is the enclosed cavity. The cavity is not genome material. A
//! cavity qualifies as a genome cavity only when the realized physical core
//! shell encloses more area than the reference volume of three joined Carbon
//! hexagons. Geometry is read only from realized `PhysicalGeometry`; catalog
//! geometry is never used as a post-realization substitute.

use crate::resources::{BaseResource, Form};
use crate::structure::OrganismStructure;
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
    fn add(self, other: Self) -> Self {
        Self { x: self.x + other.x, y: self.y + other.y }
    }
    fn sub(self, other: Self) -> Self {
        Self { x: self.x - other.x, y: self.y - other.y }
    }
    fn scale(self, factor: f64) -> Self {
        Self { x: self.x * factor, y: self.y * factor }
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
}

impl GenomeCavity {
    pub fn qualifies(&self) -> bool {
        self.area > self.minimum_area + EPS
    }
}

/// Area of the minimum reference volume: three joined Carbon hexagons.
///
/// Joining the hexagons shares boundaries but does not remove material area,
/// so the reference area is three times the physical area of one catalog
/// Carbon hexagon.
pub fn minimum_genome_cavity_area(catalog: &[BaseResource]) -> Result<f64, String> {
    let carbon = catalog
        .iter()
        .find(|resource| resource.name == "Carbon")
        .ok_or_else(|| "catalog has no Carbon resource".to_string())?;
    let area = form_area(&carbon.shape.form)
        .ok_or_else(|| "Carbon resource has no finite 2D area".to_string())?;
    let minimum = 3.0 * area;
    if !minimum.is_finite() || minimum <= 0.0 {
        return Err("Carbon reference cavity area is invalid".into());
    }
    Ok(minimum)
}

/// Find the largest physically enclosed empty region bounded by the supplied
/// realized genome-core units. The returned area is measured from their actual
/// realized rigid geometry.
pub fn analyze_genome_cavity(
    structure: &OrganismStructure,
    catalog: &[BaseResource],
    core_units: &[usize],
) -> Result<Option<GenomeCavity>, String> {
    if core_units.is_empty() {
        return Ok(None);
    }
    let minimum_area = minimum_genome_cavity_area(catalog)?;
    let mut polygons = Vec::<(usize, Vec<Point>)>::new();

    for &unit_index in core_units {
        let unit = structure
            .units
            .get(unit_index)
            .ok_or_else(|| format!("genome core references missing unit {unit_index}"))?;
        let geometry = unit
            .geometry
            .as_ref()
            .ok_or_else(|| format!("genome core unit {unit_index} has no realized geometry"))?;
        let vertices = transformed_polygon(&geometry.shape().form, unit.placement)
            .ok_or_else(|| format!("genome core unit {unit_index} has unsupported cavity geometry"))?;
        if vertices.len() < 3 {
            return Err(format!("genome core unit {unit_index} has no closed boundary"));
        }
        polygons.push((unit_index, vertices));
    }

    let mut points = Vec::<Point>::new();
    let mut point_index = HashMap::<(i64, i64), usize>::new();
    let mut edges = Vec::<Edge>::new();

    for (unit_index, vertices) in &polygons {
        let _ = unit_index;
        for i in 0..vertices.len() {
            let a = intern_point(vertices[i], &mut points, &mut point_index);
            let b = intern_point(vertices[(i + 1) % vertices.len()], &mut points, &mut point_index);
            if a != b {
                edges.push(Edge { from: a, to: b });
                edges.push(Edge { from: b, to: a });
            }
        }
    }

    if edges.is_empty() {
        return Ok(None);
    }

    let mut outgoing = vec![Vec::<usize>::new(); points.len()];
    for (index, edge) in edges.iter().enumerate() {
        outgoing[edge.from].push(index);
    }

    let mut visited = vec![false; edges.len()];
    let mut best: Option<GenomeCavity> = None;

    for start in 0..edges.len() {
        if visited[start] {
            continue;
        }
        let mut face = Vec::<usize>::new();
        let mut current = start;
        let mut closed = false;

        for _ in 0..=edges.len() {
            if visited[current] {
                closed = current == start;
                break;
            }
            visited[current] = true;
            face.push(current);
            let edge = edges[current];
            let incoming = points[edge.to].sub(points[edge.from]);
            let reverse_angle = (incoming.y.atan2(incoming.x) + std::f64::consts::PI)
                .rem_euclid(TAU);
            let mut next = None;
            let mut best_turn = f64::INFINITY;
            for &candidate in &outgoing[edge.to] {
                if candidate == reverse_edge(current) || visited[candidate] {
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
            let Some(candidate) = next else { break };
            current = candidate;
            if current == start {
                closed = true;
                break;
            }
        }

        if !closed || face.len() < 3 {
            continue;
        }

        let signed_area = face
            .iter()
            .map(|&edge_index| points[edges[edge_index].from].cross(points[edges[edge_index].to]))
            .sum::<f64>()
            * 0.5;
        if signed_area <= EPS {
            continue;
        }

        let sample_edge = edges[face[0]];
        let a = points[sample_edge.from];
        let b = points[sample_edge.to];
        let tangent = b.sub(a);
        let length = tangent.norm();
        if length <= EPS {
            continue;
        }
        let midpoint = a.add(b).scale(0.5);
        let sample = midpoint.add(Point {
            x: -tangent.y / length,
            y: tangent.x / length,
        }.scale(NODE_TOLERANCE * 10.0));

        if polygons.iter().any(|(_, polygon)| point_in_polygon(sample, polygon)) {
            continue;
        }

        let boundary_units = face
            .iter()
            .filter_map(|&edge_index| {
                let from = points[edges[edge_index].from];
                let to = points[edges[edge_index].to];
                polygons.iter().find_map(|(unit_index, polygon)| {
                    if polygon_contains_segment(polygon, from, to) {
                        Some(*unit_index)
                    } else {
                        None
                    }
                })
            })
            .collect::<Vec<_>>();

        let candidate = GenomeCavity {
            area: signed_area,
            boundary_units,
            minimum_area,
        };
        if best.as_ref().map_or(true, |current| candidate.area > current.area) {
            best = Some(candidate);
        }
    }

    Ok(best)
}

fn reverse_edge(index: usize) -> usize {
    index ^ 1
}

fn intern_point(
    point: Point,
    points: &mut Vec<Point>,
    point_index: &mut HashMap<(i64, i64), usize>,
) -> usize {
    let key = (
        (point.x / NODE_TOLERANCE).round() as i64,
        (point.y / NODE_TOLERANCE).round() as i64,
    );
    if let Some(&index) = point_index.get(&key) {
        if points[index].sub(point).norm() <= NODE_TOLERANCE {
            return index;
        }
    }
    let index = points.len();
    points.push(point);
    point_index.insert(key, index);
    index
}

fn transformed_polygon(form: &Form, placement: crate::structure::Placement) -> Option<Vec<Point>> {
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
        Form::RegularPolygon { sides, radius } if *sides >= 3 => Some(
            0.5 * (*sides as f64) * radius * radius * (TAU / *sides as f64).sin(),
        ),
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
        let crosses = (a.y > point.y) != (b.y > point.y)
            && point.x < (b.x - a.x) * (point.y - a.y) / (b.y - a.y) + a.x;
        if crosses {
            inside = !inside;
        }
    }
    inside
}

fn polygon_contains_segment(polygon: &[Point], a: Point, b: Point) -> bool {
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
    use crate::genome::initial_genome;
    use crate::resources::default_catalog;

    #[test]
    fn three_carbon_reference_is_derived_from_catalog_geometry() {
        let catalog = default_catalog();
        let carbon = catalog.iter().find(|r| r.name == "Carbon").unwrap();
        let expected = 3.0
            * form_area(&carbon.shape.form).expect("Carbon must have 2D area");
        assert!((minimum_genome_cavity_area(&catalog).unwrap() - expected).abs() < 1e-12);
    }

    #[test]
    fn initial_core_encloses_more_than_three_joined_carbons() {
        let catalog = default_catalog();
        let genome = initial_genome();
        let structure = genome.structural_blueprint.realize(&catalog).unwrap();
        let cavity = analyze_genome_cavity(
            &structure,
            &catalog,
            &genome.structural_blueprint.core_elements,
        )
        .unwrap()
        .expect("seed core must enclose a physical cavity");
        assert!(cavity.qualifies(), "cavity={cavity:?}");
    }

    #[test]
    fn opening_the_core_fails_cavity_qualification() {
        let catalog = default_catalog();
        let genome = initial_genome();
        let mut structure = genome.structural_blueprint.realize(&catalog).unwrap();
        structure.bonds.clear();
        let cavity = analyze_genome_cavity(
            &structure,
            &catalog,
            &genome.structural_blueprint.core_elements,
        )
        .unwrap();
        assert!(cavity.is_none() || !cavity.unwrap().qualifies());
    }
}
