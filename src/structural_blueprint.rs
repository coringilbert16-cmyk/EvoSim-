//! Inherited structural blueprint.
//!
//! A blueprint specifies material intent and which material elements should
//! physically attach. It never owns the resulting constituent geometry or
//! bonds; construction solves those physical details and the structure graph
//! becomes the authority for the realization.

use crate::resources::{BaseResource, InternalBond, Material};
use crate::structure::{Bond, BondEndpoint, OrganismStructure, Placement};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::{HashMap, HashSet};

fn default_core_elements() -> Vec<usize> { vec![0] }

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
    /// Desired rigid spatial frame for this blueprint element.
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
        let rotation_radians = stored.placement.rotation_radians.unwrap_or(0.0);
        Ok(Self {
            material: stored.material,
            placement: BlueprintPlacement {
                x: stored.placement.x,
                y: stored.placement.y,
                rotation_radians,
            },
        })
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlueprintConnection { pub element_a: usize, pub element_b: usize }

impl BlueprintConnection {
    pub fn canonical(self) -> Self {
        if self.element_a <= self.element_b { self } else { Self { element_a: self.element_b, element_b: self.element_a } }
    }
    fn validate(&self, blueprint: &StructuralBlueprint) -> Result<(), String> {
        if self.element_a >= blueprint.elements.len() || self.element_b >= blueprint.elements.len() { return Err("references a missing element".into()); }
        if self.element_a == self.element_b { return Err("self-connections are not permitted".into()); }
        if *self != self.canonical() { return Err("blueprint connections must use canonical element ordering".into()); }
        Ok(())
    }
}

impl StructuralBlueprint {
    pub fn new(elements: Vec<BlueprintElement>, connections: Vec<BlueprintConnection>) -> Self {
        Self { elements, connections: Self::canonical_connections(connections), core_elements: default_core_elements() }
    }
    pub fn with_core_elements(elements: Vec<BlueprintElement>, connections: Vec<BlueprintConnection>, core_elements: Vec<usize>) -> Self {
        Self { elements, connections: Self::canonical_connections(connections), core_elements }
    }
    fn canonical_connections(connections: Vec<BlueprintConnection>) -> Vec<BlueprintConnection> {
        let mut seen = HashSet::new();
        connections.into_iter().map(BlueprintConnection::canonical).filter(|c| seen.insert((c.element_a,c.element_b))).collect()
    }
    pub fn is_valid(&self) -> bool { self.validate().is_ok() }
    pub fn validate(&self) -> Result<(), String> {
        if self.elements.is_empty() { return Err("blueprint must contain at least one element".into()); }
        if self.core_elements.is_empty() { return Err("blueprint must define a genome core".into()); }
        let mut core_seen = vec![false; self.elements.len()];
        for &index in &self.core_elements {
            if index >= self.elements.len() { return Err("genome core references a missing element".into()); }
            if core_seen[index] { return Err("genome core contains a duplicate element".into()); }
            core_seen[index] = true;
        }
        let mut connection_seen = HashSet::new();
        for (i, element) in self.elements.iter().enumerate() { element.validate().map_err(|error| format!("element {i}: {error}"))?; }
        for (i, connection) in self.connections.iter().enumerate() {
            connection.validate(self).map_err(|error| format!("connection {i}: {error}"))?;
            if !connection_seen.insert((connection.element_a, connection.element_b)) { return Err("blueprint contains duplicate connections".into()); }
        }
        if self.elements.len() > 1 && !self.is_connected() { return Err("multi-element blueprint must be connected".into()); }
        if self.core_elements.len() > 1 && !self.core_is_connected() { return Err("genome core must be connected".into()); }
        Ok(())
    }
    pub fn realize(&self, catalog: &[BaseResource]) -> Result<OrganismStructure, String> {
        self.validate()?;
        let mut structure = OrganismStructure::new();
        let mut realized = HashMap::<usize, Vec<usize>>::new();
        let first = self.elements.first().ok_or_else(|| "blueprint has no elements".to_string())?;
        let first_ids = crate::construction_realization::realize_material(&mut structure, first, catalog)?;
        realized.insert(0, first_ids);
        let mut attempted = vec![false; self.elements.len()]; attempted[0] = true;
        loop {
            let mut best = None; let mut best_neighbors = 0usize;
            for index in 1..self.elements.len() {
                if attempted[index] { continue; }
                let neighbors = self.connections.iter().filter_map(|connection| {
                    let neighbor = if connection.element_a == index { connection.element_b } else if connection.element_b == index { connection.element_a } else { return None; };
                    realized.contains_key(&neighbor).then_some(neighbor)
                }).collect::<HashSet<_>>();
                if neighbors.len() > best_neighbors { best_neighbors = neighbors.len(); best = Some(index); }
            }
            let Some(index) = best else { break; }; attempted[index] = true;
            let neighbor_targets = self.connections.iter().filter_map(|connection| {
                let neighbor = if connection.element_a == index { connection.element_b } else if connection.element_b == index { connection.element_a } else { return None; };
                realized.get(&neighbor).cloned()
            }).collect::<Vec<_>>();
            if neighbor_targets.is_empty() { continue; }
            match crate::construction_realization::realize_material_with_constraints(&mut structure, &self.elements[index], catalog, &neighbor_targets) {
                Ok(ids) => { realized.insert(index, ids); }
                Err(error) => {
                    #[cfg(test)]
                    eprintln!("BLUEPRINT ELEMENT FAILURE index={index} realized_neighbors={best_neighbors} error={error}");
                }
            }
        }
        for connection in &self.connections {
            if !realized.contains_key(&connection.element_a) || !realized.contains_key(&connection.element_b) { continue; }
            if let Err(error) = realize_connection_groups(&mut structure, &realized, *connection, catalog) {
                #[cfg(test)]
                eprintln!("BLUEPRINT CONNECTION FAILURE a={} b={} error={}", connection.element_a, connection.element_b, error);
            }
        }
        #[cfg(test)]
        eprintln!("BLUEPRINT SUMMARY elements={} realized_elements={} units={} bonds={}", self.elements.len(), realized.len(), structure.units.len(), structure.bonds.len());
        Ok(structure)
    }
    pub fn is_connected(&self) -> bool {
        if self.elements.is_empty() { return false; }
        let mut visited = vec![false; self.elements.len()]; let mut stack = vec![0usize]; visited[0] = true;
        while let Some(current) = stack.pop() { for connection in &self.connections {
            let next = if connection.element_a == current { connection.element_b } else if connection.element_b == current { connection.element_a } else { continue; };
            if next < visited.len() && !visited[next] { visited[next] = true; stack.push(next); }
        }}
        visited.into_iter().all(|visited| visited)
    }
    fn core_is_connected(&self) -> bool {
        let core = self.core_elements.iter().copied().collect::<HashSet<_>>(); let mut visited = HashSet::new(); let mut stack = vec![self.core_elements[0]]; visited.insert(self.core_elements[0]);
        while let Some(current) = stack.pop() { for connection in &self.connections {
            let next = if connection.element_a == current { connection.element_b } else if connection.element_b == current { connection.element_a } else { continue; };
            if core.contains(&next) && visited.insert(next) { stack.push(next); }
        }}
        visited.len() == core.len()
    }
    pub fn total_material_amount(&self) -> f64 { self.elements.iter().map(|element| element.material.total_amount()).sum() }
    pub fn structural_mass(&self, catalog: &[BaseResource]) -> f64 { self.elements.iter().map(|element| element.material.mass(catalog)).sum() }
}

