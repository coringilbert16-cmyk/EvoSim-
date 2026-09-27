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
    start_tick_loop(runtime.clone());

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
            handle_connection(stream, runtime).await;
        });
    }
}

fn start_tick_loop(runtime: Arc<Mutex<RuntimeState>>) {
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
            }
        }
    });
}

async fn handle_connection(stream: TcpStream, runtime: Arc<Mutex<RuntimeState>>) {
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);
    let mut line = String::new();

    if reader.read_line(&mut line).await.is_err() {
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
    let mut state = runtime.lock();

    match command {
        SimulationCommand::Status => CommandResponse::value(serde_json::json!({
            "tick": state.simulation.tick,
            "running": state.simulation.running,
            "ticks_per_second": state.simulation.ticks_per_second,
            "history_ticks": state.history_ticks(),
            "observed_tick": state.observed_tick(),
            "session_id": state.session_id,
        })),
        SimulationCommand::World => CommandResponse::value(serde_json::to_value(
            ObservationProjection::world(WorldObservation::from_simulation(&state.simulation)),
        )
        .expect("world observation must serialize")),
        SimulationCommand::HistoryWorld { tick } => {
            let Some(snapshot) = state.snapshot(tick) else {
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
            let Some(observation) = OrganismObservation::from_simulation(&state.simulation, &id)
            else {
                return CommandResponse::error("not_found");
            };
            let context = ObservationContext::organism(vec![id]);
            CommandResponse::value(
                serde_json::to_value(
                    ObservationProjection::organism(context, observation)
                        .expect("validated organism observation level"),
                )
                .expect("organism observation must serialize"),
            )
        }
        SimulationCommand::Structure { id } => {
            let Some(observation) = StructureObservation::from_simulation(&state.simulation, &id)
            else {
                return CommandResponse::error("not_found");
            };
            let context = ObservationContext::structure(vec![id.clone()], Some(id));
            CommandResponse::value(
                serde_json::to_value(
                    ObservationProjection::structure(context, observation)
                        .expect("validated structure observation level"),
                )
                .expect("structure observation must serialize"),
            )
        }
        SimulationCommand::Resources => {
            let resources = state
                .simulation
                .environment
                .catalog
                .iter()
                .map(|resource| (resource.name.clone(), appearance(resource)))
                .collect::<Vec<_>>();
            CommandResponse::value(serde_json::json!({
                "resources": resources,
                "field_cell_size": state.simulation.environment.field.cell_size,
            }))
        }
        SimulationCommand::Pause => {
            state.simulation.running = false;
            CommandResponse::value(serde_json::json!({"running": false}))
        }
        SimulationCommand::Resume => {
            state.simulation.running = true;
            CommandResponse::value(serde_json::json!({"running": true}))
        }
        SimulationCommand::Step => {
            state.step();
            CommandResponse::value(serde_json::json!({"tick": state.simulation.tick}))
        }
        SimulationCommand::Speed { ticks_per_second } => {
            if !ticks_per_second.is_finite() || ticks_per_second <= 0.0 {
                return CommandResponse::error("invalid_speed");
            }
            state.simulation.ticks_per_second = ticks_per_second;
            CommandResponse::value(serde_json::json!({
                "ticks_per_second": ticks_per_second
            }))
        }
        SimulationCommand::Restore { tick } => {
            if !state.restore_tick(tick) {
                return CommandResponse::error("not_found");
            }
            CommandResponse::value(serde_json::json!({"tick": state.simulation.tick}))
        }
    }
}
