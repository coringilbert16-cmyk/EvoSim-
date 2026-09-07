//! Exact two-dimensional interface geometry.
//!
//! This module measures *shared boundary*, not volume overlap. Boundary
//! contact is therefore a prerequisite but is not itself a finite interface:
//! a point contact has zero interface length. The result is intentionally
//! independent of material composition and permeability; it is pure geometry.
//!
//! Supported rigid forms are the current circle and polygonal forms. Fluids
//! have no authoritative boundary geometry and therefore produce no finite
//! interface length.

use crate::material_geometry::PlacedMaterialPart;
use crate::resources::Form;

const GEOMETRIC_EPSILON: f64 = 1e-12;

/// Exact boundary length of one rigid placed form.
pub fn boundary_length(part: &PlacedMaterialPart) -> f64 {
    match &part.form {
        Form::Circle { radius } if radius.is_finite() && *radius > 0.0 => {
            std::f64::consts::TAU * radius
        }
        Form::Circle { .. } | Form::Fluid { .. } => 0.0,
        _form => world_polygon_vertices(part)
            .map(|vertices| polygon_perimeter(&vertices))
            .unwrap_or(0.0),
    }
}

pub fn shared_boundary_length(a:&PlacedMaterialPart,b:&PlacedMaterialPart,tolerance:f64)->f64{if !tolerance.is_finite()||tolerance<0.0{return 0.0}match(&a.form,&b.form){(Form::Circle{radius:ar},Form::Circle{radius:br})=>{if !ar.is_finite()||!br.is_finite()||*ar<=0.0||*br<=0.0{return 0.0}let centers_coincident=(a.placement.x-b.placement.x).hypot(a.placement.y-b.placement.y)<=tolerance.max(GEOMETRIC_EPSILON);if centers_coincident&&(*ar-*br).abs()<=tolerance.max(GEOMETRIC_EPSILON){return std::f64::consts::TAU*ar}0.0},(Form::Fluid{..},_)|(_,Form::Fluid{..})=>0.0,_=>polygon_shared_boundary_length(a,b,tolerance)}}

fn polygon_shared_boundary_length(a:&PlacedMaterialPart,b:&PlacedMaterialPart,tolerance:f64)->f64{let Some(av)=world_polygon_vertices(a)else{return 0.0};let Some(bv)=world_polygon_vertices(b)else{return 0.0};let mut total=0.0;for i in 0..av.len(){let a1=av[i];let a2=av[(i+1)%av.len()];for j in 0..bv.len(){let b1=bv[j];let b2=bv[(j+1)%bv.len()];total+=collinear_overlap_length(a1,a2,b1,b2,tolerance)}}total}
fn collinear_overlap_length(a:(f64,f64),b:(f64,f64),c:(f64,f64),d:(f64,f64),tolerance:f64)->f64{let ab=(b.0-a.0,b.1-a.1);let len=ab.0.hypot(ab.1);if len<=GEOMETRIC_EPSILON{return 0.0}let cross=(c.0-a.0)*ab.1-(c.1-a.1)*ab.0;let cross2=(d.0-a.0)*ab.1-(d.1-a.1)*ab.0;if cross.abs()>tolerance.max(GEOMETRIC_EPSILON)*len||cross2.abs()>tolerance.max(GEOMETRIC_EPSILON)*len{return 0.0}let t1=((c.0-a.0)*ab.0+(c.1-a.1)*ab.1)/(len*len);let t2=((d.0-a.0)*ab.0+(d.1-a.1)*ab.1)/(len*len);let overlap=(t1.max(0.0).min(1.0).min(t2.max(0.0).min(1.0))-(t1.max(0.0).min(1.0).max(t2.max(0.0).min(1.0)))).max(0.0);overlap*len}

fn world_polygon_vertices(part:&PlacedMaterialPart)->Option<Vec<(f64,f64)>>{crate::material_geometry::world_polygon_vertices(part)}
fn polygon_perimeter(vertices:&[(f64,f64)])->f64{vertices.iter().enumerate().map(|(i,a)|{let b=vertices[(i+1)%vertices.len()];(b.0-a.0).hypot(b.1-a.1)}).sum()}
#[cfg(test)]mod tests{use super::*;use crate::material_geometry::PlacedMaterialPart;use crate::resources::Form;use crate::structure::Placement;fn circle(r:f64,x:f64,y:f64)->PlacedMaterialPart{PlacedMaterialPart{part_index:0,form:Form::Circle{radius:r},placement:Placement{x,y,rotation_radians:0.0}}}#[test]fn circle_boundary_length_is_exact_circumference(){assert!((boundary_length(&circle(2.0,0.0,0.0))-std::f64::consts::TAU*2.0).abs()<1e-12)}#[test]fn tangent_circles_have_zero_shared_boundary_length(){assert_eq!(shared_boundary_length(&circle(1.0,0.0,0.0),&circle(1.0,2.0,0.0),0.0),0.0)}#[test]fn coincident_circles_share_their_entire_circumference(){assert!((shared_boundary_length(&circle(1.0,0.0,0.0),&circle(1.0,0.0,0.0),0.0)-std::f64::consts::TAU).abs()<1e-12)}}
