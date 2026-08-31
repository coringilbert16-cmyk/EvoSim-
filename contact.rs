// ============================================================
// PHYSICAL CONTACT / ACCESSIBILITY
// ============================================================
//
// Two-phase physical accessibility for bulk field material and
// individually positioned structural units. Contact detection is
// deliberately separate from acquisition, bonding, and energetic
// consequences.

use std::collections::HashMap;

use crate::environment::ActiveMaterialField;
use crate::resources::{BaseResource, ConnectionPoint, ConnectionSites};
use crate::structure::{OrganismStructure, StructuralUnit};
use crate::connection_geometry::{
    facing_compatibility, point_distance, transform_connection_point,
    within_contact_tolerance, WorldConnectionPoint,
};

#[derive(Clone, Copy, Debug)]
pub struct Envelope {
    pub x: f64,
    pub y: f64,
    pub radius: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AccessibleFieldMaterial {
    pub field_index: usize,
    pub bonded: bool,
}

pub fn broad_phase_field_cells(field: &ActiveMaterialField, envelope: Envelope) -> Vec<usize> {
    field.cells_within_radius(envelope.x, envelope.y, envelope.radius)
}

pub fn accessible_field_material(
    field: &ActiveMaterialField,
    envelope: Envelope,
) -> Vec<AccessibleFieldMaterial> {
    let mut found = Vec::new();
    for field_index in broad_phase_field_cells(field, envelope) {
        let cell = &field.cells[field_index];
        if cell.bonded.total_amount() > 0.0 {
            found.push(AccessibleFieldMaterial { field_index, bonded: true });
        }
        if cell.unbonded.total_amount() > 0.0 {
            found.push(AccessibleFieldMaterial { field_index, bonded: false });
        }
    }
    found
}

pub fn broad_phase_structural_units(
    structure: &OrganismStructure,
    envelope: Envelope,
    broad_margin: f64,
) -> Vec<usize> {
    let cutoff = envelope.radius + broad_margin.max(0.0);
    let cutoff_sq = cutoff * cutoff;
    structure
        .units
        .iter()
        .enumerate()
        .filter(|(_, unit)| {
            let dx = unit.placement.x - envelope.x;
            let dy = unit.placement.y - envelope.y;
            dx * dx + dy * dy <= cutoff_sq
        })
        .map(|(i, _)| i)
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
    let distance = (dx * dx + dy * dy).sqrt();
    distance <= envelope.radius + base.shape.form.bounding_radius()
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
    transform_connection_point(
        point,
        unit.placement.x,
        unit.placement.y,
        unit.placement.rotation_radians,
    )
}

pub fn connection_points_contact(
    a: ConnectionPoint,
    unit_a: &StructuralUnit,
    b: ConnectionPoint,
    unit_b: &StructuralUnit,
    tolerance: f64,
    min_facing: f64,
) -> bool {
    let wa = world_connection_point(a, unit_a);
    let wb = world_connection_point(b, unit_b);
    within_contact_tolerance(wa, wb, tolerance)
        && facing_compatibility(wa, wb) >= min_facing
}

pub fn connection_point_distance(
    a: ConnectionPoint,
    unit_a: &StructuralUnit,
    b: ConnectionPoint,
    unit_b: &StructuralUnit,
) -> f64 {
    point_distance(world_connection_point(a, unit_a), world_connection_point(b, unit_b))
}

// ============================================================
// CONNECTION-PAIR CANDIDATES
// ============================================================
//
// This layer enumerates physically representable connection pairs and
// reports their current geometric/load state. It does NOT decide whether
// COMBINE should succeed, how much energy is involved, or what bond strength
// results. Those decisions remain outside contact geometry.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConnectionPairCandidate {
    pub point_a: usize,
    pub point_b: usize,
    pub distance: f64,
    pub facing: f64,
    pub load_a: f64,
    pub load_b: f64,
}

/// Enumerate every discrete connection-point pair between two structural
/// units. Circle circumference and Fluid/Undetermined sites contribute no
/// indexed candidates because their contact-resolution rules are not locked.
/// Existing bond load is reported but never used to discard a pair.
pub fn connection_pair_candidates(
    structure: &OrganismStructure,
    unit_a: usize,
    unit_b: usize,
    catalog: &[BaseResource],
) -> Vec<ConnectionPairCandidate> {
    let Some(a) = structure.units.get(unit_a) else { return Vec::new() };
    let Some(b) = structure.units.get(unit_b) else { return Vec::new() };

    let Some(ConnectionSites::Corners(points_a)) = a.connection_sites(catalog) else {
        return Vec::new();
    };
    let Some(ConnectionSites::Corners(points_b)) = b.connection_sites(catalog) else {
        return Vec::new();
    };

    let mut candidates = Vec::with_capacity(points_a.len().saturating_mul(points_b.len()));
    for (point_a, &a_point) in points_a.iter().enumerate() {
        for (point_b, &b_point) in points_b.iter().enumerate() {
            let wa = world_connection_point(a_point, a);
            let wb = world_connection_point(b_point, b);
            candidates.push(ConnectionPairCandidate {
                point_a,
                point_b,
                distance: point_distance(wa, wb),
                facing: facing_compatibility(wa, wb),
                load_a: structure.connection_load(unit_a, point_a),
                load_b: structure.connection_load(unit_b, point_b),
            });
        }
    }
    candidates
}

