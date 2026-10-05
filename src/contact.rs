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
    catalog: &[crate::resources::BaseResource],
) -> Option<ConnectionEndpoint> {
    let len = world_dx.hypot(world_dy);
    if len <= 1e-12 {
        return None;
    }
    let (ux, uy) = (world_dx / len, world_dy / len);
    let (s, c) = unit.placement.rotation_radians.sin_cos();
    let lx = ux * c + uy * s;
    let ly = -ux * s + uy * c;
    let point = crate::surface_geometry::boundary_point_toward(unit.shape(catalog)?, lx, ly)?;
    match unit.shape(catalog)?.form {
        Form::Circle { .. } => Some(ConnectionEndpoint::Boundary {
            angle_radians: point.y.atan2(point.x),
        }),
        Form::Fluid { .. } => Some(ConnectionEndpoint::Fluid { x: point.x, y: point.y }),
        _ => Some(ConnectionEndpoint::BoundaryPoint { x: point.x, y: point.y }),
    }
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
    let Some(vertices_a) = shape_a.form.polygon_vertices() else {
        return Vec::new();
    };
    let Some(vertices_b) = shape_b.form.polygon_vertices() else {
        return Vec::new();
    };

    let world_a = world_vertices(&vertices_a, a);
    let world_b = world_vertices(&vertices_b, b);
    let mut out = Vec::new();

    // Exact corner/corner contacts.
    for (ia, &pa) in world_a.iter().enumerate() {
        for (ib, &pb) in world_b.iter().enumerate() {
            if same_world_point(pa, pb) {
                out.push((
                    ConnectionEndpoint::Corner { point_index: ia },
                    ConnectionEndpoint::Corner { point_index: ib },
                ));
            }
        }
    }

    // Exact corner/edge contacts in both directions.
    for (ia, &pa) in world_a.iter().enumerate() {
        for ib in 0..world_b.len() {
            let pb = world_b[ib];
            let qb = world_b[(ib + 1) % world_b.len()];
            if point_on_segment(pa, pb, qb) && !is_endpoint(pa, pb, qb) {
                if let Some(endpoint_b) = boundary_point_endpoint(b, pa, catalog) {
                    out.push((
                        ConnectionEndpoint::Corner { point_index: ia },
                        endpoint_b,
                    ));
                }
            }
        }
    }
    for (ib, &pb) in world_b.iter().enumerate() {
        for ia in 0..world_a.len() {
            let pa = world_a[ia];
            let qa = world_a[(ia + 1) % world_a.len()];
            if point_on_segment(pb, pa, qa) && !is_endpoint(pb, pa, qa) {
                if let Some(endpoint_a) = boundary_point_endpoint(a, pb, catalog) {
                    out.push((
                        endpoint_a,
                        ConnectionEndpoint::Corner { point_index: ib },
                    ));
                }
            }
        }
    }

    // Exact edge/edge intersections and collinear overlaps. A crossing has
    // one physical contact point; a coincident overlap uses its midpoint.
    for ia in 0..world_a.len() {
        let a0 = world_a[ia];
        let a1 = world_a[(ia + 1) % world_a.len()];
        for ib in 0..world_b.len() {
            let b0 = world_b[ib];
            let b1 = world_b[(ib + 1) % world_b.len()];
            for point in segment_contact_points(a0, a1, b0, b1) {
                if let (Some(ea), Some(eb)) = (
                    boundary_point_endpoint(a, point, catalog),
                    boundary_point_endpoint(b, point, catalog),
                ) {
                    out.push((ea, eb));
                }
            }
        }
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

fn same_world_point(a: (f64, f64), b: (f64, f64)) -> bool {
    (a.0 - b.0).hypot(a.1 - b.1) <= 1e-9
}

fn point_on_segment(
    point: (f64, f64),
    start: (f64, f64),
    end: (f64, f64),
) -> bool {
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

fn is_endpoint(point: (f64, f64), start: (f64, f64), end: (f64, f64)) -> bool {
    same_world_point(point, start) || same_world_point(point, end)
}

fn segment_contact_points(
    a0: (f64, f64),
    a1: (f64, f64),
    b0: (f64, f64),
    b1: (f64, f64),
) -> Vec<(f64, f64)> {
    let r = (a1.0 - a0.0, a1.1 - a0.1);
    let s = (b1.0 - b0.0, b1.1 - b0.1);
    let rxs = r.0 * s.1 - r.1 * s.0;
    let q_minus_p = (b0.0 - a0.0, b0.1 - a0.1);
    let qpxr = q_minus_p.0 * r.1 - q_minus_p.1 * r.0;
    let epsilon = 1e-10;

    if rxs.abs() > epsilon {
        let t = (q_minus_p.0 * s.1 - q_minus_p.1 * s.0) / rxs;
        let u = (q_minus_p.0 * r.1 - q_minus_p.1 * r.0) / rxs;
        if t >= -epsilon && t <= 1.0 + epsilon && u >= -epsilon && u <= 1.0 + epsilon {
            return vec![(
                a0.0 + t * r.0,
                a0.1 + t * r.1,
            )];
        }
        return Vec::new();
    }

    if qpxr.abs() > epsilon {
        return Vec::new();
    }

    let rr = r.0 * r.0 + r.1 * r.1;
    let ss = s.0 * s.0 + s.1 * s.1;
    if rr <= f64::EPSILON || ss <= f64::EPSILON {
        return Vec::new();
    }

    // Collinear overlap: project B onto A, then use the midpoint of the
    // overlap interval as the single representative bond location.
    let t0 = (q_minus_p.0 * r.0 + q_minus_p.1 * r.1) / rr;
    let t1 = ((b1.0 - a0.0) * r.0 + (b1.1 - a0.1) * r.1) / rr;
    let lo = t0.min(t1).max(0.0);
    let hi = t0.max(t1).min(1.0);
    if hi < lo - epsilon {
        return Vec::new();
    }
    let t = (lo + hi) * 0.5;
    vec![(a0.0 + t * r.0, a0.1 + t * r.1)]
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
                y: 1.0,
                rotation_radians: 0.0,
            },
        ));

        let candidates = connection_pair_candidates(&structure, a, b, &catalog);
        assert!(candidates.iter().any(|candidate| {
            matches!(candidate.endpoint_a, ConnectionEndpoint::BoundaryPoint { .. })
                && matches!(candidate.endpoint_b, ConnectionEndpoint::Corner { .. })
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
            matches!(candidate.endpoint_a, ConnectionEndpoint::BoundaryPoint { .. })
                && matches!(candidate.endpoint_b, ConnectionEndpoint::BoundaryPoint { .. })
                && candidate.distance <= 1e-9
        }));
        assert!(candidates.iter().any(|candidate| {
            let Some(pa) = endpoint_world_point(candidate.endpoint_a, &structure.units[a], &catalog)
            else {
                return false;
            };
            (pa.x - 0.0).abs() <= 1e-9 && (pa.y - 0.5).abs() <= 1e-9
        }));
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
