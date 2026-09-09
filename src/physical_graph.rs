use crate::resources::{BaseResource, ConnectionSites, Form, Material};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Placement { pub x: f64, pub y: f64, pub rotation_radians: f64 }

/// A physical attachment region. There is no authored occupancy limit.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum ConnectionEndpoint { Corner { point_index: usize }, Boundary { angle_radians: f64 } }
impl ConnectionEndpoint {
    pub fn same_location(self, other: Self) -> bool { match (self, other) {
        (Self::Corner { point_index: a }, Self::Corner { point_index: b }) => a == b,
        (Self::Boundary { angle_radians: a }, Self::Boundary { angle_radians: b }) => (a-b).abs() <= 1e-12,
        _ => false,
    }}
    fn world_point(self, placement: Placement, resource: &BaseResource) -> Option<crate::connection_geometry::WorldConnectionPoint> {
        let (x,y,nx,ny) = match self {
            Self::Corner { point_index } => { let ConnectionSites::Corners(points)=resource.shape.connection_sites() else{return None}; let p=*points.get(point_index)?; (p.x,p.y,p.direction_radians.cos(),p.direction_radians.sin()) }
            Self::Boundary { angle_radians } => { let ConnectionSites::Circumference{radius}=resource.shape.connection_sites() else{return None}; let (s,c)=angle_radians.sin_cos(); (radius*c,radius*s,c,s) }
        };
        let (s,c)=placement.rotation_radians.sin_cos();
        Some(crate::connection_geometry::WorldConnectionPoint{x:placement.x+x*c-y*s,y:placement.y+x*s+y*c,normal_x:nx*c-ny*s,normal_y:nx*s+ny*c})
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PhysicalConstituentId(pub u64);
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PhysicalConstituent { pub id: PhysicalConstituentId, pub resource_name: String, pub placement: Placement }
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PhysicalRelationship { pub constituent_a: PhysicalConstituentId, pub constituent_b: PhysicalConstituentId, pub endpoint_a: ConnectionEndpoint, pub endpoint_b: ConnectionEndpoint, pub strength: f64, #[serde(default)] pub bond_energy: f64 }
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhysicalAttachmentCandidate { pub constituent_a: PhysicalConstituentId, pub constituent_b: PhysicalConstituentId, pub endpoint_a: ConnectionEndpoint, pub endpoint_b: ConnectionEndpoint, pub distance: f64 }

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PhysicalConstituentGraph { constituents: Vec<PhysicalConstituent>, relationships: Vec<PhysicalRelationship>, next_id: u64 }
impl PhysicalConstituentGraph {
    pub fn new()->Self{Self::default()}
    pub fn constituents(&self)->&[PhysicalConstituent]{&self.constituents}
    pub fn relationships(&self)->&[PhysicalRelationship]{&self.relationships}
    pub fn constituent(&self,id:PhysicalConstituentId)->Option<&PhysicalConstituent>{self.constituents.iter().find(|c|c.id==id)}
    pub fn constituent_mut(&mut self,id:PhysicalConstituentId)->Option<&mut PhysicalConstituent>{self.constituents.iter_mut().find(|c|c.id==id)}
    pub fn add_constituent(&mut self,resource_name:impl Into<String>,placement:Placement)->PhysicalConstituentId{let id=PhysicalConstituentId(self.next_id);self.next_id=self.next_id.checked_add(1).expect("physical constituent id overflow");self.constituents.push(PhysicalConstituent{id,resource_name:resource_name.into(),placement});id}

    /// Materialization creates physical identities only. Legacy material bonds are not copied.
    pub fn materialize_material(&mut self,material:&Material,placement:Placement)->Result<Vec<PhysicalConstituentId>,&'static str>{self.materialize_parts(material,&vec![placement;material.parts.len()])}
    pub fn materialize_parts(&mut self,material:&Material,placements:&[Placement])->Result<Vec<PhysicalConstituentId>,&'static str>{
        if !material.is_valid()||material.is_empty(){return Err("material is invalid or empty")}
        if material.parts.len()!=placements.len(){return Err("one placement is required for each physical constituent")}
        if material.parts.iter().any(|(_,a)|(*a-1.0).abs()>1e-9){return Err("physical materialization requires one unit per constituent")}
        Ok(material.parts.iter().zip(placements).map(|((name,_),p)|self.add_constituent(name.clone(),*p)).collect())
    }
    fn resource<'a>(&self,id:PhysicalConstituentId,catalog:&'a[BaseResource])->Option<&'a BaseResource>{let c=self.constituent(id)?;catalog.iter().find(|r|r.name==c.resource_name)}
    fn endpoints(a:&PhysicalConstituent,b:&PhysicalConstituent,ra:&BaseResource,rb:&BaseResource)->Vec<(ConnectionEndpoint,ConnectionEndpoint)>{match(ra.shape.connection_sites(),rb.shape.connection_sites()){
        (ConnectionSites::Corners(pa),ConnectionSites::Corners(pb))=>pa.iter().enumerate().flat_map(|(ia,_)|pb.iter().enumerate().map(move|(ib,_)|(ConnectionEndpoint::Corner{point_index:ia},ConnectionEndpoint::Corner{point_index:ib}))).collect(),
        (ConnectionSites::Corners(pa),ConnectionSites::Circumference{..})=>pa.iter().enumerate().map(|(ia,_)|{let e=ConnectionEndpoint::Corner{point_index:ia};let w=e.world_point(a.placement,ra).unwrap();(e,Self::boundary_toward(b.placement,rb,w.x,w.y))}).collect(),
        (ConnectionSites::Circumference{..},ConnectionSites::Corners(pb))=>pb.iter().enumerate().map(|(ib,_)|{let e=ConnectionEndpoint::Corner{point_index:ib};let w=e.world_point(b.placement,rb).unwrap();(Self::boundary_toward(a.placement,ra,w.x,w.y),e)}).collect(),
        (ConnectionSites::Circumference{..},ConnectionSites::Circumference{..})=>vec![(Self::boundary_toward(a.placement,ra,b.placement.x,b.placement.y),Self::boundary_toward(b.placement,rb,a.placement.x,a.placement.y))],
        _=>Vec::new(),
    }}
    fn boundary_toward(p:Placement,r:&BaseResource,x:f64,y:f64)->ConnectionEndpoint{let dx=x-p.x;let dy=y-p.y;let(s,c)=p.rotation_radians.sin_cos();ConnectionEndpoint::Boundary{angle_radians:(-dx*s+dy*c).atan2(dx*c+dy*s)}}
    pub fn attachment_candidates(&self,a:PhysicalConstituentId,b:PhysicalConstituentId,catalog:&[BaseResource])->Vec<PhysicalAttachmentCandidate>{let(Some(a),Some(b))=(self.constituent(a),self.constituent(b))else{return Vec::new()};let(Some(ra),Some(rb))=(self.resource(a,catalog),self.resource(b,catalog))else{return Vec::new()};Self::endpoints(a,b,ra,rb).into_iter().filter_map(|(ea,eb)|{let wa=ea.world_point(a.placement,ra)?;let wb=eb.world_point(b.placement,rb)?;Some(PhysicalAttachmentCandidate{constituent_a:a,constituent_b:b,endpoint_a:ea,endpoint_b:eb,distance:(wa.x-wb.x).hypot(wa.y-wb.y)})}).collect()}

    /// Physical admission is geometry-driven. Bond count never limits an attachment.
    pub fn try_attach(&mut self,candidate:PhysicalAttachmentCandidate,strength:f64,bond_energy:f64,tolerance:f64,catalog:&[BaseResource])->Result<usize,&'static str>{
        let tol=tolerance.max(0.0);if candidate.distance>tol{return Err("physical endpoints are not in contact")};if !strength.is_finite()||strength<0.0||!bond_energy.is_finite()||bond_energy<0.0{return Err("attachment contains invalid physical values")};
        let(Some(a),Some(b))=(self.constituent(candidate.constituent_a).cloned(),self.constituent(candidate.constituent_b).cloned())else{return Err("unknown constituent")};
        let(Some(ra),Some(rb))=(self.resource(a.id,catalog),self.resource(b.id,catalog))else{return Err("unknown resource")};let(Some(wa),Some(wb))=(candidate.endpoint_a.world_point(a.placement,ra),candidate.endpoint_b.world_point(b.placement,rb))else{return Err("invalid endpoint")};
        if (wa.x-wb.x).hypot(wa.y-wb.y)>tol{return Err("physical endpoints are not in contact")};if rigid_overlap(&a,ra,&b,rb,1e-10){return Err("constituents strictly overlap")};
        for r in &self.relationships{let(Some(ea),Some(eb))=(self.constituent(r.constituent_a),self.constituent(r.constituent_b))else{return Err("invalid existing relationship")};let(Some(ra),Some(rb))=(self.resource(r.constituent_a,catalog),self.resource(r.constituent_b,catalog))else{return Err("unknown resource")};let(Some(pa),Some(pb))=(r.endpoint_a.world_point(ea.placement,ra),r.endpoint_b.world_point(eb.placement,rb))else{return Err("invalid existing endpoint")};if transverse((wa.x,wa.y),(wb.x,wb.y),(pa.x,pa.y),(pb.x,pb.y),1e-10)||collinear_overlap((wa.x,wa.y),(wb.x,wb.y),(pa.x,pa.y),(pb.x,pb.y),1e-10){return Err("attachment conflicts with an existing relationship")}}
        self.add_relationship(PhysicalRelationship{constituent_a:candidate.constituent_a,constituent_b:candidate.constituent_b,endpoint_a:candidate.endpoint_a,endpoint_b:candidate.endpoint_b,strength,bond_energy})
    }
    pub fn remove_constituent(&mut self,id:PhysicalConstituentId)->Option<PhysicalConstituent>{let i=self.constituents.iter().position(|c|c.id==id)?;self.relationships.retain(|r|r.constituent_a!=id&&r.constituent_b!=id);Some(self.constituents.remove(i))}
    pub fn add_relationship(&mut self,r:PhysicalRelationship)->Result<usize,&'static str>{if r.constituent_a==r.constituent_b{return Err("relationship requires two distinct constituents")};if self.constituent(r.constituent_a).is_none()||self.constituent(r.constituent_b).is_none(){return Err("relationship references an unknown constituent")};if !r.strength.is_finite()||r.strength<0.0||!r.bond_energy.is_finite()||r.bond_energy<0.0{return Err("relationship contains invalid physical values")};self.relationships.push(r);Ok(self.relationships.len()-1)}
    pub fn remove_relationship(&mut self,index:usize)->Option<PhysicalRelationship>{(index<self.relationships.len()).then(||self.relationships.remove(index))}
    pub fn relationships_touching(&self,id:PhysicalConstituentId)->impl Iterator<Item=&PhysicalRelationship>{self.relationships.iter().filter(move|r|r.constituent_a==id||r.constituent_b==id)}
    pub fn relationship_count_at(&self,id:PhysicalConstituentId,e:ConnectionEndpoint)->usize{self.relationships_touching(id).filter(|r|(r.constituent_a==id&&r.endpoint_a.same_location(e))||(r.constituent_b==id&&r.endpoint_b.same_location(e))).count()}
    pub fn connected_components(&self)->Vec<Vec<PhysicalConstituentId>>{let mut out=Vec::new();let mut unseen:std::collections::HashSet<_>=self.constituents.iter().map(|c|c.id).collect();while let Some(start)=unseen.iter().next().copied(){let mut stack=vec![start];unseen.remove(&start);let mut component=Vec::new();while let Some(id)=stack.pop(){component.push(id);for r in self.relationships_touching(id){let other=if r.constituent_a==id{r.constituent_b}else{r.constituent_a};if unseen.remove(&other){stack.push(other)}}}component.sort_unstable();out.push(component)}out}
    pub fn validate_resource_names(&self,catalog:&[BaseResource])->bool{self.constituents.iter().all(|c|catalog.iter().any(|r|r.name==c.resource_name))}
}

