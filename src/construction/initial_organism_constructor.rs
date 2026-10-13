//! Formation-driven physical construction for the first valid organism.
//!
//! Startup composes local poses from persisted Bob geometry and admits each
//! attachment only through live overlap, contact, COMBINE, and energy checks.
//! No fixed scaffold, target lattice, or artificial construction-energy reserve
//! is used.

use crate::resources::{BaseResource, Form, Material, PhysicalState, Shape};
use crate::state::EnergyLedger;
use crate::structure::Placement;

#[derive(Clone, Debug)]
pub(crate) struct ValidConstruction {
    pub structure: crate::structure::OrganismStructure,
    pub energy: f64,
    pub acquired_resource_placements: Vec<(String, Placement)>,
    pub acquired_resource_shapes: Vec<(String, Shape)>,
    pub fluid_geometry_status: FluidGeometryStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FluidGeometryStatus {
    BobSuggestionAccepted,
    CoverageMiss,
    CandidatesRejected,
}

struct PersistedAttachment {
    structure: crate::structure::OrganismStructure,
    new_unit_index: usize,
    ledger: EnergyLedger,
    energy: f64,
}

#[derive(Debug, PartialEq, Eq)]
enum PersistedAttachmentFailure {
    CoverageMiss,
    CandidatesRejected {
        suggestions: usize,
        raw_pair_placements: usize,
        pair_placements_dropped_by_cap: usize,
        edge_placements_examined: usize,
        edge_placements_added: usize,
        duplicate_placements: usize,
        placement_attempts: usize,
        last_overlap_rejection: Option<String>,
        restoration_failures: usize,
        overlap_rejections: usize,
        pair_overlap_rejections: usize,
        edge_overlap_rejections: usize,
        no_bond_contact: usize,
        evaluation_rejections: usize,
        combine_rejections: usize,
        energy_rejections: usize,
        last_combine_rejection: Option<String>,
    },
}

fn select_diverse_placements(placements: Vec<Placement>, limit: usize) -> Vec<Placement> {
    if placements.len() <= limit || limit == 0 {
        return if limit == 0 { Vec::new() } else { placements };
    }

    let min_x = placements.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
    let max_x = placements
        .iter()
        .map(|p| p.x)
        .fold(f64::NEG_INFINITY, f64::max);
    let min_y = placements.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
    let max_y = placements
        .iter()
        .map(|p| p.y)
        .fold(f64::NEG_INFINITY, f64::max);
    let spatial_scale = (max_x - min_x).max(max_y - min_y).max(1e-9);
    let mean_x = placements.iter().map(|p| p.x).sum::<f64>() / placements.len() as f64;
    let mean_y = placements.iter().map(|p| p.y).sum::<f64>() / placements.len() as f64;
    let mean_sin = placements
        .iter()
        .map(|p| p.rotation_radians.sin())
        .sum::<f64>();
    let mean_cos = placements
        .iter()
        .map(|p| p.rotation_radians.cos())
        .sum::<f64>();
    let mean_rotation = mean_sin.atan2(mean_cos);

    let distance_squared = |a: Placement, b: Placement| {
        let dx = (a.x - b.x) / spatial_scale;
        let dy = (a.y - b.y) / spatial_scale;
        let angle = (a.rotation_radians - b.rotation_radians + std::f64::consts::PI)
            .rem_euclid(std::f64::consts::TAU)
            - std::f64::consts::PI;
        dx * dx + dy * dy + (angle / std::f64::consts::PI).powi(2)
    };
    let centroid = Placement {
        x: mean_x,
        y: mean_y,
        rotation_radians: mean_rotation,
    };
    let first_index = placements
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| {
            distance_squared(**a, centroid).total_cmp(&distance_squared(**b, centroid))
        })
        .map(|(index, _)| index)
        .expect("non-empty placement list has a first candidate");

    let mut selected_indices = vec![first_index];
    while selected_indices.len() < limit {
        let next_index = placements
            .iter()
            .enumerate()
            .filter(|(index, _)| !selected_indices.contains(index))
            .max_by(|(_, a), (_, b)| {
                let min_a = selected_indices
                    .iter()
                    .map(|&selected| distance_squared(**a, placements[selected]))
                    .fold(f64::INFINITY, f64::min);
                let min_b = selected_indices
                    .iter()
                    .map(|&selected| distance_squared(**b, placements[selected]))
                    .fold(f64::INFINITY, f64::min);
                min_a.total_cmp(&min_b)
            })
            .map(|(index, _)| index)
            .expect("candidate count exceeds the selection limit");
        selected_indices.push(next_index);
    }

    selected_indices
        .into_iter()
        .map(|index| placements[index])
        .collect()
}

fn placement_signature(placement: Placement) -> String {
    format!(
        "{:.3},{:.3},{:.4}",
        placement.x, placement.y, placement.rotation_radians
    )
}

/// Replay two-piece formations already stored in Bob as local growth proposals.
/// The stored anchor constituent is rigidly aligned to the live anchor; the
/// other constituent's recorded relative pose is preserved exactly.
fn persisted_pair_formation_placements(
    anchor_material: &str,
    anchor_placement: Placement,
    candidate_resource: &BaseResource,
    library: &crate::geometry_reference_library::GeometryLibrary,
) -> Vec<Placement> {
    let mut placements = std::collections::BTreeMap::<String, Placement>::new();
    for formation in library.formations() {
        if formation.constituents.len() != 2
            || !formation.bonds.iter().any(|bond| {
                (bond.constituent_a == 0 && bond.constituent_b == 1)
                    || (bond.constituent_a == 1 && bond.constituent_b == 0)
            })
        {
            continue;
        }

        for (anchor_index, candidate_index) in [(0, 1), (1, 0)] {
            let stored_anchor = &formation.constituents[anchor_index];
            let stored_candidate = &formation.constituents[candidate_index];
            if stored_anchor.resource.as_str() != anchor_material
                || stored_candidate.resource.as_str() != candidate_resource.name.as_str()
            {
                continue;
            }

            let rotation_delta =
                anchor_placement.rotation_radians - stored_anchor.placement.rotation_radians;
            let (sin, cos) = rotation_delta.sin_cos();
            let dx = stored_candidate.placement.x - stored_anchor.placement.x;
            let dy = stored_candidate.placement.y - stored_anchor.placement.y;
            let placement = Placement {
                x: anchor_placement.x + dx * cos - dy * sin,
                y: anchor_placement.y + dx * sin + dy * cos,
                rotation_radians: stored_candidate.placement.rotation_radians + rotation_delta,
            };
            if !placement.x.is_finite()
                || !placement.y.is_finite()
                || !placement.rotation_radians.is_finite()
            {
                continue;
            }
            placements
                .entry(placement_signature(placement))
                .or_insert(placement);
        }
    }
    placements.into_values().collect()
}

