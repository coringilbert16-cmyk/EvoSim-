//! Physical contact, accessibility, and structural connection candidates.
//!
//! Geometry/topology only. COMBINE owns energetic outcome, formation, and
//! bond strength. A connection point is finite: one occupied point cannot
//! accept another overlapping bond.

use std::collections::HashMap;
use crate::environment::ActiveMaterialField;
use crate::resources::{BaseResource, ConnectionPoint, ConnectionSites};
use crate::structure::{OrganismStructure, StructuralUnit};

#[derive(Clone, Copy, Debug)]
pub struct Envelope { pub x: f64, pub y: f64, pub radius: f64 }

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AccessibleFieldMaterial { pub field_index: usize, pub bonded: bool }

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldConnectionPoint { pub x: f64, pub y: f64, pub normal_x: f64, pub normal_y: f64 }

fn transform_point(point: ConnectionPoint, unit: &StructuralUnit) -> WorldConnectionPoint {
    let (s, c) = unit.placement.rotation_radians.sin_cos();
    let (nx, ny) = (point.direction_radians.cos(), point.direction_radians.sin());
    WorldConnectionPoint {
        x: unit.placement.x + point.x * c - point.y * s,
        y: unit.placement.y + point.x * s + point.y * c,
        normal_x: nx * c - ny * s,
        normal_y: nx * s + ny * c,
    }
}

fn distance(a: WorldConnectionPoint, b: WorldConnectionPoint) -> f64 { (a.x - b.x).hypot(a.y - b.y) }
fn facing(a: WorldConnectionPoint, b: WorldConnectionPoint) -> f64 { a.normal_x * -b.normal_x + a.normal_y * -b.normal_y }

pub fn broad_phase_field_cells(field: &ActiveMaterialField, envelope: Envelope) -> Vec<usize> { field.cells_within_radius(envelope.x, envelope.y, envelope.radius) }

pub fn accessible_field_material(field: &ActiveMaterialField, envelope: Envelope) -> Vec<AccessibleFieldMaterial> {
    broad_phase_field_cells(field, envelope).into_iter().flat_map(|field_index| {
        let cell = &field.cells[field_index];
        let mut found = Vec::with_capacity(2);
        if cell.bonded.total_amount() > 0.0 { found.push(AccessibleFieldMaterial { field_index, bonded: true }); }
        if cell.unbonded.total_amount() > 0.0 { found.push(AccessibleFieldMaterial { field_index, bonded: false }); }
        found
    }).collect()
}

pub fn broad_phase_structural_units(structure: &OrganismStructure, envelope: Envelope, broad_margin: f64) -> Vec<usize> {
    let cutoff = (envelope.radius + broad_margin.max(0.0)).max(0.0);
    let cutoff_sq = cutoff * cutoff;
    structure.units.iter().enumerate().filter_map(|(i, unit)| {
        let dx = unit.placement.x - envelope.x;
        let dy = unit.placement.y - envelope.y;
        (dx * dx + dy * dy <= cutoff_sq).then_some(i)
    }).collect()
}

pub fn unit_within_envelope(envelope: Envelope, unit: &StructuralUnit, catalog: &[BaseResource]) -> bool {
    let Some(base) = catalog.iter().find(|b| b.name == unit.resource_name) else { return false; };
    let dx = unit.placement.x - envelope.x;
    let dy = unit.placement.y - envelope.y;
    (dx * dx + dy * dy).sqrt() <= envelope.radius.max(0.0) + base.shape.form.bounding_radius()
}

pub fn candidate_units_in_envelope(structure: &OrganismStructure, envelope: Envelope, catalog: &[BaseResource], broad_margin: f64) -> Vec<usize> {
    broad_phase_structural_units(structure, envelope, broad_margin).into_iter().filter(|&i| unit_within_envelope(envelope, &structure.units[i], catalog)).collect()
}

pub fn world_connection_point(point: ConnectionPoint, unit: &StructuralUnit) -> WorldConnectionPoint { transform_point(point, unit) }

pub fn connection_points_contact(a: ConnectionPoint, unit_a: &StructuralUnit, b: ConnectionPoint, unit_b: &StructuralUnit, tolerance: f64, min_facing: f64) -> bool {
    let wa = transform_point(a, unit_a); let wb = transform_point(b, unit_b);
    distance(wa, wb) <= tolerance.max(0.0) && facing(wa, wb) >= min_facing
}

pub fn connection_point_distance(a: ConnectionPoint, unit_a: &StructuralUnit, b: ConnectionPoint, unit_b: &StructuralUnit) -> f64 { distance(transform_point(a, unit_a), transform_point(b, unit_b)) }

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConnectionPairCandidate {
    pub point_a: usize, pub point_b: usize,
    pub distance: f64, pub facing: f64,
    pub load_a: f64, pub load_b: f64,
    pub available_a: bool, pub available_b: bool,
}

pub fn connection_pair_candidates(structure: &OrganismStructure, unit_a: usize, unit_b: usize, catalog: &[BaseResource]) -> Vec<ConnectionPairCandidate> {
    let Some(a) = structure.units.get(unit_a) else { return Vec::new() };
    let Some(b) = structure.units.get(unit_b) else { return Vec::new() };
    let Some(ConnectionSites::Corners(points_a)) = a.connection_sites(catalog) else { return Vec::new() };
    let Some(ConnectionSites::Corners(points_b)) = b.connection_sites(catalog) else { return Vec::new() };
    let mut out = Vec::with_capacity(points_a.len().saturating_mul(points_b.len()));
    for (point_a, &pa) in points_a.iter().enumerate() { for (point_b, &pb) in points_b.iter().enumerate() {
        let wa = transform_point(pa, a); let wb = transform_point(pb, b);
        out.push(ConnectionPairCandidate {
            point_a, point_b, distance: distance(wa, wb), facing: facing(wa, wb),
            load_a: structure.connection_load(unit_a, point_a), load_b: structure.connection_load(unit_b, point_b),
            available_a: structure.connection_count(unit_a, point_a) == 0,
            available_b: structure.connection_count(unit_b, point_b) == 0,
        });
    }}
    out
}

