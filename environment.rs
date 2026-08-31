// Public environment facade.
//
// The existing field/reservoir implementation is kept intact in
// environment_impl.rs. This facade deliberately owns the vent policy so the
// simulation cannot accidentally use the old bonded-preference/bootstrap path.
#[path = "environment_impl.rs"]
mod implementation;

pub use implementation::{
    ActiveMaterialField,
    DeepReservoir,
    FieldCell,
    ReservoirCell,
    Vent,
    MATERIAL_EPSILON,
    DEFAULT_CELL_SIZE,
    DEFAULT_DIFFUSION_FRACTION,
    DEFAULT_RESERVOIR_BLOCK_SIZE,
    DEFAULT_SETTLING_FRACTION,
    DEFAULT_SETTLING_INTERVAL_TICKS,
    apply_settling,
};

use crate::resources::Material;

/// Transfers existing reservoir material to the active field without any
/// preference for bonded or unbonded stock.
///
/// For each requested resource amount, the draw is proportional to the
/// bonded and unbonded amounts available in the local reservoir cell. Thus a
/// vent has no opinion about material state: it neither creates bonds nor
/// destroys them. The state of every transferred amount is preserved.
pub fn apply_vents(
    field: &mut ActiveMaterialField,
    reservoir: &mut DeepReservoir,
    vents: &mut [Vent],
) {
    for vent in vents.iter_mut() {
        if vent.emission_timer > 0 {
            vent.emission_timer -= 1;
            continue;
        }

        vent.emission_timer = vent.emission_interval;

        let Some(field_index) = field.index_for_position(vent.x, vent.y) else {
            continue;
        };

        let reservoir_index =
            reservoir.reservoir_index_for_field_index(field, field_index);

        let mut bonded_parts = Vec::new();
        let mut unbonded_parts = Vec::new();

        for (name, proportion) in &vent.composition {
            let requested = (vent.emission_amount * proportion).max(0.0);
            if requested <= MATERIAL_EPSILON {
                continue;
            }

            let bonded_available =
                reservoir.cells[reservoir_index].amount_of(true, name);
            let unbonded_available =
                reservoir.cells[reservoir_index].amount_of(false, name);
            let total_available = bonded_available + unbonded_available;

            if total_available <= MATERIAL_EPSILON {
                continue;
            }

            let drawn_total = requested.min(total_available);
            let bonded_share = if total_available > 0.0 {
                drawn_total * (bonded_available / total_available)
            } else {
                0.0
            };
            let unbonded_share = drawn_total - bonded_share;

            let bonded_drawn = reservoir.cells[reservoir_index]
                .take(true, name, bonded_share);
            let unbonded_drawn = reservoir.cells[reservoir_index]
                .take(false, name, unbonded_share);

            if bonded_drawn > MATERIAL_EPSILON {
                bonded_parts.push((name.clone(), bonded_drawn));
            }
            if unbonded_drawn > MATERIAL_EPSILON {
                unbonded_parts.push((name.clone(), unbonded_drawn));
            }
        }

        if !bonded_parts.is_empty() {
            field.deposit_at_index(
                field_index,
                Material {
                    parts: bonded_parts,
                    bonded: true,
                },
            );
        }

        if !unbonded_parts.is_empty() {
            field.deposit_at_index(
                field_index,
                Material {
                    parts: unbonded_parts,
                    bonded: false,
                },
            );
        }
    }
}

#[cfg(test)]
mod vent_policy_tests {
    use super::*;

    #[test]
    fn vent_preserves_unbonded_state_when_only_raw_stock_exists() {
        let mut field = ActiveMaterialField::new(1000.0, 1000.0, DEFAULT_CELL_SIZE);
        let mut reservoir = DeepReservoir::new_matching_field(
            &field,
            DEFAULT_RESERVOIR_BLOCK_SIZE,
        );
        let idx = field.index_for_position(500.0, 500.0).unwrap();
        let ridx = reservoir.reservoir_index_for_field_index(&field, idx);
        reservoir.cells[ridx].add(false, "Carbon", 100.0);

        let mut vents = vec![Vent {
            x: 500.0,
            y: 500.0,
            composition: vec![("Carbon".into(), 1.0)],
            emission_amount: 40.0,
            emission_interval: 0,
            emission_timer: 0,
        }];

        apply_vents(&mut field, &mut reservoir, &mut vents);

        assert!((field.cells[idx].unbonded.total_amount() - 40.0).abs() < 1e-9);
        assert!(field.cells[idx].bonded.total_amount() < 1e-9);
        assert!((reservoir.cells[ridx].amount_of(false, "Carbon") - 60.0).abs() < 1e-9);
    }

    #[test]
    fn vent_draws_bonded_and_unbonded_without_preference_and_preserves_state() {
        let mut field = ActiveMaterialField::new(1000.0, 1000.0, DEFAULT_CELL_SIZE);
        let mut reservoir = DeepReservoir::new_matching_field(
            &field,
            DEFAULT_RESERVOIR_BLOCK_SIZE,
        );
        let idx = field.index_for_position(500.0, 500.0).unwrap();
        let ridx = reservoir.reservoir_index_for_field_index(&field, idx);
        reservoir.cells[ridx].add(true, "Carbon", 20.0);
        reservoir.cells[ridx].add(false, "Carbon", 80.0);

        let mut vents = vec![Vent {
            x: 500.0,
            y: 500.0,
            composition: vec![("Carbon".into(), 1.0)],
            emission_amount: 50.0,
            emission_interval: 0,
            emission_timer: 0,
        }];

        apply_vents(&mut field, &mut reservoir, &mut vents);

        assert!((field.cells[idx].bonded.total_amount() - 10.0).abs() < 1e-9);
        assert!((field.cells[idx].unbonded.total_amount() - 40.0).abs() < 1e-9);
        assert!((reservoir.cells[ridx].amount_of(true, "Carbon") - 10.0).abs() < 1e-9);
        assert!((reservoir.cells[ridx].amount_of(false, "Carbon") - 40.0).abs() < 1e-9);
    }
}
