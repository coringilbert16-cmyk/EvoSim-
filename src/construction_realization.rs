//! Physical realization of blueprint material intent.
//!
//! Material realization is deliberately local. A material contains a small
//! number of constituents, so its geometry is solved from its own physical
//! relationships. Blueprint-neighbor contacts are constraints on the finished
//! material, not a second global placement search.

use crate::contact::{connection_pair_candidates, try_add_bond};
use crate::resources::{BaseResource, ConnectionSites, Form, Material};
use crate::structural_blueprint::BlueprintElement;
use crate::structure::{Bond, BondEndpoint, ConnectionEndpoint, OrganismStructure, Placement, StructuralUnit};

const CONTACT_EPSILON: f64 = 1e-9;

fn resource<'a>(catalog: &'a [BaseResource], name: &str) -> Option<&'a BaseResource> { catalog.iter().find(|r| r.name == name) }

fn endpoint_prototypes(unit: &StructuralUnit, catalog: &[BaseResource]) -> Vec<ConnectionEndpoint> {
    match unit.connection_sites(catalog) {
        Some(ConnectionSites::Corners(points)) => (0..points.len()).map(|i| ConnectionEndpoint::Corner { point_index: i }).collect(),
        Some(ConnectionSites::Endpoints(points)) => (0..points.len()).map(|i| ConnectionEndpoint::LineEndpoint { point_index: i }).collect(),
        Some(ConnectionSites::Circumference { .. }) => vec![ConnectionEndpoint::Boundary { angle_radians: 0.0 }],
        Some(ConnectionSites::Undetermined) | None => Vec::new(),
    }
}

fn local_endpoint_point(resource: &BaseResource, endpoint: ConnectionEndpoint, catalog: &[BaseResource]) -> Option<(f64, f64)> {
    let unit = StructuralUnit::new(resource.name.clone(), Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 });
    let point = endpoint.world_point(&unit, catalog)?;
    Some((point.x, point.y))
}

fn rotate_point(point: (f64, f64), angle: f64) -> (f64, f64) {
    let (s, c) = angle.sin_cos();
    (point.0 * c - point.1 * s, point.0 * s + point.1 * c)
}

fn placement_for_point_contact(local: (f64, f64), target: (f64, f64), rotation: f64) -> Placement {
    let rotated = rotate_point(local, rotation);
    Placement { x: target.0 - rotated.0, y: target.1 - rotated.1, rotation_radians: rotation }
}

fn anchor_rotation(local: (f64, f64), target: (f64, f64), anchor: Placement) -> Option<f64> {
    let vx = target.0 - anchor.x;
    let vy = target.1 - anchor.y;
    let v_len = vx.hypot(vy);
    let p_len = local.0.hypot(local.1);
    if v_len <= CONTACT_EPSILON || p_len <= CONTACT_EPSILON { return Some(0.0); }
    Some(vy.atan2(vx) - local.1.atan2(local.0))
}

fn add_unique_placement(out: &mut Vec<Placement>, placement: Placement) {
    if !placement.x.is_finite() || !placement.y.is_finite() || !placement.rotation_radians.is_finite() { return; }
    let normalized = Placement { x: placement.x, y: placement.y, rotation_radians: placement.rotation_radians.rem_euclid(std::f64::consts::TAU) };
    if !out.iter().any(|existing| (existing.x - normalized.x).abs() <= CONTACT_EPSILON && (existing.y - normalized.y).abs() <= CONTACT_EPSILON && (existing.rotation_radians - normalized.rotation_radians).abs() <= 1e-10) { out.push(normalized); }
}

#[derive(Clone, Copy, Debug)]
struct ContactTarget { unit_index: usize, endpoint: ConnectionEndpoint }

fn target_world_point(structure: &OrganismStructure, target: ContactTarget, catalog: &[BaseResource]) -> Option<(f64, f64)> {
    let unit = structure.units.get(target.unit_index)?;
    let point = target.endpoint.world_point(unit, catalog)?;
    Some((point.x, point.y))
}

