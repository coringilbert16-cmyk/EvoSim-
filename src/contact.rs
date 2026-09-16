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

fn world_center(unit: &StructuralUnit) -> crate::connection_geometry::WorldConnectionPoint {
    crate::connection_geometry::WorldConnectionPoint {
        x: unit.placement.x,
        y: unit.placement.y,
        normal_x: 0.0,
        normal_y: 0.0,
    }
}

fn continuous_endpoint(
    unit: &StructuralUnit,
    target: crate::connection_geometry::WorldConnectionPoint,
    _catalog: &[crate::resources::BaseResource],
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
    let shape = unit.realized_shape()?;
    let point = boundary_point_toward(shape, lx, ly)?;
    match &shape.form {
        Form::Circle { .. } => Some(ConnectionEndpoint::Boundary {
            angle_radians: point.y.atan2(point.x),
        }),
        Form::Fluid { .. } => None,
        _ => None,
    }
}

fn endpoint_indices(
    unit: &StructuralUnit,
    _catalog: &[crate::resources::BaseResource],
) -> Vec<ConnectionEndpoint> {
    let Some(shape) = unit.realized_shape() else {
        return Vec::new();
    };
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

fn candidate_endpoints(
    a: &StructuralUnit,
    b: &StructuralUnit,
    catalog: &[crate::resources::BaseResource],
) -> Vec<(ConnectionEndpoint, ConnectionEndpoint)> {
    let ea = endpoint_indices(a, catalog);
    let eb = endpoint_indices(b, catalog);
    if !ea.is_empty() && !eb.is_empty() {
        return ea
            .into_iter()
            .flat_map(|x| eb.iter().copied().map(move |y| (x, y)))
            .collect();
    }
    if !ea.is_empty() {
        return ea
            .into_iter()
            .filter_map(|x| {
                let wp = endpoint_world_point(x, a, catalog)?;
                continuous_endpoint(b, wp, catalog).map(|y| (x, y))
            })
            .collect();
    }
    if !eb.is_empty() {
        return eb
            .into_iter()
            .filter_map(|y| {
                let wp = endpoint_world_point(y, b, catalog)?;
                continuous_endpoint(a, wp, catalog).map(|x| (x, y))
            })
            .collect();
    }
    match (
        continuous_endpoint(a, world_center(b), catalog),
        continuous_endpoint(b, world_center(a), catalog),
    ) {
        (Some(a), Some(b)) => vec![(a, b)],
        _ => Vec::new(),
    }
}

fn endpoint_world_point(
    endpoint: ConnectionEndpoint,
    unit: &StructuralUnit,
    _catalog: &[crate::resources::BaseResource],
) -> Option<crate::connection_geometry::WorldConnectionPoint> {
    let shape = unit.realized_shape()?;
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
            let len = point.0.hypot(point.1);
            let (nx, ny) = if len > 1e-12 {
                (point.0 / len, point.1 / len)
            } else {
                (c, s)
            };
            Some(crate::connection_geometry::transform_derived_point(
                point.0,
                point.1,
                nx,
                ny,
                unit.placement.x,
                unit.placement.y,
                unit.placement.rotation_radians,
            ))
        }
        ConnectionEndpoint::Fluid { .. } => None,
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
    Ok(s.push_bond_unchecked(b))
}
