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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ConstructionQualificationFailure {
    CavityAnalysisError,
    NoCavity,
    CavityDoesNotQualify,
    InteriorAnalysisError,
    NoAccessibleInterior,
}

struct QualifiedCavity {
    acquired_resource_placements: Vec<(String, Placement)>,
    acquired_resource_shapes: Vec<(String, Shape)>,
    fluid_geometry_status: FluidGeometryStatus,
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

fn placement_signature(placement: Placement) -> String {
    format!(
        "{:.3},{:.3},{:.4}",
        placement.x, placement.y, placement.rotation_radians
    )
}

/// Rank suggestions by clearance from the nearest occupied non-anchor unit.
/// This ranks candidates only; live geometry and bond validation stay authoritative.
fn placement_frontier_score(
    structure: &crate::structure::OrganismStructure,
    anchor_index: usize,
    placement: Placement,
) -> f64 {
    structure
        .units
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != anchor_index)
        .map(|(_, unit)| (placement.x - unit.placement.x).hypot(placement.y - unit.placement.y))
        .fold(f64::INFINITY, f64::min)
}

fn rank_candidate_placements(
    structure: &crate::structure::OrganismStructure,
    anchor_index: usize,
    mut placements: Vec<Placement>,
    pair_placement_signatures: &std::collections::BTreeSet<String>,
) -> Vec<Placement> {
    placements.sort_by(|a, b| {
        placement_frontier_score(structure, anchor_index, *b)
            .total_cmp(&placement_frontier_score(structure, anchor_index, *a))
            .then_with(|| {
                let a_is_pair = pair_placement_signatures.contains(&placement_signature(*a));
                let b_is_pair = pair_placement_signatures.contains(&placement_signature(*b));
                b_is_pair.cmp(&a_is_pair)
            })
            .then_with(|| placement_signature(*a).cmp(&placement_signature(*b)))
    });
    placements
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
    for formation in library.bonded_pair_formations(anchor_material, &candidate_resource.name) {
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

/// Return the persisted two-piece formation that reproduces a rejected world-space pose.
/// This is diagnostic-only: it does not influence candidate ordering or acceptance.
fn persisted_pair_pose_provenance(
    anchor_material: &str,
    anchor_placement: Placement,
    candidate_resource: &BaseResource,
    target_placement: Placement,
    library: &crate::geometry_reference_library::GeometryLibrary,
) -> Option<String> {
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
            let replayed = Placement {
                x: anchor_placement.x + dx * cos - dy * sin,
                y: anchor_placement.y + dx * sin + dy * cos,
                rotation_radians: stored_candidate.placement.rotation_radians + rotation_delta,
            };
            if placement_signature(replayed) == placement_signature(target_placement) {
                return Some(format!(
                    "stored_anchor_index={anchor_index}; stored_anchor=({}, {}, {}); stored_candidate_index={candidate_index}; stored_candidate=({}, {}, {}); rotation_delta={rotation_delta:.6}; replayed=({}, {}, {})",
                    stored_anchor.placement.x,
                    stored_anchor.placement.y,
                    stored_anchor.placement.rotation_radians,
                    stored_candidate.placement.x,
                    stored_candidate.placement.y,
                    stored_candidate.placement.rotation_radians,
                    replayed.x,
                    replayed.y,
                    replayed.rotation_radians,
                ));
            }
        }
    }
    None
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
    // Keep every distinct persisted pair proposal; do not discard lower-ranked
    // Bob knowledge before the authoritative live checks.
    let pair_placements_dropped_by_cap = 0usize;
    let pair_placement_signatures = candidate_placements
        .iter()
        .copied()
        .map(placement_signature)
        .collect::<std::collections::BTreeSet<_>>();
    let mut seen_placements = pair_placement_signatures.clone();
    let mut edge_placements_examined = 0usize;
    let mut edge_placements_added = 0usize;
    let mut duplicate_placements = 0usize;
    for suggestion in library.suggest_persisted_rigid_edge_placements(
        anchor_material,
        anchor.placement,
        resource,
        catalog,
    ) {
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
    // Prefer suggestions with more clearance from the existing structure,
    // then exact persisted formation replays over edge-family seeds.
    candidate_placements = rank_candidate_placements(
        structure,
        anchor_index,
        candidate_placements,
        &pair_placement_signatures,
    );
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
    for suggestion in &candidate_placements {
        // Validate suggestions in rank order. Four successful branches provide
        // a bounded frontier; rejected candidates never consume that branch cap.
        if successful.len() >= 4 {
            break;
        }
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
            let source = if pair_placement_signatures.contains(&placement_signature(*suggestion)) {
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
            last_overlap_rejection.get_or_insert_with(|| {
                // Only the first overlap is retained as a diagnostic, so do not
                // rescan Bob's formations for provenance on subsequent rejections.
                let pair_provenance = if source == "pair-formation" {
                    persisted_pair_pose_provenance(
                        anchor_material,
                        anchor.placement,
                        resource,
                        candidate_pose,
                        library,
                    )
                } else {
                    None
                };
                format!(
                    "source={source}; anchor={anchor_index}:{anchor_material}@({:.4},{:.4},{:.4}); candidate={}:{}@({:.4},{:.4},{:.4}); overlaps={existing_unit_index}:{existing_material}@({:.4},{:.4},{:.4}); pair_provenance={pair_provenance:?}",
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
                )
            });
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
    let mut beam_dropped_score_counts =
        std::collections::BTreeMap::<(usize, usize, usize), usize>::new();
    let mut beam_truncation_tie_drops = 0usize;
    let mut last_beam_cutoff_score = None;
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
    let mut cavity_analysis_errors = 0usize;
    let mut states_without_cavity = 0usize;
    let mut nonqualifying_cavities = 0usize;
    let mut qualifying_cavity_states = 0usize;
    let mut inaccessible_interior_states = 0usize;
    let mut interior_analysis_errors = 0usize;

    while explored_states < state_budget && !queue.is_empty() {
        let best_index = queue
            .iter()
            .enumerate()
            .max_by_key(|(_, state)| construction_state_score(state))
            .map(|(index, _)| index)
            .expect("non-empty construction queue has a best state");
        let state = queue.swap_remove(best_index);
        explored_states += 1;

        // Every queued child is qualified exactly once before entering the queue.
        // Avoid repeating cavity and interior analysis when the state is later popped.
        let mut children = Vec::new();
        'anchors: for anchor_index in 0..state.structure.units.len() {
            anchors_considered_total += 1;
            for resource in &structural_resources {
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

                            match valid_construction(
                                &next.structure,
                                catalog,
                                acquisition_candidates,
                                library,
                            ) {
                                Ok(qualification) => {
                                    return Ok(ValidConstruction {
                                        structure: next.structure,
                                        energy: next.energy,
                                        acquired_resource_placements: qualification
                                            .acquired_resource_placements,
                                        acquired_resource_shapes: qualification
                                            .acquired_resource_shapes,
                                        fluid_geometry_status: qualification.fluid_geometry_status,
                                    });
                                }
                                Err(ConstructionQualificationFailure::CavityAnalysisError) => {
                                    cavity_analysis_errors += 1;
                                }
                                Err(ConstructionQualificationFailure::NoCavity) => {
                                    states_without_cavity += 1;
                                }
                                Err(ConstructionQualificationFailure::CavityDoesNotQualify) => {
                                    nonqualifying_cavities += 1;
                                }
                                Err(ConstructionQualificationFailure::InteriorAnalysisError) => {
                                    qualifying_cavity_states += 1;
                                    interior_analysis_errors += 1;
                                }
                                Err(ConstructionQualificationFailure::NoAccessibleInterior) => {
                                    qualifying_cavity_states += 1;
                                    inaccessible_interior_states += 1;
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
            let cutoff_score = construction_state_score(&queue[state_budget - 1]);
            last_beam_cutoff_score = Some(cutoff_score);
            for dropped in queue.iter().skip(state_budget) {
                let score = construction_state_score(dropped);
                *beam_dropped_score_counts.entry(score).or_default() += 1;
                if score == cutoff_score {
                    beam_truncation_tie_drops += 1;
                }
            }
            queue_budget_drops += queue.len() - state_budget;
            queue.truncate(state_budget);
        }
    }

    let stop_reason = if !queue.is_empty() && explored_states >= state_budget {
        "search budget exhausted before the queue was empty"
    } else if queue_budget_drops > 0 {
        "search ended with states discarded by the explicit beam budget"
    } else {
        "all explored states exhausted without a qualifying cavity"
    };
    Err(format!(
        "persisted Bob constructor: {stop_reason}; explored {explored_states}/{state_budget} states; queued={}, queue_budget_drops={queue_budget_drops}, last_beam_cutoff_score={last_beam_cutoff_score:?}, beam_dropped_score_counts={beam_dropped_score_counts:?}, beam_truncation_tie_drops={beam_truncation_tie_drops}, placement_queries_total={placement_queries_total}, placement_query_budget_exhaustions={placement_query_budget_exhaustions}, anchors_considered_total={anchors_considered_total}, candidate_states_generated_total={candidate_states_generated_total}, raw_pair_placements_total={raw_pair_placements_total}, pair_placements_dropped_by_cap_total={pair_placements_dropped_by_cap_total}, edge_placements_examined_total={edge_placements_examined_total}, edge_placements_added_total={edge_placements_added_total}, duplicate_placements_total={duplicate_placements_total}, placement_attempts_total={placement_attempts_total}, last_overlap_rejection={last_overlap_rejection:?}, coverage_misses={coverage_misses}, rejected_placements={rejected_placements}, restoration_failures={restoration_failures}, overlap_rejections={overlap_rejections}, pair_overlap_rejections={pair_overlap_rejections}, edge_overlap_rejections={edge_overlap_rejections}, no_bond_contact={no_bond_contact}, evaluation_rejections={evaluation_rejections}, combine_rejections={combine_rejections}, energy_rejections={energy_rejections}, last_combine_rejection={last_combine_rejection:?}, cavity_analysis_errors={cavity_analysis_errors}, states_without_cavity={states_without_cavity}, nonqualifying_cavities={nonqualifying_cavities}, qualifying_cavity_states={qualifying_cavity_states}, accessible_interior_states_on_failed_search=0, inaccessible_interior_states={inaccessible_interior_states}, interior_analysis_errors={interior_analysis_errors}, rigid_contact_records={contact_records}",
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

fn find_acquisition_resources(
    candidates: &[&BaseResource],
    regions: &[crate::interior_geometry::EnclosedRegion],
    catalog: &[BaseResource],
) -> Vec<(String, Placement)> {
    candidates
        .iter()
        .filter_map(|resource| {
            regions
                .iter()
                .find_map(|region| placement_fits_resource(resource, region, catalog))
                .map(|placement| (resource.name.clone(), placement))
        })
        .collect()
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
) -> Result<QualifiedCavity, ConstructionQualificationFailure> {
    let cavity = crate::cavity::analyze_genome_cavity(structure, catalog)
        .map_err(|_| ConstructionQualificationFailure::CavityAnalysisError)?
        .ok_or(ConstructionQualificationFailure::NoCavity)?;
    if !cavity.qualifies() {
        return Err(ConstructionQualificationFailure::CavityDoesNotQualify);
    }

    let regions = crate::interior_geometry::find_accessible_interior_regions(structure, catalog)
        .map_err(|_| ConstructionQualificationFailure::InteriorAnalysisError)?;
    if regions.is_empty() {
        return Err(ConstructionQualificationFailure::NoAccessibleInterior);
    }

    // Starting inventory is opportunistic, not a viability recipe. The
    // organism is valid because its realized structure has a qualifying cavity
    // and accessible processing space; missing resource types must not reject it.
    let mut acquired = find_acquisition_resources(acquisition_candidates, &regions, catalog);
    let mut acquired_shapes = Vec::new();
    let mut fluid_geometry_status = FluidGeometryStatus::CoverageMiss;

    if let Some(water) = catalog.iter().find(|resource| resource.name == "Water") {
        let (water_candidate, had_bob_records) =
            bob_fluid_acquisition_candidate(structure, &regions, water, bob_library, catalog);
        match water_candidate {
            Some((shape, placement)) => {
                acquired.push((water.name.clone(), placement));
                acquired_shapes.push((water.name.clone(), shape));
                fluid_geometry_status = FluidGeometryStatus::BobSuggestionAccepted;
            }
            None => {
                if let Some(placement) = regions
                    .iter()
                    .find_map(|region| placement_fits_resource(water, region, catalog))
                {
                    acquired.push((water.name.clone(), placement));
                }
                if had_bob_records {
                    fluid_geometry_status = FluidGeometryStatus::CandidatesRejected;
                }
            }
        }
    }

    Ok(QualifiedCavity {
        acquired_resource_placements: acquired,
        acquired_resource_shapes: acquired_shapes,
        fluid_geometry_status,
    })
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

    let bob_library = crate::geometry_reference_library::open_default_library()
        .map_err(|error| format!("failed to open persistent Bob geometry library: {error}"))?;
    construct_persisted_bob_structure(
        catalog,
        &bob_library,
        &acquisition_candidates,
        initial_energy,
    )
}

/// Build a realized, physically bonded cavity fixture without invoking the
/// production constructor. Tests of cavity, viability rejection, and boundary
/// behavior must not fail merely because startup construction is currently
/// unable to find a viable organism.
#[cfg(test)]
pub(crate) fn sealed_nitrogen_ring_fixture(
    catalog: &[BaseResource],
) -> Result<crate::structure::OrganismStructure, String> {
    let nitrogen = catalog
        .iter()
        .find(|resource| resource.name == "Nitrogen")
        .ok_or_else(|| "catalog has no Nitrogen resource".to_string())?;
    let mut structure = crate::structure::OrganismStructure::new();
    let mut grid = [[None; 5]; 5];

    for y in 0..5 {
        for x in 0..5 {
            if x != 0 && x != 4 && y != 0 && y != 4 {
                continue;
            }
            let index = add_unit(&mut structure, nitrogen, (x as f64 - 2.0, y as f64 - 2.0), catalog)?;
            grid[y][x] = Some(index);
        }
    }

    let mut ledger = EnergyLedger::default();
    let mut energy = 1_000_000.0;
    for y in 0..5 {
        for x in 0..5 {
            let Some(a) = grid[y][x] else { continue };
            for (nx, ny) in [(x + 1, y), (x, y + 1)] {
                if nx >= 5 || ny >= 5 {
                    continue;
                }
                let Some(b) = grid[ny][nx] else { continue };
                bond_units(&mut structure, a, b, catalog, &mut ledger, &mut energy)?;
            }
        }
    }
    Ok(structure)
}

/// Add one bonded external constituent to the sealed ring fixture. This
/// lets bond-selection tests distinguish genome-boundary bonds from ordinary
/// structural bonds without depending on production startup construction.
#[cfg(test)]
pub(crate) fn sealed_nitrogen_ring_with_external_unit_fixture(
    catalog: &[BaseResource],
) -> Result<crate::structure::OrganismStructure, String> {
    let mut structure = sealed_nitrogen_ring_fixture(catalog)?;
    let nitrogen = catalog
        .iter()
        .find(|resource| resource.name == "Nitrogen")
        .ok_or_else(|| "catalog has no Nitrogen resource".to_string())?;
    let anchor = structure
        .units
        .iter()
        .position(|unit| {
            (unit.placement.x - 2.0).abs() < 1e-9 && unit.placement.y.abs() < 1e-9
        })
        .ok_or_else(|| "test ring has no right-side anchor".to_string())?;
    let external = add_unit(&mut structure, nitrogen, (3.0, 0.0), catalog)?;
    let mut ledger = EnergyLedger::default();
    let mut energy = 1_000_000.0;
    bond_units(
        &mut structure,
        anchor,
        external,
        catalog,
        &mut ledger,
        &mut energy,
    )?;
    Ok(structure)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bob_candidate_ranking_prefers_clear_frontier_then_exact_pair_replay() {
        let catalog = crate::resources::default_catalog();
        let resource = |name: &str| {
            catalog
                .iter()
                .find(|resource| resource.name == name)
                .unwrap_or_else(|| panic!("{name} exists in catalog"))
        };
        let mut structure = crate::structure::OrganismStructure::new();
        let anchor_index = add_unit(&mut structure, resource("Hydrogen"), (0.0, 0.0), &catalog)
            .expect("anchor should realize");
        add_unit(&mut structure, resource("Methane"), (5.0, 0.0), &catalog)
            .expect("blocker should realize");

        let farther = Placement {
            x: 10.0,
            y: 0.0,
            rotation_radians: 0.0,
        };
        let nearer = Placement {
            x: 1.0,
            y: 0.0,
            rotation_radians: 0.0,
        };
        let ranked = rank_candidate_placements(
            &structure,
            anchor_index,
            vec![nearer, farther],
            &std::collections::BTreeSet::new(),
        );
        assert_eq!(ranked[0], farther, "greater clearance should rank first");

        let pair_replay = Placement {
            x: 10.0,
            y: 0.0,
            rotation_radians: std::f64::consts::PI,
        };
        let edge_seed = Placement {
            x: 10.0,
            y: 0.0,
            rotation_radians: 0.0,
        };
        let pair_signatures = [placement_signature(pair_replay)].into_iter().collect();
        let ranked = rank_candidate_placements(
            &structure,
            anchor_index,
            vec![edge_seed, pair_replay],
            &pair_signatures,
        );
        assert_eq!(ranked[0], pair_replay, "exact pair replay wins a score tie");
    }

    #[test]
    fn rejected_hydrogen_phosphorus_pose_is_valid_as_a_pair_but_hits_existing_methane() {
        let catalog = crate::resources::default_catalog();
        let resource = |name: &str| {
            catalog
                .iter()
                .find(|resource| resource.name == name)
                .unwrap_or_else(|| panic!("{name} exists in catalog"))
        };
        let hydrogen = resource("Hydrogen");
        let phosphorus = resource("Phosphorus");
        let methane = resource("Methane");
        let anchor_pose = Placement {
            x: 1.25,
            y: -0.2665,
            rotation_radians: 0.0,
        };
        let phosphorus_pose = Placement {
            x: 2.5,
            y: -0.433013,
            rotation_radians: -std::f64::consts::PI,
        };
        let methane_pose = Placement {
            x: 2.0387,
            y: -0.7165,
            rotation_radians: -2.0944,
        };

        let mut isolated_pair = crate::structure::OrganismStructure::new();
        let anchor_index = add_unit(
            &mut isolated_pair,
            hydrogen,
            (anchor_pose.x, anchor_pose.y),
            &catalog,
        )
        .expect("Hydrogen anchor should realize");
        isolated_pair.units[anchor_index].placement = anchor_pose;
        let phosphorus_index = add_unit(
            &mut isolated_pair,
            phosphorus,
            (phosphorus_pose.x, phosphorus_pose.y),
            &catalog,
        )
        .expect("Phosphorus candidate should realize");
        isolated_pair.units[phosphorus_index].placement = phosphorus_pose;
        let pair_overlaps = crate::construction_runtime::overlapping_unit_indices(
            &isolated_pair,
            &isolated_pair.units[phosphorus_index],
            &[phosphorus_index],
            &catalog,
        );
        assert!(
            pair_overlaps.is_empty(),
            "the reported Hydrogen-Phosphorus pair should be non-penetrating in isolation; overlaps={pair_overlaps:?}"
        );

        let mut methane_collision = crate::structure::OrganismStructure::new();
        let methane_index = add_unit(
            &mut methane_collision,
            methane,
            (methane_pose.x, methane_pose.y),
            &catalog,
        )
        .expect("Methane neighbor should realize");
        methane_collision.units[methane_index].placement = methane_pose;
        let phosphorus_index = add_unit(
            &mut methane_collision,
            phosphorus,
            (phosphorus_pose.x, phosphorus_pose.y),
            &catalog,
        )
        .expect("Phosphorus candidate should realize");
        methane_collision.units[phosphorus_index].placement = phosphorus_pose;
        let methane_overlaps = crate::construction_runtime::overlapping_unit_indices(
            &methane_collision,
            &methane_collision.units[phosphorus_index],
            &[phosphorus_index],
            &catalog,
        );
        assert_eq!(
            methane_overlaps,
            vec![methane_index],
            "the same Phosphorus pose should collide with the reported existing Methane"
        );
    }

    #[test]
    fn persisted_pair_replay_preserves_relative_pose_at_a_rotated_live_anchor() {
        let catalog = crate::resources::default_catalog();
        let carbon = catalog
            .iter()
            .find(|resource| resource.name == "Carbon")
            .expect("Carbon exists in catalog");
        let nitrogen = catalog
            .iter()
            .find(|resource| resource.name == "Nitrogen")
            .expect("Nitrogen exists in catalog");
        let root = std::env::temp_dir().join(format!(
            "evosim-pair-replay-transform-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let mut library = crate::geometry_reference_library::GeometryLibrary::open(&root, &catalog)
            .expect("isolated Bob library should open");

        let parent = crate::geometry_reference_library::GeometryFormation::single("Carbon");
        let pair = crate::geometry_reference_library::expand_formation_candidates(
            &parent, nitrogen, &catalog,
        )
        .into_iter()
        .find(|formation| {
            formation.constituents.len() == 2
                && formation
                    .constituents
                    .iter()
                    .any(|part| part.resource == "Carbon")
                && formation
                    .constituents
                    .iter()
                    .any(|part| part.resource == "Nitrogen")
                && formation.bonds.iter().any(|bond| {
                    (bond.constituent_a == 0 && bond.constituent_b == 1)
                        || (bond.constituent_a == 1 && bond.constituent_b == 0)
                })
        })
        .expect("Bob geometry should provide a valid Carbon-Nitrogen pair");
        library
            .insert(pair, &catalog)
            .expect("pair formation should persist");
        drop(library);
        let library = crate::geometry_reference_library::GeometryLibrary::open(&root, &catalog)
            .expect("persisted Bob library should reopen before replay");

        let stored = library
            .formations()
            .find(|formation| {
                formation.constituents.len() == 2
                    && formation
                        .constituents
                        .iter()
                        .any(|part| part.resource == "Carbon")
                    && formation
                        .constituents
                        .iter()
                        .any(|part| part.resource == "Nitrogen")
                    && formation.bonds.iter().any(|bond| {
                        (bond.constituent_a == 0 && bond.constituent_b == 1)
                            || (bond.constituent_a == 1 && bond.constituent_b == 0)
                    })
            })
            .expect("persisted pair should be readable")
            .clone();
        let stored_anchor_index = stored
            .constituents
            .iter()
            .position(|part| part.resource == "Carbon")
            .expect("stored pair contains Carbon");
        let stored_candidate_index = stored
            .constituents
            .iter()
            .position(|part| part.resource == "Nitrogen")
            .expect("stored pair contains Nitrogen");
        let stored_anchor = stored.constituents[stored_anchor_index].placement;
        let stored_candidate = stored.constituents[stored_candidate_index].placement;

        let live_anchor = Placement {
            x: 4.25,
            y: -3.5,
            rotation_radians: 0.73,
        };
        let rotation_delta = live_anchor.rotation_radians - stored_anchor.rotation_radians;
        let (sin, cos) = rotation_delta.sin_cos();
        let dx = stored_candidate.x - stored_anchor.x;
        let dy = stored_candidate.y - stored_anchor.y;
        let expected = Placement {
            x: live_anchor.x + dx * cos - dy * sin,
            y: live_anchor.y + dx * sin + dy * cos,
            rotation_radians: stored_candidate.rotation_radians + rotation_delta,
        };

        let suggestions =
            persisted_pair_formation_placements("Carbon", live_anchor, nitrogen, &library);
        assert!(
            suggestions.iter().any(|pose| {
                (pose.x - expected.x).hypot(pose.y - expected.y) < 1e-9
                    && (pose.rotation_radians - expected.rotation_radians).abs() < 1e-9
            }),
            "Bob replay must preserve the saved candidate transform relative to the anchor"
        );

        let mut structure = crate::structure::OrganismStructure::new();
        add_unit(
            &mut structure,
            carbon,
            (live_anchor.x, live_anchor.y),
            &catalog,
        )
        .expect("live anchor should realize");
        structure.units[0].placement.rotation_radians = live_anchor.rotation_radians;
        let instance = crate::physical_material::PhysicalMaterial::realized(
            Material::free_base(nitrogen.name.clone(), 1.0),
            vec![Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .expect("candidate material should realize");
        let indices = crate::material_restoration::restore_material(
            &mut structure,
            &instance,
            expected,
            &catalog,
        )
        .expect("persisted candidate pose should restore");
        let candidate_index = *indices.first().expect("candidate restores one unit");
        let restored = structure.units[candidate_index].placement;
        assert!(
            (restored.x - expected.x).hypot(restored.y - expected.y) < 1e-9
                && (restored.rotation_radians - expected.rotation_radians).abs() < 1e-9,
            "live restoration must preserve the replayed world-space pose"
        );
        let overlaps = crate::construction_runtime::overlapping_unit_indices(
            &structure,
            &structure.units[candidate_index],
            &indices,
            &catalog,
        );
        assert!(
            overlaps.is_empty(),
            "replaying a valid persisted pair must not introduce overlap: {overlaps:?}"
        );

        // Pair validity is tested independently from growth-context validity.
        // The exact persisted replay must make real contact and form a bond,
        // without spending or inventing construction energy.
        let initial_energy = 10_000.0;
        let mut ledger = EnergyLedger::default();
        let mut energy = initial_energy;
        bond_units(
            &mut structure,
            0,
            candidate_index,
            &catalog,
            &mut ledger,
            &mut energy,
        )
        .expect("persisted pair replay must pass live contact and COMBINE");
        assert!(
            !structure.bonds.is_empty(),
            "the restored pair must form an actual live bond"
        );
        assert_eq!(
            energy, initial_energy,
            "free-form construction energy must remain unchanged"
        );
        assert_eq!(ledger.total_heat_dissipated, 0.0);
        assert_eq!(ledger.total_potential_energy_released, 0.0);
        assert_eq!(ledger.total_usable_energy_gained, 0.0);
        assert_eq!(ledger.total_usable_energy_held, 0.0);

        // Replay the same candidate pose against a valid existing neighbor.
        // Move a second Nitrogen just 0.01 units outward from the candidate:
        // it remains within live contact tolerance of Carbon, but occupies the
        // same local growth space as the candidate being replayed.
        let mut growth_context = crate::structure::OrganismStructure::new();
        add_unit(
            &mut growth_context,
            carbon,
            (live_anchor.x, live_anchor.y),
            &catalog,
        )
        .expect("growth-context anchor should realize");
        growth_context.units[0].placement = live_anchor;
        let radial_x = expected.x - live_anchor.x;
        let radial_y = expected.y - live_anchor.y;
        let radial_length = radial_x.hypot(radial_y);
        assert!(
            radial_length > 0.0,
            "pair replay has a nonzero anchor offset"
        );
        let neighbor_pose = Placement {
            x: expected.x + radial_x / radial_length * 0.01,
            y: expected.y + radial_y / radial_length * 0.01,
            rotation_radians: expected.rotation_radians,
        };
        let neighbor_indices = crate::material_restoration::restore_material(
            &mut growth_context,
            &instance,
            neighbor_pose,
            &catalog,
        )
        .expect("existing neighboring Nitrogen should restore");
        let neighbor_index = *neighbor_indices
            .first()
            .expect("neighbor restores one unit");
        assert!(
            crate::construction_runtime::overlapping_unit_indices(
                &growth_context,
                &growth_context.units[neighbor_index],
                &neighbor_indices,
                &catalog,
            )
            .is_empty(),
            "existing neighbor must not penetrate the anchor before candidate replay"
        );
        let mut context_ledger = EnergyLedger::default();
        let mut context_energy = initial_energy;
        bond_units(
            &mut growth_context,
            0,
            neighbor_index,
            &catalog,
            &mut context_ledger,
            &mut context_energy,
        )
        .expect("existing neighboring unit must have a real live bond to the anchor");
        assert!(
            !growth_context.bonds.is_empty(),
            "growth context must contain the existing anchor-neighbor bond"
        );

        let replay_indices = crate::material_restoration::restore_material(
            &mut growth_context,
            &instance,
            expected,
            &catalog,
        )
        .expect("trial replay should restore before authoritative overlap rejection");
        let replay_index = *replay_indices.first().expect("replay restores one unit");
        let first_rejection = crate::construction_runtime::overlapping_unit_indices(
            &growth_context,
            &growth_context.units[replay_index],
            &replay_indices,
            &catalog,
        );
        assert_eq!(
            first_rejection,
            vec![neighbor_index],
            "same pair-valid pose must be rejected by the existing neighboring unit; anchor={:?}, replay={:?}, neighbor={:?}, anchor_resource={}, candidate_resource={}",
            live_anchor,
            expected,
            neighbor_pose,
            carbon.name,
            nitrogen.name,
        );
        assert_eq!(
            context_energy, initial_energy,
            "free-form growth-context replay must not alter construction energy"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn opportunistic_acquisition_does_not_require_three_distinct_resources() {
        let catalog = crate::resources::default_catalog();
        let candidates = ["Carbon", "Nitrogen"]
            .into_iter()
            .map(|name| {
                catalog
                    .iter()
                    .find(|resource| resource.name == name)
                    .expect("test resource exists in catalog")
            })
            .collect::<Vec<_>>();
        let region = crate::interior_geometry::EnclosedRegion {
            area: 100.0,
            boundary_units: Vec::new(),
            sample_point: (0.0, 0.0),
            boundary: vec![(-5.0, -5.0), (5.0, -5.0), (5.0, 5.0), (-5.0, 5.0)],
        };
        let acquired = find_acquisition_resources(&candidates, &[region], &catalog);
        assert_eq!(
            acquired.len(),
            2,
            "every resource that fits is opportunistic; three distinct types are not a gate"
        );
        assert!(acquired.iter().any(|(name, _)| name == "Carbon"));
        assert!(acquired.iter().any(|(name, _)| name == "Nitrogen"));
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

        for (name, placement) in &construction.acquired_resource_placements {
            assert!(
                catalog.iter().any(|resource| resource.name == *name),
                "acquired resource {name} must belong to the catalog"
            );
            assert!(
                placement.x.is_finite()
                    && placement.y.is_finite()
                    && placement.rotation_radians.is_finite(),
                "acquired resource placements must be finite"
            );
        }
        assert!(
            construction
                .acquired_resource_shapes
                .iter()
                .all(|(name, _)| {
                    construction
                        .acquired_resource_placements
                        .iter()
                        .any(|(acquired_name, _)| acquired_name == name)
                }),
            "any realized acquired fluid shape must have a corresponding acquired resource"
        );
    }

    #[test]
    fn production_constructor_remains_viable_across_repeated_runs() {
        let catalog = crate::resources::default_catalog();
        let supplied_energy = crate::genome::initial_genome().reproductive_energy_allocation;

        for attempt in 1..=3 {
            let construction = construct_valid_with_energy(&catalog, supplied_energy)
                .unwrap_or_else(|error| panic!("constructor attempt {attempt} failed: {error}"));
            let cavity = crate::cavity::analyze_genome_cavity(&construction.structure, &catalog)
                .expect("cavity analysis should succeed")
                .expect("each constructed organism must contain a genome cavity");
            assert!(
                cavity.qualifies(),
                "attempt {attempt} produced an unqualified cavity"
            );
            assert!(
                !construction.structure.bonds.is_empty(),
                "attempt {attempt} produced an unbonded structure"
            );
            assert_eq!(
                construction.energy, supplied_energy,
                "attempt {attempt} must preserve free-form construction energy"
            );
        }
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
                arc_boundary: Some(_),
                ..
            } if (*nominal_area - expected_area).abs() < 1e-9
        ));
        assert!(placement.x.is_finite() && placement.y.is_finite());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn five_by_five_nitrogen_ring_qualifies_as_genome_cavity() {
        let catalog = crate::resources::default_catalog();
        let nitrogen = catalog
            .iter()
            .find(|resource| resource.name == "Nitrogen")
            .expect("Nitrogen exists in the default catalog");
        let mut structure = crate::structure::OrganismStructure::new();
        let mut grid = [[None; 5]; 5];

        // A one-unit-thick perimeter around a 3x3 empty interior.
        for y in 0..5 {
            for x in 0..5 {
                if x != 0 && x != 4 && y != 0 && y != 4 {
                    continue;
                }
                let index = add_unit(
                    &mut structure,
                    nitrogen,
                    (x as f64 - 2.0, y as f64 - 2.0),
                    &catalog,
                )
                .expect("nitrogen square should realize");
                grid[y][x] = Some(index);
            }
        }

        assert_eq!(structure.units.len(), 16, "ring should contain 16 squares");

        // Build bonds only through the normal live physical bond transaction.
        let mut ledger = EnergyLedger::default();
        let mut energy = 1_000_000.0;
        let mut adjacent_pairs = 0usize;
        for y in 0..5 {
            for x in 0..5 {
                let Some(a) = grid[y][x] else { continue };
                for (nx, ny) in [(x + 1, y), (x, y + 1)] {
                    if nx >= 5 || ny >= 5 {
                        continue;
                    }
                    let Some(b) = grid[ny][nx] else { continue };
                    bond_units(
                        &mut structure,
                        a,
                        b,
                        &catalog,
                        &mut ledger,
                        &mut energy,
                    )
                    .unwrap_or_else(|error| {
                        panic!(
                            "adjacent nitrogen squares ({x},{y})-({nx},{ny}) failed to bond: {error}"
                        )
                    });
                    adjacent_pairs += 1;

                    let id_a = structure.units[a].physical_id;
                    let id_b = structure.units[b].physical_id;
                    assert!(
                        structure.bonds.iter().any(|bond| {
                            (bond.endpoint_a.constituent_id == id_a
                                && bond.endpoint_b.constituent_id == id_b)
                                || (bond.endpoint_a.constituent_id == id_b
                                    && bond.endpoint_b.constituent_id == id_a)
                        }),
                        "every neighboring perimeter pair must have a physical bond"
                    );
                }
            }
        }

        assert_eq!(adjacent_pairs, 16, "ring perimeter should have 16 joins");
        assert_eq!(
            structure.connected_components().len(),
            1,
            "the ring must be one bonded structure"
        );

        let cavity = crate::cavity::analyze_genome_cavity(&structure, &catalog)
            .expect("cavity analysis should complete")
            .expect("nitrogen ring should enclose a candidate genome cavity");
        assert!(
            cavity.qualifies(),
            "cavity should meet the size/boundary contract"
        );
        assert!(
            (cavity.area - 9.0).abs() < 1e-6,
            "expected 3x3 interior area 9.0, got {}",
            cavity.area
        );
        assert!(
            cavity.area >= cavity.minimum_area,
            "genome cavity area {} must be at least the three-Carbon minimum {}",
            cavity.area,
            cavity.minimum_area
        );
        assert_eq!(
            cavity.boundary_units.len(),
            16,
            "every square in this ring must contribute to the detected cavity boundary"
        );
        assert_eq!(
            cavity.boundary_bond_indices(&structure, &catalog).len(),
            16,
            "all 16 perimeter joins must be represented by physical bonds"
        );

        // A geometrically ring-shaped arrangement is not a sealed genome if
        // even one of its required perimeter connections is absent.
        let mut missing_boundary_bond = structure.clone();
        missing_boundary_bond.bonds.pop();
        assert!(
            crate::cavity::analyze_genome_cavity(&missing_boundary_bond, &catalog)
                .expect("cavity analysis should complete after removing a bond")
                .is_none(),
            "a ring missing one perimeter bond must not qualify as a genome cavity"
        );
    }
}
