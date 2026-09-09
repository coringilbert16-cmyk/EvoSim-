//! Physical realization of constituent attachment relationships.
//!
//! Genetic/material structure identifies physical attachment features; this
//! module derives relative placement from those features. It does not store
//! world-space coordinates in the material definition.
use crate::attachment::{AttachmentFeature,ConstituentAttachment,ConstituentId};
use crate::resources::{BaseResource,ConnectionSites,Form,Material};
use crate::structural_material::StructuralMaterial;
use crate::structure::Placement;

pub fn resolve_rigid_attachment(resource_a:&BaseResource,attachment_a:&ConstituentAttachment,resource_b:&BaseResource,attachment_b:&ConstituentAttachment)->Option<(Placement,Placement)>{let AttachmentFeature::Discrete(feature_a)=attachment_a.feature else{return None};let AttachmentFeature::Discrete(feature_b)=attachment_b.feature else{return None};resolve_rigid_discrete_features(resource_a,feature_a,resource_b,feature_b)}
fn discrete_connection_point(resource:&BaseResource,feature:u32)->Option<(f64,f64,f64)>{match resource.shape.connection_sites(){ConnectionSites::Corners(points)=>{let point=points.get(feature as usize)?;Some((point.x,point.y,point.direction_radians))},ConnectionSites::Circumference{..}|ConnectionSites::Undetermined=>None}}
pub fn resolve_rigid_discrete_features(resource_a:&BaseResource,feature_a:u32,resource_b:&BaseResource,feature_b:u32)->Option<(Placement,Placement)>{let(ax,ay,adir)=discrete_connection_point(resource_a,feature_a)?;let(bx,by,bdir)=discrete_connection_point(resource_b,feature_b)?;let rotation_b=normalize_angle(adir+std::f64::consts::PI-bdir);let(sin,cos)=rotation_b.sin_cos();let rotated_bx=bx*cos-by*sin;let rotated_by=bx*sin+by*cos;Some((Placement{x:0.0,y:0.0,rotation_radians:0.0},Placement{x:ax-rotated_bx,y:ay-rotated_by,rotation_radians:rotation_b}))}

/// Resolve a discrete attachment between physical constituents embedded inside
/// two structural units. The returned placements are unit placements, not
/// constituent placements. Internal constituent geometry is therefore honored
/// without making a first constituent a scaffold.
pub fn resolve_structural_constituent_attachment(material_a:&StructuralMaterial,constituent_a:ConstituentId,feature_a:u32,material_b:&StructuralMaterial,constituent_b:ConstituentId,feature_b:u32,catalog:&[BaseResource])->Option<(Placement,Placement)>{let resource_a_name=material_a.constituent_resource(constituent_a)?;let resource_b_name=material_b.constituent_resource(constituent_b)?;let resource_a=catalog.iter().find(|r|r.name==resource_a_name)?;let resource_b=catalog.iter().find(|r|r.name==resource_b_name)?;let local_a=material_a.rigid_constituent_placement(constituent_a,catalog)?;let local_b=material_b.rigid_constituent_placement(constituent_b,catalog)?;let(target_a,target_b)=resolve_rigid_discrete_features(resource_a,feature_a,resource_b,feature_b)?;Some((compose(inverse(local_a),target_a),compose(inverse(local_b),target_b)))}

pub fn resolve_structured_material_constituent_attachment(material_a:&Material,attachment_a:&ConstituentAttachment,material_b:&Material,attachment_b:&ConstituentAttachment,catalog:&[BaseResource])->Option<(Placement,Placement)>{let a=StructuralMaterial::from_material(material_a.clone())?;let b=StructuralMaterial::from_material(material_b.clone())?;let AttachmentFeature::Discrete(feature_a)=attachment_a.feature else{return None};let AttachmentFeature::Discrete(feature_b)=attachment_b.feature else{return None};resolve_structural_constituent_attachment(&a,attachment_a.constituent,feature_a,&b,attachment_b.constituent,feature_b,catalog)}

fn compose(parent:Placement,local:Placement)->Placement{let(sin,cos)=parent.rotation_radians.sin_cos();Placement{x:parent.x+local.x*cos-local.y*sin,y:parent.y+local.x*sin+local.y*cos,rotation_radians:parent.rotation_radians+local.rotation_radians}}
fn inverse(p:Placement)->Placement{let(sin,cos)=p.rotation_radians.sin_cos();Placement{x:-p.x*cos-p.y*sin,y:p.x*sin-p.y*cos,rotation_radians:-p.rotation_radians}}
fn normalize_angle(angle:f64)->f64{let tau=std::f64::consts::TAU;(angle+std::f64::consts::PI).rem_euclid(tau)-std::f64::consts::PI}
pub fn has_rigid_geometry(form:&Form)->bool{!matches!(form,Form::Fluid{..})}

