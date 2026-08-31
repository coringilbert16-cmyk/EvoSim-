// ============================================================
// ENVIRONMENT: ACTIVE FIELD + DEEP RESERVOIR + VENTS
// ============================================================
//
// This is the sole environmental material implementation used by
// the simulation. The world has two spatially corresponding layers:
//
//   Layer 1: DeepReservoir  - coarse, persistent material stock
//   Layer 2: ActiveField    - fine, dynamic material organisms perceive
//
// Vents transfer existing material from Layer 1 to Layer 2.
// Diffusion moves material within Layer 2.
// Settling returns material from Layer 2 to Layer 1.
//
// Bonded state is preserved by every environmental transfer. Vents
// have NO preference for bonded or unbonded material and never create
// bonds. Initial seeding is unbonded. Bonded reservoir stock can only
// arise by settling bonded field material back into the reservoir.
//
// This module deliberately contains no organism logic, preference,
// energy logic, or transformation logic.
// ============================================================

use serde::{Deserialize, Serialize};

use crate::resources::{merge_parts, Material};

pub const DEFAULT_CELL_SIZE: f64 = 25.0;
pub const DEFAULT_DIFFUSION_FRACTION: f64 = 0.05;
pub const MATERIAL_EPSILON: f64 = 1e-9;
pub const DEFAULT_RESERVOIR_BLOCK_SIZE: usize = 5;
pub const DEFAULT_SETTLING_FRACTION: f64 = 0.01;
pub const DEFAULT_SETTLING_INTERVAL_TICKS: u64 = 10;

// ============================================================
// ACTIVE FIELD
// ============================================================

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FieldCell {
    pub bonded: Material,
    pub unbonded: Material,
}

impl FieldCell {
    pub fn empty() -> Self {
        Self {
            bonded: Material { parts: Vec::new(), bonded: true },
            unbonded: Material { parts: Vec::new(), bonded: false },
        }
    }

