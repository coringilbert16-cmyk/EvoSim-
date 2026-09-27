use std::sync::Arc;
use std::time::Duration;

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

#[derive(Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
enum SimulationCommand {
    Status,
    World,
    HistoryWorld { tick: u64 },
    Organism { id: String },
    Structure { id: String },
    Resources,
    Pause,
    Resume,
    Step,
    Speed { ticks_per_second: f64 },
    Restore { tick: u64 },
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
    let runtime = Arc::new(Mutex::new(RuntimeState::new(Simulation::new(42, 10.0))));
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
        tokio::spawn(async move {
            handle_connection(stream, runtime, tick_sender).await;
        });
    }
}

fn start_tick_loop(runtime: Arc<Mutex<RuntimeState>>, tick_sender: tokio::sync::broadcast::Sender<String>) {
    tokio::spawn(async move {
        loop {
            let tick_duration = {
                let state = runtime.lock();
                if !state.simulation.running {
                    Duration::from_millis(100)
                } else {
                    let tps = state.simulation.ticks_per_second.max(0.001);
                    Duration::from_secs_f64(1.0 / tps)
                }
            };

            tokio::time::sleep(tick_duration).await;

            let mut state = runtime.lock();
            if state.simulation.running {
                state.step();
                let _ = tick_sender.send(serde_json::json!({
                    "type": "tick",
                    "tick": state.simulation.tick,
                    "session_id": state.session_id,
                }).to_string());
            }
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
                "ticks_per_second": state.simulation.ticks_per_second,
                "history_ticks": state.history_ticks(),
                "observed_tick": state.observed_tick(),
                "session_id": state.session_id,
            }))
        }
        SimulationCommand::World => {
            let projection = {
                let state = runtime.lock();
                ObservationProjection::world(WorldObservation::from_simulation(&state.simulation))
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
        SimulationCommand::Organism { id } => {
            let projection = {
                let state = runtime.lock();
                let Some(observation) =
                    OrganismObservation::from_simulation(&state.simulation, &id)
                else {
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
        SimulationCommand::Structure { id } => {
            let projection = {
                let state = runtime.lock();
                let Some(observation) =
                    StructureObservation::from_simulation(&state.simulation, &id)
                else {
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
            CommandResponse::value(serde_json::json!({"tick": tick}))
        }
        SimulationCommand::Speed { ticks_per_second } => {
            if !ticks_per_second.is_finite() || ticks_per_second <= 0.0 {
                return CommandResponse::error("invalid_speed");
            }
            let mut state = runtime.lock();
            state.simulation.ticks_per_second = ticks_per_second;
            CommandResponse::value(serde_json::json!({
                "ticks_per_second": ticks_per_second
            }))
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
