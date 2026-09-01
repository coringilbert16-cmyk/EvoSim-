use parking_lot::Mutex;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::decision::{DecisionHistory, DecisionParameters};
use crate::environment::{ActiveMaterialField, DeepReservoir, Vent};
use crate::genome::Genome;
use crate::resources::{BaseResource, Material};

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) simulation: Arc<Mutex<Simulation>>,
    pub(crate) broadcaster: broadcast::Sender<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct PropertyDeviations {
    pub(crate) mass: f64,
    pub(crate) potential_energy: f64,
    pub(crate) reactivity: f64,
    pub(crate) cohesion: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct AffinityResponses {
    pub(crate) mass: f64,
    pub(crate) potential_energy: f64,
    pub(crate) reactivity: f64,
    pub(crate) cohesion: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct ResourceObservation {
    pub(crate) name: String,
    pub(crate) properties: crate::resources::ResourceProperties,
    pub(crate) bonded: bool,
    pub(crate) perceived_amount: f64,
    pub(crate) deviations: PropertyDeviations,
    pub(crate) affinity_responses: AffinityResponses,
    pub(crate) base_desirability: f64,
    pub(crate) amount_factor: f64,
    pub(crate) potential_energy_need_factor: f64,
    pub(crate) desirability: f64,
    pub(crate) distance: f64,
    pub(crate) source_x: f64,
    pub(crate) source_y: f64,
    pub(crate) field_index: usize,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct ResourceSense {
    pub(crate) sensed_resources: Vec<ResourceObservation>,
    pub(crate) direction_x: f64,
    pub(crate) direction_y: f64,
    pub(crate) direction_strength: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) enum DevelopmentStage {
    Juvenile,
    Adult,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct Position {
    pub(crate) x: f64,
    pub(crate) y: f64,
}

pub(crate) const MAX_MEMORY_POINTS: usize = 5;
pub(crate) const MEMORY_DECAY_PER_TICK: f64 = 0.995;
pub(crate) const MEMORY_MERGE_RADIUS: f64 = 40.0;
pub(crate) const MEMORY_PRUNE_THRESHOLD: f64 = 0.01;

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct MemoryPoint {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) strength: f64,
}

pub(crate) const PROCESSING_REACH: f64 = 20.0;
pub(crate) const PROCESSING_RATE: f64 = 4.0;

#[derive(Serialize, Deserialize, Clone, Copy)]
pub(crate) enum TransformationKind {
    Break,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct ActiveTransformation {
    pub(crate) id: u64,
    pub(crate) organism_id: String,
    pub(crate) kind: TransformationKind,
    pub(crate) material: Material,
    pub(crate) complexity: f64,
    pub(crate) duration_ticks: u64,
    pub(crate) remaining_ticks: u64,
    pub(crate) decision_context_key: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Default)]
pub(crate) struct EnergyLedger {
    pub(crate) total_potential_energy_released: f64,
    pub(crate) total_usable_energy_gained: f64,
    pub(crate) total_heat_dissipated: f64,
    pub(crate) total_usable_energy_held: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct Organism {
    pub(crate) id: String,
    pub(crate) occupied_cells: Vec<Position>,
    pub(crate) genome: Genome,
    pub(crate) resource_sense: ResourceSense,
    pub(crate) memory: Vec<MemoryPoint>,
    pub(crate) decision_history: DecisionHistory,
    pub(crate) usable_energy: f64,
    pub(crate) stress: f64,
    pub(crate) stored_unbonded: Material,
    pub(crate) structure: crate::structure::OrganismStructure,
    pub(crate) development_stage: DevelopmentStage,
    pub(crate) age: u64,
    pub(crate) active_transformation_id: Option<u64>,
}

impl Organism {
    pub(crate) fn store_unbonded_material(&mut self, material: Material) {
        if material.parts.is_empty() || material.bonded {
            return;
        }
        let mut parts = std::mem::take(&mut self.stored_unbonded.parts);
        parts.extend(material.parts);
        self.stored_unbonded.parts = crate::resources::merge_parts(&parts);
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct Environment {
    pub(crate) width: f64,
    pub(crate) height: f64,
    pub(crate) catalog: Vec<BaseResource>,
    pub(crate) field: ActiveMaterialField,
    pub(crate) reservoir: DeepReservoir,
    pub(crate) vents: Vec<Vent>,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct Snapshot {
    pub(crate) tick: u64,
    pub(crate) organisms: Vec<Organism>,
    pub(crate) environment: Environment,
    pub(crate) active_transformations: Vec<ActiveTransformation>,
    pub(crate) energy_ledger: EnergyLedger,
}

pub(crate) struct Simulation {
    pub(crate) tick: u64,
    pub(crate) ticks_per_second: f64,
    pub(crate) running: bool,
    pub(crate) organisms: Vec<Organism>,
    pub(crate) environment: Environment,
    pub(crate) active_transformations: Vec<ActiveTransformation>,
    pub(crate) energy_ledger: EnergyLedger,
    pub(crate) next_organism_id: u64,
    pub(crate) next_transformation_id: u64,
    pub(crate) rng: ChaCha8Rng,
    pub(crate) decision_parameters: DecisionParameters,
}

pub(crate) const DESIRABILITY_AMOUNT_HALF_SATURATION: f64 = 100.0;
pub(crate) const DESIRABILITY_MAX: f64 = 1.0;
pub(crate) const STRESS_DECAY_PER_TICK: f64 = 0.98;