fn contact_targets_for_units(structure: &OrganismStructure, unit_indices: &[usize], catalog: &[BaseResource]) -> Vec<ContactTarget> {
    let mut targets = Vec::new();
    for &unit_index in unit_indices {
        let Some(unit) = structure.units.get(unit_index) else { continue };
        for endpoint in endpoint_prototypes(unit, catalog) { targets.push(ContactTarget { unit_index, endpoint }); }
    }
    targets
}

/// Generate only analytically determined placements. There is no arbitrary
/// rotation sweep and no global search over the organism.
fn candidate_placements_for_targets(resource: &BaseResource, targets: &[ContactTarget], structure: &OrganismStructure, anchor: Placement, anchored: bool, catalog: &[BaseResource]) -> Vec<Placement> {
    let temp = StructuralUnit::new(resource.name.clone(), Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 });
    let endpoints = endpoint_prototypes(&temp, catalog);
    let mut out = Vec::new();

    for &target in targets {
        let Some(world_target) = target_world_point(structure, target, catalog) else { continue };
        for endpoint in &endpoints {
            let Some(local) = local_endpoint_point(resource, *endpoint, catalog) else { continue };
            if anchored {
                let Some(rotation) = anchor_rotation(local, world_target, anchor) else { continue };
                let rotated = rotate_point(local, rotation);
                if (anchor.x + rotated.0 - world_target.0).abs() <= 1e-7 && (anchor.y + rotated.1 - world_target.1).abs() <= 1e-7 {
                    add_unique_placement(&mut out, Placement { x: anchor.x, y: anchor.y, rotation_radians: rotation });
                }
            } else {
                add_unique_placement(&mut out, placement_for_point_contact(local, world_target, 0.0));
            }
        }
    }

    if !anchored && targets.len() >= 2 && endpoints.len() >= 2 {
        let local_points = endpoints.iter().filter_map(|e| local_endpoint_point(resource, *e, catalog)).collect::<Vec<_>>();
        for (ia, target_a) in targets.iter().enumerate() {
            let Some(world_a) = target_world_point(structure, *target_a, catalog) else { continue };
            for target_b in targets.iter().skip(ia + 1) {
                let Some(world_b) = target_world_point(structure, *target_b, catalog) else { continue };
                for la in 0..local_points.len() {
                    for lb in (la + 1)..local_points.len() {
                        let lp_a = local_points[la]; let lp_b = local_points[lb];
                        let ldx = lp_b.0 - lp_a.0; let ldy = lp_b.1 - lp_a.1;
                        let wdx = world_b.0 - world_a.0; let wdy = world_b.1 - world_a.1;
                        let llen = ldx.hypot(ldy); let wlen = wdx.hypot(wdy);
                        if llen <= CONTACT_EPSILON || wlen <= CONTACT_EPSILON || (llen - wlen).abs() > 1e-7 * llen.max(wlen).max(1.0) { continue; }
                        let rotation = wdy.atan2(wdx) - ldy.atan2(ldx);
                        let rotated = rotate_point(lp_a, rotation);
                        add_unique_placement(&mut out, Placement { x: world_a.0 - rotated.0, y: world_a.1 - rotated.1, rotation_radians: rotation });
                    }
                }
                if let Form::Circle { radius } = &resource.shape.form {
                    let dx = world_b.0 - world_a.0; let dy = world_b.1 - world_a.1; let distance = dx.hypot(dy);
                    if distance > CONTACT_EPSILON && distance <= 2.0 * radius + CONTACT_EPSILON {
                        let along = distance / 2.0; let height_sq = radius * radius - along * along;
                        if height_sq >= -CONTACT_EPSILON {
                            let height = height_sq.max(0.0).sqrt(); let ux = dx / distance; let uy = dy / distance; let px = -uy * height; let py = ux * height;
                            add_unique_placement(&mut out, Placement { x: world_a.0 + ux * along + px, y: world_a.1 + uy * along + py, rotation_radians: 0.0 });
                            add_unique_placement(&mut out, Placement { x: world_a.0 + ux * along - px, y: world_a.1 + uy * along - py, rotation_radians: 0.0 });
                        }
                    }
                }
            }
        }
    }
    out
}

