// Settling returns active-field material to the matching deep-reservoir region.

use crate::field::{ActiveMaterialField, MATERIAL_EPSILON};
use crate::reservoir::DeepReservoir;

pub const DEFAULT_SETTLING_FRACTION: f64 = 0.01;
pub const DEFAULT_SETTLING_INTERVAL_TICKS: u64 = 10;

pub fn apply_settling(field: &mut ActiveMaterialField, reservoir: &mut DeepReservoir, fraction: f64) {
    let fraction = fraction.clamp(0.0, 1.0);
    if fraction <= 0.0 { return; }

    for field_index in 0..field.cells.len() {
        let reservoir_index = reservoir.reservoir_index_for_field_index(field, field_index);

        let bonded_total = field.cells[field_index].bonded.total_amount();
        if bonded_total > MATERIAL_EPSILON {
            let outflow = bonded_total * fraction;
            if outflow > MATERIAL_EPSILON {
                if let Some(taken) = field.cells[field_index].bonded.take(outflow) {
                    for (name, amount) in taken.parts { reservoir.cells[reservoir_index].add(true, &name, amount); }
                }
            }
        }

        let unbonded_total = field.cells[field_index].unbonded.total_amount();
        if unbonded_total > MATERIAL_EPSILON {
            let outflow = unbonded_total * fraction;
            if outflow > MATERIAL_EPSILON {
                if let Some(taken) = field.cells[field_index].unbonded.take(outflow) {
                    for (name, amount) in taken.parts { reservoir.cells[reservoir_index].add(false, &name, amount); }
                }
            }
        }
    }
}
