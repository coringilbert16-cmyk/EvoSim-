//! Physical contact and structural connection candidates.
use std::collections::HashMap;
use crate::connection_geometry::{facing_compatibility, point_distance, transform_connection_point};
use crate::resources::{ConnectionPoint, ConnectionSites};
use crate::structure::{Bond, BondEndpoint, ConnectionEndpoint, OrganismStructure, StructuralUnit};

fn transform_point(point: ConnectionPoint, unit: &StructuralUnit) -> crate::connection_geometry::WorldConnectionPoint { transform_connection_point(point, unit.placement.x, unit.placement.y, unit.placement.rotation_radians) }
fn distance(a: crate::connection_geometry::WorldConnectionPoint, b: crate::connection_geometry::WorldConnectionPoint) -> f64 { point_distance(a, b) }
fn facing(a: crate::connection_geometry::WorldConnectionPoint, b: crate::connection_geometry::WorldConnectionPoint) -> f64 { facing_compatibility(a, b) }

pub fn world_connection_point(point: ConnectionPoint, unit: &StructuralUnit) -> crate::connection_geometry::WorldConnectionPoint { transform_point(point, unit) }
pub fn connection_points_contact(a: ConnectionPoint, unit_a: &StructuralUnit, b: ConnectionPoint, unit_b: &StructuralUnit, tolerance: f64, _min_facing: f64) -> bool { let wa = transform_point(a, unit_a); let wb = transform_point(b, unit_b); distance(wa, wb) <= tolerance.max(0.0) }
pub fn connection_point_distance(a: ConnectionPoint, unit_a: &StructuralUnit, b: ConnectionPoint, unit_b: &StructuralUnit) -> f64 { distance(transform_point(a, unit_a), transform_point(b, unit_b)) }

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConnectionPairCandidate { pub endpoint_a: ConnectionEndpoint, pub endpoint_b: ConnectionEndpoint, pub distance: f64, pub facing: f64, pub load_a: f64, pub load_b: f64, pub available_a: bool, pub available_b: bool }

