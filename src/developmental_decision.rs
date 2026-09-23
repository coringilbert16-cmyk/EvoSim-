#![expect(
    dead_code,
    reason = "Staged developmental action bridge retained for simulation integration"
)]
//! Physical developmental previews used by organism action selection.
//!
//! Only genuinely competing developmental actions are physically previewed.
//! No developmental weights, penalties, or action priorities are introduced.

use crate::decision::{ActionKind, CurrentNeeds};
use crate::decision_runtime::ActionCandidate;
use crate::state::{DevelopmentStage, EnergyLedger, Environment, Organism};

pub(crate) struct DevelopmentalContext<'a> {
    pub(crate) blueprint: &'a crate::developmental_blueprint::DevelopmentalFieldBlueprint,
    pub(crate) origin: (f64, f64),
    pub(crate) orientation: f64,
    pub(crate) preferred_length: f64,
}

pub(crate) fn context<'a>(
    organism: &'a Organism,
    environment: &Environment,
) -> Option<DevelopmentalContext<'a>> {
    if !matches!(organism.development_stage, DevelopmentStage::Juvenile) {
        return None;
    }
    let (seed_mass, seed_length) =
        crate::juvenile::confirmed_seed_scale_reference(&environment.catalog).ok()?;
    let blueprint = &organism.genome.developmental_blueprint;
    Some(DevelopmentalContext {
        blueprint,
        origin: (
            organism.developmental_origin.x,
            organism.developmental_origin.y,
        ),
        orientation: organism.developmental_orientation_radians,
        preferred_length: blueprint.preferred_developmental_length(
            organism.genome.adult_mass(),
            seed_mass,
            seed_length,
        ),
    })
}

pub(crate) fn growth_fraction(organism: &Organism, environment: &Environment) -> f64 {
    context(organism, environment)
        .map(|developmental| {
            growth_fraction_for_context(organism, environment, &organism.structure, &developmental)
        })
        .unwrap_or(0.0)
}

pub(crate) fn growth_fraction_for_context(
    _organism: &Organism,
    environment: &Environment,
    structure: &crate::structure::OrganismStructure,
    developmental: &DevelopmentalContext<'_>,
) -> f64 {
    developmental
        .blueprint
        .realization_at_length(
            structure,
            &environment.catalog,
            developmental.origin,
            developmental.orientation,
            developmental.preferred_length,
        )
        .overall
        .clamp(0.0, 1.0)
}

pub(crate) fn developmental_action_scores(
    organism: &Organism,
    environment: &Environment,
    needs: CurrentNeeds,
    candidates: &[ActionCandidate],
    competing_indices: &[usize],
    developmental: Option<&DevelopmentalContext<'_>>,
    ledger: &EnergyLedger,
) -> Vec<Option<f64>> {
    let Some(developmental) = developmental else {
        return vec![None; candidates.len()];
    };
    if needs.development <= 0.0 {
        return vec![None; candidates.len()];
    }

    candidates
        .iter()
        .enumerate()
        .map(|(index, candidate)| {
            if !competing_indices.contains(&index) {
                return None;
            }
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
                        Some((
                            developmental.blueprint,
                            developmental.origin,
                            developmental.orientation,
                            developmental.preferred_length,
                        )),
                    )?;
                    Some(growth_fraction_for_context(
                        &trial,
                        environment,
                        &trial.structure,
                        developmental,
                    ))
                }
                ActionKind::Break => {
                    let bond_index = candidate
                        .context_key
                        .as_deref()
                        .and_then(|key| key.strip_prefix("bond:"))
                        .and_then(|index| index.parse::<usize>().ok())?;
                    let bond = *organism.structure.bonds.get(bond_index)?;
                    let mut structure = organism.structure.clone();
                    structure.break_matching_bond(bond)?;
                    Some(growth_fraction_for_context(
                        organism,
                        environment,
                        &structure,
                        developmental,
                    ))
                }
                _ => None,
            }
        })
        .collect()
}
