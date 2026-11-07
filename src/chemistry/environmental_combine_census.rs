//! Independent, read-only census of environmental COMBINE opportunities.
//!
//! This deliberately enumerates every unordered physical-material pair. It does
//! not call the bounded live discovery path or mutate chemistry, activation,
//! energy, or field state. Exact endpoint and chemistry eligibility still come
//! from the same authorities used by live COMBINE.

use std::collections::{BTreeMap, HashSet};
use std::time::Instant;

use crate::environment::ActiveMaterialField;
use crate::physical_material::PhysicalMaterial;
use crate::resources::BaseResource;
use crate::structure::{ConnectionEndpoint, OrganismStructure, Placement};

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
pub(crate) struct EnvironmentalCombineCensus {
    pub(crate) physical_materials: usize,
    pub(crate) unordered_pairs_total: u64,
    pub(crate) invalid_or_duplicate_identity_materials: usize,
    pub(crate) material_restorations_succeeded: usize,
    pub(crate) material_restorations_failed: usize,
    pub(crate) pairs_with_both_materials_restored: u64,
    pub(crate) broad_phase_rejected_pairs: u64,
    pub(crate) broad_phase_surviving_pairs: u64,
    pub(crate) pairs_rejected_for_cross_material_overlap: u64,
    pub(crate) constituent_pairs_evaluated: u64,
    pub(crate) physical_interfaces: u64,
    pub(crate) chemically_eligible_interfaces: u64,
    pub(crate) live_geometry_resolvable_interfaces: u64,
    pub(crate) interfaces_with_chemical_activation_signal: u64,
    pub(crate) activation_already_reached_interfaces: u64,
    pub(crate) activation_reached_after_one_more_evaluation: u64,
    pub(crate) activation_below_threshold_after_one_more_evaluation: u64,
    pub(crate) max_accumulated_reaction: f64,
    /// Instantaneous COMBINE funding uses the usable-energy pool.
    pub(crate) usable_energy_available: f64,
    pub(crate) usable_energy_sufficient_interfaces: u64,
    /// Resonance is recorded for context but is not a COMBINE funding source.
    pub(crate) resonant_energy_available: f64,
    pub(crate) resonance_funding_supported: bool,
    pub(crate) snapshot_material_ids: Vec<u64>,
    pub(crate) census_pair_ids: Vec<String>,
    pub(crate) elapsed_micros: u128,
}

struct RestoredMaterial {
    census_id: u64,
    stable_id: bool,
    structure: OrganismStructure,
    bounds: (f64, f64, f64, f64),
}

pub(crate) fn stable_material_pair_id(identity_a: u64, identity_b: u64) -> Option<String> {
    if identity_a == 0 || identity_b == 0 || identity_a == identity_b {
        return None;
    }
    Some(format!("{}:{}", identity_a.min(identity_b), identity_a.max(identity_b)))
}

pub(crate) fn stable_interface_id(
    identity_a: u64,
    identity_b: u64,
    part_a: usize,
    part_b: usize,
    endpoint_a: ConnectionEndpoint,
    endpoint_b: ConnectionEndpoint,
) -> Option<String> {
    if identity_a == 0 || identity_b == 0 || identity_a == identity_b {
        return None;
    }
    if identity_a < identity_b {
        Some(format!(
            "{}:{}:{}:{}:{:?}:{:?}",
            identity_a, identity_b, part_a, part_b, endpoint_a, endpoint_b,
        ))
    } else {
        Some(format!(
            "{}:{}:{}:{}:{:?}:{:?}",
            identity_b, identity_a, part_b, part_a, endpoint_b, endpoint_a,
        ))
    }
}

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
pub(crate) struct EnvironmentalCombineCensusComparison {
    pub(crate) census_only_count: usize,
    pub(crate) live_only_count: usize,
    pub(crate) census_only_material_pairs_never_inspected_count: usize,
    pub(crate) census_only_interfaces_not_geometrically_reached_count: usize,
    pub(crate) census_only_interfaces_rejected_by_chemistry_count: usize,
    pub(crate) live_only_sources_missing_count: usize,
    pub(crate) live_only_sources_present_but_interface_absent_count: usize,
    pub(crate) census_only_interface_ids: Vec<String>,
    pub(crate) live_only_interface_ids: Vec<String>,
    pub(crate) census_only_material_pairs_never_inspected: Vec<String>,
    pub(crate) census_only_interfaces_not_geometrically_reached: Vec<String>,
    pub(crate) census_only_interfaces_rejected_by_chemistry: Vec<String>,
    pub(crate) live_only_sources_missing: Vec<String>,
    pub(crate) live_only_sources_present_but_interface_absent: Vec<String>,
}

