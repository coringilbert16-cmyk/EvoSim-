// Deep reservoir: coarse spatial stock beneath the active material field.

use super::field::{ActiveMaterialField, MATERIAL_EPSILON};
use serde::{Deserialize, Serialize};

pub const DEFAULT_RESERVOIR_BLOCK_SIZE: usize = 5;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ReservoirCell {
    pub entries: Vec<(String, f64)>,
}

impl ReservoirCell {
    pub fn amount_of(&self, name: &str) -> f64 {
        self.entries
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, a)| *a)
            .unwrap_or(0.0)
    }

    pub fn add(&mut self, name: &str, amount: f64) {
        if amount <= 0.0 {
            return;
        }
        if let Some(existing) = self.entries.iter_mut().find(|(n, _)| n == name) {
            existing.1 += amount;
        } else {
            self.entries.push((name.to_string(), amount));
        }
    }

    pub fn take(&mut self, name: &str, amount: f64) -> f64 {
        if amount <= 0.0 {
            return 0.0;
        }
        if let Some(existing) = self.entries.iter_mut().find(|(n, _)| n == name) {
            let drawn = amount.min(existing.1);
            existing.1 -= drawn;
            drawn
        } else {
            0.0
        }
    }

    pub fn take_indiscriminate(&mut self, name: &str, amount: f64) -> f64 {
        if amount <= 0.0 {
            return 0.0;
        }
        let available = self.amount_of(name);
        if available <= MATERIAL_EPSILON {
            return 0.0;
        }
        self.take(name, amount.min(available))
    }

    pub fn total_amount(&self) -> f64 {
        self.entries.iter().map(|(_, a)| a).sum()
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
        let width_cells = ((field.width_cells as f64) / block_size as f64)
            .ceil()
            .max(1.0) as usize;
        let height_cells = ((field.height_cells as f64) / block_size as f64)
            .ceil()
            .max(1.0) as usize;
        let cells = (0..width_cells * height_cells)
            .map(|_| ReservoirCell::default())
            .collect();
        Self {
            block_size,
            width_cells,
            height_cells,
            cells,
        }
    }

    pub fn reservoir_index_for_field_index(
        &self,
        field: &ActiveMaterialField,
        field_index: usize,
    ) -> usize {
        let field_row = field_index / field.width_cells;
        let field_col = field_index % field.width_cells;
        let reservoir_row = (field_row / self.block_size).min(self.height_cells - 1);
        let reservoir_col = (field_col / self.block_size).min(self.width_cells - 1);
        reservoir_row * self.width_cells + reservoir_col
    }

    pub fn seed_uniform(&mut self, name: &str, total_amount: f64) {
        if self.cells.is_empty() || total_amount <= 0.0 {
            return;
        }
        let per_cell = total_amount / self.cells.len() as f64;
        for cell in &mut self.cells {
            cell.add(name, per_cell);
        }
    }

    pub fn total_material(&self) -> Vec<(String, f64)> {
        let mut totals: Vec<(String, f64)> = Vec::new();
        for cell in &self.cells {
            for (name, amount) in &cell.entries {
                if let Some(existing) = totals.iter_mut().find(|(n, _)| n == name) {
                    existing.1 += amount;
                } else {
                    totals.push((name.clone(), *amount));
                }
            }
        }
        totals
    }

    pub fn total_amount(&self) -> f64 {
        self.cells.iter().map(ReservoirCell::total_amount).sum()
    }
}