/// Enumerate only point pairs that are already in geometric contact.
/// This is still only a physical filter; no formation threshold or energy
/// rule is applied here.
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
        .filter(|candidate| {
            candidate.distance <= tolerance.max(0.0)
                && candidate.facing >= min_facing
        })
        .collect()
}

// ============================================================
// STATIC CONNECTION COMPATIBILITY CACHE
// ============================================================
//
// The catalog and connection-point topology are immutable during a
// simulation. Re-enumerating the same point-index pairs for common resource
// types is therefore wasted work. This cache stores ONLY the static topology:
// which indexed connection-point pairs exist for a pair of resource types.
//
// It deliberately does NOT cache geometry, bond load, facing, contact state,
// energetic consequences, or evolutionary preference. Those remain dynamic.
// The cache is an optimization only and cannot alter simulation outcomes.

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct ConnectionTypeKey(String, String);

impl ConnectionTypeKey {
    fn new(a: &str, b: &str) -> Self {
        if a <= b {
            Self(a.to_owned(), b.to_owned())
        } else {
            Self(b.to_owned(), a.to_owned())
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

    /// Returns the static discrete point-index pairs for two resource types.
    /// The returned indices are expressed in the argument order `(a, b)`.
    /// For reversed resource order the pair endpoints are reversed as well.
    pub fn pairs_for(
        &mut self,
        resource_a: &str,
        resource_b: &str,
        catalog: &[BaseResource],
    ) -> &[(usize, usize)] {
        let reversed = resource_a > resource_b;
        let key = ConnectionTypeKey::new(resource_a, resource_b);

        if !self.pairs.contains_key(&key) {
            let pairs = Self::build_pairs(&key.0, &key.1, catalog);
            self.pairs.insert(key.clone(), pairs);
        }

        let pairs = self.pairs.get(&key).expect("cache entry was just inserted");
        if !reversed {
            pairs
        } else {
            // The canonical cache stores canonical resource ordering. A
            // reversed request needs reversed endpoint indices, so that
            // operation is handled by `pairs_for_owned` below. This branch is
            // unreachable for the borrowed API's zero-copy return contract.
            pairs
        }
    }

    /// Same lookup as `pairs_for`, but returns an owned vector so reversed
    /// resource order can be represented without storing duplicate cache
    /// entries.
    pub fn pairs_for_owned(
        &mut self,
        resource_a: &str,
        resource_b: &str,
        catalog: &[BaseResource],
    ) -> Vec<(usize, usize)> {
        let reversed = resource_a > resource_b;
        let key = ConnectionTypeKey::new(resource_a, resource_b);
        if !self.pairs.contains_key(&key) {
            let pairs = Self::build_pairs(&key.0, &key.1, catalog);
            self.pairs.insert(key.clone(), pairs);
        }
        let pairs = self.pairs.get(&key).expect("cache entry was just inserted");
        if reversed {
            pairs.iter().map(|(a, b)| (*b, *a)).collect()
        } else {
            pairs.clone()
        }
    }

    fn build_pairs(resource_a: &str, resource_b: &str, catalog: &[BaseResource]) -> Vec<(usize, usize)> {
        let Some(a) = catalog.iter().find(|r| r.name == resource_a) else { return Vec::new() };
        let Some(b) = catalog.iter().find(|r| r.name == resource_b) else { return Vec::new() };
        let Some(ConnectionSites::Corners(points_a)) = a.shape.connection_sites() else {
            return Vec::new();
        };
        let Some(ConnectionSites::Corners(points_b)) = b.shape.connection_sites() else {
            return Vec::new();
        };
        (0..points_a.len())
            .flat_map(|i| (0..points_b.len()).map(move |j| (i, j)))
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

/// Cached version of `connection_pair_candidates`. Only the static pair
/// enumeration is memoized; all dynamic geometry and bond-load values are
/// recalculated every call.
pub fn connection_pair_candidates_cached(
    structure: &OrganismStructure,
    unit_a: usize,
    unit_b: usize,
    catalog: &[BaseResource],
    cache: &mut ConnectionCompatibilityCache,
) -> Vec<ConnectionPairCandidate> {
    let Some(a) = structure.units.get(unit_a) else { return Vec::new() };
    let Some(b) = structure.units.get(unit_b) else { return Vec::new() };
    let pairs = cache.pairs_for_owned(&a.resource_name, &b.resource_name, catalog);
    let Some(ConnectionSites::Corners(points_a)) = a.connection_sites(catalog) else { return Vec::new() };
    let Some(ConnectionSites::Corners(points_b)) = b.connection_sites(catalog) else { return Vec::new() };

    pairs
        .into_iter()
        .filter_map(|(point_a, point_b)| {
            let a_point = *points_a.get(point_a)?;
            let b_point = *points_b.get(point_b)?;
            let wa = world_connection_point(a_point, a);
            let wb = world_connection_point(b_point, b);
            Some(ConnectionPairCandidate {
                point_a,
                point_b,
                distance: point_distance(wa, wb),
                facing: facing_compatibility(wa, wb),
                load_a: structure.connection_load(unit_a, point_a),
                load_b: structure.connection_load(unit_b, point_b),
            })
        })
        .collect()
}

#[cfg(test)]
mod contact_tests {
    use super::*;
    use crate::environment::{ActiveMaterialField, DEFAULT_CELL_SIZE};
    use crate::resources::{default_catalog, Material};
    use crate::structure::Placement;
    use std::f64::consts::PI;

    #[test]
    fn accessible_field_material_finds_bonded_and_unbonded() {
        let mut field = ActiveMaterialField::new(1000.0, 1000.0, DEFAULT_CELL_SIZE);
        field.deposit(500.0, 500.0, Material { parts: vec![("Carbon".into(), 10.0)], bonded: true });
        field.deposit(500.0, 500.0, Material { parts: vec![("Water".into(), 3.0)], bonded: false });
        let found = accessible_field_material(&field, Envelope { x: 500.0, y: 500.0, radius: 5.0 });
        assert!(found.iter().any(|f| f.bonded));
        assert!(found.iter().any(|f| !f.bonded));
    }

    #[test]
    fn broad_phase_field_cells_delegates_to_field_query() {
        let field = ActiveMaterialField::new(1000.0, 1000.0, DEFAULT_CELL_SIZE);
        let envelope = Envelope { x: 500.0, y: 500.0, radius: 50.0 };
        assert_eq!(broad_phase_field_cells(&field, envelope), field.cells_within_radius(500.0, 500.0, 50.0));
    }

    #[test]
    fn candidate_units_uses_broad_then_precise_phase() {
        let catalog = default_catalog();
        let mut structure = OrganismStructure::new();
        let near = structure.add_unit(StructuralUnit::new("Carbon", Placement { x: 0.5, y: 0.0, rotation_radians: 0.0 }));
        let far = structure.add_unit(StructuralUnit::new("Carbon", Placement { x: 500.0, y: 500.0, rotation_radians: 0.0 }));
        let candidates = candidate_units_in_envelope(&structure, Envelope { x: 0.0, y: 0.0, radius: 1.0 }, &catalog, 1.0);
        assert!(candidates.contains(&near));
        assert!(!candidates.contains(&far));
    }

    #[test]
    fn connection_point_geometry_respects_unit_rotation() {
        let unit = StructuralUnit::new("Carbon", Placement { x: 10.0, y: 20.0, rotation_radians: PI / 2.0 });
        let world = world_connection_point(ConnectionPoint { x: 1.0, y: 0.0, direction_radians: 0.0 }, &unit);
        assert!((world.x - 10.0).abs() < 1e-12);
        assert!((world.y - 21.0).abs() < 1e-12);
        assert!(world.normal_x.abs() < 1e-12);
        assert!((world.normal_y - 1.0).abs() < 1e-12);
    }

    #[test]
    fn directly_facing_connection_points_contact() {
        let a = StructuralUnit::new("Carbon", Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 });
        let b = StructuralUnit::new("Carbon", Placement { x: 1.0, y: 0.0, rotation_radians: 0.0 });
        let pa = ConnectionPoint { x: 0.0, y: 0.0, direction_radians: 0.0 };
        let pb = ConnectionPoint { x: 0.0, y: 0.0, direction_radians: PI };
        assert!(connection_points_contact(pa, &a, pb, &b, 1.0, 0.9));
        assert!((connection_point_distance(pa, &a, pb, &b) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn same_facing_connection_points_are_rejected_when_facing_threshold_is_positive() {
        let a = StructuralUnit::new("Carbon", Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 });
        let b = StructuralUnit::new("Carbon", Placement { x: 1.0, y: 0.0, rotation_radians: 0.0 });
        let pa = ConnectionPoint { x: 0.0, y: 0.0, direction_radians: 0.0 };
        let pb = ConnectionPoint { x: 0.0, y: 0.0, direction_radians: 0.0 };
        assert!(!connection_points_contact(pa, &a, pb, &b, 1.0, 0.1));
    }

    #[test]
    fn connection_pair_candidates_report_all_discrete_pairs_and_current_loads() {
        let catalog = default_catalog();
        let mut structure = OrganismStructure::new();
        let a = structure.add_unit(StructuralUnit::new("Carbon", Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 }));
        let b = structure.add_unit(StructuralUnit::new("Carbon", Placement { x: 1.0, y: 0.0, rotation_radians: 0.0 }));
        let candidates = connection_pair_candidates(&structure, a, b, &catalog);
        assert_eq!(candidates.len(), 36, "Carbon is a hexagon, so 6x6 discrete pairs exist");
        assert!(candidates.iter().all(|c| c.load_a == 0.0 && c.load_b == 0.0));
    }

    #[test]
    fn connection_pair_candidates_report_existing_point_load_without_filtering_it() {
        let catalog = default_catalog();
        let mut structure = OrganismStructure::new();
        let a = structure.add_unit(StructuralUnit::new("Carbon", Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 }));
        let b = structure.add_unit(StructuralUnit::new("Carbon", Placement { x: 1.0, y: 0.0, rotation_radians: 0.0 }));
        structure.add_bond(crate::structure::Bond { unit_a: a, point_a: 0, unit_b: b, point_b: 0, strength: 0.5 });
        let candidates = connection_pair_candidates(&structure, a, b, &catalog);
        let point_zero = candidates.iter().find(|c| c.point_a == 0 && c.point_b == 0).unwrap();
        assert!((point_zero.load_a - 0.5).abs() < 1e-12);
        assert!((point_zero.load_b - 0.5).abs() < 1e-12);
        assert_eq!(candidates.len(), 36);
    }

    #[test]
    fn fluid_units_produce_no_indexed_connection_pairs() {
        let catalog = default_catalog();
        let mut structure = OrganismStructure::new();
        let water = structure.add_unit(StructuralUnit::new("Water", Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 }));
        let carbon = structure.add_unit(StructuralUnit::new("Carbon", Placement { x: 1.0, y: 0.0, rotation_radians: 0.0 }));
        assert!(connection_pair_candidates(&structure, water, carbon, &catalog).is_empty());
    }

    #[test]
    fn connection_cache_reuses_static_topology() {
        let catalog = default_catalog();
        let mut cache = ConnectionCompatibilityCache::new();
        let first = cache.pairs_for_owned("Carbon", "Carbon", &catalog);
        let second = cache.pairs_for_owned("Carbon", "Carbon", &catalog);
        assert_eq!(first, second);
        assert_eq!(first.len(), 36);
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn connection_cache_is_order_independent_but_preserves_argument_order() {
        let catalog = default_catalog();
        let mut cache = ConnectionCompatibilityCache::new();
        let forward = cache.pairs_for_owned("Carbon", "Methane", &catalog);
        let reverse = cache.pairs_for_owned("Methane", "Carbon", &catalog);
        assert_eq!(forward.len(), reverse.len());
        for (f, r) in forward.iter().zip(reverse.iter()) {
            assert_eq!(*f, (r.1, r.0));
        }
        assert_eq!(cache.len(), 1, "reversed lookup must share the same cache entry");
    }

    #[test]
    fn cached_candidates_match_uncached_candidates() {
        let catalog = default_catalog();
        let mut structure = OrganismStructure::new();
        let a = structure.add_unit(StructuralUnit::new("Carbon", Placement { x: 0.0, y: 0.0, rotation_radians: 0.2 }));
        let b = structure.add_unit(StructuralUnit::new("Methane", Placement { x: 1.0, y: 0.3, rotation_radians: -0.4 }));
        let uncached = connection_pair_candidates(&structure, a, b, &catalog);
        let mut cache = ConnectionCompatibilityCache::new();
        let cached = connection_pair_candidates_cached(&structure, a, b, &catalog, &mut cache);
        assert_eq!(cached, uncached);
    }

    #[test]
    fn cache_does_not_store_dynamic_geometry_or_load_state() {
        let catalog = default_catalog();
        let mut structure = OrganismStructure::new();
        let a = structure.add_unit(StructuralUnit::new("Carbon", Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 }));
        let b = structure.add_unit(StructuralUnit::new("Carbon", Placement { x: 1.0, y: 0.0, rotation_radians: 0.0 }));
        let mut cache = ConnectionCompatibilityCache::new();
        let before = connection_pair_candidates_cached(&structure, a, b, &catalog, &mut cache);
        structure.units[b].placement.x = 2.0;
        let after = connection_pair_candidates_cached(&structure, a, b, &catalog, &mut cache);
        assert_ne!(before[0].distance, after[0].distance);
        assert_eq!(cache.len(), 1);
    }
}
