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
    pub(crate) reaction_frequency_cycles_per_day: f64,
    /// Preserve accumulated progress if settlement fails without changing the
    /// physical structure (for example, because the event became stale).
    pub(crate) accumulation_key: String,
    pub(crate) accumulated_reaction: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct EnvironmentalChemicalBreakOperation {
    pub(crate) cell_index: usize,
    pub(crate) material_index: usize,
    pub(crate) bond_index: usize,
    pub(crate) source_material: crate::physical_material::PhysicalMaterial,
    pub(crate) bond: crate::physical_material::PhysicalMaterialBond,
    pub(crate) activation_signal: f64,
    pub(crate) disruption_cost: f64,
    pub(crate) reaction_frequency_cycles_per_day: f64,
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
        let Some(disruption_cost) = crate::chemistry::activation_barrier_from_bond_properties(
            bond.strength,
            a.reactivity,
            b.reactivity,
            baselines.reactivity,
        ) else {
            continue;
        };

        let frequency_a = crate::harmonics::natural_frequency_cycles_per_day(a, baselines);
        let frequency_b = crate::harmonics::natural_frequency_cycles_per_day(b, baselines);
        if !frequency_a.is_finite()
            || frequency_a <= 0.0
            || !frequency_b.is_finite()
            || frequency_b <= 0.0
        {
            continue;
        }
        // The interface reaction couples to the logarithmic midpoint of
        // its two realized materials' natural frequencies.
        let reaction_frequency_cycles_per_day =
            (0.5 * frequency_a.ln() + 0.5 * frequency_b.ln()).exp();
        if !reaction_frequency_cycles_per_day.is_finite()
            || reaction_frequency_cycles_per_day <= 0.0
        {
            continue;
        }

        // Activation is an independent signal threshold. Compatible
        // resonance may fund disruption work at settlement, but it cannot
        // lower the activation threshold or be counted in the accumulator.
        let Some(activation_signal) =
            crate::chemistry::normalized_chemistry_to_activation_signal(next)
        else {
            continue;
        };
        updated_keys.insert(key.clone());
        if crate::chemistry::activated(activation_signal, disruption_cost) {
            operations.push(ChemicalBreakOperation {
                organism_id: organism.id.clone(),
                bond,
                activation_signal,
                disruption_cost,
                reaction_frequency_cycles_per_day,
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

/// Discover chemical reactions at realized environmental material bonds in the
/// field. Environmental bonds are restored only into a trial graph to recover
/// their authoritative geometry and activation strength; allocated bond energy
/// remains attached to the environmental bond and is transferred through restoration.
/// Traverse a bounded, tick-rotated window of environmental material bonds.
/// Returns candidates and the exact number of cell/material/bond entries inspected.
/// Counting all three levels makes the work bound testable even in sparse fields.
fn discover_environmental_bonds(
    environment: &crate::state::Environment,
    tick: u64,
    candidate_limit: usize,
) -> (Vec<(usize, usize, usize)>, usize) {
    let cell_count = environment.field.cells.len();
    let limit = candidate_limit.min(MAX_REACTION_CANDIDATE_INSPECTIONS_PER_TICK);
    let mut candidates = Vec::with_capacity(limit);
    if limit == 0 || cell_count == 0 {
        return (candidates, 0);
    }

    let mut scan_steps_remaining = limit.saturating_mul(8).max(1);
    let scan_steps_initial = scan_steps_remaining;
    // Advance by the previous tick's scan window rather than one cell. The
    // scan inspects up to `limit * 8` nested entries, so moving only one
    // cell per tick causes heavy overlap and makes distant sparse material
    // effectively invisible at small budgets. A window-sized stride lets
    // empty-field scans cover successive regions while preserving the same
    // hard per-tick inspection cap.
    let scan_window = scan_steps_initial.min(cell_count).max(1);
    // Reserve two inspection slots for the material and bond nested beneath a
    // cell. Without this headroom, a sparse bond at the end of a window can be
    // perpetually skipped: the cell itself consumes the final inspection.
    let scan_stride = scan_window.saturating_sub(2).max(1);
    let cell_start = ((tick % cell_count as u64) as usize).wrapping_mul(scan_stride) % cell_count;
    'discovery: for cell_offset in 0..cell_count {
        if scan_steps_remaining == 0 {
            break;
        }
        scan_steps_remaining -= 1;
        let cell_index = (cell_start + cell_offset) % cell_count;
        let Some(cell) = environment.field.cells.get(cell_index) else {
            continue;
        };
        let material_count = cell.physical_materials.len();
        if material_count == 0 {
            continue;
        }
        let material_start = tick as usize % material_count;
        for material_offset in 0..material_count {
            if scan_steps_remaining == 0 {
                break 'discovery;
            }
            scan_steps_remaining -= 1;
            let material_index = (material_start + material_offset) % material_count;
            let Some(material) = cell.physical_materials.get(material_index) else {
                continue;
            };
            let Some(connections) = material.internal_connections.as_ref() else {
                continue;
            };
            let bond_count = connections.len();
            if bond_count == 0 {
                continue;
            }
            let bond_start = tick as usize % bond_count;
            for bond_offset in 0..bond_count {
                if scan_steps_remaining == 0 {
                    break 'discovery;
                }
                scan_steps_remaining -= 1;
                let bond_index = (bond_start + bond_offset) % bond_count;
                candidates.push((cell_index, material_index, bond_index));
                if candidates.len() >= limit {
                    break 'discovery;
                }
            }
        }
    }

    (candidates, scan_steps_initial - scan_steps_remaining)
}

pub(crate) fn accumulate_environmental(
    environment: &crate::state::Environment,
    accumulation: &mut std::collections::BTreeMap<String, f64>,
    chemistry_library: &mut ChemistryLibrary,
    tick: u64,
    remaining_discovery_work: &mut usize,
) -> Vec<EnvironmentalChemicalBreakOperation> {
    // Environmental progress belongs to the field, not to the organism
    // whose turn happens to discover it. Search the field directly; the
    // bounded candidate budget below keeps this world-owned scan finite.
    const ENVIRONMENT_PREFIX: &str = "environment|environment:";
    let current_prefix = ENVIRONMENT_PREFIX.to_owned();
    if *remaining_discovery_work == 0 {
        dissipate_unupdated_accumulations(
            accumulation,
            &current_prefix,
            &std::collections::BTreeSet::new(),
        );
        return Vec::new();
    }
    let count = (*remaining_discovery_work).min(MAX_REACTION_CANDIDATE_INSPECTIONS_PER_TICK);
    let (candidates, _scan_steps) = discover_environmental_bonds(environment, tick, count);
    let indices: Vec<usize> = (0..candidates.len()).collect();
    *remaining_discovery_work = remaining_discovery_work.saturating_sub(indices.len());
    let mut updated_keys = std::collections::BTreeSet::new();
    let mut operations = Vec::new();
    let baselines = crate::resources::ResourceBaselines::from_catalog(&environment.catalog);
    let mut compatibility_cache = ConnectionCompatibilityCache::new();

    for candidate_index in indices {
        let Some(&(cell_index, material_index, bond_index)) = candidates.get(candidate_index)
        else {
            continue;
        };
        let Some(instance) = environment
            .field
            .cells
            .get(cell_index)
            .and_then(|cell| cell.physical_materials.get(material_index))
        else {
            continue;
        };
        let Some(connection) = instance
            .internal_connections
            .as_ref()
            .and_then(|connections| connections.get(bond_index))
        else {
            continue;
        };
        // Environmental chemistry must derive its candidate geometry from the
        // realized environmental material itself. An organism graph is not a
        // valid geometry owner for a world-owned bond: it can carry unrelated
        // units, IDs, and contact context into candidate resolution.
        let mut hypothetical = crate::structure::OrganismStructure::new();
        let Some(restored_indices) = crate::material_restoration::restore_material(
            &mut hypothetical,
            instance,
            crate::structure::Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
            &environment.catalog,
        ) else {
            continue;
        };
        let Some(&unit_a) = restored_indices.get(connection.part_a) else {
            continue;
        };
        let Some(&unit_b) = restored_indices.get(connection.part_b) else {
            continue;
        };
        let Some(a) = hypothetical
            .units
            .get(unit_a)
            .and_then(|unit| unit.properties(&environment.catalog))
        else {
            continue;
        };
        let Some(b) = hypothetical
            .units
            .get(unit_b)
            .and_then(|unit| unit.properties(&environment.catalog))
        else {
            continue;
        };
        let (Some(position_a), Some(position_b)) = (a.chemical_position, b.chemical_position)
        else {
            continue;
        };
        let Some(candidate) = crate::contact::connection_pair_candidates_cached(
            &hypothetical,
            unit_a,
            unit_b,
            &environment.catalog,
            &mut compatibility_cache,
        )
        .into_iter()
        .find(|candidate| {
            (candidate.endpoint_a == connection.endpoint_a
                && candidate.endpoint_b == connection.endpoint_b)
                || (candidate.endpoint_a == connection.endpoint_b
                    && candidate.endpoint_b == connection.endpoint_a)
        }) else {
            continue;
        };
        let geometry_material_a = hypothetical.units[unit_a]
            .material
            .parts
            .first()
            .map(|part| part.0.as_str())
            .unwrap_or("unknown");
        let geometry_material_b = hypothetical.units[unit_b]
            .material
            .parts
            .first()
            .map(|part| part.0.as_str())
            .unwrap_or("unknown");
        let Some(interface) = crate::geometry_reference_library::resolve_live_contact_candidate(
            geometry_material_a,
            &hypothetical.units[unit_a],
            geometry_material_b,
            &hypothetical.units[unit_b],
            candidate,
            &environment.catalog,
        ) else {
            continue;
        };
        let material_a = material_identity(&hypothetical.units[unit_a].material);
        let material_b = material_identity(&hypothetical.units[unit_b].material);
        let key = format!(
            "{}{}:{}:{}:{}:{:?}",
            ENVIRONMENT_PREFIX,
            cell_index,
            material_index,
            bond_index,
            material_identity(&instance.material),
            connection
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
        let Some(contact_factor) = crate::chemistry::contact_factor(
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
            contact_factor,
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
        let strength = crate::combine::bond_strength(a, b);
        let Some(disruption_cost) = crate::chemistry::activation_barrier_from_bond_properties(
            strength,
            a.reactivity,
            b.reactivity,
            baselines.reactivity,
        ) else {
            continue;
        };
        let frequency_a = crate::harmonics::natural_frequency_cycles_per_day(a, baselines);
        let frequency_b = crate::harmonics::natural_frequency_cycles_per_day(b, baselines);
        if !frequency_a.is_finite()
            || frequency_a <= 0.0
            || !frequency_b.is_finite()
            || frequency_b <= 0.0
        {
            continue;
        }
        let reaction_frequency_cycles_per_day =
            (0.5 * frequency_a.ln() + 0.5 * frequency_b.ln()).exp();
        if !reaction_frequency_cycles_per_day.is_finite()
            || reaction_frequency_cycles_per_day <= 0.0
        {
            continue;
        }
        let Some(activation_signal) =
            crate::chemistry::normalized_chemistry_to_activation_signal(next)
        else {
            continue;
        };
        // Invalid geometry/barrier candidates must not pause dissipation of
        // prior environmental progress. Mark a key current only after the
        // candidate completes a valid reaction evaluation.
        updated_keys.insert(key.clone());
        if crate::chemistry::activated(activation_signal, disruption_cost) {
            operations.push(EnvironmentalChemicalBreakOperation {
                cell_index,
                material_index,
                bond_index,
                source_material: instance.clone(),
                bond: connection.clone(),
                activation_signal,
                disruption_cost,
                reaction_frequency_cycles_per_day,
                accumulation_key: key,
                accumulated_reaction: next,
            });
            accumulation.remove(&operations.last().expect("just pushed").accumulation_key);
        } else {
            accumulation.insert(key, next);
        }
    }

    dissipate_unupdated_accumulations(accumulation, &current_prefix, &updated_keys);
    operations
}

/// Apply the same bounded, deterministic work policy to environmental reactions
/// after graph reactions have used their share of the owner's realization budget.
pub(crate) fn schedule_environmental_operations(
    operations: Vec<EnvironmentalChemicalBreakOperation>,
    tick: u64,
    remaining_work: &mut usize,
    accumulation: &mut std::collections::BTreeMap<String, f64>,
) -> Vec<EnvironmentalChemicalBreakOperation> {
    if operations.is_empty() || *remaining_work == 0 {
        for operation in operations {
            accumulation
                .entry(operation.accumulation_key)
                .and_modify(|current| *current = (*current).max(operation.accumulated_reaction))
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
    for index in order {
        selected[index] = true;
    }
    let mut ready = Vec::new();
    for (index, operation) in operations.into_iter().enumerate() {
        if selected[index] {
            ready.push((keys[index].clone(), operation));
        } else {
            accumulation
                .entry(operation.accumulation_key)
                .and_modify(|current| *current = (*current).max(operation.accumulated_reaction))
                .or_insert(operation.accumulated_reaction);
        }
    }
    ready.sort_by(|left, right| left.0.cmp(&right.0));
    *remaining_work = remaining_work.saturating_sub(ready.len());
    ready.into_iter().map(|(_, operation)| operation).collect()
}

pub(crate) fn resolve_environmental(
    operation: EnvironmentalChemicalBreakOperation,
    environment: &mut crate::state::Environment,
    ledger: &mut crate::state::EnergyLedger,
) -> bool {
    if !operation.activation_signal.is_finite()
        || operation.activation_signal < 0.0
        || !operation.disruption_cost.is_finite()
        || operation.disruption_cost < 0.0
        || !crate::chemistry::activated(operation.activation_signal, operation.disruption_cost)
        || !operation.reaction_frequency_cycles_per_day.is_finite()
        || operation.reaction_frequency_cycles_per_day <= 0.0
    {
        return false;
    }
    let Some(source) = environment
        .field
        .cells
        .get(operation.cell_index)
        .and_then(|cell| cell.physical_materials.get(operation.material_index))
    else {
        return false;
    };
    if source != &operation.source_material
        || source
            .internal_connections
            .as_ref()
            .and_then(|connections| connections.get(operation.bond_index))
            != Some(&operation.bond)
    {
        return false;
    }
    let Some(pieces) = source.break_internal_bond(&operation.bond) else {
        return false;
    };
    // Environmental chemistry may spend only energy that actually reached
    // the field through the world-tone path. Stage compatible resonance
    // withdrawal first; any remainder must be covered by already-accounted
    // usable energy. A failed BREAK leaves the live reservoir untouched.
    let mut staged_resonance = environment.resonant_energy.clone();
    let resonant_work = staged_resonance.withdraw_compatible(
        operation.reaction_frequency_cycles_per_day,
        operation.disruption_cost,
    );
    if !resonant_work.is_finite() || resonant_work < 0.0 {
        return false;
    }
    let Some(prepared) = crate::transformation::prepare_chemical_break_energy_with_resonance(
        environment.chemistry_usable_energy,
        ledger,
        operation.bond.bond_energy,
        operation.activation_signal,
        operation.disruption_cost,
        resonant_work,
    ) else {
        return false;
    };
    let usable_delta = prepared.usable_energy - environment.chemistry_usable_energy;
    let next_total_usable = ledger.total_usable_energy_held + usable_delta;
    // The BREAK operation removes this exact realized bond from the physical
    // source. Do not infer the live structural-store change by subtracting two
    // cumulative f64 counters: a small reaction delta can disappear after a
    // long history while the physical bond was still removed.
    let structural_delta = operation.bond.bond_energy;
    let next_total_structural = ledger.total_structural_energy_held + structural_delta;
    let next_total_resonant = ledger.total_resonant_energy_held - resonant_work;
    if !usable_delta.is_finite()
        || !next_total_usable.is_finite()
        || next_total_usable < 0.0
        || !structural_delta.is_finite()
        || !next_total_structural.is_finite()
        || !next_total_resonant.is_finite()
        || next_total_resonant < 0.0
    {
        return false;
    }
    if !environment.field.split_physical_at_index(
        operation.cell_index,
        operation.material_index,
        &operation.source_material,
        pieces,
        &environment.catalog,
    ) {
        return false;
    }
    environment.chemistry_usable_energy = prepared.usable_energy;
    environment.resonant_energy = staged_resonance;
    *ledger = prepared.ledger;
    ledger.total_usable_energy_held = next_total_usable;
    ledger.total_structural_energy_held = next_total_structural;
    ledger.total_resonant_energy_held = next_total_resonant;
    true
}

pub(crate) fn resolve_environmental_preserving_accumulation(
    operation: EnvironmentalChemicalBreakOperation,
    environment: &mut crate::state::Environment,
    ledger: &mut crate::state::EnergyLedger,
    accumulation: &mut std::collections::BTreeMap<String, f64>,
) -> bool {
    let key = operation.accumulation_key.clone();
    let accumulated_reaction = operation.accumulated_reaction;
    let success = resolve_environmental(operation, environment, ledger);
    if !success {
        accumulation.insert(key, accumulated_reaction);
    }
    success
}

pub(crate) fn resolve(
    operation: ChemicalBreakOperation,
    organism: &mut Organism,
    ledger: &mut crate::state::EnergyLedger,
) -> bool {
    if organism.id != operation.organism_id {
        return false;
    }
    if organism.resonant_energy.total_energy().is_none()
        || !operation.activation_signal.is_finite()
        || operation.activation_signal < 0.0
        || !operation.disruption_cost.is_finite()
        || operation.disruption_cost < 0.0
        || !crate::chemistry::activated(operation.activation_signal, operation.disruption_cost)
        || !operation.reaction_frequency_cycles_per_day.is_finite()
        || operation.reaction_frequency_cycles_per_day <= 0.0
    {
        return false;
    }
    // Activation is an independent threshold, not energy and not a substitute
    // for disruption work. Compatible resonance may pay that work, but the
    // required amount is determined by the physical barrier alone.
    let required_resonance = operation.disruption_cost;
    // Work from a trial reservoir. A failed or stale BREAK must not consume
    // retained resonance; only a successfully settled structural transaction
    // commits the frequency-compatible withdrawal.
    let mut trial_reservoir = organism.resonant_energy.clone();
    let resonant_work = trial_reservoir.withdraw_compatible(
        operation.reaction_frequency_cycles_per_day,
        required_resonance,
    );
    let success = crate::transformation::settle_chemical_break_energy_with_resonance(
        organism,
        operation.bond,
        operation.activation_signal,
        operation.disruption_cost,
        resonant_work,
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
    use crate::resources::{InternalBond, Material};

    fn environmental_test_composite(
        catalog: &[crate::resources::BaseResource],
    ) -> crate::physical_material::PhysicalMaterial {
        let material = Material {
            parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
            internal_bonds: vec![InternalBond {
                part_a: 0,
                part_b: 1,
            }],
        };
        let mut composite = crate::physical_material::PhysicalMaterial::realized(
            material,
            vec![
                crate::structure::Placement {
                    x: 12.5,
                    y: 12.5,
                    rotation_radians: 0.0,
                },
                crate::structure::Placement {
                    x: 14.0,
                    y: 12.5,
                    rotation_radians: 0.0,
                },
            ],
            catalog,
        )
        .expect("test composite should be physically realizable");
        // Use the exact facing endpoints: Carbon's rightmost corner and
        // Hydrogen's left longitudinal endpoint. The inferred candidate order
        // is not authoritative for this symmetric contact.
        composite.internal_connections =
            Some(vec![crate::physical_material::PhysicalMaterialBond {
                part_a: 0,
                endpoint_a: crate::structure::ConnectionEndpoint::Corner { point_index: 0 },
                part_b: 1,
                endpoint_b: crate::structure::ConnectionEndpoint::LineEndpoint { point_index: 0 },
                bond_energy: 0.0,
            }]);
        composite
    }

    fn remove_living_organisms_preserving_inventory(simulation: &mut crate::state::Simulation) {
        for organism in simulation.organisms.drain(..) {
            let resonant_energy = organism
                .resonant_energy
                .total_energy()
                .expect("test fixture resonance must be finite");
            assert!(
                resonant_energy.abs() <= 1e-9,
                "test fixture must not discard retained resonance"
            );
            let position = organism
                .occupied_cells
                .first()
                .cloned()
                .unwrap_or(crate::state::Position { x: 0.0, y: 0.0 });
            let body = crate::decomposition::DecomposingBody::new(
                organism.structure,
                organism.usable_energy,
                position,
            )
            .expect("test fixture energy must be finite and nonnegative");
            simulation.decomposing_bodies.push(body);
        }
    }

    fn fund_environment_chemistry_for_test(simulation: &mut crate::state::Simulation, amount: f64) {
        use crate::energy_ledger::{
            EnergyLedgerAuthority, ExternalEnergySource, ExternalInputTransaction,
        };

        let mut holder = 0.0;
        let transaction = ExternalInputTransaction {
            source: ExternalEnergySource::WorldTone,
            source_available: amount,
            external_input: amount,
            usable_delta: amount,
            work_expended: 0.0,
            heat_dissipated: 0.0,
        };
        assert!(simulation
            .energy_ledger
            .settle_external_input(&mut holder, transaction));
        simulation.energy_ledger.total_usable_energy_held += amount;
        simulation.environment.chemistry_usable_energy = holder;
    }

    #[test]
    fn sunlight_captured_by_environment_powers_chemical_break_without_energy_injection() {
        let mut simulation = crate::state::Simulation::new(932, 10.0);
        for cell in &mut simulation.environment.field.cells {
            cell.physical_materials.clear();
        }
        simulation.decomposing_bodies.clear();
        simulation.active_transformations.clear();

        let mut source = environmental_test_composite(&simulation.environment.catalog);
        if let Some(placements) = &mut source.placements {
            for placement in placements {
                placement.y = 0.0;
            }
        }
        let bond = source
            .internal_connections
            .as_ref()
            .and_then(|connections| connections.first())
            .expect("test composite should contain one internal bond")
            .clone();
        let cell_index = simulation
            .environment
            .field
            .index_for_position(12.5, 0.0)
            .expect("source placement should be in the field");
        assert!(simulation.environment.field.deposit_physical_at_index(
            cell_index,
            source.clone(),
            &simulation.environment.catalog,
        ));

        let sample = crate::world_tone::WorldToneSample {
            source_x: -5.0,
            source_y: 0.0,
            frequency_cycles_per_day: 440.0,
            energy_intensity_per_tick_per_width: 10.0,
            orbital_phase: 0.75,
        };
        crate::world_tone_snapshot::WorldToneOccluderSnapshot::settle_living_organism_inputs(
            &mut simulation,
            sample,
            &std::collections::HashSet::new(),
        );
        let captured = simulation
            .environment
            .resonant_energy
            .total_energy()
            .expect("sunlight capture should leave a valid environmental spectrum");
        assert!(
            captured > f64::EPSILON,
            "sunlight must deliver a positive amount of environmental resonance"
        );
        assert!(
            simulation.energy_inventory_snapshot().is_balanced(1e-7),
            "sunlight capture must balance before chemistry spends the energy"
        );

        let reaction_frequency_cycles_per_day = simulation
            .environment
            .resonant_energy
            .components
            .iter()
            .max_by(|left, right| left.energy.total_cmp(&right.energy))
            .map(|component| component.frequency_cycles_per_day)
            .expect("sunlight must leave at least one resonant component");
        let compatible_energy = simulation
            .environment
            .resonant_energy
            .compatible_energy(reaction_frequency_cycles_per_day);
        let disruption_cost = 0.5_f64.min(compatible_energy * 0.5);
        assert!(
            disruption_cost > f64::EPSILON,
            "captured resonance must supply positive compatible reaction work"
        );
        let work_before = simulation.energy_ledger.total_work_expended;
        let operation = EnvironmentalChemicalBreakOperation {
            cell_index,
            material_index: 0,
            bond_index: 0,
            source_material: source.clone(),
            bond,
            activation_signal: 1.0,
            disruption_cost,
            reaction_frequency_cycles_per_day,
            accumulation_key: "sunlight-powered-environmental-break".into(),
            accumulated_reaction: 1.0,
        };
        assert!(resolve_environmental(
            operation,
            &mut simulation.environment,
            &mut simulation.energy_ledger,
        ));
        let remaining = simulation
            .environment
            .resonant_energy
            .total_energy()
            .expect("environment resonance must remain valid after BREAK");
        assert!((captured - remaining - disruption_cost).abs() < 1e-7);
        assert!(
            (simulation.energy_ledger.total_work_expended - work_before - disruption_cost).abs()
                < 1e-7
        );
        assert!(
            simulation.energy_inventory_snapshot().is_balanced(1e-7),
            "sunlight-captured resonance must be spent once and remain conserved"
        );
    }

    #[test]
    fn bonded_environmental_carbon_hydrogen_can_reach_natural_break_activation() {
        let mut simulation = crate::state::Simulation::new(940, 10.0);
        for cell in &mut simulation.environment.field.cells {
            cell.physical_materials.clear();
        }
        let source = environmental_test_composite(&simulation.environment.catalog);
        let cell_index = simulation
            .environment
            .field
            .index_for_position(12.5, 12.5)
            .expect("bonded test material must be inside the field");
        assert!(simulation.environment.field.deposit_physical_at_index(
            cell_index,
            source,
            &simulation.environment.catalog,
        ));

        let mut accumulation = std::collections::BTreeMap::new();
        let mut max_accumulated = 0.0_f64;
        let mut ready = false;
        for tick in 0..2_000_u64 {
            let mut discovery_work = MAX_REACTION_CANDIDATE_INSPECTIONS_PER_TICK;
            let operations = accumulate_environmental(
                &simulation.environment,
                &mut accumulation,
                &mut simulation.chemistry_library,
                tick,
                &mut discovery_work,
            );
            max_accumulated =
                max_accumulated.max(accumulation.values().copied().fold(0.0_f64, f64::max));
            if !operations.is_empty() {
                ready = true;
                break;
            }
        }
        assert!(
            ready,
            "a realized Carbon-Hydrogen environmental bond never reached BREAK activation; max_accumulated={max_accumulated:.9}, remaining_pairs={}",
            accumulation.len()
        );
    }

    #[test]
    fn environmental_discovery_work_is_bounded_at_each_budget_size() {
        let mut simulation = crate::state::Simulation::new(938, 10.0);
        for cell in &mut simulation.environment.field.cells {
            cell.physical_materials.clear();
        }

        // Empty cells are the worst case for candidate-only accounting: no
        // candidate is found, but traversal must still stop at the scan bound.
        for budget in [1_usize, 8, 64, MAX_REACTION_CANDIDATE_INSPECTIONS_PER_TICK] {
            let (candidates, inspected) =
                discover_environmental_bonds(&simulation.environment, 0, budget);
            assert!(candidates.len() <= budget);
            assert!(
                inspected <= budget.saturating_mul(8).max(1),
                "empty-field traversal exceeded the documented 8x bound: budget={budget}, inspected={inspected}"
            );
            assert!(candidates.is_empty());
        }

        // A sparse candidate at the far end must remain reachable across
        // rotated windows while every individual tick stays within its bound.
        let source = environmental_test_composite(&simulation.environment.catalog);
        let last_cell = simulation.environment.field.cells.len() - 1;
        assert!(simulation.environment.field.deposit_physical_at_index(
            last_cell,
            source,
            &simulation.environment.catalog,
        ));
        let budget = 8;
        let mut reached = false;
        for tick in 0..simulation.environment.field.cells.len() as u64 + 8 {
            let (candidates, inspected) =
                discover_environmental_bonds(&simulation.environment, tick, budget);
            assert!(candidates.len() <= budget);
            assert!(inspected <= budget * 8);
            if candidates.iter().any(|(cell, _, _)| *cell == last_cell) {
                reached = true;
                break;
            }
        }
        assert!(
            reached,
            "rotated bounded scans must eventually reach sparse material"
        );
    }

    /// Manual wall-clock benchmark for environmental candidate discovery.
    /// Run with:
    /// cargo test environmental_discovery_timing_benchmark -- --ignored --nocapture
    #[test]
    #[ignore = "manual timing benchmark; run with --ignored --nocapture"]
    fn environmental_discovery_timing_benchmark() {
        use std::time::Instant;

        let mut simulation = crate::state::Simulation::new(939, 10.0);
        for cell in &mut simulation.environment.field.cells {
            cell.physical_materials.clear();
        }
        let source = environmental_test_composite(&simulation.environment.catalog);
        let last_cell = simulation.environment.field.cells.len() - 1;
        assert!(simulation.environment.field.deposit_physical_at_index(
            last_cell,
            source,
            &simulation.environment.catalog,
        ));

        for budget in [1_usize, 16, MAX_REACTION_CANDIDATE_INSPECTIONS_PER_TICK] {
            let start = Instant::now();
            let mut candidates_seen = 0_usize;
            let mut inspected_total = 0_usize;
            let mut first_candidate_tick = None;
            // Allow a full field traversal at the smallest budget. The timing
            // benchmark now checks discovery progress as well as bounded work.
            let ticks = simulation.environment.field.cells.len() as u64 + 8;
            for tick in 0..ticks {
                let (candidates, inspected) =
                    discover_environmental_bonds(&simulation.environment, tick, budget);
                if !candidates.is_empty() && first_candidate_tick.is_none() {
                    first_candidate_tick = Some(tick);
                }
                candidates_seen += candidates.len();
                inspected_total += inspected;
            }
            let elapsed = start.elapsed();
            eprintln!(
                "environmental discovery benchmark: budget={budget}, ticks={ticks}, candidates={candidates_seen}, first_candidate_tick={first_candidate_tick:?}, inspected={inspected_total}, elapsed_ms={}",
                elapsed.as_millis()
            );
            assert!(
                inspected_total <= ticks as usize * budget.saturating_mul(8).max(1),
                "aggregate discovery work exceeded the per-tick bound"
            );
            assert!(
                first_candidate_tick.is_some(),
                "distant sparse material must be discoverable at budget={budget}"
            );
        }
    }

    #[test]
    fn sparse_environmental_discovery_rotates_with_a_bounded_scan_window() {
        let mut simulation = crate::state::Simulation::new(937, 10.0);
        for cell in &mut simulation.environment.field.cells {
            cell.physical_materials.clear();
        }
        let source = environmental_test_composite(&simulation.environment.catalog);
        // Choose a known-late field index directly. A hard-coded world-space
        // position can map into an early cell when field dimensions or cell size
        // change, making this fairness regression fail before discovery runs.
        let cell_index = simulation
            .environment
            .field
            .cells
            .len()
            .checked_sub(1)
            .expect("the environmental field must contain cells");
        assert!(
            cell_index > 8,
            "fixture requires a field larger than the first bounded scan window"
        );
        assert!(simulation.environment.field.deposit_physical_at_index(
            cell_index,
            source,
            &simulation.environment.catalog,
        ));

        let target_prefix = format!("environment|environment:{cell_index}:");
        let mut accumulation = std::collections::BTreeMap::new();
        let mut found = false;
        let max_ticks = cell_index as u64 + 8;
        for tick in 0..max_ticks {
            let mut discovery_budget = 1;
            let operations = accumulate_environmental(
                &simulation.environment,
                &mut accumulation,
                &mut simulation.chemistry_library,
                tick,
                &mut discovery_budget,
            );
            if !operations.is_empty()
                || discovery_budget == 0
                || accumulation
                    .keys()
                    .any(|key| key.starts_with(&target_prefix))
            {
                found = true;
                break;
            }
            assert_eq!(
                discovery_budget, 1,
                "empty-cell traversal must not consume the candidate budget"
            );
        }
        assert!(
            found,
            "rotating bounded traversal must eventually reach sparse environmental material"
        );
    }

    #[test]
    fn sunlight_resonance_funds_environmental_chemical_break_without_external_grant() {
        let mut simulation = crate::state::Simulation::new(936, 10.0);
        for cell in &mut simulation.environment.field.cells {
            cell.physical_materials.clear();
        }
        let source = environmental_test_composite(&simulation.environment.catalog);
        let bond = source
            .internal_connections
            .as_ref()
            .and_then(|connections| connections.first())
            .expect("test composite should contain one internal bond")
            .clone();
        let cell_index = simulation
            .environment
            .field
            .index_for_position(12.5, 12.5)
            .expect("source placement should be in the field");
        assert!(simulation.environment.field.deposit_physical_at_index(
            cell_index,
            source.clone(),
            &simulation.environment.catalog,
        ));

        let sample = crate::world_tone::WorldToneSample {
            source_x: -5.0,
            source_y: 12.5,
            frequency_cycles_per_day: 440.0,
            energy_intensity_per_tick_per_width: 10.0,
            orbital_phase: 0.75,
        };
        let external_before = simulation.energy_ledger.total_external_energy_input;
        crate::world_tone_snapshot::WorldToneOccluderSnapshot::settle_living_organism_inputs(
            &mut simulation,
            sample,
            &std::collections::HashSet::new(),
        );

        let compatible = simulation
            .environment
            .resonant_energy
            .compatible_energy(440.0);
        assert!(
            compatible > 0.1,
            "the sunlight-exposed environmental material must retain enough compatible resonance to fund work; retained={compatible}"
        );
        assert!(
            simulation.energy_ledger.total_external_energy_input > external_before,
            "energy must enter through the existing world-tone source"
        );
        assert!(
            simulation.energy_inventory_snapshot().is_balanced(1e-7),
            "sunlight capture must reconcile before the reaction"
        );

        let operation = EnvironmentalChemicalBreakOperation {
            cell_index,
            material_index: 0,
            bond_index: 0,
            source_material: source,
            bond,
            activation_signal: 1.0,
            disruption_cost: 0.1,
            reaction_frequency_cycles_per_day: 440.0,
            accumulation_key: "sunlight-funded-environmental-break".into(),
            accumulated_reaction: 1.0,
        };
        let resonance_before = simulation
            .environment
            .resonant_energy
            .total_energy()
            .expect("environment resonance must be valid");
        let work_before = simulation.energy_ledger.total_work_expended;
        assert!(resolve_environmental(
            operation,
            &mut simulation.environment,
            &mut simulation.energy_ledger,
        ));

        let resonance_after = simulation
            .environment
            .resonant_energy
            .total_energy()
            .expect("environment resonance must remain valid");
        assert!(resonance_after < resonance_before);
        assert!(
            (simulation.energy_ledger.total_work_expended - work_before - 0.1).abs() <= 1e-9,
            "the reaction must charge its full disruption work exactly once"
        );
        assert!(
            simulation.energy_inventory_snapshot().is_balanced(1e-7),
            "sunlight capture followed by environmental BREAK must conserve the world inventory"
        );
    }

    #[test]
    fn environmental_chemical_break_charges_work_and_splits_material_atomically() {
        let mut simulation = crate::state::Simulation::new(933, 10.0);
        let source = environmental_test_composite(&simulation.environment.catalog);
        let bond = source
            .internal_connections
            .as_ref()
            .and_then(|connections| connections.first())
            .expect("test composite should contain one internal bond")
            .clone();
        let cell_index = simulation
            .environment
            .field
            .index_for_position(12.5, 12.5)
            .expect("source placement should be in the field");
        assert!(simulation.environment.field.deposit_physical_at_index(
            cell_index,
            source.clone(),
            &simulation.environment.catalog,
        ));

        remove_living_organisms_preserving_inventory(&mut simulation);
        fund_environment_chemistry_for_test(&mut simulation, 1.0);
        let energy_before = simulation.environment.chemistry_usable_energy;
        let ledger_before = simulation.energy_ledger.total_work_expended;
        assert!(
            simulation.energy_inventory_snapshot().is_balanced(1e-9),
            "environment energy input must reconcile before BREAK"
        );
        let operation = EnvironmentalChemicalBreakOperation {
            cell_index,
            material_index: 0,
            bond_index: 0,
            source_material: source.clone(),
            bond,
            activation_signal: 1.0,
            disruption_cost: 0.5,
            reaction_frequency_cycles_per_day: 440.0,
            accumulation_key: "environmental-atomic-break-success".into(),
            accumulated_reaction: 1.0,
        };

        assert!(simulation.organisms.is_empty());
        assert!(resolve_environmental(
            operation,
            &mut simulation.environment,
            &mut simulation.energy_ledger,
        ));

        let pieces = &simulation.environment.field.cells[cell_index].physical_materials;
        assert_eq!(pieces.len(), 2);
        assert!(pieces.iter().all(|piece| {
            piece.material.total_amount() == 1.0
                && piece
                    .internal_connections
                    .as_ref()
                    .is_some_and(Vec::is_empty)
        }));
        assert!(!pieces.contains(&source));
        assert!(
            (energy_before - simulation.environment.chemistry_usable_energy - 0.5).abs() < 1e-9
        );
        assert!((simulation.energy_ledger.total_work_expended - ledger_before - 0.5).abs() < 1e-9);
        assert!(
            simulation.energy_inventory_snapshot().is_balanced(1e-9),
            "environment BREAK must reconcile field energy, work, and global inventory"
        );
    }

    #[test]
    fn environmental_chemical_break_spends_its_own_usable_energy() {
        let mut simulation = crate::state::Simulation::new(935, 10.0);
        let source = environmental_test_composite(&simulation.environment.catalog);
        let bond = source
            .internal_connections
            .as_ref()
            .and_then(|connections| connections.first())
            .expect("test composite should contain one internal bond")
            .clone();
        let cell_index = simulation
            .environment
            .field
            .index_for_position(12.5, 12.5)
            .expect("source placement should be in the field");
        assert!(simulation.environment.field.deposit_physical_at_index(
            cell_index,
            source.clone(),
            &simulation.environment.catalog,
        ));
        remove_living_organisms_preserving_inventory(&mut simulation);
        fund_environment_chemistry_for_test(&mut simulation, 0.5);
        let environment_energy_before = simulation.environment.chemistry_usable_energy;
        let ledger_before = simulation.energy_ledger.total_work_expended;
        let operation = EnvironmentalChemicalBreakOperation {
            cell_index,
            material_index: 0,
            bond_index: 0,
            source_material: source.clone(),
            bond,
            activation_signal: 1.0,
            disruption_cost: 0.5,
            reaction_frequency_cycles_per_day: 440.0,
            accumulation_key: "environmental-atomic-break-resonance".into(),
            accumulated_reaction: 1.0,
        };

        assert!(simulation.organisms.is_empty());
        assert!(resolve_environmental(
            operation,
            &mut simulation.environment,
            &mut simulation.energy_ledger,
        ));

        assert_eq!(
            simulation.environment.field.cells[cell_index]
                .physical_materials
                .len(),
            2
        );
        assert!(
            (environment_energy_before - simulation.environment.chemistry_usable_energy - 0.5)
                .abs()
                < 1e-9
        );
        assert!((simulation.energy_ledger.total_work_expended - ledger_before - 0.5).abs() < 1e-9);
        assert!(
            simulation.energy_inventory_snapshot().is_balanced(1e-9),
            "environment-owned energy must reconcile even with no living organisms"
        );
        assert_eq!(
            simulation.energy_ledger.total_usable_energy_held,
            crate::reproduction::total_usable_energy_held(
                &simulation.organisms,
                &simulation.decomposing_bodies,
            ) + simulation.environment.chemistry_usable_energy
        );
    }

    #[test]
    fn environmental_chemical_break_with_insufficient_work_rolls_back_everything() {
        let mut simulation = crate::state::Simulation::new(934, 10.0);
        let source = environmental_test_composite(&simulation.environment.catalog);
        let bond = source
            .internal_connections
            .as_ref()
            .and_then(|connections| connections.first())
            .expect("test composite should contain one internal bond")
            .clone();
        let cell_index = simulation
            .environment
            .field
            .index_for_position(12.5, 12.5)
            .expect("source placement should be in the field");
        assert!(simulation.environment.field.deposit_physical_at_index(
            cell_index,
            source.clone(),
            &simulation.environment.catalog,
        ));
        remove_living_organisms_preserving_inventory(&mut simulation);
        let energy_before = simulation.environment.chemistry_usable_energy;
        let ledger_before = simulation.energy_ledger;
        let inventory_before = simulation.energy_inventory_snapshot();
        assert!(inventory_before.is_balanced(1e-9));
        let field_before = simulation.environment.field.cells.clone();
        let operation = EnvironmentalChemicalBreakOperation {
            cell_index,
            material_index: 0,
            bond_index: 0,
            source_material: source,
            bond,
            activation_signal: 1.0,
            disruption_cost: 0.5,
            reaction_frequency_cycles_per_day: 440.0,
            accumulation_key: "environmental-atomic-break-no-work".into(),
            accumulated_reaction: 1.0,
        };

        assert!(!resolve_environmental(
            operation,
            &mut simulation.environment,
            &mut simulation.energy_ledger,
        ));
        assert_eq!(simulation.environment.field.cells, field_before);
        assert_eq!(
            simulation.environment.chemistry_usable_energy,
            energy_before
        );
        assert_eq!(simulation.energy_ledger, ledger_before);
        assert_eq!(simulation.energy_inventory_snapshot(), inventory_before);
    }

    #[test]
    fn environmental_commit_rejects_unactivated_operation_before_mutation() {
        let mut simulation = crate::state::Simulation::new(931, 10.0);
        let organism = &mut simulation.organisms[0];
        let operation = EnvironmentalChemicalBreakOperation {
            cell_index: usize::MAX,
            material_index: 0,
            bond_index: 0,
            source_material: crate::physical_material::PhysicalMaterial {
                material: Material::free_base("Carbon", 1.0),
                placements: None,
                internal_connections: None,
                owner_relative_origin: None,
                realized_shapes: None,
            },
            bond: crate::physical_material::PhysicalMaterialBond {
                part_a: 0,
                endpoint_a: crate::structure::ConnectionEndpoint::Corner { point_index: 0 },
                part_b: 1,
                endpoint_b: crate::structure::ConnectionEndpoint::Corner { point_index: 0 },
                bond_energy: 0.0,
            },
            activation_signal: 0.0,
            disruption_cost: 0.5,
            reaction_frequency_cycles_per_day: 440.0,
            accumulation_key: "invalid-environmental-activation".into(),
            accumulated_reaction: 0.0,
        };
        let structure_before = organism.structure.clone();
        let energy_before = organism.usable_energy;
        let resonance_before = organism.resonant_energy.total_energy().unwrap();
        let ledger_before = simulation.energy_ledger;
        let field_before = simulation.environment.field.cells.clone();

        assert!(!resolve_environmental(
            operation,
            &mut simulation.environment,
            &mut simulation.energy_ledger,
        ));
        assert_eq!(
            format!("{:?}", organism.structure),
            format!("{:?}", structure_before)
        );
        assert_eq!(organism.usable_energy, energy_before);
        assert_eq!(
            organism.resonant_energy.total_energy().unwrap(),
            resonance_before
        );
        assert_eq!(simulation.energy_ledger, ledger_before);
        assert_eq!(simulation.environment.field.cells, field_before);
    }

    #[test]
    fn environmental_commit_rejects_invalid_resonance_frequency_before_mutation() {
        let mut simulation = crate::state::Simulation::new(932, 10.0);
        let operation = EnvironmentalChemicalBreakOperation {
            cell_index: usize::MAX,
            material_index: 0,
            bond_index: 0,
            source_material: crate::physical_material::PhysicalMaterial {
                material: Material::free_base("Carbon", 1.0),
                placements: None,
                internal_connections: None,
                owner_relative_origin: None,
                realized_shapes: None,
            },
            bond: crate::physical_material::PhysicalMaterialBond {
                part_a: 0,
                endpoint_a: crate::structure::ConnectionEndpoint::Corner { point_index: 0 },
                part_b: 1,
                endpoint_b: crate::structure::ConnectionEndpoint::Corner { point_index: 0 },
                bond_energy: 0.0,
            },
            activation_signal: 1.0,
            disruption_cost: 0.5,
            reaction_frequency_cycles_per_day: f64::NAN,
            accumulation_key: "invalid-environmental-frequency".into(),
            accumulated_reaction: 1.0,
        };
        let structure_before = simulation.organisms[0].structure.clone();
        let usable_energy_before = simulation.organisms[0].usable_energy;
        let ledger_before = simulation.energy_ledger;
        let field_before = simulation.environment.field.cells.clone();

        assert!(!resolve_environmental(
            operation,
            &mut simulation.environment,
            &mut simulation.energy_ledger,
        ));
        assert_eq!(
            format!("{:?}", simulation.organisms[0].structure),
            format!("{:?}", structure_before)
        );
        assert_eq!(simulation.organisms[0].usable_energy, usable_energy_before);
        assert_eq!(simulation.energy_ledger, ledger_before);
        assert_eq!(simulation.environment.field.cells, field_before);
    }

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
    fn successful_chemical_break_still_pays_work_when_activation_covers_barrier() {
        let mut simulation = crate::state::Simulation::new(920, 10.0);
        let bond = *simulation.organisms[0]
            .structure
            .bonds
            .first()
            .expect("seed organism should contain a structural bond");
        assert!(simulation.organisms[0].resonant_energy.deposit(440.0, 1.0));
        let retained_resonance = simulation.organisms[0]
            .resonant_energy
            .total_energy()
            .expect("retained resonance should have finite energy");
        simulation.energy_ledger.initial_resonant_energy_held = retained_resonance;
        simulation.energy_ledger.total_resonant_energy_held = retained_resonance;
        let operation = ChemicalBreakOperation {
            organism_id: simulation.organisms[0].id.clone(),
            bond,
            activation_signal: 1_000.0,
            disruption_cost: 0.5,
            reaction_frequency_cycles_per_day: 440.0,
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
        assert!((before - after - 0.5).abs() < 1e-9);
        assert!(!simulation.organisms[0]
            .structure
            .bonds
            .iter()
            .any(|candidate| candidate.has_same_identity(&bond)));
    }

    #[test]
    fn successful_chemical_break_spends_resonance_for_full_disruption_work() {
        let mut simulation = crate::state::Simulation::new(924, 10.0);
        let bond = *simulation.organisms[0]
            .structure
            .bonds
            .first()
            .expect("seed organism should contain a structural bond");
        assert!(simulation.organisms[0].resonant_energy.deposit(440.0, 1.0));
        let retained_resonance = simulation.organisms[0]
            .resonant_energy
            .total_energy()
            .expect("retained resonance should have finite energy");
        simulation.energy_ledger.initial_resonant_energy_held = retained_resonance;
        simulation.energy_ledger.total_resonant_energy_held = retained_resonance;
        let operation = ChemicalBreakOperation {
            organism_id: simulation.organisms[0].id.clone(),
            bond,
            activation_signal: 0.5,
            disruption_cost: 0.5,
            reaction_frequency_cycles_per_day: 440.0,
            accumulation_key: "test|rev:0|bond".into(),
            accumulated_reaction: 0.5,
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
        assert!((before - after - 0.5).abs() < 1e-9);
        assert!(!simulation.organisms[0]
            .structure
            .bonds
            .iter()
            .any(|candidate| candidate.has_same_identity(&bond)));
    }

    #[test]
    fn resonance_cannot_substitute_for_activation_signal() {
        let mut simulation = crate::state::Simulation::new(925, 10.0);
        let bond = *simulation.organisms[0]
            .structure
            .bonds
            .first()
            .expect("seed organism should contain a structural bond");
        assert!(simulation.organisms[0].resonant_energy.deposit(440.0, 1.0));
        let operation = ChemicalBreakOperation {
            organism_id: simulation.organisms[0].id.clone(),
            bond,
            activation_signal: 0.0,
            disruption_cost: 0.5,
            reaction_frequency_cycles_per_day: 440.0,
            accumulation_key: "test|rev:0|bond".into(),
            accumulated_reaction: 0.0,
        };
        let before_resonance = simulation.organisms[0]
            .resonant_energy
            .total_energy()
            .expect("valid reservoir");
        let before_energy = simulation.organisms[0].usable_energy;
        let before_ledger = simulation.energy_ledger;
        let before_bonds = simulation.organisms[0].structure.bonds.clone();

        assert!(!resolve(
            operation,
            &mut simulation.organisms[0],
            &mut simulation.energy_ledger,
        ));
        assert_eq!(simulation.organisms[0].structure.bonds, before_bonds);
        assert_eq!(simulation.organisms[0].usable_energy, before_energy);
        assert_eq!(simulation.energy_ledger, before_ledger);
        assert_eq!(
            simulation.organisms[0].resonant_energy.total_energy(),
            Some(before_resonance)
        );
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
        simulation.organisms[0].usable_energy = 0.0;
        simulation.organisms[0].resonant_energy =
            crate::harmonics::ResonantEnergyReservoir::default();
        let disruption_cost = bond.bond_energy + 1.0;
        let operation = ChemicalBreakOperation {
            organism_id: simulation.organisms[0].id.clone(),
            bond,
            // Activation crosses the full barrier, but neither the bond nor
            // an energy holder can fund the disruption work.
            activation_signal: disruption_cost,
            disruption_cost,
            reaction_frequency_cycles_per_day: 440.0,
            accumulation_key: "test|rev:0|bond".into(),
            accumulated_reaction: disruption_cost,
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
        assert_eq!(accumulation.get("test|rev:0|bond"), Some(&disruption_cost));
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
            reaction_frequency_cycles_per_day: 440.0,
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
            reaction_frequency_cycles_per_day: 440.0,
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
    fn resonance_funded_chemical_break_reconciles_world_inventory() {
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
            activation_signal: 0.5,
            disruption_cost: 0.5,
            reaction_frequency_cycles_per_day: 440.0,
            accumulation_key: "test|rev:0|bond".into(),
            accumulated_reaction: 0.5,
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
            (before.current_resonant_energy - after.current_resonant_energy - 0.5).abs() < 1e-9,
            "the full physical disruption work must be withdrawn from resonance"
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
            reaction_frequency_cycles_per_day: 440.0,
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
        let activation_signal = disruption_cost;
        assert!(simulation.organisms[0]
            .resonant_energy
            .deposit(440.0, disruption_cost));
        let resonance_before = simulation.organisms[0]
            .resonant_energy
            .total_energy()
            .expect("resonance must be finite");
        simulation.energy_ledger.initial_resonant_energy_held = resonance_before;
        simulation.energy_ledger.total_resonant_energy_held = resonance_before;
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
            reaction_frequency_cycles_per_day: 440.0,
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
            reaction_frequency_cycles_per_day: 440.0,
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
    fn mixed_graph_and_environment_workload_shares_one_realization_budget() {
        let simulation = crate::state::Simulation::new(927, 10.0);
        let graph_bond = *simulation.organisms[0]
            .structure
            .bonds
            .first()
            .expect("seed organism should contain a structural bond");
        let source = environmental_test_composite(&simulation.environment.catalog);
        let environmental_bond = source
            .internal_connections
            .as_ref()
            .and_then(|connections| connections.first())
            .expect("environmental fixture should contain a bond")
            .clone();
        let graph_operations = (0..40)
            .map(|index| ChemicalBreakOperation {
                organism_id: simulation.organisms[0].id.clone(),
                bond: graph_bond,
                activation_signal: 1.0,
                disruption_cost: 0.5,
                reaction_frequency_cycles_per_day: 440.0,
                accumulation_key: format!("graph|candidate-{index:02}"),
                accumulated_reaction: 1.0 + index as f64,
            })
            .collect();
        let environmental_operations = (0..40)
            .map(|index| EnvironmentalChemicalBreakOperation {
                cell_index: 0,
                material_index: 0,
                bond_index: 0,
                source_material: source.clone(),
                bond: environmental_bond.clone(),
                activation_signal: 1.0,
                disruption_cost: 0.5,
                reaction_frequency_cycles_per_day: 440.0,
                accumulation_key: format!("environment|candidate-{index:02}"),
                accumulated_reaction: 100.0 + index as f64,
            })
            .collect();
        let total_budget = 3;
        let graph_budget = total_budget / 2;
        let mut graph_remaining = graph_budget;
        let mut accumulation = std::collections::BTreeMap::new();

        let graph_ready =
            schedule_operations(graph_operations, 0, &mut graph_remaining, &mut accumulation);
        let mut environmental_remaining = total_budget - graph_budget + graph_remaining;
        let environmental_ready = schedule_environmental_operations(
            environmental_operations,
            0,
            &mut environmental_remaining,
            &mut accumulation,
        );

        assert_eq!(graph_ready.len(), 1);
        assert_eq!(environmental_ready.len(), 2);
        assert_eq!(graph_ready.len() + environmental_ready.len(), total_budget);
        assert_eq!(environmental_remaining, 0);
        assert_eq!(
            accumulation.len(),
            77,
            "all deferred graph and environmental activations must retain their progress"
        );
        assert_eq!(
            accumulation.get("environment|candidate-02"),
            Some(&102.0),
            "environmental work beyond its share must retain activation progress"
        );
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
            reaction_frequency_cycles_per_day: 440.0,
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
