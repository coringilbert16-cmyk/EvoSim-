//! Physical reproduction lifecycle.
//!
//! Reproduction commits a small physical seed from the parent. The inherited
//! structural blueprint is a developmental target, not an upfront material
//! bill. Development proceeds only when the construction state actually has
//! the material needed for the next blueprint element.

use rand_chacha::ChaCha8Rng;

use crate::resources::{BaseResource, Material};
use crate::state::{DevelopmentStage, Organism, ReproductiveConstruction};
use crate::structure::{Bond, OrganismStructure, StructuralUnit};
use crate::structural_material::StructuralMaterial;

const EPSILON: f64 = 1e-12;

pub(crate) fn begin_reproduction(parent: &mut Organism, rng: &mut ChaCha8Rng) -> bool {
    if !matches!(parent.development_stage, DevelopmentStage::Adult) {
        return false;
    }
    if parent.reproductive_readiness < 1.0 - EPSILON {
        return false;
    }
    if parent.reproductive_construction.is_some() {
        return false;
    }

    let mut child_genome = parent.genome.clone();
    child_genome.mutate(rng);

    // Only the inherited seed element is committed at reproduction. The rest
    // of the blueprint remains a developmental target that must be supplied
    // later by physical material acquisition.
    let Some(seed_element) = child_genome.structural_blueprint.elements.first() else {
        return false;
    };
    let required_seed = &seed_element.material.material;
    if parent.stored_unbonded.bonded
        || parent.stored_unbonded.total_amount() + EPSILON < required_seed.total_amount()
    {
        return false;
    }
    if !has_required_material(&parent.stored_unbonded, required_seed) {
        return false;
    }

    let Some(committed_material) = take_required_material(&mut parent.stored_unbonded, required_seed) else {
        return false;
    };

    parent.reproductive_readiness = 0.0;
    parent.reproductive_construction = Some(ReproductiveConstruction {
        committed_material,
        developing_structure: OrganismStructure::new(),
        child_genome,
        next_blueprint_element: 0,
    });
    true
}

/// Advance construction by one inherited blueprint element when the required
/// physical material is actually available. Lack of material is a normal
/// developmental pause, not a failed reproduction attempt.
pub(crate) fn advance_construction(
    construction: &mut ReproductiveConstruction,
    catalog: &[BaseResource],
) -> bool {
    let blueprint = &construction.child_genome.structural_blueprint;
    let Some(element) = blueprint.elements.get(construction.next_blueprint_element) else {
        return false;
    };

    let Some(material) = take_required_material(
        &mut construction.committed_material,
        &element.material.material,
    ) else {
        return false;
    };

    let unit_material = StructuralMaterial {
        material,
        internal_bonds: element.material.internal_bonds.clone(),
    };
    let Some(unit) = StructuralUnit::from_material(unit_material, element.placement) else {
        return false;
    };

    construction.developing_structure.add_unit(unit);
    construction.next_blueprint_element += 1;
    realize_new_blueprint_connections(construction, catalog);
    true
}

/// Realize authored connections whose endpoint elements now exist.
/// The blueprint supplies inherited topology, but physical contact and bond
/// validation remain authoritative.
fn realize_new_blueprint_connections(
    construction: &mut ReproductiveConstruction,
    catalog: &[BaseResource],
) {
    let blueprint = &construction.child_genome.structural_blueprint;
    let element_count = construction.developing_structure.units.len();

    for connection in blueprint.connections.iter().copied() {
        if connection.element_a >= element_count || connection.element_b >= element_count {
            continue;
        }

        let Some(props_a) = construction.developing_structure.units[connection.element_a]
            .properties(catalog)
        else {
            continue;
        };
        let Some(props_b) = construction.developing_structure.units[connection.element_b]
            .properties(catalog)
        else {
            continue;
        };

        let candidate = crate::contact::connection_pair_candidates(
            &construction.developing_structure,
            connection.element_a,
            connection.element_b,
            catalog,
        )
        .into_iter()
        .find(|candidate| {
            candidate.point_a == connection.point_a
                && candidate.point_b == connection.point_b
                && candidate.distance <= 1.0
                && candidate.facing > 0.0
        });

        let Some(candidate) = candidate else {
            continue;
        };

        let bond = Bond {
            unit_a: connection.element_a,
            point_a: candidate.point_a,
            unit_b: connection.element_b,
            point_b: candidate.point_b,
            strength: crate::combine::bond_strength(props_a, props_b),
            bond_energy: 0.0,
        };

        if construction.developing_structure.is_valid_bond(&bond, catalog)
            && !construction
                .developing_structure
                .bonds
                .iter()
                .any(|existing| existing.has_same_identity(&bond))
        {
            construction.developing_structure.add_bond(bond);
        }
    }
}

fn has_required_material(available: &Material, required: &Material) -> bool {
    if available.bonded || required.bonded || required.is_empty() {
        return false;
    }

    crate::resources::merge_parts(&required.parts)
        .iter()
        .all(|(name, required_amount)| {
            let available_amount = available
                .parts
                .iter()
                .filter(|(available_name, amount)| available_name == name && *amount > 0.0)
                .map(|(_, amount)| *amount)
                .sum::<f64>();
            available_amount + EPSILON >= *required_amount
        })
}

