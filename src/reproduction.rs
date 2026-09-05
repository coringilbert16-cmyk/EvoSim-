//! Physical reproduction lifecycle.
//!
//! Reproduction commits only the inherited blueprint's first physical seed.
//! That seed is placed into actual contact with the parent. The resulting
//! cross-organism connection is the physical boundary through which later
//! material transfer can occur. The complete blueprint is not an upfront
//! resource bill.

use rand_chacha::ChaCha8Rng;

use crate::cell_connection::{CellConnection, CellSiteRef};
use crate::resources::{BaseResource, ConnectionSites, Material};
use crate::state::{DevelopmentStage, Organism, Position, ReproductiveConstruction};
use crate::structure::{Bond, OrganismStructure, StructuralUnit};
use crate::structural_material::StructuralMaterial;

const EPSILON: f64 = 1e-12;
const CONNECTION_TOLERANCE: f64 = 0.25;
const MIN_CONNECTION_FACING: f64 = 0.0;

pub(crate) fn begin_reproduction(
    parent: &mut Organism,
    child_id: String,
    rng: &mut ChaCha8Rng,
    catalog: &[BaseResource],
) -> bool {
    if !matches!(parent.development_stage, DevelopmentStage::Adult)
        || parent.reproductive_readiness < 1.0 - EPSILON
        || parent.reproductive_construction.is_some()
        || parent.structure.units.is_empty()
    {
        return false;
    }

    let mut child_genome = parent.genome.clone();
    child_genome.mutate(rng);

    let Some(seed_element) = child_genome.structural_blueprint.elements.first() else {
        return false;
    };
    let required_seed = &seed_element.material.material;
    if parent.stored_unbonded.bonded
        || !has_required_material(&parent.stored_unbonded, required_seed)
    {
        return false;
    }

    let Some(seed_unit) = StructuralUnit::from_material(
        StructuralMaterial {
            material: required_seed.clone(),
            internal_bonds: seed_element.material.internal_bonds.clone(),
        },
        seed_element.placement,
    ) else {
        return false;
    };
    let mut seed_structure = OrganismStructure::new();
    seed_structure.add_unit(seed_unit);

    let Some((developing_structure, connection, world_offset)) =
        attach_seed_to_parent(parent, &seed_structure, &child_id, catalog)
    else {
        return false;
    };

    let Some(_seed_material) = take_required_material(&mut parent.stored_unbonded, required_seed)
    else {
        return false;
    };

    parent.reproductive_readiness = 0.0;
    parent.reproductive_construction = Some(ReproductiveConstruction {
        child_id,
        connection,
        committed_material: Material {
            parts: Vec::new(),
            bonded: false,
        },
        developing_structure,
        child_genome,
        world_offset,
        next_blueprint_element: 1,
    });
    true
}

fn attach_seed_to_parent(
    parent: &Organism,
    seed_structure: &OrganismStructure,
    child_id: &str,
    catalog: &[BaseResource],
) -> Option<(OrganismStructure, CellConnection, Position)> {
    let child_unit = seed_structure.units.first()?;
    let ConnectionSites::Corners(child_points) = child_unit.connection_sites(catalog)? else {
        return None;
    };

    for (parent_unit_index, parent_unit) in parent.structure.units.iter().enumerate() {
        let Some(ConnectionSites::Corners(parent_points)) = parent_unit.connection_sites(catalog)
        else {
            continue;
        };

        for (parent_point_index, parent_point) in parent_points.iter().enumerate() {
            let parent_world = crate::contact::world_connection_point(*parent_point, parent_unit);

            for (child_point_index, child_point) in child_points.iter().enumerate() {
                let child_world = crate::contact::world_connection_point(*child_point, child_unit);
                let offset = Position {
                    x: parent_world.x - child_world.x,
                    y: parent_world.y - child_world.y,
                };

                let mut candidate_structure = seed_structure.clone();
                for unit in &mut candidate_structure.units {
                    unit.placement.x += offset.x;
                    unit.placement.y += offset.y;
                }

                let parent_site = CellSiteRef {
                    organism_id: parent.id.clone(),
                    unit_index: parent_unit_index,
                    point_index: parent_point_index,
                };
                let child_site = CellSiteRef {
                    organism_id: child_id.to_string(),
                    unit_index: 0,
                    point_index: child_point_index,
                };

                if let Some(connection) = CellConnection::try_establish(
                    parent_site,
                    child_site,
                    &parent.structure,
                    &candidate_structure,
                    catalog,
                    CONNECTION_TOLERANCE,
                    MIN_CONNECTION_FACING,
                ) {
                    return Some((candidate_structure, connection, offset));
                }
            }
        }
    }

    None
}

fn has_required_material(available: &Material, required: &Material) -> bool {
    if available.bonded || required.is_empty() {
        return false;
    }
    for (resource, amount) in &required.parts {
        let available_amount = available
            .parts
            .iter()
            .filter(|(name, _)| name == resource)
            .map(|(_, value)| *value)
            .sum::<f64>();
        if available_amount + EPSILON < *amount {
            return false;
        }
    }
    true
}