fn world_center(unit: &StructuralUnit) -> crate::connection_geometry::WorldConnectionPoint { crate::connection_geometry::WorldConnectionPoint { x: unit.placement.x, y: unit.placement.y, normal_x: 0.0, normal_y: 0.0 } }
fn continuous_endpoint(unit: &StructuralUnit, target: crate::connection_geometry::WorldConnectionPoint, catalog: &[crate::resources::BaseResource]) -> Option<ConnectionEndpoint> {
    let dx = target.x - unit.placement.x;
    let dy = target.y - unit.placement.y;
    let len = dx.hypot(dy);
    let (ux, uy) = if len > 1e-12 { (dx / len, dy / len) } else { (1.0, 0.0) };
    let (s, c) = unit.placement.rotation_radians.sin_cos();
    let lx = ux * c + uy * s;
    let ly = -ux * s + uy * c;
    match unit.connection_sites(catalog)? {
        ConnectionSites::Circumference { .. } => Some(ConnectionEndpoint::Boundary { angle_radians: ly.atan2(lx) }),
        ConnectionSites::Undetermined => {
            let radius = unit.shape(catalog)?.form.bounding_radius();
            Some(ConnectionEndpoint::Fluid { x: lx * radius, y: ly * radius })
        }
        ConnectionSites::Corners(_) => None,
    }
}
fn candidate_endpoints(a: &StructuralUnit, b: &StructuralUnit, catalog: &[crate::resources::BaseResource]) -> Vec<(ConnectionEndpoint, ConnectionEndpoint)> {
    let Some(sa) = a.connection_sites(catalog) else { return Vec::new() };
    let Some(sb) = b.connection_sites(catalog) else { return Vec::new() };
    match (sa, sb) {
        (ConnectionSites::Corners(pa), ConnectionSites::Corners(pb)) => pa.iter().enumerate().flat_map(|(ia, _)| pb.iter().enumerate().map(move |(ib, _)| (ConnectionEndpoint::Corner { point_index: ia }, ConnectionEndpoint::Corner { point_index: ib }))).collect(),
        (ConnectionSites::Corners(pa), _) => {
            let mut out = Vec::new();
            let target = world_center(a);
            for (ia, point) in pa.iter().enumerate() {
                let wp = transform_point(*point, a);
                if let Some(ep) = continuous_endpoint(b, wp, catalog) { out.push((ConnectionEndpoint::Corner { point_index: ia }, ep)); }
            }
            if out.is_empty() { if let Some(ep) = continuous_endpoint(b, target, catalog) { out.push((ConnectionEndpoint::Corner { point_index: 0 }, ep)); } }
            out
        }
        (_, ConnectionSites::Corners(pb)) => {
            let mut out = Vec::new();
            let target = world_center(b);
            for (ib, point) in pb.iter().enumerate() {
                let wp = transform_point(*point, b);
                if let Some(ep) = continuous_endpoint(a, wp, catalog) { out.push((ep, ConnectionEndpoint::Corner { point_index: ib })); }
            }
            if out.is_empty() { if let Some(ep) = continuous_endpoint(a, target, catalog) { out.push((ep, ConnectionEndpoint::Corner { point_index: 0 })); } }
            out
        }
        (_, _) => {
            let wa = world_center(a);
            let wb = world_center(b);
            match (continuous_endpoint(a, wb, catalog), continuous_endpoint(b, wa, catalog)) { (Some(ea), Some(eb)) => vec![(ea, eb)], _ => Vec::new() }
        }
    }
}
fn endpoint_facing(a: ConnectionEndpoint, b: ConnectionEndpoint, ua: &StructuralUnit, ub: &StructuralUnit, catalog: &[crate::resources::BaseResource]) -> Option<f64> {
    if matches!(a, ConnectionEndpoint::Fluid { .. }) || matches!(b, ConnectionEndpoint::Fluid { .. }) { return Some(1.0); }
    Some(facing(a.world_point(ua, catalog)?, b.world_point(ub, catalog)?))
}
fn candidate_for_endpoints(s: &OrganismStructure, ua: usize, ub: usize, a: ConnectionEndpoint, b: ConnectionEndpoint, c: &[crate::resources::BaseResource]) -> Option<ConnectionPairCandidate> {
    let au = s.units.get(ua)?;
    let bu = s.units.get(ub)?;
    let wa = a.world_point(au, c)?;
    let wb = b.world_point(bu, c)?;
    let fa = endpoint_facing(a, b, au, bu, c)?;
    Some(ConnectionPairCandidate { endpoint_a: a, endpoint_b: b, distance: distance(wa, wb), facing: fa, load_a: s.connection_load(ua, a, c), load_b: s.connection_load(ub, b, c), available_a: true, available_b: true })
}

pub fn connection_pair_candidates(s: &OrganismStructure, ua: usize, ub: usize, c: &[crate::resources::BaseResource]) -> Vec<ConnectionPairCandidate> {
    let Some(a) = s.units.get(ua) else { return Vec::new() };
    let Some(b) = s.units.get(ub) else { return Vec::new() };
    candidate_endpoints(a, b, c).into_iter().filter_map(|(ea, eb)| candidate_for_endpoints(s, ua, ub, ea, eb, c)).collect()
}
pub fn contacting_connection_pair_candidates(s: &OrganismStructure, ua: usize, ub: usize, c: &[crate::resources::BaseResource], t: f64, m: f64) -> Vec<ConnectionPairCandidate> { connection_pair_candidates(s, ua, ub, c).into_iter().filter(|x| x.distance <= t.max(0.0) && x.facing >= m).collect() }

