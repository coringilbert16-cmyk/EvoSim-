//! Physical reproduction lifecycle.
//!
//! Reproduction copies the parent's inherited structural blueprint. The child
//! does not invent a body plan during construction. Structural mutation is a
//! small inherited change and is limited to at most five percent of blueprint
//! elements.

use rand::Rng;
use rand_chacha::ChaCha8Rng;

use crate::resources::{BaseResource, Material};
use crate::state::{DevelopmentStage, Organism, ReproductiveConstruction};
use crate::structure::{Bond, OrganismStructure, StructuralUnit};
use crate::structural_blueprint::StructuralBlueprint;

const STRUCTURAL_MUTATION_FRACTION: f64 = 0.05;

pub(crate) fn instantiate_blueprint(blueprint: &StructuralBlueprint, catalog: &[BaseResource]) -> Result<OrganismStructure, String> {
    blueprint.validate()?;
    let mut structure = OrganismStructure::new();
    for (index, element) in blueprint.elements.iter().enumerate() {
        structure.add_unit(StructuralUnit::from_blueprint_indexed(element.material.clone(), element.geometry.clone(), element.placement, index));
    }
    for connection in &blueprint.connections {
        let _a = structure.units[connection.element_a].properties(catalog).ok_or_else(|| "blueprint element A has invalid material".to_string())?;
        let _b = structure.units[connection.element_b].properties(catalog).ok_or_else(|| "blueprint element B has invalid material".to_string())?;
        let bond = Bond { unit_a: connection.element_a, point_a: connection.point_a, unit_b: connection.element_b, point_b: connection.point_b, bond_energy: 0.0 };
        if !structure.is_valid_bond(&bond, catalog) { return Err("blueprint connection does not produce a valid structural bond".into()); }
        crate::contact::try_add_bond(&mut structure, bond, catalog).map_err(|_| "blueprint connection failed physical contact validation".to_string())?;
    }
    Ok(structure)
}

pub(crate) fn begin_reproduction(parent: &mut Organism, rng: &mut ChaCha8Rng) -> bool {
    if !matches!(parent.development_stage, DevelopmentStage::Adult) || parent.reproductive_readiness < 1.0 - f64::EPSILON || parent.reproductive_construction.is_some() { return false; }
    let mut child_genome = parent.genome.clone();
    if child_genome.structural_blueprint.validate().is_err() { return false; }
    child_genome.mutate(rng);
    mutate_structural_blueprint(&mut child_genome.structural_blueprint, rng);
    if child_genome.structural_blueprint.validate().is_err() { return false; }
    let required = child_genome.structural_blueprint.total_material_amount();
    if parent.stored_unbonded.total_amount() + f64::EPSILON < required || !has_required_material(&parent.stored_unbonded, &child_genome.structural_blueprint) { return false; }
    let Some(committed_material) = take_required_material(&mut parent.stored_unbonded, &child_genome.structural_blueprint) else { return false; };
    parent.reproductive_readiness = 0.0;
    parent.reproductive_construction = Some(ReproductiveConstruction { committed_material, developing_structure: OrganismStructure::new(), child_genome });
    true
}

pub(crate) fn advance_construction(construction: &mut ReproductiveConstruction, catalog: &[BaseResource]) -> bool {
    let blueprint = &construction.child_genome.structural_blueprint;
    if blueprint.validate().is_err() { return false; }
    let next = construction.developing_structure.units.len();
    if next >= blueprint.elements.len() { return false; }
    let element = &blueprint.elements[next];
    let required_parts = element.material.material.parts.clone();
    if !has_parts(&construction.committed_material, &required_parts) { return false; }
    let Some(material) = take_parts(&mut construction.committed_material, &required_parts) else { return false; };
    if material.total_amount() + f64::EPSILON < element.material.total_amount() { return false; }
    construction.developing_structure.add_unit(StructuralUnit::from_blueprint_indexed(element.material.clone(), element.geometry.clone(), element.placement, next));

    let connections: Vec<_> = blueprint.connections.iter()
        .filter(|connection| {
            connection.element_a < construction.developing_structure.units.len()
                && connection.element_b < construction.developing_structure.units.len()
                && !construction.developing_structure.bonds.iter().any(|bond| {
                    bond.has_same_identity(&Bond {
                        unit_a: connection.element_a,
                        point_a: connection.point_a,
                        unit_b: connection.element_b,
                        point_b: connection.point_b,
                        bond_energy: 0.0,
                    })
                })
        })
        .cloned()
        .collect();
    for connection in connections {
        let bond = Bond { unit_a: connection.element_a, point_a: connection.point_a, unit_b: connection.element_b, point_b: connection.point_b, bond_energy: 0.0 };
        if !construction.developing_structure.is_valid_bond(&bond, catalog) || crate::contact::try_add_bond(&mut construction.developing_structure, bond, catalog).is_err() { return false; }
    }
    true
}