pub(crate) fn compare_census_to_live(
    census: &EnvironmentalCombineCensus,
    live_inspected_material_pair_ids: &std::collections::BTreeSet<String>,
    live_physical_interface_ids: &std::collections::BTreeSet<String>,
    live_discovered_interface_ids: &std::collections::BTreeSet<String>,
) -> EnvironmentalCombineCensusComparison {
    let census_ids: std::collections::BTreeSet<_> =
        census.census_pair_ids.iter().cloned().collect();
    let census_only_interface_ids: Vec<_> = census_ids
        .difference(live_discovered_interface_ids)
        .cloned()
        .collect();
    let live_only_interface_ids: Vec<_> = live_discovered_interface_ids
        .difference(&census_ids)
        .cloned()
        .collect();

    let mut census_only_material_pairs_never_inspected = Vec::new();
    let mut census_only_interfaces_not_geometrically_reached = Vec::new();
    let mut census_only_interfaces_rejected_by_chemistry = Vec::new();
    for interface_id in &census_only_interface_ids {
        let mut pieces = interface_id.splitn(3, ':');
        let Some(identity_a) = pieces.next() else {
            continue;
        };
        let Some(identity_b) = pieces.next() else {
            continue;
        };
        let material_pair_id = format!("{identity_a}:{identity_b}");
        if !live_inspected_material_pair_ids.contains(&material_pair_id) {
            census_only_material_pairs_never_inspected.push(interface_id.clone());
        } else if !live_physical_interface_ids.contains(interface_id) {
            census_only_interfaces_not_geometrically_reached.push(interface_id.clone());
        } else {
            census_only_interfaces_rejected_by_chemistry.push(interface_id.clone());
        }
    }

    let snapshot_material_ids: std::collections::BTreeSet<_> =
        census.snapshot_material_ids.iter().copied().collect();
    let mut live_only_sources_missing = Vec::new();
    let mut live_only_sources_present_but_interface_absent = Vec::new();
    for interface_id in &live_only_interface_ids {
        let mut pieces = interface_id.splitn(3, ':');
        let Some(identity_a) = pieces.next().and_then(|value| value.parse::<u64>().ok()) else {
            live_only_sources_missing.push(interface_id.clone());
            continue;
        };
        let Some(identity_b) = pieces.next().and_then(|value| value.parse::<u64>().ok()) else {
            live_only_sources_missing.push(interface_id.clone());
            continue;
        };
        if snapshot_material_ids.contains(&identity_a) && snapshot_material_ids.contains(&identity_b)
        {
            live_only_sources_present_but_interface_absent.push(interface_id.clone());
        } else {
            live_only_sources_missing.push(interface_id.clone());
        }
    }

    EnvironmentalCombineCensusComparison {
        census_only_count: census_only_interface_ids.len(),
        live_only_count: live_only_interface_ids.len(),
        census_only_material_pairs_never_inspected_count:
            census_only_material_pairs_never_inspected.len(),
        census_only_interfaces_not_geometrically_reached_count:
            census_only_interfaces_not_geometrically_reached.len(),
        census_only_interfaces_rejected_by_chemistry_count:
            census_only_interfaces_rejected_by_chemistry.len(),
        live_only_sources_missing_count: live_only_sources_missing.len(),
        live_only_sources_present_but_interface_absent_count:
            live_only_sources_present_but_interface_absent.len(),
        census_only_interface_ids,
        live_only_interface_ids,
        census_only_material_pairs_never_inspected,
        census_only_interfaces_not_geometrically_reached,
        census_only_interfaces_rejected_by_chemistry,
        live_only_sources_missing,
        live_only_sources_present_but_interface_absent,
    }
}

