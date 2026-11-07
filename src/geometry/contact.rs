//! Physical contact and structural connection candidates.
use crate::connection_geometry::{
    facing_compatibility, point_distance, rigid_endpoint_world_point,
};
use crate::resources::Form;
use crate::structure::{Bond, ConnectionEndpoint, OrganismStructure, StructuralUnit};
use crate::surface_geometry::boundary_point_toward;

type Point2 = (f64, f64);
type SegmentFeatureContact = (Point2, Point2);

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

const CONNECTION_ENDPOINT_TOLERANCE: f64 = 1e-8;

fn connection_endpoint_matches(actual: ConnectionEndpoint, expected: ConnectionEndpoint) -> bool {
    match (actual, expected) {
        (
            ConnectionEndpoint::Boundary {
                angle_radians: actual,
            },
            ConnectionEndpoint::Boundary {
                angle_radians: expected,
            },
        ) if actual.is_finite() && expected.is_finite() => {
            let delta = (actual - expected + std::f64::consts::PI)
                .rem_euclid(std::f64::consts::TAU)
                - std::f64::consts::PI;
            delta.abs() <= CONNECTION_ENDPOINT_TOLERANCE
        }
        (
            ConnectionEndpoint::Fluid { x: ax, y: ay },
            ConnectionEndpoint::Fluid { x: ex, y: ey },
        ) if [ax, ay, ex, ey].iter().all(|value| value.is_finite()) => {
            (ax - ex).abs() <= CONNECTION_ENDPOINT_TOLERANCE
                && (ay - ey).abs() <= CONNECTION_ENDPOINT_TOLERANCE
        }
        _ => actual == expected,
    }
}

