//! Test-only uniform-grid storage-layout microbenchmark.
//!
//! All three layouts use the same fixed-zero-boundary leapfrog stencil and
//! deterministic row-major update order. This isolates storage/iteration cost;
//! it is not a replacement for the adaptive production operator.

use super::*;
use std::collections::BTreeMap;
use std::mem::size_of;
use std::time::Instant;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct State {
    previous: f64,
    current: f64,
}

struct Dense {
    width: usize,
    height: usize,
    cells: Vec<State>,
}

struct OrderedMap {
    width: usize,
    height: usize,
    cells: BTreeMap<usize, State>,
}

struct ActiveRegion {
    world_width: usize,
    world_height: usize,
    x0: usize,
    y0: usize,
    width: usize,
    height: usize,
    cells: Vec<State>,
}

fn profile(width: usize, height: usize, x: usize, y: usize) -> State {
    let dx = x as f64 - (width as f64 - 1.0) * 0.5;
    let dy = y as f64 - (height as f64 - 1.0) * 0.5;
    let radius_squared = dx * dx + dy * dy;
    if radius_squared >= 25.0 {
        return State::default();
    }
    let envelope = (-radius_squared / 8.0).exp() * (1.0 - radius_squared / 25.0).powi(2);
    State {
        previous: 0.1 * envelope,
        current: envelope,
    }
}

impl Dense {
    fn new(width: usize, height: usize, localized: bool) -> Self {
        let mut cells = vec![State::default(); width * height];
        for y in 0..height {
            for x in 0..width {
                cells[y * width + x] = if localized {
                    profile(width, height, x, y)
                } else {
                    State::default()
                };
            }
        }
        Self {
            width,
            height,
            cells,
        }
    }

    fn advance(&mut self, substeps: usize) {
        for _ in 0..substeps {
            let mut next = vec![0.0; self.cells.len()];
            for y in 1..self.height - 1 {
                for x in 1..self.width - 1 {
                    let i = y * self.width + x;
                    let laplacian = self.cells[i - 1].current
                        + self.cells[i + 1].current
                        + self.cells[i - self.width].current
                        + self.cells[i + self.width].current
                        - 4.0 * self.cells[i].current;
                    next[i] =
                        2.0 * self.cells[i].current - self.cells[i].previous + 0.25 * laplacian;
                }
            }
            for (cell, value) in self.cells.iter_mut().zip(next) {
                cell.previous = cell.current;
                cell.current = value;
            }
        }
    }

    fn bytes(&self) -> usize {
        self.cells.capacity() * size_of::<State>()
    }
}

impl OrderedMap {
    fn new(width: usize, height: usize, localized: bool) -> Self {
        let mut cells = BTreeMap::new();
        for y in 0..height {
            for x in 0..width {
                cells.insert(
                    y * width + x,
                    if localized {
                        profile(width, height, x, y)
                    } else {
                        State::default()
                    },
                );
            }
        }
        Self {
            width,
            height,
            cells,
        }
    }

    fn current(&self, index: usize) -> f64 {
        self.cells.get(&index).map_or(0.0, |cell| cell.current)
    }

    fn advance(&mut self, substeps: usize) {
        for _ in 0..substeps {
            let mut next = vec![0.0; self.width * self.height];
            for y in 1..self.height - 1 {
                for x in 1..self.width - 1 {
                    let i = y * self.width + x;
                    let laplacian = self.current(i - 1)
                        + self.current(i + 1)
                        + self.current(i - self.width)
                        + self.current(i + self.width)
                        - 4.0 * self.current(i);
                    next[i] = 2.0 * self.current(i)
                        - self.cells.get(&i).map_or(0.0, |cell| cell.previous)
                        + 0.25 * laplacian;
                }
            }
            for (index, cell) in &mut self.cells {
                cell.previous = cell.current;
                cell.current = next[*index];
            }
        }
    }

    // This is a lower bound: BTreeMap node links, color/allocator metadata,
    // and allocator rounding are intentionally not guessed.
    fn bytes_lower_bound(&self) -> usize {
        self.cells.len() * (size_of::<usize>() + size_of::<State>())
    }
}

