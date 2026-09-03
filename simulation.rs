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
                composition: vec![
                    ("Carbon".into(), 0.10),
                    ("Methane".into(), 0.45),
                    ("Hydrogen".into(), 0.25),
                    ("Sulfur".into(), 0.10),
                    ("Nitrogen".into(), 0.05),
                    ("Phosphorus".into(), 0.02),
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
                    ("Carbon".into(), 0.35),
                    ("Methane".into(), 0.10),
                    ("Hydrogen".into(), 0.15),
                    ("Sulfur".into(), 0.25),
                    ("Nitrogen".into(), 0.05),
                    ("Phosphorus".into(), 0.05),
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
                    ("Carbon".into(), 0.25),
                    ("Methane".into(), 0.15),
                    ("Hydrogen".into(), 0.30),
                    ("Sulfur".into(), 0.10),
                    ("Nitrogen".into(), 0.10),
                    ("Phosphorus".into(), 0.02),
                    ("Water".into(), 0.08),
                ],
                emission_amount: 100.0,
                emission_interval: 25,
                emission_timer: 0,
            },
        ];
        Environment {
            width,
            height,
            catalog,
            field,
            reservoir,
            vents,
        }
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
            stored_unbonded: crate::resources::Material {
                parts: Vec::new(),
                bonded: false,
            },
            structure: crate::structure::OrganismStructure::new(),
            development_stage: DevelopmentStage::Juvenile,
            age: 0,
            reproductive_readiness: 0.0,
            active_transformation_id: None,
        }
    }

    pub(crate) fn step_environment(&mut self) {
        apply_vents(
            &mut self.environment.field,
            &mut self.environment.reservoir,
            &mut self.environment.vents,
        );
        self.environment
            .field
            .diffuse_step(DEFAULT_DIFFUSION_FRACTION);
        if self.tick % DEFAULT_SETTLING_INTERVAL_TICKS == 0 {
            apply_settling(
                &mut self.environment.field,
                &mut self.environment.reservoir,
                DEFAULT_SETTLING_FRACTION,
            );
        }
    }

    fn structural_mass(organism: &Organism, environment: &Environment) -> f64 {
        organism
            .structure
            .units
            .iter()
            .filter_map(|unit| {
                unit.properties(&environment.catalog)
                    .map(|properties| properties.mass)
            })
            .sum()
    }

    fn current_needs(
        organism: &Organism,
        environment: &Environment,
        parameters: DecisionParameters,
    ) -> CurrentNeeds {
        let survival_reserve = parameters.survival_reserve.max(f64::EPSILON);
        let reserve_pressure = (1.0 - organism.usable_energy / survival_reserve).clamp(0.0, 1.0);
        let survival = (reserve_pressure * (1.0 + organism.stress.max(0.0))).clamp(0.0, 1.0);

        let adult_mass = parameters.adult_mass.max(f64::EPSILON);
        let maturity = (Self::structural_mass(organism, environment) / adult_mass).clamp(0.0, 1.0);
        let reproduction_reserve = parameters.reproduction_reserve.max(f64::EPSILON);
        let energy_readiness = (organism.usable_energy / reproduction_reserve).clamp(0.0, 1.0);
        let _ = (maturity, energy_readiness);
        CurrentNeeds {
            survival,
            reproduction: organism.reproductive_readiness.clamp(0.0, 1.0),
        }
    }

    fn update_reproductive_readiness(
        organism: &mut Organism,
        environment: &Environment,
        parameters: DecisionParameters,
    ) {
        let adult_mass = parameters.adult_mass.max(f64::EPSILON);
        let maturity = (Self::structural_mass(organism, environment) / adult_mass).clamp(0.0, 1.0);
        let reproduction_reserve = parameters.reproduction_reserve.max(f64::EPSILON);
        let energy_readiness = (organism.usable_energy / reproduction_reserve).clamp(0.0, 1.0);
        let accumulation =
            (maturity * energy_readiness * parameters.reproduction_accumulation_rate.max(0.0))
                .clamp(0.0, 1.0);
        organism.reproductive_readiness =
            (organism.reproductive_readiness + accumulation).clamp(0.0, 1.0);
    }

    fn action_eligibility(organism: &Organism, _environment: &Environment) -> ActionEligibility {
        ActionEligibility {
            can_move: organism.active_transformation_id.is_none(),
            can_acquire: false,
            can_combine: organism.active_transformation_id.is_none()
                && organism.structure.units.len() >= 2,
            can_break: organism.active_transformation_id.is_none()
                && !organism.structure.bonds.is_empty(),
            can_expel: false,
        }
    }

    fn decision_candidates(
        organism: &Organism,
        needs: CurrentNeeds,
        eligibility: ActionEligibility,
    ) -> Vec<ActionCandidate> {
        let mut candidates = Vec::new();
        let relevant = |action: ActionKind| {
            eligibility.permits(action) && needs.any_for(action.relevant_needs())
        };

        if relevant(ActionKind::Break) {
            candidates.extend(
                organism
                    .structure
                    .bonds
                    .iter()
                    .enumerate()
                    .map(|(index, _)| ActionCandidate {
                        action: ActionKind::Break,
                        context_key: Some(format!("bond:{index}")),
                    }),
            );
        }
        if relevant(ActionKind::Combine) {
            candidates.push(ActionCandidate {
                action: ActionKind::Combine,
                context_key: None,
            });
        }
        if relevant(ActionKind::Move) {
            candidates.push(ActionCandidate {
                action: ActionKind::Move,
                context_key: None,
            });
        }
        if relevant(ActionKind::Acquire) {
            candidates.push(ActionCandidate {
                action: ActionKind::Acquire,
                context_key: None,
            });
        }
        if relevant(ActionKind::Expel) {
            candidates.push(ActionCandidate {
                action: ActionKind::Expel,
                context_key: None,
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

        let mut completed_organisms = HashSet::new();
        for transformation in &completed {
            completed_organisms.insert(transformation.organism_id.clone());
            if let Some(organism) = self
                .organisms
                .iter_mut()
                .find(|o| o.id == transformation.organism_id)
            {
                Self::resolve_transformation(
                    transformation,
                    organism,
                    &mut self.environment,
                    &mut self.energy_ledger,
                );
            }
        }

        let environment_snapshot = self.environment.clone();
        let decision_parameters = self.decision_parameters;
        for organism in &mut self.organisms {
            organism.age += 1;
            Self::update_resource_perception(organism, &environment_snapshot);
            Self::update_memory_from_sources(organism, &environment_snapshot);
            Self::update_reproductive_readiness(
                organism,
                &environment_snapshot,
                decision_parameters,
            );
        }

        let (organisms, environment) = (&mut self.organisms, &mut self.environment);
        let mut compatibility_cache = crate::contact::ConnectionCompatibilityCache::new();
        for organism in organisms {
            if completed_organisms.contains(&organism.id) {
                continue;
            }
            let needs = Self::current_needs(organism, environment, decision_parameters);
            let eligibility = Self::action_eligibility(organism, environment);
            let context = DecisionContext { needs, eligibility };
            let candidates = Self::decision_candidates(organism, needs, eligibility);
            let Some(selected) = select_action(context, &organism.decision_history, &candidates)
            else {
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
                ActionKind::Combine => {
                    let combined = crate::combine_runtime::try_combine(
                        organism,
                        environment,
                        &mut compatibility_cache,
                    )
                    .is_some();
                    crate::decision_runtime::record_outcome(
                        &mut organism.decision_history,
                        &selected,
                        if combined {
                            crate::decision::OutcomeKind::Neutral
                        } else {
                            crate::decision::OutcomeKind::Harmful
                        },
                    );
                }
                ActionKind::Break => {
                    if let Some(transformation) = Self::try_start_transformation(
                        organism,
                        &environment.catalog,
                        &mut self.next_transformation_id,
                        &selected,
                    ) {
                        self.active_transformations.push(transformation);
                    }
                }
                ActionKind::Acquire | ActionKind::Expel => {}
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