pub(crate) fn connection_pair_candidate_matches_endpoints(
    candidate: &ConnectionPairCandidate,
    endpoint_a: ConnectionEndpoint,
    endpoint_b: ConnectionEndpoint,
) -> bool {
    connection_endpoint_matches(candidate.endpoint_a, endpoint_a)
        && connection_endpoint_matches(candidate.endpoint_b, endpoint_b)
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
        Form::Circle { .. }
        | Form::Rectangle { .. }
        | Form::RegularPolygon { .. }
        | Form::Polygon { .. } => Some(ConnectionEndpoint::Boundary {
            angle_radians: point.y.atan2(point.x),
        }),
        Form::Fluid { .. } => Some(ConnectionEndpoint::Fluid {
            x: point.x,
            y: point.y,
        }),
        Form::Line { .. } => None,
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

fn segment_feature_contacts(
    a0: (f64, f64),
    a1: (f64, f64),
    b0: (f64, f64),
    b1: (f64, f64),
) -> Vec<SegmentFeatureContact> {
    fn cross(a: (f64, f64), b: (f64, f64)) -> f64 {
        a.0 * b.1 - a.1 * b.0
    }
    fn sub(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
        (a.0 - b.0, a.1 - b.1)
    }
    fn add(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
        (a.0 + b.0, a.1 + b.1)
    }
    fn scale(a: (f64, f64), t: f64) -> (f64, f64) {
        (a.0 * t, a.1 * t)
    }
    fn projection(point: (f64, f64), start: (f64, f64), end: (f64, f64)) -> Option<(f64, f64)> {
        let edge = sub(end, start);
        let len2 = edge.0 * edge.0 + edge.1 * edge.1;
        if len2 <= 1e-24 {
            return None;
        }
        let t = ((point.0 - start.0) * edge.0 + (point.1 - start.1) * edge.1) / len2;
        if t < -1e-10 || t > 1.0000000001 {
            return None;
        }
        Some(add(start, scale(edge, t.clamp(0.0, 1.0))))
    }
    fn push_unique(out: &mut Vec<SegmentFeatureContact>, pair: SegmentFeatureContact) {
        if !out.iter().any(|&(a, b)| {
            (a.0 - pair.0 .0).hypot(a.1 - pair.0 .1) <= 1e-10
                && (a.1 - pair.0 .1).abs() <= 1e-10
                && (b.0 - pair.1 .0).hypot(b.1 - pair.1 .1) <= 1e-10
        }) {
            out.push(pair);
        }
    }

    let mut out = Vec::new();
    let ar = sub(a1, a0);
    let br = sub(b1, b0);
    let denominator = cross(ar, br);
    if denominator.abs() > 1e-12 {
        let delta = sub(b0, a0);
        let t = cross(delta, br) / denominator;
        let u = cross(delta, ar) / denominator;
        if (-1e-10..=1.0000000001).contains(&t) && (-1e-10..=1.0000000001).contains(&u) {
            let point = add(a0, scale(ar, t.clamp(0.0, 1.0)));
            push_unique(&mut out, (point, point));
        }
    }

    for point in [a0, a1] {
        if let Some(projected) = projection(point, b0, b1) {
            push_unique(&mut out, (point, projected));
        }
    }
    for point in [b0, b1] {
        if let Some(projected) = projection(point, a0, a1) {
            push_unique(&mut out, (projected, point));
        }
    }
    out
}

fn rigid_surface_candidates(
    a: &StructuralUnit,
    b: &StructuralUnit,
    catalog: &[crate::resources::BaseResource],
    limit: usize,
) -> Vec<(ConnectionEndpoint, ConnectionEndpoint)> {
    let Some(shape_a) = a.shape(catalog) else {
        return Vec::new();
    };
    let Some(shape_b) = b.shape(catalog) else {
        return Vec::new();
    };
    if limit == 0
        || matches!(
            shape_a.form,
            Form::Circle { .. } | Form::Line { .. } | Form::Fluid { .. }
        )
        || matches!(
            shape_b.form,
            Form::Circle { .. } | Form::Line { .. } | Form::Fluid { .. }
        )
    {
        return Vec::new();
    }
    let vertices_a = shape_a.form.polygon_vertices().unwrap_or_default();
    let vertices_b = shape_b.form.polygon_vertices().unwrap_or_default();
    if vertices_a.len() < 3 || vertices_b.len() < 3 {
        return Vec::new();
    }
    let world_vertex = |unit: &StructuralUnit, point: (f64, f64)| {
        let (s, c) = unit.placement.rotation_radians.sin_cos();
        (
            unit.placement.x + point.0 * c - point.1 * s,
            unit.placement.y + point.0 * s + point.1 * c,
        )
    };
    let mut out = Vec::new();
    'edges: for ai in 0..vertices_a.len() {
        let a0 = world_vertex(a, vertices_a[ai]);
        let a1 = world_vertex(a, vertices_a[(ai + 1) % vertices_a.len()]);
        for bi in 0..vertices_b.len() {
            let b0 = world_vertex(b, vertices_b[bi]);
            let b1 = world_vertex(b, vertices_b[(bi + 1) % vertices_b.len()]);
            for (point_a, point_b) in segment_feature_contacts(a0, a1, b0, b1) {
                if let (Some(ea), Some(eb)) = (
                    rigid_boundary_endpoint(
                        a,
                        point_a.0 - a.placement.x,
                        point_a.1 - a.placement.y,
                    ),
                    rigid_boundary_endpoint(
                        b,
                        point_b.0 - b.placement.x,
                        point_b.1 - b.placement.y,
                    ),
                ) {
                    out.push((ea, eb));
                    if out.len() >= limit {
                        break 'edges;
                    }
                }
            }
        }
    }
    out
}

