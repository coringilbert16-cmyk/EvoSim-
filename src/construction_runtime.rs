//! Shared construction boundary: geometry may search, COMBINE creates bonds.

use crate::combine_runtime::combine_specific_pair;
use crate::resources::{BaseResource, ConnectionSites, Material};
use crate::state::EnergyLedger;
use crate::structural_blueprint::{BlueprintElement, BlueprintPlacement};
use crate::structure::{ConnectionEndpoint, OrganismStructure, Placement, StructuralUnit};

fn resource<'a>(catalog: &'a [BaseResource], name: &str) -> Option<&'a BaseResource> {
    catalog.iter().find(|r| r.name == name)
}

fn endpoints(unit: &StructuralUnit, catalog: &[BaseResource]) -> Vec<ConnectionEndpoint> {
    match unit.connection_sites(catalog) {
        Some(ConnectionSites::Corners(points)) => (0..points.len())
            .map(|i| ConnectionEndpoint::Corner { point_index: i })
            .collect(),
        Some(ConnectionSites::Endpoints(points)) => (0..points.len())
            .map(|i| ConnectionEndpoint::LineEndpoint { point_index: i })
            .collect(),
        Some(ConnectionSites::Circumference { .. }) => {
            vec![ConnectionEndpoint::Boundary { angle_radians: 0.0 }]
        }
        _ => Vec::new(),
    }
}

fn placement(value: BlueprintPlacement) -> Placement {
    Placement {
        x: value.x,
        y: value.y,
        rotation_radians: value.rotation_radians,
    }
}

fn candidate_placements(
    structure: &OrganismStructure,
    resource: &BaseResource,
    anchor: Placement,
    targets: &[usize],
    catalog: &[BaseResource],
) -> Vec<Placement> {
    let prototype = StructuralUnit::new(resource.name.clone(), anchor);
    let _ = endpoints(&prototype, catalog);
    let mut out = vec![anchor];

    if let Some(vertices) = resource.shape.form.polygon_vertices() {
        let min_x = vertices
            .iter()
            .map(|(x, _)| *x)
            .fold(f64::INFINITY, f64::min);
        let max_x = vertices
            .iter()
            .map(|(x, _)| *x)
            .fold(f64::NEG_INFINITY, f64::max);
        let min_y = vertices
            .iter()
            .map(|(_, y)| *y)
            .fold(f64::INFINITY, f64::min);
        let max_y = vertices
            .iter()
            .map(|(_, y)| *y)
            .fold(f64::NEG_INFINITY, f64::max);
        let width = max_x - min_x;
        let height = max_y - min_y;
        if width.is_finite() && width > 0.0 && height.is_finite() && height > 0.0 {
            out.extend([
                Placement {
                    x: anchor.x + width,
                    y: anchor.y,
                    rotation_radians: anchor.rotation_radians,
                },
                Placement {
                    x: anchor.x - width,
                    y: anchor.y,
                    rotation_radians: anchor.rotation_radians,
                },
                Placement {
                    x: anchor.x,
                    y: anchor.y + height,
                    rotation_radians: anchor.rotation_radians,
                },
                Placement {
                    x: anchor.x,
                    y: anchor.y - height,
                    rotation_radians: anchor.rotation_radians,
                },
            ]);
        }
    }

    for &target in targets {
        let Some(unit) = structure.units.get(target) else {
            continue;
        };
        let Some(sites) = unit.connection_sites(catalog) else {
            continue;
        };
        let target_endpoints = match sites {
            ConnectionSites::Corners(points) => (0..points.len())
                .map(|i| ConnectionEndpoint::Corner { point_index: i })
                .collect::<Vec<_>>(),
            ConnectionSites::Endpoints(points) => (0..points.len())
                .map(|i| ConnectionEndpoint::LineEndpoint { point_index: i })
                .collect::<Vec<_>>(),
            ConnectionSites::Circumference { .. } | ConnectionSites::Undetermined => Vec::new(),
        };
        for te in target_endpoints {
            let Some(tp) = te.world_point(unit, catalog) else {
                continue;
            };
            match resource.shape.connection_sites() {
                ConnectionSites::Corners(points) | ConnectionSites::Endpoints(points) => {
                    let rotations = [
                        anchor.rotation_radians,
                        anchor.rotation_radians + std::f64::consts::FRAC_PI_2,
                        anchor.rotation_radians + std::f64::consts::PI,
                        anchor.rotation_radians + 3.0 * std::f64::consts::FRAC_PI_2,
                    ];
                    for rotation in rotations {
                        let (s, c) = rotation.sin_cos();
                        for point in &points {
                            out.push(Placement {
                                x: tp.x - (point.x * c - point.y * s),
                                y: tp.y - (point.x * s + point.y * c),
                                rotation_radians: rotation,
                            });
                        }
                    }
                }
                ConnectionSites::Circumference { .. } => {
                    let radius = resource.shape.form.bounding_radius();
                    let length = tp.normal_x.hypot(tp.normal_y);
                    if length > 1e-12 && radius.is_finite() && radius > 0.0 {
                        let nx = tp.normal_x / length;
                        let ny = tp.normal_y / length;
                        out.push(Placement {
                            x: tp.x + nx * radius,
                            y: tp.y + ny * radius,
                            rotation_radians: anchor.rotation_radians,
                        });
                        out.push(Placement {
                            x: tp.x - nx * radius,
                            y: tp.y - ny * radius,
                            rotation_radians: anchor.rotation_radians,
                        });
                    }
                }
                ConnectionSites::Undetermined => {}
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
        let targets = neighbors(material, part, &assigned);
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
                placed = Some((
                    candidate,
                    candidate_ledger,
                    candidate_energy,
                    index,
                    candidate_heat,
                ));
                break;
            }
        }
        let Some((next, next_ledger, next_energy, index, step_heat)) = placed else {
            return Err("no physically valid construction candidate".into());
        };
        trial = next;
        trial_ledger = next_ledger;
        trial_energy = next_energy;
        assigned[part] = Some(index);
        heat += step_heat;
    }

    for group in external {
        let mut formed = false;
        for &a in assigned.iter().flatten() {
            for &b in group {
                let mut candidate = trial.clone();
                let mut candidate_ledger = trial_ledger;
                let mut candidate_energy = trial_energy;
                let mut cache = crate::contact::ConnectionCompatibilityCache::new();
                if let Some(attempt) = combine_specific_pair(
                    &mut candidate,
                    a,
                    b,
                    catalog,
                    0.0,
                    &mut cache,
                    &mut candidate_ledger,
                    &mut candidate_energy,
                ) {
                    trial = candidate;
                    trial_ledger = candidate_ledger;
                    trial_energy = candidate_energy;
                    heat += attempt.work_cost;
                    formed = true;
                    break;
                }
            }
            if formed {
                break;
            }
        }
        if !formed {
            return Err("external blueprint connection could not be realized".into());
        }
    }

    let ids = assigned.into_iter().flatten().collect::<Vec<_>>();
    *structure = trial;
    *ledger = trial_ledger;
    *energy = trial_energy;
    Ok((ids, heat))
}
