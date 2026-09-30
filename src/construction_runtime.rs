#![expect(
    dead_code,
    reason = "Staged construction helper retained for subsystem integration"
)]
use crate::combine_runtime::combine_specific_pair;
use crate::resources::{BaseResource, Form, Material};
use crate::state::EnergyLedger;
use crate::structural_blueprint::{BlueprintElement, BlueprintPlacement};
use crate::structure::{ConnectionEndpoint, OrganismStructure, Placement, StructuralUnit};

type ConstructionSolution = (
    OrganismStructure,
    EnergyLedger,
    f64,
    Vec<Option<usize>>,
    f64,
    f64,
);

pub(crate) fn placement_penetrates_genome_measurement(
    candidate_resource: &BaseResource,
    placement: Placement,
    scaffold: &crate::structural_blueprint::GenomeMeasurementScaffold,
    catalog: &[BaseResource],
) -> bool {
    let Some(candidate) = crate::material_geometry::MaterialGeometry::new(
        &Material::free_base(candidate_resource.name.clone(), 1.0),
        &[placement],
        catalog,
    ) else {
        return true;
    };
    let Some(carbon) = resource(catalog, "Carbon") else {
        return true;
    };
    for guide_placement in scaffold.placements {
        let Some(guide) = crate::material_geometry::MaterialGeometry::new(
            &Material::free_base(carbon.name.clone(), 1.0),
            &[Placement {
                x: guide_placement.x,
                y: guide_placement.y,
                rotation_radians: guide_placement.rotation_radians,
            }],
            catalog,
        ) else {
            return true;
        };
        if candidate.parts.iter().any(|part| {
            guide.parts.iter().any(|guide_part| {
                crate::material_geometry::placed_forms_penetrate(part, guide_part, 1e-10)
            })
        }) {
            return true;
        }
    }
    false
}

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

fn neighbors(material: &Material, part: usize, assigned: &[Option<usize>]) -> Vec<usize> {
    material
        .internal_bonds
        .iter()
        .filter_map(|b| {
            if b.part_a == part {
                assigned[b.part_b]
            } else if b.part_b == part {
                assigned[b.part_a]
            } else {
                None
            }
        })
        .collect()
}

fn solve_external_groups(
    group_index: usize,
    structure: &OrganismStructure,
    ledger: &EnergyLedger,
    energy: f64,
    assigned: &[Option<usize>],
    external: &[Vec<usize>],
    catalog: &[BaseResource],
    heat: f64,
) -> Option<(OrganismStructure, EnergyLedger, f64, f64)> {
    let Some(group) = external.get(group_index) else {
        return Some((structure.clone(), *ledger, energy, heat));
    };

    for &new_id in assigned.iter().flatten() {
        for &target in group {
            let mut candidate = structure.clone();
            let mut candidate_ledger = *ledger;
            let mut candidate_energy = energy;
            let mut cache = crate::contact::ConnectionCompatibilityCache::new();
            let Some(attempt) = combine_specific_pair(
                &mut candidate,
                new_id,
                target,
                catalog,
                &mut cache,
                &mut candidate_ledger,
                &mut candidate_energy,
            ) else {
                continue;
            };
            if let Some(result) = solve_external_groups(
                group_index + 1,
                &candidate,
                &candidate_ledger,
                candidate_energy,
                assigned,
                external,
                catalog,
                heat + attempt.work_cost,
            ) {
                return Some(result);
            }
        }
    }
    None
}

