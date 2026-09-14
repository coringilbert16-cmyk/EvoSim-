//! Environment subsystem: active ecological material field and environmental vents.
//!
//! The active field is the complete environmental material layer. Vents are
//! independent sources that inject valid materials directly into that field.

use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::material_transfer::take_whole_unstructured;
use crate::resources::{merge_parts, Material};

pub const DEFAULT_CELL_SIZE: f64 = 25.0;
pub const DEFAULT_DIFFUSION_FRACTION: f64 = 0.05;
const MATERIAL_EPSILON: f64 = 1e-9;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FieldCell {
    /// Ecological stock occupying this field cell.
    /// Unstructured base stock may be aggregated for field physics. Structured
    /// material remains a distinct object and is never fractionally split.
    pub materials: Vec<Material>,
}

impl FieldCell {
    pub fn empty() -> Self {
        Self {
            materials: Vec::new(),
        }
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
        if !x.is_finite()
            || !y.is_finite()
            || !radius.is_finite()
            || radius < 0.0
            || self.cells.is_empty()
            || self.width_cells == 0
            || self.height_cells == 0
        {
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

    pub fn take_at(
        &mut self,
        x: f64,
        y: f64,
        material_index: usize,
        amount: f64,
    ) -> Option<Material> {
        let index = self.index_for_position(x, y)?;
        self.take_at_index(index, material_index, amount)
    }

    pub fn take_at_index(
        &mut self,
        index: usize,
        material_index: usize,
        amount: f64,
    ) -> Option<Material> {
        let material = self
            .cells
            .get_mut(index)?
            .materials
            .get_mut(material_index)?;
        let requested = if amount.is_finite() && amount >= 1.0 {
            amount.floor() as usize
        } else {
            return None;
        };
        let taken = take_whole_unstructured(material, requested)?;
        self.cells[index]
            .materials
            .retain(|material| !material.is_empty());
        Some(taken)
    }

    /// ACQUIRE transfer: one intact structured material object when available;
    /// otherwise one discrete base unit. The field may contain both, and
    /// acquisition must not destroy the identity of the structured object.
    pub fn take_for_acquisition(&mut self, index: usize) -> Option<Material> {
        let cell = self.cells.get_mut(index)?;
        if let Some(material_index) = cell.materials.iter().position(|material| {
            material.has_internal_structure()
                && material.is_valid()
                && !material.is_empty()
                && material.parts.iter().all(|(_, amount)| {
                    amount.is_finite() && *amount > 0.0 && amount.fract().abs() <= MATERIAL_EPSILON
                })
        }) {
            return Some(cell.materials.swap_remove(material_index));
        }
        let material_index = cell.materials.iter().position(|material| {
            !material.has_internal_structure() && !material.is_empty() && material.is_valid()
        })?;
        let part_index = cell.materials[material_index]
            .parts
            .iter()
            .position(|(_, amount)| {
                amount.is_finite() && *amount >= 1.0 && amount.fract().abs() <= MATERIAL_EPSILON
            })?;
        let name = cell.materials[material_index].parts[part_index].0.clone();
        cell.materials[material_index].parts[part_index].1 -= 1.0;
        cell.materials[material_index]
            .parts
            .retain(|(_, amount)| *amount > MATERIAL_EPSILON);
        let material = Material::free_base(name, 1.0);
        cell.materials.retain(|material| !material.is_empty());
        Some(material)
    }

    pub fn diffuse_step(&mut self, fraction: f64) {
        let fraction = fraction.clamp(0.0, 1.0);
        if fraction <= 0.0 {
            return;
        }
        let n = self.cells.len();
        let mut outgoing: Vec<Vec<Material>> = (0..n).map(|_| Vec::new()).collect();
        for (i, outgoing_cell) in outgoing.iter_mut().enumerate() {
            if self.neighbor_indices(i).is_empty() {
                continue;
            }
            let material_count = self.cells[i].materials.len();
            for material_index in 0..material_count {
                if self.cells[i].materials[material_index].has_internal_structure() {
                    continue;
                }
                let total = self.cells[i].materials[material_index].total_amount();
                if total <= MATERIAL_EPSILON {
                    continue;
                }
                let outflow = (total * fraction).floor() as usize;
                if outflow >= 1 {
                    if let Some(piece) = take_whole_unstructured(
                        &mut self.cells[i].materials[material_index],
                        outflow,
                    ) {
                        outgoing_cell.push(piece);
                    }
                }
            }
            self.cells[i]
                .materials
                .retain(|material| !material.is_empty());
        }
        for (i, outgoing_cell) in outgoing.iter_mut().enumerate() {
            let neighbors = self.neighbor_indices(i);
            for material in outgoing_cell.drain(..) {
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
    let total = mat.total_amount().floor() as usize;
    let base_share = total / neighbors.len();
    let remainder = total % neighbors.len();
    for (k, &neighbor_index) in neighbors.iter().enumerate() {
        let count = base_share + usize::from(k < remainder);
        if count == 0 {
            continue;
        }
        let piece = if count == mat.total_amount() as usize {
            Material {
                parts: std::mem::take(&mut mat.parts),
                internal_bonds: std::mem::take(&mut mat.internal_bonds),
            }
        } else {
            match take_whole_unstructured(&mut mat, count) {
                Some(piece) => piece,
                None => continue,
            }
        };
        if !piece.is_empty() {
            field.deposit_at_index(neighbor_index, piece);
        }
    }
}

/// A vent is an environmental source, not a chemical recipe or reservoir
/// outlet. Each emission independently chooses a valid material and a
/// fluctuating quantity around the vent's long-term average.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Vent {
    pub x: f64,
    pub y: f64,
    pub emission_amount: f64,
    pub emission_interval: u64,
    pub emission_timer: u64,
}

fn scale_material(material: &Material, amount: f64) -> Material {
    let base_amount = material.total_amount();
    if base_amount <= f64::EPSILON {
        return material.clone();
    }
    let scale = amount / base_amount;
    Material {
        parts: material
            .parts
            .iter()
            .map(|(name, part_amount)| (name.clone(), part_amount * scale))
            .collect(),
        internal_bonds: material.internal_bonds.clone(),
    }
}

/// Build the pool from which vents may emit. It contains both elemental/raw
/// materials and the approved environmental compounds. No material category
/// is preferred; selection is uniformly random among valid material kinds.
pub fn valid_vent_materials(catalog: &[crate::resources::BaseResource]) -> Vec<Material> {
    let mut materials = catalog
        .iter()
        .map(|resource| Material::free_base(resource.name.clone(), 1.0))
        .filter(Material::is_valid)
        .collect::<Vec<_>>();
    materials.extend(
        crate::environmental_materials::seed_compounds()
            .into_iter()
            .filter(Material::is_valid),
    );
    materials
}

pub fn apply_vents<R: Rng + ?Sized>(
    field: &mut ActiveMaterialField,
    catalog: &[crate::resources::BaseResource],
    vents: &mut [Vent],
    rng: &mut R,
) {
    let available = valid_vent_materials(catalog);
    if available.is_empty() {
        return;
    }

    for vent in vents.iter_mut() {
        if vent.emission_timer > 0 {
            vent.emission_timer -= 1;
            continue;
        }
        vent.emission_timer = vent.emission_interval;

        let Some(field_index) = field.index_for_position(vent.x, vent.y) else {
            continue;
        };
        let template = &available[rng.gen_range(0..available.len())];
        let fluctuation = rng.gen_range(0.5..1.5);
        let amount = (vent.emission_amount.max(0.0) * fluctuation).max(f64::EPSILON);
        field.deposit_at_index(field_index, scale_material(template, amount));
    }
}

#[cfg(test)]
#[path = "environment_tests.rs"]
mod environment_tests;