fn multi_contact_fluid_endpoint_pairs(
    fluid: &StructuralUnit,
    rigid: &StructuralUnit,
    catalog: &[crate::resources::BaseResource],
    limit: usize,
) -> Vec<(ConnectionEndpoint, ConnectionEndpoint)> {
    let Some(fluid_shape) = fluid.shape(catalog) else {
        return Vec::new();
    };
    let Form::Fluid {
        multi_contact_boundary: Some(boundary),
        ..
    } = &fluid_shape.form
    else {
        return Vec::new();
    };
    if limit == 0 {
        return Vec::new();
    }
    let tolerance = crate::combine_runtime::COMBINE_CONTACT_TOLERANCE;
    let (sin, cos) = fluid.placement.rotation_radians.sin_cos();
    let local_to_world = |x: f64, y: f64| {
        (
            fluid.placement.x + x * cos - y * sin,
            fluid.placement.y + x * sin + y * cos,
        )
    };
    let world_to_local = |x: f64, y: f64| {
        let dx = x - fluid.placement.x;
        let dy = y - fluid.placement.y;
        (dx * cos + dy * sin, -dx * sin + dy * cos)
    };
    let on_solid_contact = |x: f64, y: f64| {
        boundary.pieces.iter().any(|piece| {
            let crate::capillary_geometry::FluidBoundaryPiece::SolidContact {
                start_x,
                start_y,
                end_x,
                end_y,
                ..
            } = piece
            else {
                return false;
            };
            let dx = end_x - start_x;
            let dy = end_y - start_y;
            let length_squared = dx * dx + dy * dy;
            if length_squared <= f64::EPSILON {
                return false;
            }
            let t = (((x - start_x) * dx + (y - start_y) * dy) / length_squared).clamp(0.0, 1.0);
            let px = start_x + t * dx;
            let py = start_y + t * dy;
            (x - px).hypot(y - py) <= tolerance
        })
    };

    let mut pairs = Vec::new();
    // Discrete rigid endpoints that lie on an intended fluid contact segment.
    for rigid_endpoint in endpoint_indices(rigid, catalog) {
        let Some(world_point) = endpoint_world_point(rigid_endpoint, rigid, catalog) else {
            continue;
        };
        let local = world_to_local(world_point.x, world_point.y);
        if on_solid_contact(local.0, local.1) {
            let Some(contact_point) = crate::surface_geometry::boundary_point_at_multi_contact(
                boundary, local.0, local.1,
            ) else {
                continue;
            };
            pairs.push((
                ConnectionEndpoint::Fluid {
                    x: contact_point.x,
                    y: contact_point.y,
                },
                rigid_endpoint,
            ));
            if pairs.len() >= limit {
                return pairs;
            }
        }
    }

    // For circles and polygon edges, the midpoint of a solid-contact piece
    // provides an exact surface target. The candidate is kept only if the
    // rigid boundary maps back to that same world point.
    for piece in &boundary.pieces {
        let crate::capillary_geometry::FluidBoundaryPiece::SolidContact {
            start_x,
            start_y,
            end_x,
            end_y,
            ..
        } = piece
        else {
            continue;
        };
        for (local_x, local_y) in [
            (*start_x, *start_y),
            ((*start_x + *end_x) * 0.5, (*start_y + *end_y) * 0.5),
            (*end_x, *end_y),
        ] {
            let world = local_to_world(local_x, local_y);
            let Some(rigid_endpoint) = continuous_endpoint(
                rigid,
                crate::connection_geometry::WorldConnectionPoint {
                    x: world.0,
                    y: world.1,
                    normal_x: 0.0,
                    normal_y: 0.0,
                },
                catalog,
            ) else {
                continue;
            };
            let Some(rigid_world) = endpoint_world_point(rigid_endpoint, rigid, catalog) else {
                continue;
            };
            if (rigid_world.x - world.0).hypot(rigid_world.y - world.1) <= tolerance {
                pairs.push((
                    ConnectionEndpoint::Fluid {
                        x: local_x,
                        y: local_y,
                    },
                    rigid_endpoint,
                ));
                if pairs.len() >= limit {
                    return pairs;
                }
            }
        }
    }
    pairs
}

