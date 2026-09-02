// Deep reservoir: coarse spatial stock beneath the active material field.

use super::field::{ActiveMaterialField, MATERIAL_EPSILON};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

pub const DEFAULT_RESERVOIR_BLOCK_SIZE: usize = 5;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ReservoirCell {
    pub bonded_entries: Vec<(String, f64)>,
    pub unbonded_entries: Vec<(String, f64)>,
}

impl ReservoirCell {
    fn entries(&self, bonded: bool) -> &Vec<(String, f64)> {
        if bonded {
            &self.bonded_entries
        } else {
            &self.unbonded_entries
        }
    }
    fn entries_mut(&mut self, bonded: bool) -> &mut Vec<(String, f64)> {
        if bonded {
            &mut self.bonded_entries
        } else {
            &mut self.unbonded_entries
        }
    }
    pub fn amount_of(&self, bonded: bool, name: &str) -> f64 {
        self.entries(bonded)
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, a)| *a)
            .unwrap_or(0.0)
    }
    pub fn add(&mut self, bonded: bool, name: &str, amount: f64) {
        if amount <= 0.0 {
            return;
        }
        let entries = self.entries_mut(bonded);
        if let Some(existing) = entries.iter_mut().find(|(n, _)| n == name) {
            existing.1 += amount;
        } else {
            entries.push((name.to_string(), amount));
        }
    }
    pub fn take(&mut self, bonded: bool, name: &str, amount: f64) -> f64 {
        if amount <= 0.0 {
            return 0.0;
        }
        let entries = self.entries_mut(bonded);
        if let Some(existing) = entries.iter_mut().find(|(n, _)| n == name) {
            let drawn = amount.min(existing.1);
            existing.1 -= drawn;
            drawn
        } else {
            0.0
        }
    }
    pub fn take_indiscriminate(&mut self, name: &str, amount: f64) -> (f64, f64) {
        if amount <= 0.0 {
            return (0.0, 0.0);
        }
        let bonded_available = self.amount_of(true, name);
        let unbonded_available = self.amount_of(false, name);
        let total_available = bonded_available + unbonded_available;
        if total_available <= MATERIAL_EPSILON {
            return (0.0, 0.0);
        }
        let draw_total = amount.min(total_available);
        let bonded_draw_request = draw_total * bonded_available / total_available;
        let unbonded_draw_request = draw_total - bonded_draw_request;
        (
            self.take(true, name, bonded_draw_request),
            self.take(false, name, unbonded_draw_request),
        )
    }
    pub fn total_amount(&self) -> f64 {
        self.bonded_entries.iter().map(|(_, a)| a).sum::<f64>()
            + self.unbonded_entries.iter().map(|(_, a)| a).sum::<f64>()
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
            cell.add(false, name, per_cell);
        }
    }

    /// Replace the initial uniform unbonded stock with a deterministic,
    /// seed-dependent spatial distribution while preserving each resource's
    /// total amount. This is part of initial environment generation only; it
    /// does not alter the organism or introduce a second resource source.
    pub fn randomize_unbonded_distribution(&mut self, seed: u64) {
        if self.cells.is_empty() {
            return;
        }
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let resource_names: Vec<String> = self
            .cells
            .iter()
            .flat_map(|cell| cell.unbonded_entries.iter().map(|(name, _)| name.clone()))
            .fold(Vec::new(), |mut names, name| {
                if !names.contains(&name) {
                    names.push(name);
                }
                names
            });

        for name in resource_names {
            let total = self
                .cells
                .iter()
                .map(|cell| cell.amount_of(false, &name))
                .sum::<f64>();
            if total <= 0.0 {
                continue;
            }

            let weights: Vec<f64> = (0..self.cells.len())
                .map(|_| rng.gen_range(0.25..1.75))
                .collect();
            let weight_sum: f64 = weights.iter().sum();
            if weight_sum <= 0.0 || !weight_sum.is_finite() {
                continue;
            }

            for (cell, weight) in self.cells.iter_mut().zip(weights) {
                cell.unbonded_entries
                    .retain(|(entry_name, _)| entry_name != &name);
                cell.add(false, &name, total * weight / weight_sum);
            }
        }
    }

    pub fn total_material(&self) -> Vec<(String, f64)> {
        let mut totals: Vec<(String, f64)> = Vec::new();
        for cell in &self.cells {
            for (name, amount) in cell
                .bonded_entries
                .iter()
                .chain(cell.unbonded_entries.iter())
            {
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
