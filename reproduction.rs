//! Physical reproduction lifecycle.
//!
//! Reproduction does not split or copy the parent's existing structure. Once
//! an adult has accumulated enough actual unbonded material, the reproductive
//! decision commits that material to a persistent construction state. The
//! parent keeps its own structure intact. Later ticks will consume the
//! committed material through the ordinary structural construction system and
//! only create a separate organism when the developing structure reaches the
//! genetically determined juvenile threshold.

use rand_chacha::ChaCha8Rng;

use crate::state::{DevelopmentStage, Organism, ReproductiveConstruction};
use crate::structure::OrganismStructure;

/// The existing core-integrity invariant requires six structural units. The
/// current material model represents one structural unit as one unit of raw
/// material, so six units are the minimum physical commitment needed before
/// construction can begin.
const CORE_UNIT_COUNT: usize = 6;
const CORE_MATERIAL_AMOUNT: f64 = CORE_UNIT_COUNT as f64;

/// Begin a persistent reproductive construction process.
///
/// Nothing about the parent's existing structure is transferred. The only
/// material removed from ordinary parental use is real unbonded material that
/// is committed to the developing offspring. The child genome is copied and
/// mutated at conception; no child organism exists yet.
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
    if parent.stored_unbonded.total_amount() + f64::EPSILON < CORE_MATERIAL_AMOUNT {
        return false;
    }

    let Some(committed_material) = parent.stored_unbonded.take(CORE_MATERIAL_AMOUNT) else {
        return false;
    };

    let mut child_genome = parent.genome.clone();
    child_genome.mutate(rng);

    parent.reproductive_readiness = 0.0;
    parent.reproductive_construction = Some(ReproductiveConstruction {
        committed_material,
        developing_structure: OrganismStructure::new(),
        child_genome,
    });
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::DecisionHistory;
    use crate::genome::initial_genome;
    use crate::resources::Material;
    use crate::state::{Position, ResourceSense};

    fn adult_parent(material_amount: f64) -> Organism {
        Organism {
            id: "parent".into(),
            occupied_cells: vec![Position { x: 50.0, y: 50.0 }],
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
            structure: OrganismStructure::new(),
            development_stage: DevelopmentStage::Adult,
            age: 10,
            reproductive_readiness: 1.0,
            active_transformation_id: None,
            reproductive_construction: None,
        }
    }

    #[test]
    fn reproduction_commits_real_material_without_touching_parent_structure() {
        let mut parent = adult_parent(CORE_MATERIAL_AMOUNT + 2.0);
        let mut rng = ChaCha8Rng::seed_from_u64(7);

        assert!(begin_reproduction(&mut parent, &mut rng));
        assert!(parent.structure.units.is_empty());
        assert!(parent.structure.bonds.is_empty());
        assert_eq!(parent.stored_unbonded.total_amount(), 2.0);
        assert_eq!(parent.reproductive_readiness, 0.0);

        let construction = parent.reproductive_construction.as_ref().unwrap();
        assert_eq!(construction.committed_material.total_amount(), CORE_MATERIAL_AMOUNT);
        assert!(!construction.committed_material.bonded);
        assert!(construction.developing_structure.units.is_empty());
    }

    #[test]
    fn reproduction_requires_enough_actual_material() {
        let mut parent = adult_parent(CORE_MATERIAL_AMOUNT - 0.01);
        let mut rng = ChaCha8Rng::seed_from_u64(7);

        assert!(!begin_reproduction(&mut parent, &mut rng));
        assert!(parent.reproductive_construction.is_none());
        assert_eq!(parent.stored_unbonded.total_amount(), CORE_MATERIAL_AMOUNT - 0.01);
    }

    #[test]
    fn reproduction_does_not_create_a_child_organism() {
        let mut parent = adult_parent(CORE_MATERIAL_AMOUNT);
        let mut rng = ChaCha8Rng::seed_from_u64(11);

        assert!(begin_reproduction(&mut parent, &mut rng));
        assert!(parent.reproductive_construction.is_some());
    }
}