fn take_required_material(committed: &mut Material, required: &Material) -> Option<Material> {
    if !has_required_material(committed, required) {
        return None;
    }

    let required_parts = crate::resources::merge_parts(&required.parts);
    let mut allocations: Vec<(usize, f64)> = Vec::new();

    for (name, required_amount) in &required_parts {
        let mut remaining = *required_amount;
        for (index, (available_name, available_amount)) in committed.parts.iter().enumerate() {
            if available_name != name || *available_amount <= 0.0 || remaining <= EPSILON {
                continue;
            }
            let taken = remaining.min(*available_amount);
            allocations.push((index, taken));
            remaining -= taken;
        }
        if remaining > EPSILON {
            return None;
        }
    }

    for (index, amount) in allocations {
        committed.parts[index].1 -= amount;
    }
    committed.parts.retain(|(_, amount)| *amount > EPSILON);

    Some(Material {
        parts: required_parts,
        bonded: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use crate::decision::DecisionHistory;
    use crate::genome::{initial_genome, Genome};
    use crate::state::{Position, ResourceSense};
    use crate::structural_blueprint::{BlueprintElement, BlueprintGeometry, StructuralBlueprint};
    use crate::structure::Placement;

    fn adult_parent(material_amount: f64) -> Organism {
        Organism {
            id: "parent".into(),
            occupied_cells: vec![Position { x: 50.0, y: 50.0 }],
            genome: initial_genome(),
            resource_sense: ResourceSense { sensed_resources: Vec::new(), direction_x: 0.0, direction_y: 0.0, direction_strength: 0.0 },
            memory: Vec::new(),
            decision_history: DecisionHistory::default(),
            usable_energy: 10.0,
            stress: 0.0,
            stored_unbonded: Material::free_base("Carbon", material_amount),
            structure: OrganismStructure::new(),
            development_stage: DevelopmentStage::Adult,
            age: 10,
            reproductive_readiness: 1.0,
            active_transformation_id: None,
            reproductive_construction: None,
        }
    }

    fn blueprint_element(resource_name: &str, x: f64, y: f64) -> BlueprintElement {
        let catalog = crate::resources::default_catalog();
        let shape = catalog.iter().find(|resource| resource.name == resource_name).unwrap().shape.clone();
        BlueprintElement {
            material: StructuralMaterial::single(resource_name),
            geometry: BlueprintGeometry::single(shape),
            placement: Placement { x, y, rotation_radians: 0.0 },
        }
    }

    #[test]
    fn reproduction_commits_only_the_seed_element() {
        let mut parent = adult_parent(1.0);
        parent.genome.structural_blueprint = StructuralBlueprint::new(
            vec![
                blueprint_element("Carbon", 0.0, 0.0),
                blueprint_element("Methane", 1.0, 0.0),
            ],
            Vec::new(),
        );
        let mut rng = ChaCha8Rng::seed_from_u64(7);
        assert!(begin_reproduction(&mut parent, &mut rng));
        assert_eq!(parent.stored_unbonded.total_amount(), 0.0);
        let construction = parent.reproductive_construction.as_ref().unwrap();
        assert_eq!(construction.committed_material.parts, vec![("Carbon".into(), 1.0)]);
        assert_eq!(construction.next_blueprint_element, 0);
    }

    #[test]
    fn construction_uses_blueprint_order_and_pauses_without_required_material() {
        let catalog = crate::resources::default_catalog();
        let genome = Genome {
            traits: initial_genome().traits,
            structural_blueprint: StructuralBlueprint::new(
                vec![
                    blueprint_element("Carbon", 3.0, 4.0),
                    blueprint_element("Methane", 8.0, 9.0),
                ],
                Vec::new(),
            ),
        };
        let mut construction = ReproductiveConstruction {
            committed_material: Material::free_base("Carbon", 1.0),
            developing_structure: OrganismStructure::new(),
            child_genome: genome,
            next_blueprint_element: 0,
        };
        assert!(advance_construction(&mut construction, &catalog));
        assert_eq!(construction.next_blueprint_element, 1);
        assert_eq!(construction.developing_structure.units.len(), 1);
        assert_eq!(construction.developing_structure.units[0].placement.x, 3.0);
        assert!(!advance_construction(&mut construction, &catalog));
        assert_eq!(construction.next_blueprint_element, 1);
    }

    #[test]
    fn construction_accepts_material_incrementally() {
        let catalog = crate::resources::default_catalog();
        let genome = Genome {
            traits: initial_genome().traits,
            structural_blueprint: StructuralBlueprint::new(
                vec![blueprint_element("Carbon", 0.0, 0.0), blueprint_element("Methane", 1.0, 0.0)],
                Vec::new(),
            ),
        };
        let mut construction = ReproductiveConstruction {
            committed_material: Material::free_base("Carbon", 1.0),
            developing_structure: OrganismStructure::new(),
            child_genome: genome,
            next_blueprint_element: 0,
        };
        assert!(advance_construction(&mut construction, &catalog));
        assert!(!advance_construction(&mut construction, &catalog));
        construction.committed_material = Material::free_base("Methane", 1.0);
        assert!(advance_construction(&mut construction, &catalog));
        assert_eq!(construction.next_blueprint_element, 2);
    }

    #[test]
    fn insufficient_material_is_atomic() {
        let mut committed = Material { parts: vec![("Carbon".into(), 0.5), ("Carbon".into(), 0.25)], bonded: false };
        let required = Material::free_base("Carbon", 1.0);
        assert!(take_required_material(&mut committed, &required).is_none());
        assert_eq!(committed.parts, vec![("Carbon".into(), 0.5), ("Carbon".into(), 0.25)]);
    }
}
