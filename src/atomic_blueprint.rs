//! Atomic inherited construction blueprint.

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
    pub fn from_legacy(_legacy: &StructuralBlueprint) -> Result<Self, String> { Err("legacy structural blueprint cannot be flattened deterministically: external connections do not identify constituent endpoints".into()) }
    pub(crate) fn compile_seed_from_legacy(legacy: &StructuralBlueprint, catalog: &[BaseResource]) -> Result<Self, String> {
        legacy.validate()?; let mut structure = OrganismStructure::new(); let mut groups = HashMap::<usize, Vec<usize>>::new();
        for (index, element) in legacy.elements.iter().enumerate() { let ids = crate::seed_legacy_realization::realize_material(&mut structure, element, catalog)?; groups.insert(index, ids); }
        for connection in &legacy.connections { crate::structural_blueprint::realize_connection_groups(&mut structure, &groups, *connection, catalog)?; }
        let mut atoms = Vec::with_capacity(structure.units.len()); let mut unit_to_atom = vec![usize::MAX; structure.units.len()];
        for (element_index, element) in legacy.elements.iter().enumerate() { for &unit_index in groups.get(&element_index).ok_or_else(|| "seed compiler lost an element group".to_string())? { let unit = structure.units.get(unit_index).ok_or_else(|| "seed compiler lost a unit".to_string())?; unit_to_atom[unit_index] = atoms.len(); atoms.push(BlueprintAtom { resource: unit.resource_name().ok_or_else(|| "seed compiler encountered a non-atomic unit".to_string())?.to_string(), transform: BlueprintTransform { x: unit.placement.x, y: unit.placement.y, rotation_radians: unit.placement.rotation_radians } }); } }
        let id_to_unit = structure.units.iter().enumerate().map(|(index, unit)| (unit.physical_id, index)).collect::<HashMap<_, _>>(); let mut bonds = Vec::with_capacity(structure.bonds.len());
        for bond in &structure.bonds { let unit_a = *id_to_unit.get(&bond.endpoint_a.constituent_id).ok_or_else(|| "seed compiler found a bond with a missing endpoint A".to_string())?; let unit_b = *id_to_unit.get(&bond.endpoint_b.constituent_id).ok_or_else(|| "seed compiler found a bond with a missing endpoint B".to_string())?; bonds.push(BlueprintBond { atom_a: unit_to_atom[unit_a], endpoint_a: bond.endpoint_a.location, atom_b: unit_to_atom[unit_b], endpoint_b: bond.endpoint_b.location, required_bonds: 1 }); }
        bonds.sort_by_key(|bond| (bond.canonical().atom_a, bond.canonical().atom_b)); let mut core_atoms = Vec::new(); for &element in &legacy.core_elements { core_atoms.extend(groups.get(&element).ok_or_else(|| "seed compiler lost a core group".to_string())?.iter().map(|&unit| unit_to_atom[unit])); }
        let result = Self { anchor: BlueprintTransform { x: 0.0, y: 0.0, rotation_radians: 0.0 }, atoms, core_atoms, bonds }; result.validate()?; Ok(result)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn carbon(x: f64) -> BlueprintAtom { BlueprintAtom { resource: "Carbon".into(), transform: BlueprintTransform { x, y: 0.0, rotation_radians: 0.0 } } }
    #[test] fn atomic_blueprint_requires_explicit_constituent_endpoints() { let blueprint = AtomicBlueprint { anchor: BlueprintTransform { x: 0.0, y: 0.0, rotation_radians: 0.0 }, atoms: vec![carbon(0.0), carbon(1.0)], core_atoms: vec![0], bonds: vec![BlueprintBond { atom_a: 0, endpoint_a: ConnectionEndpoint::Corner { point_index: 0 }, atom_b: 1, endpoint_b: ConnectionEndpoint::Corner { point_index: 3 }, required_bonds: 1 }] }; assert!(blueprint.validate().is_ok()); }
    #[test] fn atomic_blueprint_rejects_duplicate_relationships() { let blueprint = AtomicBlueprint { anchor: BlueprintTransform { x: 0.0, y: 0.0, rotation_radians: 0.0 }, atoms: vec![carbon(0.0), carbon(1.0)], core_atoms: vec![0], bonds: vec![BlueprintBond { atom_a: 0, endpoint_a: ConnectionEndpoint::Corner { point_index: 0 }, atom_b: 1, endpoint_b: ConnectionEndpoint::Corner { point_index: 3 }, required_bonds: 1 }, BlueprintBond { atom_a: 1, endpoint_a: ConnectionEndpoint::Corner { point_index: 3 }, atom_b: 0, endpoint_b: ConnectionEndpoint::Corner { point_index: 0 }, required_bonds: 1 }] }; assert!(blueprint.validate().is_err()); }
    #[test] fn atomic_realization_preserves_anchor_orientation() { let blueprint = AtomicBlueprint { anchor: BlueprintTransform { x: 10.0, y: 20.0, rotation_radians: std::f64::consts::FRAC_PI_2 }, atoms: vec![BlueprintAtom { resource: "Carbon".into(), transform: BlueprintTransform { x: 2.0, y: 0.0, rotation_radians: 0.25 } }], core_atoms: vec![0], bonds: Vec::new() }; let structure = blueprint.realize(&crate::resources::default_catalog()).unwrap(); assert!((structure.units[0].placement.x - 10.0).abs() < 1e-9); assert!((structure.units[0].placement.y - 22.0).abs() < 1e-9); assert!((structure.units[0].placement.rotation_radians - (std::f64::consts::FRAC_PI_2 + 0.25)).abs() < 1e-9); }
    #[test] fn atomic_realization_fulfills_the_prescribed_endpoint_identity() { let radius = crate::resources::default_catalog().into_iter().find(|r| r.name == "Carbon").unwrap().shape.form.bounding_radius(); let blueprint = AtomicBlueprint { anchor: BlueprintTransform { x: 0.0, y: 0.0, rotation_radians: 0.0 }, atoms: vec![carbon(0.0), carbon(2.0 * radius)], core_atoms: vec![0], bonds: vec![BlueprintBond { atom_a: 0, endpoint_a: ConnectionEndpoint::Corner { point_index: 0 }, atom_b: 1, endpoint_b: ConnectionEndpoint::Corner { point_index: 3 }, required_bonds: 1 }] }; let structure = blueprint.realize(&crate::resources::default_catalog()).unwrap(); assert_eq!(structure.units.len(), 2); assert_eq!(structure.bonds.len(), 1); assert_eq!(structure.bonds[0].endpoint_a.constituent_id, structure.physical_id(0).unwrap()); assert_eq!(structure.bonds[0].endpoint_b.constituent_id, structure.physical_id(1).unwrap()); assert_eq!(structure.bonds[0].endpoint_a.location, ConnectionEndpoint::Corner { point_index: 0 }); assert_eq!(structure.bonds[0].endpoint_b.location, ConnectionEndpoint::Corner { point_index: 3 }); }
    #[test] fn legacy_flattening_never_guesses_external_topology() { let legacy = crate::genome::initial_genome().structural_blueprint; let error = AtomicBlueprint::from_legacy(&legacy).unwrap_err(); assert!(error.contains("does not identify constituent endpoints")); }
    #[test] fn seed_topology_compiles_to_exact_atomic_graph() { let legacy = crate::genome::initial_genome().structural_blueprint; let catalog = crate::resources::default_catalog(); let blueprint = AtomicBlueprint::compile_seed_from_legacy(&legacy, &catalog).unwrap(); assert_eq!(blueprint.atoms.len(), 183); assert_eq!(blueprint.core_atoms.len(), 57); assert_eq!(blueprint.bonds.len(), 278); let structure = blueprint.realize(&catalog).unwrap(); assert_eq!(structure.units.len(), 183); assert_eq!(structure.bonds.len(), 278); }
}
