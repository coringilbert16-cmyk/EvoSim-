// ============================================================
// ACTIVE MATERIAL FIELD
// ============================================================
//
// This is the "active material field" compartment from the new
// two-compartment environment architecture (deep reservoir + active
// field). It replaces the old ResourceCloud/emit_resource_clouds/
// update_resource_clouds model, which is being retired.
//
// A field is a fixed-resolution 2D grid over the world. Each cell
// holds AT MOST two Material stacks: one bonded, one unbonded. This
// intentionally reuses the existing Material type from resources.rs
// unchanged (see audit note in conversation: merging same-name parts
// via merge_parts() does not lose anything BREAK/bonding/composition/
// quantity/transformation-selection needs).
//
// This module is deliberately self-contained at this stage: it knows
// how to store, deposit, take, and diffuse material. It does NOT yet
// know about vents, the reservoir, organisms, or perception - those
// are later steps in the migration plan and will be layered on top
// once this foundational piece is verified to conserve mass on its
// own.
// ============================================================

use serde::{Deserialize, Serialize};

use crate::resources::{merge_parts, Material};

// ------------------------------------------------------------
// TUNABLE CONSTANTS
// ------------------------------------------------------------
//
// Per review decisions: cell size, diffusion fraction, and (later)
// settling rate must all be configurable rather than hard-locked, so
// they can be tuned without a redesign. These are the initial values;
// nothing about the architecture depends on these particular numbers.
// ------------------------------------------------------------

/// Default cell size in world units. A 1000x1000 world produces a
/// 40x40 field at this resolution.
pub const DEFAULT_CELL_SIZE: f64 = 25.0;

/// Default fraction of a cell's material that moves out to its
/// neighbors per diffusion step. Tunable; not a locked value.
pub const DEFAULT_DIFFUSION_FRACTION: f64 = 0.05;

/// Floating-point noise floor. Amounts at or below this are treated as zero
/// for the purposes of triggering transfers - this exists only to avoid
/// endlessly shuffling insignificant residue around. It does not delete
/// material; it just skips no-op transfers.
pub const MATERIAL_EPSILON: f64 = 1e-9;

// ------------------------------------------------------------
// FIELD CELL
// ------------------------------------------------------------

/// One grid cell's contents: exactly one bonded stack and one
/// unbonded stack. Both are always present (possibly empty) so cell
/// indexing never has to special-case a missing stack.
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

