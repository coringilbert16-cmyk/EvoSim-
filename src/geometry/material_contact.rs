//! Contact candidates between free environmental constituents.
//!
//! This module works from realized PhysicalMaterial parts and never restores
//! environmental chemistry into an organism graph.
use crate::connection_geometry::{
    facing_compatibility, point_distance, rigid_endpoint_world_point, transform_derived_point,
    WorldConnectionPoint,
};
use crate::physical_material::{PhysicalMaterial, PhysicalMaterialBond};
use crate::resources::{BaseResource, Form, Shape};
use crate::structure::{ConnectionEndpoint, Placement};
use crate::contact::ConnectionPairCandidate;
use crate::surface_geometry::boundary_point_toward;

const ENDPOINT_TOLERANCE: f64 = 1e-8;
const CONTACT_POINT_TOLERANCE: f64 = 0.1;

#[derive(Clone)]
struct ConstituentView {
    name: String,
    shape: Shape,
    placement: Placement,
}

fn view(
    material: &PhysicalMaterial,
    part_index: usize,
    catalog: &[BaseResource],
) -> Option<ConstituentView> {
    if !material.is_realized() || material.owner_relative_origin.is_some() {
        return None;
    }
    let (name, amount) = material.material.parts.get(part_index)?;
    if !amount.is_finite() || (*amount - 1.0).abs() > 1e-9 {
        return None;
    }
    Some(ConstituentView {
        name: name.clone(),
        shape: material.realized_shape_for_part(part_index, catalog)?,
        placement: *material.placements.as_ref()?.get(part_index)?,
    })
}

fn endpoint_indices(view: &ConstituentView) -> Vec<ConnectionEndpoint> {
    if view.name == "Hydrogen" && matches!(view.shape.form, Form::Rectangle { .. }) {
        return (0..2)
            .map(|point_index| ConnectionEndpoint::LineEndpoint { point_index })
            .collect();
    }
    match &view.shape.form {
        Form::Rectangle { .. } | Form::RegularPolygon { .. } | Form::Polygon { .. } => view
            .shape
            .form
            .polygon_vertices()
            .map(|vertices| {
                (0..vertices.len())
                    .map(|point_index| ConnectionEndpoint::Corner { point_index })
                    .collect()
            })
            .unwrap_or_default(),
        Form::Line { .. } => (0..2)
            .map(|point_index| ConnectionEndpoint::LineEndpoint { point_index })
            .collect(),
        Form::Circle { .. } | Form::Fluid { .. } => Vec::new(),
    }
}

fn endpoint_world_point(
    view: &ConstituentView,
    endpoint: ConnectionEndpoint,
) -> Option<WorldConnectionPoint> {
    match endpoint {
        ConnectionEndpoint::Corner { point_index } => {
            crate::connection_geometry::transform_polygon_vertex(
                &view.shape,
                point_index,
                view.placement.x,
                view.placement.y,
                view.placement.rotation_radians,
            )
        }
        ConnectionEndpoint::LineEndpoint { point_index } => {
            let line_like = matches!(view.shape.form, Form::Line { .. })
                || matches!(
                    view.shape.form,
                    Form::Rectangle { width, height }
                        if view.name == "Hydrogen" && height <= width * 0.1 + 1e-12
                );
            if !line_like {
                return None;
            }
            rigid_endpoint_world_point(
                &view.shape,
                point_index,
                view.placement.x,
                view.placement.y,
                view.placement.rotation_radians,
            )
        }
        ConnectionEndpoint::Boundary { angle_radians } => {
            let (nx, ny) = angle_radians.sin_cos();
            let point = boundary_point_toward(&view.shape, nx, ny)?;
            Some(transform_derived_point(
                point.x,
                point.y,
                point.normal_x,
                point.normal_y,
                view.placement.x,
                view.placement.y,
                view.placement.rotation_radians,
            ))
        }
        ConnectionEndpoint::Fluid { x, y } => {
            let point = match &view.shape.form {
                Form::Fluid {
                    multi_contact_boundary: Some(boundary),
                    ..
                } => crate::surface_geometry::boundary_point_at_multi_contact(boundary, x, y)?,
                _ => boundary_point_toward(&view.shape, x, y)?,
            };
            Some(transform_derived_point(
                point.x,
                point.y,
                point.normal_x,
                point.normal_y,
                view.placement.x,
                view.placement.y,
                view.placement.rotation_radians,
            ))
        }
    }
}

