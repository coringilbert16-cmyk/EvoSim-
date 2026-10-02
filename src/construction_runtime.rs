#![expect(
    dead_code,
    reason = "Staged construction helper retained for subsystem integration"
)]
use crate::construction_material_selection::{
    rank_available_construction_materials, MIN_CONSTRUCTION_MATERIAL_MATCH,
};
use crate::resources::{BaseResource, Form, PhysicalState};
use crate::state::EnergyLedger;
use crate::structural_blueprint::BlueprintPlacement;
use crate::structure::{OrganismStructure, Placement, StructuralUnit};

fn resource<'a>(catalog: &'a [BaseResource], name: &str) -> Option<&'a BaseResource> {
    catalog.iter().find(|r| r.name == name)
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
        out.push(Placement {
            x: point.0 + normal.0 * *candidate_radius,
            y: point.1 + normal.1 * *candidate_radius,
            rotation_radians: 0.0,
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
            out.push(Placement {
                x: target_contact.0 - rotated_point.0,
                y: target_contact.1 - rotated_point.1,
                rotation_radians: rotation,
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
            circle_boundary_placements(&target_shape, unit.placement, &resource.shape)
        } else if matches!(&target_shape.form, Form::Circle { .. }) {
            boundary_to_circle_placements(&target_shape, unit.placement, &resource.shape)
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

fn point_in_triangle(point: (f64, f64), triangle: &[(f64, f64); 3]) -> bool {
    fn cross(a: (f64, f64), b: (f64, f64), p: (f64, f64)) -> f64 {
        (b.0 - a.0) * (p.1 - a.1) - (b.1 - a.1) * (p.0 - a.0)
    }
    let a = cross(triangle[0], triangle[1], point);
    let b = cross(triangle[1], triangle[2], point);
    let c = cross(triangle[2], triangle[0], point);
    (a >= -1e-10 && b >= -1e-10 && c >= -1e-10) || (a <= 1e-10 && b <= 1e-10 && c <= 1e-10)
}

fn form_vertices_world(form: &Form, placement: Placement) -> Vec<(f64, f64)> {
    form.polygon_vertices()
        .unwrap_or_default()
        .into_iter()
        .map(|(x, y)| {
            let (s, c) = placement.rotation_radians.sin_cos();
            (placement.x + x * c - y * s, placement.y + x * s + y * c)
        })
        .collect()
}

fn point_inside_form(form: &Form, placement: Placement, x: f64, y: f64) -> bool {
    match form {
        Form::Circle { radius } => (x - placement.x).hypot(y - placement.y) <= *radius,
        Form::Rectangle { .. } | Form::RegularPolygon { .. } | Form::Polygon { .. } => {
            let Some(vertices) = form.polygon_vertices() else {
                return false;
            };
            let (s, c) = placement.rotation_radians.sin_cos();
            let local_x = (x - placement.x) * c + (y - placement.y) * s;
            let local_y = -(x - placement.x) * s + (y - placement.y) * c;
            let mut inside = false;
            for i in 0..vertices.len() {
                let a = vertices[i];
                let b = vertices[(i + 1) % vertices.len()];
                if (a.1 > local_y) != (b.1 > local_y)
                    && local_x < (b.0 - a.0) * (local_y - a.1) / (b.1 - a.1) + a.0
                {
                    inside = !inside;
                }
            }
            inside
        }
        Form::Line { .. } | Form::Fluid { .. } => false,
    }
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

    // Do not commit the first geometrically valid attachment. The constructor
    // is allowed to look one bond ahead: after a candidate is physically
    // attached, inspect the remaining exposed surfaces and prefer the
    // candidate that leaves the most immediately available future bonds.
    //
    // This is deliberately a bounded local lookahead, not a prescribed body
    // plan. It observes actual material and bond opportunities already present
    // in the trial structure.
    let mut best: Option<(
        usize,
        f64,
        f64,
        OrganismStructure,
        Vec<usize>,
        usize,
        crate::combine_runtime::CombineAttempt,
        EnergyLedger,
        f64,
    )> = None;

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

        for candidate_origin in crate::rigid_boundary::surface_alignment_placements(
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
                matches!(
                    (candidate.endpoint_a, candidate.endpoint_b),
                    (
                        crate::structure::ConnectionEndpoint::Boundary { .. },
                        crate::structure::ConnectionEndpoint::Boundary { .. }
                    )
                ) && candidate.distance <= SURFACE_CONTACT_TOLERANCE
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

            // Look one bond ahead through the material actually restored by
            // this candidate. Count distinct existing units for which at least
            // one exposed boundary-to-boundary bond is immediately available.
            let mut future_bonds = 0usize;
            for existing_other in 0..structure.units.len() {
                if existing_other == existing_index {
                    continue;
                }
                let available = indices.iter().any(|&new_index| {
                    if new_index == existing_other {
                        return false;
                    }
                    crate::contact::connection_pair_candidates_cached(
                        &trial,
                        new_index,
                        existing_other,
                        catalog,
                        &mut crate::contact::ConnectionCompatibilityCache::new(),
                    )
                    .into_iter()
                    .any(|future| {
                        matches!(
                            (future.endpoint_a, future.endpoint_b),
                            (
                                crate::structure::ConnectionEndpoint::Boundary { .. },
                                crate::structure::ConnectionEndpoint::Boundary { .. }
                            )
                        ) && future.distance <= SURFACE_CONTACT_TOLERANCE
                            && future.available_a
                            && future.available_b
                    })
                });
                if available {
                    future_bonds += 1;
                }
            }

            let score = (future_bonds, candidate.facing, -candidate.distance);
            let replace = best.as_ref().is_none_or(|current| {
                score.0 > current.0
                    || (score.0 == current.0
                        && (score.1 > current.1 || (score.1 == current.1 && score.2 > current.2)))
            });
            if replace {
                best = Some((
                    score.0,
                    score.1,
                    score.2,
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

    best.map(
        |(
            _future_bonds,
            _facing,
            _negative_distance,
            structure,
            indices,
            part_index,
            attempt,
            ledger,
            energy,
        )| (structure, indices, part_index, attempt, ledger, energy),
    )
}

fn realize_next_bond_driven(
    blueprint: &crate::structural_blueprint::StructuralBlueprint,
    catalog: &[BaseResource],
    structure: &OrganismStructure,
    realized_units: &[Option<Vec<usize>>],
    index: usize,
    neighbor: usize,
    genome_anchor: Placement,
    anchor_declared: BlueprintPlacement,
    new_material: &crate::physical_material::PhysicalMaterial,
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
    let existing_indices = realized_units[neighbor].as_ref()?.clone();
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

            for candidate_origin in crate::rigid_boundary::surface_alignment_placements(
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
                    placed_unit_overlaps(&trial, &trial.units[unit_index], &ignored_units, catalog)
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
                    matches!(
                        (candidate.endpoint_a, candidate.endpoint_b),
                        (
                            crate::structure::ConnectionEndpoint::Boundary { .. },
                            crate::structure::ConnectionEndpoint::Boundary { .. }
                        )
                    ) && candidate.distance <= SURFACE_CONTACT_TOLERANCE
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
                for required_neighbor in already_realized_neighbors(blueprint, index, &realized) {
                    if required_neighbor == neighbor {
                        continue;
                    }
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
            _topology_score,
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
    if blueprint
        .elements
        .iter()
        .any(|element| element.material.parts.len() != 1)
    {
        return Err(
            "bond-driven developmental construction currently requires single-constituent elements"
                .into(),
        );
    }

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
    let mut closed_connections = vec![false; blueprint.connections.len()];

    let mut anchor_storage_index = None;
    let anchor_instance = if let Some(storage) = available_materials.as_deref_mut() {
        let candidates = rank_available_construction_materials(storage, &anchor_preferred, catalog)
            .map_err(|e| e.to_string())?;
        let (storage_index, _, _) = candidates
            .into_iter()
            .find(|(storage_index, _, score)| {
                !reserved_storage_indices.contains(storage_index)
                    && *score >= MIN_CONSTRUCTION_MATERIAL_MATCH
            })
            .ok_or_else(|| format!(
                "construction material need: preferred={anchor_preferred}, threshold={MIN_CONSTRUCTION_MATERIAL_MATCH:.6}"
            ))?;
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
        // Select the next element only from its currently realized neighbors.
        // Total blueprint degree is deliberately not a tie-breaker: that would
        // use knowledge of future connections to choose which bond gets formed
        // first. If several elements are equally ready, blueprint index provides
        // a deterministic order.
        let mut next: Option<(usize, Vec<usize>)> = None;
        for index in 0..blueprint.elements.len() {
            if realized[index] {
                continue;
            }
            let neighbors = already_realized_neighbors(blueprint, index, &realized);
            if neighbors.is_empty() {
                continue;
            }
            if next
                .as_ref()
                .is_none_or(|(current_index, current_neighbors)| {
                    neighbors.len() > current_neighbors.len()
                        || (neighbors.len() == current_neighbors.len() && index < *current_index)
                })
            {
                next = Some((index, neighbors));
            }
        }

        let Some((index, neighbors)) = next else {
            return Err(
                "bond-driven constructor reached an unrealized disconnected element".into(),
            );
        };

        // One bond, one committed construction step. The constructor does
        // not look ahead and reject a material because some later connection
        // might be difficult. The next construction step gets to solve that
        // next joint using whatever material is actually available then.
        let preferred = blueprint.elements[index].material.parts[0].0.clone();
        let candidate_resources = if let Some(storage) = available_materials.as_deref() {
            let ranked = rank_available_construction_materials(storage, &preferred, catalog)
                .map_err(|e| e.to_string())?;
            let candidates = ranked
                .iter()
                .filter(|(storage_index, _, score)| {
                    !reserved_storage_indices.contains(storage_index)
                        && *score >= MIN_CONSTRUCTION_MATERIAL_MATCH
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

        if candidate_resources.is_empty() && available_materials.is_some() {
            let best = rank_available_construction_materials(
                available_materials.as_deref().expect("checked above"),
                &preferred,
                catalog,
            )
            .map_err(|e| e.to_string())?
            .first()
            .map(|candidate| candidate.2)
            .unwrap_or(0.0);
            return Err(format!(
                "construction material need: preferred={preferred}, best_available_structural_match={best:.6}, threshold={MIN_CONSTRUCTION_MATERIAL_MATCH:.6}"
            ));
        }

        let mut attached = false;
        // A not-yet-realized element may have more than one realized blueprint
        // neighbor. Each neighbor is an independently valid forward anchor;
        // failure against one must not strand the element when another neighbor
        // can admit the same physical material through the shared bond authority.
        'neighbors: for neighbor in neighbors.iter().copied() {
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

                if let Some((
                    trial_structure,
                    new_indices,
                    _part_index,
                    trial_attempt,
                    trial_ledger,
                    trial_energy,
                )) = realize_next_bond_driven(
                    blueprint,
                    catalog,
                    &structure,
                    &realized_units,
                    index,
                    neighbor,
                    genome_anchor,
                    anchor_element.placement,
                    &candidate_instance,
                    &mut nodes,
                    &construction_ledger,
                    remaining_energy,
                ) {
                    construction_ledger = trial_ledger;
                    remaining_energy = trial_energy;
                    total_heat += trial_attempt.work_cost;
                    structure = trial_structure;
                    realized[index] = true;
                    realized_units[index] = Some(new_indices.clone());
                    if storage_index != usize::MAX {
                        reserved_storage_indices.push(storage_index);
                    }
                    // This successful construction step created the physical
                    // bond for the prescribed edge that selected this neighbor.
                    // Record that edge now; later closure work only handles
                    // edges that were not already realized by a forward bond.
                    for (connection_index, connection) in blueprint.connections.iter().enumerate() {
                        if (connection.element_a == index && connection.element_b == neighbor)
                            || (connection.element_a == neighbor && connection.element_b == index)
                        {
                            closed_connections[connection_index] = true;
                            break;
                        }
                    }
                    attached = true;
                    break 'neighbors;
                }
            }
        }

        if !attached {
            return Err(format!(
                "no forward bond-driven placement found for blueprint element {index} after {nodes} placement attempts"
            ));
        }
    }

    // All elements now have permanent physical poses. Any blueprint bonds
    // between already-realized elements are completed as ordinary, single-bond
    // construction steps. This is not future lookahead: the endpoints and
    // geometry already exist, and a failed closure never moves or undoes a
    // committed bond.
    while closed_connections.iter().any(|closed| !closed) {
        let mut progressed = false;
        let mut failed_diagnostic = None;
        for (connection_index, connection) in blueprint.connections.iter().enumerate() {
            if closed_connections[connection_index] {
                continue;
            }
            let Some(units_a) = realized_units[connection.element_a].as_ref() else {
                continue;
            };
            let Some(units_b) = realized_units[connection.element_b].as_ref() else {
                continue;
            };
            'unit_pairs: for &unit_a in units_a {
                for &unit_b in units_b {
                    if unit_a == unit_b {
                        continue;
                    }
                    let candidates = crate::contact::connection_pair_candidates_cached(
                        &structure,
                        unit_a,
                        unit_b,
                        catalog,
                        &mut crate::contact::ConnectionCompatibilityCache::new(),
                    );
                    let total_candidates = candidates.len();
                    let mut contact_candidates = 0usize;
                    let mut evaluated_candidates = 0usize;
                    let mut rejected_by_bond_admission = 0usize;
                    for candidate in candidates.into_iter().filter(|candidate| {
                        candidate.distance <= SURFACE_CONTACT_TOLERANCE
                            && candidate.available_a
                            && candidate.available_b
                    }) {
                        contact_candidates += 1;
                        let Some((_, _, _, investment, _)) =
                            crate::combine_runtime::selected_candidate_evaluation(
                                &structure, unit_a, unit_b, candidate, catalog,
                            )
                        else {
                            continue;
                        };
                        evaluated_candidates += 1;

                        let mut trial_structure = structure.clone();
                        let mut trial_ledger = construction_ledger;
                        let mut trial_energy = remaining_energy;
                        let mut bond_cache = crate::contact::ConnectionCompatibilityCache::new();
                        let Some(attempt) = crate::combine_runtime::form_selected_bond(
                            &mut trial_structure,
                            unit_a,
                            unit_b,
                            candidate,
                            investment,
                            catalog,
                            &mut bond_cache,
                            &mut trial_ledger,
                            &mut trial_energy,
                        ) else {
                            rejected_by_bond_admission += 1;
                            continue;
                        };

                        structure = trial_structure;
                        construction_ledger = trial_ledger;
                        remaining_energy = trial_energy;
                        total_heat += attempt.work_cost;
                        closed_connections[connection_index] = true;
                        progressed = true;
                        break 'unit_pairs;
                    }

                    if total_candidates > 0
                        || contact_candidates > 0
                        || evaluated_candidates > 0
                        || rejected_by_bond_admission > 0
                    {
                        failed_diagnostic = Some((
                            connection_index,
                            connection.element_a,
                            connection.element_b,
                            total_candidates,
                            contact_candidates,
                            evaluated_candidates,
                            rejected_by_bond_admission,
                        ));
                    }
                }
            }
            if progressed {
                break;
            }
            if let Some((
                connection_index,
                element_a,
                element_b,
                total_candidates,
                contact_candidates,
                evaluated_candidates,
                rejected_by_bond_admission,
            )) = failed_diagnostic
            {
                return Err(format!(
                    "construction closure failed: connection={connection_index} elements=({element_a},{element_b}) candidates={total_candidates} contacts={contact_candidates} evaluated={evaluated_candidates} rejected={rejected_by_bond_admission}"
                ));
            }
            return Err("construction closure made no progress".to_string());
        }
    }

    Ok((structure, total_heat))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bond_driven_triangle_commits_all_realized_neighbor_bonds() {
        use crate::resources::Material;
        use crate::structural_blueprint::{
            BlueprintConnection, BlueprintElement, BlueprintPlacement,
        };

        let catalog = crate::resources::default_catalog();
        let radius = catalog
            .iter()
            .find(|resource| resource.name == "Carbon")
            .and_then(|resource| match resource.shape.form {
                crate::resources::Form::RegularPolygon { radius, .. } => Some(radius),
                _ => None,
            })
            .unwrap();

        let spacing = (3.0_f64).sqrt() * radius;
        let blueprint = crate::structural_blueprint::StructuralBlueprint::with_anchor_elements(
            vec![
                BlueprintElement {
                    material: Material::free_base("Carbon", 1.0),
                    placement: BlueprintPlacement {
                        x: 0.0,
                        y: 0.0,
                        rotation_radians: 0.0,
                    },
                },
                BlueprintElement {
                    material: Material::free_base("Carbon", 1.0),
                    placement: BlueprintPlacement {
                        x: spacing,
                        y: 0.0,
                        rotation_radians: 0.0,
                    },
                },
                BlueprintElement {
                    material: Material::free_base("Carbon", 1.0),
                    placement: BlueprintPlacement {
                        x: spacing / 2.0,
                        y: spacing * 0.8660254037844386,
                        rotation_radians: 0.0,
                    },
                },
            ],
            vec![
                BlueprintConnection {
                    element_a: 0,
                    element_b: 1,
                },
                BlueprintConnection {
                    element_a: 0,
                    element_b: 2,
                },
                BlueprintConnection {
                    element_a: 1,
                    element_b: 2,
                },
            ],
            vec![0],
        );

        let mut ledger = EnergyLedger::default();
        let mut energy = 1.0e6;
        let (structure, _) =
            construct_blueprint_bond_driven(&blueprint, &catalog, &mut ledger, &mut energy)
                .unwrap();

        assert_eq!(structure.units.len(), 3);
        assert_eq!(structure.bonds.len(), 3);
    }

    #[test]
    fn restored_composite_overlap_is_rejected_against_existing_structure() {
        let catalog = crate::resources::default_catalog();
        let mut structure = OrganismStructure::new();

        let existing = StructuralUnit::from_material(
            crate::resources::Material::free_base("Carbon", 1.0),
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        )
        .unwrap();
        let existing_index = structure.add_unit(existing);

        let overlapping = StructuralUnit::from_material(
            crate::resources::Material::free_base("Carbon", 1.0),
            Placement {
                x: 0.5,
                y: 0.0,
                rotation_radians: 0.0,
            },
        )
        .unwrap();

        assert!(placed_unit_overlaps(
            &structure,
            &overlapping,
            &[existing_index + 1],
            &catalog,
        ));
        assert!(!placed_unit_overlaps(
            &structure,
            &overlapping,
            &[existing_index],
            &catalog,
        ));
    }
}
