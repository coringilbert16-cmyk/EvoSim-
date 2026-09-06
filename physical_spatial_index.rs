//! Spatial lookup for persistent physical environmental objects.
//!
//! This is deliberately separate from `ActiveMaterialField`: the field is a
//! bulk ecological grid, while this index accelerates queries over explicitly
//! realized physical objects.

use crate::material_geometry::PhysicalMaterialInstance;
use crate::physical_environment::PhysicalEnvironment;

#[derive(Clone, Debug, PartialEq)]
pub struct PhysicalSpatialIndex {
    cell_size: f64,
    width_cells: usize,
    height_cells: usize,
    buckets: Vec<Vec<usize>>,
}

impl PhysicalSpatialIndex {
    pub fn new(width: f64, height: f64, cell_size: f64) -> Self {
        let cell_size = cell_size.max(1.0);
        let width_cells = (width / cell_size).ceil().max(1.0) as usize;
        let height_cells = (height / cell_size).ceil().max(1.0) as usize;
        Self {
            cell_size,
            width_cells,
            height_cells,
            buckets: vec![Vec::new(); width_cells * height_cells],
        }
    }

    pub fn clear(&mut self) {
        for bucket in &mut self.buckets {
            bucket.clear();
        }
    }

    fn index_for_position(&self, x: f64, y: f64) -> Option<usize> {
        if !x.is_finite() || !y.is_finite() || x < 0.0 || y < 0.0 {
            return None;
        }
        let col = (x / self.cell_size).floor() as usize;
        let row = (y / self.cell_size).floor() as usize;
        if col >= self.width_cells || row >= self.height_cells {
            return None;
        }
        Some(row * self.width_cells + col)
    }

    fn index_bounds(&self, min: f64, max: f64, limit: usize) -> Option<(usize, usize)> {
        if !min.is_finite() || !max.is_finite() || min > max || max < 0.0 {
            return None;
        }
        let min_index = (min.max(0.0) / self.cell_size).floor() as usize;
        let max_index = (max.max(0.0) / self.cell_size).floor() as usize;
        if min_index >= limit {
            return None;
        }
        Some((min_index.min(limit - 1), max_index.min(limit - 1)))
    }

    pub fn rebuild(&mut self, environment: &PhysicalEnvironment) {
        self.clear();
        for (index, instance) in environment.iter().enumerate() {
            self.insert_instance(index, instance);
        }
    }

    pub fn insert_instance(&mut self, object_index: usize, instance: &PhysicalMaterialInstance) {
        let Some((min_col, max_col)) = self.index_bounds(
            instance.geometry.min_x,
            instance.geometry.max_x,
            self.width_cells,
        ) else {
            return;
        };
        let Some((min_row, max_row)) = self.index_bounds(
            instance.geometry.min_y,
            instance.geometry.max_y,
            self.height_cells,
        ) else {
            return;
        };
        for row in min_row..=max_row {
            for col in min_col..=max_col {
                self.buckets[row * self.width_cells + col].push(object_index);
            }
        }
    }

    pub fn candidate_indices(&self, x: f64, y: f64, radius: f64) -> Vec<usize> {
        if !x.is_finite() || !y.is_finite() || !radius.is_finite() || radius < 0.0 {
            return Vec::new();
        }
        let Some((min_col, max_col)) = self.index_bounds(x - radius, x + radius, self.width_cells)
        else {
            return Vec::new();
        };
        let Some((min_row, max_row)) = self.index_bounds(y - radius, y + radius, self.height_cells)
        else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for row in min_row..=max_row {
            for col in min_col..=max_col {
                out.extend(&self.buckets[row * self.width_cells + col]);
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    }

    pub fn candidate_indices_for_instance(
        &self,
        instance: &PhysicalMaterialInstance,
    ) -> Vec<usize> {
        let Some((min_col, max_col)) = self.index_bounds(
            instance.geometry.min_x,
            instance.geometry.max_x,
            self.width_cells,
        ) else {
            return Vec::new();
        };
        let Some((min_row, max_row)) = self.index_bounds(
            instance.geometry.min_y,
            instance.geometry.max_y,
            self.height_cells,
        ) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for row in min_row..=max_row {
            for col in min_col..=max_col {
                out.extend(&self.buckets[row * self.width_cells + col]);
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    }

    pub fn bucket_count(&self) -> usize {
        self.buckets.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physical_environment::PhysicalEnvironment;
    use crate::resources::{default_catalog, Material};
    use crate::structure::Placement;

    fn placement(x: f64, y: f64) -> Placement {
        Placement {
            x,
            y,
            rotation_radians: 0.0,
        }
    }

    #[test]
    fn rebuild_indexes_realized_object_by_its_geometry() {
        let catalog = default_catalog();
        let mut environment = PhysicalEnvironment::new();
        environment
            .realize(
                Material::free_base("Carbon", 1.0),
                &[placement(10.0, 10.0)],
                &catalog,
            )
            .unwrap();
        environment
            .realize(
                Material::free_base("Carbon", 1.0),
                &[placement(90.0, 90.0)],
                &catalog,
            )
            .unwrap();

        let mut index = PhysicalSpatialIndex::new(100.0, 100.0, 25.0);
        index.rebuild(&environment);

        assert_eq!(index.candidate_indices(10.0, 10.0, 1.0), vec![0]);
        assert_eq!(index.candidate_indices(90.0, 90.0, 1.0), vec![1]);
    }

    #[test]
    fn broad_phase_returns_candidates_not_false_contact() {
        let catalog = default_catalog();
        let mut environment = PhysicalEnvironment::new();
        environment
            .realize(
                Material::free_base("Carbon", 1.0),
                &[placement(24.0, 24.0)],
                &catalog,
            )
            .unwrap();

        let mut index = PhysicalSpatialIndex::new(100.0, 100.0, 25.0);
        index.rebuild(&environment);

        assert_eq!(index.candidate_indices(24.0, 24.0, 5.0), vec![0]);
        assert!(index.candidate_indices(50.0, 50.0, 1.0).is_empty());
    }

    #[test]
    fn candidate_query_is_unique_when_object_spans_multiple_buckets() {
        let catalog = default_catalog();
        let mut environment = PhysicalEnvironment::new();
        environment
            .realize(
                Material::free_base("Carbon", 1.0),
                &[placement(24.0, 24.0)],
                &catalog,
            )
            .unwrap();

        let mut index = PhysicalSpatialIndex::new(100.0, 100.0, 10.0);
        index.rebuild(&environment);
        let candidates = index.candidate_indices_for_instance(environment.get(0).unwrap());
        assert_eq!(candidates, vec![0]);
    }

    #[test]
    fn invalid_query_is_empty() {
        let index = PhysicalSpatialIndex::new(100.0, 100.0, 10.0);
        assert!(index.candidate_indices(f64::NAN, 0.0, 5.0).is_empty());
        assert!(index.candidate_indices(0.0, 0.0, -1.0).is_empty());
    }

    #[test]
    fn empty_environment_produces_empty_buckets() {
        let environment = PhysicalEnvironment::new();
        let mut index = PhysicalSpatialIndex::new(100.0, 100.0, 10.0);
        index.rebuild(&environment);
        assert_eq!(index.bucket_count(), 100);
        assert!(index.candidate_indices(50.0, 50.0, 10.0).is_empty());
    }
}
