//! Physical contact and structural connection candidates.
use crate::connection_geometry::{
    facing_compatibility, point_distance, rigid_endpoint_world_point,
};
use crate::resources::Form;
use crate::structure::{Bond, ConnectionEndpoint, OrganismStructure, StructuralUnit};
use crate::surface_geometry::boundary_point_toward;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContactFeature {
    Edge,
    Corner,
    LineEndpoint,
    Surface,
    Fluid,
}

impl ContactFeature {
    pub fn bond_strength_factor(self, other: Self) -> Option<f64> {
        use ContactFeature::*;
        match (self, other) {
            (Edge, Edge) => Some(1.0),
            (Edge, Corner) | (Corner, Edge) => Some(0.5),
            (Corner, Corner) => Some(1.0),
            (LineEndpoint, LineEndpoint) => Some(1.0),
            (LineEndpoint, Corner) | (Corner, LineEndpoint) => Some(1.0),
            (LineEndpoint, Edge) | (Edge, LineEndpoint) => Some(0.5),
            // Curved/fluid contacts have no approved strength rule yet.
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContactFeatureMeasurement {
    pub feature: ContactFeature,
    /// Length of the actual physical feature when the scale rule has one.
    /// Corners have no invented characteristic length.
    pub scale: Option<f64>,
}

fn boundary_feature(
    unit: &StructuralUnit,
    endpoint: ConnectionEndpoint,
    catalog: &[crate::resources::BaseResource],
) -> Option<ContactFeatureMeasurement> {
    let shape = unit.shape(catalog)?;
    match endpoint {
        ConnectionEndpoint::Corner { .. } => Some(ContactFeatureMeasurement {
            feature: ContactFeature::Corner,
            scale: None,
        }),
        ConnectionEndpoint::LineEndpoint { .. } => match shape.form {
            Form::Line { length } => Some(ContactFeatureMeasurement {
                feature: ContactFeature::LineEndpoint,
                scale: Some(length.abs()),
            }),
            _ => None,
        },
        ConnectionEndpoint::Boundary { angle_radians } => match &shape.form {
            Form::Circle { .. } => Some(ContactFeatureMeasurement {
                feature: ContactFeature::Surface,
                scale: None,
            }),
            Form::Rectangle { .. } | Form::RegularPolygon { .. } | Form::Polygon { .. } => {
                let point = boundary_point_toward(
                    shape,
                    angle_radians.cos(),
                    angle_radians.sin(),
                )?;
                let vertices = shape.form.polygon_vertices()?;
                if let Some(point_index) = vertices.iter().position(|&(x, y)| {
                    (point.x - x).hypot(point.y - y) <= 1e-9
                }) {
                    return Some(ContactFeatureMeasurement {
                        feature: ContactFeature::Corner,
                        scale: None,
                    });
                }
                let mut best = None;
                for i in 0..vertices.len() {
                    let a = vertices[i];
                    let b = vertices[(i + 1) % vertices.len()];
                    let dx = b.0 - a.0;
                    let dy = b.1 - a.1;
                    let len_sq = dx * dx + dy * dy;
                    if len_sq <= f64::EPSILON {
                        continue;
                    }
                    let t = (((point.x - a.0) * dx + (point.y - a.1) * dy) / len_sq)
                        .clamp(0.0, 1.0);
                    let px = a.0 + t * dx;
                    let py = a.1 + t * dy;
                    let distance_sq = (point.x - px).powi(2) + (point.y - py).powi(2);
                    if best.map_or(true, |(d, _): (f64, f64)| distance_sq < d) {
                        best = Some((distance_sq, len_sq.sqrt()));
                    }
                }
                Some(ContactFeatureMeasurement {
                    feature: ContactFeature::Edge,
                    scale: best.map(|(_, length)| length),
                })
            }
            Form::Line { .. } | Form::Fluid { .. } => None,
        },
        ConnectionEndpoint::Fluid { .. } => Some(ContactFeatureMeasurement {
            feature: ContactFeature::Fluid,
            scale: None,
        }),
    }
}

pub fn contact_feature_measurement(
    unit: &StructuralUnit,
    endpoint: ConnectionEndpoint,
    catalog: &[crate::resources::BaseResource],
) -> Option<ContactFeatureMeasurement> {
    boundary_feature(unit, endpoint, catalog)
}

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
    pub feature_a: ContactFeatureMeasurement,
    pub feature_b: ContactFeatureMeasurement,
    pub bond_strength_factor: Option<f64>,
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

fn world_polygon_vertices(
    unit: &StructuralUnit,
    catalog: &[crate::resources::BaseResource],
) -> Option<Vec<(f64, f64)>> {
    let shape = unit.shape(catalog)?;
    let vertices = shape.form.polygon_vertices()?;
    let (s, c) = unit.placement.rotation_radians.sin_cos();
    Some(
        vertices
            .into_iter()
            .map(|(x, y)| {
                (
                    unit.placement.x + x * c - y * s,
                    unit.placement.y + x * s + y * c,
                )
            })
            .collect(),
    )
}

fn closest_point_on_segment(
    p: (f64, f64),
    a: (f64, f64),
    b: (f64, f64),
) -> (f64, f64) {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let length_sq = dx * dx + dy * dy;
    if length_sq <= f64::EPSILON {
        return a;
    }
    let t = (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / length_sq).clamp(0.0, 1.0);
    (a.0 + t * dx, a.1 + t * dy)
}

fn point_near(a: (f64, f64), b: (f64, f64)) -> bool {
    (a.0 - b.0).hypot(a.1 - b.1) <= 1e-9
}

fn vertex_index_at(vertices: &[(f64, f64)], point: (f64, f64)) -> Option<usize> {
    vertices
        .iter()
        .position(|&vertex| point_near(vertex, point))
}

fn cross(a: (f64, f64), b: (f64, f64), c: (f64, f64)) -> f64 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

fn collinear_overlap_midpoint(
    a0: (f64, f64),
    a1: (f64, f64),
    b0: (f64, f64),
    b1: (f64, f64),
) -> Option<(f64, f64)> {
    if cross(a0, a1, b0).abs() > 1e-9 || cross(a0, a1, b1).abs() > 1e-9 {
        return None;
    }
    let use_x = (a1.0 - a0.0).abs() >= (a1.1 - a0.1).abs();
    let (a_start, a_end, b_start, b_end) = if use_x {
        (a0.0, a1.0, b0.0, b1.0)
    } else {
        (a0.1, a1.1, b0.1, b1.1)
    };
    let lo = a_start.min(a_end).max(b_start.min(b_end));
    let hi = a_start.max(a_end).min(b_start.max(b_end));
    if hi - lo <= 1e-9 {
        return None;
    }
    let value = (lo + hi) * 0.5;
    let t = if use_x {
        (value - a0.0) / (a1.0 - a0.0)
    } else {
        (value - a0.1) / (a1.1 - a0.1)
    };
    Some((a0.0 + t * (a1.0 - a0.0), a0.1 + t * (a1.1 - a0.1)))
}

fn rigid_contact_endpoint(
    unit: &StructuralUnit,
    point: (f64, f64),
    vertex_index: Option<usize>,
    catalog: &[crate::resources::BaseResource],
) -> Option<ConnectionEndpoint> {
    if let Some(point_index) = vertex_index {
        return Some(ConnectionEndpoint::Corner { point_index });
    }
    let endpoint = rigid_boundary_endpoint(
        unit,
        point.0 - unit.placement.x,
        point.1 - unit.placement.y,
    )?;
    let shape = unit.shape(catalog)?;
    if matches!(
        shape.form,
        Form::Rectangle { .. } | Form::RegularPolygon { .. } | Form::Polygon { .. }
    ) {
        Some(endpoint)
    } else {
        None
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
    if !matches!(
        shape_a.form,
        Form::Rectangle { .. } | Form::RegularPolygon { .. } | Form::Polygon { .. }
    ) || !matches!(
        shape_b.form,
        Form::Rectangle { .. } | Form::RegularPolygon { .. } | Form::Polygon { .. }
    ) {
        return Vec::new();
    }

    let Some(vertices_a) = world_polygon_vertices(a, catalog) else {
        return Vec::new();
    };
    let Some(vertices_b) = world_polygon_vertices(b, catalog) else {
        return Vec::new();
    };

    // Generate candidates from actual polygon features only:
    // vertex-to-edge and edge-to-vertex contacts, plus edge-overlap contacts.
    // There is no angular sampling around the centerline.
    let mut out = Vec::new();

    for (index_a, &vertex_a) in vertices_a.iter().enumerate() {
        for edge in 0..vertices_b.len() {
            let b0 = vertices_b[edge];
            let b1 = vertices_b[(edge + 1) % vertices_b.len()];
            let point_b = closest_point_on_segment(vertex_a, b0, b1);
            let b_index = vertex_index_at(&vertices_b, point_b);
            if let (Some(ea), Some(eb)) = (
                rigid_contact_endpoint(a, vertex_a, Some(index_a), catalog),
                rigid_contact_endpoint(b, point_b, b_index, catalog),
            ) {
                out.push((ea, eb));
            }
        }
    }

    for (index_b, &vertex_b) in vertices_b.iter().enumerate() {
        for edge in 0..vertices_a.len() {
            let a0 = vertices_a[edge];
            let a1 = vertices_a[(edge + 1) % vertices_a.len()];
            let point_a = closest_point_on_segment(vertex_b, a0, a1);
            let a_index = vertex_index_at(&vertices_a, point_a);
            if let (Some(ea), Some(eb)) = (
                rigid_contact_endpoint(a, point_a, a_index, catalog),
                rigid_contact_endpoint(b, vertex_b, Some(index_b), catalog),
            ) {
                out.push((ea, eb));
            }
        }
    }

    for edge_a in 0..vertices_a.len() {
        let a0 = vertices_a[edge_a];
        let a1 = vertices_a[(edge_a + 1) % vertices_a.len()];
        for edge_b in 0..vertices_b.len() {
            let b0 = vertices_b[edge_b];
            let b1 = vertices_b[(edge_b + 1) % vertices_b.len()];
            let Some(point) = collinear_overlap_midpoint(a0, a1, b0, b1) else {
                continue;
            };
            let (Some(ea), Some(eb)) = (
                rigid_contact_endpoint(a, point, None, catalog),
                rigid_contact_endpoint(b, point, None, catalog),
            ) else {
                continue;
            };
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
    let feature_a = contact_feature_measurement(au, a, c)?;
    let feature_b = contact_feature_measurement(bu, b, c)?;
    Some(ConnectionPairCandidate {
        endpoint_a: a,
        endpoint_b: b,
        distance: distance(wa, wb),
        facing: endpoint_facing(a, b, au, bu, c)?,
        load_a: s.connection_load(ua, a, c),
        load_b: s.connection_load(ub, b, c),
        available_a: true,
        available_b: true,
        bond_strength_factor: feature_a.feature.bond_strength_factor(feature_b.feature),
        feature_a,
        feature_b,
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
    fn polygon_boundary_is_classified_as_edge_without_inventing_corner_scale() {
        let catalog = crate::resources::default_catalog();
        let structure = test_structure().0;
        let unit = &structure.units[0];
        let measurement = contact_feature_measurement(
            unit,
            ConnectionEndpoint::Boundary { angle_radians: 0.0 },
            &catalog,
        )
        .unwrap();
        assert_eq!(measurement.feature, ContactFeature::Edge);
        assert!(measurement.scale.is_some_and(|x| x > 0.0));

        let corner = contact_feature_measurement(
            unit,
            ConnectionEndpoint::Corner { point_index: 0 },
            &catalog,
        )
        .unwrap();
        assert_eq!(corner.feature, ContactFeature::Corner);
        assert_eq!(corner.scale, None);
    }

    #[test]
    fn contact_strength_follows_physical_feature_pair() {
        use ContactFeature::*;
        assert_eq!(Edge.bond_strength_factor(Edge), Some(1.0));
        assert_eq!(Edge.bond_strength_factor(Corner), Some(0.5));
        assert_eq!(Corner.bond_strength_factor(Corner), Some(1.0));
        assert_eq!(LineEndpoint.bond_strength_factor(LineEndpoint), Some(1.0));
        assert_eq!(LineEndpoint.bond_strength_factor(Corner), Some(1.0));
        assert_eq!(LineEndpoint.bond_strength_factor(Edge), Some(0.5));
        assert_eq!(Surface.bond_strength_factor(Edge), None);
    }

    #[test]
    #[test]
    fn polygon_boundary_facing_uses_actual_edge_normal_not_radial_direction() {
        let catalog = crate::resources::default_catalog();
        let structure = test_structure().0;
        let unit = &structure.units[0];
        let edge = ConnectionEndpoint::Boundary {
            angle_radians: 0.0,
        };
        let world = endpoint_world_point(edge, unit, &catalog).unwrap();
        assert!((world.normal_x - 1.0).abs() < 1e-12);
        assert!(world.normal_y.abs() < 1e-12);
    }

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
