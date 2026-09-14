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

fn default_core_elements() -> Vec<usize> {
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
    #[serde(default = "default_core_elements")]
    pub core_elements: Vec<usize>,
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
            core_elements: default_core_elements(),
        }
    }

    pub fn with_core_elements(
        elements: Vec<BlueprintElement>,
        connections: Vec<BlueprintConnection>,
        core_elements: Vec<usize>,
    ) -> Self {
        Self {
            elements,
            connections: Self::canonical_connections(connections),
            core_elements,
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
        if self.core_elements.is_empty() {
            return Err("blueprint must define a genome core".into());
        }
        let mut core_seen = vec![false; self.elements.len()];
        for &index in &self.core_elements {
            if index >= self.elements.len() {
                return Err("genome core references a missing element".into());
            }
            if core_seen[index] {
                return Err("genome core contains a duplicate element".into());
            }
            core_seen[index] = true;
        }
        let mut connection_seen = HashSet::new();
        for (i, e) in self.elements.iter().enumerate() {
            e.validate().map_err(|error| format!("element {i}: {error}"))?;
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
        if self.core_elements.len() > 1 && !self.core_is_connected() {
            return Err("genome core must be connected".into());
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
        let mut queue = vec![0usize];
        visited[0] = true;
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
            return Err("blueprint realization stalled before all elements were constructed".into());
        }

        let mut total_heat = 0.0;
        for index in order {
            let external = if index == 0 {
                Vec::new()
            } else {
                self.connections
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
                    .collect::<Vec<_>>()
            };
            let (ids, heat) = crate::construction_runtime::realize_material_with_context(
                &mut structure,
                &self.elements[index],
                catalog,
                ledger,
                energy,
                &external,
            )?;
            validate_element_contact(&structure, &ids, &external, catalog)?;
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

    fn core_is_connected(&self) -> bool {
        let core = self.core_elements.iter().copied().collect::<HashSet<_>>();
        let mut visited = HashSet::new();
        let mut stack = vec![self.core_elements[0]];
        visited.insert(self.core_elements[0]);
        while let Some(current) = stack.pop() {
            for connection in &self.connections {
                let next = if connection.element_a == current {
                    connection.element_b
                } else if connection.element_b == current {
                    connection.element_a
                } else {
                    continue;
                };
                if core.contains(&next) && visited.insert(next) {
                    stack.push(next);
                }
            }
        }
        visited.len() == core.len()
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
        if !ids.iter().any(|&a| {
            group.iter().any(|&b| {
                crate::contact::connection_pair_candidates(structure, a, b, catalog)
                    .iter()
                    .any(|candidate| candidate.distance <= 1e-9)
            })
        }) {
            return Err("realized material has no physical contact with a prescribed neighbor".into());
        }
    }
    Ok(())
}

/// Compatibility surface for callers that need to add an already-realized
/// blueprint connection. This function performs the operation through COMBINE;
/// it never constructs or admits a Bond directly.
pub(crate) fn realize_connection_groups(
    structure: &mut OrganismStructure,
    realized: &HashMap<usize, Vec<usize>>,
    connection: BlueprintConnection,
    catalog: &[BaseResource],
) -> Result<f64, String> {
    let a = realized
        .get(&connection.element_a)
        .ok_or_else(|| "missing realized first blueprint element".to_string())?;
    let b = realized
        .get(&connection.element_b)
        .ok_or_else(|| "missing realized second blueprint element".to_string())?;
    let mut ledger = EnergyLedger::default();
    let mut energy = 1.0e12;
    for &ua in a {
        for &ub in b {
            if let Some(attempt) = crate::combine_runtime::combine_specific_pair(
                structure,
                ua,
                ub,
                catalog,
                0.0,
                &mut crate::contact::ConnectionCompatibilityCache::new(),
                &mut ledger,
                &mut energy,
            ) {
                return Ok(attempt.work_cost);
            }
        }
    }
    Err("no physically admissible endpoint pair for blueprint connection".into())
}