// ------------------------------------------------------------
// ACTIVE MATERIAL FIELD
// ------------------------------------------------------------

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
        let cells = (0..width_cells * height_cells)
            .map(|_| FieldCell::empty())
            .collect();
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
        if col > 0 { out.push(row * self.width_cells + (col - 1)); }
        if col + 1 < self.width_cells { out.push(row * self.width_cells + (col + 1)); }
        out
    }

    pub fn deposit(&mut self, x: f64, y: f64, material: Material) -> bool {
        match self.index_for_position(x, y) {
            Some(index) => { self.deposit_at_index(index, material); true }
            None => false,
        }
    }

    pub fn deposit_at_index(&mut self, index: usize, material: Material) {
        if material.parts.is_empty() { return; }
        let cell = &mut self.cells[index];
        let target = if material.bonded { &mut cell.bonded } else { &mut cell.unbonded };
        let mut parts = std::mem::take(&mut target.parts);
        parts.extend(material.parts);
        target.parts = merge_parts(&parts);
    }

    pub fn take_at(&mut self, x: f64, y: f64, bonded: bool, amount: f64) -> Option<Material> {
        let index = self.index_for_position(x, y)?;
        self.take_at_index(index, bonded, amount)
    }

    pub fn take_at_index(&mut self, index: usize, bonded: bool, amount: f64) -> Option<Material> {
        let cell = &mut self.cells[index];
        let stack = if bonded { &mut cell.bonded } else { &mut cell.unbonded };
        stack.take(amount)
    }

    pub fn diffuse_step(&mut self, fraction: f64) {
        let fraction = fraction.clamp(0.0, 1.0);
        if fraction <= 0.0 { return; }
        let n = self.cells.len();
        let mut outgoing_bonded: Vec<Option<Material>> = vec![None; n];
        let mut outgoing_unbonded: Vec<Option<Material>> = vec![None; n];

        for i in 0..n {
            let neighbor_count = self.neighbor_indices(i).len();
            if neighbor_count == 0 { continue; }
            let bonded_total = self.cells[i].bonded.total_amount();
            if bonded_total > MATERIAL_EPSILON {
                let outflow = bonded_total * fraction;
                if outflow > MATERIAL_EPSILON { outgoing_bonded[i] = self.cells[i].bonded.take(outflow); }
            }
            let unbonded_total = self.cells[i].unbonded.total_amount();
            if unbonded_total > MATERIAL_EPSILON {
                let outflow = unbonded_total * fraction;
                if outflow > MATERIAL_EPSILON { outgoing_unbonded[i] = self.cells[i].unbonded.take(outflow); }
            }
        }

        for i in 0..n {
            let neighbors = self.neighbor_indices(i);
            if let Some(mat) = outgoing_bonded[i].take() { distribute_evenly(self, mat, &neighbors); }
            if let Some(mat) = outgoing_unbonded[i].take() { distribute_evenly(self, mat, &neighbors); }
        }
    }

    pub fn total_material(&self) -> Vec<(String, f64)> {
        let mut totals: Vec<(String, f64)> = Vec::new();
        for cell in &self.cells {
            for (name, amount) in cell.bonded.parts.iter().chain(cell.unbonded.parts.iter()) {
                if let Some(existing) = totals.iter_mut().find(|(n, _)| n == name) { existing.1 += amount; }
                else { totals.push((name.clone(), *amount)); }
            }
        }
        totals
    }

    pub fn total_amount(&self) -> f64 {
        self.cells.iter().map(FieldCell::total_amount).sum()
    }
}

// ============================================================
// DEEP RESERVOIR
// ============================================================

pub const DEFAULT_RESERVOIR_BLOCK_SIZE: usize = 5;
pub const DEFAULT_SETTLING_FRACTION: f64 = 0.01;
pub const DEFAULT_SETTLING_INTERVAL_TICKS: u64 = 10;

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

    /// Draws from the combined bonded + unbonded stock of one resource
    /// without preferring either state. The draw is proportional to the
    /// amount available in each state, so the vent has no bonded/unbonded
    /// preference and the bonded state is preserved during transfer.
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

// ============================================================
// VENTS
// ============================================================
//
// A vent transfers existing material from its local reservoir region
// into its single home cell in the active field. It does not create
// material and it does not spread material itself - spreading is the
// diffusion system's job.
//
// IMPORTANT: a vent is deliberately INDISCRIMINATE with respect to
// bonded/unbonded state. It does not prefer bonded stock, does not
// use unbonded stock only as a fallback, and does not decide that
// emitted material should be bonded. It draws the requested resource
// amount from the combined local reservoir stock proportionally to
// what is available in each state, preserving each unit's existing
// bonded/unbonded state during transfer.
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
            let requested = vent.emission_amount * proportion;
            let (from_bonded, from_unbonded) = reservoir.cells[reservoir_index]
                .take_indiscriminate(name, requested);

            if from_bonded > MATERIAL_EPSILON { bonded_parts.push((name.clone(), from_bonded)); }
            if from_unbonded > MATERIAL_EPSILON { unbonded_parts.push((name.clone(), from_unbonded)); }
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
// SETTLING (active field -> reservoir return)
// ============================================================

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

