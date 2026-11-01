use std::time::{Duration, Instant};

use crate::state::Simulation;

const DEFAULT_MAX_TICKS: u64 = 10_000;
const DEFAULT_TARGET_BIRTHS: u64 = 3;
const REPORT_INTERVAL: u64 = 100;

pub(crate) fn run_from_args(args: impl Iterator<Item = String>) {
    let mut max_ticks = DEFAULT_MAX_TICKS;
    let mut target_births = DEFAULT_TARGET_BIRTHS;
    let mut diagnostics_path: Option<String> = None;
    let mut diagnostic_interval = REPORT_INTERVAL;
    let mut environment_chemistry_experiment = false;
    let mut seed = 42_u64;
    let mut max_ticks_provided = false;
    let mut duration_seconds: Option<u64> = None;

    let mut args = args.skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--environment-chemistry-experiment" => {
                environment_chemistry_experiment = true;
            }
            "--seed" => {
                seed = args
                    .next()
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(42);
            }
            "--ticks" => {
                max_ticks = args
                    .next()
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(DEFAULT_MAX_TICKS);
                max_ticks_provided = true;
            }
            "--duration-seconds" => {
                duration_seconds = args.next().and_then(|value| value.parse().ok());
            }
            "--births" => {
                target_births = args
                    .next()
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(DEFAULT_TARGET_BIRTHS);
            }
            "--diagnostics" => {
                diagnostics_path = args.next();
            }
            "--diagnostic-interval" => {
                diagnostic_interval = args
                    .next()
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(REPORT_INTERVAL);
            }
            _ => {}
        }
    }

    if environment_chemistry_experiment {
        let ticks = if max_ticks_provided {
            max_ticks
        } else {
            24_000
        };
        run_environment_chemistry_experiment(
            seed,
            ticks,
            diagnostics_path
                .as_deref()
                .unwrap_or("environment-chemistry-diagnostics.jsonl"),
            diagnostic_interval,
            duration_seconds.map(Duration::from_secs),
        );
        return;
    }

    let mut simulation = Simulation::new(42, 10.0);
    let initial_id = simulation
        .organisms
        .first()
        .map(|organism| organism.id.clone())
        .unwrap_or_default();

    println!(
        "HEADLESS SIMULATION START: seed=42 initial_organism={initial_id}          max_ticks={max_ticks} target_births={target_births}"
    );

    let mut diagnostics = diagnostics_path.map(|path| {
        crate::diagnostics::DiagnosticsRecorder::new(
            &path,
            diagnostic_interval,
            &initial_id,
            &mut simulation,
        )
        .unwrap_or_else(|error| panic!("failed to create diagnostics file {path}: {error}"))
    });

    report(&simulation, "initial");

    let mut previous_population = simulation.living_population();
    let mut previous_births = simulation.next_organism_id.saturating_sub(2);

    for _ in 0..max_ticks {
        if let Some(diagnostics) = diagnostics.as_mut() {
            diagnostics.observe_before(&mut simulation);
        }
        simulation.step();
        if let Some(diagnostics) = diagnostics.as_mut() {
            diagnostics
                .observe_after(&mut simulation)
                .expect("failed to write simulation diagnostics");
        }

        let births = simulation.next_organism_id.saturating_sub(2);
        if births != previous_births {
            println!(
                "BIRTH EVENT: tick={} total_births={} population={} new_organism_ids_starting_at={}",
                simulation.tick,
                births,
                simulation.living_population(),
                simulation.next_organism_id.saturating_sub(1)
            );
            previous_births = births;
        }

        if simulation.tick % REPORT_INTERVAL == 0
            || simulation.living_population() != previous_population
        {
            report(&simulation, "progress");
            previous_population = simulation.living_population();
        }

        if births >= target_births {
            report(&simulation, "target reached");
            if let Some(diagnostics) = diagnostics.as_mut() {
                diagnostics
                    .finish(&mut simulation)
                    .expect("failed to finalize simulation diagnostics");
            }
            println!(
                "HEADLESS SIMULATION END: observed {} completed birth events by tick {}.",
                births, simulation.tick
            );
            return;
        }

        // Continue the physical world even when no living organisms remain.
    }

    if let Some(diagnostics) = diagnostics.as_mut() {
        diagnostics
            .finish(&mut simulation)
            .expect("failed to finalize simulation diagnostics");
    }
    report(&simulation, "tick limit reached");
    println!(
        "HEADLESS SIMULATION END: tick limit {} reached with {} completed birth events.",
        max_ticks,
        simulation.next_organism_id.saturating_sub(2)
    );
}

