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
        restoration_failures: usize,
        overlap_rejections: usize,
        no_bond_contact: usize,
        evaluation_rejections: usize,
        combine_rejections: usize,
        last_combine_rejection: Option<String>,
    },
}

/// Try one local growth step using only rigid-edge poses recorded in Bob.
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
    available_energy: f64,
) -> Result<Vec<PersistedAttachment>, PersistedAttachmentFailure> {
    let Some(anchor) = structure.units.get(anchor_index) else {
        return Err(PersistedAttachmentFailure::CandidatesRejected {
            suggestions: 0,
            restoration_failures: 0,
            overlap_rejections: 0,
            no_bond_contact: 0,
            evaluation_rejections: 0,
            combine_rejections: 0,
            last_combine_rejection: None,
        });
    };
    let Some(anchor_material) = anchor.material.parts.first().map(|part| part.0.as_str()) else {
        return Err(PersistedAttachmentFailure::CandidatesRejected {
            suggestions: 0,
            restoration_failures: 0,
            overlap_rejections: 0,
            no_bond_contact: 0,
            evaluation_rejections: 0,
            combine_rejections: 0,
            last_combine_rejection: None,
        });
    };
    let suggestions = library.suggest_persisted_rigid_edge_placements(
        anchor_material,
        anchor.placement,
        resource,
        catalog,
    );
    if suggestions.is_empty() {
        return Err(PersistedAttachmentFailure::CoverageMiss);
    }

    let mut restoration_failures = 0;
    let mut overlap_rejections = 0;
    let mut no_bond_contact = 0;
    let mut evaluation_rejections = 0;
    let mut combine_rejections = 0;
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
            suggestions: suggestions.len(),
            restoration_failures: suggestions.len(),
            overlap_rejections,
            no_bond_contact,
            evaluation_rejections,
            combine_rejections,
            last_combine_rejection: Some(
                "candidate material failed physical realization".to_owned(),
            ),
        });
    };

    let mut successful = Vec::new();
    let mut suggestions_examined = 0usize;
    for suggestion in &suggestions {
        // Bound both successful branches and rejected attempts; every returned
        // pose is still live-validated.
        if successful.len() >= 4 || suggestions_examined >= 12 {
            break;
        }
        suggestions_examined += 1;
        let mut trial = structure.clone();
        let indices = match crate::material_restoration::restore_material(
            &mut trial,
            &instance,
            suggestion.placement,
            catalog,
        ) {
            Some(indices) if !indices.is_empty() => indices,
            _ => {
                restoration_failures += 1;
                continue;
            }
        };
        if indices.iter().any(|&index| {
            crate::construction_runtime::placed_unit_overlaps(
                &trial,
                &trial.units[index],
                &indices,
                catalog,
            )
        }) {
            overlap_rejections += 1;
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
            let Some((_, _, investment, _)) = crate::combine_runtime::selected_candidate_evaluation(
                &trial,
                anchor_index,
                new_unit_index,
                candidate,
                catalog,
            ) else {
                evaluation_rejections += 1;
                continue;
            };
            let mut committed_structure = trial.clone();
            let mut committed_ledger = *ledger;
            let mut committed_energy = available_energy;
            let mut transaction_cache = crate::contact::ConnectionCompatibilityCache::new();
            if let Err(reason) = crate::combine_runtime::form_selected_bond_diagnostic(
                &mut committed_structure,
                anchor_index,
                new_unit_index,
                candidate,
                investment,
                catalog,
                &mut transaction_cache,
                &mut committed_ledger,
                &mut committed_energy,
            ) {
                combine_rejections += 1;
                last_combine_rejection = Some(reason);
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
        suggestions: suggestions.len(),
        restoration_failures,
        overlap_rejections,
        no_bond_contact,
        evaluation_rejections,
        combine_rejections,
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
        crate::combine_runtime::form_selected_bond_diagnostic(
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
    let cycle_rank = bonds.saturating_add(1).saturating_sub(units);
    (cycle_rank, bonds, usize::MAX - units)
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
    let mut visited = std::collections::BTreeSet::new();
    for resource in &structural_resources {
        let mut structure = crate::structure::OrganismStructure::new();
        add_unit(&mut structure, resource, (0.0, 0.0), catalog)?;
        let state = ConstructionSearchState {
            structure,
            ledger: EnergyLedger::default(),
            energy: initial_energy,
        };
        if visited.insert(construction_state_signature(&state.structure)) {
            queue.push(state);
        }
    }

    let mut explored_states = 0usize;
    let mut coverage_misses = 0usize;
    let mut rejected_placements = 0usize;
    let mut restoration_failures = 0usize;
    let mut overlap_rejections = 0usize;
    let mut no_bond_contact = 0usize;
    let mut evaluation_rejections = 0usize;
    let mut combine_rejections = 0usize;
    let mut last_combine_rejection = None;
    let mut queue_budget_drops = 0usize;

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
        let mut candidates_examined = 0usize;
        'anchors: for anchor_index in 0..state.structure.units.len() {
            if state.energy <= 0.0 {
                break;
            }
            for resource in &structural_resources {
                if state.energy <= 0.0 || candidates_examined >= 12 {
                    break 'anchors;
                }
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
                        restoration_failures: step_restoration_failures,
                        overlap_rejections: step_overlap_rejections,
                        no_bond_contact: step_no_bond_contact,
                        evaluation_rejections: step_evaluation_rejections,
                        combine_rejections: step_combine_rejections,
                        last_combine_rejection: step_last_combine_rejection,
                    }) => {
                        rejected_placements += suggestions;
                        restoration_failures += step_restoration_failures;
                        overlap_rejections += step_overlap_rejections;
                        no_bond_contact += step_no_bond_contact;
                        evaluation_rejections += step_evaluation_rejections;
                        combine_rejections += step_combine_rejections;
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
        "persisted Bob constructor found no qualifying cavity within {explored_states}/{state_budget} states; queued={}, queue_budget_drops={queue_budget_drops}, coverage_misses={coverage_misses}, rejected_placements={rejected_placements}, restoration_failures={restoration_failures}, overlap_rejections={overlap_rejections}, no_bond_contact={no_bond_contact}, evaluation_rejections={evaluation_rejections}, combine_rejections={combine_rejections}, last_combine_rejection={last_combine_rejection:?}, rigid_contact_records={contact_records}",
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
        assert!(
            construction.energy <= supplied_energy,
            "construction must not create or refund energy"
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
        assert!(
            attached.energy < initial_energy,
            "COMBINE must settle against supplied energy"
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
