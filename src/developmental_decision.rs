#![expect(
    dead_code,
    reason = "Staged developmental action bridge retained for simulation integration"
)]
//! Physical developmental previews used by organism action selection.
//!
//! Only genuinely competing developmental actions are physically previewed.
//! No developmental weights, penalties, or action priorities are introduced.

use crate::decision::{ActionKind, CurrentNeeds};
use crate::contact::ConnectionCompatibilityCache;
use crate::decision_runtime::ActionCandidate;
use crate::state::{DevelopmentStage, EnergyLedger, Environment, Organism};

pub(crate) struct DevelopmentalContext {
    pub(crate) blueprint: crate::developmental_blueprint::DevelopmentalFieldBlueprint,
    pub(crate) origin: (f64, f64),
    pub(crate) orientation: f64,
    pub(crate) preferred_length: f64,
    pub(crate) current_growth_fraction: f64,
    pub(crate) current_material_realized: Option<f64>,
    pub(crate) current_density_realized: Option<f64>,
}

pub(crate) fn context(
    organism: &mut Organism,
    environment: &Environment,
    seed_reference: (f64, f64),
) -> Option<DevelopmentalContext> {
    if !matches!(organism.development_stage, DevelopmentStage::Juvenile) {
        return None;
    }
    let (seed_mass, seed_length) = seed_reference;
    let current_realization = organism
        .developmental_realization_cached(&environment.catalog)
        .unwrap_or(crate::developmental_blueprint::DevelopmentalRealization {
            material: None,
            density: None,
            connectivity: None,
            overall: 0.0,
        });
    let blueprint = organism.genome.developmental_blueprint.clone();
    let origin = (
        organism.developmental_origin.x,
        organism.developmental_origin.y,
    );
    let orientation = organism.developmental_orientation_radians;
    let preferred_length = blueprint.preferred_developmental_length(
        organism.genome.adult_mass(),
        seed_mass,
        seed_length,
    );
    Some(DevelopmentalContext {
        blueprint,
        origin,
        orientation,
        preferred_length,
        current_growth_fraction: current_realization.overall,
        current_material_realized: current_realization.material,
        current_density_realized: current_realization.density,
    })
}

pub(crate) fn growth_fraction(organism: &mut Organism, environment: &Environment) -> f64 {
    let seed_reference = crate::juvenile::confirmed_seed_scale_reference(&environment.catalog).ok();
    seed_reference
        .and_then(|reference| context(organism, environment, reference))
        .map(|developmental| developmental.current_growth_fraction)
        .unwrap_or(0.0)
}

pub(crate) fn growth_fraction_for_context(
    _organism: &Organism,
    environment: &Environment,
    structure: &crate::structure::OrganismStructure,
    developmental: &DevelopmentalContext,
) -> f64 {
    growth_fraction_for_structure(
        structure,
        environment,
        &developmental.blueprint,
        developmental.origin,
        developmental.orientation,
        developmental.preferred_length,
    )
}

fn growth_fraction_for_structure(
    structure: &crate::structure::OrganismStructure,
    environment: &Environment,
    blueprint: &crate::developmental_blueprint::DevelopmentalFieldBlueprint,
    origin: (f64, f64),
    orientation: f64,
    preferred_length: f64,
) -> f64 {
    blueprint
        .realization_at_length(
            structure,
            &environment.catalog,
            origin,
            orientation,
            preferred_length,
        )
        .overall
        .clamp(0.0, 1.0)
}

