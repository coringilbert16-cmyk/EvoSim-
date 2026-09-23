use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::collections::HashSet;

use crate::decision::{
    ActionEligibility, ActionKind, CurrentNeeds, DecisionParameters, OutcomeKind,
};
use crate::decision_runtime::{select_action, ActionCandidate, DecisionContext};
use crate::energy_ledger::EnergyLedgerAuthority;
use crate::environment::{ActiveMaterialField, DEFAULT_CELL_SIZE};
use crate::genome::initial_genome;
use crate::juvenile::realize_initial;
use crate::state::{
    DevelopmentStage, EnergyLedger, Environment, Organism, Position, ResourceSense, Simulation,
};
use crate::transformation::break_candidate_is_executable;

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
        Environment {
            revision: 0,
            width,
            height,
            catalog,
            field,
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
            calculation_cache: crate::state::CalculationCache::default(),
            structure_revision: 0,
            position_revision: 0,
            id: "1".into(),
            developmental_origin: anchor.clone(),
            developmental_orientation_radians: 0.0,
            occupied_cells: vec![anchor],
            genome,
            resource_sense: ResourceSense {
                sensed_resources: Vec::new(),
                direction_x: 0.0,
                direction_y: 0.0,
                direction_strength: 0.0,
            },
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
            cached_cavity_revision: None,
            cached_cavity: None,
            cached_developmental_revision: None,
            cached_developmental_realization: None,
            cached_harmonic_key: None,
        }
    }
    fn growth_fraction(organism: &mut Organism, environment: &Environment) -> f64 {
        let genome_mass = organism.genome.adult_mass();
        let key = (organism.structure_revision, genome_mass.to_bits());
        if let Some((cached_structure, cached_mass, value)) = organism.calculation_cache.growth {
            if (cached_structure, cached_mass) == key {
                return value;
            }
        }
        let value = organism
            .genome
            .developmental_blueprint
            .realization(
                &organism.structure,
                &environment.catalog,
                (
                    organism.developmental_origin.x,
                    organism.developmental_origin.y,
                ),
                organism.developmental_orientation_radians,
                genome_mass,
            )
            .overall
            .clamp(0.0, 1.0);
        organism.calculation_cache.growth = Some((key.0, key.1, value));
        value
    }
    fn update_development_stage(organism: &mut Organism, growth_fraction: f64) {
        match organism.development_stage {
            DevelopmentStage::Offspring => {
                if organism.reproductive_construction.is_none() {
                    organism.development_stage = DevelopmentStage::Juvenile
                }
            }
            DevelopmentStage::Juvenile => {
                if growth_fraction >= ADULTHOOD_GROWTH_FRACTION {
                    organism.development_stage = DevelopmentStage::Adult
                }
            }
            DevelopmentStage::Adult => {}
        }
    }
    fn current_needs(
        organism: &Organism,
        growth_fraction: f64,
        parameters: DecisionParameters,
    ) -> CurrentNeeds {
        let survival_reserve = parameters.survival_reserve.max(f64::EPSILON);
        let reserve_pressure = (1.0 - organism.usable_energy / survival_reserve).clamp(0.0, 1.0);
        let survival = (reserve_pressure * (1.0 + organism.stress.max(0.0))).clamp(0.0, 1.0);
        let development = if matches!(organism.development_stage, DevelopmentStage::Juvenile) {
            (1.0 - growth_fraction.clamp(0.0, 1.0)).max(0.0)
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
    fn acquisition_targets(organism: &Organism, environment: &Environment) -> Vec<usize> {
        let Some(position) = organism.occupied_cells.first() else {
            return Vec::new();
        };
        let Some(field_index) = environment.field.index_for_position(position.x, position.y) else {
            return Vec::new();
        };
        let cell = &environment.field.cells[field_index];
        if cell
            .physical_materials
            .iter()
            .any(|material| material.is_realized() && !material.material.is_empty())
            || cell.materials.iter().any(|material| {
                !material.is_empty() && material.is_valid() && !material.has_internal_structure()
            })
        {
            vec![field_index]
        } else {
            Vec::new()
        }
    }
    fn acquisition_context_key(field_index: usize) -> String {
        format!("target:{field_index}")
    }
    fn action_eligibility(
        organism: &Organism,
        environment: &Environment,
        needs: CurrentNeeds,
        has_executable_break: bool,
    ) -> ActionEligibility {
        let can_build_from_storage =
            !organism.structure.units.is_empty() && !organism.stored_material.is_empty();
        let can_join_existing_structure = organism.structure.units.len() >= 2;
        ActionEligibility {
            can_move: organism.active_transformation_id.is_none(),
            can_acquire: organism.active_transformation_id.is_none()
                && !Self::acquisition_targets(organism, environment).is_empty(),
            can_combine: organism.active_transformation_id.is_none()
                && (can_build_from_storage || can_join_existing_structure),
            can_break: organism.active_transformation_id.is_none()
                && (organism.reproductive_construction.is_none()
                    || needs.survival > 0.0
                    || needs.development > 0.0
                    || organism
                        .reproductive_construction
                        .as_ref()
                        .is_some_and(|construction| construction.needs_space))
                && has_executable_break,
            can_expel: false,
        }
    }
    fn executable_break_candidates(
        organism: &Organism,
        environment: &Environment,
        needs: CurrentNeeds,
    ) -> Vec<usize> {
        let break_allowed = organism.active_transformation_id.is_none()
            && (organism.reproductive_construction.is_none()
                || needs.survival > 0.0
                || needs.development > 0.0
                || organism
                    .reproductive_construction
                    .as_ref()
                    .is_some_and(|construction| construction.needs_space));
        if !break_allowed {
            return Vec::new();
        }
        let key = (
            organism.structure_revision,
            environment.revision,
            organism.usable_energy.to_bits(),
        );
        if let Some((cached_structure, cached_environment, cached_energy, candidates)) =
            &organism.calculation_cache.break_candidates
        {
            if (*cached_structure, *cached_environment, *cached_energy) == key {
                return candidates.clone();
            }
        }
        let candidates: Vec<usize> = organism
            .structure
            .bonds
            .iter()
            .enumerate()
            .filter(|(_, bond)| break_candidate_is_executable(organism, environment, **bond))
            .map(|(index, _)| index)
            .collect();
        organism.calculation_cache.break_candidates = Some((key.0, key.1, key.2, candidates.clone()));
        candidates
    }
    fn decision_candidates(
        organism: &Organism,
        environment: &Environment,
        needs: CurrentNeeds,
        eligibility: ActionEligibility,
        executable_breaks: &[usize],
    ) -> Vec<ActionCandidate> {
        let mut candidates = Vec::new();
        let relevant = |action: ActionKind| {
            eligibility.permits(action) && needs.any_for(action.relevant_needs())
        };
        // Candidate order is the deterministic tie-break order for actions with
        // equal need pressure. Movement comes first so a stationary organism is
        // not systematically diverted into structural transformation when
        // both actions address the same need.
        if relevant(ActionKind::Move) {
            candidates.push(ActionCandidate {
                action: ActionKind::Move,
                context_key: None,
            });
        }
        // Acquisition is an interaction consequence of physical contact with
        // material at the organism's current geometry. It is not a prescribed
        // behavioral role.
        if relevant(ActionKind::Acquire) {
            candidates.extend(
                Self::acquisition_targets(organism, environment)
                    .into_iter()
                    .map(|field_index| ActionCandidate {
                        action: ActionKind::Acquire,
                        context_key: Some(Self::acquisition_context_key(field_index)),
                    }),
            );
        }
        if relevant(ActionKind::Combine) {
            candidates.push(ActionCandidate {
                action: ActionKind::Combine,
                context_key: None,
            });
        }
        if relevant(ActionKind::Expel) {
            candidates.push(ActionCandidate {
                action: ActionKind::Expel,
                context_key: None,
            });
        }
        if relevant(ActionKind::Break) {
            candidates.extend(
                executable_breaks
                    .iter()
                    .copied()
                    .map(|index| ActionCandidate {
                        action: ActionKind::Break,
                        context_key: Some(format!("bond:{index}")),
                    }),
            );
        }
        candidates
    }
    pub(crate) fn acquire_target(
        organism: &mut Organism,
        environment: &mut Environment,
        field_index: usize,
    ) -> bool {
        let Some(position) = organism.occupied_cells.first() else {
            return false;
        };
        let Some(expected_index) = environment.field.index_for_position(position.x, position.y)
        else {
            return false;
        };
        if expected_index != field_index {
            return false;
        }
        if let Some(physical) = environment.field.take_physical_for_acquisition(field_index) {
            let material = physical.material.clone();
            let placements = physical.placements.clone().unwrap_or_default();
            if organism
                .stored_material
                .store_physical(material, placements, &environment.catalog)
            {
                return true;
            }
            let _ = environment
                .field
                .deposit_physical_at_index(field_index, physical);
            return false;
        }
        let Some(material) = environment.field.take_for_acquisition(field_index) else {
            return false;
        };
        organism.store_material(material)
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
        let mut completed = Vec::new();
        if !self.active_transformations.is_empty() {
            let mut still_active = Vec::with_capacity(self.active_transformations.len());
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
        }
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
        let decision_parameters = self.decision_parameters;
        let juvenile_scale_reference =
            crate::juvenile::confirmed_seed_scale_reference(&self.environment.catalog)
                .expect("confirmed seed scale reference must be valid");
        let mut growth_fractions = Vec::with_capacity(self.organisms.len());
        for organism in &mut self.organisms {
            let growth_fraction = if matches!(
                organism.development_stage,
                DevelopmentStage::Offspring | DevelopmentStage::Juvenile
            ) {
                Self::growth_fraction(organism, &self.environment)
            } else {
                1.0
            };
            growth_fractions.push(growth_fraction);
            Self::update_development_stage(organism, growth_fraction);
            organism.apply_maintenance(&self.environment.catalog, &mut self.energy_ledger);
            let perception_key = (
                self.environment.revision,
                organism.position_revision,
                organism.usable_energy.to_bits(),
            );
            if organism.calculation_cache.perception != Some(perception_key) {
                Self::update_resource_perception(organism, &self.environment);
                organism.calculation_cache.perception = Some(perception_key);
            }
            Self::update_memory_from_sources(organism, &self.environment);
            if matches!(organism.development_stage, DevelopmentStage::Adult)
                && organism.reproductive_construction.is_none()
            {
                let _ = crate::reproduction::begin_reproduction(
                    organism,
                    &mut self.rng,
                    &self.environment.catalog,
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
                let needs = Self::current_needs(
                    &organisms[index],
                    growth_fractions[index],
                    decision_parameters,
                );
                let executable_breaks =
                    Self::executable_break_candidates(&organisms[index], environment, needs);
                let eligibility = Self::action_eligibility(
                    &organisms[index],
                    environment,
                    needs,
                    !executable_breaks.is_empty(),
                );
                let context = DecisionContext { needs, eligibility };
                let candidates = Self::decision_candidates(
                    &organisms[index],
                    environment,
                    needs,
                    eligibility,
                    &executable_breaks,
                );
                let Some(selected) =
                    select_action(context, &organisms[index].decision_history, &candidates)
                else {
                    continue;
                };
                match selected.action {
                    ActionKind::Move => {
                        let (before, rest) = organisms.split_at_mut(index);
                        let (organism, after) =
                            rest.split_first_mut().expect("index is in organisms");
                        let moved = Self::update_movement(
                            organism,
                            environment,
                            before,
                            after,
                            &mut self.rng,
                        );
                        crate::decision_runtime::record_outcome(
                            &mut organism.decision_history,
                            &selected,
                            OutcomeKind::Neutral,
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
                            let (seed_mass, seed_length) = juvenile_scale_reference;
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
                        if combined {
                            organisms[index].structure_revision =
                                organisms[index].structure_revision.wrapping_add(1);
                        }
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
                    ActionKind::Acquire => {
                        let success = selected
                            .context_key
                            .as_deref()
                            .and_then(|key| key.strip_prefix("target:"))
                            .and_then(|index| index.parse::<usize>().ok())
                            .map(|field_index| {
                                let success = Self::acquire_target(
                                    &mut organisms[index],
                                    environment,
                                    field_index,
                                );
                                if success {
                                    environment.revision = environment.revision.wrapping_add(1);
                                }
                                success
                            })
                            .unwrap_or(false);
                        crate::decision_runtime::record_outcome(
                            &mut organisms[index].decision_history,
                            &selected,
                            if success {
                                crate::decision::OutcomeKind::Neutral
                            } else {
                                crate::decision::OutcomeKind::Harmful
                            },
                        );
                    }
                    ActionKind::Expel => {}
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
}
