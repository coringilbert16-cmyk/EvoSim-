//! Local runtime chemical reaction accumulation.

use crate::chemistry_library::{ChemistryKey, ChemistryLibrary};
use crate::contact::ConnectionCompatibilityCache;
use crate::resources::BaseResource;
use crate::state::Organism;
use crate::structure::Bond;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ChemicalBreakOperation {
    pub(crate) organism_id: String,
    pub(crate) bond: Bond,
    pub(crate) activation_signal: f64,
    pub(crate) disruption_cost: f64,
    pub(crate) reaction_frequency_hz: f64,
    /// Preserve accumulated progress if settlement fails without changing the
    /// physical structure (for example, because the event became stale).
    pub(crate) accumulation_key: String,
    pub(crate) accumulated_reaction: f64,
}

/// Hard cap on physical reaction-realization attempts per simulation tick.
/// Candidate discovery and activation remain separate from this commit budget.
pub(crate) const MAX_REACTION_REALIZATIONS_PER_TICK: usize = 64;
/// Hard cap on structural bond candidates inspected by chemistry per tick.
/// This bounds discovery work independently of the realization/commit budget.
pub(crate) const MAX_REACTION_CANDIDATE_INSPECTIONS_PER_TICK: usize = 256;

/// Select a bounded, tick-rotated window over the owner's stable bond list.
/// Rotating the start prevents a large structure's tail from being permanently
/// invisible when the global discovery budget is smaller than its bond count.
fn discovery_indices(candidate_count: usize, limit: usize, tick: u64) -> Vec<usize> {
    if candidate_count == 0 || limit == 0 {
        return Vec::new();
    }
    let count = candidate_count.min(limit);
    let start = tick as usize % candidate_count;
    (0..count)
        .map(|offset| start.wrapping_add(offset) % candidate_count)
        .collect()
}

/// Divide the global per-tick realization budget fairly among owners. Any
/// remainder rotates by tick, including when there are more owners than slots.
pub(crate) fn work_budget_for_owner(
    total_budget: usize,
    owner_count: usize,
    owner_index: usize,
    tick: u64,
) -> usize {
    if owner_count == 0 || owner_index >= owner_count {
        return 0;
    }
    let base = total_budget / owner_count;
    let remainder = total_budget % owner_count;
    let rotation = tick as usize % owner_count;
    let rank = (owner_index + owner_count - rotation) % owner_count;
    base + if rank < remainder { 1 } else { 0 }
}

/// Add only work left unused by earlier owners to this owner's fair base share.
/// The caller carries the returned remainder forward, keeping total attempts
/// within the global cap without wasting slots on owners with no ready work.
pub(crate) fn work_budget_for_owner_with_carry(
    total_budget: usize,
    owner_count: usize,
    owner_index: usize,
    tick: u64,
    carried_work: usize,
) -> usize {
    work_budget_for_owner(total_budget, owner_count, owner_index, tick).saturating_add(carried_work)
}

/// Select work in stable key order, independent of the order in which callers
/// discovered candidates. Ties retain their original index for deterministic
/// behavior when multiple records share a key.
fn stable_work_order(keys: &[String], limit: usize, tick: u64) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..keys.len()).collect();
    indices.sort_by(|left, right| keys[*left].cmp(&keys[*right]).then_with(|| left.cmp(right)));
    // Rotate deterministically by tick so a permanently rejected early key
    // cannot starve later candidates whenever the work budget is saturated.
    if !indices.is_empty() {
        let rotation = tick as usize % indices.len();
        indices.rotate_left(rotation);
    }
    indices.truncate(limit);
    indices
}

/// Apply the per-tick realization budget. Activated operations that do not fit
/// are returned to the progress map without changing any physical or energy
/// state; they are revalidated by normal candidate discovery on a later tick.
pub(crate) fn schedule_operations(
    operations: Vec<ChemicalBreakOperation>,
    tick: u64,
    remaining_work: &mut usize,
    accumulation: &mut std::collections::BTreeMap<String, f64>,
) -> Vec<ChemicalBreakOperation> {
    if operations.is_empty() || *remaining_work == 0 {
        for operation in operations {
            accumulation
                .entry(operation.accumulation_key)
                .and_modify(|current| {
                    *current = (*current).max(operation.accumulated_reaction);
                })
                .or_insert(operation.accumulated_reaction);
        }
        return Vec::new();
    }

    let keys: Vec<String> = operations
        .iter()
        .map(|operation| operation.accumulation_key.clone())
        .collect();
    let order = stable_work_order(&keys, *remaining_work, tick);
    let mut selected = vec![false; operations.len()];
    for index in &order {
        selected[*index] = true;
    }

    let mut ready = Vec::with_capacity(order.len());
    for (index, operation) in operations.into_iter().enumerate() {
        if selected[index] {
            ready.push((keys[index].clone(), operation));
        } else {
            accumulation
                .entry(operation.accumulation_key)
                .and_modify(|current| {
                    *current = (*current).max(operation.accumulated_reaction);
                })
                .or_insert(operation.accumulated_reaction);
        }
    }
    ready.sort_by(|left, right| left.0.cmp(&right.0));
    *remaining_work = remaining_work.saturating_sub(ready.len());
    ready.into_iter().map(|(_, operation)| operation).collect()
}

