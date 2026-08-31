// ============================================================
// STRUCTURAL REPRESENTATION
// ============================================================
//
// The organism-side structural layer: discrete, individually
// positioned material units connected by bonds. This sits ABOVE the
// coarse bulk Material representation used everywhere else (the
// environment's ActiveMaterialField/DeepReservoir, and an organism's
// own un-instantiated stored_unbonded stock) - nothing in this module
// changes how the environment stores or moves material. It only
// describes what a resource unit looks like once an organism has
// actually built it into something.
//
// Composition/mass/potential_energy/reactivity/cohesion/geometry are
// NEVER duplicated here - a StructuralUnit only ever stores which
// catalog resource type it is; everything else is looked up from the
// immutable catalog on demand (§48 minimum-information principle).
// ============================================================

use serde::{Deserialize, Serialize};

use crate::resources::{BaseResource, ConnectionSites, ResourceProperties};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    pub x: f64,
    pub y: f64,
    pub rotation_radians: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StructuralUnit {
    pub resource_name: String,
    pub placement: Placement,
}

impl StructuralUnit {
    pub fn new(resource_name: impl Into<String>, placement: Placement) -> Self {
        Self {
            resource_name: resource_name.into(),
            placement,
        }
    }

    pub fn properties<'a>(&self, catalog: &'a [BaseResource]) -> Option<&'a ResourceProperties> {
        catalog.iter().find(|b| b.name == self.resource_name).map(|b| &b.properties)
    }

    pub fn connection_sites(&self, catalog: &[BaseResource]) -> Option<ConnectionSites> {
        catalog.iter().find(|b| b.name == self.resource_name).map(|b| b.shape.connection_sites())
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct Bond {
    pub unit_a: usize,
    pub point_a: usize,
    pub unit_b: usize,
    pub point_b: usize,
    /// 0.0-1.0, fixed at formation. Whether strength can later change
    /// (decay/reinforcement) is unresolved - not assumed either way.
    pub strength: f64,
}

impl Bond {
    pub fn touches(&self, unit: usize, point: usize) -> bool {
        (self.unit_a == unit && self.point_a == point) || (self.unit_b == unit && self.point_b == point)
    }

    /// Structural data validation only. This deliberately does not decide
    /// whether a bond SHOULD form; formation remains the responsibility of
    /// the COMBINE/contact layer.
    pub fn is_valid(&self, unit_count: usize, connection_point_count: impl Fn(usize) -> Option<usize>) -> bool {
        if self.unit_a >= unit_count || self.unit_b >= unit_count {
            return false;
        }
        if !self.strength.is_finite() || !(0.0..=1.0).contains(&self.strength) {
            return false;
        }
        match (
            connection_point_count(self.unit_a),
            connection_point_count(self.unit_b),
        ) {
            (Some(a_count), Some(b_count)) => self.point_a < a_count && self.point_b < b_count,
            _ => false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct OrganismStructure {
    pub units: Vec<StructuralUnit>,
    pub bonds: Vec<Bond>,
}

impl OrganismStructure {
    pub fn new() -> Self { Self::default() }

    pub fn add_unit(&mut self, unit: StructuralUnit) -> usize {
        self.units.push(unit);
        self.units.len() - 1
    }

    /// Adds a bond directly. Does not decide whether the bond SHOULD form.
    /// Use `is_valid_bond` when validating externally supplied structural data.
    pub fn add_bond(&mut self, bond: Bond) -> usize {
        self.bonds.push(bond);
        self.bonds.len() - 1
    }

    /// Validates a bond against this structure and the immutable resource
    /// catalog. This is useful at the COMBINE boundary without coupling this
    /// data structure to COMBINE's formation/energy decisions.
    pub fn is_valid_bond(&self, bond: &Bond, catalog: &[BaseResource]) -> bool {
        bond.is_valid(self.units.len(), |unit_index| {
            self.units.get(unit_index).and_then(|unit| {
                unit.connection_sites(catalog).and_then(|sites| match sites {
                    ConnectionSites::Corners(points) => Some(points.len()),
                    // Continuous circumference and undetermined fluid sites
                    // have no discrete point indices in the current model.
                    ConnectionSites::Circumference { .. } | ConnectionSites::Undetermined => None,
                })
            })
        })
    }

    pub fn connection_load(&self, unit: usize, point: usize) -> f64 {
        self.bonds.iter().filter(|b| b.touches(unit, point)).map(|b| b.strength).sum()
    }

    pub fn connection_count(&self, unit: usize, point: usize) -> usize {
        self.bonds.iter().filter(|b| b.touches(unit, point)).count()
    }

    pub fn break_bond(&mut self, bond_index: usize) -> Option<Bond> {
        if bond_index < self.bonds.len() { Some(self.bonds.remove(bond_index)) } else { None }
    }

    pub fn disconnect_point(&mut self, unit: usize, point: usize) -> Vec<Bond> {
        let mut removed = Vec::new();
        let mut i = 0;
        while i < self.bonds.len() {
            if self.bonds[i].touches(unit, point) {
                removed.push(self.bonds.remove(i));
            } else {
                i += 1;
            }
        }
        removed
    }

    pub fn loaded_points(&self) -> Vec<(usize, usize)> {
        let mut pairs = Vec::new();
        for bond in &self.bonds {
            for pair in [(bond.unit_a, bond.point_a), (bond.unit_b, bond.point_b)] {
                if !pairs.contains(&pair) { pairs.push(pair); }
            }
        }
        pairs
    }
}

pub fn formation_threshold(cohesion_a: f64, cohesion_b: f64, load_a: f64, load_b: f64) -> f64 {
    let load_a = load_a.max(0.0);
    let load_b = load_b.max(0.0);
    ((cohesion_a + cohesion_b) / 2.0) * (1.0 + load_a.sqrt() + load_b.sqrt())
}

#[cfg(test)]
mod structure_tests {
    use super::*;

    fn placement(x: f64, y: f64) -> Placement {
        Placement { x, y, rotation_radians: 0.0 }
    }

    #[test]
    fn add_unit_returns_a_usable_index() {
        let mut structure = OrganismStructure::new();
        let i = structure.add_unit(StructuralUnit::new("Carbon", placement(0.0, 0.0)));
        assert_eq!(i, 0);
        assert_eq!(structure.units.len(), 1);
        assert_eq!(structure.units[0].resource_name, "Carbon");
    }

    #[test]
    fn unit_properties_and_connection_sites_are_looked_up_from_catalog_not_duplicated() {
        let catalog = crate::resources::default_catalog();
        let mut structure = OrganismStructure::new();
        let i = structure.add_unit(StructuralUnit::new("Carbon", placement(0.0, 0.0)));
        let unit = &structure.units[i];
        let props = unit.properties(&catalog).unwrap();
        assert_eq!(props.cohesion, 0.95);
        match unit.connection_sites(&catalog).unwrap() {
            ConnectionSites::Corners(points) => assert_eq!(points.len(), 6),
            other => panic!("expected Corners, got {other:?}"),
        }
    }

    #[test]
    fn invalid_bond_indices_and_strength_are_rejected_by_validation() {
        let catalog = crate::resources::default_catalog();
        let mut structure = OrganismStructure::new();
        let a = structure.add_unit(StructuralUnit::new("Carbon", placement(0.0, 0.0)));
        let b = structure.add_unit(StructuralUnit::new("Methane", placement(1.0, 0.0)));

        assert!(structure.is_valid_bond(&Bond { unit_a: a, point_a: 0, unit_b: b, point_b: 0, strength: 0.5 }, &catalog));
        assert!(!structure.is_valid_bond(&Bond { unit_a: a, point_a: 99, unit_b: b, point_b: 0, strength: 0.5 }, &catalog));
        assert!(!structure.is_valid_bond(&Bond { unit_a: a, point_a: 0, unit_b: 999, point_b: 0, strength: 0.5 }, &catalog));
        assert!(!structure.is_valid_bond(&Bond { unit_a: a, point_a: 0, unit_b: b, point_b: 0, strength: 1.1 }, &catalog));
        assert!(!structure.is_valid_bond(&Bond { unit_a: a, point_a: 0, unit_b: b, point_b: 0, strength: f64::NAN }, &catalog));
    }

    #[test]
    fn fluid_and_circumference_have_no_discrete_bond_indices() {
        let catalog = crate::resources::default_catalog();
        let mut structure = OrganismStructure::new();
        let water = structure.add_unit(StructuralUnit::new("Water", placement(0.0, 0.0)));
        let carbon = structure.add_unit(StructuralUnit::new("Carbon", placement(1.0, 0.0)));
        let bond = Bond { unit_a: water, point_a: 0, unit_b: carbon, point_b: 0, strength: 0.5 };
        assert!(!structure.is_valid_bond(&bond, &catalog));
    }

    #[test]
    fn adding_a_bond_does_not_touch_units() {
        let mut structure = OrganismStructure::new();
        let a = structure.add_unit(StructuralUnit::new("Carbon", placement(0.0, 0.0)));
        let b = structure.add_unit(StructuralUnit::new("Methane", placement(1.0, 0.0)));
        structure.add_bond(Bond { unit_a: a, point_a: 0, unit_b: b, point_b: 0, strength: 0.5 });
        assert_eq!(structure.units.len(), 2);
        assert_eq!(structure.bonds.len(), 1);
    }

    #[test]
    fn connection_load_and_count_sum_only_bonds_touching_that_point() {
        let mut structure = OrganismStructure::new();
        let a = structure.add_unit(StructuralUnit::new("Carbon", placement(0.0, 0.0)));
        let b = structure.add_unit(StructuralUnit::new("Methane", placement(1.0, 0.0)));
        let c = structure.add_unit(StructuralUnit::new("Sulfur", placement(2.0, 0.0)));
        structure.add_bond(Bond { unit_a: a, point_a: 0, unit_b: b, point_b: 0, strength: 0.3 });
        structure.add_bond(Bond { unit_a: a, point_a: 0, unit_b: c, point_b: 0, strength: 0.4 });
        structure.add_bond(Bond { unit_a: a, point_a: 1, unit_b: c, point_b: 1, strength: 0.9 });
        assert!((structure.connection_load(a, 0) - 0.7).abs() < 1e-12);
        assert_eq!(structure.connection_count(a, 0), 2);
        assert_eq!(structure.connection_count(a, 1), 1);
        assert_eq!(structure.connection_count(b, 0), 1);
    }

    #[test]
    fn break_bond_removes_only_that_one_bond() {
        let mut structure = OrganismStructure::new();
        let a = structure.add_unit(StructuralUnit::new("Carbon", placement(0.0, 0.0)));
        let units: Vec<usize> = ["Methane", "Sulfur", "Nitrogen", "Phosphorus"].iter().enumerate().map(|(k, name)| structure.add_unit(StructuralUnit::new(*name, placement(k as f64 + 1.0, 0.0)))).collect();
        let bond_ab = structure.add_bond(Bond { unit_a: a, point_a: 0, unit_b: units[0], point_b: 0, strength: 0.5 });
        structure.add_bond(Bond { unit_a: a, point_a: 1, unit_b: units[1], point_b: 0, strength: 0.5 });
        structure.add_bond(Bond { unit_a: a, point_a: 2, unit_b: units[2], point_b: 0, strength: 0.5 });
        structure.add_bond(Bond { unit_a: a, point_a: 3, unit_b: units[3], point_b: 0, strength: 0.5 });
        assert_eq!(structure.bonds.len(), 4);
        structure.break_bond(bond_ab);
        assert_eq!(structure.bonds.len(), 3);
        assert_eq!(structure.connection_count(a, 0), 0);
        assert_eq!(structure.connection_count(a, 1), 1);
        assert_eq!(structure.connection_count(a, 2), 1);
        assert_eq!(structure.connection_count(a, 3), 1);
        assert_eq!(structure.units.len(), 5);
    }

    #[test]
    fn zero_bond_unit_remains_in_structure_not_auto_reverted() {
        let mut structure = OrganismStructure::new();
        let a = structure.add_unit(StructuralUnit::new("Carbon", placement(0.0, 0.0)));
        let b = structure.add_unit(StructuralUnit::new("Methane", placement(1.0, 0.0)));
        let bond = structure.add_bond(Bond { unit_a: a, point_a: 0, unit_b: b, point_b: 0, strength: 0.5 });
        structure.break_bond(bond);
        assert_eq!(structure.connection_count(a, 0), 0);
        assert_eq!(structure.units.len(), 2);
    }

    #[test]
    fn disconnect_point_removes_every_bond_at_that_point_only() {
        let mut structure = OrganismStructure::new();
        let a = structure.add_unit(StructuralUnit::new("Carbon", placement(0.0, 0.0)));
        let b = structure.add_unit(StructuralUnit::new("Methane", placement(1.0, 0.0)));
        let c = structure.add_unit(StructuralUnit::new("Sulfur", placement(2.0, 0.0)));
        let d = structure.add_unit(StructuralUnit::new("Nitrogen", placement(3.0, 0.0)));
        structure.add_bond(Bond { unit_a: a, point_a: 0, unit_b: b, point_b: 0, strength: 0.2 });
        structure.add_bond(Bond { unit_a: a, point_a: 0, unit_b: c, point_b: 0, strength: 0.3 });
        let surviving = structure.add_bond(Bond { unit_a: a, point_a: 1, unit_b: d, point_b: 0, strength: 0.4 });
        let removed = structure.disconnect_point(a, 0);
        assert_eq!(removed.len(), 2);
        assert_eq!(structure.bonds.len(), 1);
        assert_eq!(structure.connection_count(a, 0), 0);
        assert_eq!(structure.connection_count(a, 1), 1);
        assert!(!structure.bonds.is_empty());
        let _ = surviving;
    }

    #[test]
    fn formation_threshold_increases_with_existing_load_but_not_below_base() {
        let base = formation_threshold(0.5, 0.5, 0.0, 0.0);
        assert!((base - 0.5).abs() < 1e-12);
        let loaded = formation_threshold(0.5, 0.5, 1.0, 0.0);
        assert!(loaded > base);
        let more_loaded = formation_threshold(0.5, 0.5, 4.0, 0.0);
        assert!(more_loaded > loaded);
        let step1 = formation_threshold(0.5, 0.5, 4.0, 0.0) - formation_threshold(0.5, 0.5, 0.0, 0.0);
        let step2 = formation_threshold(0.5, 0.5, 9.0, 0.0) - formation_threshold(0.5, 0.5, 4.0, 0.0);
        assert!(step2 < step1);
    }

    #[test]
    fn formation_threshold_is_symmetric_in_its_two_points() {
        let a = formation_threshold(0.9, 0.1, 3.0, 1.0);
        let b = formation_threshold(0.1, 0.9, 1.0, 3.0);
        assert!((a - b).abs() < 1e-12);
    }

    #[test]
    fn negative_load_is_treated_as_zero_defensively() {
        let t = formation_threshold(0.5, 0.5, -1.0, -1.0);
        assert!(t.is_finite());
        assert!((t - formation_threshold(0.5, 0.5, 0.0, 0.0)).abs() < 1e-12);
    }
}
