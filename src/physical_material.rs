//! A physically realized material instance carried through acquisition/storage.
//!
//! `Material` owns composition and pre-existing internal bonds. This wrapper
//! owns the realized placement of each constituent. Keeping the two layers
//! explicit prevents storage or realization from inventing geometry.
use crate::resources::{BaseResource, Material};
use crate::structure::Placement;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct PhysicalMaterial {
    pub(crate) material: Material,
    /// One world-relative placement per material constituent, in `parts` order.
    /// `None` means the material has not yet supplied a physical realization;
    /// restoration must refuse to invent one.
    pub(crate) placements: Option<Vec<Placement>>,
}

impl PhysicalMaterial {
    pub(crate) fn logical(material: Material) -> Self {
        Self {
            material,
            placements: None,
        }
    }

    pub(crate) fn realized(
        material: Material,
        placements: Vec<Placement>,
        catalog: &[BaseResource],
    ) -> Option<Self> {
        if !material.is_valid() || material.parts.is_empty() || placements.len() != material.parts.len() {
            return None;
        }
        for (index, placement) in placements.iter().enumerate() {
            if !placement.x.is_finite()
                || !placement.y.is_finite()
                || !placement.rotation_radians.is_finite()
                || catalog.iter().all(|resource| resource.name != material.parts[index].0)
            {
                return None;
            }
        }
        Some(Self {
            material,
            placements: Some(placements),
        })
    }

    pub(crate) fn is_realized(&self) -> bool {
        self.placements.is_some()
    }
}
