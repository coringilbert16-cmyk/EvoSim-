//! Structural material owned by a physical structural unit.
//!
//! `Material` remains the authoritative composition + internal-structure
//! representation. `StructuralMaterial` is the physical-structure wrapper
//! that carries that material into `StructuralUnit`; it never reconstructs
//! identity from a resource name.

use serde::{Deserialize, Serialize};

use crate::resources::{BaseResource, ConnectionSites, Material, ResourceProperties, Shape};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct StructuralMaterial {
    pub material: Material,
}

impl StructuralMaterial {
    pub fn from_material(material: Material) -> Option<Self> {
        if !material.is_valid() || material.is_empty() {
            return None;
        }
        Some(Self { material })
    }

    pub fn single(resource_name: impl Into<String>) -> Self {
        Self {
            material: Material::free_base(resource_name, 1.0),
        }
    }

    pub fn material(&self) -> &Material {
        &self.material
    }

    pub fn constituents(&self) -> &[(String, f64)] {
        &self.material.parts
    }

    pub fn internal_bonds(&self) -> &[crate::resources::InternalBond] {
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
        self.material.is_valid() && !self.material.is_empty()
    }

    /// Discrete connection geometry is currently defined only for a physical
    /// unit backed by one free base constituent. A composite material has
    /// preserved identity here, but it does not yet have constituent-relative
    /// placements, so inventing a single aggregate connection geometry would
    /// be physically incorrect.
    pub fn connection_sites(&self, catalog: &[BaseResource]) -> Option<ConnectionSites> {
        let [(name, amount)] = self.material.parts.as_slice() else {
            return None;
        };
        if !self.material.internal_bonds.is_empty() || (*amount - 1.0).abs() > f64::EPSILON {
            return None;
        }
        catalog
            .iter()
            .find(|base| base.name == *name)
            .map(|base| base.shape.connection_sites())
    }

    /// Return the immutable base shape only when the structural material has
    /// an unambiguous single constituent geometry.
    pub fn shape(&self, catalog: &[BaseResource]) -> Option<&Shape> {
        let [(name, amount)] = self.material.parts.as_slice() else {
            return None;
        };
        if !self.material.internal_bonds.is_empty() || (*amount - 1.0).abs() > f64::EPSILON {
            return None;
        }
        catalog
            .iter()
            .find(|base| base.name == *name)
            .map(|base| &base.shape)
    }

    /// A cache key derived from the complete structural material identity.
    /// This is intentionally not a `StructuralUnit` field and does not make a
    /// resource name the authority for material identity.
    pub fn connection_type_key(&self) -> String {
        let mut key = String::new();
        for (index, (name, amount)) in self.material.parts.iter().enumerate() {
            if index > 0 {
                key.push('|');
            }
            key.push_str(name);
            key.push(':');
            key.push_str(&amount.to_bits().to_string());
        }
        key.push('#');
        for bond in &self.material.internal_bonds {
            key.push_str(&bond.part_a.to_string());
            key.push('-');
            key.push_str(&bond.part_b.to_string());
            key.push(';');
        }
        key
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::{default_catalog, InternalBond};

    #[test]
    fn storage_material_becomes_owned_structural_material_without_losing_structure() {
        let material = Material {
            parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
            internal_bonds: vec![InternalBond { part_a: 0, part_b: 1 }],
        };
        let structural = StructuralMaterial::from_material(material.clone()).unwrap();
        assert_eq!(structural.material(), &material);
        assert!(structural.is_composite());
        assert_eq!(structural.internal_bonds(), material.internal_bonds.as_slice());
    }

    #[test]
    fn properties_are_derived_from_owned_material() {
        let material = Material::free_base("Carbon", 1.0);
        let structural = StructuralMaterial::from_material(material).unwrap();
        let props = structural.weighted_properties(&default_catalog());
        assert_eq!(props.cohesion, 0.95);
    }

    #[test]
    fn composite_material_does_not_get_invented_single_unit_geometry() {
        let material = Material {
            parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
            internal_bonds: vec![InternalBond { part_a: 0, part_b: 1 }],
        };
        let structural = StructuralMaterial::from_material(material).unwrap();
        let catalog = default_catalog();
        assert!(structural.connection_sites(&catalog).is_none());
        assert!(structural.shape(&catalog).is_none());
    }

    #[test]
    fn serialization_round_trip_preserves_material_identity() {
        let material = Material {
            parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
            internal_bonds: vec![InternalBond { part_a: 0, part_b: 1 }],
        };
        let structural = StructuralMaterial::from_material(material).unwrap();
        let restored: StructuralMaterial =
            serde_json::from_str(&serde_json::to_string(&structural).unwrap()).unwrap();
        assert_eq!(restored, structural);
    }
}