fn origin() -> Placement {
    Placement {
        x: 0.0,
        y: 0.0,
        rotation_radians: 0.0,
    }
}

fn restore(
    source: &PhysicalMaterial,
    catalog: &[BaseResource],
) -> Option<(OrganismStructure, (f64, f64, f64, f64))> {
    if !source.is_realized()
        || source.material.is_empty()
        || !source.material.is_valid_for_catalog(catalog)
    {
        return None;
    }

    let mut structure = OrganismStructure::new();
    crate::material_restoration::restore_material(&mut structure, source, origin(), catalog)?;

    // A radius-based AABB is deliberately conservative for every supported
    // shape, including rotated and deformable fluid forms.
    let mut bounds = (
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
    );
    for unit in &structure.units {
        let radius = unit.shape(catalog)?.form.bounding_radius();
        if !radius.is_finite() || radius < 0.0 {
            return None;
        }
        bounds.0 = bounds.0.min(unit.placement.x - radius);
        bounds.1 = bounds.1.max(unit.placement.x + radius);
        bounds.2 = bounds.2.min(unit.placement.y - radius);
        bounds.3 = bounds.3.max(unit.placement.y + radius);
    }
    if ![bounds.0, bounds.1, bounds.2, bounds.3]
        .iter()
        .all(|value| value.is_finite())
    {
        return None;
    }

    Some((structure, bounds))
}

fn bounds_are_separated(a: (f64, f64, f64, f64), b: (f64, f64, f64, f64), tolerance: f64) -> bool {
    a.1 + tolerance < b.0 || b.1 + tolerance < a.0 || a.3 + tolerance < b.2 || b.3 + tolerance < a.2
}

