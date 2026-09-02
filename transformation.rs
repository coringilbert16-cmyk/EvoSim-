use crate::combine_runtime::{complete_combine, CombineTransformation};
use crate::decision::{ActionKind, DecisionHistory};
use crate::decision_runtime::{ActionCandidate, DecisionRuntime};
use crate::environment::Environment;
use crate::genome::Genome;
use crate::math::clamp01;
use crate::memory::MemoryPoint;
use crate::resources::Material;
use crate::state::{Organism, Position, ResourceSense};
use crate::structure::{Bond, OrganismStructure};

const STRESS_DECAY_PER_TICK: f64 = 0.99;

pub(crate) struct TransformationRuntime;

impl TransformationRuntime {
    pub(crate) fn begin_combine(
        organism: &mut Organism,
        environment: &mut Environment,
        target: usize,
    ) -> Option<CombineTransformation> {
        crate::combine_runtime::begin_combine(organism, environment, target)
    }

    pub(crate) fn complete_combine(
        organism: &mut Organism,
        environment: &mut Environment,
        transformation: CombineTransformation,
    ) {
        complete_combine(organism, environment, transformation);
    }

    pub(crate) fn reinforce_memory_point(
        organism: &mut Organism,
        x: f64,
        y: f64,
        reinforcement: f64,
    ) {
        if let Some(point) = organism.memory.iter_mut().find(|p| {
            (p.x - x).abs() < f64::EPSILON && (p.y - y).abs() < f64::EPSILON
        }) {
            point.strength = clamp01(point.strength + reinforcement);
        } else {
            organism.memory.push(MemoryPoint {
                x,
                y,
                strength: clamp01(reinforcement),
            });
        }
    }

    pub(crate) fn apply_energy_capacity(organism: &mut Organism) {
        organism.stress *= STRESS_DECAY_PER_TICK;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::{ActionKind, DecisionHistory};
    use crate::decision_runtime::ActionCandidate;
    use crate::genome::initial_genome;
    use crate::resources::Material;
    use crate::state::{DevelopmentStage, Position, ResourceSense};
    use crate::structure::{Bond, OrganismStructure, Placement, StructuralUnit};

    fn organism() -> Organism {
        Organism {
            id: "test".into(),
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
            usable_energy: 0.0,
            stress: 0.0,
            stored_unbonded: Material {
                parts: Vec::new(),
                bonded: false,
            },
            structure: OrganismStructure::new(),
            development_stage: DevelopmentStage::Juvenile,
            reproductive_readiness: 0.0,
            age: 0,
            active_transformation_id: None,
        }
    }

    #[test]
    fn break_transformation_uses_stored_bond_energy() {
        let mut o = organism();
        let a = o.structure.add_unit(StructuralUnit::new(
            "Carbon",
            Placement {
                x: 0.0,
                y: 0.0,
            },
        ));
        let b = o.structure.add_unit(StructuralUnit::new(
            "Carbon",
            Placement {
                x: 1.0,
                y: 0.0,
            },
        ));
        o.structure.add_bond(Bond::new(a, b, 2.0));
        assert!(o.structure.bonds.len() == 1);
    }

    #[test]
    fn decision_candidate_is_constructed_for_break() {
        let candidate = ActionCandidate::new(ActionKind::Break, Some("Carbon".into()));
        assert_eq!(candidate.action, ActionKind::Break);
    }
}
