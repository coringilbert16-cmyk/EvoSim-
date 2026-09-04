// Vents transfer existing local reservoir material into the active field.

use super::field::{ActiveMaterialField, MATERIAL_EPSILON};
use super::reservoir::DeepReservoir;
use crate::resources::Material;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Vent {
    pub x: f64,
    pub y: f64,
    pub composition: Vec<(String, f64)>,
    pub emission_amount: f64,
    pub emission_interval: u64,
    pub emission_timer: u64,
}

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
        let reservoir_index = reservoir.reservoir_index_for_field_index(field, field_index);
        let mut parts = Vec::new();
        for (name, proportion) in &vent.composition {
            let requested = vent.emission_amount * proportion;
            let drawn = reservoir.cells[reservoir_index].take_indiscriminate(name, requested);
            if drawn > MATERIAL_EPSILON {
                parts.push((name.clone(), drawn));
            }
        }
        if !parts.is_empty() {
            field.deposit_at_index(
                field_index,
                Material {
                    parts,
                    bonded: false,
                },
            );
        }
    }
}
