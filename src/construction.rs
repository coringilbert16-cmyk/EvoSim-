//! Physical realization of blueprint material intent.
//! A Material is construction intent; this module is where that intent becomes
//! individual physical constituents and realized graph relationships.
use crate::contact::{connection_pair_candidates, try_add_bond};
use crate::resources::{BaseResource, ConnectionSites, Material};
use crate::structural_blueprint::BlueprintElement;
use crate::structure::{Bond, BondEndpoint, ConnectionEndpoint, OrganismStructure, Placement, StructuralUnit};

const CONTACT_EPSILON: f64 = 1e-9;

fn resource(catalog: &[BaseResource], name: &str) -> Option<&BaseResource> {
    catalog.iter().find(|r| r.name == name)
}

fn endpoint_prototypes(unit: &StructuralUnit, catalog: &[BaseResource]) -> Vec<ConnectionEndpoint> {
    match unit.connection_sites(catalog) {
        Some(ConnectionSites::Corners(points)) => (0..points.len()).map(|point_index| ConnectionEndpoint::Corner { point_index }).collect(),
        Some(ConnectionSites::Circumference { .. }) => vec![ConnectionEndpoint::Boundary { angle_radians: 0.0 }],
        Some(ConnectionSites::Undetermined) => {
            let radius = unit.shape(catalog).map(|s| s.form.bounding_radius()).unwrap_or(0.0);
            vec![ConnectionEndpoint::Fluid { x: radius, y: 0.0 }]
        }
        None => Vec::new(),
    }
}

fn world_endpoint(unit: &StructuralUnit, endpoint: ConnectionEndpoint, catalog: &[BaseResource]) -> Option<crate::connection_geometry::WorldConnectionPoint> {
    endpoint.world_point(unit, catalog)
}

fn placement_for_new_constituent(
    neighbor: &StructuralUnit,
    neighbor_endpoint: ConnectionEndpoint,
    new_resource: &BaseResource,
    desired_x: f64,
    desired_y: f64,
    catalog: &[BaseResource],
) -> Option<Placement> {
    let target = world_endpoint(neighbor, neighbor_endpoint, catalog)?;
    let temp = StructuralUnit::new(new_resource.name.clone(), Placement { x: desired_x, y: desired_y, rotation_radians: 0.0 });
    for endpoint in endpoint_prototypes(&temp, catalog) {
        match endpoint {
            ConnectionEndpoint::Corner { point_index } => {
                let sites = temp.connection_sites(catalog)?;
                let ConnectionSites::Corners(points) = sites else { continue };
                let p = points.get(point_index)?;
                let neighbor_normal = (target.normal_x, target.normal_y);
                let desired_normal = (-neighbor_normal.0, -neighbor_normal.1);
                let local_normal = (p.direction_radians.cos(), p.direction_radians.sin());
                let rotation = desired_normal.1.atan2(desired_normal.0) - local_normal.1.atan2(local_normal.0);
                let (s, c) = rotation.sin_cos();
                let x = target.x - (p.x * c - p.y * s);
                let y = target.y - (p.x * s + p.y * c);
                return Some(Placement { x, y, rotation_radians: rotation });
            }
            ConnectionEndpoint::Boundary { .. } | ConnectionEndpoint::Fluid { .. } => {
                let radius = match endpoint {
                    ConnectionEndpoint::Boundary { .. } => new_resource.shape.form.bounding_radius(),
                    ConnectionEndpoint::Fluid { .. } => new_resource.shape.form.bounding_radius(),
                    ConnectionEndpoint::Corner { .. } => unreachable!(),
                };
                let dx = target.x - desired_x;
                let dy = target.y - desired_y;
                let len = dx.hypot(dy);
                let (ux, uy) = if len > CONTACT_EPSILON { (dx / len, dy / len) } else { (1.0, 0.0) };
                return Some(Placement { x: target.x - ux * radius, y: target.y - uy * radius, rotation_radians: 0.0 });
            }
        }
    }
    None
}

fn connect_new_unit(structure: &mut OrganismStructure, neighbor_index: usize, new_index: usize, catalog: &[BaseResource]) -> Option<f64> {
    let mut candidates = connection_pair_candidates(structure, neighbor_index, new_index, catalog);
    candidates.sort_by(|a, b| a.distance.total_cmp(&b.distance).then_with(|| b.facing.total_cmp(&a.facing)));
    let neighbor_id = structure.physical_id(neighbor_index)?;
    let new_id = structure.physical_id(new_index)?;
    let neighbor_properties = structure.units.get(neighbor_index)?.properties(catalog)?;
    let new_properties = structure.units.get(new_index)?.properties(catalog)?;
    for candidate in candidates {
        if candidate.distance > CONTACT_EPSILON { continue; }
        let evaluation = crate::combine::evaluate_formation(candidate, neighbor_properties.cohesion, new_properties.cohesion);
        let (_, work, _) = crate::combine::required_investment(neighbor_properties, new_properties, evaluation, 0.0).ok()?;
        let strength = crate::combine::bond_strength(neighbor_properties, new_properties);
        if !strength.is_finite() || !(0.0..=1.0).contains(&strength) { continue; }
        let bond = Bond { endpoint_a: BondEndpoint::new(neighbor_id, candidate.endpoint_a), endpoint_b: BondEndpoint::new(new_id, candidate.endpoint_b), strength, bond_energy: 0.0 };
        let mut trial = structure.clone();
        if try_add_bond(&mut trial, bond, catalog).is_ok() { *structure = trial; return Some(work); }
    }
    None
}