fn solve_parts(
    part: usize,
    structure: &OrganismStructure,
    ledger: &EnergyLedger,
    energy: f64,
    assigned: &[Option<usize>],
    material: &Material,
    anchor: Placement,
    catalog: &[BaseResource],
    external: &[Vec<usize>],
    genome_measurement: Option<&crate::structural_blueprint::GenomeMeasurementScaffold>,
    heat: f64,
    score: f64,
) -> Option<ConstructionSolution> {
    if part == material.parts.len() {
        let (structure, ledger, energy, heat) = solve_external_groups(
            0, structure, ledger, energy, assigned, external, catalog, heat,
        )?;
        return Some((structure, ledger, energy, assigned.to_vec(), heat, score));
    }

    let resource = resource(catalog, &material.parts[part].0)?;
    let mut targets = neighbors(material, part, assigned);
    for group in external {
        for &target in group {
            if !targets.contains(&target) {
                targets.push(target);
            }
        }
    }

    let mut best = None;
    for candidate_placement in candidate_placements(structure, resource, anchor, &targets, catalog)
    {
        if let Some(scaffold) = genome_measurement {
            if placement_penetrates_genome_measurement(
                resource,
                candidate_placement,
                scaffold,
                catalog,
            ) {
                continue;
            }
        }
        let placement_score =
            (candidate_placement.x - anchor.x).hypot(candidate_placement.y - anchor.y);
        let candidate_score = score + placement_score;
        if best.as_ref().is_some_and(
            |result: &(
                OrganismStructure,
                EnergyLedger,
                f64,
                Vec<Option<usize>>,
                f64,
                f64,
            )| { candidate_score >= result.5 },
        ) {
            continue;
        }

        let mut candidate = structure.clone();
        let mut candidate_ledger = *ledger;
        let mut candidate_energy = energy;
        let mut unit = StructuralUnit::new(resource.name.clone(), candidate_placement);
        if !unit.realize_default_geometry(catalog) {
            continue;
        }
        let index = candidate.add_unit(unit);
        let mut candidate_assigned = assigned.to_vec();
        candidate_assigned[part] = Some(index);
        let mut candidate_heat = heat;
        let mut cache = crate::contact::ConnectionCompatibilityCache::new();
        let mut ok = true;

        for bond in material
            .internal_bonds
            .iter()
            .filter(|b| b.part_a == part || b.part_b == part)
        {
            let (Some(a), Some(b)) = (
                candidate_assigned[bond.part_a],
                candidate_assigned[bond.part_b],
            ) else {
                continue;
            };
            match combine_specific_pair(
                &mut candidate,
                a,
                b,
                catalog,
                &mut cache,
                &mut candidate_ledger,
                &mut candidate_energy,
            ) {
                Some(attempt) => candidate_heat += attempt.work_cost,
                None => {
                    ok = false;
                    break;
                }
            }
        }
        if !ok {
            continue;
        }

        if let Some(result) = solve_parts(
            part + 1,
            &candidate,
            &candidate_ledger,
            candidate_energy,
            &candidate_assigned,
            material,
            anchor,
            catalog,
            external,
            genome_measurement,
            candidate_heat,
            candidate_score,
        ) {
            if best.as_ref().is_none_or(|current| result.5 < current.5) {
                best = Some(result);
            }
        }
    }
    best
}

pub(crate) fn realize_material_with_context(
    structure: &mut OrganismStructure,
    element: &BlueprintElement,
    catalog: &[BaseResource],
    ledger: &mut EnergyLedger,
    energy: &mut f64,
    external: &[Vec<usize>],
    genome_measurement: Option<&crate::structural_blueprint::GenomeMeasurementScaffold>,
) -> Result<(Vec<usize>, f64), String> {
    let material = &element.material;
    let anchor = placement(element.placement);
    if material.parts.is_empty() {
        return Err("construction material must contain at least one constituent".into());
    }

    let assigned = vec![None; material.parts.len()];
    let Some((trial, trial_ledger, trial_energy, assigned, heat, _score)) = solve_parts(
        0,
        structure,
        ledger,
        *energy,
        &assigned,
        material,
        anchor,
        catalog,
        external,
        genome_measurement,
        0.0,
        0.0,
    ) else {
        let resource_name = material
            .parts
            .first()
            .map(|(name, _)| name.as_str())
            .unwrap_or("<none>");
        let targets = external.iter().flatten().copied().collect::<Vec<_>>();
        let candidate_count = resource(catalog, resource_name)
            .map(|resource| {
                candidate_placements(structure, resource, anchor, &targets, catalog).len()
            })
            .unwrap_or(0);
        return Err(format!(
            "construction placement could not satisfy physical constraints (material={resource_name}, external_targets={targets:?}, candidate_placements={candidate_count})"
        ));
    };

    *structure = trial;
    *ledger = trial_ledger;
    *energy = trial_energy;
    Ok((assigned.into_iter().flatten().collect(), heat))
}

pub(crate) fn realize_material(
    structure: &mut OrganismStructure,
    material: &Material,
    anchor: Placement,
    catalog: &[BaseResource],
    ledger: &mut EnergyLedger,
    energy: &mut f64,
) -> Result<Vec<usize>, String> {
    let element = BlueprintElement {
        material: material.clone(),
        placement: BlueprintPlacement {
            x: anchor.x,
            y: anchor.y,
            rotation_radians: anchor.rotation_radians,
        },
    };
    realize_material_with_context(structure, &element, catalog, ledger, energy, &[], None)
        .map(|(indices, _)| indices)
}

fn blueprint_endpoint_options(resource: &BaseResource) -> Vec<ConnectionEndpoint> {
    match &resource.shape.form {
        Form::Rectangle { .. } | Form::RegularPolygon { .. } | Form::Polygon { .. } => resource
            .shape
            .form
            .polygon_vertices()
            .unwrap_or_default()
            .iter()
            .enumerate()
            .map(|(point_index, _)| ConnectionEndpoint::Corner { point_index })
            .collect(),
        Form::Line { .. } => vec![
            ConnectionEndpoint::LineEndpoint { point_index: 0 },
            ConnectionEndpoint::LineEndpoint { point_index: 1 },
        ],
        Form::Circle { .. } => (0..72)
            .map(|i| ConnectionEndpoint::Boundary {
                angle_radians: std::f64::consts::TAU * i as f64 / 72.0,
            })
            .collect(),
        Form::Fluid { .. } => Vec::new(),
    }
}

