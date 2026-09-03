//! Physical budding/reproduction boundary.
//!
//! Reproduction transfers real physical material and usable energy from a
//! parent into a new Offspring. The offspring begins from a physical copy of
//! the parent's current core rather than a reproduction-only starter recipe.

use rand_chacha::ChaCha8Rng;

use crate::decision::DecisionHistory;
use crate::resources::Material;
use crate::state::{DevelopmentStage, Environment, Organism, Position, ResourceSense};

const EPSILON: f64 = 1e-12;
const READINESS_THRESHOLD: f64 = 1.0;

fn structural_mass(organism: &Organism, environment: &Environment) -> f64 {
    organism
        .structure
        .units
        .iter()
        .filter_map(|unit| unit.properties(&environment.catalog).map(|properties| properties.mass))
        .sum()
}

/// Attempt one physical reproductive investment from a parent that has full
/// reproductive readiness.
///
/// The offspring inherits the parent's actual structural core and a real
/// share of the parent's usable energy. No secondary offspring resource pool
/// is created. The inherited genome is mutated after the physical offspring
/// has been created.
pub(crate) fn try_form_bud(
    parent: &mut Organism,
    _environment: &Environment,
    child_id: String,
    rng: &mut ChaCha8Rng,
) -> Option<Organism> {
    if parent.reproductive_readiness + EPSILON < READINESS_THRESHOLD {
        return None;
    }

    if parent.structure.units.is_empty() {
        return None;
    }

    let energy_budget = parent.usable_energy * parent.genome.reproductive_investment();
    if !energy_budget.is_finite() || energy_budget <= EPSILON {
        return None;
    }

    let invested_structure = parent.structure.clone();
    let invested_mass = structural_mass(parent, _environment);
    if !invested_mass.is_finite() || invested_mass <= EPSILON {
        return None;
    }

    // Reproduction transfers real physical structure and usable energy to the
    // offspring. The parent does not retain a duplicate of the transferred
    // core. This is intentionally a whole-core transfer for now: there is no
    // special bud recipe or hidden offspring reserve.
    parent.usable_energy -= energy_budget;
    parent.reproductive_readiness = 0.0;

    let mut child = Organism {
        id: child_id,
        occupied_cells: vec![parent
            .occupied_cells
            .first()
            .cloned()
            .unwrap_or(Position { x: 0.0, y: 0.0 })],
        genome: parent.genome.clone(),
        resource_sense: ResourceSense {
            sensed_resources: Vec::new(),
            direction_x: 0.0,
            direction_y: 0.0,
            direction_strength: 0.0,
        },
        memory: Vec::new(),
        decision_history: DecisionHistory::default(),
        usable_energy: energy_budget,
        stress: 0.0,
        stored_unbonded: Material {
            parts: Vec::new(),
            bonded: false,
        },
        structure: invested_structure,
        development_stage: DevelopmentStage::Offspring,
        age: 0,
        reproductive_readiness: 0.0,
        active_transformation_id: None,
    };

    child.genome.mutate(rng);
    Some(child)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genome::initial_genome;
    use crate::resources::Material;
    use crate::state::{Environment, Position, ResourceSense};
    use crate::structure::{OrganismStructure, Placement, StructuralUnit};
    use rand::SeedableRng;

    fn environment() -> Environment {
        Environment {
            width: 1000.0,
            height: 1000.0,
            catalog: crate::resources::default_catalog(),
            field: crate::environment::ActiveMaterialField::new(1000.0, 1000.0, 10.0),
            reservoir: crate::environment::DeepReservoir::new(1000.0, 1000.0, 100.0),
            vents: Vec::new(),
        }
    }

    fn parent(environment: &Environment) -> Organism {
        let mut structure = OrganismStructure::new();
        structure.add_unit(StructuralUnit::new(
            "Carbon",
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));
        structure.add_unit(StructuralUnit::new(
            "Carbon",
            Placement {
                x: 1.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));

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
                parts: Vec::new(),
                bonded: false,
            },
            structure,
            development_stage: DevelopmentStage::Adult,
            age: 100,
            reproductive_readiness: 1.0,
            active_transformation_id: None,
        };

        // Ensure the fixture itself is physically meaningful under the
        // catalog; the production function does not otherwise need a second
        // structural representation.
        assert!(structural_mass(&organism, environment) > 0.0);
        organism
    }

    #[test]
    fn reproduction_transfers_the_real_parent_core_and_energy() {
        let environment = environment();
        let mut parent = parent(&environment);
        let original_units = parent.structure.units.clone();
        let original_energy = parent.usable_energy;
        let original_mass = structural_mass(&parent, &environment);
        let mut rng = ChaCha8Rng::seed_from_u64(7);

        let child = try_form_bud(&mut parent, &environment, "2".into(), &mut rng)
            .expect("ready parent should form offspring");

        assert!(matches!(child.development_stage, DevelopmentStage::Offspring));
        assert_eq!(child.structure.units, original_units);
        assert_eq!(structural_mass(&child, &environment), original_mass);
        assert!(parent.usable_energy < original_energy);
        assert!(child.usable_energy > 0.0);
        assert_eq!(parent.reproductive_readiness, 0.0);
    }

    #[test]
    fn offspring_has_no_secondary_resource_pool() {
        let environment = environment();
        let mut parent = parent(&environment);
        let mut rng = ChaCha8Rng::seed_from_u64(7);

        let child = try_form_bud(&mut parent, &environment, "2".into(), &mut rng)
            .expect("ready parent should form offspring");

        assert!(child.stored_unbonded.is_empty());
        assert!(child.structure.units.len() >= 1);
    }
}
