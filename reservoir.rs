// Deep reservoir: coarse spatial stock beneath the active material field.

use serde::{Deserialize, Serialize};
use crate::field::{ActiveMaterialField, MATERIAL_EPSILON};

pub const DEFAULT_RESERVOIR_BLOCK_SIZE: usize = 5;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ReservoirCell {
    pub bonded_entries: Vec<(String, f64)>,
    pub unbonded_entries: Vec<(String, f64)>,
}

impl ReservoirCell {
    fn entries(&self, bonded: bool) -> &Vec<(String, f64)> {
        if bonded { &self.bonded_entries } else { &self.unbonded_entries }
    }

    fn entries_mut(&mut self, bonded: bool) -> &mut Vec<(String, f64)> {
        if bonded { &mut self.bonded_entries } else { &mut self.unbonded_entries }
    }

    pub fn amount_of(&self, bonded: bool, name: &str) -> f64 {
        self.entries(bonded).iter().find(|(n, _)| n == name).map(|(_, a)| *a).unwrap_or(0.0)
    }

    pub fn add(&mut self, bonded: bool, name: &str, amount: f64) {
        if amount <= 0.0 { return; }
        let entries = self.entries_mut(bonded);
        if let Some(existing) = entries.iter_mut().find(|(n, _)| n == name) { existing.1 += amount; }
        else { entries.push((name.to_string(), amount)); }
    }

    pub fn take(&mut self, bonded: bool, name: &str, amount: f64) -> f64 {
        if amount <= 0.0 { return 0.0; }
        let entries = self.entries_mut(bonded);
        if let Some(existing) = entries.iter_mut().find(|(n, _)| n == name) {
            let drawn = amount.min(existing.1);
            existing.1 -= drawn;
            drawn
        } else { 0.0 }
    }

    /// Draw from combined bonded + unbonded stock proportionally to availability.
    /// No bonded/unbonded preference is applied; each unit preserves its state.
    pub fn take_indiscriminate(&mut self, name: &str, amount: f64) -> (f64, f64) {
        if amount <= 0.0 { return (0.0, 0.0); }
        let bonded_available = self.amount_of(true, name);
        let unbonded_available = self.amount_of(false, name);
        let total_available = bonded_available + unbonded_available;
        if total_available <= MATERIAL_EPSILON { return (0.0, 0.0); }
        let draw_total = amount.min(total_available);
        let bonded_draw_request = draw_total * bonded_available / total_available;
        let unbonded_draw_request = draw_total - bonded_draw_request;
        let bonded_draw = self.take(true, name, bonded_draw_request);
        let unbonded_draw = self.take(false, name, unbonded_draw_request);
        (bonded_draw, unbonded_draw)
    }

    pub fn total_amount(&self) -> f64 {
        let bonded: f64 = self.bonded_entries.iter().map(|(_, a)| a).sum();
        let unbonded: f64 = self.unbonded_entries.iter().map(|(_, a)| a).sum();
        bonded + unbonded
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DeepReservoir {
    pub block_size: usize,
    pub width_cells: usize,
    pub height_cells: usize,
    pub cells: Vec<ReservoirCell>,
}

impl DeepReservoir {
    pub fn new_matching_field(field: &ActiveMaterialField, block_size: usize) -> Self {
        let block_size = block_size.max(1);
        let width_cells = ((field.width_cells as f64) / block_size as f64).ceil().max(1.0) as usize;
        let height_cells = ((field.height_cells as f64) / block_size as f64).ceil().max(1.0) as usize;
        let cells = (0..width_cells * height_cells).map(|_| ReservoirCell::default()).collect();
        Self { block_size, width_cells, height_cells, cells }
    }

    pub fn reservoir_index_for_field_index(&self, field: &ActiveMaterialField, field_index: usize) -> usize {
        let field_row = field_index / field.width_cells;
        let field_col = field_index % field.width_cells;
        let reservoir_row = (field_row / self.block_size).min(self.height_cells - 1);
        let reservoir_col = (field_col / self.block_size).min(self.width_cells - 1);
        reservoir_row * self.width_cells + reservoir_col
    }

    pub fn seed_uniform(&mut self, name: &str, total_amount: f64) {
        if self.cells.is_empty() || total_amount <= 0.0 { return; }
        let per_cell = total_amount / self.cells.len() as f64;
        for cell in &mut self.cells { cell.add(false, name, per_cell); }
    }

    pub fn total_material(&self) -> Vec<(String, f64)> {
        let mut totals: Vec<(String, f64)> = Vec::new();
        for cell in &self.cells {
            for (name, amount) in cell.bonded_entries.iter().chain(cell.unbonded_entries.iter()) {
                if let Some(existing) = totals.iter_mut().find(|(n, _)| n == name) { existing.1 += amount; }
                else { totals.push((name.clone(), *amount)); }
            }
        }
        totals
    }

    pub fn total_amount(&self) -> f64 { self.cells.iter().map(ReservoirCell::total_amount).sum() }
}
