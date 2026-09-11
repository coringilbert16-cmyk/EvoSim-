//! Physical atomic seed produced from the minimal cell target.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::resources::BaseResource;
use crate::structural_blueprint::StructuralBlueprint;
use crate::structure::{Bond, BondEndpoint, ConnectionEndpoint, OrganismStructure, Placement, StructuralUnit};

const CONTACT_EPSILON: f64 = 1e-9;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct BlueprintTransform { pub x: f64, pub y: f64, #[serde(default)] pub rotation_radians: f64 }
impl BlueprintTransform { pub fn is_valid(&self) -> bool { self.x.is_finite() && self.y.is_finite() && self.rotation_radians.is_finite() } }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BlueprintAtom { pub resource: String, pub transform: BlueprintTransform }
impl BlueprintAtom { pub fn is_valid(&self) -> bool { !self.resource.is_empty() && self.transform.is_valid() } }

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct BlueprintBond { pub atom_a: usize, pub endpoint_a: ConnectionEndpoint, pub atom_b: usize, pub endpoint_b: ConnectionEndpoint, #[serde(default = "default_required_bonds")] pub required_bonds: u16 }
fn default_required_bonds() -> u16 { 1 }
impl BlueprintBond {
    pub fn is_valid(&self, atom_count: usize) -> bool { self.atom_a < atom_count && self.atom_b < atom_count && self.atom_a != self.atom_b && self.required_bonds > 0 }
    pub fn canonical(self) -> Self { if self.atom_a <= self.atom_b { self } else { Self { atom_a: self.atom_b, endpoint_a: self.endpoint_b, atom_b: self.atom_a, endpoint_b: self.endpoint_a, required_bonds: self.required_bonds } } }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AtomicBlueprint { pub anchor: BlueprintTransform, pub atoms: Vec<BlueprintAtom>, pub core_atoms: Vec<usize>, pub bonds: Vec<BlueprintBond> }

impl AtomicBlueprint {
    pub fn validate(&self) -> Result<(), String> {
        if !self.anchor.is_valid() { return Err("blueprint anchor is not finite".into()); }
        if self.atoms.is_empty() { return Err("atomic blueprint must contain at least one atom".into()); }
        for (i, atom) in self.atoms.iter().enumerate() { if !atom.is_valid() { return Err(format!("atomic blueprint atom {i} is invalid")); } }
        if self.core_atoms.is_empty() { return Err("atomic blueprint must define a core".into()); }
        let mut core_seen = vec![false; self.atoms.len()];
        for &atom in &self.core_atoms { if atom >= self.atoms.len() { return Err("atomic blueprint core references a missing atom".into()); } if core_seen[atom] { return Err("atomic blueprint core contains a duplicate atom".into()); } core_seen[atom] = true; }
        for (i, bond) in self.bonds.iter().enumerate() { if !bond.is_valid(self.atoms.len()) { return Err(format!("atomic blueprint bond {i} is invalid")); } if bond.required_bonds != 1 { return Err("atomic blueprint currently permits exactly one physical bond per prescribed relationship".into()); } let canonical = bond.canonical(); if self.bonds[..i].iter().map(|previous| previous.canonical()).any(|previous| previous == canonical) { return Err("atomic blueprint contains duplicate bonds".into()); } }
        Ok(())
    }

    pub fn realize(&self, catalog: &[BaseResource]) -> Result<OrganismStructure, String> {
        self.validate()?;
        let mut structure = OrganismStructure::new(); let (sin_anchor, cos_anchor) = self.anchor.rotation_radians.sin_cos(); let mut ids = Vec::with_capacity(self.atoms.len());
        for atom in &self.atoms { let placement = Placement { x: self.anchor.x + atom.transform.x * cos_anchor - atom.transform.y * sin_anchor, y: self.anchor.y + atom.transform.x * sin_anchor + atom.transform.y * cos_anchor, rotation_radians: self.anchor.rotation_radians + atom.transform.rotation_radians }; let mut unit = StructuralUnit::new(atom.resource.clone(), placement); if !unit.realize_default_geometry(catalog) { return Err(format!("atomic blueprint references invalid resource {}", atom.resource)); } ids.push(structure.add_unit(unit)); }
        for prescribed in &self.bonds { let a = ids[prescribed.atom_a]; let b = ids[prescribed.atom_b]; let unit_a = structure.units.get(a).ok_or_else(|| "missing realized atom A".to_string())?; let unit_b = structure.units.get(b).ok_or_else(|| "missing realized atom B".to_string())?; let point_a = prescribed.endpoint_a.world_point(unit_a, catalog).ok_or_else(|| "prescribed endpoint A is invalid for its resource".to_string())?; let point_b = prescribed.endpoint_b.world_point(unit_b, catalog).ok_or_else(|| "prescribed endpoint B is invalid for its resource".to_string())?; if (point_a.x - point_b.x).hypot(point_a.y - point_b.y) > CONTACT_EPSILON { return Err(format!("prescribed bond between atoms {} and {} is not physically realized", prescribed.atom_a, prescribed.atom_b)); } let props_a = unit_a.properties(catalog).ok_or_else(|| "invalid atom A properties".to_string())?; let props_b = unit_b.properties(catalog).ok_or_else(|| "invalid atom B properties".to_string())?; let strength = crate::combine::bond_strength(props_a, props_b); if !strength.is_finite() || !(0.0..=1.0).contains(&strength) { return Err("prescribed bond has invalid physical strength".into()); } let id_a = structure.physical_id(a).ok_or_else(|| "missing physical atom A".to_string())?; let id_b = structure.physical_id(b).ok_or_else(|| "missing physical atom B".to_string())?; let bond = Bond { endpoint_a: BondEndpoint::new(id_a, prescribed.endpoint_a), endpoint_b: BondEndpoint::new(id_b, prescribed.endpoint_b), strength, bond_energy: 0.0 }; if crate::contact::try_add_bond(&mut structure, bond, catalog).is_err() { return Err(format!("prescribed bond between atoms {} and {} is physically invalid", prescribed.atom_a, prescribed.atom_b)); } }
        Ok(structure)
    }

    pub(crate) fn compile_seed(target: &StructuralBlueprint, catalog: &[BaseResource]) -> Result<Self, String> {
        target.validate()?;
        let mut structure = OrganismStructure::new();
        let mut groups = HashMap::<usize, Vec<usize>>::new();
        let first = target.elements.first().ok_or_else(|| "seed target has no elements".to_string())?;
        let first_ids = crate::construction_realization::realize_material(&mut structure, first, catalog)
            .map_err(|error| format!("seed element 0 failed: {error}"))?;
        groups.insert(0, first_ids);
        let mut attempted = vec![false; target.elements.len()];
        attempted[0] = true;
        loop {
            let mut best = None;
            let mut best_neighbors = 0usize;
            for index in 1..target.elements.len() {
                if attempted[index] { continue; }
                let neighbors = target.connections.iter().filter_map(|connection| {
                    let neighbor = if connection.element_a == index { connection.element_b } else if connection.element_b == index { connection.element_a } else { return None };
                    groups.contains_key(&neighbor).then_some(neighbor)
                }).collect::<std::collections::HashSet<_>>();
                if neighbors.len() > best_neighbors { best_neighbors = neighbors.len(); best = Some(index); }
            }
            let Some(index) = best else { break };
            attempted[index] = true;
            let neighbor_targets = target.connections.iter().filter_map(|connection| {
                let neighbor = if connection.element_a == index { connection.element_b } else if connection.element_b == index { connection.element_a } else { return None };
                groups.get(&neighbor).cloned()
            }).collect::<Vec<_>>();
            if neighbor_targets.is_empty() { return Err(format!("seed element {index} has no realized neighbor")); }
            let ids = crate::construction_realization::realize_material_with_constraints(&mut structure, &target.elements[index], catalog, &neighbor_targets)
                .map_err(|error| {
                    let name = target.elements[index].material.parts.iter().map(|(name, _)| name.as_str()).collect::<Vec<_>>().join("+");
                    format!("seed element {index} ({name}) failed with {best_neighbors} realized neighbors: {error}")
                })?;
            groups.insert(index, ids);
        }
        if groups.len() != target.elements.len() { return Err(format!("coordinated seed realization stopped at {}/{} elements", groups.len(), target.elements.len())); }
        for connection in &target.connections { crate::structural_blueprint::realize_connection_groups(&mut structure, &groups, *connection, catalog)?; }
        let mut atoms = Vec::with_capacity(structure.units.len());
        let mut unit_to_atom = vec![usize::MAX; structure.units.len()];
        for element_index in 0..target.elements.len() {
            for &unit_index in groups.get(&element_index).ok_or_else(|| "seed compiler lost an element group".to_string())? {
                let unit = structure.units.get(unit_index).ok_or_else(|| "seed compiler lost a unit".to_string())?;
                unit_to_atom[unit_index] = atoms.len();
                atoms.push(BlueprintAtom { resource: unit.resource_name().ok_or_else(|| "seed compiler encountered a non-atomic unit".to_string())?.to_string(), transform: BlueprintTransform { x: unit.placement.x, y: unit.placement.y, rotation_radians: unit.placement.rotation_radians } });
            }
        }
        let id_to_unit = structure.units.iter().enumerate().map(|(index, unit)| (unit.physical_id, index)).collect::<HashMap<_, _>>();
        let mut bonds = Vec::with_capacity(structure.bonds.len());
        for bond in &structure.bonds {
            let unit_a = *id_to_unit.get(&bond.endpoint_a.constituent_id).ok_or_else(|| "seed compiler found a bond with a missing endpoint A".to_string())?;
            let unit_b = *id_to_unit.get(&bond.endpoint_b.constituent_id).ok_or_else(|| "seed compiler found a bond with a missing endpoint B".to_string())?;
            bonds.push(BlueprintBond { atom_a: unit_to_atom[unit_a], endpoint_a: bond.endpoint_a.location, atom_b: unit_to_atom[unit_b], endpoint_b: bond.endpoint_b.location, required_bonds: 1 });
        }
        bonds.sort_by_key(|bond| (bond.canonical().atom_a, bond.canonical().atom_b));
        let mut core_atoms = Vec::new();
        for &element in &target.core_elements { core_atoms.extend(groups.get(&element).ok_or_else(|| "seed compiler lost a core group".to_string())?.iter().map(|&unit| unit_to_atom[unit])); }
        let result = Self { anchor: BlueprintTransform { x: 0.0, y: 0.0, rotation_radians: 0.0 }, atoms, core_atoms, bonds };
        result.validate()?;
        Ok(result)
    }
}
