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
use crate::structure::{OrganismStructure, Placement, StructuralUnit};

const CORE_UNIT_COUNT: usize = 6;
const CORE_MATERIAL_AMOUNT: f64 = CORE_UNIT_COUNT as f64;

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

/// Advance reproductive construction by exactly one physical structural unit.
///
/// This is deliberately only the first construction primitive: it consumes
/// real committed material and creates a real structural unit. Bond formation,
/// core completion, development, and birth remain separate lifecycle steps.
pub(crate) fn advance_construction(construction: &mut ReproductiveConstruction) -> bool {
    let Some((resource_name, _)) = construction
        .committed_material
        .parts
        .iter()
        .find(|(_, amount)| *amount >= 1.0 - f64::EPSILON)
        .map(|(name, amount)| (name.clone(), *amount))
    else {
        return false;
    };

    let placement = construction_placement(construction);
    construction
        .developing_structure
        .add_unit(StructuralUnit::new(resource_name.clone(), placement));

    if let Some((_, stored_amount)) = construction
        .committed_material
        .parts
        .iter_mut()
        .find(|(name, _)| *name == resource_name)
    {
        *stored_amount -= 1.0;
    }
    construction
        .committed_material
        .parts
        .retain(|(_, amount)| *amount > 1e-12);
    true
}

fn construction_placement(construction: &ReproductiveConstruction) -> Placement {
    let compactness = construction.child_genome.construction_compactness();
    let branching = construction.child_genome.construction_branching();
    let index = construction.developing_structure.units.len() as f64;

    if index == 0.0 {
        return Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 };
    }

    let radius = 0.9 + (1.0 - compactness) * 0.6;
    let angle = index * std::f64::consts::FRAC_PI_3
        + (branching - 0.5) * std::f64::consts::FRAC_PI_2;
    Placement {
        x: radius * angle.cos(),
        y: radius * angle.sin(),
        rotation_radians: angle * branching,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use crate::decision::DecisionHistory;
    use crate::genome::initial_genome;
    use crate::resources::Material;
    use crate::state::{Position, ResourceSense};

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
    fn construction_consumes_one_real_unit_per_step() {
        let mut parent = adult_parent(CORE_MATERIAL_AMOUNT);
        let mut rng = ChaCha8Rng::seed_from_u64(7);
        assert!(begin_reproduction(&mut parent, &mut rng));
        let construction = parent.reproductive_construction.as_mut().unwrap();
        assert!(advance_construction(construction));
        assert_eq!(construction.developing_structure.units.len(), 1);
        assert_eq!(construction.committed_material.total_amount(), 5.0);
        assert_eq!(parent.structure.units.len(), 0);
        assert!(advance_construction(construction));
        assert_eq!(construction.developing_structure.units.len(), 2);
        assert_eq!(construction.committed_material.total_amount(), 4.0);
    }

    #[test]
    fn construction_stops_when_no_full_unit_remains() {
        let mut parent = adult_parent(CORE_MATERIAL_AMOUNT);
        let mut rng = ChaCha8Rng::seed_from_u64(7);
        assert!(begin_reproduction(&mut parent, &mut rng));
        let construction = parent.reproductive_construction.as_mut().unwrap();
        for _ in 0..CORE_UNIT_COUNT {
            assert!(advance_construction(construction));
        }
        assert!(!advance_construction(construction));
        assert!(construction.committed_material.is_empty());
        assert_eq!(construction.developing_structure.units.len(), CORE_UNIT_COUNT);
    }
}
