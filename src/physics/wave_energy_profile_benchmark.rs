//! Manual release-mode profile for the adaptive wave solver's global passes.
//!
//! Run with:
//! cargo test --release --locked --lib benchmark_wave_energy_accounting_cost -- --ignored --nocapture
//!
//! The three initial conditions distinguish exact-zero, compact-support, and
//! Gaussian-tail fields. The current solver is expected to visit the full
//! sparse map for all three; this benchmark establishes the baseline before
//! any active-region implementation is considered.

use super::{
    is_computational_boundary, AdaptiveCell, AdaptiveSpatialGrid, AdaptiveWaveField, CellKey,
};

#[test]
#[ignore = "manual wave energy accounting profile"]
fn benchmark_wave_energy_accounting_cost() {
    use std::hint::black_box;
    use std::time::Instant;

    const SIZES: [u32; 4] = [32, 64, 128, 250];
    const TICKS: u64 = 20;
    const COMPACT_RADIUS: f64 = 6.0;

    eprintln!(
        "case,width,height,leaves,substeps_per_tick,energy_passes,energy_pass_ms,integrated_state_passes,integrated_state_ms,advance_ms"
    );

    for size in SIZES {
        for case in ["exact_zero", "compact_support", "gaussian_tails"] {
            let mut grid = AdaptiveSpatialGrid::new(size, size, 1.0, 2)
                .expect("profile grid dimensions are valid");
            let center = (size as f64 - 1.0) * 0.5;

            if case != "exact_zero" {
                for y in 0..size {
                    for x in 0..size {
                        let key = CellKey { level: 0, x, y };
                        if is_computational_boundary(&grid, key) {
                            continue;
                        }
                        let dx = x as f64 - center;
                        let dy = y as f64 - center;
                        let radius_squared = dx * dx + dy * dy;
                        if case == "compact_support" && radius_squared > COMPACT_RADIUS.powi(2) {
                            continue;
                        }
                        let envelope = (-radius_squared / 18.0).exp();
                        let phase = 0.7 * dx;
                        grid.set_state(
                            key,
                            AdaptiveCell {
                                previous: envelope * (phase + 0.08).cos(),
                                current: envelope * phase.cos(),
                            },
                        )
                        .expect("profile state must be finite");
                    }
                }
            }

            let mut field = AdaptiveWaveField::new(grid, 1.0, 1.0)
                .expect("profile solver must satisfy its CFL bound");
            let leaves = field.grid().leaf_count();
            let substeps = field.substeps_per_tick();
            let energy_passes = 2 * substeps as u64 * TICKS + TICKS;
            let integrated_state_passes = 2 * TICKS;

            let energy_start = Instant::now();
            for _ in 0..energy_passes {
                black_box(field.energy());
            }
            let energy_elapsed = energy_start.elapsed();

            let integral_start = Instant::now();
            for _ in 0..integrated_state_passes {
                black_box(field.grid().integrated_state());
            }
            let integral_elapsed = integral_start.elapsed();

            let advance_start = Instant::now();
            field
                .advance_ticks(TICKS)
                .expect("profile ticks must advance");
            let advance_elapsed = advance_start.elapsed();

            eprintln!(
                "{case},{size},{size},{leaves},{substeps},{energy_passes},{:.3},{integrated_state_passes},{:.3},{:.3}",
                energy_elapsed.as_secs_f64() * 1000.0,
                integral_elapsed.as_secs_f64() * 1000.0,
                advance_elapsed.as_secs_f64() * 1000.0,
            );
            black_box(field.energy());
        }
    }
}
