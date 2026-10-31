use std::collections::VecDeque;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::Value;

use crate::state::Simulation;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::process::{Child, Command};

pub(crate) const HISTORY_CAPACITY: usize = 30;
pub(crate) const HISTORY_INTERVAL: u64 = 100;

pub(crate) struct RuntimeState {
    pub(crate) simulation: Simulation,
    pub(crate) measured_ticks_per_second: f64,
    history: VecDeque<Simulation>,
    pub(crate) session_id: String,
}

fn new_session_id() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| format!("{}-{}", std::process::id(), duration.as_nanos()))
        .unwrap_or_else(|_| "unknown".into())
}

impl RuntimeState {
    pub(crate) fn new(simulation: Simulation) -> Self {
        let mut history = VecDeque::with_capacity(HISTORY_CAPACITY);
        history.push_back(simulation.clone());
        let session_id = new_session_id();
        Self {
            simulation,
            measured_ticks_per_second: 0.0,
            history,
            session_id,
        }
    }

    pub(crate) fn reset(&mut self) {
        let ticks_per_second = self.simulation.ticks_per_second;
        self.simulation = crate::environment_chemistry_experiment::build(42).simulation;
        self.simulation.ticks_per_second = ticks_per_second;
        self.simulation.running = true;
        self.measured_ticks_per_second = 0.0;
        self.session_id = new_session_id();
        self.history.clear();
        self.history.push_back(self.simulation.clone());
    }

    pub(crate) fn record_tick(&mut self) {
        if self.simulation.tick % HISTORY_INTERVAL != 0 {
            return;
        }
        self.history.push_back(self.simulation.clone());
        while self.history.len() > HISTORY_CAPACITY {
            self.history.pop_front();
        }
    }

    pub(crate) fn step(&mut self) {
        self.simulation.step();
        self.record_tick();
    }

    pub(crate) fn observed_tick(&self) -> u64 {
        let target = self.simulation.tick.saturating_sub(15);
        self.history
            .iter()
            .rev()
            .find(|snapshot| snapshot.tick <= target)
            .map_or(0, |snapshot| snapshot.tick)
    }

    pub(crate) fn snapshot(&self, tick: u64) -> Option<Simulation> {
        self.history
            .iter()
            .find(|snapshot| snapshot.tick == tick)
            .cloned()
    }

    pub(crate) fn history_ticks(&self) -> Vec<u64> {
        self.history.iter().map(|snapshot| snapshot.tick).collect()
    }

    pub(crate) fn restore_tick(&mut self, tick: u64) -> bool {
        // Restore the nearest retained checkpoint, then replay authoritative
        // simulation steps to reach the exact requested tick.
        if tick > self.simulation.tick {
            return false;
        }
        let Some(snapshot) = self.history.iter()
            .filter(|snapshot| snapshot.tick <= tick)
            .max_by_key(|snapshot| snapshot.tick)
            .cloned()
        else {
            return false;
        };
        self.simulation = snapshot;
        while self.simulation.tick < tick {
            self.simulation.step();
        }
        self.simulation.running = false;
        self.history.retain(|candidate| candidate.tick <= tick);
        if self.history.back().map(|candidate| candidate.tick) != Some(tick) {
            self.history.push_back(self.simulation.clone());
        }
        while self.history.len() > HISTORY_CAPACITY {
            self.history.pop_front();
        }
        true
    }
}

pub(crate) struct SimulationProcess {
    child: Child,
    port: u16,
}

impl SimulationProcess {
    pub(crate) async fn spawn() -> Self {
        let port = std::net::TcpListener::bind(("127.0.0.1", 0))
            .expect("could not allocate simulation control port")
            .local_addr()
            .expect("could not read simulation control port")
            .port();

        let executable = std::env::current_exe().expect("could not locate EvoSim executable");
        let child = Command::new(executable)
            .kill_on_drop(true)
            .arg("--simulation-child")
            .arg(port.to_string())
            .spawn()
            .expect("could not start simulation child");

        let process = Self { child, port };

        for _ in 0..100 {
            if TcpStream::connect(("127.0.0.1", port)).await.is_ok() {
                return process;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        panic!("simulation child did not become ready");
    }

    pub(crate) async fn subscribe(&mut self) -> Result<TcpStream, &'static str> {
        if self
            .child
            .try_wait()
            .map_err(|_| "simulation_unavailable")?
            .is_some()
        {
            *self = Self::spawn().await;
        }

        let mut stream = TcpStream::connect(("127.0.0.1", self.port))
            .await
            .map_err(|_| "simulation_unavailable")?;
        stream
            .write_all(b"{\"command\":\"subscribe\"}\n")
            .await
            .map_err(|_| "simulation_unavailable")?;
        Ok(stream)
    }

    pub(crate) async fn request(&mut self, command: Value) -> Result<Value, &'static str> {
        if self
            .child
            .try_wait()
            .map_err(|_| "simulation_unavailable")?
            .is_some()
        {
            *self = Self::spawn().await;
        }

        let mut stream = TcpStream::connect(("127.0.0.1", self.port))
            .await
            .map_err(|_| "simulation_unavailable")?;

        let mut encoded = serde_json::to_vec(&command).map_err(|_| "invalid_command")?;
        encoded.push(b'\n');
        stream
            .write_all(&encoded)
            .await
            .map_err(|_| "simulation_unavailable")?;

        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .await
            .map_err(|_| "simulation_unavailable")?;

        let response: Value =
            serde_json::from_str(&line).map_err(|_| "invalid_simulation_response")?;

        if response.get("ok").and_then(Value::as_bool) != Some(true) {
            return Err(match response.get("error").and_then(Value::as_str) {
                Some("not_found") => "not_found",
                Some("invalid_speed") => "invalid_speed",
                Some("invalid_command") => "invalid_command",
                Some("simulation_unavailable") => "simulation_unavailable",
                _ => "simulation_error",
            });
        }

        response
            .get("value")
            .cloned()
            .ok_or("invalid_simulation_response")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_is_bounded_and_keeps_latest_ticks() {
        let mut runtime = RuntimeState::new(Simulation::new(42, 10.0));
        for tick in 1..=3_500 {
            runtime.simulation.tick = tick;
            runtime.record_tick();
        }

        assert_eq!(runtime.history.len(), HISTORY_CAPACITY);
        assert_eq!(runtime.history_ticks().first(), Some(&600));
        assert_eq!(runtime.history_ticks().last(), Some(&3_500));
    }

    #[test]
    fn reset_starts_a_new_session_and_restores_the_experiment() {
        let mut runtime = RuntimeState::new(Simulation::new(42, 10.0));
        let previous_session = runtime.session_id.clone();
        runtime.simulation.tick = 250;
        runtime.reset();

        assert_ne!(runtime.session_id, previous_session);
        assert_eq!(runtime.simulation.tick, 0);
        assert!(runtime.simulation.running);
        assert_eq!(runtime.history_ticks(), vec![0]);
    }

    #[test]
    fn restore_replaces_live_state_and_discards_future_history() {
        let mut runtime = RuntimeState::new(Simulation::new(42, 10.0));
        for tick in 1..=500 {
            runtime.simulation.tick = tick;
            runtime.record_tick();
        }

        assert!(runtime.restore_tick(250));
        assert_eq!(runtime.simulation.tick, 250);
        assert!(!runtime.simulation.running);
        assert_eq!(runtime.history_ticks(), vec![0, 100, 200, 250]);
        assert!(!runtime.restore_tick(500));
    }
}
