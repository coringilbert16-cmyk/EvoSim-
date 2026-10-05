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
    match &shape.form {
        // Rigid polygon contacts are resolved by the exact surface-feature
        // path below. Only line endpoints remain discrete here because
        // line↔line and line↔continuous-surface contacts still need their
        // physical endpoints as candidate features.
        Form::Line { .. } => (0..2)
            .map(|i| ConnectionEndpoint::LineEndpoint { point_index: i })
            .collect(),
        Form::Rectangle { .. }
        | Form::RegularPolygon { .. }
        | Form::Polygon { .. }
        | Form::Circle { .. }
        | Form::Fluid { .. } => Vec::new(),
    }
}

fn rigid_boundary_endpoint(
    unit: &StructuralUnit,
    world_point: (f64, f64),
    catalog: &[crate::resources::BaseResource],
) -> Option<ConnectionEndpoint> {
    let dx = world_point.0 - unit.placement.x;
    let dy = world_point.1 - unit.placement.y;
    let (s, c) = unit.placement.rotation_radians.sin_cos();
    let local_x = dx * c + dy * s;
    let local_y = -dx * s + dy * c;
    let point = crate::surface_geometry::boundary_point_at(
        unit.shape(catalog)?,
        local_x,
        local_y,
    )?;
    match unit.shape(catalog)?.form {
        Form::Circle { .. } => Some(ConnectionEndpoint::Boundary {
            angle_radians: point.y.atan2(point.x),
        }),
        Form::Fluid { .. } => Some(ConnectionEndpoint::Fluid {
            x: point.x,
            y: point.y,
        }),
        _ => Some(ConnectionEndpoint::BoundaryPoint {
            x: point.x,
            y: point.y,
        }),
    }
}

fn same_world_point(a: (f64, f64), b: (f64, f64)) -> bool {
    (a.0 - b.0).hypot(a.1 - b.1) <= 1e-9
}

fn point_on_segment(point: (f64, f64), start: (f64, f64), end: (f64, f64)) -> bool {
    let dx = end.0 - start.0;
    let dy = end.1 - start.1;
    let length = dx.hypot(dy);
    if length <= f64::EPSILON {
        return same_world_point(point, start);
    }
    let cross = (point.0 - start.0) * dy - (point.1 - start.1) * dx;
    if cross.abs() > 1e-9 * length {
        return false;
    }
    let dot = (point.0 - start.0) * dx + (point.1 - start.1) * dy;
    dot >= -1e-9 && dot <= length * length + 1e-9
}

fn circle_polygon_surface_candidates(
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

    let mut out = Vec::new();

    let mut collect = |circle: &StructuralUnit,
                       circle_is_a: bool,
                       polygon: &StructuralUnit,
                       polygon_shape: &crate::resources::Shape| {
        let radius = match &circle.shape(catalog)?.form {
            crate::resources::Form::Circle { radius } => *radius,
            _ => return Some(()),
        };
        let Some(vertices) = polygon_shape.form.polygon_vertices() else {
            return Some(());
        };

        let world_vertices = world_vertices(&vertices, polygon);
        for i in 0..world_vertices.len() {
            let p0 = world_vertices[i];
            let p1 = world_vertices[(i + 1) % world_vertices.len()];
            let dx = p1.0 - p0.0;
            let dy = p1.1 - p0.1;
            let fx = p0.0 - circle.placement.x;
            let fy = p0.1 - circle.placement.y;
            let a_coef = dx * dx + dy * dy;
            if a_coef <= f64::EPSILON {
                continue;
            }
            let b_coef = 2.0 * (fx * dx + fy * dy);
            let c_coef = fx * fx + fy * fy - radius * radius;
            let discriminant = b_coef * b_coef - 4.0 * a_coef * c_coef;
            if discriminant < -1e-10 {
                continue;
            }

            let sqrt_discriminant = discriminant.max(0.0).sqrt();
            let mut roots = [0.0; 2];
            let root_count = if sqrt_discriminant <= 1e-10 {
                roots[0] = -b_coef / (2.0 * a_coef);
                1
            } else {
                roots[0] = (-b_coef - sqrt_discriminant) / (2.0 * a_coef);
                roots[1] = (-b_coef + sqrt_discriminant) / (2.0 * a_coef);
                2
            };

            for root in roots.into_iter().take(root_count) {
                if !(-1e-9..=1.0 + 1e-9).contains(&root) {
                    continue;
                }
                let t = root.clamp(0.0, 1.0);
                let point = (p0.0 + t * dx, p0.1 + t * dy);
                let local_x = point.0 - circle.placement.x;
                let local_y = point.1 - circle.placement.y;
                let (s, c) = circle.placement.rotation_radians.sin_cos();
                let circle_x = local_x * c + local_y * s;
                let circle_y = -local_x * s + local_y * c;
                let circle_endpoint = ConnectionEndpoint::Boundary {
                    angle_radians: circle_y.atan2(circle_x),
                };
                let polygon_endpoint = if same_world_point(point, p0) {
                    ConnectionEndpoint::Corner { point_index: i }
                } else if same_world_point(point, p1) {
                    ConnectionEndpoint::Corner {
                        point_index: (i + 1) % world_vertices.len(),
                    }
                } else {
                    let Some(endpoint) = boundary_point_endpoint(polygon, point, catalog) else {
                        continue;
                    };
                    endpoint
                };
                let candidate = if circle_is_a {
                    (circle_endpoint, polygon_endpoint)
                } else {
                    (polygon_endpoint, circle_endpoint)
                };
                if !out
                    .iter()
                    .any(|existing: &(ConnectionEndpoint, ConnectionEndpoint)| {
                        existing.0.same_location(candidate.0)
                            && existing.1.same_location(candidate.1)
                    })
                {
                    out.push(candidate);
                }
            }
        }
        Some(())
    };

    match (&shape_a.form, &shape_b.form) {
        (
            Form::Circle { .. },
            Form::Rectangle { .. } | Form::RegularPolygon { .. } | Form::Polygon { .. },
        ) => {
            let _ = collect(a, true, b, shape_b);
        }
        (
            Form::Rectangle { .. } | Form::RegularPolygon { .. } | Form::Polygon { .. },
            Form::Circle { .. },
        ) => {
            let _ = collect(b, false, a, shape_a);
        }
        _ => {}
    }

    out
}