fn units_strictly_overlap(a: &StructuralUnit, b: &StructuralUnit, catalog: &[BaseResource]) -> bool {
    let (Some(shape_a), Some(shape_b)) = (a.shape(catalog), b.shape(catalog)) else { return false };
    let pa = crate::material_geometry::PlacedMaterialPart { part_index: 0, form: shape_a.form.clone(), placement: a.placement };
    let pb = crate::material_geometry::PlacedMaterialPart { part_index: 1, form: shape_b.form.clone(), placement: b.placement };
    if !crate::material_geometry::placed_forms_overlap(&pa, &pb, 0.0) { return false; }
    let dx = b.placement.x - a.placement.x; let dy = b.placement.y - a.placement.y; let distance = dx.hypot(dy);
    let (sx, sy) = if distance > CONTACT_EPSILON { (dx / distance, dy / distance) } else { (1.0, 0.0) };
    let scale = pa.form.bounding_radius().max(pb.form.bounding_radius()).max(1.0); let epsilon = 1e-8 * scale;
    let shifted = crate::material_geometry::PlacedMaterialPart { part_index: 0, form: pa.form, placement: Placement { x: a.placement.x - sx * epsilon, y: a.placement.y - sy * epsilon, rotation_radians: a.placement.rotation_radians } };
    crate::material_geometry::placed_forms_overlap(&shifted, &pb, 0.0)
}

fn internal_contact_exists(structure: &OrganismStructure, a: usize, b: usize, catalog: &[BaseResource]) -> bool {
    connection_pair_candidates(structure, a, b, catalog).into_iter().any(|candidate| candidate.distance <= CONTACT_EPSILON)
}

fn partial_configuration_valid(structure: &OrganismStructure, material: &Material, assigned: &[Option<usize>], catalog: &[BaseResource]) -> bool {
    for bond in &material.internal_bonds {
        let (Some(a), Some(b)) = (assigned[bond.part_a], assigned[bond.part_b]) else { continue };
        if !internal_contact_exists(structure, a, b, catalog) { return false; }
    }
    true
}

fn choose_next_part(material: &Material, assigned: &[Option<usize>]) -> Option<usize> {
    if assigned.iter().all(Option::is_none) { return Some(0); }
    let mut best = None; let mut best_score = i32::MIN;
    for part in 0..material.parts.len() {
        if assigned[part].is_some() { continue; }
        let neighbors = material.internal_bonds.iter().filter(|bond| (bond.part_a == part && assigned[bond.part_b].is_some()) || (bond.part_b == part && assigned[bond.part_a].is_some())).count() as i32;
        let degree = material.internal_bonds.iter().filter(|bond| bond.part_a == part || bond.part_b == part).count() as i32;
        let score = neighbors * 100 + degree * 10 - part as i32;
        if score > best_score { best_score = score; best = Some(part); }
    }
    best
}

