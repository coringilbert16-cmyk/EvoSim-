//! Sparse, balanced quadtree storage for the production spatial resonance field.
//!
//! This first adaptive-grid slice owns physical cell coordinates and persistent
//! leapfrog state, and provides conservative refine/coarsen transfers. It does
//! not yet advance the wave equation: the coarse/fine finite-volume operator
//! must be integrated and validated before this storage is connected to ticks.

use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AdaptiveGridError {
    InvalidDimensions,
    InvalidParameter,
    MaximumRefinementReached,
    MissingCell,
    IncompleteChildren,
    UnbalancedRefinement,
    InvalidState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct CellKey {
    /// Refinement level; zero is the configured base-cell resolution.
    pub(crate) level: u8,
    /// Integer cell coordinate at this level.
    pub(crate) x: u32,
    pub(crate) y: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AdaptiveCell {
    pub(crate) previous: f64,
    pub(crate) current: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CellBounds {
    pub(crate) min_x: f64,
    pub(crate) min_y: f64,
    pub(crate) max_x: f64,
    pub(crate) max_y: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SharedFace {
    pub(crate) first: CellKey,
    pub(crate) second: CellKey,
    /// Physical length of the shared face.
    pub(crate) length: f64,
    /// Physical distance between the two cell centers, normal to the face.
    pub(crate) center_distance: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct IntegratedState {
    /// Area integral of displacement at the current leapfrog level.
    pub(crate) current_displacement: f64,
    /// Area integral of displacement at the previous leapfrog level.
    pub(crate) previous_displacement: f64,
}

#[derive(Clone, Debug)]
pub(crate) struct AdaptiveSpatialGrid {
    width: u32,
    height: u32,
    base_spacing: f64,
    max_refinement_level: u8,
    leaves: BTreeMap<CellKey, AdaptiveCell>,
}

impl AdaptiveSpatialGrid {
    pub(crate) fn new(
        width: u32,
        height: u32,
        base_spacing: f64,
        max_refinement_level: u8,
    ) -> Result<Self, AdaptiveGridError> {
        if width == 0 || height == 0 {
            return Err(AdaptiveGridError::InvalidDimensions);
        }
        if !base_spacing.is_finite()
            || base_spacing <= 0.0
            || max_refinement_level > 20
        {
            return Err(AdaptiveGridError::InvalidParameter);
        }
        let finest_scale = 1_u32
            .checked_shl(max_refinement_level as u32)
            .ok_or(AdaptiveGridError::InvalidDimensions)?;
        width
            .checked_mul(finest_scale)
            .and_then(|_| height.checked_mul(finest_scale))
            .ok_or(AdaptiveGridError::InvalidDimensions)?;
        let capacity = (width as usize)
            .checked_mul(height as usize)
            .ok_or(AdaptiveGridError::InvalidDimensions)?;
        let mut leaves = BTreeMap::new();
        for y in 0..height {
            for x in 0..width {
                leaves.insert(
                    CellKey { level: 0, x, y },
                    AdaptiveCell {
                        previous: 0.0,
                        current: 0.0,
                    },
                );
            }
        }
        debug_assert_eq!(leaves.len(), capacity);
        Ok(Self {
            width,
            height,
            base_spacing,
            max_refinement_level,
            leaves,
        })
    }

    pub(crate) fn width(&self) -> u32 {
        self.width
    }

    pub(crate) fn height(&self) -> u32 {
        self.height
    }

    pub(crate) fn base_spacing(&self) -> f64 {
        self.base_spacing
    }

    pub(crate) fn max_refinement_level(&self) -> u8 {
        self.max_refinement_level
    }

    pub(crate) fn leaf_count(&self) -> usize {
        self.leaves.len()
    }

    pub(crate) fn leaves(&self) -> impl Iterator<Item = (CellKey, AdaptiveCell)> + '_ {
        self.leaves.iter().map(|(key, state)| (*key, *state))
    }

    pub(crate) fn cell_bounds(&self, key: CellKey) -> Option<CellBounds> {
        if !self.leaves.contains_key(&key) {
            return None;
        }
        let scale = 2_u32.checked_pow(key.level as u32)? as f64;
        let spacing = self.base_spacing / scale;
        let min_x = key.x as f64 * spacing;
        let min_y = key.y as f64 * spacing;
        Some(CellBounds {
            min_x,
            min_y,
            max_x: min_x + spacing,
            max_y: min_y + spacing,
        })
    }

    pub(crate) fn set_state(
        &mut self,
        key: CellKey,
        state: AdaptiveCell,
    ) -> Result<(), AdaptiveGridError> {
        if !state.previous.is_finite() || !state.current.is_finite() {
            return Err(AdaptiveGridError::InvalidState);
        }
        let cell = self
            .leaves
            .get_mut(&key)
            .ok_or(AdaptiveGridError::MissingCell)?;
        *cell = state;
        Ok(())
    }

    /// Refines one leaf into four children. Child values inherit the parent
    /// state as piecewise-constant cell averages, preserving both area
    /// integrals exactly. Refinement is rejected if it would violate 2:1
    /// balance with an existing face neighbor.
    pub(crate) fn refine(&mut self, key: CellKey) -> Result<[CellKey; 4], AdaptiveGridError> {
        let parent = *self
            .leaves
            .get(&key)
            .ok_or(AdaptiveGridError::MissingCell)?;
        if key.level >= self.max_refinement_level {
            return Err(AdaptiveGridError::MaximumRefinementReached);
        }
        let child_level = key.level + 1;
        let child_x = key.x.checked_mul(2).ok_or(AdaptiveGridError::InvalidDimensions)?;
        let child_y = key.y.checked_mul(2).ok_or(AdaptiveGridError::InvalidDimensions)?;
        let children = [
            CellKey { level: child_level, x: child_x, y: child_y },
            CellKey { level: child_level, x: child_x + 1, y: child_y },
            CellKey { level: child_level, x: child_x, y: child_y + 1 },
            CellKey { level: child_level, x: child_x + 1, y: child_y + 1 },
        ];

        let parent_bounds = self.cell_bounds(key).ok_or(AdaptiveGridError::MissingCell)?;
        for (neighbor_key, _) in self.leaves.iter() {
            if *neighbor_key != key
                && cells_share_face(parent_bounds, self.cell_bounds(*neighbor_key).unwrap())
                && neighbor_key.level + 1 < child_level
            {
                return Err(AdaptiveGridError::UnbalancedRefinement);
            }
        }

        self.leaves.remove(&key);
        for child in children {
            self.leaves.insert(child, parent);
        }
        Ok(children)
    }

    /// Coarsens four sibling leaves into their parent using area-weighted
    /// restriction. The parent is not created if it would violate 2:1 balance.
    pub(crate) fn coarsen(
        &mut self,
        parent: CellKey,
    ) -> Result<AdaptiveCell, AdaptiveGridError> {
        let child_level = parent
            .level
            .checked_add(1)
            .ok_or(AdaptiveGridError::IncompleteChildren)?;
        let child_x = parent.x.checked_mul(2).ok_or(AdaptiveGridError::IncompleteChildren)?;
        let child_y = parent.y.checked_mul(2).ok_or(AdaptiveGridError::IncompleteChildren)?;
        let children = [
            CellKey { level: child_level, x: child_x, y: child_y },
            CellKey { level: child_level, x: child_x + 1, y: child_y },
            CellKey { level: child_level, x: child_x, y: child_y + 1 },
            CellKey { level: child_level, x: child_x + 1, y: child_y + 1 },
        ];
        let states = children.map(|key| {
            self.leaves
                .get(&key)
                .copied()
                .ok_or(AdaptiveGridError::IncompleteChildren)
        });
        let [a, b, c, d] = states;
        let states = [a?, b?, c?, d?];
        let parent_state = AdaptiveCell {
            previous: states.iter().map(|state| state.previous).sum::<f64>() * 0.25,
            current: states.iter().map(|state| state.current).sum::<f64>() * 0.25,
        };
        if !parent_state.previous.is_finite() || !parent_state.current.is_finite() {
            return Err(AdaptiveGridError::InvalidState);
        }

        let parent_bounds = self.bounds_for_key(parent);
        for (neighbor_key, _) in self.leaves.iter() {
            if children.contains(neighbor_key) {
                continue;
            }
            if cells_share_face(parent_bounds, self.cell_bounds(*neighbor_key).unwrap())
                && neighbor_key.level > parent.level + 1
            {
                return Err(AdaptiveGridError::UnbalancedRefinement);
            }
        }

        for child in children {
            self.leaves.remove(&child);
        }
        self.leaves.insert(parent, parent_state);
        Ok(parent_state)
    }

    /// Returns the state of the leaf containing a physical position. Cell
    /// values are finite-volume averages; this is not a nearest-cell fallback.
    pub(crate) fn sample(&self, x: f64, y: f64) -> Option<AdaptiveCell> {
        if !x.is_finite()
            || !y.is_finite()
            || x < 0.0
            || y < 0.0
            || x >= self.width as f64 * self.base_spacing
            || y >= self.height as f64 * self.base_spacing
        {
            return None;
        }
        for level in (0..=self.max_refinement_level).rev() {
            let spacing = self.base_spacing / 2_u32.checked_pow(level as u32)? as f64;
            let key = CellKey {
                level,
                x: (x / spacing).floor() as u32,
                y: (y / spacing).floor() as u32,
            };
            if let Some(state) = self.leaves.get(&key) {
                return Some(*state);
            }
        }
        None
    }

    /// Enumerates each face-connected leaf pair once in deterministic key order.
    /// Face conductance for the finite-volume operator is length / center distance.
    pub(crate) fn shared_faces(&self) -> Vec<SharedFace> {
        type Segment = (u32, u32, CellKey);
        let finest_scale = 1_u32 << self.max_refinement_level;
        let finest_spacing = self.base_spacing / finest_scale as f64;
        let mut vertical_left: BTreeMap<u32, Vec<Segment>> = BTreeMap::new();
        let mut vertical_right: BTreeMap<u32, Vec<Segment>> = BTreeMap::new();
        let mut horizontal_bottom: BTreeMap<u32, Vec<Segment>> = BTreeMap::new();
        let mut horizontal_top: BTreeMap<u32, Vec<Segment>> = BTreeMap::new();

        for key in self.leaves.keys().copied() {
            let scale = 1_u32 << (self.max_refinement_level - key.level);
            let min_x = key.x * scale;
            let min_y = key.y * scale;
            let max_x = min_x + scale;
            let max_y = min_y + scale;
            vertical_left
                .entry(min_x)
                .or_default()
                .push((min_y, max_y, key));
            vertical_right
                .entry(max_x)
                .or_default()
                .push((min_y, max_y, key));
            horizontal_bottom
                .entry(min_y)
                .or_default()
                .push((min_x, max_x, key));
            horizontal_top
                .entry(max_y)
                .or_default()
                .push((min_x, max_x, key));
        }

        let mut faces = Vec::new();
        append_shared_faces(
            &vertical_right,
            &vertical_left,
            finest_spacing,
            self.max_refinement_level,
            &mut faces,
        );
        append_shared_faces(
            &horizontal_top,
            &horizontal_bottom,
            finest_spacing,
            &mut faces,
        );
        faces.sort_by_key(|face| (face.first, face.second));
        faces
    }

    /// Area integrals of the two persistent leapfrog displacement levels.
    /// Refinement and coarsening preserve these quantities to roundoff.
    pub(crate) fn integrated_state(&self) -> IntegratedState {
        let mut total = IntegratedState::default();
        for (key, state) in &self.leaves {
            let area = self.cell_area(*key);
            total.current_displacement += area * state.current;
            total.previous_displacement += area * state.previous;
        }
        total
    }

    fn bounds_for_key(&self, key: CellKey) -> CellBounds {
        let spacing = self.base_spacing / 2_u32.pow(key.level as u32) as f64;
        let min_x = key.x as f64 * spacing;
        let min_y = key.y as f64 * spacing;
        CellBounds {
            min_x,
            min_y,
            max_x: min_x + spacing,
            max_y: min_y + spacing,
        }
    }

    fn cell_area(&self, key: CellKey) -> f64 {
        let spacing = self.base_spacing / 2_u32.pow(key.level as u32) as f64;
        spacing * spacing
    }
}

fn append_shared_faces(
    first_sides: &BTreeMap<u32, Vec<(u32, u32, CellKey)>>,
    second_sides: &BTreeMap<u32, Vec<(u32, u32, CellKey)>>,
    finest_spacing: f64,
    max_refinement_level: u8,
    output: &mut Vec<SharedFace>,
) {
    for (coordinate, first_segments) in first_sides {
        let Some(second_segments) = second_sides.get(coordinate) else {
            continue;
        };
        let mut first = first_segments.clone();
        let mut second = second_segments.clone();
        first.sort_by_key(|segment| (segment.0, segment.1, segment.2));
        second.sort_by_key(|segment| (segment.0, segment.1, segment.2));
        let (mut first_index, mut second_index) = (0, 0);
        while first_index < first.len() && second_index < second.len() {
            let (first_start, first_end, first_key) = first[first_index];
            let (second_start, second_end, second_key) = second[second_index];
            let overlap_start = first_start.max(second_start);
            let overlap_end = first_end.min(second_end);
            if overlap_end > overlap_start {
                let first_width =
                    1_u32 << (max_refinement_level - first_key.level);
                let second_width =
                    1_u32 << (max_refinement_level - second_key.level);
                let center_distance =
                    0.5 * (first_width + second_width) as f64 * finest_spacing;
                let (first_key, second_key) = if first_key < second_key {
                    (first_key, second_key)
                } else {
                    (second_key, first_key)
                };
                output.push(SharedFace {
                    first: first_key,
                    second: second_key,
                    length: (overlap_end - overlap_start) as f64 * finest_spacing,
                    center_distance,
                });
            }
            if first_end <= second_end {
                first_index += 1;
            }
            if second_end <= first_end {
                second_index += 1;
            }
        }
    }
}

fn cells_share_face(a: CellBounds, b: CellBounds) -> bool {
    shared_face_geometry(a, b).is_some()
}

fn shared_face_geometry(a: CellBounds, b: CellBounds) -> Option<(f64, f64)> {
    let tolerance = 1.0e-12 * a.max_x.abs().max(a.max_y.abs()).max(b.max_x.abs()).max(b.max_y.abs()).max(1.0);
    let vertical_overlap = a.max_y.min(b.max_y) - a.min_y.max(b.min_y);
    let horizontal_overlap = a.max_x.min(b.max_x) - a.min_x.max(b.min_x);
    if (a.max_x - b.min_x).abs() <= tolerance || (b.max_x - a.min_x).abs() <= tolerance {
        if vertical_overlap > tolerance {
            return Some((vertical_overlap, 0.5 * (a.max_x - a.min_x + b.max_x - b.min_x)));
        }
    }
    if (a.max_y - b.min_y).abs() <= tolerance || (b.max_y - a.min_y).abs() <= tolerance {
        if horizontal_overlap > tolerance {
            return Some((horizontal_overlap, 0.5 * (a.max_y - a.min_y + b.max_y - b.min_y)));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) {
        assert!((a - b).abs() <= 1.0e-12 * a.abs().max(b.abs()).max(1.0), "{a} != {b}");
    }

    #[test]
    fn refine_and_coarsen_preserve_area_integrals_of_both_time_levels() {
        let mut grid = AdaptiveSpatialGrid::new(3, 2, 2.0, 3).unwrap();
        let key = CellKey { level: 0, x: 1, y: 0 };
        grid.set_state(key, AdaptiveCell { previous: -0.75, current: 1.25 }).unwrap();
        let before = grid.integrated_state();
        let children = grid.refine(key).unwrap();
        assert_eq!(children.len(), 4);
        assert_eq!(grid.leaf_count(), 11);
        let refined = grid.integrated_state();
        close(refined.current_displacement, before.current_displacement);
        close(refined.previous_displacement, before.previous_displacement);

        let restored = grid.coarsen(key).unwrap();
        assert_eq!(restored, AdaptiveCell { previous: -0.75, current: 1.25 });
        let after = grid.integrated_state();
        close(after.current_displacement, before.current_displacement);
        close(after.previous_displacement, before.previous_displacement);
        assert_eq!(grid.leaf_count(), 6);
    }

    #[test]
    fn shared_faces_partition_a_coarse_face_without_gaps_or_double_counting() {
        let mut grid = AdaptiveSpatialGrid::new(2, 1, 1.0, 2).unwrap();
        grid.refine(CellKey { level: 0, x: 0, y: 0 }).unwrap();
        let faces = grid.shared_faces();
        let coarse = CellKey { level: 0, x: 1, y: 0 };
        let coarse_side: f64 = faces
            .iter()
            .filter(|face| face.first == coarse || face.second == coarse)
            .filter(|face| {
                let other = if face.first == coarse { face.second } else { face.first };
                other.level == 1
            })
            .map(|face| face.length)
            .sum();
        close(coarse_side, 1.0);
        assert!(faces.iter().all(|face| face.length > 0.0 && face.center_distance > 0.0));
    }

    #[test]
    fn refinement_enforces_two_to_one_neighbor_balance() {
        let mut grid = AdaptiveSpatialGrid::new(2, 1, 1.0, 3).unwrap();
        grid.refine(CellKey { level: 0, x: 0, y: 0 }).unwrap();
        let child = CellKey { level: 1, x: 1, y: 0 };
        assert_eq!(grid.refine(child), Err(AdaptiveGridError::UnbalancedRefinement));
    }

    #[test]
    fn physical_position_queries_resolve_the_finest_containing_leaf() {
        let mut grid = AdaptiveSpatialGrid::new(2, 2, 1.0, 2).unwrap();
        let key = CellKey { level: 0, x: 0, y: 0 };
        grid.set_state(key, AdaptiveCell { previous: 1.0, current: 2.0 }).unwrap();
        grid.refine(key).unwrap();
        let child = CellKey { level: 1, x: 1, y: 0 };
        grid.set_state(child, AdaptiveCell { previous: 3.0, current: 4.0 }).unwrap();
        assert_eq!(grid.sample(0.75, 0.25), Some(AdaptiveCell { previous: 3.0, current: 4.0 }));
        assert_eq!(grid.sample(-0.1, 0.25), None);
        assert_eq!(grid.sample(2.0, 0.25), None);
        assert_eq!(grid.sample(f64::NAN, 0.25), None);
    }

    #[test]
    fn coarsening_rejects_a_parent_that_would_break_two_to_one_balance() {
        let mut grid = AdaptiveSpatialGrid::new(2, 1, 1.0, 3).unwrap();
        let left_parent = CellKey { level: 0, x: 0, y: 0 };
        let right_parent = CellKey { level: 0, x: 1, y: 0 };
        grid.refine(left_parent).unwrap();
        grid.refine(right_parent).unwrap();
        // The right parent's leftmost child may refine to level 2 beside
        // level-1 leaves. Coarsening the left parent would then put a level-0
        // cell beside a level-2 leaf, violating 2:1 balance.
        let interface_child = CellKey { level: 1, x: 2, y: 0 };
        grid.refine(interface_child).unwrap();
        assert_eq!(
            grid.coarsen(left_parent),
            Err(AdaptiveGridError::UnbalancedRefinement)
        );
    }
}
