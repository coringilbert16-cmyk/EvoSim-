use crate::combine_runtime::combine_specific_pair;
use crate::resources::{BaseResource, Form, Material};
use crate::state::EnergyLedger;
use crate::structural_blueprint::{BlueprintElement, BlueprintPlacement};
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
                    let half = candidate_length.to_owned() / 2.0;
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

pub(crate) fn realize_material_with_context(
    structure: &mut OrganismStructure,
    element: &BlueprintElement,
    catalog: &[BaseResource],
    ledger: &mut EnergyLedger,
    energy: &mut f64,
    external: &[Vec<usize>],
) -> Result<(Vec<usize>, f64), String> {
    let material = &element.material;
    let anchor = placement(element.placement);
    let mut trial = structure.clone();
    let mut trial_ledger = *ledger;
    let mut trial_energy = *energy;
    let mut assigned = vec![None; material.parts.len()];
    let mut heat = 0.0;
    for part in 0..material.parts.len() {
        let resource =
            resource(catalog, &material.parts[part].0).ok_or("invalid construction resource")?;
        let mut targets = neighbors(material, part, &assigned);
        for group in external {
            for &target in group {
                if !targets.contains(&target) {
                    targets.push(target);
                }
            }
        }
        let mut placed = None;
        for candidate_placement in candidate_placements(&trial, resource, anchor, &targets, catalog)
        {
            let mut candidate = trial.clone();
            let mut candidate_ledger = trial_ledger;
            let mut candidate_energy = trial_energy;
            let mut unit = StructuralUnit::new(resource.name.clone(), candidate_placement);
            if !unit.realize_default_geometry(catalog) {
                continue;
            }
            let index = candidate.add_unit(unit);
            let mut candidate_assigned = assigned.clone();
            candidate_assigned[part] = Some(index);
            let mut cache = crate::contact::ConnectionCompatibilityCache::new();
            let mut candidate_heat = 0.0;
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
            if ok {
                for group in external {
                    let mut formed = false;
                    for &target in group {
                        if let Some(attempt) = combine_specific_pair(
                            &mut candidate,
                            index,
                            target,
                            catalog,
                            0.0,
                            &mut cache,
                            &mut candidate_ledger,
                            &mut candidate_energy,
                        ) {
                            candidate_heat += attempt.work_cost;
                            formed = true;
                            break;
                        }
                    }
                    if !formed {
                        ok = false;
                        break;
                    }
                }
            }
            if ok {
                trial = candidate;
                trial_ledger = candidate_ledger;
                trial_energy = candidate_energy;
                assigned = candidate_assigned;
                heat = candidate_heat;
                placed = Some(index);
                break;
            }
        }
        if placed.is_none() {
            return Err("construction placement could not satisfy physical constraints".into());
        }
    }
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
