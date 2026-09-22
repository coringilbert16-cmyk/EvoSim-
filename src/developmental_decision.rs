#![expect(
    dead_code,
    reason = "Staged developmental action bridge retained for simulation integration"
)]
//! Physical developmental previews used by organism action selection.
//!
//! This module evaluates candidate actions through the existing realized
//! structure and COMBINE machinery. It introduces no developmental weights,
//! penalties, or action priorities.

use crate::decision::{ActionKind, CurrentNeeds};
use crate::decision_runtime::ActionCandidate;
use crate::state::{DevelopmentStage, EnergyLedger, Environment, Organism};

pub(crate) fn growth_fraction(organism: &Organism, environment: &Environment) -> f64 {
    growth_fraction_for_structure(organism, environment, &organism.structure)
}

fn growth_fraction_for_structure(
    organism: &Organism,
    environment: &Environment,
    structure: &crate::structure::PhysicalStructure,
) -> f64 {
    organism
        .genome
        .developmental_blueprint
        .realization(
            structure,
            &environment.catalog,
            (
                organism.developmental_origin.x,
                organism.developmental_origin.y,
            ),
            organism.developmental_orientation_radians,
            organism.genome.adult_mass(),
        )
        .overall
        .clamp(0.0, 1.0)
}

pub(crate) fn developmental_action_scores(
    organism: &Organism,
    environment: &Environment,
    needs: CurrentNeeds,
    candidates: &[ActionCandidate],
    ledger: &EnergyLedger,
) -> Vec<Option<f64>> {
    if !matches!(
        organism.development_stage,
        DevelopmentStage::Juvenile
    ) || needs.development <= 0.0
    {
        return vec![None; candidates.len()];
    }

    let blueprint = &organism.genome.developmental_blueprint;
    let (seed_mass, seed_length) =
        crate::juvenile::confirmed_seed_scale_reference(&environment.catalog)
            .expect("confirmed seed scale reference must be valid");
    let preferred_length = blueprint.preferred_developmental_length(
        organism.genome.adult_mass(),
        seed_mass,
        seed_length,
    );
    let developmental = Some((
        blueprint,
        (
            organism.developmental_origin.x,
            organism.developmental_origin.y,
        ),
        organism.developmental_orientation_radians,
        preferred_length,
    ));

    candidates
        .iter()
        .map(|candidate| {
            match candidate.action {
                ActionKind::Combine => {
                    let mut trial = organism.clone();
                    let mut trial_ledger = *ledger;
                    let mut cache = crate::contact::ConnectionCompatibilityCache::new();
                    crate::combine_runtime::try_combine(
                        &mut trial,
                        environment,
                        &mut cache,
                        &mut trial_ledger,
                        developmental,
                    )?;
                    Some(growth_fraction(&trial, environment))
                }
                ActionKind::Break => {
                    let index = candidate
                        .context_key
                        .as_deref()
                        .and_then(|key| key.strip_prefix("bond:"))
                        .and_then(|index| index.parse::<usize>().ok())?;
                    let bond = *organism.structure.bonds.get(index)?;
                    let mut structure = organism.structure.clone();
                    structure.break_matching_bond(bond)?;
                    Some(growth_fraction_for_structure(
                        organism,
                        environment,
                        &structure,
                    ))
                }
                _ => None,
            }
        })
        .collect()
}
