//! Physical realization of blueprint material intent.
//!
//! Construction is transactional: a material is solved in an isolated
//! placement state first, with its internal relationships and already-known
//! external blueprint contacts treated as physical constraints. Only a
//! complete solution is committed to the organism structure.

use crate::contact::{connection_pair_candidates, try_add_bond};
use crate::resources::{BaseResource, ConnectionSites, Form, Material};
use crate::structural_blueprint::BlueprintElement;
use crate::structure::{
    Bond, BondEndpoint, ConnectionEndpoint, OrganismStructure, Placement, StructuralUnit,
};

const CONTACT_EPSILON: f64 = 1e-9;
const ROTATION_SAMPLES: usize = 24;

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

fn placement_for_point_contact(
    local_point: (f64, f64),
    target: (f64, f64),
    rotation: f64,
) -> Placement {
    let rotated = rotate_point(local_point, rotation);
    Placement {
        x: target.0 - rotated.0,
        y: target.1 - rotated.1,
        rotation_radians: rotation,
    }
}

fn anchor_optimal_rotation(local_point: (f64, f64), target: (f64, f64), anchor: Placement) -> f64 {
    let vx = target.0 - anchor.x;
    let vy = target.1 - anchor.y;
    let v_len = vx.hypot(vy);
    let p_len = local_point.0.hypot(local_point.1);
    if v_len <= CONTACT_EPSILON || p_len <= CONTACT_EPSILON {
        return 0.0;
    }
    vy.atan2(vx) - local_point.1.atan2(local_point.0)
}

fn add_unique_placement(out: &mut Vec<Placement>, placement: Placement) {
    if !placement.x.is_finite()
        || !placement.y.is_finite()
        || !placement.rotation_radians.is_finite()
    {
        return;
    }
    let rotation = placement.rotation_radians.rem_euclid(std::f64::consts::TAU);
    let normalized = Placement {
        x: placement.x,
        y: placement.y,
        rotation_radians: rotation,
    };
    if !out.iter().any(|existing| {
        (existing.x - normalized.x).abs() <= CONTACT_EPSILON
            && (existing.y - normalized.y).abs() <= CONTACT_EPSILON
            && (existing.rotation_radians - normalized.rotation_radians).abs() <= 1e-10
    }) {
        out.push(normalized);
    }
}

fn two_point_rigid_placements(
    local_a: (f64, f64),
    target_a: (f64, f64),
    local_b: (f64, f64),
    target_b: (f64, f64),
) -> Option<Placement> {
    let local_dx = local_b.0 - local_a.0;
    let local_dy = local_b.1 - local_a.1;
    let target_dx = target_b.0 - target_a.0;
    let target_dy = target_b.1 - target_a.1;
    let local_len = local_dx.hypot(local_dy);
    let target_len = target_dx.hypot(target_dy);
    if local_len <= CONTACT_EPSILON || target_len <= CONTACT_EPSILON {
        return None;
    }
    if (local_len - target_len).abs() > 1e-7 * local_len.max(target_len).max(1.0) {
        return None;
    }
    let rotation = target_dy.atan2(target_dx) - local_dy.atan2(local_dx);
    let rotated_a = rotate_point(local_a, rotation);
    Some(Placement {
        x: target_a.0 - rotated_a.0,
        y: target_a.1 - rotated_a.1,
        rotation_radians: rotation,
    })
}

fn circle_centers_for_two_targets(
    radius: f64,
    target_a: (f64, f64),
    target_b: (f64, f64),
) -> Vec<Placement> {
    let dx = target_b.0 - target_a.0;
    let dy = target_b.1 - target_a.1;
    let distance = dx.hypot(dy);
    if distance <= CONTACT_EPSILON || distance > 2.0 * radius + CONTACT_EPSILON {
        return Vec::new();
    }
    let along = distance / 2.0;
    let height_sq = radius * radius - along * along;
    if height_sq < -CONTACT_EPSILON {
        return Vec::new();
    }
    let height = height_sq.max(0.0).sqrt();
    let ux = dx / distance;
    let uy = dy / distance;
    let px = -uy * height;
    let py = ux * height;
    vec![
        Placement {
            x: target_a.0 + ux * along + px,
            y: target_a.1 + uy * along + py,
            rotation_radians: 0.0,
        },
        Placement {
            x: target_a.0 + ux * along - px,
            y: target_a.1 + uy * along - py,
            rotation_radians: 0.0,
        },
    ]
}