/// Try one local growth step using only persisted pair formations and rigid-edge poses from Bob.
/// The returned structure and energy are committed together only after the
/// ordinary live COMBINE transaction succeeds. No intended lattice pose,
/// snapping, or analytic fallback is available in this path.
fn try_persisted_bob_attachments(
    structure: &crate::structure::OrganismStructure,
    anchor_index: usize,
    resource: &BaseResource,
    library: &crate::geometry_reference_library::GeometryLibrary,
    catalog: &[BaseResource],
    ledger: &EnergyLedger,
    _available_energy: f64,
) -> Result<Vec<PersistedAttachment>, PersistedAttachmentFailure> {
    let Some(anchor) = structure.units.get(anchor_index) else {
        return Err(PersistedAttachmentFailure::CandidatesRejected {
            suggestions: 0,
            raw_pair_placements: 0,
            pair_placements_dropped_by_cap: 0,
            edge_placements_examined: 0,
            edge_placements_added: 0,
            duplicate_placements: 0,
            placement_attempts: 0,
            last_overlap_rejection: None,
            restoration_failures: 0,
            overlap_rejections: 0,
            pair_overlap_rejections: 0,
            edge_overlap_rejections: 0,
            no_bond_contact: 0,
            evaluation_rejections: 0,
            combine_rejections: 0,
            energy_rejections: 0,
            last_combine_rejection: None,
        });
    };
    let Some(anchor_material) = anchor.material.parts.first().map(|part| part.0.as_str()) else {
        return Err(PersistedAttachmentFailure::CandidatesRejected {
            suggestions: 0,
            raw_pair_placements: 0,
            pair_placements_dropped_by_cap: 0,
            edge_placements_examined: 0,
            edge_placements_added: 0,
            duplicate_placements: 0,
            placement_attempts: 0,
            last_overlap_rejection: None,
            restoration_failures: 0,
            overlap_rejections: 0,
            pair_overlap_rejections: 0,
            edge_overlap_rejections: 0,
            no_bond_contact: 0,
            evaluation_rejections: 0,
            combine_rejections: 0,
            energy_rejections: 0,
            last_combine_rejection: None,
        });
    };
    let mut candidate_placements =
        persisted_pair_formation_placements(anchor_material, anchor.placement, resource, library);
    let raw_pair_placements = candidate_placements.len();
    // Keep persisted pair proposals diverse and bounded. Do not pre-reject
    // proposals using a separate geometry path: restoration followed by the
    // authoritative live overlap/contact/COMBINE checks decides validity.
    candidate_placements = select_diverse_placements(candidate_placements, 6);
    let pair_placements_selected = candidate_placements.len();
    let pair_placements_dropped_by_cap =
        raw_pair_placements.saturating_sub(candidate_placements.len());
    let mut seen_placements = candidate_placements
        .iter()
        .copied()
        .map(placement_signature)
        .collect::<std::collections::BTreeSet<_>>();
    let mut edge_placements_examined = 0usize;
    let mut edge_placements_added = 0usize;
    let mut duplicate_placements = 0usize;
    for suggestion in library.suggest_persisted_rigid_edge_placements(
        anchor_material,
        anchor.placement,
        resource,
        catalog,
    ) {
        if candidate_placements.len() >= 12 {
            break;
        }
        edge_placements_examined += 1;
        if !seen_placements.insert(placement_signature(suggestion.placement)) {
            duplicate_placements += 1;
            continue;
        }
        // Keep edge suggestions for the same live validation as pair poses.
        candidate_placements.push(suggestion.placement);
        edge_placements_added += 1;
    }
    if candidate_placements.is_empty() && raw_pair_placements == 0 && edge_placements_examined == 0
    {
        return Err(PersistedAttachmentFailure::CoverageMiss);
    }
    let mut placement_attempts = 0usize;
    let mut last_overlap_rejection = None;

    let mut restoration_failures = 0;
    let mut overlap_rejections = 0;
    let mut pair_overlap_rejections = 0;
    let mut edge_overlap_rejections = 0;
    let mut no_bond_contact = 0;
    let mut evaluation_rejections = 0;
    let mut combine_rejections = 0;
    let mut energy_rejections = 0;
    let mut last_combine_rejection = None;
    let Some(instance) = crate::physical_material::PhysicalMaterial::realized(
        Material::free_base(resource.name.clone(), 1.0),
        vec![Placement {
            x: 0.0,
            y: 0.0,
            rotation_radians: 0.0,
        }],
        catalog,
    ) else {
        return Err(PersistedAttachmentFailure::CandidatesRejected {
            suggestions: candidate_placements.len(),
            raw_pair_placements,
            pair_placements_dropped_by_cap,
            edge_placements_examined,
            edge_placements_added,
            duplicate_placements,
            placement_attempts,
            last_overlap_rejection,
            restoration_failures: candidate_placements.len(),
            overlap_rejections,
            pair_overlap_rejections,
            edge_overlap_rejections,
            no_bond_contact,
            evaluation_rejections,
            combine_rejections,
            energy_rejections: 0,
            last_combine_rejection: Some(
                "candidate material failed physical realization".to_owned(),
            ),
        });
    };

    let mut successful = Vec::new();
    let mut suggestions_examined = 0usize;
    for (suggestion_index, suggestion) in candidate_placements.iter().enumerate() {
        // Bound both successful branches and rejected attempts; every returned
        // pose is still live-validated.
        if successful.len() >= 4 || suggestions_examined >= 12 {
            break;
        }
        suggestions_examined += 1;
        placement_attempts += 1;
        let mut trial = structure.clone();
        let indices = match crate::material_restoration::restore_material(
            &mut trial,
            &instance,
            *suggestion,
            catalog,
        ) {
            Some(indices) if !indices.is_empty() => indices,
            _ => {
                restoration_failures += 1;
                continue;
            }
        };
        let overlap = indices.iter().find_map(|&candidate_index| {
            crate::construction_runtime::overlapping_unit_indices(
                &trial,
                &trial.units[candidate_index],
                &indices,
                catalog,
            )
            .first()
            .copied()
            .map(|existing_index| (candidate_index, existing_index))
        });
        if let Some((candidate_unit_index, existing_unit_index)) = overlap {
            overlap_rejections += 1;
            let source = if suggestion_index < pair_placements_selected {
                pair_overlap_rejections += 1;
                "pair-formation"
            } else {
                edge_overlap_rejections += 1;
                "rigid-edge"
            };
            let candidate_pose = trial.units[candidate_unit_index].placement;
            let existing_pose = trial.units[existing_unit_index].placement;
            let existing_material = trial.units[existing_unit_index]
                .material
                .parts
                .first()
                .map(|part| part.0.as_str())
                .unwrap_or("unknown");
            last_overlap_rejection = Some(format!(
                "source={source}; anchor={anchor_index}:{anchor_material}@({:.4},{:.4},{:.4}); candidate={}:{}@({:.4},{:.4},{:.4}); overlaps={existing_unit_index}:{existing_material}@({:.4},{:.4},{:.4})",
                anchor.placement.x,
                anchor.placement.y,
                anchor.placement.rotation_radians,
                resource.name,
                candidate_unit_index,
                candidate_pose.x,
                candidate_pose.y,
                candidate_pose.rotation_radians,
                existing_pose.x,
                existing_pose.y,
                existing_pose.rotation_radians,
            ));
            continue;
        }

        let new_unit_index = indices[0];
        let mut bond_cache = crate::contact::ConnectionCompatibilityCache::new();
        let contacts = crate::contact::connection_pair_candidates_cached(
            &trial,
            anchor_index,
            new_unit_index,
            catalog,
            &mut bond_cache,
        )
        .into_iter()
        .filter(|candidate| {
            candidate.distance <= crate::combine_runtime::COMBINE_CONTACT_TOLERANCE
                && candidate.available_a
                && candidate.available_b
        })
        .collect::<Vec<_>>();
        if contacts.is_empty() {
            no_bond_contact += 1;
            continue;
        }

        for candidate in contacts {
            let Some((_, _, investment, required_energy)) =
                crate::combine_runtime::selected_candidate_evaluation(
                    &trial,
                    anchor_index,
                    new_unit_index,
                    candidate,
                    catalog,
                )
            else {
                evaluation_rejections += 1;
                continue;
            };
            let _ = required_energy;
            let mut committed_structure = trial.clone();
            let mut committed_ledger = *ledger;
            let mut committed_energy = _available_energy;
            let mut transaction_cache = crate::contact::ConnectionCompatibilityCache::new();
            if let Err(reason) =
                crate::combine_runtime::form_selected_bond_for_freeform_construction(
                    &mut committed_structure,
                    anchor_index,
                    new_unit_index,
                    candidate,
                    investment,
                    catalog,
                    &mut transaction_cache,
                    &mut committed_ledger,
                    &mut committed_energy,
                )
            {
                combine_rejections += 1;
                last_combine_rejection = Some(reason.to_owned());
                continue;
            }
            successful.push(PersistedAttachment {
                structure: committed_structure,
                new_unit_index,
                ledger: committed_ledger,
                energy: committed_energy,
            });
            break;
        }
    }

    if !successful.is_empty() {
        return Ok(successful);
    }

    Err(PersistedAttachmentFailure::CandidatesRejected {
        suggestions: candidate_placements.len(),
        raw_pair_placements,
        pair_placements_dropped_by_cap,
        edge_placements_examined,
        edge_placements_added,
        duplicate_placements,
        placement_attempts,
        last_overlap_rejection,
        restoration_failures,
        overlap_rejections,
        pair_overlap_rejections,
        edge_overlap_rejections,
        no_bond_contact,
        evaluation_rejections,
        combine_rejections,
        energy_rejections,
        last_combine_rejection,
    })
}

