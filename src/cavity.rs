//! Physical genome-cavity qualification from realized rigid geometry.

use crate::resources::{BaseResource, Form};
use crate::structure::{OrganismStructure, PhysicalConstituentId, Placement};
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
}

impl GenomeCavity {
    pub fn qualifies(&self) -> bool {
        self.area > self.minimum_area + EPS
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
    let area = form_area(&carbon.shape.form)
        .ok_or_else(|| "Carbon has no finite 2D area".to_string())?;
    let minimum = 3.0 * area;
    if !minimum.is_finite() || minimum <= 0.0 {
        return Err("invalid Carbon cavity reference area".into());
    }
    Ok(minimum)
}

/// Analyze only realized core geometry. A cavity is valid only when its
/// boundary is physically sealed by bonds between the core constituents.
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
    for &index in core_units {
        let unit = structure
            .units
            .get(index)
            .ok_or_else(|| format!("genome core references missing unit {index}"))?;
        let geometry = unit
            .geometry
            .as_ref()
            .ok_or_else(|| format!("genome core unit {index} has no realized geometry"))?;
        let polygon = transformed_polygon(&geometry.shape().form, unit.placement)
            .ok_or_else(|| format!("genome core unit {index} has unsupported cavity geometry"))?;
        if polygon.len() < 3 {
            return Err(format!("genome core unit {index} has no closed boundary"));
        }
        polygons.push((index, polygon));
    }
    if !boundary_bonds_are_sealed(structure, &polygons) {
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
        if polygons.iter().any(|(_, polygon)| point_in_polygon(sample, polygon)) {
            continue;
        }
        let boundary_units = face
            .iter()
            .filter_map(|&i| {
                let a = points[edges[i].from];
                let b = points[edges[i].to];
                polygons.iter().find_map(|(unit, polygon)| {
                    segment_in_polygon_boundary(a, b, polygon).then_some(*unit)
                })
            })
            .collect();
        let candidate = GenomeCavity {
            area,
            boundary_units,
            minimum_area,
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

fn boundary_bonds_are_sealed(
    structure: &OrganismStructure,
    polygons: &[(usize, Vec<Point>)],
) -> bool {
    let mut required = Vec::<(PhysicalConstituentId, PhysicalConstituentId)>::new();
    for i in 0..polygons.len() {
        for j in (i + 1)..polygons.len() {
            if !polygon_edges_touch(&polygons[i].1, &polygons[j].1) {
                continue;
            }
            let a = structure.units[polygons[i].0].physical_id;
            let b = structure.units[polygons[j].0].physical_id;
            required.push(if a.0 < b.0 { (a, b) } else { (b, a) });
        }
    }
    if required.is_empty() {
        return false;
    }
    required.into_iter().all(|(a, b)| {
        structure.bonds.iter().any(|bond| {
            let x = bond.endpoint_a.constituent_id;
            let y = bond.endpoint_b.constituent_id;
            (x == a && y == b) || (x == b && y == a)
        })
    })
}

fn polygon_edges_touch(a: &[Point], b: &[Point]) -> bool {
    (0..a.len()).any(|i| {
        let a0 = a[i];
        let a1 = a[(i + 1) % a.len()];
        (0..b.len()).any(|j| {
            let b0 = b[j];
            let b1 = b[(j + 1) % b.len()];
            a0.sub(b0).norm() <= NODE_TOLERANCE
                || a0.sub(b1).norm() <= NODE_TOLERANCE
                || a1.sub(b0).norm() <= NODE_TOLERANCE
                || a1.sub(b1).norm() <= NODE_TOLERANCE
        })
    })
}

fn intern(
    point: Point,
    points: &mut Vec<Point>,
    index: &mut HashMap<(i64, i64), usize>,
) -> usize {
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
        Form::RegularPolygon { sides, radius } if *sides >= 3 => Some(
            0.5 * *sides as f64 * radius * radius * (TAU / *sides as f64).sin(),
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
    use crate::genome::initial_genome;
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
    fn seed_core_exceeds_three_joined_carbon_reference() {
        let catalog = default_catalog();
        let genome = initial_genome();
        let structure = genome.structural_blueprint.realize(&catalog).unwrap();
        let cavity = analyze_genome_cavity(
            &structure,
            &catalog,
            &genome.structural_blueprint.core_elements,
        )
        .unwrap()
        .expect("seed core must be sealed");
        assert!(cavity.qualifies(), "cavity={cavity:?}");
    }

    #[test]
    fn unbonded_core_is_not_a_genome() {
        let catalog = default_catalog();
        let genome = initial_genome();
        let mut structure = genome.structural_blueprint.realize(&catalog).unwrap();
        structure.bonds.clear();
        assert!(
            analyze_genome_cavity(
                &structure,
                &catalog,
                &genome.structural_blueprint.core_elements,
            )
            .unwrap()
            .is_none()
        );
    }
}
