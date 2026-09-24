#![expect(dead_code, reason = "Staged API retained for subsystem integration")]
//! Physical genome-cavity qualification from realized rigid geometry.

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
    pub(crate) boundary_polygon: Vec<Point>,
}

impl GenomeCavity {
    pub fn qualifies(&self) -> bool {
        self.area + EPS >= self.minimum_area
    }

    pub fn contains_point(&self, x: f64, y: f64) -> bool {
        if !x.is_finite() || !y.is_finite() || self.boundary_polygon.len() < 3 {
            return false;
        }
        point_in_polygon(Point { x, y }, &self.boundary_polygon)
    }

    /// Return the physical bond indices that form the qualifying genome-cavity
    /// boundary. This is derived from the realized physical genome criterion;
    /// it is not a second stored genome representation.
    pub fn boundary_bond_indices(&self, structure: &OrganismStructure) -> Vec<usize> {
        let boundary_ids: std::collections::HashSet<_> = self
            .boundary_units
            .iter()
            .filter_map(|&index| structure.units.get(index).map(|unit| unit.physical_id))
            .collect();
        structure
            .bonds
            .iter()
            .enumerate()
            .filter_map(|(index, bond)| {
                (boundary_ids.contains(&bond.endpoint_a.constituent_id)
                    && boundary_ids.contains(&bond.endpoint_b.constituent_id))
                .then_some(index)
            })
            .collect()
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
    let minimum_area = minimum_genome_cavity_area(catalog)?;
    let mut polygons = Vec::<(usize, Vec<Point>)>::new();
    for (index, unit) in structure.units.iter().enumerate() {
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
    for (_, polygon) in &polygons {
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
        for &i in &face {
            let a = points[edges[i].from];
            let b = points[edges[i].to];
            if let Some(unit) = polygons.iter().find_map(|(unit, polygon)| {
                segment_in_polygon_boundary(a, b, polygon).then_some(*unit)
            }) {
                if !boundary_units.contains(&unit) {
                    boundary_units.push(unit);
                }
            }
        }
        if boundary_units.len() < 2 {
            continue;
        }
        let sealed = boundary_units.iter().enumerate().all(|(index, &unit_a)| {
            let unit_b = boundary_units[(index + 1) % boundary_units.len()];
            if unit_a == unit_b {
                return true;
            }
            let id_a = structure.units[unit_a].physical_id;
            let id_b = structure.units[unit_b].physical_id;
            structure.bonds.iter().any(|bond| {
                let x = bond.endpoint_a.constituent_id;
                let y = bond.endpoint_b.constituent_id;
                (x == id_a && y == id_b) || (x == id_b && y == id_a)
            })
        });
        if !sealed {
            continue;
        }
        let boundary_polygon = face
            .iter()
            .map(|&i| points[edges[i].from])
            .collect();
        let candidate = GenomeCavity {
            area,
            boundary_units,
            minimum_area,
            boundary_polygon,
        };
        if best
            .as_ref()
            .map_or(true, |current: &GenomeCavity| area > current.area)
        {
            best = Some(candidate);
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
