use serde::{Deserialize, Serialize};

use crate::decision::{ActionKind, DecisionHistory};
use crate::genome::Genome;
use crate::material_storage::MaterialStorage;
use crate::resources::{BaseResource, Material};
use crate::structure::{Bond, OrganismStructure};

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub(crate) struct ResourceSense {
    pub(crate) sensed_resources: Vec<ResourceObservation>,
    pub(crate) direction_x: f64,
    pub(crate) direction_y: f64,
    pub(crate) direction_strength: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct ResourceObservation {
    pub(crate) name: String,
    pub(crate) amount: f64,
    pub(crate) distance: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct MemoryPoint {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) strength: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) enum DevelopmentStage {
    Offspring,
    Juvenile,
    Adult,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct Position {
    pub(crate) x: f64,
    pub(crate) y: f64,
}

pub(crate) const MAX_MEMORY_POINTS: usize = 5;
pub(crate) const MEMORY_DECAY_PER_TICK: f64 = 0.995;
pub(crate) const MEMORY_MERGE_RADIUS: f64 = 40.0;
pub(crate) const MEMORY_PRUNE_THRESHOLD: f64 = 0.01;

/// Independent processing capacities. Neither is an acquisition amount.
pub(crate) const COMBINE_PROCESSING_RATE: usize = 1;
pub(crate) const BREAK_PROCESSING_RATE: usize = 1;

#[derive(Serialize, Deserialize, Clone, Copy)]
pub(crate) enum TransformationKind {
    Break,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct ActiveTransformation {
    pub(crate) id: u64,
    pub(crate) organism_id: String,
    pub(crate) kind: TransformationKind,
    /// Retained for snapshot compatibility. BREAK no longer derives energy from this material.
    pub(crate) material: Material,
    #[serde(default)]
    pub(crate) bond: Option<Bond>,
    pub(crate) complexity: f64,
    pub(crate) duration_ticks: u64,
    pub(crate) remaining_ticks: u64,
    pub(crate) decision_context_key: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct ReproductiveConstruction {
    /// Committed inventory remains independent material objects until consumed
    /// by construction. Commitment itself performs no COMBINE.
    pub(crate) committed_material: MaterialStorage,
    pub(crate) developing_structure: OrganismStructure,
    pub(crate) child_genome: Genome,
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
    /// Acquired material inventory. Storage itself never transforms material.
    pub(crate) stored_material: MaterialStorage,
    pub(crate) structure: OrganismStructure,
    pub(crate) development_stage: DevelopmentStage,
    pub(crate) age: u64,
    #[serde(default)]
    pub(crate) reproductive_readiness: f64,
    pub(crate) active_transformation_id: Option<u64>,
    #[serde(default)]
    pub(crate) reproductive_construction: Option<ReproductiveConstruction>,
}

impl Organism {
    pub(crate) fn store_material(&mut self, material: Material) -> bool {
        self.stored_material.store(material)
    }

    pub(crate) fn structural_mass(&self, catalog: &[BaseResource]) -> f64 {
        self.structure
            .units
            .iter()
            .filter_map(|unit| unit.properties(catalog).map(|properties| properties.mass))
            .sum()
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
}
