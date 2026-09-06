use crate::resources::{BaseResource, ConnectionPoint, ConnectionSites};
use crate::structural_material::StructuralMaterial;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Placement { pub x: f64, pub y: f64, pub rotation_radians: f64 }

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct StructuralUnit {
    pub material: StructuralMaterial,
    pub placement: Placement,
}
impl StructuralUnit {
    pub fn new(resource_name: impl Into<String>, placement: Placement) -> Self {
        Self { material: StructuralMaterial::single(resource_name), placement }
    }
    pub fn from_material(material: crate::resources::Material, placement: Placement) -> Option<Self> {
        Some(Self { material: StructuralMaterial::from_material(material)?, placement })
    }
    pub fn resource_name(&self) -> Option<&str> {
        let [(name, amount)] = self.material.constituents() else { return None; };
        if self.material.internal_bonds().is_empty() && (*amount - 1.0).abs() <= f64::EPSILON { Some(name.as_str()) } else { None }
    }
    pub fn properties(&self, catalog: &[BaseResource]) -> Option<crate::resources::ResourceProperties> {
        if !self.material.is_valid() { return None; }
        Some(self.material.weighted_properties(catalog))
    }
    pub fn connection_sites(&self, catalog: &[BaseResource]) -> Option<ConnectionSites> { self.material.connection_sites(catalog) }
    pub fn shape(&self, catalog: &[BaseResource]) -> Option<&crate::resources::Shape> { self.material.shape(catalog) }
}

