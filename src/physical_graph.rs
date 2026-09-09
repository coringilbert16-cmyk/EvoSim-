use crate::resources::{BaseResource, Material};
use serde::{Deserialize, Serialize};

/// World placement of one physically instantiated constituent.
/// This type belongs to the physical graph layer so physical identity does not
/// depend on the legacy organism-structure representation.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    pub x: f64,
    pub y: f64,
    pub rotation_radians: f64,
}

/// Physical region at which a relationship may attach.
/// There is deliberately no authored bond-count field: physical fit and
/// geometry determine whether additional relationships can be admitted.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum ConnectionEndpoint {
    Corner { point_index: usize },
    Boundary { angle_radians: f64 },
    Fluid { x: f64, y: f64 },
}

impl ConnectionEndpoint {
    pub fn same_location(self, other: Self) -> bool {
        match (self, other) {
            (Self::Corner { point_index: a }, Self::Corner { point_index: b }) => a == b,
            (Self::Boundary { angle_radians: a }, Self::Boundary { angle_radians: b }) => {
                (a - b).abs() <= 1e-12
            }
            (Self::Fluid { x: ax, y: ay }, Self::Fluid { x: bx, y: by }) => {
                (ax - bx).hypot(ay - by) <= 1e-12
            }
            _ => false,
        }
    }
}

