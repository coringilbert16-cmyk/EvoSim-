use crate::resources::{ConnectionPoint, ConnectionSites, ResourceProperties};
use crate::structural_blueprint::BlueprintGeometry;
use crate::structural_material::StructuralMaterial;
use serde::{Deserialize, Serialize};
use std::ops::Deref;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Placement { pub x: f64, pub y: f64, pub rotation_radians: f64 }

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct UnitProperties(pub ResourceProperties);
impl Deref for UnitProperties { type Target = ResourceProperties; fn deref(&self) -> &Self::Target { &self.0 } }

/// A physical instance of one inherited blueprint element.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct StructuralUnit {
    pub material: StructuralMaterial,
    pub geometry: BlueprintGeometry,
    pub placement: Placement,
    /// Identity link to the inherited blueprint element. This is structural
    /// identity, not an anatomical role tag; it lets growth/repair restore the
    /// authored configuration without redesigning it.
    #[serde(default)]
    pub blueprint_index: Option<usize>,
}

impl StructuralUnit {
    /// Legacy synthetic construction retained only for in-module unit tests.
    /// Production/runtime code must construct physical units from an inherited
    /// blueprint via `from_blueprint_indexed`.
    #[cfg(test)]
    pub fn new(resource_name: impl Into<String>, placement: Placement) -> Self {
        let resource_name = resource_name.into();
        let catalog = crate::resources::default_catalog();
        let shape = catalog.iter().find(|resource| resource.name == resource_name).map(|resource| resource.shape.clone()).unwrap_or(crate::resources::Shape { form: crate::resources::Form::Circle { radius: 0.1 } });
        Self { material: StructuralMaterial::single(resource_name), geometry: BlueprintGeometry::single(shape), placement, blueprint_index: None }
    }

    pub fn from_blueprint(material: StructuralMaterial, geometry: BlueprintGeometry, placement: Placement) -> Self {
        Self { material, geometry, placement, blueprint_index: None }
    }

    pub fn from_blueprint_indexed(material: StructuralMaterial, geometry: BlueprintGeometry, placement: Placement, blueprint_index: usize) -> Self {
        Self { material, geometry, placement, blueprint_index: Some(blueprint_index) }
    }

    pub fn properties(&self, catalog: &[crate::resources::BaseResource]) -> Option<UnitProperties> {
        if !self.material.is_valid() { return None; }
        Some(UnitProperties(self.material.weighted_properties(catalog)))
    }

