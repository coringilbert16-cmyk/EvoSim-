//! Physical reproduction lifecycle.
//!
//! Reproduction does not split or copy the parent's existing structure. Once
//! an adult has accumulated enough actual unbonded material to realize the
//! child's inherited blueprint, the reproductive decision commits that exact
//! material to a persistent construction state. The parent keeps its own
//! structure intact. Later ticks consume that committed material to realize
//! the child's inherited structural blueprint.

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
    if parent.reproductive_readiness < 1.0 - f64::EPSILON {
        return false;
    }
    if parent.reproductive_construction.is_some() {
        return false;
    }

    let mut child_genome = parent.genome.clone();
    child_genome.mutate(rng);
    let required_material = child_genome.structural_blueprint.required_material();

    if !has_required_material(&parent.stored_unbonded, &required_material) {
        return false;
    }

    let Some(committed_material) = take_required_material(
        &mut parent.stored_unbonded,
        &required_material,
    ) else {
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

/// Advance reproductive construction by exactly one inherited blueprint
/// element. The blueprint determines both which element is constructed next
/// and its authored placement. Construction consumes only actual committed
/// material that matches the element's material requirements.
///
/// Once the target elements exist, authored blueprint connections are realized
/// through the same physical contact and bond-validation mechanics used
/// elsewhere.
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
///
/// The blueprint supplies inherited topology, but it does not bypass physics.
/// The requested endpoint pair must be physically contacting and facing, and
/// the resulting bond must pass the same intrinsic validation used elsewhere.
/// Connection points are not consumed by successful bonds.
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

/// Remove exactly the constituent amounts required by the supplied blueprint
/// material. Requirements are aggregated by resource type before this point,
/// and are re-aggregated here for individual blueprint elements so duplicate
/// constituent entries cannot cause partial consumption. The complete removal
/// plan is calculated before any stock is mutated.
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
    use crate::genome::initial_genome;
    use crate::state::{Position, ResourceSense};
    use crate::structural_blueprint::{BlueprintConnection, BlueprintElement, BlueprintGeometry, StructuralBlueprint};
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
    fn reproduction_commits_exact_blueprint_requirement() {
        let mut parent = adult_parent(2.0);
        let expected = parent.genome.structural_blueprint.required_material();
        assert_eq!(expected.parts, vec![("Carbon".into(), 1.0)]);
        let mut rng = ChaCha8Rng::seed_from_u64(7);
        assert!(begin_reproduction(&mut parent, &mut rng));
        assert!(parent.structure.units.is_empty());
        assert!(parent.structure.bonds.is_empty());
        assert_eq!(parent.stored_unbonded.total_amount(), 1.0);
        assert_eq!(parent.reproductive_readiness, 0.0);
        let construction = parent.reproductive_construction.as_ref().unwrap();
        assert_eq!(construction.committed_material.total_amount(), 1.0);
        assert_eq!(construction.committed_material.parts, expected.parts);
        assert!(!construction.committed_material.bonded);
        assert!(construction.developing_structure.units.is_empty());
        assert_eq!(construction.next_blueprint_element, 0);
    }

    #[test]
    fn reproduction_requires_all_blueprint_constituents_not_just_total_amount() {
        let mut parent = adult_parent(2.0);
        parent.genome.structural_blueprint = StructuralBlueprint::new(
            vec![
                blueprint_element("Carbon", 0.0, 0.0),
                blueprint_element("Methane", 1.0, 0.0),
            ],
            Vec::new(),
        );
        let mut rng = ChaCha8Rng::seed_from_u64(7);
        assert!(!begin_reproduction(&mut parent, &mut rng));
        assert_eq!(parent.stored_unbonded.total_amount(), 2.0);
        assert!(parent.reproductive_construction.is_none());
        assert_eq!(parent.reproductive_readiness, 1.0);
    }

    #[test]
    fn reproduction_commits_composite_blueprint_requirement_exactly() {
        let catalog = crate::resources::default_catalog();
        let composite = BlueprintElement {
            material: StructuralMaterial {
                material: Material {
                    parts: vec![("Carbon".into(), 2.0), ("Methane".into(), 1.0)],
                    bonded: true,
                },
                internal_bonds: Vec::new(),
            },
            geometry: BlueprintGeometry {
                constituents: vec![
                    crate::structural_blueprint::ConstituentGeometry {
                        part_index: 0,
                        shape: catalog[0].shape.clone(),
                        placement: Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 },
                    },
                    crate::structural_blueprint::ConstituentGeometry {
                        part_index: 1,
                        shape: catalog[1].shape.clone(),
                        placement: Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 },
                    },
                ],
                connection_regions: Vec::new(),
            },
            placement: Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 },
        };
        let mut parent = adult_parent(5.0);
        parent.genome.structural_blueprint = StructuralBlueprint::new(vec![composite], Vec::new());
        let required = parent.genome.structural_blueprint.required_material();
        assert_eq!(required.parts, vec![("Carbon".into(), 2.0), ("Methane".into(), 1.0)]);
        parent.stored_unbonded = Material { parts: vec![("Methane".into(), 1.0), ("Carbon".into(), 3.0)], bonded: false };
        let mut rng = ChaCha8Rng::seed_from_u64(7);
        assert!(begin_reproduction(&mut parent, &mut rng));
        assert_eq!(parent.stored_unbonded.parts, vec![("Carbon".into(), 1.0)]);
        let construction = parent.reproductive_construction.as_ref().unwrap();
        assert_eq!(construction.committed_material.parts, required.parts);
    }

    #[test]
    fn insufficient_required_material_consumes_nothing() {
        let mut committed = Material { parts: vec![("Carbon".into(), 0.5), ("Carbon".into(), 0.25)], bonded: false };
        let required = Material::free_base("Carbon", 1.0);
        assert!(take_required_material(&mut committed, &required).is_none());
        assert_eq!(committed.parts, vec![("Carbon".into(), 0.5), ("Carbon".into(), 0.25)]);
    }

    #[test]
    fn duplicate_required_parts_are_aggregated_before_consumption() {
        let mut committed = Material { parts: vec![("Carbon".into(), 1.5), ("Carbon".into(), 0.5)], bonded: false };
        let required = Material { parts: vec![("Carbon".into(), 1.0), ("Carbon".into(), 1.0)], bonded: false };
        let taken = take_required_material(&mut committed, &required).unwrap();
        assert_eq!(taken.parts, vec![("Carbon".into(), 2.0)]);
        assert!(committed.is_empty());
    }

    #[test]
    fn construction_starts_at_blueprint_element_zero_and_advances_one_element() {
        let catalog = crate::resources::default_catalog();
        let mut genome = initial_genome();
        genome.structural_blueprint = StructuralBlueprint::new(vec![blueprint_element("Carbon", 3.0, 4.0), blueprint_element("Methane", 8.0, 9.0)], Vec::new());
        let mut construction = ReproductiveConstruction { committed_material: Material { parts: vec![("Carbon".into(), 1.0), ("Methane".into(), 1.0)], bonded: false }, developing_structure: OrganismStructure::new(), child_genome: genome, next_blueprint_element: 0 };
        assert!(advance_construction(&mut construction, &catalog));
        assert_eq!(construction.next_blueprint_element, 1);
        assert_eq!(construction.developing_structure.units.len(), 1);
        assert_eq!(construction.developing_structure.units[0].material.primary_resource_name(), Some("Carbon"));
        assert_eq!(construction.developing_structure.units[0].placement.x, 3.0);
        assert_eq!(construction.developing_structure.units[0].placement.y, 4.0);
        assert!(advance_construction(&mut construction, &catalog));
        assert_eq!(construction.next_blueprint_element, 2);
        assert_eq!(construction.developing_structure.units.len(), 2);
        assert_eq!(construction.developing_structure.units[1].material.primary_resource_name(), Some("Methane"));
        assert_eq!(construction.developing_structure.units[1].placement.x, 8.0);
        assert_eq!(construction.developing_structure.units[1].placement.y, 9.0);
    }

    #[test]
    fn construction_uses_blueprint_order_not_available_material_order() {
        let catalog = crate::resources::default_catalog();
        let mut genome = initial_genome();
        genome.structural_blueprint = StructuralBlueprint::new(vec![blueprint_element("Methane", 1.0, 2.0), blueprint_element("Carbon", 3.0, 4.0)], Vec::new());
        let mut construction = ReproductiveConstruction { committed_material: Material { parts: vec![("Carbon".into(), 1.0), ("Methane".into(), 1.0)], bonded: false }, developing_structure: OrganismStructure::new(), child_genome: genome, next_blueprint_element: 0 };
        assert!(advance_construction(&mut construction, &catalog));
        assert_eq!(construction.developing_structure.units[0].material.primary_resource_name(), Some("Methane"));
        assert_eq!(construction.committed_material.parts, vec![("Carbon".into(), 1.0)]);
    }

    #[test]
    fn construction_does_not_consume_material_when_required_blueprint_material_is_unavailable() {
        let catalog = crate::resources::default_catalog();
        let mut genome = initial_genome();
        genome.structural_blueprint = StructuralBlueprint::new(vec![blueprint_element("Methane", 1.0, 2.0)], Vec::new());
        let mut construction = ReproductiveConstruction { committed_material: Material::free_base("Carbon", 1.0), developing_structure: OrganismStructure::new(), child_genome: genome, next_blueprint_element: 0 };
        assert!(!advance_construction(&mut construction, &catalog));
        assert_eq!(construction.developing_structure.units.len(), 0);
        assert_eq!(construction.next_blueprint_element, 0);
        assert_eq!(construction.committed_material.parts, vec![("Carbon".into(), 1.0)]);
    }

    #[test]
    fn construction_stops_after_blueprint_is_realized() {
        let catalog = crate::resources::default_catalog();
        let mut genome = initial_genome();
        genome.structural_blueprint = StructuralBlueprint::new(vec![blueprint_element("Carbon", 0.0, 0.0)], Vec::new());
        let mut construction = ReproductiveConstruction { committed_material: Material::free_base("Carbon", 2.0), developing_structure: OrganismStructure::new(), child_genome: genome, next_blueprint_element: 0 };
        assert!(advance_construction(&mut construction, &catalog));
        assert!(!advance_construction(&mut construction, &catalog));
        assert_eq!(construction.next_blueprint_element, 1);
        assert_eq!(construction.developing_structure.units.len(), 1);
        assert_eq!(construction.committed_material.total_amount(), 1.0);
    }

    #[test]
    fn blueprint_element_material_is_instantiated_as_physical_structural_material() {
        let catalog = crate::resources::default_catalog();
        let mut genome = initial_genome();
        genome.structural_blueprint = StructuralBlueprint::new(vec![blueprint_element("Carbon", 5.0, 6.0)], Vec::new());
        let mut construction = ReproductiveConstruction { committed_material: Material::free_base("Carbon", 1.0), developing_structure: OrganismStructure::new(), child_genome: genome, next_blueprint_element: 0 };
        assert!(advance_construction(&mut construction, &catalog));
        let unit = &construction.developing_structure.units[0];
        assert!(unit.material.is_valid());
        assert_eq!(unit.material.total_amount(), 1.0);
        assert_eq!(unit.material.primary_resource_name(), Some("Carbon"));
    }

    #[test]
    fn authored_blueprint_connection_is_realized_only_when_physical_contact_exists() {
        let catalog = crate::resources::default_catalog();
        let mut genome = initial_genome();
        genome.structural_blueprint = StructuralBlueprint::new(
            vec![blueprint_element("Carbon", 0.0, 0.0), blueprint_element("Carbon", 0.877382, 0.0)],
            vec![BlueprintConnection { element_a: 0, point_a: 0, element_b: 1, point_b: 3 }],
        );
        let mut construction = ReproductiveConstruction {
            committed_material: Material::free_base("Carbon", 2.0),
            developing_structure: OrganismStructure::new(),
            child_genome: genome,
            next_blueprint_element: 0,
        };
        assert!(advance_construction(&mut construction, &catalog));
        assert!(construction.developing_structure.bonds.is_empty());
        assert!(advance_construction(&mut construction, &catalog));
        assert_eq!(construction.developing_structure.bonds.len(), 1);
    }
}
