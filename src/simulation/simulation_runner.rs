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
    let mut profile_chemistry = false;
    let mut seed = 42_u64;
    let mut max_ticks_provided = false;
    let mut duration_seconds: Option<u64> = None;

    let mut args = args.skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--environment-chemistry-experiment" => {
                environment_chemistry_experiment = true;
            }
            "--profile-chemistry" => {
                profile_chemistry = true;
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

    let mut simulation = Simulation::new(seed, 10.0);
    simulation
        .chemistry_library
        .set_performance_enabled(profile_chemistry);
    let initial_id = simulation
        .organisms
        .first()
        .map(|organism| organism.id.clone())
        .unwrap_or_default();

    println!(
        "HEADLESS SIMULATION START: seed={seed} initial_organism={initial_id}          max_ticks={max_ticks} target_births={target_births}"
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
    if simulation.chemistry_library.performance_enabled() {
        let performance = simulation.chemistry_library.performance_summary();
        let average_micros = |stage: &str| {
            performance
                .stages
                .get(stage)
                .map_or(0, |timing| timing.total_micros / timing.calls.max(1))
        };
        let maximum_micros = |stage: &str| {
            performance
                .stages
                .get(stage)
                .map_or(0, |timing| timing.max_micros)
        };
        let maximum_tick = |stage: &str| {
            performance
                .stages
                .get(stage)
                .map_or(0, |timing| timing.max_tick)
        };
        // Keep diagnostics comparable with pre-cache-rename experiment baselines.
        let average_micros_alias = |current: &str, legacy: &str| {
            performance
                .stages
                .get(current)
                .or_else(|| performance.stages.get(legacy))
                .map_or(0, |timing| timing.total_micros / timing.calls.max(1))
        };
        let maximum_micros_alias = |current: &str, legacy: &str| {
            performance
                .stages
                .get(current)
                .or_else(|| performance.stages.get(legacy))
                .map_or(0, |timing| timing.max_micros)
        };
        let counter_alias = |current: &str, legacy: &str| {
            performance
                .counters
                .get(current)
                .or_else(|| performance.counters.get(legacy))
                .copied()
                .unwrap_or(0)
        };
        println!(
            "[organism-chemistry-profile] cache_hits={} cache_misses={} break_ready={} break_scheduled={} break_committed={} break_rejected={} env_break_candidates={} restored_materials={} key_avg/max_us={}/{} lookup_avg/max_us={}/{} potential_avg/max_us={}/{} organism_break_discovery_avg/max_us={}/{} organism_break_settlement_avg/max_us={}/{} organism_env_break_selection_avg/max_us={}/{} organism_break_restoration_avg/max_us={}/{} worst_break_discovery_tick={} worst_break_settlement_tick={} worst_break_restoration_tick={}",
            counter_alias("static_potential_cache_hits", "cache_hits"),
            counter_alias("static_potential_cache_misses", "cache_misses"),
            performance.counters.get("organism_break_ready_candidates").copied().unwrap_or(0),
            performance.counters.get("organism_break_scheduled").copied().unwrap_or(0),
            performance.counters.get("organism_break_committed").copied().unwrap_or(0),
            performance.counters.get("organism_break_rejected").copied().unwrap_or(0),
            performance
                .counters
                .get("organism_environmental_break_candidates")
                .copied()
                .unwrap_or(0),
            performance.counters.get("organism_break_materials_restored").copied().unwrap_or(0),
            average_micros_alias("static_potential_key_construction", "chemistry_key_construction"),
            maximum_micros_alias("static_potential_key_construction", "chemistry_key_construction"),
            average_micros("chemistry_lookup"),
            maximum_micros("chemistry_lookup"),
            average_micros("static_potential_calculation"),
            maximum_micros("static_potential_calculation"),
            average_micros("organism_break_discovery"),
            maximum_micros("organism_break_discovery"),
            average_micros("organism_break_transaction_settlement"),
            maximum_micros("organism_break_transaction_settlement"),
            average_micros("organism_environmental_break_selection"),
            maximum_micros("organism_environmental_break_selection"),
            average_micros("organism_break_material_restoration"),
            maximum_micros("organism_break_material_restoration"),
            maximum_tick("organism_break_discovery"),
            maximum_tick("organism_break_transaction_settlement"),
            maximum_tick("organism_break_material_restoration"),
        );
        if label == "target reached" || label == "tick limit reached" {
            for (name, timing) in &performance.stages {
                println!(
                    "[chemistry-profile-stage] name={} calls={} work_items={} total_us={} avg_us={} max_us={} max_tick={}",
                    name,
                    timing.calls,
                    timing.work_items,
                    timing.total_micros,
                    timing.total_micros / timing.calls.max(1),
                    timing.max_micros,
                    timing.max_tick,
                );
            }
            for (name, value) in &performance.counters {
                println!("[chemistry-profile-counter] name={} value={}", name, value);
            }
        }
    }
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
    simulation.chemistry_library.set_performance_enabled(true);
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
            let performance = simulation.chemistry_library.performance_summary();
            let average_micros = |stage: &str| {
                performance
                    .stages
                    .get(stage)
                    .map_or(0, |timing| timing.total_micros / timing.calls.max(1))
            };
            let maximum_micros = |stage: &str| {
                performance
                    .stages
                    .get(stage)
                    .map_or(0, |timing| timing.max_micros)
            };
            let maximum_tick = |stage: &str| {
                performance
                    .stages
                    .get(stage)
                    .map_or(0, |timing| timing.max_tick)
            };
            let average_micros_alias = |current: &str, legacy: &str| {
                performance
                    .stages
                    .get(current)
                    .or_else(|| performance.stages.get(legacy))
                    .map_or(0, |timing| timing.total_micros / timing.calls.max(1))
            };
            let maximum_micros_alias = |current: &str, legacy: &str| {
                performance
                    .stages
                    .get(current)
                    .or_else(|| performance.stages.get(legacy))
                    .map_or(0, |timing| timing.max_micros)
            };
            let maximum_tick_alias = |current: &str, legacy: &str| {
                performance
                    .stages
                    .get(current)
                    .or_else(|| performance.stages.get(legacy))
                    .map_or(0, |timing| timing.max_tick)
            };
            let counter_alias = |current: &str, legacy: &str| {
                performance
                    .counters
                    .get(current)
                    .or_else(|| performance.counters.get(legacy))
                    .copied()
                    .unwrap_or(0)
            };
            println!(
                "[chemistry] tick={} COMBINE ready/scheduled/committed/rejected={}/{}/{}/{} ENV_BREAK={}/{}/{}/{} ORG_BREAK ready/scheduled/committed/rejected={}/{}/{}/{} discovery_us={} settlement_us={} organism_env_break_candidates={} organism_env_break_selection_us={} break_materials_restored={} break_restore_us={} materials={} constituents={} resonance={:.6} usable={:.6} energy_residual={:?}",
                simulation.tick,
                counters.combine_ready_candidates,
                counters.combine_scheduled,
                counters.combine_committed,
                counters.combine_rejected,
                counters.break_ready_candidates,
                counters.break_scheduled,
                counters.break_committed,
                counters.break_rejected,
                counters.organism_break_ready_candidates,
                counters.organism_break_scheduled,
                counters.organism_break_committed,
                counters.organism_break_rejected,
                counters.organism_break_discovery_elapsed_micros,
                counters.organism_break_settlement_elapsed_micros,
                counters.organism_environmental_break_candidates,
                counters.organism_environmental_break_selection_elapsed_micros,
                counters.organism_break_materials_restored,
                counters.organism_break_material_restoration_elapsed_micros,
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
            println!(
                "[chemistry-perf] static_potential_cache_hits={} static_potential_cache_misses={} static_potential_cache_inserts={} static_key_avg/max_us={}/{} lookup_avg/max_us={}/{} potential_avg/max_us={}/{} persistence_reload_avg_us={} persistence_append_avg_us={} manifest_avg_us={} worst_static_key_tick={} worst_lookup_tick={} worst_potential_tick={}",
                counter_alias("static_potential_cache_hits", "cache_hits"),
                counter_alias("static_potential_cache_misses", "cache_misses"),
                counter_alias("static_potential_cache_inserts", "cache_inserts"),
                average_micros_alias("static_potential_key_construction", "chemistry_key_construction"),
                maximum_micros_alias("static_potential_key_construction", "chemistry_key_construction"),
                average_micros("chemistry_lookup"),
                maximum_micros("chemistry_lookup"),
                average_micros("static_potential_calculation"),
                maximum_micros("static_potential_calculation"),
                average_micros("persistence_reload_read"),
                average_micros("persistence_append_sync"),
                average_micros("persistence_manifest_update"),
                maximum_tick_alias("static_potential_key_construction", "chemistry_key_construction"),
                maximum_tick("chemistry_lookup"),
                maximum_tick("static_potential_calculation"),
            );
            println!(
                "[chemistry-stages] spatial_query_avg/max_us={}/{} anchor_cells={} anchor_materials={} target_cells={} material_restore_avg/max_us={}/{} geometry_avg/max_us={}/{} interface_resolution_avg/max_us={}/{} contact_eval_avg/max_us={}/{} contact_generation_avg/max_us={}/{} candidate_evaluation_avg/max_us={}/{} constituent_pairs={} contact_candidates={} pair_inspections={} chemistry_accum_avg/max_us={}/{} cleanup_avg/max_us={}/{} schedule_avg/max_us={}/{} transaction_avg/max_us={}/{} worst_spatial_tick={} worst_restore_tick={} worst_geometry_tick={} worst_interface_tick={} worst_contact_tick={} worst_generation_tick={} worst_candidate_eval_tick={} worst_accumulation_tick={} worst_transaction_tick={}",
                average_micros("environmental_combine_spatial_cell_query"),
                maximum_micros("environmental_combine_spatial_cell_query"),
                performance
                .counters
                .get("environmental_combine_anchor_cells")
                .copied()
                .unwrap_or(0),
                performance
                .counters
                .get("environmental_combine_anchor_materials")
                .copied()
                .unwrap_or(0),
                performance
                .counters
                .get("environmental_combine_target_cells")
                .copied()
                .unwrap_or(0),
                average_micros("environmental_combine_material_restoration"),
                maximum_micros("environmental_combine_material_restoration"),
                average_micros("environmental_combine_geometry_validation"),
                maximum_micros("environmental_combine_geometry_validation"),
                average_micros("environmental_combine_interface_resolution"),
                maximum_micros("environmental_combine_interface_resolution"),
                average_micros("environmental_combine_contact_generation_evaluation"),
                maximum_micros("environmental_combine_contact_generation_evaluation"),
                average_micros("environmental_combine_contact_generation"),
                maximum_micros("environmental_combine_contact_generation"),
                average_micros("environmental_combine_candidate_evaluation"),
                maximum_micros("environmental_combine_candidate_evaluation"),
                performance
                .counters
                .get("environmental_combine_constituent_pairs")
                .copied()
                .unwrap_or(0),
                performance
                .counters
                .get("environmental_combine_contact_candidates")
                .copied()
                .unwrap_or(0),
                performance
                .counters
                .get("environmental_combine_constituent_pairs_inspected")
                .copied()
                .unwrap_or(0),
                average_micros("environmental_combine_chemistry_accumulation"),
                maximum_micros("environmental_combine_chemistry_accumulation"),
                average_micros("environmental_combine_accumulation_cleanup"),
                maximum_micros("environmental_combine_accumulation_cleanup"),
                average_micros("environmental_combine_scheduling"),
                maximum_micros("environmental_combine_scheduling"),
                average_micros("environmental_combine_transaction_settlement"),
                maximum_micros("environmental_combine_transaction_settlement"),
                maximum_tick("environmental_combine_spatial_cell_query"),
                maximum_tick("environmental_combine_material_restoration"),
                maximum_tick("environmental_combine_geometry_validation"),
                maximum_tick("environmental_combine_interface_resolution"),
                maximum_tick("environmental_combine_contact_generation_evaluation"),
                maximum_tick("environmental_combine_contact_generation"),
                maximum_tick("environmental_combine_candidate_evaluation"),
                maximum_tick("environmental_combine_chemistry_accumulation"),
                maximum_tick("environmental_combine_transaction_settlement"),
            );
            println!(
                "[chemistry-persistence] lock_wait_avg/max_us={}/{} lock_hold_avg/max_us={}/{} reload_avg/max_us={}/{} serialize_avg/max_us={}/{} append_sync_avg/max_us={}/{} manifest_avg/max_us={}/{} worst_reload_tick={} worst_append_tick={}",
                average_micros("persistence_lock_wait"),
                maximum_micros("persistence_lock_wait"),
                average_micros("persistence_lock_hold"),
                maximum_micros("persistence_lock_hold"),
                average_micros("persistence_reload_read"),
                maximum_micros("persistence_reload_read"),
                average_micros("persistence_serialization"),
                maximum_micros("persistence_serialization"),
                average_micros("persistence_append_sync"),
                maximum_micros("persistence_append_sync"),
                average_micros("persistence_manifest_update"),
                maximum_micros("persistence_manifest_update"),
                maximum_tick("persistence_reload_read"),
                maximum_tick("persistence_append_sync"),
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
