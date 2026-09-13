use crate::combine::experimental_interaction;
use crate::contact::ConnectionCandidate;
use crate::resources::ResourceProperties;
use crate::state::{EnergyLedger, Environment, Organism};
use crate::structure::{Bond, OrganismStructure};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BreakEvaluation {
    pub(crate) net_energy: f64,
    pub(crate) bond_energy: f64,
    pub(crate) interaction_energy: f64,
    pub(crate) work: f64,
}

pub(crate) fn water_field_amount(environment: &Environment, position: &crate::state::Position) -> f64 {
    environment
        .field
        .index_for_position(position.x, position.y)
        .map(|index| {
            environment.field.cells[index]
                .materials
                .iter()
                .flat_map(|material| material.parts.iter())
                .filter(|(name, _)| name == "Water")
                .map(|(_, amount)| *amount)
                .sum()
        })
        .unwrap_or(0.0)
}

pub(crate) fn break_work_cost(
    a: ResourceProperties,
    b: ResourceProperties,
    complexity: f64,
) -> f64 {
    crate::combine::bond_strength(a, b) * complexity.max(0.0)
}

pub(crate) fn evaluate_bond_break(
    structure: &OrganismStructure,
    target: Bond,
    a: ResourceProperties,
    b: ResourceProperties,
    candidate: ConnectionCandidate,
    water: f64,
    complexity: f64,
) -> Option<BreakEvaluation> {
    if !target.bond_energy.is_finite() || target.bond_energy < 0.0 {
        return None;
    }

    let interaction = experimental_interaction(a, b, candidate, water);
    let interaction_energy = -interaction.signed_value;
    let work = break_work_cost(a, b, complexity);
    if !interaction_energy.is_finite() || !work.is_finite() || work < 0.0 {
        return None;
    }

    let net_energy = target.bond_energy + interaction_energy - work;
    if !net_energy.is_finite() {
        return None;
    }

    if !structure.bonds.iter().any(|bond| bond.has_same_identity(&target)) {
        return None;
    }

    Some(BreakEvaluation {
        net_energy,
        bond_energy: target.bond_energy,
        interaction_energy,
        work,
    })
}

pub(crate) fn execute_break(
    structure: &mut OrganismStructure,
    target: Bond,
    available_energy: &mut f64,
    evaluation: BreakEvaluation,
    ledger: &mut EnergyLedger,
) -> bool {
    if !available_energy.is_finite() || evaluation.net_energy.is_nan() {
        return false;
    }
    if evaluation.net_energy < 0.0
        && *available_energy + f64::EPSILON < -evaluation.net_energy
    {
        return false;
    }
    if structure.break_matching_bond(target).is_none() {
        return false;
    }

    *available_energy += evaluation.net_energy;
    if evaluation.work > 0.0 {
        ledger.total_heat_dissipated += evaluation.work;
    }
    if evaluation.bond_energy > 0.0 {
        ledger.total_potential_energy_released += evaluation.bond_energy;
    }
    if evaluation.interaction_energy > 0.0 {
        ledger.total_potential_energy_released += evaluation.interaction_energy;
    }
    if evaluation.net_energy > 0.0 {
        ledger.total_usable_energy_gained += evaluation.net_energy;
    }
    true
}

pub(crate) fn evaluate_organism_bond_break(
    organism: &Organism,
    environment: &Environment,
    target: Bond,
    complexity: f64,
) -> Option<BreakEvaluation> {
    let ia = organism.structure.unit_index(target.endpoint_a.constituent_id)?;
    let ib = organism.structure.unit_index(target.endpoint_b.constituent_id)?;
    let a = organism.structure.units.get(ia)?.properties(&environment.catalog)?;
    let b = organism.structure.units.get(ib)?.properties(&environment.catalog)?;
    let candidate = crate::contact::connection_pair_candidates(
        &organism.structure,
        ia,
        ib,
        &environment.catalog,
    )
    .into_iter()
    .find(|candidate| {
        candidate.endpoint_a == target.endpoint_a.location
            && candidate.endpoint_b == target.endpoint_b.location
    })?;

    evaluate_bond_break(
        &organism.structure,
        target,
        a,
        b,
        candidate,
        water_field_amount(
            environment,
            organism.occupied_cells.first().unwrap_or(&crate::state::Position { x: 0.0, y: 0.0 }),
        ),
        complexity,
    )
}

pub(crate) fn resolve_stress_break(
    organism: &mut Organism,
    environment: &Environment,
    ledger: &mut EnergyLedger,
) -> bool {
    let Some((_, target)) = organism
        .structure
        .bonds
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            a.strength
                .partial_cmp(&b.strength)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    else {
        return false;
    };
    let target = *target;
    let Some(evaluation) = evaluate_organism_bond_break(
        organism,
        environment,
        target,
        crate::math::complexity(2.0),
    ) else {
        return false;
    };

    let worked = evaluation.work;
    if !execute_break(
        &mut organism.structure,
        target,
        &mut organism.usable_energy,
        evaluation,
        ledger,
    ) {
        return false;
    }
    organism.add_transaction_stress(worked);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn break_net_energy_is_represented_by_evaluation() {
        let structure = OrganismStructure::new();
        let target = Bond {
            endpoint_a: crate::structure::BondEndpoint::new(crate::structure::PhysicalConstituentId(0), crate::contact::ConnectionPointLocation::Center),
            endpoint_b: crate::structure::BondEndpoint::new(crate::structure::PhysicalConstituentId(1), crate::contact::ConnectionPointLocation::Center),
            strength: 1.0,
            bond_energy: 10.0,
        };
        assert!(evaluate_bond_break(
            &structure,
            target,
            ResourceProperties { mass: 1.0, potential_energy: 1.0, reactivity: 1.0, cohesion: 1.0 },
            ResourceProperties { mass: 1.0, potential_energy: 1.0, reactivity: 1.0, cohesion: 1.0 },
            ConnectionCandidate::default(),
            0.0,
            2.0,
        ).is_none());
    }
}
