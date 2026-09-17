//! Canonical realized physical-material value used by Phase 1 transfer paths.
use crate::resources::{BaseResource, Material};
use crate::structure::Placement;

#[derive(Clone, Debug, PartialEq)]
pub struct CanonicalPhysicalMaterial {
    pub material: Material,
    pub placements: Vec<Placement>,
}

impl CanonicalPhysicalMaterial {
    pub fn new(material: Material, placements: Vec<Placement>, catalog: &[BaseResource]) -> Option<Self> {
        if !material.is_valid() || material.parts.is_empty() || placements.len() != material.parts.len() {
            return None;
        }
        if placements.iter().any(|p| !p.x.is_finite() || !p.y.is_finite() || !p.rotation_radians.is_finite()) {
            return None;
        }
        if material.parts.iter().any(|(name, _)| catalog.iter().all(|r| r.name != *name)) {
            return None;
        }
        Some(Self { material, placements })
    }

    pub fn part_placement(&self, index: usize) -> Option<Placement> {
        self.placements.get(index).copied()
    }
}
