#![expect(dead_code, reason = "Staged API retained for subsystem integration")]
//! Environment subsystem: the persistent active ecological material field.
//!
//! The active field is the complete environmental material layer. It remains
//! stable until an organism or an explicit environmental event changes it.

use serde::{Deserialize, Serialize};

use crate::material_transfer::take_whole_unstructured;
use crate::physical_material::PhysicalMaterial;
use crate::resources::{merge_parts, Material};

pub const DEFAULT_CELL_SIZE: f64 = 25.0;
const MATERIAL_EPSILON: f64 = 1e-9;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FieldCell {
    /// Aggregate stock for unstructured material. Aggregation is a spatial
    /// optimization only; it is not used to represent an existing composite.
    pub materials: Vec<Material>,
    /// Existing physically realized material objects. Their composition,
    /// internal bonds, and relative realization travel together during physical
    /// boundary transfer and environmental movement.
    #[serde(default)]
    pub physical_materials: Vec<PhysicalMaterial>,
}

impl FieldCell {
    pub fn empty() -> Self {
        Self {
            materials: Vec::new(),
            physical_materials: Vec::new(),
        }
    }

    pub fn total_amount(&self) -> f64 {
        self.materials
            .iter()
            .map(Material::total_amount)
            .sum::<f64>()
            + self
                .physical_materials
                .iter()
                .map(|material| material.material.total_amount())
                .sum::<f64>()
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
        for material in &self.physical_materials {
            for (name, amount) in &material.material.parts {
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
    #[serde(default)]
    pub(crate) revision: u64,
}

pub(crate) enum FieldDeposit {
    Logical(Material),
    Physical(PhysicalMaterial),
}

impl From<Material> for FieldDeposit {
    fn from(material: Material) -> Self {
        Self::Logical(material)
    }
}

impl From<PhysicalMaterial> for FieldDeposit {
    fn from(material: PhysicalMaterial) -> Self {
        Self::Physical(material)
    }
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
            revision: 0,
        }
    }

    pub fn row_col_for_position(&self, x: f64, y: f64) -> Option<(usize, usize)> {
        if !x.is_finite()
            || !y.is_finite()
            || x < 0.0
            || x >= self.width_cells as f64 * self.cell_size
        {
            return None;
        }
        let wrapped_y = y.rem_euclid(self.height_cells as f64 * self.cell_size);
        let col = (x / self.cell_size).floor() as usize;
        let row = (wrapped_y / self.cell_size).floor() as usize;
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
        let min_col = (min_x / self.cell_size).floor() as usize;
        let max_col =
            ((max_x / self.cell_size).floor() as usize).min(self.width_cells.saturating_sub(1));
        if min_col >= self.width_cells || min_col > max_col {
            return Vec::new();
        }

        let field_height = self.height_cells as f64 * self.cell_size;
        let wrapped_y = y.rem_euclid(field_height);
        let radius_squared = radius * radius;
        let mut indices = Vec::new();
        for row in 0..self.height_cells {
            for col in min_col..=max_col {
                let index = row * self.width_cells + col;
                let (cell_x, cell_y) = self.cell_center(index);
                let dx = cell_x - x;
                let direct_dy = (cell_y - wrapped_y).abs();
                let dy = direct_dy.min(field_height - direct_dy);
                if dx * dx + dy * dy <= radius_squared {
                    indices.push(index);
                }
            }
        }
        indices
    }

    /// Return field cells whose spatial bounds intersect the supplied
    /// axis-aligned bounds. Horizontal coordinates remain bounded; vertical
    /// coordinates wrap continuously from top to bottom.
    fn cells_intersecting_bounds(
        &self,
        min_x: f64,
        max_x: f64,
        min_y: f64,
        max_y: f64,
    ) -> Vec<usize> {
        if !min_x.is_finite()
            || !max_x.is_finite()
            || !min_y.is_finite()
            || !max_y.is_finite()
            || min_x > max_x
            || min_y > max_y
        {
            return Vec::new();
        }

        let min_col = ((min_x / self.cell_size).floor() as isize).max(0) as usize;
        let max_col = ((max_x / self.cell_size).floor() as isize)
            .min(self.width_cells.saturating_sub(1) as isize)
            .max(0) as usize;
        if min_col > max_col || self.width_cells == 0 || self.height_cells == 0 {
            return Vec::new();
        }

        let row_start = (min_y / self.cell_size).floor() as isize;
        let row_end = (max_y / self.cell_size).floor() as isize;
        let row_count = row_end.saturating_sub(row_start).saturating_add(1) as usize;
        let rows: Vec<usize> = if row_count >= self.height_cells {
            (0..self.height_cells).collect()
        } else {
            (0..row_count)
                .map(|offset| {
                    (row_start + offset as isize).rem_euclid(self.height_cells as isize) as usize
                })
                .collect()
        };

        let mut indices = Vec::with_capacity(rows.len() * (max_col - min_col + 1));
        for row in rows {
            for col in min_col..=max_col {
                indices.push(row * self.width_cells + col);
            }
        }
        indices
    }

    /// Remove the already-realized physical constituents whose placement points
    /// are inside the organism's realized body geometry. The field grid is a
    /// spatial index: only cells intersecting the body's bounds are examined,
    /// while physical containment remains authoritative.
    pub(crate) fn take_contained_physical_materials(
        &mut self,
        body: &crate::organism_geometry::OrganismBodyGeometry,
    ) -> Vec<PhysicalMaterial> {
        let candidate_indices =
            self.cells_intersecting_bounds(body.min_x, body.max_x, body.min_y, body.max_y);
        let mut contained = Vec::new();
        for index in candidate_indices {
            let cell = &mut self.cells[index];
            let mut remaining = Vec::with_capacity(cell.physical_materials.len());
            for physical in cell.physical_materials.drain(..) {
                let Some(placements) = physical.placements.as_ref() else {
                    remaining.push(physical);
                    continue;
                };
                if placements.len() != physical.material.parts.len() {
                    remaining.push(physical);
                    continue;
                }
                let selected: Vec<usize> = placements
                    .iter()
                    .enumerate()
                    .filter_map(|(index, placement)| {
                        body.contains_point(placement.x, placement.y)
                            .then_some(index)
                    })
                    .collect();
                if selected.is_empty() {
                    remaining.push(physical);
                    continue;
                }
                if selected.len() == placements.len() {
                    contained.push(physical);
                    continue;
                }
                match crate::material_transfer::split_physical_material(&physical, &selected) {
                    Some((inside, outside)) => {
                        contained.push(inside);
                        remaining.push(outside);
                    }
                    None => remaining.push(physical),
                }
            }
            cell.physical_materials = remaining;
        }
        if !contained.is_empty() {
            self.revision = self.revision.wrapping_add(1);
        }
        contained
    }

    pub fn neighbor_indices(&self, index: usize) -> Vec<usize> {
        let (row, col) = self.row_col_for_index(index);
        let mut out = Vec::with_capacity(4);
        let above = if row == 0 {
            self.height_cells.saturating_sub(1)
        } else {
            row - 1
        };
        let below = if row + 1 == self.height_cells {
            0
        } else {
            row + 1
        };
        out.push(above * self.width_cells + col);
        out.push(below * self.width_cells + col);
        if col > 0 {
            out.push(row * self.width_cells + (col - 1));
        }
        if col + 1 < self.width_cells {
            out.push(row * self.width_cells + (col + 1));
        }
        out
    }

    pub(crate) fn deposit<T: Into<FieldDeposit>>(&mut self, x: f64, y: f64, material: T) -> bool {
        let Some(index) = self.index_for_position(x, y) else {
            return false;
        };
        match material.into() {
            FieldDeposit::Logical(material) => {
                self.deposit_at_index(index, material);
                true
            }
            FieldDeposit::Physical(material) => self.deposit_physical_at_index(index, material),
        }
    }

    pub fn deposit_at_index(&mut self, index: usize, material: Material) {
        if material.is_empty() || !material.is_valid() {
            return;
        }
        self.revision = self.revision.wrapping_add(1);
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

    /// Deposit an already realized physical material. This operation transfers
    /// the existing object; it does not create or re-form any of its bonds.
    pub fn deposit_physical_at_index(&mut self, index: usize, material: PhysicalMaterial) -> bool {
        if !material.is_realized() || material.material.is_empty() || !material.material.is_valid()
        {
            return false;
        }
        self.cells[index].physical_materials.push(material);
        self.revision = self.revision.wrapping_add(1);
        true
    }

    pub fn deposit_physical(&mut self, x: f64, y: f64, material: PhysicalMaterial) -> bool {
        let Some(index) = self.index_for_position(x, y) else {
            return false;
        };
        self.deposit_physical_at_index(index, material)
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

#[cfg(test)]
#[path = "environment_tests.rs"]
mod environment_tests;
