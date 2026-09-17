#![expect(dead_code, reason = "Staged API retained for subsystem integration")]
//! A physically realized material instance carried through acquisition/storage.
//!
//! `Material` owns composition and the identity of pre-existing internal bonds.
//! `PhysicalMaterial` owns the complete realized arrangement needed to restore
//! those bonds without inventing connection endpoints at the organism boundary.
use crate::contact::connection_pair_candidates;
use crate::resources::{BaseResource, Material};
use crate::structure::{ConnectionEndpoint, OrganismStructure, Placement, StructuralUnit};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct PhysicalMaterialBond {
    pub(crate) part_a: usize,
    pub(crate) endpoint_a: ConnectionEndpoint,
    pub(crate) part_b: usize,
    pub(crate) endpoint_b: ConnectionEndpoint,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct PhysicalMaterial {
    pub(crate) material: Material,
    pub(crate) placements: Option<Vec<Placement>>,
    #[serde(default)]
    pub(crate) internal_connections: Option<Vec<PhysicalMaterialBond>>,
}

impl PhysicalMaterial {
    pub(crate) fn logical(material: Material) -> Self {
        Self {
            material,
            placements: None,
            internal_connections: None,
        }
    }

    pub(crate) fn realized(
        material: Material,
        placements: Vec<Placement>,
        catalog: &[BaseResource],
    ) -> Option<Self> {
        if !material.is_valid()
            || material.parts.is_empty()
            || placements.len() != material.parts.len()
        {
            return None;
        }
        if placements.iter().any(|placement| {
            !placement.x.is_finite()
                || !placement.y.is_finite()
                || !placement.rotation_radians.is_finite()
        }) {
            return None;
        }

        let mut structure = OrganismStructure::new();
        for ((name, amount), placement) in material.parts.iter().zip(placements.iter()) {
            if (*amount - 1.0).abs() > 1e-9 {
                return None;
            }
            let mut unit =
                StructuralUnit::from_material(Material::free_base(name.clone(), 1.0), *placement)?;
            if !unit.realize_default_geometry(catalog) {
                return None;
            }
            structure.add_unit(unit);
        }

        let mut internal_connections = Vec::with_capacity(material.internal_bonds.len());
        for bond in &material.internal_bonds {
            // `Material::InternalBond` records which constituents are bonded,
            // while the physical endpoints are derived from their realization.
            // More than one endpoint pairing can be geometrically valid for a
            // symmetric realization. Preserve one deterministic physical choice
            // rather than rejecting an otherwise valid material.
            let candidate =
                connection_pair_candidates(&structure, bond.part_a, bond.part_b, catalog)
                    .into_iter()
                    .find(|candidate| {
                        candidate.available_a && candidate.available_b && candidate.distance <= 1.0
                    })?;
            internal_connections.push(PhysicalMaterialBond {
                part_a: bond.part_a,
                endpoint_a: candidate.endpoint_a,
                part_b: bond.part_b,
                endpoint_b: candidate.endpoint_b,
            });
        }

        Some(Self {
            material,
            placements: Some(placements),
            internal_connections: Some(internal_connections),
        })
    }

    pub(crate) fn is_realized(&self) -> bool {
        self.placements.is_some() && self.internal_connections.is_some()
    }

    /// Re-express this realization in a deterministic local frame anchored on
    /// its first constituent. This changes only coordinate representation; all
    /// relative positions, orientations, geometry, and endpoint identities are
    /// preserved. World position is supplied later by the owning context.
    pub(crate) fn into_intrinsic_frame(self) -> Option<Self> {
        let placements = self.placements?;
        let origin = *placements.first()?;
        let (sin, cos) = origin.rotation_radians.sin_cos();
        let rebased = placements
            .into_iter()
            .map(|placement| {
                let dx = placement.x - origin.x;
                let dy = placement.y - origin.y;
                Placement {
                    x: dx * cos + dy * sin,
                    y: -dx * sin + dy * cos,
                    rotation_radians: placement.rotation_radians - origin.rotation_radians,
                }
            })
            .collect();
        Some(Self {
            material: self.material,
            placements: Some(rebased),
            internal_connections: self.internal_connections,
        })
    }
}