fn distribute_evenly(field: &mut ActiveMaterialField, mut mat: Material, neighbors: &[usize]) {
    if neighbors.is_empty() { return; }
    let share = mat.total_amount() / neighbors.len() as f64;
    for (k, &neighbor_index) in neighbors.iter().enumerate() {
        let is_last = k == neighbors.len() - 1;
        let piece = if is_last {
            Material { parts: std::mem::take(&mut mat.parts), bonded: mat.bonded }
        } else {
            match mat.take(share) { Some(piece) => piece, None => continue }
        };
        if !piece.parts.is_empty() { field.deposit_at_index(neighbor_index, piece); }
    }
}

// ============================================================
// TESTS
// ============================================================

#[cfg(test)]
mod field_tests {
    use super::*;

    fn make_bonded(name: &str, amount: f64) -> Material {
        Material { parts: vec![(name.to_string(), amount)], bonded: true }
    }

    fn make_unbonded(name: &str, amount: f64) -> Material {
        Material { parts: vec![(name.to_string(), amount)], bonded: false }
    }

    #[test]
    fn field_has_expected_dimensions() {
        let field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
        assert_eq!(field.width_cells, 40);
        assert_eq!(field.height_cells, 40);
        assert_eq!(field.cells.len(), 1600);
    }

    #[test]
    fn field_starts_empty() {
        let field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
        assert_eq!(field.total_amount(), 0.0);
        assert!(field.total_material().is_empty());
    }

    #[test]
    fn out_of_bounds_position_is_none() {
        let field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
        assert!(field.index_for_position(-1.0, 5.0).is_none());
        assert!(field.index_for_position(5.0, 1000.0).is_none());
        assert!(field.index_for_position(1000.0, 5.0).is_none());
        assert!(field.index_for_position(f64::NAN, 5.0).is_none());
    }

    #[test]
    fn deposit_and_query_bonded_and_unbonded_independently() {
        let mut field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
        field.deposit(500.0, 500.0, make_bonded("Methane", 10.0));
        field.deposit(500.0, 500.0, make_unbonded("Carbon", 3.0));
        let index = field.index_for_position(500.0, 500.0).unwrap();
        let cell = &field.cells[index];
        assert!((cell.bonded.total_amount() - 10.0).abs() < 1e-9);
        assert!((cell.unbonded.total_amount() - 3.0).abs() < 1e-9);
    }

    #[test]
    fn deposit_merges_same_resource_into_existing_stack() {
        let mut field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
        field.deposit(10.0, 10.0, make_bonded("Methane", 5.0));
        field.deposit(10.0, 10.0, make_bonded("Methane", 7.0));
        field.deposit(10.0, 10.0, make_bonded("Hydrogen", 2.0));
        let index = field.index_for_position(10.0, 10.0).unwrap();
        let cell = &field.cells[index];
        assert_eq!(cell.bonded.parts.len(), 2);
        assert!((cell.bonded.total_amount() - 14.0).abs() < 1e-9);
    }

    #[test]
    fn take_removes_up_to_available_amount_and_no_more() {
        let mut field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
        field.deposit(50.0, 50.0, make_bonded("Carbon", 4.0));
        let taken = field.take_at(50.0, 50.0, true, 100.0).unwrap();
        assert!((taken.total_amount() - 4.0).abs() < 1e-9);
        let index = field.index_for_position(50.0, 50.0).unwrap();
        assert!(field.cells[index].bonded.total_amount() < 1e-9);
    }

    #[test]
    fn take_from_wrong_stack_does_not_touch_the_other() {
        let mut field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
        field.deposit(50.0, 50.0, make_bonded("Carbon", 4.0));
        field.deposit(50.0, 50.0, make_unbonded("Carbon", 9.0));
        let taken = field.take_at(50.0, 50.0, true, 4.0).unwrap();
        assert!((taken.total_amount() - 4.0).abs() < 1e-9);
        let index = field.index_for_position(50.0, 50.0).unwrap();
        assert!((field.cells[index].unbonded.total_amount() - 9.0).abs() < 1e-9);
    }