fn continuous_endpoint(
    view: &ConstituentView,
    target: WorldConnectionPoint,
) -> Option<ConnectionEndpoint> {
    let dx = target.x - view.placement.x;
    let dy = target.y - view.placement.y;
    let length = dx.hypot(dy);
    if !length.is_finite() || length <= 1e-12 {
        return None;
    }
    let (ux, uy) = (dx / length, dy / length);
    let (sin, cos) = view.placement.rotation_radians.sin_cos();
    let local_x = ux * cos + uy * sin;
    let local_y = -ux * sin + uy * cos;
    let point = boundary_point_toward(&view.shape, local_x, local_y)?;
    match view.shape.form {
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

fn endpoint_matches(actual: ConnectionEndpoint, expected: ConnectionEndpoint) -> bool {
    match (actual, expected) {
        (
            ConnectionEndpoint::Boundary { angle_radians: a },
            ConnectionEndpoint::Boundary { angle_radians: b },
        ) if a.is_finite() && b.is_finite() => {
            let delta = (a - b + std::f64::consts::PI)
                .rem_euclid(std::f64::consts::TAU)
                - std::f64::consts::PI;
            delta.abs() <= ENDPOINT_TOLERANCE
        }
        (
            ConnectionEndpoint::Fluid { x: ax, y: ay },
            ConnectionEndpoint::Fluid { x: bx, y: by },
        ) if [ax, ay, bx, by].iter().all(|v| v.is_finite()) => {
            (ax - bx).abs() <= ENDPOINT_TOLERANCE
                && (ay - by).abs() <= ENDPOINT_TOLERANCE
        }
        _ => actual == expected,
    }
}

fn connection_occupies(
    connection: &PhysicalMaterialBond,
    part_index: usize,
    endpoint: ConnectionEndpoint,
) -> bool {
    (connection.part_a == part_index && endpoint_matches(connection.endpoint_a, endpoint))
        || (connection.part_b == part_index && endpoint_matches(connection.endpoint_b, endpoint))
}

fn endpoint_load(
    material: &PhysicalMaterial,
    part_index: usize,
    endpoint: ConnectionEndpoint,
    excluded_connection: Option<usize>,
) -> f64 {
    material
        .internal_connections
        .as_deref()
        .unwrap_or_default()
        .iter()
        .enumerate()
        .filter(|(index, connection)| {
            Some(*index) != excluded_connection
                && connection_occupies(connection, part_index, endpoint)
        })
        .map(|(_, connection)| crate::combine::experimental_bond_strength(connection.bond_energy))
        .sum()
}

fn endpoint_available(
    material: &PhysicalMaterial,
    part_index: usize,
    endpoint: ConnectionEndpoint,
    excluded_connection: Option<usize>,
) -> bool {
    !material
        .internal_connections
        .as_deref()
        .unwrap_or_default()
        .iter()
        .enumerate()
        .any(|(index, connection)| {
            Some(index) != excluded_connection
                && connection_occupies(connection, part_index, endpoint)
        })
}

fn rigid_boundary_endpoint(
    view: &ConstituentView,
    world_dx: f64,
    world_dy: f64,
) -> Option<ConnectionEndpoint> {
    let length = world_dx.hypot(world_dy);
    if !length.is_finite() || length <= 1e-12 {
        return None;
    }
    let (ux, uy) = (world_dx / length, world_dy / length);
    let (sin, cos) = view.placement.rotation_radians.sin_cos();
    Some(ConnectionEndpoint::Boundary {
        angle_radians: (uy * cos - ux * sin).atan2(ux * cos + uy * sin),
    })
}

type Point2 = (f64, f64);
type SegmentPair = (Point2, Point2);

fn segment_feature_contacts(
    a0: Point2,
    a1: Point2,
    b0: Point2,
    b1: Point2,
) -> Vec<SegmentPair> {
    fn cross(a: Point2, b: Point2) -> f64 {
        a.0 * b.1 - a.1 * b.0
    }
    fn sub(a: Point2, b: Point2) -> Point2 {
        (a.0 - b.0, a.1 - b.1)
    }
    fn add(a: Point2, b: Point2) -> Point2 {
        (a.0 + b.0, a.1 + b.1)
    }
    fn scale(a: Point2, t: f64) -> Point2 {
        (a.0 * t, a.1 * t)
    }
    fn projection(point: Point2, start: Point2, end: Point2) -> Option<Point2> {
        let edge = sub(end, start);
        let length_squared = edge.0 * edge.0 + edge.1 * edge.1;
        if length_squared <= 1e-24 {
            return None;
        }
        let t = ((point.0 - start.0) * edge.0 + (point.1 - start.1) * edge.1)
            / length_squared;
        if !(-1e-10..=1.0000000001).contains(&t) {
            return None;
        }
        Some(add(start, scale(edge, t.clamp(0.0, 1.0))))
    }
    fn push_unique(out: &mut Vec<SegmentPair>, pair: SegmentPair) {
        if !out.iter().any(|&(a, b)| {
            (a.0 - pair.0.0).hypot(a.1 - pair.0.1) <= 1e-10
                && (b.0 - pair.1.0).hypot(b.1 - pair.1.1) <= 1e-10
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
        if (-1e-10..=1.0000000001).contains(&t)
            && (-1e-10..=1.0000000001).contains(&u)
        {
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
    a: &ConstituentView,
    b: &ConstituentView,
    limit: usize,
) -> Vec<(ConnectionEndpoint, ConnectionEndpoint)> {
    if limit == 0
        || matches!(a.shape.form, Form::Circle { .. } | Form::Line { .. } | Form::Fluid { .. })
        || matches!(b.shape.form, Form::Circle { .. } | Form::Line { .. } | Form::Fluid { .. })
    {
        return Vec::new();
    }
    let va = a.shape.form.polygon_vertices().unwrap_or_default();
    let vb = b.shape.form.polygon_vertices().unwrap_or_default();
    if va.len() < 3 || vb.len() < 3 {
        return Vec::new();
    }
    let world_vertex = |view: &ConstituentView, point: Point2| {
        let (sin, cos) = view.placement.rotation_radians.sin_cos();
        (
            view.placement.x + point.0 * cos - point.1 * sin,
            view.placement.y + point.0 * sin + point.1 * cos,
        )
    };
    let mut out = Vec::new();
    'edges: for ai in 0..va.len() {
        let a0 = world_vertex(a, va[ai]);
        let a1 = world_vertex(a, va[(ai + 1) % va.len()]);
        for bi in 0..vb.len() {
            let b0 = world_vertex(b, vb[bi]);
            let b1 = world_vertex(b, vb[(bi + 1) % vb.len()]);
            for (pa, pb) in segment_feature_contacts(a0, a1, b0, b1) {
                if let (Some(ea), Some(eb)) = (
                    rigid_boundary_endpoint(a, pa.0 - a.placement.x, pa.1 - a.placement.y),
                    rigid_boundary_endpoint(b, pb.0 - b.placement.x, pb.1 - b.placement.y),
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

fn multi_contact_fluid_pairs(
    fluid: &ConstituentView,
    rigid: &ConstituentView,
    limit: usize,
) -> Vec<(ConnectionEndpoint, ConnectionEndpoint)> {
    let Form::Fluid {
        multi_contact_boundary: Some(boundary),
        ..
    } = &fluid.shape.form
    else {
        return Vec::new();
    };
    if limit == 0 {
        return Vec::new();
    }
    let (sin, cos) = fluid.placement.rotation_radians.sin_cos();
    let to_world = |x: f64, y: f64| {
        (
            fluid.placement.x + x * cos - y * sin,
            fluid.placement.y + x * sin + y * cos,
        )
    };
    let to_local = |x: f64, y: f64| {
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
            let t = (((x - start_x) * dx + (y - start_y) * dy) / length_squared)
                .clamp(0.0, 1.0);
            let px = start_x + t * dx;
            let py = start_y + t * dy;
            (x - px).hypot(y - py) <= CONTACT_POINT_TOLERANCE
        })
    };
    let mut out = Vec::new();
    for rigid_endpoint in endpoint_indices(rigid) {
        let Some(world) = endpoint_world_point(rigid, rigid_endpoint) else {
            continue;
        };
        let local = to_local(world.x, world.y);
        if on_solid_contact(local.0, local.1) {
            if let Some(point) = crate::surface_geometry::boundary_point_at_multi_contact(
                boundary, local.0, local.1,
            ) {
                out.push((
                    ConnectionEndpoint::Fluid { x: point.x, y: point.y },
                    rigid_endpoint,
                ));
                if out.len() >= limit {
                    return out;
                }
            }
        }
    }
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
        for (x, y) in [
            (*start_x, *start_y),
            ((*start_x + *end_x) * 0.5, (*start_y + *end_y) * 0.5),
            (*end_x, *end_y),
        ] {
            let world = to_world(x, y);
            let Some(endpoint) = continuous_endpoint(
                rigid,
                WorldConnectionPoint {
                    x: world.0,
                    y: world.1,
                    normal_x: 0.0,
                    normal_y: 0.0,
                },
            ) else {
                continue;
            };
            let Some(rigid_world) = endpoint_world_point(rigid, endpoint) else {
                continue;
            };
            if (rigid_world.x - world.0).hypot(rigid_world.y - world.1)
                <= CONTACT_POINT_TOLERANCE
            {
                out.push((ConnectionEndpoint::Fluid { x, y }, endpoint));
                if out.len() >= limit {
                    return out;
                }
            }
        }
    }
    out
}

fn candidate_endpoints(a: &ConstituentView, b: &ConstituentView) -> Vec<(ConnectionEndpoint, ConnectionEndpoint)> {
    let mut out = Vec::new();
    let ea = endpoint_indices(a);
    let eb = endpoint_indices(b);
    if !ea.is_empty() && !eb.is_empty() {
        for x in ea {
            for y in eb.iter().copied() {
                out.push((x, y));
            }
        }
    } else if !ea.is_empty() {
        for x in ea {
            if let Some(point) = endpoint_world_point(a, x) {
                if let Some(y) = continuous_endpoint(b, point) {
                    out.push((x, y));
                }
            }
        }
    } else if !eb.is_empty() {
        for y in eb {
            if let Some(point) = endpoint_world_point(b, y) {
                if let Some(x) = continuous_endpoint(a, point) {
                    out.push((x, y));
                }
            }
        }
    } else {
        let center_a = WorldConnectionPoint {
            x: a.placement.x,
            y: a.placement.y,
            normal_x: 0.0,
            normal_y: 0.0,
        };
        let center_b = WorldConnectionPoint {
            x: b.placement.x,
            y: b.placement.y,
            normal_x: 0.0,
            normal_y: 0.0,
        };
        if let (Some(x), Some(y)) = (continuous_endpoint(a, center_b), continuous_endpoint(b, center_a)) {
            out.push((x, y));
        }
    }
    out.extend(rigid_surface_candidates(
        a,
        b,
        usize::MAX.saturating_sub(out.len()),
    ));
    if matches!(a.shape.form, Form::Fluid { multi_contact_boundary: Some(_), .. }) {
        out.extend(multi_contact_fluid_pairs(a, b, usize::MAX.saturating_sub(out.len())));
    }
    if matches!(b.shape.form, Form::Fluid { multi_contact_boundary: Some(_), .. }) {
        out.extend(
            multi_contact_fluid_pairs(b, a, usize::MAX.saturating_sub(out.len()))
                .into_iter()
                .map(|(fluid, rigid)| (rigid, fluid)),
        );
    }
    out
}

pub(crate) fn connection_pair_candidates_for_material_parts(
    source_a: &PhysicalMaterial,
    part_a: usize,
    excluded_connection_a: Option<usize>,
    source_b: &PhysicalMaterial,
    part_b: usize,
    catalog: &[BaseResource],
) -> Option<Vec<ConnectionPairCandidate>> {
    let a = view(source_a, part_a, catalog)?;
    let b = view(source_b, part_b, catalog)?;
    let mut candidates = Vec::new();
    for (endpoint_a, endpoint_b) in candidate_endpoints(&a, &b) {
        let point_a = endpoint_world_point(&a, endpoint_a)?;
        let point_b = endpoint_world_point(&b, endpoint_b)?;
        let available_a = endpoint_available(source_a, part_a, endpoint_a, excluded_connection_a);
        let available_b = endpoint_available(source_b, part_b, endpoint_b, None);
        let candidate = ConnectionPairCandidate {
            endpoint_a,
            endpoint_b,
            distance: point_distance(point_a, point_b),
            facing: facing_compatibility(point_a, point_b),
            load_a: endpoint_load(source_a, part_a, endpoint_a, excluded_connection_a),
            load_b: endpoint_load(source_b, part_b, endpoint_b, None),
            available_a,
            available_b,
        };
        if !candidates.iter().any(|existing: &ConnectionPairCandidate| {
            endpoint_matches(existing.endpoint_a, endpoint_a)
                && endpoint_matches(existing.endpoint_b, endpoint_b)
        }) {
            candidates.push(candidate);
        }
    }
    Some(candidates)
}

pub(crate) fn endpoint_world_point_for_material_part(
    material: &PhysicalMaterial,
    part_index: usize,
    endpoint: ConnectionEndpoint,
    catalog: &[BaseResource],
) -> Option<WorldConnectionPoint> {
    endpoint_world_point(&view(material, part_index, catalog)?, endpoint)
}
