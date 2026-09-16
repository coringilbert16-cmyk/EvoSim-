//! Unified inherited structural blueprint authority.
//!
//! Blueprint intent is separate from physical realization. Actual constituent
//! and bond creation is delegated to the construction runtime, which delegates
//! every bond admission to COMBINE. `realize()` is a local, non-persistent
//! preview; simulation construction must use `realize_with_context()`.

use crate::resources::{BaseResource, Material};
use crate::state::EnergyLedger;
use crate::structure::OrganismStructure;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::{HashMap, HashSet};

fn default_anchor_elements() -> Vec<usize> {
    vec![0]
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct BlueprintPlacement {
    pub x: f64,
    pub y: f64,
    #[serde(default)]
    pub rotation_radians: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct StructuralBlueprint {
    pub elements: Vec<BlueprintElement>,
    pub connections: Vec<BlueprintConnection>,
    /// Construction anchors identify where realization may begin. They are
    /// not a biological genome definition and do not identify the genome.
    #[serde(default = "default_anchor_elements")]
    pub anchor_elements: Vec<usize>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct BlueprintElement {
    pub material: Material,
    pub placement: BlueprintPlacement,
}

impl<'de> Deserialize<'de> for BlueprintElement {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct StoredPlacement {
            x: f64,
            y: f64,
            #[serde(default)]
            rotation_radians: Option<f64>,
        }
        #[derive(Deserialize)]
        struct Stored {
            material: Material,
            placement: StoredPlacement,
        }
        let stored = Stored::deserialize(deserializer)?;
        Ok(Self {
            material: stored.material,
            placement: BlueprintPlacement {
                x: stored.placement.x,
                y: stored.placement.y,
                rotation_radians: stored.placement.rotation_radians.unwrap_or(0.0),
            },
        })
    }
}

impl BlueprintElement {
    pub fn validate(&self) -> Result<(), String> {
        if !self.material.is_valid() {
            return Err("material is invalid".into());
        }
        if !self.placement.x.is_finite() || !self.placement.y.is_finite() {
            return Err("blueprint placement must be finite".into());
        }
        if !self.placement.rotation_radians.is_finite() {
            return Err("blueprint orientation must be finite".into());
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlueprintConnection {
    pub element_a: usize,
    pub element_b: usize,
}

impl BlueprintConnection {
    pub fn canonical(self) -> Self {
        if self.element_a <= self.element_b {
            self
        } else {
            Self {
                element_a: self.element_b,
                element_b: self.element_a,
            }
        }
    }

    fn validate(&self, b: &StructuralBlueprint) -> Result<(), String> {
        if self.element_a >= b.elements.len() || self.element_b >= b.elements.len() {
            return Err("references a missing element".into());
        }
        if self.element_a == self.element_b {
            return Err("self-connections are not permitted".into());
        }
        if *self != self.canonical() {
            return Err("blueprint connections must use canonical element ordering".into());
        }
        Ok(())
    }
}

impl StructuralBlueprint {
    pub fn new(elements: Vec<BlueprintElement>, connections: Vec<BlueprintConnection>) -> Self {
        Self {
            elements,
            connections: Self::canonical_connections(connections),
            anchor_elements: default_anchor_elements(),
        }
    }

    pub fn with_anchor_elements(
        elements: Vec<BlueprintElement>,
        connections: Vec<BlueprintConnection>,
        anchor_elements: Vec<usize>,
    ) -> Self {
        Self {
            elements,
            connections: Self::canonical_connections(connections),
            anchor_elements,
        }
    }

    fn canonical_connections(connections: Vec<BlueprintConnection>) -> Vec<BlueprintConnection> {
        let mut seen = HashSet::new();
        connections
            .into_iter()
            .map(BlueprintConnection::canonical)
            .filter(|c| seen.insert((c.element_a, c.element_b)))
            .collect()
    }

    pub fn is_valid(&self) -> bool {
        self.validate().is_ok()
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.elements.is_empty() {
            return Err("blueprint must contain at least one element".into());
        }
        if self.anchor_elements.is_empty() {
            return Err("blueprint must define at least one construction anchor".into());
        }
        let mut anchor_seen = vec![false; self.elements.len()];
        for &index in &self.anchor_elements {
            if index >= self.elements.len() {
                return Err("construction anchor references a missing element".into());
            }
            if anchor_seen[index] {
                return Err("construction anchors contain a duplicate element".into());
            }
            anchor_seen[index] = true;
        }
        let mut connection_seen = HashSet::new();
        for (i, e) in self.elements.iter().enumerate() {
            e.validate()
                .map_err(|error| format!("element {i}: {error}"))?;
        }
        for (i, c) in self.connections.iter().enumerate() {
            c.validate(self)
                .map_err(|error| format!("connection {i}: {error}"))?;
            if !connection_seen.insert((c.element_a, c.element_b)) {
                return Err("blueprint contains duplicate connections".into());
            }
        }
        if self.elements.len() > 1 && !self.is_connected() {
            return Err("multi-element blueprint must be connected".into());
        }
        Ok(())
    }

    /// Non-persistent physical preview. It uses a private trial energy budget
    /// solely so COMBINE can evaluate its real admission rules. No simulation
    /// ledger, organism energy, or structure is mutated by this method.
    pub fn realize(&self, catalog: &[BaseResource]) -> Result<OrganismStructure, String> {
        let mut ledger = EnergyLedger::default();
        let mut preview_energy = 1.0e12;
        self.realize_with_context(catalog, &mut ledger, &mut preview_energy)
            .map(|(structure, _)| structure)
    }

    /// Actual blueprint realization. All elements share one energy holder and
    /// one ledger; each material's internal and external bonds are admitted by
    /// the same COMBINE runtime.
    pub fn realize_with_context(
        &self,
        catalog: &[BaseResource],
        ledger: &mut EnergyLedger,
        energy: &mut f64,
    ) -> Result<(OrganismStructure, f64), String> {
        self.validate()?;
        let mut structure = OrganismStructure::new();
        let mut realized = HashMap::<usize, Vec<usize>>::new();
        let mut order = Vec::with_capacity(self.elements.len());
        let mut visited = vec![false; self.elements.len()];
        let mut queue = vec![self.anchor_elements[0]];
        visited[self.anchor_elements[0]] = true;
        while let Some(current) = queue.pop() {
            order.push(current);
            for connection in &self.connections {
                let neighbor = if connection.element_a == current {
                    connection.element_b
                } else if connection.element_b == current {
                    connection.element_a
                } else {
                    continue;
                };
                if !visited[neighbor] {
                    visited[neighbor] = true;
                    queue.push(neighbor);
                }
            }
        }
        if order.len() != self.elements.len() {
            return Err(
                "blueprint realization stalled before all elements were constructed".into(),
            );
        }

        let mut total_heat = 0.0;
        for index in order {
            let external = self
                .connections
                .iter()
                .filter_map(|connection| {
                    let neighbor = if connection.element_a == index {
                        connection.element_b
                    } else if connection.element_b == index {
                        connection.element_a
                    } else {
                        return None;
                    };
                    realized.get(&neighbor).cloned()
                })
                .collect::<Vec<_>>();
            let (ids, heat) = crate::construction_runtime::realize_material_with_context(
                &mut structure,
                &self.elements[index],
                catalog,
                ledger,
                energy,
                &external,
            )
            .map_err(|error| format!("element {index} construction failed: {error}"))?;
            validate_element_contact(&structure, &ids, &external, catalog)
                .map_err(|error| format!("element {index} contact validation failed: {error}"))?;
            realized.insert(index, ids);
            total_heat += heat;
        }

        Ok((structure, total_heat))
    }

    pub fn is_connected(&self) -> bool {
        if self.elements.is_empty() {
            return false;
        }
        let mut visited = vec![false; self.elements.len()];
        let mut stack = vec![0usize];
        visited[0] = true;
        while let Some(current) = stack.pop() {
            for connection in &self.connections {
                let next = if connection.element_a == current {
                    connection.element_b
                } else if connection.element_b == current {
                    connection.element_a
                } else {
                    continue;
                };
                if next < visited.len() && !visited[next] {
                    visited[next] = true;
                    stack.push(next);
                }
            }
        }
        visited.into_iter().all(|visited| visited)
    }

    pub fn total_material_amount(&self) -> f64 {
        self.elements
            .iter()
            .map(|element| element.material.total_amount())
            .sum()
    }

    pub fn structural_mass(&self, catalog: &[BaseResource]) -> f64 {
        self.elements
            .iter()
            .map(|element| element.material.mass(catalog))
            .sum()
    }
}

fn validate_element_contact(
    structure: &OrganismStructure,
    ids: &[usize],
    neighbors: &[Vec<usize>],
    catalog: &[BaseResource],
) -> Result<(), String> {
    for &id in ids {
        if structure.units.get(id).is_none() {
            return Err("realized material references a missing constituent".to_string());
        }
    }
    for group in neighbors {
        let min_distance = ids
            .iter()
            .flat_map(|&a| {
                group.iter().flat_map(move |&b| {
                    crate::contact::connection_pair_candidates(structure, a, b, catalog)
                        .into_iter()
                        .map(|candidate| candidate.distance)
                })
            })
            .fold(f64::INFINITY, f64::min);
        if min_distance > 1e-9 {
            return Err(format!(
                "realized material has no physical contact with a prescribed neighbor (minimum endpoint distance: {min_distance})"
            ));
        }
    }
    Ok(())
}