#[cfg(test)]mod tests{use super::*;use crate::attachment::{AttachmentFeature,ConstituentAttachment};use crate::resources::default_catalog;use crate::material_structure::{InternalAttachmentBond,MaterialConstituent,MaterialStructure};use crate::attachment::ConstituentId;
fn resource<'a>(catalog:&'a[BaseResource],name:&str)->&'a BaseResource{catalog.iter().find(|resource|resource.name==name).unwrap()}
fn point(resource:&BaseResource,feature:u32)->(f64,f64){let ConnectionSites::Corners(points)=resource.shape.connection_sites()else{panic!("test resource does not expose a discrete connection feature")};let point=points.get(feature as usize).unwrap();(point.x,point.y)}
#[test]fn hydrogen_terminal_attachment_derives_contacting_placement(){let catalog=default_catalog();let carbon=resource(&catalog,"Carbon");let hydrogen=resource(&catalog,"Hydrogen");let carbon_attachment=ConstituentAttachment{constituent:ConstituentId(0),feature:AttachmentFeature::Discrete(0)};let hydrogen_attachment=ConstituentAttachment{constituent:ConstituentId(1),feature:AttachmentFeature::Discrete(0)};let(a,b)=resolve_rigid_attachment(carbon,&carbon_attachment,hydrogen,&hydrogen_attachment).unwrap();let ca=transform_point(point(carbon,0),a);let cb=transform_point(point(hydrogen,0),b);assert!((ca.0-cb.0).abs()<1e-12);assert!((ca.1-cb.1).abs()<1e-12)}
#[test]fn composite_constituent_attachment_uses_internal_geometry(){let catalog=default_catalog();let material=Material{composition:Vec::new(),structure:Some(MaterialStructure{constituents:vec![MaterialConstituent{id:ConstituentId(1),resource:"Carbon".into()},MaterialConstituent{id:ConstituentId(2),resource:"Carbon".into()}],internal_bonds:vec![InternalAttachmentBond{a:ConstituentAttachment{constituent:ConstituentId(1),feature:AttachmentFeature::Discrete(0)},b:ConstituentAttachment{constituent:ConstituentId(2),feature:AttachmentFeature::Discrete(3)}}]})};let sm=StructuralMaterial::from_material(material.clone()).unwrap();let p=resolve_structural_constituent_attachment(&sm,ConstituentId(1),1,&sm,ConstituentId(2),2,&catalog).unwrap();assert!(p.0.x.is_finite()&&p.1.x.is_finite())}
#[test]fn polygon_corner_identity_is_resolved_from_resource_geometry(){let catalog=default_catalog();let carbon=resource(&catalog,"Carbon");let(_,placement)=resolve_rigid_discrete_features(carbon,2,carbon,0).unwrap();assert!(placement.x.is_finite()&&placement.y.is_finite())}
#[test]fn invalid_discrete_feature_is_rejected(){let catalog=default_catalog();let carbon=resource(&catalog,"Carbon");assert!(resolve_rigid_discrete_features(carbon,99,carbon,0).is_none())}
#[test]fn fluid_has_no_rigid_geometry(){let catalog=default_catalog();let water=resource(&catalog,"Water");assert!(!has_rigid_geometry(&water.shape.form))}
#[test]fn continuous_attachment_is_not_faked_as_a_socket(){let catalog=default_catalog();let carbon=resource(&catalog,"Carbon");let water=resource(&catalog,"Water");let ca=ConstituentAttachment{constituent:ConstituentId(0),feature:AttachmentFeature::Boundary};let wa=ConstituentAttachment{constituent:ConstituentId(1),feature:AttachmentFeature::Fluid};assert!(resolve_rigid_attachment(carbon,&ca,water,&wa).is_none())}
}

fn transform_point(point:(f64,f64),placement:Placement)->(f64,f64){let(x,y)=point;let(sin,cos)=placement.rotation_radians.sin_cos(); (placement.x+x*cos-y*sin,placement.y+x*sin+y*cos)}
