//! Physical geometry for environmental material instances.
//!
//! `Material` remains the authoritative composition + structure identity. This
//! module represents where that material's constituents physically exist in
//! space. It deliberately does not add geometry to `Material` itself, because
//! the same physical material definition may be transferred between systems
//! while its spatial placement is an instance-level fact.

use crate::resources::{BaseResource, Form, Material};
use crate::structure::Placement;

#[derive(Clone, Debug, PartialEq)]
pub struct PlacedMaterialPart {
    pub part_index: usize,
    pub form: Form,
    pub placement: Placement,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MaterialGeometry {
    pub parts: Vec<PlacedMaterialPart>,
    pub min_x: f64,
    pub max_x: f64,
    pub min_y: f64,
    pub max_y: f64,
}

/// A physical environmental instance: authoritative material identity plus
/// the spatial realization of its constituents.
///
/// This is intentionally distinct from ecological bulk stock. Bulk stock can
/// be aggregated in a field cell without inventing arbitrary constituent
/// positions; a physical instance cannot exist without explicit geometry.
#[derive(Clone, Debug, PartialEq)]
pub struct PhysicalMaterialInstance {
    pub material: Material,
    pub geometry: MaterialGeometry,
}

impl PhysicalMaterialInstance {
    pub fn new(
        material: Material,
        placements: &[Placement],
        catalog: &[BaseResource],
    ) -> Option<Self> {
        let geometry = MaterialGeometry::new(&material, placements, catalog)?;
        Some(Self { material, geometry })
    }
}

impl MaterialGeometry {
    /// Build physical geometry for a material instance from its constituent
    /// placements and the immutable resource catalog.
    ///
    /// Composition and structure remain owned by `material`; this type only
    /// supplies the spatial realization required for future contact and
    /// boundary calculations.
    pub fn new(
        material: &Material,
        placements: &[Placement],
        catalog: &[BaseResource],
    ) -> Option<Self> {
        if !material.is_valid() || placements.len() != material.parts.len() {
            return None;
        }
        if material.parts.is_empty() {
            return None;
        }

        let mut parts = Vec::with_capacity(material.parts.len());
        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        for (part_index, ((resource_name, _), placement)) in
            material.parts.iter().zip(placements.iter()).enumerate()
        {
            let resource = catalog.iter().find(|resource| resource.name == *resource_name)?;
            if !resource.shape.is_valid()
                || !placement.x.is_finite()
                || !placement.y.is_finite()
                || !placement.rotation_radians.is_finite()
            {
                return None;
            }

            let radius = resource.shape.form.bounding_radius();
            min_x = min_x.min(placement.x - radius);
            max_x = max_x.max(placement.x + radius);
            min_y = min_y.min(placement.y - radius);
            max_y = max_y.max(placement.y + radius);

            parts.push(PlacedMaterialPart {
                part_index,
                form: resource.shape.form.clone(),
                placement: *placement,
            });
        }

        Some(Self {
            parts,
            min_x,
            max_x,
            min_y,
            max_y,
        })
    }

    /// Conservative broad-phase test. This is not a shape-level contact test.
    pub fn bounding_box_contains(&self, x: f64, y: f64) -> bool {
        x >= self.min_x && x <= self.max_x && y >= self.min_y && y <= self.max_y
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::default_catalog;

    #[test]
    fn geometry_preserves_material_part_identity_and_placement() {
        let catalog = default_catalog();
        let material = Material::free_base("Carbon", 1.0);
        let placements = [Placement {
            x: 12.0,
            y: 8.0,
            rotation_radians: 0.25,
        }];

        let geometry = MaterialGeometry::new(&material, &placements, &catalog).unwrap();
        assert_eq!(geometry.parts.len(), 1);
        assert_eq!(geometry.parts[0].part_index, 0);
        assert_eq!(geometry.parts[0].placement, placements[0]);
        assert!(geometry.bounding_box_contains(12.0, 8.0));
    }

    #[test]
    fn physical_instance_keeps_material_and_geometry_together() {
        let catalog = default_catalog();
        let material = Material::free_base("Carbon", 1.0);
        let placements = [Placement {
            x: 4.0,
            y: 6.0,
            rotation_radians: 0.0,
        }];

        let instance = PhysicalMaterialInstance::new(material.clone(), &placements, &catalog)
            .unwrap();
        assert_eq!(instance.material, material);
        assert_eq!(instance.geometry.parts[0].placement, placements[0]);
    }

    #[test]
    fn structured_material_requires_one_placement_per_constituent() {
        let catalog = default_catalog();
        let material = Material {
            parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
            internal_bonds: vec![crate::resources::InternalBond {
                part_a: 0,
                part_b: 1,
            }],
        };
        let placements = [Placement {
            x: 0.0,
            y: 0.0,
            rotation_radians: 0.0,
        }];

        assert!(MaterialGeometry::new(&material, &placements, &catalog).is_none());
    }

    #[test]
    fn invalid_geometry_is_rejected() {
        let catalog = default_catalog();
        let material = Material::free_base("Carbon", 1.0);
        let placements = [Placement {
            x: f64::NAN,
            y: 0.0,
            rotation_radians: 0.0,
        }];

        assert!(MaterialGeometry::new(&material, &placements, &catalog).is_none());
    }
}
