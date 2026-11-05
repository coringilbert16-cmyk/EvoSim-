//! Preflight evaluation for a coupled environmental displacement reaction.
//!
//! This module deliberately evaluates, but does not commit, a proposed
//! A-B + C -> A-C + B replacement. Runtime integration must supply activation
//! accumulated from C's validated physical contact with A. Keeping that signal
//! explicit prevents an unrelated BREAK followed by COMBINE from masquerading
//! as displacement.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use crate::contact::{ConnectionCompatibilityCache, ConnectionPairCandidate};
use crate::physical_material::{PhysicalMaterial, PhysicalMaterialBond};
use crate::state::{EnergyLedger, Environment};

use crate::combine::{bond_strength, intrinsic_bond_potential};
use crate::energy_ledger::{EnergyLedgerAuthority, EnergyReason, ReactionEnergyBalance};
use crate::resources::{BaseResource, ResourceProperties};
use crate::structure::{Bond, OrganismStructure};

/// Energy and activation assessment for one proposed constituent replacement.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplacementPreflight {
    /// Activation supplied by the incoming constituent's contact with the anchor.
    pub(crate) incoming_activation: f64,
    /// Existing A-B bond's disruption threshold.
    pub(crate) disruption_barrier: f64,
    /// Stored potential magnitude of the existing A-B bond.
    pub(crate) old_bond_energy: f64,
    /// Stored potential magnitude of the proposed A-C bond.
    pub(crate) replacement_bond_energy: f64,
    /// Existing BREAK authority's physical disruption work requirement.
    pub(crate) disruption_work: f64,
    /// Complete signed bond-graph energy budget for the replacement.
    pub(crate) energy: ReactionEnergyBalance,
}

impl DisplacementPreflight {
    pub(crate) fn is_activated(self) -> bool {
        self.incoming_activation > 0.0
            && crate::chemistry::activated(self.incoming_activation, self.disruption_barrier)
    }

    #[cfg(test)]
    pub(crate) fn is_energy_favorable_without_external_input(self) -> bool {
        self.energy.required_external_input <= 1e-12
    }

    #[cfg(test)]
    pub(crate) fn can_commit(self, legitimate_external_input: f64) -> bool {
        self.is_activated() && self.energy.can_commit_with(legitimate_external_input)
    }
}

/// Accumulate one tick of activation from the incoming constituent's live
/// contact with the anchor. This mirrors environmental COMBINE's approved
/// static-potential, contact-falloff, facing-engagement, and dissipation equation.
fn displacement_chemical_properties(
    unit: &crate::structure::StructuralUnit,
    catalog: &[BaseResource],
) -> Option<ResourceProperties> {
    let mut properties = unit.properties(catalog)?;
    if let [(name, _)] = unit.material.parts.as_slice() {
        properties.chemical_position = catalog
            .iter()
            .find(|resource| &resource.name == name)
            .and_then(|resource| resource.properties.chemical_position);
    }
    Some(properties)
}

/// Accumulate one tick of activation from the incoming constituent's live
/// contact with the anchor. Elemental chemical positions are read from the
/// catalog here, without changing aggregate material properties or assigning
/// an averaged coordinate to a mixed compound.
pub(crate) fn accumulate_incoming_activation(
    anchor: &crate::structure::StructuralUnit,
    incoming: &crate::structure::StructuralUnit,
    candidate: ConnectionPairCandidate,
    previous: f64,
    catalog: &[BaseResource],
) -> Option<f64> {
    if candidate.distance < 0.0
        || candidate.distance > crate::combine_runtime::COMBINE_CONTACT_TOLERANCE
        || !candidate.available_a
        || !candidate.available_b
    {
        return None;
    }
    let anchor = displacement_chemical_properties(anchor, catalog)?;
    let incoming = displacement_chemical_properties(incoming, catalog)?;
    let interaction = crate::combine::chemical_interaction(
        anchor,
        incoming,
        candidate,
        crate::chemistry::CHEMICAL_K,
        crate::chemistry::CHEMICAL_D_MAX,
        crate::chemistry::CHEMICAL_CONTACT_RADIUS,
        crate::chemistry::CHEMICAL_MAX_FORCE,
    )?;
    let contact = crate::chemistry::contact_factor(
        candidate.distance,
        crate::chemistry::CHEMICAL_CONTACT_RADIUS,
    )?;
    let engagement = crate::chemistry::interface_engagement(candidate.facing)?;
    crate::chemistry::accumulate_reaction(
        previous,
        interaction.static_potential,
        contact,
        engagement,
        crate::chemistry::CHEMICAL_DISSIPATION,
    )
}

/// Fully evaluated trial topology and its independently exported physical products.
/// Nothing in this value has been committed to the live environment.
#[derive(Clone, Debug)]
pub(crate) struct DisplacementTrial {
    #[cfg(test)]
    pub(crate) structure: OrganismStructure,
    #[cfg(test)]
    source_units: Vec<crate::structure::StructuralUnit>,
    #[cfg(test)]
    source_bonds: Vec<Bond>,
    pub(crate) products: Vec<crate::physical_material::PhysicalMaterial>,
    #[cfg(test)]
    pub(crate) removed_bond: Bond,
    #[cfg(test)]
    pub(crate) replacement_bond: Bond,
    pub(crate) energy: ReactionEnergyBalance,
    pub(crate) formation_work: f64,
    pub(crate) disruption_work: f64,
}

