//! Deterministic construction of one valid juvenile realization.
//!
//! The genome owns architectural intent. This module asks the genome for a
//! developmental construction target, realizes it through the unified
//! construction runtime, and validates the resulting physical structure
//! against the juvenile viability contract.
use crate::genome::Genome;
use crate::juvenile_requirements::{validate_realized_juvenile, JuvenileViabilityRequirements};
use crate::resources::BaseResource;
use crate::state::EnergyLedger;
use crate::structural_blueprint::StructuralBlueprint;
use crate::structure::OrganismStructure;

pub(crate) const JUVENILE_INITIAL_ENERGY_RESERVE: f64 = 16.0;
const TRIAL_ENERGY: f64 = 1.0e12;
const EPS: f64 = 1e-8;

/// Realize the juvenile target generated from inherited architecture.
#[allow(dead_code)]
pub(crate) fn realize_initial_for_genome(
    genome: &Genome,
    catalog: &[BaseResource],
) -> Result<(OrganismStructure, EnergyLedger, f64), String> {
    let target = genome.developmental_construction_target(catalog, true)?;
    realize_initial_with_reserve(&target, catalog, genome.juvenile_energy_reserve)
}

/// Compatibility entry point for callers that already hold a concrete
/// developmental target. The target is still only a construction artifact;
/// physical structure remains authoritative after realization.
pub(crate) fn realize_initial(
    blueprint: &StructuralBlueprint,
    catalog: &[BaseResource],
) -> Result<(OrganismStructure, EnergyLedger, f64), String> {
    realize_initial_with_reserve(blueprint, catalog, JUVENILE_INITIAL_ENERGY_RESERVE)
}

pub(crate) fn realize_initial_with_reserve(
    blueprint: &StructuralBlueprint,
    catalog: &[BaseResource],
    reserve_energy: f64,
) -> Result<(OrganismStructure, EnergyLedger, f64), String> {
    if !blueprint.is_valid() {
        return Err("juvenile construction target is invalid".into());
    }
    if !reserve_energy.is_finite() || reserve_energy <= 0.0 {
        return Err("juvenile energy reserve must be finite and positive".into());
    }

    // First realization is a non-persistent energy requirement calculation.
    // The actual physical assembly is still performed by the same unified
    // blueprint/construction/COMBINE path used elsewhere.
    let mut trial_ledger = EnergyLedger::default();
    let mut trial_energy = TRIAL_ENERGY;
    blueprint
        .realize_with_context(catalog, &mut trial_ledger, &mut trial_energy)
        .map_err(|error| format!("juvenile construction target could not be realized: {error}"))?;
    let required_initial_energy = TRIAL_ENERGY - trial_energy;
    if !required_initial_energy.is_finite() || required_initial_energy < 0.0 {
        return Err("juvenile construction produced an invalid energy requirement".into());
    }

    let mut ledger = EnergyLedger::default();
    let mut energy = required_initial_energy + reserve_energy;
    let (structure, _) = blueprint
        .realize_with_context(catalog, &mut ledger, &mut energy)
        .map_err(|error| format!("juvenile construction target could not be realized: {error}"))?;

    if !energy.is_finite() || energy + EPS < reserve_energy {
        return Err(format!(
            "juvenile initialization could not preserve its reserve: remaining={energy}"
        ));
    }
    validate_realized_juvenile(
        &structure,
        catalog,
        JuvenileViabilityRequirements::default(),
    )?;
    Ok((structure, ledger, energy))
}
