//! Physical reproduction lifecycle.
//!
//! Reproduction commits a small physical seed from the parent. The inherited
//! structural blueprint is a developmental target, not an upfront material
//! bill. The seed immediately becomes a physical offspring attachment; later
//! construction can proceed only when additional material is supplied through
//! that physical relationship.

use rand_chacha::ChaCha8Rng;

use crate::cell_connection::{CellConnection, CellSiteRef};
use crate::resources::{BaseResource, ConnectionSites, Material};
use crate::state::{DevelopmentStage, Organism, Position, ReproductiveConstruction};
use crate::structure::{Bond, OrganismStructure, StructuralUnit};
use crate::structural_material::StructuralMaterial;

const EPSILON: f64 = 1e-12;
const CONNECTION_TOLERANCE: f64 = 0.25;
const MIN_CONNECTION_FACING: f64 = 0.0;

/// Begin reproduction by committing only the inherited seed element and
/// attaching that seed physically to the parent. The attachment is a real
/// cross-organism cell connection; it does not itself transfer material.
pub(crate) fn begin_reproduction(
    parent: &mut Organism,
    child_id: String,
    rng: &mut ChaCha8Rng,
    catalog: &[BaseResource],
) -> bool {
    if !matches!(parent.development_stage, DevelopmentStage::Adult) {
        return false;
    }
    if parent.reproductive_readiness < 1.0 - EPSILON {
        return false;
    }
    if parent.reproductive_construction.is_some() || parent.structure.units.is_empty() {
        return false;
    }

    let mut child_genome = parent.genome.clone();
    child_genome.mutate(rng);

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

    // Validate the physical attachment before mutating the parent's store.
    let seed_unit = StructuralUnit::from_material(
        StructuralMaterial {
            material: required_seed.clone(),
            internal_bonds: seed_element.material.internal_bonds.clone(),
        },
        seed_element.placement,
    )?;
    let mut seed_structure = OrganismStructure::new();
    seed_structure.add_unit(seed_unit);
    let Some((developing_structure, connection, world_offset)) = attach_seed_to_parent(
        parent,
        &seed_structure,
        &child_id,
        catalog,
    ) else {
        return false;
    };

    let Some(committed_material) = take_required_material(&mut parent.stored_unbonded, required_seed) else {
        return false;
    };
    debug_assert_eq!(committed_material.total_amount(), required_seed.total_amount());

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
        let ConnectionSites::Corners(parent_points) = parent_unit.connection_sites(catalog) else {
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
                let Ok(connection) = CellConnection::try_establish(
                    parent_site,
                    child_site,
                    &parent.structure,
                    &candidate_structure,
                    catalog,
                    CONNECTION_TOLERANCE,
                    MIN_CONNECTION_FACING,
                ) else {
                    continue;
                };
                return Some((candidate_structure, connection, offset));
            }
        }
    }
    None
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

    let placement = crate::structure::Placement {
        x: element.placement.x + construction.world_offset.x,
        y: element.placement.y + construction.world_offset.y,
        rotation_radians: element.placement.rotation_radians,
    };
    let unit_material = StructuralMaterial {
        material,
        internal_bonds: element.material.internal_bonds.clone(),
    };
    let Some(unit) = StructuralUnit::from_material(unit_material, placement) else {
        return false;
    };

    construction.developing_structure.add_unit(unit);
    construction.next_blueprint_element += 1;
    realize_new_blueprint_connections(construction, catalog);
    true
}

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
    use crate::state::ResourceSense;
    use crate::structural_blueprint::{BlueprintElement, BlueprintGeometry, StructuralBlueprint};
    use crate::structure::Placement;

    fn adult_parent(material_amount: f64) -> Organism {
        let mut structure = OrganismStructure::new();
        structure.add_unit(StructuralUnit::new("Carbon", Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 }));
        Organism {
            id: "parent".into(),
            occupied_cells: vec![Position { x: 0.0, y: 0.0 }],
            genome: initial_genome(),
            resource_sense: ResourceSense { sensed_resources: Vec::new(), direction_x: 0.0, direction_y: 0.0, direction_strength: 0.0 },
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
        let shape = catalog.iter().find(|resource| resource.name == resource_name).unwrap().shape.clone();
        BlueprintElement {
            material: StructuralMaterial::single(resource_name),
            geometry: BlueprintGeometry::single(shape),
            placement: Placement { x, y, rotation_radians: 0.0 },
        }
    }

    fn test_connection() -> CellConnection {
        let catalog = crate::resources::default_catalog();
        let parent = adult_parent(0.0);
        let mut child = OrganismStructure::new();
        child.add_unit(StructuralUnit::new("Carbon", Placement { x: 1.0, y: 0.0, rotation_radians: 0.0 }));
        CellConnection::try_establish(
            CellSiteRef { organism_id: "parent".into(), unit_index: 0, point_index: 0 },
            CellSiteRef { organism_id: "child".into(), unit_index: 0, point_index: 3 },
            &parent.structure,
            &child,
            &catalog,
            CONNECTION_TOLERANCE,
            MIN_CONNECTION_FACING,
        ).expect("test endpoints should be in physical contact")
    }

    #[test]
    fn reproduction_commits_seed_and_establishes_physical_connection() {
        let mut parent = adult_parent(1.0);
        parent.genome.structural_blueprint = StructuralBlueprint::new(
            vec![blueprint_element("Carbon", 0.0, 0.0), blueprint_element("Methane", 1.0, 0.0)],
            Vec::new(),
        );
        let mut rng = ChaCha8Rng::seed_from_u64(7);
        let catalog = crate::resources::default_catalog();
        assert!(begin_reproduction(&mut parent, "child".into(), &mut rng, &catalog));
        assert_eq!(parent.stored_unbonded.total_amount(), 0.0);
        let construction = parent.reproductive_construction.as_ref().unwrap();
        assert_eq!(construction.child_id, "child");
        assert!(construction.committed_material.is_empty());
        assert_eq!(construction.developing_structure.units.len(), 1);
        assert_eq!(construction.next_blueprint_element, 1);
        assert!(construction.connection.crosses_organism_boundary());
    }

    #[test]
    fn construction_uses_blueprint_order_and_pauses_without_required_material() {
        let catalog = crate::resources::default_catalog();
        let genome = Genome {
            traits: initial_genome().traits,
            structural_blueprint: StructuralBlueprint::new(
                vec![blueprint_element("Carbon", 0.0, 0.0), blueprint_element("Methane", 8.0, 9.0)],
                Vec::new(),
            ),
        };
        let world_offset = Position { x: 1.0, y: 2.0 };
        let mut construction = ReproductiveConstruction {
            child_id: "child".into(),
            connection: test_connection(),
            committed_material: Material::free_base("Methane", 1.0),
            developing_structure: {
                let mut s = OrganismStructure::new();
                s.add_unit(StructuralUnit::new("Carbon", Placement { x: world_offset.x, y: world_offset.y, rotation_radians: 0.0 }));
                s
            },
            child_genome: genome,
            world_offset,
            next_blueprint_element: 1,
        };
        assert!(advance_construction(&mut construction, &catalog));
        assert_eq!(construction.next_blueprint_element, 2);
        assert_eq!(construction.developing_structure.units.len(), 2);
        assert_eq!(construction.developing_structure.units[1].placement.x, 9.0);
        assert_eq!(construction.developing_structure.units[1].placement.y, 11.0);
        assert!(!advance_construction(&mut construction, &catalog));
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
            child_id: "child".into(),
            connection: test_connection(),
            committed_material: Material { parts: Vec::new(), bonded: false },
            developing_structure: {
                let mut s = OrganismStructure::new();
                s.add_unit(StructuralUnit::new("Carbon", Placement { x: 1.0, y: 0.0, rotation_radians: 0.0 }));
                s
            },
            child_genome: genome,
            world_offset: Position { x: 1.0, y: 0.0 },
            next_blueprint_element: 1,
        };
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
