//! Checked polygon-union boundary arrangement and area (Phase 3A.4c).
//! Polygon inputs are validated by 3A.4a; ambiguity is rejected, never guessed.

use crate::phase3_boundary::checked_exposed_boundary_length;
use crate::phase3_polygon::{checked_polygon_area, PolygonArea};
use crate::resources::Form;
use crate::source_exposure::ExposedBoundaryInterval;

const ULP_FACTOR: f64 = 32.0;
const PARAM_EPS: f64 = 1.0e-10;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PolygonUnionArea {
    pub(crate) square_units: f64,
    pub(crate) uncertainty: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PolygonBoundaryLoop {
    /// Closed oriented vertices; first and last are equal.
    pub(crate) vertices: Vec<(f64, f64)>,
    /// Positive outer loops, negative hole loops.
    pub(crate) signed_area: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PolygonUnionBoundary {
    pub(crate) area: PolygonUnionArea,
    pub(crate) perimeter: f64,
    pub(crate) perimeter_uncertainty: f64,
    pub(crate) loops: Vec<PolygonBoundaryLoop>,
}

fn cross(a: (f64, f64), b: (f64, f64)) -> f64 { a.0*b.1-a.1*b.0 }
fn sub(a: (f64, f64), b: (f64, f64)) -> (f64,f64) { (a.0-b.0,a.1-b.1) }
fn add(a: (f64, f64), b: (f64, f64)) -> (f64,f64) { (a.0+b.0,a.1+b.1) }
fn mul(a: (f64, f64), k: f64) -> (f64,f64) { (a.0*k,a.1*k) }
fn distance(a:(f64,f64),b:(f64,f64))->f64 { sub(a,b).0.hypot(sub(a,b).1) }
fn finite(p:(f64,f64))->bool { p.0.is_finite() && p.1.is_finite() }

fn point_in_polygon(p:(f64,f64), v:&[(f64,f64)]) -> Option<bool> {
    let mut inside=false;
    for i in 0..v.len() {
        let a=v[i]; let b=v[(i+1)%v.len()];
        let ab=sub(b,a); let ap=sub(p,a);
        if cross(ab,ap)==0.0 && (p.0-a.0)*(p.0-b.0)+(p.1-a.1)*(p.1-b.1)<=0.0 {
            return None; // boundary sample: classification unresolved
        }
        if (a.1>p.1)!=(b.1>p.1) {
            let x=a.0+(p.1-a.1)*(b.0-a.0)/(b.1-a.1);
            if p.0<x { inside=!inside; }
        }
    }
    Some(inside)
}

fn union_contains(p:(f64,f64), polys:&[Vec<(f64,f64)>])->Option<bool> {
    let mut any=false;
    for poly in polys {
        match point_in_polygon(p,poly)? { true=>any=true, false=>{} }
    }
    Some(any)
}

fn edge_intersections(a:(f64,f64),b:(f64,f64),c:(f64,f64),d:(f64,f64),tol:f64)
    ->Option<Vec<(f64,f64)>> {
    let r=sub(b,a); let s=sub(d,c); let q=sub(c,a);
    let den=cross(r,s);
    let scale=r.0.hypot(r.1)*s.0.hypot(s.1);
    if !den.is_finite() || !scale.is_finite() { return None; }
    if den.abs()<=tol*scale.max(tol) {
        if cross(q,r).abs()>tol*r.0.hypot(r.1) { return Some(vec![]); }
        let rr=r.0*r.0+r.1*r.1; let ss=s.0*s.0+s.1*s.1;
        if rr<=0.0 || ss<=0.0 || !rr.is_finite() || !ss.is_finite(){return None;}
        let mut out=Vec::new();
        for p in [c,d] {
            let t=((p.0-a.0)*r.0+(p.1-a.1)*r.1)/rr;
            if t>=-PARAM_EPS && t<=1.0+PARAM_EPS {out.push((t.clamp(0.0,1.0),0.0));}
        }
        for p in [a,b] {
            let u=((p.0-c.0)*s.0+(p.1-c.1)*s.1)/ss;
            if u>=-PARAM_EPS && u<=1.0+PARAM_EPS {
                let t=((p.0-a.0)*r.0+(p.1-a.1)*r.1)/rr;
                out.push((t.clamp(0.0,1.0),u.clamp(0.0,1.0)));
            }
        }
        return Some(out);
    }
    let t=cross(q,s)/den; let u=cross(q,r)/den;
    if !t.is_finite()||!u.is_finite(){return None;}
    if t>=-PARAM_EPS&&t<=1.0+PARAM_EPS&&u>=-PARAM_EPS&&u<=1.0+PARAM_EPS {
        Some(vec![(t.clamp(0.0,1.0),u.clamp(0.0,1.0))])
    } else {Some(vec![])}
}

fn signed_area(vertices:&[(f64,f64)])->Option<(f64,f64)> {
    if vertices.len()<4 || vertices.first()!=vertices.last(){return None;}
    let origin=vertices[0]; let mut sum=0.0; let mut comp=0.0; let mut abs=0.0;
    let mut perimeter=0.0;
    for w in vertices.windows(2) {
        let a=sub(w[0],origin); let b=sub(w[1],origin);
        let term=cross(a,b);
        if !term.is_finite(){return None;}
        abs+=term.abs(); let y=term-comp; let t=sum+y; comp=(t-sum)-y; sum=t;
        perimeter+=distance(w[0],w[1]);
    }
    let area=sum*0.5;
    let uncertainty=f64::EPSILON*ULP_FACTOR*abs*0.5;
    (area.is_finite()&&perimeter.is_finite()&&area.abs()>uncertainty).then_some((area,perimeter))
}

/// Validate and measure a polygon union. Supports the strict 3A.4a convex
/// polygon forms. Any unresolved crossing, vertex weld, or loop topology fails.
pub(crate) fn checked_polygon_union(forms:&[Form])->Option<PolygonUnionBoundary>{
    if forms.is_empty(){return None;}
    let mut polys=Vec::with_capacity(forms.len());
    let mut edges=Vec::<(usize,usize,(f64,f64),(f64,f64))>::new();
    let mut area_bounds=0.0; let mut area_unc=0.0;
    let mut scale=0.0_f64; let mut coord=0.0_f64;
    for (pi,form) in forms.iter().enumerate(){
        let area:PolygonArea=checked_polygon_area(form)?;
        area_bounds+=area.square_units; area_unc+=area.uncertainty;
        let vertices=form.polygon_vertices()?;
        if vertices.iter().any(|p|!finite(*p)){return None;}
        // Form vertices are local coordinates; each form's authored polygon
        // coordinates define its placement within the union arrangement.
        let world=vertices.clone();
        for p in &world {coord=coord.max(p.0.abs()).max(p.1.abs());}
        for i in 0..world.len(){
            let a=world[i];let b=world[(i+1)%world.len()];
            let len=distance(a,b);if !len.is_finite()||len<=0.0{return None;}
            scale=scale.max(len);
            edges.push((pi,i,a,b));
        }
        polys.push(world);
    }
    let tol=coord.max(scale)*f64::EPSILON*ULP_FACTOR;
    if !tol.is_finite()||tol<=0.0{return None;}
    let mut cuts=vec![vec![0.0,1.0];edges.len()];
    for i in 0..edges.len(){
        for j in i+1..edges.len(){
            if edges[i].0==edges[j].0 {continue;}
            let hits=edge_intersections(edges[i].2,edges[i].3,edges[j].2,edges[j].3,tol)?;
            for (t,u) in hits {cuts[i].push(t);cuts[j].push(u);}
        }
    }
    let mut boundary=Vec::<((f64,f64),(f64,f64))>::new();
    for (ei,e) in edges.iter().enumerate(){
        let mut ts=cuts[ei].clone();ts.sort_by(f64::total_cmp);
        ts.dedup_by(|a,b|(*a-*b).abs()<=PARAM_EPS);
        let dir=sub(e.3,e.2);let len=distance(e.2,e.3);
        for pair in ts.windows(2){
            if pair[1]-pair[0]<=PARAM_EPS{continue;}
            let a=add(e.2,mul(dir,pair[0]));let b=add(e.2,mul(dir,pair[1]));
            let mid=mul(add(a,b),0.5);let tangent=mul(dir,1.0/len);
            let normal=(-tangent.1,tangent.0); // left side
            let mut nearest=f64::INFINITY;
            for (k,other) in edges.iter().enumerate(){
                if k==ei{continue;}
                let v=sub(other.3,other.2);let vv=v.0*v.0+v.1*v.1;
                if vv<=0.0{continue;}
                let t=(((mid.0-other.2.0)*v.0+(mid.1-other.2.1)*v.1)/vv).clamp(0.0,1.0);
                nearest=nearest.min(distance(mid,add(other.2,mul(v,t))));
            }
            let offset=(len*1.0e-7).min(nearest*0.2).max(tol*8.0);
            if !offset.is_finite()||offset>=nearest*0.45{return None;}
            let left=union_contains(add(mid,mul(normal,offset)),&polys)?;
            let right=union_contains(add(mid,mul(normal,-offset)),&polys)?;
            if left==right{continue;}
            let oriented=if left {(a,b)} else {(b,a)};
            // Coincident source polygons produce the same union edge more
            // than once. Keep one deterministic geometric copy before loop
            // assembly; perimeter validation independently guards duplicates.
            if !boundary.iter().any(|(u,v)|distance(*u,oriented.0)<=tol*4.0
                && distance(*v,oriented.1)<=tol*4.0) {
                boundary.push(oriented);
            }
        }
    }
    if boundary.is_empty(){return None;}
    // Stitch directed segments. At a vertex with multiple continuations the
    // angular successor is selected; equal/near-equal choices are ambiguous.
    let mut unused=vec![true;boundary.len()];
    let mut loops=Vec::new();let mut area_sum=0.0;let mut area_comp=0.0;
    let mut total_perimeter=0.0;
    while let Some(start_index)=unused.iter().position(|x|*x){
        let start=boundary[start_index].0;let mut current=start_index;
        let mut vertices=vec![start];let mut guard=0usize;
        loop{
            if !unused[current]{return None;}
            unused[current]=false;let (a,b)=boundary[current];
            if distance(*vertices.last()?,a)>tol*4.0{return None;}
            vertices.push(b);guard+=1;if guard>boundary.len(){return None;}
            if distance(b,start)<=tol*4.0{*vertices.last_mut()?=start;break;}
            let incoming=sub(b,a);let mut candidates=Vec::new();
            for (idx,(ca,cb)) in boundary.iter().enumerate(){
                if unused[idx]&&distance(*ca,b)<=tol*4.0{
                    let out=sub(*cb,*ca);
                    let angle=cross(incoming,out).atan2(incoming.0*out.0+incoming.1*out.1);
                    candidates.push((angle,idx));
                }
            }
            if candidates.is_empty(){return None;}
            candidates.sort_by(|x,y|x.0.total_cmp(&y.0));
            if candidates.len()>1&&(candidates[1].0-candidates[0].0).abs()<=1e-9{return None;}
            current=candidates[0].1;
        }
        let (signed,perim)=signed_area(&vertices)?;
        area_sum+=signed;total_perimeter+=perim;
        loops.push(PolygonBoundaryLoop{vertices,signed_area:signed});
    }
    if unused.iter().any(|x|*x){return None;}
    if area_sum<=0.0||!area_sum.is_finite(){return None;}
    let intervals:Vec<_>=loops.iter().flat_map(|lp|lp.vertices.windows(2).map(|w|{
        let d=sub(w[1],w[0]);let l=distance(w[0],w[1]);
        ExposedBoundaryInterval{body_index:0,unit_index:0,start:w[0],end:w[1],outward_normal:(d.1/l,-d.0/l)}
    })).collect();
    let checked=checked_exposed_boundary_length(&intervals)?;
    let discrepancy=(checked.world_units-total_perimeter).abs();
    if discrepancy>checked.uncertainty+tol*intervals.len() as f64{return None;}
    let uncertainty=area_unc+f64::EPSILON*ULP_FACTOR*area_bounds.max(area_sum);
    Some(PolygonUnionBoundary{
        area:PolygonUnionArea{square_units:area_sum,uncertainty},
        perimeter:checked.world_units,
        perimeter_uncertainty:checked.uncertainty,
        loops,
    })
}

#[cfg(test)]
mod tests{
 use super::*;
 fn rect(w:f64,h:f64)->Form{Form::Rectangle{width:w,height:h}}
 #[test] fn disjoint_and_overlap_are_measured_once(){
  let a=rect(2.,2.);let b=Form::Polygon{vertices:vec![(1.,0.),(3.,0.),(3.,2.),(1.,2.)]};
  let u=checked_polygon_union(&[a,b]).unwrap();assert!((u.area.square_units-6.).abs()<1e-8);
  assert!((u.perimeter-10.).abs()<1e-8);
 }
 #[test] fn contained_polygon_does_not_add_boundary(){
  let a=Form::Rectangle{width:4.,height:4.};let b=Form::Rectangle{width:1.,height:1.};
  let u=checked_polygon_union(&[a,b]).unwrap();assert!((u.area.square_units-16.).abs()<1e-8);
  assert!((u.perimeter-16.).abs()<1e-8);
 }
 #[test] fn duplicate_and_shared_edge(){
  let a=Form::Rectangle{width:2.,height:2.};let b=Form::Polygon{vertices:vec![(2.,0.),(4.,0.),(4.,2.),(2.,2.)]};
  assert!((checked_polygon_union(&[a.clone(),a]).unwrap().area.square_units-4.).abs()<1e-8);
  assert!((checked_polygon_union(&[Form::Rectangle{width:2.,height:2.},b]).unwrap().area.square_units-8.).abs()<1e-8);
 }
}
