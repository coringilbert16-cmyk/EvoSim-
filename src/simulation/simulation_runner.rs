use crate::state::Simulation;
use std::time::{Duration, Instant};

const DEFAULT_MAX_TICKS: u64 = 10_000;
const REPORT_INTERVAL: u64 = 100;

pub(crate) fn run_from_args(args: impl Iterator<Item = String>) {
    let mut max_ticks = DEFAULT_MAX_TICKS;
    let mut diagnostics_path: Option<String> = None;
    let mut diagnostic_interval = REPORT_INTERVAL;
    let mut environment_chemistry_experiment = false;
    let mut plan004_resource_feed = false;
    let mut seed = 42_u64;
    let mut max_ticks_provided = false;
    let mut duration_seconds: Option<u64> = None;

    let mut args = args.skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--environment-chemistry-experiment" => {
                environment_chemistry_experiment = true;
            }
            "--plan004-resource-feed" => {
                plan004_resource_feed = true;
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
            "--seconds" => {
                duration_seconds = args.next().and_then(|value| value.parse().ok());
            }
            "--births" => {
                let _ = args.next();
                eprintln!("--births is disabled while organism rules are dormant.");
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
        let ticks = if max_ticks_provided || duration_seconds.is_some() {
            max_ticks
        } else {
            24_000
        };
        assert!(
            !plan004_resource_feed || duration_seconds.is_some_and(|seconds| seconds >= 3_600),
            "PLAN-004 resource feed requires a real-time run of at least 3,600 seconds"
        );
        run_environment_chemistry_experiment(
            seed,
            ticks,
            duration_seconds,
            plan004_resource_feed,
            diagnostics_path
                .as_deref()
                .unwrap_or("environment-chemistry-diagnostics.jsonl"),
            diagnostic_interval,
        );
        return;
    }

    let mut simulation = Simulation::new(seed, 10.0);
    simulation.record_environmental_molecular_history = true;
    let mut diagnostics = diagnostics_path.map(|path| {
        crate::diagnostics::EnvironmentalChemistryDiagnosticsRecorder::new(
            &path,
            diagnostic_interval,
            seed,
            duration_seconds,
            Some(max_ticks),
            &simulation,
            &[],
        )
        .unwrap_or_else(|error| {
            panic!("failed to create chemistry diagnostics file {path}: {error}")
        })
    });

    report(&simulation, "initial");

    for _ in 0..max_ticks {
        simulation.step();
        if let Some(diagnostics) = diagnostics.as_mut() {
            diagnostics
                .observe(&simulation)
                .expect("failed to write environmental chemistry diagnostics");
        }

        if simulation.tick % REPORT_INTERVAL == 0 {
            report(&simulation, "progress");
        }
    }

    if let Some(diagnostics) = diagnostics.as_mut() {
        diagnostics
            .finish(&simulation)
            .expect("failed to finalize environmental chemistry diagnostics");
    }
    report(&simulation, "tick limit reached");
    println!(
        "HEADLESS CHEMISTRY SIMULATION END: tick limit {} reached.",
        max_ticks
    );
}

fn report(simulation: &Simulation, label: &str) {
    let physical_material_count = simulation
        .environment
        .field
        .cells
        .iter()
        .map(|cell| cell.physical_materials.len())
        .sum::<usize>();
    let diagnostics = simulation.environmental_chemistry_diagnostics;

    println!(
        "[{label}] tick={} environmental_objects={} environmental_constituents={} environment_resonance={:.6} BREAK={} COMBINE={} DISPLACEMENT={} dormant_organism_holders={}",
        simulation.tick,
        physical_material_count,
        simulation.environment.field.total_amount(),
        simulation.environment.resonant_energy.total_energy().unwrap_or(f64::NAN),
        diagnostics.break_committed,
        diagnostics.combine_committed,
        diagnostics.displacement_committed,
        simulation.organisms.len(),
    );
}

