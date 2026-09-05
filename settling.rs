// Settling returns active-field material to the matching deep-reservoir region.

use super::field::{ActiveMaterialField, MATERIAL_EPSILON};
use super::reservoir::DeepReservoir;

pub const DEFAULT_SETTLING_FRACTION: f64 = 0.01;
pub const DEFAULT_SETTLING_INTERVAL_TICKS: u64 = 10;

pub fn apply_settling(
    field: &mut ActiveMaterialField,
    reservoir: &mut DeepReservoir,
    fraction: f64,
) {
    let fraction = fraction.clamp(0.0, 1.0);
    if fraction <= 0.0 {
        return;
    }

    for field_index in 0..field.cells.len() {
        let reservoir_index = reservoir.reservoir_index_for_field_index(field, field_index);
        let material_count = field.cells[field_index].materials.len();

        for material_index in 0..material_count {
            let total = field.cells[field_index].materials[material_index].total_amount();
            if total <= MATERIAL_EPSILON {
                continue;
            }
            let outflow = total * fraction;
            if outflow <= MATERIAL_EPSILON {
                continue;
            }

            if let Some(taken) = field.cells[field_index].materials[material_index].take(outflow) {
                // The deep reservoir remains an aggregate ecological store in
                // Phase 2. Material structure is preserved while it is in the
                // active field; reservoir identity is addressed in Phase 5.
                for (name, amount) in taken.parts {
                    reservoir.cells[reservoir_index].add(&name, amount);
                }
            }
        }

        field.cells[field_index]
            .materials
            .retain(|material| !material.is_empty());
    }
}
