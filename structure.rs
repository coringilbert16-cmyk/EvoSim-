use crate::resources::{BaseResource, ConnectionPoint, ConnectionSites, ResourceProperties};
use serde::{Deserialize, Serialize};

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
        catalog
            .iter()
            .find(|b| b.name == self.resource_name)
            .map(|b| &b.properties)
    }
    pub fn connection_sites(&self, catalog: &[BaseResource]) -> Option<ConnectionSites> {
        catalog
            .iter()
            .find(|b| b.name == self.resource_name)
            .map(|b| b.shape.connection_sites())
    }
}

/// Identity of a discrete connection point belonging to a structural unit.
///
/// Connection points are intrinsic to the immutable base-resource geometry,
/// so a complex structure does not create or store a second set of aggregate
/// points. The pair `(unit_index, point_index)` remains the authoritative
/// identity of every discrete connection site.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ConnectionSiteRef {
    pub unit_index: usize,
    pub point_index: usize,
}

/// Stable identity of a structural bond within an organism.
///
/// Bond IDs are assigned monotonically when bonds enter the structure and are
/// never derived from the bond vector index. This keeps an active BREAK target
/// stable even when unrelated bonds are removed and vector indices shift.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BondId(pub u64);

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Bond {
    #[serde(default)]
    pub id: BondId,
    pub unit_a: usize,
    pub point_a: usize,
    pub unit_b: usize,
    pub point_b: usize,
    pub strength: f64,
    #[serde(default)]
    pub bond_energy: f64,
}
impl PartialEq for Bond {
    fn eq(&self, other: &Self) -> bool {
        self.unit_a == other.unit_a
            && self.point_a == other.point_a
            && self.unit_b == other.unit_b
            && self.point_b == other.point_b
            && self.strength == other.strength
            && self.bond_energy == other.bond_energy
    }
}
impl Bond {
    pub fn touches(&self, unit: usize, point: usize) -> bool {
        (self.unit_a == unit && self.point_a == point)
            || (self.unit_b == unit && self.point_b == point)
    }
    pub fn is_valid(
        &self,
        unit_count: usize,
        connection_point_count: impl Fn(usize) -> Option<usize>,
    ) -> bool {
        if self.unit_a >= unit_count || self.unit_b >= unit_count {
            return false;
        }
        if !self.strength.is_finite() || !(0.0..=1.0).contains(&self.strength) {
            return false;
        }
        if !self.bond_energy.is_finite() || self.bond_energy < 0.0 {
            return false;
        }
        match (
            connection_point_count(self.unit_a),
            connection_point_count(self.unit_b),
        ) {
            (Some(a), Some(b)) => self.point_a < a && self.point_b < b,
            _ => false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct OrganismStructure {
    pub units: Vec<StructuralUnit>,
    pub bonds: Vec<Bond>,
    #[serde(default = "default_next_bond_id")]
    next_bond_id: u64,
}

fn default_next_bond_id() -> u64 {
    1
}

impl OrganismStructure {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn add_unit(&mut self, unit: StructuralUnit) -> usize {
        self.units.push(unit);
        self.units.len() - 1
    }
    pub fn add_bond(&mut self, mut bond: Bond) -> usize {
        if bond.id.0 == 0 || self.bonds.iter().any(|existing| existing.id == bond.id) {
            bond.id = self.allocate_bond_id();
        } else if bond.id.0 >= self.next_bond_id {
            self.next_bond_id = bond.id.0.saturating_add(1);
        }
        self.bonds.push(bond);
        self.bonds.len() - 1
    }
    fn allocate_bond_id(&mut self) -> BondId {
        let id = BondId(self.next_bond_id.max(1));
        self.next_bond_id = id.0.saturating_add(1);
        id
    }
    /// Repair IDs from snapshots created before stable bond identity existed.
    /// Zero IDs are legacy markers; duplicate nonzero IDs are also repaired so
    /// every live bond has exactly one stable identity.
    pub fn ensure_bond_ids(&mut self) {
        let mut next = self.next_bond_id.max(1);
        let mut used = std::collections::HashSet::new();
        for bond in &mut self.bonds {
            if bond.id.0 == 0 || !used.insert(bond.id) {
                while used.contains(&BondId(next)) || next == 0 {
                    next = next.saturating_add(1).max(1);
                }
                bond.id = BondId(next);
                used.insert(bond.id);
                next = next.saturating_add(1).max(1);
            } else if bond.id.0 >= next {
                next = bond.id.0.saturating_add(1).max(1);
            }
        }
        self.next_bond_id = next;
    }
    pub fn bond_id_at(&self, bond_index: usize) -> Option<BondId> {
        self.bonds.get(bond_index).map(|bond| bond.id)
    }
    pub fn bond_by_id(&self, id: BondId) -> Option<Bond> {
        self.bonds.iter().find(|bond| bond.id == id).copied()
    }
    pub fn break_bond_by_id(&mut self, id: BondId) -> Option<Bond> {
        let index = self.bonds.iter().position(|bond| bond.id == id)?;
        self.break_bond(index)
    }
    pub fn is_valid_bond(&self, bond: &Bond, catalog: &[BaseResource]) -> bool {
        bond.is_valid(self.units.len(), |i| {
            self.units
                .get(i)
                .and_then(|u| u.connection_sites(catalog))
                .and_then(|sites| match sites {
                    ConnectionSites::Corners(points) => Some(points.len()),
                    ConnectionSites::Circumference { .. } | ConnectionSites::Undetermined => None,
                })
        })
    }

    /// Return a discrete connection point by its structural identity.
    ///
    /// The point is derived from the base resource catalog and is not stored
    /// separately on the structure. This keeps complex-material geometry
    /// grounded in the constituent units' immutable physical geometry.
    pub fn connection_site(
        &self,
        site: ConnectionSiteRef,
        catalog: &[BaseResource],
    ) -> Option<ConnectionPoint> {
        let unit = self.units.get(site.unit_index)?;
        match unit.connection_sites(catalog)? {
            ConnectionSites::Corners(points) => points.get(site.point_index).copied(),
            ConnectionSites::Circumference { .. } | ConnectionSites::Undetermined => None,
        }
    }

    /// Return every intrinsic discrete connection point that is not occupied
    /// by a bond. A point remains available regardless of which connected
    /// component its unit belongs to; physical distance/facing eligibility is
    /// evaluated separately by the contact/geometry system.
    pub fn available_connection_sites(&self, catalog: &[BaseResource]) -> Vec<ConnectionSiteRef> {
        let mut sites = Vec::new();
        for unit_index in 0..self.units.len() {
            let Some(ConnectionSites::Corners(points)) =
                self.units[unit_index].connection_sites(catalog)
            else {
                continue;
            };
            for point_index in 0..points.len() {
                if self.connection_count(unit_index, point_index) == 0 {
                    sites.push(ConnectionSiteRef {
                        unit_index,
                        point_index,
                    });
                }
            }
        }
        sites
    }

    /// Derive the connected components of the structural graph from bonds.
    ///
    /// Units are graph nodes and bonds are edges. Components are therefore
    /// not persisted as duplicate state: breaking or forming a bond changes
    /// component membership automatically on the next query.
    pub fn connected_components(&self) -> Vec<Vec<usize>> {
        let mut adjacency = vec![Vec::<usize>::new(); self.units.len()];
        for bond in &self.bonds {
            if bond.unit_a >= self.units.len() || bond.unit_b >= self.units.len() {
                continue;
            }
            adjacency[bond.unit_a].push(bond.unit_b);
            adjacency[bond.unit_b].push(bond.unit_a);
        }

        let mut visited = vec![false; self.units.len()];
        let mut components = Vec::new();

        for start in 0..self.units.len() {
            if visited[start] {
                continue;
            }

            let mut stack = vec![start];
            visited[start] = true;
            let mut component = Vec::new();

            while let Some(unit) = stack.pop() {
                component.push(unit);
                for &neighbor in &adjacency[unit] {
                    if !visited[neighbor] {
                        visited[neighbor] = true;
                        stack.push(neighbor);
                    }
                }
            }

            component.sort_unstable();
            components.push(component);
        }

        components
    }

    /// Return the unoccupied discrete connection sites belonging to a
    /// particular connected component.
    pub fn component_connection_sites(
        &self,
        component: &[usize],
        catalog: &[BaseResource],
    ) -> Vec<ConnectionSiteRef> {
        let component_set: std::collections::HashSet<usize> = component.iter().copied().collect();
        self.available_connection_sites(catalog)
            .into_iter()
            .filter(|site| component_set.contains(&site.unit_index))
            .collect()
    }

    pub fn connection_load(&self, unit: usize, point: usize) -> f64 {
        self.bonds
            .iter()
            .filter(|b| b.touches(unit, point))
            .map(|b| b.strength)
            .sum()
    }
    pub fn connection_count(&self, unit: usize, point: usize) -> usize {
        self.bonds.iter().filter(|b| b.touches(unit, point)).count()
    }
    pub fn break_bond(&mut self, bond_index: usize) -> Option<Bond> {
        if bond_index < self.bonds.len() {
            Some(self.bonds.remove(bond_index))
        } else {
            None
        }
    }
    pub fn break_matching_bond(&mut self, target: Bond) -> Option<Bond> {
        let index = self.bonds.iter().position(|bond| *bond == target)?;
        self.break_bond(index)
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
                if !pairs.contains(&pair) {
                    pairs.push(pair);
                }
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
mod tests {
    use super::*;
    fn placement(x: f64, y: f64) -> Placement {
        Placement {
            x,
            y,
            rotation_radians: 0.0,
        }
    }
    fn unit(s: &mut OrganismStructure, name: &str, x: f64, y: f64) -> usize {
        s.add_unit(StructuralUnit::new(name, placement(x, y)))
    }
    fn bond(a: usize, ap: usize, b: usize, bp: usize, strength: f64, energy: f64) -> Bond {
        Bond {
            id: BondId(0),
            unit_a: a,
            point_a: ap,
            unit_b: b,
            point_b: bp,
            strength,
            bond_energy: energy,
        }
    }

    #[test]
    fn unit_properties_and_connection_sites_are_catalog_derived() {
        let catalog = crate::resources::default_catalog();
        let mut s = OrganismStructure::new();
        let i = unit(&mut s, "Carbon", 0.0, 0.0);
        assert_eq!(s.units[i].properties(&catalog).unwrap().cohesion, 0.95);
        match s.units[i].connection_sites(&catalog).unwrap() {
            ConnectionSites::Corners(p) => assert_eq!(p.len(), 6),
            other => panic!("expected corners, got {other:?}"),
        }
    }

    #[test]
    fn connection_site_identity_is_derived_from_unit_and_point_index() {
        let catalog = crate::resources::default_catalog();
        let mut s = OrganismStructure::new();
        let i = unit(&mut s, "Carbon", 0.0, 0.0);
        let point = s.connection_site(
            ConnectionSiteRef {
                unit_index: i,
                point_index: 0,
            },
            &catalog,
        );
        assert!(point.is_some());
        let point = point.unwrap();
        assert!((point.x - 0.438691).abs() < 1e-6);
        assert!(point.y.abs() < 1e-12);
        assert!(point.direction_radians.abs() < 1e-12);
    }

    #[test]
    fn available_connection_sites_exclude_occupied_points() {
        let catalog = crate::resources::default_catalog();
        let mut s = OrganismStructure::new();
        let a = unit(&mut s, "Carbon", 0.0, 0.0);
        let b = unit(&mut s, "Methane", 1.0, 0.0);
        s.add_bond(bond(a, 0, b, 0, 0.5, 2.0));

        let sites = s.available_connection_sites(&catalog);
        assert!(!sites.contains(&ConnectionSiteRef {
            unit_index: a,
            point_index: 0
        }));
        assert!(!sites.contains(&ConnectionSiteRef {
            unit_index: b,
            point_index: 0
        }));
        assert!(sites.contains(&ConnectionSiteRef {
            unit_index: a,
            point_index: 1
        }));
        assert!(sites.contains(&ConnectionSiteRef {
            unit_index: b,
            point_index: 1
        }));
    }

    #[test]
    fn connected_components_form_from_bond_graph() {
        let mut s = OrganismStructure::new();
        let a = unit(&mut s, "Carbon", 0.0, 0.0);
        let b = unit(&mut s, "Methane", 1.0, 0.0);
        let c = unit(&mut s, "Carbon", 2.0, 0.0);
        let _d = unit(&mut s, "Methane", 10.0, 0.0);
        s.add_bond(bond(a, 0, b, 0, 0.5, 2.0));
        s.add_bond(bond(b, 1, c, 0, 0.5, 3.0));

        let components = s.connected_components();
        assert_eq!(components, vec![vec![0, 1, 2], vec![3]]);
    }

    #[test]
    fn breaking_a_bond_splits_the_derived_components_without_extra_state() {
        let mut s = OrganismStructure::new();
        let a = unit(&mut s, "Carbon", 0.0, 0.0);
        let b = unit(&mut s, "Methane", 1.0, 0.0);
        let c = unit(&mut s, "Carbon", 2.0, 0.0);
        let first = bond(a, 0, b, 0, 0.5, 2.0);
        let second = bond(b, 1, c, 0, 0.5, 3.0);
        s.add_bond(first);
        s.add_bond(second);
        assert_eq!(s.connected_components(), vec![vec![0, 1, 2]]);

        assert_eq!(s.break_matching_bond(second), Some(second));
        assert_eq!(s.connected_components(), vec![vec![0, 1], vec![2]]);
    }

    #[test]
    fn component_connection_sites_are_only_unoccupied_sites_from_component_units() {
        let catalog = crate::resources::default_catalog();
        let mut s = OrganismStructure::new();
        let a = unit(&mut s, "Carbon", 0.0, 0.0);
        let b = unit(&mut s, "Methane", 1.0, 0.0);
        let c = unit(&mut s, "Carbon", 10.0, 0.0);
        s.add_bond(bond(a, 0, b, 0, 0.5, 2.0));

        let components = s.connected_components();
        let first_sites = s.component_connection_sites(&components[0], &catalog);
        let second_sites = s.component_connection_sites(&components[1], &catalog);

        assert!(!first_sites.contains(&ConnectionSiteRef {
            unit_index: a,
            point_index: 0
        }));
        assert!(!first_sites.contains(&ConnectionSiteRef {
            unit_index: b,
            point_index: 0
        }));
        assert!(first_sites
            .iter()
            .all(|site| site.unit_index == a || site.unit_index == b));
        assert!(second_sites.iter().all(|site| site.unit_index == c));
    }

    #[test]
    fn bond_energy_is_separate_from_strength_and_serialized() {
        let original = bond(0, 0, 1, 0, 0.25, 4.5);
        let json = serde_json::to_string(&original).unwrap();
        let restored: Bond = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.strength, 0.25);
        assert_eq!(restored.bond_energy, 4.5);
        assert_eq!(restored.id, original.id);
    }
    #[test]
    fn legacy_bond_without_energy_deserializes_to_zero() {
        let restored: Bond = serde_json::from_str(
            r#"{"unit_a":0,"point_a":0,"unit_b":1,"point_b":0,"strength":0.5}"#,
        )
        .unwrap();
        assert_eq!(restored.bond_energy, 0.0);
        assert_eq!(restored.id, BondId(0));
    }
    #[test]
    fn invalid_bond_energy_is_rejected() {
        assert!(bond(0, 0, 1, 0, 0.5, 1.0).is_valid(2, |_| Some(1)));
        assert!(!bond(0, 0, 1, 0, 0.5, -1.0).is_valid(2, |_| Some(1)));
        assert!(!bond(0, 0, 1, 0, 0.5, f64::NAN).is_valid(2, |_| Some(1)));
    }
    #[test]
    fn connection_load_and_count_track_strength_not_energy() {
        let mut s = OrganismStructure::new();
        let a = unit(&mut s, "Carbon", 0.0, 0.0);
        let b = unit(&mut s, "Methane", 1.0, 0.0);
        s.add_bond(bond(a, 0, b, 0, 0.3, 9.0));
        assert!((s.connection_load(a, 0) - 0.3).abs() < 1e-12);
        assert_eq!(s.connection_count(a, 0), 1);
    }
    #[test]
    fn break_bond_returns_the_stored_energy_with_the_bond() {
        let mut s = OrganismStructure::new();
        let a = unit(&mut s, "Carbon", 0.0, 0.0);
        let b = unit(&mut s, "Methane", 1.0, 0.0);
        let i = s.add_bond(bond(a, 0, b, 0, 0.8, 7.25));
        let removed = s.break_bond(i).unwrap();
        assert_eq!(removed.bond_energy, 7.25);
        assert!(s.bonds.is_empty());
        assert_eq!(s.units.len(), 2);
    }
    #[test]
    fn break_matching_bond_removes_the_exact_structural_bond() {
        let mut s = OrganismStructure::new();
        let a = unit(&mut s, "Carbon", 0.0, 0.0);
        let b = unit(&mut s, "Methane", 1.0, 0.0);
        let target = bond(a, 0, b, 0, 0.8, 7.25);
        s.add_bond(target);
        let removed = s.break_matching_bond(target).unwrap();
        assert_eq!(removed, target);
        assert!(s.bonds.is_empty());
    }
    #[test]
    fn bond_ids_are_unique_and_stable_across_vector_removal() {
        let mut s = OrganismStructure::new();
        let a = unit(&mut s, "Carbon", 0.0, 0.0);
        let b = unit(&mut s, "Methane", 1.0, 0.0);
        let c = unit(&mut s, "Carbon", 2.0, 0.0);
        s.add_bond(bond(a, 0, b, 0, 0.5, 2.0));
        s.add_bond(bond(b, 1, c, 0, 0.5, 3.0));
        let first_id = s.bonds[0].id;
        let second_id = s.bonds[1].id;
        assert_ne!(first_id, second_id);
        assert_eq!(s.break_bond_by_id(first_id).unwrap().id, first_id);
        assert_eq!(s.bonds[0].id, second_id);
        assert_eq!(s.bond_by_id(second_id).unwrap().id, second_id);
    }
    #[test]
    fn bond_id_is_not_derived_from_physical_state() {
        let mut s = OrganismStructure::new();
        let a = unit(&mut s, "Carbon", 0.0, 0.0);
        let b = unit(&mut s, "Methane", 1.0, 0.0);
        let index = s.add_bond(bond(a, 0, b, 0, 0.5, 2.0));
        let id = s.bonds[index].id;
        s.bonds[index].strength = 0.9;
        s.bonds[index].bond_energy = 7.0;
        assert_eq!(s.bond_by_id(id).unwrap().id, id);
        assert_eq!(s.bond_by_id(id).unwrap().strength, 0.9);
        assert_eq!(s.bond_by_id(id).unwrap().bond_energy, 7.0);
    }
    #[test]
    fn legacy_zero_bond_ids_are_migrated_without_collisions() {
        let mut s = OrganismStructure {
            units: Vec::new(),
            bonds: vec![
                bond(0, 0, 1, 0, 0.5, 1.0),
                bond(0, 1, 1, 1, 0.5, 2.0),
            ],
            next_bond_id: 1,
        };
        s.ensure_bond_ids();
        assert!(s.bonds.iter().all(|bond| bond.id.0 > 0));
        assert_ne!(s.bonds[0].id, s.bonds[1].id);
        assert_eq!(s.next_bond_id, 3);
    }
    #[test]
    fn formation_threshold_is_symmetric_and_diminishing_with_load() {
        let base = formation_threshold(0.5, 0.5, 0.0, 0.0);
        let loaded = formation_threshold(0.5, 0.5, 1.0, 0.0);
        let more = formation_threshold(0.5, 0.5, 4.0, 0.0);
        let a = formation_threshold(0.9, 0.1, 3.0, 1.0);
        let b = formation_threshold(0.1, 0.9, 1.0, 3.0);
        assert!((base - 0.5).abs() < 1e-12);
        assert!(loaded > base && more > loaded);
        assert!((a - b).abs() < 1e-12);
    }
}