fn rigid_overlap(a:&PhysicalConstituent,ra:&BaseResource,b:&PhysicalConstituent,rb:&BaseResource,eps:f64)->bool{let d=(a.placement.x-b.placement.x).hypot(a.placement.y-b.placement.y);let ar=ra.shape.form.bounding_radius();let br=rb.shape.form.bounding_radius();if d>=ar+br-eps{return false}match(&ra.shape.form,&rb.shape.form){(Form::Circle{radius:x},Form::Circle{radius:y})=>d<x+y-eps,(Form::Circle{radius},p)=>circle_polygon_overlap(&a.placement,*radius,&b.placement,p,eps),(p,Form::Circle{radius})=>circle_polygon_overlap(&b.placement,*radius,&a.placement,p,eps),_=>polygon_overlap(&a.placement,&ra.shape.form,&b.placement,&rb.shape.form,eps)}}
fn world_vertices(f:&Form,p:&Placement)->Option<Vec<(f64,f64)>>{let v=f.polygon_vertices()?;let(s,c)=p.rotation_radians.sin_cos();Some(v.into_iter().map(|(x,y)|(p.x+x*c-y*s,p.y+x*s+y*c)).collect())}
fn polygon_axes(v:&[(f64,f64)])->Vec<(f64,f64)>{v.iter().enumerate().map(|(i,&(x,y))|{let(x2,y2)=v[(i+1)%v.len()];let(ex,ey)=(x2-x,y2-y);let l=ex.hypot(ey);(-ey/l,ex/l)}).collect()}
fn project(v:&[(f64,f64)],a:(f64,f64))->(f64,f64){v.iter().map(|(x,y)|x*a.0+y*a.1).fold((f64::INFINITY,f64::NEG_INFINITY),|(lo,hi),x|(lo.min(x),hi.max(x)))}
fn polygon_overlap(a:&Placement,fa:&Form,b:&Placement,fb:&Form,eps:f64)->bool{let(Some(va),Some(vb))=(world_vertices(fa,a),world_vertices(fb,b))else{return false};let mut axes=polygon_axes(&va);axes.extend(polygon_axes(&vb));axes.into_iter().all(|axis|{let(am,ax)=project(&va,axis);let(bm,bx)=project(&vb,axis);ax>bm+eps&&bx>am+eps})}
fn point_segment_distance(p:(f64,f64),a:(f64,f64),b:(f64,f64))->f64{let(dx,dy)=(b.0-a.0,b.1-a.1);let l=dx*dx+dy*dy;if l<=f64::EPSILON{return(p.0-a.0).hypot(p.1-a.1)}let t=(((p.0-a.0)*dx+(p.1-a.1)*dy)/l).clamp(0.0,1.0);(p.0-(a.0+t*dx)).hypot(p.1-(a.1+t*dy))}
fn point_in_polygon(p:(f64,f64),v:&[(f64,f64)])->bool{let mut inside=false;for i in 0..v.len(){let(x1,y1)=v[i];let(x2,y2)=v[(i+1)%v.len()];if(y1>p.1)!=(y2>p.1)&&p.0<(x2-x1)*(p.1-y1)/(y2-y1)+x1{inside=!inside}}inside}
fn circle_polygon_overlap(c:&Placement,r:f64,p:&Placement,f:&Form,eps:f64)->bool{let Some(v)=world_vertices(f,p)else{return false};if point_in_polygon((c.x,c.y),&v){return true}v.iter().enumerate().any(|(i,&a)|point_segment_distance((c.x,c.y),a,v[(i+1)%v.len()])<r-eps)}
fn transverse(a:(f64,f64),b:(f64,f64),c:(f64,f64),d:(f64,f64),eps:f64)->bool{fn cr(a:(f64,f64),b:(f64,f64),c:(f64,f64))->f64{(b.0-a.0)*(c.1-a.1)-(b.1-a.1)*(c.0-a.0)}let x=cr(a,b,c);let y=cr(a,b,d);let z=cr(c,d,a);let w=cr(c,d,b);x.abs()>eps&&y.abs()>eps&&z.abs()>eps&&w.abs()>eps&&x.signum()!=y.signum()&&z.signum()!=w.signum()}
fn collinear_overlap(a:(f64,f64),b:(f64,f64),c:(f64,f64),d:(f64,f64),eps:f64)->bool{let cross=(b.0-a.0)*(c.1-a.1)-(b.1-a.1)*(c.0-a.0);let cross2=(b.0-a.0)*(d.1-a.1)-(b.1-a.1)*(d.0-a.0);if cross.abs()>eps||cross2.abs()>eps{return false}let len=(b.0-a.0).hypot(b.1-a.1);if len<=eps{return false}let ux=(b.0-a.0)/len;let uy=(b.1-a.1)/len;let t0=(c.0-a.0)*ux+(c.1-a.1)*uy;let t1=(d.0-a.0)*ux+(d.1-a.1)*uy;t1.max(t0).min(len)-t0.min(t1).max(0.0)>eps}

