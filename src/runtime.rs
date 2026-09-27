use std::collections::VecDeque;

use crate::state::Simulation;

pub(crate) const HISTORY_CAPACITY: usize = 30;

pub(crate) struct RuntimeState {
    pub(crate) simulation: Simulation,
    history: VecDeque<Simulation>,
}

impl RuntimeState {
    pub(crate) fn new(simulation: Simulation) -> Self {
        let mut history = VecDeque::with_capacity(HISTORY_CAPACITY);
        history.push_back(simulation.clone());
        Self { simulation, history }
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