fn reaction_key(bond: &Bond, interface_signature: &str) -> String {
    format!(
        "{:?}|{:?}|{}",
        bond.endpoint_a, bond.endpoint_b, interface_signature
    )
}

fn material_identity(material: &crate::resources::Material) -> String {
    // Preserve the established base-resource keys used by the checked-in
    // chemistry cache. A one-part, one-unit material is still identified by
    // its resource name; amounts and composition remain explicit for mixtures
    // and non-unit quantities.
    if material.parts.len() == 1
        && crate::chemistry_library::quantized_amount(material.parts[0].1) == 1_000_000_000
    {
        return material.parts[0].0.clone();
    }

    let mut parts = material.parts.clone();
    parts.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
    });
    parts
        .into_iter()
        .map(|(name, amount)| {
            format!(
                "{}@{}",
                name,
                crate::chemistry_library::quantized_amount(amount)
            )
        })
        .collect::<Vec<_>>()
        .join("+")
}

fn dissipate_unupdated_accumulations(
    accumulation: &mut std::collections::BTreeMap<String, f64>,
    current_prefix: &str,
    updated_keys: &std::collections::BTreeSet<String>,
) {
    accumulation.retain(|key, value| {
        if !key.starts_with(current_prefix) || updated_keys.contains(key) {
            return true;
        }
        *value *= 1.0 - crate::chemistry::CHEMICAL_DISSIPATION;
        value.is_finite() && *value > 0.0
    });
}