    #[test]
    fn diffusion_zero_fraction_is_a_noop() {
        let mut field = ActiveMaterialField::new(200.0, 200.0, 25.0);
        field.deposit(100.0, 100.0, make_bonded("Methane", 10.0));
        field.diffuse_step(0.0);
        let index = field.index_for_position(100.0, 100.0).unwrap();
        assert!((field.cells[index].bonded.total_amount() - 10.0).abs() < 1e-9);
    }

    #[test]
    fn diffusion_spreads_material_to_all_four_neighbors_from_interior_cell() {
        let mut field = ActiveMaterialField::new(200.0, 200.0, 25.0);
        field.deposit(100.0, 100.0, make_bonded("Methane", 100.0));
        let center = field.index_for_position(100.0, 100.0).unwrap();
        assert_eq!(field.neighbor_indices(center).len(), 4);
        field.diffuse_step(0.2);
        for &n in &field.neighbor_indices(center) {
            assert!(field.cells[n].bonded.total_amount() > 0.0);
        }
    }

    #[test]
    fn diffusion_conserves_total_mass_over_many_steps() {
        let mut field = ActiveMaterialField::new(500.0, 500.0, 25.0);
        field.deposit(250.0, 250.0, make_bonded("Methane", 500.0));
        field.deposit(50.0, 450.0, make_unbonded("Carbon", 300.0));
        field.deposit(0.0, 0.0, make_bonded("Hydrogen", 50.0));
        let before = field.total_amount();
        for _ in 0..200 { field.diffuse_step(DEFAULT_DIFFUSION_FRACTION); }
        let after = field.total_amount();
        assert!((before - after).abs() < 1e-6);
    }

    #[test]
    fn diffusion_conserves_mass_per_resource_type_not_just_total() {
        let mut field = ActiveMaterialField::new(300.0, 300.0, 25.0);
        field.deposit(150.0, 150.0, make_bonded("Methane", 200.0));
        field.deposit(150.0, 150.0, make_bonded("Carbon", 80.0));
        for _ in 0..50 { field.diffuse_step(0.1); }
        let totals = field.total_material();
        let methane = totals.iter().find(|(n, _)| n == "Methane").unwrap().1;
        let carbon = totals.iter().find(|(n, _)| n == "Carbon").unwrap().1;
        assert!((methane - 200.0).abs() < 1e-6);
        assert!((carbon - 80.0).abs() < 1e-6);
    }

    #[test]
    fn corner_cell_diffusion_conserves_mass_with_only_two_neighbors() {
        let mut field = ActiveMaterialField::new(200.0, 200.0, 25.0);
        field.deposit(0.0, 0.0, make_bonded("Sulfur", 40.0));
        let corner = field.index_for_position(0.0, 0.0).unwrap();
        assert_eq!(field.neighbor_indices(corner).len(), 2);
        let before = field.total_amount();
        for _ in 0..30 { field.diffuse_step(0.15); }
        let after = field.total_amount();
        assert!((before - after).abs() < 1e-6);
    }

    #[test]
    fn repeated_diffusion_eventually_spreads_material_across_the_field() {
        let mut field = ActiveMaterialField::new(200.0, 200.0, 25.0);
        field.deposit(0.0, 0.0, make_bonded("Water", 640.0));
        for _ in 0..500 { field.diffuse_step(0.1); }
        let nonzero_cells = field.cells.iter().filter(|c| c.bonded.total_amount() > 1e-6).count();
        assert!(nonzero_cells > 1);
    }
}

#[cfg(test)]
mod reservoir_and_vent_tests {
    use super::*;

    fn field_and_reservoir() -> (ActiveMaterialField, DeepReservoir) {
        let field = ActiveMaterialField::new(1000.0, 1000.0, DEFAULT_CELL_SIZE);
        let reservoir = DeepReservoir::new_matching_field(&field, DEFAULT_RESERVOIR_BLOCK_SIZE);
        (field, reservoir)
    }

