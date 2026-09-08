// Active material field: fixed-resolution 2D grid holding ecological material stock.

use crate::field_material::FieldMaterial;
use crate::material_transfer::take_whole_unstructured;
use crate::resources::Material;
use serde::{Deserialize, Serialize};

pub const DEFAULT_CELL_SIZE: f64 = 25.0;
pub const DEFAULT_DIFFUSION_FRACTION: f64 = 0.05;
pub const MATERIAL_EPSILON: f64 = 1e-9;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FieldCell { pub materials: Vec<FieldMaterial> }
impl FieldCell {
    pub fn empty() -> Self { Self { materials: Vec::new() } }
    pub fn total_amount(&self) -> f64 { self.materials.iter().map(|m| m.material.total_amount()).sum() }
    pub fn total_material(&self) -> Vec<(String, f64)> {
        let mut totals = Vec::new();
        for field_material in &self.materials {
            for (name, amount) in &field_material.material.parts {
                if let Some(existing) = totals.iter_mut().find(|(n, _)| n == name) { existing.1 += amount; }
                else { totals.push((name.clone(), *amount)); }
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
    next_material_id: u64,
}
impl ActiveMaterialField {
    pub fn new(world_width: f64, world_height: f64, cell_size: f64) -> Self {
        let cell_size = cell_size.max(1.0);
        let width_cells = (world_width / cell_size).ceil().max(1.0) as usize;
        let height_cells = (world_height / cell_size).ceil().max(1.0) as usize;
        let cells = (0..width_cells * height_cells).map(|_| FieldCell::empty()).collect();
        Self { cell_size, width_cells, height_cells, cells, next_material_id: 1 }
    }
    pub fn row_col_for_position(&self, x: f64, y: f64) -> Option<(usize, usize)> {
        if !x.is_finite() || !y.is_finite() || x < 0.0 || y < 0.0 { return None; }
        let col = (x / self.cell_size).floor() as usize; let row = (y / self.cell_size).floor() as usize;
        if col >= self.width_cells || row >= self.height_cells { return None; }
        Some((row, col))
    }
    pub fn index_for_position(&self, x: f64, y: f64) -> Option<usize> { self.row_col_for_position(x, y).map(|(row, col)| row * self.width_cells + col) }
    fn row_col_for_index(&self, index: usize) -> (usize, usize) { (index / self.width_cells, index % self.width_cells) }
    pub fn cell_center(&self, index: usize) -> (f64, f64) { let (row, col) = self.row_col_for_index(index); ((col as f64 + 0.5) * self.cell_size, (row as f64 + 0.5) * self.cell_size) }
    pub fn cells_within_radius(&self, x: f64, y: f64, radius: f64) -> Vec<usize> {
        if !x.is_finite() || !y.is_finite() || !radius.is_finite() || radius < 0.0 || self.cells.is_empty() || self.width_cells == 0 || self.height_cells == 0 { return Vec::new(); }
        let min_x = (x - radius).max(0.0); let max_x = x + radius; let min_y = (y - radius).max(0.0); let max_y = y + radius;
        let min_col = (min_x / self.cell_size).floor() as usize; let max_col = ((max_x / self.cell_size).floor() as usize).min(self.width_cells.saturating_sub(1));
        let min_row = (min_y / self.cell_size).floor() as usize; let max_row = ((max_y / self.cell_size).floor() as usize).min(self.height_cells.saturating_sub(1));
        if min_col >= self.width_cells || min_row >= self.height_cells || min_col > max_col || min_row > max_row { return Vec::new(); }
        let radius_squared = radius * radius; let mut indices = Vec::new();
        for row in min_row..=max_row { for col in min_col..=max_col { let index = row * self.width_cells + col; let (cell_x, cell_y) = self.cell_center(index); let dx = cell_x - x; let dy = cell_y - y; if dx * dx + dy * dy <= radius_squared { indices.push(index); } } }
        indices
    }
    pub fn neighbor_indices(&self, index: usize) -> Vec<usize> {
        let (row, col) = self.row_col_for_index(index); let mut out = Vec::with_capacity(4);
        if row > 0 { out.push((row - 1) * self.width_cells + col); } if row + 1 < self.height_cells { out.push((row + 1) * self.width_cells + col); }
        if col > 0 { out.push(row * self.width_cells + col - 1); } if col + 1 < self.width_cells { out.push(row * self.width_cells + col + 1); } out
    }
    fn allocate_id(&mut self) -> u64 { let id = self.next_material_id; self.next_material_id = self.next_material_id.saturating_add(1); id }

    pub fn deposit(&mut self, x: f64, y: f64, material: Material) -> bool {
        let Some(index) = self.index_for_position(x, y) else { return false; };
        if material.has_internal_structure() { return false; }
        self.deposit_unstructured_at(index, material, x, y); true
    }
    /// Compatibility helper for cell-only callers. Geometry-sensitive code
    /// should use `deposit` or `deposit_structured` with explicit coordinates.
    pub fn deposit_at_index(&mut self, index: usize, material: Material) {
        if material.has_internal_structure() { return; }
        let (x, y) = self.cell_center(index); self.deposit_unstructured_at(index, material, x, y);
    }
    pub fn deposit_structured(&mut self, material: Material, placements: Vec<crate::structure::Placement>) -> bool {
        if material.is_empty() || !material.is_valid() || !material.has_internal_structure() || placements.len() != material.parts.len() { return false; }
        let Some((x, y)) = placements.first().map(|p| (p.x, p.y)) else { return false; };
        let Some(index) = self.index_for_position(x, y) else { return false; };
        let id = self.allocate_id(); let Some(instance) = FieldMaterial::new(id, material, placements) else { return false; };
        self.cells[index].materials.push(instance); true
    }
    fn deposit_unstructured_at(&mut self, index: usize, material: Material, x: f64, y: f64) {
        if material.is_empty() || !material.is_valid() || material.has_internal_structure() || !x.is_finite() || !y.is_finite() { return; }
        let id = self.allocate_id();
        let placements = material.parts.iter().map(|_| crate::structure::Placement { x, y, rotation_radians: 0.0 }).collect();
        if let Some(instance) = FieldMaterial::new(id, material, placements) { self.cells[index].materials.push(instance); }
    }
    pub fn take_at(&mut self, x: f64, y: f64, material_index: usize, amount: f64) -> Option<Material> { let index = self.index_for_position(x, y)?; self.take_at_index(index, material_index, amount) }
    pub fn take_at_index(&mut self, index: usize, material_index: usize, amount: f64) -> Option<Material> {
        let material = self.cells.get_mut(index)?.materials.get_mut(material_index)?;
        let requested = if amount.is_finite() && amount >= 1.0 { amount.floor() as usize } else { return None; };
        let taken = take_whole_unstructured(&mut material.material, requested)?;
        self.cells[index].materials.retain(|material| !material.material.is_empty()); Some(taken)
    }
    pub fn take_for_acquisition(&mut self, index: usize) -> Option<Material> {
        let cell = self.cells.get_mut(index)?;
        let material_index = cell.materials.iter().position(|m| !m.material.has_internal_structure() && !m.material.is_empty() && m.material.is_valid())?;
        let part_index = cell.materials[material_index].material.parts.iter().position(|(_, amount)| amount.is_finite() && *amount >= 1.0 && amount.fract().abs() <= MATERIAL_EPSILON)?;
        let name = cell.materials[material_index].material.parts[part_index].0.clone();
        cell.materials[material_index].material.parts[part_index].1 -= 1.0;
        cell.materials[material_index].material.parts.retain(|(_, amount)| *amount > MATERIAL_EPSILON);
        let material = Material::free_base(name, 1.0); cell.materials.retain(|m| !m.material.is_empty()); Some(material)
    }
    pub fn diffuse_step(&mut self, fraction: f64) {
        let fraction = fraction.clamp(0.0, 1.0); if fraction <= 0.0 { return; }
        let n = self.cells.len(); let mut moves: Vec<(usize, FieldMaterial)> = Vec::new();
        for i in 0..n {
            let neighbors = self.neighbor_indices(i); if neighbors.is_empty() { continue; }
            let material_count = self.cells[i].materials.len();
            for material_index in 0..material_count {
                if self.cells[i].materials[material_index].material.has_internal_structure() { continue; }
                let total = self.cells[i].materials[material_index].material.total_amount(); if total <= MATERIAL_EPSILON { continue; }
                let outflow = (total * fraction).floor() as usize; if outflow < 1 { continue; }
                let source = self.cells[i].materials[material_index].clone();
                let Some(piece) = take_whole_unstructured(&mut self.cells[i].materials[material_index].material, outflow) else { continue; };
                let Some(source_position) = source.placements.first().copied() else { continue; };
                let base_share = outflow / neighbors.len(); let remainder = outflow % neighbors.len();
                for (k, &neighbor_index) in neighbors.iter().enumerate() {
                    let count = base_share + usize::from(k < remainder); if count == 0 { continue; }
                    let mut piece_part = piece.clone();
                    if count != outflow { let Some(taken) = take_whole_unstructured(&mut piece_part, count) else { continue; }; piece_part = taken; }
                    let (target_x, target_y) = self.cell_center(neighbor_index); let dx = target_x - source_position.x; let dy = target_y - source_position.y;
                    let id = self.allocate_id();
                    let placements = piece_part.parts.iter().map(|_| crate::structure::Placement { x: source_position.x + dx, y: source_position.y + dy, rotation_radians: source_position.rotation_radians }).collect();
                    if let Some(instance) = FieldMaterial::new(id, piece_part, placements) { moves.push((neighbor_index, instance)); }
                }
            }
            self.cells[i].materials.retain(|m| !m.material.is_empty());
        }
        for (neighbor_index, instance) in moves { self.cells[neighbor_index].materials.push(instance); }
    }
    pub fn total_material(&self) -> Vec<(String, f64)> {
        let mut totals = Vec::new();
        for cell in &self.cells { for (name, amount) in cell.total_material() { if let Some(existing) = totals.iter_mut().find(|(n, _)| n == &name) { existing.1 += amount; } else { totals.push((name, amount)); } } }
        totals
    }
    pub fn total_amount(&self) -> f64 { self.cells.iter().map(FieldCell::total_amount).sum() }
}