#[cfg(test)]mod tests{use super::*;use crate::resources::{default_catalog,InternalBond};fn p(x:f64,y:f64)->Placement{Placement{x,y,rotation_radians:0.0}}
#[test]fn materialization_creates_individual_constituents_without_copying_legacy_structure(){let m=Material{parts:vec![("Carbon".into(),1.0),("Hydrogen".into(),1.0)],internal_bonds:vec![InternalBond{part_a:0,part_b:1}]};let mut g=PhysicalConstituentGraph::new();let ids=g.materialize_parts(&m,&[p(0.0,0.0),p(1.0,0.0)]).unwrap();assert_eq!(ids.len(),2);assert!(g.relationships().is_empty());assert_ne!(g.constituent(ids[0]).unwrap().placement,g.constituent(ids[1]).unwrap().placement)}
#[test]fn aggregate_material_cannot_become_one_physical_constituent(){let mut g=PhysicalConstituentGraph::new();assert!(g.materialize_material(&Material::free_base("Carbon",2.0),p(0.0,0.0)).is_err())}
#[test]fn water_is_a_circular_continuous_boundary(){let c=default_catalog();let w=c.iter().find(|r|r.name=="Water").unwrap();assert!(matches!(w.shape.form,Form::Circle{..}));assert!(matches!(w.shape.connection_sites(),ConnectionSites::Circumference{..}))}
#[test]fn attachment_uses_geometry_not_bond_count(){let c=default_catalog();let mut g=PhysicalConstituentGraph::new();let r=match c.iter().find(|x|x.name=="Carbon").unwrap().shape.form{Form::RegularPolygon{radius,..}=>radius,_=>0.0};let a=g.add_constituent("Carbon",p(0.0,0.0));let b=g.add_constituent("Carbon",p(2.0*r,0.0));let cs=g.attachment_candidates(a,b,&c);let candidate=cs.into_iter().min_by(|x,y|x.distance.total_cmp(&y.distance)).unwrap();g.try_attach(candidate,1.0,1.0,1e-9,&c).unwrap();assert!(g.try_attach(candidate,1.0,1.0,1e-9,&c).is_ok())}
#[test]fn overlapping_rigid_constituents_are_rejected(){let c=default_catalog();let mut g=PhysicalConstituentGraph::new();let a=g.add_constituent("Carbon",p(0.0,0.0));let b=g.add_constituent("Carbon",p(0.1,0.0));let candidate=g.attachment_candidates(a,b,&c).into_iter().min_by(|x,y|x.distance.total_cmp(&y.distance)).unwrap();assert!(g.try_attach(candidate,1.0,1.0,1e-9,&c).is_err())}}