/// Build the proposed topology on a clone. The caller must discover the supplied
/// replacement candidate after the old A-B bond has been removed from an
/// equivalent trial graph, so the freed endpoint is available to contact search.
pub(crate) fn build_displacement_trial(
    source: &OrganismStructure,
    anchor_unit: usize,
    displaced_unit: usize,
    incoming_unit: usize,
    old_bond: Bond,
    replacement_candidate: ConnectionPairCandidate,
    investment: f64,
    incoming_activation: f64,
    reactivity_baseline: f64,
    dissipated_heat: f64,
    catalog: &[BaseResource],
) -> Option<DisplacementTrial> {
    if anchor_unit >= source.units.len()
        || displaced_unit >= source.units.len()
        || incoming_unit >= source.units.len()
        || anchor_unit == displaced_unit
        || anchor_unit == incoming_unit
        || displaced_unit == incoming_unit
    {
        return None;
    }

    let anchor_id = source.physical_id(anchor_unit)?;
    let displaced_id = source.physical_id(displaced_unit)?;
    let old_connects_expected_pair = (old_bond.endpoint_a.constituent_id == anchor_id
        && old_bond.endpoint_b.constituent_id == displaced_id)
        || (old_bond.endpoint_b.constituent_id == anchor_id
            && old_bond.endpoint_a.constituent_id == displaced_id);
    if !old_connects_expected_pair
        || !source
            .bonds
            .iter()
            .any(|bond| bond.has_same_identity(&old_bond))
    {
        return None;
    }

    // A displacement must occupy the endpoint actually freed by A-B. Merely
    // forming A-C at some other available site while independently removing
    // A-B would be a BREAK plus COMBINE, not a physical replacement.
    let freed_anchor_endpoint = if old_bond.endpoint_a.constituent_id == anchor_id {
        old_bond.endpoint_a.location
    } else {
        old_bond.endpoint_b.location
    };
    if replacement_candidate.endpoint_a != freed_anchor_endpoint {
        return None;
    }

    let anchor = source.units[anchor_unit].properties(catalog)?;
    let displaced = source.units[displaced_unit].properties(catalog)?;
    let incoming = source.units[incoming_unit].properties(catalog)?;
    let old_strength = old_bond.strength;
    let preflight = evaluate_displacement_preflight(
        anchor,
        displaced,
        incoming,
        old_strength,
        incoming_activation,
        reactivity_baseline,
        0.0,
        dissipated_heat,
    )?;
    if !preflight.is_activated() || (old_bond.bond_energy - preflight.old_bond_energy).abs() > 1e-9
    {
        return None;
    }

    let mut trial = source.clone();
    trial.break_matching_bond(old_bond)?;

    // Revalidate the replacement contact and chemistry against the trial graph,
    // where the old endpoint is now genuinely available.
    let (formation, formation_work, required_investment, _) =
        crate::combine_runtime::selected_candidate_evaluation(
            &trial,
            anchor_unit,
            incoming_unit,
            replacement_candidate,
            catalog,
        )?;
    if !crate::combine::formation_succeeds(formation, investment)
        || (required_investment - investment).abs() > 1e-9
    {
        return None;
    }

    let anchor_id = trial.physical_id(anchor_unit)?;
    let incoming_id = trial.physical_id(incoming_unit)?;
    if anchor_id != source.physical_id(anchor_unit)?
        || incoming_id != source.physical_id(incoming_unit)?
    {
        return None;
    }
    let replacement_bond = crate::combine_runtime::admit_selected_candidate_to_trial_structure(
        &mut trial,
        anchor_unit,
        incoming_unit,
        replacement_candidate,
        catalog,
    )?;

    let reactant_structural = crate::energy_ledger::structural_potential(source)?;
    let product_structural = crate::energy_ledger::structural_potential(&trial)?;
    let disruption_work = preflight.disruption_work;
    let total_work = formation_work + disruption_work;
    let energy = ReactionEnergyBalance::calculate(
        reactant_structural,
        product_structural,
        total_work,
        dissipated_heat,
    )?;
    let products = crate::environmental_combine::physical_materials_from_structure_components(
        &trial, catalog,
    )?;
    if products.len() < 2 {
        return None;
    }

    Some(DisplacementTrial {
        #[cfg(test)]
        structure: trial,
        #[cfg(test)]
        source_units: source.units.clone(),
        #[cfg(test)]
        source_bonds: source.bonds.clone(),
        products,
        #[cfg(test)]
        removed_bond: old_bond,
        #[cfg(test)]
        replacement_bond,
        energy,
        formation_work,
        disruption_work,
    })
}

/// Commit a previously evaluated trial against the unchanged source graph.
/// Energy settlement is staged first; the graph, ledger, and usable holder are
/// updated only after every validation and affordability check succeeds.
#[cfg(test)]
pub(crate) fn commit_displacement_trial(
    source: &mut OrganismStructure,
    trial: DisplacementTrial,
    ledger: &mut crate::state::EnergyLedger,
    usable_energy: &mut f64,
) -> Option<Vec<crate::physical_material::PhysicalMaterial>> {
    if source.units != trial.source_units || source.bonds != trial.source_bonds {
        return None;
    }

    let transaction = trial.energy.settlement_transaction(EnergyReason::Combine)?;
    let mut staged_ledger = *ledger;
    let mut staged_usable_energy = *usable_energy;
    if !staged_ledger.settle_transaction(&mut staged_usable_energy, transaction) {
        return None;
    }
    let usable_delta = staged_usable_energy - *usable_energy;
    let next_total_usable = staged_ledger.total_usable_energy_held + usable_delta;
    let next_total_structural =
        staged_ledger.total_structural_energy_held + trial.energy.net_reaction_change;
    if !usable_delta.is_finite()
        || !next_total_usable.is_finite()
        || next_total_usable < 0.0
        || !next_total_structural.is_finite()
    {
        return None;
    }
    staged_ledger.total_usable_energy_held = next_total_usable;
    staged_ledger.total_structural_energy_held = next_total_structural;

    *source = trial.structure;
    *ledger = staged_ledger;
    *usable_energy = staged_usable_energy;
    Some(trial.products)
}

/// Evaluate the coupled A-B -> A-C replacement without mutating any graph or
/// energy holder. incoming_activation must be accumulated from C's own
/// contact/engagement with A; it must not be copied from the old A-B bond.
///
/// The structural-energy terms use the same intrinsic bond-potential authority
/// as ordinary COMBINE. Physical work and heat are included once in the single
/// net reaction balance. This function does not claim to model unrepresented
/// chemical free-energy terms.
pub(crate) fn evaluate_displacement_preflight(
    anchor: ResourceProperties,
    displaced: ResourceProperties,
    incoming: ResourceProperties,
    old_bond_strength: f64,
    incoming_activation: f64,
    reactivity_baseline: f64,
    physical_work: f64,
    dissipated_heat: f64,
) -> Option<DisplacementPreflight> {
    if !incoming_activation.is_finite() || incoming_activation < 0.0 {
        return None;
    }

    let expected_old_strength = bond_strength(anchor, displaced);
    if !old_bond_strength.is_finite()
        || old_bond_strength < 0.0
        || (old_bond_strength - expected_old_strength).abs() > 1e-9
    {
        return None;
    }

    let replacement_strength = bond_strength(anchor, incoming);
    if !replacement_strength.is_finite() {
        return None;
    }

    let old_bond_energy = intrinsic_bond_potential(anchor, displaced, old_bond_strength);
    let replacement_bond_energy = intrinsic_bond_potential(anchor, incoming, replacement_strength);
    if !old_bond_energy.is_finite()
        || old_bond_energy < 0.0
        || !replacement_bond_energy.is_finite()
        || replacement_bond_energy < 0.0
    {
        return None;
    }

    let disruption_barrier = crate::chemistry::activation_barrier_from_bond_properties(
        old_bond_strength,
        anchor.reactivity,
        displaced.reactivity,
        reactivity_baseline,
    )?;

    // Existing environmental BREAK treats the activation barrier as required
    // physical disruption work, separately from the stored bond potential.
    // Include that work and the new-bond formation work in one net transaction.
    let total_work = physical_work + disruption_barrier;
    if !total_work.is_finite() {
        return None;
    }
    // Structural potential is negative for a retained stable bond. Replacing
    // one bond with another is evaluated as one complete pre/post-state change.
    let energy = ReactionEnergyBalance::calculate(
        -old_bond_energy,
        -replacement_bond_energy,
        total_work,
        dissipated_heat,
    )?;

    Some(DisplacementPreflight {
        incoming_activation,
        disruption_barrier,
        old_bond_energy,
        replacement_bond_energy,
        disruption_work: disruption_barrier,
        energy,
    })
}