pub(crate) fn realize_connection_groups(structure: &mut OrganismStructure, realized: &HashMap<usize, Vec<usize>>, connection: BlueprintConnection, catalog: &[BaseResource]) -> Result<f64, String> {
    let a = realized.get(&connection.element_a).ok_or_else(|| "missing realized first blueprint element".to_string())?;
    let b = realized.get(&connection.element_b).ok_or_else(|| "missing realized second blueprint element".to_string())?;
    for &ua in a { for &ub in b {
        let Some(pa) = structure.units.get(ua).and_then(|unit| unit.properties(catalog)) else { continue; };
        let Some(pb) = structure.units.get(ub).and_then(|unit| unit.properties(catalog)) else { continue; };
        let mut cache = crate::contact::ConnectionCompatibilityCache::new();
        let candidates = crate::contact::connection_pair_candidates_cached(structure, ua, ub, catalog, &mut cache);
        let id_a = structure.physical_id(ua).ok_or_else(|| "missing first physical constituent".to_string())?;
        let id_b = structure.physical_id(ub).ok_or_else(|| "missing second physical constituent".to_string())?;
        for candidate in candidates {
            if candidate.distance > 1e-9 { continue; }
            let evaluation = crate::combine::evaluate_formation(candidate, pa.cohesion, pb.cohesion);
            let (_, work, _) = crate::combine::required_investment(pa, pb, evaluation, 0.0).map_err(|error| format!("formation investment failed: {error:?}"))?;
            let strength = crate::combine::bond_strength(pa, pb); if !strength.is_finite() || !(0.0..=1.0).contains(&strength) { continue; }
            let bond = Bond { endpoint_a: BondEndpoint::new(id_a, candidate.endpoint_a), endpoint_b: BondEndpoint::new(id_b, candidate.endpoint_b), strength, bond_energy: 0.0 };
            let mut trial = structure.clone(); if crate::contact::try_add_bond(&mut trial, bond, catalog).is_ok() { *structure = trial; return Ok(work); }
        }
    }}
    Err("no physically admissible endpoint pair for blueprint connection".into())
}

impl BlueprintElement {
    pub fn validate(&self) -> Result<(), String> {
        if !self.material.is_valid() { return Err("material is invalid".into()); }
        if !self.placement.x.is_finite() || !self.placement.y.is_finite() { return Err("blueprint construction location must be finite".into()); }
        if !self.placement.rotation_radians.is_finite() { return Err("blueprint orientation must be finite".into()); }
        if self.material.parts.iter().any(|(_, amount)| (*amount - 1.0).abs() > f64::EPSILON) { return Err("each blueprint constituent must represent exactly one material unit".into()); }
        if self.material.parts.len() == 1 && !self.material.has_internal_structure() { return Ok(()); }
        if !self.material.has_internal_structure() { return Err("multi-constituent structural material must have internal bonds".into()); }
        if !material_structure_is_connected(&self.material) { return Err("internal structural material must be connected".into()); }
        Ok(())
    }
}
fn material_structure_is_connected(material: &Material) -> bool {
    if material.parts.len() <= 1 { return true; }
    let mut visited = vec![false; material.parts.len()]; let mut stack = vec![0usize]; visited[0] = true;
    while let Some(current) = stack.pop() { for InternalBond { part_a, part_b } in &material.internal_bonds {
        let next = if *part_a == current { *part_b } else if *part_b == current { *part_a } else { continue; };
        if next < visited.len() && !visited[next] { visited[next] = true; stack.push(next); }
    }}
    visited.into_iter().all(|visited| visited)
}
