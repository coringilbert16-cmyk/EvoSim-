use serde::{Deserialize, Serialize};
use crate::resources::{BaseResource, ConnectionSites, ResourceProperties};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Placement { pub x: f64, pub y: f64, pub rotation_radians: f64 }

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StructuralUnit { pub resource_name: String, pub placement: Placement }
impl StructuralUnit {
    pub fn new(resource_name: impl Into<String>, placement: Placement) -> Self { Self { resource_name: resource_name.into(), placement } }
    pub fn properties<'a>(&self, catalog: &'a [BaseResource]) -> Option<&'a ResourceProperties> { catalog.iter().find(|b| b.name == self.resource_name).map(|b| &b.properties) }
    pub fn connection_sites(&self, catalog: &[BaseResource]) -> Option<ConnectionSites> { catalog.iter().find(|b| b.name == self.resource_name).map(|b| b.shape.connection_sites()) }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Bond {
    pub unit_a: usize, pub point_a: usize, pub unit_b: usize, pub point_b: usize,
    pub strength: f64,
    #[serde(default)] pub bond_energy: f64,
}
impl Bond {
    pub fn touches(&self, unit: usize, point: usize) -> bool { (self.unit_a == unit && self.point_a == point) || (self.unit_b == unit && self.point_b == point) }
    pub fn is_valid(&self, unit_count: usize, connection_point_count: impl Fn(usize) -> Option<usize>) -> bool {
        if self.unit_a >= unit_count || self.unit_b >= unit_count { return false; }
        if !self.strength.is_finite() || !(0.0..=1.0).contains(&self.strength) { return false; }
        if !self.bond_energy.is_finite() || self.bond_energy < 0.0 { return false; }
        match (connection_point_count(self.unit_a), connection_point_count(self.unit_b)) {
            (Some(a), Some(b)) => self.point_a < a && self.point_b < b, _ => false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct OrganismStructure { pub units: Vec<StructuralUnit>, pub bonds: Vec<Bond> }
impl OrganismStructure {
    pub fn new() -> Self { Self::default() }
    pub fn add_unit(&mut self, unit: StructuralUnit) -> usize { self.units.push(unit); self.units.len() - 1 }
    pub fn add_bond(&mut self, bond: Bond) -> usize { self.bonds.push(bond); self.bonds.len() - 1 }
    pub fn is_valid_bond(&self, bond: &Bond, catalog: &[BaseResource]) -> bool {
        bond.is_valid(self.units.len(), |i| self.units.get(i).and_then(|u| u.connection_sites(catalog)).and_then(|sites| match sites { ConnectionSites::Corners(points) => Some(points.len()), ConnectionSites::Circumference { .. } | ConnectionSites::Undetermined => None }))
    }
    pub fn connection_load(&self, unit: usize, point: usize) -> f64 { self.bonds.iter().filter(|b| b.touches(unit, point)).map(|b| b.strength).sum() }
    pub fn connection_count(&self, unit: usize, point: usize) -> usize { self.bonds.iter().filter(|b| b.touches(unit, point)).count() }
    pub fn break_bond(&mut self, bond_index: usize) -> Option<Bond> { if bond_index < self.bonds.len() { Some(self.bonds.remove(bond_index)) } else { None } }
    pub fn break_matching_bond(&mut self, target: Bond) -> Option<Bond> { let index = self.bonds.iter().position(|bond| *bond == target)?; self.break_bond(index) }
    pub fn disconnect_point(&mut self, unit: usize, point: usize) -> Vec<Bond> { let mut removed = Vec::new(); let mut i = 0; while i < self.bonds.len() { if self.bonds[i].touches(unit, point) { removed.push(self.bonds.remove(i)); } else { i += 1; } } removed }
    pub fn loaded_points(&self) -> Vec<(usize, usize)> { let mut pairs = Vec::new(); for bond in &self.bonds { for pair in [(bond.unit_a, bond.point_a), (bond.unit_b, bond.point_b)] { if !pairs.contains(&pair) { pairs.push(pair); } } } pairs }
}

pub fn formation_threshold(cohesion_a: f64, cohesion_b: f64, load_a: f64, load_b: f64) -> f64 {
    let load_a = load_a.max(0.0); let load_b = load_b.max(0.0); ((cohesion_a + cohesion_b) / 2.0) * (1.0 + load_a.sqrt() + load_b.sqrt())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn placement(x: f64, y: f64) -> Placement { Placement { x, y, rotation_radians: 0.0 } }
    fn unit(s: &mut OrganismStructure, name: &str, x: f64, y: f64) -> usize { s.add_unit(StructuralUnit::new(name, placement(x, y))) }
    fn bond(a: usize, ap: usize, b: usize, bp: usize, strength: f64, energy: f64) -> Bond { Bond { unit_a: a, point_a: ap, unit_b: b, point_b: bp, strength, bond_energy: energy } }

    #[test] fn unit_properties_and_connection_sites_are_catalog_derived() {
        let catalog = crate::resources::default_catalog(); let mut s = OrganismStructure::new(); let i = unit(&mut s, "Carbon", 0.0, 0.0);
        assert_eq!(s.units[i].properties(&catalog).unwrap().cohesion, 0.95); match s.units[i].connection_sites(&catalog).unwrap() { ConnectionSites::Corners(p) => assert_eq!(p.len(), 6), other => panic!("expected corners, got {other:?}") }
    }
    #[test] fn bond_energy_is_separate_from_strength_and_serialized() {
        let original = bond(0, 0, 1, 0, 0.25, 4.5); let json = serde_json::to_string(&original).unwrap(); let restored: Bond = serde_json::from_str(&json).unwrap(); assert_eq!(restored.strength, 0.25); assert_eq!(restored.bond_energy, 4.5);
    }
    #[test] fn legacy_bond_without_energy_deserializes_to_zero() {
        let restored: Bond = serde_json::from_str(r#"{"unit_a":0,"point_a":0,"unit_b":1,"point_b":0,"strength":0.5}"#).unwrap(); assert_eq!(restored.bond_energy, 0.0);
    }
    #[test] fn invalid_bond_energy_is_rejected() {
        assert!(bond(0, 0, 1, 0, 0.5, 1.0).is_valid(2, |_| Some(1))); assert!(!bond(0, 0, 1, 0, 0.5, -1.0).is_valid(2, |_| Some(1))); assert!(!bond(0, 0, 1, 0, 0.5, f64::NAN).is_valid(2, |_| Some(1)));
    }
    #[test] fn connection_load_and_count_track_strength_not_energy() {
        let mut s = OrganismStructure::new(); let a = unit(&mut s, "Carbon", 0.0, 0.0); let b = unit(&mut s, "Methane", 1.0, 0.0); s.add_bond(bond(a, 0, b, 0, 0.3, 9.0)); assert!((s.connection_load(a, 0) - 0.3).abs() < 1e-12); assert_eq!(s.connection_count(a, 0), 1);
    }
    #[test] fn break_bond_returns_the_stored_energy_with_the_bond() {
        let mut s = OrganismStructure::new(); let a = unit(&mut s, "Carbon", 0.0, 0.0); let b = unit(&mut s, "Methane", 1.0, 0.0); let i = s.add_bond(bond(a, 0, b, 0, 0.8, 7.25)); let removed = s.break_bond(i).unwrap(); assert_eq!(removed.bond_energy, 7.25); assert!(s.bonds.is_empty()); assert_eq!(s.units.len(), 2);
    }
    #[test] fn break_matching_bond_removes_the_exact_structural_bond() {
        let mut s = OrganismStructure::new(); let a = unit(&mut s, "Carbon", 0.0, 0.0); let b = unit(&mut s, "Methane", 1.0, 0.0); let target = bond(a, 0, b, 0, 0.8, 7.25); s.add_bond(target); let removed = s.break_matching_bond(target).unwrap(); assert_eq!(removed, target); assert!(s.bonds.is_empty());
    }
    #[test] fn formation_threshold_is_symmetric_and_diminishing_with_load() {
        let base = formation_threshold(0.5, 0.5, 0.0, 0.0); let loaded = formation_threshold(0.5, 0.5, 1.0, 0.0); let more = formation_threshold(0.5, 0.5, 4.0, 0.0); let a = formation_threshold(0.9, 0.1, 3.0, 1.0); let b = formation_threshold(0.1, 0.9, 1.0, 3.0);
        assert!((base - 0.5).abs() < 1e-12); assert!(loaded > base && more > loaded); assert!((a - b).abs() < 1e-12);
    }
}
