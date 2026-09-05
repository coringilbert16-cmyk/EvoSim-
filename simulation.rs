use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::collections::HashSet;

use crate::decision::{ActionEligibility, ActionKind, CurrentNeeds, DecisionParameters};
use crate::decision_runtime::{select_action, ActionCandidate, DecisionContext};
use crate::environment::{
    apply_settling, apply_vents, ActiveMaterialField, DeepReservoir, Vent, DEFAULT_CELL_SIZE,
    DEFAULT_DIFFUSION_FRACTION, DEFAULT_RESERVOIR_BLOCK_SIZE, DEFAULT_SETTLING_FRACTION,
    DEFAULT_SETTLING_INTERVAL_TICKS,
};
use crate::genome::initial_genome;
use crate::state::{
    DevelopmentStage, EnergyLedger, Environment, Organism, Position, ResourceSense, Simulation,
    Snapshot,
};

impl Simulation {
    pub(crate) fn new(seed: u64, ticks_per_second: f64) -> Self {
        let rng = ChaCha8Rng::seed_from_u64(seed);
        let environment = Self::create_environment();
        let organism = Self::create_initial_organism();
        Self {
            tick: 0,
            ticks_per_second,
            running: true,
            organisms: vec![organism],
            environment,
            active_transformations: Vec::new(),
            energy_ledger: EnergyLedger::default(),
            next_organism_id: 2,
            next_transformation_id: 1,
            rng,
            decision_parameters: DecisionParameters::default(),
        }
    }

    fn create_environment() -> Environment {
        let catalog = crate::resources::default_catalog();
        let width = 1000.0;
        let height = 1000.0;
        let field = ActiveMaterialField::new(width, height, DEFAULT_CELL_SIZE);
        let mut reservoir = DeepReservoir::new_matching_field(&field, DEFAULT_RESERVOIR_BLOCK_SIZE);
        let starting_amounts: [(&str, f64); 7] = [
            ("Carbon", 10_000.0),
            ("Methane", 5_000.0),
            ("Hydrogen", 5_000.0),
            ("Sulfur", 5_000.0),
            ("Nitrogen", 5_000.0),
            ("Phosphorus", 5_000.0),
            ("Water", 20_000.0),
        ];
        for (name, amount) in starting_amounts {
            reservoir.seed_uniform(name, amount);
        }
        let vents = vec![
            Vent {
                x: 250.0,
                y: 250.0,
                composition: vec![("Carbon".into(), 0.10),("Methane".into(), 0.45),("Hydrogen".into(), 0.25),("Sulfur".into(), 0.10),("Nitrogen".into(), 0.05),("Phosphorus".into(), 0.02),("Water".into(), 0.03)],
                emission_amount: 100.0, emission_interval: 20, emission_timer: 0,
            },
            Vent {
                x: 750.0,
                y: 300.0,
                composition: vec![("Carbon".into(), 0.35),("Methane".into(), 0.10),("Hydrogen".into(), 0.15),("Sulfur".into(), 0.25),("Nitrogen".into(), 0.05),("Phosphorus".into(), 0.05),("Water".into(), 0.05)],
                emission_amount: 100.0, emission_interval: 30, emission_timer: 0,
            },
            Vent {
                x: 520.0,
                y: 550.0,
                composition: vec![("Carbon".into(), 0.25),("Methane".into(), 0.15),("Hydrogen".into(), 0.30),("Sulfur".into(), 0.10),("Nitrogen".into(), 0.10),("Phosphorus".into(), 0.02),("Water".into(), 0.08)],
                emission_amount: 100.0, emission_interval: 25, emission_timer: 0,
            },
        ];
        Environment { width, height, catalog, field, reservoir, vents }
    }

    pub(crate) fn create_initial_organism() -> Organism {
        Organism {
            id: "1".into(),
            occupied_cells: vec![Position { x: 500.0, y: 500.0 }],
            genome: initial_genome(),
            resource_sense: ResourceSense { sensed_resources: Vec::new(), direction_x: 0.0, direction_y: 0.0, direction_strength: 0.0 },
            memory: Vec::new(),
            decision_history: crate::decision::DecisionHistory::default(),
            usable_energy: 0.0,
            stress: 0.0,
            stored_unbonded: crate::resources::Material { parts: Vec::new(), internal_bonds: Vec::new() },
            structure: crate::structure::OrganismStructure::new(),
            development_stage: DevelopmentStage::Juvenile,
            age: 0,
            reproductive_readiness: 0.0,
            active_transformation_id: None,
            reproductive_construction: None,
        }
    }

    pub(crate) fn step_environment(&mut self) {
        apply_vents(&mut self.environment.field, &mut self.environment.reservoir, &mut self.environment.vents);
        self.environment.field.diffuse_step(DEFAULT_DIFFUSION_FRACTION);
        if self.tick % DEFAULT_SETTLING_INTERVAL_TICKS == 0 {
            apply_settling(&mut self.environment.field, &mut self.environment.reservoir, DEFAULT_SETTLING_FRACTION);
        }
    }

    fn structural_mass(organism: &Organism, environment: &Environment) -> f64 {
        organism.structure.units.iter().filter_map(|unit| unit.properties(&environment.catalog).map(|properties| properties.mass)).sum()
    }

    fn current_needs(organism: &Organism, environment: &Environment, parameters: DecisionParameters) -> CurrentNeeds {
        let survival_reserve = parameters.survival_reserve.max(f64::EPSILON);
        let reserve_pressure = (1.0 - organism.usable_energy / survival_reserve).clamp(0.0, 1.0);
        let survival = (reserve_pressure * (1.0 + organism.stress.max(0.0))).clamp(0.0, 1.0);
        let adult_mass = parameters.adult_mass.max(f64::EPSILON);
        let maturity = (Self::structural_mass(organism, environment) / adult_mass).clamp(0.0, 1.0);
        let reproduction_reserve = parameters.reproduction_reserve.max(f64::EPSILON);
        let energy_readiness = (organism.usable_energy / reproduction_reserve).clamp(0.0, 1.0);
        let _ = (maturity, energy_readiness);
        CurrentNeeds { survival, reproduction: organism.reproductive_readiness.clamp(0.0, 1.0) }
    }
}
