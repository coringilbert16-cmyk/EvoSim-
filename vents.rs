// Vents transfer existing local reservoir material into the active field.

use serde::{Deserialize, Serialize};
use super::field::{ActiveMaterialField, MATERIAL_EPSILON};
use super::reservoir::DeepReservoir;
use crate::resources::Material;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Vent { pub x: f64, pub y: f64, pub composition: Vec<(String, f64)>, pub emission_amount: f64, pub emission_interval: u64, pub emission_timer: u64 }

pub fn apply_vents(field: &mut ActiveMaterialField, reservoir: &mut DeepReservoir, vents: &mut [Vent]) {
    for vent in vents.iter_mut() {
        if vent.emission_timer > 0 { vent.emission_timer -= 1; continue; }
        vent.emission_timer = vent.emission_interval;
        let Some(field_index) = field.index_for_position(vent.x, vent.y) else { continue; };
        let reservoir_index = reservoir.reservoir_index_for_field_index(field, field_index);
        let mut bonded_parts = Vec::new(); let mut unbonded_parts = Vec::new();
        for (name, proportion) in &vent.composition {
            let requested = vent.emission_amount * proportion;
            let (from_bonded, from_unbonded) = reservoir.cells[reservoir_index].take_indiscriminate(name, requested);
            if from_bonded > MATERIAL_EPSILON { bonded_parts.push((name.clone(), from_bonded)); }
            if from_unbonded > MATERIAL_EPSILON { unbonded_parts.push((name.clone(), from_unbonded)); }
        }
        if !bonded_parts.is_empty() { field.deposit_at_index(field_index, Material { parts: bonded_parts, bonded: true }); }
        if !unbonded_parts.is_empty() { field.deposit_at_index(field_index, Material { parts: unbonded_parts, bonded: false }); }
    }
}
