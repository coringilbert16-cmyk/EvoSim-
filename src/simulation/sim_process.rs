use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

use crate::observation::{
    ObservationContext, ObservationProjection, OrganismObservation, StructureObservation,
    WorldObservation,
};
use crate::resource_visualization::appearance;
use crate::runtime::RuntimeState;
use crate::state::Simulation;

const DEFAULT_TICKS_PER_SECOND: f64 = 10.0;
const MIN_TICKS_PER_SECOND: f64 = 0.1;
const MAX_TICKS_PER_SECOND: f64 = 100.0;
const DISPLAY_EVENT_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
enum SimulationCommand {
    Status,
    World {
        bounds: Option<(f64, f64, f64, f64)>,
        known_organisms: Vec<(String, u64, u64)>,
        known_cells: Vec<(usize, u64)>,
    },
    HistoryWorld {
        tick: u64,
    },
    Organism {
        id: String,
        known_structure_revision: Option<u64>,
        known_position_revision: Option<u64>,
    },
    Structure {
        id: String,
        known_structure_revision: Option<u64>,
    },
    Resources,
    Pause,
    Resume,
    Step,
    Speed {
        ticks_per_second: f64,
    },
    Reset,
    Restore {
        tick: u64,
    },
}

#[derive(Serialize)]
struct CommandResponse {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<&'static str>,
}

impl CommandResponse {
    fn value(value: serde_json::Value) -> Self {
        Self {
            ok: true,
            value: Some(value),
            error: None,
        }
    }

    fn error(error: &'static str) -> Self {
        Self {
            ok: false,
            value: None,
            error: Some(error),
        }
    }
}

pub(crate) async fn run_child(port: u16) {
    let mut experiment = crate::environment_chemistry_experiment::build(42).simulation;
    experiment.ticks_per_second = DEFAULT_TICKS_PER_SECOND;
    experiment.running = true;
    let runtime = Arc::new(Mutex::new(RuntimeState::new(experiment)));
    let (tick_sender, _) = tokio::sync::broadcast::channel::<String>(32);
    start_tick_loop(runtime.clone(), tick_sender.clone());

    let listener = TcpListener::bind(("127.0.0.1", port))
        .await
        .expect("simulation child could not bind its control port");

    println!("SIMULATION_READY {port}");

    loop {
        let Ok((stream, _)) = listener.accept().await else {
            continue;
        };
        let runtime = runtime.clone();
        let tick_sender = tick_sender.clone();
        tokio::spawn(async move {
            handle_connection(stream, runtime, tick_sender).await;
        });
    }
}

fn start_tick_loop(
    runtime: Arc<Mutex<RuntimeState>>,
    tick_sender: tokio::sync::broadcast::Sender<String>,
) {
    tokio::spawn(async move {
        let mut last_display_event = Instant::now();
        let mut last_rate_sample = Instant::now();
        let mut last_rate_tick = 0_u64;
        let mut next_tick_deadline = Instant::now();
        let mut scheduled_ticks_per_second = DEFAULT_TICKS_PER_SECOND;

        loop {
            let (running, ticks_per_second) = {
                let state = runtime.lock();
                (
                    state.simulation.running,
                    state.simulation.ticks_per_second,
                )
            };

            if !running {
                next_tick_deadline = Instant::now();
                tokio::time::sleep(Duration::from_millis(50)).await;
            } else {
                let rate = ticks_per_second.clamp(MIN_TICKS_PER_SECOND, MAX_TICKS_PER_SECOND);
                if (rate - scheduled_ticks_per_second).abs() > f64::EPSILON {
                    scheduled_ticks_per_second = rate;
                    next_tick_deadline = Instant::now();
                }
                let period = Duration::from_secs_f64(1.0 / rate);
                let now = Instant::now();
                if next_tick_deadline < now {
                    next_tick_deadline = now;
                }
                if next_tick_deadline > now {
                    let wait = next_tick_deadline.duration_since(now).min(DISPLAY_EVENT_INTERVAL);
                    tokio::time::sleep(wait).await;
                }

                let tick_now = Instant::now();
                if tick_now >= next_tick_deadline {
                    {
                        let mut state = runtime.lock();
                        // Recheck after waiting: pause/speed commands may have arrived.
                        if state.simulation.running {
                            state.step();
                            let sample_now = Instant::now();
                            if sample_now.duration_since(last_rate_sample) >= Duration::from_secs(1) {
                                let elapsed = sample_now.duration_since(last_rate_sample).as_secs_f64();
                                state.measured_ticks_per_second =
                                    state.simulation.tick.saturating_sub(last_rate_tick) as f64 / elapsed;
                                last_rate_tick = state.simulation.tick;
                                last_rate_sample = sample_now;
                            }
                        }
                    }

                    let after_tick = Instant::now();
                    next_tick_deadline += period;
                    // Do not run catch-up bursts after a slow tick or a long pause.
                    if next_tick_deadline <= after_tick {
                        next_tick_deadline = after_tick + period;
                    }
                }
            }

            // The simulation never waits for a browser frame. Publish only the newest
            // tick at a modest cadence; subscribers naturally skip obsolete states.
            if last_display_event.elapsed() >= DISPLAY_EVENT_INTERVAL {
                let state = runtime.lock();
                let _ = tick_sender.send(
                    serde_json::json!({
                        "type": "tick",
                        "tick": state.simulation.tick,
                        "session_id": state.session_id,
                    })
                    .to_string(),
                );
                last_display_event = Instant::now();
            }

            tokio::task::yield_now().await;
        }
    });
}

