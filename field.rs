// Active material field: fixed-resolution 2D grid holding physical material stacks.

use crate::resources::{merge_parts, Material};
use serde::{Deserialize, Serialize};

pub const DEFAULT_CELL_SIZE: f64 = 25.0;
pub const DEFAULT_DIFFUSION_FRACTION: f64 = 0.05;
pub const MATERIAL_EPSILON: f64 = 1e-9;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FieldCell {
    /// Physical material stacks occupying this ecological cell.
    ///
    /// Unstructured base stock may be aggregated into one entry. Structured
    /// material remains a distinct physical object and is not fractionally
    /// split by ecological diffusion.
    /// Material composition and internal structure remain authoritative.
    pub materials: Vec<Material>,
}

impl FieldCell {
    pub fn empty() -> Self {
        Self { materials: Vec::new() }
    }

    pub fn total_amount(&self) -> f64 {
        self.materials.iter().map(Material::total_amount).sum()
    }

    pub fn total_material(&self) -> Vec<(String, f64)> {
        let mut totals = Vec::new();
        for material in &self.materials {
            for (name, amount) in &material.parts {
                if let Some(existing) = totals.iter_mut().find(|(n, _)| n == name) {
                    existing.1 += amount;
                } else {
                    totals.push((name.clone(), *amount));
                }
            }
        }
        totals
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

    pub fn cell_center(&self, index: usize) -> (f64, f64) {
        let (row, col) = self.row_col_for_index(index);
        (
            (col as f64 + 0.5) * self.cell_size,
            (row as f64 + 0.5) * self.cell_size,
        )
    }

    pub fn cells_within_radius(&self, x: f64, y: f64, radius: f64) -> Vec<usize> {
        if !x.is_finite() || !y.is_finite() || !radius.is_finite() || radius < 0.0 {
            return Vec::new();
        }
        if self.cells.is_empty() || self.width_cells == 0 || self.height_cells == 0 {
            return Vec::new();
        }
        let min_x = (x - radius).max(0.0);
        let max_x = x + radius;
        let min_y = (y - radius).max(0.0);
        let max_y = y + radius;
        let min_col = (min_x / self.cell_size).floor() as usize;
        let max_col =
            ((max_x / self.cell_size).floor() as usize).min(self.width_cells.saturating_sub(1));
        let min_row = (min_y / self.cell_size).floor() as usize;
        let max_row =
            ((max_y / self.cell_size).floor() as usize).min(self.height_cells.saturating_sub(1));
        if min_col >= self.width_cells
            || min_row >= self.height_cells
            || min_col > max_col
            || min_row > max_row
        {
            return Vec::new();
        }
        let radius_squared = radius * radius;
        let mut indices = Vec::new();
        for row in min_row..=max_row {
            for col in min_col..=max_col {
                let index = row * self.width_cells + col;
                let (cell_x, cell_y) = self.cell_center(index);
                let dx = cell_x - x;
                let dy = cell_y - y;
                if dx * dx + dy * dy <= radius_squared {
                    indices.push(index);
                }
            }
        }
        indices
    }

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
        if material.is_empty() || !material.is_valid() {
            return;
        }
        let cell = &mut self.cells[index];
        if material.has_internal_structure() {
            cell.materials.push(material);
            return;
        }

        if let Some(existing) = cell
            .materials
            .iter_mut()
            .find(|existing| !existing.has_internal_structure())
        {
            let mut parts = std::mem::take(&mut existing.parts);
            parts.extend(material.parts);
            existing.parts = merge_parts(&parts);
        } else {
            cell.materials.push(material);
        }
    }

    pub fn take_at(&mut self, x: f64, y: f64, material_index: usize, amount: f64) -> Option<Material> {
        let index = self.index_for_position(x, y)?;
        self.take_at_index(index, material_index, amount)
    }

    pub fn take_at_index(&mut self, index: usize, material_index: usize, amount: f64) -> Option<Material> {
        let material = self.cells.get_mut(index)?.materials.get_mut(material_index)?;
        let taken = material.take(amount);
        self.cells[index].materials.retain(|material| !material.is_empty());
        taken
    }

    pub fn diffuse_step(&mut self, fraction: f64) {
        let fraction = fraction.clamp(0.0, 1.0);
        if fraction <= 0.0 {
            return;
        }
        let n = self.cells.len();
        let mut outgoing: Vec<Vec<Material>> = (0..n).map(|_| Vec::new()).collect();

        for i in 0..n {
            let neighbor_count = self.neighbor_indices(i).len();
            if neighbor_count == 0 {
                continue;
            }
            let material_count = self.cells[i].materials.len();
            for material_index in 0..material_count {
                // Diffusion transfers free ecological stock. A structured
                // material is a physical object; fractionally splitting it
                // here would duplicate its internal bond graph into multiple
                // stacks on every step and cause exponential stack growth.
                if self.cells[i].materials[material_index].has_internal_structure() {
                    continue;
                }

                let total = self.cells[i].materials[material_index].total_amount();
                if total <= MATERIAL_EPSILON {
                    continue;
                }
                let outflow = total * fraction;
                if outflow > MATERIAL_EPSILON {
                    if let Some(piece) = self.cells[i].materials[material_index].take(outflow) {
                        outgoing[i].push(piece);
                    }
                }
            }
            self.cells[i].materials.retain(|material| !material.is_empty());
        }

        for i in 0..n {
            let neighbors = self.neighbor_indices(i);
            for material in outgoing[i].drain(..) {
                distribute_evenly(self, material, &neighbors);
            }
        }
    }

    pub fn total_material(&self) -> Vec<(String, f64)> {
        let mut totals: Vec<(String, f64)> = Vec::new();
        for cell in &self.cells {
            for (name, amount) in cell.total_material() {
                if let Some(existing) = totals.iter_mut().find(|(n, _)| n == &name) {
                    existing.1 += amount;
                } else {
                    totals.push((name, amount));
                }
            }
        }
        totals
    }

    pub fn total_amount(&self) -> f64 {
        self.cells.iter().map(FieldCell::total_amount).sum()
    }
}

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
                internal_bonds: std::mem::take(&mut mat.internal_bonds),
            }
        } else {
            match mat.take(share) {
                Some(piece) => piece,
                None => continue,
            }
        };
        if !piece.is_empty() {
            field.deposit_at_index(neighbor_index, piece);
        }
    }
}
