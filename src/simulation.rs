use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::collections::HashSet;

use crate::decision::{ActionEligibility, ActionKind, CurrentNeeds, DecisionParameters};
use crate::decision_runtime::{select_action, ActionCandidate, DecisionContext};
use crate::energy_ledger::EnergyLedgerAuthority;
use crate::environment::{
    apply_vents, ActiveMaterialField, Vent, DEFAULT_CELL_SIZE, DEFAULT_DIFFUSION_FRACTION,
};
use crate::genome::initial_genome;
use crate::juvenile::realize_initial;
use crate::state::{
    DevelopmentStage, EnergyLedger, Environment, Organism, Position, ResourceSense, Simulation,
};

const ADULTHOOD_GROWTH_FRACTION: f64 = 0.90;

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
            decomposing_bodies: Vec::new(),
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
        let mut field = ActiveMaterialField::new(width, height, DEFAULT_CELL_SIZE);
        crate::environmental_materials::seed_initial_landscape(&mut field);
        let vents = vec![
            Vent {
                x: 250.0,
                y: 250.0,
                emission_amount: 50.0,
                emission_interval: 20,
                emission_timer: 0,
            },
            Vent {
                x: 750.0,
                y: 300.0,
                emission_amount: 50.0,
                emission_interval: 30,
                emission_timer: 0,
            },
            Vent {
                x: 520.0,
                y: 550.0,
                emission_amount: 50.0,
                emission_interval: 25,
                emission_timer: 0,
            },
        ];
        Environment {
            width,
            height,
            catalog,
            field,
            vents,
        }
    }

    pub(crate) fn create_initial_organism() -> Organism {
        let genome = initial_genome();
        let catalog = crate::resources::default_catalog();
        let juvenile_target = genome
            .developmental_construction_target(&catalog)
            .expect("initial architecture must produce a viable juvenile target");
        let (structure, _construction_ledger, initial_energy) =
            realize_initial(&juvenile_target, &catalog)
                .expect("initial juvenile target must be physically realizable");
        let mut stored_material = crate::material_storage::MaterialStorage::default();
        assert!(stored_material.store(genome.juvenile_reserve.clone()));
        Organism {
            id: "1".into(),
            occupied_cells: vec![Position { x: 500.0, y: 500.0 }],
            genome,
            resource_sense: ResourceSense {
                sensed_resources: Vec::new(),
                direction_x: 0.0,
                direction_y: 0.0,
                direction_strength: 0.0,
            },
            memory: Vec::new(),
            decision_history: crate::decision::DecisionHistory::default(),
            usable_energy: initial_energy,
            stress: 0.0,
            stress_threshold: crate::state::INITIAL_STRESS_THRESHOLD,
            stored_material,
            structure,
            development_stage: DevelopmentStage::Juvenile,
            age: 0,
            reproductive_readiness: 0.0,
            active_transformation_id: None,
            reproductive_construction: None,
        }
    }

    pub(crate) fn step_environment(&mut self) {
        apply_vents(
            &mut self.environment.field,
            &self.environment.catalog,
            &mut self.environment.vents,
            &mut self.rng,
        );
        self.environment
            .field
            .diffuse_step(DEFAULT_DIFFUSION_FRACTION);
    }

    fn mature_structural_mass(organism: &Organism, environment: &Environment) -> f64 {
        organism
            .genome
            .mature_construction_target()
            .ok()
            .map(|target| target.structural_mass(&environment.catalog))
            .unwrap_or(0.0)
    }

    fn growth_fraction(organism: &Organism, environment: &Environment) -> f64 {
        let mature_mass = Self::mature_structural_mass(organism, environment);
        if !mature_mass.is_finite() || mature_mass <= 0.0 {
            return 0.0;
        }
        (organism.structural_mass(&environment.catalog) / mature_mass).max(0.0)
    }

    fn update_development_stage(organism: &mut Organism, environment: &Environment) {
        match organism.development_stage {
            DevelopmentStage::Offspring => {
                if organism.reproductive_construction.is_none() {
                    organism.development_stage = DevelopmentStage::Juvenile
                }
            }
            DevelopmentStage::Juvenile => {
                if Self::growth_fraction(organism, environment) >= ADULTHOOD_GROWTH_FRACTION {
                    organism.development_stage = DevelopmentStage::Adult
                }
            }
            DevelopmentStage::Adult => {}
        }
    }

    fn current_needs(
        organism: &Organism,
        environment: &Environment,
        parameters: DecisionParameters,
    ) -> CurrentNeeds {
        let survival_reserve = parameters.survival_reserve.max(f64::EPSILON);
        let reserve_pressure = (1.0 - organism.usable_energy / survival_reserve).clamp(0.0, 1.0);
        let survival = (reserve_pressure * (1.0 + organism.stress.max(0.0))).clamp(0.0, 1.0);
        let _ = environment;
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
        if !matches!(organism.development_stage, DevelopmentStage::Adult) {
            return;
        }
        let mature_mass = Self::mature_structural_mass(organism, environment).max(f64::EPSILON);
        let maturity =
            (organism.structural_mass(&environment.catalog) / mature_mass).clamp(0.0, 1.0);
        let reproduction_reserve = parameters.reproduction_reserve.max(f64::EPSILON);
        let energy_readiness = (organism.usable_energy / reproduction_reserve).clamp(0.0, 1.0);
        let accumulation =
            (maturity * energy_readiness * parameters.reproduction_accumulation_rate.max(0.0))
                .clamp(0.0, 1.0);
        organism.reproductive_readiness =
            (organism.reproductive_readiness + accumulation).clamp(0.0, 1.0)
    }

    fn acquisition_targets(organism: &Organism, environment: &Environment) -> Vec<usize> {
        let Some(position) = organism.occupied_cells.first() else {
            return Vec::new();
        };
        let Some(field_index) = environment.field.index_for_position(position.x, position.y) else {
            return Vec::new();
        };
        if environment.field.cells[field_index]
            .materials
            .iter()
            .any(|material| !material.is_empty() && material.is_valid())
        {
            vec![field_index]
        } else {
            Vec::new()
        }
    }

    fn acquisition_context_key(field_index: usize) -> String {
        format!("target:{field_index}")
    }

    fn action_eligibility(organism: &Organism, environment: &Environment) -> ActionEligibility {
        let can_build_from_storage = !organism.structure.units.is_empty()
            && organism.stored_material.count_unstructured() > 0;
        let can_join_existing_structure = organism.structure.units.len() >= 2;
        ActionEligibility {
            can_move: organism.active_transformation_id.is_none(),
            can_acquire: organism.active_transformation_id.is_none()
                && !Self::acquisition_targets(organism, environment).is_empty(),
            can_combine: organism.active_transformation_id.is_none()
                && (can_build_from_storage || can_join_existing_structure),
            can_break: organism.active_transformation_id.is_none()
                && !organism.structure.bonds.is_empty(),
            can_expel: false,
        }
    }

    fn decision_candidates(
        organism: &Organism,
        environment: &Environment,
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
            )
        }
        if relevant(ActionKind::Combine) {
            candidates.push(ActionCandidate {
                action: ActionKind::Combine,
                context_key: None,
            })
        }
        if relevant(ActionKind::Move) {
            candidates.push(ActionCandidate {
                action: ActionKind::Move,
                context_key: None,
            })
        }
        if relevant(ActionKind::Acquire) {
            candidates.extend(
                Self::acquisition_targets(organism, environment)
                    .into_iter()
                    .map(|field_index| ActionCandidate {
                        action: ActionKind::Acquire,
                        context_key: Some(Self::acquisition_context_key(field_index)),
                    }),
            )
        }
        candidates
    }

    fn perform_action(
        &mut self,
        organism_index: usize,
        candidate: &ActionCandidate,
    ) -> Result<(), String> {
        let organism = self
            .organisms
            .get_mut(organism_index)
            .ok_or_else(|| "organism missing".to_string())?;
        match candidate.action {
            ActionKind::Break => {
                let key = candidate
                    .context_key
                    .as_deref()
                    .ok_or_else(|| "break action missing bond context".to_string())?;
                let index = key
                    .strip_prefix("bond:")
                    .ok_or_else(|| "invalid break context".to_string())?
                    .parse::<usize>()
                    .map_err(|_| "invalid break bond index".to_string())?;
                let result = crate::combine_runtime::break_specific_bond(
                    &mut organism.structure,
                    index,
                    &self.environment.catalog,
                    &mut self.energy_ledger,
                    &mut organism.usable_energy,
                )?;
                organism.add_transaction_stress(result);
                Ok(())
            }
            ActionKind::Combine => {
                let result = crate::combine_runtime::combine(
                    organism,
                    &self.environment.catalog,
                    &mut self.energy_ledger,
                )?;
                organism.add_transaction_stress(result);
                Ok(())
            }
            ActionKind::Move => {
                let direction = (organism.resource_sense.direction_x, organism.resource_sense.direction_y);
                let strength = organism.resource_sense.direction_strength.max(0.0);
                if strength <= 0.0 {
                    return Ok(());
                }
                let dx = direction.0 * strength;
                let dy = direction.1 * strength;
                let positions = organism.occupied_cells.clone();
                let Some(first) = positions.first() else {
                    return Ok(());
                };
                let target = Position {
                    x: first.x + dx,
                    y: first.y + dy,
                };
                let _ = target;
                Ok(())
            }
            ActionKind::Acquire => {
                let key = candidate
                    .context_key
                    .as_deref()
                    .ok_or_else(|| "acquire action missing field context".to_string())?;
                let index = key
                    .strip_prefix("target:")
                    .ok_or_else(|| "invalid acquire context".to_string())?
                    .parse::<usize>()
                    .map_err(|_| "invalid acquire field index".to_string())?;
                crate::material_transfer::acquire_from_field(
                    organism,
                    &mut self.environment.field,
                    index,
                    &self.environment.catalog,
                )?;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    pub(crate) fn step(&mut self) {
        self.step_environment();
        let organism_count = self.organisms.len();
        for organism_index in 0..organism_count {
            if organism_index >= self.organisms.len() {
                break;
            }
            let needs = {
                let organism = &self.organisms[organism_index];
                Self::current_needs(organism, &self.environment, self.decision_parameters)
            };
            let eligibility = {
                let organism = &self.organisms[organism_index];
                Self::action_eligibility(organism, &self.environment)
            };
            let candidates = {
                let organism = &self.organisms[organism_index];
                Self::decision_candidates(organism, &self.environment, needs, eligibility)
            };
            if let Some(candidate) = select_action(
                &candidates,
                &DecisionContext {
                    needs,
                    memory: &self.organisms[organism_index].memory,
                    history: &self.organisms[organism_index].decision_history,
                    rng: &mut self.rng,
                },
            ) {
                let _ = self.perform_action(organism_index, &candidate);
            }
            if let Some(organism) = self.organisms.get_mut(organism_index) {
                Self::update_reproductive_readiness(
                    organism,
                    &self.environment,
                    self.decision_parameters,
                );
                Self::update_development_stage(organism, &self.environment);
                organism.age = organism.age.saturating_add(1);
            }
        }
        self.tick = self.tick.saturating_add(1);
    }
}
