//! Independent, read-only census of environmental COMBINE opportunities.
//!
//! This deliberately enumerates every unordered physical-material pair. It does
//! not call the bounded live discovery path or mutate chemistry, activation,
//! energy, or field state. Exact endpoint and chemistry eligibility still come
//! from the same authorities used by live COMBINE.

use std::collections::HashSet;
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
    /// Interfaces whose instantaneous usable-energy requirement is met.
    /// This does not include compatible resonance or accumulated activation.
    pub(crate) usable_energy_sufficient_interfaces: u64,
    pub(crate) census_pair_ids: Vec<String>,
    pub(crate) elapsed_micros: u128,
}

struct RestoredMaterial {
    census_id: u64,
    stable_id: bool,
    structure: OrganismStructure,
    bounds: (f64, f64, f64, f64),
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
    pub(crate) census_only_interface_ids: Vec<String>,
    pub(crate) live_only_interface_ids: Vec<String>,
}

pub(crate) fn compare_census_to_live(
    census: &EnvironmentalCombineCensus,
    live_interface_ids: &[String],
) -> EnvironmentalCombineCensusComparison {
    let census_ids: std::collections::BTreeSet<_> =
        census.census_pair_ids.iter().cloned().collect();
    let live_ids: std::collections::BTreeSet<_> = live_interface_ids.iter().cloned().collect();
    let census_only_interface_ids: Vec<_> = census_ids.difference(&live_ids).cloned().collect();
    let live_only_interface_ids: Vec<_> = live_ids.difference(&census_ids).cloned().collect();
    EnvironmentalCombineCensusComparison {
        census_only_count: census_only_interface_ids.len(),
        live_only_count: live_only_interface_ids.len(),
        census_only_interface_ids,
        live_only_interface_ids,
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
) -> EnvironmentalCombineCensus {
    let started = Instant::now();
    let sources: Vec<&PhysicalMaterial> = field
        .cells
        .iter()
        .flat_map(|cell| cell.physical_materials.iter())
        .collect();

    let mut census = EnvironmentalCombineCensus {
        physical_materials: sources.len(),
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
        for restored_b in restored.iter().skip(index_a + 1) {
            let Some(material_b) = restored_b.as_ref() else {
                continue;
            };
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
        let census = census_environmental_combine(&field, &catalog, 0.0);
        let after = serde_json::to_vec(&field).expect("serialize field");

        assert_eq!(before, after, "census must not mutate the field");
        assert_eq!(census.physical_materials, 3);
        assert_eq!(census.unordered_pairs_total, 3);
        assert_eq!(census.material_restorations_succeeded, 3);
        assert_eq!(census.pairs_with_both_materials_restored, 3);
    }

    #[test]
    fn census_assigns_snapshot_local_ids_without_mutating_legacy_instances() {
        let catalog = crate::resources::default_catalog();
        let mut field = ActiveMaterialField::new(25.0, 25.0, 25.0);
        field.cells[0]
            .physical_materials
            .push(physical_carbon(0.0, 0, &catalog));
        field.cells[0]
            .physical_materials
            .push(physical_carbon(4.0, 0, &catalog));

        let census = census_environmental_combine(&field, &catalog, 0.0);
        assert_eq!(census.invalid_or_duplicate_identity_materials, 2);
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
