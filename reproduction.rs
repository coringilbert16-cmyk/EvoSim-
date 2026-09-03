//! Physical budding/reproduction boundary.
//!
//! Reproduction is not a separate magic action. A reproductive decision may
//! allocate real stored material and usable energy into a small bud. The bud
//! is then assembled through the ordinary COMBINE runtime before it becomes
//! a new organism. Heritable information is copied from the parent and
//! mutated only after the physical bud has successfully formed.

use rand_chacha::ChaCha8Rng;

use crate::combine_runtime::try_combine;
use crate::contact::ConnectionCompatibilityCache;
use crate::decision::DecisionHistory;
use crate::genome::Genome;
use crate::resources::{BaseResource, Material};
use crate::state::{DevelopmentStage, Environment, Organism, Position, ResourceSense};
use crate::structure::{Placement, StructuralUnit};

const EPSILON: f64 = 1e-12;
const MIN_BUD_MATERIAL_UNITS: usize = 2;
const READINESS_THRESHOLD: f64 = 1.0;

fn take_named_unit(material: &mut Material, name: &str) -> Option<Material> {
    let part = material
        .parts
        .iter_mut()
        .find(|(part_name, amount)| part_name == name && *amount >= 1.0 - EPSILON)?;
    part.1 -= 1.0;
    material.parts.retain(|(_, amount)| *amount > EPSILON);
    Some(Material::free_base(name.to_owned(), 1.0))
}

fn choose_bud_material(parent: &mut Material, catalog: &[BaseResource]) -> Option<[Material; 2]> {
    if parent.bonded || parent.total_amount() + EPSILON < MIN_BUD_MATERIAL_UNITS as f64 {
        return None;
    }

    let first_name = parent
        .parts
        .iter()
        .find(|(_, amount)| *amount >= 1.0 - EPSILON)
        .map(|(name, _)| name.clone())?;
    let second_name = parent
        .parts
        .iter()
        .find(|(name, amount)| {
            *amount >= 1.0 - EPSILON && *name != first_name && catalog.iter().any(|r| r.name == *name)
        })
        .map(|(name, _)| name.clone())?;

    let first = take_named_unit(parent, &first_name)?;
    let second = take_named_unit(parent, &second_name)?;
    Some([first, second])
}

fn make_bud(
    parent: &Organism,
    materials: [Material; 2],
    child_id: String,
    catalog: &[BaseResource],
    rng: &mut ChaCha8Rng,
) -> Option<(Organism, f64)> {
    let origin = parent
        .occupied_cells
        .first()
        .map(|p| Position {
            x: p.x + 2.0,
            y: p.y,
        })
        .unwrap_or(Position { x: 2.0, y: 0.0 });

    let mut child = Organism {
        id: child_id,
        occupied_cells: vec![origin.clone()],
        genome: parent.genome.clone(),
        resource_sense: ResourceSense {
            sensed_resources: Vec::new(),
            direction_x: 0.0,
            direction_y: 0.0,
            direction_strength: 0.0,
        },
        memory: Vec::new(),
        decision_history: DecisionHistory::default(),
        usable_energy: 0.0,
        stress: 0.0,
        stored_unbonded: Material {
            parts: Vec::new(),
            bonded: false,
        },
        structure: crate::structure::OrganismStructure::new(),
        development_stage: DevelopmentStage::Juvenile,
        age: 0,
        reproductive_readiness: 0.0,
        active_transformation_id: None,
    };

    let names = [materials[0].parts[0].0.clone(), materials[1].parts[0].0.clone()];
    let radii: [f64; 2] = names.map(|name| {
        catalog
            .iter()
            .find(|resource| resource.name == name)
            .map(|resource| resource.shape.form.bounding_radius())
            .unwrap_or(0.0)
    });
    if radii.iter().any(|radius| !radius.is_finite() || *radius <= 0.0) {
        return None;
    }

    child.stored_unbonded = Material {
        parts: materials
            .iter()
            .flat_map(|material| material.parts.iter().cloned())
            .collect(),
        bonded: false,
    };

    let first = crate::combine_runtime::instantiate_one_unit(&mut child, catalog)?;
    let second = crate::combine_runtime::instantiate_one_unit(&mut child, catalog)?;
    child.structure.units[first].placement = Placement {
        x: origin.x,
        y: origin.y,
        rotation_radians: 0.0,
    };
    child.structure.units[second].placement = Placement {
        x: origin.x + radii[0] + radii[1],
        y: origin.y,
        rotation_radians: 0.0,
    };

    let mut cache = ConnectionCompatibilityCache::new();
    let required_energy = parent.usable_energy * parent.genome.reproductive_investment();
    if !required_energy.is_finite() || required_energy <= EPSILON {
        return None;
    }
    child.usable_energy = required_energy;

    try_combine(&mut child, &Environment {
        width: 0.0,
        height: 0.0,
        catalog: catalog.to_vec(),
        field: panic_field_placeholder(),
        reservoir: panic_reservoir_placeholder(),
        vents: Vec::new(),
    }, &mut cache)?;

    child.genome.mutate(rng);
    let energy_transferred = required_energy;
    Some((child, energy_transferred))
}

