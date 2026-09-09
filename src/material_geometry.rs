//! Physical geometry for environmental material instances.
//!
//! `Material` owns composition and physical structure. This module owns only
//! spatial realization of the material instance. Constituent ordering is an
//! implementation detail; attachment identity is carried by MaterialStructure.

use crate::resources::{BaseResource, Form, Material};
use crate::structure::Placement;

#[derive(Clone, Debug, PartialEq)]
pub struct PlacedMaterialPart { pub part_index: usize, pub form: Form, pub placement: Placement }

#[derive(Clone, Debug, PartialEq)]
pub struct MaterialGeometry { pub parts: Vec<PlacedMaterialPart>, pub min_x: f64, pub max_x: f64, pub min_y: f64, pub max_y: f64 }

#[derive(Clone, Debug, PartialEq)]
pub struct PhysicalMaterialInstance { pub material: Material, pub geometry: MaterialGeometry }

impl PhysicalMaterialInstance {
    pub fn new(material: Material, placements: &[Placement], catalog: &[BaseResource]) -> Option<Self> {
        Some(Self { geometry: MaterialGeometry::new(&material, placements, catalog)?, material })
    }
}

impl MaterialGeometry {
    pub fn new(material: &Material, placements: &[Placement], catalog: &[BaseResource]) -> Option<Self> {
        if !material.is_valid() || material.empty() { return None; }
        let resources: Vec<&BaseResource> = if let Some(structure) = material.structure() {
            structure.constituents.iter().map(|c| catalog.iter().find(|r| r.name == c.resource)).collect::<Option<_>>()?
        } else {
            if material.composition().len() != placements.len() { return None; }
            material.composition().iter().map(|c| catalog.iter().find(|r| r.name == c.resource)).collect::<Option<_>>()?
        };
        if resources.len() != placements.len() || resources.is_empty() { return None; }

        let mut parts = Vec::with_capacity(resources.len());
        let mut min_x = f64::INFINITY; let mut max_x = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY; let mut max_y = f64::NEG_INFINITY;
        for (part_index, (resource, placement)) in resources.into_iter().zip(placements.iter()).enumerate() {
            if !resource.shape.is_valid() || !placement.x.is_finite() || !placement.y.is_finite() || !placement.rotation_radians.is_finite() { return None; }
            let radius = resource.shape.form.rigid_bounding_radius()?;
            min_x = min_x.min(placement.x - radius); max_x = max_x.max(placement.x + radius);
            min_y = min_y.min(placement.y - radius); max_y = max_y.max(placement.y + radius);
            parts.push(PlacedMaterialPart { part_index, form: resource.shape.form.clone(), placement: *placement });
        }
        Some(Self { parts, min_x, max_x, min_y, max_y })
    }

    pub fn bounding_box_contains(&self, x: f64, y: f64) -> bool {
        x >= self.min_x && x <= self.max_x && y >= self.min_y && y <= self.max_y
    }
}

/// Physical overlap for rigid forms. Fluid deliberately has no invented
/// boundary; its geometry must be handled by a fluid/interface solver.
pub fn placed_forms_overlap(a: &PlacedMaterialPart, b: &PlacedMaterialPart, tolerance: f64) -> bool {
    if !tolerance.is_finite() || !a.placement.x.is_finite() || !a.placement.y.is_finite() || !b.placement.x.is_finite() || !b.placement.y.is_finite() { return false; }
    let tolerance = tolerance.max(0.0);
    let Some(ar) = a.form.rigid_bounding_radius() else { return false; };
    let Some(br) = b.form.rigid_bounding_radius() else { return false; };
    let distance = (a.placement.x - b.placement.x).hypot(a.placement.y - b.placement.y);
    if distance > ar + br + tolerance { return false; }
    match (&a.form, &b.form) {
        (Form::Circle { radius: ar }, Form::Circle { radius: br }) => distance <= ar + br + tolerance,
        (Form::Circle { radius }, polygon) => circle_polygon_overlap(a, *radius, b, polygon, tolerance),
        (polygon, Form::Circle { radius }) => circle_polygon_overlap(b, *radius, a, polygon, tolerance),
        _ => polygons_overlap(a, b, tolerance),
    }
}

fn circle_polygon_overlap(circle: &PlacedMaterialPart, radius: f64, polygon: &PlacedMaterialPart, form: &Form, tolerance: f64) -> bool {
    let Some(vertices) = world_polygon_vertices(form, polygon.placement) else { return false; };
    let center = (circle.placement.x, circle.placement.y);
    if point_in_polygon(center, &vertices) { return true; }
    vertices.iter().enumerate().any(|(i, &start)| point_segment_distance(center, start, vertices[(i + 1) % vertices.len()]) <= radius + tolerance)
}