fn adjacent_realized(material: &Material, part: usize, realized: &[Option<usize>]) -> Vec<usize> {
    material.internal_bonds.iter().filter_map(|bond| {
        let other = if bond.part_a == part { Some(bond.part_b) } else if bond.part_b == part { Some(bond.part_a) } else { None }?;
        realized.get(other).and_then(|index| *index)
    }).collect()
}

/// Realize one blueprint material packet into individual physical constituents.
/// The first constituent is only the construction seed; it has no biological
/// root/core meaning. Subsequent placement is first-fit and never optimized.
pub(crate) fn realize_material(structure: &mut OrganismStructure, element: &BlueprintElement, catalog: &[BaseResource]) -> Result<Vec<usize>, String> {
    let material = &element.material;
    if !material.is_valid() || material.parts.is_empty() { return Err("invalid material intent".into()); }
    if material.parts.iter().any(|(_, amount)| (*amount - 1.0).abs() > f64::EPSILON) { return Err("physical construction requires one unit per material constituent".into()); }
    if material.parts.len() > 1 && !material.has_internal_structure() { return Err("multi-constituent material requires construction relationships".into()); }
    if material.parts.len() > 1 && material.internal_bonds.iter().any(|b| b.part_a >= material.parts.len() || b.part_b >= material.parts.len() || b.part_a == b.part_b) { return Err("material contains an invalid construction relationship".into()); }
    let mut realized = vec![None; material.parts.len()];
    let (first_name, _) = material.parts.first().ok_or_else(|| "material has no constituents".to_string())?;
    if resource(catalog, first_name).is_none() { return Err("material references a missing resource".into()); }
    let first = StructuralUnit::new(first_name.clone(), Placement { x: element.placement.x, y: element.placement.y, rotation_radians: 0.0 });
    let first_index = structure.add_unit(first);
    realized[0] = Some(first_index);

    let mut total_work = 0.0;
    while realized.iter().any(Option::is_none) {
        let mut progress = false;
        for part in 0..material.parts.len() {
            if realized[part].is_some() { continue; }
            let Some(neighbor_index) = adjacent_realized(material, part, &realized).into_iter().next() else { continue };
            let (name, _) = &material.parts[part];
            let Some(base) = resource(catalog, name) else { return Err(format!("material references missing resource {name}")); };
            let neighbor = structure.units.get(neighbor_index).ok_or_else(|| "construction neighbor disappeared".to_string())?.clone();
            let placement = placement_for_new_constituent(&neighbor, endpoint_prototypes(&neighbor, catalog).into_iter().next()?, base, element.placement.x, element.placement.y, catalog).ok_or_else(|| format!("no physical placement found for constituent {part}"))?;
            let new_index = structure.add_unit(StructuralUnit::new(name.clone(), placement));
            if let Some(work) = connect_new_unit(structure, neighbor_index, new_index, catalog) {
                realized[part] = Some(new_index); total_work += work; progress = true;
            } else {
                structure.units.pop();
            }
        }
        if !progress { return Err("construction could not realize the material relationship graph".into()); }
    }

    // Relationships whose two endpoints were already present are attempted now.
    // Failure is intentional: construction does not rearrange previously placed material.
    for bond in &material.internal_bonds {
        let a = realized[bond.part_a].ok_or_else(|| "missing realized constituent".to_string())?;
        let b = realized[bond.part_b].ok_or_else(|| "missing realized constituent".to_string())?;
        if structure.bonds.iter().any(|existing| {
            let Some(ai) = structure.unit_index(existing.endpoint_a.constituent_id) else { return false; };
            let Some(bi) = structure.unit_index(existing.endpoint_b.constituent_id) else { return false; };
            (ai == a && bi == b) || (ai == b && bi == a)
        }) { continue; }
        if connect_new_unit(structure, a, b, catalog).is_none() { return Err("material relationship could not be physically realized".into()); }
    }
    let _ = total_work;
    Ok(realized.into_iter().map(|x| x.expect("all material constituents realized")).collect())
}