    #[test]
    fn reservoir_grid_is_coarser_than_field_and_spatially_aligned() {
        let (field, reservoir) = field_and_reservoir();
        assert_eq!(reservoir.width_cells, 8);
        assert_eq!(reservoir.height_cells, 8);
        assert!(reservoir.cells.len() < field.cells.len());
    }

    #[test]
    fn seeding_distributes_total_evenly_and_conserves_it() {
        let (_, mut reservoir) = field_and_reservoir();
        reservoir.seed_uniform("Carbon", 6400.0);
        assert!((reservoir.total_amount() - 6400.0).abs() < 1e-6);
        assert!((reservoir.cells[0].amount_of(false, "Carbon") - 100.0).abs() < 1e-9);
        assert_eq!(reservoir.cells[0].amount_of(true, "Carbon"), 0.0);
    }

    #[test]
    fn vent_draws_only_from_its_own_region_not_a_global_pool() {
        let (mut field, mut reservoir) = field_and_reservoir();
        let region_a_amount = 50.0;
        let region_b_amount = 999.0;
        let idx_a = reservoir.reservoir_index_for_field_index(&field, field.index_for_position(10.0, 10.0).unwrap());
        let idx_b = reservoir.reservoir_index_for_field_index(&field, field.index_for_position(900.0, 900.0).unwrap());
        reservoir.cells[idx_a].add(false, "Methane", region_a_amount);
        reservoir.cells[idx_b].add(false, "Methane", region_b_amount);

        let mut vents = vec![
            Vent { x: 10.0, y: 10.0, composition: vec![("Methane".into(), 1.0)], emission_amount: 200.0, emission_interval: 0, emission_timer: 0 },
            Vent { x: 900.0, y: 900.0, composition: vec![("Methane".into(), 1.0)], emission_amount: 10.0, emission_interval: 0, emission_timer: 0 },
        ];
        apply_vents(&mut field, &mut reservoir, &mut vents);
        assert!(reservoir.cells[idx_a].amount_of(false, "Methane") < 1e-9);
        let field_idx_a = field.index_for_position(10.0, 10.0).unwrap();
        assert!((field.cells[field_idx_a].unbonded.total_amount() - region_a_amount).abs() < 1e-9);
        assert_eq!(field.cells[field_idx_a].bonded.total_amount(), 0.0);
        assert!((reservoir.cells[idx_b].amount_of(false, "Methane") - (region_b_amount - 10.0)).abs() < 1e-9);
    }

    #[test]
    fn vent_draw_is_indiscriminate_across_bonded_and_unbonded_stock() {
        let (mut field, mut reservoir) = field_and_reservoir();
        let field_index = field.index_for_position(500.0, 500.0).unwrap();
        let reservoir_index = reservoir.reservoir_index_for_field_index(&field, field_index);
        reservoir.cells[reservoir_index].add(true, "Carbon", 20.0);
        reservoir.cells[reservoir_index].add(false, "Carbon", 80.0);

        let mut vents = vec![Vent {
            x: 500.0,
            y: 500.0,
            composition: vec![("Carbon".into(), 1.0)],
            emission_amount: 50.0,
            emission_interval: 0,
            emission_timer: 0,
        }];
        apply_vents(&mut field, &mut reservoir, &mut vents);

        // 20% of the available Carbon was bonded and 80% unbonded;
        // the 50-unit draw therefore transfers 10 bonded + 40 unbonded.
        assert!((field.cells[field_index].bonded.total_amount() - 10.0).abs() < 1e-9);
        assert!((field.cells[field_index].unbonded.total_amount() - 40.0).abs() < 1e-9);
        assert!((reservoir.cells[reservoir_index].amount_of(true, "Carbon") - 10.0).abs() < 1e-9);
        assert!((reservoir.cells[reservoir_index].amount_of(false, "Carbon") - 40.0).abs() < 1e-9);
    }