#[derive(Clone, Debug, PartialEq, Eq, Hash)] struct ConnectionTypeKey(String, String);
impl ConnectionTypeKey { fn new(a: &str, b: &str) -> Self { if a <= b { Self(a.into(), b.into()) } else { Self(b.into(), a.into()) } } }
#[derive(Clone, Debug, Default)] pub struct ConnectionCompatibilityCache { pairs: HashMap<ConnectionTypeKey, Vec<(usize, usize)>> }
impl ConnectionCompatibilityCache { pub fn new() -> Self { Self::default() } pub fn pairs_for_owned(&mut self, a: &str, b: &str, c: &[crate::resources::BaseResource]) -> Vec<(usize, usize)> { let rev = a > b; let k = ConnectionTypeKey::new(a, b); if !self.pairs.contains_key(&k) { self.pairs.insert(k.clone(), Self::build_pairs(&k.0, &k.1, c)); } let p = self.pairs.get(&k).unwrap(); if rev { p.iter().map(|(x, y)| (*y, *x)).collect() } else { p.clone() } } fn build_pairs(a: &str, b: &str, c: &[crate::resources::BaseResource]) -> Vec<(usize, usize)> { let Some(a) = c.iter().find(|r| r.name == a) else { return Vec::new() }; let Some(b) = c.iter().find(|r| r.name == b) else { return Vec::new() }; let ConnectionSites::Corners(pa) = a.shape.connection_sites() else { return Vec::new() }; let ConnectionSites::Corners(pb) = b.shape.connection_sites() else { return Vec::new() }; (0..pa.len()).flat_map(|i| (0..pb.len()).map(move |j| (i, j))).collect() } pub fn len(&self) -> usize { self.pairs.len() } pub fn is_empty(&self) -> bool { self.pairs.is_empty() } pub fn clear(&mut self) { self.pairs.clear() } }
pub fn connection_pair_candidates_cached(s: &OrganismStructure, ua: usize, ub: usize, c: &[crate::resources::BaseResource], cache: &mut ConnectionCompatibilityCache) -> Vec<ConnectionPairCandidate> {
    let Some(a) = s.units.get(ua) else { return Vec::new() };
    let Some(b) = s.units.get(ub) else { return Vec::new() };
    let continuous = !matches!(a.connection_sites(c), Some(ConnectionSites::Corners(_))) || !matches!(b.connection_sites(c), Some(ConnectionSites::Corners(_)));
    if continuous { return connection_pair_candidates(s, ua, ub, c); }
    let Some(ka) = a.resource_name() else { return Vec::new() };
    let Some(kb) = b.resource_name() else { return Vec::new() };
    cache.pairs_for_owned(ka, kb, c).into_iter().filter_map(|(ia, ib)| candidate_for_endpoints(s, ua, ub, ConnectionEndpoint::Corner { point_index: ia }, ConnectionEndpoint::Corner { point_index: ib }, c)).collect()
}

fn segment_crosses_transversely(a0: (f64, f64), a1: (f64, f64), b0: (f64, f64), b1: (f64, f64), eps: f64) -> bool { fn cross(ax: f64, ay: f64, bx: f64, by: f64) -> f64 { ax * by - ay * bx } let ax = a1.0 - a0.0; let ay = a1.1 - a0.1; let bx = b1.0 - b0.0; let by = b1.1 - b0.1; let c = cross(ax, ay, bx, by); if c.abs() <= eps { return false; } let c1 = cross(ax, ay, b0.0 - a0.0, b0.1 - a0.1); let c2 = cross(ax, ay, b1.0 - a0.0, b1.1 - a0.1); let c3 = cross(bx, by, a0.0 - b0.0, a0.1 - b0.1); let c4 = cross(bx, by, a1.0 - b0.0, a1.1 - b0.1); ((c1 > eps && c2 < -eps) || (c1 < -eps && c2 > eps)) && ((c3 > eps && c4 < -eps) || (c3 < -eps && c4 > eps)) }
fn bond_geometry_is_valid(s: &OrganismStructure, b: &Bond, c: &[crate::resources::BaseResource]) -> bool {
    let Some(a) = s.units.get(b.endpoint_a.unit_index).and_then(|u| b.endpoint_a.location.world_point(u, c)) else { return false };
    let Some(d) = s.units.get(b.endpoint_b.unit_index).and_then(|u| b.endpoint_b.location.world_point(u, c)) else { return false };
    if distance(a, d) <= 1e-12 { return true; }
    let eps = 1e-10;
    for existing in &s.bonds {
        if existing.has_same_identity(b) { continue; }
        let Some(e0) = s.units.get(existing.endpoint_a.unit_index).and_then(|u| existing.endpoint_a.location.world_point(u, c)) else { return false };
        let Some(e1) = s.units.get(existing.endpoint_b.unit_index).and_then(|u| existing.endpoint_b.location.world_point(u, c)) else { return false };
        if segment_crosses_transversely((a.x, a.y), (d.x, d.y), (e0.x, e0.y), (e1.x, e1.y), eps) { return false; }
        let same_line = ((d.y - a.y) * (e0.x - a.x) - (d.x - a.x) * (e0.y - a.y)).abs() <= eps && ((d.y - a.y) * (e1.x - a.x) - (d.x - a.x) * (e1.y)).abs() <= eps;
        if same_line { let len = distance(a, d); if len > 1e-12 { let ux = (d.x - a.x) / len; let uy = (d.y - a.y) / len; let t0 = (e0.x - a.x) * ux + (e0.y - a.y) * uy; let t1 = (e1.x - a.x) * ux + (e1.y - a.y) * uy; let lo = t0.min(t1).max(0.0); let hi = t0.max(t1).min(len); if hi - lo > 1e-10 { return false; } } }
    }
    true
}