pub(crate) fn accumulate(
    organism: &Organism,
    catalog: &[BaseResource],
    accumulation: &mut std::collections::BTreeMap<String, f64>,
    chemistry_library: &mut ChemistryLibrary,
    tick: u64,
    remaining_discovery_work: &mut usize,
) -> Vec<ChemicalBreakOperation> {
    if organism.structure.bonds.is_empty() {
        let organism_prefix = format!("{}|rev:", organism.id);
        accumulation.retain(|key, _| !key.starts_with(&organism_prefix));
        return Vec::new();
    }
    let current_prefix = format!("{}|rev:{}|", organism.id, organism.structure_revision);
    accumulation.retain(|key, _| {
        !key.starts_with(&format!("{}|rev:", organism.id)) || key.starts_with(&current_prefix)
    });
    let mut cache = ConnectionCompatibilityCache::new();
    let baselines = crate::resources::ResourceBaselines::from_catalog(catalog);
    let bond_count = organism.structure.bonds.len();
    let mut operations = Vec::new();
    let mut updated_keys = std::collections::BTreeSet::new();
    let indices = discovery_indices(bond_count, *remaining_discovery_work, tick);
    *remaining_discovery_work = remaining_discovery_work.saturating_sub(indices.len());

    for index in indices {
        let Some(bond) = organism.structure.bonds.get(index).copied() else {
            continue;
        };
        let Some(unit_a) = organism
            .structure
            .unit_index(bond.endpoint_a.constituent_id)
        else {
            continue;
        };
        let Some(unit_b) = organism
            .structure
            .unit_index(bond.endpoint_b.constituent_id)
        else {
            continue;
        };
        let Some(a) = organism
            .structure
            .units
            .get(unit_a)
            .and_then(|u| u.properties(catalog))
        else {
            continue;
        };
        let Some(b) = organism
            .structure
            .units
            .get(unit_b)
            .and_then(|u| u.properties(catalog))
        else {
            continue;
        };
        let Some(position_a) = a.chemical_position else {
            continue;
        };
        let Some(position_b) = b.chemical_position else {
            continue;
        };

        let candidate = crate::contact::connection_pair_candidates_cached(
            &organism.structure,
            unit_a,
            unit_b,
            catalog,
            &mut cache,
        )
        .into_iter()
        .find(|candidate| {
            (candidate.endpoint_a == bond.endpoint_a.location
                && candidate.endpoint_b == bond.endpoint_b.location)
                || (candidate.endpoint_a == bond.endpoint_b.location
                    && candidate.endpoint_b == bond.endpoint_a.location)
        });
        let Some(candidate) = candidate else {
            continue;
        };

        let geometry_material_a = organism.structure.units[unit_a]
            .material
            .parts
            .first()
            .map(|part| part.0.as_str())
            .unwrap_or("unknown");
        let geometry_material_b = organism.structure.units[unit_b]
            .material
            .parts
            .first()
            .map(|part| part.0.as_str())
            .unwrap_or("unknown");
        let material_a = material_identity(&organism.structure.units[unit_a].material);
        let material_b = material_identity(&organism.structure.units[unit_b].material);
        let Some(interface) = crate::geometry_reference_library::resolve_live_contact_candidate(
            geometry_material_a,
            &organism.structure.units[unit_a],
            geometry_material_b,
            &organism.structure.units[unit_b],
            candidate,
            catalog,
        ) else {
            continue;
        };
        let key = format!(
            "{}|rev:{}|{}",
            organism.id,
            organism.structure_revision,
            reaction_key(&bond, &interface.signature)
        );

        let key_record = ChemistryKey::from_live_geometry(
            &material_a,
            &material_b,
            &interface,
            position_a,
            position_b,
        );
        let potential = match chemistry_library.get(&key_record) {
            Some(record) => record.static_potential,
            None => {
                let Some(calculated) = crate::chemistry::interaction_potential(
                    position_a,
                    position_b,
                    crate::chemistry::CHEMICAL_K,
                    crate::chemistry::CHEMICAL_D_MAX,
                ) else {
                    continue;
                };
                match chemistry_library.get_or_insert_static_potential(key_record, calculated) {
                    Ok(Some(value)) => value,
                    Ok(None) | Err(_) => calculated,
                }
            }
        };
        let Some(contact) = crate::chemistry::contact_factor(
            candidate.distance,
            crate::chemistry::CHEMICAL_CONTACT_RADIUS,
        ) else {
            continue;
        };
        let Some(engagement) = crate::chemistry::interface_engagement(candidate.facing) else {
            continue;
        };
        let previous = accumulation.get(&key).copied().unwrap_or(0.0);
        let Some(next) = crate::chemistry::accumulate_reaction(
            previous,
            potential,
            contact,
            engagement,
            crate::chemistry::CHEMICAL_DISSIPATION,
        ) else {
            continue;
        };

        if next <= 0.0 {
            updated_keys.insert(key.clone());
            accumulation.remove(&key);
            continue;
        }
        let Some(barrier) = crate::chemistry::activation_barrier_from_bond_strength(bond.strength)
        else {
            continue;
        };

        let frequency_a = crate::harmonics::natural_frequency_hz(a, baselines);
        let frequency_b = crate::harmonics::natural_frequency_hz(b, baselines);
        if !frequency_a.is_finite()
            || frequency_a <= 0.0
            || !frequency_b.is_finite()
            || frequency_b <= 0.0
        {
            continue;
        }
        // The interface reaction couples to the logarithmic midpoint of
        // its two realized materials' natural frequencies.
        let reaction_frequency_hz = (0.5 * frequency_a.ln() + 0.5 * frequency_b.ln()).exp();
        if !reaction_frequency_hz.is_finite() || reaction_frequency_hz <= 0.0 {
            continue;
        }

        // Compatible retained resonance lowers the remaining activation
        // requirement. It is not copied into the accumulator: resolve() will
        // withdraw the same energy only if the bond transaction succeeds.
        let resonance_credit = organism
            .resonant_energy
            .compatible_energy(reaction_frequency_hz)
            .min(barrier);
        let effective_barrier = (barrier - resonance_credit).max(0.0);
        let Some(activation_signal) =
            crate::chemistry::normalized_chemistry_to_activation_signal(next)
        else {
            continue;
        };
        updated_keys.insert(key.clone());
        if crate::chemistry::activated(activation_signal, effective_barrier) {
            operations.push(ChemicalBreakOperation {
                organism_id: organism.id.clone(),
                bond,
                activation_signal,
                disruption_cost: barrier,
                reaction_frequency_hz,
                accumulation_key: key.clone(),
                accumulated_reaction: next,
            });
            accumulation.remove(&key);
        } else {
            accumulation.insert(key, next);
        }
    }
    // Reaction progress loses energy every tick even when a previously known
    // interface cannot be resolved this tick. Otherwise missing geometry or
    // invalid chemistry data accidentally pauses dissipation indefinitely.
    dissipate_unupdated_accumulations(accumulation, &current_prefix, &updated_keys);
    operations
}

