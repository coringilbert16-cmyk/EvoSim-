//! Physical realization of blueprint material intent.

use crate::contact::{connection_pair_candidates, try_add_bond};
use crate::resources::{BaseResource, ConnectionSites, Material};
use crate::structural_blueprint::BlueprintElement;
use crate::structure::{
    Bond, BondEndpoint, ConnectionEndpoint, OrganismStructure, Placement, StructuralUnit,
};

const CONTACT_EPSILON: f64 = 1e-9;

fn resource<'a>(catalog: &'a [BaseResource], name: &str) -> Option<&'a BaseResource> {
    catalog.iter().find(|r| r.name == name)
}

fn endpoint_prototypes(
    unit: &StructuralUnit,
    catalog: &[BaseResource],
) -> Vec<ConnectionEndpoint> {
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

fn placement_candidates_for_new(
    neighbor: &StructuralUnit,
    neighbor_endpoint: ConnectionEndpoint,
    resource: &BaseResource,
    desired_x: f64,
    desired_y: f64,
    catalog: &[BaseResource],
) -> Option<Vec<Placement>> {
    let target = neighbor_endpoint.world_point(neighbor, catalog)?;
    let temp = StructuralUnit::new(
        resource.name.clone(),
        Placement {
            x: desired_x,
            y: desired_y,
            rotation_radians: 0.0,
        },
    );
    let mut out = Vec::new();
    for endpoint in endpoint_prototypes(&temp, catalog) {
        match endpoint {
            ConnectionEndpoint::Corner { point_index }
            | ConnectionEndpoint::LineEndpoint { point_index } => {
                let sites = temp.connection_sites(catalog)?;
                let p = match sites {
                    ConnectionSites::Corners(points) | ConnectionSites::Endpoints(points) => {
                        *points.get(point_index)?
                    }
                    _ => continue,
                };
                out.push(Placement {
                    x: target.x - p.x,
                    y: target.y - p.y,
                    rotation_radians: 0.0,
                });
            }
            ConnectionEndpoint::Boundary { .. } | ConnectionEndpoint::Fluid { .. } => {
                let radius = resource.shape.form.bounding_radius();
                let dx = target.x - desired_x;
                let dy = target.y - desired_y;
                let distance = dx.hypot(dy);
                let (ux, uy) = if distance > CONTACT_EPSILON {
                    (dx / distance, dy / distance)
                } else {
                    (1.0, 0.0)
                };
                out.push(Placement {
                    x: target.x - ux * radius,
                    y: target.y - uy * radius,
                    rotation_radians: 0.0,
                });
            }
        }
    }
    Some(out)
}

fn units_strictly_overlap(a: &StructuralUnit, b: &StructuralUnit, catalog: &[BaseResource]) -> bool {
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

fn adjacent_realized(material: &Material, part: usize, realized: &[Option<usize>]) -> Vec<usize> {
    material
        .internal_bonds
        .iter()
        .filter_map(|bond| {
            let other = if bond.part_a == part {
                bond.part_b
            } else if bond.part_b == part {
                bond.part_a
            } else {
                return None;
            };
            realized.get(other).and_then(|index| *index)
        })
        .collect()
}

fn try_place_and_connect(
    structure: &mut OrganismStructure,
    neighbor_index: usize,
    occupied_indices: &[usize],
    name: &str,
    desired_x: f64,
    desired_y: f64,
    catalog: &[BaseResource],
) -> Option<usize> {
    let neighbor = structure.units.get(neighbor_index)?.clone();
    let base = resource(catalog, name)?;
    let mut candidates = Vec::new();

    for neighbor_endpoint in endpoint_prototypes(&neighbor, catalog) {
        let Some(placements) = placement_candidates_for_new(
            &neighbor,
            neighbor_endpoint,
            base,
            desired_x,
            desired_y,
            catalog,
        ) else {
            continue;
        };
        for placement in placements {
            candidates.push(placement);
        }
    }

    candidates.sort_by(|a, b| {
        let da = (a.x - desired_x).hypot(a.y - desired_y);
        let db = (b.x - desired_x).hypot(b.y - desired_y);
        da.total_cmp(&db)
            .then_with(|| a.x.total_cmp(&b.x))
            .then_with(|| a.y.total_cmp(&b.y))
    });

    for placement in candidates {
        let mut unit = StructuralUnit::new(name.to_owned(), placement);
        if !unit.realize_default_geometry(catalog) {
            continue;
        }

        if occupied_indices
            .iter()
            .copied()
            .any(|index| units_strictly_overlap(&unit, &structure.units[index], catalog))
        {
            continue;
        }

        let new_index = structure.add_unit(unit);
        if connect_units(structure, neighbor_index, new_index, catalog).is_some() {
            return Some(new_index);
        }
        structure.units.pop();
    }
    None
}

fn already_related(structure: &OrganismStructure, a: usize, b: usize) -> bool {
    let Some(id_a) = structure.physical_id(a) else {
        return false;
    };
    let Some(id_b) = structure.physical_id(b) else {
        return false;
    };
    structure.bonds.iter().any(|bond| {
        (bond.endpoint_a.constituent_id == id_a && bond.endpoint_b.constituent_id == id_b)
            || (bond.endpoint_a.constituent_id == id_b && bond.endpoint_b.constituent_id == id_a)
    })
}

/// Expand construction-intent material into individual physical constituents.
/// The first constituent is the physical anchor at the blueprint placement.
/// Remaining constituents are placed at the closest valid physical contact to
/// that anchor while preserving the material relationship graph and rejecting
/// strict overlap with every constituent already in the same material.
/// The physical constituent owns the realized geometry used by contact and bond validation.
pub(crate) fn realize_material(
    structure: &mut OrganismStructure,
    element: &BlueprintElement,
    catalog: &[BaseResource],
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

    let mut realized = vec![None; material.parts.len()];
    let (first_name, _) = &material.parts[0];
    if resource(catalog, first_name).is_none() {
        return Err(format!("material references missing resource {first_name}"));
    }
    let mut first_unit = StructuralUnit::new(
        first_name.clone(),
        Placement {
            x: element.placement.x,
            y: element.placement.y,
            rotation_radians: 0.0,
        },
    );
    if !first_unit.realize_default_geometry(catalog) {
        return Err(format!("material references invalid resource {first_name}"));
    }
    realized[0] = Some(structure.add_unit(first_unit));

    while realized.iter().any(Option::is_none) {
        let mut progress = false;
        for part in 0..material.parts.len() {
            if realized[part].is_some() {
                continue;
            }
            let Some(neighbor) = adjacent_realized(material, part, &realized)
                .into_iter()
                .next()
            else {
                continue;
            };
            let (name, _) = &material.parts[part];
            let occupied = realized.iter().filter_map(|index| *index).collect::<Vec<_>>();
            if let Some(index) = try_place_and_connect(
                structure,
                neighbor,
                &occupied,
                name,
                element.placement.x,
                element.placement.y,
                catalog,
            ) {
                realized[part] = Some(index);
                progress = true;
            }
        }
        if !progress {
            let unresolved = material
                .parts
                .iter()
                .enumerate()
                .filter(|(i, _)| realized[*i].is_none())
                .map(|(i, (name, _))| format!("{i}:{name}"))
                .collect::<Vec<_>>()
                .join(",");
            return Err(format!(
                "construction could not realize material relationship graph; unresolved {unresolved}"
            ));
        }
    }

    for relationship in &material.internal_bonds {
        let a = realized[relationship.part_a]
            .ok_or_else(|| "missing realized constituent".to_string())?;
        let b = realized[relationship.part_b]
            .ok_or_else(|| "missing realized constituent".to_string())?;
        if already_related(structure, a, b) {
            continue;
        }
        if connect_units(structure, a, b, catalog).is_none() {
            return Err("material relationship could not be physically realized".into());
        }
    }

    Ok(realized
        .into_iter()
        .map(|index| index.expect("all constituents realized"))
        .collect())
}
