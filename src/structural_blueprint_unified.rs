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

    pub fn realize(&self, catalog: &[BaseResource]) -> Result<OrganismStructure, String> {
        let mut ledger = EnergyLedger::default();
        let mut preview_energy = 1.0e12;
        self.realize_with_context(catalog, &mut ledger, &mut preview_energy)
            .map(|(structure, _)| structure)
    }

    pub fn realize_with_context(
        &self,
        catalog: &[BaseResource],
        ledger: &mut EnergyLedger,
        energy: &mut f64,
    ) -> Result<(OrganismStructure, f64), String> {
        self.validate()?;

        // First try the authored spatial realization directly. This is the
        // zero-displacement solution and therefore must be preferred whenever
        // COMBINE can admit all declared connections at those positions.
        let mut direct_structure = OrganismStructure::new();
        let mut direct_ledger = *ledger;
        let mut direct_energy = *energy;
        let mut direct_realized = HashMap::<usize, Vec<usize>>::new();
        let mut direct_heat = 0.0;
        let mut direct_ok = true;
        for index in 0..self.elements.len() {
            match crate::construction_runtime::realize_material_with_context(
                &mut direct_structure,
                &self.elements[index],
                catalog,
                &mut direct_ledger,
                &mut direct_energy,
                &[],
            ) {
                Ok((ids, heat)) => {
                    direct_realized.insert(index, ids);
                    direct_heat += heat;
                }
                Err(_) => {
                    direct_ok = false;
                    break;
                }
            }
        }
        if direct_ok {
            for connection in &self.connections {
                let mut connected = false;
                'pair: for &a in direct_realized
                    .get(&connection.element_a)
                    .into_iter()
                    .flatten()
                {
                    for &b in direct_realized
                        .get(&connection.element_b)
                        .into_iter()
                        .flatten()
                    {
                        let mut cache = crate::contact::ConnectionCompatibilityCache::new();
                        if let Some(attempt) = crate::combine_runtime::combine_specific_pair(
                            &mut direct_structure,
                            a,
                            b,
                            catalog,
                            0.0,
                            &mut cache,
                            &mut direct_ledger,
                            &mut direct_energy,
                        ) {
                            direct_heat += attempt.work_cost;
                            connected = true;
                            break 'pair;
                        }
                    }
                }
                if !connected {
                    direct_ok = false;
                    break;
                }
            }
        }
        if direct_ok {
            *ledger = direct_ledger;
            *energy = direct_energy;
            return Ok((direct_structure, direct_heat));
        }

        // If the inherited layout cannot be admitted as-is, fall back to the
        // construction solver. It can move a realization to a physically valid
        // alternative while retaining the blueprint as the spatial target.
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
        if min_distance > crate::combine_runtime::COMBINE_CONTACT_TOLERANCE {
            return Err(format!(
                "realized material has no physical contact with a prescribed neighbor (minimum endpoint distance: {min_distance})"
            ));
        }
    }
    Ok(())
}