fn endpoint_local_point(
    resource: &BaseResource,
    endpoint: ConnectionEndpoint,
    catalog: &[BaseResource],
) -> Option<(f64, f64)> {
    let unit = StructuralUnit::new(
        resource.name.clone(),
        Placement {
            x: 0.0,
            y: 0.0,
            rotation_radians: 0.0,
        },
    );
    let point = endpoint.world_point(&unit, catalog)?;
    Some((point.x, point.y))
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

fn angle_error(a: f64, b: f64) -> f64 {
    let mut d = (a - b).rem_euclid(std::f64::consts::TAU);
    if d > std::f64::consts::PI {
        d -= std::f64::consts::TAU;
    }
    d.abs()
}

fn blueprint_pose_score(candidate: Placement, target: BlueprintPlacement) -> f64 {
    (candidate.x - target.x).hypot(candidate.y - target.y)
        + 0.25 * angle_error(candidate.rotation_radians, target.rotation_radians)
}

fn placed_unit_overlaps(
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
        crate::material_geometry::placed_forms_overlap(&candidate_part, &existing_part, 0.0)
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

fn candidate_penetrates_measurement(
    candidate: &StructuralUnit,
    scaffold: &crate::structural_blueprint::GenomeMeasurementScaffold,
    anchor: Placement,
    catalog: &[BaseResource],
) -> bool {
    let Some(carbon) = resource(catalog, "Carbon") else {
        return true;
    };
    let transform = |placement: BlueprintPlacement| {
        let (s, c) = anchor.rotation_radians.sin_cos();
        Placement {
            x: anchor.x + placement.x * c - placement.y * s,
            y: anchor.y + placement.x * s + placement.y * c,
            rotation_radians: anchor.rotation_radians + placement.rotation_radians,
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
    if crate::organism_geometry::point_in_form(
        &candidate_shape.form,
        candidate.placement,
        centroid.0,
        centroid.1,
    ) {
        return true;
    }
    false
}

fn best_existing_connection(
    structure: &OrganismStructure,
    new_unit: usize,
    existing_unit: usize,
    catalog: &[BaseResource],
) -> Option<(ConnectionEndpoint, ConnectionEndpoint)> {
    crate::contact::connection_pair_candidates(structure, existing_unit, new_unit, catalog)
        .into_iter()
        .filter(|candidate| candidate.available_a && candidate.available_b)
        .max_by(|a, b| {
            a.facing
                .partial_cmp(&b.facing)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    b.distance
                        .partial_cmp(&a.distance)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        })
        .map(|candidate| (candidate.endpoint_a, candidate.endpoint_b))
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
/// that joint, score the locked pose against the blueprint, then commit bonds.
pub(crate) fn construct_blueprint_bond_driven(
    blueprint: &crate::structural_blueprint::StructuralBlueprint,
    catalog: &[BaseResource],
    ledger: &mut EnergyLedger,
    energy: &mut f64,
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
    let anchor_resource = resource(catalog, &anchor_element.material.parts[0].0)
        .ok_or_else(|| "construction anchor references an unknown resource".to_string())?;

    let mut structure = OrganismStructure::new();
    let mut realized = vec![false; blueprint.elements.len()];
    let mut realized_units = vec![None; blueprint.elements.len()];
    let mut total_heat = 0.0;
    let mut anchor_unit = StructuralUnit::new(
        anchor_resource.name.clone(),
        placement(anchor_element.placement),
    );
    if !anchor_unit.realize_default_geometry(catalog) {
        return Err("construction anchor has invalid geometry".into());
    }
    let anchor_unit_index = structure.add_unit(anchor_unit);
    realized[anchor_index] = true;
    realized_units[anchor_index] = Some(anchor_unit_index);
    let genome_anchor = structure.units[anchor_unit_index].placement;
    let mut progress = 1usize;

    while progress < blueprint.elements.len() {
        let mut next = None;
        for index in 0..blueprint.elements.len() {
            if realized[index] {
                continue;
            }
            let neighbors = already_realized_neighbors(blueprint, index, &realized);
            if !neighbors.is_empty() {
                next = Some((index, neighbors));
                break;
            }
        }
        let Some((index, neighbors)) = next else {
            return Err(
                "bond-driven construction stalled before all blueprint elements were realized"
                    .into(),
            );
        };
        let element = &blueprint.elements[index];
        let new_resource = resource(catalog, &element.material.parts[0].0).ok_or_else(|| {
            format!(
                "blueprint references unknown resource {}",
                element.material.parts[0].0
            )
        })?;
        let new_endpoints = blueprint_endpoint_options(new_resource);
        if new_endpoints.is_empty() {
            return Err(format!(
                "resource {} has no usable construction connection points",
                new_resource.name
            ));
        }

        let mut best_trial: Option<(f64, OrganismStructure, EnergyLedger, f64, usize)> = None;
        for &neighbor in &neighbors {
            let existing_index = realized_units[neighbor]
                .ok_or_else(|| "realized neighbor is missing".to_string())?;
            let existing_resource =
                resource(catalog, &blueprint.elements[neighbor].material.parts[0].0)
                    .ok_or_else(|| "blueprint references unknown neighbor resource".to_string())?;
            let existing_endpoints = blueprint_endpoint_options(existing_resource);
            for endpoint_a in existing_endpoints {
                let joint = endpoint_a
                    .world_point(&structure.units[existing_index], catalog)
                    .ok_or_else(|| "existing connection point has no world position".to_string())?;
                for endpoint_b in new_endpoints.iter().copied() {
                    let local_b = match endpoint_local_point(new_resource, endpoint_b, catalog) {
                        Some(v) => v,
                        None => continue,
                    };
                    let mut angles = Vec::with_capacity(361);
                    angles.push(element.placement.rotation_radians);
                    for step in 0..360 {
                        angles.push(std::f64::consts::TAU * step as f64 / 360.0);
                    }
                    for angle in angles {
                        let candidate_placement =
                            placement_for_joint(local_b, (joint.x, joint.y), angle);
                        let mut candidate_unit =
                            StructuralUnit::new(new_resource.name.clone(), candidate_placement);
                        if !candidate_unit.realize_default_geometry(catalog) {
                            continue;
                        }
                        if let Some(scaffold) = blueprint.genome_measurement.as_ref() {
                            if candidate_penetrates_measurement(
                                &candidate_unit,
                                scaffold,
                                genome_anchor,
                                catalog,
                            ) {
                                continue;
                            }
                        }
                        let mut trial = structure.clone();
                        let new_unit_index = trial.add_unit(candidate_unit);
                        if placed_unit_overlaps(
                            &trial,
                            &trial.units[new_unit_index],
                            &[new_unit_index, existing_index],
                            catalog,
                        ) {
                            continue;
                        }

                        let mut trial_ledger = *ledger;
                        let mut trial_energy = *energy;
                        let mut trial_heat = 0.0;
                        let mut all_bonds_ok = true;
                        let mut bond_cache = crate::contact::ConnectionCompatibilityCache::new();

                        for &neighbor_index in &neighbors {
                            let Some(existing) = realized_units[neighbor_index] else {
                                all_bonds_ok = false;
                                break;
                            };
                            let Some((a, b)) =
                                best_existing_connection(&trial, new_unit_index, existing, catalog)
                            else {
                                all_bonds_ok = false;
                                break;
                            };
                            let Some(attempt) = crate::combine_runtime::form_specific_bond(
                                &mut trial,
                                existing,
                                new_unit_index,
                                a,
                                b,
                                catalog,
                                &mut bond_cache,
                                &mut trial_ledger,
                                &mut trial_energy,
                            ) else {
                                all_bonds_ok = false;
                                break;
                            };
                            trial_heat += attempt.work_cost;
                        }
                        if !all_bonds_ok {
                            continue;
                        }
                        let score = blueprint_pose_score(candidate_placement, element.placement);
                        if best_trial.as_ref().is_none_or(|best| score < best.0) {
                            best_trial =
                                Some((score, trial, trial_ledger, trial_energy, new_unit_index));
                            // Keep heat with the selected trial, not globally across discarded angles.
                        }
                    }
                }
            }
        }
        let Some((_, trial, trial_ledger, trial_energy, new_unit_index)) = best_trial else {
            return Err(format!(
                "no valid bond-driven pose found for blueprint element {index}"
            ));
        };
        let committed_heat = trial_ledger.total_heat_dissipated - ledger.total_heat_dissipated;
        total_heat += committed_heat;
        structure = trial;
        *ledger = trial_ledger;
        *energy = trial_energy;
        realized[index] = true;
        realized_units[index] = Some(new_unit_index);
        progress += 1;
    }

    if blueprint.genome_measurement.is_some()
        && crate::cavity::analyze_genome_cavity(&structure, catalog)?.is_none()
    {
        return Err(
            "bond-driven scaffolded construction did not produce a qualifying final cavity".into(),
        );
    }
    Ok((structure, total_heat))
}