pub fn contacting_connection_pair_candidates(structure: &OrganismStructure, unit_a: usize, unit_b: usize, catalog: &[BaseResource], tolerance: f64, min_facing: f64) -> Vec<ConnectionPairCandidate> {
    connection_pair_candidates(structure, unit_a, unit_b, catalog).into_iter().filter(|c| c.distance <= tolerance.max(0.0) && c.facing >= min_facing && c.available_a && c.available_b).collect()
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)] struct ConnectionTypeKey(String, String);
impl ConnectionTypeKey { fn new(a: &str, b: &str) -> Self { if a <= b { Self(a.into(), b.into()) } else { Self(b.into(), a.into()) } } }

#[derive(Clone, Debug, Default)] pub struct ConnectionCompatibilityCache { pairs: HashMap<ConnectionTypeKey, Vec<(usize, usize)>> }
impl ConnectionCompatibilityCache {
    pub fn new() -> Self { Self::default() }
    pub fn pairs_for_owned(&mut self, a: &str, b: &str, catalog: &[BaseResource]) -> Vec<(usize, usize)> {
        let reversed = a > b; let key = ConnectionTypeKey::new(a,b);
        if !self.pairs.contains_key(&key) { self.pairs.insert(key.clone(), Self::build_pairs(&key.0,&key.1,catalog)); }
        let pairs = self.pairs.get(&key).unwrap();
        if reversed { pairs.iter().map(|(x,y)| (*y,*x)).collect() } else { pairs.clone() }
    }
    fn build_pairs(a: &str, b: &str, catalog: &[BaseResource]) -> Vec<(usize,usize)> {
        let Some(a) = catalog.iter().find(|r| r.name == a) else { return Vec::new() };
        let Some(b) = catalog.iter().find(|r| r.name == b) else { return Vec::new() };
        let Some(ConnectionSites::Corners(pa)) = a.shape.connection_sites() else { return Vec::new() };
        let Some(ConnectionSites::Corners(pb)) = b.shape.connection_sites() else { return Vec::new() };
        (0..pa.len()).flat_map(|i| (0..pb.len()).map(move |j| (i,j))).collect()
    }
    pub fn len(&self) -> usize { self.pairs.len() }
    pub fn is_empty(&self) -> bool { self.pairs.is_empty() }
    pub fn clear(&mut self) { self.pairs.clear(); }
}

pub fn connection_pair_candidates_cached(structure: &OrganismStructure, unit_a: usize, unit_b: usize, catalog: &[BaseResource], cache: &mut ConnectionCompatibilityCache) -> Vec<ConnectionPairCandidate> {
    let Some(a) = structure.units.get(unit_a) else { return Vec::new() }; let Some(b) = structure.units.get(unit_b) else { return Vec::new() };
    let pairs = cache.pairs_for_owned(&a.resource_name,&b.resource_name,catalog);
    let Some(ConnectionSites::Corners(pa)) = a.connection_sites(catalog) else { return Vec::new() }; let Some(ConnectionSites::Corners(pb)) = b.connection_sites(catalog) else { return Vec::new() };
    pairs.into_iter().filter_map(|(ia,ib)| { let a_point=*pa.get(ia)?; let b_point=*pb.get(ib)?; let wa=transform_point(a_point,a); let wb=transform_point(b_point,b); Some(ConnectionPairCandidate { point_a:ia, point_b:ib, distance:distance(wa,wb), facing:facing(wa,wb), load_a:structure.connection_load(unit_a,ia), load_b:structure.connection_load(unit_b,ib), available_a:structure.connection_count(unit_a,ia)==0, available_b:structure.connection_count(unit_b,ib)==0 }) }).collect()
}

/// Safe structural mutation boundary. One connection point can hold one bond.
pub fn try_add_bond(structure: &mut OrganismStructure, bond: crate::structure::Bond, catalog: &[BaseResource]) -> Result<usize, &'static str> {
    if !structure.is_valid_bond(&bond,catalog) { return Err("invalid bond"); }
    if structure.connection_count(bond.unit_a,bond.point_a)!=0 || structure.connection_count(bond.unit_b,bond.point_b)!=0 { return Err("connection point already occupied"); }
    Ok(structure.add_bond(bond))
}

#[cfg(test)]
mod tests {
    use super::*; use crate::resources::default_catalog; use crate::structure::{Bond,Placement};
    #[test] fn occupied_points_are_not_available() {
        let catalog=default_catalog(); let mut s=OrganismStructure::new();
        let a=s.add_unit(StructuralUnit::new("Carbon",Placement{x:0.0,y:0.0,rotation_radians:0.0}));
        let b=s.add_unit(StructuralUnit::new("Carbon",Placement{x:1.0,y:0.0,rotation_radians:0.0}));
        let c=s.add_unit(StructuralUnit::new("Carbon",Placement{x:2.0,y:0.0,rotation_radians:0.0}));
        assert!(try_add_bond(&mut s,Bond{unit_a:a,point_a:0,unit_b:b,point_b:0,strength:0.5},&catalog).is_ok());
        assert_eq!(try_add_bond(&mut s,Bond{unit_a:a,point_a:0,unit_b:c,point_b:0,strength:0.5},&catalog),Err("connection point already occupied"));
    }
}
