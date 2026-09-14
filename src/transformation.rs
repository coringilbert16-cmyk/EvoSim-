use crate::decision::{ActionKind, OutcomeKind};
use crate::decision_runtime::ActionCandidate;
use crate::state::{ActiveTransformation, EnergyLedger, Environment, Organism, Simulation};
fn water_field_amount(environment: &Environment, organism: &Organism) -> f64 {
    organism
        .occupied_cells
        .first()
        .and_then(|p| environment.field.index_for_position(p.x, p.y))
        .map(|i| {
            environment.field.cells[i]
                .materials
                .iter()
                .flat_map(|m| m.parts.iter())
                .filter(|(n, _)| n == "Water")
                .map(|(_, a)| *a)
                .sum()
        })
        .unwrap_or(0.0)
}
fn break_net_energy(bond_energy: f64, interaction_energy: f64, work_cost: f64) -> Option<f64> {
    if !bond_energy.is_finite()
        || bond_energy < 0.0
        || !interaction_energy.is_finite()
        || !work_cost.is_finite()
        || work_cost < 0.0
    {
        return None;
    }
    let net = bond_energy + interaction_energy - work_cost;
    if net.is_finite() {
        Some(net)
    } else {
        None
    }
}
fn settle_break_energy(
    organism: &mut Organism,
    bond: crate::structure::Bond,
    break_interaction_energy: f64,
    net: f64,
    work: f64,
    ledger: &mut EnergyLedger,
) -> bool {
    if net < 0.0 && organism.usable_energy + f64::EPSILON < -net {
        return false;
    }
    if organism.structure.break_matching_bond(bond).is_none() {
        return false;
    }
    organism.add_transaction_stress(work);
    if work > 0.0 {
        ledger.total_heat_dissipated += work
    }
    organism.usable_energy += net;
    if bond.bond_energy > 0.0 {
        ledger.total_potential_energy_released += bond.bond_energy;
    }
    if break_interaction_energy > 0.0 {
        ledger.total_potential_energy_released += break_interaction_energy;
    }
    if net > 0.0 {
        ledger.total_usable_energy_gained += net;
    }
    true
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
    let Some(ia) = organism
        .structure
        .unit_index(target.endpoint_a.constituent_id)
    else {
        return false;
    };
    let Some(ib) = organism
        .structure
        .unit_index(target.endpoint_b.constituent_id)
    else {
        return false;
    };
    let Some(a) = organism
        .structure
        .units
        .get(ia)
        .and_then(|u| u.properties(&environment.catalog))
    else {
        return false;
    };
    let Some(b) = organism
        .structure
        .units
        .get(ib)
        .and_then(|u| u.properties(&environment.catalog))
    else {
        return false;
    };
    let work = break_work_cost(a, b, crate::math::complexity(2.0));
    if !work.is_finite() || work < 0.0 {
        return false;
    }
    let Some(candidate) = crate::contact::connection_pair_candidates(
        &organism.structure,
        ia,
        ib,
        &environment.catalog,
    )
    .into_iter()
    .find(|c| {
        c.endpoint_a == target.endpoint_a.location && c.endpoint_b == target.endpoint_b.location
    }) else {
        return false;
    };
    let formation_interaction = crate::combine::experimental_interaction(
        a,
        b,
        candidate,
        water_field_amount(environment, organism),
    );
    let break_interaction_energy = -formation_interaction.signed_value;
    let Some(net) = break_net_energy(target.bond_energy, break_interaction_energy, work) else {
        return false;
    };
    settle_break_energy(
        organism,
        target,
        break_interaction_energy,
        net,
        work,
        ledger,
    )
}
impl Simulation {
    pub(crate) fn try_start_transformation(
        organism: &mut Organism,
        catalog: &[crate::resources::BaseResource],
        next_id: &mut u64,
        decision: &ActionCandidate,
    ) -> Option<ActiveTransformation> {
        if decision.action != ActionKind::Break || organism.active_transformation_id.is_some() {
            return None;
        }
        let key = decision.context_key.as_deref()?;
        let index = key.strip_prefix("bond:")?.parse::<usize>().ok()?;
        let bond = *organism.structure.bonds.get(index)?;
        if !bond.bond_energy.is_finite() || bond.bond_energy < 0.0 {
            return None;
        }
        let ia = organism
            .structure
            .unit_index(bond.endpoint_a.constituent_id)?;
        let ib = organism
            .structure
            .unit_index(bond.endpoint_b.constituent_id)?;
        let a = organism.structure.units.get(ia)?.properties(catalog)?;
        let b = organism.structure.units.get(ib)?.properties(catalog)?;
        let complexity = crate::math::complexity(2.0);
        let _work = break_work_cost(a, b, complexity);
        let duration = 1_u64.max(complexity.ceil() as u64);
        let t = ActiveTransformation {
            id: *next_id,
            organism_id: organism.id.clone(),
            kind: crate::state::TransformationKind::Break,
            material: crate::resources::Material::free_base("", 0.0),
            bond: Some(bond),
            complexity,
            duration_ticks: duration,
            remaining_ticks: duration,
            decision_context_key: decision.context_key.clone(),
        };
        *next_id += 1;
        organism.active_transformation_id = Some(t.id);
        Some(t)
    }
    pub(crate) fn resolve_transformation(
        transformation: &ActiveTransformation,
        organism: &mut Organism,
        environment: &mut Environment,
        ledger: &mut EnergyLedger,
    ) {
        let Some(target) = transformation.bond else {
            organism.active_transformation_id = None;
            return;
        };
        let Some(ia) = organism
            .structure
            .unit_index(target.endpoint_a.constituent_id)
        else {
            organism.active_transformation_id = None;
            return;
        };
        let Some(ib) = organism
            .structure
            .unit_index(target.endpoint_b.constituent_id)
        else {
            organism.active_transformation_id = None;
            return;
        };
        if !organism
            .structure
            .bonds
            .iter()
            .any(|b| b.has_same_identity(&target))
        {
            organism.active_transformation_id = None;
            return;
        }
        let a = match organism.structure.units[ia].properties(&environment.catalog) {
            Some(x) => x,
            None => {
                organism.active_transformation_id = None;
                return;
            }
        };
        let b = match organism.structure.units[ib].properties(&environment.catalog) {
            Some(x) => x,
            None => {
                organism.active_transformation_id = None;
                return;
            }
        };
        let work = break_work_cost(a, b, transformation.complexity);
        if !work.is_finite() || work < 0.0 {
            organism.active_transformation_id = None;
            return;
        }
        let candidate = match crate::contact::connection_pair_candidates(
            &organism.structure,
            ia,
            ib,
            &environment.catalog,
        )
        .into_iter()
        .find(|c| {
            c.endpoint_a == target.endpoint_a.location && c.endpoint_b == target.endpoint_b.location
        }) {
            Some(x) => x,
            None => {
                organism.active_transformation_id = None;
                return;
            }
        };
        let formation_interaction = crate::combine::experimental_interaction(
            a,
            b,
            candidate,
            water_field_amount(environment, organism),
        );
        let break_interaction_energy = -formation_interaction.signed_value;
        let net = match break_net_energy(target.bond_energy, break_interaction_energy, work) {
            Some(x) => x,
            None => {
                organism.active_transformation_id = None;
                return;
            }
        };
        if net < 0.0 && organism.usable_energy + f64::EPSILON < -net {
            organism.active_transformation_id = None;
            let candidate = ActionCandidate {
                action: ActionKind::Break,
                context_key: transformation.decision_context_key.clone(),
            };
            crate::decision_runtime::record_outcome(
                &mut organism.decision_history,
                &candidate,
                OutcomeKind::Harmful,
            );
            return;
        }
        if organism.structure.break_matching_bond(target).is_none() {
            organism.active_transformation_id = None;
            return;
        }
        organism.add_transaction_stress(work);
        if work > 0.0 {
            ledger.total_heat_dissipated += work
        }
        organism.usable_energy += net;
        if target.bond_energy > 0.0 {
            ledger.total_potential_energy_released += target.bond_energy;
        }
        if break_interaction_energy > 0.0 {
            ledger.total_potential_energy_released += break_interaction_energy;
        }
        if net > 0.0 {
            ledger.total_usable_energy_gained += net;
        }
        organism.active_transformation_id = None;
        let outcome = if net > f64::EPSILON {
            OutcomeKind::Beneficial
        } else if net < -f64::EPSILON {
            OutcomeKind::Harmful
        } else {
            OutcomeKind::Neutral
        };
        let candidate = ActionCandidate {
            action: ActionKind::Break,
            context_key: transformation.decision_context_key.clone(),
        };
        crate::decision_runtime::record_outcome(
            &mut organism.decision_history,
            &candidate,
            outcome,
        );
        if net > 0.0 {
            let reinforcement = (net * organism.genome.memory_strength()).clamp(0.0, 1.0);
            let (x, y) = organism
                .occupied_cells
                .first()
                .map(|p| (p.x, p.y))
                .unwrap_or((0.0, 0.0));
            reinforce_memory_point(organism, x, y, reinforcement)
        }
    }
}
pub(crate) fn break_work_cost(
    a: crate::resources::ResourceProperties,
    b: crate::resources::ResourceProperties,
    complexity: f64,
) -> f64 {
    crate::combine::bond_strength(a, b) * complexity.max(0.0)
}
pub(crate) fn reinforce_memory_point(organism: &mut Organism, x: f64, y: f64, reinforcement: f64) {
    if let Some(point) = organism
        .memory
        .iter_mut()
        .find(|p| (p.x - x).abs() < f64::EPSILON && (p.y - y).abs() < f64::EPSILON)
    {
        point.strength = crate::math::clamp01(point.strength + reinforcement)
    } else {
        organism.memory.push(crate::state::MemoryPoint {
            x,
            y,
            strength: crate::math::clamp01(reinforcement),
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn break_net_energy_supports_all_three_signs() {
        assert!(break_net_energy(10.0, 5.0, 3.0).unwrap() > 0.0);
        assert_eq!(break_net_energy(10.0, 0.0, 10.0).unwrap(), 0.0);
        assert!(break_net_energy(10.0, -5.0, 6.0).unwrap() < 0.0)
    }
    #[test]
    fn invalid_break_energy_is_rejected() {
        assert!(break_net_energy(-1.0, 0.0, 1.0).is_none());
        assert!(break_net_energy(1.0, f64::NAN, 1.0).is_none());
        assert!(break_net_energy(1.0, 0.0, -1.0).is_none())
    }
}
