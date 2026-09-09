use crate::resources::{BaseResource, ConnectionPoint, ConnectionSites};
use crate::structural_material::StructuralMaterial;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Placement { pub x: f64, pub y: f64, pub rotation_radians: f64 }

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct StructuralUnit { pub material: StructuralMaterial, pub placement: Placement }

impl StructuralUnit {
    pub fn new(resource_name: impl Into<String>, placement: Placement) -> Self { Self { material: StructuralMaterial::single(resource_name), placement } }
    pub fn from_material(material: crate::resources::Material, placement: Placement) -> Option<Self> { Some(Self { material: StructuralMaterial::from_material(material)?, placement }) }
    pub fn resource_name(&self) -> Option<&str> { let [(name, amount)] = self.material.constituents() else { return None }; if self.material.internal_bonds().is_empty() && (*amount - 1.0).abs() <= f64::EPSILON { Some(name.as_str()) } else { None } }
    pub fn properties(&self, catalog: &[BaseResource]) -> Option<crate::resources::ResourceProperties> { if !self.material.is_valid() || !self.material.resolves_in_catalog(catalog) { return None } Some(self.material.weighted_properties(catalog)) }
    pub fn connection_sites(&self, catalog: &[BaseResource]) -> Option<ConnectionSites> { self.material.connection_sites(catalog) }
    pub fn shape<'a>(&self, catalog: &'a [BaseResource]) -> Option<&'a crate::resources::Shape> { self.material.shape(catalog) }
}

