#![expect(
    dead_code,
    reason = "Staged construction helper retained for subsystem integration"
)]
use crate::construction_material_selection::{
    rank_available_construction_materials, MIN_CONSTRUCTION_MATERIAL_MATCH,
};
use crate::material_geometry::MaterialGeometry;
use crate::resources::{BaseResource, Form, Material, PhysicalState};
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

fn blueprint_cavity_reference_world(
    blueprint: &crate::structural_blueprint::StructuralBlueprint,
    anchor_world: Placement,
    anchor_declared: BlueprintPlacement,
) -> Placement {
    let mut degrees = vec![0usize; blueprint.elements.len()];
    for connection in &blueprint.connections {
        degrees[connection.element_a] += 1;
        degrees[connection.element_b] += 1;
    }
    let mut count = 0usize;
    let mut x = 0.0;
    let mut y = 0.0;
    for (index, element) in blueprint.elements.iter().enumerate() {
        if degrees[index] < 2 {
            continue;
        }
        count += 1;
        x += element.placement.x;
        y += element.placement.y;
    }
    if count == 0 {
        for element in &blueprint.elements {
            count += 1;
            x += element.placement.x;
            y += element.placement.y;
        }
    }
    let reference_x = x / count.max(1) as f64;
    let reference_y = y / count.max(1) as f64;
    let (s, c) = anchor_world.rotation_radians.sin_cos();
    Placement {
        x: anchor_world.x + (reference_x - anchor_declared.x) * c
            - (reference_y - anchor_declared.y) * s,
        y: anchor_world.y
            + (reference_x - anchor_declared.x) * s
            + (reference_y - anchor_declared.y) * c,
        rotation_radians: anchor_world.rotation_radians,
    }
}

fn candidate_penetrates_measurement(
    candidate: &StructuralUnit,
    scaffold: &crate::structural_blueprint::GenomeMeasurementScaffold,
    scaffold_origin: Placement,
    catalog: &[BaseResource],
) -> bool {
    let Some(carbon) = resource(catalog, "Carbon") else {
        return true;
    };
    let transform = |placement: BlueprintPlacement| {
        let (s, c) = scaffold_origin.rotation_radians.sin_cos();
        Placement {
            x: scaffold_origin.x + placement.x * c - placement.y * s,
            y: scaffold_origin.y + placement.x * s + placement.y * c,
            rotation_radians: scaffold_origin.rotation_radians + placement.rotation_radians,
        }
    };
    for placement in scaffold.placements {
        let world = transform(placement);
        let Some(guide) = MaterialGeometry::new(
            &Material::free_base(carbon.name.clone(), 1.0),
            &[world],
            catalog,
        ) else {
            return true;
        };
        let Some(candidate_geometry) =
            MaterialGeometry::new(&candidate.material, &[candidate.placement], catalog)
        else {
            return true;
        };
        if candidate_geometry.parts.iter().any(|part| {
            guide.parts.iter().any(|guide_part| {
                crate::material_geometry::placed_forms_penetrate(part, guide_part, 1e-10)
            })
        }) {
            return true;
        }
    }
    let triangle = [
        transform(scaffold.placements[0]),
        transform(scaffold.placements[1]),
        transform(scaffold.placements[2]),
    ];
    let triangle_points = [
        (triangle[0].x, triangle[0].y),
        (triangle[1].x, triangle[1].y),
        (triangle[2].x, triangle[2].y),
    ];
    let Some(candidate_shape) = candidate.shape(catalog) else {
        return true;
    };
    let vertices = form_vertices_world(&candidate_shape.form, candidate.placement);
    if vertices
        .iter()
        .any(|point| point_in_triangle(*point, &triangle_points))
    {
        return true;
    }
    let centroid = (
        (triangle_points[0].0 + triangle_points[1].0 + triangle_points[2].0) / 3.0,
        (triangle_points[0].1 + triangle_points[1].1 + triangle_points[2].1) / 3.0,
    );
    point_inside_form(
        &candidate_shape.form,
        candidate.placement,
        centroid.0,
        centroid.1,
    )
}

