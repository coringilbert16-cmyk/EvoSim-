//! A physically realized material instance carried through acquisition/storage.
//!
//! `Material` owns composition and pre-existing internal bonds. This wrapper
//! owns the realized placement of each constituent. A realization is accepted
//! only when those placements determine each pre-existing bond unambiguously.
use crate::contact::connection_pair_candidates;
use crate::resources::{BaseResource, Material};
use crate::structure::{OrganismStructure, Placement, StructuralUnit};
use serde::{Deserialize, Serialize};

const CONTACT_TOLERANCE: f64 = 1.0;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct PhysicalMaterial {
    pub(crate) material: Material,
    /// One placement per material constituent, in `parts` order. Placements
    /// are relative to the material's part-0 origin; they are transformed into
    /// world placement only when the material is restored into a structure.
    /// `None` means no physical realization was supplied and restoration must
    /// refuse to invent one.
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
        if !material.is_valid()
            || material.parts.is_empty()
            || placements.len() != material.parts.len()
        {
            return None;
        }
        for (index, placement) in placements.iter().enumerate() {
            if !placement.x.is_finite()
                || !placement.y.is_finite()
                || !placement.rotation_radians.is_finite()
                || catalog
                    .iter()
                    .all(|resource| resource.name != material.parts[index].0)
            {
                return None;
            }
        }

        // A physical realization must contain enough information to recover
        // its pre-existing internal bonds without choosing among multiple
        // possible connection endpoints. If the placement is ambiguous, this
        // object is not yet a complete physical realization and must not enter
        // the environment/storage authority path.
        let mut structure = OrganismStructure::new();
        let mut indices = Vec::with_capacity(material.parts.len());
        for ((name, amount), placement) in material.parts.iter().zip(placements.iter()) {
            if (*amount - 1.0).abs() > f64::EPSILON || amount.fract().abs() > f64::EPSILON {
                return None;
            }
            let mut unit = StructuralUnit::from_material(
                Material::free_base(name.clone(), *amount),
                *placement,
            )?;
            if !unit.realize_default_geometry(catalog) {
                return None;
            }
            indices.push(structure.add_unit(unit));
        }

        for internal in &material.internal_bonds {
            let unit_a = *indices.get(internal.part_a)?;
            let unit_b = *indices.get(internal.part_b)?;
            let candidates = connection_pair_candidates(&structure, unit_a, unit_b, catalog)
                .into_iter()
                .filter(|candidate| candidate.distance <= CONTACT_TOLERANCE)
                .collect::<Vec<_>>();
            if candidates.len() != 1 {
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
