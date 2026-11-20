use crate::state::Simulation;
use std::time::{Duration, Instant};

const DEFAULT_MAX_TICKS: u64 = 10_000;
const REPORT_INTERVAL: u64 = 100;

pub(crate) fn run_from_args(args: impl Iterator<Item = String>) {
    let mut max_ticks = DEFAULT_MAX_TICKS;
    let mut diagnostics_path: Option<String> = None;
    let mut diagnostic_interval = REPORT_INTERVAL;
    let mut environment_chemistry_experiment = false;
    let mut seed = 42_u64;
    let mut max_ticks_provided = false;
    let mut duration_seconds: Option<u64> = None;
    let mut matched_catalysis_config_path: Option<String> = None;
    let mut matched_catalysis_condition: Option<String> = None;
    let mut matched_catalysis_preflight = false;

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
            "--matched-catalysis-preflight" => {
                matched_catalysis_preflight = true;
            }
            "--matched-catalysis-config" => {
                matched_catalysis_config_path = args.next();
            }
            "--matched-catalysis-condition" => {
                matched_catalysis_condition = args.next();
            }
            _ => {}
        }
    }

    if matched_catalysis_preflight {
        let config_path = matched_catalysis_config_path
            .as_deref()
            .expect("--matched-catalysis-preflight requires --matched-catalysis-config");
        let contents = std::fs::read_to_string(config_path).unwrap_or_else(|error| {
            panic!("failed to read matched-catalysis config {config_path}: {error}")
        });
        let config = serde_json::from_str::<
            crate::environment_chemistry_experiment::MatchedCatalysisConfig,
        >(&contents)
        .unwrap_or_else(|error| panic!("invalid matched-catalysis config {config_path}: {error}"));
        match crate::environment_chemistry_experiment::preflight_matched_pair(seed, &config) {
            Ok(report) => println!("MATCHED CATALYSIS EXPOSURE PREFLIGHT PASSED: {report}"),
            Err(error) => panic!("MATCHED CATALYSIS EXPOSURE PREFLIGHT BLOCKED: {error}"),
        }
        return;
    }

    if environment_chemistry_experiment {
        let ticks = if max_ticks_provided || duration_seconds.is_some() {
            max_ticks
        } else {
            24_000
        };
        let matched_setup = match (
            matched_catalysis_config_path.as_deref(),
            matched_catalysis_condition.as_deref(),
        ) {
            (None, None) => None,
            (Some(config_path), Some(condition)) => {
                let contents = std::fs::read_to_string(config_path).unwrap_or_else(|error| {
                    panic!("failed to read matched-catalysis config {config_path}: {error}")
                });
                let config = serde_json::from_str::<
                    crate::environment_chemistry_experiment::MatchedCatalysisConfig,
                >(&contents)
                .unwrap_or_else(|error| {
                    panic!("invalid matched-catalysis config {config_path}: {error}")
                });
                let condition =
                    crate::environment_chemistry_experiment::MatchedCatalysisCondition::parse(
                        condition,
                    )
                    .unwrap_or_else(|error| panic!("{error}"));
                Some((config, condition))
            }
            _ => panic!(
                "--matched-catalysis-config and --matched-catalysis-condition must be supplied together"
            ),
        };
        run_environment_chemistry_experiment(
            seed,
            ticks,
            duration_seconds,
            diagnostics_path
                .as_deref()
                .unwrap_or("environment-chemistry-diagnostics.jsonl"),
            diagnostic_interval,
            matched_setup,
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
    diagnostics_path: &str,
    diagnostic_interval: u64,
    matched_setup: Option<(
        crate::environment_chemistry_experiment::MatchedCatalysisConfig,
        crate::environment_chemistry_experiment::MatchedCatalysisCondition,
    )>,
) {
    let fixture = match matched_setup.as_ref() {
        Some((config, condition)) => {
            crate::environment_chemistry_experiment::build_matched(seed, config, *condition)
                .unwrap_or_else(|error| {
                    panic!("failed to build matched-catalysis fixture: {error}")
                })
        }
        None => crate::environment_chemistry_experiment::build(seed),
    };
    let mut simulation = fixture.simulation;
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
    if let Some(metadata) = fixture.experiment_metadata.as_ref() {
        diagnostics
            .record_experiment_setup(metadata)
            .unwrap_or_else(|error| {
                panic!("failed to record matched-catalysis setup metadata: {error}")
            });
    }

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
        "ENVIRONMENTAL CHEMISTRY START: seed={seed} duration_seconds={:?} tick_limit={:?} organisms={} independent_resources={independent_resources} molecules={molecules} constituents={constituents} physical_objects={physical_objects} piles={:?} hotspot={:?} diagnostics={diagnostics_path}",
        duration_seconds,
        planned_ticks,
        simulation.organisms.len(),
        fixture.pile_sizes,
        simulation.hotspot
    );

    let started_at = Instant::now();
    while duration_seconds.map_or(simulation.tick < max_ticks, |seconds| {
        started_at.elapsed() < Duration::from_secs(seconds)
    }) {
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