    #[test]
    fn vent_does_not_convert_unbonded_material_into_bonded_material() {
        let (mut field, mut reservoir) = field_and_reservoir();
        let field_index = field.index_for_position(500.0, 500.0).unwrap();
        let reservoir_index = reservoir.reservoir_index_for_field_index(&field, field_index);
        reservoir.cells[reservoir_index].add(false, "Carbon", 100.0);

        let mut vents = vec![Vent {
            x: 500.0,
            y: 500.0,
            composition: vec![("Carbon".into(), 1.0)],
            emission_amount: 30.0,
            emission_interval: 0,
            emission_timer: 0,
        }];
        apply_vents(&mut field, &mut reservoir, &mut vents);

        assert!((field.cells[field_index].unbonded.total_amount() - 30.0).abs() < 1e-9);
        assert!(field.cells[field_index].bonded.total_amount() < 1e-9);
        assert!((reservoir.cells[reservoir_index].amount_of(false, "Carbon") - 70.0).abs() < 1e-9);
    }

    #[test]
    fn vent_preserves_bonded_material_as_bonded_when_it_is_the_available_stock() {
        let (mut field, mut reservoir) = field_and_reservoir();
        let field_index = field.index_for_position(500.0, 500.0).unwrap();
        let reservoir_index = reservoir.reservoir_index_for_field_index(&field, field_index);
        reservoir.cells[reservoir_index].add(true, "Methane", 50.0);

        let mut vents = vec![Vent {
            x: 500.0,
            y: 500.0,
            composition: vec![("Methane".into(), 1.0)],
            emission_amount: 20.0,
            emission_interval: 0,
            emission_timer: 0,
        }];
        apply_vents(&mut field, &mut reservoir, &mut vents);

        assert!((field.cells[field_index].bonded.total_amount() - 20.0).abs() < 1e-9);
        assert!(field.cells[field_index].unbonded.total_amount() < 1e-9);
        assert!((reservoir.cells[reservoir_index].amount_of(true, "Methane") - 30.0).abs() < 1e-9);
    }

    #[test]
    fn venting_conserves_total_material_reservoir_plus_field() {
        let (mut field, mut reservoir) = field_and_reservoir();
        reservoir.seed_uniform("Carbon", 5000.0);
        let mut vents = vec![Vent {
            x: 250.0, y: 250.0,
            composition: vec![("Carbon".into(), 1.0)],
            emission_amount: 30.0, emission_interval: 2, emission_timer: 0,
        }];
        let before = reservoir.total_amount() + field.total_amount();
        for _ in 0..50 { apply_vents(&mut field, &mut reservoir, &mut vents); }
        let after = reservoir.total_amount() + field.total_amount();
        assert!((before - after).abs() < 1e-6);
    }

    #[test]
    fn settling_drains_both_bonded_and_unbonded_stacks() {
        let (mut field, mut reservoir) = field_and_reservoir();
        let idx = field.index_for_position(500.0, 500.0).unwrap();
        field.deposit_at_index(idx, Material { parts: vec![("Carbon".into(), 100.0)], bonded: true });
        field.deposit_at_index(idx, Material { parts: vec![("Carbon".into(), 40.0)], bonded: false });
        for _ in 0..20 { apply_settling(&mut field, &mut reservoir, DEFAULT_SETTLING_FRACTION); }
        assert!(field.cells[idx].bonded.total_amount() < 100.0);
        assert!(field.cells[idx].unbonded.total_amount() < 40.0);
        assert!(reservoir.total_amount() > 0.0);
    }

