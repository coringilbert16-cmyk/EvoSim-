//! Physical contact and structural connection candidates.
use crate::connection_geometry::{
    facing_compatibility, point_distance, rigid_endpoint_world_point,
};
use crate::resources::Form;
use crate::structure::{Bond, ConnectionEndpoint, OrganismStructure, StructuralUnit};
use crate::surface_geometry::boundary_point_toward;

fn distance(
    a: crate::connection_geometry::WorldConnectionPoint,
    b: crate::connection_geometry::WorldConnectionPoint,
) -> f64 {
    point_distance(a, b)
}

fn facing(
    a: crate::connection_geometry::WorldConnectionPoint,
    b: crate::connection_geometry::WorldConnectionPoint,
) -> f64 {
    facing_compatibility(a, b)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConnectionPairCandidate {
    pub endpoint_a: ConnectionEndpoint,
    pub endpoint_b: ConnectionEndpoint,
    pub distance: f64,
    pub facing: f64,
    pub load_a: f64,
    pub load_b: f64,
    pub available_a: bool,
    pub available_b: bool,
}

pub(crate) fn world_center(
    unit: &StructuralUnit,
) -> crate::connection_geometry::WorldConnectionPoint {
    crate::connection_geometry::WorldConnectionPoint {
        x: unit.placement.x,
        y: unit.placement.y,
        normal_x: 0.0,
        normal_y: 0.0,
    }
}

pub(crate) fn continuous_endpoint(
    unit: &StructuralUnit,
    target: crate::connection_geometry::WorldConnectionPoint,
    catalog: &[crate::resources::BaseResource],
) -> Option<ConnectionEndpoint> {
    let dx = target.x - unit.placement.x;
    let dy = target.y - unit.placement.y;
    let len = dx.hypot(dy);
    if len <= 1e-12 {
        return None;
    }
    let (ux, uy) = (dx / len, dy / len);
    let (s, c) = unit.placement.rotation_radians.sin_cos();
    let lx = ux * c + uy * s;
    let ly = -ux * s + uy * c;
    let shape = unit.shape(catalog)?;
    let point = boundary_point_toward(shape, lx, ly)?;
    match &shape.form {
        Form::Circle { .. } => Some(ConnectionEndpoint::Boundary {
            angle_radians: point.y.atan2(point.x),
        }),
        Form::Fluid { .. } => Some(ConnectionEndpoint::Fluid {
            x: point.x,
            y: point.y,
        }),
        _ => None,
    }
}

pub(crate) fn endpoint_indices(
    unit: &StructuralUnit,
    catalog: &[crate::resources::BaseResource],
) -> Vec<ConnectionEndpoint> {
    let Some(shape) = unit.shape(catalog) else {
        return Vec::new();
    };
    // Hydrogen is finite-area, but its primary structural interfaces remain
    // line-like: exactly two points, one at the center of each longitudinal end.
    // The 0.1 thickness belongs to collision/contact geometry, not to the
    // primary endpoint topology.
    if unit.material.parts.len() == 1
        && unit.material.parts[0].0 == "Hydrogen"
        && matches!(shape.form, Form::Rectangle { .. })
    {
        return (0..2)
            .map(|i| ConnectionEndpoint::LineEndpoint { point_index: i })
            .collect();
    }

    match &shape.form {
        Form::Rectangle { .. } | Form::RegularPolygon { .. } | Form::Polygon { .. } => {
            let count = shape.form.polygon_vertices().map_or(0, |v| v.len());
            (0..count)
                .map(|i| ConnectionEndpoint::Corner { point_index: i })
                .collect()
        }
        Form::Line { .. } => (0..2)
            .map(|i| ConnectionEndpoint::LineEndpoint { point_index: i })
            .collect(),
        Form::Circle { .. } | Form::Fluid { .. } => Vec::new(),
    }
}

fn rigid_boundary_endpoint(
    unit: &StructuralUnit,
    world_dx: f64,
    world_dy: f64,
) -> Option<ConnectionEndpoint> {
    let len = world_dx.hypot(world_dy);
    if len <= 1e-12 {
        return None;
    }
    let (ux, uy) = (world_dx / len, world_dy / len);
    let (s, c) = unit.placement.rotation_radians.sin_cos();
    Some(ConnectionEndpoint::Boundary {
        angle_radians: (uy * c - ux * s).atan2(ux * c + uy * s),
    })
}

fn rigid_surface_candidates(
    a: &StructuralUnit,
    b: &StructuralUnit,
    catalog: &[crate::resources::BaseResource],
) -> Vec<(ConnectionEndpoint, ConnectionEndpoint)> {
    let Some(shape_a) = a.shape(catalog) else {
        return Vec::new();
    };
    let Some(shape_b) = b.shape(catalog) else {
        return Vec::new();
    };
    if matches!(
        shape_a.form,
        Form::Circle { .. } | Form::Line { .. } | Form::Fluid { .. }
    ) || matches!(
        shape_b.form,
        Form::Circle { .. } | Form::Line { .. } | Form::Fluid { .. }
    ) {
        return Vec::new();
    }

    let dx = b.placement.x - a.placement.x;
    let dy = b.placement.y - a.placement.y;
    let distance = dx.hypot(dy);
    if distance <= 1e-12 {
        return Vec::new();
    }

    // The constructor still selects official connection points for placement.
    // These additional endpoints are only the physical bond locations: a bond
    // may land anywhere the two rigid boundaries actually touch.
    let mut out = Vec::new();
    let center_angle = dy.atan2(dx);
    for step in -16..=16 {
        let angle = center_angle + step as f64 * std::f64::consts::PI / 32.0;
        let (s, c) = angle.sin_cos();
        if let (Some(ea), Some(eb)) = (
            rigid_boundary_endpoint(a, c, s),
            rigid_boundary_endpoint(b, -c, -s),
        ) {
            out.push((ea, eb));
        }
    }

    // Also aim at every vertex direction. This catches asymmetric contacts
    // where the closest point on a flat surface is offset from the centerline.
    for vertex in shape_b.form.polygon_vertices().unwrap_or_default() {
        let (s, c) = b.placement.rotation_radians.sin_cos();
        let world_x = b.placement.x + vertex.0 * c - vertex.1 * s;
        let world_y = b.placement.y + vertex.0 * s + vertex.1 * c;
        if let (Some(ea), Some(eb)) = (
            rigid_boundary_endpoint(a, world_x - a.placement.x, world_y - a.placement.y),
            rigid_boundary_endpoint(b, a.placement.x - world_x, a.placement.y - world_y),
        ) {
            out.push((ea, eb));
        }
    }
    for vertex in shape_a.form.polygon_vertices().unwrap_or_default() {
        let (s, c) = a.placement.rotation_radians.sin_cos();
        let world_x = a.placement.x + vertex.0 * c - vertex.1 * s;
        let world_y = a.placement.y + vertex.0 * s + vertex.1 * c;
        if let (Some(ea), Some(eb)) = (
            rigid_boundary_endpoint(a, world_x - a.placement.x, world_y - a.placement.y),
            rigid_boundary_endpoint(b, world_x - b.placement.x, world_y - b.placement.y),
        ) {
            out.push((ea, eb));
        }
    }
    out
}

fn candidate_endpoints(
    a: &StructuralUnit,
    b: &StructuralUnit,
    catalog: &[crate::resources::BaseResource],
) -> Vec<(ConnectionEndpoint, ConnectionEndpoint)> {
    let ea = endpoint_indices(a, catalog);
    let eb = endpoint_indices(b, catalog);
    let mut candidates = if !ea.is_empty() && !eb.is_empty() {
        ea.into_iter()
            .flat_map(|x| eb.iter().copied().map(move |y| (x, y)))
            .collect()
    } else if !ea.is_empty() {
        ea.into_iter()
            .filter_map(|x| {
                let wp = endpoint_world_point(x, a, catalog)?;
                continuous_endpoint(b, wp, catalog).map(|y| (x, y))
            })
            .collect()
    } else if !eb.is_empty() {
        eb.into_iter()
            .filter_map(|y| {
                let wp = endpoint_world_point(y, b, catalog)?;
                continuous_endpoint(a, wp, catalog).map(|x| (x, y))
            })
            .collect()
    } else {
        match (
            continuous_endpoint(a, world_center(b), catalog),
            continuous_endpoint(b, world_center(a), catalog),
        ) {
            (Some(a), Some(b)) => vec![(a, b)],
            _ => Vec::new(),
        }
    };

    candidates.extend(rigid_surface_candidates(a, b, catalog));
    candidates
}

fn endpoint_world_point(
    endpoint: ConnectionEndpoint,
    unit: &StructuralUnit,
    catalog: &[crate::resources::BaseResource],
) -> Option<crate::connection_geometry::WorldConnectionPoint> {
    let shape = unit.shape(catalog)?;
    match endpoint {
        ConnectionEndpoint::Corner { point_index }
        | ConnectionEndpoint::LineEndpoint { point_index } => rigid_endpoint_world_point(
            shape,
            point_index,
            unit.placement.x,
            unit.placement.y,
            unit.placement.rotation_radians,
        ),
        ConnectionEndpoint::Boundary { angle_radians } => {
            let (s, c) = angle_radians.sin_cos();
            let point = boundary_point_toward(shape, c, s)?;
            let len = point.x.hypot(point.y);
            let (nx, ny) = if len > 1e-12 {
                (point.x / len, point.y / len)
            } else {
                (c, s)
            };
            Some(crate::connection_geometry::transform_derived_point(
                point.x,
                point.y,
                nx,
                ny,
                unit.placement.x,
                unit.placement.y,
                unit.placement.rotation_radians,
            ))
        }
        ConnectionEndpoint::Fluid { x, y } => {
            let len = x.hypot(y);
            let (nx, ny) = if len > 1e-12 {
                (x / len, y / len)
            } else {
                (0.0, 0.0)
            };
            Some(crate::connection_geometry::transform_derived_point(
                x,
                y,
                nx,
                ny,
                unit.placement.x,
                unit.placement.y,
                unit.placement.rotation_radians,
            ))
        }
    }
}

fn endpoint_facing(
    a: ConnectionEndpoint,
    b: ConnectionEndpoint,
    ua: &StructuralUnit,
    ub: &StructuralUnit,
    catalog: &[crate::resources::BaseResource],
) -> Option<f64> {
    Some(facing(
        endpoint_world_point(a, ua, catalog)?,
        endpoint_world_point(b, ub, catalog)?,
    ))
}

fn candidate_for_endpoints(
    s: &OrganismStructure,
    ua: usize,
    ub: usize,
    a: ConnectionEndpoint,
    b: ConnectionEndpoint,
    c: &[crate::resources::BaseResource],
) -> Option<ConnectionPairCandidate> {
    let au = s.units.get(ua)?;
    let bu = s.units.get(ub)?;
    let wa = endpoint_world_point(a, au, c)?;
    let wb = endpoint_world_point(b, bu, c)?;
    Some(ConnectionPairCandidate {
        endpoint_a: a,
        endpoint_b: b,
        distance: distance(wa, wb),
        facing: endpoint_facing(a, b, au, bu, c)?,
        load_a: s.connection_load(ua, a, c),
        load_b: s.connection_load(ub, b, c),
        available_a: true,
        available_b: true,
    })
}

pub fn connection_pair_candidates(
    s: &OrganismStructure,
    ua: usize,
    ub: usize,
    c: &[crate::resources::BaseResource],
) -> Vec<ConnectionPairCandidate> {
    let (Some(a), Some(b)) = (s.units.get(ua), s.units.get(ub)) else {
        return Vec::new();
    };
    candidate_endpoints(a, b, c)
        .into_iter()
        .filter_map(|(a, b)| candidate_for_endpoints(s, ua, ub, a, b, c))
        .collect()
}

#[allow(dead_code)]
pub fn contacting_connection_pair_candidates(
    s: &OrganismStructure,
    ua: usize,
    ub: usize,
    c: &[crate::resources::BaseResource],
    t: f64,
    m: f64,
) -> Vec<ConnectionPairCandidate> {
    connection_pair_candidates(s, ua, ub, c)
        .into_iter()
        .filter(|x| x.distance <= t.max(0.0) && x.facing >= m)
        .collect()
}

#[derive(Clone, Debug, Default)]
pub struct ConnectionCompatibilityCache;

impl ConnectionCompatibilityCache {
    pub fn new() -> Self {
        Self
    }
}

pub fn connection_pair_candidates_cached(
    s: &OrganismStructure,
    ua: usize,
    ub: usize,
    c: &[crate::resources::BaseResource],
    _cache: &mut ConnectionCompatibilityCache,
) -> Vec<ConnectionPairCandidate> {
    connection_pair_candidates(s, ua, ub, c)
}

pub fn try_add_bond(
    s: &mut OrganismStructure,
    b: Bond,
    c: &[crate::resources::BaseResource],
) -> Result<usize, &'static str> {
    if !s.is_valid_bond(&b, c) {
        return Err("invalid bond");
    }
    if s.bonds
        .iter()
        .any(|existing| existing.has_same_identity(&b))
    {
        return Err("duplicate bond");
    }
    Ok(s.push_bond_unchecked(b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::structure::{BondEndpoint, PhysicalConstituentId, Placement, StructuralUnit};

    fn test_structure() -> (OrganismStructure, [PhysicalConstituentId; 2]) {
        let mut structure = OrganismStructure::new();
        let a = structure.add_unit(StructuralUnit::new(
            "Carbon",
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));
        let b = structure.add_unit(StructuralUnit::new(
            "Carbon",
            Placement {
                x: 10.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));
        let physical_ids = [
            structure.units[a].physical_id,
            structure.units[b].physical_id,
        ];
        (structure, physical_ids)
    }

    #[test]
    fn hydrogen_exposes_exactly_two_primary_endpoints() {
        let catalog = crate::resources::default_catalog();
        let unit = StructuralUnit::new(
            "Hydrogen",
            crate::structure::Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        );
        let endpoints = endpoint_indices(&unit, &catalog);
        assert_eq!(endpoints.len(), 2);
        assert!(endpoints.iter().all(|endpoint| matches!(
            endpoint,
            ConnectionEndpoint::LineEndpoint { point_index: 0 | 1 }
        )));
        let left = endpoint_world_point(endpoints[0], &unit, &catalog).unwrap();
        let right = endpoint_world_point(endpoints[1], &unit, &catalog).unwrap();
        assert_eq!((left.x, left.y), (-0.5, 0.0));
        assert_eq!((right.x, right.y), (0.5, 0.0));
    }

    #[test]
    fn try_add_bond_rejects_repeated_connection_points_in_either_endpoint_order() {
        let catalog = crate::resources::default_catalog();
        let (mut structure, [id_a, id_b]) = test_structure();
        let first = Bond {
            endpoint_a: BondEndpoint::new(id_a, ConnectionEndpoint::Corner { point_index: 0 }),
            endpoint_b: BondEndpoint::new(id_b, ConnectionEndpoint::Corner { point_index: 0 }),
            strength: 0.5,
            bond_energy: 1.0,
        };

        assert_eq!(try_add_bond(&mut structure, first, &catalog), Ok(0));
        assert_eq!(structure.bonds.len(), 1);

        let same_order = first;
        assert_eq!(
            try_add_bond(&mut structure, same_order, &catalog),
            Err("duplicate bond")
        );
        assert_eq!(structure.bonds.len(), 1);

        let reversed = Bond {
            endpoint_a: first.endpoint_b,
            endpoint_b: first.endpoint_a,
            ..first
        };
        assert_eq!(
            try_add_bond(&mut structure, reversed, &catalog),
            Err("duplicate bond")
        );
        assert_eq!(structure.bonds.len(), 1);
    }
}
