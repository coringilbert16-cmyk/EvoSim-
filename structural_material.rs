use serde::{Deserialize, Serialize};
use crate::resources::{BaseResource, InternalBond, Material, ResourceProperties};

/// Compatibility wrapper around the authoritative `Material` representation.
///
/// Material now owns both composition and internal structure. This type no
/// longer stores a second physical representation; it exists only until the
/// remaining callers/tests are migrated to `Material` directly.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct StructuralMaterial {
    pub material: Material,
}

impl StructuralMaterial {
    pub fn single(resource_name: impl Into<String>) -> Self {
        Self {
            material: Material::free_base(resource_name, 1.0),
        }
    }

    pub fn combine(a: &Material, b: &Material) -> Option<Self> {
        if a.is_empty() || b.is_empty() {
            return None;
        }
        Some(Self {
            material: crate::resources::combine_materials(&[a.clone(), b.clone()]),
        })
    }

    pub fn constituents(&self) -> &[(String, f64)] {
        &self.material.parts
    }

    pub fn internal_bonds(&self) -> &[InternalBond] {
        &self.material.internal_bonds
    }

    pub fn total_amount(&self) -> f64 {
        self.material.total_amount()
    }

    pub fn mass(&self, catalog: &[BaseResource]) -> f64 {
        self.material.mass(catalog)
    }

    pub fn weighted_properties(&self, catalog: &[BaseResource]) -> ResourceProperties {
        self.material.weighted_properties(catalog)
    }

    pub fn is_composite(&self) -> bool {
        self.material.parts.len() > 1
    }

    pub fn is_valid(&self) -> bool {
        self.material.is_valid()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::default_catalog;

    #[test]
    fn single_resource_remains_single_constituent() {
        let m = StructuralMaterial::single("Carbon");
        assert!(!m.is_composite());
        assert!(m.is_valid());
        assert!(m.internal_bonds().is_empty());
    }

    #[test]
    fn composite_preserves_constituent_identity() {
        let a = Material::free_base("Carbon", 1.0);
        let b = Material::free_base("Methane", 1.0);
        let m = StructuralMaterial::combine(&a, &b).unwrap();
        assert_eq!(
            m.constituents(),
            &[("Carbon".into(), 1.0), ("Methane".into(), 1.0)]
        );
        assert_eq!(
            m.internal_bonds(),
            &[InternalBond { part_a: 0, part_b: 1 }]
        );
        assert!(m.is_valid());
    }

    #[test]
    fn composite_properties_are_derived() {
        let c = default_catalog();
        let a = Material::free_base("Carbon", 1.0);
        let b = Material::free_base("Methane", 1.0);
        let m = StructuralMaterial::combine(&a, &b).unwrap();
        let p = m.weighted_properties(&c);
        assert!((p.mass - 1.0).abs() < 1e-12);
        assert!((p.potential_energy - 10.5).abs() < 1e-12);
        assert!((p.reactivity - 2.05).abs() < 1e-12);
        assert!((p.cohesion - 0.525).abs() < 1e-12);
    }

    #[test]
    fn serialization_round_trip_preserves_identity() {
        let a = Material::free_base("Carbon", 1.0);
        let b = Material::free_base("Methane", 1.0);
        let m = StructuralMaterial::combine(&a, &b).unwrap();
        let restored: StructuralMaterial =
            serde_json::from_str(&serde_json::to_string(&m).unwrap()).unwrap();
        assert_eq!(restored, m);
    }
}