pub(crate) fn resolve(
    operation: ChemicalBreakOperation,
    organism: &mut Organism,
    ledger: &mut crate::state::EnergyLedger,
) -> bool {
    if organism.id != operation.organism_id {
        return false;
    }
    if organism.resonant_energy.total_energy().is_none() {
        return false;
    }
    if !operation.activation_signal.is_finite()
        || operation.activation_signal < 0.0
        || !operation.disruption_cost.is_finite()
        || operation.disruption_cost < 0.0
    {
        return false;
    }
    let processing_efficiency = organism.genome.processing_efficiency();
    // The activation signal is a threshold, not an energy source. The actual
    // transaction separately accounts for stored bond energy, usable energy,
    // and any resonance withdrawn from the trial reservoir.
    let required_resonance = (operation.disruption_cost - operation.activation_signal).max(0.0);
    // Work from a trial reservoir. A failed or stale BREAK must not consume
    // retained resonance; only a successfully settled structural transaction
    // commits the frequency-compatible withdrawal.
    let mut trial_reservoir = organism.resonant_energy.clone();
    let resonant_work =
        trial_reservoir.withdraw_compatible(operation.reaction_frequency_hz, required_resonance);
    let success = crate::transformation::settle_chemical_break_energy_with_resonance(
        organism,
        operation.bond,
        operation.activation_signal,
        operation.disruption_cost,
        resonant_work,
        processing_efficiency,
        ledger,
    );
    if success {
        organism.resonant_energy = trial_reservoir;
    }
    // A failed event is stale or physically invalid. Its accumulation was
    // already removed when the event crossed the barrier, and the next
    // structure revision will invalidate any remaining interface state.
    success
}

/// Resolve a triggered reaction while preserving its accumulated progress
/// if settlement fails. A later structure revision naturally invalidates the
/// old key through the normal revision-pruning path.
pub(crate) fn resolve_preserving_accumulation(
    operation: ChemicalBreakOperation,
    organism: &mut Organism,
    ledger: &mut crate::state::EnergyLedger,
    accumulation: &mut std::collections::BTreeMap<String, f64>,
) -> bool {
    let key = operation.accumulation_key.clone();
    let accumulated_reaction = operation.accumulated_reaction;
    let success = resolve(operation, organism, ledger);
    if !success {
        accumulation.insert(key, accumulated_reaction);
    }
    success
}

#[cfg(test)]
mod material_identity_tests {
    use super::*;
    use crate::resources::Material;

    #[test]
    fn discovery_budget_is_bounded_and_rotates_across_bond_list() {
        assert_eq!(discovery_indices(0, 10, 0), Vec::<usize>::new());
        assert_eq!(discovery_indices(5, 0, 0), Vec::<usize>::new());
        assert_eq!(discovery_indices(5, 2, 0), vec![0, 1]);
        assert_eq!(discovery_indices(5, 2, 1), vec![1, 2]);
        assert_eq!(discovery_indices(5, 2, 4), vec![4, 0]);
        assert_eq!(discovery_indices(5, 99, 3), vec![3, 4, 0, 1, 2]);
    }

    #[test]
    fn unobserved_reaction_progress_dissipates_once_per_tick() {
        let mut accumulation = std::collections::BTreeMap::from([
            ("organism|rev:4|bond-a".to_string(), 2.0),
            ("organism|rev:4|bond-b".to_string(), 1.0),
            ("other|rev:4|bond-c".to_string(), 3.0),
        ]);
        let updated = std::collections::BTreeSet::from(["organism|rev:4|bond-a".to_string()]);

        dissipate_unupdated_accumulations(&mut accumulation, "organism|rev:4|", &updated);

        assert_eq!(accumulation.get("organism|rev:4|bond-a"), Some(&2.0));
        assert!((accumulation["organism|rev:4|bond-b"] - 0.9).abs() < 1e-12);
        assert_eq!(accumulation.get("other|rev:4|bond-c"), Some(&3.0));
    }

    #[test]
    fn one_unit_base_material_keeps_legacy_resource_name() {
        assert_eq!(
            material_identity(&Material::free_base("Carbon", 1.0)),
            "Carbon"
        );
    }

    #[test]
    fn composite_and_non_unit_material_identities_keep_amounts() {
        assert_eq!(
            material_identity(&Material::free_base("Carbon", 2.0)),
            "Carbon@2000000000"
        );
        let mixed = Material {
            parts: vec![("Hydrogen".into(), 1.0), ("Carbon".into(), 1.0)],
            internal_bonds: Vec::new(),
        };
        assert_eq!(
            material_identity(&mixed),
            "Carbon@1000000000+Hydrogen@1000000000"
        );
    }
    #[test]
    fn successful_chemical_break_does_not_spend_resonance_when_reaction_covers_barrier() {
        let mut simulation = crate::state::Simulation::new(920, 10.0);
        let bond = *simulation.organisms[0]
            .structure
            .bonds
            .first()
            .expect("seed organism should contain a structural bond");
        assert!(simulation.organisms[0].resonant_energy.deposit(440.0, 1.0));
        let operation = ChemicalBreakOperation {
            organism_id: simulation.organisms[0].id.clone(),
            bond,
            activation_signal: 1_000.0,
            disruption_cost: 0.5,
            reaction_frequency_hz: 440.0,
            accumulation_key: "test|rev:0|bond".into(),
            accumulated_reaction: 2.5,
        };
        let before = simulation.organisms[0]
            .resonant_energy
            .total_energy()
            .unwrap();
        assert!(resolve(
            operation,
            &mut simulation.organisms[0],
            &mut simulation.energy_ledger,
        ));
        let after = simulation.organisms[0]
            .resonant_energy
            .total_energy()
            .unwrap();
        assert!((before - after).abs() < 1e-9);
        assert!(!simulation.organisms[0]
            .structure
            .bonds
            .iter()
            .any(|candidate| candidate.has_same_identity(&bond)));
    }