/// Enumerate all unique unordered material-instance pairs in the frozen field.
///
/// Existing nonzero environmental identities are retained. Legacy zero IDs and
/// duplicate IDs receive deterministic snapshot-local IDs and are reported as
/// unstable, without writing those IDs back to the field.
pub(crate) fn census_environmental_combine(
    field: &ActiveMaterialField,
    catalog: &[BaseResource],
    usable_energy: f64,
    resonant_energy: f64,
    accumulation: &BTreeMap<String, f64>,
) -> EnvironmentalCombineCensus {
    let started = Instant::now();
    // Mirror the live identity initialization on a private snapshot. This
    // preserves the same deterministic cell/material ordering without
    // mutating the simulation's field.
    let mut snapshot_field = field.clone();
    snapshot_field.assign_missing_material_identities();
    let locations: Vec<(usize, usize)> = snapshot_field
        .cells
        .iter()
        .enumerate()
        .flat_map(|(cell_index, cell)| {
            (0..cell.physical_materials.len())
                .map(move |material_index| (cell_index, material_index))
        })
        .collect();
    let sources: Vec<&PhysicalMaterial> = snapshot_field
        .cells
        .iter()
        .flat_map(|cell| cell.physical_materials.iter())
        .collect();

    let mut census = EnvironmentalCombineCensus {
        physical_materials: sources.len(),
        usable_energy_available: usable_energy,
        resonant_energy_available: resonant_energy,
        resonance_funding_supported: false,
        unordered_pairs_total: (sources.len() as u64)
            .saturating_mul(sources.len().saturating_sub(1) as u64)
            / 2,
        ..EnvironmentalCombineCensus::default()
    };

    let mut used_ids = HashSet::with_capacity(sources.len());
    let mut next_fallback_id = sources
        .iter()
        .map(|source| source.environmental_identity)
        .max()
        .unwrap_or(0)
        .saturating_add(1)
        .max(1);
    let mut restored = Vec::with_capacity(sources.len());

    for source in &sources {
        let requested_id = source.environmental_identity;
        let stable_id = requested_id != 0 && used_ids.insert(requested_id);
        let census_id = if stable_id {
            requested_id
        } else {
            census.invalid_or_duplicate_identity_materials += 1;
            while next_fallback_id == 0 || used_ids.contains(&next_fallback_id) {
                let Some(next) = next_fallback_id.checked_add(1) else {
                    next_fallback_id = 0;
                    break;
                };
                next_fallback_id = next;
            }
            let fallback = next_fallback_id;
            if fallback != 0 {
                used_ids.insert(fallback);
                next_fallback_id = fallback.checked_add(1).unwrap_or(0);
            }
            fallback
        };

        match restore(source, catalog) {
            Some((structure, bounds)) => {
                census.material_restorations_succeeded += 1;
                if stable_id {
                    census.snapshot_material_ids.push(census_id);
                }
                restored.push(Some(RestoredMaterial {
                    census_id,
                    stable_id,
                    structure,
                    bounds,
                }));
            }
            None => {
                census.material_restorations_failed += 1;
                restored.push(None);
            }
        }
    }

    for (index_a, restored_a) in restored.iter().enumerate() {
        let Some(material_a) = restored_a.as_ref() else {
            continue;
        };
        let (cell_a, material_index_a) = locations[index_a];
        let source_a = sources[index_a];
        for (index_b, restored_b) in restored.iter().enumerate().skip(index_a + 1) {
            let Some(material_b) = restored_b.as_ref() else {
                continue;
            };
            let (cell_b, material_index_b) = locations[index_b];
            let source_b = sources[index_b];
            census.pairs_with_both_materials_restored += 1;

            if bounds_are_separated(
                material_a.bounds,
                material_b.bounds,
                crate::combine_runtime::COMBINE_CONTACT_TOLERANCE,
            ) {
                census.broad_phase_rejected_pairs += 1;
                continue;
            }
            census.broad_phase_surviving_pairs += 1;

            let mut combined = material_a.structure.clone();
            let first_b_unit = combined.units.len();
            if combined
                .append_graph_remapping_ids(&material_b.structure)
                .is_none()
            {
                continue;
            }
            let indices_a: Vec<usize> = (0..first_b_unit).collect();
            let indices_b: Vec<usize> = (first_b_unit..combined.units.len()).collect();

            // A COMBINE pair cannot be physically eligible if any constituent
            // of the incoming object penetrates the already-restored object.
            if indices_b.iter().any(|&unit_index| {
                crate::construction_runtime::placed_unit_overlaps(
                    &combined,
                    &combined.units[unit_index],
                    &indices_b,
                    catalog,
                )
            }) {
                census.pairs_rejected_for_cross_material_overlap += 1;
                continue;
            }

            let mut compatibility_cache = crate::contact::ConnectionCompatibilityCache::new();
            for &unit_a in &indices_a {
                for &unit_b in &indices_b {
                    census.constituent_pairs_evaluated += 1;
                    for candidate in crate::contact::connection_pair_candidates_cached(
                        &combined,
                        unit_a,
                        unit_b,
                        catalog,
                        &mut compatibility_cache,
                    ) {
                        if !candidate.available_a
                            || !candidate.available_b
                            || candidate.distance
                                > crate::combine_runtime::COMBINE_CONTACT_TOLERANCE
                        {
                            continue;
                        }
                        census.physical_interfaces += 1;
                        let Some((_formation, _work, _investment, required_energy)) =
                            crate::combine_runtime::selected_candidate_evaluation(
                                &combined, unit_a, unit_b, candidate, catalog,
                            )
                        else {
                            continue;
                        };
                        census.chemically_eligible_interfaces += 1;

                        let [(material_name_a, _)] =
                            combined.units[unit_a].material.parts.as_slice()
                        else {
                            continue;
                        };
                        let [(material_name_b, _)] =
                            combined.units[unit_b].material.parts.as_slice()
                        else {
                            continue;
                        };
                        if crate::geometry_reference_library::resolve_live_contact_candidate(
                            material_name_a,
                            &combined.units[unit_a],
                            material_name_b,
                            &combined.units[unit_b],
                            candidate,
                            catalog,
                        )
                        .is_none()
                        {
                            continue;
                        }
                        census.live_geometry_resolvable_interfaces += 1;

                        let Some(properties_a) = combined.units[unit_a].properties(catalog) else {
                            continue;
                        };
                        let Some(properties_b) = combined.units[unit_b].properties(catalog) else {
                            continue;
                        };
                        let Some(position_a) =
                            crate::environmental_combine::constituent_chemical_position(
                                &combined.units[unit_a],
                                catalog,
                            )
                        else {
                            continue;
                        };
                        let Some(position_b) =
                            crate::environmental_combine::constituent_chemical_position(
                                &combined.units[unit_b],
                                catalog,
                            )
                        else {
                            continue;
                        };
                        let Some(contact) = crate::chemistry::contact_factor(
                            candidate.distance,
                            crate::chemistry::CHEMICAL_CONTACT_RADIUS,
                        ) else {
                            continue;
                        };
                        let Some(engagement) =
                            crate::chemistry::interface_engagement(candidate.facing)
                        else {
                            continue;
                        };
                        let Some(static_potential) = crate::chemistry::interaction_potential(
                            position_a,
                            position_b,
                            crate::chemistry::CHEMICAL_K,
                            crate::chemistry::CHEMICAL_D_MAX,
                        ) else {
                            continue;
                        };
                        census.interfaces_with_chemical_activation_signal += 1;

                        let accumulation_key = crate::environmental_combine::source_pair_key(
                            cell_a,
                            material_index_a,
                            source_a,
                            cell_b,
                            material_index_b,
                            source_b,
                            unit_a,
                            unit_b,
                            candidate,
                        );
                        let previous = accumulation.get(&accumulation_key).copied().unwrap_or(0.0);
                        let Some(barrier) = crate::chemistry::activation_barrier_from_bond_strength(
                            crate::combine::bond_strength(properties_a, properties_b),
                        ) else {
                            continue;
                        };
                        if crate::chemistry::normalized_chemistry_to_activation_signal(previous)
                            .is_some_and(|signal| crate::chemistry::activated(signal, barrier))
                        {
                            census.activation_already_reached_interfaces += 1;
                        }
                        census.max_accumulated_reaction =
                            census.max_accumulated_reaction.max(previous);

                        if let Some(next) = crate::chemistry::accumulate_reaction(
                            previous,
                            static_potential,
                            contact,
                            engagement,
                            crate::chemistry::CHEMICAL_DISSIPATION,
                        ) {
                            if next > 0.0 {
                                census.max_accumulated_reaction =
                                    census.max_accumulated_reaction.max(next);
                                if crate::chemistry::normalized_chemistry_to_activation_signal(next)
                                    .is_some_and(|signal| {
                                        crate::chemistry::activated(signal, barrier)
                                    })
                                {
                                    census.activation_reached_after_one_more_evaluation += 1;
                                } else {
                                    census.activation_below_threshold_after_one_more_evaluation +=
                                        1;
                                }
                            }
                        }

                        if usable_energy.is_finite()
                            && usable_energy >= 0.0
                            && required_energy.is_finite()
                            && required_energy <= usable_energy
                        {
                            census.usable_energy_sufficient_interfaces += 1;
                        }

                        if material_a.stable_id && material_b.stable_id {
                            if let Some(interface_id) = stable_interface_id(
                                material_a.census_id,
                                material_b.census_id,
                                unit_a,
                                unit_b - first_b_unit,
                                candidate.endpoint_a,
                                candidate.endpoint_b,
                            ) {
                                census.census_pair_ids.push(interface_id);
                            }
                        }
                    }
                }
            }
        }
    }

    census.census_pair_ids.sort();
    census.census_pair_ids.dedup();
    census.elapsed_micros = started.elapsed().as_micros();
    census
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::Material;

    fn physical_carbon(x: f64, identity: u64, catalog: &[BaseResource]) -> PhysicalMaterial {
        let mut material = PhysicalMaterial::realized(
            Material::free_base("Carbon", 1.0),
            vec![Placement {
                x,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            catalog,
        )
        .expect("valid physical carbon");
        material.environmental_identity = identity;
        material
    }

    #[test]
    fn census_enumerates_same_cell_pairs_and_is_read_only() {
        let catalog = crate::resources::default_catalog();
        let mut field = ActiveMaterialField::new(25.0, 25.0, 25.0);
        field.cells[0]
            .physical_materials
            .push(physical_carbon(0.0, 11, &catalog));
        field.cells[0]
            .physical_materials
            .push(physical_carbon(4.0, 12, &catalog));
        field.cells[0]
            .physical_materials
            .push(physical_carbon(8.0, 13, &catalog));

        let before = serde_json::to_vec(&field).expect("serialize field");
        let census = census_environmental_combine(&field, &catalog, 0.0, 0.0, &BTreeMap::new());
        let after = serde_json::to_vec(&field).expect("serialize field");

        assert_eq!(before, after, "census must not mutate the field");
        assert_eq!(census.physical_materials, 3);
        assert_eq!(census.unordered_pairs_total, 3);
        assert_eq!(census.material_restorations_succeeded, 3);
        assert_eq!(census.pairs_with_both_materials_restored, 3);
    }

    #[test]
    fn stable_interface_identity_is_independent_of_material_pair_order() {
        let endpoint_a = ConnectionEndpoint::Corner { point_index: 0 };
        let endpoint_b = ConnectionEndpoint::Boundary {
            angle_radians: 0.25,
        };
        assert_eq!(
            stable_interface_id(10, 20, 2, 3, endpoint_a, endpoint_b),
            stable_interface_id(20, 10, 3, 2, endpoint_b, endpoint_a)
        );
    }

    #[test]
    fn comparison_separates_uninspected_geometry_and_chemistry_gaps() {
        let census = EnvironmentalCombineCensus {
            snapshot_material_ids: vec![1, 2],
            census_pair_ids: vec![
                "1:2:0:0:corner-a:corner-b".to_owned(),
                "1:2:0:1:corner-c:corner-d".to_owned(),
            ],
            ..EnvironmentalCombineCensus::default()
        };
        let inspected = std::collections::BTreeSet::from(["1:2".to_owned()]);
        let physical = std::collections::BTreeSet::from([
            "1:2:0:0:corner-a:corner-b".to_owned(),
        ]);
        let discovered = std::collections::BTreeSet::from([
            "1:2:9:9:old-a:old-b".to_owned(),
            "3:4:0:0:missing-a:missing-b".to_owned(),
        ]);

        let comparison = compare_census_to_live(&census, &inspected, &physical, &discovered);
        assert_eq!(comparison.census_only_count, 2);
        assert_eq!(
            comparison.census_only_interfaces_not_geometrically_reached_count,
            1
        );
        assert_eq!(
            comparison.census_only_interfaces_rejected_by_chemistry_count,
            1
        );
        assert_eq!(comparison.census_only_material_pairs_never_inspected_count, 0);
        assert_eq!(comparison.live_only_count, 2);
        assert_eq!(comparison.live_only_sources_missing_count, 1);
        assert_eq!(
            comparison.live_only_sources_present_but_interface_absent_count,
            1
        );
    }

    #[test]
    fn census_uses_live_ordered_snapshot_ids_without_mutating_legacy_instances() {
        let catalog = crate::resources::default_catalog();
        let mut field = ActiveMaterialField::new(25.0, 25.0, 25.0);
        field.cells[0]
            .physical_materials
            .push(physical_carbon(0.0, 0, &catalog));
        field.cells[0]
            .physical_materials
            .push(physical_carbon(4.0, 0, &catalog));

        let census = census_environmental_combine(&field, &catalog, 0.0, 0.0, &BTreeMap::new());
        assert_eq!(census.invalid_or_duplicate_identity_materials, 0);
        assert_eq!(
            field.cells[0].physical_materials[0].environmental_identity,
            0
        );
        assert_eq!(
            field.cells[0].physical_materials[1].environmental_identity,
            0
        );
    }
}
