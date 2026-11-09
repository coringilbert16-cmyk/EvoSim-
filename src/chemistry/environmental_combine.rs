//! Bounded, field-owned environmental COMBINE.
//!
//! Environmental material pairs use the same live contact evaluator, chemical
//! activation accumulation, and energy-ledger transaction as organism COMBINE.
//! The field owns both source objects and the resulting realized compound.
use std::collections::{BTreeMap, BTreeSet, HashSet};

use crate::contact::{ConnectionCompatibilityCache, ConnectionPairCandidate};
use crate::physical_material::{PhysicalMaterial, PhysicalMaterialBond};
use crate::resources::{BaseResource, InternalBond, Material};
use crate::state::{EnergyLedger, Environment};
use crate::structure::{OrganismStructure, Placement};

const ENVIRONMENT_PREFIX: &str = "environment|combine:";
const FIELD_SCAN_ANCHOR_LIMIT: usize = 64;
const FIELD_TARGET_CELL_INSPECTION_LIMIT: usize = 64;
const FIELD_COMPONENT_PAIR_LIMIT: usize = 128;
const BOUNDS_PADDING: f64 = 2.1;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct EnvironmentalCombineOperation {
    pub(crate) cell_a: usize,
    pub(crate) material_a: usize,
    pub(crate) source_a: PhysicalMaterial,
    pub(crate) cell_b: usize,
    pub(crate) material_b: usize,
    pub(crate) source_b: PhysicalMaterial,
    /// Indices relative to the respective restored source objects.
    pub(crate) unit_a: usize,
    pub(crate) unit_b: usize,
    pub(crate) candidate: ConnectionPairCandidate,
    pub(crate) investment: f64,
    pub(crate) accumulation_key: String,
    pub(crate) accumulated_reaction: f64,
}

#[derive(Clone, Copy, Debug)]
struct SelectedContact {
    unit_a: usize,
    unit_b: usize,
    candidate: ConnectionPairCandidate,
    investment: f64,
    required_energy: f64,
}

fn origin() -> Placement {
    Placement {
        x: 0.0,
        y: 0.0,
        rotation_radians: 0.0,
    }
}

fn source_constituent_identity(
    cell: usize,
    material: usize,
    source: &PhysicalMaterial,
    unit: usize,
) -> String {
    if source.environmental_identity == 0 {
        // Direct subsystem tests and legacy callers can inspect a field before
        // its first transport/identity-assignment pass. Keep those unassigned
        // instances distinct without pretending their fallback key is
        // movement-stable.
        format!("unassigned:{cell}:{material}:{unit}")
    } else {
        format!("{}:{unit}", source.environmental_identity)
    }
}

fn source_pair_accumulation_prefix(
    cell_a: usize,
    material_a: usize,
    source_a: &PhysicalMaterial,
    cell_b: usize,
    material_b: usize,
    source_b: &PhysicalMaterial,
    unit_a: usize,
    unit_b: usize,
) -> String {
    // Assigned physical contacts follow material identity, not transient
    // cell/vector positions or world-space placement. Canonicalize the pair
    // so crossing cell/slot order does not create a new activation history.
    let left = source_constituent_identity(cell_a, material_a, source_a, unit_a);
    let right = source_constituent_identity(cell_b, material_b, source_b, unit_b);
    let (first, second) = if left <= right {
        (left, right)
    } else {
        (right, left)
    };
    format!("{ENVIRONMENT_PREFIX}{first}|{second}|")
}

fn source_pair_key(
    cell_a: usize,
    material_a: usize,
    source_a: &PhysicalMaterial,
    cell_b: usize,
    material_b: usize,
    source_b: &PhysicalMaterial,
    unit_a: usize,
    unit_b: usize,
    candidate: ConnectionPairCandidate,
) -> String {
    let first = source_constituent_identity(cell_a, material_a, source_a, unit_a);
    let second = source_constituent_identity(cell_b, material_b, source_b, unit_b);
    let (first, second, endpoint_a, endpoint_b) = if first <= second {
        (&first, &second, candidate.endpoint_a, candidate.endpoint_b)
    } else {
        (&second, &first, candidate.endpoint_b, candidate.endpoint_a)
    };
    format!("{ENVIRONMENT_PREFIX}{first}|{second}|{endpoint_a:?}|{endpoint_b:?}")
}

fn environmental_accumulation_totals(accumulation: &BTreeMap<String, f64>) -> (usize, f64) {
    accumulation
        .iter()
        .filter(|(key, _)| key.starts_with(ENVIRONMENT_PREFIX))
        .fold((0, 0.0), |(count, total), (_, value)| {
            (count + 1, total + *value)
        })
}

fn record_unupdated_accumulations(
    accumulation: &BTreeMap<String, f64>,
    updated_keys: &BTreeSet<String>,
    diagnostics: &mut crate::state::EnvironmentalCombineSearchDiagnostics,
) {
    // A bounded search not reaching an interaction is not evidence that the
    // interaction became invalid. Keep its accumulated progress until that
    // exact interface is revisited; accumulate_reaction applies normal
    // dissipation when the interface is actually evaluated.
    for (key, value) in accumulation {
        if key.starts_with(ENVIRONMENT_PREFIX) && !updated_keys.contains(key) {
            diagnostics.accumulation_keys_preserved_unsearched += 1;
            if value.is_finite() {
                diagnostics.accumulation_value_preserved_unsearched += *value;
            }
        }
    }
    let (entries, total) = environmental_accumulation_totals(accumulation);
    diagnostics.accumulation_entries_after_search = entries;
    diagnostics.accumulation_value_after_search = total;
}

fn constituent_prefix_from_accumulation_key(key: &str) -> Option<&str> {
    let mut separators = key.match_indices('|');
    separators.next()?;
    separators.next()?;
    let (third_separator, _) = separators.next()?;
    Some(&key[..=third_separator])
}

fn is_current_combine_accumulation_key(key: &str) -> bool {
    let mut fields = key.split('|');
    if fields.next() != Some("environment") {
        return false;
    }
    let Some(first) = fields
        .next()
        .and_then(|field| field.strip_prefix("combine:"))
    else {
        return false;
    };
    let Some(second) = fields.next() else {
        return false;
    };
    // The remaining fields are the two endpoint identities. Older keys
    // included cell/slot and a placement-sensitive debug signature in the
    // first section; discard those histories rather than retaining unreachable
    // entries forever after the key contract changes.
    let Some(_endpoint_a) = fields.next() else {
        return false;
    };
    let Some(_endpoint_b) = fields.next() else {
        return false;
    };
    if fields.next().is_some() {
        return false;
    }
    let is_pair = |value: &str| {
        let unassigned = value.starts_with("unassigned:");
        let identity = value.strip_prefix("unassigned:").unwrap_or(value);
        let mut parts = identity.split(':');
        if unassigned {
            parts
                .next()
                .is_some_and(|cell| cell.parse::<usize>().is_ok())
                && parts
                    .next()
                    .is_some_and(|slot| slot.parse::<usize>().is_ok())
                && parts
                    .next()
                    .is_some_and(|unit| unit.parse::<usize>().is_ok())
                && parts.next().is_none()
        } else {
            parts
                .next()
                .is_some_and(|id| id.parse::<u64>().is_ok_and(|id| id != 0))
                && parts
                    .next()
                    .is_some_and(|unit| unit.parse::<usize>().is_ok())
                && parts.next().is_none()
        }
    };
    is_pair(first) && is_pair(second)
}

fn invalidate_unupdated_constituent_accumulations(
    accumulation: &mut BTreeMap<String, f64>,
    checked_prefixes: &BTreeSet<String>,
    updated_keys: &BTreeSet<String>,
    diagnostics: &mut crate::state::EnvironmentalCombineSearchDiagnostics,
) {
    if checked_prefixes.is_empty() {
        return;
    }
    accumulation.retain(|key, value| {
        let was_checked = constituent_prefix_from_accumulation_key(key)
            .is_some_and(|prefix| checked_prefixes.contains(prefix));
        if key.starts_with(ENVIRONMENT_PREFIX) && was_checked && !updated_keys.contains(key) {
            diagnostics.accumulation_keys_invalidated += 1;
            if value.is_finite() {
                diagnostics.accumulation_value_invalidated += *value;
            }
            false
        } else {
            true
        }
    });
}

fn restored_source(
    source: &PhysicalMaterial,
    environment: &Environment,
) -> Option<(OrganismStructure, Vec<usize>)> {
    if !source.is_realized()
        || source.material.is_empty()
        || !source.material.is_valid_for_catalog(&environment.catalog)
    {
        return None;
    }
    let mut structure = OrganismStructure::new();
    let indices = crate::material_restoration::restore_material(
        &mut structure,
        source,
        origin(),
        &environment.catalog,
    )?;
    Some((structure, indices))
}

#[cfg_attr(not(test), expect(dead_code, reason = "Planned API"))]
fn select_best_contact(
    structure: &OrganismStructure,
    indices_a: &[usize],
    indices_b: &[usize],
    catalog: &[BaseResource],
    cache: &mut ConnectionCompatibilityCache,
    remaining_work: &mut usize,
) -> Option<SelectedContact> {
    select_best_contact_from_offset(
        structure,
        indices_a,
        indices_b,
        catalog,
        cache,
        0,
        remaining_work,
        None,
    )
    .into_iter()
    .next()
}

/// Advance the bounded constituent-pair window by one pair per tick.
/// Every pair therefore becomes the first inspected pair once per full cycle,
/// even when the available per-tick work budget varies.
fn component_pair_start(tick: u64, pair_count: usize) -> usize {
    if pair_count == 0 {
        0
    } else {
        tick as usize % pair_count
    }
}

fn select_best_contact_from_offset(
    structure: &OrganismStructure,
    indices_a: &[usize],
    indices_b: &[usize],
    catalog: &[BaseResource],
    cache: &mut ConnectionCompatibilityCache,
    pair_start: usize,
    remaining_work: &mut usize,
    diagnostics: Option<&mut crate::state::EnvironmentalCombineSearchDiagnostics>,
) -> Vec<SelectedContact> {
    let mut diagnostics = diagnostics;
    let Some(pair_count) = indices_a.len().checked_mul(indices_b.len()) else {
        return Vec::new();
    };
    if pair_count == 0 {
        return Vec::new();
    }

    let mut selected_contacts = Vec::new();
    let mut constituent_pairs_with_valid_interfaces = 0usize;
    let inspected_pairs = (*remaining_work).min(pair_count);
    for offset in 0..inspected_pairs {
        *remaining_work = (*remaining_work).saturating_sub(1);
        let pair_index = pair_start.wrapping_add(offset) % pair_count;
        if let Some(diagnostics) = diagnostics.as_deref_mut() {
            diagnostics.constituent_pairs_inspected += 1;
        }
        let unit_a = indices_a[pair_index / indices_b.len()];
        let unit_b = indices_b[pair_index % indices_b.len()];
        let chemical_potential = structure
            .units
            .get(unit_a)
            .and_then(|unit| constituent_chemical_position(unit, catalog))
            .zip(
                structure
                    .units
                    .get(unit_b)
                    .and_then(|unit| constituent_chemical_position(unit, catalog)),
            )
            .and_then(|(position_a, position_b)| {
                crate::chemistry::interaction_potential(
                    position_a,
                    position_b,
                    crate::chemistry::CHEMICAL_K,
                    crate::chemistry::CHEMICAL_D_MAX,
                )
            });
        let candidates = crate::contact::connection_pair_candidates_cached(
            structure, unit_a, unit_b, catalog, cache,
        );
        let mut pair_valid_interfaces = 0usize;
        for candidate in candidates {
            let Some((evaluation, _work, investment, required_energy)) =
                crate::combine_runtime::selected_candidate_evaluation(
                    structure, unit_a, unit_b, candidate, catalog,
                )
            else {
                continue;
            };
            pair_valid_interfaces += 1;
            selected_contacts.push(SelectedContact {
                unit_a,
                unit_b,
                candidate: evaluation.candidate,
                investment,
                required_energy,
            });
            if let Some(diagnostics) = diagnostics.as_deref_mut() {
                diagnostics.interfaces_sent_to_chemistry += 1;
            }
        }
        if pair_valid_interfaces > 0 {
            constituent_pairs_with_valid_interfaces += 1;
        }
        if let Some(diagnostics) = diagnostics.as_deref_mut() {
            diagnostics.physically_evaluable_interfaces += pair_valid_interfaces;
            if pair_valid_interfaces > 1 && chemical_potential.is_some_and(|value| value > 0.0) {
                diagnostics.chemically_favorable_pairs_with_multiple_interfaces += 1;
            }
        }
    }
    if let Some(diagnostics) = diagnostics {
        diagnostics.constituent_pairs_with_valid_interfaces +=
            constituent_pairs_with_valid_interfaces;
    }

    // Keep deterministic best-first ordering for callers that only need one
    // contact, while returning every valid interface to environmental chemistry.
    selected_contacts.sort_by(|left, right| {
        left.required_energy
            .partial_cmp(&right.required_energy)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                left.candidate
                    .distance
                    .partial_cmp(&right.candidate.distance)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| {
                format!(
                    "{:?}|{:?}",
                    left.candidate.endpoint_a, left.candidate.endpoint_b
                )
                .cmp(&format!(
                    "{:?}|{:?}",
                    right.candidate.endpoint_a, right.candidate.endpoint_b
                ))
            })
    });
    selected_contacts
}

/// Resolve chemical identity from one realized physical constituent.
/// Aggregate material properties intentionally have no single chemical position.
fn constituent_chemical_position(
    unit: &crate::structure::StructuralUnit,
    catalog: &[BaseResource],
) -> Option<f64> {
    let [(material_name, _)] = unit.material.parts.as_slice() else {
        return None;
    };
    catalog
        .iter()
        .find(|resource| resource.name == *material_name)
        .and_then(|resource| resource.properties.chemical_position)
}

fn chemical_step(
    environment: &Environment,
    chemistry_library: &mut crate::chemistry_library::ChemistryLibrary,
    structure: &OrganismStructure,
    selected: SelectedContact,
    indices_a: &[usize],
    indices_b: &[usize],
    source_a: &PhysicalMaterial,
    source_b: &PhysicalMaterial,
    cell_a: usize,
    material_a: usize,
    cell_b: usize,
    material_b: usize,
    accumulation: &mut BTreeMap<String, f64>,
    updated_keys: &mut BTreeSet<String>,
    operations: &mut Vec<EnvironmentalCombineOperation>,
    tick: u64,
    journal: &mut crate::environmental_evaluation::EvaluationJournal,
) {
    macro_rules! reject_step {
        () => {{
            journal.record(
                crate::environmental_evaluation::EnvironmentalReactionEvaluation::<()>::rejected(
                    tick,
                    crate::environmental_evaluation::EnvironmentalReactionKind::Combine,
                    crate::environmental_evaluation::EvaluationStage::Chemistry,
                    "combine_chemistry_validation_failed",
                ),
            );
            return;
        }};
    }
    // A structural unit is one physical constituent. Resolve its chemical
    // position from that constituent's catalog identity; aggregate properties
    // intentionally do not invent a molecule-wide chemical position.
    let Some(unit_a) = structure.units.get(selected.unit_a) else {
        reject_step!();
    };
    let Some(unit_b) = structure.units.get(selected.unit_b) else {
        reject_step!();
    };
    let [(material_name_a, _)] = unit_a.material.parts.as_slice() else {
        reject_step!();
    };
    let [(material_name_b, _)] = unit_b.material.parts.as_slice() else {
        reject_step!();
    };
    let Some(a) = unit_a.properties(&environment.catalog) else {
        reject_step!();
    };
    let Some(b) = unit_b.properties(&environment.catalog) else {
        reject_step!();
    };
    let Some(position_a) = constituent_chemical_position(unit_a, &environment.catalog) else {
        reject_step!();
    };
    let Some(position_b) = constituent_chemical_position(unit_b, &environment.catalog) else {
        reject_step!();
    };
    // The static potential depends on canonical chemistry identity and
    // chemical positions, not the geometry variant that happens to be touching.
    let cache_key = chemistry_library.key_from_chemical_pair(
        material_name_a,
        material_name_b,
        position_a,
        position_b,
    );
    let static_potential = match chemistry_library.get(&cache_key) {
        Some(record) => record.static_potential,
        None => {
            let Some(calculated) = crate::chemistry::interaction_potential(
                position_a,
                position_b,
                crate::chemistry::CHEMICAL_K,
                crate::chemistry::CHEMICAL_D_MAX,
            ) else {
                reject_step!();
            };
            match chemistry_library.get_or_insert_static_potential(cache_key, calculated) {
                Ok(Some(value)) => value,
                Ok(None) | Err(_) => calculated,
            }
        }
    };
    // Resolve exact live geometry after the invariant cache lookup. This
    // remains mandatory for contact, facing, engagement, and reaction checks.
    let Some(_interface) = crate::geometry_reference_library::resolve_live_contact_candidate(
        material_name_a,
        &structure.units[selected.unit_a],
        material_name_b,
        &structure.units[selected.unit_b],
        selected.candidate,
        &environment.catalog,
    ) else {
        reject_step!();
    };
    let Some(contact) = crate::chemistry::contact_factor(
        selected.candidate.distance,
        crate::chemistry::CHEMICAL_CONTACT_RADIUS,
    ) else {
        reject_step!();
    };
    let Some(engagement) = crate::chemistry::interface_engagement(selected.candidate.facing) else {
        reject_step!();
    };
    let key = source_pair_key(
        cell_a,
        material_a,
        source_a,
        cell_b,
        material_b,
        source_b,
        selected.unit_a,
        selected.unit_b,
        selected.candidate,
    );
    let previous = accumulation.get(&key).copied().unwrap_or(0.0);
    let Some(next) = crate::chemistry::accumulate_reaction(
        previous,
        static_potential,
        contact,
        engagement,
        crate::chemistry::CHEMICAL_DISSIPATION,
    ) else {
        reject_step!();
    };
    if next <= 0.0 {
        updated_keys.insert(key.clone());
        accumulation.remove(&key);
        reject_step!();
    }
    let strength = crate::combine::bond_strength(a, b);
    let Some(barrier) = crate::chemistry::activation_barrier_from_bond_strength(strength) else {
        reject_step!();
    };
    let Some(activation_signal) = crate::chemistry::normalized_chemistry_to_activation_signal(next)
    else {
        reject_step!();
    };
    updated_keys.insert(key.clone());
    if crate::chemistry::activated(activation_signal, barrier) {
        // Selected indices address the combined trial graph. Store each unit's
        // position within its own restored source instead of subtracting recipe
        // entry counts, which need not equal the number of realized constituents.
        let Some(unit_a) = indices_a.iter().position(|index| *index == selected.unit_a) else {
            reject_step!();
        };
        let Some(unit_b) = indices_b.iter().position(|index| *index == selected.unit_b) else {
            reject_step!();
        };
        accumulation.remove(&key);
        journal.record(
            crate::environmental_evaluation::EnvironmentalReactionEvaluation::ready(
                tick,
                crate::environmental_evaluation::EnvironmentalReactionKind::Combine,
                (),
            ),
        );
        operations.push(EnvironmentalCombineOperation {
            cell_a,
            material_a,
            source_a: source_a.clone(),
            cell_b,
            material_b,
            source_b: source_b.clone(),
            unit_a,
            unit_b,
            candidate: selected.candidate,
            investment: selected.investment,
            accumulation_key: key,
            accumulated_reaction: next,
        });
    } else {
        journal.record(
            crate::environmental_evaluation::EnvironmentalReactionEvaluation::<()>::deferred(
                tick,
                crate::environmental_evaluation::EnvironmentalReactionKind::Combine,
                crate::environmental_evaluation::EvaluationStage::Activation,
                "activation_accumulation_below_threshold",
            ),
        );
        accumulation.insert(key, next);
    }
}