    pub fn total_amount(&self) -> f64 {
        self.bonded.total_amount() + self.unbonded.total_amount()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ActiveMaterialField {
    pub cell_size: f64,
    pub width_cells: usize,
    pub height_cells: usize,
    pub cells: Vec<FieldCell>,
}

impl ActiveMaterialField {
    pub fn new(world_width: f64, world_height: f64, cell_size: f64) -> Self {
        let cell_size = cell_size.max(1.0);
        let width_cells = (world_width / cell_size).ceil().max(1.0) as usize;
        let height_cells = (world_height / cell_size).ceil().max(1.0) as usize;
        let cells = (0..width_cells * height_cells).map(|_| FieldCell::empty()).collect();
        Self { cell_size, width_cells, height_cells, cells }
    }

    pub fn row_col_for_position(&self, x: f64, y: f64) -> Option<(usize, usize)> {
        if !x.is_finite() || !y.is_finite() || x < 0.0 || y < 0.0 { return None; }
        let col = (x / self.cell_size).floor() as usize;
        let row = (y / self.cell_size).floor() as usize;
        if col >= self.width_cells || row >= self.height_cells { return None; }
        Some((row, col))
    }

    pub fn index_for_position(&self, x: f64, y: f64) -> Option<usize> {
        self.row_col_for_position(x, y).map(|(row, col)| row * self.width_cells + col)
    }

    fn row_col_for_index(&self, index: usize) -> (usize, usize) {
        (index / self.width_cells, index % self.width_cells)
    }

    pub fn cell_center(&self, index: usize) -> (f64, f64) {
        let (row, col) = self.row_col_for_index(index);
        ((col as f64 + 0.5) * self.cell_size, (row as f64 + 0.5) * self.cell_size)
    }

    pub fn neighbor_indices(&self, index: usize) -> Vec<usize> {
        let (row, col) = self.row_col_for_index(index);
        let mut out = Vec::with_capacity(4);
        if row > 0 { out.push((row - 1) * self.width_cells + col); }
        if row + 1 < self.height_cells { out.push((row + 1) * self.width_cells + col); }
        if col > 0 { out.push(row * self.width_cells + col - 1); }
        if col + 1 < self.width_cells { out.push(row * self.width_cells + col + 1); }
        out
    }

    /// Returns field cells whose centers lie within the requested radius.
    /// This is the perception broad phase; physical contact applies its own
    /// geometry afterward.
    pub fn cells_within_radius(&self, x: f64, y: f64, radius: f64) -> Vec<usize> {
        if !x.is_finite() || !y.is_finite() || !radius.is_finite() || radius < 0.0 { return Vec::new(); }
        let r2 = radius * radius;
        self.cells.iter().enumerate().filter_map(|(i, _)| {
            let (cx, cy) = self.cell_center(i);
            let dx = cx - x;
            let dy = cy - y;
            if dx * dx + dy * dy <= r2 { Some(i) } else { None }
        }).collect()
    }

    pub fn deposit(&mut self, x: f64, y: f64, material: Material) -> bool {
        let Some(index) = self.index_for_position(x, y) else { return false; };
        self.deposit_at_index(index, material);
        true
    }

    pub fn deposit_at_index(&mut self, index: usize, material: Material) {
        if material.parts.is_empty() || index >= self.cells.len() { return; }
        let target = if material.bonded { &mut self.cells[index].bonded } else { &mut self.cells[index].unbonded };
        let mut parts = std::mem::take(&mut target.parts);
        parts.extend(material.parts);
        target.parts = merge_parts(&parts);
    }

    pub fn take_at(&mut self, x: f64, y: f64, bonded: bool, amount: f64) -> Option<Material> {
        let index = self.index_for_position(x, y)?;
        self.take_at_index(index, bonded, amount)
    }

    pub fn take_at_index(&mut self, index: usize, bonded: bool, amount: f64) -> Option<Material> {
        let cell = self.cells.get_mut(index)?;
        if bonded { cell.bonded.take(amount) } else { cell.unbonded.take(amount) }
    }

    pub fn total_material(&self) -> Vec<(String, f64)> {
        let mut totals = Vec::new();
        for cell in &self.cells {
            for (name, amount) in cell.bonded.parts.iter().chain(cell.unbonded.parts.iter()) {
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
        self.cells.iter().map(FieldCell::total_amount).sum()
    }

    pub fn diffuse_step(&mut self, fraction: f64) {
        let fraction = fraction.clamp(0.0, 1.0);
        if fraction <= 0.0 { return; }

        let mut outgoing_bonded: Vec<Option<Material>> = vec![None; self.cells.len()];
        let mut outgoing_unbonded: Vec<Option<Material>> = vec![None; self.cells.len()];

        for i in 0..self.cells.len() {
            if self.neighbor_indices(i).is_empty() { continue; }
            let bonded_total = self.cells[i].bonded.total_amount();
            if bonded_total > MATERIAL_EPSILON { outgoing_bonded[i] = self.cells[i].bonded.take(bonded_total * fraction); }
            let unbonded_total = self.cells[i].unbonded.total_amount();
            if unbonded_total > MATERIAL_EPSILON { outgoing_unbonded[i] = self.cells[i].unbonded.take(unbonded_total * fraction); }
        }

        for i in 0..self.cells.len() {
            let neighbors = self.neighbor_indices(i);
            if let Some(material) = outgoing_bonded[i].take() { distribute_evenly(self, material, &neighbors); }
            if let Some(material) = outgoing_unbonded[i].take() { distribute_evenly(self, material, &neighbors); }
        }
    }
}

// ============================================================
// DEEP RESERVOIR
// ============================================================

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ReservoirCell {
    pub bonded_entries: Vec<(String, f64)>,
    pub unbonded_entries: Vec<(String, f64)>,
}

impl ReservoirCell {
    fn entries(&self, bonded: bool) -> &[(String, f64)] {
        if bonded { &self.bonded_entries } else { &self.unbonded_entries }
    }

    fn entries_mut(&mut self, bonded: bool) -> &mut Vec<(String, f64)> {
        if bonded { &mut self.bonded_entries } else { &mut self.unbonded_entries }
    }

    pub fn amount_of(&self, bonded: bool, name: &str) -> f64 {
        self.entries(bonded).iter().find(|(n, _)| n == name).map(|(_, a)| *a).unwrap_or(0.0)
    }

    pub fn add(&mut self, bonded: bool, name: &str, amount: f64) {
        if !amount.is_finite() || amount <= 0.0 { return; }
        let entries = self.entries_mut(bonded);
        if let Some(existing) = entries.iter_mut().find(|(n, _)| n == name) { existing.1 += amount; }
        else { entries.push((name.to_owned(), amount)); }
    }

    pub fn take(&mut self, bonded: bool, name: &str, amount: f64) -> f64 {
        if !amount.is_finite() || amount <= 0.0 { return 0.0; }
        let entries = self.entries_mut(bonded);
        if let Some(existing) = entries.iter_mut().find(|(n, _)| n == name) {
            let drawn = amount.min(existing.1);
            existing.1 -= drawn;
            drawn
        } else { 0.0 }
    }

    pub fn total_amount(&self) -> f64 {
        self.bonded_entries.iter().map(|(_, a)| a).sum::<f64>() + self.unbonded_entries.iter().map(|(_, a)| a).sum::<f64>()
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
        let row = (field_row / self.block_size).min(self.height_cells - 1);
        let col = (field_col / self.block_size).min(self.width_cells - 1);
        row * self.width_cells + col
    }

    pub fn seed_uniform(&mut self, name: &str, total_amount: f64) {
        if self.cells.is_empty() || !total_amount.is_finite() || total_amount <= 0.0 { return; }
        let per_cell = total_amount / self.cells.len() as f64;
        for cell in &mut self.cells { cell.add(false, name, per_cell); }
    }

    pub fn total_material(&self) -> Vec<(String, f64)> {
        let mut totals = Vec::new();
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

// ============================================================
// VENTS
// ============================================================

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Vent {
    pub x: f64,
    pub y: f64,
    pub composition: Vec<(String, f64)>,
    pub emission_amount: f64,
    pub emission_interval: u64,
    pub emission_timer: u64,
}

/// A vent is an indiscriminate transfer mechanism. For each requested
/// resource type, the vent draws from the local reservoir in proportion
/// to the bonded/unbonded stock available. It never changes bonded state.
pub fn apply_vents(field: &mut ActiveMaterialField, reservoir: &mut DeepReservoir, vents: &mut [Vent]) {
    for vent in vents.iter_mut() {
        if vent.emission_timer > 0 {
            vent.emission_timer -= 1;
            continue;
        }
        vent.emission_timer = vent.emission_interval;

        let Some(field_index) = field.index_for_position(vent.x, vent.y) else { continue; };
        let reservoir_index = reservoir.reservoir_index_for_field_index(field, field_index);
        let mut bonded_parts = Vec::new();
        let mut unbonded_parts = Vec::new();

        for (name, proportion) in &vent.composition {
            let requested = (vent.emission_amount * proportion).max(0.0);
            if requested <= MATERIAL_EPSILON { continue; }

            let bonded_available = reservoir.cells[reservoir_index].amount_of(true, name);
            let unbonded_available = reservoir.cells[reservoir_index].amount_of(false, name);
            let total_available = bonded_available + unbonded_available;
            if total_available <= MATERIAL_EPSILON { continue; }

            let drawn = requested.min(total_available);
            let bonded_request = drawn * bonded_available / total_available;
            let unbonded_request = drawn - bonded_request;
            let bonded_drawn = reservoir.cells[reservoir_index].take(true, name, bonded_request);
            let unbonded_drawn = reservoir.cells[reservoir_index].take(false, name, unbonded_request);

            if bonded_drawn > MATERIAL_EPSILON { bonded_parts.push((name.clone(), bonded_drawn)); }
            if unbonded_drawn > MATERIAL_EPSILON { unbonded_parts.push((name.clone(), unbonded_drawn)); }
        }

        if !bonded_parts.is_empty() {
            field.deposit_at_index(field_index, Material { parts: bonded_parts, bonded: true });
        }
        if !unbonded_parts.is_empty() {
            field.deposit_at_index(field_index, Material { parts: unbonded_parts, bonded: false });
        }
    }
}

// ============================================================
// SETTLING
// ============================================================

/// Returns a fraction of both field stacks to the corresponding reservoir
/// cell. Bonded remains bonded; unbonded remains unbonded.
pub fn apply_settling(field: &mut ActiveMaterialField, reservoir: &mut DeepReservoir, fraction: f64) {
    let fraction = fraction.clamp(0.0, 1.0);
    if fraction <= 0.0 { return; }

    for field_index in 0..field.cells.len() {
        let reservoir_index = reservoir.reservoir_index_for_field_index(field, field_index);

        let bonded_total = field.cells[field_index].bonded.total_amount();
        if bonded_total > MATERIAL_EPSILON {
            if let Some(taken) = field.cells[field_index].bonded.take(bonded_total * fraction) {
                for (name, amount) in taken.parts { reservoir.cells[reservoir_index].add(true, &name, amount); }
            }
        }

        let unbonded_total = field.cells[field_index].unbonded.total_amount();
        if unbonded_total > MATERIAL_EPSILON {
            if let Some(taken) = field.cells[field_index].unbonded.take(unbonded_total * fraction) {
                for (name, amount) in taken.parts { reservoir.cells[reservoir_index].add(false, &name, amount); }
            }
        }
    }
}

fn distribute_evenly(field: &mut ActiveMaterialField, mut material: Material, neighbors: &[usize]) {
    if neighbors.is_empty() { return; }
    let share = material.total_amount() / neighbors.len() as f64;
    for (i, &neighbor) in neighbors.iter().enumerate() {
        let piece = if i == neighbors.len() - 1 {
            Material { parts: std::mem::take(&mut material.parts), bonded: material.bonded }
        } else {
            match material.take(share) { Some(piece) => piece, None => continue }
        };
        field.deposit_at_index(neighbor, piece);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bonded(name: &str, amount: f64) -> Material {
        Material { parts: vec![(name.to_owned(), amount)], bonded: true }
    }
    fn raw(name: &str, amount: f64) -> Material {
        Material { parts: vec![(name.to_owned(), amount)], bonded: false }
    }

    #[test]
    fn field_has_expected_dimensions() {
        let field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
        assert_eq!((field.width_cells, field.height_cells), (40, 40));
        assert_eq!(field.cells.len(), 1600);
    }

    #[test]
    fn field_preserves_bonded_and_unbonded_stacks() {
        let mut field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
        field.deposit(500.0, 500.0, bonded("Methane", 10.0));
        field.deposit(500.0, 500.0, raw("Carbon", 3.0));
        let cell = &field.cells[field.index_for_position(500.0, 500.0).unwrap()];
        assert!((cell.bonded.total_amount() - 10.0).abs() < 1e-9);
        assert!((cell.unbonded.total_amount() - 3.0).abs() < 1e-9);
    }

    #[test]
    fn diffusion_conserves_total_mass() {
        let mut field = ActiveMaterialField::new(300.0, 300.0, 25.0);
        field.deposit(150.0, 150.0, bonded("Methane", 200.0));
        field.deposit(0.0, 0.0, raw("Carbon", 80.0));
        let before = field.total_amount();
        for _ in 0..100 { field.diffuse_step(0.1); }
        assert!((field.total_amount() - before).abs() < 1e-6);
    }

    #[test]
    fn seeding_is_unbonded() {
        let field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
        let mut reservoir = DeepReservoir::new_matching_field(&field, 5);
        reservoir.seed_uniform("Carbon", 6400.0);
        assert!((reservoir.total_amount() - 6400.0).abs() < 1e-6);
        assert_eq!(reservoir.cells[0].amount_of(true, "Carbon"), 0.0);
    }

    #[test]
    fn vent_draw_is_spatially_local_and_state_preserving() {
        let mut field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
        let mut reservoir = DeepReservoir::new_matching_field(&field, 5);
        let idx = field.index_for_position(500.0, 500.0).unwrap();
        let ridx = reservoir.reservoir_index_for_field_index(&field, idx);
        reservoir.cells[ridx].add(true, "Carbon", 20.0);
        reservoir.cells[ridx].add(false, "Carbon", 80.0);

        let mut vents = vec![Vent {
            x: 500.0, y: 500.0,
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

    #[test]
    fn settling_preserves_state() {
        let mut field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
        let mut reservoir = DeepReservoir::new_matching_field(&field, 5);
        let idx = field.index_for_position(500.0, 500.0).unwrap();
        field.deposit_at_index(idx, bonded("Sulfur", 100.0));
        field.deposit_at_index(idx, raw("Water", 50.0));
        let before = field.total_amount() + reservoir.total_amount();
        for _ in 0..100 { apply_settling(&mut field, &mut reservoir, 0.01); }
        let after = field.total_amount() + reservoir.total_amount();
        assert!((after - before).abs() < 1e-6);
        let ridx = reservoir.reservoir_index_for_field_index(&field, idx);
        assert!(reservoir.cells[ridx].amount_of(true, "Sulfur") > 0.0);
        assert!(reservoir.cells[ridx].amount_of(false, "Water") > 0.0);
    }
}
