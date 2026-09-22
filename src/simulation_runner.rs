use crate::state::{DevelopmentStage, Simulation};

const DEFAULT_MAX_TICKS: u64 = 10_000;
const DEFAULT_TARGET_BIRTHS: u64 = 3;
const REPORT_INTERVAL: u64 = 100;

pub(crate) fn run_from_args(args: impl Iterator<Item = String>) {
    let mut max_ticks = DEFAULT_MAX_TICKS;
    let mut target_births = DEFAULT_TARGET_BIRTHS;

    let mut args = args.skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--ticks" => {
                max_ticks = args
                    .next()
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(DEFAULT_MAX_TICKS);
            }
            "--births" => {
                target_births = args
                    .next()
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(DEFAULT_TARGET_BIRTHS);
            }
            _ => {}
        }
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

    report(&simulation, "initial");

    let mut previous_population = simulation.organisms.len();
    let mut previous_births = simulation.next_organism_id.saturating_sub(2);

    for _ in 0..max_ticks {
        simulation.step();

        let births = simulation.next_organism_id.saturating_sub(2);
        if births != previous_births {
            println!(
                "BIRTH EVENT: tick={} total_births={} population={} new_organism_ids_starting_at={}",
                simulation.tick,
                births,
                simulation.organisms.len(),
                simulation.next_organism_id.saturating_sub(1)
            );
            previous_births = births;
        }

        if simulation.tick % REPORT_INTERVAL == 0
            || simulation.organisms.len() != previous_population
        {
            report(&simulation, "progress");
            previous_population = simulation.organisms.len();
        }

        if births >= target_births {
            report(&simulation, "target reached");
            println!(
                "HEADLESS SIMULATION END: observed {} completed birth events by tick {}.",
                births, simulation.tick
            );
            return;
        }

        if simulation.organisms.is_empty() {
            report(&simulation, "population extinct");
            println!(
                "HEADLESS SIMULATION END: population reached zero at tick {}.",
                simulation.tick
            );
            return;
        }
    }

    report(&simulation, "tick limit reached");
    println!(
        "HEADLESS SIMULATION END: tick limit {} reached with {} completed birth events.",
        max_ticks,
        simulation.next_organism_id.saturating_sub(2)
    );
}

fn report(simulation: &Simulation, label: &str) {
    let offspring = simulation
        .organisms
        .iter()
        .filter(|organism| matches!(organism.development_stage, DevelopmentStage::Offspring))
        .count();
    let juveniles = simulation
        .organisms
        .iter()
        .filter(|organism| matches!(organism.development_stage, DevelopmentStage::Juvenile))
        .count();
    let adults = simulation
        .organisms
        .iter()
        .filter(|organism| matches!(organism.development_stage, DevelopmentStage::Adult))
        .count();
    let reproducing = simulation
        .organisms
        .iter()
        .filter(|organism| organism.reproductive_construction.is_some())
        .count();

    println!(
        "[{label}] tick={} population={} offspring={} juvenile={} adult={} reproducing={}          decomposing_bodies={} active_transformations={} completed_births={}",
        simulation.tick,
        simulation.organisms.len(),
        offspring,
        juveniles,
        adults,
        reproducing,
        simulation.decomposing_bodies.len(),
        simulation.active_transformations.len(),
        simulation.next_organism_id.saturating_sub(2),
    );
}