impl<'de> Deserialize<'de> for StructuralUnit {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
        #[derive(Deserialize)]
        struct Stored { material: Option<StructuralMaterial>, resource_name: Option<String>, placement: Placement }
        let stored = Stored::deserialize(deserializer)?;
        let material = match stored.material {
            Some(material) => material,
            None => StructuralMaterial::single(stored.resource_name.ok_or_else(|| serde::de::Error::custom("structural unit has no material"))?),
        };
        if !material.is_valid() { return Err(serde::de::Error::custom("invalid structural material")); }
        Ok(Self { material, placement: stored.placement })
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ConnectionSiteRef { pub unit_index: usize, pub point_index: usize }

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Bond {
    pub unit_a: usize, pub point_a: usize, pub unit_b: usize, pub point_b: usize,
    pub strength: f64,
    #[serde(default)] pub bond_energy: f64,
}
impl Bond {
    pub fn touches(&self, unit: usize, point: usize) -> bool { (self.unit_a == unit && self.point_a == point) || (self.unit_b == unit && self.point_b == point) }
    pub fn has_same_identity(&self, other: &Bond) -> bool {
        (self.unit_a == other.unit_a && self.point_a == other.point_a && self.unit_b == other.unit_b && self.point_b == other.point_b)
            || (self.unit_a == other.unit_b && self.point_a == other.point_b && self.unit_b == other.unit_a && self.point_b == other.point_a)
    }
    pub fn is_valid(&self, unit_count: usize, connection_point_count: impl Fn(usize) -> Option<usize>) -> bool {
        if self.unit_a >= unit_count || self.unit_b >= unit_count || !self.bond_energy.is_finite() || self.bond_energy < 0.0 { return false; }
        match (connection_point_count(self.unit_a), connection_point_count(self.unit_b)) { (Some(a), Some(b)) => self.point_a < a && self.point_b < b, _ => false }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct OrganismStructure { pub units: Vec<StructuralUnit>, pub bonds: Vec<Bond> }
impl OrganismStructure {
    pub fn new() -> Self { Self::default() }
    pub fn add_unit(&mut self, unit: StructuralUnit) -> usize { self.units.push(unit); self.units.len()-1 }
    pub fn add_bond(&mut self, bond: Bond) -> usize { self.bonds.push(bond); self.bonds.len()-1 }
    pub fn is_valid_bond(&self, bond: &Bond, catalog: &[BaseResource]) -> bool {
        if !bond.is_valid(self.units.len(), |i| self.units.get(i).and_then(|u| u.connection_sites(catalog)).and_then(|sites| match sites { ConnectionSites::Corners(points) => Some(points.len()), _ => None })) { return false; }
        let Some(a)=self.units[bond.unit_a].properties(catalog) else { return false; }; let Some(b)=self.units[bond.unit_b].properties(catalog) else { return false; };
        let strength=crate::combine::bond_strength(a,b); strength.is_finite() && (0.0..=1.0).contains(&strength)
    }
    pub fn connection_site(&self, site: ConnectionSiteRef, catalog: &[BaseResource]) -> Option<ConnectionPoint> {
        match self.units.get(site.unit_index)?.connection_sites(catalog)? { ConnectionSites::Corners(points)=>points.get(site.point_index).copied(), _=>None }
    }
    pub fn available_connection_sites(&self, catalog: &[BaseResource]) -> Vec<ConnectionSiteRef> {
        let mut out=Vec::new();
        for i in 0..self.units.len() { let Some(ConnectionSites::Corners(points))=self.units[i].connection_sites(catalog) else {continue;}; for p in 0..points.len(){ if self.connection_count(i,p)==0 {out.push(ConnectionSiteRef{unit_index:i,point_index:p});} } }
        out
    }
    pub fn connected_components(&self) -> Vec<Vec<usize>> {
        let mut adjacency=vec![Vec::<usize>::new();self.units.len()]; for b in &self.bonds { if b.unit_a<self.units.len()&&b.unit_b<self.units.len(){adjacency[b.unit_a].push(b.unit_b);adjacency[b.unit_b].push(b.unit_a);} }
        let mut visited=vec![false;self.units.len()];let mut components=Vec::new(); for start in 0..self.units.len(){if visited[start]{continue;}let mut stack=vec![start];visited[start]=true;let mut c=Vec::new();while let Some(u)=stack.pop(){c.push(u);for &n in &adjacency[u]{if !visited[n]{visited[n]=true;stack.push(n);}}}c.sort_unstable();components.push(c);} components
    }
    pub fn component_connection_sites(&self, component:&[usize], catalog:&[BaseResource])->Vec<ConnectionSiteRef>{let set:std::collections::HashSet<usize>=component.iter().copied().collect();self.available_connection_sites(catalog).into_iter().filter(|s|set.contains(&s.unit_index)).collect()}
    pub fn connection_load(&self, unit:usize, point:usize, catalog:&[BaseResource])->f64{self.bonds.iter().filter(|b|b.touches(unit,point)).filter_map(|b|Some(crate::combine::bond_strength(self.units.get(b.unit_a)?.properties(catalog)?,self.units.get(b.unit_b)?.properties(catalog)?))).sum()}
    pub fn connection_count(&self, unit:usize, point:usize)->usize{self.bonds.iter().filter(|b|b.touches(unit,point)).count()}
    pub fn break_bond(&mut self, index:usize)->Option<Bond>{if index<self.bonds.len(){Some(self.bonds.remove(index))}else{None}}
    pub fn break_matching_bond(&mut self,target:Bond)->Option<Bond>{let i=self.bonds.iter().position(|b|b.has_same_identity(&target))?;self.break_bond(i)}
    pub fn disconnect_point(&mut self,unit:usize,point:usize)->Vec<Bond>{let mut out=Vec::new();let mut i=0;while i<self.bonds.len(){if self.bonds[i].touches(unit,point){out.push(self.bonds.remove(i));}else{i+=1;}}out}
    pub fn loaded_points(&self)->Vec<(usize,usize)>{let mut out=Vec::new();for b in &self.bonds{for p in [(b.unit_a,b.point_a),(b.unit_b,b.point_b)]{if !out.contains(&p){out.push(p);}}}out}
}

pub fn formation_threshold(cohesion_a:f64,cohesion_b:f64,load_a:f64,load_b:f64)->f64{let load_a=load_a.max(0.0);let load_b=load_b.max(0.0);((cohesion_a+cohesion_b)/2.0)*(1.0+load_a.sqrt()+load_b.sqrt())}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn material_is_owned_by_structural_unit(){let unit=StructuralUnit::new("Carbon",Placement{x:0.0,y:0.0,rotation_radians:0.0});assert_eq!(unit.material.constituents(),&[("Carbon".into(),1.0)]);}
    #[test] fn legacy_resource_name_deserializes_into_material(){let u:StructuralUnit=serde_json::from_str(r#"{"resource_name":"Carbon","placement":{"x":0.0,"y":0.0,"rotation_radians":0.0}}"#).unwrap();assert_eq!(u.resource_name(),Some("Carbon"));}
    #[test] fn composite_material_identity_is_preserved(){let m=crate::resources::Material{parts:vec![("Carbon".into(),1.0),("Hydrogen".into(),1.0)],internal_bonds:vec![crate::resources::InternalBond{part_a:0,part_b:1}]};let u=StructuralUnit::from_material(m.clone(),Placement{x:0.0,y:0.0,rotation_radians:0.0}).unwrap();assert_eq!(u.material.material(),&m);assert!(u.connection_sites(&crate::resources::default_catalog()).is_none());}
}
