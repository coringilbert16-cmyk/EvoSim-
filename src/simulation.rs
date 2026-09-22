use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::collections::HashSet;

#[cfg(test)]
mod simulation_material_tests;

use crate::decision::{ActionEligibility, ActionKind, CurrentNeeds, DecisionParameters};
use crate::decision_runtime::{
    select_action_with_developmental_scores, ActionCandidate, DecisionContext,
};
use crate::energy_ledger::EnergyLedgerAuthority;
use crate::environment::{
    apply_vents, ActiveMaterialField, Vent, DEFAULT_CELL_SIZE, DEFAULT_DIFFUSION_FRACTION,
};
use crate::genome::initial_genome;
use crate::juvenile::realize_initial;
use crate::state::{DevelopmentStage, EnergyLedger, Environment, Organism, Position, Simulation};
use crate::structure::Placement;

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
        crate::environmental_materials::seed_initial_landscape(&mut field, &catalog);
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
        let seed_baseline = crate::juvenile::confirmed_seed_baseline(&catalog)
            .expect("confirmed original seed baseline must be valid");
        let (mut structure, _construction_ledger, initial_energy) =
            realize_initial(&seed_baseline, &catalog)
                .expect("confirmed original seed must be physically realizable");

        let anchor = Position { x: 500.0, y: 500.0 };
        for unit in &mut structure.units {
            unit.placement.x += anchor.x;
            unit.placement.y += anchor.y;
        }

        let mut stored_material = crate::material_storage::MaterialStorage::default();
        assert!(stored_material.store(genome.juvenile_reserve.clone()));
        Organism {
            id: "1".into(),
            developmental_origin: anchor.clone(),
            developmental_orientation_radians: 0.0,
            occupied_cells: vec![anchor],
            genome,
            harmonic_spectrum: crate::harmonics::ToneSpectrum::empty(),
            memory: Vec::new(),
            decision_history: crate::decision::DecisionHistory::default(),
            usable_energy: initial_energy,
            stress: 0.0,
            stress_threshold: crate::state::INITIAL_STRESS_THRESHOLD,
            stored_material,
            structure,
            development_stage: DevelopmentStage::Juvenile,
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
    fn update_development_stage(organism: &mut Organism, environment: &Environment) {
        match organism.development_stage {
            DevelopmentStage::Offspring => {
                if organism.reproductive_construction.is_none() {
                    organism.development_stage = DevelopmentStage::Juvenile
                }
            }
            DevelopmentStage::Juvenile => {
                if crate::developmental_decision::growth_fraction(organism, environment)
                    >= ADULTHOOD_GROWTH_FRACTION
                {
                    organism.development_stage = DevelopmentStage::Adult
                }
            }
            DevelopmentStage::Adult => {}
        }
    }
    pub(crate) fn transfer_contained_environmental_material(
        organism: &mut Organism,
        environment: &mut Environment,
    ) {
        let Some(body) = crate::organism_geometry::OrganismBodyGeometry::from_structure(
            &organism.structure,
            &environment.catalog,
        ) else {
            return;
        };
        let anchor = organism
            .occupied_cells
            .first()
            .cloned()
            .unwrap_or(Position { x: 0.0, y: 0.0 });
        for physical in environment.field.take_contained_physical_materials(&body) {
            if organism
                .stored_material
                .store_physical_instance_at_owner_anchor(
                    physical.clone(),
                    Placement {
                        x: anchor.x,
                        y: anchor.y,
                        rotation_radians: 0.0,
                    },
                )
            {
                continue;
            }
            if let Some(placement) = physical
                .placements
                .as_ref()
                .and_then(|placements| placements.first())
            {
                let _ = environment
                    .field
                    .deposit(placement.x, placement.y, physical);
            }
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
        let development = if matches!(organism.development_stage, DevelopmentStage::Juvenile) {
            (1.0 - crate::developmental_decision::growth_fraction(organism, environment)
                .clamp(0.0, 1.0))
            .max(0.0)
        } else {
            0.0
        };
        CurrentNeeds {
            survival,
            reproduction: if matches!(organism.development_stage, DevelopmentStage::Adult) {
                1.0
            } else {
                0.0
            },
            development,
        }
    }
    fn action_eligibility(
        organism: &Organism,
        _environment: &Environment,
        needs: CurrentNeeds,
    ) -> ActionEligibility {
        let can_build_from_storage =
            !organism.structure.units.is_empty() && !organism.stored_material.is_empty();
        let can_join_existing_structure = organism.structure.units.len() >= 2;
        ActionEligibility {
            can_move: organism.active_transformation_id.is_none(),
            can_combine: organism.active_transformation_id.is_none()
                && (can_build_from_storage || can_join_existing_structure),
            can_break: organism.active_transformation_id.is_none()
                && !organism.structure.bonds.is_empty()
                && (organism.reproductive_construction.is_none()
                    || needs.survival > 0.0
                    || needs.development > 0.0
                    || organism
                        .reproductive_construction
                        .as_ref()
                        .is_some_and(|construction| construction.needs_space)),
            can_expel: organism.active_transformation_id.is_none()
                && organism.stored_material.physical_count() > 0,
        }
    }
    fn decision_candidates(
        organism: &Organism,
        _environment: &Environment,
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
        if relevant(ActionKind::Expel) {
            candidates.extend(
                organism
                    .stored_material
                    .entries
                    .iter()
                    .enumerate()
                    .filter(|(_, entry)| {
                        matches!(entry, crate::material_storage::StoredMaterial::Physical(_))
                    })
                    .map(|(index, _)| ActionCandidate {
                        action: ActionKind::Expel,
                        context_key: Some(format!("stored:{index}")),
                    }),
            );
        }
        candidates
    }
    fn recycle_dead_organism(
        environment: &mut Environment,
        organism: &mut Organism,
        ledger: &mut EnergyLedger,
    ) -> Option<crate::decomposition::DecomposingBody> {
        let position = organism.occupied_cells.first().cloned()?;
        for entry in organism.stored_material.drain_entries() {
            match entry {
                crate::material_storage::StoredMaterial::Logical(material) => {
                    environment.field.deposit(position.x, position.y, material);
                }
                crate::material_storage::StoredMaterial::Physical(material) => {
                    environment.field.deposit(position.x, position.y, material);
                }
            }
        }
        if let Some(mut construction) = organism.reproductive_construction.take() {
            for entry in construction.committed_material.drain_entries() {
                match entry {
                    crate::material_storage::StoredMaterial::Logical(material) => {
                        environment.field.deposit(position.x, position.y, material);
                    }
                    crate::material_storage::StoredMaterial::Physical(material) => {
                        environment.field.deposit(position.x, position.y, material);
                    }
                }
            }
        }
        let mut body =
            crate::decomposition::DecomposingBody::new(organism.structure.clone(), 0.0, position)?;
        let energy = organism.usable_energy;
        if !ledger.transfer(&mut organism.usable_energy, &mut body.energy_budget, energy) {
            return None;
        }
        Some(body)
    }
    fn process_decomposing_bodies(&mut self) {
        let mut finished_indices = Vec::new();
        for index in 0..self.decomposing_bodies.len() {
            if self.decomposing_bodies[index].is_finished() {
                let position = self.decomposing_bodies[index].position.clone();
                let materials = self.decomposing_bodies[index]
                    .release_finished_material(&self.environment.catalog);
                for material in materials {
                    self.environment
                        .field
                        .deposit(position.x, position.y, material);
                }
                finished_indices.push(index);
                continue;
            }
            let Some(step) = crate::decomposition::resolve_one_bond_with_ledger(
                &mut self.decomposing_bodies[index],
                &self.environment,
                &mut self.energy_ledger,
            ) else {
                continue;
            };
            if step.net_energy > 0.0 {
                if let Some(organism_index) = crate::decomposition::harvestable_decomposition_energy(
                    &self.organisms,
                    &self.decomposing_bodies[index].position,
                ) {
                    let amount = step
                        .net_energy
                        .min(self.decomposing_bodies[index].energy_budget);
                    let (body, organism) = {
                        let body = &mut self.decomposing_bodies[index].energy_budget;
                        let organism = &mut self.organisms[organism_index].usable_energy;
                        (body, organism)
                    };
                    let _ = self.energy_ledger.transfer(body, organism, amount);
                }
            }
            if let Some(materials) = step.released_material {
                let position = self.decomposing_bodies[index].position.clone();
                for material in materials {
                    self.environment
                        .field
                        .deposit(position.x, position.y, material);
                }
                finished_indices.push(index);
            }
        }
        for index in finished_indices.into_iter().rev() {
            self.decomposing_bodies.remove(index);
        }
    }
    pub(crate) fn step(&mut self) {
        self.tick += 1;
        self.step_environment();
        let mut still_active = Vec::new();
        let mut completed = Vec::new();
        for mut transformation in self.active_transformations.drain(..) {
            if transformation.remaining_ticks > 0 {
                transformation.remaining_ticks -= 1
            }
            if transformation.remaining_ticks == 0 {
                completed.push(transformation)
            } else {
                still_active.push(transformation)
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
            Self::update_development_stage(organism, &environment_snapshot);
            organism.apply_maintenance(&environment_snapshot.catalog, &mut self.energy_ledger);
            crate::harmonics::update_organism_harmonics(organism, &environment_snapshot);
            Self::transfer_contained_environmental_material(organism, &mut self.environment);
            Self::update_memory_from_sources(organism, &environment_snapshot);
            if matches!(organism.development_stage, DevelopmentStage::Adult)
                && organism.reproductive_construction.is_none()
            {
                let _ = crate::reproduction::begin_reproduction(
                    organism,
                    &mut self.rng,
                    &environment_snapshot.catalog,
                    &mut self.energy_ledger,
                );
            }
        }
        {
            let (organisms, environment) = (&mut self.organisms, &mut self.environment);
            let mut compatibility_cache = crate::contact::ConnectionCompatibilityCache::new();
            for index in 0..organisms.len() {
                if completed_organisms.contains(&organisms[index].id) {
                    continue;
                }
                if organisms[index].structure.bonds.is_empty()
                    && organisms[index].stress
                        >= organisms[index]
                            .stress_threshold
                            .max(crate::state::MIN_STRESS_THRESHOLD)
                {
                    continue;
                }
                let needs =
                    Self::current_needs(&organisms[index], environment, decision_parameters);
                let eligibility = Self::action_eligibility(&organisms[index], environment, needs);
                let context = DecisionContext { needs, eligibility };
                let candidates =
                    Self::decision_candidates(&organisms[index], environment, needs, eligibility);
                let developmental_scores =
                    crate::developmental_decision::developmental_action_scores(
                        &organisms[index],
                        environment,
                        needs,
                        &candidates,
                        &self.energy_ledger,
                    );
                let Some(selected) = select_action_with_developmental_scores(
                    context,
                    &organisms[index].decision_history,
                    &candidates,
                    &developmental_scores,
                ) else {
                    continue;
                };
                match selected.action {
                    ActionKind::Move => {
                        let organism_count = organisms.len();
                        let (before, rest) = organisms.split_at_mut(index);
                        let (organism, after) =
                            rest.split_first_mut().expect("index is in organisms");
                        let mut others = Vec::with_capacity(organism_count.saturating_sub(1));
                        for other in before.iter() {
                            others.push((*other).clone());
                        }
                        for other in after.iter() {
                            others.push((*other).clone());
                        }
                        let moved = Self::update_movement(organism, environment, &mut others);
                        if moved {
                            for (original, trial) in
                                before.iter_mut().chain(after.iter_mut()).zip(others)
                            {
                                original.occupied_cells = trial.occupied_cells;
                                original.structure = trial.structure;
                            }
                        }
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
                        let developmental_blueprint =
                            organisms[index].genome.developmental_blueprint.clone();
                        let (blueprint, origin, orientation) = (
                            &developmental_blueprint,
                            (
                                organisms[index].developmental_origin.x,
                                organisms[index].developmental_origin.y,
                            ),
                            organisms[index].developmental_orientation_radians,
                        );
                        let developmental = if matches!(
                            organisms[index].development_stage,
                            DevelopmentStage::Juvenile
                        ) {
                            let (seed_mass, seed_length) =
                                crate::juvenile::confirmed_seed_scale_reference(
                                    &environment.catalog,
                                )
                                .expect("confirmed seed scale reference must be valid");
                            let preferred_length = blueprint.preferred_developmental_length(
                                organisms[index].genome.adult_mass(),
                                seed_mass,
                                seed_length,
                            );
                            Some((blueprint, origin, orientation, preferred_length))
                        } else {
                            None
                        };
                        let combined = crate::combine_runtime::try_combine(
                            &mut organisms[index],
                            environment,
                            &mut compatibility_cache,
                            &mut self.energy_ledger,
                            developmental,
                        )
                        .is_some();
                        crate::decision_runtime::record_outcome(
                            &mut organisms[index].decision_history,
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
                            &mut organisms[index],
                            &environment.catalog,
                            &mut self.next_transformation_id,
                            &selected,
                        ) {
                            self.active_transformations.push(transformation);
                        }
                    }
                    ActionKind::Expel => {
                        let expelled = selected
                            .context_key
                            .as_deref()
                            .and_then(|key| key.strip_prefix("stored:"))
                            .and_then(|index| index.parse::<usize>().ok())
                            .map(|storage_index| {
                                crate::expulsion::expel_physical_material(
                                    &mut organisms[index],
                                    environment,
                                    storage_index,
                                )
                            })
                            .unwrap_or(false);
                        crate::decision_runtime::record_outcome(
                            &mut organisms[index].decision_history,
                            &selected,
                            if expelled {
                                crate::decision::OutcomeKind::Neutral
                            } else {
                                crate::decision::OutcomeKind::Harmful
                            },
                        );
                    }
                }
            }
        }
        let mut offspring = Vec::new();
        let mut next_organism_id = self.next_organism_id;
        for organism in &mut self.organisms {
            if organism.reproductive_construction.is_some() {
                let Some(parent_body) =
                    crate::reproduction::parent_body_geometry(organism, &self.environment.catalog)
                else {
                    continue;
                };
                let (status, stress) = {
                    let construction = organism
                        .reproductive_construction
                        .as_mut()
                        .expect("reproductive construction exists");
                    crate::reproduction::advance_construction(
                        &organism.structure,
                        &mut organism.stored_material,
                        construction,
                        &self.environment,
                        &mut self.energy_ledger,
                        &mut organism.usable_energy,
                        &mut self.rng,
                        &parent_body,
                    )
                };
                if let Some(stress) = stress {
                    organism.add_transaction_stress(stress);
                }
                if matches!(
                    status,
                    crate::reproduction::ConstructionStatus::Ready
                        | crate::reproduction::ConstructionStatus::Detached
                        | crate::reproduction::ConstructionStatus::Dead
                        | crate::reproduction::ConstructionStatus::DeadEnd
                ) {
                    let child_id = next_organism_id.to_string();
                    if let Some(child) = crate::reproduction::finish_reproduction(
                        organism,
                        child_id,
                        &self.environment.catalog,
                        &mut self.energy_ledger,
                    ) {
                        next_organism_id += 1;
                        offspring.push(child);
                    }
                }
            }
        }
        self.next_organism_id = next_organism_id;
        self.organisms.extend(offspring);
        let mut survivors = Vec::with_capacity(self.organisms.len());
        for mut organism in self.organisms.drain(..) {
            let dead = Self::apply_survival_damage(
                &mut organism,
                &self.environment,
                &mut self.energy_ledger,
                &mut self.rng,
            );
            if dead {
                if organism.reproductive_construction.is_some() {
                    let child_id = next_organism_id.to_string();
                    if let Some(child) = crate::reproduction::finish_reproduction(
                        &mut organism,
                        child_id,
                        &self.environment.catalog,
                        &mut self.energy_ledger,
                    ) {
                        next_organism_id += 1;
                        survivors.push(child);
                    }
                }
                if let Some(body) = Self::recycle_dead_organism(
                    &mut self.environment,
                    &mut organism,
                    &mut self.energy_ledger,
                ) {
                    self.decomposing_bodies.push(body);
                }
            } else {
                survivors.push(organism);
            }
        }
        self.organisms = survivors;
        self.process_decomposing_bodies();
        let live_ids: HashSet<String> = self.organisms.iter().map(|o| o.id.clone()).collect();
        self.active_transformations
            .retain(|t| live_ids.contains(&t.organism_id));
        self.energy_ledger.total_usable_energy_held =
            crate::reproduction::total_usable_energy_held(&self.organisms);
    }
    pub(crate) fn apply_survival_damage(
        organism: &mut Organism,
        environment: &Environment,
        ledger: &mut EnergyLedger,
        rng: &mut ChaCha8Rng,
    ) -> bool {
        let threshold = organism
            .stress_threshold
            .max(crate::state::MIN_STRESS_THRESHOLD);
        let lethal_before_decay = organism.stress >= threshold;
        organism.stress *= crate::state::STRESS_DECAY_PER_TICK;
        if organism.structure.bonds.is_empty() && lethal_before_decay {
            return true;
        }
        organism.apply_stress_damage(environment, ledger, rng)
    }
    #[cfg(test)]
    pub(crate) fn total_material_in_system(&self) -> f64 {
        simulation_material_tests::total_material_in_system(self)
    }
}
