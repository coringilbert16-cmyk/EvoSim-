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

    // Enumerate exact points on overlapping rigid faces. The constructor
    // aligns faces first; contact must use those same physical faces rather
    // than rediscovering them through an angular ray sweep.
    let mut out = Vec::new();
    let edges_a = crate::rigid_boundary::polygon_edges(shape_a);
    let edges_b = crate::rigid_boundary::polygon_edges(shape_b);
    let world_point = |unit: &StructuralUnit, point: (f64, f64)| {
        let (s, c) = unit.placement.rotation_radians.sin_cos();
        (
            unit.placement.x + point.0 * c - point.1 * s,
            unit.placement.y + point.0 * s + point.1 * c,
        )
    };
    let cross = |ax: f64, ay: f64, bx: f64, by: f64| ax * by - ay * bx;

    for (edge_a, &(a0, a1)) in edges_a.iter().enumerate() {
        let wa0 = world_point(a, a0);
        let wa1 = world_point(a, a1);
        let adx = wa1.0 - wa0.0;
        let ady = wa1.1 - wa0.1;
        let alen2 = adx * adx + ady * ady;
        if alen2 <= f64::EPSILON {
            continue;
        }

        for (edge_b, &(b0, b1)) in edges_b.iter().enumerate() {
            let wb0 = world_point(b, b0);
            let wb1 = world_point(b, b1);
            let bdx = wb1.0 - wb0.0;
            let bdy = wb1.1 - wb0.1;
            let blen2 = bdx * bdx + bdy * bdy;
            if blen2 <= f64::EPSILON {
                continue;
            }

            let parallel_error = cross(adx, ady, bdx, bdy).abs();
            let offset_error = cross(adx, ady, wb0.0 - wa0.0, wb0.1 - wa0.1).abs();
            let scale = alen2.sqrt() * blen2.sqrt();
            if parallel_error > 1.0e-8 * scale || offset_error > 1.0e-8 * alen2.sqrt() {
                continue;
            }

            let project_a =
                |point: (f64, f64)| ((point.0 - wa0.0) * adx + (point.1 - wa0.1) * ady) / alen2;
            let b0_t = project_a(wb0);
            let b1_t = project_a(wb1);
            let overlap_start = 0.0_f64.max(b0_t.min(b1_t));
            let overlap_end = 1.0_f64.min(b0_t.max(b1_t));
            if overlap_end + 1.0e-10 < overlap_start {
                continue;
            }

            let fraction_a = ((overlap_start + overlap_end) * 0.5).clamp(0.0, 1.0);
            let contact_x = wa0.0 + adx * fraction_a;
            let contact_y = wa0.1 + ady * fraction_a;
            let fraction_b = ((contact_x - wb0.0) * bdx + (contact_y - wb0.1) * bdy) / blen2;
            if !(0.0..=1.0).contains(&fraction_b) {
                continue;
            }

            out.push((
                ConnectionEndpoint::Surface {
                    edge_index: edge_a,
                    fraction: fraction_a,
                },
                ConnectionEndpoint::Surface {
                    edge_index: edge_b,
                    fraction: fraction_b,
                },
            ));
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
        ConnectionEndpoint::Surface {
            edge_index,
            fraction,
        } => {
            let point = crate::surface_geometry::surface_point(shape, edge_index, fraction)?;
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

fn candidate_for_endpoints_with_loads(
    s: &OrganismStructure,
    ua: usize,
    ub: usize,
    a: ConnectionEndpoint,
    b: ConnectionEndpoint,
    c: &[crate::resources::BaseResource],
    load_a: f64,
    load_b: f64,
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
        load_a,
        load_b,
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
.filter_map(|(a, b)| {
            candidate_for_endpoints_with_loads(
                s,
                ua,
                ub,
                a,
                b,
                c,
                s.connection_load(ua, a, c),
                s.connection_load(ub, b, c),
            )
        })
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
pub struct ConnectionCompatibilityCache {
    loads: Vec<(crate::structure::PhysicalConstituentId, ConnectionEndpoint, f64)>,
}

impl ConnectionCompatibilityCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_bond(
        &mut self,
        id_a: crate::structure::PhysicalConstituentId,
        endpoint_a: ConnectionEndpoint,
        id_b: crate::structure::PhysicalConstituentId,
        endpoint_b: ConnectionEndpoint,
        bond_energy: f64,
    ) {
        let load = crate::combine::experimental_bond_strength(bond_energy);
        self.loads.push((id_a, endpoint_a, load));
        self.loads.push((id_b, endpoint_b, load));
    }

    fn load(
        &self,
        id: crate::structure::PhysicalConstituentId,
        location: ConnectionEndpoint,
    ) -> Option<f64> {
        let mut found = false;
        let total = self
            .loads
            .iter()
            .filter(|(stored_id, stored_location, _)| {
                if *stored_id == id && stored_location.same_location(location) {
                    found = true;
                    true
                } else {
                    false
                }
            })
            .map(|(_, _, load)| *load)
            .sum();
        found.then_some(total)
    }
}

pub fn connection_pair_candidates_cached(
    s: &OrganismStructure,
    ua: usize,
    ub: usize,
    c: &[crate::resources::BaseResource],
    cache: &mut ConnectionCompatibilityCache,
) -> Vec<ConnectionPairCandidate> {
    let (Some(a), Some(b)) = (s.units.get(ua), s.units.get(ub)) else {
        return Vec::new();
    };
    candidate_endpoints(a, b, c)
        .into_iter()
        .filter_map(|(a, b)| {
            let id_a = s.physical_id(ua)?;
            let id_b = s.physical_id(ub)?;
            candidate_for_endpoints_with_loads(
                s,
                ua,
                ub,
                a,
                b,
                c,
                cache
                    .load(id_a, a)
                    .unwrap_or_else(|| s.connection_load(ua, a, c)),
                cache
                    .load(id_b, b)
                    .unwrap_or_else(|| s.connection_load(ub, b, c)),
            )
        })
        .collect()
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