#[cfg_attr(not(test), expect(dead_code, reason = "Test helper"))]
pub(crate) fn accumulate_environmental(
    environment: &Environment,
    accumulation: &mut BTreeMap<String, f64>,
    chemistry_library: &mut crate::chemistry_library::ChemistryLibrary,
    tick: u64,
    remaining_discovery_work: &mut usize,
) -> Vec<EnvironmentalCombineOperation> {
    let mut diagnostics = crate::state::EnvironmentalCombineSearchDiagnostics::default();
    accumulate_environmental_with_diagnostics(
        environment,
        accumulation,
        chemistry_library,
        tick,
        remaining_discovery_work,
        &mut diagnostics,
    )
}

pub(crate) fn accumulate_environmental_with_diagnostics(
    environment: &Environment,
    accumulation: &mut BTreeMap<String, f64>,
    chemistry_library: &mut crate::chemistry_library::ChemistryLibrary,
    tick: u64,
    remaining_discovery_work: &mut usize,
    diagnostics: &mut crate::state::EnvironmentalCombineSearchDiagnostics,
) -> Vec<EnvironmentalCombineOperation> {
    // Retire placement/index-based COMBINE histories once. They cannot be
    // mapped safely onto stable physical identities, so carrying them forward
    // would leak unreachable progress and diagnostics indefinitely.
    let unassigned_materials_exist = environment
        .field
        .cells
        .iter()
        .flat_map(|cell| cell.physical_materials.iter())
        .any(|material| material.environmental_identity == 0);
    accumulation.retain(|key, _| {
        !key.starts_with(ENVIRONMENT_PREFIX)
            || (is_current_combine_accumulation_key(key)
                && (unassigned_materials_exist || !key.contains("unassigned:")))
    });
    let (initial_accumulation_entries, initial_accumulation_value) =
        environmental_accumulation_totals(accumulation);
    diagnostics.accumulation_entries_before_search = initial_accumulation_entries;
    diagnostics.accumulation_value_before_search = initial_accumulation_value;
    if *remaining_discovery_work == 0 {
        diagnostics.budget_exhausted = true;
        record_unupdated_accumulations(accumulation, &BTreeSet::new(), diagnostics);
        return Vec::new();
    }
    let field_cell_count = environment.field.cells.len();
    if field_cell_count == 0 {
        record_unupdated_accumulations(accumulation, &BTreeSet::new(), diagnostics);
        return Vec::new();
    }

    let mut anchor_work = (*remaining_discovery_work)
        .min(FIELD_SCAN_ANCHOR_LIMIT)
        .min(crate::chemical_reaction::MAX_REACTION_CANDIDATE_INSPECTIONS_PER_TICK);
    let cell_scan_count = field_cell_count.min(FIELD_SCAN_ANCHOR_LIMIT);
    let mut operations = Vec::new();
    let mut journal = crate::environmental_evaluation::EvaluationJournal::default();
    let mut updated_keys = BTreeSet::new();
    let mut checked_prefixes = BTreeSet::new();
    let mut cache = ConnectionCompatibilityCache::new();
    let cell_start = tick as usize % field_cell_count;

    'anchors: for cell_offset in 0..cell_scan_count {
        if anchor_work == 0 || *remaining_discovery_work == 0 {
            break;
        }
        let cell_a = (cell_start + cell_offset) % field_cell_count;
        diagnostics.anchor_cells_inspected += 1;
        // Charge every anchor-cell inspection, including empty cells. Otherwise
        // a sparse field could traverse the scan window without consuming budget.
        *remaining_discovery_work = (*remaining_discovery_work).saturating_sub(1);
        let material_count = environment.field.cells[cell_a].physical_materials.len();
        if material_count == 0 {
            continue;
        }
        let material_start = (tick as usize + cell_a) % material_count;
        for material_offset in 0..material_count {
            if anchor_work == 0 || *remaining_discovery_work == 0 {
                break 'anchors;
            }
            anchor_work -= 1;
            // Charge each anchor material inspection against the caller's
            // shared per-tick budget. The local anchor cap alone is not enough:
            // a small supplied budget must also bound work on empty/sparse fields.
            *remaining_discovery_work = (*remaining_discovery_work).saturating_sub(1);
            diagnostics.anchor_materials_inspected += 1;
            let material_a = (material_start + material_offset) % material_count;
            let source_a = environment.field.cells[cell_a].physical_materials[material_a].clone();
            let Some((source_structure, indices_a)) = restored_source(&source_a, environment)
            else {
                continue;
            };
            // Environmental COMBINE must discover isolated physical objects too.
            // from_structure intentionally excludes unbonded components, which are
            // precisely the reactants this pass is responsible for finding.
            let Some(body) =
                crate::organism_geometry::OrganismBodyGeometry::from_all_realized_units(
                    &source_structure,
                )
            else {
                continue;
            };
            // Bound the spatial query itself, not just the subsequent loop:
            // very large source bounds must not allocate or traverse an unbounded
            // target-cell vector before the per-tick inspection budget applies.
            // Advance by one inspection window per tick so cells beyond the first
            // window remain eligible instead of being starved by large bounds.
            let target_cell_limit =
                FIELD_TARGET_CELL_INSPECTION_LIMIT.min(*remaining_discovery_work);
            let target_cells = environment.field.cells_intersecting_bounds_limited(
                body.min_x - BOUNDS_PADDING,
                body.max_x + BOUNDS_PADDING,
                body.min_y - BOUNDS_PADDING,
                body.max_y + BOUNDS_PADDING,
                target_cell_limit,
                (tick as usize)
                    .wrapping_add(cell_a)
                    .wrapping_add(material_a),
            );
            diagnostics.target_cell_windows_capped +=
                usize::from(target_cell_limit > 0 && target_cells.len() == target_cell_limit);
            for &cell_b in &target_cells {
                // Canonical ordering is a free prefilter: this cell pair is
                // discovered from its lower-index anchor and must not consume
                // this tick's shared work budget a second time.
                if cell_b < cell_a {
                    continue;
                }
                if *remaining_discovery_work == 0 {
                    break 'anchors;
                }
                // Cell inspections count even when a cell contains no material;
                // otherwise sparse fields could scan many cells for free.
                *remaining_discovery_work = (*remaining_discovery_work).saturating_sub(1);
                diagnostics.target_cells_inspected += 1;
                let target_count = environment.field.cells[cell_b].physical_materials.len();
                if target_count == 0 {
                    continue;
                }
                let target_material_start = (tick as usize + cell_b + material_a) % target_count;
                for target_offset in 0..target_count {
                    let material_b = (target_material_start + target_offset) % target_count;
                    if (cell_b, material_b) <= (cell_a, material_a) {
                        diagnostics.backward_ordered_target_materials += 1;
                        continue;
                    }
                    if *remaining_discovery_work == 0 {
                        break 'anchors;
                    }
                    *remaining_discovery_work = (*remaining_discovery_work).saturating_sub(1);
                    diagnostics.target_materials_inspected += 1;
                    diagnostics.forward_ordered_target_material_pair_inspections += 1;
                    if cell_a == cell_b {
                        diagnostics.same_cell_source_pair_inspections += 1;
                    }
                    let source_b =
                        environment.field.cells[cell_b].physical_materials[material_b].clone();
                    if !source_b.is_realized() || source_b.material.is_empty() {
                        diagnostics.unrealized_target_material_pairs += 1;
                        continue;
                    }
                    let mut trial_structure = source_structure.clone();
                    let Some(indices_b) = crate::material_restoration::restore_material(
                        &mut trial_structure,
                        &source_b,
                        origin(),
                        &environment.catalog,
                    ) else {
                        diagnostics.target_material_pairs_restore_failed += 1;
                        continue;
                    };
                    diagnostics.target_material_pairs_restored += 1;
                    if indices_b.iter().any(|index| {
                        crate::construction_runtime::placed_unit_overlaps(
                            &trial_structure,
                            &trial_structure.units[*index],
                            &indices_b,
                            &environment.catalog,
                        )
                    }) {
                        diagnostics.target_material_pairs_rejected_for_overlap += 1;
                        continue;
                    }
                    let mut component_work =
                        (*remaining_discovery_work).min(FIELD_COMPONENT_PAIR_LIMIT);
                    let component_work_initial = component_work;
                    let pair_count = indices_a.len().saturating_mul(indices_b.len());
                    diagnostics.constituent_pairs_available += pair_count;
                    diagnostics.constituent_pairs_uninspected_by_window +=
                        pair_count.saturating_sub(component_work_initial);
                    diagnostics.constituent_pair_windows_truncated +=
                        usize::from(component_work_initial < pair_count);
                    let uninspected_pairs = pair_count.saturating_sub(component_work_initial);
                    journal.record_deferred_count(
                        tick,
                        crate::environmental_evaluation::EnvironmentalReactionKind::Combine,
                        crate::environmental_evaluation::EvaluationStage::CandidateValidation,
                        "bounded_constituent_pair_window_exhausted",
                        uninspected_pairs,
                    );
                    let pair_start = component_pair_start(tick, pair_count);
                    let selected = select_best_contact_from_offset(
                        &trial_structure,
                        &indices_a,
                        &indices_b,
                        &environment.catalog,
                        &mut cache,
                        pair_start,
                        &mut component_work,
                        Some(&mut *diagnostics),
                    );
                    let component_work_used = component_work_initial.saturating_sub(component_work);
                    *remaining_discovery_work =
                        (*remaining_discovery_work).saturating_sub(component_work_used);
                    if pair_count > 0 {
                        for offset in 0..component_work_used {
                            let pair_index = pair_start.wrapping_add(offset) % pair_count;
                            let unit_a = indices_a[pair_index / indices_b.len()];
                            let unit_b = indices_b[pair_index % indices_b.len()];
                            checked_prefixes.insert(source_pair_accumulation_prefix(
                                cell_a, material_a, &source_a, cell_b, material_b, &source_b,
                                unit_a, unit_b,
                            ));
                        }
                    }
                    for selected in selected {
                        chemical_step(
                            environment,
                            chemistry_library,
                            &trial_structure,
                            selected,
                            &indices_a,
                            &indices_b,
                            &source_a,
                            &source_b,
                            cell_a,
                            material_a,
                            cell_b,
                            material_b,
                            accumulation,
                            &mut updated_keys,
                            &mut operations,
                            tick,
                            &mut journal,
                        );
                    }
                }
            }
        }
    }

    invalidate_unupdated_constituent_accumulations(
        accumulation,
        &checked_prefixes,
        &updated_keys,
        diagnostics,
    );
    diagnostics.budget_exhausted = *remaining_discovery_work == 0;
    // Exhaustive field-wide reference count. This is deliberately separate
    // from spatially eligible pairs: it exposes the portion of the global
    // material-pair space not reached by this bounded broad-phase pass.
    let realized_material_count = environment
        .field
        .cells
        .iter()
        .flat_map(|cell| cell.physical_materials.iter())
        .filter(|material| material.is_realized() && !material.material.is_empty())
        .count();
    diagnostics.global_realized_material_pairs =
        realized_material_count.saturating_mul(realized_material_count.saturating_sub(1)) / 2;
    diagnostics.global_realized_material_pairs_unvisited = diagnostics
        .global_realized_material_pairs
        .saturating_sub(diagnostics.forward_ordered_target_material_pair_inspections);
    diagnostics.accumulation_keys_updated = updated_keys.len();
    diagnostics.evaluation_outcomes = journal.counts();
    record_unupdated_accumulations(accumulation, &updated_keys, diagnostics);

    // The simulation tick is the stable identity of this COMBINE evaluation
    // pass. At this boundary every operation has already passed the existing
    // COMBINE chemistry, activation accumulation, and energy checks; the
    // envelope records readiness for the unchanged reaction-specific scheduler.
    // Discovery and all COMBINE policy remain in their existing implementation.
    operations
        .into_iter()
        .filter_map(|operation| {
            crate::environmental_evaluation::EnvironmentalReactionEvaluation::ready(
                tick,
                crate::environmental_evaluation::EnvironmentalReactionKind::Combine,
                operation,
            )
            .into_ready()
        })
        .collect()
}

/// Production COMBINE entry point backed by the tick's shared environmental
/// snapshot. The spatial cursor supplies material-pair discovery; the existing
/// COMBINE endpoint, chemistry, activation, and energy evaluators remain native.
#[expect(
    dead_code,
    reason = "Compatibility wrapper for isolated reaction tests"
)]
pub(crate) fn accumulate_environmental_with_shared_snapshot(
    environment: &Environment,
    accumulation: &mut BTreeMap<String, f64>,
    chemistry_library: &mut crate::chemistry_library::ChemistryLibrary,
    tick: u64,
    remaining_discovery_work: &mut usize,
    diagnostics: &mut crate::state::EnvironmentalCombineSearchDiagnostics,
    snapshot: &crate::environmental_broad_phase::EnvironmentalMaterialSnapshot,
) -> Vec<EnvironmentalCombineOperation> {
    let mut shared_bounds = vec![None; snapshot.materials.len()];
    let mut shared_candidate_pairs = Vec::new();
    let mut completed_anchors = vec![None; snapshot.materials.len()];
    accumulate_environmental_with_shared_snapshot_and_candidates(
        environment,
        accumulation,
        chemistry_library,
        tick,
        remaining_discovery_work,
        diagnostics,
        snapshot,
        &mut shared_bounds,
        &mut shared_candidate_pairs,
        &mut completed_anchors,
    )
}

pub(crate) fn accumulate_environmental_with_shared_snapshot_and_candidates(
    environment: &Environment,
    accumulation: &mut BTreeMap<String, f64>,
    chemistry_library: &mut crate::chemistry_library::ChemistryLibrary,
    tick: u64,
    remaining_discovery_work: &mut usize,
    diagnostics: &mut crate::state::EnvironmentalCombineSearchDiagnostics,
    snapshot: &crate::environmental_broad_phase::EnvironmentalMaterialSnapshot,
    shared_bounds: &mut Vec<Option<crate::environmental_broad_phase::MaterialSpatialBounds>>,
    shared_candidate_pairs: &mut Vec<crate::environmental_broad_phase::MaterialPairRecord>,
    completed_anchors: &mut Vec<Option<usize>>,
) -> Vec<EnvironmentalCombineOperation> {
    if shared_bounds.len() != snapshot.materials.len() {
        *shared_bounds = vec![None; snapshot.materials.len()];
    }
    shared_candidate_pairs.clear();
    if completed_anchors.len() != snapshot.materials.len() {
        *completed_anchors = vec![None; snapshot.materials.len()];
    } else {
        completed_anchors.fill(None);
    }
    if snapshot.unassigned_materials != 0 {
        return accumulate_environmental_with_diagnostics(
            environment,
            accumulation,
            chemistry_library,
            tick,
            remaining_discovery_work,
            diagnostics,
        );
    }

    accumulation.retain(|key, _| {
        !key.starts_with(ENVIRONMENT_PREFIX) || is_current_combine_accumulation_key(key)
    });
    let (initial_entries, initial_value) = environmental_accumulation_totals(accumulation);
    diagnostics.accumulation_entries_before_search = initial_entries;
    diagnostics.accumulation_value_before_search = initial_value;
    if *remaining_discovery_work == 0 || environment.field.cells.is_empty() {
        diagnostics.budget_exhausted = *remaining_discovery_work == 0;
        record_unupdated_accumulations(accumulation, &BTreeSet::new(), diagnostics);
        return Vec::new();
    }

    // Prepare only the current anchor's geometry. This avoids the previous
    // full-field, unbudgeted bounds pass and lets each anchor share its restored
    // source structure with every target pair found around it.
    let mut operations = Vec::new();
    let mut journal = crate::environmental_evaluation::EvaluationJournal::default();
    let mut updated_keys = BTreeSet::new();
    let mut checked_prefixes = BTreeSet::new();
    let mut cache = ConnectionCompatibilityCache::new();

    let cell_count = environment.field.cells.len();
    let mut anchor_order = (0..snapshot.materials.len()).collect::<Vec<_>>();
    anchor_order.sort_by_key(|&index| {
        let record = &snapshot.materials[index];
        let cell_offset = if cell_count == 0 {
            0
        } else {
            record
                .locator
                .cell_index
                .wrapping_add(cell_count)
                .wrapping_sub(tick as usize % cell_count)
                % cell_count
        };
        let material_count = environment
            .field
            .cells
            .get(record.locator.cell_index)
            .map_or(1, |cell| cell.physical_materials.len())
            .max(1);
        let material_start =
            (tick as usize).wrapping_add(record.locator.cell_index) % material_count;
        let material_offset = record
            .locator
            .material_index
            .wrapping_add(material_count)
            .wrapping_sub(material_start)
            % material_count;
        (cell_offset, material_offset)
    });

    for anchor_index in anchor_order {
        if *remaining_discovery_work == 0 {
            break;
        }
        let Some(record_a) = snapshot.materials.get(anchor_index) else {
            continue;
        };
        let Some(source_a) = environment
            .field
            .cells
            .get(record_a.locator.cell_index)
            .and_then(|cell| cell.physical_materials.get(record_a.locator.material_index))
            .filter(|material| material.environmental_identity == record_a.identity.0)
            .cloned()
        else {
            continue;
        };
        // Charge anchor restoration by constituent count. Large anchors that
        // do not fit this tick's remaining budget are deferred to a later,
        // tick-rotated pass rather than forcing an unbounded preparation scan.
        let anchor_preparation_work = record_a.constituent_count.max(1);
        if anchor_preparation_work > *remaining_discovery_work {
            continue;
        }
        *remaining_discovery_work =
            (*remaining_discovery_work).saturating_sub(anchor_preparation_work);
        let Some((source_structure, indices_a)) = restored_source(&source_a, environment) else {
            continue;
        };
        let Some(body) = crate::organism_geometry::OrganismBodyGeometry::from_all_realized_units(
            &source_structure,
        ) else {
            continue;
        };
        shared_bounds[anchor_index] =
            Some(crate::environmental_broad_phase::MaterialSpatialBounds {
                min_x: body.min_x,
                max_x: body.max_x,
                min_y: body.min_y,
                max_y: body.max_y,
            });
        let Some(mut cursor) =
            crate::environmental_broad_phase::IndexedSpatialCursor::new_for_anchor(
                snapshot,
                &environment.field,
                crate::environmental_broad_phase::PairOrientation::ForwardOrdered,
                tick as usize,
                anchor_index,
            )
        else {
            continue;
        };

        while *remaining_discovery_work > 0 {
            // Single-work-unit pages preserve the exact target-cell/material
            // continuation while leaving budget for constituent evaluation.
            let target_cell_limit =
                FIELD_TARGET_CELL_INSPECTION_LIMIT.min(*remaining_discovery_work);
            let page = cursor.next_page(
                snapshot,
                &environment.field,
                shared_bounds.as_slice(),
                BOUNDS_PADDING,
                target_cell_limit,
                1,
                1,
            );
            *remaining_discovery_work =
                (*remaining_discovery_work).saturating_sub(page.visited_work as usize);
            diagnostics.anchor_materials_inspected += page.visited_anchor_initializations as usize;
            diagnostics.target_cells_inspected += page.visited_cell_checks as usize;
            diagnostics.target_materials_inspected += page.visited_material_checks as usize;
            if page.visited_work == 0 && page.pairs.is_empty() {
                break;
            }

            for pair in page.pairs {
                let Some(record_b) = snapshot.materials.get(pair.right) else {
                    continue;
                };
                let Some(source_b) = environment
                    .field
                    .cells
                    .get(record_b.locator.cell_index)
                    .and_then(|cell| cell.physical_materials.get(record_b.locator.material_index))
                    .filter(|material| material.environmental_identity == record_b.identity.0)
                    .cloned()
                else {
                    continue;
                };
                if snapshot
                    .bond_indices_by_material
                    .get(&record_a.identity)
                    .is_some_and(|indices| !indices.is_empty())
                {
                    shared_candidate_pairs.push(pair);
                }
                diagnostics.forward_ordered_target_material_pair_inspections += 1;
                if record_a.locator.cell_index == record_b.locator.cell_index {
                    diagnostics.same_cell_source_pair_inspections += 1;
                }

                let mut trial_structure = source_structure.clone();
                let Some(indices_b) = crate::material_restoration::restore_material(
                    &mut trial_structure,
                    &source_b,
                    origin(),
                    &environment.catalog,
                ) else {
                    diagnostics.target_material_pairs_restore_failed += 1;
                    continue;
                };
                if indices_b.iter().any(|index| {
                    crate::construction_runtime::placed_unit_overlaps(
                        &trial_structure,
                        &trial_structure.units[*index],
                        &indices_b,
                        &environment.catalog,
                    )
                }) {
                    diagnostics.target_material_pairs_rejected_for_overlap += 1;
                    continue;
                }
                diagnostics.target_material_pairs_restored += 1;

                let mut component_work =
                    (*remaining_discovery_work).min(FIELD_COMPONENT_PAIR_LIMIT);
                let component_work_initial = component_work;
                let pair_count = indices_a.len().saturating_mul(indices_b.len());
                diagnostics.constituent_pairs_available += pair_count;
                diagnostics.constituent_pairs_uninspected_by_window +=
                    pair_count.saturating_sub(component_work_initial);
                diagnostics.constituent_pair_windows_truncated +=
                    usize::from(component_work_initial < pair_count);
                journal.record_deferred_count(
                    tick,
                    crate::environmental_evaluation::EnvironmentalReactionKind::Combine,
                    crate::environmental_evaluation::EvaluationStage::CandidateValidation,
                    "bounded_constituent_pair_window_exhausted",
                    pair_count.saturating_sub(component_work_initial),
                );
                let pair_start = component_pair_start(tick, pair_count);
                let selected = select_best_contact_from_offset(
                    &trial_structure,
                    &indices_a,
                    &indices_b,
                    &environment.catalog,
                    &mut cache,
                    pair_start,
                    &mut component_work,
                    Some(&mut *diagnostics),
                );
                let component_work_used = component_work_initial.saturating_sub(component_work);
                *remaining_discovery_work =
                    (*remaining_discovery_work).saturating_sub(component_work_used);
                if pair_count > 0 {
                    for offset in 0..component_work_used {
                        let pair_index = pair_start.wrapping_add(offset) % pair_count;
                        let unit_a = indices_a[pair_index / indices_b.len()];
                        let unit_b = indices_b[pair_index % indices_b.len()];
                        checked_prefixes.insert(source_pair_accumulation_prefix(
                            record_a.locator.cell_index,
                            record_a.locator.material_index,
                            &source_a,
                            record_b.locator.cell_index,
                            record_b.locator.material_index,
                            &source_b,
                            unit_a,
                            unit_b,
                        ));
                    }
                }
                for selected in selected {
                    chemical_step(
                        environment,
                        chemistry_library,
                        &trial_structure,
                        selected,
                        &indices_a,
                        &indices_b,
                        &source_a,
                        &source_b,
                        record_a.locator.cell_index,
                        record_a.locator.material_index,
                        record_b.locator.cell_index,
                        record_b.locator.material_index,
                        accumulation,
                        &mut updated_keys,
                        &mut operations,
                        tick,
                        &mut journal,
                    );
                }
            }
            if page.complete {
                completed_anchors[anchor_index] = Some(target_cell_limit);
                break;
            }
        }
    }

    invalidate_unupdated_constituent_accumulations(
        accumulation,
        &checked_prefixes,
        &updated_keys,
        diagnostics,
    );
    diagnostics.budget_exhausted = *remaining_discovery_work == 0;
    let material_count = snapshot.materials.len();
    diagnostics.global_realized_material_pairs =
        material_count.saturating_mul(material_count.saturating_sub(1)) / 2;
    diagnostics.global_realized_material_pairs_unvisited = diagnostics
        .global_realized_material_pairs
        .saturating_sub(diagnostics.forward_ordered_target_material_pair_inspections);
    diagnostics.accumulation_keys_updated = updated_keys.len();
    diagnostics.evaluation_outcomes = journal.counts();
    record_unupdated_accumulations(accumulation, &updated_keys, diagnostics);

    operations
        .into_iter()
        .filter_map(|operation| {
            crate::environmental_evaluation::EnvironmentalReactionEvaluation::ready(
                tick,
                crate::environmental_evaluation::EnvironmentalReactionKind::Combine,
                operation,
            )
            .into_ready()
        })
        .collect()
}

