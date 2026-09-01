use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use crate::decision::{ActionEligibility, ActionKind, CurrentNeeds};
use crate::decision_runtime::{select_action, ActionCandidate, DecisionContext};
use crate::environment::{
    apply_settling, apply_vents, ActiveMaterialField, DeepReservoir, Vent,
    DEFAULT_CELL_SIZE, DEFAULT_DIFFUSION_FRACTION, DEFAULT_RESERVOIR_BLOCK_SIZE,
    DEFAULT_SETTLING_FRACTION, DEFAULT_SETTLING_INTERVAL_TICKS,
};
use crate::genome::initial_genome;
use crate::state::{
    DevelopmentStage, EnergyLedger, Environment, Organism, Position, Snapshot,
    Simulation, ResourceSense, PROCESSING_REACH,
};

const ENERGY_NEED_THRESHOLD: f64 = 1.0;
const RAW_MATERIAL_NEED_THRESHOLD: f64 = 1.0;
const STRESS_RELIEF_THRESHOLD: f64 = 1.0;
const CONSTRUCTION_MATERIAL_THRESHOLD: f64 = 1.0;

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
                composition: vec![
                    ("Carbon".into(), 0.10), ("Methane".into(), 0.45),
                    ("Hydrogen".into(), 0.25), ("Sulfur".into(), 0.10),
                    ("Nitrogen".into(), 0.05), ("Phosphorus".into(), 0.02),
                    ("Water".into(), 0.03),
                ],
                emission_amount: 100.0,
                emission_interval: 20,
                emission_timer: 0,
            },
            Vent {
                x: 750.0,
                y: 300.0,
                composition: vec![
                    ("Carbon".into(), 0.35), ("Methane".into(), 0.10),
                    ("Hydrogen".into(), 0.15), ("Sulfur".into(), 0.25),
                    ("Nitrogen".into(), 0.05), ("Phosphorus".into(), 0.05),
                    ("Water".into(), 0.05),
                ],
                emission_amount: 100.0,
                emission_interval: 30,
                emission_timer: 0,
            },
            Vent {
                x: 520.0,
                y: 550.0,
                composition: vec![
                    ("Carbon".into(), 0.25), ("Methane".into(), 0.15),
                    ("Hydrogen".into(), 0.30), ("Sulfur".into(), 0.10),
                    ("Nitrogen".into(), 0.10), ("Phosphorus".into(), 0.02),
                    ("Water".into(), 0.08),
                ],
                emission_amount: 100.0,
                emission_interval: 25,
                emission_timer: 0,
            },
        ];

        Environment { width, height, catalog, field, reservoir, vents }
    }

    pub(crate) fn create_initial_organism() -> Organism {
        Organism {
            id: "1".into(),
            occupied_cells: vec![Position { x: 500.0, y: 500.0 }],
            genome: initial_genome(),
            resource_sense: ResourceSense {
                sensed_resources: Vec::new(),
                direction_x: 0.0,
                direction_y: 0.0,
                direction_strength: 0.0,
            },
            memory: Vec::new(),
            decision_history: crate::decision::DecisionHistory::default(),
            usable_energy: 0.0,
            stress: 0.0,
            stored_unbonded: crate::resources::Material { parts: Vec::new(), bonded: false },
            structure: crate::structure::OrganismStructure::new(),
            development_stage: DevelopmentStage::Juvenile,
            age: 0,
            active_transformation_id: None,
        }
    }

    pub(crate) fn step_environment(&mut self) {
        apply_vents(
            &mut self.environment.field,
            &mut self.environment.reservoir,
            &mut self.environment.vents,
        );
        self.environment.field.diffuse_step(DEFAULT_DIFFUSION_FRACTION);
        if self.tick % DEFAULT_SETTLING_INTERVAL_TICKS == 0 {
            apply_settling(
                &mut self.environment.field,
                &mut self.environment.reservoir,
                DEFAULT_SETTLING_FRACTION,
            );
        }
    }

    fn current_needs(organism: &Organism) -> CurrentNeeds {
        let raw_material = organism.stored_unbonded.total_amount();
        CurrentNeeds {
            energy: organism.usable_energy < ENERGY_NEED_THRESHOLD,
            material: raw_material < RAW_MATERIAL_NEED_THRESHOLD,
            construction: raw_material >= CONSTRUCTION_MATERIAL_THRESHOLD
                && organism.structure.units.is_empty(),
            relief: organism.stress >= STRESS_RELIEF_THRESHOLD,
            exploration: organism.resource_sense.sensed_resources.is_empty(),
        }
    }

    fn action_eligibility(
        organism: &Organism,
        environment: &Environment,
    ) -> ActionEligibility {
        let can_break = organism.active_transformation_id.is_none()
            && organism.resource_sense.sensed_resources.iter().any(|resource| {
                resource.desirability > 0.0
                    && resource.distance <= PROCESSING_REACH
                    && resource.bonded
                    && environment
                        .field
                        .cells
                        .get(resource.field_index)
                        .is_some_and(|cell| {
                            cell.bonded.can_break() && cell.bonded.total_amount() > 0.0
                        })
            });

        ActionEligibility {
            can_move: organism.active_transformation_id.is_none(),
            can_acquire: false,
            can_combine: false,
            can_break,
            can_expel: false,
        }
    }

    fn decision_candidates(organism: &Organism) -> Vec<ActionCandidate> {
        let mut candidates = vec![ActionCandidate {
            action: ActionKind::Move,
            context_key: None,
        }];

        if let Some(target) = organism
            .resource_sense
            .sensed_resources
            .iter()
            .filter(|resource| {
                resource.desirability > 0.0
                    && resource.distance <= PROCESSING_REACH
                    && resource.bonded
            })
            .max_by(|a, b| {
                a.desirability
                    .partial_cmp(&b.desirability)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        {
            candidates.push(ActionCandidate {
                action: ActionKind::Break,
                context_key: Some(target.name.clone()),
            });
        }

        candidates
    }

    pub(crate) fn step(&mut self) -> Snapshot {
        self.tick += 1;
        self.step_environment();

        let mut still_active = Vec::new();
        let mut completed = Vec::new();
        for mut transformation in self.active_transformations.drain(..) {
            if transformation.remaining_ticks > 0 {
                transformation.remaining_ticks -= 1;
            }
            if transformation.remaining_ticks == 0 {
                completed.push(transformation);
            } else {
                still_active.push(transformation);
            }
        }
        self.active_transformations = still_active;

        for transformation in &completed {
            if let Some(organism) = self.organisms.iter_mut().find(|o| o.id == transformation.organism_id) {
                Self::resolve_transformation(
                    transformation,
                    organism,
                    &mut self.environment,
                    &mut self.energy_ledger,
                );
            }
        }

        let environment_snapshot = self.environment.clone();
        for organism in &mut self.organisms {
            organism.age += 1;
            Self::update_resource_perception(organism, &environment_snapshot);
            Self::update_memory_from_sources(organism, &environment_snapshot);
        }

        let (organisms, environment) = (&mut self.organisms, &mut self.environment);
        for organism in organisms {
            let context = DecisionContext {
                needs: Self::current_needs(organism),
                eligibility: Self::action_eligibility(organism, environment),
            };
            let candidates = Self::decision_candidates(organism);
            let Some(selected) = select_action(context, &organism.decision_history, &candidates) else {
                continue;
            };

            match selected.action {
                ActionKind::Move => {
                    let moved = Self::update_movement(organism, environment);
                    crate::decision_runtime::record_outcome(
                        &mut organism.decision_history,
                        &selected,
                        if moved {
                            crate::decision::OutcomeKind::Neutral
                        } else {
                            crate::decision::OutcomeKind::Harmful
                        },
                    );
                }
                ActionKind::Break => {
                    if let Some(transformation) = Self::try_start_transformation(
                        organism,
                        &mut environment.field,
                        &mut self.next_transformation_id,
                        &selected,
                    ) {
                        self.active_transformations.push(transformation);
                    }
                }
                ActionKind::Acquire | ActionKind::Combine | ActionKind::Expel => {
                    // These actions remain mechanically unavailable until their
                    // physical executors are integrated. The decision layer
                    // therefore never selects them merely because a need exists.
                }
            }
        }

        for organism in &mut self.organisms {
            Self::apply_energy_capacity(organism);
        }
        self.energy_ledger.total_usable_energy_held =
            self.organisms.iter().map(|o| o.usable_energy).sum();

        self.snapshot()
    }

    pub(crate) fn snapshot(&self) -> Snapshot {
        Snapshot {
            tick: self.tick,
            organisms: self.organisms.clone(),
            environment: self.environment.clone(),
            active_transformations: self.active_transformations.clone(),
            energy_ledger: self.energy_ledger,
        }
    }

    #[cfg(test)]
    pub(crate) fn total_material_in_system(&self) -> f64 {
        let mut total = self.environment.field.total_amount();
        total += self.environment.reservoir.total_amount();
        for transformation in &self.active_transformations {
            total += transformation.material.total_amount();
        }
        for organism in &self.organisms {
            total += organism.stored_unbonded.total_amount();
            total += organism.structure.units.len() as f64;
        }
        total
    }
}
