use crate::resources::{BaseResource, Material};

/// Environmental material that is no longer biologically owned by an organism.
///
/// A decomposing body retains its structural material and bond topology until
/// BREAK operations progressively release its constituents. `energy_budget`
/// is the usable energy carried out of the organism at death and is the only
/// energy available to pay net-negative decomposition steps.
#[derive(Clone, Debug)]
pub struct DecomposingBody {
    pub material: Material,
    pub energy_budget: f64,
}

impl DecomposingBody {
    pub fn new(material: Material, energy_budget: f64) -> Option<Self> {
        if !material.is_valid() || !energy_budget.is_finite() || energy_budget < 0.0 {
            return None;
        }
        Some(Self {
            material,
            energy_budget,
        })
    }

    pub fn is_finished(&self) -> bool {
        !self.material.has_internal_structure()
    }

    pub fn can_attempt_break(&self) -> bool {
        self.material.can_break()
    }

    /// Return the remaining unstructured constituents once all bonds have
    /// been dismantled. No material is destroyed by decomposition.
    pub fn release_if_finished(self, catalog: &[BaseResource]) -> Option<(Material, f64)> {
        if !self.is_finished() || self.material.potential_energy(catalog).is_nan() {
            return None;
        }
        Some((self.material, self.energy_budget))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn death_energy_is_finite_and_nonnegative() {
        let material = Material::free_base("Carbon", 1.0);
        assert!(DecomposingBody::new(material, 4.0).is_some());
        assert!(DecomposingBody::new(Material::free_base("Carbon", 1.0), -1.0).is_none());
    }

    #[test]
    fn unstructured_material_is_already_finished() {
        let material = Material::free_base("Carbon", 1.0);
        let body = DecomposingBody::new(material, 0.0).unwrap();
        assert!(body.is_finished());
    }
}