pub(crate) fn schedule_environmental_operations(
    mut operations: Vec<EnvironmentalCombineOperation>,
    tick: u64,
    remaining_work: &mut usize,
    accumulation: &mut BTreeMap<String, f64>,
) -> Vec<EnvironmentalCombineOperation> {
    if operations.is_empty() || *remaining_work == 0 {
        for operation in operations {
            accumulation
                .entry(operation.accumulation_key)
                .and_modify(|current| *current = (*current).max(operation.accumulated_reaction))
                .or_insert(operation.accumulated_reaction);
        }
        return Vec::new();
    }

    operations.sort_by(|left, right| left.accumulation_key.cmp(&right.accumulation_key));
    let rotate = tick as usize % operations.len();
    operations.rotate_left(rotate);
    let mut used_cells = HashSet::new();
    let mut ready = Vec::new();
    for operation in operations {
        let cells_available =
            !used_cells.contains(&operation.cell_a) && !used_cells.contains(&operation.cell_b);
        if cells_available && *remaining_work > 0 {
            used_cells.insert(operation.cell_a);
            used_cells.insert(operation.cell_b);
            *remaining_work -= 1;
            ready.push(operation);
        } else {
            accumulation
                .entry(operation.accumulation_key)
                .and_modify(|current| *current = (*current).max(operation.accumulated_reaction))
                .or_insert(operation.accumulated_reaction);
        }
    }
    ready.sort_by(|left, right| left.accumulation_key.cmp(&right.accumulation_key));
    ready
}

fn physical_material_from_structure(
    structure: &OrganismStructure,
    catalog: &[BaseResource],
) -> Option<PhysicalMaterial> {
    if structure.units.is_empty() {
        return None;
    }
    let mut parts = Vec::with_capacity(structure.units.len());
    let mut placements = Vec::with_capacity(structure.units.len());
    let mut shapes = Vec::with_capacity(structure.units.len());
    for unit in &structure.units {
        if unit.material.parts.len() != 1 || !unit.material.internal_bonds.is_empty() {
            return None;
        }
        let (name, amount) = unit.material.parts.first()?.clone();
        if !amount.is_finite() || amount <= 0.0 || amount.fract().abs() > 1e-9 {
            return None;
        }
        parts.push((name, amount));
        placements.push(unit.placement);
        shapes.push(unit.shape(catalog)?.clone());
    }

    let mut internal_bonds = Vec::with_capacity(structure.bonds.len());
    let mut internal_connections = Vec::with_capacity(structure.bonds.len());
    for bond in &structure.bonds {
        if !bond.bond_energy.is_finite() || bond.bond_energy < 0.0 {
            return None;
        }
        let part_a = structure.unit_index(bond.endpoint_a.constituent_id)?;
        let part_b = structure.unit_index(bond.endpoint_b.constituent_id)?;
        internal_bonds.push(InternalBond { part_a, part_b });
        internal_connections.push(PhysicalMaterialBond {
            part_a,
            endpoint_a: bond.endpoint_a.location,
            part_b,
            endpoint_b: bond.endpoint_b.location,
            bond_energy: bond.bond_energy,
        });
    }
    let material = Material {
        parts,
        internal_bonds,
    };
    if !material.is_valid_for_catalog(catalog) {
        return None;
    }
    let physical = PhysicalMaterial {
        material,
        environmental_identity: 0,
        placements: Some(placements),
        internal_connections: Some(internal_connections),
        owner_relative_origin: None,
        realized_shapes: Some(shapes),
    };
    physical.is_realized().then_some(physical)
}

/// Export every connected product from a trial reaction graph. This is
/// intentionally separate from the single-product COMBINE conversion so a
/// coupled replacement can release a displaced constituent without losing
/// constituent identity, geometry, or retained bond-energy allocations.
pub(crate) fn physical_materials_from_structure_components(
    structure: &OrganismStructure,
    catalog: &[BaseResource],
) -> Option<Vec<PhysicalMaterial>> {
    let components = structure.connected_components();
    if components.is_empty() {
        return None;
    }
    let mut products = Vec::with_capacity(components.len());
    for component in components {
        let ids = component
            .iter()
            .map(|index| structure.physical_id(*index))
            .collect::<Option<Vec<_>>>()?;
        let subgraph = structure.subgraph_for_physical_ids(&ids, &[])?;
        products.push(physical_material_from_structure(&subgraph, catalog)?);
    }
    Some(products)
}

pub(crate) fn resolve_environmental(
    operation: EnvironmentalCombineOperation,
    environment: &mut Environment,
    ledger: &mut EnergyLedger,
) -> bool {
    let Some(source_a) = environment
        .field
        .cells
        .get(operation.cell_a)
        .and_then(|cell| cell.physical_materials.get(operation.material_a))
    else {
        return false;
    };
    let Some(source_b) = environment
        .field
        .cells
        .get(operation.cell_b)
        .and_then(|cell| cell.physical_materials.get(operation.material_b))
    else {
        return false;
    };
    if source_a != &operation.source_a || source_b != &operation.source_b {
        return false;
    }

    let Some((mut trial_structure, indices_a)) = restored_source(&operation.source_a, environment)
    else {
        return false;
    };
    let Some(indices_b) = crate::material_restoration::restore_material(
        &mut trial_structure,
        &operation.source_b,
        origin(),
        &environment.catalog,
    ) else {
        return false;
    };
    if indices_b.iter().any(|index| {
        crate::construction_runtime::placed_unit_overlaps(
            &trial_structure,
            &trial_structure.units[*index],
            &indices_b,
            &environment.catalog,
        )
    }) {
        return false;
    }
    let Some(&unit_a) = indices_a.get(operation.unit_a) else {
        return false;
    };
    let Some(&unit_b) = indices_b.get(operation.unit_b) else {
        return false;
    };

    // Discovery normally schedules only activated pairs, but the commit
    // boundary must enforce that contract too: an operation may be stale or
    // constructed by another caller. Recompute the activation barrier from
    // the restored live materials and the operation's accumulated chemistry.
    let (Some(properties_a), Some(properties_b)) = (
        trial_structure.units[unit_a].properties(&environment.catalog),
        trial_structure.units[unit_b].properties(&environment.catalog),
    ) else {
        return false;
    };
    let strength = crate::combine::bond_strength(properties_a, properties_b);
    let Some(barrier) = crate::chemistry::activation_barrier_from_bond_strength(strength) else {
        return false;
    };
    let Some(activation_signal) =
        crate::chemistry::normalized_chemistry_to_activation_signal(operation.accumulated_reaction)
    else {
        return false;
    };
    if !crate::chemistry::activated(activation_signal, barrier) {
        return false;
    }

    let Some(reactant_structural_energy) =
        crate::energy_ledger::structural_potential(&trial_structure)
    else {
        return false;
    };
    let mut staged_ledger = *ledger;
    let mut staged_usable_energy = environment.chemistry_usable_energy;
    let mut cache = ConnectionCompatibilityCache::new();
    if crate::combine_runtime::form_selected_bond_diagnostic(
        &mut trial_structure,
        unit_a,
        unit_b,
        operation.candidate,
        operation.investment,
        &environment.catalog,
        &mut cache,
        &mut staged_ledger,
        &mut staged_usable_energy,
    )
    .is_err()
    {
        return false;
    }
    if !crate::combine_runtime::coalesce_fluid_pair_after_bond(
        &mut trial_structure,
        unit_a,
        unit_b,
        &environment.catalog,
    ) {
        return false;
    }
    let Some(product_structural_energy) =
        crate::energy_ledger::structural_potential(&trial_structure)
    else {
        return false;
    };
    let structural_delta = product_structural_energy - reactant_structural_energy;
    if !structural_delta.is_finite() {
        return false;
    }
    let Some(product) = physical_material_from_structure(&trial_structure, &environment.catalog)
    else {
        return false;
    };

    // Keep world-held totals synchronized with the environment-owned physical
    // result. Use the authoritative pre/post structure directly: subtracting
    // cumulative signed-delta counters can lose a small reaction's change after
    // many prior reactions and is not the physical state change itself.
    let usable_delta = staged_usable_energy - environment.chemistry_usable_energy;
    let next_total_usable = staged_ledger.total_usable_energy_held + usable_delta;
    let next_total_structural = staged_ledger.total_structural_energy_held + structural_delta;
    if !usable_delta.is_finite()
        || !next_total_usable.is_finite()
        || next_total_usable < 0.0
        || !structural_delta.is_finite()
        || !next_total_structural.is_finite()
    {
        return false;
    }
    staged_ledger.total_usable_energy_held = next_total_usable;
    staged_ledger.total_structural_energy_held = next_total_structural;

    // All fallible checks are complete. Remove in descending index order for a
    // same-cell pair, insert the product at the first source slot, and invalidate
    // every touched cell exactly once.
    if operation.cell_a == operation.cell_b {
        if operation.material_a >= operation.material_b {
            return false;
        }
        let cell = &mut environment.field.cells[operation.cell_a];
        cell.physical_materials.remove(operation.material_b);
        cell.physical_materials.remove(operation.material_a);
        cell.physical_materials
            .insert(operation.material_a, product);
        environment.field.mark_changed_at_index(operation.cell_a);
    } else {
        environment.field.cells[operation.cell_a]
            .physical_materials
            .remove(operation.material_a);
        environment.field.cells[operation.cell_b]
            .physical_materials
            .remove(operation.material_b);
        environment.field.cells[operation.cell_a]
            .physical_materials
            .insert(operation.material_a, product);
        environment.field.mark_changed_at_index(operation.cell_a);
        environment.field.mark_changed_at_index(operation.cell_b);
    }
    environment.chemistry_usable_energy = staged_usable_energy;
    *ledger = staged_ledger;
    true
}