const ENVIRONMENTAL_DISPLACEMENT_PREFIX: &str = "environment|displacement:";
const DISPLACEMENT_FIELD_ANCHOR_LIMIT: usize = 64;
const DISPLACEMENT_TARGET_CELL_LIMIT: usize = 64;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct EnvironmentalDisplacementOperation {
    pub(crate) cell_a: usize,
    pub(crate) material_a: usize,
    pub(crate) source_a: PhysicalMaterial,
    pub(crate) cell_b: usize,
    pub(crate) material_b: usize,
    pub(crate) source_b: PhysicalMaterial,
    pub(crate) anchor_unit: usize,
    pub(crate) displaced_unit: usize,
    pub(crate) incoming_unit: usize,
    pub(crate) bond_index: usize,
    pub(crate) old_connection: PhysicalMaterialBond,
    pub(crate) candidate: ConnectionPairCandidate,
    pub(crate) investment: f64,
    pub(crate) activation_signal: f64,
    pub(crate) reaction_frequency_cycles_per_day: f64,
    pub(crate) accumulation_key: String,
    pub(crate) accumulated_reaction: f64,
    pub(crate) work_required: f64,
}

fn restore_displacement_source(
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
        crate::structure::Placement {
            x: 0.0,
            y: 0.0,
            rotation_radians: 0.0,
        },
        &environment.catalog,
    )?;
    Some((structure, indices))
}

fn displacement_accumulation_key(
    source_a: &PhysicalMaterial,
    source_b: &PhysicalMaterial,
    bond_index: usize,
    anchor_part: usize,
    incoming_part: usize,
    candidate: ConnectionPairCandidate,
) -> String {
    format!(
        "{ENVIRONMENTAL_DISPLACEMENT_PREFIX}{}:{}:{bond_index}:{anchor_part}:{incoming_part}:{:?}:{:?}",
        if source_a.environmental_identity == 0 {
            format!("{source_a:?}")
        } else {
            source_a.environmental_identity.to_string()
        },
        if source_b.environmental_identity == 0 {
            format!("{source_b:?}")
        } else {
            source_b.environmental_identity.to_string()
        },
        candidate.endpoint_a,
        candidate.endpoint_b,
    )
}

fn dissipate_unupdated_displacement_accumulations(
    accumulation: &mut BTreeMap<String, f64>,
    updated_keys: &BTreeSet<String>,
) {
    accumulation.retain(|key, value| {
        if !key.starts_with(ENVIRONMENTAL_DISPLACEMENT_PREFIX) || updated_keys.contains(key) {
            return true;
        }
        *value *= 1.0 - crate::chemistry::CHEMICAL_DISSIPATION;
        value.is_finite() && *value > 0.0
    });
}

fn best_freed_endpoint_contact(
    combined: &OrganismStructure,
    anchor_unit: usize,
    incoming_unit: usize,
    old_bond: Bond,
    freed_endpoint: crate::structure::ConnectionEndpoint,
    catalog: &[BaseResource],
) -> Option<(ConnectionPairCandidate, f64)> {
    let mut topology_trial = combined.clone();
    topology_trial.break_matching_bond(old_bond)?;
    let mut cache = ConnectionCompatibilityCache::new();
    let mut best: Option<(ConnectionPairCandidate, f64, f64)> = None;
    for candidate in crate::contact::connection_pair_candidates_cached(
        &topology_trial,
        anchor_unit,
        incoming_unit,
        catalog,
        &mut cache,
    ) {
        if candidate.endpoint_a != freed_endpoint
            || !candidate.available_a
            || !candidate.available_b
            || !candidate.distance.is_finite()
            || candidate.distance < 0.0
            || candidate.distance > crate::combine_runtime::COMBINE_CONTACT_TOLERANCE
            || !candidate.facing.is_finite()
            || candidate.facing <= 0.0
        {
            continue;
        }
        let Some((_evaluation, _formation_work, investment, required_energy)) =
            crate::combine_runtime::selected_candidate_evaluation(
                &topology_trial,
                anchor_unit,
                incoming_unit,
                candidate,
                catalog,
            )
        else {
            continue;
        };
        let replace = best.map_or(true, |(current, _, current_energy)| {
            required_energy
                .partial_cmp(&current_energy)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    candidate
                        .distance
                        .partial_cmp(&current.distance)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .is_lt()
        });
        if replace {
            best = Some((candidate, investment, required_energy));
        }
    }
    best.map(|(candidate, investment, _)| (candidate, investment))
}

fn displacement_reaction_frequency(
    anchor: &crate::structure::StructuralUnit,
    displaced: &crate::structure::StructuralUnit,
    catalog: &[BaseResource],
) -> Option<f64> {
    let baselines = crate::resources::ResourceBaselines::from_catalog(catalog);
    let frequency_a =
        crate::harmonics::natural_frequency_cycles_per_day(anchor.properties(catalog)?, baselines);
    let frequency_b = crate::harmonics::natural_frequency_cycles_per_day(
        displaced.properties(catalog)?,
        baselines,
    );
    if !frequency_a.is_finite()
        || frequency_a <= 0.0
        || !frequency_b.is_finite()
        || frequency_b <= 0.0
    {
        return None;
    }
    let frequency = (0.5 * frequency_a.ln() + 0.5 * frequency_b.ln()).exp();
    (frequency.is_finite() && frequency > 0.0).then_some(frequency)
}