    #[test]
    fn settling_preserves_bonded_status_in_the_reservoir() {
        let (mut field, mut reservoir) = field_and_reservoir();
        let field_index = field.index_for_position(500.0, 500.0).unwrap();
        let reservoir_index = reservoir.reservoir_index_for_field_index(&field, field_index);
        field.deposit_at_index(field_index, Material { parts: vec![("Sulfur".into(), 200.0)], bonded: true });
        for _ in 0..50 { apply_settling(&mut field, &mut reservoir, DEFAULT_SETTLING_FRACTION); }
        assert!(reservoir.cells[reservoir_index].amount_of(true, "Sulfur") > 0.0);
        assert_eq!(reservoir.cells[reservoir_index].amount_of(false, "Sulfur"), 0.0);
    }

    #[test]
    fn settled_bonded_material_can_be_re_released_by_a_vent_still_bonded() {
        let (mut field, mut reservoir) = field_and_reservoir();
        let field_index = field.index_for_position(500.0, 500.0).unwrap();
        let reservoir_index = reservoir.reservoir_index_for_field_index(&field, field_index);
        field.deposit_at_index(field_index, Material { parts: vec![("Nitrogen".into(), 500.0)], bonded: true });
        for _ in 0..500 { apply_settling(&mut field, &mut reservoir, 0.05); }
        assert!(field.cells[field_index].bonded.total_amount() < 1.0);
        let bonded_in_reservoir = reservoir.cells[reservoir_index].amount_of(true, "Nitrogen");
        assert!(bonded_in_reservoir > 400.0);

        let mut vents = vec![Vent {
            x: 500.0, y: 500.0,
            composition: vec![("Nitrogen".into(), 1.0)],
            emission_amount: 50.0, emission_interval: 0, emission_timer: 0,
        }];
        apply_vents(&mut field, &mut reservoir, &mut vents);
        assert!((field.cells[field_index].bonded.total_amount() - 50.0).abs() < 1e-6);
        assert_eq!(field.cells[field_index].unbonded.total_amount(), 0.0);
        assert!((reservoir.cells[reservoir_index].amount_of(true, "Nitrogen") - (bonded_in_reservoir - 50.0)).abs() < 1e-6);
    }

    #[test]
    fn settling_conserves_total_material_field_plus_reservoir() {
        let (mut field, mut reservoir) = field_and_reservoir();
        let idx = field.index_for_position(500.0, 500.0).unwrap();
        field.deposit_at_index(idx, Material { parts: vec![("Water".into(), 300.0)], bonded: false });
        field.deposit_at_index(idx, Material { parts: vec![("Hydrogen".into(), 150.0)], bonded: true });
        let before = field.total_amount() + reservoir.total_amount();
        for _ in 0..100 { apply_settling(&mut field, &mut reservoir, DEFAULT_SETTLING_FRACTION); }
        let after = field.total_amount() + reservoir.total_amount();
        assert!((before - after).abs() < 1e-6);
    }

    #[test]
    fn full_environment_loop_conserves_material_over_many_ticks() {
        let (mut field, mut reservoir) = field_and_reservoir();
        reservoir.seed_uniform("Carbon", 20_000.0);
        reservoir.seed_uniform("Methane", 10_000.0);
        reservoir.seed_uniform("Water", 15_000.0);
        let mut vents = vec![
            Vent { x: 250.0, y: 250.0, composition: vec![("Carbon".into(), 0.5), ("Methane".into(), 0.5)], emission_amount: 40.0, emission_interval: 5, emission_timer: 0 },
            Vent { x: 750.0, y: 750.0, composition: vec![("Water".into(), 1.0)], emission_amount: 20.0, emission_interval: 8, emission_timer: 0 },
        ];
        let before = field.total_amount() + reservoir.total_amount();
        for tick in 0..2000u64 {
            apply_vents(&mut field, &mut reservoir, &mut vents);
            field.diffuse_step(DEFAULT_DIFFUSION_FRACTION);
            if tick % DEFAULT_SETTLING_INTERVAL_TICKS == 0 { apply_settling(&mut field, &mut reservoir, DEFAULT_SETTLING_FRACTION); }
        }
        let after = field.total_amount() + reservoir.total_amount();
        assert!((before - after).abs() < 1e-4);
    }
}
