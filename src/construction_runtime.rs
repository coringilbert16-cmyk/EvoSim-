#![expect(
    dead_code,
    reason = "Staged construction helper retained for subsystem integration"
)]
use crate::construction_material_selection::rank_available_construction_materials;
use crate::resources::{BaseResource, Form, PhysicalState};
use crate::state::EnergyLedger;
use crate::structural_blueprint::BlueprintPlacement;
use crate::structure::{OrganismStructure, Placement, StructuralUnit};

fn resource<'a>(catalog: &'a [BaseResource], name: &str) -> Option<&'a BaseResource> {
    catalog.iter().find(|r| r.name == name)
}

fn commit_reserved_storage(
    available_materials: &mut Option<&mut crate::material_storage::MaterialStorage>,
    reserved_storage_indices: &[usize],
) -> Result<(), String> {
    let Some(storage) = available_materials.as_deref_mut() else {
        return Ok(());
    };

    let mut indices = reserved_storage_indices.to_vec();
    indices.sort_unstable();
    indices.dedup();
    if indices.iter().any(|&index| {
        !matches!(
            storage.entries.get(index),
            Some(crate::material_storage::StoredMaterial::Physical(instance))
                if instance.is_realized()
        )
    }) {
        return Err("construction inventory changed before commit".into());
    }

    for index in indices.into_iter().rev() {
        storage.take_physical_at(index).ok_or_else(|| {
            "construction could not consume reserved physical material".to_string()
        })?;
    }
    Ok(())
}

fn placement(p: BlueprintPlacement) -> Placement {
    Placement {
        x: p.x,
        y: p.y,
        rotation_radians: p.rotation_radians,
    }
}

/// Construction uses a stricter geometric contact check than COMBINE's
/// generic admission tolerance. Face-length compatibility and physical
/// contact are separate concepts: the former may differ by 0.5, while a
/// face-driven construction placement must actually put its selected
/// boundaries at the same physical location.
const SURFACE_CONTACT_TOLERANCE: f64 = 1.0e-8;

fn normalize_angle(angle: f64) -> f64 {
    (angle + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU) - std::f64::consts::PI
}

/// Explicit non-polygon construction path for curved boundaries. A circle has
/// no face to enumerate, so it is placed boundary-to-boundary against sampled
/// target boundary directions rather than being silently treated as a polygon.
fn circle_boundary_placements(
    target_shape: &crate::resources::Shape,
    target_placement: Placement,
    candidate_shape: &crate::resources::Shape,
    candidate_relative_placement: Placement,
) -> Vec<Placement> {
    let crate::resources::Form::Circle {
        radius: candidate_radius,
    } = &candidate_shape.form
    else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for step in 0..32 {
        let angle = step as f64 * std::f64::consts::TAU / 32.0;
        let (s, c) = angle.sin_cos();
        let Some(target_boundary) =
            crate::surface_geometry::boundary_point_toward(target_shape, c, s)
        else {
            continue;
        };
        let (rs, rc) = target_placement.rotation_radians.sin_cos();
        let point = (
            target_placement.x + target_boundary.x * rc - target_boundary.y * rs,
            target_placement.y + target_boundary.x * rs + target_boundary.y * rc,
        );
        let normal = (
            target_boundary.normal_x * rc - target_boundary.normal_y * rs,
            target_boundary.normal_x * rs + target_boundary.normal_y * rc,
        );
        let (rs, rc) = candidate_relative_placement.rotation_radians.sin_cos();
        let rotated_relative = (
            candidate_relative_placement.x * rc - candidate_relative_placement.y * rs,
            candidate_relative_placement.x * rs + candidate_relative_placement.y * rc,
        );
        out.push(Placement {
            x: point.0 + normal.0 * *candidate_radius - rotated_relative.0,
            y: point.1 + normal.1 * *candidate_radius - rotated_relative.1,
            rotation_radians: normalize_angle(-candidate_relative_placement.rotation_radians),
        });
    }
    out
}

/// Explicit reverse path for a curved target boundary. A polygonal/linear
/// candidate is oriented so its sampled boundary normal opposes the circle's
/// radial normal, then translated to exact boundary contact.
fn boundary_to_circle_placements(
    target_shape: &crate::resources::Shape,
    target_placement: Placement,
    candidate_shape: &crate::resources::Shape,
    candidate_relative_placement: Placement,
) -> Vec<Placement> {
    let crate::resources::Form::Circle {
        radius: target_radius,
    } = &target_shape.form
    else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for target_step in 0..16 {
        let target_angle = target_step as f64 * std::f64::consts::TAU / 16.0;
        let (ts, tc) = target_angle.sin_cos();
        let target_contact = (
            target_placement.x + *target_radius * tc,
            target_placement.y + *target_radius * ts,
        );
        let target_normal_angle = target_angle + target_placement.rotation_radians;

        for candidate_step in 0..16 {
            let candidate_angle = candidate_step as f64 * std::f64::consts::TAU / 16.0;
            let (cs, cc) = candidate_angle.sin_cos();
            let Some(candidate_boundary) =
                crate::surface_geometry::boundary_point_toward(candidate_shape, cc, cs)
            else {
                continue;
            };
            let candidate_normal_angle = candidate_boundary
                .normal_y
                .atan2(candidate_boundary.normal_x);
            let rotation = normalize_angle(
                target_normal_angle + std::f64::consts::PI - candidate_normal_angle,
            );
            let (rs, rc) = rotation.sin_cos();
            let rotated_point = (
                candidate_boundary.x * rc - candidate_boundary.y * rs,
                candidate_boundary.x * rs + candidate_boundary.y * rc,
            );
            let (relative_s, relative_c) = candidate_relative_placement.rotation_radians.sin_cos();
            let rotated_relative = (
                candidate_relative_placement.x * relative_c
                    - candidate_relative_placement.y * relative_s,
                candidate_relative_placement.x * relative_s
                    + candidate_relative_placement.y * relative_c,
            );
            out.push(Placement {
                x: target_contact.0 - rotated_point.0 - rotated_relative.0,
                y: target_contact.1 - rotated_point.1 - rotated_relative.1,
                rotation_radians: normalize_angle(
                    rotation - candidate_relative_placement.rotation_radians,
                ),
            });
        }
    }
    out
}

