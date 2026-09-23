use crate::decision::{ActionKind, OutcomeKind};
use crate::decision_runtime::ActionCandidate;
use crate::energy_ledger::{EnergyLedgerAuthority, EnergyReason, EnergyTransaction};
use crate::state::{ActiveTransformation, EnergyLedger, Environment, Organism, Simulation};
use rand::Rng;
use rand_chacha::ChaCha8Rng;

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
    net.is_finite().then_some(net)
}

fn settle_break_energy(
    organism: &mut Organism,
    bond: crate::structure::Bond,
    break_interaction_energy: f64,
    work: f64,
    ledger: &mut EnergyLedger,
) -> bool {
    let mut trial_structure = organism.structure.clone();
    if trial_structure.break_matching_bond(bond).is_none() {
        return false;
    }
    let net = bond.bond_energy + break_interaction_energy - work;
    let tx = EnergyTransaction {
        reason: EnergyReason::Break,
        potential_released: break_interaction_energy.max(0.0),
        usable_delta: net,
        structural_delta: -bond.bond_energy,
        heat_dissipated: work + (-break_interaction_energy).max(0.0),
    };
    if !ledger.settle_transaction(&mut organism.usable_energy, tx) {
        return false;
    }
    organism.structure = trial_structure;
    organism.add_transaction_stress(work);
    true
}

fn stress_break_candidate_indices(organism: &Organism, environment: &Environment) -> Vec<usize> {
    let genome_bonds =
        crate::cavity::analyze_genome_cavity(&organism.structure, &environment.catalog)
            .ok()
            .flatten()
            .map(|cavity| cavity.boundary_bond_indices(&organism.structure))
            .unwrap_or_default();
    let candidates: Vec<usize> = organism
        .structure
        .bonds
        .iter()
        .enumerate()
        .filter_map(|(index, _)| (!genome_bonds.contains(&index)).then_some(index))
        .collect();
    candidates
}

pub(crate) fn resolve_stress_break(
    organism: &mut Organism,
    environment: &Environment,
    ledger: &mut EnergyLedger,
    rng: &mut ChaCha8Rng,
) -> bool {
    let candidate_indices = stress_break_candidate_indices(organism, environment);
    let Some(&target_index) = candidate_indices.get(rng.gen_range(0..candidate_indices.len()))
    else {
        return false;
    };
    let target = organism.structure.bonds[target_index];
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
    if net < 0.0 && organism.usable_energy + f64::EPSILON < -net {
        return false;
    }
    settle_break_energy(organism, target, break_interaction_energy, work, ledger)
}

pub(crate) fn break_candidate_is_executable(
    organism: &Organism,
    environment: &Environment,
    target: crate::structure::Bond,
) -> bool {
    if organism.active_transformation_id.is_some()
        || !target.bond_energy.is_finite()
        || target.bond_energy < 0.0
    {
        return false;
    }
    let Some(ia) = organism.structure.unit_index(target.endpoint_a.constituent_id) else {
        return false;
    };
    let Some(ib) = organism.structure.unit_index(target.endpoint_b.constituent_id) else {
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
    let interaction = crate::combine::experimental_interaction(
        a,
        b,
        candidate,
        water_field_amount(environment, organism),
    );
    let Some(net) = break_net_energy(target.bond_energy, -interaction.signed_value, work) else {
        return false;
    };
    net >= 0.0 || organism.usable_energy + f64::EPSILON >= -net
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
        let Some(net) = break_net_energy(target.bond_energy, break_interaction_energy, work) else {
            organism.active_transformation_id = None;
            return;
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
        if !settle_break_energy(organism, target, break_interaction_energy, work, ledger) {
            organism.active_transformation_id = None;
            return;
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
            let capacity =
                crate::cavity::analyze_genome_cavity(&organism.structure, &environment.catalog)
                    .ok()
                    .flatten()
                    .filter(|cavity| cavity.qualifies())
                    .map(|cavity| crate::memory::memory_capacity(&cavity));
            if let Some(capacity) = capacity {
                crate::memory::reinforce_memory_point(organism, x, y, reinforcement, capacity);
            } else {
                organism.memory.clear();
            }
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
    fn stress_break_candidates_exclude_genome_boundary_bonds() {
        let genome = crate::genome::initial_genome();
        let catalog = crate::resources::default_catalog();
        let blueprint = crate::juvenile::confirmed_seed_baseline(&catalog).unwrap();
        let (structure, _, _) = crate::juvenile::realize_initial(&blueprint, &catalog).unwrap();
        let organism = crate::state::Organism {
            id: "test".into(),
            developmental_origin: crate::state::Position { x: 0.0, y: 0.0 },
            developmental_orientation_radians: 0.0,
            occupied_cells: vec![crate::state::Position { x: 0.0, y: 0.0 }],
            genome,
            resource_sense: crate::state::ResourceSense {
                sensed_resources: Vec::new(),
                direction_x: 0.0,
                direction_y: 0.0,
                direction_strength: 0.0,
            },
            memory: Vec::new(),
            decision_history: crate::decision::DecisionHistory::default(),
            usable_energy: 1_000_000.0,
            stress: 0.0,
            stress_threshold: crate::state::INITIAL_STRESS_THRESHOLD,
            stored_material: crate::material_storage::MaterialStorage::default(),
            structure,
            development_stage: crate::state::DevelopmentStage::Juvenile,
            active_transformation_id: None,
            reproductive_construction: None,
        };
        let environment = crate::state::Environment {
            width: 1000.0,
            height: 1000.0,
            catalog: catalog.clone(),
            field: crate::environment::ActiveMaterialField::new(
                1000.0,
                1000.0,
                crate::environment::DEFAULT_CELL_SIZE,
            ),
            vents: Vec::new(),
        };
        let candidates = stress_break_candidate_indices(&organism, &environment);
        assert!(!candidates.is_empty());
        let genome_bonds = crate::cavity::analyze_genome_cavity(&organism.structure, &catalog)
            .unwrap()
            .unwrap()
            .boundary_bond_indices(&organism.structure);
        assert!(candidates.iter().all(|index| !genome_bonds.contains(index)));
    }

    #[test]
    fn invalid_break_energy_is_rejected() {
        assert!(break_net_energy(-1.0, 0.0, 1.0).is_none());
        assert!(break_net_energy(1.0, f64::NAN, 1.0).is_none());
        assert!(break_net_energy(1.0, 0.0, -1.0).is_none())
    }
}