fn polygons_overlap(a: &PlacedMaterialPart, b: &PlacedMaterialPart, tolerance: f64) -> bool {
    let Some(av) = world_polygon_vertices(&a.form, a.placement) else { return false; };
    let Some(bv) = world_polygon_vertices(&b.form, b.placement) else { return false; };
    let mut axes = polygon_axes(&av); axes.extend(polygon_axes(&bv));
    axes.into_iter().all(|(x, y)| { let (amin, amax) = project_polygon(&av, x, y); let (bmin, bmax) = project_polygon(&bv, x, y); !(amax + tolerance < bmin || bmax + tolerance < amin) })
}

fn world_polygon_vertices(form: &Form, placement: Placement) -> Option<Vec<(f64, f64)>> {
    let vertices = match form {
        Form::Line { length, radius } => vec![(-length/2.0, -radius), (length/2.0, -radius), (length/2.0, radius), (-length/2.0, radius)],
        other => other.polygon_vertices()?,
    };
    let (sin, cos) = placement.rotation_radians.sin_cos();
    Some(vertices.into_iter().map(|(x,y)| (placement.x + x*cos - y*sin, placement.y + x*sin + y*cos)).collect())
}

fn polygon_axes(vertices: &[(f64,f64)]) -> Vec<(f64,f64)> {
    vertices.iter().enumerate().map(|(i,&(x1,y1))| { let (x2,y2)=vertices[(i+1)%vertices.len()]; let ex=x2-x1; let ey=y2-y1; let l=ex.hypot(ey); (-ey/l,ex/l) }).collect()
}
fn project_polygon(vertices: &[(f64,f64)], x:f64, y:f64)->(f64,f64){vertices.iter().fold((f64::INFINITY,f64::NEG_INFINITY),|(min,max),&(px,py)|{let p=px*x+py*y;(min.min(p),max.max(p))})}
fn point_in_polygon((px,py):(f64,f64),v:&[(f64,f64)])->bool{let mut inside=false;for i in 0..v.len(){let (x1,y1)=v[i];let (x2,y2)=v[(i+1)%v.len()];if (y1>py)!=(y2>py)&&px<(x2-x1)*(py-y1)/(y2-y1)+x1{inside=!inside;}}inside}
fn point_segment_distance((px,py):(f64,f64),(sx,sy):(f64,f64),(ex,ey):(f64,f64))->f64{let dx=ex-sx;let dy=ey-sy;let l2=dx*dx+dy*dy;if l2<=f64::EPSILON{return(px-sx).hypot(py-sy)}let t=(((px-sx)*dx+(py-sy)*dy)/l2).clamp(0.0,1.0);(px-(sx+t*dx)).hypot(py-(sy+t*dy))}

#[cfg(test)]
mod tests {
    use super::*; use crate::resources::{default_catalog, Material};
    fn part(form:Form,x:f64,y:f64)->PlacedMaterialPart{PlacedMaterialPart{part_index:0,form,placement:Placement{x,y,rotation_radians:0.0}}}
    #[test] fn free_material_geometry_uses_composition(){let c=default_catalog();let m=Material::free_base("Carbon",1.0);let p=[Placement{x:2.0,y:3.0,rotation_radians:0.0}];let g=MaterialGeometry::new(&m,&p,&c).unwrap();assert_eq!(g.parts.len(),1);assert!(g.bounding_box_contains(2.0,3.0));}
    #[test] fn structured_material_geometry_uses_constituents(){let c=default_catalog();let s=crate::material_structure::MaterialStructure{constituents:vec![crate::material_structure::MaterialConstituent{id:crate::attachment::ConstituentId(1),resource:"Carbon".into()}],internal_bonds:vec![]};let m=Material{composition:vec![],structure:Some(s)};let p=[Placement{x:0.0,y:0.0,rotation_radians:0.0}];assert!(MaterialGeometry::new(&m,&p,&c).is_some());}
    #[test] fn fluid_is_not_given_rigid_collision_geometry(){let a=part(Form::Fluid{nominal_area:10.0},0.0,0.0);let b=part(Form::Circle{radius:1.0},0.0,0.0);assert!(!placed_forms_overlap(&a,&b,0.0));}
    #[test] fn separated_rigid_shapes_do_not_overlap(){let a=part(Form::Circle{radius:1.0},0.0,0.0);let b=part(Form::Circle{radius:1.0},3.0,0.0);assert!(!placed_forms_overlap(&a,&b,0.0));}
}
