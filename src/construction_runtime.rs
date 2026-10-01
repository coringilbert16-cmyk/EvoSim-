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
use crate::structure::{ConnectionEndpoint, OrganismStructure, Placement, StructuralUnit};

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
        let target_endpoints: Vec<ConnectionEndpoint> = match &target_shape.form {
            Form::Rectangle { .. } | Form::RegularPolygon { .. } | Form::Polygon { .. } => {
                let count = target_shape.form.polygon_vertices().map_or(0, |v| v.len());
                (0..count)
                    .map(|i| ConnectionEndpoint::Corner { point_index: i })
                    .collect()
            }
            Form::Line { .. } => (0..2)
                .map(|i| ConnectionEndpoint::LineEndpoint { point_index: i })
                .collect(),
            Form::Circle { .. } | Form::Fluid { .. } => Vec::new(),
        };
        for te in target_endpoints {
            let Some(tp) = te.world_point(unit, catalog) else {
                continue;
            };
            match (&resource.shape.form, te) {
                (
                    Form::Rectangle { .. } | Form::RegularPolygon { .. } | Form::Polygon { .. },
                    ConnectionEndpoint::Corner {
                        point_index: target_index,
                    },
                ) => {
                    let Some(candidate_count) =
                        resource.shape.form.polygon_vertices().map(|v| v.len())
                    else {
                        continue;
                    };
                    for candidate_index in 0..candidate_count {
                        if let (Some(candidate_normal), Some(target_normal)) = (
                            crate::rigid_boundary::corner_normal(&resource.shape, candidate_index),
                            crate::rigid_boundary::corner_normal(target_shape, target_index),
                        ) {
                            let candidate_angle = candidate_normal.1.atan2(candidate_normal.0);
                            let target_angle = target_normal.1.atan2(target_normal.0);
                            let rotation = target_angle + std::f64::consts::PI - candidate_angle;
                            if let Some(local) = crate::rigid_boundary::world_vertex(
                                &resource.shape,
                                candidate_index,
                                Placement {
                                    x: 0.0,
                                    y: 0.0,
                                    rotation_radians: rotation,
                                },
                            ) {
                                out.push(Placement {
                                    x: tp.x - local.0,
                                    y: tp.y - local.1,
                                    rotation_radians: rotation,
                                });
                            }
                        }
                        for rotation in crate::rigid_boundary::corner_alignment_rotations(
                            &resource.shape,
                            candidate_index,
                            target_shape,
                            target_index,
                            unit.placement.rotation_radians,
                        ) {
                            let Some(local) = crate::rigid_boundary::world_vertex(
                                &resource.shape,
                                candidate_index,
                                Placement {
                                    x: 0.0,
                                    y: 0.0,
                                    rotation_radians: rotation,
                                },
                            ) else {
                                continue;
                            };
                            out.push(Placement {
                                x: tp.x - local.0,
                                y: tp.y - local.1,
                                rotation_radians: rotation,
                            });
                        }
                    }
                }
                (
                    Form::Rectangle { .. } | Form::RegularPolygon { .. } | Form::Polygon { .. },
                    ConnectionEndpoint::LineEndpoint {
                        point_index: target_index,
                    },
                ) => {
                    let Some(candidate_count) =
                        resource.shape.form.polygon_vertices().map(|v| v.len())
                    else {
                        continue;
                    };
                    let Some(target_normal) =
                        crate::rigid_boundary::line_endpoint_normal(target_shape, target_index)
                    else {
                        continue;
                    };
                    let target_normal_angle = target_normal.1.atan2(target_normal.0);
                    for candidate_index in 0..candidate_count {
                        let Some(candidate_normal) =
                            crate::rigid_boundary::corner_normal(&resource.shape, candidate_index)
                        else {
                            continue;
                        };
                        let candidate_normal_angle = candidate_normal.1.atan2(candidate_normal.0);
                        let rotation =
                            target_normal_angle + std::f64::consts::PI - candidate_normal_angle;
                        let Some(local) = crate::rigid_boundary::world_vertex(
                            &resource.shape,
                            candidate_index,
                            Placement {
                                x: 0.0,
                                y: 0.0,
                                rotation_radians: rotation,
                            },
                        ) else {
                            continue;
                        };
                        out.push(Placement {
                            x: tp.x - local.0,
                            y: tp.y - local.1,
                            rotation_radians: rotation,
                        });
                    }
                }
                (
                    Form::Line {
                        length: candidate_length,
                    },
                    ConnectionEndpoint::Corner {
                        point_index: target_index,
                    },
                ) => {
                    let Some(target_normal) =
                        crate::rigid_boundary::corner_normal(target_shape, target_index)
                    else {
                        continue;
                    };
                    let target_normal_angle = target_normal.1.atan2(target_normal.0);
                    let half = *candidate_length / 2.0;
                    for candidate_index in 0..2 {
                        let candidate_normal = crate::rigid_boundary::line_endpoint_normal(
                            &resource.shape,
                            candidate_index,
                        )
                        .unwrap();
                        let candidate_normal_angle = candidate_normal.1.atan2(candidate_normal.0);
                        let rotation =
                            target_normal_angle + std::f64::consts::PI - candidate_normal_angle;
                        let local_x = if candidate_index == 0 { -half } else { half };
                        let (s, c) = rotation.sin_cos();
                        let lx = local_x * c;
                        let ly = local_x * s;
                        out.push(Placement {
                            x: tp.x - lx,
                            y: tp.y - ly,
                            rotation_radians: rotation,
                        });
                    }
                }
                (
                    Form::Line {
                        length: candidate_length,
                    },
                    ConnectionEndpoint::LineEndpoint {
                        point_index: target_index,
                    },
                ) => {
                    if !matches!(target_shape.form, Form::Line { .. }) {
                        continue;
                    }
                    let half = *candidate_length / 2.0;
                    for candidate_index in 0..2 {
                        let candidate_endpoint_x = if candidate_index == 0 { -half } else { half };
                        for rotation in crate::rigid_boundary::line_endpoint_alignment_rotations(
                            candidate_index,
                            target_index,
                            unit.placement.rotation_radians,
                        ) {
                            let (s, c) = rotation.sin_cos();
                            let lx = candidate_endpoint_x * c;
                            let ly = candidate_endpoint_x * s;
                            out.push(Placement {
                                x: tp.x - lx,
                                y: tp.y - ly,
                                rotation_radians: rotation,
                            });
                        }
                    }
                }
                _ => {}
            }
        }
    }
    out.sort_by(|a, b| {
        (a.x - anchor.x)
            .hypot(a.y - anchor.y)
            .partial_cmp(&(b.x - anchor.x).hypot(b.y - anchor.y))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out.dedup_by(|a, b| {
        (a.x - b.x).abs() <= 1e-10
            && (a.y - b.y).abs() <= 1e-10
            && (a.rotation_radians - b.rotation_radians).abs() <= 1e-10
    });
    out
}

fn structure_unit_endpoint_options(
    unit: &StructuralUnit,
    catalog: &[BaseResource],
) -> Vec<ConnectionEndpoint> {
    let Some(shape) = unit.shape(catalog) else {
        return Vec::new();
    };
    match &shape.form {
        Form::Rectangle { .. } | Form::RegularPolygon { .. } | Form::Polygon { .. } => shape
            .form
            .polygon_vertices()
            .map(|vertices| {
                (0..vertices.len())
                    .map(|point_index| ConnectionEndpoint::Corner { point_index })
                    .collect()
            })
            .unwrap_or_default(),
        Form::Line { .. } => (0..2)
            .map(|point_index| ConnectionEndpoint::LineEndpoint { point_index })
            .collect(),
        Form::Circle { .. } | Form::Fluid { .. } => Vec::new(),
    }
}

fn placement_for_joint(
    local_point: (f64, f64),
    joint: (f64, f64),
    rotation_radians: f64,
) -> Placement {
    let (s, c) = rotation_radians.sin_cos();
    Placement {
        x: joint.0 - (local_point.0 * c - local_point.1 * s),
        y: joint.1 - (local_point.0 * s + local_point.1 * c),
        rotation_radians,
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
fn physical_material_endpoint_options(
    instance: &crate::physical_material::PhysicalMaterial,
    catalog: &[BaseResource],
) -> Vec<(usize, ConnectionEndpoint)> {
    let Some(placements) = instance.placements.as_ref() else {
        return Vec::new();
    };
    instance
        .material
        .parts
        .iter()
        .zip(placements.iter())
        .enumerate()
        .flat_map(|(part_index, ((name, amount), placement))| {
            if (*amount - 1.0).abs() > 1e-9 {
                return Vec::new();
            }
            let Some(unit) = StructuralUnit::from_material(
                crate::resources::Material::free_base(name.clone(), *amount),
                *placement,
            ) else {
                return Vec::new();
            };
            let mut endpoints = crate::contact::endpoint_indices(&unit, catalog)
                .into_iter()
                .map(move |endpoint| (part_index, endpoint))
                .collect::<Vec<_>>();
            if endpoints.is_empty()
                && resource(catalog, name.as_str())
                    .is_some_and(|resource| resource.physical_state == PhysicalState::Fluid)
            {
                if let Some(endpoint) = crate::contact::continuous_endpoint(
                    &unit,
                    crate::contact::world_center(&unit),
                    catalog,
                ) {
                    endpoints.push((part_index, endpoint));
                }
            }
            endpoints
        })
        .collect()
}

fn physical_material_endpoint_local_point(
    instance: &crate::physical_material::PhysicalMaterial,
    part_index: usize,
    endpoint: ConnectionEndpoint,
    catalog: &[BaseResource],
) -> Option<crate::connection_geometry::WorldConnectionPoint> {
    let placements = instance.placements.as_ref()?;
    let (name, amount) = instance.material.parts.get(part_index)?;
    let placement = *placements.get(part_index)?;
    let unit = StructuralUnit::from_material(
        crate::resources::Material::free_base(name.clone(), *amount),
        placement,
    )?;
    endpoint.world_point(&unit, catalog)
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
    let existing_endpoints = structure_unit_endpoint_options(existing_unit, catalog);
    let new_endpoints = physical_material_endpoint_options(new_material, catalog);
    if existing_endpoints.is_empty() || new_endpoints.is_empty() {
        return None;
    }

    for endpoint_a in existing_endpoints {
        let joint = endpoint_a.world_point(existing_unit, catalog)?;
        for (part_index, endpoint_b) in new_endpoints.iter().copied() {
            let local_b = physical_material_endpoint_local_point(
                new_material,
                part_index,
                endpoint_b,
                catalog,
            )?;

            for step in 0..360 {
                let angle = std::f64::consts::TAU * step as f64 / 360.0;
                let candidate_origin =
                    placement_for_joint((local_b.x, local_b.y), (joint.x, joint.y), angle);
                *nodes += 1;
                if *nodes > 500_000 {
                    return None;
                }

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
                .find(|candidate| {
                    candidate.endpoint_a == endpoint_a
                        && candidate.endpoint_b == endpoint_b
                        && candidate.distance <= crate::combine_runtime::COMBINE_CONTACT_TOLERANCE
                        && candidate.available_a
                        && candidate.available_b
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
    }
    None
}

fn realize_next_bond_driven(
    blueprint: &crate::structural_blueprint::StructuralBlueprint,
    catalog: &[BaseResource],
    structure: &OrganismStructure,
    realized_units: &[Option<Vec<usize>>],
    _index: usize,
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
    let new_endpoints = physical_material_endpoint_options(new_material, catalog);

    if existing_indices.is_empty() || new_endpoints.is_empty() {
        return None;
    }

    let mut best_candidate: Option<(
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
        let existing_endpoints = structure_unit_endpoint_options(existing_unit, catalog);
        for endpoint_a in existing_endpoints {
            let joint = endpoint_a.world_point(&structure.units[existing_index], catalog)?;
            for (part_index, endpoint_b) in new_endpoints.iter().copied() {
                let local_b = physical_material_endpoint_local_point(
                    new_material,
                    part_index,
                    endpoint_b,
                    catalog,
                )?;

                // The declared blueprint pose is a preference, not a placement
                // command. Start at the rotation that puts this physical material's
                // selected endpoint on the joint while aiming its local endpoint
                // toward the declared target, then sweep the full circle.
                let target = blueprint.elements[_index].placement;
                let (s, c) = genome_anchor.rotation_radians.sin_cos();
                let target_world = (
                    genome_anchor.x + (target.x - anchor_declared.x) * c
                        - (target.y - anchor_declared.y) * s,
                    genome_anchor.y
                        + (target.x - anchor_declared.x) * s
                        + (target.y - anchor_declared.y) * c,
                );
                let ideal_angle = (joint.y - target_world.1).atan2(joint.x - target_world.0)
                    - local_b.y.atan2(local_b.x);
                for step in 0..360 {
                    let offset = std::f64::consts::TAU * step as f64 / 360.0;
                    let angle = ideal_angle + offset;
                    let candidate_origin =
                        placement_for_joint((local_b.x, local_b.y), (joint.x, joint.y), angle);
                    let target_distance = (candidate_origin.x - target_world.0)
                        .hypot(candidate_origin.y - target_world.1);

                    // Do not prune solely because this pose is farther from the
                    // declared preference than the best candidate found so far.
                    // Physical validity is evaluated only after restoration,
                    // penetration checks, and the shared bond admission. A farther
                    // pose may be the first (or only) physically valid one, so
                    // pruning here would silently turn preference into authority.

                    *nodes += 1;
                    if *nodes > 500_000 {
                        return None;
                    }

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

                    // The official connection points determine where the
                    // constructor works from and how the new material is placed.
                    // Once the material is physically placed, the actual bond may
                    // land anywhere on the touching boundaries.
                    let Some(candidate) = crate::contact::connection_pair_candidates_cached(
                        &trial,
                        existing_index,
                        new_unit_index,
                        catalog,
                        &mut bond_cache,
                    )
                    .into_iter()
                    .filter(|candidate| {
                        candidate.distance
                            <= crate::combine_runtime::COMBINE_CONTACT_TOLERANCE
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

                    if best_candidate
                        .as_ref()
                        .is_none_or(|current| target_distance < current.0)
                    {
                        best_candidate = Some((
                            target_distance,
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
    }

    best_candidate.map(
        |(_, trial, indices, part_index, attempt, trial_ledger, trial_energy)| {
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
                        candidate.distance <= crate::combine_runtime::COMBINE_CONTACT_TOLERANCE
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