fn candidate_endpoints_bounded(
    a: &StructuralUnit,
    b: &StructuralUnit,
    catalog: &[crate::resources::BaseResource],
    limit: usize,
) -> (Vec<(ConnectionEndpoint, ConnectionEndpoint)>, bool) {
    let mut candidates = Vec::new();
    if limit == 0 {
        return (candidates, true);
    }
    let ea = endpoint_indices(a, catalog);
    let eb = endpoint_indices(b, catalog);
    if !ea.is_empty() && !eb.is_empty() {
        for x in ea {
            for y in eb.iter().copied() {
                candidates.push((x, y));
                if candidates.len() >= limit {
                    return (candidates, true);
                }
            }
        }
    } else if !ea.is_empty() {
        for x in ea {
            if let Some(wp) = endpoint_world_point(x, a, catalog) {
                if let Some(y) = continuous_endpoint(b, wp, catalog) {
                    candidates.push((x, y));
                    if candidates.len() >= limit {
                        return (candidates, true);
                    }
                }
            }
        }
    } else if !eb.is_empty() {
        for y in eb {
            if let Some(wp) = endpoint_world_point(y, b, catalog) {
                if let Some(x) = continuous_endpoint(a, wp, catalog) {
                    candidates.push((x, y));
                    if candidates.len() >= limit {
                        return (candidates, true);
                    }
                }
            }
        }
    } else if let (Some(x), Some(y)) = (
        continuous_endpoint(a, world_center(b), catalog),
        continuous_endpoint(b, world_center(a), catalog),
    ) {
        candidates.push((x, y));
        if candidates.len() >= limit {
            return (candidates, true);
        }
    }

    let remaining = limit - candidates.len();
    candidates.extend(rigid_surface_candidates(a, b, catalog, remaining));
    if candidates.len() >= limit {
        return (candidates, true);
    }
    let remaining = limit - candidates.len();
    if matches!(
        a.shape(catalog).map(|shape| &shape.form),
        Some(Form::Fluid {
            multi_contact_boundary: Some(_),
            ..
        })
    ) {
        candidates.extend(multi_contact_fluid_endpoint_pairs(a, b, catalog, remaining));
        if candidates.len() >= limit {
            return (candidates, true);
        }
    }
    let remaining = limit - candidates.len();
    if matches!(
        b.shape(catalog).map(|shape| &shape.form),
        Some(Form::Fluid {
            multi_contact_boundary: Some(_),
            ..
        })
    ) {
        candidates.extend(
            multi_contact_fluid_endpoint_pairs(b, a, catalog, remaining)
                .into_iter()
                .map(|(fluid_endpoint, rigid_endpoint)| (rigid_endpoint, fluid_endpoint)),
        );
        if candidates.len() >= limit {
            return (candidates, true);
        }
    }
    (candidates, false)
}

fn candidate_endpoints(
    a: &StructuralUnit,
    b: &StructuralUnit,
    catalog: &[crate::resources::BaseResource],
) -> Vec<(ConnectionEndpoint, ConnectionEndpoint)> {
    candidate_endpoints_bounded(a, b, catalog, usize::MAX).0
}

