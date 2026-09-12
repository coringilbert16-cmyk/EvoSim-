//! Continuous enclosed-region analysis for realized organism geometry.

use crate::resources::{BaseResource, Form};
use crate::state::Organism;
use crate::structure::{OrganismStructure, Placement};
use std::collections::HashSet;

pub const MIN_CAVITY_AREA_FRACTION: f64 = 0.03; // EXPERIMENTAL: subject to balancing.
const EPS: f64 = 1e-9;

#[derive(Clone, Copy, Debug)]
enum PrimitiveGeometry { Segment { a: Point, b: Point }, Circle { center: Point, radius: f64 } }
#[derive(Clone, Copy, Debug)]
struct Primitive { unit_index: usize, geometry: PrimitiveGeometry }
#[derive(Clone, Copy, Debug, PartialEq)]
struct Point { x: f64, y: f64 }
impl Point {
    fn add(self, other: Self) -> Self { Self { x: self.x + other.x, y: self.y + other.y } }
    fn sub(self, other: Self) -> Self { Self { x: self.x - other.x, y: self.y - other.y } }
    fn scale(self, s: f64) -> Self { Self { x: self.x * s, y: self.y * s } }
    fn cross(self, other: Self) -> f64 { self.x * other.y - self.y * other.x }
    fn dot(self, other: Self) -> f64 { self.x * other.x + self.y * other.y }
    fn norm(self) -> f64 { self.dot(self).sqrt() }
}
#[derive(Clone, Debug)]
struct Cut { parameter: f64, point: Point }
#[derive(Clone, Copy, Debug)]
struct HalfEdge { from: usize, to: usize, twin: usize, unit_index: usize, geometry: PieceGeometry }
#[derive(Clone, Copy, Debug)]
enum PieceGeometry { Segment { a: Point, b: Point }, Arc { center: Point, radius: f64, start: f64, end: f64 } }

#[derive(Clone, Debug)]
pub struct EnclosedRegion { pub area: f64, pub boundary_units: Vec<usize>, pub bonded: bool }
#[derive(Clone, Debug, Default)]
pub struct GeometryTopology { pub occupied_area: f64, pub enclosed_regions: Vec<EnclosedRegion> }

impl GeometryTopology {
    pub fn total_enclosed_area(&self) -> f64 { self.enclosed_regions.iter().map(|r| r.area).sum() }
    pub fn qualifying_enclosed_area(&self, structure: &OrganismStructure, catalog: &[BaseResource]) -> f64 {
        let minimum_unit_area = structure.units.iter().filter_map(|u| u.shape(catalog).and_then(|s| form_area(&s.form))).filter(|a| *a > EPS).fold(f64::INFINITY, f64::min);
        if !minimum_unit_area.is_finite() { return 0.0; }
        self.enclosed_regions.iter().filter(|r| r.bonded && r.area + EPS >= minimum_unit_area).map(|r| r.area).sum()
    }
    pub fn has_genome_bearing_enclosure(&self, structure: &OrganismStructure, catalog: &[BaseResource]) -> bool {
        self.occupied_area > EPS && self.qualifying_enclosed_area(structure, catalog) + EPS >= self.occupied_area * MIN_CAVITY_AREA_FRACTION
    }
}