async fn handle_connection(
    stream: TcpStream,
    runtime: Arc<Mutex<RuntimeState>>,
    tick_sender: tokio::sync::broadcast::Sender<String>,
) {
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);
    let mut line = String::new();

    if reader.read_line(&mut line).await.is_err() {
        return;
    }

    if line.trim() == r#"{"command":"subscribe"}"# {
        let mut receiver = tick_sender.subscribe();
        loop {
            match receiver.recv().await {
                Ok(event) => {
                    let mut encoded = event.into_bytes();
                    encoded.push(b'\n');
                    if write_half.write_all(&encoded).await.is_err() {
                        break;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
        return;
    }

    let response = match serde_json::from_str::<SimulationCommand>(&line) {
        Ok(command) => execute(command, &runtime),
        Err(_) => CommandResponse::error("invalid_command"),
    };

    let Ok(mut encoded) = serde_json::to_vec(&response) else {
        return;
    };
    encoded.push(b'\n');
    let _ = write_half.write_all(&encoded).await;
}

fn execute(command: SimulationCommand, runtime: &Arc<Mutex<RuntimeState>>) -> CommandResponse {
    match command {
        SimulationCommand::Status => {
            let state = runtime.lock();
            CommandResponse::value(serde_json::json!({
                "tick": state.simulation.tick,
                "running": state.simulation.running,
                "population": state.simulation.living_population(),
                "ticks_per_second": state.simulation.ticks_per_second,
                "measured_ticks_per_second": state.measured_ticks_per_second,
                "history_ticks": state.history_ticks(),
                "observed_tick": state.observed_tick(),
                "session_id": state.session_id,
            }))
        }
        SimulationCommand::World {
            bounds,
            known_organisms,
            known_cells,
        } => {
            let projection = {
                let state = runtime.lock();
                ObservationProjection::world(WorldObservation::from_simulation_in_bounds(
                    &state.simulation,
                    bounds,
                    &known_organisms,
                    &known_cells,
                ))
            };
            CommandResponse::value(
                serde_json::to_value(projection).expect("world observation must serialize"),
            )
        }
        SimulationCommand::HistoryWorld { tick } => {
            let snapshot = {
                let state = runtime.lock();
                state.snapshot(tick)
            };
            let Some(snapshot) = snapshot else {
                return CommandResponse::error("not_found");
            };
            CommandResponse::value(
                serde_json::to_value(ObservationProjection::world(
                    WorldObservation::from_simulation(&snapshot),
                ))
                .expect("historical world observation must serialize"),
            )
        }
        SimulationCommand::Organism {
            id,
            known_structure_revision,
            known_position_revision,
        } => {
            let projection = {
                let state = runtime.lock();
                let Some(observation) = OrganismObservation::from_simulation_with_revisions(
                    &state.simulation,
                    &id,
                    known_structure_revision,
                    known_position_revision,
                ) else {
                    return CommandResponse::error("not_found");
                };
                let context = ObservationContext::organism(vec![id]);
                ObservationProjection::organism(context, observation)
                    .expect("validated organism observation level")
            };
            CommandResponse::value(
                serde_json::to_value(projection).expect("organism observation must serialize"),
            )
        }
        SimulationCommand::Structure {
            id,
            known_structure_revision,
        } => {
            let projection = {
                let state = runtime.lock();
                let Some(observation) = StructureObservation::from_simulation_with_revision(
                    &state.simulation,
                    &id,
                    known_structure_revision,
                ) else {
                    return CommandResponse::error("not_found");
                };
                let context = ObservationContext::structure(vec![id.clone()], Some(id));
                ObservationProjection::structure(context, observation)
                    .expect("validated structure observation level")
            };
            CommandResponse::value(
                serde_json::to_value(projection).expect("structure observation must serialize"),
            )
        }
        SimulationCommand::Resources => {
            let (catalog, field_cell_size) = {
                let state = runtime.lock();
                (
                    state.simulation.environment.catalog.clone(),
                    state.simulation.environment.field.cell_size,
                )
            };
            let resources = catalog
                .iter()
                .map(|resource| (resource.name.clone(), appearance(resource)))
                .collect::<Vec<_>>();
            CommandResponse::value(serde_json::json!({
                "resources": resources,
                "field_cell_size": field_cell_size,
            }))
        }
        SimulationCommand::Pause => {
            let mut state = runtime.lock();
            state.simulation.running = false;
            CommandResponse::value(serde_json::json!({"running": false}))
        }
        SimulationCommand::Resume => {
            let mut state = runtime.lock();
            state.simulation.running = true;
            CommandResponse::value(serde_json::json!({"running": true}))
        }
        SimulationCommand::Step => {
            let mut state = runtime.lock();
            state.step();
            let tick = state.simulation.tick;
            CommandResponse::value(
                serde_json::json!({"tick": tick, "running": state.simulation.running}),
            )
        }
        SimulationCommand::Speed { ticks_per_second } => {
            if !ticks_per_second.is_finite()
                || !(MIN_TICKS_PER_SECOND..=MAX_TICKS_PER_SECOND).contains(&ticks_per_second)
            {
                return CommandResponse::error("invalid_speed");
            }
            let mut state = runtime.lock();
            state.simulation.ticks_per_second = ticks_per_second;
            CommandResponse::value(serde_json::json!({
                "ticks_per_second": ticks_per_second
            }))
        }
        SimulationCommand::Reset => {
            let mut state = runtime.lock();
            state.reset();
            CommandResponse::value(
                serde_json::json!({"tick": 0, "running": true, "session_id": state.session_id}),
            )
        }
        SimulationCommand::Restore { tick } => {
            let mut state = runtime.lock();
            if !state.restore_tick(tick) {
                return CommandResponse::error("not_found");
            }
            let restored_tick = state.simulation.tick;
            CommandResponse::value(serde_json::json!({"tick": restored_tick}))
        }
    }
}
