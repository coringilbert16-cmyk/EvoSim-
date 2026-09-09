use serde::{Deserialize, Serialize};
use crate::math::{complexity, exponential_influence};
use crate::material_structure::MaterialStructure;

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct ResourceProperties { pub mass:f64, pub potential_energy:f64, pub reactivity:f64, pub cohesion:f64 }
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BaseResource { pub name:String, pub properties:ResourceProperties, pub shape:Shape }
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Form { Circle{radius:f64}, Line{length:f64,radius:f64}, Rectangle{width:f64,height:f64}, RegularPolygon{sides:u8,radius:f64}, Polygon{vertices:Vec<(f64,f64)>}, Fluid{nominal_area:f64} }
impl Form {
 pub fn is_valid(&self)->bool{match self{Self::Circle{radius}=>radius.is_finite()&&*radius>0.0,Self::Line{length,radius}=>length.is_finite()&&radius.is_finite()&&*length>0.0&&*radius>0.0,Self::Rectangle{width,height}=>width.is_finite()&&height.is_finite()&&*width>0.0&&*height>0.0,Self::RegularPolygon{sides,radius}=>*sides>=3&&radius.is_finite()&&*radius>0.0,Self::Polygon{vertices}=>vertices.len()>=3&&vertices.iter().all(|(x,y)|x.is_finite()&&y.is_finite()),Self::Fluid{nominal_area}=>nominal_area.is_finite()&&*nominal_area>0.0}}
 pub fn polygon_vertices(&self)->Option<Vec<(f64,f64)>>{match self{Self::Circle{..}|Self::Line{..}|Self::Fluid{..}=>None,Self::Rectangle{width,height}=>{let(hw,hh)=(width/2.0,height/2.0);Some(vec![(-hw,-hh),(hw,-hh),(hw,hh),(-hw,hh)])},Self::RegularPolygon{sides,radius}=>Some((0..*sides as usize).map(|k|{let a=k as f64*std::f64::consts::TAU/(*sides as f64);(radius*a.cos(),radius*a.sin())}).collect()),Self::Polygon{vertices}=>Some(vertices.clone())}}
 pub fn rigid_bounding_radius(&self)->Option<f64>{match self{Self::Circle{radius}=>Some(*radius),Self::Line{length,radius}=>Some(((length/2.0).powi(2)+radius.powi(2)).sqrt()),Self::Rectangle{width,height}=>Some(((width/2.0).powi(2)+(height/2.0).powi(2)).sqrt()),Self::RegularPolygon{radius,..}=>Some(*radius),Self::Polygon{vertices}=>Some(vertices.iter().map(|(x,y)|(x*x+y*y).sqrt()).fold(0.0,f64::max)),Self::Fluid{..}=>None}}
 pub fn bounding_radius(&self)->f64{self.rigid_bounding_radius().unwrap_or(0.0)}
}
#[derive(Serialize,Deserialize,Clone,Copy,Debug,PartialEq)]
pub struct ConnectionPoint{pub x:f64,pub y:f64,pub direction_radians:f64}
impl ConnectionPoint{pub fn is_valid(&self)->bool{self.x.is_finite()&&self.y.is_finite()&&self.direction_radians.is_finite()}}
#[derive(Serialize,Deserialize,Clone,Debug,PartialEq)]
pub enum ConnectionSites{Corners(Vec<ConnectionPoint>),Circumference{radius:f64},Undetermined}
#[derive(Serialize,Deserialize,Clone,Debug)]
pub struct Shape{pub form:Form}
impl Shape{pub fn is_valid(&self)->bool{self.form.is_valid()} pub fn connection_sites(&self)->ConnectionSites{match &self.form{Form::Circle{radius}=>ConnectionSites::Circumference{radius:*radius},Form::Line{length,..}=>ConnectionSites::Corners(vec![ConnectionPoint{x:-length/2.0,y:0.0,direction_radians:std::f64::consts::PI},ConnectionPoint{x:length/2.0,y:0.0,direction_radians:0.0}]),Form::Fluid{..}=>ConnectionSites::Undetermined,other=>ConnectionSites::Corners(other.polygon_vertices().unwrap().into_iter().map(|(x,y)|ConnectionPoint{x,y,direction_radians:y.atan2(x)}).collect())}}}
#[derive(Serialize,Deserialize,Clone,Copy,Debug)]
pub struct ResourceBaselines{pub mass:f64,pub potential_energy:f64,pub reactivity:f64,pub cohesion:f64}
impl ResourceBaselines{pub fn from_catalog(catalog:&[BaseResource])->Self{if catalog.is_empty(){return Self{mass:0.0,potential_energy:0.0,reactivity:0.0,cohesion:0.0}}let n=catalog.len()as f64;Self{mass:catalog.iter().map(|r|r.properties.mass).sum::<f64>()/n,potential_energy:catalog.iter().map(|r|r.properties.potential_energy).sum::<f64>()/n,reactivity:catalog.iter().map(|r|r.properties.reactivity).sum::<f64>()/n,cohesion:catalog.iter().map(|r|r.properties.cohesion).sum::<f64>()/n}}}
#[derive(Serialize,Deserialize,Clone,Debug,PartialEq)]
pub struct MaterialComponent{pub resource:String,pub amount:f64}
#[derive(Serialize,Deserialize,Clone,Debug,PartialEq)]
pub struct Material{pub composition:Vec<MaterialComponent>,pub structure:Option<MaterialStructure>}
impl Material{
 pub fn free_base(name:impl Into<String>,amount:f64)->Self{Self{composition:vec![MaterialComponent{resource:name.into(),amount}],structure:None}}
 pub fn is_structured(&self)->bool{self.structure.is_some()}
 pub fn has_internal_structure(&self)->bool{self.structure.as_ref().map_or(false,|s|!s.internal_bonds.is_empty())}
 pub fn is_connected(&self)->bool{self.structure.as_ref().map_or(self.composition.len()<=1,MaterialStructure::is_connected)}
 pub fn is_valid(&self)->bool{if self.composition.iter().any(|c|c.resource.is_empty()||!c.amount.is_finite()||c.amount<=0.0){return false}match &self.structure{None=>true,Some(s)=>self.composition.is_empty()&&s.is_valid()&&!s.constituents.is_empty()}}
 pub fn total_amount(&self)->f64{match &self.structure{Some(s)=>s.constituents.len()as f64,None=>self.composition.iter().map(|c|c.amount).sum()}}
 pub fn is_empty(&self)->bool{self.total_amount()<=1e-12}
 pub fn can_break(&self)->bool{self.structure.as_ref().map_or(false,|s|s.constituents.len()>=2&&!s.internal_bonds.is_empty())}
 pub fn potential_energy(&self,catalog:&[BaseResource])->f64{match &self.structure{Some(s)=>s.constituents.iter().map(|c|fresh_energy(catalog,&c.resource,1.0)).sum(),None=>self.composition.iter().map(|c|fresh_energy(catalog,&c.resource,c.amount)).sum()}}
 pub fn mass(&self,catalog:&[BaseResource])->f64{match &self.structure{Some(s)=>s.constituents.iter().map(|c|catalog.iter().find(|b|b.name==c.resource).map(|b|b.properties.mass).unwrap_or(0.0)).sum(),None=>self.composition.iter().map(|c|catalog.iter().find(|b|b.name==c.resource).map(|b|b.properties.mass*c.amount).unwrap_or(0.0)).sum()}}
 pub fn weighted_properties(&self,catalog:&[BaseResource])->ResourceProperties{let mut mass=0.0;let mut pe=0.0;let mut reac=0.0;let mut coh=0.0;let mut w=0.0;match &self.structure{Some(s)=>{for c in &s.constituents{if let Some(b)=catalog.iter().find(|b|b.name==c.resource){w+=1.0;mass+=b.properties.mass;pe+=b.properties.potential_energy;reac+=b.properties.reactivity;coh+=b.properties.cohesion}}},None=>{for c in &self.composition{if let Some(b)=catalog.iter().find(|b|b.name==c.resource){w+=c.amount;mass+=b.properties.mass*c.amount;pe+=b.properties.potential_energy*c.amount;reac+=b.properties.reactivity*c.amount;coh+=b.properties.cohesion*c.amount}}}}if w<=0.0{return ResourceProperties{mass:0.0,potential_energy:0.0,reactivity:0.0,cohesion:0.0}}ResourceProperties{mass:mass/w,potential_energy:pe/w,reactivity:reac/w,cohesion:coh/w}}
 pub fn take(&mut self,amount:f64)->Option<Material>{if self.is_structured(){return None}let total=self.total_amount();if amount<=0.0||total<=0.0{return None}let frac=amount.min(total)/total;let mut out=Vec::new();for c in &mut self.composition{let piece=c.amount*frac;c.amount-=piece;out.push(MaterialComponent{resource:c.resource.clone(),amount:piece})}self.composition.retain(|c|c.amount>1e-12);Some(Material{composition:out,structure:None})}
 pub fn composition(&self)->&[MaterialComponent]{&self.composition}
 pub fn structure(&self)->Option<&MaterialStructure>{self.structure.as_ref()}
}
pub fn merge_parts(parts:&[MaterialComponent])->Vec<MaterialComponent>{let mut out:Vec<MaterialComponent>=Vec::new();for c in parts{if let Some(e)=out.iter_mut().find(|e|e.resource==c.resource){e.amount+=c.amount}else{out.push(c.clone())}}out.retain(|c|c.amount>1e-12);out}
pub fn combine_work_cost(material:&Material,catalog:&[BaseResource],water_field:f64)->f64{let n=material.total_amount().max(2.0);let p=material.weighted_properties(catalog);let r=exponential_influence(effective_reactivity(p.reactivity,water_field));let c=p.cohesion.clamp(0.0,1.0);(complexity(n)*(1.0+c)*(1.25-r)).max(0.2)}
pub fn effective_reactivity(reactivity:f64,water_field:f64)->f64{reactivity/(1.0+water_field.max(0.0))}
pub fn property_ranges(catalog:&[BaseResource])->ResourceProperties{if catalog.is_empty(){return ResourceProperties{mass:1.0,potential_energy:1.0,reactivity:1.0,cohesion:1.0}}let(mut amin,mut amax)=(f64::INFINITY,f64::NEG_INFINITY);let(mut emin,mut emax)=(f64::INFINITY,f64::NEG_INFINITY);let(mut rmin,mut rmax)=(f64::INFINITY,f64::NEG_INFINITY);let(mut cmin,mut cmax)=(f64::INFINITY,f64::NEG_INFINITY);for r in catalog{amin=amin.min(r.properties.mass);amax=amax.max(r.properties.mass);emin=emin.min(r.properties.potential_energy);emax=emax.max(r.properties.potential_energy);let x=exponential_influence(r.properties.reactivity);rmin=rmin.min(x);rmax=rmax.max(x);cmin=cmin.min(r.properties.cohesion);cmax=cmax.max(r.properties.cohesion)}ResourceProperties{mass:(amax-amin).max(f64::EPSILON),potential_energy:(emax-emin).max(f64::EPSILON),reactivity:(rmax-rmin).max(f64::EPSILON),cohesion:(cmax-cmin).max(f64::EPSILON)}}
pub const NOMINAL_UNIT_AREA:f64=0.5;
pub fn default_catalog()->Vec<BaseResource>{vec![
BaseResource{name:"Carbon".into(),properties:ResourceProperties{mass:1.0,potential_energy:1.0,reactivity:0.1,cohesion:0.95},shape:Shape{form:Form::RegularPolygon{sides:6,radius:0.438691}}},
BaseResource{name:"Methane".into(),properties:ResourceProperties{mass:0.75,potential_energy:20.0,reactivity:4.0,cohesion:0.1},shape:Shape{form:Form::RegularPolygon{sides:3,radius:0.620403}}},
BaseResource{name:"Hydrogen".into(),properties:ResourceProperties{mass:0.25,potential_energy:12.0,reactivity:3.5,cohesion:0.05},shape:Shape{form:Form::Line{length:2.342920,radius:0.1}}},
BaseResource{name:"Sulfur".into(),properties:ResourceProperties{mass:1.5,potential_energy:8.0,reactivity:2.0,cohesion:0.45},shape:Shape{form:Form::RegularPolygon{sides:5,radius:0.458577}}},
BaseResource{name:"Nitrogen".into(),properties:ResourceProperties{mass:1.25,potential_energy:0.75,reactivity:0.35,cohesion:0.7},shape:Shape{form:Rectangle{width:1.511858,height:0.330719}}},
BaseResource{name:"Phosphorus".into(),properties:ResourceProperties{mass:1.75,potential_energy:1.5,reactivity:0.75,cohesion:0.6},shape:Shape{form:Form::Polygon{vertices:vec![(-0.408248,-0.408248),(0.408248,-0.408248),(0.408248,0.0),(0.0,0.0),(0.0,0.408248),(-0.408248,0.408248)]}}},
BaseResource{name:"Water".into(),properties:ResourceProperties{mass:1.0,potential_energy:0.0,reactivity:0.0,cohesion:0.5},shape:Shape{form:Form::Fluid{nominal_area:NOMINAL_UNIT_AREA}}}]}
pub fn fresh_energy(catalog:&[BaseResource],name:&str,amount:f64)->f64{catalog.iter().find(|b|b.name==name).map(|b|b.properties.potential_energy*amount).unwrap_or(0.0)}
#[cfg(test)]mod shape_tests{use super::*;#[test]fn catalog_still_constructs_with_seven_resources(){assert_eq!(default_catalog().len(),7)}#[test]fn every_catalog_resource_has_a_valid_shape(){for r in default_catalog(){assert!(r.shape.is_valid())}}#[test]fn fluid_has_no_rigid_bounding_radius(){let w=default_catalog().into_iter().find(|r|r.name=="Water").unwrap();assert_eq!(w.shape.form.rigid_bounding_radius(),None)}#[test]fn rigid_forms_have_rigid_bounding_radii(){for r in default_catalog(){if !matches!(r.shape.form,Form::Fluid{..}){assert!(r.shape.form.rigid_bounding_radius().unwrap().is_finite())}}}}
