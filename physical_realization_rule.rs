//! Deterministic policy for deciding when bulk material may be realized physically.
//!
//! This layer intentionally contains policy, while `physical_realization` owns the
//! conservative state transition. A rule produces a specification only; it never
//! mutates the field or physical environment.

use crate::environment::ActiveMaterialField;
use crate::resources::Material;
use crate::structure::Placement;

#[derive(Clone, Debug, PartialEq)]
pub struct PhysicalRealizationSpec {
    pub field_cell_index: usize,
    pub material_index: usize,
    pub amount: f64,
    pub placements: Vec<Placement>,
}

pub trait PhysicalRealizationRule {
    fn propose(
        &self,
        field: &ActiveMaterialField,
        cell_index: usize,
        material_index: usize,
    ) -> Option<PhysicalRealizationSpec>;
}

/// Explicit caller-supplied realization policy.
///
/// `amount` is the quantity consumed from bulk stock. The caller also supplies
/// the physical placement, so this rule never invents geometry from quantity.
#[derive(Clone, Debug, PartialEq)]
pub struct ExplicitRealizationRule {
    pub minimum_amount: f64,
    pub amount: f64,
    pub placements: Vec<Placement>,
}

impl ExplicitRealizationRule {
    pub fn new(minimum_amount: f64, amount: f64, placements: Vec<Placement>) -> Option<Self> {
        if !minimum_amount.is_finite()
            || minimum_amount < 0.0
            || !amount.is_finite()
            || amount <= 0.0
            || placements.is_empty()
        {
            return None;
        }
        Some(Self {
            minimum_amount,
            amount,
            placements,
        })
    }
}

impl PhysicalRealizationRule for ExplicitRealizationRule {
    fn propose(
        &self,
        field: &ActiveMaterialField,
        cell_index: usize,
        material_index: usize,
    ) -> Option<PhysicalRealizationSpec> {
        let cell = field.cells.get(cell_index)?;
        let material = cell.materials.get(material_index)?;
        if !material.is_valid() || material.total_amount() < self.minimum_amount {
            return None;
        }
        Some(PhysicalRealizationSpec {
            field_cell_index: cell_index,
            material_index,
            amount: self.amount.min(material.total_amount()),
            placements: self.placements.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::FieldCell;

    fn material() -> Material {
        Material::free_base("Carbon", 10.0).unwrap()
    }

    #[test]
    fn rule_only_proposes_when_threshold_is_met() {
        let field = ActiveMaterialField {
            width: 1,
            height: 1,
            cells: vec![FieldCell {
                materials: vec![material()],
            }],
        };
        let rule = ExplicitRealizationRule::new(5.0, 4.0, vec![Placement::default()]).unwrap();
        assert_eq!(rule.propose(&field, 0, 0).unwrap().amount, 4.0);
    }

    #[test]
    fn rule_caps_realized_amount_to_available_quantity() {
        let field = ActiveMaterialField {
            width: 1,
            height: 1,
            cells: vec![FieldCell {
                materials: vec![material()],
            }],
        };
        let rule = ExplicitRealizationRule::new(5.0, 20.0, vec![Placement::default()]).unwrap();
        assert_eq!(rule.propose(&field, 0, 0).unwrap().amount, 10.0);
    }

    #[test]
    fn invalid_rule_is_rejected() {
        assert!(ExplicitRealizationRule::new(0.0, 0.0, vec![Placement::default()]).is_none());
        assert!(ExplicitRealizationRule::new(0.0, 1.0, Vec::new()).is_none());
    }
}