pub fn analyze(structure: &OrganismStructure, catalog: &[BaseResource]) -> Result<GeometryTopology, String> {
    let primitives = extract_primitives(structure, catalog)?;
    if primitives.is_empty() { return Ok(GeometryTopology::default()); }
    let mut cuts: Vec<Vec<Cut>> = primitives.iter().map(initial_cuts).collect();
    for i in 0..primitives.len() { for j in (i + 1)..primitives.len() {
        if primitives[i].unit_index == primitives[j].unit_index { continue; }
        for (pi, pj) in intersections(primitives[i].geometry, primitives[j].geometry) {
            cuts[i].push(Cut { parameter: pi, point: point_at(primitives[i].geometry, pi) });
            cuts[j].push(Cut { parameter: pj, point: point_at(primitives[j].geometry, pj) });
        }
    }}
    let mut nodes = Vec::<Point>::new();
    let mut normalized = Vec::<Vec<(f64, usize)>>::with_capacity(cuts.len());
    for primitive_cuts in cuts {
        let mut local = primitive_cuts;
        local.sort_by(|a, b| a.parameter.total_cmp(&b.parameter));
        let mut entries = Vec::new();
        for cut in local {
            if entries.iter().any(|(p, _): &(f64, usize)| (*p - cut.parameter).abs() <= EPS) { continue; }
            entries.push((cut.parameter, node_for(&mut nodes, cut.point)));
        }
        normalized.push(entries);
    }
    let mut edges = Vec::<HalfEdge>::new();
    for (i, primitive) in primitives.iter().enumerate() {
        let entries = &normalized[i];
        match primitive.geometry {
            PrimitiveGeometry::Segment { .. } => for pair in entries.windows(2) {
                if pair[0].1 != pair[1].1 { add_edge_pair(&mut edges, primitive.unit_index, piece_between(primitive.geometry, pair[0].0, pair[1].0)); }
            },
            PrimitiveGeometry::Circle { .. } => {
                if entries.len() == 1 { add_edge_pair(&mut edges, primitive.unit_index, piece_between(primitive.geometry, entries[0].0, entries[0].0 + std::f64::consts::TAU)); }
                else if entries.len() > 1 { for j in 0..entries.len() {
                    let start = entries[j].0; let mut end = entries[(j + 1) % entries.len()].0;
                    if j + 1 == entries.len() { end += std::f64::consts::TAU; }
                    if end - start > EPS { add_edge_pair(&mut edges, primitive.unit_index, piece_between(primitive.geometry, start, end)); }
                }}
            }
        }
    }
    let faces = trace_faces(&edges);
    let mut topology = GeometryTopology::default();
    for face in faces {
        let area = face.iter().map(|&i| edge_area(edges[i].geometry)).sum::<f64>();
        if area <= EPS { continue; }
        let sample = face_sample(&edges[face[0]].geometry);
        let occupied = primitives.iter().any(|p| point_inside(p.geometry, sample));
        if occupied { topology.occupied_area += area; }
        else {
            let boundary_units = face.iter().map(|&i| edges[i].unit_index).collect::<Vec<_>>();
            topology.enclosed_regions.push(EnclosedRegion { area, boundary_units: boundary_units.clone(), bonded: bonded_boundary(&boundary_units, structure) });
        }
    }
    Ok(topology)
}

pub fn organism_has_genome_bearing_enclosure(organism: &Organism, catalog: &[BaseResource]) -> Result<bool, String> {
    let topology = analyze(&organism.structure, catalog)?;
    Ok(topology.has_genome_bearing_enclosure(&organism.structure, catalog))
}

