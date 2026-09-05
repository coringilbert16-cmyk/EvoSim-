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
        let mut retained = Vec::new();
        let materials = std::mem::take(&mut field.cells[field_index].materials);

        for mut material in materials {
            // The current deep reservoir is intentionally an aggregate
            // ecological store. Until Phase 5 gives it material identity,
            // structured material must remain in the active field rather than
            // being flattened and losing its physical structure.
            if material.has_internal_structure() {
                retained.push(material);
                continue;
            }

            let total = material.total_amount();
            if total <= MATERIAL_EPSILON {
                continue;
            }
            let outflow = total * fraction;
            if outflow <= MATERIAL_EPSILON {
                retained.push(material);
                continue;
            }

            if let Some(taken) = material.take(outflow) {
                for (name, amount) in taken.parts {
                    reservoir.cells[reservoir_index].add(&name, amount);
                }
            }
            if !material.is_empty() {
                retained.push(material);
            }
        }

        field.cells[field_index].materials = retained;
    }
}
