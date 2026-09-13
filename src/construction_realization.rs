//! Physical realization of blueprint material intent.

use crate::contact::{connection_pair_candidates, try_add_bond};
use crate::resources::{BaseResource, ConnectionSites, Form, Material};
use crate::structural_blueprint::BlueprintElement;
use crate::structure::{
    Bond, BondEndpoint, ConnectionEndpoint, OrganismStructure, Placement, StructuralUnit,
};

const CONTACT_EPSILON: f64 = 1e-9;

fn resource<'a>(catalog: &'a [BaseResource], name: &str) -> Option<&'a BaseResource> {
    catalog.iter().find(|r| r.name == name)
}

fn endpoint_prototypes(unit: &StructuralUnit, catalog: &[BaseResource]) -> Vec<ConnectionEndpoint> {
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
        Some(ConnectionSites::Undetermined) | None => Vec::new(),
    }
}

fn local_endpoint_point(
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

fn rotate_point(point: (f64, f64), angle: f64) -> (f64, f64) {
    let (s, c) = angle.sin_cos();
    (point.0 * c - point.1 * s, point.0 * s + point.1 * c)
}

fn placement_for_point_contact(local: (f64, f64), target: (f64, f64), rotation: f64) -> Placement {
    let rotated = rotate_point(local, rotation);
    Placement {
        x: target.0 - rotated.0,
        y: target.1 - rotated.1,
        rotation_radians: rotation,
    }
}

fn add_unique_placement(out: &mut Vec<Placement>, placement: Placement) {
    if !placement.x.is_finite()
        || !placement.y.is_finite()
        || !placement.rotation_radians.is_finite()
    {
        return;
    }
    let normalized = Placement {
        x: placement.x,
        y: placement.y,
        rotation_radians: placement.rotation_radians.rem_euclid(std::f64::consts::TAU),
    };
    if !out.iter().any(|e| {
        (e.x - normalized.x).abs() <= CONTACT_EPSILON
            && (e.y - normalized.y).abs() <= CONTACT_EPSILON
            && (e.rotation_radians - normalized.rotation_radians).abs() <= 1e-10
    }) {
        out.push(normalized)
    }
}

#[derive(Clone, Copy, Debug)]
struct ContactTarget {
    unit_index: usize,
    endpoint: ConnectionEndpoint,
}

fn target_world_point(
    s: &OrganismStructure,
    t: ContactTarget,
    c: &[BaseResource],
) -> Option<(f64, f64)> {
    let u = s.units.get(t.unit_index)?;
    let p = t.endpoint.world_point(u, c)?;
    Some((p.x, p.y))
}

fn contact_targets_for_units(
    s: &OrganismStructure,
    indices: &[usize],
    c: &[BaseResource],
) -> Vec<ContactTarget> {
    let mut out = Vec::new();
    for &i in indices {
        let Some(u) = s.units.get(i) else { continue };
        for e in endpoint_prototypes(u, c) {
            out.push(ContactTarget {
                unit_index: i,
                endpoint: e,
            })
        }
    }
    out
}

fn circle_center(s: &OrganismStructure, i: usize, c: &[BaseResource]) -> Option<(f64, f64, f64)> {
    let u = s.units.get(i)?;
    let shape = u.shape(c)?;
    let Form::Circle { radius } = &shape.form else {
        return None;
    };
    Some((u.placement.x, u.placement.y, *radius))
}

fn add_circle_contact_candidates(
    out: &mut Vec<Placement>,
    resource: &BaseResource,
    targets: &[ContactTarget],
    s: &OrganismStructure,
    anchor: Placement,
    fixed_rotation: f64,
    c: &[BaseResource],
) {
    let Form::Circle { radius } = &resource.shape.form else {
        return;
    };
    let target_units = targets
        .iter()
        .map(|t| t.unit_index)
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    for &ti in &target_units {
        let Some((tx, ty, tr)) = circle_center(s, ti, c) else {
            continue;
        };
        let d = *radius + tr;
        if d <= CONTACT_EPSILON {
            continue;
        }
        let angle = (anchor.y - ty).atan2(anchor.x - tx);
        add_unique_placement(
            out,
            Placement {
                x: tx + d * angle.cos(),
                y: ty + d * angle.sin(),
                rotation_radians: fixed_rotation,
            },
        )
    }
    for ia in 0..target_units.len() {
        let Some((ax, ay, ar)) = circle_center(s, target_units[ia], c) else {
            continue;
        };
        for &bi in target_units.iter().skip(ia + 1) {
            let Some((bx, by, br)) = circle_center(s, bi, c) else {
                continue;
            };
            let dx = bx - ax;
            let dy = by - ay;
            let d = dx.hypot(dy);
            let ra = *radius + ar;
            let rb = *radius + br;
            if d <= CONTACT_EPSILON
                || d > ra + rb + CONTACT_EPSILON
                || d < (ra - rb).abs() - CONTACT_EPSILON
            {
                continue;
            }
            let along = (ra * ra - rb * rb + d * d) / (2.0 * d);
            let h2 = ra * ra - along * along;
            if h2 < -CONTACT_EPSILON {
                continue;
            }
            let h = h2.max(0.0).sqrt();
            let ux = dx / d;
            let uy = dy / d;
            let px = -uy * h;
            let py = ux * h;
            add_unique_placement(
                out,
                Placement {
                    x: ax + ux * along + px,
                    y: ay + uy * along + py,
                    rotation_radians: fixed_rotation,
                },
            );
            add_unique_placement(
                out,
                Placement {
                    x: ax + ux * along - px,
                    y: ay + uy * along - py,
                    rotation_radians: fixed_rotation,
                },
            )
        }
    }
}

fn candidate_placements_for_targets(
    resource: &BaseResource,
    targets: &[ContactTarget],
    s: &OrganismStructure,
    anchor: Placement,
    fixed_rotation: f64,
    c: &[BaseResource],
) -> Vec<Placement> {
    let temp = StructuralUnit::new(
        resource.name.clone(),
        Placement {
            x: 0.0,
            y: 0.0,
            rotation_radians: 0.0,
        },
    );
    let endpoints = endpoint_prototypes(&temp, c);
    let mut out = Vec::new();
    add_circle_contact_candidates(&mut out, resource, targets, s, anchor, fixed_rotation, c);
    for &target in targets {
        let Some(world) = target_world_point(s, target, c) else {
            continue;
        };
        for endpoint in &endpoints {
            let Some(local) = local_endpoint_point(resource, *endpoint, c) else {
                continue;
            };
            add_unique_placement(
                &mut out,
                placement_for_point_contact(local, world, fixed_rotation),
            )
        }
    }
    if targets.len() >= 2 && endpoints.len() >= 2 {
        let local_points = endpoints
            .iter()
            .filter_map(|e| local_endpoint_point(resource, *e, c))
            .collect::<Vec<_>>();
        for (ia, a) in targets.iter().enumerate() {
            let Some(wa) = target_world_point(s, *a, c) else {
                continue;
            };
            for b in targets.iter().skip(ia + 1) {
                let Some(wb) = target_world_point(s, *b, c) else {
                    continue;
                };
                for la in 0..local_points.len() {
                    for lb in (la + 1)..local_points.len() {
                        let pa = local_points[la];
                        let pb = local_points[lb];
                        let ldx = pb.0 - pa.0;
                        let ldy = pb.1 - pa.1;
                        let wdx = wb.0 - wa.0;
                        let wdy = wb.1 - wa.1;
                        let llen = ldx.hypot(ldy);
                        let wlen = wdx.hypot(wdy);
                        if llen <= CONTACT_EPSILON
                            || wlen <= CONTACT_EPSILON
                            || (llen - wlen).abs() > 1e-7 * llen.max(wlen).max(1.0)
                        {
                            continue;
                        }
                        let rotated = rotate_point(pa, fixed_rotation);
                        let expected_b = rotate_point(pb, fixed_rotation);
                        if (wa.0 + expected_b.0 - wb.0).hypot(wa.1 + expected_b.1 - wb.1) > 1e-7 {
                            continue;
                        }
                        add_unique_placement(
                            &mut out,
                            Placement {
                                x: wa.0 - rotated.0,
                                y: wa.1 - rotated.1,
                                rotation_radians: fixed_rotation,
                            },
                        )
                    }
                }
            }
        }
    }
    out.retain(|p| (p.rotation_radians - fixed_rotation).abs() <= 1e-10);

    // Blueprint placement is a soft target: physical candidates remain the
    // only admissible choices, but candidates closest to the inherited frame
    // are attempted first. This preserves constructional variation whenever
    // the preferred arrangement is unavailable.
    out.sort_by(|a, b| {
        let da = (a.x - anchor.x).hypot(a.y - anchor.y);
        let db = (b.x - anchor.x).hypot(b.y - anchor.y);
        da.partial_cmp(&db)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                let da = (a.rotation_radians - fixed_rotation).abs();
                let db = (b.rotation_radians - fixed_rotation).abs();
                da.partial_cmp(&db)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });
    add_unique_placement(
        &mut out,
        Placement {
            rotation_radians: fixed_rotation,
            ..anchor
        },
    );
    out
}

fn units_strictly_overlap(a: &StructuralUnit, b: &StructuralUnit, c: &[BaseResource]) -> bool {
    let (Some(sa), Some(sb)) = (a.shape(c), b.shape(c)) else {
        return false;
    };
    let pa = crate::material_geometry::PlacedMaterialPart {
        part_index: 0,
        form: sa.form.clone(),
        placement: a.placement,
    };
    let pb = crate::material_geometry::PlacedMaterialPart {
        part_index: 1,
        form: sb.form.clone(),
        placement: b.placement,
    };
    crate::material_geometry::placed_forms_penetrate(&pa, &pb, 0.0)
}

fn internal_contact_exists(s: &OrganismStructure, a: usize, b: usize, c: &[BaseResource]) -> bool {
    connection_pair_candidates(s, a, b, c)
        .into_iter()
        .any(|x| x.distance <= CONTACT_EPSILON)
}

fn partial_configuration_valid(
    s: &OrganismStructure,
    m: &Material,
    a: &[Option<usize>],
    c: &[BaseResource],
) -> bool {
    for bond in &m.internal_bonds {
        let (Some(x), Some(y)) = (a[bond.part_a], a[bond.part_b]) else {
            continue;
        };
        if !internal_contact_exists(s, x, y, c) {
            return false;
        }
    }
    true
}

fn choose_next_part(m: &Material, a: &[Option<usize>]) -> Option<usize> {
    let mut best = None;
    let mut score_best = i32::MIN;
    for p in 0..m.parts.len() {
        if a[p].is_some() {
            continue;
        }
        let n = m
            .internal_bonds
            .iter()
            .filter(|b| {
                (b.part_a == p && a[b.part_b].is_some())
                    || (b.part_b == p && a[b.part_a].is_some())
            })
            .count() as i32;
        let degree = m
            .internal_bonds
            .iter()
            .filter(|b| b.part_a == p || b.part_b == p)
            .count() as i32;
        let score = n * 100 + degree * 10 - p as i32;
        if score > score_best {
            score_best = score;
            best = Some(p)
        }
    }
    best
}

fn solve_material_placements(
    s: &OrganismStructure,
    e: &BlueprintElement,
    c: &[BaseResource],
    external: &[Vec<usize>],
) -> Result<Vec<Placement>, String> {
    let m = &e.material;
    let mut placements = vec![None; m.parts.len()];
    let mut assigned = vec![None; m.parts.len()];
    let anchor = Placement {
        x: e.placement.x,
        y: e.placement.y,
        rotation_radians: e.placement.rotation_radians,
    };

    fn search(
        base: &OrganismStructure,
        m: &Material,
        anchor: Placement,
        fixed_rotation: f64,
        placements: &mut [Option<Placement>],
        assigned: &mut [Option<usize>],
        external: &[Vec<usize>],
        c: &[BaseResource],
    ) -> bool {
        if assigned.iter().all(Option::is_some) {
            return true;
        }
        let Some(part) = choose_next_part(m, assigned) else {
            return false;
        };
        let Some(res) = resource(c, &m.parts[part].0) else {
            return false;
        };
        let mut working = base.clone();
        let mut wi = vec![None; m.parts.len()];
        for i in 0..m.parts.len() {
            let Some(p) = placements[i] else { continue };
            let mut u = StructuralUnit::new(m.parts[i].0.clone(), p);
            if !u.realize_default_geometry(c)
                || working
                    .units
                    .iter()
                    .any(|old| units_strictly_overlap(&u, old, c))
            {
                return false;
            }
            wi[i] = Some(working.add_unit(u))
        }
        let mut target_units = Vec::new();
        for bond in &m.internal_bonds {
            let other = if bond.part_a == part && assigned[bond.part_b].is_some() {
                wi[bond.part_b]
            } else if bond.part_b == part && assigned[bond.part_a].is_some() {
                wi[bond.part_a]
            } else {
                None
            };
            if let Some(i) = other {
                target_units.push(i)
            }
        }
        if assigned.iter().all(Option::is_none) {
            for constraint in external {
                target_units.extend(constraint.iter().copied())
            }
        }
        target_units.sort_unstable();
        target_units.dedup();
        let targets = contact_targets_for_units(&working, &target_units, c);
        let candidates =
            candidate_placements_for_targets(res, &targets, &working, anchor, fixed_rotation, c);
        for candidate in candidates {
            let mut trial = base.clone();
            let mut ti = vec![None; m.parts.len()];
            let mut valid = true;
            for i in 0..m.parts.len() {
                let p = if i == part {
                    Some(candidate)
                } else {
                    placements[i]
                };
                let Some(p) = p else { continue };
                let mut u = StructuralUnit::new(m.parts[i].0.clone(), p);
                if !u.realize_default_geometry(c)
                    || trial
                        .units
                        .iter()
                        .any(|old| units_strictly_overlap(&u, old, c))
                {
                    valid = false;
                    break;
                }
                ti[i] = Some(trial.add_unit(u))
            }
            if !valid || !partial_configuration_valid(&trial, m, &ti, c) {
                continue;
            }
            placements[part] = Some(candidate);
            assigned[part] = ti[part];
            if search(
                base,
                m,
                anchor,
                fixed_rotation,
                placements,
                assigned,
                external,
                c,
            ) {
                return true;
            }
            assigned[part] = None;
            placements[part] = None
        }
        false
    }

    if !search(
        s,
        m,
        anchor,
        e.placement.rotation_radians,
        &mut placements,
        &mut assigned,
        external,
        c,
    ) {
        return Err("no physically valid material realization satisfies its constraints".into());
    }
    placements
        .into_iter()
        .map(|p| p.ok_or_else(|| "material solver returned incomplete realization".to_string()))
        .collect()
}

fn connect_units(s: &mut OrganismStructure, a: usize, b: usize, c: &[BaseResource]) -> Option<f64> {
    let candidates = connection_pair_candidates(s, a, b, c);
    let id_a = s.physical_id(a)?;
    let id_b = s.physical_id(b)?;
    let pa = s.units.get(a)?.properties(c)?;
    let pb = s.units.get(b)?.properties(c)?;
    for candidate in candidates {
        if candidate.distance > CONTACT_EPSILON {
            continue;
        }
        let evaluation = crate::combine::evaluate_formation(candidate, pa.cohesion, pb.cohesion);
        let (_, work, _) = match crate::combine::required_investment(pa, pb, evaluation, 0.0) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let strength = crate::combine::bond_strength(pa, pb);
        if !strength.is_finite() || !(0.0..=1.0).contains(&strength) {
            continue;
        }
        let bond = Bond {
            endpoint_a: BondEndpoint::new(id_a, candidate.endpoint_a),
            endpoint_b: BondEndpoint::new(id_b, candidate.endpoint_b),
            strength,
            bond_energy: 0.0,
        };
        let mut trial = s.clone();
        if try_add_bond(&mut trial, bond, c).is_ok() {
            *s = trial;
            return Some(work);
        }
    }
    None
}

fn commit_material(
    s: &mut OrganismStructure,
    m: &Material,
    p: &[Placement],
    c: &[BaseResource],
) -> Result<Vec<usize>, String> {
    let mut trial = s.clone();
    let mut indices = Vec::with_capacity(p.len());
    for ((name, _), placement) in m.parts.iter().zip(p.iter()) {
        let mut u = StructuralUnit::new(name.clone(), *placement);
        if !u.realize_default_geometry(c) {
            return Err(format!("material references invalid resource {name}"));
        }
        if trial
            .units
            .iter()
            .any(|old| units_strictly_overlap(&u, old, c))
        {
            return Err("material realization contains strict constituent overlap".into());
        }
        indices.push(trial.add_unit(u))
    }
    for rel in &m.internal_bonds {
        let a = indices[rel.part_a];
        let b = indices[rel.part_b];
        if !internal_contact_exists(&trial, a, b, c) {
            return Err("material relationship could not be physically realized".into());
        }
        if connect_units(&mut trial, a, b, c).is_none() {
            return Err("material relationship could not be committed as a physical bond".into());
        }
    }
    *s = trial;
    Ok(indices)
}

pub(crate) fn realize_material(
    s: &mut OrganismStructure,
    e: &BlueprintElement,
    c: &[BaseResource],
) -> Result<Vec<usize>, String> {
    realize_material_with_constraints(s, e, c, &[])
}

pub(crate) fn realize_material_with_constraints(
    s: &mut OrganismStructure,
    e: &BlueprintElement,
    c: &[BaseResource],
    external: &[Vec<usize>],
) -> Result<Vec<usize>, String> {
    let m = &e.material;
    if !m.is_valid() || m.parts.is_empty() {
        return Err("invalid material intent".into());
    }
    if m.parts.iter().any(|(_, a)| (*a - 1.0).abs() > f64::EPSILON) {
        return Err("physical construction requires one unit per material constituent".into());
    }
    if m.parts.len() > 1 && !m.has_internal_structure() {
        return Err("multi-constituent material requires construction relationships".into());
    }
    if m.internal_bonds
        .iter()
        .any(|b| b.part_a >= m.parts.len() || b.part_b >= m.parts.len() || b.part_a == b.part_b)
    {
        return Err("material contains an invalid construction relationship".into());
    }
    if external.iter().any(|x| x.is_empty()) {
        return Err("material has an empty external construction constraint".into());
    }
    let placements = solve_material_placements(s, e, c, external)?;
    commit_material(s, m, &placements, c)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::{default_catalog, InternalBond};

    fn carbon_hydrogen_material() -> Material {
        Material {
            parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
            internal_bonds: vec![InternalBond {
                part_a: 0,
                part_b: 1,
            }],
        }
    }

    #[test]
    fn single_material_is_realized_at_anchor() {
        let c = default_catalog();
        let e = BlueprintElement {
            material: Material::free_base("Carbon", 1.0),
            placement: crate::structural_blueprint::BlueprintPlacement {
                x: 2.0,
                y: 3.0,
                rotation_radians: 0.0,
            },
        };
        let mut s = OrganismStructure::new();
        let ids = realize_material(&mut s, &e, &c).unwrap();
        assert_eq!(ids.len(), 1);
        assert_eq!(s.units[0].placement.x, 2.0);
        assert_eq!(s.units[0].placement.y, 3.0)
    }

    #[test]
    fn composite_realization_preserves_internal_contact() {
        let c = default_catalog();
        let e = BlueprintElement {
            material: carbon_hydrogen_material(),
            placement: crate::structural_blueprint::BlueprintPlacement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        };
        let mut s = OrganismStructure::new();
        let ids = realize_material(&mut s, &e, &c).unwrap();
        assert_eq!(ids.len(), 2);
        assert_eq!(s.bonds.len(), 1);
        assert!(connection_pair_candidates(&s, ids[0], ids[1], &c)
            .into_iter()
            .any(|x| x.distance <= CONTACT_EPSILON))
    }

    #[test]
    fn physical_candidates_are_ranked_by_blueprint_anchor() {
        let a = Placement {
            x: 10.0,
            y: 0.0,
            rotation_radians: 0.0,
        };
        let mut candidates = vec![
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
            Placement {
                x: 9.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
            Placement {
                x: 30.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ];
        candidates.sort_by(|x, y| {
            (x.x - a.x)
                .hypot(x.y - a.y)
                .partial_cmp(&(y.x - a.x).hypot(y.y - a.y))
                .unwrap()
        });
        assert_eq!(candidates[0].x, 9.0);
        assert_eq!(candidates[1].x, 0.0);
        assert_eq!(candidates[2].x, 30.0);
    }

    #[test]
    fn anchor_is_preference_not_unconditional_placement() {
        let anchor = Placement {
            x: 0.0,
            y: 0.0,
            rotation_radians: 0.0,
        };
        let candidate = Placement {
            x: 2.0,
            y: 0.0,
            rotation_radians: 0.0,
        };
        assert!(candidate.x != anchor.x || candidate.y != anchor.y);
    }
}