pub(crate) fn resolve_environmental_preserving_accumulation(
    operation: EnvironmentalCombineOperation,
    environment: &mut Environment,
    ledger: &mut EnergyLedger,
    accumulation: &mut BTreeMap<String, f64>,
) -> bool {
    let key = operation.accumulation_key.clone();
    let accumulated_reaction = operation.accumulated_reaction;
    let success = resolve_environmental(operation, environment, ledger);
    if !success {
        accumulation.insert(key, accumulated_reaction);
    }
    success
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct DiscoveredContact {
    unit_a: usize,
    unit_b: usize,
    candidate: ConnectionPairCandidate,
}

/// Neutral bounded contact discovery for the shared inventory adapter.
/// This stops at physical endpoint eligibility and deliberately does not call
/// selected_candidate_evaluation, which applies COMBINE-specific chemistry,
/// formation cost, and energy policy.
#[allow(dead_code)] // Stage 2 reference adapter; production discovery remains unchanged.
fn discover_contact_interfaces_from_offset(
    structure: &OrganismStructure,
    indices_a: &[usize],
    indices_b: &[usize],
    catalog: &[BaseResource],
    pair_start: usize,
    remaining_work: &mut usize,
) -> (Vec<DiscoveredContact>, usize) {
    let Some(pair_count) = indices_a.len().checked_mul(indices_b.len()) else {
        return (Vec::new(), 0);
    };
    if pair_count == 0 {
        return (Vec::new(), 0);
    }

    let inspected_pairs = (*remaining_work).min(pair_count);
    let mut discovered = Vec::new();
    let mut raw_candidates_inspected = 0usize;
    for offset in 0..inspected_pairs {
        *remaining_work = (*remaining_work).saturating_sub(1);
        let pair_index = pair_start.wrapping_add(offset) % pair_count;
        let unit_a = indices_a[pair_index / indices_b.len()];
        let unit_b = indices_b[pair_index % indices_b.len()];
        let (candidates, raw_endpoint_count, _limit_reached) =
            crate::contact::connection_pair_candidates_bounded(
                structure,
                unit_a,
                unit_b,
                catalog,
                usize::MAX,
            );
        raw_candidates_inspected = raw_candidates_inspected.saturating_add(raw_endpoint_count);
        for candidate in candidates {
            if !candidate.available_a
                || !candidate.available_b
                || !candidate.distance.is_finite()
                || candidate.distance < 0.0
                || candidate.distance > crate::combine_runtime::COMBINE_CONTACT_TOLERANCE
            {
                continue;
            }
            // Facing and load are retained as shared facts. Do not apply
            // COMBINE-specific evaluator rejection here: the inventory must
            // not discard a candidate another reaction evaluator may use.
            discovered.push(DiscoveredContact {
                unit_a,
                unit_b,
                candidate,
            });
        }
    }
    (discovered, raw_candidates_inspected)
}

/// Project the complete geometry-eligible COMBINE contact set into the shared
/// inventory before any COMBINE-specific evaluator is applied.
#[allow(dead_code)] // Stage 2 adapter; production cutover waits for cross-path parity.
pub(crate) fn append_discovered_contacts_to_inventory(
    builder: &mut crate::environmental_interaction::EnvironmentalInteractionInventoryBuilder,
    discovered: &[DiscoveredContact],
    indices_a: &[usize],
    indices_b: &[usize],
    source_a: &PhysicalMaterial,
    source_b: &PhysicalMaterial,
    locator_a: crate::environmental_interaction::MaterialLocator,
    locator_b: crate::environmental_interaction::MaterialLocator,
) -> Result<usize, crate::environmental_interaction::InventoryBuildError> {
    use crate::environmental_interaction::{
        ConstituentRef, ContactContext, ContactInterfaceRecord, MaterialIdentity, MaterialRecord,
    };

    const TOPOLOGY_GENERATION: u64 = 0;
    let identity_a = MaterialIdentity(source_a.environmental_identity);
    let identity_b = MaterialIdentity(source_b.environmental_identity);
    builder.upsert_material(MaterialRecord {
        identity: identity_a,
        locator: locator_a,
        topology_generation: TOPOLOGY_GENERATION,
        // No per-material revision exists; pass_id versions the immutable
        // inventory snapshot. Zero is explicitly "version unavailable".
        state_version: 0,
        constituent_count: indices_a.len(),
    })?;
    builder.upsert_material(MaterialRecord {
        identity: identity_b,
        locator: locator_b,
        topology_generation: TOPOLOGY_GENERATION,
        state_version: 0,
        constituent_count: indices_b.len(),
    })?;

    let mut records = Vec::with_capacity(discovered.len());
    for contact in discovered {
        let local_a = indices_a
            .iter()
            .position(|index| *index == contact.unit_a)
            .ok_or(
                crate::environmental_interaction::InventoryBuildError::InvalidConstituentIndex,
            )?;
        let local_b = indices_b
            .iter()
            .position(|index| *index == contact.unit_b)
            .ok_or(
                crate::environmental_interaction::InventoryBuildError::InvalidConstituentIndex,
            )?;
        records.push(ContactInterfaceRecord {
            constituent_a: ConstituentRef {
                material: identity_a,
                topology_generation: TOPOLOGY_GENERATION,
                local_index: local_a,
            },
            endpoint_a: contact.candidate.endpoint_a,
            constituent_b: ConstituentRef {
                material: identity_b,
                topology_generation: TOPOLOGY_GENERATION,
                local_index: local_b,
            },
            endpoint_b: contact.candidate.endpoint_b,
            context: ContactContext::Ordinary,
            candidate: contact.candidate,
        });
    }

    let mut inserted = 0usize;
    for record in records {
        inserted += usize::from(builder.add_contact(record)?);
    }
    Ok(inserted)
}

/// Run the neutral COMBINE contact-discovery window and project its complete
/// geometry-eligible result. No COMBINE evaluator or activation state is read.
#[allow(dead_code)] // Stage 2 reference adapter; not wired into production yet.
pub(crate) fn discover_and_append_contacts_to_inventory(
    builder: &mut crate::environmental_interaction::EnvironmentalInteractionInventoryBuilder,
    structure: &OrganismStructure,
    indices_a: &[usize],
    indices_b: &[usize],
    catalog: &[BaseResource],
    pair_start: usize,
    remaining_work: &mut usize,
    source_a: &PhysicalMaterial,
    source_b: &PhysicalMaterial,
    locator_a: crate::environmental_interaction::MaterialLocator,
    locator_b: crate::environmental_interaction::MaterialLocator,
) -> Result<(usize, usize), crate::environmental_interaction::InventoryBuildError> {
    let initial_work = *remaining_work;
    let (discovered, raw_candidates_inspected) = discover_contact_interfaces_from_offset(
        structure,
        indices_a,
        indices_b,
        catalog,
        pair_start,
        remaining_work,
    );
    let inspected_pairs = initial_work.saturating_sub(*remaining_work);
    {
        let work = builder.work_mut();
        work.material_pair_visits = work.material_pair_visits.saturating_add(1);
        work.constituent_pair_visits = work
            .constituent_pair_visits
            .saturating_add(inspected_pairs as u64);
        work.raw_endpoint_candidates = work
            .raw_endpoint_candidates
            .saturating_add(raw_candidates_inspected as u64);
        work.geometry_evaluation_operations = work
            .geometry_evaluation_operations
            .saturating_add(raw_candidates_inspected as u64);
    }
    let inserted = append_discovered_contacts_to_inventory(
        builder,
        &discovered,
        indices_a,
        indices_b,
        source_a,
        source_b,
        locator_a,
        locator_b,
    )?;
    Ok((inserted, inspected_pairs))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Controlled one-tick physical COMBINE diagnostic for all unordered base pairs.
    ///
    /// This deliberately does not force activation or alter chemistry thresholds.
    /// It searches bounded poses for a valid contact, then lets the existing
    /// environmental chemical step report whether reaction progress is made.
    #[test]
    fn unsearched_environmental_accumulations_are_preserved_without_decay() {
        let key = format!("{ENVIRONMENT_PREFIX}unvisited-interface");
        let mut accumulation = BTreeMap::new();
        accumulation.insert(key.clone(), 0.75);
        let updated_keys = BTreeSet::new();
        let mut diagnostics = crate::state::EnvironmentalCombineSearchDiagnostics::default();

        record_unupdated_accumulations(&accumulation, &updated_keys, &mut diagnostics);

        assert_eq!(
            accumulation.get(&key),
            Some(&0.75),
            "a bounded search that does not revisit an interface must not dissipate its progress"
        );
        assert_eq!(diagnostics.accumulation_keys_preserved_unsearched, 1);
        assert_eq!(diagnostics.accumulation_value_preserved_unsearched, 0.75);
        assert_eq!(diagnostics.accumulation_keys_dissipated, 0);
        assert_eq!(diagnostics.accumulation_value_dissipated, 0.0);
    }

    #[test]
    fn rechecked_ineligible_interfaces_are_invalidated_without_touching_unsearched_keys() {
        let checked_prefix = format!("{ENVIRONMENT_PREFIX}checked-source-pair|unit-b|");
        let stale_key = format!("{checked_prefix}stale-interface");
        let active_key = format!("{checked_prefix}active-interface");
        let unsearched_key = format!("{ENVIRONMENT_PREFIX}unsearched-source-pair|unit-b|interface");
        let mut accumulation = BTreeMap::new();
        accumulation.insert(stale_key.clone(), 0.5);
        accumulation.insert(active_key.clone(), 0.25);
        accumulation.insert(unsearched_key.clone(), 0.75);
        let checked_prefixes = BTreeSet::from([checked_prefix]);
        let updated_keys = BTreeSet::from([active_key.clone()]);
        let mut diagnostics = crate::state::EnvironmentalCombineSearchDiagnostics::default();

        invalidate_unupdated_constituent_accumulations(
            &mut accumulation,
            &checked_prefixes,
            &updated_keys,
            &mut diagnostics,
        );

        assert!(!accumulation.contains_key(&stale_key));
        assert_eq!(accumulation.get(&active_key), Some(&0.25));
        assert_eq!(accumulation.get(&unsearched_key), Some(&0.75));
        assert_eq!(diagnostics.accumulation_keys_invalidated, 1);
        assert_eq!(diagnostics.accumulation_value_invalidated, 0.5);
    }

    #[test]
    fn controlled_physical_pair_matrix_classifies_all_base_resource_pairs() {
        let mut simulation = crate::state::Simulation::new(23029, 10.0);
        let catalog = simulation.environment.catalog.clone();
        let mut pair_count = 0usize;
        let mut contact_count = 0usize;

        for i in 0..catalog.len() {
            for j in i..catalog.len() {
                let resource_a = &catalog[i];
                let resource_b = &catalog[j];
                let source_a = PhysicalMaterial::realized(
                    Material::free_base(resource_a.name.clone(), 1.0),
                    vec![Placement {
                        x: 10.0,
                        y: 10.0,
                        rotation_radians: 0.0,
                    }],
                    &catalog,
                )
                .expect("each canonical resource must realize as a physical object");

                let mut fixture = None;
                'poses: for step in 1..=80 {
                    let distance = step as f64 * 0.05;
                    for rotation_step in 0..12 {
                        let source_b = PhysicalMaterial::realized(
                            Material::free_base(resource_b.name.clone(), 1.0),
                            vec![Placement {
                                x: 10.0 + distance,
                                y: 10.0,
                                rotation_radians: rotation_step as f64 * std::f64::consts::PI / 6.0,
                            }],
                            &catalog,
                        )
                        .expect("each canonical resource pose must realize");
                        let Some((mut structure, indices_a)) =
                            restored_source(&source_a, &simulation.environment)
                        else {
                            continue;
                        };
                        let Some(indices_b) = crate::material_restoration::restore_material(
                            &mut structure,
                            &source_b,
                            origin(),
                            &catalog,
                        ) else {
                            continue;
                        };
                        if indices_b.iter().any(|index| {
                            crate::construction_runtime::placed_unit_overlaps(
                                &structure,
                                &structure.units[*index],
                                &indices_b,
                                &catalog,
                            )
                        }) {
                            continue;
                        }
                        let mut cache = ConnectionCompatibilityCache::new();
                        let mut work = FIELD_COMPONENT_PAIR_LIMIT;
                        let Some(selected) = select_best_contact(
                            &structure, &indices_a, &indices_b, &catalog, &mut cache, &mut work,
                        ) else {
                            continue;
                        };
                        let name_a = structure.units[selected.unit_a]
                            .material
                            .parts
                            .first()
                            .map(|part| part.0.as_str());
                        let name_b = structure.units[selected.unit_b]
                            .material
                            .parts
                            .first()
                            .map(|part| part.0.as_str());
                        let interface = name_a.zip(name_b).and_then(|(a, b)| {
                            crate::geometry_reference_library::resolve_live_contact_candidate(
                                a,
                                &structure.units[selected.unit_a],
                                b,
                                &structure.units[selected.unit_b],
                                selected.candidate,
                                &catalog,
                            )
                        });
                        if interface.is_some() && selected.candidate.facing > 0.0 {
                            fixture = Some((source_b, structure, indices_a, indices_b, selected));
                            break 'poses;
                        }
                    }
                }

                let position_a = resource_a.properties.chemical_position;
                let position_b = resource_b.properties.chemical_position;
                let potential = position_a.zip(position_b).and_then(|(a, b)| {
                    crate::chemistry::interaction_potential(
                        a,
                        b,
                        crate::chemistry::CHEMICAL_K,
                        crate::chemistry::CHEMICAL_D_MAX,
                    )
                });

                if let Some((source_b, structure, indices_a, indices_b, selected)) = fixture {
                    contact_count += 1;
                    let mut accumulation = BTreeMap::new();
                    let mut updated = BTreeSet::new();
                    let mut operations = Vec::new();
                    let mut evaluation_journal =
                        crate::environmental_evaluation::EvaluationJournal::default();
                    chemical_step(
                        &simulation.environment,
                        &mut simulation.chemistry_library,
                        &structure,
                        selected,
                        &indices_a,
                        &indices_b,
                        &source_a,
                        &source_b,
                        0,
                        0,
                        0,
                        1,
                        &mut accumulation,
                        &mut updated,
                        &mut operations,
                        0,
                        &mut evaluation_journal,
                    );
                    let outcome = if !operations.is_empty() {
                        "activated"
                    } else if !accumulation.is_empty() {
                        "accumulating"
                    } else {
                        "contact_no_progress_or_rejected"
                    };
                    println!(
                        "PHYSICAL_PAIR_MATRIX|{}|{}|potential={:.9}|contact=eligible|distance={:.6}|facing={:.6}|outcome={}",
                        resource_a.name,
                        resource_b.name,
                        potential.unwrap_or(f64::NAN),
                        selected.candidate.distance,
                        selected.candidate.facing,
                        outcome
                    );
                } else {
                    println!(
                        "PHYSICAL_PAIR_MATRIX|{}|{}|potential={:.9}|contact=no_eligible_pose|outcome=geometry_or_contact_gate",
                        resource_a.name,
                        resource_b.name,
                        potential.unwrap_or(f64::NAN)
                    );
                }
                pair_count += 1;
            }
        }

        assert_eq!(
            pair_count, 28,
            "controlled matrix must classify all unordered pairs"
        );
        assert!(
            contact_count > 0,
            "the controlled pose search must exercise the physical contact path"
        );
    }

    /// A bonded two-constituent molecule must remain chemically addressable
    /// through its individual physical constituents when it meets a free base.
    /// This diagnostic exercises restoration, bounded physical contact selection,
    /// and the existing chemical accumulation path without forcing a reaction.
    #[test]
    fn compound_base_matrix_exercises_constituent_level_chemistry() {
        let mut simulation = crate::state::Simulation::new(23229, 10.0);
        let catalog = simulation.environment.catalog.clone();
        let compound = PhysicalMaterial::realized(
            Material {
                parts: vec![("Carbon".into(), 1.0), ("Carbon".into(), 1.0)],
                internal_bonds: vec![InternalBond {
                    part_a: 0,
                    part_b: 1,
                }],
            },
            vec![
                Placement {
                    x: 10.0,
                    y: 10.0,
                    rotation_radians: 0.0,
                },
                Placement {
                    x: 11.5,
                    y: 10.0 + 3.0_f64.sqrt() / 2.0,
                    rotation_radians: 0.0,
                },
            ],
            &catalog,
        )
        .expect("the bonded carbon compound must have a valid physical realization");
        let (compound_structure, compound_indices) =
            restored_source(&compound, &simulation.environment)
                .expect("the stored compound must restore with its existing bond");
        assert_eq!(compound_indices.len(), 2);
        assert_eq!(compound_structure.bonds.len(), 1);

        let mut classified = 0usize;
        let mut eligible_contacts = 0usize;
        let mut chemical_progress = 0usize;
        for resource in &catalog {
            let mut selected_fixture = None;
            'poses: for step in 1..=80 {
                let distance = step as f64 * 0.05;
                for rotation_step in 0..12 {
                    let base = PhysicalMaterial::realized(
                        Material::free_base(resource.name.clone(), 1.0),
                        vec![Placement {
                            x: 10.0 + distance,
                            y: 10.0,
                            rotation_radians: rotation_step as f64 * std::f64::consts::PI / 6.0,
                        }],
                        &catalog,
                    )
                    .expect("each canonical base resource must realize");
                    let mut structure = compound_structure.clone();
                    let Some(base_indices) = crate::material_restoration::restore_material(
                        &mut structure,
                        &base,
                        origin(),
                        &catalog,
                    ) else {
                        continue;
                    };
                    if base_indices.iter().any(|index| {
                        crate::construction_runtime::placed_unit_overlaps(
                            &structure,
                            &structure.units[*index],
                            &base_indices,
                            &catalog,
                        )
                    }) {
                        continue;
                    }
                    let mut cache = ConnectionCompatibilityCache::new();
                    let mut work = FIELD_COMPONENT_PAIR_LIMIT;
                    let Some(selected) = select_best_contact(
                        &structure,
                        &compound_indices,
                        &base_indices,
                        &catalog,
                        &mut cache,
                        &mut work,
                    ) else {
                        continue;
                    };
                    let name_a = structure.units[selected.unit_a].material.parts[0]
                        .0
                        .as_str();
                    let name_b = structure.units[selected.unit_b].material.parts[0]
                        .0
                        .as_str();
                    let interface =
                        crate::geometry_reference_library::resolve_live_contact_candidate(
                            name_a,
                            &structure.units[selected.unit_a],
                            name_b,
                            &structure.units[selected.unit_b],
                            selected.candidate,
                            &catalog,
                        );
                    if interface.is_some() && selected.candidate.facing > 0.0 {
                        selected_fixture = Some((base, structure, base_indices, selected));
                        break 'poses;
                    }
                }
            }

            if let Some((base, structure, base_indices, selected)) = selected_fixture {
                eligible_contacts += 1;
                let mut accumulation = BTreeMap::new();
                let mut updated = BTreeSet::new();
                let mut operations = Vec::new();
                let mut evaluation_journal =
                    crate::environmental_evaluation::EvaluationJournal::default();
                chemical_step(
                    &simulation.environment,
                    &mut simulation.chemistry_library,
                    &structure,
                    selected,
                    &compound_indices,
                    &base_indices,
                    &compound,
                    &base,
                    0,
                    0,
                    0,
                    1,
                    &mut accumulation,
                    &mut updated,
                    &mut operations,
                    0,
                    &mut evaluation_journal,
                );
                if !accumulation.is_empty() || !operations.is_empty() {
                    chemical_progress += 1;
                }
                println!(
                    "COMPOUND_BASE_MATRIX|Carbon+Carbon|{}|contact=eligible|accumulating={}|activated={}",
                    resource.name,
                    !accumulation.is_empty(),
                    !operations.is_empty()
                );
            } else {
                println!(
                    "COMPOUND_BASE_MATRIX|Carbon+Carbon|{}|contact=no_eligible_pose",
                    resource.name
                );
            }
            classified += 1;
        }

        assert_eq!(
            classified,
            catalog.len(),
            "every base partner must be classified"
        );
        assert!(
            eligible_contacts > 0,
            "the compound must expose a physical interface to at least one base"
        );
        assert!(
            chemical_progress > 0,
            "at least one eligible compound/base contact must advance existing chemistry"
        );
    }

    #[test]
    fn legacy_location_based_combine_keys_are_not_treated_as_current_progress() {
        let current =
            "environment|combine:101:0|202:0|Corner { point_index: 0 }|Corner { point_index: 0 }";
        let legacy =
            "environment|combine:4:2:PhysicalMaterial { placement: (1, 2) }:0|8:3:PhysicalMaterial { placement: (3, 4) }:0|Corner { point_index: 0 }|Corner { point_index: 0 }";
        assert!(is_current_combine_accumulation_key(current));
        assert!(!is_current_combine_accumulation_key(legacy));
    }

    #[test]
    fn environmental_pair_keys_follow_material_identity_across_movement() {
        let catalog = crate::resources::default_catalog();
        let mut a = PhysicalMaterial::realized(
            Material::free_base("Carbon", 1.0),
            vec![Placement {
                x: 1.0,
                y: 2.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .unwrap();
        a.environmental_identity = 101;
        let mut b = PhysicalMaterial::realized(
            Material::free_base("Hydrogen", 1.0),
            vec![Placement {
                x: 2.0,
                y: 2.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .unwrap();
        b.environmental_identity = 202;
        let candidate = ConnectionPairCandidate {
            endpoint_a: crate::structure::ConnectionEndpoint::Corner { point_index: 0 },
            endpoint_b: crate::structure::ConnectionEndpoint::Corner { point_index: 0 },
            distance: 0.0,
            facing: 1.0,
            load_a: 0.0,
            load_b: 0.0,
            available_a: true,
            available_b: true,
        };

        let original = source_pair_key(0, 0, &a, 0, 1, &b, 0, 0, candidate);

        // Same physical objects and interface after transport: only transient
        // locations, vector slots, placements, and traversal order have changed.
        let mut moved_a = a.clone();
        moved_a.placements.as_mut().unwrap()[0].x = 31.0;
        moved_a.placements.as_mut().unwrap()[0].rotation_radians = 0.4;
        let mut moved_b = b.clone();
        moved_b.placements.as_mut().unwrap()[0].x = 29.0;
        let moved = source_pair_key(9, 7, &moved_a, 3, 2, &moved_b, 0, 0, candidate);
        assert_eq!(original, moved);
        let mut accumulated_progress = BTreeMap::new();
        accumulated_progress.insert(original.clone(), 0.75);
        assert_eq!(accumulated_progress.get(&moved), Some(&0.75));

        // If traversal order reverses, endpoint ownership reverses with it but
        // the canonical physical interface remains identical.
        let reversed_candidate = ConnectionPairCandidate {
            endpoint_a: candidate.endpoint_b,
            endpoint_b: candidate.endpoint_a,
            ..candidate
        };
        assert_eq!(
            original,
            source_pair_key(3, 2, &b, 9, 7, &a, 0, 0, reversed_candidate)
        );

        // Distinct physical instances, constituents, and endpoints must never
        // inherit the same activation history.
        let mut other_a = a.clone();
        other_a.environmental_identity = 303;
        assert_ne!(
            original,
            source_pair_key(0, 0, &other_a, 0, 1, &b, 0, 0, candidate)
        );
        assert_ne!(
            original,
            source_pair_key(0, 0, &a, 0, 1, &b, 1, 0, candidate)
        );
        let other_endpoint = ConnectionPairCandidate {
            endpoint_a: crate::structure::ConnectionEndpoint::Corner { point_index: 1 },
            ..candidate
        };
        assert_ne!(
            original,
            source_pair_key(0, 0, &a, 0, 1, &b, 0, 0, other_endpoint)
        );

        // Unassigned materials are possible in direct subsystem callers before
        // transport. Their fallback identity must still distinguish instances.
        let mut unassigned_a = a.clone();
        unassigned_a.environmental_identity = 0;
        let mut unassigned_b = b.clone();
        unassigned_b.environmental_identity = 0;
        let fallback = source_pair_key(0, 0, &unassigned_a, 0, 1, &unassigned_b, 0, 0, candidate);
        assert!(is_current_combine_accumulation_key(&fallback));
        assert_ne!(
            fallback,
            source_pair_key(0, 0, &unassigned_a, 1, 1, &unassigned_b, 0, 0, candidate)
        );
    }

    #[test]
    fn environmental_source_restoration_preserves_world_placements() {
        let simulation = crate::state::Simulation::new(941, 10.0);
        let source = PhysicalMaterial::realized(
            Material::free_base("Carbon", 1.0),
            vec![Placement {
                x: 37.0,
                y: 42.0,
                rotation_radians: 0.25,
            }],
            &simulation.environment.catalog,
        )
        .unwrap();
        let (structure, _) = restored_source(&source, &simulation.environment).unwrap();
        assert_eq!(
            structure.units[0].placement,
            source.placements.as_ref().unwrap()[0]
        );
    }

    #[test]
    fn environmental_combine_discovery_charges_empty_cell_scans_to_budget() {
        let mut simulation = crate::state::Simulation::new(940, 10.0);
        for cell in &mut simulation.environment.field.cells {
            cell.physical_materials.clear();
        }
        let mut accumulation = BTreeMap::new();
        let mut budget = 1;

        let operations = accumulate_environmental(
            &simulation.environment,
            &mut accumulation,
            &mut simulation.chemistry_library,
            0,
            &mut budget,
        );

        assert!(operations.is_empty());
        assert_eq!(
            budget, 0,
            "even an empty anchor cell must consume the bounded discovery budget"
        );
    }

    #[test]
    fn environmental_combine_discovery_does_no_work_with_zero_budget() {
        let mut simulation = crate::state::Simulation::new(939, 10.0);
        let mut accumulation = BTreeMap::new();
        let mut budget = 0;

        let operations = accumulate_environmental(
            &simulation.environment,
            &mut accumulation,
            &mut simulation.chemistry_library,
            0,
            &mut budget,
        );

        assert!(operations.is_empty());
        assert_eq!(budget, 0);
    }

    #[test]
    fn environmental_combine_realization_scheduler_respects_budget_and_preserves_deferred_progress()
    {
        let catalog = crate::resources::default_catalog();
        let source = PhysicalMaterial::realized(
            Material::free_base("Carbon", 1.0),
            vec![origin()],
            &catalog,
        )
        .expect("single-resource material should realize");
        let candidate = ConnectionPairCandidate {
            endpoint_a: crate::structure::ConnectionEndpoint::Corner { point_index: 0 },
            endpoint_b: crate::structure::ConnectionEndpoint::Corner { point_index: 0 },
            distance: 0.0,
            facing: 1.0,
            load_a: 0.0,
            load_b: 0.0,
            available_a: true,
            available_b: true,
        };
        let operation = |cell_a, cell_b, key: &str, progress| EnvironmentalCombineOperation {
            cell_a,
            material_a: 0,
            source_a: source.clone(),
            cell_b,
            material_b: 0,
            source_b: source.clone(),
            unit_a: 0,
            unit_b: 0,
            candidate,
            investment: 0.0,
            accumulation_key: key.to_owned(),
            accumulated_reaction: progress,
        };
        let operations = vec![
            operation(0, 1, "environment|combine:a", 0.75),
            operation(2, 3, "environment|combine:b", 0.5),
        ];
        let mut accumulation = BTreeMap::new();
        let mut remaining_work = 1;

        let scheduled = schedule_environmental_operations(
            operations,
            0,
            &mut remaining_work,
            &mut accumulation,
        );

        assert_eq!(
            scheduled.len(),
            1,
            "one unit of realization budget permits one operation"
        );
        assert_eq!(
            remaining_work, 0,
            "the scheduler must charge every realized operation"
        );
        assert_eq!(
            accumulation.len(),
            1,
            "deferred activation progress must be retained"
        );
        assert_eq!(
            scheduled[0].accumulation_key, "environment|combine:a",
            "stable ordering must schedule the first key at tick zero"
        );
        assert_eq!(
            accumulation.get("environment|combine:b"),
            Some(&0.5),
            "the deferred operation's accumulated chemistry must survive"
        );
    }

    /// Commit two already-bonded molecules into one four-constituent product.
    /// Both original bond graphs must survive, one external bond must be added,
    /// and the environmental energy inventory must remain balanced.
    #[test]
    fn environmental_compound_compound_combine_commits_four_part_product() {
        let mut simulation = crate::state::Simulation::new(23430, 10.0);
        let catalog = simulation.environment.catalog.clone();
        simulation.environment.width = 30.0;
        simulation.environment.height = 30.0;
        simulation.environment.field =
            crate::environment::ActiveMaterialField::new(30.0, 30.0, 10.0);
        let cell = simulation
            .environment
            .field
            .index_for_position(10.0, 10.0)
            .expect("compound fixtures must lie in the field");

        let compound_a = PhysicalMaterial::realized(
            Material {
                parts: vec![("Carbon".into(), 1.0), ("Carbon".into(), 1.0)],
                internal_bonds: vec![InternalBond {
                    part_a: 0,
                    part_b: 1,
                }],
            },
            vec![
                Placement {
                    x: 10.0,
                    y: 10.0,
                    rotation_radians: 0.0,
                },
                Placement {
                    x: 11.5,
                    y: 10.0 + 3.0_f64.sqrt() / 2.0,
                    rotation_radians: 0.0,
                },
            ],
            &catalog,
        )
        .expect("carbon dimer must be a valid bonded compound");
        let original_a_energy = compound_a
            .internal_connections
            .as_ref()
            .and_then(|bonds| bonds.first())
            .map(|bond| bond.bond_energy)
            .expect("first compound must retain its allocated bond energy");

        let (structure_a, indices_a) = restored_source(&compound_a, &simulation.environment)
            .expect("first compound must restore with its original bond");
        assert_eq!(indices_a.len(), 2);
        assert_eq!(structure_a.bonds.len(), 1);

        let mut fixture = None;
        'resources: for resource in &catalog {
            if resource.physical_state == crate::resources::PhysicalState::Fluid {
                continue;
            }
            let carbon_position = catalog
                .iter()
                .find(|entry| entry.name == "Carbon")
                .and_then(|entry| entry.properties.chemical_position);
            let separation = carbon_position
                .zip(resource.properties.chemical_position)
                .map(|(a, b)| (a - b).abs())
                .unwrap_or(0.0);
            if separation <= f64::EPSILON {
                continue;
            }

            let mut compound_b = None;
            'internal_pose: for step in 1..=60 {
                let distance = step as f64 * 0.05;
                for rotation_step in 0..12 {
                    let angle = rotation_step as f64 * std::f64::consts::PI / 6.0;
                    let (sin, cos) = angle.sin_cos();
                    let candidate = PhysicalMaterial::realized(
                        Material {
                            parts: vec![(resource.name.clone(), 1.0), (resource.name.clone(), 1.0)],
                            internal_bonds: vec![InternalBond {
                                part_a: 0,
                                part_b: 1,
                            }],
                        },
                        vec![
                            Placement {
                                x: 10.0,
                                y: 10.0,
                                rotation_radians: angle,
                            },
                            Placement {
                                x: 10.0 + distance * cos,
                                y: 10.0 + distance * sin,
                                rotation_radians: angle,
                            },
                        ],
                        &catalog,
                    );
                    let Some(candidate) = candidate else {
                        continue;
                    };
                    if let Some((restored, indices)) =
                        restored_source(&candidate, &simulation.environment)
                    {
                        if indices.len() == 2 && restored.bonds.len() == 1 {
                            compound_b = Some(candidate);
                            break 'internal_pose;
                        }
                    }
                }
            }
            let Some(compound_b) = compound_b else {
                continue;
            };
            let original_b_energy = compound_b
                .internal_connections
                .as_ref()
                .and_then(|bonds| bonds.first())
                .map(|bond| bond.bond_energy)
                .expect("second compound must retain its allocated bond energy");
            let relative = compound_b
                .placements
                .as_ref()
                .expect("realized second compound must retain placements");
            let first = relative[0];
            let second = relative[1];
            let local_x = second.x - first.x;
            let local_y = second.y - first.y;

            for step in 1..=80 {
                let distance = step as f64 * 0.05;
                for rotation_step in 0..12 {
                    let angle = rotation_step as f64 * std::f64::consts::PI / 6.0;
                    let (sin, cos) = angle.sin_cos();
                    let rotated_x = local_x * cos - local_y * sin;
                    let rotated_y = local_x * sin + local_y * cos;
                    let base = PhysicalMaterial::realized(
                        compound_b.material.clone(),
                        vec![
                            Placement {
                                x: 10.0 + distance,
                                y: 10.0,
                                rotation_radians: first.rotation_radians + angle,
                            },
                            Placement {
                                x: 10.0 + distance + rotated_x,
                                y: 10.0 + rotated_y,
                                rotation_radians: second.rotation_radians + angle,
                            },
                        ],
                        &catalog,
                    )
                    .expect("translated second compound must retain its internal bond");

                    let (mut structure, restored_a) =
                        restored_source(&compound_a, &simulation.environment)
                            .expect("first compound must restore for each external pose");
                    let Some(restored_b) = crate::material_restoration::restore_material(
                        &mut structure,
                        &base,
                        origin(),
                        &catalog,
                    ) else {
                        continue;
                    };
                    if restored_b.iter().any(|index| {
                        crate::construction_runtime::placed_unit_overlaps(
                            &structure,
                            &structure.units[*index],
                            &restored_b,
                            &catalog,
                        )
                    }) {
                        continue;
                    }
                    let mut cache = ConnectionCompatibilityCache::new();
                    let mut work = FIELD_COMPONENT_PAIR_LIMIT;
                    let Some(selected) = select_best_contact(
                        &structure,
                        &restored_a,
                        &restored_b,
                        &catalog,
                        &mut cache,
                        &mut work,
                    ) else {
                        continue;
                    };
                    let name_a = structure.units[selected.unit_a].material.parts[0]
                        .0
                        .as_str();
                    let name_b = structure.units[selected.unit_b].material.parts[0]
                        .0
                        .as_str();
                    let interface =
                        crate::geometry_reference_library::resolve_live_contact_candidate(
                            name_a,
                            &structure.units[selected.unit_a],
                            name_b,
                            &structure.units[selected.unit_b],
                            selected.candidate,
                            &catalog,
                        );
                    if selected.candidate.distance > 0.25
                        || selected.candidate.facing <= 0.5
                        || interface.is_none()
                    {
                        continue;
                    }

                    let mut accumulation = BTreeMap::new();
                    let mut updated = BTreeSet::new();
                    let mut operations = Vec::new();
                    let mut evaluation_journal =
                        crate::environmental_evaluation::EvaluationJournal::default();
                    chemical_step(
                        &simulation.environment,
                        &mut simulation.chemistry_library,
                        &structure,
                        selected,
                        &restored_a,
                        &restored_b,
                        &compound_a,
                        &base,
                        cell,
                        0,
                        cell,
                        1,
                        &mut accumulation,
                        &mut updated,
                        &mut operations,
                        0,
                        &mut evaluation_journal,
                    );
                    if !accumulation.is_empty() || !operations.is_empty() {
                        fixture = Some((base, resource.name.clone(), original_b_energy));
                        break 'resources;
                    }
                }
            }
        }

        let (compound_b, partner_name, original_b_energy) =
            fixture.expect("two bonded compounds must expose a chemically progressing interface");
        assert!(simulation.environment.field.deposit_physical_at_index(
            cell,
            compound_a.clone(),
            &catalog,
        ));
        assert!(simulation.environment.field.deposit_physical_at_index(
            cell,
            compound_b.clone(),
            &catalog,
        ));
        simulation.environment.chemistry_usable_energy = 1_000.0;
        let baseline = simulation.energy_inventory_snapshot();
        simulation.energy_ledger = crate::state::EnergyLedger::default();
        simulation.energy_ledger.initial_usable_energy_held = baseline.current_usable_energy;
        simulation.energy_ledger.total_usable_energy_held = baseline.current_usable_energy;
        simulation.energy_ledger.initial_resonant_energy_held = baseline.current_resonant_energy;
        simulation.energy_ledger.total_resonant_energy_held = baseline.current_resonant_energy;
        simulation.energy_ledger.initial_structural_energy_held =
            baseline.current_structural_energy;
        simulation.energy_ledger.total_structural_energy_held = baseline.current_structural_energy;

        let mut accumulation = BTreeMap::new();
        let mut committed = false;
        for tick in 0..2_000_u64 {
            let mut discovery_work =
                crate::chemical_reaction::MAX_REACTION_CANDIDATE_INSPECTIONS_PER_TICK / 4;
            let operations = accumulate_environmental(
                &simulation.environment,
                &mut accumulation,
                &mut simulation.chemistry_library,
                tick,
                &mut discovery_work,
            );
            let mut realization_work =
                crate::chemical_reaction::MAX_REACTION_REALIZATIONS_PER_TICK / 4;
            let scheduled = schedule_environmental_operations(
                operations,
                tick,
                &mut realization_work,
                &mut accumulation,
            );
            if let Some(operation) = scheduled.into_iter().next() {
                committed = resolve_environmental(
                    operation,
                    &mut simulation.environment,
                    &mut simulation.energy_ledger,
                );
                if committed {
                    break;
                }
            }
        }
        assert!(
            committed,
            "live compound/compound discovery must settle a COMBINE transaction; partner={partner_name}, accumulated={:?}",
            accumulation.values().copied().collect::<Vec<_>>()
        );

        let products = &simulation.environment.field.cells[cell].physical_materials;
        assert_eq!(products.len(), 1, "both molecules must become one product");
        let product = &products[0];
        assert_eq!(product.material.parts.len(), 4);
        assert_eq!(product.material.parts[0], ("Carbon".to_owned(), 1.0));
        assert_eq!(product.material.parts[1], ("Carbon".to_owned(), 1.0));
        assert_eq!(product.material.parts[2], (partner_name.clone(), 1.0));
        assert_eq!(product.material.parts[3], (partner_name, 1.0));
        assert_eq!(product.material.internal_bonds.len(), 3);
        assert!(
            product.material.internal_bonds.iter().any(|bond| {
                (bond.part_a == 0 && bond.part_b == 1) || (bond.part_a == 1 && bond.part_b == 0)
            }),
            "first molecule's original bond must survive"
        );
        assert!(
            product.material.internal_bonds.iter().any(|bond| {
                (bond.part_a == 2 && bond.part_b == 3) || (bond.part_a == 3 && bond.part_b == 2)
            }),
            "second molecule's original bond must survive"
        );
        assert!(
            product.material.internal_bonds.iter().any(|bond| {
                (bond.part_a < 2 && bond.part_b >= 2) || (bond.part_b < 2 && bond.part_a >= 2)
            }),
            "one new bond must connect the two original molecules"
        );
        assert_eq!(product.placements.as_ref().map(Vec::len), Some(4));
        assert_eq!(product.realized_shapes.as_ref().map(Vec::len), Some(4));

        let connections = product
            .internal_connections
            .as_ref()
            .expect("four-part product must preserve its physical bond graph");
        assert_eq!(connections.len(), 3);
        assert!(
            connections.iter().any(|bond| {
                ((bond.part_a == 0 && bond.part_b == 1) || (bond.part_a == 1 && bond.part_b == 0))
                    && (bond.bond_energy - original_a_energy).abs() <= 1e-9
            }),
            "first original bond energy must be preserved"
        );
        assert!(
            connections.iter().any(|bond| {
                ((bond.part_a == 2 && bond.part_b == 3) || (bond.part_a == 3 && bond.part_b == 2))
                    && (bond.bond_energy - original_b_energy).abs() <= 1e-9
            }),
            "second original bond energy must be preserved"
        );
        assert!(
            connections.iter().any(|bond| {
                ((bond.part_a < 2 && bond.part_b >= 2) || (bond.part_b < 2 && bond.part_a >= 2))
                    && bond.bond_energy > 0.0
            }),
            "new inter-molecule bond must receive allocated energy"
        );

        let expected_mass = compound_a.material.mass(&catalog) + compound_b.material.mass(&catalog);
        assert!(
            (product.material.mass(&catalog) - expected_mass).abs() <= 1e-9,
            "four-part COMBINE must conserve total material mass"
        );
        let inventory = simulation.energy_inventory_snapshot();
        assert!(
            (simulation.energy_ledger.total_usable_energy_held - inventory.current_usable_energy)
                .abs()
                <= 1e-6,
            "usable-energy ledger must match live holders"
        );
        assert!(
            (simulation.energy_ledger.total_structural_energy_held
                - inventory.current_structural_energy)
                .abs()
                <= 1e-6,
            "structural-energy ledger must match live bonds"
        );
        assert!(
            inventory.is_balanced_without_creation(1e-6),
            "compound/compound COMBINE must conserve energy; residual={:?}",
            inventory.residual()
        );

        // Break the newly formed inter-molecule bond, not either original
        // intramolecular bond. The two original compounds must be restored.
        let combined = product.clone();
        let connections = combined
            .internal_connections
            .as_ref()
            .expect("combined molecule must retain its realized bonds");
        let inter_molecule_bond_index = connections
            .iter()
            .position(|bond| {
                (bond.part_a < 2 && bond.part_b >= 2) || (bond.part_b < 2 && bond.part_a >= 2)
            })
            .expect("four-part product must contain an inter-molecule bond");
        let inter_molecule_bond = connections[inter_molecule_bond_index].clone();
        let break_operation = crate::chemical_reaction::EnvironmentalChemicalBreakOperation {
            cell_index: cell,
            material_index: 0,
            bond_index: inter_molecule_bond_index,
            source_material: combined,
            bond: inter_molecule_bond,
            activation_signal: 1.0,
            disruption_cost: 0.5,
            reaction_frequency_cycles_per_day: 440.0,
            accumulation_key: "compound-compound-four-part-round-trip-break".into(),
            accumulated_reaction: 1.0,
        };
        assert!(
            crate::chemical_reaction::resolve_environmental(
                break_operation,
                &mut simulation.environment,
                &mut simulation.energy_ledger,
            ),
            "BREAK must settle against the exact four-part COMBINE product"
        );

        let split = &simulation.environment.field.cells[cell].physical_materials;
        assert_eq!(
            split.len(),
            2,
            "BREAK must restore the two original molecules"
        );
        assert!(
            split.iter().all(|piece| piece.material.parts.len() == 2),
            "each restored molecule must retain its two original constituents"
        );
        assert!(
            split
                .iter()
                .all(|piece| piece.material.internal_bonds.len() == 1),
            "BREAK must preserve each molecule's original internal bond"
        );
        assert!(
            split.iter().all(|piece| {
                piece
                    .internal_connections
                    .as_ref()
                    .is_some_and(|bonds| bonds.len() == 1)
            }),
            "each restored molecule must retain its original realized bond energy"
        );
        assert!(
            split.iter().all(|piece| {
                piece.placements.as_ref().map(Vec::len) == Some(2)
                    && piece.realized_shapes.as_ref().map(Vec::len) == Some(2)
            }),
            "each restored molecule must retain both constituent realizations"
        );
        let mut restored_bond_energies: Vec<_> = split
            .iter()
            .filter_map(|piece| {
                piece
                    .internal_connections
                    .as_ref()
                    .and_then(|bonds| bonds.first())
                    .map(|bond| bond.bond_energy)
            })
            .collect();
        let mut original_bond_energies = [original_a_energy, original_b_energy];
        restored_bond_energies.sort_by(f64::total_cmp);
        original_bond_energies.sort_by(f64::total_cmp);
        assert_eq!(restored_bond_energies.len(), 2);
        assert!(
            restored_bond_energies
                .iter()
                .zip(original_bond_energies.iter())
                .all(|(restored, original)| (restored - original).abs() <= 1e-9),
            "BREAK must preserve the exact two original bond-energy allocations"
        );
        let split_parts: Vec<_> = split
            .iter()
            .flat_map(|piece| piece.material.parts.iter().cloned())
            .collect();
        let mut expected_parts = compound_a.material.parts.clone();
        expected_parts.extend(compound_b.material.parts.clone());
        let mut actual_parts = split_parts;
        actual_parts.sort_by(|left, right| {
            left.0
                .cmp(&right.0)
                .then_with(|| left.1.total_cmp(&right.1))
        });
        expected_parts.sort_by(|left, right| {
            left.0
                .cmp(&right.0)
                .then_with(|| left.1.total_cmp(&right.1))
        });
        assert_eq!(
            actual_parts, expected_parts,
            "COMBINE followed by BREAK must preserve the exact constituent multiset"
        );
        let split_mass: f64 = split
            .iter()
            .map(|piece| piece.material.mass(&catalog))
            .sum();
        assert!(
            (split_mass - expected_mass).abs() <= 1e-9,
            "four-part BREAK must conserve total material mass"
        );

        let after_break = simulation.energy_inventory_snapshot();
        assert!(
            (simulation.energy_ledger.total_usable_energy_held - after_break.current_usable_energy)
                .abs()
                <= 1e-6,
            "BREAK must keep the usable-energy ledger aligned with live holders"
        );
        assert!(
            (simulation.energy_ledger.total_structural_energy_held
                - after_break.current_structural_energy)
                .abs()
                <= 1e-6,
            "BREAK must keep the structural-energy ledger aligned with live bonds"
        );
        assert!(
            after_break.is_balanced_without_creation(1e-6),
            "four-part COMBINE/BREAK round trip must conserve energy; residual={:?}",
            after_break.residual()
        );
    }

    /// Diagnostic matrix for bonded compound + bonded compound encounters.
    /// Each side remains a physical constituent graph; chemistry is evaluated
    /// only at the selected constituent interface, never on averaged molecules.
    #[test]
    fn compound_compound_matrix_exercises_constituent_level_chemistry() {
        let mut simulation = crate::state::Simulation::new(23329, 10.0);
        let catalog = simulation.environment.catalog.clone();
        let carbon_compound = PhysicalMaterial::realized(
            Material {
                parts: vec![("Carbon".into(), 1.0), ("Carbon".into(), 1.0)],
                internal_bonds: vec![InternalBond {
                    part_a: 0,
                    part_b: 1,
                }],
            },
            vec![
                Placement {
                    x: 10.0,
                    y: 10.0,
                    rotation_radians: 0.0,
                },
                Placement {
                    x: 11.5,
                    y: 10.0 + 3.0_f64.sqrt() / 2.0,
                    rotation_radians: 0.0,
                },
            ],
            &catalog,
        )
        .expect("carbon dimer must be a valid bonded compound");
        let (carbon_structure, carbon_indices) =
            restored_source(&carbon_compound, &simulation.environment)
                .expect("carbon dimer must restore with its original bond");
        assert_eq!(carbon_indices.len(), 2);
        assert_eq!(carbon_structure.bonds.len(), 1);

        let mut classified = 0usize;
        let mut bonded_compounds = 0usize;
        let mut eligible_contacts = 0usize;
        let mut chemical_progress = 0usize;

        for resource in &catalog {
            let mut compound_b = None;
            'internal_pose: for step in 1..=60 {
                let distance = step as f64 * 0.05;
                for rotation_step in 0..12 {
                    let angle = rotation_step as f64 * std::f64::consts::PI / 6.0;
                    let (sin, cos) = angle.sin_cos();
                    let candidate = PhysicalMaterial::realized(
                        Material {
                            parts: vec![(resource.name.clone(), 1.0), (resource.name.clone(), 1.0)],
                            internal_bonds: vec![InternalBond {
                                part_a: 0,
                                part_b: 1,
                            }],
                        },
                        vec![
                            Placement {
                                x: 10.0,
                                y: 10.0,
                                rotation_radians: angle,
                            },
                            Placement {
                                x: 10.0 + distance * cos,
                                y: 10.0 + distance * sin,
                                rotation_radians: angle,
                            },
                        ],
                        &catalog,
                    );
                    let Some(candidate) = candidate else {
                        continue;
                    };
                    if let Some((structure, indices)) =
                        restored_source(&candidate, &simulation.environment)
                    {
                        if indices.len() == 2 && structure.bonds.len() == 1 {
                            compound_b = Some(candidate);
                            break 'internal_pose;
                        }
                    }
                }
            }

            let Some(compound_b) = compound_b else {
                println!(
                    "COMPOUND_COMPOUND_MATRIX|Carbon+Carbon|{}+{}|contact=no_bonded_realization",
                    resource.name, resource.name
                );
                classified += 1;
                continue;
            };
            bonded_compounds += 1;
            let relative = compound_b
                .placements
                .as_ref()
                .expect("realized compound must retain placements");
            let origin_b = relative[0];
            let second_b = relative[1];
            let local_x = second_b.x - origin_b.x;
            let local_y = second_b.y - origin_b.y;
            let mut selected_fixture = None;

            'external_pose: for step in 1..=80 {
                let distance = step as f64 * 0.05;
                for rotation_step in 0..12 {
                    let angle = rotation_step as f64 * std::f64::consts::PI / 6.0;
                    let (sin, cos) = angle.sin_cos();
                    let rotated_x = local_x * cos - local_y * sin;
                    let rotated_y = local_x * sin + local_y * cos;
                    let base = PhysicalMaterial::realized(
                        compound_b.material.clone(),
                        vec![
                            Placement {
                                x: 10.0 + distance,
                                y: 10.0,
                                rotation_radians: origin_b.rotation_radians + angle,
                            },
                            Placement {
                                x: 10.0 + distance + rotated_x,
                                y: 10.0 + rotated_y,
                                rotation_radians: second_b.rotation_radians + angle,
                            },
                        ],
                        &catalog,
                    )
                    .expect("translated bonded compound must retain its realization");
                    let (mut structure, indices_a) =
                        restored_source(&carbon_compound, &simulation.environment)
                            .expect("carbon dimer must restore for each pose");
                    let Some(indices_b) = crate::material_restoration::restore_material(
                        &mut structure,
                        &base,
                        origin(),
                        &catalog,
                    ) else {
                        continue;
                    };
                    if indices_b.iter().any(|index| {
                        crate::construction_runtime::placed_unit_overlaps(
                            &structure,
                            &structure.units[*index],
                            &indices_b,
                            &catalog,
                        )
                    }) {
                        continue;
                    }
                    let mut cache = ConnectionCompatibilityCache::new();
                    let mut work = FIELD_COMPONENT_PAIR_LIMIT;
                    let Some(selected) = select_best_contact(
                        &structure, &indices_a, &indices_b, &catalog, &mut cache, &mut work,
                    ) else {
                        continue;
                    };
                    let name_a = structure.units[selected.unit_a].material.parts[0]
                        .0
                        .as_str();
                    let name_b = structure.units[selected.unit_b].material.parts[0]
                        .0
                        .as_str();
                    let interface =
                        crate::geometry_reference_library::resolve_live_contact_candidate(
                            name_a,
                            &structure.units[selected.unit_a],
                            name_b,
                            &structure.units[selected.unit_b],
                            selected.candidate,
                            &catalog,
                        );
                    if selected.candidate.distance <= 0.25
                        && selected.candidate.facing > 0.5
                        && interface.is_some()
                    {
                        selected_fixture = Some((base, structure, indices_a, indices_b, selected));
                        break 'external_pose;
                    }
                }
            }

            if let Some((base, structure, indices_a, indices_b, selected)) = selected_fixture {
                eligible_contacts += 1;
                let mut accumulation = BTreeMap::new();
                let mut updated = BTreeSet::new();
                let mut operations = Vec::new();
                let mut evaluation_journal =
                    crate::environmental_evaluation::EvaluationJournal::default();
                chemical_step(
                    &simulation.environment,
                    &mut simulation.chemistry_library,
                    &structure,
                    selected,
                    &indices_a,
                    &indices_b,
                    &carbon_compound,
                    &base,
                    0,
                    0,
                    0,
                    1,
                    &mut accumulation,
                    &mut updated,
                    &mut operations,
                    0,
                    &mut evaluation_journal,
                );
                let outcome = if !operations.is_empty() {
                    chemical_progress += 1;
                    "activated"
                } else if !accumulation.is_empty() {
                    chemical_progress += 1;
                    "accumulating"
                } else {
                    "contact_no_progress"
                };
                println!(
                    "COMPOUND_COMPOUND_MATRIX|Carbon+Carbon|{}+{}|contact=eligible|outcome={}",
                    resource.name, resource.name, outcome
                );
            } else {
                println!(
                    "COMPOUND_COMPOUND_MATRIX|Carbon+Carbon|{}+{}|contact=no_eligible_pose",
                    resource.name, resource.name
                );
            }
            classified += 1;
        }

        assert_eq!(classified, catalog.len());
        assert!(
            bonded_compounds > 0,
            "at least one canonical resource must form a bonded homodimer"
        );
        assert!(
            eligible_contacts > 0,
            "two bonded compounds must expose at least one eligible external interface"
        );
        assert!(
            chemical_progress > 0,
            "at least one compound/compound interface must advance existing chemistry"
        );
    }

    /// End-to-end transaction: a bonded molecule combines with a free base,
    /// retaining its original bond while adding one new bond and conserving
    /// material and energy through environmental discovery and settlement.
    #[test]
    fn environmental_compound_base_combine_commits_three_part_product() {
        let mut simulation = crate::state::Simulation::new(23230, 10.0);
        let catalog = simulation.environment.catalog.clone();
        simulation.environment.width = 30.0;
        simulation.environment.height = 30.0;
        simulation.environment.field =
            crate::environment::ActiveMaterialField::new(30.0, 30.0, 10.0);
        let cell = simulation
            .environment
            .field
            .index_for_position(10.0, 10.0)
            .expect("compound fixture must lie in the field");

        let compound = PhysicalMaterial::realized(
            Material {
                parts: vec![("Carbon".into(), 1.0), ("Carbon".into(), 1.0)],
                internal_bonds: vec![InternalBond {
                    part_a: 0,
                    part_b: 1,
                }],
            },
            vec![
                Placement {
                    x: 10.0,
                    y: 10.0,
                    rotation_radians: 0.0,
                },
                Placement {
                    x: 11.5,
                    y: 10.0 + 3.0_f64.sqrt() / 2.0,
                    rotation_radians: 0.0,
                },
            ],
            &catalog,
        )
        .expect("bonded carbon reactant must be physically realized");
        let original_bond_energy = compound
            .internal_connections
            .as_ref()
            .and_then(|bonds| bonds.first())
            .map(|bond| bond.bond_energy)
            .expect("compound must carry its original allocated bond energy");

        // Find a real constituent-level interface whose live chemistry can
        // accumulate; do not inject a reaction or bypass the physical gate.
        let (compound_structure, compound_indices) =
            restored_source(&compound, &simulation.environment)
                .expect("compound must restore with its bond graph");
        let mut selected_fixture = None;
        'resources: for resource in &catalog {
            let position_carbon = catalog
                .iter()
                .find(|entry| entry.name == "Carbon")
                .and_then(|entry| entry.properties.chemical_position);
            let separation = position_carbon
                .zip(resource.properties.chemical_position)
                .map(|(a, b)| (a - b).abs())
                .unwrap_or(0.0);
            if separation < 9.0 {
                continue;
            }
            for step in 1..=80 {
                let distance = step as f64 * 0.05;
                for rotation_step in 0..12 {
                    let base = PhysicalMaterial::realized(
                        Material::free_base(resource.name.clone(), 1.0),
                        vec![Placement {
                            x: 10.0 + distance,
                            y: 10.0,
                            rotation_radians: rotation_step as f64 * std::f64::consts::PI / 6.0,
                        }],
                        &catalog,
                    )
                    .expect("canonical base resource must realize");
                    let mut structure = compound_structure.clone();
                    let Some(base_indices) = crate::material_restoration::restore_material(
                        &mut structure,
                        &base,
                        origin(),
                        &catalog,
                    ) else {
                        continue;
                    };
                    if base_indices.iter().any(|index| {
                        crate::construction_runtime::placed_unit_overlaps(
                            &structure,
                            &structure.units[*index],
                            &base_indices,
                            &catalog,
                        )
                    }) {
                        continue;
                    }
                    let mut cache = ConnectionCompatibilityCache::new();
                    let mut work = FIELD_COMPONENT_PAIR_LIMIT;
                    let Some(selected) = select_best_contact(
                        &structure,
                        &compound_indices,
                        &base_indices,
                        &catalog,
                        &mut cache,
                        &mut work,
                    ) else {
                        continue;
                    };
                    let name_a = structure.units[selected.unit_a].material.parts[0]
                        .0
                        .as_str();
                    let name_b = structure.units[selected.unit_b].material.parts[0]
                        .0
                        .as_str();
                    let interface =
                        crate::geometry_reference_library::resolve_live_contact_candidate(
                            name_a,
                            &structure.units[selected.unit_a],
                            name_b,
                            &structure.units[selected.unit_b],
                            selected.candidate,
                            &catalog,
                        );
                    if selected.candidate.distance <= 0.25
                        && selected.candidate.facing > 0.5
                        && interface.is_some()
                    {
                        selected_fixture = Some((base, resource.name.clone()));
                        break 'resources;
                    }
                }
            }
        }
        let (base, partner_name) = selected_fixture
            .expect("at least one high-separation base partner must expose a live contact");

        assert!(simulation.environment.field.deposit_physical_at_index(
            cell,
            compound.clone(),
            &catalog,
        ));
        assert!(simulation.environment.field.deposit_physical_at_index(
            cell,
            base.clone(),
            &catalog,
        ));
        simulation.environment.chemistry_usable_energy = 1_000.0;
        let baseline = simulation.energy_inventory_snapshot();
        simulation.energy_ledger = crate::state::EnergyLedger::default();
        simulation.energy_ledger.initial_usable_energy_held = baseline.current_usable_energy;
        simulation.energy_ledger.total_usable_energy_held = baseline.current_usable_energy;
        simulation.energy_ledger.initial_resonant_energy_held = baseline.current_resonant_energy;
        simulation.energy_ledger.total_resonant_energy_held = baseline.current_resonant_energy;
        simulation.energy_ledger.initial_structural_energy_held =
            baseline.current_structural_energy;
        simulation.energy_ledger.total_structural_energy_held = baseline.current_structural_energy;

        let mut accumulation = BTreeMap::new();
        let mut committed = false;
        for tick in 0..1_000_u64 {
            let mut discovery_work =
                crate::chemical_reaction::MAX_REACTION_CANDIDATE_INSPECTIONS_PER_TICK / 4;
            let operations = accumulate_environmental(
                &simulation.environment,
                &mut accumulation,
                &mut simulation.chemistry_library,
                tick,
                &mut discovery_work,
            );
            let mut realization_work =
                crate::chemical_reaction::MAX_REACTION_REALIZATIONS_PER_TICK / 4;
            let scheduled = schedule_environmental_operations(
                operations,
                tick,
                &mut realization_work,
                &mut accumulation,
            );
            if let Some(operation) = scheduled.into_iter().next() {
                committed = resolve_environmental(
                    operation,
                    &mut simulation.environment,
                    &mut simulation.energy_ledger,
                );
                if committed {
                    break;
                }
            }
        }
        assert!(
            committed,
            "live compound/base discovery must settle a COMBINE transaction; partner={partner_name}, accumulated={:?}",
            accumulation.values().copied().collect::<Vec<_>>()
        );

        let product = &simulation.environment.field.cells[cell].physical_materials;
        assert_eq!(
            product.len(),
            1,
            "both reactants must be replaced by one product"
        );
        let product = &product[0];
        assert_eq!(product.material.parts.len(), 3);
        assert_eq!(product.material.parts[0], ("Carbon".to_owned(), 1.0));
        assert_eq!(product.material.parts[1], ("Carbon".to_owned(), 1.0));
        assert_eq!(product.material.parts[2], (partner_name, 1.0));
        assert_eq!(product.material.internal_bonds.len(), 2);
        assert!(
            product.material.internal_bonds.iter().any(|bond| {
                (bond.part_a == 0 && bond.part_b == 1) || (bond.part_a == 1 && bond.part_b == 0)
            }),
            "the original carbon-carbon bond must remain intact"
        );
        assert!(
            product.material.internal_bonds.iter().any(|bond| {
                (bond.part_a == 0 && bond.part_b == 2)
                    || (bond.part_a == 2 && bond.part_b == 0)
                    || (bond.part_a == 1 && bond.part_b == 2)
                    || (bond.part_a == 2 && bond.part_b == 1)
            }),
            "the new base constituent must be bonded to the original molecule"
        );
        assert_eq!(product.placements.as_ref().map(Vec::len), Some(3));
        assert_eq!(product.realized_shapes.as_ref().map(Vec::len), Some(3));
        let connections = product
            .internal_connections
            .as_ref()
            .expect("product must preserve realized bond endpoints and energies");
        assert_eq!(connections.len(), 2);
        assert!(
            connections.iter().any(|bond| {
                ((bond.part_a == 0 && bond.part_b == 1) || (bond.part_a == 1 && bond.part_b == 0))
                    && (bond.bond_energy - original_bond_energy).abs() <= 1e-9
            }),
            "original bond energy must be preserved exactly"
        );

        let inventory = simulation.energy_inventory_snapshot();
        assert!(
            (simulation.energy_ledger.total_usable_energy_held - inventory.current_usable_energy)
                .abs()
                <= 1e-6,
            "usable-energy ledger must match live holders"
        );
        assert!(
            (simulation.energy_ledger.total_structural_energy_held
                - inventory.current_structural_energy)
                .abs()
                <= 1e-6,
            "structural-energy ledger must match live bonds"
        );
        assert!(
            inventory.is_balanced_without_creation(1e-6),
            "compound/base COMBINE must conserve energy; residual={:?}",
            inventory.residual()
        );
    }

    #[test]
    fn environmental_pair_combine_commits_a_realized_product() {
        let mut simulation = crate::state::Simulation::new(942, 10.0);
        let catalog = simulation.environment.catalog.clone();
        for cell in &mut simulation.environment.field.cells {
            cell.physical_materials.clear();
        }
        simulation.environment.width = 30.0;
        simulation.environment.height = 30.0;
        simulation.environment.field =
            crate::environment::ActiveMaterialField::new(30.0, 30.0, 10.0);
        let cell = simulation
            .environment
            .field
            .index_for_position(10.0, 10.0)
            .unwrap();

        let mut selected_pair = None;
        'search: for resource_a in &catalog {
            if resource_a.physical_state == crate::resources::PhysicalState::Fluid {
                continue;
            }
            for resource_b in &catalog {
                if resource_b.physical_state == crate::resources::PhysicalState::Fluid {
                    continue;
                }
                let chemistry_separation = match (
                    resource_a.properties.chemical_position,
                    resource_b.properties.chemical_position,
                ) {
                    (Some(a), Some(b)) => (a - b).abs(),
                    _ => continue,
                };
                if chemistry_separation < 9.0 {
                    continue;
                }
                for step in 1..=60 {
                    let distance = step as f64 * 0.05;
                    for rotation_step in 0..12 {
                        let rotation = rotation_step as f64 * std::f64::consts::PI / 6.0;
                        let source_a = PhysicalMaterial::realized(
                            Material::free_base(resource_a.name.clone(), 1.0),
                            vec![Placement {
                                x: 10.0,
                                y: 10.0,
                                rotation_radians: 0.0,
                            }],
                            &catalog,
                        );
                        let source_b = PhysicalMaterial::realized(
                            Material::free_base(resource_b.name.clone(), 1.0),
                            vec![Placement {
                                x: 10.0 + distance,
                                y: 10.0,
                                rotation_radians: rotation,
                            }],
                            &catalog,
                        );
                        let (Some(source_a), Some(source_b)) = (source_a, source_b) else {
                            continue;
                        };
                        let Some((mut structure, indices_a)) =
                            restored_source(&source_a, &simulation.environment)
                        else {
                            continue;
                        };
                        let Some(indices_b) = crate::material_restoration::restore_material(
                            &mut structure,
                            &source_b,
                            origin(),
                            &catalog,
                        ) else {
                            continue;
                        };
                        if indices_b.iter().any(|index| {
                            crate::construction_runtime::placed_unit_overlaps(
                                &structure,
                                &structure.units[*index],
                                &indices_b,
                                &catalog,
                            )
                        }) {
                            continue;
                        }
                        let mut cache = ConnectionCompatibilityCache::new();
                        let mut work = FIELD_COMPONENT_PAIR_LIMIT;
                        if let Some(selected) = select_best_contact(
                            &structure, &indices_a, &indices_b, &catalog, &mut cache, &mut work,
                        ) {
                            let material_name_a = structure.units[selected.unit_a]
                                .material
                                .parts
                                .first()
                                .map(|part| part.0.as_str());
                            let material_name_b = structure.units[selected.unit_b]
                                .material
                                .parts
                                .first()
                                .map(|part| part.0.as_str());
                            let live_interface = material_name_a.zip(material_name_b).and_then(
                                |(name_a, name_b)| {
                                    crate::geometry_reference_library::resolve_live_contact_candidate(
                                        name_a,
                                        &structure.units[selected.unit_a],
                                        name_b,
                                        &structure.units[selected.unit_b],
                                        selected.candidate,
                                        &catalog,
                                    )
                                },
                            );
                            if selected.candidate.distance <= 0.25
                                && selected.candidate.facing > 0.5
                                && live_interface.is_some()
                            {
                                selected_pair = Some((source_a, source_b, selected));
                                break 'search;
                            }
                        }
                    }
                }
            }
        }

        let (source_a, source_b, selected) =
            selected_pair.expect("catalog should contain a physically eligible material pair");

        // Isolate chemical activation from spatial discovery. If this direct call
        // cannot accumulate, the rejection is inside chemical_step rather than
        // the field scan or per-tick budgets.
        let (mut direct_structure, direct_indices_a) =
            restored_source(&source_a, &simulation.environment)
                .expect("selected source A must restore for diagnostic");
        let direct_indices_b = crate::material_restoration::restore_material(
            &mut direct_structure,
            &source_b,
            origin(),
            &catalog,
        )
        .expect("selected source B must restore for diagnostic");
        let mut direct_accumulation = BTreeMap::new();
        let mut direct_updated = BTreeSet::new();
        let mut direct_operations = Vec::new();
        let mut evaluation_journal = crate::environmental_evaluation::EvaluationJournal::default();
        chemical_step(
            &simulation.environment,
            &mut simulation.chemistry_library,
            &direct_structure,
            selected,
            &direct_indices_a,
            &direct_indices_b,
            &source_a,
            &source_b,
            cell,
            0,
            cell,
            1,
            &mut direct_accumulation,
            &mut direct_updated,
            &mut direct_operations,
            0,
            &mut evaluation_journal,
        );
        assert!(
            !direct_accumulation.is_empty() || !direct_operations.is_empty(),
            "direct chemical_step rejected the fixture before accumulation; selected={selected:?}, updated={direct_updated:?}"
        );

        assert!(simulation.environment.field.deposit_physical_at_index(
            cell,
            source_a.clone(),
            &catalog,
        ));
        assert!(simulation.environment.field.deposit_physical_at_index(
            cell,
            source_b.clone(),
            &catalog,
        ));
        simulation.environment.chemistry_usable_energy = 1_000.0;
        let baseline = simulation.energy_inventory_snapshot();
        simulation.energy_ledger = crate::state::EnergyLedger::default();
        simulation.energy_ledger.initial_usable_energy_held = baseline.current_usable_energy;
        simulation.energy_ledger.total_usable_energy_held = baseline.current_usable_energy;
        simulation.energy_ledger.initial_resonant_energy_held = baseline.current_resonant_energy;
        simulation.energy_ledger.total_resonant_energy_held = baseline.current_resonant_energy;
        simulation.energy_ledger.initial_structural_energy_held =
            baseline.current_structural_energy;
        simulation.energy_ledger.total_structural_energy_held = baseline.current_structural_energy;
        // Simulate an older cumulative history whose small next structural
        // increment rounds away. The live held total must still track the
        // actual COMBINE/BREAK bond mutations.
        simulation.energy_ledger.total_structural_energy_delta = -1.0e16;

        let mut accumulation = BTreeMap::new();
        let mut discovered_operation = None;
        for tick in 0..500_u64 {
            let mut discovery_work =
                crate::chemical_reaction::MAX_REACTION_CANDIDATE_INSPECTIONS_PER_TICK / 4;
            let operations = accumulate_environmental(
                &simulation.environment,
                &mut accumulation,
                &mut simulation.chemistry_library,
                tick,
                &mut discovery_work,
            );
            let mut realization_work =
                crate::chemical_reaction::MAX_REACTION_REALIZATIONS_PER_TICK / 4;
            let scheduled = schedule_environmental_operations(
                operations,
                tick,
                &mut realization_work,
                &mut accumulation,
            );
            if let Some(operation) = scheduled.into_iter().next() {
                discovered_operation = Some(operation);
                break;
            }
        }
        let operation = discovered_operation.unwrap_or_else(|| {
            panic!(
                "live environmental discovery must accumulate activation and schedule a contact pair; accumulated values={:?}",
                accumulation.values().copied().collect::<Vec<_>>()
            )
        });
        // Capture the reactant composition independently of the transaction so
        // the realized product can be checked for composition and aggregate-property
        // conservation, not only for the existence of a newly formed bond.
        let expected_material = Material {
            parts: operation
                .source_a
                .material
                .parts
                .iter()
                .chain(operation.source_b.material.parts.iter())
                .cloned()
                .collect(),
            internal_bonds: Vec::new(),
        };
        let expected_mass = expected_material.mass(&catalog);
        let expected_properties = expected_material.weighted_properties(&catalog);
        let expected_part_count = expected_material.parts.len();
        assert!(
            crate::chemistry::activated(
                operation.accumulated_reaction,
                crate::chemistry::activation_barrier_from_bond_strength(
                    crate::combine::bond_strength(
                        operation
                            .source_a
                            .material
                            .parts
                            .first()
                            .and_then(|(name, _)| {
                                simulation
                                    .environment
                                    .catalog
                                    .iter()
                                    .find(|resource| resource.name == *name)
                            })
                            .expect("discovered source A resource must exist")
                            .properties,
                        operation
                            .source_b
                            .material
                            .parts
                            .first()
                            .and_then(|(name, _)| {
                                simulation
                                    .environment
                                    .catalog
                                    .iter()
                                    .find(|resource| resource.name == *name)
                            })
                            .expect("discovered source B resource must exist")
                            .properties,
                    ),
                )
                .expect("live bond strength must define an activation barrier"),
            ),
            "the discovery pipeline must schedule only after accumulated chemistry reaches activation"
        );
        // The resolver is a transaction boundary, not merely a trusted
        // scheduler callback: an under-activated operation must leave the
        // source objects, usable holder, and ledger untouched.
        let original_materials = simulation.environment.field.cells[cell]
            .physical_materials
            .clone();
        let original_usable = simulation.environment.chemistry_usable_energy;
        let original_ledger = simulation.energy_ledger;
        let mut underactivated = operation.clone();
        underactivated.accumulated_reaction = 0.0;
        assert!(!resolve_environmental(
            underactivated,
            &mut simulation.environment,
            &mut simulation.energy_ledger,
        ));
        assert_eq!(
            simulation.environment.field.cells[cell].physical_materials,
            original_materials
        );
        assert_eq!(
            simulation.environment.chemistry_usable_energy,
            original_usable
        );
        assert_eq!(simulation.energy_ledger, original_ledger);

        assert!(resolve_environmental(
            operation,
            &mut simulation.environment,
            &mut simulation.energy_ledger,
        ));

        let product = &simulation.environment.field.cells[cell].physical_materials;
        assert_eq!(product.len(), 1);
        assert_eq!(
            product[0].material.parts, expected_material.parts,
            "COMBINE must preserve every constituent and its identity"
        );
        assert!(
            (product[0].material.mass(&catalog) - expected_mass).abs() <= 1e-9,
            "COMBINE must conserve total material mass"
        );
        assert_eq!(
            product[0].placements.as_ref().map(Vec::len),
            Some(expected_part_count),
            "the compound must retain one physical placement per constituent"
        );
        assert_eq!(
            product[0].realized_shapes.as_ref().map(Vec::len),
            Some(expected_part_count),
            "the compound must retain each constituent's realized geometry"
        );
        let aggregate_properties = product[0].material.weighted_properties(&catalog);
        assert!(
            (aggregate_properties.mass - expected_properties.mass).abs() <= 1e-9
                && (aggregate_properties.potential_energy - expected_properties.potential_energy)
                    .abs()
                    <= 1e-9
                && (aggregate_properties.reactivity - expected_properties.reactivity).abs() <= 1e-9
                && (aggregate_properties.cohesion - expected_properties.cohesion).abs() <= 1e-9,
            "compound aggregate properties must be the composition-weighted result"
        );
        assert!(
            aggregate_properties.chemical_position.is_none(),
            "a mixed compound must not be assigned an invented single chemical position"
        );
        assert_eq!(product[0].material.internal_bonds.len(), 1);
        let connections = product[0]
            .internal_connections
            .as_ref()
            .expect("combined product must retain its bond graph");
        assert_eq!(connections.len(), 1);
        assert!(connections[0].bond_energy.is_finite());
        assert!(connections[0].bond_energy > 0.0);
        let inventory = simulation.energy_inventory_snapshot();
        assert!(
            (simulation.energy_ledger.total_usable_energy_held - inventory.current_usable_energy)
                .abs()
                <= 1e-6,
            "COMBINE must keep the ledger usable-energy total aligned with its live holders"
        );
        assert!(
            (simulation.energy_ledger.total_structural_energy_held
                - inventory.current_structural_energy)
                .abs()
                <= 1e-6,
            "COMBINE must keep the ledger structural-energy total aligned with its live bonds"
        );
        let residual = inventory.residual();
        assert!(
            inventory.is_balanced_without_creation(1e-6),
            concat!(
                "environmental COMBINE must conserve the complete energy inventory; ",
                "before={:?}, after={:?}, residual={:?}; ",
                "deltas: usable={:.12}, resonant={:.12}, structural={:.12}, ",
                "external_input={:.12}, escaped_tone={:.12}, work={:.12}, heat={:.12}; ",
                "product parts={:?}, realized bond_energy={:.12}; ",
                "candidate required_energy={:.12}, investment={:.12}, distance={:.12}; ",
                "ledger totals: usable={:.12}, structural={:.12}, work={:.12}, heat={:.12}"
            ),
            baseline,
            inventory,
            residual,
            inventory.current_usable_energy - baseline.current_usable_energy,
            inventory.current_resonant_energy - baseline.current_resonant_energy,
            inventory.current_structural_energy - baseline.current_structural_energy,
            inventory.external_energy_input - baseline.external_energy_input,
            inventory.world_tone_energy_escaped - baseline.world_tone_energy_escaped,
            inventory.work_expended - baseline.work_expended,
            inventory.heat_dissipated - baseline.heat_dissipated,
            product[0]
                .material
                .parts
                .iter()
                .map(|(name, _)| name.as_str())
                .collect::<Vec<_>>(),
            connections[0].bond_energy,
            selected.required_energy,
            selected.investment,
            selected.candidate.distance,
            simulation.energy_ledger.total_usable_energy_held,
            simulation.energy_ledger.total_structural_energy_held,
            simulation.energy_ledger.total_work_expended,
            simulation.energy_ledger.total_heat_dissipated
        );

        // Carry the exact COMBINE product into the environmental BREAK path.
        // This proves the allocated bond energy survives the field-owned round
        // trip and is settled once when the product is split again.
        let combined = product[0].clone();
        let bond = combined
            .internal_connections
            .as_ref()
            .and_then(|connections| connections.first())
            .expect("combined product must expose its allocated bond")
            .clone();
        let break_operation = crate::chemical_reaction::EnvironmentalChemicalBreakOperation {
            cell_index: cell,
            material_index: 0,
            bond_index: 0,
            source_material: combined,
            bond,
            activation_signal: 1.0,
            disruption_cost: 0.5,
            reaction_frequency_cycles_per_day: 440.0,
            accumulation_key: "environmental-combine-round-trip-break".into(),
            accumulated_reaction: 1.0,
        };
        assert!(crate::chemical_reaction::resolve_environmental(
            break_operation,
            &mut simulation.environment,
            &mut simulation.energy_ledger,
        ));
        let split = &simulation.environment.field.cells[cell].physical_materials;
        assert_eq!(
            split.len(),
            2,
            "environmental BREAK must restore both source objects"
        );
        assert!(
            split.iter().all(|piece| piece
                .internal_connections
                .as_ref()
                .is_some_and(Vec::is_empty)),
            "split sources must no longer retain the realized bond"
        );
        let after_break = simulation.energy_inventory_snapshot();
        assert!(
            (simulation.energy_ledger.total_usable_energy_held - after_break.current_usable_energy)
                .abs()
                <= 1e-6,
            "BREAK must keep the ledger usable-energy total aligned with its live holders"
        );
        assert!(
            (simulation.energy_ledger.total_structural_energy_held
                - after_break.current_structural_energy)
                .abs()
                <= 1e-6,
            "BREAK must keep the ledger structural-energy total aligned with its live bonds"
        );
        assert!(
            after_break.is_balanced_without_creation(1e-6),
            "COMBINE followed by environmental BREAK must conserve the complete inventory; inventory={after_break:?}, residual={:?}",
            after_break.residual()
        );
    }

    #[test]
    fn bounded_component_contact_search_rotates_to_late_constituent_pairs() {
        let catalog = crate::resources::default_catalog();
        let mut structure = OrganismStructure::new();
        let mut indices_a = Vec::new();
        let mut indices_b = Vec::new();

        for index in 0..13 {
            indices_a.push(structure.add_unit(crate::structure::StructuralUnit::new(
                "Carbon",
                Placement {
                    x: index as f64 * 1.5,
                    y: index as f64 * 3.0_f64.sqrt() / 2.0,
                    rotation_radians: 0.0,
                },
            )));
        }
        for index in 0..13 {
            // Build a second connected carbon chain extending away from its
            // target contact with the final constituent of the first chain.
            let placement = Placement {
                x: 37.5 - index as f64 * 1.5,
                y: (index as f64 + 1.0) * 3.0_f64.sqrt() / 2.0,
                rotation_radians: 0.0,
            };
            indices_b.push(
                structure.add_unit(crate::structure::StructuralUnit::new("Carbon", placement)),
            );
        }

        let mut bond_cache = ConnectionCompatibilityCache::new();
        for indices in [&indices_a, &indices_b] {
            for pair in indices.windows(2) {
                let candidates = crate::contact::connection_pair_candidates_cached(
                    &structure,
                    pair[0],
                    pair[1],
                    &catalog,
                    &mut bond_cache,
                );
                let candidate = candidates.into_iter().next().expect(
                    "adjacent carbon constituents must expose a bondable physical interface",
                );
                let id_a = structure
                    .physical_id(pair[0])
                    .expect("first carbon constituent must have a stable physical ID");
                let id_b = structure
                    .physical_id(pair[1])
                    .expect("second carbon constituent must have a stable physical ID");
                structure.push_bond_unchecked(crate::structure::Bond {
                    endpoint_a: crate::structure::BondEndpoint::new(id_a, candidate.endpoint_a),
                    endpoint_b: crate::structure::BondEndpoint::new(id_b, candidate.endpoint_b),
                    strength: 0.5,
                    bond_energy: 1.0,
                });
            }
        }
        assert_eq!(structure.bonds.len(), 24);

        let pair_count = indices_a.len() * indices_b.len();
        assert_eq!(pair_count, 169);
        let mut cache = ConnectionCompatibilityCache::new();
        let mut first_window_work = FIELD_COMPONENT_PAIR_LIMIT;
        let first_window = select_best_contact_from_offset(
            &structure,
            &indices_a,
            &indices_b,
            &catalog,
            &mut cache,
            component_pair_start(0, pair_count),
            &mut first_window_work,
            None,
        );
        assert!(
            first_window.is_empty(),
            "the first bounded row-major window must not accidentally inspect the final constituent pair"
        );
        assert_eq!(first_window_work, 0);

        let mut second_window_work = FIELD_COMPONENT_PAIR_LIMIT;
        let mut search_diagnostics = crate::state::EnvironmentalCombineSearchDiagnostics::default();
        let late_window = select_best_contact_from_offset(
            &structure,
            &indices_a,
            &indices_b,
            &catalog,
            &mut cache,
            component_pair_start(41, pair_count),
            &mut second_window_work,
            Some(&mut search_diagnostics),
        );
        assert!(
            late_window.iter().any(|selected| {
                selected.unit_a == indices_a[12] && selected.unit_b == indices_b[12]
            }),
            "the rotated window must retain the late physical contact within the cycle"
        );
        assert_eq!(second_window_work, 0);
        assert_eq!(
            search_diagnostics.constituent_pairs_inspected,
            FIELD_COMPONENT_PAIR_LIMIT
        );
        assert!(
            search_diagnostics.constituent_pairs_with_valid_interfaces > 0,
            "the rotated window must report its physically valid constituent pair"
        );
        assert_eq!(
            search_diagnostics.constituent_pairs_suppressed_by_global_selection, 0,
            "every inspected constituent pair with a valid interface must reach chemistry"
        );
        assert_eq!(
            search_diagnostics.interfaces_suppressed_by_global_selection, 0,
            "no physically valid interface may be suppressed by a single-contact selector"
        );
        assert_eq!(
            search_diagnostics.interfaces_sent_to_chemistry,
            search_diagnostics.physically_evaluable_interfaces,
            "every physically evaluable interface must receive an independent chemistry step"
        );

        let varied_budget_pair_count = 1_000;
        let visited_starts: HashSet<_> = (0..varied_budget_pair_count)
            .map(|tick| component_pair_start(tick as u64, varied_budget_pair_count))
            .collect();
        assert_eq!(
            visited_starts.len(),
            varied_budget_pair_count,
            "the rotating start must visit every pair even when the available work budget changes"
        );
    }

    #[test]
    fn heterogeneous_compound_contacts_retain_constituent_chemical_identity() {
        let simulation = crate::state::Simulation::new(23729, 10.0);
        let catalog = simulation.environment.catalog.clone();
        let mut compound_fixture = None;

        'constituents: for resource_a in &catalog {
            if resource_a.physical_state == crate::resources::PhysicalState::Fluid {
                continue;
            }
            for resource_b in &catalog {
                if resource_b.physical_state == crate::resources::PhysicalState::Fluid
                    || resource_a.name == resource_b.name
                    || resource_a.properties.chemical_position
                        == resource_b.properties.chemical_position
                {
                    continue;
                }
                for step in 1..=60 {
                    let distance = step as f64 * 0.05;
                    for rotation_step in 0..12 {
                        let angle = rotation_step as f64 * std::f64::consts::PI / 6.0;
                        let (sin, cos) = angle.sin_cos();
                        let candidate = PhysicalMaterial::realized(
                            Material {
                                parts: vec![
                                    (resource_a.name.clone(), 1.0),
                                    (resource_b.name.clone(), 1.0),
                                ],
                                internal_bonds: vec![InternalBond {
                                    part_a: 0,
                                    part_b: 1,
                                }],
                            },
                            vec![
                                Placement {
                                    x: 10.0,
                                    y: 10.0,
                                    rotation_radians: 0.0,
                                },
                                Placement {
                                    x: 10.0 + distance * cos,
                                    y: 10.0 + distance * sin,
                                    rotation_radians: angle,
                                },
                            ],
                            &catalog,
                        );
                        let Some(candidate) = candidate else {
                            continue;
                        };
                        if let Some((structure, indices)) =
                            restored_source(&candidate, &simulation.environment)
                        {
                            if indices.len() == 2 && structure.bonds.len() == 1 {
                                compound_fixture = Some(candidate);
                                break 'constituents;
                            }
                        }
                    }
                }
            }
        }

        let compound = compound_fixture
            .expect("catalog must provide a physically realizable heterogeneous bonded pair");
        assert_eq!(compound.material.parts.len(), 2);
        assert_ne!(
            compound.material.parts[0].0, compound.material.parts[1].0,
            "the characterization fixture must contain two distinct resources"
        );
        assert_eq!(
            compound
                .material
                .weighted_properties(&catalog)
                .chemical_position,
            None,
            "a heterogeneous molecule must not have one averaged chemical position"
        );

        let (compound_structure, compound_indices) =
            restored_source(&compound, &simulation.environment)
                .expect("heterogeneous compound must restore as constituent units");
        assert_eq!(compound_indices.len(), 2);
        assert_eq!(compound_structure.bonds.len(), 1);

        let mut selected_fixture = None;
        'external: for resource in &catalog {
            if resource.physical_state == crate::resources::PhysicalState::Fluid {
                continue;
            }
            for step in 1..=80 {
                let distance = step as f64 * 0.05;
                for rotation_step in 0..12 {
                    let angle = rotation_step as f64 * std::f64::consts::PI / 6.0;
                    let base = PhysicalMaterial::realized(
                        Material::free_base(resource.name.clone(), 1.0),
                        vec![Placement {
                            x: 10.0 + distance,
                            y: 10.0,
                            rotation_radians: angle,
                        }],
                        &catalog,
                    );
                    let Some(base) = base else {
                        continue;
                    };
                    let (mut structure, indices_a) =
                        restored_source(&compound, &simulation.environment)
                            .expect("heterogeneous compound must restore for each contact pose");
                    let Some(indices_b) = crate::material_restoration::restore_material(
                        &mut structure,
                        &base,
                        origin(),
                        &catalog,
                    ) else {
                        continue;
                    };
                    if indices_b.iter().any(|index| {
                        crate::construction_runtime::placed_unit_overlaps(
                            &structure,
                            &structure.units[*index],
                            &indices_b,
                            &catalog,
                        )
                    }) {
                        continue;
                    }
                    let mut cache = ConnectionCompatibilityCache::new();
                    let mut work = FIELD_COMPONENT_PAIR_LIMIT;
                    let Some(selected) = select_best_contact(
                        &structure, &indices_a, &indices_b, &catalog, &mut cache, &mut work,
                    ) else {
                        continue;
                    };
                    let interface =
                        crate::geometry_reference_library::resolve_live_contact_candidate(
                            structure.units[selected.unit_a].material.parts[0]
                                .0
                                .as_str(),
                            &structure.units[selected.unit_a],
                            structure.units[selected.unit_b].material.parts[0]
                                .0
                                .as_str(),
                            &structure.units[selected.unit_b],
                            selected.candidate,
                            &catalog,
                        );
                    if selected.candidate.distance > 0.25
                        || selected.candidate.facing <= 0.5
                        || interface.is_none()
                    {
                        continue;
                    }
                    let Some(external_position) = catalog
                        .iter()
                        .find(|candidate| candidate.name == resource.name)
                        .and_then(|candidate| candidate.properties.chemical_position)
                    else {
                        continue;
                    };
                    let mut interface_potentials = indices_a.iter().filter_map(|index| {
                        let position =
                            constituent_chemical_position(&structure.units[*index], &catalog)?;
                        crate::chemistry::interaction_potential(
                            position,
                            external_position,
                            crate::chemistry::CHEMICAL_K,
                            crate::chemistry::CHEMICAL_D_MAX,
                        )
                    });
                    let (Some(first_potential), Some(second_potential)) =
                        (interface_potentials.next(), interface_potentials.next())
                    else {
                        continue;
                    };
                    if first_potential <= 0.0
                        || second_potential <= 0.0
                        || (first_potential - second_potential).abs() <= f64::EPSILON
                    {
                        continue;
                    }
                    selected_fixture = Some((structure, indices_a, indices_b, base, selected));
                    break 'external;
                }
            }
        }

        let (structure, compound_indices, external_indices, external_material, selected) =
            selected_fixture.expect(
                "heterogeneous molecule must expose a physically eligible constituent interface",
            );
        assert!(compound_indices.contains(&selected.unit_a));
        let [(selected_name, _)] = structure.units[selected.unit_a].material.parts.as_slice()
        else {
            panic!("selected molecule contact must resolve to one physical constituent");
        };
        assert!(
            compound
                .material
                .parts
                .iter()
                .any(|(name, _)| name == selected_name),
            "selected contact identity must be one of the molecule's original constituents"
        );
        let selected_resource = catalog
            .iter()
            .find(|resource| resource.name == *selected_name)
            .expect("selected constituent must exist in the base catalog");
        let selected_unit = &structure.units[selected.unit_a];
        let selected_properties = selected_unit
            .properties(&catalog)
            .expect("selected constituent aggregate properties must remain available");
        assert_eq!(
            selected_properties.chemical_position, None,
            "weighted properties must not manufacture a molecule-wide chemical position"
        );
        assert_eq!(
            constituent_chemical_position(selected_unit, &catalog),
            selected_resource.properties.chemical_position,
            "the chemistry path must resolve the contacting constituent's catalog position"
        );

        let external_name = structure.units[selected.unit_b].material.parts[0]
            .0
            .as_str();
        let external_position = catalog
            .iter()
            .find(|resource| resource.name == external_name)
            .and_then(|resource| resource.properties.chemical_position)
            .expect("external contact constituent must have a chemical position");
        let interface_potentials: Vec<_> = compound_indices
            .iter()
            .filter_map(|index| {
                let unit = &structure.units[*index];
                let position = constituent_chemical_position(unit, &catalog)?;
                crate::chemistry::interaction_potential(
                    position,
                    external_position,
                    crate::chemistry::CHEMICAL_K,
                    crate::chemistry::CHEMICAL_D_MAX,
                )
            })
            .collect();
        assert_eq!(
            interface_potentials.len(),
            2,
            "both bonded constituents must independently expose chemistry identities"
        );
        assert_ne!(
            interface_potentials[0], interface_potentials[1],
            "different constituent interfaces must produce distinct interaction potentials against the same external material"
        );

        // Exercise the actual environmental chemistry path, not just the
        // constituent lookup helper. Its persisted key must carry the exact
        // contacting constituent positions and its accumulation must be live.
        let temporary_store = std::env::temp_dir().join(format!(
            "evosim-heterogeneous-interface-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock must be after the Unix epoch")
                .as_nanos()
        ));
        let mut chemistry_library =
            crate::chemistry_library::ChemistryLibrary::open(&temporary_store)
                .expect("test chemistry library must open");
        let mut accumulation = BTreeMap::new();
        let mut updated_keys = BTreeSet::new();
        let mut operations = Vec::new();
        let mut evaluation_journal = crate::environmental_evaluation::EvaluationJournal::default();
        chemical_step(
            &simulation.environment,
            &mut chemistry_library,
            &structure,
            selected,
            &compound_indices,
            &external_indices,
            &compound,
            &external_material,
            0,
            0,
            0,
            1,
            &mut accumulation,
            &mut updated_keys,
            &mut operations,
            0,
            &mut evaluation_journal,
        );
        assert_eq!(
            chemistry_library.len(),
            1,
            "the live chemistry step must cache one canonical static-potential pair"
        );
        assert!(
            !accumulation.is_empty() || !operations.is_empty(),
            "a positive, eligible heterogeneous interface must advance chemistry or schedule its activated transaction"
        );
        let interface = crate::geometry_reference_library::resolve_live_contact_candidate(
            selected_name,
            selected_unit,
            external_name,
            &structure.units[selected.unit_b],
            selected.candidate,
            &catalog,
        )
        .expect("the selected live interface must remain resolvable");
        assert!(
            !interface.signature.is_empty(),
            "live geometry identity must resolve independently of the static-potential cache"
        );
        let selected_position = constituent_chemical_position(selected_unit, &catalog)
            .expect("selected constituent must retain its chemical position");
        let external_position =
            constituent_chemical_position(&structure.units[selected.unit_b], &catalog)
                .expect("external constituent must retain its chemical position");
        let chemistry_key = chemistry_library.key_from_chemical_pair(
            selected_name,
            external_name,
            selected_position,
            external_position,
        );
        let cached = chemistry_library
            .get(&chemistry_key)
            .expect("chemical step must reuse the canonical static-potential cache key");
        let expected_potential = crate::chemistry::interaction_potential(
            selected_position,
            external_position,
            crate::chemistry::CHEMICAL_K,
            crate::chemistry::CHEMICAL_D_MAX,
        )
        .expect("catalog positions must produce a valid interaction potential");
        assert!(
            (cached.static_potential - expected_potential).abs() <= 1e-12,
            "cached chemistry must use the contacting constituent's position, not an aggregate value"
        );
        let _ = std::fs::remove_dir_all(temporary_store);
    }

    #[test]
    fn environmental_product_preserves_allocated_internal_bond_energy() {
        let catalog = crate::resources::default_catalog();
        let mut structure = OrganismStructure::new();
        let carbon = crate::structure::StructuralUnit::from_material(
            Material::free_base("Carbon", 1.0),
            Placement {
                x: 1.0,
                y: 2.0,
                rotation_radians: 0.0,
            },
        )
        .unwrap();
        let hydrogen = crate::structure::StructuralUnit::from_material(
            Material::free_base("Carbon", 1.0),
            Placement {
                // Carbon has a vertex on +x, so its horizontal axial neighbor
                // is offset by (1.5, sqrt(3)/2). Using x=2.5 with the same y
                // overlaps the hexagons instead of producing a valid edge contact.
                // Keep the regression focused on allocated bond-energy persistence.
                x: 2.5,
                y: 2.0 + 3.0_f64.sqrt() / 2.0,
                rotation_radians: 0.0,
            },
        )
        .unwrap();
        let a = structure.add_unit(carbon);
        let b = structure.add_unit(hydrogen);
        let id_a = structure.physical_id(a).unwrap();
        let id_b = structure.physical_id(b).unwrap();
        let candidate = crate::contact::connection_pair_candidates(&structure, a, b, &catalog)
            .into_iter()
            .find(|candidate| candidate.available_a && candidate.available_b)
            .expect("carbon-hydrogen pair should expose a compatible contact");
        let bond = crate::structure::Bond {
            endpoint_a: crate::structure::BondEndpoint::new(id_a, candidate.endpoint_a),
            endpoint_b: crate::structure::BondEndpoint::new(id_b, candidate.endpoint_b),
            strength: 0.5,
            bond_energy: 2.5,
        };
        assert!(
            crate::contact::try_add_bond(&mut structure, bond, &catalog).is_ok(),
            "valid realized contact should accept the test bond"
        );
        let Some(physical) = physical_material_from_structure(&structure, &catalog) else {
            panic!("newly formed physical compound must retain its bond");
        };
        assert_eq!(
            physical
                .internal_connections
                .as_ref()
                .unwrap()
                .iter()
                .map(|bond| bond.bond_energy)
                .sum::<f64>(),
            2.5
        );
        // Exercise the actual storage path, not only a direct serialization-
        // shaped handoff: allocated bond energy and endpoint identity must
        // survive conversion to the intrinsic storage frame and retrieval.
        let mut storage = crate::material_storage::MaterialStorage::default();
        assert!(storage.store_physical_instance(physical.clone(), &catalog));
        let stored = storage
            .take_matching_physical(&physical.material, &catalog)
            .expect("stored product must retain its realized physical instance");
        assert_eq!(
            stored
                .internal_connections
                .as_ref()
                .unwrap()
                .iter()
                .map(|bond| bond.bond_energy)
                .sum::<f64>(),
            2.5
        );

        let mut restored = OrganismStructure::new();
        let restored_indices = crate::material_restoration::restore_material(
            &mut restored,
            &stored,
            origin(),
            &catalog,
        )
        .expect("stored product must restore its physical bond graph");
        assert_eq!(restored_indices.len(), 2);
        assert_eq!(restored.bonds.len(), 1);
        assert_eq!(restored.bonds[0].bond_energy, 2.5);
        assert_eq!(
            crate::energy_ledger::structural_potential(&restored),
            Some(-2.5),
            "restoration must preserve the same signed structural potential"
        );
    }
}

