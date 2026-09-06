//! Authoritative physical geometry derived from an organism's actual structure.

use crate::resources::{BaseResource, Form};
use crate::structure::OrganismStructure;

#[derive(Clone, Debug, PartialEq)]
pub struct PlacedForm { pub unit_index: usize, pub form: Form, pub x: f64, pub y: f64, pub rotation_radians: f64 }
#[derive(Clone, Debug, PartialEq)]
pub struct OrganismBodyGeometry { pub parts: Vec<PlacedForm>, pub min_x:f64, pub max_x:f64, pub min_y:f64, pub max_y:f64 }
impl OrganismBodyGeometry {
 pub fn from_structure(structure:&OrganismStructure,catalog:&[BaseResource])->Option<Self>{
  if structure.units.is_empty(){return None;} let mut parts=Vec::with_capacity(structure.units.len());let(mut min_x,mut max_x,mut min_y,mut max_y)=(f64::INFINITY,f64::NEG_INFINITY,f64::INFINITY,f64::NEG_INFINITY);
  for(i,unit)in structure.units.iter().enumerate(){let shape=unit.shape(catalog)?;if !shape.is_valid()||!unit.placement.x.is_finite()||!unit.placement.y.is_finite()||!unit.placement.rotation_radians.is_finite(){return None;}let r=shape.form.bounding_radius();min_x=min_x.min(unit.placement.x-r);max_x=max_x.max(unit.placement.x+r);min_y=min_y.min(unit.placement.y-r);max_y=max_y.max(unit.placement.y+r);parts.push(PlacedForm{unit_index:i,form:shape.form.clone(),x:unit.placement.x,y:unit.placement.y,rotation_radians:unit.placement.rotation_radians});}
  Some(Self{parts,min_x,max_x,min_y,max_y})
 }
 pub fn bounding_box_contains(&self,x:f64,y:f64)->bool{x>=self.min_x&&x<=self.max_x&&y>=self.min_y&&y<=self.max_y}
 pub fn bounding_radius_about(&self,x:f64,y:f64)->f64{self.parts.iter().map(|p|(p.x-x).hypot(p.y-y)+p.form.bounding_radius()).fold(0.0,f64::max)}
}
#[cfg(test)]mod tests{use super::*;use crate::resources::default_catalog;use crate::structure::{OrganismStructure,Placement,StructuralUnit};#[test]fn body_geometry_uses_unit_owned_material_geometry(){let c=default_catalog();let mut s=OrganismStructure::new();s.add_unit(StructuralUnit::new("Carbon",Placement{x:10.0,y:20.0,rotation_radians:0.0}));s.add_unit(StructuralUnit::new("Hydrogen",Placement{x:30.0,y:20.0,rotation_radians:0.0}));let b=OrganismBodyGeometry::from_structure(&s,&c).unwrap();assert_eq!(b.parts.len(),2);assert!(b.max_x>b.min_x)}#[test]fn composite_material_without_constituent_geometry_is_rejected(){let m=crate::resources::Material{parts:vec![("Carbon".into(),1.0),("Hydrogen".into(),1.0)],internal_bonds:vec![crate::resources::InternalBond{part_a:0,part_b:1}]};let mut s=OrganismStructure::new();s.add_unit(StructuralUnit::from_material(m,Placement{x:0.0,y:0.0,rotation_radians:0.0}).unwrap());assert!(OrganismBodyGeometry::from_structure(&s,&default_catalog()).is_none())}#[test]fn empty_structure_has_no_body(){assert!(OrganismBodyGeometry::from_structure(&OrganismStructure::new(),&default_catalog()).is_none())}#[test]fn invalid_catalog_geometry_rejects_body(){let mut c=default_catalog();c[0].shape.form=Form::Circle{radius:-1.0};let mut s=OrganismStructure::new();s.add_unit(StructuralUnit::new(c[0].name.clone(),Placement{x:0.0,y:0.0,rotation_radians:0.0}));assert!(OrganismBodyGeometry::from_structure(&s,&c).is_none())}}