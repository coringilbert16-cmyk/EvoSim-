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

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct Bond {
    pub unit_a: usize,
    pub point_a: usize,
    pub unit_b: usize,
    pub point_b: usize,
    /// Bond strength is the structural strength produced by surplus investment.
    pub strength: f64,
    /// Energy stored in this bond's structural state. This is the energy BREAK
    /// releases/consumes; it is never reconstructed from resource potential energy.
    #[serde(default)]
    pub bond_energy: f64,
}

impl Bond {
    pub fn touches(&self, unit: usize, point: usize) -> bool { (self.unit_a == unit && self.point_a == point) || (self.unit_b == unit && self.point_b == point) }
    pub fn is_valid(&self, unit_count: usize, connection_point_count: impl Fn(usize) -> Option<usize>) -> bool {
        if self.unit_a >= unit_count || self.unit_b >= unit_count { return false; }
        if !self.strength.is_finite() || !(0.0..=1.0).contains(&self.strength) { return false; }
        if !self.bond_energy.is_finite() || self.bond_energy < 0.0 { return false; }
        match (connection_point_count(self.unit_a), connection_point_count(self.unit_b)) {
            (Some(a), Some(b)) => self.point_a < a && self.point_b < b,
            _ => false,
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
    pub fn disconnect_point(&mut self, unit: usize, point: usize) -> Vec<Bond> { let mut removed = Vec::new(); let mut i = 0; while i < self.bonds.len() { if self.bonds[i].touches(unit, point) { removed.push(self.bonds.remove(i)); } else { i += 1; } } removed }
    pub fn loaded_points(&self) -> Vec<(usize, usize)> { let mut pairs = Vec::new(); for bond in &self.bonds { for pair in [(bond.unit_a, bond.point_a), (bond.unit_b, bond.point_b)] { if !pairs.contains(&pair) { pairs.push(pair); } } } pairs }
}

pub fn formation_threshold(cohesion_a: f64, cohesion_b: f64, load_a: f64, load_b: f64) -> f64 { let load_a = load_a.max(0.0); let load_b = load_b.max(0.0); ((cohesion_a + cohesion_b) / 2.0) * (1.0 + load_a.sqrt() + load_b.sqrt()) }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bond_energy_is_independent_of_strength() {
        let bond = Bond { unit_a: 0, point_a: 0, unit_b: 1, point_b: 0, strength: 0.75, bond_energy: 4.5 };
        assert_eq!(bond.strength, 0.75);
        assert_eq!(bond.bond_energy, 4.5);
    }
    #[test]
    fn negative_bond_energy_is_invalid() {
        let bond = Bond { unit_a: 0, point_a: 0, unit_b: 1, point_b: 0, strength: 0.5, bond_energy: -1.0 };
        assert!(!bond.is_valid(2, |_| Some(1)));
    }
}