/// The single structural bond-admission authority. Numerical connection occupancy is deliberately not part of this decision.
pub fn try_add_bond(s: &mut OrganismStructure, b: Bond, c: &[crate::resources::BaseResource]) -> Result<usize, &'static str> { if !s.is_valid_bond(&b, c) { return Err("invalid bond"); } if !bond_geometry_is_valid(s, &b, c) { return Err("bond geometry overlaps existing bond"); } Ok(s.push_bond_unchecked(b)) }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::default_catalog;
    use crate::structure::Placement;
    fn carbon_bond(a: usize, b: usize) -> Bond { Bond { endpoint_a: BondEndpoint { unit_index: a, location: ConnectionEndpoint::Corner { point_index: 0 } }, endpoint_b: BondEndpoint { unit_index: b, location: ConnectionEndpoint::Corner { point_index: 2 } }, strength: 0.05, bond_energy: 1.0 } }
    #[test] fn corner_contact_does_not_require_opposing_vertex_normals() { let catalog = default_catalog(); let a = StructuralUnit::new("Carbon", Placement { x: 0.0, y: 0.0, rotation_radians: std::f64::consts::FRAC_PI_6 }); let r = 0.438_691_f64; let b = StructuralUnit::new("Carbon", Placement { x: 3.0_f64.sqrt() * r, y: 0.0, rotation_radians: std::f64::consts::FRAC_PI_6 }); let pa = a.connection_sites(&catalog).unwrap(); let pb = b.connection_sites(&catalog).unwrap(); let (ConnectionSites::Corners(pa), ConnectionSites::Corners(pb)) = (pa, pb) else { panic!("Carbon must expose corners") }; assert!(connection_points_contact(pa[0], &a, pb[2], &b, 1e-9, 1.0 - 1e-9)); }
    #[test] fn repeated_bonds_at_one_contact_are_not_rejected_by_occupancy() { let catalog = default_catalog(); let mut s = OrganismStructure::new(); s.add_unit(StructuralUnit::new("Carbon", Placement { x: 0.0, y: 0.0, rotation_radians: std::f64::consts::FRAC_PI_6 })); s.add_unit(StructuralUnit::new("Carbon", Placement { x: 3.0_f64.sqrt() * 0.438_691, y: 0.0, rotation_radians: std::f64::consts::FRAC_PI_6 })); let first = carbon_bond(0, 1); let second = carbon_bond(0, 1); assert!(try_add_bond(&mut s, first, &catalog).is_ok()); assert!(try_add_bond(&mut s, second, &catalog).is_ok()); }
    #[test] fn hydrogen_uses_continuous_boundary_without_socket_indices() { let catalog = default_catalog(); let mut s = OrganismStructure::new(); s.add_unit(StructuralUnit::new("Hydrogen", Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 })); s.add_unit(StructuralUnit::new("Carbon", Placement { x: 0.5, y: 0.0, rotation_radians: 0.0 })); let candidates = connection_pair_candidates(&s, 0, 1, &catalog); assert_eq!(candidates.len(), 6); assert!(candidates.iter().all(|c| matches!(c.endpoint_a, ConnectionEndpoint::Boundary { .. }))); }
    #[test] fn water_uses_continuous_fluid_contact_region() { let catalog = default_catalog(); let mut s = OrganismStructure::new(); s.add_unit(StructuralUnit::new("Water", Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 })); s.add_unit(StructuralUnit::new("Hydrogen", Placement { x: 1.0, y: 0.0, rotation_radians: 0.0 })); let candidates = connection_pair_candidates(&s, 0, 1, &catalog); assert_eq!(candidates.len(), 1); assert!(matches!(candidates[0].endpoint_a, ConnectionEndpoint::Fluid { .. })); }
}