/// Stable identity for one physically instantiated constituent.
/// Aggregate material quantities are never physical graph nodes.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PhysicalConstituentId(pub u64);

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PhysicalConstituent {
    pub id: PhysicalConstituentId,
    pub resource_name: String,
    pub placement: Placement,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PhysicalRelationship {
    pub constituent_a: PhysicalConstituentId,
    pub constituent_b: PhysicalConstituentId,
    pub endpoint_a: ConnectionEndpoint,
    pub endpoint_b: ConnectionEndpoint,
    pub strength: f64,
    #[serde(default)]
    pub bond_energy: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PhysicalConstituentGraph {
    constituents: Vec<PhysicalConstituent>,
    relationships: Vec<PhysicalRelationship>,
    next_id: u64,
}

impl PhysicalConstituentGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn constituents(&self) -> &[PhysicalConstituent] {
        &self.constituents
    }

    pub fn relationships(&self) -> &[PhysicalRelationship] {
        &self.relationships
    }

    pub fn constituent(&self, id: PhysicalConstituentId) -> Option<&PhysicalConstituent> {
        self.constituents.iter().find(|c| c.id == id)
    }

    pub fn constituent_mut(&mut self, id: PhysicalConstituentId) -> Option<&mut PhysicalConstituent> {
        self.constituents.iter_mut().find(|c| c.id == id)
    }

    pub fn add_constituent(
        &mut self,
        resource_name: impl Into<String>,
        placement: Placement,
    ) -> PhysicalConstituentId {
        let id = PhysicalConstituentId(self.next_id);
        self.next_id = self
            .next_id
            .checked_add(1)
            .expect("physical constituent id overflow");
        self.constituents.push(PhysicalConstituent {
            id,
            resource_name: resource_name.into(),
            placement,
        });
        id
    }

    /// Materialization converts a physically instantiable material into
    /// individual graph constituents. It deliberately creates NO physical
    /// relationships: composition is input data, while actual structure is
    /// established only by physical realization.
    ///
    /// Every graph constituent represents one physical unit. Therefore this
    /// seam accepts only unit quantities; aggregate quantities must be split
    /// by storage/acquisition before becoming physical constituents.
    pub fn materialize_material(
        &mut self,
        material: &Material,
        placement: Placement,
    ) -> Result<Vec<PhysicalConstituentId>, &'static str> {
        if !material.is_valid() || material.is_empty() {
            return Err("material is invalid or empty");
        }
        if material
            .parts
            .iter()
            .any(|(_, amount)| (*amount - 1.0).abs() > 1e-9)
        {
            return Err("physical materialization requires one unit per constituent");
        }

        Ok(material
            .parts
            .iter()
            .map(|(name, _)| self.add_constituent(name.clone(), placement))
            .collect())
    }

    pub fn remove_constituent(&mut self, id: PhysicalConstituentId) -> Option<PhysicalConstituent> {
        let index = self.constituents.iter().position(|c| c.id == id)?;
        self.relationships
            .retain(|r| r.constituent_a != id && r.constituent_b != id);
        Some(self.constituents.remove(index))
    }

    /// Relationship admission never enforces a fixed number of bonds per endpoint.
    /// Geometry/capacity validation belongs to the physical contact layer.
    pub fn add_relationship(
        &mut self,
        relationship: PhysicalRelationship,
    ) -> Result<usize, &'static str> {
        if relationship.constituent_a == relationship.constituent_b {
            return Err("relationship requires two distinct constituents");
        }
        if self.constituent(relationship.constituent_a).is_none()
            || self.constituent(relationship.constituent_b).is_none()
        {
            return Err("relationship references an unknown constituent");
        }
        if !relationship.strength.is_finite()
            || relationship.strength < 0.0
            || !relationship.bond_energy.is_finite()
            || relationship.bond_energy < 0.0
        {
            return Err("relationship contains invalid physical values");
        }
        self.relationships.push(relationship);
        Ok(self.relationships.len() - 1)
    }

    pub fn remove_relationship(&mut self, index: usize) -> Option<PhysicalRelationship> {
        (index < self.relationships.len()).then(|| self.relationships.remove(index))
    }

    pub fn relationships_touching(
        &self,
        id: PhysicalConstituentId,
    ) -> impl Iterator<Item = &PhysicalRelationship> {
        self.relationships
            .iter()
            .filter(move |r| r.constituent_a == id || r.constituent_b == id)
    }

    pub fn relationship_count_at(
        &self,
        id: PhysicalConstituentId,
        endpoint: ConnectionEndpoint,
    ) -> usize {
        self.relationships
            .iter()
            .filter(|r| {
                (r.constituent_a == id && r.endpoint_a.same_location(endpoint))
                    || (r.constituent_b == id && r.endpoint_b.same_location(endpoint))
            })
            .count()
    }

    pub fn connected_components(&self) -> Vec<Vec<PhysicalConstituentId>> {
        let mut components = Vec::new();
        let mut unseen: std::collections::HashSet<_> =
            self.constituents.iter().map(|c| c.id).collect();

        while let Some(start) = unseen.iter().next().copied() {
            let mut stack = vec![start];
            unseen.remove(&start);
            let mut component = Vec::new();

            while let Some(id) = stack.pop() {
                component.push(id);
                for relationship in self.relationships_touching(id) {
                    let other = if relationship.constituent_a == id {
                        relationship.constituent_b
                    } else {
                        relationship.constituent_a
                    };
                    if unseen.remove(&other) {
                        stack.push(other);
                    }
                }
            }

            component.sort_unstable();
            components.push(component);
        }

        components
    }

    pub fn validate_resource_names(&self, catalog: &[BaseResource]) -> bool {
        self.constituents
            .iter()
            .all(|c| catalog.iter().any(|r| r.name == c.resource_name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::Material;

    fn placement(x: f64, y: f64) -> Placement {
        Placement {
            x,
            y,
            rotation_radians: 0.0,
        }
    }

    #[test]
    fn graph_nodes_are_individual_constituents() {
        let mut graph = PhysicalConstituentGraph::new();
        let a = graph.add_constituent("Carbon", placement(0.0, 0.0));
        let b = graph.add_constituent("Hydrogen", placement(1.0, 0.0));
        assert_ne!(a, b);
        assert_eq!(graph.constituents().len(), 2);
    }

    #[test]
    fn materialization_creates_constituents_without_inventing_structure() {
        let material = Material {
            parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
            internal_bonds: vec![crate::resources::InternalBond { part_a: 0, part_b: 1 }],
        };
        let mut graph = PhysicalConstituentGraph::new();
        let ids = graph
            .materialize_material(&material, placement(0.0, 0.0))
            .unwrap();
        assert_eq!(ids.len(), 2);
        assert!(graph.relationships().is_empty());
    }

    #[test]
    fn aggregate_material_cannot_become_one_constituent() {
        let material = Material::free_base("Carbon", 2.0);
        let mut graph = PhysicalConstituentGraph::new();
        assert!(graph
            .materialize_material(&material, placement(0.0, 0.0))
            .is_err());
    }

    #[test]
    fn constituent_identity_survives_unrelated_removal() {
        let mut graph = PhysicalConstituentGraph::new();
        let a = graph.add_constituent("Carbon", placement(0.0, 0.0));
        let b = graph.add_constituent("Hydrogen", placement(1.0, 0.0));
        let c = graph.add_constituent("Carbon", placement(2.0, 0.0));
        graph.remove_constituent(b);
        assert!(graph.constituent(a).is_some());
        assert!(graph.constituent(c).is_some());
        assert_eq!(
            graph.constituents().iter().map(|x| x.id).collect::<Vec<_>>(),
            vec![a, c]
        );
    }

    #[test]
    fn relationship_does_not_impose_endpoint_occupancy_limit() {
        let mut graph = PhysicalConstituentGraph::new();
        let a = graph.add_constituent("Carbon", placement(0.0, 0.0));
        let b = graph.add_constituent("Hydrogen", placement(1.0, 0.0));
        let c = graph.add_constituent("Hydrogen", placement(-1.0, 0.0));
        let endpoint = ConnectionEndpoint::Corner { point_index: 0 };

        for other in [b, c] {
            graph
                .add_relationship(PhysicalRelationship {
                    constituent_a: a,
                    constituent_b: other,
                    endpoint_a: endpoint,
                    endpoint_b: endpoint,
                    strength: 1.0,
                    bond_energy: 1.0,
                })
                .unwrap();
        }

        assert_eq!(graph.relationship_count_at(a, endpoint), 2);
    }

    #[test]
    fn connected_components_follow_physical_relationships() {
        let mut graph = PhysicalConstituentGraph::new();
        let a = graph.add_constituent("Carbon", placement(0.0, 0.0));
        let b = graph.add_constituent("Hydrogen", placement(1.0, 0.0));
        let c = graph.add_constituent("Water", placement(5.0, 0.0));

        graph
            .add_relationship(PhysicalRelationship {
                constituent_a: a,
                constituent_b: b,
                endpoint_a: ConnectionEndpoint::Corner { point_index: 0 },
                endpoint_b: ConnectionEndpoint::Corner { point_index: 0 },
                strength: 1.0,
                bond_energy: 1.0,
            })
            .unwrap();

        let components = graph.connected_components();
        assert_eq!(components.len(), 2);
        assert!(components.contains(&vec![a, b]));
        assert!(components.contains(&vec![c]));
    }
}