fn take_required_material(available: &mut Material, required: &Material) -> Option<Material> {
    if available.bonded || required.is_empty() || !has_required_material(available, required) {
        return None;
    }

    let mut taken_parts = Vec::new();
    for (resource, required_amount) in &required.parts {
        let mut remaining = *required_amount;
        for (name, amount) in &mut available.parts {
            if name == resource && remaining > EPSILON {
                let taken = amount.min(remaining);
                *amount -= taken;
                remaining -= taken;
                if taken > EPSILON {
                    taken_parts.push((name.clone(), taken));
                }
            }
        }
    }
    available.parts.retain(|(_, amount)| *amount > EPSILON);
    Some(Material {
        parts: taken_parts,
        bonded: false,
    })
}

fn advance_construction(
    construction: &mut ReproductiveConstruction,
    catalog: &[BaseResource],
) -> bool {
    let Some(element) = construction
        .child_genome
        .structural_blueprint
        .elements
        .get(construction.next_blueprint_element)
    else {
        return true;
    };

    if !has_required_material(&construction.committed_material, &element.material.material) {
        return false;
    }

    let Some(unit) = StructuralUnit::from_material(
        StructuralMaterial {
            material: element.material.material.clone(),
            internal_bonds: element.material.internal_bonds.clone(),
        },
        Placement {
            x: construction.world_offset.x + element.placement.x,
            y: construction.world_offset.y + element.placement.y,
            rotation_radians: element.placement.rotation_radians,
        },
    ) else {
        return false;
    };

    let Some(_material) = take_required_material(
        &mut construction.committed_material,
        &element.material.material,
    ) else {
        return false;
    };

    construction.developing_structure.add_unit(unit);
    construction.next_blueprint_element += 1;
    let _ = catalog;
    construction.next_blueprint_element
        >= construction.child_genome.structural_blueprint.elements.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::DecisionHistory;
    use crate::genome::{initial_genome, Genome};
    use crate::state::ResourceSense;
    use crate::structural_blueprint::{BlueprintElement, BlueprintGeometry, StructuralBlueprint};
    use crate::structure::Placement;
    use rand::SeedableRng;

    fn adult_parent(material_amount: f64) -> Organism {
        let mut structure = OrganismStructure::new();
        structure.add_unit(StructuralUnit::new(
            "Carbon",
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));
        Organism {
            id: "parent".into(),
            occupied_cells: vec![Position { x: 0.0, y: 0.0 }],
            genome: initial_genome(),
            resource_sense: ResourceSense {
                sensed_resources: Vec::new(),
                direction_x: 0.0,
                direction_y: 0.0,
                direction_strength: 0.0,
            },
            memory: Vec::new(),
            decision_history: DecisionHistory::default(),
            usable_energy: 10.0,
            stress: 0.0,
            stored_unbonded: Material::free_base("Carbon", material_amount),
            structure,
            development_stage: DevelopmentStage::Adult,
            age: 10,
            reproductive_readiness: 1.0,
            active_transformation_id: None,
            reproductive_construction: None,
        }
    }

    fn blueprint_element(resource_name: &str, x: f64, y: f64) -> BlueprintElement {
        let catalog = crate::resources::default_catalog();
        let shape = catalog
            .iter()
            .find(|resource| resource.name == resource_name)
            .unwrap()
            .shape
            .clone();
        BlueprintElement {
            material: StructuralMaterial::single(resource_name),
            geometry: BlueprintGeometry::single(shape),
            placement: Placement {
                x,
                y,
                rotation_radians: 0.0,
            },
        }
    }

    fn test_connection() -> CellConnection {
        let catalog = crate::resources::default_catalog();
        let parent = adult_parent(0.0);
        let mut child = OrganismStructure::new();
        child.add_unit(StructuralUnit::new(
            "Carbon",
            Placement {
                x: 1.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));
        CellConnection::try_establish(
            CellSiteRef {
                organism_id: "parent".into(),
                unit_index: 0,
                point_index: 0,
            },
            CellSiteRef {
                organism_id: "child".into(),
                unit_index: 0,
                point_index: 3,
            },
            &parent.structure,
            &child,
            &catalog,
            CONNECTION_TOLERANCE,
            MIN_CONNECTION_FACING,
        )
        .expect("test endpoints should be in physical contact")
    }

    #[test]
    fn reproduction_commits_seed_and_establishes_physical_connection() {
        let mut parent = adult_parent(1.0);
        parent.genome.structural_blueprint = StructuralBlueprint::new(
            vec![
                blueprint_element("Carbon", 0.0, 0.0),
                blueprint_element("Methane", 1.0, 0.0),
            ],
            Vec::new(),
        );
        let mut rng = ChaCha8Rng::seed_from_u64(7);
        let catalog = crate::resources::default_catalog();

        assert!(begin_reproduction(
            &mut parent,
            "child".into(),
            &mut rng,
            &catalog,
        ));
        assert_eq!(parent.stored_unbonded.total_amount(), 0.0);

        let construction = parent.reproductive_construction.as_ref().unwrap();
        assert_eq!(construction.child_id, "child");
        assert!(construction.committed_material.is_empty());
        assert_eq!(construction.developing_structure.units.len(), 1);
        assert_eq!(construction.next_blueprint_element, 1);
        assert!(construction.connection.crosses_organism_boundary());
    }

    #[test]
    fn construction_waits_for_material_received_through_connection() {
        let catalog = crate::resources::default_catalog();
        let genome = Genome {
            traits: initial_genome().traits,
            structural_blueprint: StructuralBlueprint::new(
                vec![
                    blueprint_element("Carbon", 0.0, 0.0),
                    blueprint_element("Methane", 1.0, 0.0),
                ],
                Vec::new(),
            ),
        };
        let connection = test_connection();
        let mut construction = ReproductiveConstruction {
            child_id: "child".into(),
            connection: connection.clone(),
            committed_material: Material {
                parts: Vec::new(),
                bonded: false,
            },
            developing_structure: {
                let mut structure = OrganismStructure::new();
                structure.add_unit(StructuralUnit::new(
                    "Carbon",
                    Placement {
                        x: 1.0,
                        y: 0.0,
                        rotation_radians: 0.0,
                    },
                ));
                structure
            },
            child_genome: genome,
            world_offset: Position { x: 1.0, y: 0.0 },
            next_blueprint_element: 1,
        };

        assert!(!advance_construction(&mut construction, &catalog));

        let mut parent = adult_parent(1.0);
        let mut child = adult_parent(0.0);
        child.id = "child".into();
        child.structure.units[0].placement.x = 1.0;

        crate::material_transfer::transfer_unbonded_material(
            &connection,
            CellSiteRef {
                organism_id: "parent".into(),
                unit_index: 0,
                point_index: 0,
            },
            CellSiteRef {
                organism_id: "child".into(),
                unit_index: 0,
                point_index: 3,
            },
            &mut parent,
            &mut child,
            1.0,
        )
        .expect("connected parent-to-offspring transfer should succeed");

        construction.committed_material = child.stored_unbonded;

        assert!(advance_construction(&mut construction, &catalog));
        assert_eq!(construction.next_blueprint_element, 2);
        assert_eq!(construction.developing_structure.units.len(), 2);
        assert!(construction.committed_material.is_empty());
    }

    #[test]
    fn construction_uses_blueprint_order_and_world_offset() {
        let catalog = crate::resources::default_catalog();
        let genome = Genome {
            traits: initial_genome().traits,
            structural_blueprint: StructuralBlueprint::new(
                vec![
                    blueprint_element("Carbon", 0.0, 0.0),
                    blueprint_element("Methane", 8.0, 9.0),
                ],
                Vec::new(),
            ),
        };
        let world_offset = Position { x: 1.0, y: 2.0 };
        let mut construction = ReproductiveConstruction {
            child_id: "child".into(),
            connection: test_connection(),
            committed_material: Material::free_base("Methane", 1.0),
            developing_structure: {
                let mut structure = OrganismStructure::new();
                structure.add_unit(StructuralUnit::new(
                    "Carbon",
                    Placement {
                        x: 1.0,
                        y: 2.0,
                        rotation_radians: 0.0,
                    },
                ));
                structure
            },
            child_genome: genome,
            world_offset,
            next_blueprint_element: 1,
        };

        assert!(advance_construction(&mut construction, &catalog));
        let placement = construction.developing_structure.units[1].placement;
        assert!((placement.x - 9.0).abs() < 1e-9);
        assert!((placement.y - 11.0).abs() < 1e-9);
    }

    #[test]
    fn insufficient_material_does_not_advance_construction() {
        let catalog = crate::resources::default_catalog();
        let genome = Genome {
            traits: initial_genome().traits,
            structural_blueprint: StructuralBlueprint::new(
                vec![
                    blueprint_element("Carbon", 0.0, 0.0),
                    blueprint_element("Methane", 1.0, 0.0),
                ],
                Vec::new(),
            ),
        };
        let mut construction = ReproductiveConstruction {
            child_id: "child".into(),
            connection: test_connection(),
            committed_material: Material::free_base("Methane", 0.5),
            developing_structure: {
                let mut structure = OrganismStructure::new();
                structure.add_unit(StructuralUnit::new(
                    "Carbon",
                    Placement {
                        x: 1.0,
                        y: 0.0,
                        rotation_radians: 0.0,
                    },
                ));
                structure
            },
            child_genome: genome,
            world_offset: Position { x: 1.0, y: 0.0 },
            next_blueprint_element: 1,
        };

        assert!(!advance_construction(&mut construction, &catalog));
        assert_eq!(construction.next_blueprint_element, 1);
        assert_eq!(construction.developing_structure.units.len(), 1);
        assert_eq!(construction.committed_material.total_amount(), 0.5);
    }
}
