use std::collections::VecDeque;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::state::Simulation;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::process::{Child, Command};

pub(crate) const HISTORY_CAPACITY: usize = 30;

pub(crate) struct RuntimeState {
    pub(crate) simulation: Simulation,
    history: VecDeque<Simulation>,
    pub(crate) session_id: String,
}

impl RuntimeState {
    pub(crate) fn new(simulation: Simulation) -> Self {
        let mut history = VecDeque::with_capacity(HISTORY_CAPACITY);
        history.push_back(simulation.clone());
        let session_id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis().to_string())
            .unwrap_or_else(|_| "unknown".into());
        Self {
            simulation,
            history,
            session_id,
        }
    }

    pub(crate) fn record_tick(&mut self) {
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
        self.simulation.tick.saturating_sub(15)
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
        let Some(snapshot) = self.history.iter().find(|snapshot| snapshot.tick == tick) else {
            return false;
        };
        self.simulation = snapshot.clone();
        self.history.retain(|candidate| candidate.tick <= tick);
        true
    }
}

pub(crate) struct SimulationProcess {
    child: Child,
    port: u16,
}

impl Drop for SimulationProcess {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
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

    pub(crate) async fn request(&mut self, command: Value) -> Result<Value, &'static str> {
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
            return Err(response
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("simulation_error"));
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
        for tick in 1..=35 {
            runtime.simulation.tick = tick;
            runtime.record_tick();
        }

        assert_eq!(runtime.history.len(), HISTORY_CAPACITY);
        assert_eq!(runtime.history_ticks().first(), Some(&6));
        assert_eq!(runtime.history_ticks().last(), Some(&35));
    }

    #[test]
    fn restore_replaces_live_state_and_discards_future_history() {
        let mut runtime = RuntimeState::new(Simulation::new(42, 10.0));
        for tick in 1..=5 {
            runtime.simulation.tick = tick;
            runtime.record_tick();
        }

        assert!(runtime.restore_tick(3));
        assert_eq!(runtime.simulation.tick, 3);
        assert_eq!(runtime.history_ticks(), vec![0, 1, 2, 3]);
        assert!(!runtime.restore_tick(5));
    }
}
