//! Blueprint-constrained structural repair.
//!
//! Damage may remove structural units or bonds. Repair restores only the
//! missing configuration authored by the organism's inherited blueprint. It
//! never chooses a new placement, topology, or material identity.

use crate::resources::{BaseResource, Material};
use crate::state::Organism;
use crate::structure::{Bond, StructuralUnit};

pub(crate) fn repair_one_element(organism: &mut Organism, catalog: &[BaseResource]) -> bool {
    let blueprint = &organism.genome.structural_blueprint;
    if blueprint.validate().is_err() { return false; }
    let current_mass = organism.structural_mass(catalog);
    if current_mass >= organism.genome.adult_mass() - f64::EPSILON { return false; }

    let Some(index) = blueprint.elements.iter().enumerate().find(|(index, _)| !organism.structure.units.iter().any(|unit| unit.blueprint_index == Some(*index))).map(|(index, _)| index) else {
        return repair_missing_bonds(organism, catalog);
    };
    let element = &blueprint.elements[index];
    if current_mass + element.material.mass(catalog) > organism.genome.adult_mass() + f64::EPSILON { return false; }
    let required = element.material.material.parts.clone();
    if !has_parts(&organism.stored_unbonded, &required) { return false; }

    let original_structure = organism.structure.clone();
    let original_material = organism.stored_unbonded.clone();
    if take_parts(&mut organism.stored_unbonded, &required).is_none() { return false; }
    organism.structure.add_unit(StructuralUnit::from_blueprint_indexed(element.material.clone(), element.geometry.clone(), element.placement, index));
    if !repair_missing_bonds(organism, catalog) {
        organism.structure = original_structure;
        organism.stored_unbonded = original_material;
        return false;
    }
    true
}

fn repair_missing_bonds(organism: &mut Organism, catalog: &[BaseResource]) -> bool {
    let blueprint = &organism.genome.structural_blueprint;
    for connection in &blueprint.connections {
        let Some(unit_a) = organism.structure.units.iter().position(|unit| unit.blueprint_index == Some(connection.element_a)) else { continue; };
        let Some(unit_b) = organism.structure.units.iter().position(|unit| unit.blueprint_index == Some(connection.element_b)) else { continue; };
        let bond_exists = organism.structure.bonds.iter().any(|bond| bond.unit_a == unit_a && bond.point_a == connection.point_a && bond.unit_b == unit_b && bond.point_b == connection.point_b || bond.unit_a == unit_b && bond.point_a == connection.point_b && bond.unit_b == unit_a && bond.point_b == connection.point_a);
        if bond_exists { continue; }
        let bond = Bond { unit_a, point_a: connection.point_a, unit_b, point_b: connection.point_b, bond_energy: 0.0 };
        if !organism.structure.is_valid_bond(&bond, catalog) || crate::contact::try_add_bond(&mut organism.structure, bond, catalog).is_err() { return false; }
    }
    true
}

fn has_parts(material: &Material, required: &[(String, f64)]) -> bool {
    required.iter().all(|(name, amount)| material.parts.iter().find(|(candidate, _)| candidate == name).map(|(_, available)| *available).unwrap_or(0.0) + f64::EPSILON >= *amount)
}

fn take_parts(material: &mut Material, required: &[(String, f64)]) -> Option<Material> {
    if !has_parts(material, required) { return None; }
    let mut taken = Vec::new();
    for (name, amount) in required { let entry = material.parts.iter_mut().find(|(candidate, _)| candidate == name)?; entry.1 -= *amount; taken.push((name.clone(), *amount)); }
    material.parts.retain(|(_, amount)| *amount > 1e-12);
    Some(Material { parts: crate::resources::merge_parts(&taken), bonded: false })
}
