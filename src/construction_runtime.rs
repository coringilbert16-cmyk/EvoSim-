#![expect(dead_code, reason = "Staged API retained for subsystem integration")]
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
);

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
                0.0,
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
    heat: f64,
    fixed_placement: bool,
) -> Option<ConstructionSolution> {
    if part == material.parts.len() {
        let (structure, ledger, energy, heat) = solve_external_groups(
            0, structure, ledger, energy, assigned, external, catalog, heat,
        )?;
        return Some((structure, ledger, energy, assigned.to_vec(), heat));
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

    let candidates = candidate_placements(structure, resource, anchor, &targets, catalog);
    let candidates = if fixed_placement {
        candidates.into_iter().take(1).collect()
    } else {
        candidates
    };
    for candidate_placement in candidates {
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
                0.0,
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
            candidate_heat,
            fixed_placement,
        ) {
            return Some(result);
        }
    }
    None
}

pub(crate) fn realize_material_with_context(
    structure: &mut OrganismStructure,
    element: &BlueprintElement,
    catalog: &[BaseResource],
    ledger: &mut EnergyLedger,
    energy: &mut f64,
    external: &[Vec<usize>],
) -> Result<(Vec<usize>, f64), String> {
    realize_material_with_context_mode(
        structure, element, catalog, ledger, energy, external, false,
    )
}

pub(crate) fn realize_material_at_fixed_placement_with_context(
    structure: &mut OrganismStructure,
    element: &BlueprintElement,
    catalog: &[BaseResource],
    ledger: &mut EnergyLedger,
    energy: &mut f64,
    external: &[Vec<usize>],
) -> Result<(Vec<usize>, f64), String> {
    realize_material_with_context_mode(
        structure, element, catalog, ledger, energy, external, true,
    )
}

fn realize_material_with_context_mode(
    structure: &mut OrganismStructure,
    element: &BlueprintElement,
    catalog: &[BaseResource],
    ledger: &mut EnergyLedger,
    energy: &mut f64,
    external: &[Vec<usize>],
    fixed_placement: bool,
) -> Result<(Vec<usize>, f64), String> {
    let material = &element.material;
    let anchor = placement(element.placement);
    if material.parts.is_empty() {
        return Err("construction material must contain at least one constituent".into());
    }

    let assigned = vec![None; material.parts.len()];
    let Some((trial, trial_ledger, trial_energy, assigned, heat)) = solve_parts(
        0, structure, ledger, *energy, &assigned, material, anchor, catalog, external, 0.0,
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
    realize_material_with_context(structure, &element, catalog, ledger, energy, &[])
        .map(|(indices, _)| indices)
}
