//! Geometry helpers and physical connection-region representation.
//! Continuous boundaries intentionally have no socket indices.
use crate::math::directional_compatibility;
use crate::resources::{ConnectionPoint, ConnectionSites};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ConnectionRegion {
    Corner(WorldConnectionPoint),
    Boundary { center_x: f64, center_y: f64, radius: f64 },
    Fluid { center_x: f64, center_y: f64, effective_radius: f64 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldConnectionPoint { pub x: f64, pub y: f64, pub normal_x: f64, pub normal_y: f64 }

pub fn transform_connection_point(point: ConnectionPoint, origin_x: f64, origin_y: f64, rotation_radians: f64) -> WorldConnectionPoint {
    let (s,c)=rotation_radians.sin_cos(); let (nx,ny)=(point.direction_radians.cos(),point.direction_radians.sin());
    WorldConnectionPoint{x:origin_x+point.x*c-point.y*s,y:origin_y+point.x*s+point.y*c,normal_x:nx*c-ny*s,normal_y:nx*s+ny*c}
}

/// Convert resource-level sites into physical regions without inventing
/// numbered sockets for continuous boundaries or fluid material.
pub fn transform_connection_regions(sites:&ConnectionSites,origin_x:f64,origin_y:f64,rotation_radians:f64,fluid_effective_radius:f64)->Vec<ConnectionRegion>{
    match sites {
        ConnectionSites::Corners(points)=>points.iter().copied().map(|p|ConnectionRegion::Corner(transform_connection_point(p,origin_x,origin_y,rotation_radians))).collect(),
        ConnectionSites::Circumference{radius}=>vec![ConnectionRegion::Boundary{center_x:origin_x,center_y:origin_y,radius:radius.max(0.0)}],
        ConnectionSites::Undetermined=>vec![ConnectionRegion::Fluid{center_x:origin_x,center_y:origin_y,effective_radius:fluid_effective_radius.max(0.0)}],
    }
}

pub fn point_distance(a:WorldConnectionPoint,b:WorldConnectionPoint)->f64{(a.x-b.x).hypot(a.y-b.y)}
pub fn facing_compatibility(a:WorldConnectionPoint,b:WorldConnectionPoint)->f64{directional_compatibility(a.normal_x,a.normal_y,-b.normal_x,-b.normal_y)}
pub fn within_contact_tolerance(a:WorldConnectionPoint,b:WorldConnectionPoint,tolerance:f64)->bool{point_distance(a,b)<=tolerance.max(0.0)}
impl ConnectionRegion {
 pub fn representative_point(self)->Option<WorldConnectionPoint>{match self{Self::Corner(p)=>Some(p),Self::Boundary{..}|Self::Fluid{..}=>None}}
 pub fn center(self)->(f64,f64){match self{Self::Corner(p)=>(p.x,p.y),Self::Boundary{center_x,center_y,..}|Self::Fluid{center_x,center_y,..}=>(center_x,center_y)}}
}
#[cfg(test)]mod tests{use super::*;use std::f64::consts::{FRAC_PI_2,PI};fn cp(x:f64,y:f64,d:f64)->ConnectionPoint{ConnectionPoint{x,y,direction_radians:d}}
#[test]fn transform_rotates_point_and_direction(){let r=transform_connection_point(cp(1.0,0.0,0.0),10.0,20.0,FRAC_PI_2);assert!((r.x-10.0).abs()<1e-12);assert!((r.y-21.0).abs()<1e-12);assert!(r.normal_x.abs()<1e-12);assert!((r.normal_y-1.0).abs()<1e-12)}
#[test]fn distance_is_euclidean(){let a=transform_connection_point(cp(0.0,0.0,0.0),0.0,0.0,0.0);let b=transform_connection_point(cp(0.0,0.0,0.0),3.0,4.0,0.0);assert!((point_distance(a,b)-5.0).abs()<1e-12)}
#[test]fn directly_facing_normals_have_maximum_compatibility(){let a=transform_connection_point(cp(0.0,0.0,0.0),0.0,0.0,0.0);let b=transform_connection_point(cp(0.0,0.0,PI),1.0,0.0,0.0);assert!((facing_compatibility(a,b)-1.0).abs()<1e-12)}
#[test]fn perpendicular_surfaces_have_zero_compatibility(){let a=transform_connection_point(cp(0.0,0.0,0.0),0.0,0.0,0.0);let b=transform_connection_point(cp(0.0,0.0,FRAC_PI_2),1.0,0.0,0.0);assert!(facing_compatibility(a,b).abs()<1e-12)}
#[test]fn continuous_regions_have_no_fake_socket_identity(){let b=ConnectionRegion::Boundary{center_x:1.0,center_y:2.0,radius:3.0};let f=ConnectionRegion::Fluid{center_x:4.0,center_y:5.0,effective_radius:6.0};assert!(b.representative_point().is_none());assert!(f.representative_point().is_none());assert_eq!(b.center(),(1.0,2.0));assert_eq!(f.center(),(4.0,5.0))}
#[test]fn continuous_sites_map_to_regions(){let b=transform_connection_regions(&ConnectionSites::Circumference{radius:2.0},3.0,4.0,0.0,0.0);assert_eq!(b,vec![ConnectionRegion::Boundary{center_x:3.0,center_y:4.0,radius:2.0}]);let f=transform_connection_regions(&ConnectionSites::Undetermined,3.0,4.0,0.0,5.0);assert_eq!(f,vec![ConnectionRegion::Fluid{center_x:3.0,center_y:4.0,effective_radius:5.0}])}
#[test]fn negative_tolerance_does_not_create_contact(){let a=transform_connection_point(cp(0.0,0.0,0.0),0.0,0.0,0.0);let b=transform_connection_point(cp(0.0,0.0,0.0),1.0,0.0,0.0);assert!(!within_contact_tolerance(a,b,-1.0))}}