/// Discover live A-B + C contacts. The old bond is removed only on a trial
/// graph before candidate generation, so C must physically occupy the endpoint
/// that A-B actually frees.
pub(crate) fn accumulate_environmental(
    environment: &Environment,
    accumulation: &mut BTreeMap<String, f64>,
    tick: u64,
    remaining_discovery_work: &mut usize,
) -> Vec<EnvironmentalDisplacementOperation> {
    if *remaining_discovery_work == 0 || environment.field.cells.is_empty() {
        dissipate_unupdated_displacement_accumulations(accumulation, &BTreeSet::new());
        return Vec::new();
    }

    let mut anchor_work = (*remaining_discovery_work)
        .min(DISPLACEMENT_FIELD_ANCHOR_LIMIT)
        .min(crate::chemical_reaction::MAX_REACTION_CANDIDATE_INSPECTIONS_PER_TICK);
    let cell_count = environment.field.cells.len();
    let cell_scan_count = cell_count.min(DISPLACEMENT_FIELD_ANCHOR_LIMIT);
    let cell_start = tick as usize % cell_count;
    let mut operations = Vec::new();
    let mut updated_keys = BTreeSet::new();
    let baselines = crate::resources::ResourceBaselines::from_catalog(&environment.catalog);

    'anchors: for cell_offset in 0..cell_scan_count {
        if anchor_work == 0 || *remaining_discovery_work == 0 {
            break;
        }
        let cell_a = (cell_start + cell_offset) % cell_count;
        *remaining_discovery_work = remaining_discovery_work.saturating_sub(1);
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
            *remaining_discovery_work = remaining_discovery_work.saturating_sub(1);
            let material_a = (material_start + material_offset) % material_count;
            let source_a = environment.field.cells[cell_a].physical_materials[material_a].clone();
            let Some(connections) = source_a.internal_connections.as_ref() else {
                continue;
            };
            if connections.is_empty() {
                continue;
            }
            let Some((source_structure, indices_a)) =
                restore_displacement_source(&source_a, environment)
            else {
                continue;
            };
            let Some(body) =
                crate::organism_geometry::OrganismBodyGeometry::from_all_realized_units(
                    &source_structure,
                )
            else {
                continue;
            };
            let target_cells = environment.field.cells_intersecting_bounds_limited(
                body.min_x - 2.1,
                body.max_x + 2.1,
                body.min_y - 2.1,
                body.max_y + 2.1,
                DISPLACEMENT_TARGET_CELL_LIMIT.min(*remaining_discovery_work),
                (tick as usize)
                    .wrapping_add(cell_a)
                    .wrapping_add(material_a),
            );

            for &cell_b in &target_cells {
                if *remaining_discovery_work == 0 {
                    break 'anchors;
                }
                *remaining_discovery_work = remaining_discovery_work.saturating_sub(1);
                let target_count = environment.field.cells[cell_b].physical_materials.len();
                if target_count == 0 {
                    continue;
                }
                let target_start = (tick as usize + cell_b + material_a) % target_count;
                for target_offset in 0..target_count {
                    if *remaining_discovery_work == 0 {
                        break 'anchors;
                    }
                    *remaining_discovery_work = remaining_discovery_work.saturating_sub(1);
                    let material_b = (target_start + target_offset) % target_count;
                    if cell_a == cell_b && material_a == material_b {
                        continue;
                    }
                    let source_b =
                        environment.field.cells[cell_b].physical_materials[material_b].clone();
                    let mut combined = source_structure.clone();
                    let Some(indices_b) = crate::material_restoration::restore_material(
                        &mut combined,
                        &source_b,
                        crate::structure::Placement {
                            x: 0.0,
                            y: 0.0,
                            rotation_radians: 0.0,
                        },
                        &environment.catalog,
                    ) else {
                        continue;
                    };
                    if indices_b.iter().any(|index| {
                        crate::construction_runtime::placed_unit_overlaps(
                            &combined,
                            &combined.units[*index],
                            &indices_b,
                            &environment.catalog,
                        )
                    }) {
                        continue;
                    }

                    for (bond_index, connection) in connections.iter().enumerate() {
                        if *remaining_discovery_work == 0 {
                            break 'anchors;
                        }
                        *remaining_discovery_work = remaining_discovery_work.saturating_sub(1);
                        let (Some(&part_a), Some(&part_b)) = (
                            indices_a.get(connection.part_a),
                            indices_a.get(connection.part_b),
                        ) else {
                            continue;
                        };
                        let (Some(id_a), Some(id_b)) =
                            (combined.physical_id(part_a), combined.physical_id(part_b))
                        else {
                            continue;
                        };
                        let expected_bond = Bond {
                            endpoint_a: crate::structure::BondEndpoint::new(
                                id_a,
                                connection.endpoint_a,
                            ),
                            endpoint_b: crate::structure::BondEndpoint::new(
                                id_b,
                                connection.endpoint_b,
                            ),
                            strength: combined.units[part_a]
                                .properties(&environment.catalog)
                                .zip(combined.units[part_b].properties(&environment.catalog))
                                .map(|(a, b)| bond_strength(a, b))
                                .unwrap_or(f64::NAN),
                            bond_energy: connection.bond_energy,
                        };
                        let Some(old_bond) = combined
                            .bonds
                            .iter()
                            .find(|bond| bond.has_same_identity(&expected_bond))
                            .cloned()
                        else {
                            continue;
                        };

                        for (anchor_part, displaced_part, freed_endpoint) in [
                            (connection.part_a, connection.part_b, connection.endpoint_a),
                            (connection.part_b, connection.part_a, connection.endpoint_b),
                        ] {
                            let (Some(&anchor_unit), Some(&displaced_unit)) =
                                (indices_a.get(anchor_part), indices_a.get(displaced_part))
                            else {
                                continue;
                            };
                            let Some(frequency) = displacement_reaction_frequency(
                                &combined.units[anchor_unit],
                                &combined.units[displaced_unit],
                                &environment.catalog,
                            ) else {
                                continue;
                            };
                            for (incoming_part, &incoming_unit) in indices_b.iter().enumerate() {
                                if *remaining_discovery_work == 0 {
                                    break 'anchors;
                                }
                                *remaining_discovery_work =
                                    remaining_discovery_work.saturating_sub(1);
                                let Some((candidate, investment)) = best_freed_endpoint_contact(
                                    &combined,
                                    anchor_unit,
                                    incoming_unit,
                                    old_bond,
                                    freed_endpoint,
                                    &environment.catalog,
                                ) else {
                                    continue;
                                };
                                let key = displacement_accumulation_key(
                                    &source_a,
                                    &source_b,
                                    bond_index,
                                    anchor_part,
                                    incoming_part,
                                    candidate,
                                );
                                let previous = accumulation.get(&key).copied().unwrap_or(0.0);
                                let Some(next) = accumulate_incoming_activation(
                                    &combined.units[anchor_unit],
                                    &combined.units[incoming_unit],
                                    candidate,
                                    previous,
                                    &environment.catalog,
                                ) else {
                                    continue;
                                };
                                updated_keys.insert(key.clone());
                                if next <= 0.0 {
                                    accumulation.remove(&key);
                                    continue;
                                }
                                let Some(activation_signal) =
                                    crate::chemistry::normalized_chemistry_to_activation_signal(
                                        next,
                                    )
                                else {
                                    continue;
                                };
                                let Some(trial) = build_displacement_trial(
                                    &combined,
                                    anchor_unit,
                                    displaced_unit,
                                    incoming_unit,
                                    old_bond.clone(),
                                    candidate,
                                    investment,
                                    activation_signal,
                                    baselines.reactivity,
                                    0.0,
                                    &environment.catalog,
                                ) else {
                                    accumulation.insert(key, next);
                                    continue;
                                };
                                let work_required = trial.formation_work + trial.disruption_work;
                                operations.push(EnvironmentalDisplacementOperation {
                                    cell_a,
                                    material_a,
                                    source_a: source_a.clone(),
                                    cell_b,
                                    material_b,
                                    source_b: source_b.clone(),
                                    anchor_unit: anchor_part,
                                    displaced_unit: displaced_part,
                                    incoming_unit: incoming_part,
                                    bond_index,
                                    old_connection: connection.clone(),
                                    candidate,
                                    investment,
                                    activation_signal,
                                    reaction_frequency_cycles_per_day: frequency,
                                    accumulation_key: key.clone(),
                                    accumulated_reaction: next,
                                    work_required,
                                });
                                accumulation.remove(&key);
                            }
                        }
                    }
                }
            }
        }
    }

    dissipate_unupdated_displacement_accumulations(accumulation, &updated_keys);
    operations
}