fn run_environment_chemistry_experiment(
    seed: u64,
    max_ticks: u64,
    duration_seconds: Option<u64>,
    plan004_resource_feed: bool,
    diagnostics_path: &str,
    diagnostic_interval: u64,
) {
    let fixture = crate::environment_chemistry_experiment::build(seed);
    let mut simulation = fixture.simulation;
    let feed_reference_positions = plan004_resource_feed
        .then(|| crate::environment_resource_feed::initial_free_resource_positions(&simulation));
    simulation.record_local_resonance_diagnostics = true;
    let planned_ticks = duration_seconds.is_none().then_some(max_ticks);
    let mut diagnostics = crate::diagnostics::EnvironmentalChemistryDiagnosticsRecorder::new(
        diagnostics_path,
        diagnostic_interval,
        seed,
        duration_seconds,
        planned_ticks,
        &simulation,
        &fixture.pile_sizes,
    )
    .unwrap_or_else(|error| {
        panic!("failed to create environmental chemistry diagnostics {diagnostics_path}: {error}")
    });

    let (physical_objects, independent_resources, molecules, constituents) = simulation
        .environment
        .field
        .cells
        .iter()
        .flat_map(|cell| cell.physical_materials.iter())
        .fold(
            (0_usize, 0_usize, 0_usize, 0_usize),
            |mut counts, material| {
                counts.0 += 1;
                counts.3 += material.material.parts.len();
                if material.material.parts.len() == 1 {
                    counts.1 += 1;
                } else {
                    counts.2 += 1;
                }
                counts
            },
        );

    println!(
        "ENVIRONMENTAL CHEMISTRY START: seed={seed} duration_seconds={:?} tick_limit={:?} organisms={} independent_resources={independent_resources} molecules={molecules} constituents={constituents} physical_objects={physical_objects} resource_feed={plan004_resource_feed} piles={:?} hotspot={:?} diagnostics={diagnostics_path}",
        duration_seconds,
        planned_ticks,
        simulation.organisms.len(),
        fixture.pile_sizes,
        simulation.hotspot
    );

    diagnostics
        .record_resource_feed_plan(plan004_resource_feed)
        .expect("failed to record PLAN-004 experiment condition");
    println!(
        "PLAN-004 RESOURCE FEED: enabled={plan004_resource_feed} pulse_count={} pulse_size={} total_resources={} first_pulse_seconds={} interval_seconds={} feed_elements_per_pulse=12_each_of_six",
        crate::environment_resource_feed::PLAN004_RESOURCE_FEED_PULSE_COUNT,
        crate::environment_resource_feed::PLAN004_RESOURCE_FEED_PULSE_SIZE,
        crate::environment_resource_feed::PLAN004_RESOURCE_FEED_TOTAL,
        crate::environment_resource_feed::PLAN004_RESOURCE_FEED_FIRST_PULSE_SECONDS,
        crate::environment_resource_feed::PLAN004_RESOURCE_FEED_PULSE_INTERVAL_SECONDS
    );

    let started_at = Instant::now();
    let mut next_feed_pulse = 0_usize;
    while duration_seconds.map_or(simulation.tick < max_ticks, |seconds| {
        started_at.elapsed() < Duration::from_secs(seconds)
    }) {
        if plan004_resource_feed {
            let elapsed = started_at.elapsed();
            while next_feed_pulse
                < crate::environment_resource_feed::PLAN004_RESOURCE_FEED_PULSE_COUNT
            {
                let scheduled_seconds =
                    crate::environment_resource_feed::PLAN004_RESOURCE_FEED_FIRST_PULSE_SECONDS
                        + next_feed_pulse as u64
                            * crate::environment_resource_feed::PLAN004_RESOURCE_FEED_PULSE_INTERVAL_SECONDS;
                if elapsed < Duration::from_secs(scheduled_seconds) {
                    break;
                }
                let particles = crate::environment_resource_feed::inject_plan004_resource_pulse(
                    &mut simulation,
                    seed,
                    next_feed_pulse,
                    feed_reference_positions
                        .as_deref()
                        .expect("feed arm retains its tick-zero resource placement reference"),
                );
                let elapsed_wall_seconds = started_at.elapsed().as_secs_f64();
                diagnostics
                    .record_resource_feed(
                        simulation.tick,
                        elapsed_wall_seconds,
                        next_feed_pulse + 1,
                        &particles,
                    )
                    .expect("failed to record PLAN-004 resource feed");
                println!(
                    "PLAN-004 RESOURCE FEED PULSE: pulse={} tick={} elapsed_wall_seconds={:.3} resources={}",
                    next_feed_pulse + 1,
                    simulation.tick,
                    elapsed_wall_seconds,
                    particles.len()
                );
                next_feed_pulse += 1;
            }
        }
        simulation.step();
        diagnostics
            .observe(&simulation)
            .expect("failed to write environmental chemistry diagnostics");
        if simulation.tick % 100 == 0 || simulation.tick == max_ticks {
            let counters = simulation.environmental_chemistry_diagnostics;
            let discovery = counters.displacement_discovery;
            let energy = simulation.energy_inventory_snapshot();
            println!(
                "[chemistry] tick={} displacement ready/scheduled/committed/rejected={}/{}/{}/{} scan anchors/bonded={}/{} targets cells/materials={}/{} contacts searches/found={}/{} activation updates/met/below/status_unavailable={}/{}/{}/{} trial_rejected={} max_accumulated_reaction={:.4} work_remaining={} COMBINE={}/{}/{}/{} BREAK={}/{}/{}/{} scan/candidates/checked={}/{}/{} restore_ok/fail={}/{} map/property/position/endpoint/interface/chemistry_reject={}/{}/{}/{}/{}/{} activation/below/met={}/{}/{} preserved/dissipated={}/{} remaining={} materials={} constituents={} resonance={:.6} usable={:.6} energy_residual={:?}",
                simulation.tick,
                counters.displacement_ready_candidates,
                counters.displacement_scheduled,
                counters.displacement_committed,
                counters.displacement_rejected,
                discovery.anchor_materials_inspected,
                discovery.bonded_anchor_materials,
                discovery.target_cells_inspected,
                discovery.target_materials_inspected,
                discovery.replacement_contact_searches,
                discovery.replacement_contacts_found,
                discovery.activation_updates,
                discovery.activation_threshold_met,
                discovery.activation_below_threshold,
                discovery.activation_status_unavailable,
                discovery.trial_build_rejections_after_activation,
                discovery.max_accumulated_reaction,
                counters.displacement_discovery_work_remaining,
                counters.combine_ready_candidates,
                counters.combine_scheduled,
                counters.combine_committed,
                counters.combine_rejected,
                counters.break_ready_candidates,
                counters.break_scheduled,
                counters.break_committed,
                counters.break_rejected,
                counters.break_discovery.scan_entries_inspected,
                counters.break_discovery.bond_candidates_inspected,
                counters.break_discovery.bond_candidates_checked,
                counters.break_discovery.material_restorations_succeeded,
                counters.break_discovery.material_restoration_rejections,
                counters.break_discovery.candidate_mapping_rejections,
                counters.break_discovery.candidate_property_rejections,
                counters.break_discovery.candidate_position_rejections,
                counters.break_discovery.candidate_endpoint_rejections,
                counters.break_discovery.candidate_interface_rejections,
                counters.break_discovery.candidate_chemistry_rejections,
                counters.break_discovery.activation_updates,
                counters.break_discovery.activation_below_threshold,
                counters.break_discovery.activation_threshold_met,
                counters.break_discovery.progress_preserved_unsearched,
                counters.break_discovery.progress_dissipated_checked,
                counters.break_discovery_work_remaining,
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

    assert!(
        !plan004_resource_feed
            || next_feed_pulse
                == crate::environment_resource_feed::PLAN004_RESOURCE_FEED_PULSE_COUNT,
        "all six PLAN-004 resource feed pulses must occur during the one-hour run"
    );
    println!(
        "PLAN-004 RESOURCE FEED COMPLETE: enabled={plan004_resource_feed} pulses_delivered={next_feed_pulse} total_resources_delivered={}",
        next_feed_pulse * crate::environment_resource_feed::PLAN004_RESOURCE_FEED_PULSE_SIZE
    );
    diagnostics
        .finish(&simulation)
        .expect("failed to finalize environmental chemistry diagnostics");
    let final_inventory = simulation.energy_inventory_snapshot();
    crate::property_profile::report();
    println!(
        "ENVIRONMENTAL CHEMISTRY END: tick={} elapsed_wall_seconds={:.3} physical_resources={} constituents={} last_tick_COMBINE_committed={} last_tick_BREAK_committed={} energy_residual={:?} diagnostics={diagnostics_path}",
        simulation.tick,
        started_at.elapsed().as_secs_f64(),
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