/// Compatibility wrapper retained for the existing runtime callers. The
/// placement generator is now surface-driven; it no longer enumerates
/// corner-to-corner angles.
pub(crate) fn candidate_placements(
    structure: &OrganismStructure,
    resource: &BaseResource,
    anchor: Placement,
    targets: &[usize],
    catalog: &[BaseResource],
) -> Vec<Placement> {
    let mut out = vec![anchor];

    for &target in targets {
        let Some(unit) = structure.units.get(target) else {
            continue;
        };
        let Some(target_shape) = unit.shape(catalog) else {
            continue;
        };

        let placements = if matches!(&resource.shape.form, Form::Circle { .. }) {
            circle_boundary_placements(
                &target_shape,
                unit.placement,
                &resource.shape,
                Placement {
                    x: 0.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
            )
        } else if matches!(&target_shape.form, Form::Circle { .. }) {
            boundary_to_circle_placements(
                &target_shape,
                unit.placement,
                &resource.shape,
                Placement {
                    x: 0.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
            )
        } else {
            crate::rigid_boundary::surface_alignment_placements(
                &target_shape,
                unit.placement,
                &resource.shape,
                Placement {
                    x: 0.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
            )
        };
        out.extend(placements);
    }

    out.sort_by(|a, b| {
        (a.x - anchor.x)
            .hypot(a.y - anchor.y)
            .partial_cmp(&(b.x - anchor.x).hypot(b.y - anchor.y))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out.dedup_by(|a, b| {
        (a.x - b.x).abs() <= 1.0e-10
            && (a.y - b.y).abs() <= 1.0e-10
            && normalize_angle(a.rotation_radians - b.rotation_radians).abs() <= 1.0e-10
    });
    out
}
fn material_part_candidate_placements(
    existing_shape: &crate::resources::Shape,
    existing_placement: Placement,
    candidate_shape: &crate::resources::Shape,
    candidate_relative_placement: Placement,
) -> Vec<Placement> {
    if matches!(&candidate_shape.form, Form::Circle { .. }) {
        circle_boundary_placements(
            existing_shape,
            existing_placement,
            candidate_shape,
            candidate_relative_placement,
        )
    } else if matches!(&existing_shape.form, Form::Circle { .. }) {
        boundary_to_circle_placements(
            existing_shape,
            existing_placement,
            candidate_shape,
            candidate_relative_placement,
        )
    } else {
        crate::rigid_boundary::surface_alignment_placements(
            existing_shape,
            existing_placement,
            candidate_shape,
            candidate_relative_placement,
        )
    }
}

pub(crate) fn placed_unit_overlaps(
    structure: &OrganismStructure,
    candidate: &StructuralUnit,
    ignored_units: &[usize],
    catalog: &[BaseResource],
) -> bool {
    let Some(candidate_shape) = candidate.shape(catalog) else {
        return true;
    };
    let candidate_part = crate::material_geometry::PlacedMaterialPart {
        part_index: 0,
        form: candidate_shape.form.clone(),
        placement: candidate.placement,
    };
    structure.units.iter().enumerate().any(|(index, unit)| {
        if ignored_units.contains(&index) {
            return false;
        }
        let Some(shape) = unit.shape(catalog) else {
            return true;
        };
        let existing_part = crate::material_geometry::PlacedMaterialPart {
            part_index: index + 1,
            form: shape.form.clone(),
            placement: unit.placement,
        };
        crate::material_geometry::placed_forms_penetrate(&candidate_part, &existing_part, 0.0)
    })
}

fn already_realized_neighbors(
    blueprint: &crate::structural_blueprint::StructuralBlueprint,
    index: usize,
    realized: &[bool],
) -> Vec<usize> {
    blueprint
        .connections
        .iter()
        .filter_map(|connection| {
            let neighbor = if connection.element_a == index {
                connection.element_b
            } else if connection.element_b == index {
                connection.element_a
            } else {
                return None;
            };
            realized
                .get(neighbor)
                .copied()
                .filter(|v| *v)
                .map(|_| neighbor)
        })
        .collect()
}

/// Sequential developmental constructor: choose A.x and B.y, rotate B about
/// that exact joint, and keep searching all available endpoint pairs and
/// orientations until a physically valid joint is found. In particular, when
/// a later connection would overshoot a previously established anchor, the
/// constructor tries the other connection points on that anchor rather than
/// abandoning the cavity.
fn score_supplemental_trial(
    structure: &OrganismStructure,
    trial_structure: &OrganismStructure,
    catalog: &[BaseResource],
) -> (usize, f64, f64) {
    let mut cache = crate::contact::ConnectionCompatibilityCache::new();
    let mut future_bonds = 0usize;
    let mut best_facing = f64::NEG_INFINITY;
    let mut contact_distance = f64::INFINITY;

    for new_index in structure.units.len()..trial_structure.units.len() {
        for other_index in 0..structure.units.len() {
            for candidate in crate::contact::connection_pair_candidates_cached(
                trial_structure,
                new_index,
                other_index,
                catalog,
                &mut cache,
            ) {
                if candidate.distance <= SURFACE_CONTACT_TOLERANCE
                    && candidate.available_a
                    && candidate.available_b
                {
                    future_bonds += 1;
                    best_facing = best_facing.max(candidate.facing);
                    contact_distance = contact_distance.min(candidate.distance);
                }
            }
        }
    }

    (future_bonds, best_facing, contact_distance)
}

pub(crate) fn try_attach_physical_material_bond_driven(
    structure: &OrganismStructure,
    existing_index: usize,
    new_material: &crate::physical_material::PhysicalMaterial,
    catalog: &[BaseResource],
    nodes: &mut usize,
    ledger: &EnergyLedger,
    available_energy: f64,
) -> Option<(
    OrganismStructure,
    Vec<usize>,
    usize,
    crate::combine_runtime::CombineAttempt,
    EnergyLedger,
    f64,
)> {
    let existing_unit = structure.units.get(existing_index)?;
    let existing_shape = existing_unit.shape(catalog)?;
    let placements = new_material.placements.as_ref()?;

    // Hot-path attachment primitive. Candidate generation is geometry-first;
    // the first candidate that satisfies the real physical COMBINE transaction
    // is accepted. Blueprint similarity and higher-level developmental
    // preference belong to the caller, not to this physical primitive.
    for (part_index, ((name, amount), relative)) in new_material
        .material
        .parts
        .iter()
        .zip(placements.iter())
        .enumerate()
    {
        if (*amount - 1.0).abs() > 1e-9 {
            continue;
        }
        let Some(candidate_shape) = resource(catalog, name).map(|resource| &resource.shape) else {
            continue;
        };

        for candidate_origin in material_part_candidate_placements(
            existing_shape,
            existing_unit.placement,
            candidate_shape,
            *relative,
        ) {
            *nodes += 1;

            let mut trial = structure.clone();
            let Some(indices) = crate::material_restoration::restore_material(
                &mut trial,
                new_material,
                candidate_origin,
                catalog,
            ) else {
                continue;
            };

            let new_unit_index = *indices.get(part_index)?;
            let ignored_units = indices.clone();
            if indices.iter().any(|index| {
                placed_unit_overlaps(&trial, &trial.units[*index], &ignored_units, catalog)
            }) {
                continue;
            }

            let mut trial_ledger = *ledger;
            let mut trial_energy = available_energy;
            let mut bond_cache = crate::contact::ConnectionCompatibilityCache::new();
            let Some(candidate) = crate::contact::connection_pair_candidates_cached(
                &trial,
                existing_index,
                new_unit_index,
                catalog,
                &mut bond_cache,
            )
            .into_iter()
            .filter(|candidate| {
                candidate.distance <= SURFACE_CONTACT_TOLERANCE
                    && candidate.available_a
                    && candidate.available_b
            })
            .max_by(|a, b| {
                a.facing
                    .partial_cmp(&b.facing)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| {
                        b.distance
                            .partial_cmp(&a.distance)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
            }) else {
                continue;
            };

            let Some((_, _, _, investment, _required_energy)) =
                crate::combine_runtime::selected_candidate_evaluation(
                    &trial,
                    existing_index,
                    new_unit_index,
                    candidate,
                    catalog,
                )
            else {
                continue;
            };

            let Some(attempt) = crate::combine_runtime::form_selected_bond(
                &mut trial,
                existing_index,
                new_unit_index,
                candidate,
                investment,
                catalog,
                &mut bond_cache,
                &mut trial_ledger,
                &mut trial_energy,
            ) else {
                continue;
            };

            return Some((
                trial,
                indices,
                part_index,
                attempt,
                trial_ledger,
                trial_energy,
            ));
        }
    }

    None
}

fn realize_next_bond_driven(
    blueprint: &crate::structural_blueprint::StructuralBlueprint,
    catalog: &[BaseResource],
    structure: &OrganismStructure,
    realized_units: &[Option<Vec<usize>>],
    index: usize,
    existing_index: usize,
    genome_anchor: Placement,
    anchor_declared: BlueprintPlacement,
    new_material: &crate::physical_material::PhysicalMaterial,
    nodes: &mut usize,
    ledger: &EnergyLedger,
    available_energy: f64,
) -> Option<(
    usize,
    OrganismStructure,
    Vec<usize>,
    usize,
    crate::combine_runtime::CombineAttempt,
    EnergyLedger,
    f64,
)> {
    let existing_indices = vec![existing_index];
    let placements = new_material.placements.as_ref()?;
    if existing_indices.is_empty() {
        return None;
    }

    let target = blueprint.elements[index].placement;
    let (s, c) = genome_anchor.rotation_radians.sin_cos();
    let target_world = (
        genome_anchor.x + (target.x - anchor_declared.x) * c - (target.y - anchor_declared.y) * s,
        genome_anchor.y + (target.x - anchor_declared.x) * s + (target.y - anchor_declared.y) * c,
    );
    let target_rotation = normalize_angle(
        genome_anchor.rotation_radians + target.rotation_radians - anchor_declared.rotation_radians,
    );

    // Candidate score is a preference only. Physical validity is established
    // first; among valid placements, prefer candidates that already satisfy
    // more of the element's required realized-neighbor topology, then match
    // the declared position and orientation.
    let mut best_candidate: Option<(
        usize,
        f64,
        f64,
        f64,
        OrganismStructure,
        Vec<usize>,
        usize,
        crate::combine_runtime::CombineAttempt,
        EnergyLedger,
        f64,
    )> = None;

    for existing_index in existing_indices {
        let existing_unit = structure.units.get(existing_index)?;
        let Some(existing_shape) = existing_unit.shape(catalog) else {
            continue;
        };

        for (part_index, ((name, amount), relative)) in new_material
            .material
            .parts
            .iter()
            .zip(placements.iter())
            .enumerate()
        {
            if (*amount - 1.0).abs() > 1e-9 {
                continue;
            }
            let Some(candidate_shape) = resource(catalog, name).map(|resource| &resource.shape)
            else {
                continue;
            };

            for candidate_origin in material_part_candidate_placements(
                existing_shape,
                existing_unit.placement,
                candidate_shape,
                *relative,
            ) {
                *nodes += 1;

                let mut trial = structure.clone();
                let Some(indices) = crate::material_restoration::restore_material(
                    &mut trial,
                    new_material,
                    candidate_origin,
                    catalog,
                ) else {
                    continue;
                };

                let new_unit_index = *indices.get(part_index)?;
                let ignored_units = indices.clone();
                if indices.iter().any(|unit_index| {
                    placed_unit_overlaps(&trial, &trial.units[*unit_index], &ignored_units, catalog)
                }) {
                    continue;
                }

                let mut trial_ledger = *ledger;
                let mut trial_energy = available_energy;
                let mut bond_cache = crate::contact::ConnectionCompatibilityCache::new();
                let Some(candidate) = crate::contact::connection_pair_candidates_cached(
                    &trial,
                    existing_index,
                    new_unit_index,
                    catalog,
                    &mut bond_cache,
                )
                .into_iter()
                .filter(|candidate| {
                    candidate.distance <= SURFACE_CONTACT_TOLERANCE
                        && candidate.available_a
                        && candidate.available_b
                })
                .min_by(|a, b| {
                    a.distance
                        .partial_cmp(&b.distance)
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then_with(|| {
                            b.facing
                                .partial_cmp(&a.facing)
                                .unwrap_or(std::cmp::Ordering::Equal)
                        })
                }) else {
                    continue;
                };

                let Some((_, _, _, investment, _required_energy)) =
                    crate::combine_runtime::selected_candidate_evaluation(
                        &trial,
                        existing_index,
                        new_unit_index,
                        candidate,
                        catalog,
                    )
                else {
                    continue;
                };

                let Some(attempt) = crate::combine_runtime::form_selected_bond(
                    &mut trial,
                    existing_index,
                    new_unit_index,
                    candidate,
                    investment,
                    catalog,
                    &mut bond_cache,
                    &mut trial_ledger,
                    &mut trial_energy,
                ) else {
                    continue;
                };

                let actual_candidate = trial.units[new_unit_index].placement;
                let target_distance = (actual_candidate.x - target_world.0)
                    .hypot(actual_candidate.y - target_world.1);
                let rotation_error =
                    normalize_angle(actual_candidate.rotation_radians - target_rotation).abs();

                let mut topology_score = 1usize;
                for required_neighbor in
                    already_realized_neighbors(blueprint, index, &realized_flags)
                {
                    let Some(neighbor_units) = realized_units[required_neighbor].as_ref() else {
                        continue;
                    };
                    let closes_neighbor = neighbor_units.iter().any(|&neighbor_unit| {
                        crate::contact::connection_pair_candidates_cached(
                            &trial,
                            new_unit_index,
                            neighbor_unit,
                            catalog,
                            &mut crate::contact::ConnectionCompatibilityCache::new(),
                        )
                        .into_iter()
                        .filter(|candidate| {
                            candidate.distance <= SURFACE_CONTACT_TOLERANCE
                                && candidate.available_a
                                && candidate.available_b
                        })
                        .any(|candidate| {
                            crate::combine_runtime::selected_candidate_evaluation(
                                &trial,
                                new_unit_index,
                                neighbor_unit,
                                candidate,
                                catalog,
                            )
                            .is_some()
                        })
                    });
                    if closes_neighbor {
                        topology_score += 1;
                    }
                }

                let better = best_candidate.as_ref().is_none_or(|current| {
                    topology_score > current.0
                        || (topology_score == current.0
                            && (target_distance < current.1
                                || (target_distance == current.1 && rotation_error < current.2)))
                });
                if better {
                    best_candidate = Some((
                        topology_score,
                        target_distance,
                        rotation_error,
                        candidate.distance,
                        trial,
                        indices,
                        part_index,
                        attempt,
                        trial_ledger,
                        trial_energy,
                    ));
                }
            }
        }
    }

    best_candidate.map(
        |(
            topology_score,
            _target_distance,
            _rotation_error,
            _contact_distance,
            trial,
            indices,
            part_index,
            attempt,
            trial_ledger,
            trial_energy,
        )| {
            (
                topology_score,
                trial,
                indices,
                part_index,
                attempt,
                trial_ledger,
                trial_energy,
            )
        },
    )
}

/// Close one still-unrealized developmental connection using only the
/// physical units already realized for its two blueprint elements. This does
/// not move either side or create material: it is a bond-only closure step
/// through the same COMBINE/physical-contact authority as forward growth.
fn close_one_realized_blueprint_connection(
    blueprint: &crate::structural_blueprint::StructuralBlueprint,
    catalog: &[BaseResource],
    structure: &OrganismStructure,
    realized_units: &[Option<Vec<usize>>],
    ledger: &EnergyLedger,
    available_energy: f64,
) -> Option<(
    OrganismStructure,
    crate::combine_runtime::CombineAttempt,
    EnergyLedger,
    f64,
)> {
    let mut best: Option<(
        usize,
        f64,
        OrganismStructure,
        crate::combine_runtime::CombineAttempt,
        EnergyLedger,
        f64,
    )> = None;

    for connection in &blueprint.connections {
        let Some(a_units) = realized_units
            .get(connection.element_a)
            .and_then(Option::as_ref)
        else {
            continue;
        };
        let Some(b_units) = realized_units
            .get(connection.element_b)
            .and_then(Option::as_ref)
        else {
            continue;
        };

        // One physical bond closes this developmental edge. Do not create a
        // second bond for the same blueprint connection merely because a
        // composite element exposes another pair of touching constituents.
        let already_closed = a_units.iter().any(|&unit_a| {
            let Some(id_a) = structure.physical_id(unit_a) else {
                return false;
            };
            b_units.iter().any(|&unit_b| {
                let Some(id_b) = structure.physical_id(unit_b) else {
                    return false;
                };
                structure.bonds.iter().any(|bond| {
                    (bond.endpoint_a.constituent_id == id_a
                        && bond.endpoint_b.constituent_id == id_b)
                        || (bond.endpoint_a.constituent_id == id_b
                            && bond.endpoint_b.constituent_id == id_a)
                })
            })
        });
        if already_closed {
            continue;
        }

        let mut connection_candidate_count = 0usize;
        let mut best_connection: Option<(
            f64,
            OrganismStructure,
            crate::combine_runtime::CombineAttempt,
            EnergyLedger,
            f64,
        )> = None;

        for &unit_a in a_units {
            for &unit_b in b_units {
                if unit_a == unit_b {
                    continue;
                }
                let Some(id_a) = structure.physical_id(unit_a) else {
                    continue;
                };
                let Some(id_b) = structure.physical_id(unit_b) else {
                    continue;
                };

                // An existing bond between these physical constituents already
                // closes this developmental connection. Internal bonds inside
                // a composite material cannot masquerade as an external edge
                // because the two IDs must belong to the two distinct elements.
                if structure.bonds.iter().any(|bond| {
                    (bond.endpoint_a.constituent_id == id_a
                        && bond.endpoint_b.constituent_id == id_b)
                        || (bond.endpoint_a.constituent_id == id_b
                            && bond.endpoint_b.constituent_id == id_a)
                }) {
                    continue;
                }

                let mut cache = crate::contact::ConnectionCompatibilityCache::new();
                for candidate in crate::contact::connection_pair_candidates_cached(
                    structure, unit_a, unit_b, catalog, &mut cache,
                )
                .into_iter()
                .filter(|candidate| {
                    candidate.distance <= SURFACE_CONTACT_TOLERANCE
                        && candidate.available_a
                        && candidate.available_b
                }) {
                    let Some((_, _, _, investment, _required_energy)) =
                        crate::combine_runtime::selected_candidate_evaluation(
                            structure, unit_a, unit_b, candidate, catalog,
                        )
                    else {
                        continue;
                    };

                    let mut trial = structure.clone();
                    let mut trial_ledger = *ledger;
                    let mut trial_energy = available_energy;
                    let Some(attempt) = crate::combine_runtime::form_selected_bond(
                        &mut trial,
                        unit_a,
                        unit_b,
                        candidate,
                        investment,
                        catalog,
                        &mut cache,
                        &mut trial_ledger,
                        &mut trial_energy,
                    ) else {
                        continue;
                    };

                    connection_candidate_count += 1;
                    let score = candidate.facing;
                    let replace = best_connection
                        .as_ref()
                        .is_none_or(|current| score > current.0);
                    if replace {
                        best_connection = Some((score, trial, attempt, trial_ledger, trial_energy));
                    }
                }
            }
        }

        if let Some((score, trial, attempt, trial_ledger, trial_energy)) = best_connection {
            let replace = best.as_ref().is_none_or(|current| {
                connection_candidate_count < current.0
                    || (connection_candidate_count == current.0 && score > current.1)
            });
            if replace {
                best = Some((
                    connection_candidate_count,
                    score,
                    trial,
                    attempt,
                    trial_ledger,
                    trial_energy,
                ));
            }
        }
    }

    best.map(|(_, _, structure, attempt, ledger, energy)| (structure, attempt, ledger, energy))
}

/// Bond-driven construction is forward-only. Once a bond is formed it is
/// never undone. When a requested joint cannot be realized, the constructor
/// keeps trying available material variants and orientations until it finds a
/// physically valid attachment. The resulting physical graph, not the declared
/// poses, is authoritative.
pub(crate) fn construct_blueprint_bond_driven(
    blueprint: &crate::structural_blueprint::StructuralBlueprint,
    catalog: &[BaseResource],
    ledger: &mut EnergyLedger,
    energy: &mut f64,
) -> Result<(OrganismStructure, f64), String> {
    construct_blueprint_bond_driven_internal(blueprint, catalog, ledger, energy, None)
}

/// Bond-driven construction using actual physical inventory. The structural
/// blueprint supplies the material preference; storage supplies the material
/// that can actually be used. No candidate below the structural match
/// threshold is consumed.
pub(crate) fn construct_blueprint_bond_driven_with_materials(
    blueprint: &crate::structural_blueprint::StructuralBlueprint,
    catalog: &[BaseResource],
    available_materials: &mut crate::material_storage::MaterialStorage,
    ledger: &mut EnergyLedger,
    energy: &mut f64,
) -> Result<(OrganismStructure, f64), String> {
    construct_blueprint_bond_driven_internal(
        blueprint,
        catalog,
        ledger,
        energy,
        Some(available_materials),
    )
}

fn construct_blueprint_bond_driven_internal(
    blueprint: &crate::structural_blueprint::StructuralBlueprint,
    catalog: &[BaseResource],
    ledger: &mut EnergyLedger,
    energy: &mut f64,
    mut available_materials: Option<&mut crate::material_storage::MaterialStorage>,
) -> Result<(OrganismStructure, f64), String> {
    if blueprint.elements.is_empty() {
        return Err("blueprint must contain at least one element".into());
    }
    // A blueprint element may describe an intact composite physical material.
    // Restoration, not this constructor, owns the constituent-count invariant.
    let anchor_index = *blueprint
        .anchor_elements
        .first()
        .ok_or_else(|| "blueprint has no construction anchor".to_string())?;
    let anchor_element = blueprint
        .elements
        .get(anchor_index)
        .ok_or_else(|| "construction anchor references a missing element".to_string())?;
    let anchor_preferred = anchor_element.material.parts[0].0.clone();

    let mut structure = OrganismStructure::new();
    let mut realized = vec![false; blueprint.elements.len()];
    // Each blueprint element maps to every physical unit restored for that
    // element. A stored composite remains one developmental element, but any
    // of its physical constituents may legitimately provide a later bond
    // endpoint.
    let mut realized_units = vec![None::<Vec<usize>>; blueprint.elements.len()];
    let mut construction_ledger = *ledger;
    let mut remaining_energy = *energy;
    let mut total_heat = 0.0;
    let mut nodes = 0usize;
    let mut reserved_storage_indices = Vec::<usize>::new();
    let mut deferred_elements = vec![false; blueprint.elements.len()];

    let mut anchor_storage_index = None;
    let anchor_instance = if let Some(storage) = available_materials.as_deref_mut() {
        let candidates = rank_available_construction_materials(storage, &anchor_preferred, catalog)
            .map_err(|e| e.to_string())?;
        let storage_index = candidates
            .into_iter()
            .find(|(storage_index, _, score)| {
                *score >= crate::construction_material_selection::MIN_CONSTRUCTION_MATERIAL_MATCH
                    && !reserved_storage_indices.contains(storage_index)
            })
            .map(|(storage_index, _, _)| storage_index)
            .ok_or_else(|| {
                "construction has no usable physical material for its initial structure".to_string()
            })?;
        let crate::material_storage::StoredMaterial::Physical(instance) =
            storage.entries.get(storage_index).cloned().ok_or_else(|| {
                "selected construction anchor material disappeared from storage".to_string()
            })?;
        anchor_storage_index = Some(storage_index);
        instance
    } else {
        let anchor_resource = resource(catalog, &anchor_preferred)
            .ok_or_else(|| "construction anchor references an unknown resource".to_string())?;
        crate::physical_material::PhysicalMaterial::realized(
            crate::resources::Material::free_base(anchor_resource.name.clone(), 1.0),
            vec![Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            catalog,
        )
        .ok_or_else(|| "construction anchor has invalid geometry".to_string())?
    };

    let anchor_origin = placement(anchor_element.placement);
    let anchor_indices = crate::material_restoration::restore_material(
        &mut structure,
        &anchor_instance,
        anchor_origin,
        catalog,
    )
    .ok_or_else(|| "construction anchor has invalid physical realization".to_string())?;
    if let Some(storage_index) = anchor_storage_index {
        if available_materials
            .as_deref()
            .and_then(|storage| storage.entries.get(storage_index))
            .is_none()
        {
            return Err(
                "selected construction anchor material disappeared before commit".to_string(),
            );
        }
        reserved_storage_indices.push(storage_index);
    }
    let anchor_unit_index = *anchor_indices
        .first()
        .ok_or_else(|| "construction anchor restored no physical units".to_string())?;
    realized[anchor_index] = true;
    realized_units[anchor_index] = Some(anchor_indices.clone());
    let genome_anchor = structure.units[anchor_unit_index].placement;
    while !realized.iter().all(|value| *value) {
        // The blueprint chooses what we would like to build next, but it does
        // not choose which physical unit must receive it. Every realized
        // physical unit is part of the construction frontier.
        // Forward-only construction makes the choice of the next
        // developmental element an ordering decision. Prefer an unrealized
        // element whose declared neighbors are already realized, because its
        // physical attachment can then establish one of the blueprint's
        // existing required relationships immediately. Among otherwise
        // equivalent candidates, retain blueprint order as the deterministic
        // tie-breaker. This does not add a topology requirement: elements with
        // no currently realized neighbor remain eligible.
        let Some(index) = (0..blueprint.elements.len())
            .filter(|candidate| !realized[*candidate] && !deferred_elements[*candidate])
            .max_by_key(|candidate| {
                let realized_neighbors =
                    already_realized_neighbors(blueprint, *candidate, &realized).len();
                (realized_neighbors, std::cmp::Reverse(*candidate))
            })
        else {
            break;
        };

        // One bond, one committed construction step. The constructor may try
        // every physical frontier unit and every acceptable material before
        // declaring this developmental addition impossible.

        let preferred = blueprint.elements[index].material.parts[0].0.clone();
        let candidate_resources = if let Some(storage) = available_materials.as_deref() {
            let ranked = rank_available_construction_materials(storage, &preferred, catalog)
                .map_err(|e| e.to_string())?;
            let candidates = ranked
                .iter()
                .filter(|(storage_index, _, score)| {
                    *score
                        >= crate::construction_material_selection::MIN_CONSTRUCTION_MATERIAL_MATCH
                        && !reserved_storage_indices.contains(storage_index)
                })
                .cloned()
                .collect::<Vec<_>>();

            candidates
        } else {
            // Developmental construction may substitute material when the
            // preferred material cannot make the requested physical bond.
            // Preference still controls the search order; physical geometry
            // and the shared bond authority decide what is actually admitted.
            let mut candidates = Vec::new();
            if resource(catalog, &preferred)
                .is_some_and(|candidate| candidate.physical_state == PhysicalState::Rigid)
            {
                candidates.push((usize::MAX, preferred.clone(), 1.0));
            }
            for candidate in catalog {
                if candidate.name == preferred || candidate.physical_state != PhysicalState::Rigid {
                    continue;
                }
                candidates.push((usize::MAX, candidate.name.clone(), 0.0));
            }
            // Water is the final developmental construction fallback after
            // all rigid material alternatives have failed.
            if resource(catalog, "Water").is_some() {
                candidates.push((usize::MAX, "Water".to_string(), 0.0));
            }
            candidates
        };

        // An unavailable preferred material is not construction failure. The
        // blueprint preference is only a search ordering; any available
        // physical material may be considered before supplemental construction.
        if candidate_resources.is_empty() && available_materials.is_some() {
            break;
        }

        // Search the current physical frontier once and choose the best
        // locally valid realization. No future organism is simulated here:
        // blueprint similarity and local topology are sufficient for this
        // developmental decision. This keeps the normal path linear in the
        // current frontier rather than exponential in future branches.
        let mut best_developmental: Option<(
            usize,
            f64,
            f64,
            usize,
            OrganismStructure,
            Vec<usize>,
            usize,
            crate::combine_runtime::CombineAttempt,
            EnergyLedger,
            f64,
        )> = None;

        let target = blueprint.elements[index].placement;
        let (s, c) = genome_anchor.rotation_radians.sin_cos();
        let target_world = (
            genome_anchor.x + (target.x - anchor_element.placement.x) * c
                - (target.y - anchor_element.placement.y) * s,
            genome_anchor.y
                + (target.x - anchor_element.placement.x) * s
                + (target.y - anchor_element.placement.y) * c,
        );

        for existing_index in 0..structure.units.len() {
            for (storage_index, candidate_name, _) in candidate_resources.iter().cloned() {
                let candidate_instance = if let Some(storage) = available_materials.as_deref() {
                    let Some(crate::material_storage::StoredMaterial::Physical(instance)) =
                        storage.entries.get(storage_index)
                    else {
                        continue;
                    };
                    instance.clone()
                } else {
                    let candidate_resource =
                        resource(catalog, &candidate_name).ok_or_else(|| {
                            format!("unknown preferred construction resource {candidate_name}")
                        })?;
                    crate::physical_material::PhysicalMaterial::realized(
                        crate::resources::Material::free_base(candidate_resource.name.clone(), 1.0),
                        vec![Placement {
                            x: 0.0,
                            y: 0.0,
                            rotation_radians: 0.0,
                        }],
                        catalog,
                    )
                    .ok_or_else(|| {
                        "preferred construction material could not be realized".to_string()
                    })?
                };

                let Some((
                    topology_score,
                    trial_structure,
                    new_indices,
                    part_index,
                    trial_attempt,
                    trial_ledger,
                    trial_energy,
                )) = realize_next_bond_driven(
                    blueprint,
                    catalog,
                    &structure,
                    &realized_units,
                    index,
                    existing_index,
                    genome_anchor,
                    anchor_element.placement,
                    &candidate_instance,
                    &mut nodes,
                    &construction_ledger,
                    remaining_energy,
                )
                else {
                    continue;
                };

                let Some(new_index) = new_indices.get(part_index).copied() else {
                    continue;
                };
                let Some(new_unit) = trial_structure.units.get(new_index) else {
                    continue;
                };
                let target_distance = (new_unit.placement.x - target_world.0)
                    .hypot(new_unit.placement.y - target_world.1);
                let target_rotation = normalize_angle(
                    genome_anchor.rotation_radians + target.rotation_radians
                        - anchor_element.placement.rotation_radians,
                );
                let rotation_error =
                    normalize_angle(new_unit.placement.rotation_radians - target_rotation).abs();

                let better = best_developmental.as_ref().is_none_or(|current| {
                    topology_score > current.0
                        || (topology_score == current.0
                            && (target_distance < current.1
                                || (target_distance == current.1
                                    && (rotation_error < current.2
                                        || (rotation_error == current.2
                                            && storage_index < current.3)))))
                });
                if better {
                    best_developmental = Some((
                        topology_score,
                        target_distance,
                        rotation_error,
                        storage_index,
                        trial_structure,
                        new_indices,
                        part_index,
                        trial_attempt,
                        trial_ledger,
                        trial_energy,
                    ));
                }
            }
        }

        let attached = if let Some((
            _topology_score,
            _topology_score,
            _target_distance,
            storage_index,
            trial_structure,
            new_indices,
            _part_index,
            trial_attempt,
            trial_ledger,
            trial_energy,
        )) = best_developmental
        {
            construction_ledger = trial_ledger;
            remaining_energy = trial_energy;
            total_heat += trial_attempt.work_cost;
            structure = trial_structure;
            realized[index] = true;
            realized_units[index] = Some(new_indices);
            if storage_index != usize::MAX {
                reserved_storage_indices.push(storage_index);
            }
            deferred_elements.fill(false);
            true
        } else {
            false
        };

        if !attached {
            // One blocked developmental request does not block the rest of the
            // blueprint. Defer it, try another unrealized developmental goal,
            // and revisit this one after any successful physical growth.
            deferred_elements[index] = true;
        }

        // Placement and bond closure are separate forward-only steps. A new
        // unit may already be physically touching another realized unit, but
        // that contact does not become a bond merely because the placement
        // happened to touch it. Close one still-open blueprint edge at a time,
        // without moving or undoing any committed structure.
        while let Some((closed_structure, attempt, closed_ledger, closed_energy)) =
            close_one_realized_blueprint_connection(
                blueprint,
                catalog,
                &structure,
                &realized_units,
                &construction_ledger,
                remaining_energy,
            )
        {
            structure = closed_structure;
            construction_ledger = closed_ledger;
            remaining_energy = closed_energy;
            total_heat += attempt.work_cost;
        }
    }

    // All developmental elements may be realized before the last prescribed
    // physical edges become bondable. Run the same bond-only closure pass once
    // more before abandoning the developmental phase.
    while let Some((closed_structure, attempt, closed_ledger, closed_energy)) =
        close_one_realized_blueprint_connection(
            blueprint,
            catalog,
            &structure,
            &realized_units,
            &construction_ledger,
            remaining_energy,
        )
    {
        structure = closed_structure;
        construction_ledger = closed_ledger;
        remaining_energy = closed_energy;
        total_heat += attempt.work_cost;
    }

    if crate::cavity::analyze_genome_cavity(&structure, catalog)?
        .is_some_and(|cavity| cavity.qualifies())
    {
        commit_reserved_storage(&mut available_materials, &reserved_storage_indices)?;
        *ledger = construction_ledger;
        *energy = remaining_energy;
        return Ok((structure, total_heat));
    }

    // The blueprint is guidance, not a material ceiling. If its requested    // developmental elements are exhausted and the realized structure is still
    // not viable, continue from the actual physical frontier. This phase has no
    // blueprint topology requirement; it searches real material and real contact
    // opportunities until viability is reached or no physical continuation exists.
    // With real storage, every successful supplemental step consumes and
    // reserves one finite inventory entry, so the search has an exact physical
    // inventory bound. The no-storage API synthesizes catalog material, so it
    // cannot prove physical exhaustion; its limit is explicitly a computational
    // search guard, not a biological organism-size rule.
    const SYNTHETIC_SUPPLEMENTAL_SEARCH_LIMIT: usize = 64;
    let supplemental_budget = available_materials
        .as_ref()
        .map_or(SYNTHETIC_SUPPLEMENTAL_SEARCH_LIMIT, |storage| {
            storage.entries.len()
        });

    for _ in 0..supplemental_budget {
        if crate::cavity::analyze_genome_cavity(&structure, catalog)?
            .is_some_and(|cavity| cavity.qualifies())
        {
            commit_reserved_storage(&mut available_materials, &reserved_storage_indices)?;
            *ledger = construction_ledger;
            *energy = remaining_energy;
            return Ok((structure, total_heat));
        }

        let frontier = (0..structure.units.len()).collect::<Vec<_>>();
        let mut best_supplemental: Option<(
            usize,
            f64,
            f64,
            usize,
            OrganismStructure,
            crate::combine_runtime::CombineAttempt,
            EnergyLedger,
            f64,
        )> = None;

        // Evaluate every currently available physical continuation before
        // committing. Search order is never allowed to become constructor
        // behavior. The score is entirely derived from the realized trial:
        // future exposed bond opportunities first, then bond facing, then
        // contact distance. Blueprint order is not involved in this phase.
        for existing_index in frontier {
            if let Some(storage) = available_materials.as_deref() {
                let candidates = (0..storage.entries.len())
                    .filter(|storage_index| !reserved_storage_indices.contains(storage_index))
                    .filter_map(|storage_index| match storage.entries.get(storage_index) {
                        Some(crate::material_storage::StoredMaterial::Physical(instance)) => {
                            Some((storage_index, instance.clone()))
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>();

                for (storage_index, candidate_instance) in candidates {
                    let Some((
                        trial_structure,
                        _indices,
                        _part_index,
                        attempt,
                        trial_ledger,
                        trial_energy,
                    )) = try_attach_physical_material_bond_driven(
                        &structure,
                        existing_index,
                        &candidate_instance,
                        catalog,
                        &mut nodes,
                        &construction_ledger,
                        remaining_energy,
                    )
                    else {
                        continue;
                    };

                    let (future_bonds, best_facing, contact_distance) =
                        score_supplemental_trial(&structure, &trial_structure, catalog);

                    let replace = best_supplemental.as_ref().is_none_or(|current| {
                        future_bonds > current.0
                            || (future_bonds == current.0
                                && (best_facing > current.1
                                    || (best_facing == current.1
                                        && (contact_distance < current.2
                                            || (contact_distance == current.2
                                                && storage_index < current.3)))))
                    });

                    if replace {
                        best_supplemental = Some((
                            future_bonds,
                            best_facing,
                            contact_distance,
                            storage_index,
                            trial_structure,
                            attempt,
                            trial_ledger,
                            trial_energy,
                        ));
                    }
                }
            } else {
                for (resource_index, candidate_resource) in catalog.iter().enumerate() {
                    let Some(candidate_instance) =
                        crate::physical_material::PhysicalMaterial::realized(
                            crate::resources::Material::free_base(
                                candidate_resource.name.clone(),
                                1.0,
                            ),
                            vec![Placement {
                                x: 0.0,
                                y: 0.0,
                                rotation_radians: 0.0,
                            }],
                            catalog,
                        )
                    else {
                        continue;
                    };

                    let Some((
                        trial_structure,
                        _indices,
                        _part_index,
                        attempt,
                        trial_ledger,
                        trial_energy,
                    )) = try_attach_physical_material_bond_driven(
                        &structure,
                        existing_index,
                        &candidate_instance,
                        catalog,
                        &mut nodes,
                        &construction_ledger,
                        remaining_energy,
                    )
                    else {
                        continue;
                    };

                    let (future_bonds, best_facing, contact_distance) =
                        score_supplemental_trial(&structure, &trial_structure, catalog);

                    let replace = best_supplemental.as_ref().is_none_or(|current| {
                        future_bonds > current.0
                            || (future_bonds == current.0
                                && (best_facing > current.1
                                    || (best_facing == current.1
                                        && (contact_distance < current.2
                                            || (contact_distance == current.2
                                                && resource_index < current.3)))))
                    });

                    if replace {
                        best_supplemental = Some((
                            future_bonds,
                            best_facing,
                            contact_distance,
                            resource_index,
                            trial_structure,
                            attempt,
                            trial_ledger,
                            trial_energy,
                        ));
                    }
                }
            }
        }

        let Some((
            _future_bonds,
            _best_facing,
            _contact_distance,
            candidate_order,
            trial_structure,
            attempt,
            trial_ledger,
            trial_energy,
        )) = best_supplemental
        else {
            break;
        };

        structure = trial_structure;
        construction_ledger = trial_ledger;
        remaining_energy = trial_energy;
        total_heat += attempt.work_cost;
        if candidate_order != usize::MAX {
            reserved_storage_indices.push(candidate_order);
        }
    }

    Err(
        "construction exhausted all currently available physical continuation attempts without realizing a viable physical organism"
            .to_string(),
    )
}
