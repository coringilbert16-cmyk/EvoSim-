//! Inherited structural blueprint.
use crate::resources::{BaseResource, InternalBond, Material};
use crate::structure::{Bond, BondEndpoint, OrganismStructure, Placement, StructuralUnit};
use serde::{Deserialize, Serialize};

fn default_core_elements() -> Vec<usize> { vec![0] }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct StructuralBlueprint {
    pub elements: Vec<BlueprintElement>,
    pub connections: Vec<BlueprintConnection>,
    #[serde(default = "default_core_elements")]
    pub core_elements: Vec<usize>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BlueprintElement {
    pub material: Material,
    /// Desired construction location. Rotation is deliberately not inherited:
    /// physical orientation emerges when constituents are fitted together.
    pub placement: Placement,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlueprintConnection { pub element_a: usize, pub element_b: usize }

impl StructuralBlueprint {
    pub fn new(elements: Vec<BlueprintElement>, connections: Vec<BlueprintConnection>) -> Self { Self { elements, connections, core_elements: default_core_elements() } }
    pub fn with_core_elements(elements: Vec<BlueprintElement>, connections: Vec<BlueprintConnection>, core_elements: Vec<usize>) -> Self { Self { elements, connections, core_elements } }
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
        for (i, e) in self.elements.iter().enumerate() { e.validate().map_err(|x| format!("element {i}: {x}"))?; }
        for (i, c) in self.connections.iter().enumerate() { c.validate(self).map_err(|x| format!("connection {i}: {x}"))?; }
        if self.elements.len() > 1 && !self.is_connected() { return Err("multi-element blueprint must be connected".into()); }
        if self.core_elements.len() > 1 && !self.core_is_connected() { return Err("genome core must be connected".into()); }
        Ok(())
    }

    pub fn realize(&self, catalog: &[BaseResource]) -> Result<OrganismStructure, String> {
        self.validate()?;
        let mut structure = OrganismStructure::new();
        let mut realized = std::collections::HashMap::<usize, Vec<usize>>::new();
        for (index, element) in self.elements.iter().enumerate() {
            let ids = crate::construction::realize_material(&mut structure, element, catalog)?;
            realized.insert(index, ids);
        }
        for connection in &self.connections {
            realize_connection_groups(&mut structure, &realized, *connection, catalog)?;
        }
        Ok(structure)
    }

    pub fn is_connected(&self) -> bool {
        if self.elements.is_empty() { return false; }
        let mut visited = vec![false; self.elements.len()]; let mut stack = vec![0usize]; visited[0] = true;
        while let Some(cur) = stack.pop() {
            for c in &self.connections {
                let next = if c.element_a == cur { c.element_b } else if c.element_b == cur { c.element_a } else { continue };
                if next < visited.len() && !visited[next] { visited[next] = true; stack.push(next); }
            }
        }
        visited.into_iter().all(|x| x)
    }
    fn core_is_connected(&self) -> bool {
        let core = self.core_elements.iter().copied().collect::<std::collections::HashSet<_>>();
        let mut visited = std::collections::HashSet::new(); let mut stack = vec![self.core_elements[0]]; visited.insert(self.core_elements[0]);
        while let Some(cur) = stack.pop() {
            for c in &self.connections {
                let next = if c.element_a == cur { c.element_b } else if c.element_b == cur { c.element_a } else { continue };
                if core.contains(&next) && visited.insert(next) { stack.push(next); }
            }
        }
        visited.len() == core.len()
    }
    pub fn total_material_amount(&self) -> f64 { self.elements.iter().map(|e| e.material.total_amount()).sum() }
    pub fn structural_mass(&self, catalog: &[BaseResource]) -> f64 { self.elements.iter().map(|e| e.material.mass(catalog)).sum() }
}

pub(crate) fn realize_connection_groups(structure: &mut OrganismStructure, realized: &std::collections::HashMap<usize, Vec<usize>>, connection: BlueprintConnection, catalog: &[BaseResource]) -> Result<f64, String> {
    let a = realized.get(&connection.element_a).ok_or_else(|| "missing realized first blueprint element".to_string())?;
    let b = realized.get(&connection.element_b).ok_or_else(|| "missing realized second blueprint element".to_string())?;
    for &ua in a {
        for &ub in b {
            let Some(pa) = structure.units.get(ua).and_then(|u| u.properties(catalog)) else { continue };
            let Some(pb) = structure.units.get(ub).and_then(|u| u.properties(catalog)) else { continue };
            let mut cache = crate::contact::ConnectionCompatibilityCache::new();
            let mut candidates = crate::contact::connection_pair_candidates_cached(structure, ua, ub, catalog, &mut cache);
            candidates.sort_by(|x, y| x.distance.total_cmp(&y.distance).then_with(|| y.facing.total_cmp(&x.facing)));
            let id_a = structure.physical_id(ua).ok_or_else(|| "missing first physical constituent".to_string())?;
            let id_b = structure.physical_id(ub).ok_or_else(|| "missing second physical constituent".to_string())?;
            for candidate in candidates {
                let evaluation = crate::combine::evaluate_formation(candidate, pa.cohesion, pb.cohesion);
                let (_, work, _) = crate::combine::required_investment(pa, pb, evaluation, 0.0).map_err(|e| format!("formation investment failed: {:?}", e))?;
                let strength = crate::combine::bond_strength(pa, pb);
                if !strength.is_finite() || !(0.0..=1.0).contains(&strength) { continue; }
                let bond = Bond { endpoint_a: BondEndpoint::new(id_a, candidate.endpoint_a), endpoint_b: BondEndpoint::new(id_b, candidate.endpoint_b), strength, bond_energy: 0.0 };
                if structure.bonds.iter().any(|existing| existing.has_same_identity(&bond)) { continue; }
                let mut trial = structure.clone();
                if crate::contact::try_add_bond(&mut trial, bond, catalog).is_ok() { *structure = trial; return Ok(work); }
            }
        }
    }
    Err("no physically admissible endpoint pair for blueprint connection".into())
}

impl BlueprintElement {
    pub fn validate(&self) -> Result<(), String> {
        if !self.material.is_valid() { return Err("material is invalid".into()); }
        if !self.placement.x.is_finite() || !self.placement.y.is_finite() || !self.placement.rotation_radians.is_finite() { return Err("placement must be finite".into()); }
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
    while let Some(cur) = stack.pop() {
        for InternalBond { part_a, part_b } in &material.internal_bonds {
            let next = if *part_a == cur { *part_b } else if *part_b == cur { *part_a } else { continue };
            if next < visited.len() && !visited[next] { visited[next] = true; stack.push(next); }
        }
    }
    visited.into_iter().all(|x| x)
}

impl BlueprintConnection { fn validate(&self, b: &StructuralBlueprint) -> Result<(), String> { if self.element_a >= b.elements.len() || self.element_b >= b.elements.len() { return Err("references a missing element".into()); } if self.element_a == self.element_b { return Err("self-connections are not permitted".into()); } Ok(()) } }