fn line_polygon_surface_candidates(
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
    let mut out = Vec::new();

    let mut collect = |line: &StructuralUnit,
                       line_is_a: bool,
                       polygon: &StructuralUnit,
                       polygon_shape: &crate::resources::Shape| {
        let Some(vertices) = polygon_shape.form.polygon_vertices() else {
            return;
        };
        for endpoint_index in 0..2 {
            let line_endpoint = ConnectionEndpoint::LineEndpoint {
                point_index: endpoint_index,
            };
            let Some(world_endpoint) = endpoint_world_point(line_endpoint, line, catalog) else {
                continue;
            };
            let point = (world_endpoint.x, world_endpoint.y);
            let world_polygon = world_vertices(&vertices, polygon);
            for i in 0..world_polygon.len() {
                let start = world_polygon[i];
                let end = world_polygon[(i + 1) % world_polygon.len()];
                if !point_on_segment(point, start, end) {
                    continue;
                }
                let polygon_endpoint = if same_world_point(point, start) {
                    ConnectionEndpoint::Corner { point_index: i }
                } else if same_world_point(point, end) {
                    ConnectionEndpoint::Corner {
                        point_index: (i + 1) % world_polygon.len(),
                    }
                } else {
                    let Some(endpoint) = boundary_point_endpoint(polygon, point, catalog) else {
                        continue;
                    };
                    endpoint
                };
                let candidate = if line_is_a {
                    (line_endpoint, polygon_endpoint)
                } else {
                    (polygon_endpoint, line_endpoint)
                };
                if !out
                    .iter()
                    .any(|existing: &(ConnectionEndpoint, ConnectionEndpoint)| {
                        existing.0.same_location(candidate.0)
                            && existing.1.same_location(candidate.1)
                    })
                {
                    out.push(candidate);
                }
            }
        }
    };

    match (&shape_a.form, &shape_b.form) {
        (
            Form::Line { .. },
            Form::Rectangle { .. } | Form::RegularPolygon { .. } | Form::Polygon { .. },
        ) => {
            collect(a, true, b, shape_b);
        }
        (
            Form::Rectangle { .. } | Form::RegularPolygon { .. } | Form::Polygon { .. },
            Form::Line { .. },
        ) => {
            collect(b, false, a, shape_a);
        }
        _ => {}
    }
    out
}

fn world_vertices(vertices: &[(f64, f64)], unit: &StructuralUnit) -> Vec<(f64, f64)> {
    let (s, c) = unit.placement.rotation_radians.sin_cos();
    vertices
        .iter()
        .map(|&(x, y)| {
            (
                unit.placement.x + x * c - y * s,
                unit.placement.y + x * s + y * c,
            )
        })
        .collect()
}