fn add_unit(
    structure: &mut crate::structure::OrganismStructure,
    resource: &BaseResource,
    center: (f64, f64),
    catalog: &[BaseResource],
) -> Result<usize, String> {
    let instance = crate::physical_material::PhysicalMaterial::realized(
        Material::free_base(resource.name.clone(), 1.0),
        vec![Placement {
            x: 0.0,
            y: 0.0,
            rotation_radians: 0.0,
        }],
        catalog,
    )
    .ok_or_else(|| format!("failed to realize construction unit {}", resource.name))?;
    let indices = crate::material_restoration::restore_material(
        structure,
        &instance,
        Placement {
            x: center.0,
            y: center.1,
            rotation_radians: 0.0,
        },
        catalog,
    )
    .ok_or_else(|| format!("failed to restore construction unit {}", resource.name))?;
    indices.first().copied().ok_or_else(|| {
        format!(
            "construction unit {} restored no physical constituent",
            resource.name
        )
    })
}

fn bond_units(
    structure: &mut crate::structure::OrganismStructure,
    unit_a: usize,
    unit_b: usize,
    catalog: &[BaseResource],
    ledger: &mut EnergyLedger,
    energy: &mut f64,
) -> Result<(), String> {
    let mut bonded = false;
    let mut used_a = Vec::new();
    let mut used_b = Vec::new();

    // A shared wall segment is sealed by at most two endpoint bonds. Recompute
    // the physical candidate graph after each commit so endpoint availability
    // cannot become stale after the first permanent bond.
    for _ in 0..2 {
        let mut cache = crate::contact::ConnectionCompatibilityCache::new();
        let candidates = crate::contact::connection_pair_candidates_cached(
            structure, unit_a, unit_b, catalog, &mut cache,
        )
        .into_iter()
        .filter(|candidate| {
            candidate.distance <= crate::combine_runtime::COMBINE_CONTACT_TOLERANCE
                && candidate.available_a
                && candidate.available_b
        })
        .collect::<Vec<_>>();

        let candidate = candidates
            .iter()
            .filter(|candidate| {
                matches!(
                    candidate.endpoint_a,
                    crate::structure::ConnectionEndpoint::Corner { .. }
                ) && matches!(
                    candidate.endpoint_b,
                    crate::structure::ConnectionEndpoint::Corner { .. }
                ) && !used_a.contains(&candidate.endpoint_a)
                    && !used_b.contains(&candidate.endpoint_b)
            })
            .min_by(|a, b| {
                a.distance
                    .partial_cmp(&b.distance)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned()
            .or_else(|| {
                if bonded {
                    None
                } else {
                    candidates.into_iter().max_by(|a, b| {
                        a.distance
                            .partial_cmp(&b.distance)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                }
            });

        let Some(candidate) = candidate else {
            if bonded {
                break;
            }
            return Err(format!(
                "no physical contact between construction units {unit_a} and {unit_b}"
            ));
        };

        let endpoint_a = candidate.endpoint_a.clone();
        let endpoint_b = candidate.endpoint_b.clone();
        let (_, _, investment, _) = crate::combine_runtime::selected_candidate_evaluation(
            structure, unit_a, unit_b, candidate, catalog,
        )
        .ok_or_else(|| format!("physical bond candidate {unit_a}-{unit_b} failed evaluation"))?;

        // Recompute candidates on the next iteration after this transaction
        // changes endpoint availability.
        crate::combine_runtime::form_selected_bond_for_freeform_construction(
            structure, unit_a, unit_b, candidate, investment, catalog, &mut cache, ledger, energy,
        )
        .map_err(|reason| {
            format!("physical bond transaction {unit_a}-{unit_b} failed: {reason}")
        })?;

        used_a.push(endpoint_a);
        used_b.push(endpoint_b);
        bonded = true;
    }

    Ok(())
}

#[derive(Clone)]
struct ConstructionSearchState {
    structure: crate::structure::OrganismStructure,
    ledger: EnergyLedger,
    energy: f64,
}

fn construction_state_signature(structure: &crate::structure::OrganismStructure) -> String {
    let unit_keys = structure
        .units
        .iter()
        .map(|unit| {
            let materials = unit
                .material
                .parts
                .iter()
                .map(|part| part.0.as_str())
                .collect::<Vec<_>>()
                .join("+");
            format!(
                "{materials}@{:.3},{:.3},{:.4}",
                unit.placement.x, unit.placement.y, unit.placement.rotation_radians
            )
        })
        .collect::<Vec<_>>();
    let mut sorted_units = unit_keys.clone();
    sorted_units.sort();

    let mut bond_keys = structure
        .bonds
        .iter()
        .map(|bond| {
            let a_index = structure
                .units
                .iter()
                .position(|unit| unit.physical_id == bond.endpoint_a.constituent_id)
                .unwrap_or(usize::MAX);
            let b_index = structure
                .units
                .iter()
                .position(|unit| unit.physical_id == bond.endpoint_b.constituent_id)
                .unwrap_or(usize::MAX);
            let a = format!(
                "{}:{:?}",
                unit_keys.get(a_index).map(String::as_str).unwrap_or("?"),
                bond.endpoint_a.location
            );
            let b = format!(
                "{}:{:?}",
                unit_keys.get(b_index).map(String::as_str).unwrap_or("?"),
                bond.endpoint_b.location
            );
            if a <= b {
                format!("{a}--{b}")
            } else {
                format!("{b}--{a}")
            }
        })
        .collect::<Vec<_>>();
    bond_keys.sort();
    format!("{}#{}", sorted_units.join(";"), bond_keys.join(";"))
}

/// Complete physically available contacts through the ordinary bond authority.
/// Every candidate structure is an isolated trial, so a failed optional closure
/// cannot corrupt another queued construction state.
fn close_touching_bonds(
    structure: &mut crate::structure::OrganismStructure,
    new_unit_index: usize,
    catalog: &[BaseResource],
    ledger: &mut EnergyLedger,
    energy: &mut f64,
) {
    if new_unit_index >= structure.units.len() {
        return;
    }
    // Adding one unit cannot change contacts between older units. Check only
    // the new unit's interfaces, and attempt each interface at most once.
    for other_index in 0..structure.units.len() {
        if other_index == new_unit_index {
            continue;
        }
        let (a, b) = if other_index < new_unit_index {
            (other_index, new_unit_index)
        } else {
            (new_unit_index, other_index)
        };
        let mut cache = crate::contact::ConnectionCompatibilityCache::new();
        let touches =
            crate::contact::connection_pair_candidates_cached(structure, a, b, catalog, &mut cache)
                .into_iter()
                .any(|candidate| {
                    candidate.distance <= crate::combine_runtime::COMBINE_CONTACT_TOLERANCE
                        && candidate.available_a
                        && candidate.available_b
                });
        if touches {
            let _ = bond_units(structure, a, b, catalog, ledger, energy);
        }
    }
}

fn construction_state_score(state: &ConstructionSearchState) -> (usize, usize, usize) {
    let units = state.structure.units.len();
    let bonds = state.structure.bonds.len();
    (bonds.saturating_add(1).saturating_sub(units), units, bonds)
}

/// Search local arrangements from persisted Bob edge families. Every added
/// constituent must come from a stored pose; live overlap, contact, COMBINE,
/// and energy accounting remain authoritative. A bounded beam prioritizes
/// structures with actual bonded cycles, which are more promising cavity
/// candidates than open chains.
fn construct_persisted_bob_structure(
    catalog: &[BaseResource],
    library: &crate::geometry_reference_library::GeometryLibrary,
    acquisition_candidates: &[&BaseResource],
    initial_energy: f64,
) -> Result<ValidConstruction, String> {
    if !initial_energy.is_finite() || initial_energy < 0.0 {
        return Err("constructor requires finite non-negative supplied energy".into());
    }
    let structural_resources = catalog
        .iter()
        .filter(|resource| {
            resource.physical_state == PhysicalState::Rigid
                && resource.shape.is_valid_physical_shape()
        })
        .collect::<Vec<_>>();
    if structural_resources.is_empty() {
        return Err("catalog contains no valid rigid construction materials".into());
    }

    let contact_records = library.rigid_contact_families().count()
        + library.rigid_point_contact_families().count()
        + library.rigid_vertex_contact_families().count();
    let state_budget = contact_records.saturating_mul(2).clamp(16, 64);
    let mut queue = Vec::new();
    let mut visited = std::collections::BTreeSet::<String>::new();
    for resource in &structural_resources {
        let mut structure = crate::structure::OrganismStructure::new();
        add_unit(&mut structure, resource, (0.0, 0.0), catalog)?;
        let state = ConstructionSearchState {
            structure,
            ledger: EnergyLedger::default(),
            energy: initial_energy,
        };
        let signature = construction_state_signature(&state.structure);
        if visited.insert(signature) {
            queue.push(state);
        }
    }

    let mut explored_states = 0usize;
    let mut coverage_misses = 0usize;
    let mut rejected_placements = 0usize;
    let mut restoration_failures = 0usize;
    let mut overlap_rejections = 0usize;
    let mut pair_overlap_rejections = 0usize;
    let mut edge_overlap_rejections = 0usize;
    let mut no_bond_contact = 0usize;
    let mut evaluation_rejections = 0usize;
    let mut combine_rejections = 0usize;
    let mut energy_rejections = 0usize;
    let mut last_combine_rejection = None;
    let mut queue_budget_drops = 0usize;
    let mut placement_queries_total = 0usize;
    let mut placement_query_budget_exhaustions = 0usize;
    let mut anchors_considered_total = 0usize;
    let mut candidate_states_generated_total = 0usize;
    let mut raw_pair_placements_total = 0usize;
    let mut pair_placements_dropped_by_cap_total = 0usize;
    let mut edge_placements_examined_total = 0usize;
    let mut edge_placements_added_total = 0usize;
    let mut duplicate_placements_total = 0usize;
    let mut placement_attempts_total = 0usize;
    let mut last_overlap_rejection = None;

    while explored_states < state_budget && !queue.is_empty() {
        let best_index = queue
            .iter()
            .enumerate()
            .max_by_key(|(_, state)| construction_state_score(state))
            .map(|(index, _)| index)
            .expect("non-empty construction queue has a best state");
        let state = queue.swap_remove(best_index);
        explored_states += 1;

        let cavity = crate::cavity::analyze_genome_cavity(&state.structure, catalog)
            .ok()
            .flatten();
        if cavity.as_ref().is_some_and(|cavity| cavity.qualifies()) {
            if let Some((
                acquired_resource_placements,
                acquired_resource_shapes,
                fluid_geometry_status,
            )) = valid_construction(&state.structure, catalog, acquisition_candidates, library)
            {
                return Ok(ValidConstruction {
                    structure: state.structure,
                    energy: state.energy,
                    acquired_resource_placements,
                    acquired_resource_shapes,
                    fluid_geometry_status,
                });
            }
        }

        let mut children = Vec::new();
        let mut placement_queries_examined = 0usize;
        let mut candidates_examined = 0usize;
        'anchors: for anchor_index in 0..state.structure.units.len() {
            anchors_considered_total += 1;
            if placement_queries_examined >= 12 {
                placement_query_budget_exhaustions += 1;
                break;
            }
            for resource in &structural_resources {
                if placement_queries_examined >= 12 {
                    placement_query_budget_exhaustions += 1;
                    break 'anchors;
                }
                placement_queries_examined += 1;
                placement_queries_total += 1;
                match try_persisted_bob_attachments(
                    &state.structure,
                    anchor_index,
                    resource,
                    library,
                    catalog,
                    &state.ledger,
                    state.energy,
                ) {
                    Ok(attachments) => {
                        for attachment in attachments {
                            if candidates_examined >= 12 {
                                break 'anchors;
                            }
                            candidates_examined += 1;
                            candidate_states_generated_total += 1;
                            debug_assert!(
                                attachment.new_unit_index < attachment.structure.units.len()
                            );
                            let new_unit_index = attachment.new_unit_index;
                            let mut next = ConstructionSearchState {
                                structure: attachment.structure,
                                ledger: attachment.ledger,
                                energy: attachment.energy,
                            };
                            close_touching_bonds(
                                &mut next.structure,
                                new_unit_index,
                                catalog,
                                &mut next.ledger,
                                &mut next.energy,
                            );
                            let signature = construction_state_signature(&next.structure);
                            if !visited.insert(signature) {
                                continue;
                            }

                            let cavity =
                                crate::cavity::analyze_genome_cavity(&next.structure, catalog)
                                    .ok()
                                    .flatten();
                            if cavity.as_ref().is_some_and(|cavity| cavity.qualifies()) {
                                if let Some((
                                    acquired_resource_placements,
                                    acquired_resource_shapes,
                                    fluid_geometry_status,
                                )) = valid_construction(
                                    &next.structure,
                                    catalog,
                                    acquisition_candidates,
                                    library,
                                ) {
                                    return Ok(ValidConstruction {
                                        structure: next.structure,
                                        energy: next.energy,
                                        acquired_resource_placements,
                                        acquired_resource_shapes,
                                        fluid_geometry_status,
                                    });
                                }
                            }
                            children.push(next);
                        }
                    }
                    Err(PersistedAttachmentFailure::CoverageMiss) => {
                        coverage_misses += 1;
                    }
                    Err(PersistedAttachmentFailure::CandidatesRejected {
                        suggestions,
                        raw_pair_placements,
                        pair_placements_dropped_by_cap,
                        edge_placements_examined,
                        edge_placements_added,
                        duplicate_placements,
                        placement_attempts,
                        last_overlap_rejection: step_last_overlap_rejection,
                        restoration_failures: step_restoration_failures,
                        overlap_rejections: step_overlap_rejections,
                        pair_overlap_rejections: step_pair_overlap_rejections,
                        edge_overlap_rejections: step_edge_overlap_rejections,
                        no_bond_contact: step_no_bond_contact,
                        evaluation_rejections: step_evaluation_rejections,
                        combine_rejections: step_combine_rejections,
                        energy_rejections: step_energy_rejections,
                        last_combine_rejection: step_last_combine_rejection,
                    }) => {
                        raw_pair_placements_total += raw_pair_placements;
                        pair_placements_dropped_by_cap_total += pair_placements_dropped_by_cap;
                        edge_placements_examined_total += edge_placements_examined;
                        edge_placements_added_total += edge_placements_added;
                        duplicate_placements_total += duplicate_placements;
                        placement_attempts_total += placement_attempts;
                        if step_last_overlap_rejection.is_some() {
                            last_overlap_rejection = step_last_overlap_rejection;
                        }
                        rejected_placements += suggestions;
                        restoration_failures += step_restoration_failures;
                        overlap_rejections += step_overlap_rejections;
                        pair_overlap_rejections += step_pair_overlap_rejections;
                        edge_overlap_rejections += step_edge_overlap_rejections;
                        no_bond_contact += step_no_bond_contact;
                        evaluation_rejections += step_evaluation_rejections;
                        combine_rejections += step_combine_rejections;
                        energy_rejections += step_energy_rejections;
                        if step_last_combine_rejection.is_some() {
                            last_combine_rejection = step_last_combine_rejection;
                        }
                    }
                }
            }
        }

        queue.extend(children);
        queue.sort_by_key(|state| std::cmp::Reverse(construction_state_score(state)));
        if queue.len() > state_budget {
            queue_budget_drops += queue.len() - state_budget;
            queue.truncate(state_budget);
        }
    }

    Err(format!(
        "persisted Bob constructor found no qualifying cavity within {explored_states}/{state_budget} states; queued={}, queue_budget_drops={queue_budget_drops}, placement_queries_total={placement_queries_total}, placement_query_budget_exhaustions={placement_query_budget_exhaustions}, anchors_considered_total={anchors_considered_total}, candidate_states_generated_total={candidate_states_generated_total}, raw_pair_placements_total={raw_pair_placements_total}, pair_placements_dropped_by_cap_total={pair_placements_dropped_by_cap_total}, edge_placements_examined_total={edge_placements_examined_total}, edge_placements_added_total={edge_placements_added_total}, duplicate_placements_total={duplicate_placements_total}, placement_attempts_total={placement_attempts_total}, last_overlap_rejection={last_overlap_rejection:?}, coverage_misses={coverage_misses}, rejected_placements={rejected_placements}, restoration_failures={restoration_failures}, overlap_rejections={overlap_rejections}, pair_overlap_rejections={pair_overlap_rejections}, edge_overlap_rejections={edge_overlap_rejections}, no_bond_contact={no_bond_contact}, evaluation_rejections={evaluation_rejections}, combine_rejections={combine_rejections}, energy_rejections={energy_rejections}, last_combine_rejection={last_combine_rejection:?}, rigid_contact_records={contact_records}",
        queue.len()
    ))
}

fn placement_fits_resource(
    resource: &BaseResource,
    region: &crate::interior_geometry::EnclosedRegion,
    catalog: &[BaseResource],
) -> Option<Placement> {
    // Probe the region's sample point and a small deterministic set of
    // nearby points. This is a bounded geometric check, not a raster search;
    // no scaffold-specific layout or target coordinates are assumed.
    let points = [
        region.sample_point,
        (region.sample_point.0 * 0.75, region.sample_point.1 * 0.75),
        (region.sample_point.0 * 0.5, region.sample_point.1 * 0.5),
        (region.sample_point.0 * 1.25, region.sample_point.1 * 1.25),
    ];
    for &(x, y) in &points {
        if !region.contains_point(x, y) {
            continue;
        }
        for rotation_index in 0..4 {
            let placement = Placement {
                x,
                y,
                rotation_radians: rotation_index as f64 * std::f64::consts::FRAC_PI_2,
            };
            let Some(physical) = crate::physical_material::PhysicalMaterial::realized(
                Material::free_base(resource.name.clone(), 1.0),
                vec![placement],
                catalog,
            ) else {
                continue;
            };
            if crate::environment::ActiveMaterialField::physical_is_fully_inside_any_region(
                &physical,
                std::slice::from_ref(region),
                catalog,
            ) {
                return Some(placement);
            }
        }
    }
    None
}

fn available_acquisition_resources<'a>(catalog: &'a [BaseResource]) -> Vec<&'a BaseResource> {
    catalog
        .iter()
        .filter(|resource| resource.name != "Water")
        .filter(|resource| resource.shape.is_valid_physical_shape())
        .collect()
}

fn find_acquisition_set(
    candidates: &[&BaseResource],
    regions: &[crate::interior_geometry::EnclosedRegion],
    catalog: &[BaseResource],
) -> Option<Vec<(String, Placement)>> {
    for first in 0..candidates.len() {
        for second in first + 1..candidates.len() {
            for third in second + 1..candidates.len() {
                let selected = [candidates[first], candidates[second], candidates[third]];
                let mut placements = Vec::with_capacity(3);
                let mut fits = true;
                for resource in selected {
                    let Some(placement) = regions
                        .iter()
                        .find_map(|region| placement_fits_resource(resource, region, catalog))
                    else {
                        fits = false;
                        break;
                    };
                    placements.push((resource.name.clone(), placement));
                }
                if fits {
                    return Some(placements);
                }
            }
        }
    }
    None
}

fn resource_area(resource: &BaseResource) -> Option<f64> {
    match &resource.shape.form {
        Form::Circle { radius } => Some(std::f64::consts::PI * radius * radius),
        Form::Fluid { nominal_area, .. } => Some(*nominal_area),
        _ => None,
    }
}

fn fluid_free_boundary_probe_points(shape: &Shape, placement: Placement) -> Vec<(f64, f64)> {
    let local_points: Vec<(f64, f64)> = match &shape.form {
        Form::Fluid {
            multi_contact_boundary: Some(boundary),
            ..
        } => boundary
            .pieces
            .iter()
            .filter_map(|piece| match piece {
                crate::capillary_geometry::FluidBoundaryPiece::FreeArc(arc) => {
                    let angle = arc.start_angle_radians + arc.sweep_radians * 0.5;
                    Some((
                        arc.center_x + arc.radius * angle.cos(),
                        arc.center_y + arc.radius * angle.sin(),
                    ))
                }
                crate::capillary_geometry::FluidBoundaryPiece::SolidContact { .. } => None,
            })
            .collect(),
        Form::Fluid {
            arc_boundary: Some(arc),
            ..
        } => {
            let angle = arc.start_angle_radians + arc.sweep_radians * 0.5;
            vec![(
                arc.center_x + arc.radius * angle.cos(),
                arc.center_y + arc.radius * angle.sin(),
            )]
        }
        _ => Vec::new(),
    };
    let (sin, cos) = placement.rotation_radians.sin_cos();
    local_points
        .into_iter()
        .map(|(x, y)| {
            (
                placement.x + x * cos - y * sin,
                placement.y + x * sin + y * cos,
            )
        })
        .collect()
}

fn fluid_segment_contains_world_point(shape: &Shape, placement: Placement, x: f64, y: f64) -> bool {
    let dx = x - placement.x;
    let dy = y - placement.y;
    let (sin, cos) = placement.rotation_radians.sin_cos();
    let local_x = dx * cos + dy * sin;
    let local_y = -dx * sin + dy * cos;
    match &shape.form {
        Form::Fluid {
            multi_contact_boundary: Some(boundary),
            ..
        } => boundary.contains_point(local_x, local_y),
        Form::Fluid {
            arc_boundary: Some(arc),
            ..
        } => {
            let start = arc.start_point();
            let end = arc.end_point();
            let chord_side =
                (end.0 - start.0) * (local_y - start.1) - (end.1 - start.1) * (local_x - start.0);
            let midpoint_angle = arc.start_angle_radians + arc.sweep_radians * 0.5;
            let midpoint = (
                arc.center_x + arc.radius * midpoint_angle.cos(),
                arc.center_y + arc.radius * midpoint_angle.sin(),
            );
            let arc_side = (end.0 - start.0) * (midpoint.1 - start.1)
                - (end.1 - start.1) * (midpoint.0 - start.0);
            chord_side * arc_side >= -1e-9
                && (local_x - arc.center_x).hypot(local_y - arc.center_y) <= arc.radius + 1e-9
        }
        _ => false,
    }
}

fn rigid_form_contains_world_point(form: &Form, placement: Placement, x: f64, y: f64) -> bool {
    let dx = x - placement.x;
    let dy = y - placement.y;
    let (sin, cos) = placement.rotation_radians.sin_cos();
    let local_x = dx * cos + dy * sin;
    let local_y = -dx * sin + dy * cos;
    match form {
        Form::Circle { radius } => local_x.hypot(local_y) <= *radius + 1e-9,
        Form::Line { length } => local_y.abs() <= 1e-9 && local_x.abs() <= length * 0.5 + 1e-9,
        _ => {
            let Some(vertices) = form.polygon_vertices() else {
                return false;
            };
            let mut inside = false;
            for index in 0..vertices.len() {
                let (x1, y1) = vertices[index];
                let (x2, y2) = vertices[(index + 1) % vertices.len()];
                let cross = (local_x - x1) * (y2 - y1) - (local_y - y1) * (x2 - x1);
                if cross.abs() <= 1e-9
                    && local_x >= x1.min(x2) - 1e-9
                    && local_x <= x1.max(x2) + 1e-9
                    && local_y >= y1.min(y2) - 1e-9
                    && local_y <= y1.max(y2) + 1e-9
                {
                    return true;
                }
                if (y1 > local_y) != (y2 > local_y)
                    && local_x < (x2 - x1) * (local_y - y1) / (y2 - y1) + x1
                {
                    inside = !inside;
                }
            }
            inside
        }
    }
}

fn rigid_part_world_points(form: &Form, placement: Placement) -> Vec<(f64, f64)> {
    let local = match form {
        Form::Circle { .. } => vec![(0.0, 0.0)],
        Form::Line { length } => vec![(-length * 0.5, 0.0), (length * 0.5, 0.0)],
        _ => form.polygon_vertices().unwrap_or_default(),
    };
    let (sin, cos) = placement.rotation_radians.sin_cos();
    local
        .into_iter()
        .map(|(x, y)| {
            (
                placement.x + x * cos - y * sin,
                placement.y + x * sin + y * cos,
            )
        })
        .collect()
}

/// Query persisted singleton-parent fluid records, realize each candidate exactly,
/// and accept only a pose that passes live region and structure validation.
/// The boolean reports whether Bob had any matching records, so a coverage miss
/// cannot be confused with suggestions that were rejected by live geometry.
fn bob_fluid_acquisition_candidate(
    structure: &crate::structure::OrganismStructure,
    regions: &[crate::interior_geometry::EnclosedRegion],
    water: &BaseResource,
    library: &crate::geometry_reference_library::GeometryLibrary,
    catalog: &[BaseResource],
) -> (Option<(Shape, Placement)>, bool) {
    let Some(water_area) = resource_area(water) else {
        return (None, false);
    };
    let mut had_records = false;
    for (anchor_index, anchor_unit) in structure.units.iter().enumerate() {
        let Some((anchor_name, _)) = anchor_unit.material.parts.first() else {
            continue;
        };
        let Some(anchor_resource) = catalog
            .iter()
            .find(|resource| resource.name == *anchor_name)
        else {
            continue;
        };
        if anchor_resource.physical_state == PhysicalState::Fluid {
            continue;
        }
        let parent = crate::geometry_reference_library::GeometryFormation::single(
            anchor_resource.name.clone(),
        );
        let suggestions: Vec<_> = library
            .fluid_formations_for_parent(&parent.signature, &water.name)
            .collect();
        if !suggestions.is_empty() {
            had_records = true;
        }
        for suggestion in suggestions {
            if suggestion.anchor_constituent != 0
                || (suggestion.area - water_area).abs() > 1e-9 * water_area.max(1.0)
            {
                continue;
            }
            for bulge_left in [true, false] {
                let Some((shape, relative_pose)) =
                    suggestion.realize_against_parent(&parent, catalog, bulge_left)
                else {
                    continue;
                };
                let (sin, cos) = anchor_unit.placement.rotation_radians.sin_cos();
                let placement = Placement {
                    x: anchor_unit.placement.x + relative_pose.x * cos - relative_pose.y * sin,
                    y: anchor_unit.placement.y + relative_pose.x * sin + relative_pose.y * cos,
                    rotation_radians: anchor_unit.placement.rotation_radians
                        + relative_pose.rotation_radians,
                };
                if !shape.is_valid_physical_shape() {
                    continue;
                }

                let mut candidate_catalog = catalog.to_vec();
                let Some(candidate_water) = candidate_catalog
                    .iter_mut()
                    .find(|resource| resource.name == water.name)
                else {
                    continue;
                };
                candidate_water.shape = shape.clone();
                let Some(physical) = crate::physical_material::PhysicalMaterial::realized(
                    Material::free_base(water.name.clone(), 1.0),
                    vec![placement],
                    &candidate_catalog,
                ) else {
                    continue;
                };
                if !crate::environment::ActiveMaterialField::physical_is_fully_inside_any_region(
                    &physical,
                    regions,
                    &candidate_catalog,
                ) {
                    continue;
                }

                let fluid_part = crate::material_geometry::PlacedMaterialPart {
                    part_index: 0,
                    form: shape.form.clone(),
                    placement,
                };
                let mut valid = true;
                for (other_index, other_unit) in structure.units.iter().enumerate() {
                    let Some(other_shape) = other_unit.shape(catalog) else {
                        valid = false;
                        break;
                    };
                    let other_part = crate::material_geometry::PlacedMaterialPart {
                        part_index: other_index,
                        form: other_shape.form.clone(),
                        placement: other_unit.placement,
                    };
                    let rigid_points =
                        rigid_part_world_points(&other_shape.form, other_unit.placement);
                    let free_boundary_probes = fluid_free_boundary_probe_points(&shape, placement);
                    if free_boundary_probes.is_empty() {
                        valid = false;
                        break;
                    }
                    if free_boundary_probes.iter().any(|&(x, y)| {
                        rigid_form_contains_world_point(
                            &other_shape.form,
                            other_unit.placement,
                            x,
                            y,
                        )
                    }) || fluid_segment_contains_world_point(
                        &shape,
                        placement,
                        other_unit.placement.x,
                        other_unit.placement.y,
                    ) || rigid_points
                        .iter()
                        .any(|&(x, y)| fluid_segment_contains_world_point(&shape, placement, x, y))
                    {
                        valid = false;
                        break;
                    }
                    if other_index == anchor_index {
                        if !crate::material_geometry::placed_forms_rigid_contact(
                            &fluid_part,
                            &other_part,
                            crate::material_geometry::GEOMETRY_EPSILON,
                        ) {
                            valid = false;
                            break;
                        }
                    } else if crate::material_geometry::placed_forms_boundary_contact(
                        &fluid_part,
                        &other_part,
                        crate::material_geometry::GEOMETRY_EPSILON,
                    ) {
                        valid = false;
                        break;
                    }
                }
                if valid {
                    return (Some((shape, placement)), true);
                }
            }
        }
    }
    (None, had_records)
}

fn valid_construction(
    structure: &crate::structure::OrganismStructure,
    catalog: &[BaseResource],
    acquisition_candidates: &[&BaseResource],
    bob_library: &crate::geometry_reference_library::GeometryLibrary,
) -> Option<(
    Vec<(String, Placement)>,
    Vec<(String, Shape)>,
    FluidGeometryStatus,
)> {
    let cavity = crate::cavity::analyze_genome_cavity(structure, catalog)
        .ok()
        .flatten()?;
    if !cavity.qualifies() {
        return None;
    }

    let regions =
        crate::interior_geometry::find_accessible_interior_regions(structure, catalog).ok()?;
    if regions.is_empty() {
        return None;
    }

    let mut acquired = find_acquisition_set(acquisition_candidates, &regions, catalog)?;
    let water = catalog.iter().find(|resource| resource.name == "Water")?;
    let (water_candidate, had_bob_records) =
        bob_fluid_acquisition_candidate(structure, &regions, water, bob_library, catalog);
    let (water_shape, water_placement, fluid_geometry_status) =
        match (water_candidate, had_bob_records) {
            (Some((shape, placement)), _) => (
                Some(shape),
                placement,
                FluidGeometryStatus::BobSuggestionAccepted,
            ),
            (None, true) => (
                None,
                regions
                    .iter()
                    .find_map(|region| placement_fits_resource(water, region, catalog))?,
                FluidGeometryStatus::CandidatesRejected,
            ),
            (None, false) => (
                None,
                regions
                    .iter()
                    .find_map(|region| placement_fits_resource(water, region, catalog))?,
                FluidGeometryStatus::CoverageMiss,
            ),
        };
    acquired.push((water.name.clone(), water_placement));
    let acquired_shapes = water_shape
        .map(|shape| vec![(water.name.clone(), shape)])
        .unwrap_or_default();
    Some((acquired, acquired_shapes, fluid_geometry_status))
}

#[cfg(test)]
pub(crate) fn construct_valid(catalog: &[BaseResource]) -> Result<ValidConstruction, String> {
    construct_valid_with_energy(
        catalog,
        crate::genome::initial_genome().reproductive_energy_allocation,
    )
}

pub(crate) fn construct_valid_with_energy(
    catalog: &[BaseResource],
    initial_energy: f64,
) -> Result<ValidConstruction, String> {
    let acquisition_candidates = available_acquisition_resources(catalog);
    if acquisition_candidates.len() < 3 {
        return Err("catalog does not contain three non-water acquisition resources".into());
    }
    if !catalog.iter().any(|resource| resource.name == "Water") {
        return Err("catalog does not contain Water".into());
    }

    let bob_library = crate::geometry_reference_library::open_default_library()
        .map_err(|error| format!("failed to open persistent Bob geometry library: {error}"))?;
    construct_persisted_bob_structure(
        catalog,
        &bob_library,
        &acquisition_candidates,
        initial_energy,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pair_pose_selection_preserves_geometric_diversity() {
        let placements = (0..6)
            .map(|index| Placement {
                x: index as f64 * 0.1,
                y: 0.0,
                rotation_radians: 0.0,
            })
            .chain((0..6).map(|index| Placement {
                x: 10.0 + index as f64 * 0.1,
                y: 0.0,
                rotation_radians: std::f64::consts::PI,
            }))
            .collect::<Vec<_>>();

        let selected = select_diverse_placements(placements.clone(), 6);
        let repeated = select_diverse_placements(placements, 6);

        assert_eq!(selected.len(), 6);
        assert_eq!(selected, repeated);
        let min_x = selected
            .iter()
            .map(|placement| placement.x)
            .fold(f64::INFINITY, f64::min);
        let max_x = selected
            .iter()
            .map(|placement| placement.x)
            .fold(f64::NEG_INFINITY, f64::max);
        assert!(
            max_x - min_x > 10.0,
            "selection should retain candidates from both spatial clusters"
        );
    }

    #[test]
    fn production_constructor_builds_a_bob_formed_cavity_with_supplied_energy() {
        let catalog = crate::resources::default_catalog();
        let supplied_energy = crate::genome::initial_genome().reproductive_energy_allocation;
        let construction = construct_valid_with_energy(&catalog, supplied_energy)
            .expect("persisted-Bob constructor should realize the initial organism");

        let cavity = crate::cavity::analyze_genome_cavity(&construction.structure, &catalog)
            .expect("cavity analysis should succeed")
            .expect("constructed organism must contain a genome cavity");
        assert!(cavity.qualifies());
        assert!(!construction.structure.bonds.is_empty());
        assert!(construction.energy.is_finite());
        assert!(construction.energy >= 0.0);
        assert_eq!(
            construction.energy, supplied_energy,
            "free-form construction must not debit real starting energy"
        );

        let acquired_names = construction
            .acquired_resource_placements
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        assert!(acquired_names.contains("Water"));
        assert!(
            acquired_names
                .iter()
                .filter(|name| **name != "Water")
                .count()
                >= 3,
            "the initial organism must acquire at least three non-water resource types"
        );
        assert!(construction
            .acquired_resource_placements
            .iter()
            .any(|(name, _)| name == "Water"));
    }

    #[test]
    fn persisted_bob_attachment_reports_missing_coverage_without_fallback() {
        let catalog = crate::resources::default_catalog();
        let carbon = catalog
            .iter()
            .find(|resource| resource.name == "Carbon")
            .unwrap();
        let root = std::env::temp_dir().join(format!(
            "evosim-persisted-attachment-empty-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let library = crate::geometry_reference_library::GeometryLibrary::open(&root, &catalog)
            .expect("isolated Bob library should open");
        let mut structure = crate::structure::OrganismStructure::new();
        add_unit(&mut structure, carbon, (0.0, 0.0), &catalog).expect("anchor should realize");
        let result = try_persisted_bob_attachments(
            &structure,
            0,
            carbon,
            &library,
            &catalog,
            &EnergyLedger::default(),
            1_000_000.0,
        );
        match result {
            Err(PersistedAttachmentFailure::CoverageMiss) => {}
            Err(other) => panic!("unexpected persisted attachment failure: {other:?}"),
            Ok(_) => panic!("empty Bob library must not produce an attachment"),
        }
        assert_eq!(
            structure.units.len(),
            1,
            "coverage miss must not mutate the live structure"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn persisted_bob_attachment_commits_only_after_live_combine_succeeds() {
        let catalog = crate::resources::default_catalog();
        let carbon = catalog
            .iter()
            .find(|resource| resource.name == "Carbon")
            .unwrap();
        let root = std::env::temp_dir().join(format!(
            "evosim-persisted-attachment-recorded-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let mut library = crate::geometry_reference_library::GeometryLibrary::open(&root, &catalog)
            .expect("isolated Bob library should open");
        let parent = crate::geometry_reference_library::GeometryFormation::single("Carbon");
        library
            .insert(parent.clone(), &catalog)
            .expect("Carbon singleton should persist");
        let families = crate::geometry_reference_library::generate_rigid_contact_families(
            &parent, carbon, &catalog,
        );
        assert!(
            !families.is_empty(),
            "fixture should generate Carbon contact families"
        );
        library
            .insert_rigid_contact_families(families)
            .expect("generated contact families should persist");

        let mut structure = crate::structure::OrganismStructure::new();
        add_unit(&mut structure, carbon, (0.0, 0.0), &catalog).expect("anchor should realize");
        let initial_energy = 1_000_000.0;
        let attachments = try_persisted_bob_attachments(
            &structure,
            0,
            carbon,
            &library,
            &catalog,
            &EnergyLedger::default(),
            initial_energy,
        )
        .expect(
            "persisted Bob proposal should pass live restoration, overlap, contact, and COMBINE",
        );
        let attached = attachments
            .first()
            .expect("at least one attachment should succeed");
        assert_eq!(attached.structure.units.len(), 2);
        assert_eq!(attached.new_unit_index, 1);
        assert!(!attached.structure.bonds.is_empty());
        assert_eq!(
            attached.energy, initial_energy,
            "free-form construction must not debit the organism's real energy"
        );
        assert_eq!(
            attached.ledger.total_heat_dissipated, 0.0,
            "free-form construction must not settle a real COMBINE energy transaction"
        );
        assert!(attached.ledger.total_heat_dissipated.is_finite());
        assert_eq!(
            structure.units.len(),
            1,
            "trial growth must not mutate the caller's structure"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn persisted_bob_fluid_suggestion_reaches_live_constructor_validation() {
        let catalog = crate::resources::default_catalog();
        let root =
            std::env::temp_dir().join(format!("evosim-constructor-fluid-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let mut library = crate::geometry_reference_library::GeometryLibrary::open(&root, &catalog)
            .expect("isolated Bob library should open");
        let parent = crate::geometry_reference_library::GeometryFormation::single("Carbon");
        library
            .insert(parent.clone(), &catalog)
            .expect("Carbon singleton should persist");

        let water = catalog
            .iter()
            .find(|resource| resource.name == "Water")
            .expect("Water exists in catalog");
        let (contacts, diagnostics) =
            crate::geometry_reference_library::generate_water_contact_families_with_diagnostics(
                &parent, water, &catalog,
            );
        assert_eq!(diagnostics.capillary_solver_failures, 0);
        assert!(!contacts.is_empty());
        let boundaries =
            crate::geometry_reference_library::generate_fluid_boundary_families_from_contact_families(
                water, &contacts,
            );
        let formations =
            crate::geometry_reference_library::fluid_formations_from_boundaries(&boundaries);
        assert!(!formations.is_empty());
        assert_eq!(
            library
                .insert_fluid_formations(formations.clone(), &catalog)
                .unwrap(),
            formations.len()
        );

        let carbon = catalog
            .iter()
            .find(|resource| resource.name == "Carbon")
            .unwrap();
        let mut structure = crate::structure::OrganismStructure::new();
        add_unit(&mut structure, carbon, (0.0, 0.0), &catalog)
            .expect("Carbon anchor should realize");
        let region = crate::interior_geometry::EnclosedRegion {
            area: 400.0,
            boundary_units: vec![],
            sample_point: (0.0, 0.0),
            boundary: vec![(-10.0, -10.0), (10.0, -10.0), (10.0, 10.0), (-10.0, 10.0)],
        };
        let (candidate, had_records) = bob_fluid_acquisition_candidate(
            &structure,
            std::slice::from_ref(&region),
            water,
            &library,
            &catalog,
        );
        assert!(
            had_records,
            "construction must query the persisted Bob fluid records"
        );
        let (shape, placement) =
            candidate.expect("at least one exact fluid suggestion should pass live validation");
        assert!(shape.is_valid());
        let expected_area = match &water.shape.form {
            Form::Circle { radius } => std::f64::consts::PI * radius * radius,
            Form::Fluid { nominal_area, .. } => *nominal_area,
            _ => panic!("Water must have a fluid-compatible default shape"),
        };
        assert!(matches!(
            &shape.form,
            Form::Fluid {
                nominal_area,
                boundary: None,
                arc_boundary: Some(_), ..} if (*nominal_area - expected_area).abs() < 1e-9
        ));
        assert!(placement.x.is_finite() && placement.y.is_finite());
        let _ = std::fs::remove_dir_all(root);
    }
}