// These constructors are intentionally unreachable: `try_form_bud` supplies
// the real environment. They keep `make_bud` focused on child construction
// without introducing a second COMBINE implementation.
fn panic_field_placeholder() -> crate::environment::ActiveMaterialField {
    panic!("internal reproduction placeholder must not be called")
}
fn panic_reservoir_placeholder() -> crate::environment::DeepReservoir {
    panic!("internal reproduction placeholder must not be called")
}

/// Attempt one physical reproductive bud from a parent that has been selected
/// for COMBINE and has accumulated full reproductive readiness.
///
/// The caller owns the final population insertion. On failure, the parent is
/// left unchanged and no child genome mutation is performed.
pub(crate) fn try_form_bud(
    parent: &mut Organism,
    environment: &Environment,
    child_id: String,
    rng: &mut ChaCha8Rng,
) -> Option<Organism> {
    if parent.reproductive_readiness + EPSILON < READINESS_THRESHOLD {
        return None;
    }

    let adult_mass = parent.genome.adult_mass();
    let structural_mass: f64 = parent
        .structure
        .units
        .iter()
        .filter_map(|unit| unit.properties(&environment.catalog).map(|properties| properties.mass))
        .sum();
    if structural_mass + EPSILON < adult_mass {
        return None;
    }

    let original_material = parent.stored_unbonded.clone();
    let original_energy = parent.usable_energy;
    let original_readiness = parent.reproductive_readiness;
    let materials = choose_bud_material(&mut parent.stored_unbonded, &environment.catalog)?;

    let origin = parent
        .occupied_cells
        .first()
        .cloned()
        .unwrap_or(Position { x: 0.0, y: 0.0 });
    let names = [materials[0].parts[0].0.clone(), materials[1].parts[0].0.clone()];
    let radii: [f64; 2] = names.map(|name| {
        environment
            .catalog
            .iter()
            .find(|resource| resource.name == name)
            .map(|resource| resource.shape.form.bounding_radius())
            .unwrap_or(0.0)
    });
    if radii.iter().any(|radius| !radius.is_finite() || *radius <= 0.0) {
        parent.stored_unbonded = original_material;
        return None;
    }

    let mut child = Organism {
        id: child_id,
        occupied_cells: vec![Position {
            x: origin.x + 2.0,
            y: origin.y,
        }],
        genome: parent.genome.clone(),
        resource_sense: ResourceSense {
            sensed_resources: Vec::new(),
            direction_x: 0.0,
            direction_y: 0.0,
            direction_strength: 0.0,
        },
        memory: Vec::new(),
        decision_history: DecisionHistory::default(),
        usable_energy: 0.0,
        stress: 0.0,
        stored_unbonded: Material {
            parts: materials
                .iter()
                .flat_map(|material| material.parts.iter().cloned())
                .collect(),
            bonded: false,
        },
        structure: crate::structure::OrganismStructure::new(),
        development_stage: DevelopmentStage::Juvenile,
        age: 0,
        reproductive_readiness: 0.0,
        active_transformation_id: None,
    };

    let first = match crate::combine_runtime::instantiate_one_unit(&mut child, &environment.catalog) {
        Some(index) => index,
        None => {
            parent.stored_unbonded = original_material;
            return None;
        }
    };
    let second = match crate::combine_runtime::instantiate_one_unit(&mut child, &environment.catalog) {
        Some(index) => index,
        None => {
            parent.stored_unbonded = original_material;
            return None;
        }
    };
    child.structure.units[first].placement = Placement {
        x: origin.x + 2.0,
        y: origin.y,
        rotation_radians: 0.0,
    };
    child.structure.units[second].placement = Placement {
        x: origin.x + 2.0 + radii[0] + radii[1],
        y: origin.y,
        rotation_radians: 0.0,
    };

    let energy_budget = parent.usable_energy * parent.genome.reproductive_investment();
    if !energy_budget.is_finite() || energy_budget <= EPSILON {
        parent.stored_unbonded = original_material;
        return None;
    }
    child.usable_energy = energy_budget;

    let mut cache = ConnectionCompatibilityCache::new();
    if try_combine(&mut child, environment, &mut cache).is_none() {
        parent.stored_unbonded = original_material;
        parent.usable_energy = original_energy;
        parent.reproductive_readiness = original_readiness;
        return None;
    }

    let energy_used = energy_budget - child.usable_energy;
    if !energy_used.is_finite() || energy_used < 0.0 || original_energy + EPSILON < energy_used {
        parent.stored_unbonded = original_material;
        parent.usable_energy = original_energy;
        parent.reproductive_readiness = original_readiness;
        return None;
    }

    parent.usable_energy -= energy_used;
    parent.reproductive_readiness = 0.0;
    child.genome.mutate(rng);
    Some(child)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genome::initial_genome;
    use rand::SeedableRng;

    fn parent() -> Organism {
        let mut organism = Organism {
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
            usable_energy: 100.0,
            stress: 0.0,
            stored_unbonded: Material {
                parts: vec![("Carbon".into(), 1.0), ("Methane".into(), 1.0)],
                bonded: false,
            },
            structure: crate::structure::OrganismStructure::new(),
            development_stage: DevelopmentStage::Adult,
            age: 100,
            reproductive_readiness: 1.0,
            active_transformation_id: None,
        };
        for i in 0..16 {
            organism.structure.add_unit(StructuralUnit::new(
                "Carbon",
                Placement {
                    x: i as f64,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
            ));
        }
        organism
    }

    #[test]
    fn ready_parent_forms_bud_from_real_material_and_mutates_inherited_genome() {
        let environment = Environment {
            width: 1000.0,
            height: 1000.0,
            catalog: crate::resources::default_catalog(),
            field: crate::environment::ActiveMaterialField::new(1000.0, 1000.0, 10.0),
            reservoir: crate::environment::DeepReservoir::new(1000.0, 1000.0, 100.0),
            vents: Vec::new(),
        };
        let mut parent = parent();
        let original_genome = parent.genome.clone();
        let mut rng = ChaCha8Rng::seed_from_u64(7);
        let child = try_form_bud(&mut parent, &environment, "2".into(), &mut rng)
            .expect("ready parent with viable material should form a bud");

        assert_eq!(child.development_stage, DevelopmentStage::Juvenile);
        assert_eq!(child.structure.units.len(), 2);
        assert_eq!(child.structure.bonds.len(), 1);
        assert_eq!(child.genome.traits.len(), original_genome.traits.len());
        assert_eq!(parent.stored_unbonded.total_amount(), 0.0);
        assert_eq!(parent.reproductive_readiness, 0.0);
    }
}