fn boundary_point_endpoint(
    unit: &StructuralUnit,
    world_point: (f64, f64),
    catalog: &[crate::resources::BaseResource],
) -> Option<ConnectionEndpoint> {
    let (s, c) = unit.placement.rotation_radians.sin_cos();
    let dx = world_point.0 - unit.placement.x;
    let dy = world_point.1 - unit.placement.y;
    let local_x = dx * c + dy * s;
    let local_y = -dx * s + dy * c;
    let point = crate::surface_geometry::boundary_point_at(unit.shape(catalog)?, local_x, local_y)?;
    Some(ConnectionEndpoint::BoundaryPoint {
        x: point.x,
        y: point.y,
    })
}

fn rigid_surface_candidates(
    a: &StructuralUnit,
    b: &StructuralUnit,
    catalog: &[crate::resources::BaseResource],
) -> Vec<(ConnectionEndpoint, ConnectionEndpoint)> {
    let Some(shape_a) = a.shape(catalog) else { return Vec::new(); };
    let Some(shape_b) = b.shape(catalog) else { return Vec::new(); };

    // Preserve exact analytic contacts for circle/polygon and line/polygon
    // pairs. Polygon/polygon contacts continue through the exact feature path below.
    let mut out = circle_polygon_surface_candidates(a, b, catalog);
    out.extend(line_polygon_surface_candidates(a, b, catalog));

    let Some(vertices_a) = shape_a.form.polygon_vertices() else { return out; };
    let Some(vertices_b) = shape_b.form.polygon_vertices() else { return out; };
    if vertices_a.len() < 3 || vertices_b.len() < 3 { return out; }

    fn world_vertex(unit: &StructuralUnit, vertex: (f64, f64)) -> (f64, f64) {
        let (s, c) = unit.placement.rotation_radians.sin_cos();
        (
            unit.placement.x + vertex.0 * c - vertex.1 * s,
            unit.placement.y + vertex.0 * s + vertex.1 * c,
        )
    }

    fn project(point: (f64, f64), start: (f64, f64), end: (f64, f64)) -> (f64, f64) {
        let dx = end.0 - start.0;
        let dy = end.1 - start.1;
        let length_sq = dx * dx + dy * dy;
        if length_sq <= f64::EPSILON { return start; }
        let t = (((point.0 - start.0) * dx + (point.1 - start.1) * dy) / length_sq)
            .clamp(0.0, 1.0);
        (start.0 + t * dx, start.1 + t * dy)
    }

    fn intersection(
        a0: (f64, f64), a1: (f64, f64),
        b0: (f64, f64), b1: (f64, f64),
    ) -> Vec<(f64, f64)> {
        let r = (a1.0 - a0.0, a1.1 - a0.1);
        let s = (b1.0 - b0.0, b1.1 - b0.1);
        let denominator = r.0 * s.1 - r.1 * s.0;
        let qp = (b0.0 - a0.0, b0.1 - a0.1);
        if denominator.abs() <= 1e-12 {
            // Parallel edges can overlap over a real segment. Preserve one
            // exact representative of that edge-edge contact: its midpoint.
            if (qp.0 * r.1 - qp.1 * r.0).abs() > 1e-10 {
                return Vec::new();
            }
            let length_sq = r.0 * r.0 + r.1 * r.1;
            if length_sq <= f64::EPSILON {
                return Vec::new();
            }
            let t0 = ((b0.0 - a0.0) * r.0 + (b0.1 - a0.1) * r.1) / length_sq;
            let t1 = ((b1.0 - a0.0) * r.0 + (b1.1 - a0.1) * r.1) / length_sq;
            let lo = t0.min(t1).max(0.0);
            let hi = t0.max(t1).min(1.0);
            if lo <= hi + 1e-10 {
                let t = (lo + hi) * 0.5;
                return vec![(a0.0 + t * r.0, a0.1 + t * r.1)];
            }
            return Vec::new();
        }
        let t = (qp.0 * s.1 - qp.1 * s.0) / denominator;
        let u = (qp.0 * r.1 - qp.1 * r.0) / denominator;
        if (-1e-10..=1.0000000001).contains(&t)
            && (-1e-10..=1.0000000001).contains(&u)
        {
            vec![(a0.0 + t * r.0, a0.1 + t * r.1)]
        } else {
            Vec::new()
        }
    }

    fn endpoint_at_world_point(
        unit: &StructuralUnit,
        shape: &crate::resources::Shape,
        world_point: (f64, f64),
        catalog: &[crate::resources::BaseResource],
    ) -> Option<ConnectionEndpoint> {
        let vertices = shape.form.polygon_vertices()?;
        let (s, c) = unit.placement.rotation_radians.sin_cos();
        for (index, &(x, y)) in vertices.iter().enumerate() {
            let wx = unit.placement.x + x * c - y * s;
            let wy = unit.placement.y + x * s + y * c;
            if (wx - world_point.0).hypot(wy - world_point.1) <= 1e-9 {
                return Some(ConnectionEndpoint::Corner { point_index: index });
            }
        }
        rigid_boundary_endpoint(unit, world_point, catalog)
    }

    fn push(
        out: &mut Vec<(ConnectionEndpoint, ConnectionEndpoint)>,
        a: &StructuralUnit, b: &StructuralUnit,
        shape_a: &crate::resources::Shape,
        shape_b: &crate::resources::Shape,
        point_a: (f64, f64), point_b: (f64, f64),
        catalog: &[crate::resources::BaseResource],
    ) {
        if (point_a.0 - a.placement.x).hypot(point_a.1 - a.placement.y) <= 1e-12
            || (point_b.0 - b.placement.x).hypot(point_b.1 - b.placement.y) <= 1e-12
        {
            return;
        }
        let Some(ea) = endpoint_at_world_point(a, shape_a, point_a, catalog) else { return; };
        let Some(eb) = endpoint_at_world_point(b, shape_b, point_b, catalog) else { return; };
        out.push((ea, eb));
    }

    let world_a = vertices_a.iter().copied().map(|v| world_vertex(a, v)).collect::<Vec<_>>();
    let world_b = vertices_b.iter().copied().map(|v| world_vertex(b, v)).collect::<Vec<_>>();
    // Exact candidates are generated only from real boundary features:
    // vertex-to-edge closest points and edge intersections. There is no
    // arbitrary angular sampling or coarse direction grid.
    for &vertex in &world_a {
        for i in 0..world_b.len() {
            let edge_start = world_b[i];
            let edge_end = world_b[(i + 1) % world_b.len()];
            push(&mut out, a, b, shape_a, shape_b, vertex, project(vertex, edge_start, edge_end), catalog);
        }
    }
    for &vertex in &world_b {
        for i in 0..world_a.len() {
            let edge_start = world_a[i];
            let edge_end = world_a[(i + 1) % world_a.len()];
            push(&mut out, a, b, shape_a, shape_b, project(vertex, edge_start, edge_end), vertex, catalog);
        }
    }
    for i in 0..world_a.len() {
        let a0 = world_a[i];
        let a1 = world_a[(i + 1) % world_a.len()];
        for j in 0..world_b.len() {
            let b0 = world_b[j];
            let b1 = world_b[(j + 1) % world_b.len()];
            for point in intersection(a0, a1, b0, b1) {
                push(&mut out, a, b, shape_a, shape_b, point, point, catalog);
            }
        }
    }
    out.dedup_by(|(a0, b0), (a1, b1)| a0.same_location(*a1) && b0.same_location(*b1));
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

    let mut unique = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        if !unique
            .iter()
            .any(|existing: &(ConnectionEndpoint, ConnectionEndpoint)| {
                existing.0.same_location(candidate.0) && existing.1.same_location(candidate.1)
            })
        {
            unique.push(candidate);
        }
    }
    unique
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
        ConnectionEndpoint::BoundaryPoint { x, y } => {
            let point = crate::surface_geometry::boundary_point_at(shape, x, y)?;
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

pub(crate) fn candidate_for_endpoints(
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
    if s.bonds.iter().any(|existing| existing.has_same_physical_identity(&b, s, c)) {
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
    fn duplicate_bond_rejects_corner_and_boundary_point_at_same_physical_location() {
        let catalog = crate::resources::default_catalog();
        let mut structure = OrganismStructure::new();
        let a = structure.add_unit(StructuralUnit::new(
            "Nitrogen",
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));
        let b = structure.add_unit(StructuralUnit::new(
            "Nitrogen",
            Placement {
                x: 1.0,
                y: 0.5,
                rotation_radians: 0.0,
            },
        ));
        let corner = ConnectionEndpoint::Corner { point_index: 1 };
        let corner_world = corner
            .world_point(&structure.units[a], &catalog)
            .expect("corner should resolve");
        let boundary = ConnectionEndpoint::BoundaryPoint {
            x: corner_world.x,
            y: corner_world.y,
        };
        let first = Bond {
            endpoint_a: BondEndpoint::new(structure.units[a].physical_id, corner),
            endpoint_b: BondEndpoint::new(
                structure.units[b].physical_id,
                ConnectionEndpoint::BoundaryPoint { x: -0.5, y: 0.0 },
            ),
            strength: 1.0,
            bond_energy: 1.0,
        };
        try_add_bond(&mut structure, first, &catalog).expect("first bond should be admitted");

        let duplicate = Bond {
            endpoint_a: BondEndpoint::new(structure.units[a].physical_id, boundary),
            endpoint_b: first.endpoint_b,
            strength: 1.0,
            bond_energy: 1.0,
        };
        assert_eq!(
            try_add_bond(&mut structure, duplicate, &catalog),
            Err("duplicate bond")
        );

        let distinct = Bond {
            endpoint_a: BondEndpoint::new(
                structure.units[a].physical_id,
                ConnectionEndpoint::Corner { point_index: 0 },
            ),
            endpoint_b: first.endpoint_b,
            strength: 1.0,
            bond_energy: 1.0,
        };
        assert_eq!(try_add_bond(&mut structure, distinct, &catalog), Ok(1));
        assert_eq!(structure.bonds.len(), 2);
    }

    #[test]
    fn connection_load_matches_corner_and_boundary_point_representations() {
        let catalog = crate::resources::default_catalog();
        let mut structure = OrganismStructure::new();
        let a = structure.add_unit(StructuralUnit::new(
            "Nitrogen",
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));
        let b = structure.add_unit(StructuralUnit::new(
            "Nitrogen",
            Placement {
                x: 1.0,
                y: 0.5,
                rotation_radians: 0.0,
            },
        ));
        let corner = ConnectionEndpoint::Corner { point_index: 1 };
        let boundary = ConnectionEndpoint::BoundaryPoint { x: 0.5, y: -0.5 };
        let bond = Bond {
            endpoint_a: BondEndpoint::new(structure.units[a].physical_id, corner),
            endpoint_b: BondEndpoint::new(
                structure.units[b].physical_id,
                ConnectionEndpoint::BoundaryPoint { x: -0.5, y: 0.0 },
            ),
            strength: 1.0,
            bond_energy: 1.0,
        };
        try_add_bond(&mut structure, bond, &catalog).expect("bond should be admitted");

        let corner_load = structure.connection_load(a, corner, &catalog);
        let boundary_load = structure.connection_load(a, boundary, &catalog);
        assert!(corner_load > 0.0);
        assert_eq!(corner_load, boundary_load);
    }

    #[test]
    fn selected_endpoint_pair_resolves_without_candidate_discovery() {
        let catalog = crate::resources::default_catalog();
        let mut structure = OrganismStructure::new();
        let a = structure.add_unit(StructuralUnit::new(
            "Nitrogen",
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));
        let b = structure.add_unit(StructuralUnit::new(
            "Nitrogen",
            Placement {
                x: 1.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));

        let candidate = candidate_for_endpoints(
            &structure,
            a,
            b,
            ConnectionEndpoint::BoundaryPoint { x: 0.5, y: 0.0 },
            ConnectionEndpoint::BoundaryPoint { x: -0.5, y: 0.0 },
            &catalog,
        )
        .expect("selected physical endpoint pair should resolve directly");

        assert_eq!(
            candidate.endpoint_a,
            ConnectionEndpoint::BoundaryPoint { x: 0.5, y: 0.0 }
        );
        assert_eq!(
            candidate.endpoint_b,
            ConnectionEndpoint::BoundaryPoint { x: -0.5, y: 0.0 }
        );
        assert!(candidate.distance <= crate::combine_runtime::COMBINE_CONTACT_TOLERANCE);

        let discovered = connection_pair_candidates(&structure, a, b, &catalog)
            .into_iter()
            .find(|discovered| {
                discovered.endpoint_a.same_location(candidate.endpoint_a)
                    && discovered.endpoint_b.same_location(candidate.endpoint_b)
            })
            .expect("direct resolution must match the discovered physical candidate");
        assert!((candidate.distance - discovered.distance).abs() <= 1e-12);
        assert!((candidate.facing - discovered.facing).abs() <= 1e-12);
        assert!((candidate.load_a - discovered.load_a).abs() <= 1e-12);
        assert!((candidate.load_b - discovered.load_b).abs() <= 1e-12);
    }

    #[test]
    fn exact_polygon_contact_candidates_include_corner_edge_contact() {
        let catalog = crate::resources::default_catalog();
        let mut structure = OrganismStructure::new();
        let a = structure.add_unit(StructuralUnit::new(
            "Nitrogen",
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));
        let b = structure.add_unit(StructuralUnit::new(
            "Nitrogen",
            Placement {
                x: 1.0,
                y: 0.5,
                rotation_radians: 0.0,
            },
        ));

        let candidates = connection_pair_candidates(&structure, a, b, &catalog);
        assert!(candidates.iter().any(|candidate| {
            matches!(candidate.endpoint_a, ConnectionEndpoint::Corner { .. })
                && matches!(
                    candidate.endpoint_b,
                    ConnectionEndpoint::BoundaryPoint { .. }
                )
                && candidate.distance <= 1e-9
        }));
    }

    #[test]
    fn exact_polygon_contact_candidates_use_midpoint_for_collinear_edge_overlap() {
        let catalog = crate::resources::default_catalog();
        let mut structure = OrganismStructure::new();
        let a = structure.add_unit(StructuralUnit::new(
            "Nitrogen",
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));
        let b = structure.add_unit(StructuralUnit::new(
            "Nitrogen",
            Placement {
                x: 0.0,
                y: 1.0,
                rotation_radians: 0.0,
            },
        ));

        let candidates = connection_pair_candidates(&structure, a, b, &catalog);
        assert!(candidates.iter().any(|candidate| {
            matches!(
                candidate.endpoint_a,
                ConnectionEndpoint::BoundaryPoint { .. }
            ) && matches!(
                candidate.endpoint_b,
                ConnectionEndpoint::BoundaryPoint { .. }
            ) && candidate.distance <= 1e-9
        }));
        assert!(candidates.iter().any(|candidate| {
            let Some(pa) =
                endpoint_world_point(candidate.endpoint_a, &structure.units[a], &catalog)
            else {
                return false;
            };
            (pa.x - 0.0).abs() <= 1e-9 && (pa.y - 0.5).abs() <= 1e-9
        }));
    }

    #[test]
    fn exact_circle_polygon_contact_candidates_include_tangent_contact() {
        let catalog = crate::resources::default_catalog();
        let mut structure = OrganismStructure::new();
        let circle = structure.add_unit(StructuralUnit::new(
            "Water",
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));
        let radius = match catalog
            .iter()
            .find(|resource| resource.name == "Water")
            .unwrap()
            .shape
            .form
        {
            crate::resources::Form::Circle { radius } => radius,
            _ => panic!("Water must be circular"),
        };
        let rectangle = structure.add_unit(StructuralUnit::new(
            "Nitrogen",
            Placement {
                x: radius + 0.5,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));

        let candidates = connection_pair_candidates(&structure, circle, rectangle, &catalog);
        assert!(candidates.iter().any(|candidate| {
            matches!(candidate.endpoint_a, ConnectionEndpoint::Boundary { .. })
                && matches!(
                    candidate.endpoint_b,
                    ConnectionEndpoint::BoundaryPoint { .. }
                )
                && candidate.distance <= 1e-9
        }));
    }

    #[test]
    fn exact_circle_polygon_contact_candidates_include_both_secant_points() {
        let catalog = crate::resources::default_catalog();
        let mut structure = OrganismStructure::new();
        let circle = structure.add_unit(StructuralUnit::new(
            "Water",
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));
        let rectangle = structure.add_unit(StructuralUnit::new(
            "Nitrogen",
            Placement {
                x: 0.3,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));

        let candidates = connection_pair_candidates(&structure, circle, rectangle, &catalog);
        let points = candidates
            .iter()
            .filter_map(|candidate| {
                if !matches!(candidate.endpoint_a, ConnectionEndpoint::Boundary { .. })
                    || !matches!(
                        candidate.endpoint_b,
                        ConnectionEndpoint::BoundaryPoint { .. }
                    )
                    || candidate.distance > 1e-9
                {
                    return None;
                }
                endpoint_world_point(candidate.endpoint_a, &structure.units[circle], &catalog)
                    .map(|point| point.y)
            })
            .collect::<Vec<_>>();

        assert!(points.iter().any(|y| *y > 0.1));
        assert!(points.iter().any(|y| *y < -0.1));
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