fn has_required_material(material: &Material, blueprint: &StructuralBlueprint) -> bool {
    let required: Vec<(String, f64)> = blueprint.elements.iter().flat_map(|element| element.material.material.parts.iter().cloned()).collect();
    has_parts(material, &required)
}
fn has_parts(material: &Material, required: &[(String, f64)]) -> bool {
    required.iter().all(|(name, amount)| material.parts.iter().find(|(candidate, _)| candidate == name).map(|(_, value)| *value).unwrap_or(0.0) + f64::EPSILON >= *amount)
}
fn take_required_material(material: &mut Material, blueprint: &StructuralBlueprint) -> Option<Material> {
    let required: Vec<(String, f64)> = blueprint.elements.iter().flat_map(|element| element.material.material.parts.iter().cloned()).collect();
    take_parts(material, &required)
}
fn take_parts(material: &mut Material, required: &[(String, f64)]) -> Option<Material> {
    if !has_parts(material, required) { return None; }
    let mut taken = Vec::new();
    for (name, amount) in required { let entry = material.parts.iter_mut().find(|(candidate, _)| candidate == name)?; entry.1 -= *amount; taken.push((name.clone(), *amount)); }
    material.parts.retain(|(_, amount)| *amount > 1e-12);
    Some(Material { parts: crate::resources::merge_parts(&taken), bonded: false })
}

fn mutate_structural_blueprint(blueprint: &mut StructuralBlueprint, rng: &mut ChaCha8Rng) {
    if blueprint.elements.is_empty() || rng.gen::<f64>() >= STRUCTURAL_MUTATION_FRACTION { return; }
    let index = (rng.gen::<f64>() * blueprint.elements.len() as f64) as usize % blueprint.elements.len();
    let original = blueprint.elements[index].placement;
    blueprint.elements[index].placement.x += rng.gen_range(-0.05..0.05);
    blueprint.elements[index].placement.y += rng.gen_range(-0.05..0.05);
    if blueprint.validate().is_err() { blueprint.elements[index].placement = original; }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use crate::decision::DecisionHistory;
    use crate::genome::initial_genome;
    use crate::resources::default_catalog;
    use crate::state::{Position, ResourceSense};

    fn adult_parent(material_amount: f64) -> Organism {
        Organism { id: "parent".into(), occupied_cells: vec![Position { x: 50.0, y: 50.0 }], genome: initial_genome(), resource_sense: ResourceSense { sensed_resources: Vec::new(), direction_x: 0.0, direction_y: 0.0, direction_strength: 0.0 }, memory: Vec::new(), decision_history: DecisionHistory::default(), usable_energy: 10.0, stress: 0.0, stored_unbonded: Material::free_base("Carbon", material_amount), structure: OrganismStructure::new(), development_stage: DevelopmentStage::Adult, age: 10, reproductive_readiness: 1.0, active_transformation_id: None, reproductive_construction: None }
    }
    #[test] fn reproduction_requires_an_inherited_blueprint() { let mut parent = adult_parent(100.0); let mut rng = ChaCha8Rng::seed_from_u64(7); assert!(!begin_reproduction(&mut parent, &mut rng)); assert!(parent.reproductive_construction.is_none()); }
    #[test] fn initial_blueprint_instantiates_as_a_physical_structure() { let genome = crate::genome::initial_genome(); let structure = instantiate_blueprint(&genome.structural_blueprint, &default_catalog()).unwrap(); assert_eq!(structure.units.len(), genome.structural_blueprint.elements.len()); assert_eq!(structure.bonds.len(), genome.structural_blueprint.connections.len()); }
    #[test] fn material_helpers_preserve_exact_blueprint_composition() { let mut material = Material::free_base("Carbon", 3.0); let required = vec![("Carbon".into(), 2.0)]; let taken = take_parts(&mut material, &required).unwrap(); assert_eq!(taken.parts, required); assert_eq!(material.total_amount(), 1.0); }
    #[test] fn construction_uses_blueprint_elements_not_core_constants() { let catalog = default_catalog(); let mut parent = adult_parent(100.0); parent.genome.structural_blueprint = StructuralBlueprint::new(vec![crate::structural_blueprint::BlueprintElement { material: crate::structural_material::StructuralMaterial::single("Carbon"), geometry: crate::structural_blueprint::BlueprintGeometry::single(catalog[0].shape.clone()), placement: crate::structure::Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 } }], Vec::new()); let mut rng = ChaCha8Rng::seed_from_u64(7); assert!(begin_reproduction(&mut parent, &mut rng)); let construction = parent.reproductive_construction.as_mut().unwrap(); assert!(advance_construction(construction, &catalog)); assert_eq!(construction.developing_structure.units.len(), 1); assert_eq!(construction.committed_material.total_amount(), 0.0); }
}