/// Keep field-owned displacement commits mutually exclusive with other
/// operations touching either source cell. Deferred activation remains stored.
pub(crate) fn schedule_environmental_operations(
    mut operations: Vec<EnvironmentalDisplacementOperation>,
    tick: u64,
    remaining_work: &mut usize,
    accumulation: &mut BTreeMap<String, f64>,
) -> Vec<EnvironmentalDisplacementOperation> {
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

pub(crate) fn resolve_environmental(
    operation: EnvironmentalDisplacementOperation,
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

    let Some((mut combined, indices_a)) =
        restore_displacement_source(&operation.source_a, environment)
    else {
        return false;
    };
    let Some(indices_b) = crate::material_restoration::restore_material(
        &mut combined,
        &operation.source_b,
        crate::structure::Placement {
            x: 0.0,
            y: 0.0,
            rotation_radians: 0.0,
        },
        &environment.catalog,
    ) else {
        return false;
    };
    if indices_b.iter().any(|index| {
        crate::construction_runtime::placed_unit_overlaps(
            &combined,
            &combined.units[*index],
            &indices_b,
            &environment.catalog,
        )
    }) {
        return false;
    }
    let (Some(&anchor_unit), Some(&displaced_unit), Some(&incoming_unit)) = (
        indices_a.get(operation.anchor_unit),
        indices_a.get(operation.displaced_unit),
        indices_b.get(operation.incoming_unit),
    ) else {
        return false;
    };
    let baselines = crate::resources::ResourceBaselines::from_catalog(&environment.catalog);
    let Some(old_connection) = operation
        .source_a
        .internal_connections
        .as_ref()
        .and_then(|connections| connections.get(operation.bond_index))
    else {
        return false;
    };
    if old_connection != &operation.old_connection {
        return false;
    }
    let (Some(part_a_unit), Some(part_b_unit)) = (
        indices_a.get(old_connection.part_a).copied(),
        indices_a.get(old_connection.part_b).copied(),
    ) else {
        return false;
    };
    let (Some(part_a_id), Some(part_b_id)) = (
        combined.physical_id(part_a_unit),
        combined.physical_id(part_b_unit),
    ) else {
        return false;
    };
    let (Some(part_a_properties), Some(part_b_properties)) = (
        combined.units[part_a_unit].properties(&environment.catalog),
        combined.units[part_b_unit].properties(&environment.catalog),
    ) else {
        return false;
    };
    let expected_old_bond = Bond {
        endpoint_a: crate::structure::BondEndpoint::new(part_a_id, old_connection.endpoint_a),
        endpoint_b: crate::structure::BondEndpoint::new(part_b_id, old_connection.endpoint_b),
        strength: bond_strength(part_a_properties, part_b_properties),
        bond_energy: old_connection.bond_energy,
    };
    let Some(old_bond) = combined
        .bonds
        .iter()
        .find(|bond| bond.has_same_identity(&expected_old_bond))
        .cloned()
    else {
        return false;
    };
    let Some(trial) = build_displacement_trial(
        &combined,
        anchor_unit,
        displaced_unit,
        incoming_unit,
        old_bond,
        operation.candidate,
        operation.investment,
        operation.activation_signal,
        baselines.reactivity,
        0.0,
        &environment.catalog,
    ) else {
        return false;
    };
    let Some(derived_activation) =
        crate::chemistry::normalized_chemistry_to_activation_signal(operation.accumulated_reaction)
    else {
        return false;
    };
    if (derived_activation - operation.activation_signal).abs() > 1e-9 {
        return false;
    }
    let Some(expected_frequency) = displacement_reaction_frequency(
        &combined.units[anchor_unit],
        &combined.units[displaced_unit],
        &environment.catalog,
    ) else {
        return false;
    };
    if (expected_frequency - operation.reaction_frequency_cycles_per_day).abs() > 1e-9 {
        return false;
    }
    // Resonance directly funds physical work; it is not first credited to the
    // usable-energy holder. This keeps the usable debit equal to the unfunded
    // remainder while the ledger still records the complete physical work.
    let mut staged_resonance = environment.resonant_energy.clone();
    let Some(resonant_before) = staged_resonance.total_energy() else {
        return false;
    };
    let resonance_work_budget = trial
        .energy
        .physical_work
        .min(trial.energy.required_external_input);
    let resonant_work = staged_resonance.withdraw_compatible(
        operation.reaction_frequency_cycles_per_day,
        resonance_work_budget,
    );
    let Some(resonant_after) = staged_resonance.total_energy() else {
        return false;
    };
    if !resonant_work.is_finite()
        || resonant_work < 0.0
        || !resonant_before.is_finite()
        || !resonant_after.is_finite()
        || resonant_work > resonance_work_budget + 1e-9
        || (resonant_before - resonant_after - resonant_work).abs() > 1e-9
    {
        return false;
    }

    let remaining_physical_work = trial.energy.physical_work - resonant_work;
    let Some(funded_energy) = ReactionEnergyBalance::calculate(
        trial.energy.reactant_energy,
        trial.energy.product_energy,
        remaining_physical_work,
        trial.energy.dissipated_heat,
    ) else {
        return false;
    };
    if funded_energy.required_external_input > environment.chemistry_usable_energy {
        return false;
    }
    let Some(transaction) = funded_energy.settlement_transaction(EnergyReason::Combine) else {
        return false;
    };

    let mut staged_ledger = *ledger;
    let mut staged_usable_energy = environment.chemistry_usable_energy;
    if !staged_ledger.settle_transaction(&mut staged_usable_energy, transaction) {
        return false;
    }
    let usable_delta = staged_usable_energy - environment.chemistry_usable_energy;
    let structural_delta = trial.energy.net_reaction_change;
    let next_total_usable = staged_ledger.total_usable_energy_held + usable_delta;
    let next_total_resonant = ledger.total_resonant_energy_held - resonant_work;
    let next_total_structural = staged_ledger.total_structural_energy_held + structural_delta;
    let next_total_work = ledger.total_work_expended + trial.energy.physical_work;
    if !usable_delta.is_finite()
        || !next_total_usable.is_finite()
        || next_total_usable < 0.0
        || !next_total_resonant.is_finite()
        || next_total_resonant < 0.0
        || !next_total_structural.is_finite()
        || !next_total_work.is_finite()
    {
        return false;
    }
    staged_ledger.total_usable_energy_held = next_total_usable;
    staged_ledger.total_resonant_energy_held = next_total_resonant;
    staged_ledger.total_structural_energy_held = next_total_structural;
    staged_ledger.total_work_expended = next_total_work;

    let mut staged_products = Vec::with_capacity(trial.products.len());
    for product in trial.products {
        if !product.is_realized()
            || product.material.is_empty()
            || !product.material.is_valid_for_catalog(&environment.catalog)
        {
            return false;
        }
        let Some(placement) = product
            .placements
            .as_ref()
            .and_then(|placements| placements.first())
        else {
            return false;
        };
        let Some(destination) = environment
            .field
            .index_for_position(placement.x, placement.y)
        else {
            return false;
        };
        staged_products.push((destination, product));
    }
    if staged_products.len() < 2 {
        return false;
    }

    // All validation and energy work is staged. Remove both source objects
    // without index drift, then deposit each physical component at its own
    // retained world position.
    if operation.cell_a == operation.cell_b {
        let cell = &mut environment.field.cells[operation.cell_a];
        let high = operation.material_a.max(operation.material_b);
        let low = operation.material_a.min(operation.material_b);
        if high == low {
            return false;
        }
        cell.physical_materials.remove(high);
        cell.physical_materials.remove(low);
    } else {
        environment.field.cells[operation.cell_a]
            .physical_materials
            .remove(operation.material_a);
        environment.field.cells[operation.cell_b]
            .physical_materials
            .remove(operation.material_b);
    }
    let mut changed_cells = BTreeSet::new();
    changed_cells.insert(operation.cell_a);
    changed_cells.insert(operation.cell_b);
    for (destination, product) in staged_products {
        environment.field.cells[destination]
            .physical_materials
            .push(product);
        changed_cells.insert(destination);
    }
    for cell in changed_cells {
        environment.field.mark_changed_at_index(cell);
    }
    environment.field.assign_missing_material_identities();
    environment.chemistry_usable_energy = staged_usable_energy;
    environment.resonant_energy = staged_resonance;
    *ledger = staged_ledger;
    true
}

pub(crate) fn resolve_environmental_preserving_accumulation(
    operation: EnvironmentalDisplacementOperation,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::default_catalog;
    use crate::structure::BondEndpoint;

    fn properties(name: &str) -> ResourceProperties {
        default_catalog()
            .into_iter()
            .find(|resource| resource.name == name)
            .expect("canonical resource")
            .properties
    }

    fn catalog_reactivity_baseline() -> f64 {
        let catalog = default_catalog();
        catalog
            .iter()
            .map(|resource| resource.properties.reactivity)
            .sum::<f64>()
            / catalog.len() as f64
    }

    #[test]
    fn carbon_oxygen_can_replace_carbon_hydrogen_when_incoming_contact_activates_it() {
        let anchor = properties("Carbon");
        let displaced = properties("Hydrogen");
        let incoming = properties("Oxygen");
        let old_strength = bond_strength(anchor, displaced);
        let provisional = evaluate_displacement_preflight(
            anchor,
            displaced,
            incoming,
            old_strength,
            10.0,
            catalog_reactivity_baseline(),
            0.0,
            0.0,
        )
        .expect("finite catalog-backed preflight");

        assert!(provisional.replacement_bond_energy > provisional.old_bond_energy);
        assert!(provisional.is_activated());
        assert!(provisional.is_energy_favorable_without_external_input());
        assert!(provisional.can_commit(0.0));
        assert!(provisional.energy.residual(0.0).unwrap().abs() <= 1e-9);
    }

    #[test]
    fn weaker_carbon_nitrogen_replacement_of_carbon_oxygen_needs_real_energy_input() {
        let anchor = properties("Carbon");
        let displaced = properties("Oxygen");
        let incoming = properties("Nitrogen");
        let old_strength = bond_strength(anchor, displaced);
        let provisional = evaluate_displacement_preflight(
            anchor,
            displaced,
            incoming,
            old_strength,
            10.0,
            catalog_reactivity_baseline(),
            0.0,
            0.0,
        )
        .expect("finite catalog-backed preflight");

        assert!(provisional.replacement_bond_energy < provisional.old_bond_energy);
        assert!(provisional.is_activated());
        assert!(provisional.energy.required_external_input > 0.0);
        assert!(!provisional.can_commit(0.0));
        assert!(provisional.can_commit(provisional.energy.required_external_input));
        assert!(
            provisional
                .energy
                .residual(provisional.energy.required_external_input)
                .unwrap()
                .abs()
                <= 1e-9
        );
    }

    #[test]
    fn favorable_replacement_is_rejected_without_sufficient_incoming_activation() {
        let anchor = properties("Carbon");
        let displaced = properties("Hydrogen");
        let incoming = properties("Oxygen");
        let old_strength = bond_strength(anchor, displaced);
        let provisional = evaluate_displacement_preflight(
            anchor,
            displaced,
            incoming,
            old_strength,
            0.0,
            catalog_reactivity_baseline(),
            0.0,
            0.0,
        )
        .expect("finite catalog-backed preflight");

        assert!(provisional.is_energy_favorable_without_external_input());
        assert!(!provisional.is_activated());
        assert!(!provisional.can_commit(0.0));
    }

    #[test]
    fn invalid_or_unrelated_old_bond_strength_fails_closed() {
        let anchor = properties("Carbon");
        let displaced = properties("Nitrogen");
        let incoming = properties("Oxygen");
        assert!(evaluate_displacement_preflight(
            anchor,
            displaced,
            incoming,
            f64::NAN,
            10.0,
            catalog_reactivity_baseline(),
            0.0,
            0.0,
        )
        .is_none());
        assert!(evaluate_displacement_preflight(
            anchor,
            displaced,
            incoming,
            0.0,
            10.0,
            catalog_reactivity_baseline(),
            0.0,
            0.0,
        )
        .is_none());
    }

    fn three_participant_fixture(
        catalog: &[BaseResource],
    ) -> (OrganismStructure, Bond, ConnectionPairCandidate, f64) {
        let mut source = OrganismStructure::new();
        source.add_unit(crate::structure::StructuralUnit::new(
            "Carbon",
            crate::structure::Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));
        source.add_unit(crate::structure::StructuralUnit::new(
            "Hydrogen",
            crate::structure::Placement {
                x: 1.5,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));
        source.add_unit(crate::structure::StructuralUnit::new(
            "Oxygen",
            crate::structure::Placement {
                x: 1.577_350_269_189_625_8,
                y: 0.12,
                rotation_radians: std::f64::consts::PI,
            },
        ));

        let mut cache = crate::contact::ConnectionCompatibilityCache::new();
        let old_candidate =
            crate::contact::connection_pair_candidates_cached(&source, 0, 1, catalog, &mut cache)
                .into_iter()
                .find(|candidate| {
                    candidate.distance <= crate::combine_runtime::COMBINE_CONTACT_TOLERANCE
                        && candidate.available_a
                        && candidate.available_b
                })
                .expect("Carbon-Hydrogen physical contact");
        let anchor = source.units[0].properties(catalog).unwrap();
        let displaced = source.units[1].properties(catalog).unwrap();
        let strength = bond_strength(anchor, displaced);
        let old_bond = Bond {
            endpoint_a: BondEndpoint::new(source.physical_id(0).unwrap(), old_candidate.endpoint_a),
            endpoint_b: BondEndpoint::new(source.physical_id(1).unwrap(), old_candidate.endpoint_b),
            strength,
            bond_energy: intrinsic_bond_potential(anchor, displaced, strength),
        };
        crate::contact::try_add_bond(&mut source, old_bond, catalog)
            .expect("initial A-B bond should be physically valid");

        let mut replacement_trial = source.clone();
        replacement_trial.break_matching_bond(old_bond).unwrap();
        let replacement_candidate = crate::contact::connection_pair_candidates_cached(
            &replacement_trial,
            0,
            2,
            catalog,
            &mut cache,
        )
        .into_iter()
        .find(|candidate| {
            candidate.endpoint_a == old_candidate.endpoint_a
                && candidate.distance >= 0.0
                && candidate.distance <= crate::combine_runtime::COMBINE_CONTACT_TOLERANCE
                && candidate.facing.is_finite()
                && candidate.facing > 0.0
                && candidate.available_a
                && candidate.available_b
        })
        .expect("freed Carbon endpoint should expose an engaged incoming Oxygen contact");
        let (_, _, investment, _) = crate::combine_runtime::selected_candidate_evaluation(
            &replacement_trial,
            0,
            2,
            replacement_candidate,
            catalog,
        )
        .expect("replacement candidate should pass ordinary COMBINE evaluation");
        (source, old_bond, replacement_candidate, investment)
    }

    #[test]
    fn incoming_activation_is_accumulated_from_the_replacement_contact() {
        let catalog = default_catalog();
        let (source, _old_bond, candidate, _investment) = three_participant_fixture(&catalog);
        let activation = accumulate_incoming_activation(
            &source.units[0],
            &source.units[2],
            candidate,
            0.0,
            &catalog,
        )
        .expect("valid live contact should accumulate incoming influence");
        assert!(activation > 0.0);
        assert!(activation.is_finite());
    }

    #[test]
    fn trial_replacement_splits_displaced_constituent_and_preserves_source() {
        let catalog = default_catalog();
        let (source, old_bond, candidate, investment) = three_participant_fixture(&catalog);
        let source_before = source.clone();
        let trial = build_displacement_trial(
            &source,
            0,
            1,
            2,
            old_bond,
            candidate,
            investment,
            10.0,
            catalog_reactivity_baseline(),
            0.0,
            &catalog,
        )
        .expect("favorable activated replacement should produce a trial");

        assert_eq!(source.units, source_before.units);
        assert_eq!(source.bonds, source_before.bonds);
        assert_eq!(trial.structure.bonds.len(), 1);
        assert_eq!(trial.products.len(), 2);
        assert!(trial.products.iter().any(|product| {
            product.material.parts.iter().any(|part| part.0 == "Carbon")
                && product.material.parts.iter().any(|part| part.0 == "Oxygen")
                && product.material.internal_bonds.len() == 1
        }));
        assert!(trial.products.iter().any(|product| {
            product.material.parts == vec![("Hydrogen".to_owned(), 1.0)]
                && product.material.internal_bonds.is_empty()
        }));
        assert!(
            trial.disruption_work > 0.0 && trial.energy.required_external_input > 0.0,
            "a favorable replacement still has to fund the old bond's physical disruption work"
        );
        assert_eq!(trial.removed_bond, old_bond);
        let bonded_product = trial
            .products
            .iter()
            .find(|product| product.material.parts.len() == 2)
            .expect("A-C component should contain two constituents");
        assert_eq!(
            trial.replacement_bond.bond_energy,
            bonded_product.internal_connections.as_ref().unwrap()[0].bond_energy
        );
    }

    #[test]
    fn live_environmental_displacement_commits_two_products_with_resonance_funding() {
        let mut simulation = crate::state::Simulation::new(23029, 10.0);
        let catalog = simulation.environment.catalog.clone();
        for cell in &mut simulation.environment.field.cells {
            cell.materials.clear();
            cell.physical_materials.clear();
        }

        let (mut source, _old_bond, _candidate, _investment) = three_participant_fixture(&catalog);
        for unit in &mut source.units {
            unit.placement.x += 10.0;
            unit.placement.y += 10.0;
        }
        let mut reactants =
            crate::environmental_combine::physical_materials_from_structure_components(
                &source, &catalog,
            )
            .expect("the bonded reactant and incoming constituent should realize separately");
        assert_eq!(reactants.len(), 2);
        reactants.sort_by_key(|material| {
            material.internal_connections.as_ref().map_or(0, Vec::len) == 0
        });
        assert!(reactants[0]
            .internal_connections
            .as_ref()
            .is_some_and(|connections| !connections.is_empty()));
        let field_cell = simulation
            .environment
            .field
            .index_for_position(10.0, 10.0)
            .expect("fixture is inside the environment");
        simulation.environment.field.cells[field_cell]
            .physical_materials
            .extend(reactants.iter().cloned());
        simulation
            .environment
            .field
            .assign_missing_material_identities();

        let frequency =
            displacement_reaction_frequency(&source.units[0], &source.units[1], &catalog)
                .expect("the old bond has a valid natural reaction frequency");
        let (mut combined, indices_a) =
            restore_displacement_source(&reactants[0], &simulation.environment)
                .expect("bonded field source should restore");
        let indices_b = crate::material_restoration::restore_material(
            &mut combined,
            &reactants[1],
            crate::structure::Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
            &catalog,
        )
        .expect("incoming field source should restore in the shared material frame");
        assert!(
            !indices_b.iter().any(|index| {
                crate::construction_runtime::placed_unit_overlaps(
                    &combined,
                    &combined.units[*index],
                    &indices_b,
                    &catalog,
                )
            }),
            "incoming constituent must not overlap the bonded source"
        );
        let connection = reactants[0]
            .internal_connections
            .as_ref()
            .and_then(|connections| connections.first())
            .expect("bonded source should preserve its old connection")
            .clone();
        let anchor_unit = indices_a[connection.part_a];
        let incoming_unit = indices_b[0];
        let old_bond = combined
            .bonds
            .first()
            .copied()
            .expect("restored source should contain its old bond");
        let (candidate, _investment) = best_freed_endpoint_contact(
            &combined,
            anchor_unit,
            incoming_unit,
            old_bond,
            connection.endpoint_a,
            &catalog,
        )
        .expect("incoming source should contact the endpoint freed by the old bond");
        let first_activation = accumulate_incoming_activation(
            &combined.units[anchor_unit],
            &combined.units[incoming_unit],
            candidate,
            0.0,
            &catalog,
        )
        .expect("the live replacement contact should accumulate chemistry");
        assert!(first_activation > 0.0);

        let key = displacement_accumulation_key(
            &reactants[0],
            &reactants[1],
            0,
            connection.part_a,
            0,
            candidate,
        );
        let mut accumulation = BTreeMap::new();
        let mut discovery_work =
            crate::chemical_reaction::MAX_REACTION_CANDIDATE_INSPECTIONS_PER_TICK;
        let mut operations = accumulate_environmental(
            &simulation.environment,
            &mut accumulation,
            field_cell as u64,
            &mut discovery_work,
        );
        assert!(
            accumulation.contains_key(&key) || !operations.is_empty(),
            "field scheduler should discover and accumulate the validated contact"
        );
        let operation = if let Some(operation) = operations.pop() {
            operation
        } else {
            let one_unit_signal = crate::chemistry::normalized_chemistry_to_activation_signal(1.0)
                .expect("unit chemistry should normalize");
            accumulation.insert(key, 100.0 / one_unit_signal);
            let mut discovery_work =
                crate::chemical_reaction::MAX_REACTION_CANDIDATE_INSPECTIONS_PER_TICK;
            accumulate_environmental(
                &simulation.environment,
                &mut accumulation,
                field_cell as u64,
                &mut discovery_work,
            )
            .into_iter()
            .next()
            .expect("triggered field contact should produce a displacement operation")
        };
        assert!(operation.work_required > 0.0);
        assert!(operation.activation_signal > 0.0);

        let environment_before = simulation.environment.clone();
        let ledger_before = simulation.energy_ledger;
        assert!(
            !resolve_environmental(
                operation.clone(),
                &mut simulation.environment,
                &mut simulation.energy_ledger,
            ),
            "an unfunded displacement must not mutate the field or energy holders"
        );
        assert_eq!(
            simulation.environment.field.cells[field_cell]
                .physical_materials
                .len(),
            2
        );
        assert_eq!(simulation.energy_ledger, ledger_before);
        assert_eq!(
            simulation.environment.resonant_energy,
            environment_before.resonant_energy
        );

        assert!(simulation
            .environment
            .resonant_energy
            .deposit(frequency, 1000.0));
        simulation.energy_ledger.total_resonant_energy_held += 1000.0;
        let resonant_before = simulation
            .environment
            .resonant_energy
            .total_energy()
            .expect("valid staged resonance");
        assert!(resolve_environmental(
            operation,
            &mut simulation.environment,
            &mut simulation.energy_ledger,
        ));

        let products = &simulation.environment.field.cells[field_cell].physical_materials;
        assert_eq!(products.len(), 2);
        assert!(products.iter().any(|product| {
            product.material.parts.iter().any(|part| part.0 == "Carbon")
                && product.material.parts.iter().any(|part| part.0 == "Oxygen")
                && product.material.internal_bonds.len() == 1
        }));
        assert!(products.iter().any(|product| {
            product.material.parts == vec![("Hydrogen".to_owned(), 1.0)]
                && product.material.internal_bonds.is_empty()
        }));
        let resonant_after = simulation
            .environment
            .resonant_energy
            .total_energy()
            .expect("valid committed resonance");
        assert!(resonant_after < resonant_before);
        assert!(
            (simulation.energy_ledger.total_resonant_energy_held - resonant_after).abs() < 1e-8
        );
    }

    #[test]
    fn replacement_must_use_the_endpoint_freed_by_the_displaced_bond() {
        let catalog = default_catalog();
        let (source, old_bond, candidate, investment) = three_participant_fixture(&catalog);
        assert_eq!(candidate.endpoint_a, old_bond.endpoint_a.location);
        let mut unrelated_candidate = candidate;
        unrelated_candidate.endpoint_a = match candidate.endpoint_a {
            crate::structure::ConnectionEndpoint::Corner { point_index } => {
                crate::structure::ConnectionEndpoint::Corner {
                    point_index: point_index.wrapping_add(1),
                }
            }
            crate::structure::ConnectionEndpoint::LineEndpoint { point_index } => {
                crate::structure::ConnectionEndpoint::LineEndpoint {
                    point_index: point_index.wrapping_add(1),
                }
            }
            crate::structure::ConnectionEndpoint::Boundary { angle_radians } => {
                crate::structure::ConnectionEndpoint::Boundary {
                    angle_radians: angle_radians + 0.1,
                }
            }
            crate::structure::ConnectionEndpoint::Fluid { x, y } => {
                crate::structure::ConnectionEndpoint::Fluid { x: x + 0.1, y }
            }
        };
        assert_ne!(unrelated_candidate.endpoint_a, old_bond.endpoint_a.location);
        assert!(build_displacement_trial(
            &source,
            0,
            1,
            2,
            old_bond,
            unrelated_candidate,
            investment,
            10.0,
            catalog_reactivity_baseline(),
            0.0,
            &catalog,
        )
        .is_none());
    }

    #[test]
    fn committed_trial_updates_graph_energy_and_products_together() {
        let catalog = default_catalog();
        let (mut source, old_bond, candidate, investment) = three_participant_fixture(&catalog);
        let mut ledger = crate::state::EnergyLedger {
            total_structural_energy_held: crate::energy_ledger::structural_potential(&source)
                .unwrap(),
            ..Default::default()
        };
        let trial = build_displacement_trial(
            &source,
            0,
            1,
            2,
            old_bond,
            candidate,
            investment,
            10.0,
            catalog_reactivity_baseline(),
            0.0,
            &catalog,
        )
        .expect("valid displacement trial");

        let available_energy = trial.energy.required_external_input;
        assert!(available_energy > 0.0);
        ledger.total_usable_energy_held = available_energy;
        let mut usable_energy = available_energy;
        let products =
            commit_displacement_trial(&mut source, trial, &mut ledger, &mut usable_energy)
                .expect("funded favorable trial should commit atomically");

        assert_eq!(source.bonds.len(), 1);
        assert_eq!(products.len(), 2);
        assert!(usable_energy.is_finite() && usable_energy >= 0.0);
        assert!(usable_energy < available_energy);
        assert_eq!(
            ledger.total_structural_energy_held,
            -source.bonds[0].bond_energy
        );
    }

    #[test]
    fn stale_trial_cannot_commit_or_change_energy_holders() {
        let catalog = default_catalog();
        let (mut source, old_bond, candidate, investment) = three_participant_fixture(&catalog);
        let trial = build_displacement_trial(
            &source,
            0,
            1,
            2,
            old_bond,
            candidate,
            investment,
            10.0,
            catalog_reactivity_baseline(),
            0.0,
            &catalog,
        )
        .expect("valid displacement trial");
        source.units[0].placement.x += 0.01;
        let graph_before = source.clone();
        let mut ledger = crate::state::EnergyLedger::default();
        let ledger_before = ledger;
        let mut usable_energy = 0.0;

        assert!(
            commit_displacement_trial(&mut source, trial, &mut ledger, &mut usable_energy,)
                .is_none()
        );
        assert_eq!(source.units, graph_before.units);
        assert_eq!(source.bonds, graph_before.bonds);
        assert_eq!(ledger, ledger_before);
        assert_eq!(usable_energy, 0.0);
    }

    #[test]
    fn trial_replacement_rejects_insufficient_incoming_activation_without_mutation() {
        let catalog = default_catalog();
        let (source, old_bond, candidate, investment) = three_participant_fixture(&catalog);
        let before = source.clone();
        assert!(build_displacement_trial(
            &source,
            0,
            1,
            2,
            old_bond,
            candidate,
            investment,
            0.0,
            catalog_reactivity_baseline(),
            0.0,
            &catalog,
        )
        .is_none());
        assert_eq!(source.units, before.units);
        assert_eq!(source.bonds, before.bonds);
    }
}