fn solve_material_placements(structure: &OrganismStructure, element: &BlueprintElement, catalog: &[BaseResource], external_constraints: &[Vec<usize>]) -> Result<Vec<Placement>, String> {
    let material = &element.material;
    let mut placements = vec![None; material.parts.len()];
    let mut assigned = vec![None; material.parts.len()];
    let anchor = Placement { x: element.placement.x, y: element.placement.y, rotation_radians: 0.0 };

    fn search(base: &OrganismStructure, material: &Material, anchor: Placement, placements: &mut [Option<Placement>], assigned: &mut [Option<usize>], external: &[Vec<usize>], catalog: &[BaseResource]) -> bool {
        if assigned.iter().all(Option::is_some) { return true; }
        let Some(part) = choose_next_part(material, assigned) else { return false; };
        let Some(res) = resource(catalog, &material.parts[part].0) else { return false; };
        let mut working = base.clone(); let mut working_indices = vec![None; material.parts.len()];
        for i in 0..material.parts.len() {
            let Some(p) = placements[i] else { continue };
            let mut unit = StructuralUnit::new(material.parts[i].0.clone(), p);
            if !unit.realize_default_geometry(catalog) || working.units.iter().any(|old| units_strictly_overlap(&unit, old, catalog)) { return false; }
            working_indices[i] = Some(working.add_unit(unit));
        }
        let mut target_units = Vec::new();
        for bond in &material.internal_bonds {
            let other = if bond.part_a == part && assigned[bond.part_b].is_some() { working_indices[bond.part_b] } else if bond.part_b == part && assigned[bond.part_a].is_some() { working_indices[bond.part_a] } else { None };
            if let Some(index) = other { target_units.push(index); }
        }
        for constraint in external { target_units.extend(constraint.iter().copied()); }
        target_units.sort_unstable(); target_units.dedup();
        let target_endpoints = contact_targets_for_units(&working, &target_units, catalog);
        let candidates = if target_endpoints.is_empty() && part == 0 { vec![anchor] } else { candidate_placements_for_targets(res, &target_endpoints, &working, anchor, part == 0, catalog) };
        for candidate in candidates {
            let mut trial = base.clone(); let mut trial_indices = vec![None; material.parts.len()]; let mut valid = true;
            for i in 0..material.parts.len() {
                let p = if i == part { Some(candidate) } else { placements[i] };
                let Some(p) = p else { continue };
                let mut unit = StructuralUnit::new(material.parts[i].0.clone(), p);
                if !unit.realize_default_geometry(catalog) || trial.units.iter().any(|old| units_strictly_overlap(&unit, old, catalog)) { valid = false; break; }
                trial_indices[i] = Some(trial.add_unit(unit));
            }
            if !valid || !partial_configuration_valid(&trial, material, &trial_indices, catalog) { continue; }
            placements[part] = Some(candidate); assigned[part] = trial_indices[part];
            if search(base, material, anchor, placements, assigned, external, catalog) { return true; }
            assigned[part] = None; placements[part] = None;
        }
        false
    }

    if !search(structure, material, anchor, &mut placements, &mut assigned, external_constraints, catalog) { return Err("no physically valid material realization satisfies its constraints".into()); }
    placements.into_iter().map(|p| p.ok_or_else(|| "material solver returned incomplete realization".to_string())).collect()
}

fn connect_units(structure: &mut OrganismStructure, a: usize, b: usize, catalog: &[BaseResource]) -> Option<f64> {
    let candidates = connection_pair_candidates(structure, a, b, catalog); let id_a = structure.physical_id(a)?; let id_b = structure.physical_id(b)?; let pa = structure.units.get(a)?.properties(catalog)?; let pb = structure.units.get(b)?.properties(catalog)?;
    for candidate in candidates {
        if candidate.distance > CONTACT_EPSILON { continue; }
        let evaluation = crate::combine::evaluate_formation(candidate, pa.cohesion, pb.cohesion); let (_, work, _) = crate::combine::required_investment(pa, pb, evaluation, 0.0).ok()?; let strength = crate::combine::bond_strength(pa, pb);
        if !strength.is_finite() || !(0.0..=1.0).contains(&strength) { continue; }
        let bond = Bond { endpoint_a: BondEndpoint::new(id_a, candidate.endpoint_a), endpoint_b: BondEndpoint::new(id_b, candidate.endpoint_b), strength, bond_energy: 0.0 };
        let mut trial = structure.clone(); if try_add_bond(&mut trial, bond, catalog).is_ok() { *structure = trial; return Some(work); }
    }
    None
}

