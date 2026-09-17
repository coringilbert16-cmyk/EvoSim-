use crate::connection_geometry::WorldConnectionPoint;
use crate::physical_geometry::PhysicalGeometry;
use crate::resources::{BaseResource, Material};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashSet;
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    pub x: f64,
    pub y: f64,
    pub rotation_radians: f64,
}
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct PhysicalConstituentId(pub u64);
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct StructuralUnit {
    pub physical_id: PhysicalConstituentId,
    pub material: Material,
    pub placement: Placement,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geometry: Option<PhysicalGeometry>,
}
impl StructuralUnit {
    pub fn new(resource_name: impl Into<String>, placement: Placement) -> Self {
        Self {
            physical_id: PhysicalConstituentId(0),
            material: Material::free_base(resource_name, 1.0),
            placement,
            geometry: None,
        }
    }
    pub fn from_material(material: Material, placement: Placement) -> Option<Self> {
        if !material.is_valid() || material.is_empty() || material.has_internal_structure() {
            return None;
        }
        Some(Self {
            physical_id: PhysicalConstituentId(0),
            material,
            placement,
            geometry: None,
        })
    }
    pub fn properties(
        &self,
        catalog: &[BaseResource],
    ) -> Option<crate::resources::ResourceProperties> {
        if !self.material.is_valid()
            || !self
                .material
                .parts
                .iter()
                .all(|(name, _)| catalog.iter().any(|base| base.name == *name))
        {
            return None;
        }
        Some(self.material.weighted_properties(catalog))
    }
    fn external_geometry_resource<'a>(
        &self,
        catalog: &'a [BaseResource],
    ) -> Option<&'a BaseResource> {
        if self.material.has_internal_structure() {
            return None;
        }
        let (name, amount) = self.material.parts.first()?;
        if (*amount - 1.0).abs() > f64::EPSILON {
            return None;
        }
        catalog.iter().find(|base| base.name == *name)
    }
    pub fn shape<'a>(&'a self, catalog: &'a [BaseResource]) -> Option<&'a crate::resources::Shape> {
        if let Some(geometry) = &self.geometry {
            return Some(geometry.shape());
        }
        if !self.material.has_internal_structure() {
            let [(name, amount)] = self.material.parts.as_slice() else {
                return None;
            };
            if (*amount - 1.0).abs() > f64::EPSILON {
                return None;
            }
            return catalog
                .iter()
                .find(|base| base.name == *name)
                .map(|base| &base.shape);
        }
        self.external_geometry_resource(catalog)
            .map(|base| &base.shape)
    }
    pub fn realize_default_geometry(&mut self, catalog: &[BaseResource]) -> bool {
        if self.geometry.is_some() {
            return true;
        }
        let Some(shape) = self.shape(catalog).cloned() else {
            return false;
        };
        self.geometry = Some(PhysicalGeometry::from_default(&shape));
        true
    }
}
impl<'de> Deserialize<'de> for StructuralUnit {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct LegacyStructuralMaterial {
            material: Material,
        }
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum StoredMaterial {
            Direct(Material),
            Wrapped(LegacyStructuralMaterial),
        }
        #[derive(Deserialize)]
        struct Stored {
            #[serde(default)]
            physical_id: PhysicalConstituentId,
            material: Option<StoredMaterial>,
            resource_name: Option<String>,
            placement: Placement,
            #[serde(default)]
            geometry: Option<PhysicalGeometry>,
        }
        let s = Stored::deserialize(deserializer)?;
        let m = match s.material {
            Some(StoredMaterial::Direct(m)) => m,
            Some(StoredMaterial::Wrapped(w)) => w.material,
            None => Material::free_base(
                s.resource_name
                    .ok_or_else(|| serde::de::Error::custom("structural unit has no material"))?,
                1.0,
            ),
        };
        if !m.is_valid() || m.is_empty() {
            return Err(serde::de::Error::custom("invalid material"));
        }
        if m.has_internal_structure() {
            return Err(serde::de::Error::custom(
                "composite material cannot be loaded as a StructuralUnit; restore its physical constituents and bonds through the organism graph",
            ));
        }
        Ok(Self {
            physical_id: s.physical_id,
            material: m,
            placement: s.placement,
            geometry: s.geometry,
        })
    }
}
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum ConnectionEndpoint {
    Corner { point_index: usize },
    LineEndpoint { point_index: usize },
    Boundary { angle_radians: f64 },
    Fluid { x: f64, y: f64 },
}
#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
pub struct BondEndpoint {
    pub constituent_id: PhysicalConstituentId,
    pub location: ConnectionEndpoint,
}
impl BondEndpoint {
    pub fn new(constituent_id: PhysicalConstituentId, location: ConnectionEndpoint) -> Self {
        Self {
            constituent_id,
            location,
        }
    }
}
impl<'de> Deserialize<'de> for BondEndpoint {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct StoredBondEndpoint {
            constituent_id: PhysicalConstituentId,
            location: ConnectionEndpoint,
        }
        let stored = StoredBondEndpoint::deserialize(deserializer)?;
        Ok(Self {
            constituent_id: stored.constituent_id,
            location: stored.location,
        })
    }
}
