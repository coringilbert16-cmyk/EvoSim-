//! Quantity-driven formation of physical environmental material.
//!
//! Bulk field material remains aggregated until this explicit formation pass
//! decides that enough compatible quantity exists to create a physical object.
//! The formation pass never invents internal bonds: composition and structure
//! remain authoritative in `Material`, while geometry is supplied as an
//! instance-level realization.

use crate::resources::{BaseResource, Form, Material};
use crate::state::Environment;
use crate::structure::Placement;

pub const DEFAULT_FORMATION_MINIMUM_AMOUNT: f64 = 1.0;
pub const DEFAULT_FORMATION_FRACTION: f64 = 0.10;

#[derive(Clone, Debug, PartialEq)]
pub struct PhysicalFormationRule {
    pub resource_name: String,
    pub minimum_amount: f64,
    pub fraction_to_realize: f64,
}

impl PhysicalFormationRule {
    pub fn new(resource_name: impl Into<String>, minimum_amount: f64, fraction_to_realize: f64) -> Option<Self> {
        let resource_name = resource_name.into();
        if resource_name.is_empty()
            || !minimum_amount.is_finite()
            || minimum_amount <= 0.0
            || !fraction_to_realize.is_finite()
            || fraction_to_realize <= 0.0
            || fraction_to_realize > 1.0
        {
            return None;
        }
        Some(Self {
            resource_name,
            minimum_amount,
            fraction_to_realize,
        })
    }

    pub fn default_for(resource_name: impl Into<String>) -> Option<Self> {
        let name = resource_name.into();
        if name.is_empty() {
            return None;
        }
        Some(Self {
            resource_name: name,
            minimum_amount: DEFAULT_FORMATION_MINIMUM_AMOUNT,
            fraction_to_realize: DEFAULT_FORMATION_FRACTION,
        })
    }

    fn eligible(&self, material: &Material, catalog: &[BaseResource]) -> bool {
        if material.has_internal_structure() || material.parts.len() != 1 {
            return false;
        }
        let Some((name, amount)) = material.parts.first() else {
            return false;
        };
        if name != &self.resource_name || *amount < self.minimum_amount {
            return false;
        }
        let Some(resource) = catalog.iter().find(|resource| resource.name == self.resource_name) else {
            return false;
        };
        resource.shape.is_valid() && !matches!(resource.shape.form, Form::Fluid { .. })
    }

    fn amount_to_realize(&self, available: f64) -> f64 {
        (available * self.fraction_to_realize).min(available)
    }
}

/// Run quantity-driven physical formation over every ecological field cell.
///
/// Each successful realization removes exactly the realized amount from bulk
/// stock and creates exactly that amount as one persistent physical instance.
/// Cell-center placement is the spatial realization rule for this first
/// environmental formation layer; it is not used as contact authority.
pub(crate) fn apply_physical_formation(
    environment: &mut Environment,
    rules: &[PhysicalFormationRule],
) -> usize {
    let mut proposals = Vec::new();

    for (cell_index, cell) in environment.field.cells.iter().enumerate() {
        let (x, y) = environment.field.cell_center(cell_index);
        for (material_index, material) in cell.materials.iter().enumerate() {
            for rule in rules {
                if !rule.eligible(material, &environment.catalog) {
                    continue;
                }
                let amount = rule.amount_to_realize(material.total_amount());
                if amount > 0.0 {
                    proposals.push((cell_index, material_index, amount, Placement {
                        x,
                        y,
                        rotation_radians: 0.0,
                    }));
                }
            }
        }
    }

    let mut realized = 0;
    // Material indices can shift when a source becomes empty, so process in
    // reverse cell/material order after proposals have been collected.
    proposals.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1)));
    for (cell_index, material_index, amount, placement) in proposals {
        if environment
            .realize_field_material(cell_index, material_index, amount, &[placement])
            .is_ok()
        {
            realized += 1;
        }
    }
    realized
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::ActiveMaterialField;
    use crate::physical_environment::PhysicalEnvironment;
    use crate::resources::default_catalog;
    use crate::state::Environment;

    fn environment() -> Environment {
        Environment {
            width: 100.0,
            height: 100.0,
            catalog: default_catalog(),
            field: ActiveMaterialField::new(100.0, 100.0, 25.0),
            reservoir: Default::default(),
            vents: Vec::new(),
            physical: PhysicalEnvironment::new(),
        }
    }

    #[test]
    fn formation_requires_threshold_and_uses_fraction_of_available_quantity() {
        let rule = PhysicalFormationRule::new("Carbon", 10.0, 0.25).unwrap();
        assert!((rule.amount_to_realize(40.0) - 10.0).abs() < 1e-12);
        assert!(!rule.eligible(&Material::free_base("Carbon", 9.0), &default_catalog()));
        assert!(rule.eligible(&Material::free_base("Carbon", 40.0), &default_catalog()));
    }

    #[test]
    fn formation_creates_physical_object_and_preserves_total_material() {
        let mut environment = environment();
        environment.field.deposit_at_index(0, Material::free_base("Carbon", 20.0));
        let rule = PhysicalFormationRule::new("Carbon", 10.0, 0.25).unwrap();
        let before = environment.field.total_amount() + environment.physical.total_material_amount();

        assert_eq!(apply_physical_formation(&mut environment, &[rule]), 1);
        assert!((environment.field.total_amount() - 15.0).abs() < 1e-12);
        assert!((environment.physical.total_material_amount() - 5.0).abs() < 1e-12);
        assert!((environment.field.total_amount() + environment.physical.total_material_amount() - before).abs() < 1e-12);
    }

    #[test]
    fn formation_does_not_realize_fluid_material() {
        let mut environment = environment();
        environment.field.deposit_at_index(0, Material::free_base("Water", 20.0));
        let rule = PhysicalFormationRule::new("Water", 1.0, 0.5).unwrap();
        assert_eq!(apply_physical_formation(&mut environment, &[rule]), 0);
        assert!(environment.physical.is_empty());
        assert!((environment.field.total_amount() - 20.0).abs() < 1e-12);
    }

    #[test]
    fn formation_does_not_flatten_existing_structure() {
        let mut environment = environment();
        let material = Material {
            parts: vec![("Carbon".into(), 5.0), ("Hydrogen".into(), 5.0)],
            internal_bonds: vec![crate::resources::InternalBond { part_a: 0, part_b: 1 }],
        };
        environment.field.deposit_at_index(0, material.clone());
        let rule = PhysicalFormationRule::new("Carbon", 1.0, 0.5).unwrap();
        assert_eq!(apply_physical_formation(&mut environment, &[rule]), 0);
        assert!(environment.physical.is_empty());
        assert_eq!(environment.field.cells[0].materials[0], material);
    }
}