#[derive(Clone, Copy, Debug)]
struct ContactTarget {
    unit_index: usize,
    endpoint: ConnectionEndpoint,
}

fn target_world_point(
    structure: &OrganismStructure,
    target: ContactTarget,
    catalog: &[BaseResource],
) -> Option<(f64, f64)> {
    let unit = structure.units.get(target.unit_index)?;
    let point = target.endpoint.world_point(unit, catalog)?;
    Some((point.x, point.y))
}

fn contact_targets_for_unit(
    structure: &OrganismStructure,
    unit_indices: &[usize],
    catalog: &[BaseResource],
) -> Vec<ContactTarget> {
    let mut targets = Vec::new();
    for &unit_index in unit_indices {
        let Some(unit) = structure.units.get(unit_index) else {
            continue;
        };
        for endpoint in endpoint_prototypes(unit, catalog) {
            targets.push(ContactTarget { unit_index, endpoint });
        }
    }
    targets
}

fn candidate_placements_for_target(
    resource: &BaseResource,
    target: ContactTarget,
    structure: &OrganismStructure,
    anchor: Placement,
    catalog: &[BaseResource],
) -> Vec<Placement> {
    let Some(target_point) = target_world_point(structure, target, catalog) else {
        return Vec::new();
    };
    let temp = StructuralUnit::new(
        resource.name.clone(),
        Placement {
            x: 0.0,
            y: 0.0,
            rotation_radians: 0.0,
        },
    );
    let mut out = Vec::new();
    for endpoint in endpoint_prototypes(&temp, catalog) {
        let Some(local) = local_endpoint_point(resource, endpoint, catalog) else {
            continue;
        };
        let optimal = anchor_optimal_rotation(local, target_point, anchor);
        add_unique_placement(
            &mut out,
            placement_for_point_contact(local, target_point, optimal),
        );
        for sample in 0..ROTATION_SAMPLES {
            let rotation = std::f64::consts::TAU * sample as f64 / ROTATION_SAMPLES as f64;
            add_unique_placement(
                &mut out,
                placement_for_point_contact(local, target_point, rotation),
            );
        }
    }
    out
}

fn candidate_placements_for_targets(
    resource: &BaseResource,
    targets: &[ContactTarget],
    structure: &OrganismStructure,
    anchor: Placement,
    catalog: &[BaseResource],
) -> Vec<Placement> {
    let mut out = Vec::new();
    for &target in targets {
        out.extend(candidate_placements_for_target(
            resource, target, structure, anchor, catalog,
        ));
    }

    if targets.len() >= 2 {
        let temp = StructuralUnit::new(
            resource.name.clone(),
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        );
        let endpoints = endpoint_prototypes(&temp, catalog);
        let local_points = endpoints
            .into_iter()
            .filter_map(|endpoint| local_endpoint_point(resource, endpoint, catalog))
            .collect::<Vec<_>>();

        for (target_a_index, target_a) in targets.iter().enumerate() {
            let Some(world_a) = target_world_point(structure, *target_a, catalog) else {
                continue;
            };
            for target_b in targets.iter().skip(target_a_index + 1) {
                let Some(world_b) = target_world_point(structure, *target_b, catalog) else {
                    continue;
                };
                for (local_a_index, local_a) in local_points.iter().enumerate() {
                    for local_b in local_points.iter().skip(local_a_index + 1) {
                        if let Some(placement) = two_point_rigid_placements(
                            *local_a, world_a, *local_b, world_b,
                        ) {
                            add_unique_placement(&mut out, placement);
                        }
                    }
                }
                if let Form::Circle { radius } = &resource.shape.form {
                    for placement in circle_centers_for_two_targets(*radius, world_a, world_b) {
                        add_unique_placement(&mut out, placement);
                    }
                }
            }
        }
    }
    out
}

