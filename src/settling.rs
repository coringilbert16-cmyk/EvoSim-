// Settling returns active-field material to the matching deep-reservoir region.

use super::field::{ActiveMaterialField, MATERIAL_EPSILON};
use crate::material_transfer::take_whole_unstructured;
use super::reservoir::DeepReservoir;

pub const DEFAULT_SETTLING_FRACTION: f64 = 0.01;
pub const DEFAULT_SETTLING_INTERVAL_TICKS: u64 = 10;

pub fn apply_settling(
    field: &mut ActiveMaterialField,
    reservoir: &mut DeepReservoir,
    fraction: f64,
) {
    let fraction = fraction.clamp(0.0, 1.0);
    if fraction <= 0.0 { return; }

    for field_index in 0..field.cells.len() {
        let reservoir_index = reservoir.reservoir_index_for_field_index(field, field_index);
        let mut retained = Vec::new();
        let materials = std::mem::take(&mut field.cells[field_index].materials);

        for mut field_material in materials {
            // The deep reservoir is an aggregate ecological store. Structured
            // field material has physical constituent geometry and therefore
            // cannot be flattened into the reservoir without losing state.
            if field_material.material.has_internal_structure() {
                retained.push(field_material);
                continue;
            }

            let total = field_material.material.total_amount();
            if total <= MATERIAL_EPSILON { continue; }
            let outflow = (total * fraction).floor() as usize;
            if outflow == 0 {
                retained.push(field_material);
                continue;
            }

            if let Some(taken) = take_whole_unstructured(&mut field_material.material, outflow) {
                for (name, amount) in taken.parts {
                    reservoir.cells[reservoir_index].add(&name, amount);
                }
            }
            if !field_material.material.is_empty() {
                retained.push(field_material);
            }
        }

        field.cells[field_index].materials = retained;
    }
}
