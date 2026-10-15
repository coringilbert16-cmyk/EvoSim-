//! Physical genome-cavity qualification from realized rigid geometry.

use crate::resources::{BaseResource, Form};
use crate::structure::{OrganismStructure, Placement};
use std::collections::HashSet;
use std::f64::consts::TAU;

const EPS: f64 = 1e-8;
const NODE_TOLERANCE: f64 = 1e-7;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Point {
    x: f64,
    y: f64,
}

impl Point {
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }

    fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y
    }

    fn norm(self) -> f64 {
        self.dot(self).sqrt()
    }
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

    /// A genome cavity qualifies only when it has finite area at least equal
    /// to three Carbon constituents and a real physical boundary. Sealing and
    /// The area threshold is a hard minimum, not a suggestion. Sealing is
    /// established by the realized physical boundary and bond graph.
    pub fn qualifies(&self) -> bool {
        self.area.is_finite()
            && self.area > EPS
            && self.area >= self.minimum_area
            && self.minimum_area.is_finite()
            && self.minimum_area > 0.0
            && self.boundary_units.len() >= 3
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
        let boundary_ids = self
            .boundary_units
            .iter()
            .filter_map(|&index| structure.units.get(index).map(|unit| unit.physical_id))
            .collect::<HashSet<_>>();

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

/// The hard minimum cavity area is exactly the area of three Carbon constituents.
/// Equality qualifies; no additional margin is added. Carbon geometry sets the
/// area threshold, but does not prescribe the genome's topology.
pub fn minimum_genome_cavity_area(catalog: &[BaseResource]) -> Result<f64, String> {
    let carbon = catalog
        .iter()
        .find(|resource| resource.name == "Carbon")
        .ok_or_else(|| "catalog has no Carbon resource".to_string())?;
    let area =
        form_area(&carbon.shape.form).ok_or_else(|| "Carbon has no finite 2D area".to_string())?;
    let minimum = 3.0 * area;
    if !minimum.is_finite() || minimum <= 0.0 {
        return Err("invalid Carbon cavity minimum area".into());
    }
    Ok(minimum)
}

/// Analyze the realized physical structure for a sealed cavity whose area
/// meets the three-Carbon minimum. No predefined subset of units is treated as
/// the genome; the physical cavity and the full viability contract determine
/// qualification.
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
        if !region.area.is_finite()
            || region.area <= EPS
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
                        structure, catalog, unit_index, segment_a, segment_b,
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

        // Every shared material/material seam between two cavity-wall units
        // must have a live bond. Component membership alone is insufficient:
        // a ring remains one connected component even after one wall join is
        // removed, and duplicated endpoint records must not mask a missing join.
        if !boundary_units_have_bonded_shared_edges(structure, catalog, &region.boundary_units) {
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

fn boundary_units_have_bonded_shared_edges(
    structure: &OrganismStructure,
    catalog: &[BaseResource],
    boundary_units: &[usize],
) -> bool {
    for (position, &a_index) in boundary_units.iter().enumerate() {
        let Some(a_unit) = structure.units.get(a_index) else {
            return false;
        };
        let Some(a_shape) = a_unit.shape(catalog) else {
            return false;
        };
        let Some(a_polygon) = transformed_polygon(&a_shape.form, a_unit.placement) else {
            continue;
        };

        for &b_index in &boundary_units[position + 1..] {
            let Some(b_unit) = structure.units.get(b_index) else {
                return false;
            };
            let Some(b_shape) = b_unit.shape(catalog) else {
                return false;
            };
            let Some(b_polygon) = transformed_polygon(&b_shape.form, b_unit.placement) else {
                continue;
            };

            let shares_edge = a_polygon
                .iter()
                .enumerate()
                .any(|(i, &a0)| {
                    let a1 = a_polygon[(i + 1) % a_polygon.len()];
                    b_polygon.iter().enumerate().any(|(j, &b0)| {
                        let b1 = b_polygon[(j + 1) % b_polygon.len()];
                        (a0.sub(b0).norm() <= NODE_TOLERANCE
                            && a1.sub(b1).norm() <= NODE_TOLERANCE)
                            || (a0.sub(b1).norm() <= NODE_TOLERANCE
                                && a1.sub(b0).norm() <= NODE_TOLERANCE)
                    })
                });
            if !shares_edge {
                continue;
            }

            let a_id = a_unit.physical_id;
            let b_id = b_unit.physical_id;
            let bonded = structure
                .bonds
                .iter()
                .any(|bond| {
                    (bond.endpoint_a.constituent_id == a_id
                        && bond.endpoint_b.constituent_id == b_id)
                        || (bond.endpoint_a.constituent_id == b_id
                            && bond.endpoint_b.constituent_id == a_id)
                });
            if !bonded {
                return false;
            }
        }
    }
    true
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
    fn three_carbon_minimum_comes_from_carbon_geometry() {
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
    fn cavity_below_three_carbon_minimum_does_not_qualify() {
        let mut cavity = GenomeCavity::test_fixture(99.999, 100.0);
        cavity.boundary_units = vec![0, 1, 2];
        assert!(!cavity.qualifies());
    }

    #[test]
    fn cavity_at_exact_three_carbon_minimum_qualifies() {
        let mut cavity = GenomeCavity::test_fixture(100.0, 100.0);
        cavity.boundary_units = vec![0, 1, 2];
        assert!(cavity.qualifies());
    }

    #[test]
    fn cavity_above_three_carbon_minimum_qualifies() {
        let mut cavity = GenomeCavity::test_fixture(100.001, 100.0);
        cavity.boundary_units = vec![0, 1, 2];
        assert!(cavity.qualifies());
    }

    #[test]
    fn nonfinite_or_nonpositive_cavity_area_does_not_qualify() {
        for area in [f64::NAN, f64::INFINITY, 0.0, -1.0] {
            let mut cavity = GenomeCavity::test_fixture(area, 0.0);
            cavity.boundary_units = vec![0, 1, 2];
            assert!(!cavity.qualifies(), "area={area}");
        }
    }

    #[test]
    fn realized_structure_cavity_qualifies_without_a_predefined_core() {
        let catalog = default_catalog();
        let structure = crate::initial_organism_constructor::sealed_nitrogen_ring_fixture(&catalog)
            .expect("test ring should be physically realized and bonded");
        let cavity = analyze_genome_cavity(&structure, &catalog)
            .unwrap()
            .expect("realized structure must contain a sealed qualifying cavity");
        assert!(cavity.qualifies(), "cavity={cavity:?}");
        assert!(
            cavity.area >= minimum_genome_cavity_area(&catalog).unwrap(),
            "realized cavity must meet the three-Carbon minimum"
        );
        // The four corner squares close the outer perimeter but do not face
        // the cavity. The 12 side squares contribute 8 adjacent side-to-side
        // joins; the live bond transaction represents each join with 2 bonds.
        assert_eq!(cavity.boundary_units.len(), 12);
        assert_eq!(cavity.boundary_bond_indices(&structure, &catalog).len(), 16);
    }

    #[test]
    fn unbonded_structure_is_not_a_genome() {
        let catalog = default_catalog();
        let mut structure =
            crate::initial_organism_constructor::sealed_nitrogen_ring_fixture(&catalog)
                .expect("test ring should be physically realized and bonded");
        structure.bonds.clear();
        assert!(
            analyze_genome_cavity(&structure, &catalog)
                .unwrap()
                .is_none(),
            "geometry alone must not qualify an unbonded ring as a genome"
        );
    }
}