    #[test]
    fn successful_chemical_break_spends_only_resonance_needed_for_reaction_shortfall() {
        let mut simulation = crate::state::Simulation::new(924, 10.0);
        let bond = *simulation.organisms[0]
            .structure
            .bonds
            .first()
            .expect("seed organism should contain a structural bond");
        assert!(simulation.organisms[0].resonant_energy.deposit(440.0, 1.0));
        let operation = ChemicalBreakOperation {
            organism_id: simulation.organisms[0].id.clone(),
            bond,
            activation_signal: 0.25,
            disruption_cost: 0.5,
            reaction_frequency_hz: 440.0,
            accumulation_key: "test|rev:0|bond".into(),
            accumulated_reaction: 0.25,
        };
        let before = simulation.organisms[0]
            .resonant_energy
            .total_energy()
            .unwrap();
        assert!(resolve(
            operation,
            &mut simulation.organisms[0],
            &mut simulation.energy_ledger,
        ));
        let after = simulation.organisms[0]
            .resonant_energy
            .total_energy()
            .unwrap();
        assert!((before - after - 0.25).abs() < 1e-9);
        assert!(!simulation.organisms[0]
            .structure
            .bonds
            .iter()
            .any(|candidate| candidate.has_same_identity(&bond)));
    }

    #[test]
    fn failed_chemical_settlement_preserves_triggered_reaction_progress() {
        let mut simulation = crate::state::Simulation::new(921, 10.0);
        let bond = *simulation.organisms[0]
            .structure
            .bonds
            .first()
            .expect("seed organism should contain a structural bond");
        let original_bond_count = simulation.organisms[0].structure.bonds.len();
        let operation = ChemicalBreakOperation {
            organism_id: simulation.organisms[0].id.clone(),
            bond,
            // Insufficient physical energy forces settlement rejection after
            // activation, without changing the physical structure.
            activation_signal: 0.1,
            disruption_cost: 0.5,
            reaction_frequency_hz: 440.0,
            accumulation_key: "test|rev:0|bond".into(),
            accumulated_reaction: 7.25,
        };
        let mut accumulation = std::collections::BTreeMap::new();
        assert!(!resolve_preserving_accumulation(
            operation,
            &mut simulation.organisms[0],
            &mut simulation.energy_ledger,
            &mut accumulation,
        ));
        assert_eq!(
            simulation.organisms[0].structure.bonds.len(),
            original_bond_count
        );
        assert_eq!(accumulation.get("test|rev:0|bond"), Some(&7.25));
    }

    #[test]
    fn large_activation_signal_cannot_pay_a_physical_energy_shortfall() {
        let mut simulation = crate::state::Simulation::new(925, 10.0);
        let bond = *simulation.organisms[0]
            .structure
            .bonds
            .first()
            .expect("seed organism should contain a structural bond");
        let original_bond_count = simulation.organisms[0].structure.bonds.len();
        let original_ledger = simulation.energy_ledger;
        simulation.organisms[0].usable_energy = 0.0;
        simulation.organisms[0].resonant_energy =
            crate::harmonics::ResonantEnergyReservoir::default();
        let original_resonance = simulation.organisms[0]
            .resonant_energy
            .total_energy()
            .expect("empty resonance reservoir must have a valid total");
        let operation = ChemicalBreakOperation {
            organism_id: simulation.organisms[0].id.clone(),
            bond,
            // The signal easily crosses the activation threshold, but the
            // physical bond and energy holders cannot pay the requested work.
            activation_signal: 1_000_000.0,
            disruption_cost: bond.bond_energy + 1.0,
            reaction_frequency_hz: 440.0,
            accumulation_key: "test|rev:0|bond".into(),
            accumulated_reaction: 1_000_000.0,
        };

        assert!(!resolve(
            operation,
            &mut simulation.organisms[0],
            &mut simulation.energy_ledger,
        ));
        assert_eq!(
            simulation.organisms[0].structure.bonds.len(),
            original_bond_count
        );
        assert_eq!(simulation.energy_ledger, original_ledger);
        assert_eq!(simulation.organisms[0].usable_energy, 0.0);
        assert_eq!(
            simulation.organisms[0].resonant_energy.total_energy(),
            Some(original_resonance)
        );
    }

