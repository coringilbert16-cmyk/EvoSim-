//! Physical contact, accessibility, and structural connection candidates.
//!
//! Geometry/topology only. COMBINE owns energetic outcome, formation, and
//! bond strength. A connection point is a finite site: once occupied by a
//! bond it cannot form another bond.

use std::collections::HashMap;

pub use crate::connection_geometry::WorldConnectionPoint;
use crate::connection_geometry::{
    facing_compatibility, point_distance, transform_connection_point,
};
use crate::environment::ActiveMaterialField;
use crate::resources::{BaseResource, ConnectionPoint, ConnectionSites};
use crate::structure::{OrganismStructure, StructuralUnit};

#[derive(Clone, Copy, Debug)]
pub struct Envelope {
    pub x: f64,
    pub y: f64,
    pub radius: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AccessibleFieldMaterial {
    pub field_index: usize,
    /// Index of the physical material stack within the field cell.
    pub material_index: usize,
}

fn transform_point(point: ConnectionPoint, unit: &StructuralUnit) -> WorldConnectionPoint {
    transform_connection_point(
        point,
        unit.placement.x,
        unit.placement.y,
        unit.placement.rotation_radians,
    )
}

fn distance(a: WorldConnectionPoint, b: WorldConnectionPoint) -> f64 {
    point_distance(a, b)
}
fn facing(a: WorldConnectionPoint, b: WorldConnectionPoint) -> f64 {
    facing_compatibility(a, b)
}

pub fn broad_phase_field_cells(field: &ActiveMaterialField, envelope: Envelope) -> Vec<usize> {
    field.cells_within_radius(envelope.x, envelope.y, envelope.radius)
}

pub fn accessible_field_material(
    field: &ActiveMaterialField,
    envelope: Envelope,
) -> Vec<AccessibleFieldMaterial> {
    broad_phase_field_cells(field, envelope)
        .into_iter()
        .flat_map(|field_index| {
            field.cells[field_index]
                .materials
                .iter()
                .enumerate()
                .filter(|(_, material)| material.total_amount() > 0.0)
                .map(move |(material_index, _)| AccessibleFieldMaterial {
                    field_index,
                    material_index,
                })
        })
        .collect()
}

pub fn broad_phase_structural_units(
    structure: &OrganismStructure,
    envelope: Envelope,
    broad_margin: f64,
) -> Vec<usize> {
    let cutoff = (envelope.radius + broad_margin.max(0.0)).max(0.0);
    let cutoff_sq = cutoff * cutoff;
    structure
        .units
        .iter()
        .enumerate()
        .filter_map(|(i, u)| {
            let dx = u.placement.x - envelope.x;
            let dy = u.placement.y - envelope.y;
            (dx * dx + dy * dy <= cutoff_sq).then_some(i)
        })
        .collect()
}

pub fn unit_within_envelope(
    envelope: Envelope,
    unit: &StructuralUnit,
    catalog: &[BaseResource],
) -> bool {
    let Some(base) = catalog.iter().find(|b| b.name == unit.resource_name) else {
        return false;
    };
    let dx = unit.placement.x - envelope.x;
    let dy = unit.placement.y - envelope.y;
    (dx * dx + dy * dy).sqrt() <= envelope.radius.max(0.0) + base.shape.form.bounding_radius()
}

pub fn candidate_units_in_envelope(
    structure: &OrganismStructure,
    envelope: Envelope,
    catalog: &[BaseResource],
    broad_margin: f64,
) -> Vec<usize> {
    broad_phase_structural_units(structure, envelope, broad_margin)
        .into_iter()
        .filter(|&i| unit_within_envelope(envelope, &structure.units[i], catalog))
        .collect()
}

pub fn world_connection_point(
    point: ConnectionPoint,
    unit: &StructuralUnit,
) -> WorldConnectionPoint {
    transform_point(point, unit)
}

pub fn connection_points_contact(
    a: ConnectionPoint,
    unit_a: &StructuralUnit,
    b: ConnectionPoint,
    unit_b: &StructuralUnit,
    tolerance: f64,
    min_facing: f64,
) -> bool {
    let wa = transform_point(a, unit_a);
    let wb = transform_point(b, unit_b);
    distance(wa, wb) <= tolerance.max(0.0) && facing(wa, wb) >= min_facing
}

pub fn connection_point_distance(
    a: ConnectionPoint,
    unit_a: &StructuralUnit,
    b: ConnectionPoint,
    unit_b: &StructuralUnit,
) -> f64 {
    distance(transform_point(a, unit_a), transform_point(b, unit_b))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConnectionPairCandidate {
    pub point_a: usize,
    pub point_b: usize,
    pub distance: f64,
    pub facing: f64,
    pub load_a: f64,
    pub load_b: f64,
    pub available_a: bool,
    pub available_b: bool,
}

fn connection_point_has_space(structure: &OrganismStructure, unit: usize, point: usize) -> bool {
    structure.connection_count(unit, point) == 0
}

pub fn connection_pair_candidates(
    structure: &OrganismStructure,
    unit_a: usize,
    unit_b: usize,
    catalog: &[BaseResource],
) -> Vec<ConnectionPairCandidate> {
    let Some(a) = structure.units.get(unit_a) else {
        return Vec::new();
    };
    let Some(b) = structure.units.get(unit_b) else {
        return Vec::new();
    };
    let Some(ConnectionSites::Corners(points_a)) = a.connection_sites(catalog) else {
        return Vec::new();
    };
    let Some(ConnectionSites::Corners(points_b)) = b.connection_sites(catalog) else {
        return Vec::new();
    };
    let mut out = Vec::with_capacity(points_a.len().saturating_mul(points_b.len()));
    for (point_a, &pa) in points_a.iter().enumerate() {
        for (point_b, &pb) in points_b.iter().enumerate() {
            let wa = transform_point(pa, a);
            let wb = transform_point(pb, b);
            out.push(ConnectionPairCandidate {
                point_a,
                point_b,
                distance: distance(wa, wb),
                facing: facing(wa, wb),
                load_a: structure.connection_load(unit_a, point_a, catalog),
                load_b: structure.connection_load(unit_b, point_b, catalog),
                available_a: connection_point_has_space(structure, unit_a, point_a),
                available_b: connection_point_has_space(structure, unit_b, point_b),
            });
        }
    }
    out
}

pub fn contacting_connection_pair_candidates(
    structure: &OrganismStructure,
    unit_a: usize,
    unit_b: usize,
    catalog: &[BaseResource],
    tolerance: f64,
    min_facing: f64,
) -> Vec<ConnectionPairCandidate> {
    connection_pair_candidates(structure, unit_a, unit_b, catalog)
        .into_iter()
        .filter(|c| {
            c.distance <= tolerance.max(0.0)
                && c.facing >= min_facing
                && c.available_a
                && c.available_b
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct ConnectionTypeKey(String, String);

impl ConnectionTypeKey {
    fn new(a: &str, b: &str) -> Self {
        if a <= b {
            Self(a.into(), b.into())
        } else {
            Self(b.into(), a.into())
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ConnectionCompatibilityCache {
    pairs: HashMap<ConnectionTypeKey, Vec<(usize, usize)>>,
}

impl ConnectionCompatibilityCache {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn pairs_for_owned(
        &mut self,
        a: &str,
        b: &str,
        catalog: &[BaseResource],
    ) -> Vec<(usize, usize)> {
        let reversed = a > b;
        let key = ConnectionTypeKey::new(a, b);
        if !self.pairs.contains_key(&key) {
            self.pairs
                .insert(key.clone(), Self::build_pairs(&key.0, &key.1, catalog));
        }
        let pairs = self.pairs.get(&key).unwrap();
        if reversed {
            pairs.iter().map(|(x, y)| (*y, *x)).collect()
        } else {
            pairs.clone()
        }
    }
    fn build_pairs(a: &str, b: &str, catalog: &[BaseResource]) -> Vec<(usize, usize)> {
        let Some(a) = catalog.iter().find(|r| r.name == a) else {
            return Vec::new();
        };
        let Some(b) = catalog.iter().find(|r| r.name == b) else {
            return Vec::new();
        };
        let ConnectionSites::Corners(pa) = a.shape.connection_sites() else {
            return Vec::new();
        };
        let ConnectionSites::Corners(pb) = b.shape.connection_sites() else {
            return Vec::new();
        };
        (0..pa.len())
            .flat_map(|i| (0..pb.len()).map(move |j| (i, j)))
            .collect()
    }
    pub fn len(&self) -> usize {
        self.pairs.len()
    }
    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }
    pub fn clear(&mut self) {
        self.pairs.clear();
    }
}

pub fn connection_pair_candidates_cached(
    structure: &OrganismStructure,
    unit_a: usize,
    unit_b: usize,
    catalog: &[BaseResource],
    cache: &mut ConnectionCompatibilityCache,
) -> Vec<ConnectionPairCandidate> {
    let Some(a) = structure.units.get(unit_a) else {
        return Vec::new();
    };
    let Some(b) = structure.units.get(unit_b) else {
        return Vec::new();
    };
    let pairs = cache.pairs_for_owned(&a.resource_name, &b.resource_name, catalog);
    let Some(ConnectionSites::Corners(pa)) = a.connection_sites(catalog) else {
        return Vec::new();
    };
    let Some(ConnectionSites::Corners(pb)) = b.connection_sites(catalog) else {
        return Vec::new();
    };
    pairs
        .into_iter()
        .filter_map(|(ia, ib)| {
            let wa = transform_point(*pa.get(ia)?, a);
            let wb = transform_point(*pb.get(ib)?, b);
            Some(ConnectionPairCandidate {
                point_a: ia,
                point_b: ib,
                distance: distance(wa, wb),
                facing: facing(wa, wb),
                load_a: structure.connection_load(unit_a, ia, catalog),
                load_b: structure.connection_load(unit_b, ib, catalog),
                available_a: connection_point_has_space(structure, unit_a, ia),
                available_b: connection_point_has_space(structure, unit_b, ib),
            })
        })
        .collect()
}

pub fn try_add_bond(
    structure: &mut OrganismStructure,
    bond: crate::structure::Bond,
    catalog: &[BaseResource],
) -> Result<usize, &'static str> {
    if !structure.is_valid_bond(&bond, catalog) {
        return Err("invalid bond");
    }
    if !connection_point_has_space(structure, bond.unit_a, bond.point_a)
        || !connection_point_has_space(structure, bond.unit_b, bond.point_b)
    {
        return Err("bond geometry overlaps existing bond");
    }
    Ok(structure.add_bond(bond))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::default_catalog;
    use crate::structure::{Bond, Placement};

    fn unit(s: &mut OrganismStructure, name: &str, x: f64, y: f64) -> usize {
        s.add_unit(StructuralUnit::new(
            name,
            Placement {
                x,
                y,
                rotation_radians: 0.0,
            },
        ))
    }
    fn bond(a: usize, ap: usize, b: usize, bp: usize) -> Bond {
        Bond {
            unit_a: a,
            point_a: ap,
            unit_b: b,
            point_b: bp,
            strength: 0.5,
            bond_energy: 1.0,
        }
    }

    #[test]
    fn occupied_point_cannot_accept_another_bond() {
        let catalog = default_catalog();
        let mut s = OrganismStructure::new();
        let a = unit(&mut s, "Carbon", 0.0, 0.0);
        let b = unit(&mut s, "Carbon", 1.0, 0.0);
        let c = unit(&mut s, "Carbon", 0.0, 1.0);
        assert!(try_add_bond(&mut s, bond(a, 0, b, 0), &catalog).is_ok());
        assert_eq!(
            try_add_bond(&mut s, bond(a, 0, c, 0), &catalog),
            Err("bond geometry overlaps existing bond")
        );
        assert_eq!(s.connection_count(a, 0), 1);
    }

    #[test]
    fn point_rejects_second_bond_even_when_geometry_is_different() {
        let catalog = default_catalog();
        let mut s = OrganismStructure::new();
        let a = unit(&mut s, "Carbon", 0.0, 0.0);
        let b = unit(&mut s, "Carbon", 1.0, 0.0);
        let c = unit(&mut s, "Carbon", 0.0, 1.0);
        assert!(try_add_bond(&mut s, bond(a, 0, b, 0), &catalog).is_ok());
        assert_eq!(
            try_add_bond(&mut s, bond(a, 0, c, 0), &catalog),
            Err("bond geometry overlaps existing bond")
        );
    }
}
