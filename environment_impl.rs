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
// nothing about the *architecture* depends on these particular numbers.
// ------------------------------------------------------------

/// Default cell size in world units. A 1000x1000 world produces a
/// 40x40 field at this resolution.
pub const DEFAULT_CELL_SIZE: f64 = 25.0;

/// Default fraction of a cell's material that moves out to its
/// neighbors per diffusion step. Tunable; not a locked value.
pub const DEFAULT_DIFFUSION_FRACTION: f64 = 0.05;

/// Floating-point noise floor. Amounts at or below this are treated
/// as zero for the purposes of triggering transfers - this exists
/// only to avoid endlessly shuffling insignificant residue around,
/// per the "epsilon eliminates float residue, not an artificial
/// disappearance mechanism" decision. It does not delete material;
/// it just skips no-op transfers.
pub const MATERIAL_EPSILON: f64 = 1e-9;

// ------------------------------------------------------------
// FIELD CELL
// ------------------------------------------------------------

/// One grid cell's contents: exactly one bonded stack and one
/// unbonded stack. Both are always present (possibly empty) so cell
// indexing never has to special-case a missing stack.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FieldCell {
    pub bonded: Material,
    pub unbonded: Material,
}

impl FieldCell {
    pub fn empty() -> Self {
        Self {
            bonded: Material {
                parts: Vec::new(),
                bonded: true,
            },
            unbonded: Material {
                parts: Vec::new(),
                bonded: false,
            },
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

    /// Row-major: index = row * width_cells + col.
    pub cells: Vec<FieldCell>,
}

impl ActiveMaterialField {
    /// Builds a field covering `world_width` x `world_height` world
    /// units at the given cell size. Any partial trailing cell is
    /// rounded up so the whole world is covered.
    pub fn new(world_width: f64, world_height: f64, cell_size: f64) -> Self {
        let cell_size = cell_size.max(1.0);

        let width_cells = (world_width / cell_size).ceil().max(1.0) as usize;
        let height_cells = (world_height / cell_size).ceil().max(1.0) as usize;

        let cells = (0..width_cells * height_cells)
            .map(|_| FieldCell::empty())
            .collect();

        Self {
            cell_size,
            width_cells,
            height_cells,
            cells,
        }
    }

    // --------------------------------------------------------
    // COORDINATE MAPPING
    // --------------------------------------------------------

    pub fn row_col_for_position(&self, x: f64, y: f64) -> Option<(usize, usize)> {
        if !x.is_finite() || !y.is_finite() || x < 0.0 || y < 0.0 {
            return None;
        }

        let col = (x / self.cell_size).floor() as usize;
        let row = (y / self.cell_size).floor() as usize;

        if col >= self.width_cells || row >= self.height_cells {
            return None;
        }

        Some((row, col))
    }

    pub fn index_for_position(&self, x: f64, y: f64) -> Option<usize> {
        self.row_col_for_position(x, y)
            .map(|(row, col)| row * self.width_cells + col)
    }

    fn row_col_for_index(&self, index: usize) -> (usize, usize) {
        (index / self.width_cells, index % self.width_cells)
    }

    /// World-space center of a cell, e.g. for placing vents or
    /// reporting positions to the frontend.
    pub fn cell_center(&self, index: usize) -> (f64, f64) {
        let (row, col) = self.row_col_for_index(index);
        (
            (col as f64 + 0.5) * self.cell_size,
            (row as f64 + 0.5) * self.cell_size,
        )
    }

    /// Indices of the up-to-4 orthogonal neighbors of a cell. Edge
    /// and corner cells simply have fewer neighbors - diffusion below
    /// accounts for this rather than wrapping or padding.
    pub fn neighbor_indices(&self, index: usize) -> Vec<usize> {
        let (row, col) = self.row_col_for_index(index);
        let mut out = Vec::with_capacity(4);

        if row > 0 {
            out.push((row - 1) * self.width_cells + col);
        }
        if row + 1 < self.height_cells {
            out.push((row + 1) * self.width_cells + col);
        }
        if col > 0 {
            out.push(row * self.width_cells + (col - 1));
        }
        if col + 1 < self.width_cells {
            out.push(row * self.width_cells + (col + 1));
        }

        out
    }

    // --------------------------------------------------------
    // DEPOSIT
    // --------------------------------------------------------

    /// Merges `material` into the cell at world position (x, y).
    /// Routes to the bonded or unbonded stack based on
    /// `material.bonded`. Returns false (and deposits nothing) if the
    /// position is outside the field.
    pub fn deposit(&mut self, x: f64, y: f64, material: Material) -> bool {
        match self.index_for_position(x, y) {
            Some(index) => {
                self.deposit_at_index(index, material);
                true
            }
            None => false,
        }
    }

    pub fn deposit_at_index(&mut self, index: usize, material: Material) {
        if material.parts.is_empty() {
            return;
        }

        let cell = &mut self.cells[index];
        let target = if material.bonded {
            &mut cell.bonded
        } else {
            &mut cell.unbonded
        };

        let mut parts = std::mem::take(&mut target.parts);
        parts.extend(material.parts);
        target.parts = merge_parts(&parts);
    }

    // --------------------------------------------------------
    // TAKE
    // --------------------------------------------------------

    /// Removes up to `amount` from the bonded or unbonded stack at
    /// world position (x, y), proportionally across that stack's
    /// composition (via Material::take). Returns None if the
    /// position is out of bounds or there is nothing to take.
    pub fn take_at(&mut self, x: f64, y: f64, bonded: bool, amount: f64) -> Option<Material> {
        let index = self.index_for_position(x, y)?;
        self.take_at_index(index, bonded, amount)
    }

    pub fn take_at_index(&mut self, index: usize, bonded: bool, amount: f64) -> Option<Material> {
        let cell = &mut self.cells[index];
        let stack = if bonded {
            &mut cell.bonded
        } else {
            &mut cell.unbonded
        };
        stack.take(amount)
    }

    // --------------------------------------------------------
    // DIFFUSION
    // --------------------------------------------------------
    //
    // Each cell pushes `fraction` of its bonded stack and `fraction`
    // of its unbonded stack out to its existing neighbors, split
    // evenly. This is done in two passes (collect outgoing material
    // from a snapshot, then distribute it) so that material leaving
    // one cell in this step cannot be re-diffused again within the
    // same step - each cell's outflow is computed exactly once,
    // making conservation trivial to reason about: everything that
    // leaves a cell is deposited into exactly one neighbor.
    //
    // Edge/corner cells have fewer neighbors, so they simply retain
    // more (their outflow is still `fraction` of their own material,
    // just split across fewer destinations) - this is the natural,
    // non-wrapping consequence of a bounded grid and is intentional.
    // --------------------------------------------------------

    pub fn diffuse_step(&mut self, fraction: f64) {
        let fraction = fraction.clamp(0.0, 1.0);
        if fraction <= 0.0 {
            return;
        }

        let n = self.cells.len();

        // Pass 1: pull outgoing material out of every cell into a
        // per-cell holding buffer, based on a fixed snapshot of who
        // each cell's neighbors are (topology never changes, so no
        // separate snapshot of amounts is needed - taking directly
        // from self.cells is fine since we never read a cell's
        // amount after taking from it below).
        let mut outgoing_bonded: Vec<Option<Material>> = vec![None; n];
        let mut outgoing_unbonded: Vec<Option<Material>> = vec![None; n];

        for i in 0..n {
            let neighbor_count = self.neighbor_indices(i).len();
            if neighbor_count == 0 {
                continue;
            }

            let bonded_total = self.cells[i].bonded.total_amount();
            if bonded_total > MATERIAL_EPSILON {
                let outflow = bonded_total * fraction;
                if outflow > MATERIAL_EPSILON {
                    outgoing_bonded[i] = self.cells[i].bonded.take(outflow);
                }
            }

            let unbonded_total = self.cells[i].unbonded.total_amount();
            if unbonded_total > MATERIAL_EPSILON {
                let outflow = unbonded_total * fraction;
                if outflow > MATERIAL_EPSILON {
                    outgoing_unbonded[i] = self.cells[i].unbonded.take(outflow);
                }
            }
        }

        // Pass 2: distribute each cell's outgoing material evenly
        // across its neighbors. The last neighbor gets whatever
        // remains (rather than an equal float share) so rounding
        // residue can't leak mass instead of just landing slightly
        // unevenly.
        for i in 0..n {
            let neighbors = self.neighbor_indices(i);

            if let Some(mat) = outgoing_bonded[i].take() {
                distribute_evenly(self, mat, &neighbors);
            }

            if let Some(mat) = outgoing_unbonded[i].take() {
                distribute_evenly(self, mat, &neighbors);
            }
        }
    }

    // --------------------------------------------------------
    // DIAGNOSTICS
    // --------------------------------------------------------

    /// Total amount of every resource type currently held in the
    /// field (bonded + unbonded, all cells). Used for conservation
    /// checks, not called every tick.
    pub fn total_material(&self) -> Vec<(String, f64)> {
        let mut totals: Vec<(String, f64)> = Vec::new();
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
}

// ============================================================
// DEEP RESERVOIR
// ============================================================
//
// The reservoir is deliberately much coarser than the active field
// (per review decision: "preserve spatial correspondence... rather
// than treating the reservoir as one global pool"). It is a second,
// lower-resolution grid where each reservoir cell aggregates a
// square block of active-field cells. A vent draws only from the
// reservoir cell(s) under its own region, and settling returns
// material only to the reservoir cell under the field cell it came
// from - so different regions of the world can genuinely run low on
// a resource independently of each other, rather than all sharing
// one abstract global pool.
//
// A reservoir cell holds two sets of named amounts, mirroring the
// field's bonded/unbonded split. This is what makes it safe for
// bonded material to move between the field and the reservoir in
// either direction: bonded material settling out of the field stays
// recorded as bonded in the reservoir, and a vent releasing it later
// hands back genuinely-already-bonded material - no debonding or
// rebonding is implied by the trip, so this does not conflict with
// the locked rule that only BREAK changes bonded status (§15/§52).
//
// It only becomes "real" active material - something an organism can
// perceive and process - once a vent moves it into the field.
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

    /// Removes up to `amount` of `name` from the bonded or unbonded
    /// side, capped by what's actually present. Returns the amount
    /// actually removed - callers must use this return value rather
    /// than assuming the full request was satisfied.
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

    pub fn total_amount(&self) -> f64 {
        let bonded: f64 = self.bonded_entries.iter().map(|(_, a)| a).sum();
        let unbonded: f64 = self.unbonded_entries.iter().map(|(_, a)| a).sum();
        bonded + unbonded
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DeepReservoir {
    /// Side length, in active-field cells, of the square block each
    /// reservoir cell aggregates.
    pub block_size: usize,
    pub width_cells: usize,
    pub height_cells: usize,
    pub cells: Vec<ReservoirCell>,
}

impl DeepReservoir {
    /// Builds a reservoir grid sized to match `field` at the given
    /// block size (how many field cells, per side, aggregate into
    /// one reservoir cell).
    pub fn new_matching_field(field: &ActiveMaterialField, block_size: usize) -> Self {
        let block_size = block_size.max(1);

        let width_cells = ((field.width_cells as f64) / (block_size as f64))
            .ceil()
            .max(1.0) as usize;
        let height_cells = ((field.height_cells as f64) / (block_size as f64))
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

    /// Maps an active-field cell index to the reservoir cell that
    /// spatially corresponds to it.
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

    /// Distributes `total_amount` of `name` evenly across every
    /// reservoir cell's UNBONDED stock. Used for initial world
    /// seeding - deep reservoir stock starts as raw material, since
    /// nothing has bonded it yet. (Bonded reservoir stock only ever
    /// arises later, from settled bonded field material.)
    pub fn seed_uniform(&mut self, name: &str, total_amount: f64) {
        if self.cells.is_empty() || total_amount <= 0.0 {
            return;
        }
        let per_cell = total_amount / self.cells.len() as f64;
        for cell in &mut self.cells {
            cell.add(false, name, per_cell);
        }
    }

    pub fn total_material(&self) -> Vec<(String, f64)> {
        let mut totals: Vec<(String, f64)> = Vec::new();
        for cell in &self.cells {
            for (name, amount) in cell.bonded_entries.iter().chain(cell.unbonded_entries.iter()) {
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

// ============================================================
// VENTS
// ============================================================
//
// A vent transfers existing material from its local reservoir
// region into its single home cell in the active field. It does not
// create material (draws are capped by what the reservoir cell
// actually has) and it does not spread material itself - per review
// decision, spreading is diffusion's job, not the vent's.
//
// A vent always emits bonded material into the field (organisms need
// bonded material to BREAK). It draws preferentially from the
// reservoir's already-bonded stock for a region - a genuine transfer
// of real bonded material, no new bonding implied - and only falls
// back to marking raw stock as bonded on the way out when the region
// doesn't have enough bonded stock yet. That fallback is the original
// §16 bootstrap permission ("some vents MAY emit pre-bonded material
// when necessary"), which remains essential early on, before any
// bonded material has had a chance to accumulate via settling.
// ============================================================

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Vent {
    pub x: f64,
    pub y: f64,

    /// Proportions of emission_amount drawn per resource type. Not
    /// required to sum to exactly 1.0, mirroring the old model.
    pub composition: Vec<(String, f64)>,

    pub emission_amount: f64,
    pub emission_interval: u64,
    pub emission_timer: u64,
}

/// Advances every vent's timer and, for any vent whose timer has
/// elapsed, draws material from its local reservoir region and
/// deposits it into its home active-field cell as bonded material.
///
/// A vent positioned outside the field or whose region has nothing
/// left to draw simply emits nothing that tick - this is not an
/// error, just an empty draw.
pub fn apply_vents(field: &mut ActiveMaterialField, reservoir: &mut DeepReservoir, vents: &mut [Vent]) {
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

            // Prefer real bonded stock first...
            let from_bonded = reservoir.cells[reservoir_index].take(true, name, requested);

            // ...then fall back to raw stock (marked bonded on the
            // way out) only for whatever the bonded stock couldn't
            // cover. This is the §16 bootstrap fallback, not the
            // normal case once bonded stock exists.
            let remaining = requested - from_bonded;
            let from_unbonded = if remaining > MATERIAL_EPSILON {
                reservoir.cells[reservoir_index].take(false, name, remaining)
            } else {
                0.0
            };

            let drawn = from_bonded + from_unbonded;
            if drawn > 0.0 {
                parts.push((name.clone(), drawn));
            }
        }

        if !parts.is_empty() {
            field.deposit_at_index(
                field_index,
                Material {
                    parts,
                    bonded: true,
                },
            );
        }
    }
}

// ============================================================
// SETTLING (active field -> reservoir return)
// ============================================================
//
// Per review decision: a single global, tunable return rate for now
// (no per-material settling property yet). Settling drains BOTH the
// bonded and unbonded stacks of each field cell, at the same rate,
// into the matching stack (bonded -> bonded, unbonded -> unbonded)
// of the corresponding reservoir cell.
//
// Because the reservoir now tracks bonded and unbonded stock
// separately (see ReservoirCell), this transfer never changes a
// material's bonded status - it's a pure relocation, not a
// bonding/debonding event, so it doesn't conflict with the locked
// rule that only BREAK changes bonded status (§15) or the "no
// automatic raw resource restoration" rule (§52). Bonded material
// that settles stays bonded, and can later be released from a vent
// as genuinely-already-bonded material (see apply_vents).
// ============================================================

pub fn apply_settling(field: &mut ActiveMaterialField, reservoir: &mut DeepReservoir, fraction: f64) {
    let fraction = fraction.clamp(0.0, 1.0);
    if fraction <= 0.0 {
        return;
    }

    for field_index in 0..field.cells.len() {
        let reservoir_index = reservoir.reservoir_index_for_field_index(field, field_index);

        let bonded_total = field.cells[field_index].bonded.total_amount();
        if bonded_total > MATERIAL_EPSILON {
            let outflow = bonded_total * fraction;
            if outflow > MATERIAL_EPSILON {
                if let Some(taken) = field.cells[field_index].bonded.take(outflow) {
                    for (name, amount) in taken.parts {
                        reservoir.cells[reservoir_index].add(true, &name, amount);
                    }
                }
            }
        }

        let unbonded_total = field.cells[field_index].unbonded.total_amount();
        if unbonded_total > MATERIAL_EPSILON {
            let outflow = unbonded_total * fraction;
            if outflow > MATERIAL_EPSILON {
                if let Some(taken) = field.cells[field_index].unbonded.take(outflow) {
                    for (name, amount) in taken.parts {
                        reservoir.cells[reservoir_index].add(false, &name, amount);
                    }
                }
            }
        }
    }
}

/// Splits `mat`'s full amount evenly across `neighbors` and deposits
/// each piece directly into `field`. The last neighbor receives
/// whatever remains rather than a computed equal share, so this
/// exactly conserves `mat`'s total amount regardless of
/// floating-point rounding in the intermediate divisions.
fn distribute_evenly(field: &mut ActiveMaterialField, mut mat: Material, neighbors: &[usize]) {
    if neighbors.is_empty() {
        return;
    }

    let share = mat.total_amount() / neighbors.len() as f64;

    for (k, &neighbor_index) in neighbors.iter().enumerate() {
        let is_last = k == neighbors.len() - 1;

        let piece = if is_last {
            Material {
                parts: std::mem::take(&mut mat.parts),
                bonded: mat.bonded,
            }
        } else {
            match mat.take(share) {
                Some(piece) => piece,
                None => continue,
            }
        };

        if !piece.parts.is_empty() {
            field.deposit_at_index(neighbor_index, piece);
        }
    }
}

// ============================================================
// TESTS
// ============================================================
//
// Scoped to this module only: field construction, deposit/take
// round-tripping, and diffusion conservation. No vents, reservoir,
// or organism integration yet - those get their own tests in later
// migration steps.
// ============================================================

#[cfg(test)]
mod field_tests {
    use super::*;

    fn make_bonded(name: &str, amount: f64) -> Material {
        Material {
            parts: vec![(name.to_string(), amount)],
            bonded: true,
        }
    }

    fn make_unbonded(name: &str, amount: f64) -> Material {
        Material {
            parts: vec![(name.to_string(), amount)],
            bonded: false,
        }
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

        assert_eq!(cell.bonded.parts.len(), 2, "same-name parts should merge, not duplicate");
        assert!((cell.bonded.total_amount() - 14.0).abs() < 1e-9);
    }

    #[test]
    fn take_removes_up_to_available_amount_and_no_more() {
        let mut field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
        field.deposit(50.0, 50.0, make_bonded("Carbon", 4.0));

        let taken = field.take_at(50.0, 50.0, true, 100.0).unwrap();
        assert!((taken.total_amount() - 4.0).abs() < 1e-9, "take should cap at what's present");

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
        let mut field = ActiveMaterialField::new(200.0, 200.0, 25.0); // 8x8 grid
        // Center-ish cell, away from edges, so it has exactly 4 neighbors.
        field.deposit(100.0, 100.0, make_bonded("Methane", 100.0));
        let center = field.index_for_position(100.0, 100.0).unwrap();
        assert_eq!(field.neighbor_indices(center).len(), 4);

        field.diffuse_step(0.2);

        for &n in &field.neighbor_indices(center) {
            assert!(
                field.cells[n].bonded.total_amount() > 0.0,
                "expected neighbor {n} to receive diffused material"
            );
        }
    }

    #[test]
    fn diffusion_conserves_total_mass_over_many_steps() {
        let mut field = ActiveMaterialField::new(500.0, 500.0, 25.0);

        field.deposit(250.0, 250.0, make_bonded("Methane", 500.0));
        field.deposit(50.0, 450.0, make_unbonded("Carbon", 300.0)); // near a corner
        field.deposit(0.0, 0.0, make_bonded("Hydrogen", 50.0)); // exact corner

        let before = field.total_amount();

        for _ in 0..200 {
            field.diffuse_step(DEFAULT_DIFFUSION_FRACTION);
        }

        let after = field.total_amount();

        assert!(
            (before - after).abs() < 1e-6,
            "diffusion must conserve mass exactly (within float error): before={before}, after={after}"
        );
    }

    #[test]
    fn diffusion_conserves_mass_per_resource_type_not_just_total() {
        let mut field = ActiveMaterialField::new(300.0, 300.0, 25.0);
        field.deposit(150.0, 150.0, make_bonded("Methane", 200.0));
        field.deposit(150.0, 150.0, make_bonded("Carbon", 80.0));

        for _ in 0..50 {
            field.diffuse_step(0.1);
        }

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
        assert_eq!(field.neighbor_indices(corner).len(), 2, "corner cell should have exactly 2 neighbors");

        let before = field.total_amount();
        for _ in 0..30 {
            field.diffuse_step(0.15);
        }
        let after = field.total_amount();

        assert!((before - after).abs() < 1e-6);
    }

    #[test]
    fn repeated_diffusion_eventually_spreads_material_across_the_field() {
        let mut field = ActiveMaterialField::new(200.0, 200.0, 25.0); // 8x8 = 64 cells
        field.deposit(0.0, 0.0, make_bonded("Water", 640.0));

        for _ in 0..500 {
            field.diffuse_step(0.1);
        }

        let nonzero_cells = field
            .cells
            .iter()
            .filter(|c| c.bonded.total_amount() > 1e-6)
            .count();

        assert!(
            nonzero_cells > 1,
            "expected material to have spread beyond the origin cell after many diffusion steps"
        );
    }
}

#[cfg(test)]
mod reservoir_and_vent_tests {
    use super::*;

    fn field_and_reservoir() -> (ActiveMaterialField, DeepReservoir) {
        let field = ActiveMaterialField::new(1000.0, 1000.0, DEFAULT_CELL_SIZE); // 40x40
        let reservoir = DeepReservoir::new_matching_field(&field, DEFAULT_RESERVOIR_BLOCK_SIZE); // 8x8
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
        reservoir.seed_uniform("Carbon", 6400.0); // 100 per cell across 64 cells
        assert!((reservoir.total_amount() - 6400.0).abs() < 1e-6);
        assert!((reservoir.cells[0].amount_of(false, "Carbon") - 100.0).abs() < 1e-9);
        assert_eq!(reservoir.cells[0].amount_of(true, "Carbon"), 0.0, "seeding is raw stock, not bonded");
    }

    #[test]
    fn vent_draws_from_its_own_region_not_a_global_pool() {
        let (mut field, mut reservoir) = field_and_reservoir();

        // Two vents in two different corners/regions of the world.
        let region_a_amount = 50.0;
        let region_b_amount = 999.0;

        // Manually seed distinct amounts per region so we can prove
        // spatial separation (seed_uniform would make them equal).
        let idx_a = reservoir.reservoir_index_for_field_index(
            &field,
            field.index_for_position(10.0, 10.0).unwrap(),
        );
        let idx_b = reservoir.reservoir_index_for_field_index(
            &field,
            field.index_for_position(900.0, 900.0).unwrap(),
        );
        reservoir.cells[idx_a].add(false, "Methane", region_a_amount);
        reservoir.cells[idx_b].add(false, "Methane", region_b_amount);

        let mut vents = vec![
            Vent {
                x: 10.0,
                y: 10.0,
                composition: vec![("Methane".into(), 1.0)],
                emission_amount: 200.0, // requests more than region A has
                emission_interval: 0,
                emission_timer: 0,
            },
            Vent {
                x: 900.0,
                y: 900.0,
                composition: vec![("Methane".into(), 1.0)],
                emission_amount: 10.0,
                emission_interval: 0,
                emission_timer: 0,
            },
        ];

        apply_vents(&mut field, &mut reservoir, &mut vents);

        // Region A only had 50 to give, even though it requested 200.
        assert!(reservoir.cells[idx_a].amount_of(false, "Methane") < 1e-9);
        let field_idx_a = field.index_for_position(10.0, 10.0).unwrap();
        assert!((field.cells[field_idx_a].bonded.total_amount() - region_a_amount).abs() < 1e-9);

        // Region B is essentially untouched by region A's vent.
        assert!((reservoir.cells[idx_b].amount_of(false, "Methane") - (region_b_amount - 10.0)).abs() < 1e-9);
    }

    #[test]
    fn vent_prefers_real_bonded_reservoir_stock_over_raw_fallback() {
        let (mut field, mut reservoir) = field_and_reservoir();

        let field_index = field.index_for_position(500.0, 500.0).unwrap();
        let reservoir_index = reservoir.reservoir_index_for_field_index(&field, field_index);

        // Region has both bonded and unbonded Carbon available.
        reservoir.cells[reservoir_index].add(true, "Carbon", 20.0);
        reservoir.cells[reservoir_index].add(false, "Carbon", 1000.0);

        let mut vents = vec![Vent {
            x: 500.0,
            y: 500.0,
            composition: vec![("Carbon".into(), 1.0)],
            emission_amount: 15.0, // less than the bonded stock alone
            emission_interval: 0,
            emission_timer: 0,
        }];

        apply_vents(&mut field, &mut reservoir, &mut vents);

        // Bonded stock covered the whole request; raw stock untouched.
        assert!((reservoir.cells[reservoir_index].amount_of(true, "Carbon") - 5.0).abs() < 1e-9);
        assert!((reservoir.cells[reservoir_index].amount_of(false, "Carbon") - 1000.0).abs() < 1e-9);
    }

    #[test]
    fn vent_falls_back_to_raw_stock_when_bonded_stock_is_insufficient() {
        let (mut field, mut reservoir) = field_and_reservoir();

        let field_index = field.index_for_position(500.0, 500.0).unwrap();
        let reservoir_index = reservoir.reservoir_index_for_field_index(&field, field_index);

        reservoir.cells[reservoir_index].add(true, "Carbon", 4.0); // not enough alone
        reservoir.cells[reservoir_index].add(false, "Carbon", 1000.0);

        let mut vents = vec![Vent {
            x: 500.0,
            y: 500.0,
            composition: vec![("Carbon".into(), 1.0)],
            emission_amount: 10.0,
            emission_interval: 0,
            emission_timer: 0,
        }];

        apply_vents(&mut field, &mut reservoir, &mut vents);

        // Bonded stock fully drained; the remaining 6 came from raw stock.
        assert_eq!(reservoir.cells[reservoir_index].amount_of(true, "Carbon"), 0.0);
        assert!((reservoir.cells[reservoir_index].amount_of(false, "Carbon") - 994.0).abs() < 1e-9);

        let bonded_deposited = field.cells[field_index].bonded.total_amount();
        assert!((bonded_deposited - 10.0).abs() < 1e-9);
    }

    #[test]
    fn vent_deposits_pre_bonded_material() {
        let (mut field, mut reservoir) = field_and_reservoir();
        reservoir.seed_uniform("Methane", 10_000.0);

        let mut vents = vec![Vent {
            x: 500.0,
            y: 500.0,
            composition: vec![("Methane".into(), 1.0)],
            emission_amount: 50.0,
            emission_interval: 0,
            emission_timer: 0,
        }];

        apply_vents(&mut field, &mut reservoir, &mut vents);

        let idx = field.index_for_position(500.0, 500.0).unwrap();
        assert!(field.cells[idx].bonded.total_amount() > 0.0, "vent output must be bonded (§16)");
        assert_eq!(field.cells[idx].unbonded.total_amount(), 0.0);
    }

    #[test]
    fn venting_conserves_total_material_reservoir_plus_field() {
        let (mut field, mut reservoir) = field_and_reservoir();
        reservoir.seed_uniform("Carbon", 5000.0);

        let mut vents = vec![Vent {
            x: 250.0,
            y: 250.0,
            composition: vec![("Carbon".into(), 1.0)],
            emission_amount: 30.0,
            emission_interval: 2,
            emission_timer: 0,
        }];

        let before = reservoir.total_amount() + field.total_amount();

        for _ in 0..50 {
            apply_vents(&mut field, &mut reservoir, &mut vents);
        }

        let after = reservoir.total_amount() + field.total_amount();
        assert!((before - after).abs() < 1e-6);
    }

    #[test]
    fn settling_drains_both_bonded_and_unbonded_stacks() {
        let (mut field, mut reservoir) = field_and_reservoir();

        let idx = field.index_for_position(500.0, 500.0).unwrap();
        field.deposit_at_index(
            idx,
            Material { parts: vec![("Carbon".into(), 100.0)], bonded: true },
        );
        field.deposit_at_index(
            idx,
            Material { parts: vec![("Carbon".into(), 40.0)], bonded: false },
        );

        for _ in 0..20 {
            apply_settling(&mut field, &mut reservoir, DEFAULT_SETTLING_FRACTION);
        }

        // Both stacks drain toward the reservoir now.
        assert!(field.cells[idx].bonded.total_amount() < 100.0);
        assert!(field.cells[idx].unbonded.total_amount() < 40.0);
        assert!(reservoir.total_amount() > 0.0);
    }

    #[test]
    fn settling_preserves_bonded_status_in_the_reservoir() {
        let (mut field, mut reservoir) = field_and_reservoir();
        let field_index = field.index_for_position(500.0, 500.0).unwrap();
        let reservoir_index = reservoir.reservoir_index_for_field_index(&field, field_index);

        field.deposit_at_index(
            field_index,
            Material { parts: vec![("Sulfur".into(), 200.0)], bonded: true },
        );

        for _ in 0..50 {
            apply_settling(&mut field, &mut reservoir, DEFAULT_SETTLING_FRACTION);
        }

        // Settled material landed in the reservoir's BONDED side, not
        // the unbonded side - a relocation, not a debonding event.
        assert!(reservoir.cells[reservoir_index].amount_of(true, "Sulfur") > 0.0);
        assert_eq!(reservoir.cells[reservoir_index].amount_of(false, "Sulfur"), 0.0);
    }

    #[test]
    fn settled_bonded_material_can_be_re_released_by_a_vent_still_bonded() {
        // Full round trip: bonded field material -> settles into the
        // reservoir's bonded stock -> a vent later draws it back out
        // -> it re-enters the field still bonded, at no point having
        // been raw in between.
        let (mut field, mut reservoir) = field_and_reservoir();
        let field_index = field.index_for_position(500.0, 500.0).unwrap();
        let reservoir_index = reservoir.reservoir_index_for_field_index(&field, field_index);

        field.deposit_at_index(
            field_index,
            Material { parts: vec![("Nitrogen".into(), 500.0)], bonded: true },
        );

        // Settle it fully into the reservoir.
        for _ in 0..500 {
            apply_settling(&mut field, &mut reservoir, 0.05);
        }
        assert!(field.cells[field_index].bonded.total_amount() < 1.0);
        let bonded_in_reservoir = reservoir.cells[reservoir_index].amount_of(true, "Nitrogen");
        assert!(bonded_in_reservoir > 400.0);

        // A vent at the same location draws it back into the field.
        let mut vents = vec![Vent {
            x: 500.0,
            y: 500.0,
            composition: vec![("Nitrogen".into(), 1.0)],
            emission_amount: 50.0,
            emission_interval: 0,
            emission_timer: 0,
        }];
        apply_vents(&mut field, &mut reservoir, &mut vents);

        assert!((field.cells[field_index].bonded.total_amount() - 50.0).abs() < 1e-6);
        assert_eq!(field.cells[field_index].unbonded.total_amount(), 0.0);
        assert!(
            (reservoir.cells[reservoir_index].amount_of(true, "Nitrogen") - (bonded_in_reservoir - 50.0)).abs()
                < 1e-6
        );
    }

    #[test]
    fn settling_conserves_total_material_field_plus_reservoir() {
        let (mut field, mut reservoir) = field_and_reservoir();
        let idx = field.index_for_position(500.0, 500.0).unwrap();
        field.deposit_at_index(
            idx,
            Material { parts: vec![("Water".into(), 300.0)], bonded: false },
        );
        field.deposit_at_index(
            idx,
            Material { parts: vec![("Hydrogen".into(), 150.0)], bonded: true },
        );

        let before = field.total_amount() + reservoir.total_amount();

        for _ in 0..100 {
            apply_settling(&mut field, &mut reservoir, DEFAULT_SETTLING_FRACTION);
        }

        let after = field.total_amount() + reservoir.total_amount();
        assert!((before - after).abs() < 1e-6);
    }

    /// Full environment-only loop (vents + diffusion + settling, no
    /// organisms) over many ticks. This is the conservation check
    /// requested before touching perception/organisms at all.
    #[test]
    fn full_environment_loop_conserves_material_over_many_ticks() {
        let (mut field, mut reservoir) = field_and_reservoir();
        reservoir.seed_uniform("Carbon", 20_000.0);
        reservoir.seed_uniform("Methane", 10_000.0);
        reservoir.seed_uniform("Water", 15_000.0);

        let mut vents = vec![
            Vent {
                x: 250.0,
                y: 250.0,
                composition: vec![("Carbon".into(), 0.5), ("Methane".into(), 0.5)],
                emission_amount: 40.0,
                emission_interval: 5,
                emission_timer: 0,
            },
            Vent {
                x: 750.0,
                y: 750.0,
                composition: vec![("Water".into(), 1.0)],
                emission_amount: 20.0,
                emission_interval: 8,
                emission_timer: 0,
            },
        ];

        let before = field.total_amount() + reservoir.total_amount();

        for tick in 0..2000u64 {
            apply_vents(&mut field, &mut reservoir, &mut vents);
            field.diffuse_step(DEFAULT_DIFFUSION_FRACTION);
            if tick % DEFAULT_SETTLING_INTERVAL_TICKS == 0 {
                apply_settling(&mut field, &mut reservoir, DEFAULT_SETTLING_FRACTION);
            }
        }

        let after = field.total_amount() + reservoir.total_amount();

        assert!(
            (before - after).abs() < 1e-4,
            "environment-only loop must conserve total material: before={before}, after={after}"
        );
    }
}