    #[test]
    fn chemical_break_reconciles_world_energy_inventory() {
        let mut simulation = crate::state::Simulation::new(922, 10.0);
        let before = simulation.energy_inventory_snapshot();
        assert!(
            before.is_balanced(1e-9),
            "initial inventory: {:?}",
            before.residual()
        );
        let bond = *simulation.organisms[0]
            .structure
            .bonds
            .first()
            .expect("seed organism should contain a structural bond");
        let operation = ChemicalBreakOperation {
            organism_id: simulation.organisms[0].id.clone(),
            bond,
            activation_signal: 1_000.0,
            disruption_cost: 0.0,
            reaction_frequency_hz: 440.0,
            accumulation_key: "test|rev:0|bond".into(),
            accumulated_reaction: 2.5,
        };
        assert!(resolve(
            operation,
            &mut simulation.organisms[0],
            &mut simulation.energy_ledger,
        ));
        let after = simulation.energy_inventory_snapshot();
        assert!(
            after.is_balanced(1e-9),
            "chemical BREAK must reconcile the world inventory: residual={:?}",
            after.residual()
        );
        assert!(!simulation.organisms[0]
            .structure
            .bonds
            .iter()
            .any(|candidate| candidate.has_same_identity(&bond)));
    }

    #[test]
    fn chemical_break_with_resonance_shortfall_reconciles_world_inventory() {
        let mut simulation = crate::state::Simulation::new(925, 10.0);
        assert!(simulation.organisms[0].resonant_energy.deposit(440.0, 1.0));
        let retained_resonance = simulation.organisms[0]
            .resonant_energy
            .total_energy()
            .expect("retained resonance should have finite energy");
        // The fixture deposit is part of the test's starting state, not a
        // simulated world-tone input. Set the oracle's tick-zero baseline to
        // that explicitly seeded reservoir so only the BREAK is measured.
        simulation.energy_ledger.initial_resonant_energy_held = retained_resonance;
        simulation.energy_ledger.total_resonant_energy_held = retained_resonance;
        let before = simulation.energy_inventory_snapshot();
        assert!(
            before.is_balanced(1e-9),
            "initial inventory: {:?}",
            before.residual()
        );

        let bond = *simulation.organisms[0]
            .structure
            .bonds
            .first()
            .expect("seed organism should contain a structural bond");
        let operation = ChemicalBreakOperation {
            organism_id: simulation.organisms[0].id.clone(),
            bond,
            activation_signal: 0.25,
            disruption_cost: 0.5,
            reaction_frequency_hz: 440.0,
            accumulation_key: "test|rev:0|bond".into(),
            accumulated_reaction: 0.25,
        };
        assert!(resolve(
            operation,
            &mut simulation.organisms[0],
            &mut simulation.energy_ledger,
        ));

        let after = simulation.energy_inventory_snapshot();
        assert!(
            after.is_balanced(1e-9),
            "resonance-assisted chemical BREAK must reconcile the world inventory: residual={:?}",
            after.residual()
        );
        assert!(
            (before.current_resonant_energy - after.current_resonant_energy - 0.25).abs() < 1e-9,
            "only the reaction-energy shortfall should be withdrawn from resonance"
        );
        assert!(!simulation.organisms[0]
            .structure
            .bonds
            .iter()
            .any(|candidate| candidate.has_same_identity(&bond)));
    }

    #[test]
    fn failed_stale_chemical_break_does_not_consume_retained_resonance() {
        let mut simulation = crate::state::Simulation::new(921, 10.0);
        let bond = *simulation.organisms[0]
            .structure
            .bonds
            .first()
            .expect("seed organism should contain a structural bond");
        assert!(simulation.organisms[0].resonant_energy.deposit(440.0, 1.0));
        let operation = ChemicalBreakOperation {
            organism_id: simulation.organisms[0].id.clone(),
            bond,
            activation_signal: 1_000.0,
            disruption_cost: 0.5,
            reaction_frequency_hz: 440.0,
            accumulation_key: "test|rev:0|bond".into(),
            accumulated_reaction: 2.5,
        };
        assert!(simulation.organisms[0]
            .structure
            .break_matching_bond(bond)
            .is_some());
        let before = simulation.organisms[0]
            .resonant_energy
            .total_energy()
            .unwrap();
        assert!(!resolve(
            operation,
            &mut simulation.organisms[0],
            &mut simulation.energy_ledger,
        ));
        assert!(
            (simulation.organisms[0]
                .resonant_energy
                .total_energy()
                .unwrap()
                - before)
                .abs()
                < 1e-12
        );
    }