#[cfg(test)]
mod shared_inventory_adapter_tests {
    use super::*;
    use crate::environmental_interaction::{
        EnvironmentalInteractionInventoryBuilder, MaterialLocator,
    };
    use crate::resources::Material;
    use crate::structure::ConnectionEndpoint;

    fn source(identity: u64, name: &str) -> PhysicalMaterial {
        PhysicalMaterial {
            material: Material::free_base(name.to_owned(), 1.0),
            environmental_identity: identity,
            placements: None,
            internal_connections: None,
            owner_relative_origin: None,
            realized_shapes: None,
        }
    }

    fn discovered(endpoint_a: usize, endpoint_b: usize) -> DiscoveredContact {
        DiscoveredContact {
            unit_a: 0,
            unit_b: 1,
            candidate: ConnectionPairCandidate {
                endpoint_a: ConnectionEndpoint::Corner {
                    point_index: endpoint_a,
                },
                endpoint_b: ConnectionEndpoint::Corner {
                    point_index: endpoint_b,
                },
                distance: 0.25,
                facing: 0.8,
                load_a: 0.0,
                load_b: 0.0,
                available_a: true,
                available_b: true,
            },
        }
    }

    #[test]
    fn combine_adapter_preserves_all_geometry_contacts_before_chemistry_evaluation() {
        let source_a = source(101, "Carbon");
        let source_b = source(202, "Oxygen");
        let indices_a = [0];
        let indices_b = [1];
        let mut builder = EnvironmentalInteractionInventoryBuilder::new(11);
        let inserted = append_discovered_contacts_to_inventory(
            &mut builder,
            &[discovered(0, 0), discovered(1, 0)],
            &indices_a,
            &indices_b,
            &source_a,
            &source_b,
            MaterialLocator {
                cell_index: 3,
                material_index: 1,
            },
            MaterialLocator {
                cell_index: 4,
                material_index: 2,
            },
        )
        .unwrap();
        let inventory = builder.finish();
        assert_eq!(inserted, 2);
        assert_eq!(inventory.contact_interfaces.len(), 2);
        assert_ne!(
            inventory.contact_interfaces[0].endpoint_a,
            inventory.contact_interfaces[1].endpoint_a
        );
        assert!(inventory.contact_interfaces.iter().all(|contact| {
            contact.context == crate::environmental_interaction::ContactContext::Ordinary
        }));
    }