fn install_genome_measurement_scaffold(
    structure: &mut OrganismStructure,
    scaffold: &crate::structural_blueprint::GenomeMeasurementScaffold,
    scaffold_origin: Placement,
    catalog: &[BaseResource],
) -> Result<Vec<crate::structure::PhysicalConstituentId>, String> {
    let carbon = resource(catalog, "Carbon")
        .ok_or_else(|| "genome measurement scaffold requires Carbon".to_string())?;
    let transform = |p: BlueprintPlacement| {
        let (s, c) = scaffold_origin.rotation_radians.sin_cos();
        Placement {
            x: scaffold_origin.x + p.x * c - p.y * s,
            y: scaffold_origin.y + p.x * s + p.y * c,
            rotation_radians: scaffold_origin.rotation_radians + p.rotation_radians,
        }
    };

    let mut indices = [0usize; 3];
    for (slot, placement) in scaffold.placements.into_iter().enumerate() {
        let mut unit = StructuralUnit::new(carbon.name.clone(), transform(placement));
        if !unit.realize_default_geometry(catalog) {
            return Err("genome measurement scaffold has invalid Carbon geometry".into());
        }
        indices[slot] = structure.add_unit(unit);
    }

    for (a, b) in scaffold.bonds {
        let candidates =
            crate::contact::connection_pair_candidates(structure, indices[a], indices[b], catalog);
        let candidate = candidates
            .into_iter()
            .filter(|candidate| {
                candidate.distance <= crate::combine_runtime::COMBINE_CONTACT_TOLERANCE
                    && candidate.available_a
                    && candidate.available_b
            })
            .min_by(|left, right| {
                left.distance
                    .partial_cmp(&right.distance)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .ok_or_else(|| {
                "genome measurement scaffold cannot realize its internal Carbon bond".to_string()
            })?;
        crate::combine_runtime::form_internal_construction_bond(
            structure, indices[a], indices[b], candidate, catalog,
        )
        .map_err(|_| "genome measurement scaffold produced an invalid internal bond")?;
    }

    Ok(indices
        .into_iter()
        .map(|index| structure.units[index].physical_id)
        .collect())
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
                endpoints.push((
                    part_index,
                    ConnectionEndpoint::Boundary { angle_radians: 0.0 },
                ));
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
    realized_units: &[Option<usize>],
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
    let existing_index = realized_units[neighbor]?;
    let existing_unit = structure.units.get(existing_index)?;
    let existing_endpoints = structure_unit_endpoint_options(existing_unit, catalog);
    let new_endpoints = physical_material_endpoint_options(new_material, catalog);

    if existing_endpoints.is_empty() || new_endpoints.is_empty() {
        return None;
    }

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

                if let Some(scaffold) = blueprint.genome_measurement.as_ref() {
                    if indices.iter().any(|index| {
                        candidate_penetrates_measurement(
                            &trial.units[*index],
                            scaffold,
                            blueprint_cavity_reference_world(
                                blueprint,
                                genome_anchor,
                                anchor_element.placement,
                            ),
                            catalog,
                        )
                    }) {
                        continue;
                    }
                }

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
    let mut realized_units = vec![None; blueprint.elements.len()];
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
    realized_units[anchor_index] = Some(anchor_unit_index);
    let genome_anchor = structure.units[anchor_unit_index].placement;
    let temporary_scaffold_ids = blueprint
        .genome_measurement
        .as_ref()
        .map(|scaffold| {
            install_genome_measurement_scaffold(
                &mut structure,
                scaffold,
                blueprint_cavity_reference_world(
                    blueprint,
                    genome_anchor,
                    anchor_element.placement,
                ),
                catalog,
            )
        })
        .transpose()?
        .unwrap_or_default();

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
        let neighbor = neighbors[0];
        let candidate_resources = if let Some(storage) = available_materials.as_deref() {
            let ranked = rank_available_construction_materials(storage, &preferred, catalog)
                .map_err(|e| e.to_string())?;
            let mut candidates = ranked
                .iter()
                .filter(|(storage_index, _, score)| {
                    !reserved_storage_indices.contains(storage_index)
                        && *score >= MIN_CONSTRUCTION_MATERIAL_MATCH
                })
                .cloned()
                .collect::<Vec<_>>();

            // Water is the final physical construction fallback. It may be
            // used even when its structural similarity is below the normal
            // material threshold, but only after every qualifying material
            // has failed to make this bond.
            if let Some(water) = ranked.iter().find(|(storage_index, name, _)| {
                !reserved_storage_indices.contains(storage_index) && name == "Water"
            }) {
                if !candidates
                    .iter()
                    .any(|(storage_index, _, _)| *storage_index == water.0)
                {
                    candidates.push(water.clone());
                }
            }
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
        for (storage_index, candidate_name, _) in candidate_resources {
            let candidate_instance = if let Some(storage) = available_materials.as_deref() {
                let Some(crate::material_storage::StoredMaterial::Physical(instance)) =
                    storage.entries.get(storage_index)
                else {
                    continue;
                };
                instance.clone()
            } else {
                let candidate_resource = resource(catalog, &candidate_name).ok_or_else(|| {
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
                neighbor,
                genome_anchor,
                anchor_element.placement,
                &candidate_instance,
                &mut nodes,
                &construction_ledger,
                remaining_energy,
            ) {
                let new_unit_index = *new_indices
                    .get(part_index)
                    .ok_or_else(|| "successful material endpoint index disappeared".to_string())?;

                construction_ledger = trial_ledger;
                remaining_energy = trial_energy;
                total_heat += trial_attempt.work_cost;
                structure = trial_structure;
                realized[index] = true;
                realized_units[index] = Some(new_unit_index);
                if storage_index != usize::MAX {
                    reserved_storage_indices.push(storage_index);
                }
                // This successful construction step created the physical bond
                // for the prescribed edge that selected this neighbor. Record
                // that edge now; later closure work only handles edges that
                // were not already realized by a forward bond.
                for (connection_index, connection) in blueprint.connections.iter().enumerate() {
                    if (connection.element_a == index && connection.element_b == neighbor)
                        || (connection.element_a == neighbor && connection.element_b == index)
                    {
                        closed_connections[connection_index] = true;
                    }
                }
                attached = true;
                break;
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
            let Some(unit_a) = realized_units[connection.element_a] else {
                continue;
            };
            let Some(unit_b) = realized_units[connection.element_b] else {
                continue;
            };
            if structure.bonds.iter().any(|bond| {
                let a = bond.endpoint_a.constituent_id;
                let b = bond.endpoint_b.constituent_id;
                let pa = structure.units[unit_a].physical_id;
                let pb = structure.units[unit_b].physical_id;
                (a == pa && b == pb) || (a == pb && b == pa)
            }) {
                closed_connections[connection_index] = true;
                progressed = true;
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

                // A prescribed realized-realized bond is still one ordinary
                // construction bond. Try every physically admissible endpoint
                // pair rather than letting the first candidate that reaches
                // contact veto the entire connection.
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
                break;
            }
            if progressed {
                break;
            }

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
        if !progressed {
            let (
                connection_index,
                element_a,
                element_b,
                total_candidates,
                contact_candidates,
                evaluated_candidates,
                rejected_by_bond_admission,
            ) = failed_diagnostic.expect("failed closure must record diagnostics");
            let physical_a = realized_units[element_a];
            let physical_b = realized_units[element_b];
            let bond_present = physical_a.zip(physical_b).is_some_and(|(a, b)| {
                let pa = structure.units[a].physical_id;
                let pb = structure.units[b].physical_id;
                structure.bonds.iter().any(|bond| {
                    (bond.endpoint_a.constituent_id == pa && bond.endpoint_b.constituent_id == pb)
                        || (bond.endpoint_a.constituent_id == pb
                            && bond.endpoint_b.constituent_id == pa)
                })
            });
            let center_distance = physical_a
                .zip(physical_b)
                .map(|(a, b)| {
                    (structure.units[a].placement.x - structure.units[b].placement.x)
                        .hypot(structure.units[a].placement.y - structure.units[b].placement.y)
                })
                .unwrap_or(f64::NAN);
            return Err(format!(
                "bond-driven construction could not close prescribed connection {connection_index} (elements {element_a}-{element_b}): candidates={total_candidates}, contacts={contact_candidates}, evaluated={evaluated_candidates}, bond_admission_rejections={rejected_by_bond_admission}, closed_flag={}, bond_present={}, center_distance={center_distance:.6}",
                closed_connections[connection_index], bond_present
            ));
        }
    }

    if blueprint.genome_measurement.is_some() {
        structure.remove_units_by_physical_ids(&temporary_scaffold_ids);
        let cavity = crate::cavity::analyze_genome_cavity(&structure, catalog)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| {
                let carbon_positions = structure
                    .units
                    .iter()
                    .enumerate()
                    .filter(|(_, unit)| unit.material.parts.first().is_some_and(|(name, _)| name == "Carbon"))
                    .map(|(index, unit)| format!("{index}:({:.4},{:.4})", unit.placement.x, unit.placement.y))
                    .collect::<Vec<_>>()
                    .join(" ");
                format!(
                    "bond-driven construction did not form a qualifying genome cavity: units={}, bonds={}, carbons={carbon_positions}",
                    structure.units.len(),
                    structure.bonds.len()
                )
            })?;
        if !cavity.qualifies() {
            return Err(format!(
                "bond-driven construction formed an undersized genome cavity: area={:.6}, minimum={:.6}, boundary_units={:?}",
                cavity.area, cavity.minimum_area, cavity.boundary_units
            ));
        }
    }

    if let Some(storage) = available_materials.as_deref_mut() {
        reserved_storage_indices.sort_unstable();
        if reserved_storage_indices
            .iter()
            .any(|&storage_index| storage.entries.get(storage_index).is_none())
        {
            return Err(
                "construction reservation became invalid before material consumption".into(),
            );
        }
        for storage_index in reserved_storage_indices.into_iter().rev() {
            storage.take_physical_at(storage_index).ok_or_else(|| {
                format!("construction could not consume reserved material index {storage_index}")
            })?;
        }
    }

    *ledger = construction_ledger;
    *energy = remaining_energy;
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
    fn genome_measurement_scaffold_uses_blueprint_frame_not_anchor_as_its_center() {
        let catalog = crate::resources::default_catalog();
        let scaffold =
            crate::structural_blueprint::GenomeMeasurementScaffold::three_carbon_reference(
                &catalog,
            )
            .unwrap();
        let mut structure = OrganismStructure::new();
        let ids = install_genome_measurement_scaffold(
            &mut structure,
            &scaffold,
            Placement {
                x: 10.0,
                y: 20.0,
                rotation_radians: 0.0,
            },
            BlueprintPlacement {
                x: 2.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
            &catalog,
        )
        .unwrap();

        let scaffold_unit = structure
            .units
            .iter()
            .find(|unit| unit.physical_id == ids[0])
            .expect("scaffold unit must be installed");
        let expected = (
            10.0 + scaffold.placements[0].x - 2.0,
            20.0 + scaffold.placements[0].y,
        );
        assert!((scaffold_unit.placement.x - expected.0).abs() <= 1e-10);
        assert!((scaffold_unit.placement.y - expected.1).abs() <= 1e-10);
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

    #[test]
    fn temporary_genome_scaffold_is_real_physical_geometry_and_is_removed() {
        let catalog = crate::resources::default_catalog();
        let scaffold =
            crate::structural_blueprint::GenomeMeasurementScaffold::three_carbon_reference(
                &catalog,
            )
            .unwrap();
        let mut structure = OrganismStructure::new();
        let ids = install_genome_measurement_scaffold(
            &mut structure,
            &scaffold,
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
            BlueprintPlacement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
            &catalog,
        )
        .unwrap();

        assert_eq!(ids.len(), 3);
        assert_eq!(structure.units.len(), 3);
        assert_eq!(structure.bonds.len(), 3);
        assert!(structure.bonds.iter().all(|bond| bond.bond_energy == 0.0));

        structure.remove_units_by_physical_ids(&ids);
        assert!(structure.units.is_empty());
        assert!(structure.bonds.is_empty());
    }
}
