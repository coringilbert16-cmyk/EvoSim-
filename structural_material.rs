use serde::{Deserialize, Serialize};
use crate::resources::{BaseResource, Material, ResourceProperties};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct StructuralMaterial {
    pub material: Material,
    pub internal_bonds: Vec<InternalBond>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct InternalBond {
    pub part_a: usize,
    pub part_b: usize,
}

impl StructuralMaterial {
    pub fn single(resource_name: impl Into<String>) -> Self {
        Self { material: Material { parts: vec![(resource_name.into(), 1.0)], bonded: true }, internal_bonds: Vec::new() }
    }

    pub fn combine(a: &Material, b: &Material) -> Option<Self> {
        if a.is_empty() || b.is_empty() { return None; }
        let mut parts = a.parts.clone();
        let b_start = parts.len();
        parts.extend(b.parts.iter().cloned());
        Some(Self { material: Material { parts, bonded: true }, internal_bonds: vec![InternalBond { part_a: 0, part_b: b_start }] })
    }

    pub fn constituents(&self) -> &[(String, f64)] { &self.material.parts }
    pub fn total_amount(&self) -> f64 { self.material.total_amount() }
    pub fn mass(&self, catalog: &[BaseResource]) -> f64 { self.material.mass(catalog) }
    pub fn weighted_properties(&self, catalog: &[BaseResource]) -> ResourceProperties { self.material.weighted_properties(catalog) }
    pub fn is_composite(&self) -> bool { self.material.parts.len() > 1 }
    pub fn is_valid(&self) -> bool {
        let count = self.material.parts.len();
        !self.material.parts.is_empty()
            && self.material.parts.iter().all(|(_, amount)| amount.is_finite() && *amount > 0.0)
            && self.internal_bonds.iter().all(|bond| bond.part_a < count && bond.part_b < count && bond.part_a != bond.part_b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::default_catalog;
    #[test] fn single_resource_remains_single_constituent() { let m=StructuralMaterial::single("Carbon"); assert!(!m.is_composite()); assert!(m.is_valid()); assert!(m.internal_bonds.is_empty()); }
    #[test] fn composite_preserves_constituent_identity() { let a=Material {parts:vec![("Carbon".into(),1.0)],bonded:true}; let b=Material {parts:vec![("Methane".into(),1.0)],bonded:true}; let m=StructuralMaterial::combine(&a,&b).unwrap(); assert_eq!(m.constituents(), &[("Carbon".into(),1.0),("Methane".into(),1.0)]); assert_eq!(m.internal_bonds,vec![InternalBond{part_a:0,part_b:1}]); assert!(m.is_valid()); }
    #[test] fn composite_properties_are_derived() { let c=default_catalog(); let a=Material{parts:vec![("Carbon".into(),1.0)],bonded:true}; let b=Material{parts:vec![("Methane".into(),1.0)],bonded:true}; let m=StructuralMaterial::combine(&a,&b).unwrap(); let p=m.weighted_properties(&c); assert!((p.mass-1.0).abs()<1e-12); assert!((p.potential_energy-10.5).abs()<1e-12); assert!((p.reactivity-2.05).abs()<1e-12); assert!((p.cohesion-0.525).abs()<1e-12); }
    #[test] fn unbonded_inputs_are_accepted() { let a=Material::free_base("Carbon",1.0); let b=Material::free_base("Methane",1.0); let m=StructuralMaterial::combine(&a,&b).unwrap(); assert!(m.is_valid()); assert!(m.material.bonded); }
    #[test] fn serialization_round_trip_preserves_identity() { let a=Material{parts:vec![("Carbon".into(),1.0)],bonded:true}; let b=Material{parts:vec![("Methane".into(),1.0)],bonded:true}; let m=StructuralMaterial::combine(&a,&b).unwrap(); let restored:StructuralMaterial=serde_json::from_str(&serde_json::to_string(&m).unwrap()).unwrap(); assert_eq!(restored,m); }
}