impl<'de> Deserialize<'de> for StructuralUnit {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
        #[derive(Deserialize)] struct Stored { material: Option<StructuralMaterial>, resource_name: Option<String>, placement: Placement }
        let s = Stored::deserialize(deserializer)?;
        let m = match s.material { Some(m) => m, None => StructuralMaterial::single(s.resource_name.ok_or_else(|| serde::de::Error::custom("structural unit has no material"))?) };
        if !m.is_valid() { return Err(serde::de::Error::custom("invalid structural material")); }
        Ok(Self { material: m, placement: s.placement })
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ConnectionSiteRef { pub unit_index: usize, pub point_index: usize }

/// A bond endpoint is either an authored discrete connection point or a location on a continuous boundary.
/// Continuous locations are stored in the unit's local coordinates; no fabricated numbered sites are used.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum ConnectionEndpoint { Discrete(usize), Continuous { x: f64, y: f64 } }

impl ConnectionEndpoint {
    pub fn discrete(index: usize) -> Self { Self::Discrete(index) }
    pub fn is_finite(&self) -> bool { match *self { Self::Discrete(_) => true, Self::Continuous { x, y } => x.is_finite() && y.is_finite() } }
    pub fn discrete_index(&self) -> Option<usize> { match *self { Self::Discrete(i) => Some(i), Self::Continuous { .. } => None } }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Bond { pub unit_a: usize, pub point_a: ConnectionEndpoint, pub unit_b: usize, pub point_b: ConnectionEndpoint, pub strength: f64, #[serde(default)] pub bond_energy: f64 }

impl Bond {
    pub fn discrete(unit_a: usize, point_a: usize, unit_b: usize, point_b: usize, strength: f64, bond_energy: f64) -> Self { Self { unit_a, point_a: ConnectionEndpoint::Discrete(point_a), unit_b, point_b: ConnectionEndpoint::Discrete(point_b), strength, bond_energy } }
    pub fn touches(&self, unit: usize, endpoint: ConnectionEndpoint) -> bool { (self.unit_a == unit && self.point_a == endpoint) || (self.unit_b == unit && self.point_b == endpoint) }
    pub fn touches_discrete(&self, unit: usize, point: usize) -> bool { self.touches(unit, ConnectionEndpoint::Discrete(point)) }
    pub fn has_same_identity(&self, other: &Bond) -> bool { (self.unit_a == other.unit_a && self.point_a == other.point_a && self.unit_b == other.unit_b && self.point_b == other.point_b) || (self.unit_a == other.unit_b && self.point_a == other.point_b && self.unit_b == other.unit_a && self.point_b == other.point_a) }
    pub fn is_valid(&self, n: usize, sites: impl Fn(usize) -> Option<ConnectionSites>) -> bool {
        if self.unit_a >= n || self.unit_b >= n || self.unit_a == self.unit_b || !self.bond_energy.is_finite() || self.bond_energy <= 0.0 || !self.point_a.is_finite() || !self.point_b.is_finite() { return false; }
        fn endpoint_valid(endpoint: ConnectionEndpoint, sites: ConnectionSites) -> bool {
            match (sites, endpoint) {
                (ConnectionSites::Corners(points), ConnectionEndpoint::Discrete(i)) => i < points.len(),
                (ConnectionSites::Circumference { radius }, ConnectionEndpoint::Continuous { x, y }) => radius.is_finite() && radius > 0.0 && ((x * x + y * y).sqrt() - radius).abs() <= 1e-6,
                (ConnectionSites::Undetermined, ConnectionEndpoint::Continuous { .. }) => true,
                _ => false,
            }
        }
        endpoint_valid(self.point_a, sites(self.unit_a).unwrap_or(ConnectionSites::Undetermined)) && endpoint_valid(self.point_b, sites(self.unit_b).unwrap_or(ConnectionSites::Undetermined))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct OrganismStructure { pub units: Vec<StructuralUnit>, pub bonds: Vec<Bond> }

impl OrganismStructure {
    pub fn new() -> Self { Self::default() }
    pub fn add_unit(&mut self, unit: StructuralUnit) -> usize { self.units.push(unit); self.units.len() - 1 }
    pub fn add_bond(&mut self, bond: Bond) -> usize { self.bonds.push(bond); self.bonds.len() - 1 }
    pub fn is_valid_bond(&self, bond: &Bond, catalog: &[BaseResource]) -> bool {
        if !bond.is_valid(self.units.len(), |i| self.units.get(i).and_then(|u| u.connection_sites(catalog))) { return false; }
        let Some(a) = self.units[bond.unit_a].properties(catalog) else { return false };
        let Some(b) = self.units[bond.unit_b].properties(catalog) else { return false };
        let strength = crate::combine::bond_strength(a, b);
        strength.is_finite() && (0.0..=1.0).contains(&strength)
    }
    pub fn connection_site(&self, site: ConnectionSiteRef, catalog: &[BaseResource]) -> Option<ConnectionPoint> { match self.units.get(site.unit_index)?.connection_sites(catalog)? { ConnectionSites::Corners(points) => points.get(site.point_index).copied(), _ => None } }
    pub fn connection_endpoint_local(&self, unit_index: usize, endpoint: ConnectionEndpoint, catalog: &[BaseResource]) -> Option<ConnectionPoint> {
        let sites = self.units.get(unit_index)?.connection_sites(catalog)?;
        match (sites, endpoint) {
            (ConnectionSites::Corners(points), ConnectionEndpoint::Discrete(i)) => points.get(i).copied(),
            (ConnectionSites::Circumference { radius }, ConnectionEndpoint::Continuous { x, y }) => Some(ConnectionPoint { x, y, direction_radians: y.atan2(x) }).filter(|_| radius.is_finite() && radius > 0.0 && ((x*x+y*y).sqrt()-radius).abs() <= 1e-6),
            (ConnectionSites::Undetermined, ConnectionEndpoint::Continuous { x, y }) => Some(ConnectionPoint { x, y, direction_radians: y.atan2(x) }),
            _ => None,
        }
    }
    pub fn available_connection_sites(&self, catalog: &[BaseResource]) -> Vec<ConnectionSiteRef> {
        let mut out = Vec::new();
        for i in 0..self.units.len() { let Some(ConnectionSites::Corners(points)) = self.units[i].connection_sites(catalog) else { continue }; for j in 0..points.len() { out.push(ConnectionSiteRef { unit_index: i, point_index: j }); } }
        out
    }
    pub fn connected_components(&self) -> Vec<Vec<usize>> {
        let mut adjacency = vec![Vec::<usize>::new(); self.units.len()];
        for bond in &self.bonds { if bond.unit_a < self.units.len() && bond.unit_b < self.units.len() { adjacency[bond.unit_a].push(bond.unit_b); adjacency[bond.unit_b].push(bond.unit_a); } }
        let mut visited = vec![false; self.units.len()]; let mut out = Vec::new();
        for start in 0..self.units.len() { if visited[start] { continue } let mut stack = vec![start]; visited[start] = true; let mut component = Vec::new(); while let Some(unit) = stack.pop() { component.push(unit); for &next in &adjacency[unit] { if !visited[next] { visited[next] = true; stack.push(next); } } } component.sort_unstable(); out.push(component); }
        out
    }
    pub fn component_connection_sites(&self, component: &[usize], catalog: &[BaseResource]) -> Vec<ConnectionSiteRef> { let set: std::collections::HashSet<usize> = component.iter().copied().collect(); self.available_connection_sites(catalog).into_iter().filter(|site| set.contains(&site.unit_index)).collect() }
    pub fn connection_load_endpoint(&self, unit: usize, endpoint: ConnectionEndpoint, _catalog: &[BaseResource]) -> f64 { self.bonds.iter().filter(|bond| bond.touches(unit, endpoint)).map(|bond| crate::combine::experimental_bond_strength(bond.bond_energy)).sum() }
    pub fn connection_load(&self, unit: usize, point: usize, catalog: &[BaseResource]) -> f64 { self.connection_load_endpoint(unit, ConnectionEndpoint::Discrete(point), catalog) }
    pub fn connection_count_endpoint(&self, unit: usize, endpoint: ConnectionEndpoint) -> usize { self.bonds.iter().filter(|bond| bond.touches(unit, endpoint)).count() }
    pub fn connection_count(&self, unit: usize, point: usize) -> usize { self.connection_count_endpoint(unit, ConnectionEndpoint::Discrete(point)) }
    pub fn break_bond(&mut self, index: usize) -> Option<Bond> { if index < self.bonds.len() { Some(self.bonds.remove(index)) } else { None } }
    pub fn break_matching_bond(&mut self, target: Bond) -> Option<Bond> { let index = self.bonds.iter().position(|bond| bond.has_same_identity(&target))?; self.break_bond(index) }
    pub fn disconnect_point(&mut self, unit: usize, point: usize) -> Vec<Bond> { self.disconnect_endpoint(unit, ConnectionEndpoint::Discrete(point)) }
    pub fn disconnect_endpoint(&mut self, unit: usize, endpoint: ConnectionEndpoint) -> Vec<Bond> { let mut out = Vec::new(); let mut i = 0; while i < self.bonds.len() { if self.bonds[i].touches(unit, endpoint) { out.push(self.bonds.remove(i)); } else { i += 1; } } out }
    pub fn loaded_points(&self) -> Vec<(usize, usize)> { let mut out = Vec::new(); for bond in &self.bonds { for (unit, endpoint) in [(bond.unit_a, bond.point_a), (bond.unit_b, bond.point_b)] { if let ConnectionEndpoint::Discrete(point) = endpoint { if !out.contains(&(unit, point)) { out.push((unit, point)); } } } } out }
}

pub fn formation_threshold(a: f64, b: f64, load_a: f64, load_b: f64) -> f64 { let la = load_a.max(0.0); let lb = load_b.max(0.0); ((a + b) / 2.0) * (1.0 + la.sqrt() + lb.sqrt()) }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn material_is_owned_by_unit() { let unit = StructuralUnit::new("Carbon", Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 }); assert_eq!(unit.material.constituents(), &[("Carbon".into(), 1.0)]); }
    #[test] fn composite_identity_is_preserved() { let material = crate::resources::Material { parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)], internal_bonds: vec![crate::resources::InternalBond { part_a: 0, part_b: 1 }] }; let unit = StructuralUnit::from_material(material.clone(), Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 }).unwrap(); assert_eq!(unit.material.material(), &material); assert!(matches!(unit.connection_sites(&crate::resources::default_catalog()), Some(ConnectionSites::Corners(_)))); }
    #[test] fn connection_load_tracks_derived_bond_energy_strength() { let catalog = crate::resources::default_catalog(); let mut structure = OrganismStructure::new(); let a = structure.add_unit(StructuralUnit::new("Carbon", Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 })); let b = structure.add_unit(StructuralUnit::new("Carbon", Placement { x: 1.0, y: 0.0, rotation_radians: 0.0 })); structure.add_bond(Bond::discrete(a, 0, b, 0, 0.05, 1.0)); let expected = crate::combine::experimental_bond_strength(1.0); assert!((structure.connection_load(a, 0, &catalog) - expected).abs() < 1e-12); structure.bonds[0].strength = 0.95; assert!((structure.connection_load(a, 0, &catalog) - expected).abs() < 1e-12); }
    #[test] fn continuous_hydrogen_endpoint_is_not_a_numbered_site() { let catalog = crate::resources::default_catalog(); let mut structure = OrganismStructure::new(); let h = structure.add_unit(StructuralUnit::new("Hydrogen", Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 })); let endpoint = ConnectionEndpoint::Continuous { x: 1.0, y: 0.0 }; assert!(structure.connection_endpoint_local(h, endpoint, &catalog).is_some()); assert!(structure.available_connection_sites(&catalog).iter().all(|s| s.unit_index != h)); }
    #[test] fn continuous_water_endpoint_has_no_fixed_site_index() { let catalog = crate::resources::default_catalog(); let mut structure = OrganismStructure::new(); let w = structure.add_unit(StructuralUnit::new("Water", Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 })); let endpoint = ConnectionEndpoint::Continuous { x: 0.25, y: -0.5 }; assert!(structure.connection_endpoint_local(w, endpoint, &catalog).is_some()); assert!(structure.available_connection_sites(&catalog).iter().all(|s| s.unit_index != w)); }
}