    #[test]
    fn neutral_discovery_adapter_captures_contacts_before_combine_evaluation() {
        let catalog = crate::resources::default_catalog();
        let mut structure = OrganismStructure::new();
        let mut indices_a = Vec::new();
        let mut indices_b = Vec::new();
        for index in 0..13 {
            indices_a.push(structure.add_unit(crate::structure::StructuralUnit::new(
                "Carbon",
                Placement {
                    x: index as f64 * 1.5,
                    y: index as f64 * 3.0_f64.sqrt() / 2.0,
                    rotation_radians: 0.0,
                },
            )));
        }
        for index in 0..13 {
            indices_b.push(structure.add_unit(crate::structure::StructuralUnit::new(
                "Carbon",
                Placement {
                    x: 37.5 - index as f64 * 1.5,
                    y: (index as f64 + 1.0) * 3.0_f64.sqrt() / 2.0,
                    rotation_radians: 0.0,
                },
            )));
        }
        let mut source_a = source(301, "Carbon");
        let mut source_b = source(302, "Carbon");
        source_a.material.parts = vec![("Carbon".to_owned(), 13.0)];
        source_b.material.parts = vec![("Carbon".to_owned(), 13.0)];

        let mut builder = EnvironmentalInteractionInventoryBuilder::new(12);
        let mut remaining_work = FIELD_COMPONENT_PAIR_LIMIT;
        let (inserted, inspected_pairs) = discover_and_append_contacts_to_inventory(
            &mut builder,
            &structure,
            &indices_a,
            &indices_b,
            &catalog,
            component_pair_start(41, indices_a.len() * indices_b.len()),
            &mut remaining_work,
            &source_a,
            &source_b,
            MaterialLocator {
                cell_index: 1,
                material_index: 0,
            },
            MaterialLocator {
                cell_index: 2,
                material_index: 0,
            },
        )
        .unwrap();
        let inventory = builder.finish();

        // Compare unique physical interfaces, not duplicate endpoint proposals.
        // The inventory intentionally deduplicates tolerance-equivalent endpoints.
        let mut inventory_evaluable: Vec<(usize, usize, ConnectionPairCandidate)> = Vec::new();
        for contact in &inventory.contact_interfaces {
            let Some(&unit_a) = indices_a.get(contact.constituent_a.local_index) else {
                continue;
            };
            let Some(&unit_b) = indices_b.get(contact.constituent_b.local_index) else {
                continue;
            };
            if crate::combine_runtime::selected_candidate_evaluation(
                &structure,
                unit_a,
                unit_b,
                contact.candidate,
                &catalog,
            )
            .is_none()
            {
                continue;
            }
            let duplicate = inventory_evaluable
                .iter()
                .any(|(existing_a, existing_b, existing)| {
                    *existing_a == unit_a
                        && *existing_b == unit_b
                        && crate::contact::connection_pair_candidate_matches_endpoints(
                            existing,
                            contact.endpoint_a,
                            contact.endpoint_b,
                        )
                });
            if !duplicate {
                inventory_evaluable.push((unit_a, unit_b, contact.candidate));
            }
        }

        let mut production_work = FIELD_COMPONENT_PAIR_LIMIT;
        let mut production_cache = ConnectionCompatibilityCache::new();
        let production_contacts = select_best_contact_from_offset(
            &structure,
            &indices_a,
            &indices_b,
            &catalog,
            &mut production_cache,
            component_pair_start(41, indices_a.len() * indices_b.len()),
            &mut production_work,
            None,
        );
        let mut production_evaluable: Vec<(usize, usize, ConnectionPairCandidate)> = Vec::new();
        for contact in &production_contacts {
            let duplicate =
                production_evaluable
                    .iter()
                    .any(|(existing_a, existing_b, existing)| {
                        *existing_a == contact.unit_a
                            && *existing_b == contact.unit_b
                            && crate::contact::connection_pair_candidate_matches_endpoints(
                                existing,
                                contact.candidate.endpoint_a,
                                contact.candidate.endpoint_b,
                            )
                    });
            if !duplicate {
                production_evaluable.push((contact.unit_a, contact.unit_b, contact.candidate));
            }
        }
        assert_eq!(inventory_evaluable.len(), production_evaluable.len());
        assert!(inventory_evaluable
            .iter()
            .all(|(unit_a, unit_b, candidate)| {
                production_evaluable
                    .iter()
                    .any(|(production_a, production_b, production)| {
                        *production_a == *unit_a
                            && *production_b == *unit_b
                            && crate::contact::connection_pair_candidate_matches_endpoints(
                                production,
                                candidate.endpoint_a,
                                candidate.endpoint_b,
                            )
                    })
            }));
        assert_eq!(inspected_pairs, FIELD_COMPONENT_PAIR_LIMIT);
        assert_eq!(remaining_work, 0);
        assert!(inserted > 0);
        assert_eq!(inventory.contact_interfaces.len(), inserted);
        assert_eq!(inventory.work.material_pair_visits, 1);
        assert_eq!(
            inventory.work.constituent_pair_visits,
            FIELD_COMPONENT_PAIR_LIMIT as u64
        );
        assert_eq!(
            inventory.work.geometry_evaluation_operations,
            inventory.work.raw_endpoint_candidates
        );
        assert!(inventory.work.raw_endpoint_candidates >= inserted as u64);
        assert!(inventory.contact_interfaces.iter().all(|contact| {
            contact.constituent_a.local_index < indices_a.len()
                && contact.constituent_b.local_index < indices_b.len()
        }));
    }
}
