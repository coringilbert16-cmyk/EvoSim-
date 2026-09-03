use serde::{Deserialize, Serialize};

use crate::resources::{BaseResource, Material};

/// Derived macroscopic chemistry for one material stack.
///
/// This is intentionally a projection, not new mutable chemistry. All values
/// are calculated from the material composition and immutable resource
/// properties in the catalog.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct MaterialSummary {
    pub(crate) amount: f64,
    pub(crate) mass: f64,
    pub(crate) potential_energy: f64,
    pub(crate) reactivity: f64,
    pub(crate) cohesion: f64,
}

impl MaterialSummary {
    pub(crate) fn from_material(material: &Material, catalog: &[BaseResource]) -> Self {
        let amount = material.total_amount();
        let mass = material.mass(catalog);
        let properties = material.weighted_properties(catalog);
        Self {
            amount,
            mass,
            potential_energy: material.potential_energy(catalog),
            reactivity: properties.reactivity,
            cohesion: properties.cohesion,
        }
    }
}

/// Authoritative Floor 0 -> Floor 1 projection for material already held by a
/// cell/organism. It separates structural, stored-unbonded, and immediately
/// available chemical state without inventing additional chemistry.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct CellMaterialProjection {
    pub(crate) structural: MaterialSummary,
    pub(crate) stored_unbonded: MaterialSummary,
    pub(crate) total: MaterialSummary,
    pub(crate) bonded_amount: f64,
    pub(crate) unbonded_amount: f64,
}

impl CellMaterialProjection {
    pub(crate) fn from_materials(
        structural: &Material,
        stored_unbonded: &Material,
        catalog: &[BaseResource],
    ) -> Self {
        let structural_summary = MaterialSummary::from_material(structural, catalog);
        let stored_summary = MaterialSummary::from_material(stored_unbonded, catalog);

        let total_amount = structural_summary.amount + stored_summary.amount;
        let total_mass = structural_summary.mass + stored_summary.mass;
        let total_potential_energy =
            structural_summary.potential_energy + stored_summary.potential_energy;

        let total = if total_amount > 0.0 {
            let reactivity = (structural_summary.reactivity * structural_summary.amount
                + stored_summary.reactivity * stored_summary.amount)
                / total_amount;
            let cohesion = (structural_summary.cohesion * structural_summary.amount
                + stored_summary.cohesion * stored_summary.amount)
                / total_amount;
            MaterialSummary {
                amount: total_amount,
                mass: total_mass,
                potential_energy: total_potential_energy,
                reactivity,
                cohesion,
            }
        } else {
            MaterialSummary::default()
        };

        Self {
            structural: structural_summary,
            stored_unbonded: stored_summary,
            total,
            bonded_amount: structural_summary.amount,
            unbonded_amount: stored_summary.amount,
        }
    }

    pub(crate) fn available_potential_energy(&self) -> f64 {
        self.stored_unbonded.potential_energy
    }

    pub(crate) fn structural_fraction(&self) -> f64 {
        if self.total.amount <= f64::EPSILON {
            0.0
        } else {
            (self.structural.amount / self.total.amount).clamp(0.0, 1.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::default_catalog;

    #[test]
    fn projection_derives_energy_and_mass_from_material() {
        let catalog = default_catalog();
        let structural = Material {
            parts: vec![("Carbon".into(), 2.0), ("Methane".into(), 1.0)],
            bonded: true,
        };
        let stored = Material::free_base("Hydrogen", 3.0);

        let projection = CellMaterialProjection::from_materials(&structural, &stored, &catalog);

        assert_eq!(projection.structural.amount, 3.0);
        assert_eq!(projection.stored_unbonded.amount, 3.0);
        assert_eq!(projection.total.mass, 6.0);
        assert_eq!(projection.total.potential_energy, 44.0);
        assert_eq!(projection.bonded_amount, 3.0);
        assert_eq!(projection.unbonded_amount, 3.0);
        assert!((projection.structural_fraction() - 0.5).abs() < 1e-12);
    }

    #[test]
    fn projection_does_not_store_independent_potential_energy() {
        let catalog = default_catalog();
        let structural = Material::free_base("Carbon", 1.0);
        let stored = Material::free_base("Methane", 1.0);
        let projection = CellMaterialProjection::from_materials(&structural, &stored, &catalog);

        assert_eq!(projection.available_potential_energy(), 20.0);
        assert_eq!(projection.total.potential_energy, 21.0);
    }
}