fn units_strictly_overlap(
    a: &StructuralUnit,
    b: &StructuralUnit,
    catalog: &[BaseResource],
) -> bool {
    let (Some(shape_a), Some(shape_b)) = (a.shape(catalog), b.shape(catalog)) else {
        return false;
    };
    let pa = crate::material_geometry::PlacedMaterialPart {
        part_index: 0,
        form: shape_a.form.clone(),
        placement: a.placement,
    };
    let pb = crate::material_geometry::PlacedMaterialPart {
        part_index: 1,
        form: shape_b.form.clone(),
        placement: b.placement,
    };
    if !crate::material_geometry::placed_forms_overlap(&pa, &pb, 0.0) {
        return false;
    }
    let dx = b.placement.x - a.placement.x;
    let dy = b.placement.y - a.placement.y;
    let distance = dx.hypot(dy);
    let (sx, sy) = if distance > CONTACT_EPSILON {
        (dx / distance, dy / distance)
    } else {
        (1.0, 0.0)
    };
    let scale = pa.form.bounding_radius().max(pb.form.bounding_radius()).max(1.0);
    let epsilon = 1e-8 * scale;
    let shifted = crate::material_geometry::PlacedMaterialPart {
        part_index: 0,
        form: pa.form,
        placement: Placement {
            x: a.placement.x - sx * epsilon,
            y: a.placement.y - sy * epsilon,
            rotation_radians: a.placement.rotation_radians,
        },
    };
    crate::material_geometry::placed_forms_overlap(&shifted, &pb, 0.0)
}

fn internal_contact_exists(
    structure: &OrganismStructure,
    a: usize,
    b: usize,
    catalog: &[BaseResource],
) -> bool {
    connection_pair_candidates(structure, a, b, catalog)
        .into_iter()
        .any(|candidate| candidate.distance <= CONTACT_EPSILON)
}

fn external_constraint_satisfied(
    structure: &OrganismStructure,
    material_indices: &[usize],
    external_units: &[usize],
    catalog: &[BaseResource],
) -> bool {
    material_indices.iter().copied().any(|material_index| {
        external_units.iter().copied().any(|external_index| {
            internal_contact_exists(structure, material_index, external_index, catalog)
        })
    })
}

fn partial_configuration_valid(
    structure: &OrganismStructure,
    material: &Material,
    assigned: &[Option<usize>],
    catalog: &[BaseResource],
) -> bool {
    for bond in &material.internal_bonds {
        let (Some(a), Some(b)) = (assigned[bond.part_a], assigned[bond.part_b]) else {
            continue;
        };
        if !internal_contact_exists(structure, a, b, catalog) {
            return false;
        }
    }
    true
}

fn choose_next_part(
    material: &Material,
    assigned: &[Option<usize>],
    external_constraints: &[Vec<usize>],
) -> Option<usize> {
    let mut best = None;
    let mut best_score = i32::MIN;
    for part in 0..material.parts.len() {
        if assigned[part].is_some() {
            continue;
        }
        let assigned_neighbors = material
            .internal_bonds
            .iter()
            .filter(|bond| {
                (bond.part_a == part && assigned[bond.part_b].is_some())
                    || (bond.part_b == part && assigned[bond.part_a].is_some())
            })
            .count() as i32;
        let external_relevance = if external_constraints.is_empty() {
            0
        } else {
            external_constraints.len() as i32
        };
        let score = assigned_neighbors * 100 + external_relevance - part as i32;
        if score > best_score {
            best_score = score;
            best = Some(part);
        }
    }
    best
}

