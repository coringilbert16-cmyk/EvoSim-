//! Atomic conversion from bulk field material into a persistent physical object.
//!
//! `ActiveMaterialField` remains the owner of aggregated ecological stock.
//! This module provides the explicit transaction that moves a selected amount
//! into `PhysicalEnvironment` only after the physical geometry is known to be
//! valid.

use crate::material_geometry::PhysicalMaterialInstance;
use crate::state::Environment;
use crate::structure::Placement;

impl Environment {
    /// Realize part of a bulk field material as a persistent physical object.
    ///
    /// Validation is performed against a clone before the field is changed.
    /// Therefore invalid geometry, invalid catalog references, or mismatched
    /// placements leave the source stock untouched and create no object.
    pub(crate) fn realize_field_material(
        &mut self,
        cell_index: usize,
        material_index: usize,
        amount: f64,
        placements: &[Placement],
    ) -> Result<usize, String> {
        let source = self
            .field
            .cells
            .get(cell_index)
            .and_then(|cell| cell.materials.get(material_index))
            .ok_or_else(|| "source material does not exist".to_string())?;

        let candidate = source
            .clone()
            .take(amount)
            .ok_or_else(|| "source material cannot provide the requested amount".to_string())?;

        // Constructing the instance first is the commit precondition. No
        // source material has been removed yet.
        let instance = PhysicalMaterialInstance::new(candidate, placements, &self.catalog)
            .ok_or_else(|| "material cannot be physically realized".to_string())?;

        let realized = self
            .field
            .take_at_index(cell_index, material_index, amount)
            .ok_or_else(|| "source material changed during realization".to_string())?;

        // The validated candidate and the actual withdrawal must be identical
        // because both use Material::take with the same source and amount.
        debug_assert_eq!(realized, instance.material);

        self.physical.materials.push(instance);
        Ok(self.physical.materials.len() - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::ActiveMaterialField;
    use crate::resources::{default_catalog, Material};
    use crate::state::Environment;

    fn placement(x: f64, y: f64) -> Placement {
        Placement {
            x,
            y,
            rotation_radians: 0.0,
        }
    }

    fn environment() -> Environment {
        Environment {
            width: 100.0,
            height: 100.0,
            catalog: default_catalog(),
            field: ActiveMaterialField::new(100.0, 100.0, 25.0),
            reservoir: Default::default(),
            vents: Vec::new(),
            physical: Default::default(),
        }
    }

    #[test]
    fn successful_realization_moves_exact_material_from_bulk_to_physical() {
        let mut environment = environment();
        environment
            .field
            .deposit_at_index(0, Material::free_base("Carbon", 10.0));

        let before = environment.field.total_amount() + environment.physical.total_material_amount();
        let index = environment
            .realize_field_material(0, 0, 4.0, &[placement(10.0, 10.0)])
            .unwrap();

        assert_eq!(index, 0);
        assert!((environment.field.total_amount() - 6.0).abs() < 1e-12);
        assert!((environment.physical.total_material_amount() - 4.0).abs() < 1e-12);
        let after = environment.field.total_amount() + environment.physical.total_material_amount();
        assert!((after - before).abs() < 1e-12);
    }

    #[test]
    fn invalid_realization_leaves_bulk_untouched() {
        let mut environment = environment();
        environment
            .field
            .deposit_at_index(0, Material::free_base("Carbon", 10.0));

        let result = environment.realize_field_material(
            0,
            0,
            4.0,
            &[placement(f64::NAN, 10.0)],
        );

        assert!(result.is_err());
        assert!((environment.field.total_amount() - 10.0).abs() < 1e-12);
        assert!(environment.physical.is_empty());
    }

    #[test]
    fn realization_requires_exact_source_and_placement_geometry() {
        let mut environment = environment();
        environment
            .field
            .deposit_at_index(0, Material::free_base("Carbon", 10.0));

        let result = environment.realize_field_material(0, 0, 4.0, &[]);

        assert!(result.is_err());
        assert!((environment.field.total_amount() - 10.0).abs() < 1e-12);
        assert!(environment.physical.is_empty());
    }
}