fn extract_primitives(structure: &OrganismStructure, catalog: &[BaseResource]) -> Result<Vec<Primitive>, String> {
    let mut out = Vec::new();
    for (unit_index, unit) in structure.units.iter().enumerate() {
        let shape = unit.shape(catalog).ok_or_else(|| format!("unit {unit_index} has no realized geometry"))?;
        let placement = unit.placement;
        match shape.form {
            Form::Circle { radius } => out.push(Primitive { unit_index, geometry: PrimitiveGeometry::Circle { center: Point { x: placement.x, y: placement.y }, radius } }),
            Form::Line { length } => out.push(Primitive { unit_index, geometry: PrimitiveGeometry::Segment { a: transform(Point { x: -length / 2.0, y: 0.0 }, placement), b: transform(Point { x: length / 2.0, y: 0.0 }, placement) } }),
            Form::Rectangle { width, height } => add_polygon_edges(&mut out, unit_index, &rectangle_vertices(width, height), placement),
            Form::RegularPolygon { sides, radius } => {
                let vertices = (0..sides as usize).map(|i| { let a = i as f64 * std::f64::consts::TAU / sides as f64; Point { x: radius * a.cos(), y: radius * a.sin() } }).collect::<Vec<_>>();
                add_polygon_edges(&mut out, unit_index, &vertices, placement);
            }
            Form::Polygon { vertices } => add_polygon_edges(&mut out, unit_index, &vertices.into_iter().map(|(x,y)| Point {x,y}).collect::<Vec<_>>(), placement),
            Form::Fluid { nominal_area } => out.push(Primitive { unit_index, geometry: PrimitiveGeometry::Circle { center: Point { x: placement.x, y: placement.y }, radius: (nominal_area / std::f64::consts::PI).sqrt() } }),
        }
    }
    Ok(out)
}
fn rectangle_vertices(width: f64, height: f64) -> Vec<Point> { let (hw, hh) = (width / 2.0, height / 2.0); vec![Point{x:-hw,y:-hh},Point{x:hw,y:-hh},Point{x:hw,y:hh},Point{x:-hw,y:hh}] }
fn add_polygon_edges(out: &mut Vec<Primitive>, unit_index: usize, vertices: &[Point], placement: Placement) { if vertices.len() < 2 { return; } for i in 0..vertices.len() { out.push(Primitive { unit_index, geometry: PrimitiveGeometry::Segment { a: transform(vertices[i], placement), b: transform(vertices[(i+1)%vertices.len()], placement) } }); } }
fn transform(p: Point, placement: Placement) -> Point { let (s,c)=placement.rotation_radians.sin_cos(); Point{x:placement.x+p.x*c-p.y*s,y:placement.y+p.x*s+p.y*c} }
fn form_area(form: &Form) -> Option<f64> { match form { Form::Circle{radius}=>Some(std::f64::consts::PI*radius*radius), Form::Line{..}=>None, Form::Rectangle{width,height}=>Some(width*height), Form::RegularPolygon{sides,radius} if *sides>=3=>Some(0.5*(*sides as f64)*radius*radius*(std::f64::consts::TAU/(*sides as f64)).sin()), Form::Polygon{vertices} if vertices.len()>=3=>Some((0..vertices.len()).map(|i|vertices[i].0*vertices[(i+1)%vertices.len()].1-vertices[(i+1)%vertices.len()].0*vertices[i].1).sum::<f64>().abs()*0.5), Form::Fluid{nominal_area}=>Some(*nominal_area), _=>None } }
fn initial_cuts(p: &Primitive) -> Vec<Cut> { match p.geometry { PrimitiveGeometry::Segment{a,b}=>vec![Cut{parameter:0.0,point:a},Cut{parameter:1.0,point:b}], PrimitiveGeometry::Circle{center,radius}=>vec![Cut{parameter:0.0,point:Point{x:center.x+radius,y:center.y}}] } }
fn point_at(g: PrimitiveGeometry, t: f64) -> Point { match g { PrimitiveGeometry::Segment{a,b}=>a.add(b.sub(a).scale(t)), PrimitiveGeometry::Circle{center,radius}=>Point{x:center.x+radius*t.cos(),y:center.y+radius*t.sin()} } }
fn intersections(a: PrimitiveGeometry,b: PrimitiveGeometry)->Vec<(f64,f64)>{match(a,b){(PrimitiveGeometry::Segment{a,b},PrimitiveGeometry::Segment{a:c,b:d})=>segment_intersections(a,b,c,d),(PrimitiveGeometry::Segment{a,b},PrimitiveGeometry::Circle{center,radius})=>line_circle_intersections(a,b,center,radius).into_iter().map(|(t,p)|(t,circle_parameter(center,p))).collect(),(PrimitiveGeometry::Circle{center,radius},PrimitiveGeometry::Segment{a,b})=>line_circle_intersections(a,b,center,radius).into_iter().map(|(t,p)|(circle_parameter(center,p),t)).collect(),(PrimitiveGeometry::Circle{center:a,radius:ra},PrimitiveGeometry::Circle{center:b,radius:rb})=>circle_circle_intersections(a,ra,b,rb).into_iter().map(|p|(circle_parameter(a,p),circle_parameter(b,p))).collect()}}
fn segment_intersections(a:Point,b:Point,c:Point,d:Point)->Vec<(f64,f64)>{let r=b.sub(a);let s=d.sub(c);let den=r.cross(s);if den.abs()<=EPS{return Vec::new()}let q=c.sub(a);let t=q.cross(s)/den;let u=q.cross(r)/den;if t>=-EPS&&t<=1.0+EPS&&u>=-EPS&&u<=1.0+EPS{vec![(t.clamp(0.0,1.0),u.clamp(0.0,1.0))]}else{Vec::new()}}
fn line_circle_intersections(a:Point,b:Point,center:Point,radius:f64)->Vec<(f64,Point)>{let d=b.sub(a);let f=a.sub(center);let aa=d.dot(d);if aa<=EPS{return Vec::new()}let bb=2.0*f.dot(d);let cc=f.dot(f)-radius*radius;let disc=bb*bb-4.0*aa*cc;if disc < -EPS{return Vec::new()}let root=disc.max(0.0).sqrt();let mut out=Vec::new();for t in [(-bb-root)/(2.0*aa),(-bb+root)/(2.0*aa)]{if t>=-EPS&&t<=1.0+EPS{let t=t.clamp(0.0,1.0);if !out.iter().any(|(u,_):&(f64,Point)|(*u-t).abs()<=EPS){out.push((t,a.add(d.scale(t)))}}}out}
fn circle_circle_intersections(a:Point,ra:f64,b:Point,rb:f64)->Vec<Point>{let delta=b.sub(a);let d=delta.norm();if d<=EPS||d>ra+rb+EPS||d<(ra-rb).abs()-EPS{return Vec::new()}let x=(ra*ra-rb*rb+d*d)/(2.0*d);let h2=ra*ra-x*x;if h2 < -EPS{return Vec::new()}let h=h2.max(0.0).sqrt();let u=delta.scale(1.0/d);let p=a.add(u.scale(x));let perp=Point{x:-u.y,y:u.x};if h<=EPS{vec![p]}else{vec![p.add(perp.scale(h)),p.sub(perp.scale(h))]}}
fn circle_parameter(center:Point,p:Point)->f64{(p.y-center.y).atan2(p.x-center.x).rem_euclid(std::f64::consts::TAU)}
fn node_for(nodes:&mut Vec<Point>,p:Point)->usize{if let Some((i,_))=nodes.iter().enumerate().find(|(_,q)|q.sub(p).norm()<=1e-7){return i}nodes.push(p);nodes.len()-1}
fn piece_between(g:PrimitiveGeometry,start:f64,end:f64)->PieceGeometry{match g{PrimitiveGeometry::Segment{..}=>PieceGeometry::Segment{a:point_at(g,start),b:point_at(g,end)},PrimitiveGeometry::Circle{center,radius}=>PieceGeometry::Arc{center,radius,start,end}}}
fn add_edge_pair(edges:&mut Vec<HalfEdge>,unit_index:usize,geometry:PieceGeometry){let(a,b)=match geometry{PieceGeometry::Segment{a,b}=>(a,b),PieceGeometry::Arc{center,radius,start,end}=>(Point{x:center.x+radius*start.cos(),y:center.y+radius*start.sin()},Point{x:center.x+radius*end.cos(),y:center.y+radius*end.sin()})};let base=edges.len();edges.push(HalfEdge{from:0,to:0,twin:base+1,unit_index,geometry});edges.push(HalfEdge{from:0,to:0,twin:base,unit_index,geometry:reverse_piece(geometry)});let _=(a,b);}
fn reverse_piece(g:PieceGeometry)->PieceGeometry{match g{PieceGeometry::Segment{a,b}=>PieceGeometry::Segment{a:b,b:a},PieceGeometry::Arc{center,radius,start,end}=>PieceGeometry::Arc{center,radius,start:end,end:start}}}
fn edge_area(g:PieceGeometry)->f64{match g{PieceGeometry::Segment{a,b}=>0.5*a.cross(b),PieceGeometry::Arc{center,radius,start,end}=>{let d=end-start;0.5*(center.x*radius*(end.sin()-start.sin())+center.y*radius*(start.cos()-end.cos())+radius*radius*d)}}}
fn face_sample(g:&PieceGeometry)->Point{let(p,t)=match *g{PieceGeometry::Segment{a,b}=>(a.add(b).scale(0.5),b.sub(a)),PieceGeometry::Arc{center,radius,start,end}=>{let a=start+(end-start)*0.5;(Point{x:center.x+radius*a.cos(),y:center.y+radius*a.sin()},Point{x:-a.sin(),y:a.cos()})}};let n=t.norm().max(EPS);p.add(Point{x:-t.y/n,y:t.x/n}.scale(1e-7))}
fn point_inside(g:PrimitiveGeometry,p:Point)->bool{match g{PrimitiveGeometry::Circle{center,radius}=>p.sub(center).norm()<radius-EPS,PrimitiveGeometry::Segment{..}=>false}}
fn trace_faces(edges:&[HalfEdge])->Vec<Vec<usize>>{let mut points=Vec::<Point>::new();let mut endpoints=Vec::with_capacity(edges.len());for e in edges{let(a,b)=match e.geometry{PieceGeometry::Segment{a,b}=>(a,b),PieceGeometry::Arc{center,radius,start,end}=>(Point{x:center.x+radius*start.cos(),y:center.y+radius*start.sin()},Point{x:center.x+radius*end.cos(),y:center.y+radius*end.sin()})};endpoints.push((node_for(&mut points,a),node_for(&mut points,b)));}let mut local=edges.to_vec();for(i,(a,b))in endpoints.into_iter().enumerate(){local[i].from=a;local[i].to=b;}let mut outgoing=vec![Vec::<usize>::new();points.len()];for(i,e)in local.iter().enumerate(){outgoing[e.from].push(i);}let mut visited=vec![false;local.len()];let mut faces=Vec::new();for start in 0..local.len(){if visited[start]{continue}let mut face=Vec::new();let mut cur=start;for _ in 0..=local.len(){if visited[cur]{break}visited[cur]=true;face.push(cur);let e=local[cur];let incoming=tangent_at_end(e.geometry);let rev=(incoming.y.atan2(incoming.x)+std::f64::consts::PI).rem_euclid(std::f64::consts::TAU);let mut best=None;let mut best_delta=f64::INFINITY;for &candidate in &outgoing[e.to]{if candidate==e.twin{continue}let t=tangent_at_start(local[candidate].geometry);let a=t.y.atan2(t.x).rem_euclid(std::f64::consts::TAU);let d=(rev-a).rem_euclid(std::f64::consts::TAU);if d<best_delta{best_delta=d;best=Some(candidate)}}let Some(next)=best else{break};cur=next;if cur==start{break}}if cur==start&&!face.is_empty(){faces.push(face)}}faces}
fn tangent_at_start(g:PieceGeometry)->Point{match g{PieceGeometry::Segment{a,b}=>b.sub(a),PieceGeometry::Arc{start,..}=>Point{x:-start.sin(),y:start.cos()}}}
fn tangent_at_end(g:PieceGeometry)->Point{match g{PieceGeometry::Segment{a,b}=>b.sub(a),PieceGeometry::Arc{end,..}=>Point{x:-end.sin(),y:end.cos()}}}
fn bonded_boundary(units:&[usize],structure:&OrganismStructure)->bool{if units.is_empty(){return false}let mut bonds=HashSet::<(u64,u64)>::new();for bond in &structure.bonds{let a=bond.endpoint_a.constituent_id.0;let b=bond.endpoint_b.constituent_id.0;bonds.insert(if a<b{(a,b)}else{(b,a)});}for i in 0..units.len(){let a=units[i];let b=units[(i+1)%units.len()];if a==b{continue}let(Some(ua),Some(ub))=(structure.units.get(a),structure.units.get(b))else{return false};let ids=(ua.physical_id.0,ub.physical_id.0);let key=if ids.0<ids.1{ids}else{(ids.1,ids.0)};if !bonds.contains(&key){return false}}true}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genome::initial_genome;
    use crate::resources::default_catalog;
    #[test]
    fn seed_has_an_enclosed_region_in_continuous_geometry(){let c=default_catalog();let s=initial_genome().structural_blueprint.realize(&c).unwrap();let t=analyze(&s,&c).unwrap();assert!(t.total_enclosed_area()>0.0,"seed did not form a bounded empty region: {t:?}");}
    #[test]
    fn unbonded_seed_geometry_is_not_a_genome_bearing_structure(){let c=default_catalog();let mut s=initial_genome().structural_blueprint.realize(&c).unwrap();s.bonds.clear();let t=analyze(&s,&c).unwrap();assert!(t.enclosed_regions.iter().any(|r|!r.bonded));assert!(!t.has_genome_bearing_enclosure(&s,&c));}
    #[test]
    fn bonded_seed_geometry_meets_experimental_three_percent_rule(){let c=default_catalog();let s=initial_genome().structural_blueprint.realize(&c).unwrap();let t=analyze(&s,&c).unwrap();assert!(t.has_genome_bearing_enclosure(&s,&c),"seed topology={t:?}");}
}
