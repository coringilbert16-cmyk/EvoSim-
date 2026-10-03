#![expect(dead_code, reason = "Staged API retained for subsystem integration")]
//! Physical genome-cavity qualification from realized rigid geometry.

use crate::resources::{BaseResource, Form};
use crate::structure::{ConnectionEndpoint, OrganismStructure, Placement};
use std::collections::{HashMap, HashSet};
use std::f64::consts::TAU;

const EPS: f64 = 1e-8;
const NODE_TOLERANCE: f64 = 1e-7;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Point {
    x: f64,
    y: f64,
}

fn bond_seals_segment(
    structure: &OrganismStructure,
    catalog: &[BaseResource],
    unit_a: usize,
    unit_b: usize,
    segment_a: Point,
    segment_b: Point,
) -> bool {
    let Some(expected_a) = structure.units.get(unit_a) else {
        return false;
    };
    let Some(expected_b) = structure.units.get(unit_b) else {
        return false;
    };
    let id_a = expected_a.physical_id;
    let id_b = expected_b.physical_id;

    let same_point = |x: crate::connection_geometry::WorldConnectionPoint, y: Point| {
        (x.x - y.x).hypot(x.y - y.y) <= NODE_TOLERANCE * 10.0
    };

    let mut seals_a = false;
    let mut seals_b = false;

    for bond in &structure.bonds {
        let endpoint_a_is_unit_a = bond.endpoint_a.constituent_id == id_a;
        let endpoint_b_is_unit_b = bond.endpoint_b.constituent_id == id_b;
        let endpoint_a_is_unit_b = bond.endpoint_a.constituent_id == id_b;
        let endpoint_b_is_unit_a = bond.endpoint_b.constituent_id == id_a;

        if !(endpoint_a_is_unit_a && endpoint_b_is_unit_b
            || endpoint_a_is_unit_b && endpoint_b_is_unit_a)
        {
            continue;
        }

        let (unit_for_a, endpoint_a) = if endpoint_a_is_unit_a {
            (expected_a, bond.endpoint_a.location)
        } else {
            (expected_b, bond.endpoint_a.location)
        };
        let (unit_for_b, endpoint_b) = if endpoint_b_is_unit_b {
            (expected_b, bond.endpoint_b.location)
        } else {
            (expected_a, bond.endpoint_b.location)
        };

        let Some(world_a) = endpoint_a.world_point(unit_for_a, catalog) else {
            continue;
        };
        let Some(world_b) = endpoint_b.world_point(unit_for_b, catalog) else {
            continue;
        };

        let at_segment_a = same_point(world_a, segment_a) && same_point(world_b, segment_a);
        let at_segment_b = same_point(world_a, segment_b) && same_point(world_b, segment_b);

        seals_a |= at_segment_a;
        seals_b |= at_segment_b;
    }

    seals_a && seals_b
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
    /// Each segment is an exposed edge of exactly one structural unit.
    boundary_segments: Vec<(Point, Point, usize)>,
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
    /// Every segment is an actual exposed edge of the material surrounding the
    /// empty cavity.
    pub(crate) fn boundary_segments(&self) -> Vec<((f64, f64), (f64, f64), usize)> {
        self.boundary_segments
            .iter()
            .map(|&(a, b, unit)| ((a.x, a.y), (b.x, b.y), unit))
            .collect()
    }

    /// Return bonds whose two constituents belong to the realized cavity wall.
    /// The cavity boundary itself is an exposed material surface and therefore
    /// does not require a bond running along the empty side of that surface.
    pub fn boundary_bond_indices(
        &self,
        structure: &OrganismStructure,
        _catalog: &[BaseResource],
    ) -> Vec<usize> {
        structure
            .bonds
            .iter()
            .enumerate()
            .filter_map(|(index, bond)| {
                let a = structure.units.iter().position(|unit| {
                    unit.physical_id == bond.endpoint_a.constituent_id
                })?;
                let b = structure.units.iter().position(|unit| {
                    unit.physical_id == bond.endpoint_b.constituent_id
                })?;
                (self.boundary_units.contains(&a) && self.boundary_units.contains(&b))
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
    let component_set: HashSet<usize> = candidate_indices.iter().copied().collect();

    // Interior geometry is the single source of truth for realized planar
    // faces. Do not rebuild the same planar graph here.
    let mut best = None;
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

        // A qualifying genome cavity is an empty face bounded by exposed
        // material surfaces. A boundary segment must belong to exactly one
        // surrounding structural unit. Shared material/material seams are
        // internal structure, not exposed cavity walls.
        let mut boundary_segments = Vec::with_capacity(region.boundary.len());
        let mut valid = true;
        for (i, &(ax, ay)) in region.boundary.iter().enumerate() {
            let (bx, by) = region.boundary[(i + 1) % region.boundary.len()];
            let segment_a = Point { x: ax, y: ay };
            let segment_b = Point { x: bx, y: by };

            let owners = region
                .boundary_units
                .iter()
                .copied()
                .filter(|&unit_index| {
                    unit_boundary_matches_segment(
                        structure,
                        catalog,
                        unit_index,
                        segment_a,
                        segment_b,
                    )
                })
                .collect::<Vec<_>>();

            if owners.len() != 1 {
                valid = false;
                break;
            }
            boundary_segments.push((segment_a, segment_b, owners[0]));
        }

        if !valid {
            continue;
        }

        let candidate = GenomeCavity {
            area: region.area,
            boundary_units: region.boundary_units.clone(),
            minimum_area,
            boundary_segments,
        };

        if best
            .as_ref()
            .map_or(true, |current: &GenomeCavity| candidate.area > current.area)
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
    fn two_endpoint_bonds_seal_a_shared_hex_wall() {
        let catalog = default_catalog();
        let mut structure = OrganismStructure::new();
        let a = structure.add_unit(crate::structure::StructuralUnit::new(
            "Carbon",
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));
        let b = structure.add_unit(crate::structure::StructuralUnit::new(
            "Carbon",
            Placement {
                x: 1.5,
                y: 3.0_f64.sqrt() * 0.5,
                rotation_radians: 0.0,
            },
        ));
        let id_a = structure.units[a].physical_id;
        let id_b = structure.units[b].physical_id;
        let bonds = [
            (
                ConnectionEndpoint::Corner { point_index: 0 },
                ConnectionEndpoint::Corner { point_index: 4 },
            ),
            (
                ConnectionEndpoint::Corner { point_index: 1 },
                ConnectionEndpoint::Corner { point_index: 3 },
            ),
        ];
        for (endpoint_a, endpoint_b) in bonds {
            let bond = crate::structure::Bond {
                endpoint_a: crate::structure::BondEndpoint::new(id_a, endpoint_a),
                endpoint_b: crate::structure::BondEndpoint::new(id_b, endpoint_b),
                strength: 0.95,
                bond_energy: 0.95,
            };
            crate::contact::try_add_bond(&mut structure, bond, &catalog)
                .expect("shared-wall endpoint bond should be physically valid");
        }

        assert!(bond_seals_segment(
            &structure,
            &catalog,
            a,
            b,
            Point { x: 1.0, y: 0.0 },
            Point {
                x: 0.5,
                y: 3.0_f64.sqrt() * 0.5,
            },
        ));
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