fn report(simulation: &Simulation, label: &str) {
    let reproducing = simulation
        .organisms
        .iter()
        .filter(|organism| {
            organism.reproductive_construction.is_some()
                && matches!(
                    crate::cavity::analyze_genome_cavity(
                        &organism.structure,
                        &simulation.environment.catalog,
                    ),
                    Ok(Some(ref cavity)) if cavity.qualifies()
                )
        })
        .count();

    println!(
        "[{label}] tick={} population={} reproducing={}          decomposing_bodies={} active_transformations={} completed_births={}",
        simulation.tick,
        simulation.living_population(),
        reproducing,
        simulation.decomposing_bodies.len(),
        simulation.active_transformations.len(),
        simulation.next_organism_id.saturating_sub(2),
    );
}

fn run_environment_chemistry_experiment(
    seed: u64,
    max_ticks: u64,
    diagnostics_path: &str,
    diagnostic_interval: u64,
    duration_limit: Option<Duration>,
) {
    let fixture = crate::environment_chemistry_experiment::build(seed);
    let mut simulation = fixture.simulation;
    let mut diagnostics = crate::diagnostics::EnvironmentalChemistryDiagnosticsRecorder::new(
        diagnostics_path,
        diagnostic_interval,
        seed,
        &simulation,
        &fixture.pile_sizes,
    )
    .unwrap_or_else(|error| {
        panic!("failed to create environmental chemistry diagnostics {diagnostics_path}: {error}")
    });

    let initial_physical_resources = simulation
        .environment
        .field
        .cells
        .iter()
        .map(|cell| cell.physical_materials.len())
        .sum::<usize>();
    let initial_constituents = simulation
        .environment
        .field
        .cells
        .iter()
        .flat_map(|cell| cell.physical_materials.iter())
        .map(|material| material.material.parts.len())
        .sum::<usize>();
    println!(
        "ENVIRONMENTAL CHEMISTRY START: seed={seed} tick_limit={max_ticks} duration_limit_seconds={:?} organisms={} physical_resources={initial_physical_resources} constituents={initial_constituents} piles={:?} hotspot={:?} diagnostics={diagnostics_path}",
        duration_limit.map(|duration| duration.as_secs()),
        simulation.organisms.len(),
        fixture.pile_sizes,
        simulation.hotspot
    );

    let started_at = Instant::now();
    let mut completed_ticks = 0_u64;
    loop {
        let limit_reached = match duration_limit {
            Some(limit) => started_at.elapsed() >= limit,
            None => completed_ticks >= max_ticks,
        };
        if limit_reached {
            break;
        }

        simulation.step();
        completed_ticks += 1;
        diagnostics
            .observe(&simulation)
            .expect("failed to write environmental chemistry diagnostics");
        if simulation.tick % 1_000 == 0 {
            let counters = simulation.environmental_chemistry_diagnostics;
            let energy = simulation.energy_inventory_snapshot();
            println!(
                "[chemistry] tick={} COMBINE ready/scheduled/committed/rejected={}/{}/{}/{} BREAK={}/{}/{}/{} materials={} constituents={} resonance={:.6} usable={:.6} energy_residual={:?}",
                simulation.tick,
                counters.combine_ready_candidates,
                counters.combine_scheduled,
                counters.combine_committed,
                counters.combine_rejected,
                counters.break_ready_candidates,
                counters.break_scheduled,
                counters.break_committed,
                counters.break_rejected,
                simulation.environment.field.cells.iter()
                    .map(|cell| cell.physical_materials.len())
                    .sum::<usize>(),
                simulation.environment.field.cells.iter()
                    .flat_map(|cell| cell.physical_materials.iter())
                    .map(|material| material.material.parts.len())
                    .sum::<usize>(),
                simulation.environment.resonant_energy.total_energy().unwrap_or(f64::NAN),
                simulation.environment.chemistry_usable_energy,
                energy.residual()
            );
        }
    }

    let elapsed = started_at.elapsed();
    diagnostics
        .finish(&simulation)
        .expect("failed to finalize environmental chemistry diagnostics");
    let final_inventory = simulation.energy_inventory_snapshot();
    let stop_reason = if duration_limit.is_some() {
        "wall-clock duration reached"
    } else {
        "tick limit reached"
    };
    println!(
        "ENVIRONMENTAL CHEMISTRY END: reason={stop_reason} elapsed_seconds={:.3} tick={} physical_resources={} constituents={} last_tick_COMBINE_committed={} last_tick_BREAK_committed={} energy_residual={:?} diagnostics={diagnostics_path}",
        elapsed.as_secs_f64(),
        simulation.tick,
        simulation.environment.field.cells.iter()
            .map(|cell| cell.physical_materials.len())
            .sum::<usize>(),
        simulation.environment.field.cells.iter()
            .flat_map(|cell| cell.physical_materials.iter())
            .map(|material| material.material.parts.len())
            .sum::<usize>(),
        simulation.environmental_chemistry_diagnostics.combine_committed,
        simulation.environmental_chemistry_diagnostics.break_committed,
        final_inventory.residual()
    );
}