fn commit_material(structure: &mut OrganismStructure, material: &Material, placements: &[Placement], catalog: &[BaseResource], external_constraints: &[Vec<usize>]) -> Result<Vec<usize>, String> {
    let mut trial = structure.clone(); let mut indices = Vec::with_capacity(placements.len());
    for ((name, _), placement) in material.parts.iter().zip(placements.iter()) {
        let mut unit = StructuralUnit::new(name.clone(), *placement);
        if !unit.realize_default_geometry(catalog) { return Err(format!("material references invalid resource {name}")); }
        if trial.units.iter().any(|old| units_strictly_overlap(&unit, old, catalog)) { return Err("material realization contains strict constituent overlap".into()); }
        indices.push(trial.add_unit(unit));
    }
    for relationship in &material.internal_bonds {
        let a = indices[relationship.part_a]; let b = indices[relationship.part_b];
        if !internal_contact_exists(&trial, a, b, catalog) { return Err("material relationship could not be physically realized".into()); }
        if connect_units(&mut trial, a, b, catalog).is_none() { return Err("material relationship could not be committed as a physical bond".into()); }
    }
    for constraint in external_constraints {
        let mut connected = false;
        'pair: for &a in &indices { for &b in constraint { if connect_units(&mut trial, a, b, catalog).is_some() { connected = true; break 'pair; } } }
        if !connected { return Err("required blueprint contact could not be committed as a physical bond".into()); }
    }
    *structure = trial; Ok(indices)
}

pub(crate) fn realize_material(structure: &mut OrganismStructure, element: &BlueprintElement, catalog: &[BaseResource]) -> Result<Vec<usize>, String> { realize_material_with_constraints(structure, element, catalog, &[]) }

pub(crate) fn realize_material_with_constraints(structure: &mut OrganismStructure, element: &BlueprintElement, catalog: &[BaseResource], external_constraints: &[Vec<usize>]) -> Result<Vec<usize>, String> {
    let material = &element.material;
    if !material.is_valid() || material.parts.is_empty() { return Err("invalid material intent".into()); }
    if material.parts.iter().any(|(_, amount)| (*amount - 1.0).abs() > f64::EPSILON) { return Err("physical construction requires one unit per material constituent".into()); }
    if material.parts.len() > 1 && !material.has_internal_structure() { return Err("multi-constituent material requires construction relationships".into()); }
    if material.internal_bonds.iter().any(|b| b.part_a >= material.parts.len() || b.part_b >= material.parts.len() || b.part_a == b.part_b) { return Err("material contains an invalid construction relationship".into()); }
    if external_constraints.iter().any(|constraint| constraint.is_empty()) { return Err("material has an empty external construction constraint".into()); }
    let placements = solve_material_placements(structure, element, catalog, external_constraints)?;
    commit_material(structure, material, &placements, catalog, external_constraints)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::{default_catalog, InternalBond};

    fn carbon_hydrogen_material() -> Material { Material { parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)], internal_bonds: vec![InternalBond { part_a: 0, part_b: 1 }] } }

    #[test]
    fn single_material_is_realized_at_anchor() {
        let catalog = default_catalog();
        let element = BlueprintElement { material: Material::free_base("Carbon", 1.0), placement: crate::structural_blueprint::BlueprintPlacement { x: 2.0, y: 3.0 } };
        let mut structure = OrganismStructure::new(); let ids = realize_material(&mut structure, &element, &catalog).unwrap();
        assert_eq!(ids.len(), 1); assert_eq!(structure.units[0].placement.x, 2.0); assert_eq!(structure.units[0].placement.y, 3.0);
    }

    #[test]
    fn composite_realization_preserves_internal_contact() {
        let catalog = default_catalog();
        let element = BlueprintElement { material: carbon_hydrogen_material(), placement: crate::structural_blueprint::BlueprintPlacement { x: 0.0, y: 0.0 } };
        let mut structure = OrganismStructure::new(); let ids = realize_material(&mut structure, &element, &catalog).unwrap();
        assert_eq!(ids.len(), 2); assert_eq!(structure.bonds.len(), 1);
        assert!(connection_pair_candidates(&structure, ids[0], ids[1], &catalog).into_iter().any(|candidate| candidate.distance <= CONTACT_EPSILON));
    }
}