impl ActiveRegion {
    fn new(world_width: usize, world_height: usize, localized: bool) -> Self {
        if !localized {
            return Self {
                world_width,
                world_height,
                x0: 1,
                y0: 1,
                width: 0,
                height: 0,
                cells: Vec::new(),
            };
        }
        let center_x = (world_width - 1) / 2;
        let center_y = (world_height - 1) / 2;
        let radius = 5usize;
        let x0 = center_x.saturating_sub(radius).max(1);
        let y0 = center_y.saturating_sub(radius).max(1);
        let x1 = (center_x + radius + 1).min(world_width - 1);
        let y1 = (center_y + radius + 1).min(world_height - 1);
        let width = x1 - x0;
        let height = y1 - y0;
        let mut cells = vec![State::default(); width * height];
        for y in y0..y1 {
            for x in x0..x1 {
                cells[(y - y0) * width + (x - x0)] = profile(world_width, world_height, x, y);
            }
        }
        Self {
            world_width,
            world_height,
            x0,
            y0,
            width,
            height,
            cells,
        }
    }

    fn get(&self, x: usize, y: usize) -> State {
        if x < self.x0 || y < self.y0 || x >= self.x0 + self.width || y >= self.y0 + self.height {
            State::default()
        } else {
            self.cells[(y - self.y0) * self.width + (x - self.x0)]
        }
    }

    fn advance(&mut self, substeps: usize) {
        for _ in 0..substeps {
            if self.width == 0 || self.height == 0 {
                continue;
            }
            // The exact stencil support expands by at most one cell per substep.
            // No amplitude threshold is used; cells are retained even when tiny.
            let new_x0 = self.x0.saturating_sub(1).max(1);
            let new_y0 = self.y0.saturating_sub(1).max(1);
            let new_x1 = (self.x0 + self.width + 1).min(self.world_width - 1);
            let new_y1 = (self.y0 + self.height + 1).min(self.world_height - 1);
            let new_width = new_x1 - new_x0;
            let new_height = new_y1 - new_y0;
            let mut next = vec![0.0; new_width * new_height];
            let mut shifted = vec![State::default(); new_width * new_height];

            for y in new_y0..new_y1 {
                for x in new_x0..new_x1 {
                    let i = (y - new_y0) * new_width + (x - new_x0);
                    let center = self.get(x, y);
                    let laplacian = self.get(x - 1, y).current
                        + self.get(x + 1, y).current
                        + self.get(x, y - 1).current
                        + self.get(x, y + 1).current
                        - 4.0 * center.current;
                    next[i] = 2.0 * center.current - center.previous + 0.25 * laplacian;
                    shifted[i] = State {
                        previous: center.current,
                        current: 0.0,
                    };
                }
            }
            for (cell, value) in shifted.iter_mut().zip(next) {
                cell.current = value;
            }
            self.x0 = new_x0;
            self.y0 = new_y0;
            self.width = new_width;
            self.height = new_height;
            self.cells = shifted;
        }
    }

    fn bytes(&self) -> usize {
        self.cells.capacity() * size_of::<State>()
    }
}

fn max_difference(dense: &Dense, map: &OrderedMap, region: &ActiveRegion) -> f64 {
    let mut maximum: f64 = 0.0;
    for y in 0..dense.height {
        for x in 0..dense.width {
            let index = y * dense.width + x;
            let dense_cell = dense.cells[index];
            let map_cell = map.cells[&index];
            let region_cell = region.get(x, y);
            for (a, b) in [
                (dense_cell.previous, map_cell.previous),
                (dense_cell.current, map_cell.current),
                (dense_cell.previous, region_cell.previous),
                (dense_cell.current, region_cell.current),
            ] {
                maximum = maximum.max((a - b).abs());
            }
        }
    }
    maximum
}