fn solve_material_placements(
    structure: &OrganismStructure,
    element: &BlueprintElement,
    catalog: &[BaseResource],
    external_constraints: &[Vec<usize>],
) -> Result<Vec<Placement>, String> {
    let material = &element.material;
    let mut placements = vec![None; material.parts.len()];
    let mut assigned = vec![None; material.parts.len()];
    let anchor = Placement {
        x: element.placement.x,
        y: element.placement.y,
        rotation_radians: 0.0,
    };

    fn search(
        base_structure: &OrganismStructure,
        material: &Material,
        anchor: Placement,
        placements: &mut [Option<Placement>],
        assigned: &mut [Option<usize>],
        external_constraints: &[Vec<usize>],
        catalog: &[BaseResource],
    ) -> bool {
        if assigned.iter().all(Option::is_some) {
            let mut final_structure = base_structure.clone();
            let mut indices = Vec::with_capacity(material.parts.len());
            for i in 0..material.parts.len() {
                let Some(placement) = placements[i] else {
                    return false;
                };
                let mut unit = StructuralUnit::new(material.parts[i].0.clone(), placement);
                if !unit.realize_default_geometry(catalog) {
                    return false;
                }
                for previous_index in 0..final_structure.units.len() {
                    if units_strictly_overlap(
                        &unit,
                        &final_structure.units[previous_index],
                        catalog,
                    ) {
                        return false;
                    }
                }
                indices.push(final_structure.add_unit(unit));
            }
            for relationship in &material.internal_bonds {
                let a = indices[relationship.part_a];
                let b = indices[relationship.part_b];
                if connect_units(&mut final_structure, a, b, catalog).is_none() {
                    return false;
                }
            }
            for constraint in external_constraints {
                let mut connected = false;
                'pair: for &a in &indices {
                    for &b in constraint {
                        if connect_units(&mut final_structure, a, b, catalog).is_some() {
                            connected = true;
                            break 'pair;
                        }
                    }
                }
                if !connected {
                    return false;
                }
            }
            return true;
        }

        let Some(part) = choose_next_part(material, assigned, external_constraints) else {
            return false;
        };
        let name = &material.parts[part].0;
        let Some(base_resource) = resource(catalog, name) else {
            return false;
        };

        let mut working = base_structure.clone();
        let mut working_indices = vec![None; material.parts.len()];
        for i in 0..material.parts.len() {
            let Some(placement) = placements[i] else {
                continue;
            };
            let mut unit = StructuralUnit::new(material.parts[i].0.clone(), placement);
            if !unit.realize_default_geometry(catalog) {
                return false;
            }
            working_indices[i] = Some(working.add_unit(unit));
        }

        let mut target_units = material
            .internal_bonds
            .iter()
            .filter_map(|bond| {
                if bond.part_a == part && assigned[bond.part_b].is_some() {
                    working_indices[bond.part_b]
                } else if bond.part_b == part && assigned[bond.part_a].is_some() {
                    working_indices[bond.part_a]
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        for constraint in external_constraints {
            target_units.extend(constraint.iter().copied());
        }
        target_units.sort_unstable();
        target_units.dedup();

        let mut candidates = Vec::new();
        if part == 0 {
            // The blueprint fixes the anchor position, not orientation. Keep
            // neutral orientation as a valid option and add contact-compatible
            // rotations without ever moving the anchor.
            candidates.push(anchor);
        }
        if !target_units.is_empty() {
            let targets = contact_targets_for_unit(&working, &target_units, catalog);
            let mut contact_candidates = candidate_placements_for_targets(
                base_resource,
                &targets,
                &working,
                anchor,
                catalog,
            );
            if part == 0 {
                contact_candidates.retain(|candidate| {
                    (candidate.x - anchor.x).abs() <= 1e-7
                        && (candidate.y - anchor.y).abs() <= 1e-7
                });
            }
            candidates.extend(contact_candidates);
        }

        candidates.sort_by(|a, b| {
            let da = (a.x - anchor.x).hypot(a.y - anchor.y);
            let db = (b.x - anchor.x).hypot(b.y - anchor.y);
            da.total_cmp(&db)
                .then_with(|| a.rotation_radians.total_cmp(&b.rotation_radians))
        });

        for candidate in candidates {
            let mut trial = base_structure.clone();
            let mut trial_indices = vec![None; material.parts.len()];
            let mut valid = true;
            for i in 0..material.parts.len() {
                let placement = if i == part { Some(candidate) } else { placements[i] };
                let Some(placement) = placement else {
                    continue;
                };
                let mut unit = StructuralUnit::new(material.parts[i].0.clone(), placement);
                if !unit.realize_default_geometry(catalog) {
                    valid = false;
                    break;
                }
                for previous_index in 0..trial.units.len() {
                    if units_strictly_overlap(&unit, &trial.units[previous_index], catalog) {
                        valid = false;
                        break;
                    }
                }
                if !valid {
                    break;
                }
                let new_index = trial.add_unit(unit);
                trial_indices[i] = Some(new_index);
            }
            if !valid {
                continue;
            }
            if !partial_configuration_valid(&trial, material, &trial_indices, catalog) {
                continue;
            }

            placements[part] = Some(candidate);
            assigned[part] = trial_indices[part];
            if search(
                base_structure,
                material,
                anchor,
                placements,
                assigned,
                external_constraints,
                catalog,
            ) {
                return true;
            }
            assigned[part] = None;
            placements[part] = None;
        }
        false
    }

    if !search(
        structure,
        material,
        anchor,
        &mut placements,
        &mut assigned,
        external_constraints,
        catalog,
    ) {
        return Err("no physically valid material realization satisfies its constraints".into());
    }

    placements
        .into_iter()
        .map(|placement| {
            placement.ok_or_else(|| "material solver returned incomplete realization".to_string())
        })
        .collect()
}

fn connect_units(
    structure: &mut OrganismStructure,
    a: usize,
    b: usize,
    catalog: &[BaseResource],
) -> Option<f64> {
    let candidates = connection_pair_candidates(structure, a, b, catalog);
    let id_a = structure.physical_id(a)?;
    let id_b = structure.physical_id(b)?;
    let pa = structure.units.get(a)?.properties(catalog)?;
    let pb = structure.units.get(b)?.properties(catalog)?;
    for candidate in candidates {
        if candidate.distance > CONTACT_EPSILON {
            continue;
        }
        let evaluation = crate::combine::evaluate_formation(candidate, pa.cohesion, pb.cohesion);
        let (_, work, _) = crate::combine::required_investment(pa, pb, evaluation, 0.0).ok()?;
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
        let mut trial = structure.clone();
        if try_add_bond(&mut trial, bond, catalog).is_ok() {
            *structure = trial;
            return Some(work);
        }
    }
    None
}

fn commit_material(
    structure: &mut OrganismStructure,
    material: &Material,
    placements: &[Placement],
    catalog: &[BaseResource],
    external_constraints: &[Vec<usize>],
) -> Result<Vec<usize>, String> {
    let mut trial = structure.clone();
    let mut indices = Vec::with_capacity(placements.len());
    for ((name, _), placement) in material.parts.iter().zip(placements.iter()) {
        let mut unit = StructuralUnit::new(name.clone(), *placement);
        if !unit.realize_default_geometry(catalog) {
            return Err(format!("material references invalid resource {name}"));
        }
        for previous_index in 0..trial.units.len() {
            if units_strictly_overlap(&unit, &trial.units[previous_index], catalog) {
                return Err("material realization contains strict constituent overlap".into());
            }
        }
        indices.push(trial.add_unit(unit));
    }

    for relationship in &material.internal_bonds {
        let a = indices[relationship.part_a];
        let b = indices[relationship.part_b];
        if connection_pair_candidates(&trial, a, b, catalog)
            .into_iter()
            .all(|candidate| candidate.distance > CONTACT_EPSILON)
        {
            return Err("material relationship could not be physically realized".into());
        }
        if connect_units(&mut trial, a, b, catalog).is_none() {
            return Err("material relationship could not be committed as a physical bond".into());
        }
    }

    // Blueprint connections are physical bonds. Resolve all required external
    // contacts on the same trial graph so this material commit is atomic.
    for constraint in external_constraints {
        let mut connected = false;
        'pair: for &a in &indices {
            for &b in constraint {
                if connect_units(&mut trial, a, b, catalog).is_some() {
                    connected = true;
                    break 'pair;
                }
            }
        }
        if !connected {
            return Err("required blueprint contact could not be committed as a physical bond".into());
        }
    }

    *structure = trial;
    Ok(indices)
}

/// Compatibility entry point for callers that have no external blueprint
/// constraints (for example, reproduction of a material in isolation).
pub(crate) fn realize_material(
    structure: &mut OrganismStructure,
    element: &BlueprintElement,
    catalog: &[BaseResource],
) -> Result<Vec<usize>, String> {
    realize_material_with_constraints(structure, element, catalog, &[])
}

/// Solve and commit one material while respecting already-realized physical
/// neighbors represented by `external_constraints`. Each entry is one required
/// blueprint connection and contains the physical constituents of the already-
/// realized neighboring element; at least one constituent contact must satisfy it.
pub(crate) fn realize_material_with_constraints(
    structure: &mut OrganismStructure,
    element: &BlueprintElement,
    catalog: &[BaseResource],
    external_constraints: &[Vec<usize>],
) -> Result<Vec<usize>, String> {
    let material = &element.material;
    if !material.is_valid() || material.parts.is_empty() {
        return Err("invalid material intent".into());
    }
    if material
        .parts
        .iter()
        .any(|(_, amount)| (*amount - 1.0).abs() > f64::EPSILON)
    {
        return Err("physical construction requires one unit per material constituent".into());
    }
    if material.parts.len() > 1 && !material.has_internal_structure() {
        return Err("multi-constituent material requires construction relationships".into());
    }
    if material.internal_bonds.iter().any(|b| {
        b.part_a >= material.parts.len() || b.part_b >= material.parts.len() || b.part_a == b.part_b
    }) {
        return Err("material contains an invalid construction relationship".into());
    }
    if external_constraints.iter().any(|constraint| constraint.is_empty()) {
        return Err("material has an empty external construction constraint".into());
    }

    let placements = solve_material_placements(structure, element, catalog, external_constraints)?;
    commit_material(
        structure,
        material,
        &placements,
        catalog,
        external_constraints,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::{default_catalog, InternalBond};

    fn carbon_hydrogen_material() -> Material {
        Material {
            parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
            internal_bonds: vec![InternalBond { part_a: 0, part_b: 1 }],
        }
    }

    #[test]
    fn single_material_is_realized_at_anchor() {
        let catalog = default_catalog();
        let element = BlueprintElement {
            material: Material::free_base("Carbon", 1.0),
            placement: crate::structural_blueprint::BlueprintPlacement { x: 2.0, y: 3.0 },
        };
        let mut structure = OrganismStructure::new();
        let ids = realize_material(&mut structure, &element, &catalog).unwrap();
        assert_eq!(ids.len(), 1);
        assert_eq!(structure.units[0].placement.x, 2.0);
        assert_eq!(structure.units[0].placement.y, 3.0);
    }

    #[test]
    fn composite_realization_preserves_internal_contact() {
        let catalog = default_catalog();
        let element = BlueprintElement {
            material: carbon_hydrogen_material(),
            placement: crate::structural_blueprint::BlueprintPlacement { x: 0.0, y: 0.0 },
        };
        let mut structure = OrganismStructure::new();
        let ids = realize_material(&mut structure, &element, &catalog).unwrap();
        assert_eq!(ids.len(), 2);
        assert_eq!(structure.bonds.len(), 1);
        assert!(connection_pair_candidates(&structure, ids[0], ids[1], &catalog)
            .into_iter()
            .any(|candidate| candidate.distance <= CONTACT_EPSILON));
    }
}