fn combine_developmental_preview_score(
    organism: &Organism,
    environment: &Environment,
    developmental: &DevelopmentalContext,
) -> Option<f64> {
    let mut best = None;

    // Stored material: score prospective placement directly. This predicts the
    // local developmental contribution without cloning the organism, structure,
    // or energy ledger.
    if let Some(raw) = organism.stored_material.first_material() {
        let first_resource = raw.parts.first()?.0.as_str();
        let geometry_source = environment
            .catalog
            .iter()
            .find(|resource| resource.name == first_resource)?;

        for ua in 0..organism.structure.units.len() {
            let anchor = organism.structure.units[ua].placement;
            for placement in crate::construction_runtime::candidate_placements(
                &organism.structure,
                geometry_source,
                anchor,
                &[ua],
                &environment.catalog,
            ) {
                let local = crate::developmental_blueprint::developmental_point(
                    placement.x,
                    placement.y,
                    developmental.origin,
                    developmental.orientation,
                );
                let score = crate::developmental_blueprint::CANDIDATE_MATERIAL_WEIGHT
                    * developmental.blueprint.material_preference_scaled(
                        first_resource,
                        local.0,
                        local.1,
                        developmental.preferred_length,
                    )
                    + crate::developmental_blueprint::CANDIDATE_DENSITY_WEIGHT
                        * developmental.blueprint.density_preference_scaled(
                            local.0,
                            local.1,
                            developmental.preferred_length,
                        )
                    + crate::developmental_blueprint::CANDIDATE_CONNECTIVITY_WEIGHT
                        * developmental.blueprint.connectivity_preference_scaled(
                            local.0,
                            local.1,
                            developmental.preferred_length,
                        );
                best = Some(best.map_or(score, |current: f64| current.max(score)));
            }
        }
    }

    // Existing structural units: a prospective bond changes connectivity, so
    // score the bond midpoint without constructing a trial structure.
    if organism.structure.units.len() >= 2 {
        let mut cache = ConnectionCompatibilityCache::new();
        for ua in 0..organism.structure.units.len() {
            for ub in ua + 1..organism.structure.units.len() {
                for candidate in crate::combine::eligible_candidates(
                    &organism.structure,
                    ua,
                    ub,
                    &environment.catalog,
                    &mut cache,
                ) {
                    let Some(a) = candidate
                        .endpoint_a
                        .world_point(&organism.structure.units[ua], &environment.catalog)
                    else {
                        continue;
                    };
                    let Some(b) = candidate
                        .endpoint_b
                        .world_point(&organism.structure.units[ub], &environment.catalog)
                    else {
                        continue;
                    };
                    let midpoint = ((a.x + b.x) * 0.5, (a.y + b.y) * 0.5);
                    let local = crate::developmental_blueprint::developmental_point(
                        midpoint.0,
                        midpoint.1,
                        developmental.origin,
                        developmental.orientation,
                    );
                    let score = developmental.blueprint.connectivity_preference_scaled(
                        local.0,
                        local.1,
                        developmental.preferred_length,
                    );
                    best = Some(best.map_or(score, |current: f64| current.max(score)));
                }
            }
        }
    }

    best.map(|score| score.clamp(0.0, 1.0))
}

pub(crate) fn developmental_action_scores(
    organism: &Organism,
    environment: &Environment,
    needs: CurrentNeeds,
    candidates: &[ActionCandidate],
    competing_indices: &[usize],
    developmental: Option<&DevelopmentalContext>,
    _ledger: &EnergyLedger,
) -> Vec<Option<f64>> {
    let Some(developmental) = developmental else {
        return vec![None; candidates.len()];
    };
    if needs.development <= 0.0 {
        return vec![None; candidates.len()];
    }

    let combine_score = if competing_indices.iter().any(|&index| {
        candidates
            .get(index)
            .is_some_and(|candidate| candidate.action == ActionKind::Combine)
    }) {
        combine_developmental_preview_score(organism, environment, developmental)
    } else {
        None
    };

    candidates
        .iter()
        .enumerate()
        .map(|(index, candidate)| {
            if !competing_indices.contains(&index) {
                return None;
            }
            match candidate.action {
                ActionKind::Combine => combine_score,
                ActionKind::NoTransaction => Some(developmental.current_growth_fraction),
                ActionKind::Break => {
                    let bond_index = candidate
                        .context_key
                        .as_deref()
                        .and_then(|key| key.strip_prefix("bond:"))
                        .and_then(|index| index.parse::<usize>().ok())?;
                    Some(
                        developmental
                            .blueprint
                            .realization_after_break_with_components(
                                &organism.structure,
                                &environment.catalog,
                                developmental.origin,
                                developmental.orientation,
                                developmental.preferred_length,
                                bond_index,
                                developmental.current_material_realized,
                                developmental.current_density_realized,
                            )?
                            .overall
                            .clamp(0.0, 1.0),
                    )
                }
                _ => None,
            }
        })
        .collect()
}