    #[test]
    fn rejected_chemical_break_preserves_bond_and_resonance_when_ledger_is_invalid() {
        let mut simulation = crate::state::Simulation::new(923, 10.0);
        let bond = *simulation.organisms[0]
            .structure
            .bonds
            .first()
            .expect("seed organism should contain a structural bond");
        assert!(
            bond.bond_energy.is_finite() && bond.bond_energy > 0.0,
            "fixture bond must have positive finite stored energy"
        );
        let disruption_cost = bond.bond_energy * 0.5;
        let activation_signal = disruption_cost * 0.5;
        assert!(simulation.organisms[0]
            .resonant_energy
            .deposit(440.0, disruption_cost));
        let resonance_before = simulation.organisms[0]
            .resonant_energy
            .total_energy()
            .expect("resonance must be finite");
        // A non-finite cumulative sink makes ledger settlement reject after
        // candidate preparation but before any physical or reservoir commit.
        simulation.energy_ledger.total_heat_dissipated = f64::NAN;
        let structure_before = simulation.organisms[0].structure.clone();
        let usable_before = simulation.organisms[0].usable_energy;

        let operation = ChemicalBreakOperation {
            organism_id: simulation.organisms[0].id.clone(),
            bond,
            activation_signal,
            disruption_cost,
            reaction_frequency_hz: 440.0,
            accumulation_key: "test|rev:0|bond".into(),
            accumulated_reaction: activation_signal,
        };
        assert!(
            !resolve(
                operation,
                &mut simulation.organisms[0],
                &mut simulation.energy_ledger,
            ),
            "invalid cumulative heat state must reject the BREAK transaction"
        );
        assert_eq!(
            simulation.organisms[0].structure.units,
            structure_before.units
        );
        assert_eq!(
            simulation.organisms[0].structure.bonds,
            structure_before.bonds
        );
        assert_eq!(
            simulation.organisms[0]
                .resonant_energy
                .total_energy()
                .unwrap(),
            resonance_before
        );
        assert_eq!(simulation.organisms[0].usable_energy, usable_before);
        assert!(simulation.energy_ledger.total_heat_dissipated.is_nan());
        assert_eq!(simulation.energy_ledger.total_work_expended, 0.0);
    }

    #[test]
    fn bondless_structure_discards_prior_revision_reaction_accumulation() {
        let mut simulation = crate::state::Simulation::new(924, 10.0);
        let organism = &mut simulation.organisms[0];
        let stale_key = format!(
            "{}|rev:{}|stale-reaction",
            organism.id, organism.structure_revision
        );
        let unrelated_key = "other-organism|rev:3|unrelated".to_string();
        simulation
            .chemical_reaction_accumulation
            .insert(stale_key.clone(), 0.75);
        organism.structure.bonds.clear();
        organism.mark_structure_changed();
        let current_revision_key = format!(
            "{}|rev:{}|stale-current-revision",
            organism.id, organism.structure_revision
        );
        simulation
            .chemical_reaction_accumulation
            .insert(current_revision_key.clone(), 0.5);
        simulation
            .chemical_reaction_accumulation
            .insert(unrelated_key.clone(), 0.25);
        let mut discovery_budget = MAX_REACTION_CANDIDATE_INSPECTIONS_PER_TICK;
        assert!(accumulate(
            organism,
            &simulation.environment.catalog,
            &mut simulation.chemical_reaction_accumulation,
            &mut simulation.chemistry_library,
            simulation.tick,
            &mut discovery_budget,
        )
        .is_empty());

        assert!(
            !simulation
                .chemical_reaction_accumulation
                .contains_key(&stale_key),
            "old-revision accumulation must be removed even when no bonds remain"
        );
        assert!(
            !simulation
                .chemical_reaction_accumulation
                .contains_key(&current_revision_key),
            "bondless organisms must not retain current-revision reaction entries"
        );
        assert_eq!(
            simulation
                .chemical_reaction_accumulation
                .get(&unrelated_key),
            Some(&0.25)
        );
    }

    #[test]
    fn scheduler_orders_work_by_stable_key_and_respects_budget() {
        let keys = vec!["zeta".into(), "alpha".into(), "middle".into()];
        assert_eq!(stable_work_order(&keys, 2, 0), vec![1, 2]);
        assert_eq!(stable_work_order(&keys, 0, 0), Vec::<usize>::new());
        assert_eq!(stable_work_order(&keys, 8, 0), vec![1, 2, 0]);
    }

    #[test]
    fn scheduler_breaks_duplicate_key_ties_by_original_index() {
        let keys = vec!["same".into(), "same".into(), "first".into()];
        assert_eq!(stable_work_order(&keys, 2, 0), vec![2, 0]);
    }

    #[test]
    fn per_owner_budget_rotates_remainder_and_large_populations() {
        let first_tick: Vec<usize> = (0..3)
            .map(|index| work_budget_for_owner(64, 3, index, 0))
            .collect();
        let next_tick: Vec<usize> = (0..3)
            .map(|index| work_budget_for_owner(64, 3, index, 1))
            .collect();
        assert_eq!(first_tick, vec![22, 21, 21]);
        assert_eq!(next_tick, vec![21, 22, 21]);
        assert_eq!(first_tick.iter().sum::<usize>(), 64);

        let first_slots: Vec<usize> = (0..100)
            .map(|index| work_budget_for_owner(64, 100, index, 0))
            .collect();
        let next_slots: Vec<usize> = (0..100)
            .map(|index| work_budget_for_owner(64, 100, index, 1))
            .collect();
        assert_eq!(first_slots.iter().sum::<usize>(), 64);
        assert_eq!(next_slots.iter().sum::<usize>(), 64);
        assert_eq!(first_slots[0], 1);
        assert_eq!(first_slots[64], 0);
        assert_eq!(next_slots[0], 0);
        assert_eq!(next_slots[64], 1);
    }