pub(crate) fn endpoint_world_point(
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
            Some(crate::connection_geometry::transform_derived_point(
                point.x,
                point.y,
                point.normal_x,
                point.normal_y,
                unit.placement.x,
                unit.placement.y,
                unit.placement.rotation_radians,
            ))
        }
        ConnectionEndpoint::Fluid { x, y } => {
            let point = match &shape.form {
                Form::Fluid {
                    multi_contact_boundary: Some(boundary),
                    ..
                } => crate::surface_geometry::boundary_point_at_multi_contact(boundary, x, y)?,
                _ => boundary_point_toward(shape, x, y)?,
            };
            Some(crate::connection_geometry::transform_derived_point(
                point.x,
                point.y,
                point.normal_x,
                point.normal_y,
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

/// Resolve only the endpoint pair already recorded by a physical bond.
///
/// BREAK discovery knows the bond endpoints before asking for geometry
/// candidates. Keep authoritative endpoint enumeration and ordering, but
/// avoid materializing geometry for every possible pair when only one pair
/// can match the recorded bond.
pub(crate) fn connection_pair_candidate_for_endpoints(
    s: &OrganismStructure,
    ua: usize,
    ub: usize,
    c: &[crate::resources::BaseResource],
    endpoint_a: ConnectionEndpoint,
    endpoint_b: ConnectionEndpoint,
) -> Option<ConnectionPairCandidate> {
    let (Some(a), Some(b)) = (s.units.get(ua), s.units.get(ub)) else {
        return None;
    };
    let (candidate_a, candidate_b) =
        candidate_endpoints(a, b, c)
            .into_iter()
            .find(|(candidate_a, candidate_b)| {
                connection_endpoint_matches(*candidate_a, endpoint_a)
                    && connection_endpoint_matches(*candidate_b, endpoint_b)
            })?;
    candidate_for_endpoints(s, ua, ub, candidate_a, candidate_b, c)
}

#[allow(dead_code)] // Diagnostic-only until the census is explicitly integrated.
/// Generate at most `limit` raw endpoint pairs before geometry conversion.
/// The boolean is true when the limit stopped or exhausted candidate generation.
pub(crate) fn connection_pair_candidates_bounded(
    s: &OrganismStructure,
    ua: usize,
    ub: usize,
    c: &[crate::resources::BaseResource],
    limit: usize,
) -> (Vec<ConnectionPairCandidate>, usize, bool) {
    let (Some(a), Some(b)) = (s.units.get(ua), s.units.get(ub)) else {
        return (Vec::new(), 0, false);
    };
    let (endpoints, limit_reached) = candidate_endpoints_bounded(a, b, c, limit);
    let raw_count = endpoints.len();
    let candidates = endpoints
        .into_iter()
        .filter_map(|(a, b)| candidate_for_endpoints(s, ua, ub, a, b, c))
        .collect();
    (candidates, raw_count, limit_reached)
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
    fn bounded_candidate_generation_returns_the_exact_complete_prefix() {
        let catalog = crate::resources::default_catalog();
        let (structure, _) = test_structure();
        let complete = connection_pair_candidates(&structure, 0, 1, &catalog);
        let (bounded, raw_count, limit_reached) =
            connection_pair_candidates_bounded(&structure, 0, 1, &catalog, 2);

        assert!(complete.len() > 2);
        assert_eq!(raw_count, 2);
        assert!(limit_reached);
        assert_eq!(bounded, complete[..2]);

        let (empty, raw_count, limit_reached) =
            connection_pair_candidates_bounded(&structure, 0, 1, &catalog, 0);
        assert!(empty.is_empty());
        assert_eq!(raw_count, 0);
        assert!(limit_reached);
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
    fn segment_feature_contacts_use_exact_intersections_and_projections() {
        let crossing = segment_feature_contacts((-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0));
        assert!(crossing
            .iter()
            .any(|(a, b)| { (a.0.abs() + a.1.abs() + b.0.abs() + b.1.abs()) < 1e-12 }));

        let touching = segment_feature_contacts((-1.0, 0.0), (1.0, 0.0), (0.5, 1.0), (0.5, 2.0));
        assert!(touching.iter().any(|(a, b)| {
            (a.0 - 0.5).abs() < 1e-12
                && a.1.abs() < 1e-12
                && (b.0 - 0.5).abs() < 1e-12
                && (b.1 - 1.0).abs() < 1e-12
        }));
    }

    #[test]
    fn boundary_endpoints_preserve_physical_surface_normals() {
        let catalog = crate::resources::default_catalog();
        let unit = StructuralUnit::new(
            "Carbon",
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        );
        let endpoint = ConnectionEndpoint::Boundary { angle_radians: 0.1 };
        let point = endpoint_world_point(endpoint, &unit, &catalog).unwrap();
        let normal_length = point.normal_x.hypot(point.normal_y);
        assert!(
            (normal_length - 1.0).abs() < 1e-12,
            "surface normal must remain normalized"
        );

        let rotated = StructuralUnit::new(
            "Carbon",
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: std::f64::consts::FRAC_PI_2,
            },
        );
        let rotated_point = endpoint_world_point(endpoint, &rotated, &catalog).unwrap();
        assert!((rotated_point.normal_x + point.normal_y).abs() < 1e-12);
        assert!((rotated_point.normal_y - point.normal_x).abs() < 1e-12);
    }

    #[test]
    fn targeted_candidate_lookup_matches_full_enumeration_in_both_endpoint_orders() {
        let catalog = crate::resources::default_catalog();
        let (structure, _) = test_structure();
        let all_candidates = connection_pair_candidates(&structure, 0, 1, &catalog);
        assert!(!all_candidates.is_empty());

        for expected in &all_candidates {
            assert_eq!(
                connection_pair_candidate_for_endpoints(
                    &structure,
                    0,
                    1,
                    &catalog,
                    expected.endpoint_a,
                    expected.endpoint_b,
                ),
                Some(*expected),
            );
            let reversed_expected = all_candidates
                .iter()
                .find(|candidate| {
                    candidate.endpoint_a == expected.endpoint_b
                        && candidate.endpoint_b == expected.endpoint_a
                })
                .copied();
            assert_eq!(
                connection_pair_candidate_for_endpoints(
                    &structure,
                    0,
                    1,
                    &catalog,
                    expected.endpoint_b,
                    expected.endpoint_a,
                ),
                reversed_expected,
            );
        }

        assert_eq!(
            connection_pair_candidate_for_endpoints(
                &structure,
                0,
                1,
                &catalog,
                ConnectionEndpoint::Boundary {
                    angle_radians: 123.456,
                },
                ConnectionEndpoint::Boundary {
                    angle_radians: -98.765,
                },
            ),
            None,
        );
    }

    #[test]
    fn targeted_boundary_endpoint_lookup_tolerates_rotation_roundoff() {
        let catalog = crate::resources::default_catalog();
        let molecule = crate::environment_chemistry_experiment::realize_natural_molecule(
            &["Hydrogen".into(), "Sulfur".into()],
            Placement {
                x: 10.0,
                y: 10.0,
                rotation_radians: 0.0,
            },
            &catalog,
        );
        let connection = molecule
            .internal_connections
            .as_ref()
            .and_then(|connections| connections.first())
            .expect("seed molecule should retain its COMBINE endpoint record");
        assert!(matches!(
            connection.endpoint_a,
            ConnectionEndpoint::Boundary { .. }
        ));
        assert!(matches!(
            connection.endpoint_b,
            ConnectionEndpoint::Boundary { .. }
        ));
        let mut structure = OrganismStructure::new();
        let indices = crate::material_restoration::restore_material(
            &mut structure,
            &molecule,
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
            &catalog,
        )
        .expect("seed molecule must restore");
        let expected = connection_pair_candidate_for_endpoints(
            &structure,
            indices[connection.part_a],
            indices[connection.part_b],
            &catalog,
            connection.endpoint_a,
            connection.endpoint_b,
        )
        .expect("seed endpoint pair must resolve before rotation");
        let jitter = |endpoint| match endpoint {
            ConnectionEndpoint::Boundary { angle_radians } => ConnectionEndpoint::Boundary {
                angle_radians: angle_radians + 1e-12,
            },
            endpoint => endpoint,
        };

        assert_eq!(
            connection_pair_candidate_for_endpoints(
                &structure,
                indices[connection.part_a],
                indices[connection.part_b],
                &catalog,
                jitter(connection.endpoint_a),
                jitter(connection.endpoint_b),
            ),
            Some(expected),
            "transport-induced trigonometric roundoff must not orphan an existing bond"
        );
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