#[test]
fn dense_uniform_kernel_matches_production_adaptive_uniform_operator() {
    const SIZE: usize = 32;
    const TICKS: u64 = 20;

    let mut grid = AdaptiveSpatialGrid::new(SIZE as u32, SIZE as u32, 1.0, 2).unwrap();
    for y in 0..SIZE {
        for x in 0..SIZE {
            let state = profile(SIZE, SIZE, x, y);
            grid.set_state(
                CellKey {
                    level: 0,
                    x: x as u32,
                    y: y as u32,
                },
                AdaptiveCell {
                    previous: state.previous,
                    current: state.current,
                },
            )
            .unwrap();
        }
    }

    let mut production_operator = AdaptiveWaveField::new(grid, 1.0, 1.0).unwrap();
    let initial_energy = production_operator.energy();
    assert_eq!(production_operator.substeps_per_tick(), 2);
    production_operator.advance_ticks(TICKS).unwrap();

    let mut dense = Dense::new(SIZE, SIZE, true);
    dense.advance(TICKS as usize * production_operator.substeps_per_tick());

    let mut maximum_difference: f64 = 0.0;
    for (key, state) in production_operator.grid().leaves() {
        let index = key.y as usize * SIZE + key.x as usize;
        maximum_difference = maximum_difference
            .max((state.previous - dense.cells[index].previous).abs())
            .max((state.current - dense.cells[index].current).abs());
    }
    assert!(
        maximum_difference <= 1.0e-12,
        "dense uniform stencil differs from adaptive production operator by {maximum_difference:e}"
    );
    let final_energy = production_operator.energy();
    assert!(
        (final_energy - initial_energy).abs() <= 1.0e-10 * initial_energy.abs().max(1.0),
        "uniform production operator energy drift exceeded tolerance"
    );
}

#[test]
#[ignore = "manual uniform-grid storage-layout benchmark"]
fn benchmark_fixed_domain_domain_scaling_storage_layouts() {
    use std::hint::black_box;

    const SIZES: [usize; 4] = [32, 64, 128, 250];
    const LOGICAL_TICKS: usize = 20;
    const SUBSTEPS_PER_TICK: usize = 2;
    const SUBSTEPS: usize = LOGICAL_TICKS * SUBSTEPS_PER_TICK;

    eprintln!(
        "case,width,height,layout,build_ms,advance_ms,stored_cells,state_payload_bytes_estimate,max_abs_difference"
    );
    for size in SIZES {
        for localized in [false, true] {
            let case = if localized { "localized_wave" } else { "empty" };
            let dense_build = Instant::now();
            let mut dense = Dense::new(size, size, localized);
            let dense_build_elapsed = dense_build.elapsed();
            let dense_start = Instant::now();
            dense.advance(SUBSTEPS);
            let dense_elapsed = dense_start.elapsed();

            let map_build = Instant::now();
            let mut map = OrderedMap::new(size, size, localized);
            let map_build_elapsed = map_build.elapsed();
            let map_start = Instant::now();
            map.advance(SUBSTEPS);
            let map_elapsed = map_start.elapsed();

            let region_build = Instant::now();
            let mut region = ActiveRegion::new(size, size, localized);
            let region_build_elapsed = region_build.elapsed();
            let region_start = Instant::now();
            region.advance(SUBSTEPS);
            let region_elapsed = region_start.elapsed();

            let difference = max_difference(&dense, &map, &region);
            assert!(
                difference <= 1.0e-12,
                "storage layouts diverged by {difference:e}"
            );

            eprintln!(
                "{case},{size},{size},dense_vec,{:.3},{:.3},{},{},{difference:.6e}",
                dense_build_elapsed.as_secs_f64() * 1000.0,
                dense_elapsed.as_secs_f64() * 1000.0,
                dense.cells.len(),
                dense.bytes(),
            );
            eprintln!(
                "{case},{size},{size},ordered_map,{:.3},{:.3},{},{},{difference:.6e}",
                map_build_elapsed.as_secs_f64() * 1000.0,
                map_elapsed.as_secs_f64() * 1000.0,
                map.cells.len(),
                map.bytes_lower_bound(),
            );
            eprintln!(
                "{case},{size},{size},active_region,{:.3},{:.3},{},{},{difference:.6e}",
                region_build_elapsed.as_secs_f64() * 1000.0,
                region_elapsed.as_secs_f64() * 1000.0,
                region.cells.len(),
                region.bytes(),
            );
            black_box((
                dense.cells[size * size / 2],
                map.cells[&(size * size / 2)],
                region.get(size / 2, size / 2),
            ));
        }
    }
}