    pub fn connection_sites(&self, _catalog: &[crate::resources::BaseResource]) -> Option<ConnectionSites> { Some(self.geometry.envelope.connection_sites()) }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ConnectionSiteRef { pub unit_index: usize, pub point_index: usize }

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Bond {
    pub unit_a: usize,
    pub point_a: usize,
    pub unit_b: usize,
    pub point_b: usize,
    pub strength: f64,
    #[serde(default)]
    pub bond_energy: f64,
}
impl Bond {
    pub fn touches(&self, unit: usize, point: usize) -> bool { (self.unit_a == unit && self.point_a == point) || (self.unit_b == unit && self.point_b == point) }
    pub fn has_same_identity(&self, other: &Bond) -> bool {
        (self.unit_a == other.unit_a && self.point_a == other.point_a && self.unit_b == other.unit_b && self.point_b == other.point_b)
            || (self.unit_a == other.unit_b && self.point_a == other.point_b && self.unit_b == other.unit_a && self.point_b == other.point_a)
    }
    pub fn is_valid(&self, unit_count: usize, connection_point_count: impl Fn(usize) -> Option<usize>) -> bool {
        if self.unit_a >= unit_count || self.unit_b >= unit_count || self.unit_a == self.unit_b { return false; }
        if !self.bond_energy.is_finite() || self.bond_energy < 0.0 { return false; }
        match (connection_point_count(self.unit_a), connection_point_count(self.unit_b)) { (Some(a), Some(b)) => self.point_a < a && self.point_b < b, _ => false }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct OrganismStructure { pub units: Vec<StructuralUnit>, pub bonds: Vec<Bond> }
impl OrganismStructure {
    pub fn new() -> Self { Self::default() }
    pub fn add_unit(&mut self, unit: StructuralUnit) -> usize { self.units.push(unit); self.units.len() - 1 }
    pub fn add_bond(&mut self, bond: Bond) -> usize { self.bonds.push(bond); self.bonds.len() - 1 }
    pub fn is_valid_bond(&self, bond: &Bond, catalog: &[crate::resources::BaseResource]) -> bool {
        if !bond.is_valid(self.units.len(), |i| self.units.get(i).map(|u| u.geometry.connection_regions.len())) { return false; }
        let Some(props_a) = self.units[bond.unit_a].properties(catalog) else { return false; };
        let Some(props_b) = self.units[bond.unit_b].properties(catalog) else { return false; };
        let strength = crate::combine::bond_strength(*props_a, *props_b);
        strength.is_finite() && (0.0..=1.0).contains(&strength)
    }
    pub fn connection_site(&self, site: ConnectionSiteRef, _catalog: &[crate::resources::BaseResource]) -> Option<ConnectionPoint> { self.units.get(site.unit_index)?.geometry.connection_regions.get(site.point_index).map(|region| region.point) }
    pub fn available_connection_sites(&self, _catalog: &[crate::resources::BaseResource]) -> Vec<ConnectionSiteRef> {
        self.units.iter().enumerate().flat_map(|(unit_index, unit)| (0..unit.geometry.connection_regions.len()).map(move |point_index| ConnectionSiteRef { unit_index, point_index })).collect()
    }
    pub fn connected_components(&self) -> Vec<Vec<usize>> {
        let mut adjacency = vec![Vec::<usize>::new(); self.units.len()];
        for bond in &self.bonds { if bond.unit_a < self.units.len() && bond.unit_b < self.units.len() { adjacency[bond.unit_a].push(bond.unit_b); adjacency[bond.unit_b].push(bond.unit_a); } }
        let mut visited = vec![false; self.units.len()]; let mut components = Vec::new();
        for start in 0..self.units.len() { if visited[start] { continue; } let mut stack = vec![start]; visited[start] = true; let mut component = Vec::new(); while let Some(unit) = stack.pop() { component.push(unit); for &neighbor in &adjacency[unit] { if !visited[neighbor] { visited[neighbor] = true; stack.push(neighbor); } } } component.sort_unstable(); components.push(component); }
        components
    }
    pub fn component_connection_sites(&self, component: &[usize], catalog: &[crate::resources::BaseResource]) -> Vec<ConnectionSiteRef> {
        let component_set: std::collections::HashSet<usize> = component.iter().copied().collect();
        self.available_connection_sites(catalog).into_iter().filter(|site| component_set.contains(&site.unit_index)).collect()
    }
    pub fn connection_load(&self, unit: usize, point: usize, catalog: &[crate::resources::BaseResource]) -> f64 {
        self.bonds.iter().filter(|b| b.touches(unit, point)).filter_map(|bond| { let props_a = self.units.get(bond.unit_a)?.properties(catalog)?; let props_b = self.units.get(bond.unit_b)?.properties(catalog)?; Some(crate::combine::bond_strength(*props_a, *props_b)) }).sum()
    }
    pub fn connection_count(&self, unit: usize, point: usize) -> usize { self.bonds.iter().filter(|b| b.touches(unit, point)).count() }
    pub fn break_bond(&mut self, bond_index: usize) -> Option<Bond> { if bond_index < self.bonds.len() { Some(self.bonds.remove(bond_index)) } else { None } }
    pub fn break_matching_bond(&mut self, target: Bond) -> Option<Bond> { let index = self.bonds.iter().position(|bond| bond.has_same_identity(target))?; self.break_bond(index) }
    pub fn disconnect_point(&mut self, unit: usize, point: usize) -> Vec<Bond> { let mut removed = Vec::new(); let mut i = 0; while i < self.bonds.len() { if self.bonds[i].touches(unit, point) { removed.push(self.bonds.remove(i)); } else { i += 1; } } removed }
    pub fn loaded_points(&self) -> Vec<(usize, usize)> { let mut pairs = Vec::new(); for bond in &self.bonds { for pair in [(bond.unit_a, bond.point_a), (bond.unit_b, bond.point_b)] { if !pairs.contains(&pair) { pairs.push(pair); } } } pairs }
}

pub fn formation_threshold(cohesion_a: f64, cohesion_b: f64, load_a: f64, load_b: f64) -> f64 { let load_a = load_a.max(0.0); let load_b = load_b.max(0.0); ((cohesion_a + cohesion_b) / 2.0) * (1.0 + load_a.sqrt() + load_b.sqrt()) }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::default_catalog;
    fn placement(x: f64, y: f64) -> Placement { Placement { x, y, rotation_radians: 0.0 } }
    fn unit(s: &mut OrganismStructure, name: &str, x: f64, y: f64) -> usize { s.add_unit(StructuralUnit::new(name, placement(x, y))) }
    fn bond(a: usize, ap: usize, b: usize, bp: usize, strength: f64, energy: f64) -> Bond { Bond { unit_a: a, point_a: ap, unit_b: b, point_b: bp, strength, bond_energy: energy } }
    #[test] fn unit_properties_and_geometry_are_derived() { let catalog = default_catalog(); let mut s = OrganismStructure::new(); let i = unit(&mut s, "Carbon", 0.0, 0.0); assert_eq!(s.units[i].properties(&catalog).unwrap().cohesion, 0.95); assert_eq!(s.units[i].geometry.connection_regions.len(), 6); }
    #[test] fn connection_regions_are_not_occupancy_limited() { let catalog = default_catalog(); let mut s = OrganismStructure::new(); let a = unit(&mut s, "Carbon", 0.0, 0.0); let b = unit(&mut s, "Methane", 1.0, 0.0); s.add_bond(bond(a, 0, b, 0, 0.5, 2.0)); let sites = s.available_connection_sites(&catalog); assert!(sites.contains(&ConnectionSiteRef { unit_index: a, point_index: 0 })); assert!(sites.contains(&ConnectionSiteRef { unit_index: b, point_index: 0 })); }
    #[test] fn connected_components_form_from_bond_graph() { let mut s = OrganismStructure::new(); let a = unit(&mut s, "Carbon", 0.0, 0.0); let b = unit(&mut s, "Methane", 1.0, 0.0); let c = unit(&mut s, "Carbon", 2.0, 0.0); let _d = unit(&mut s, "Methane", 10.0, 0.0); s.add_bond(bond(a, 0, b, 0, 0.5, 2.0)); s.add_bond(bond(b, 1, c, 0, 0.5, 3.0)); assert_eq!(s.connected_components(), vec![vec![0, 1, 2], vec![3]]); }
    #[test] fn breaking_a_bond_splits_components() { let mut s = OrganismStructure::new(); let a = unit(&mut s, "Carbon", 0.0, 0.0); let b = unit(&mut s, "Methane", 1.0, 0.0); let c = unit(&mut s, "Carbon", 2.0, 0.0); let first = bond(a, 0, b, 0, 0.5, 2.0); let second = bond(b, 1, c, 0, 0.5, 3.0); s.add_bond(first); s.add_bond(second); assert_eq!(s.connected_components(), vec![vec![0, 1, 2]]); assert_eq!(s.break_matching_bond(second), Some(second)); assert_eq!(s.connected_components(), vec![vec![0, 1], vec![2]]); }
    #[test] fn bond_energy_is_separate_from_strength() { let original = bond(0, 0, 1, 0, 0.25, 4.5); let restored: Bond = serde_json::from_str(&serde_json::to_string(&original).unwrap()).unwrap(); assert_eq!(restored.strength, 0.25); assert_eq!(restored.bond_energy, 4.5); }
    #[test] fn legacy_bond_without_energy_deserializes_to_zero() { let restored: Bond = serde_json::from_str(r#"{"unit_a":0,"point_a":0,"unit_b":1,"point_b":0,"strength":0.5}"#).unwrap(); assert_eq!(restored.bond_energy, 0.0); }
    #[test] fn invalid_bond_energy_is_rejected() { assert!(bond(0, 0, 1, 0, 0.5, 1.0).is_valid(2, |_| Some(1))); assert!(!bond(0, 0, 1, 0, 0.5, -1.0).is_valid(2, |_| Some(1))); assert!(!bond(0, 0, 1, 0, 0.5, f64::NAN).is_valid(2, |_| Some(1))); }
    #[test] fn is_valid_bond_ignores_stored_strength() { let catalog = default_catalog(); let mut s = OrganismStructure::new(); let a = unit(&mut s, "Carbon", 0.0, 0.0); let b = unit(&mut s, "Methane", 1.0, 0.0); assert!(s.is_valid_bond(&bond(a, 0, b, 0, f64::NAN, 2.0), &catalog)); }
    #[test] fn connection_load_uses_intrinsic_strength() { let catalog = default_catalog(); let mut s = OrganismStructure::new(); let a = unit(&mut s, "Carbon", 0.0, 0.0); let b = unit(&mut s, "Methane", 1.0, 0.0); s.add_bond(bond(a, 0, b, 0, 0.0, 9.0)); let expected = crate::combine::bond_strength(*s.units[a].properties(&catalog).unwrap(), *s.units[b].properties(&catalog).unwrap()); assert!((s.connection_load(a, 0, &catalog) - expected).abs() < 1e-12); assert_eq!(s.connection_count(a, 0), 1); }
    #[test] fn break_bond_returns_energy() { let mut s = OrganismStructure::new(); let a = unit(&mut s, "Carbon", 0.0, 0.0); let b = unit(&mut s, "Methane", 1.0, 0.0); let i = s.add_bond(bond(a, 0, b, 0, 0.8, 7.25)); assert_eq!(s.break_bond(i).unwrap().bond_energy, 7.25); assert!(s.bonds.is_empty()); }
    #[test] fn break_matching_bond_uses_endpoint_identity() { let mut s = OrganismStructure::new(); let a = unit(&mut s, "Carbon", 0.0, 0.0); let b = unit(&mut s, "Methane", 1.0, 0.0); let stored = bond(a, 0, b, 0, 0.7999999999, 7.5); let snapshot = bond(b, 0, a, 0, 0.8, 7.25); s.add_bond(stored); assert_eq!(s.break_matching_bond(snapshot), Some(stored)); }
    #[test] fn formation_threshold_is_symmetric_and_increases_with_load() { let base = formation_threshold(0.5, 0.5, 0.0, 0.0); let loaded = formation_threshold(0.5, 0.5, 1.0, 0.0); let more = formation_threshold(0.5, 0.5, 4.0, 0.0); let a = formation_threshold(0.9, 0.1, 3.0, 1.0); let b = formation_threshold(0.1, 0.9, 1.0, 3.0); assert!(loaded > base); assert!(more > loaded); assert!((a - b).abs() < 1e-12); }
}
