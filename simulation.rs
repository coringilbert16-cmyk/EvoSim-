use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

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
    Snapshot, PROCESSING_RATE, PROCESSING_REACH,
};

impl Simulation {
    pub(crate) fn new(seed: u64, ticks_per_second: f64) -> Self {
        let rng = ChaCha8Rng::seed_from_u64(seed);
        let environment = Self::create_environment();
        let mut organism = Self::create_initial_organism();
        organism.usable_energy = 25.0;
        organism.store_unbonded_material(crate::resources::Material {
            parts: vec![
                ("Carbon".into(), 84.0),
                ("Methane".into(), 100.0),
                ("Hydrogen".into(), 100.0),
                ("Sulfur".into(), 100.0),
                ("Nitrogen".into(), 100.0),
                ("Phosphorus".into(), 100.0),
                ("Water".into(), 100.0),
            ],
            bonded: false,
        });
        for i in 0..16 {
            let angle = (i as f64) * std::f64::consts::TAU / 16.0;
            organism
                .structure
                .add_unit(crate::structure::StructuralUnit::new(
                    "Carbon",
                    crate::structure::Placement {
                        x: 500.0 + angle.cos() * 2.0,
                        y: 500.0 + angle.sin() * 2.0,
                        rotation_radians: 0.0,
                    },
                ));
        }
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
            .filter_map(|unit| unit.properties(&environment.catalog).map(|p| p.mass))
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
        let _maturity = (Self::structural_mass(organism, environment)
            / organism.genome.adult_mass().max(f64::EPSILON))
        .clamp(0.0, 1.0);
        let _energy_readiness = (organism.usable_energy
            / parameters.reproduction_reserve.max(f64::EPSILON))
        .clamp(0.0, 1.0);
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
        let maturity = (Self::structural_mass(organism, environment)
            / organism.genome.adult_mass().max(f64::EPSILON))
        .clamp(0.0, 1.0);
        let energy_readiness = (organism.usable_energy
            / parameters.reproduction_reserve.max(f64::EPSILON))
        .clamp(0.0, 1.0);
        let accumulation =
            (maturity * energy_readiness * parameters.reproduction_accumulation_rate.max(0.0))
                .clamp(0.0, 1.0);
        organism.reproductive_readiness =
            (organism.reproductive_readiness + accumulation).clamp(0.0, 1.0);
    }

    fn can_acquire(organism: &Organism) -> bool {
        organism.active_transformation_id.is_none()
            && organism
                .resource_sense
                .sensed_resources
                .iter()
                .any(|r| !r.bonded && r.distance <= PROCESSING_REACH && r.perceived_amount > 0.0)
    }

    fn action_eligibility(organism: &Organism, _environment: &Environment) -> ActionEligibility {
        ActionEligibility {
            can_move: organism.active_transformation_id.is_none(),
            can_acquire: Self::can_acquire(organism),
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
                    .filter_map(|bond| {
                        (bond.id.0 > 0).then_some(ActionCandidate {
                            action: ActionKind::Break,
                            context_key: Some(format!("bond:{}", bond.id.0)),
                        })
                    }),
            );
        }
        if relevant(ActionKind::Combine) {
            candidates.push(ActionCandidate {
                action: ActionKind::Combine,
                context_key: None,
            });
        }
        if relevant(ActionKind::Acquire) {
            candidates.push(ActionCandidate {
                action: ActionKind::Acquire,
                context_key: None,
            });
        }
        if relevant(ActionKind::Move) {
            candidates.push(ActionCandidate {
                action: ActionKind::Move,
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

    fn acquire(organism: &mut Organism, environment: &mut Environment) -> bool {
        let target = organism
            .resource_sense
            .sensed_resources
            .iter()
            .filter(|r| !r.bonded && r.distance <= PROCESSING_REACH && r.perceived_amount > 0.0)
            .max_by(|a, b| {
                a.desirability
                    .partial_cmp(&b.desirability)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        let Some(target) = target else { return false };
        let amount = target.perceived_amount.clamp(0.0, PROCESSING_RATE);
        let Some(material) = environment
            .field
            .take_at_index(target.field_index, false, amount)
        else {
            return false;
        };
        if material.total_amount() <= 0.0 {
            return false;
        }
        organism.store_unbonded_material(material);
        true
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
                completed.push(transformation)
            } else {
                still_active.push(transformation)
            }
        }
        self.active_transformations = still_active;
        for transformation in &completed {
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
                ActionKind::Acquire => {
                    let acquired = Self::acquire(organism, environment);
                    crate::decision_runtime::record_outcome(
                        &mut organism.decision_history,
                        &selected,
                        if acquired {
                            crate::decision::OutcomeKind::Neutral
                        } else {
                            crate::decision::OutcomeKind::Harmful
                        },
                    );
                }
                ActionKind::Combine => {
                    let combine_attempt = crate::combine_runtime::try_combine(
                        organism,
                        environment,
                        &mut compatibility_cache,
                    );
                    let combined = combine_attempt.is_some();
                    if let Some(attempt) = combine_attempt {
                        self.energy_ledger.record_combine(
                            attempt.potential_energy_released,
                            attempt.energy_paid,
                            attempt.formation_work,
                            attempt.bond_energy,
                            attempt.usable_energy_gained,
                            attempt.heat_dissipated,
                        );
                    }
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
                        &mut self.next_transformation_id,
                        &selected,
                    ) {
                        self.active_transformations.push(transformation);
                    }
                }
                ActionKind::Expel => {}
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
        let mut total =
            self.environment.field.total_amount() + self.environment.reservoir.total_amount();
        for transformation in &self.active_transformations {
            total += transformation.material.total_amount();
        }
        for organism in &self.organisms {
            total += organism.stored_unbonded.total_amount();
            total += organism
                .structure
                .units
                .iter()
                .filter_map(|unit| unit.properties(&self.environment.catalog).map(|p| p.mass))
                .sum::<f64>();
        }
        total
    }
}