    #[test]
    fn scheduler_carries_unused_owner_budget_forward_within_global_cap() {
        let simulation = crate::state::Simulation::new(926, 10.0);
        let bond = *simulation.organisms[0]
            .structure
            .bonds
            .first()
            .expect("seed organism should contain a structural bond");
        let operation = |owner, candidate| ChemicalBreakOperation {
            organism_id: format!("owner-{owner}"),
            bond,
            activation_signal: 1.0,
            disruption_cost: 0.5,
            reaction_frequency_hz: 440.0,
            accumulation_key: format!("owner-{owner}|candidate-{candidate}"),
            accumulated_reaction: 1.0,
        };
        let candidate_counts = [0, 10, 100];
        let mut carry = 0;
        let mut realized_attempts = 0;
        let mut accumulation = std::collections::BTreeMap::new();

        for (owner, count) in candidate_counts.into_iter().enumerate() {
            let mut remaining = work_budget_for_owner_with_carry(
                MAX_REACTION_REALIZATIONS_PER_TICK,
                candidate_counts.len(),
                owner,
                0,
                carry,
            );
            let operations = (0..count)
                .map(|candidate| operation(owner, candidate))
                .collect();
            let ready = schedule_operations(operations, 0, &mut remaining, &mut accumulation);
            realized_attempts += ready.len();
            carry = remaining;
        }

        assert_eq!(realized_attempts, MAX_REACTION_REALIZATIONS_PER_TICK);
        assert_eq!(carry, 0);
        assert_eq!(accumulation.len(), 46);
    }

    #[test]
    fn scheduler_rotation_prevents_permanent_first_key_starvation() {
        let keys = vec!["zeta".into(), "alpha".into(), "middle".into()];
        assert_eq!(stable_work_order(&keys, 2, 0), vec![1, 2]);
        assert_eq!(stable_work_order(&keys, 2, 1), vec![2, 0]);
    }

    #[test]
    fn scheduler_defers_over_budget_activation_without_losing_progress() {
        let simulation = crate::state::Simulation::new(925, 10.0);
        let bond = *simulation.organisms[0]
            .structure
            .bonds
            .first()
            .expect("seed organism should contain a structural bond");
        let operation = |key: &str, progress: f64| ChemicalBreakOperation {
            organism_id: simulation.organisms[0].id.clone(),
            bond,
            activation_signal: progress,
            disruption_cost: 0.5,
            reaction_frequency_hz: 440.0,
            accumulation_key: key.into(),
            accumulated_reaction: progress,
        };
        let mut accumulation = std::collections::BTreeMap::new();
        let mut remaining = 1;
        let ready = schedule_operations(
            vec![operation("z-last", 7.25), operation("a-first", 2.5)],
            0,
            &mut remaining,
            &mut accumulation,
        );

        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].accumulation_key, "a-first");
        assert_eq!(remaining, 0);
        assert_eq!(accumulation.get("z-last"), Some(&7.25));

        let deferred = schedule_operations(
            vec![operation("b-next-tick", 3.75)],
            1,
            &mut remaining,
            &mut accumulation,
        );
        assert!(deferred.is_empty());
        assert_eq!(accumulation.get("b-next-tick"), Some(&3.75));
    }

    #[test]
    fn base_contact_key_matches_checked_in_cache_identity() {
        let carbon = Material::free_base("Carbon", 1.0);
        let hydrogen = Material::free_base("Hydrogen", 1.0);
        let interface = crate::geometry_reference_library::resolve_live_contact_interface(
            "Carbon",
            crate::structure::ConnectionEndpoint::Corner { point_index: 0 },
            "Hydrogen",
            crate::structure::ConnectionEndpoint::LineEndpoint { point_index: 0 },
        );
        let key = ChemistryKey::from_live_geometry(
            material_identity(&carbon),
            material_identity(&hydrogen),
            &interface,
            10.5,
            5.5,
        );
        assert_eq!(
            key.schema_version,
            crate::chemistry_library::CHEMISTRY_LIBRARY_SCHEMA_VERSION
        );
        assert_eq!(key.material_a, "Carbon");
        assert_eq!(key.material_b, "Hydrogen");
        assert_eq!(key.interface_class, "rigid_point");
        assert_eq!(
            key.interface_signature,
            "live-v1|Carbon:corner:0|Hydrogen:line:0"
        );
    }
}
