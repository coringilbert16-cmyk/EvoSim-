use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::collections::HashSet;

use crate::energy_ledger::{EnergyLedgerAuthority, ExternalEnergySource, ExternalInputTransaction};

use crate::decision::{ActionEligibility, ActionKind, CurrentNeeds, DecisionParameters};
use crate::decision_runtime::{
    select_action_with_developmental_scores, ActionCandidate, DecisionContext,
};
use crate::environment::{ActiveMaterialField, DEFAULT_CELL_SIZE};
use crate::genome::initial_genome;
use crate::state::{EnergyLedger, Environment, Organism, Position, Simulation};
use crate::structure::Placement;

fn should_prepare_transformation(
    kind: &crate::state::TransformationKind,
    remaining_ticks: u64,
) -> bool {
    match kind {
        crate::state::TransformationKind::Combine => remaining_ticks == 0,
        _ => remaining_ticks == 1,
    }
}

const BUDDING_AVAILABILITY_FRACTION: f64 = 0.90;

fn restore_failed_transformation_material(
    transformation: &crate::state::ActiveTransformation,
    mut organism: Option<&mut Organism>,
    environment: &mut Environment,
) -> bool {
    if let Some(owner) = organism.as_deref_mut() {
        if owner.active_transformation_id == Some(transformation.id) {
            owner.active_transformation_id = None;
        }
    }

    if let Some((_, source)) = transformation.combine_environmental_source.as_ref() {
        if !crate::transformation::restore_combine_environmental_source(transformation, environment)
        {
            if let Some(owner) = organism.as_deref_mut() {
                crate::transformation::quarantine_or_retain_environmental_source(
                    source,
                    owner,
                    environment,
                );
            } else {
                crate::transformation::retain_environmental_escrow_in_field(source, environment);
            }
        }
        return true;
    }

    let Some(source) = transformation.stored_material.as_ref() else {
        return true;
    };
    if transformation.environmental_source {
        if !crate::transformation::restore_environmental_break_source(transformation, environment) {
            if let Some(owner) = organism.as_deref_mut() {
                crate::transformation::quarantine_or_retain_environmental_source(
                    source,
                    owner,
                    environment,
                );
            } else {
                // An owner can disappear while a multi-tick operation is active.
                // Keep the world-space source in the field if exact restoration fails.
                crate::transformation::retain_environmental_escrow_in_field(source, environment);
            }
        }
        return true;
    }

    // A stored-material BREAK temporarily removes its source from the
    // organism's storage when the transformation starts. If preparation fails,
    // return that escrow to its original holder position when metadata is valid.
    // Intrinsic coordinates are not world coordinates and must never be used
    // as a fallback deposit.
    let Some(organism) = organism else {
        // Keep the active transformation as an explicit escrow if its owner
        // disappeared. The caller quarantines it instead of discarding material.
        return false;
    };
    let entry = crate::material_storage::StoredMaterial::Physical(source.clone());
    if let Some(index) = crate::transformation::original_stored_source_index(transformation)
        .filter(|index| *index <= organism.stored_material.entries.len())
    {
        organism.stored_material.entries.insert(index, entry);
    } else {
        // Missing/corrupt index metadata cannot support exact positional rollback.
        // Preserve the escrow at the end rather than silently clamping the index.
        organism.stored_material.entries.push(entry);
    }
    true
}

impl Simulation {
    /// Configure or disable the rotating source. Invalid values are rejected.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "Runtime configuration helper is exercised by focused tests; the live tick uses the configured source cycle"
        )
    )]
    pub(crate) fn configure_world_tone(
        &mut self,
        config: Option<crate::world_tone::WorldToneRuntimeConfig>,
    ) -> bool {
        if config.is_some_and(|candidate| {
            !candidate.is_valid()
                || candidate
                    .sample(0, self.environment.width, self.environment.height)
                    .is_none()
        }) {
            return false;
        }
        self.world_tone = config;
        self.world_tone_cycle = config.and_then(|candidate| {
            crate::world_tone::WorldToneCycle::precompute(
                candidate,
                self.environment.width,
                self.environment.height,
            )
        });
        for organism in &mut self.organisms {
            organism.cached_harmonic_key = None;
        }
        true
    }

    pub(crate) fn new(seed: u64, ticks_per_second: f64) -> Self {
        let rng = ChaCha8Rng::seed_from_u64(seed);
        let environment = Self::create_environment();
        let seed_scale_reference =
            crate::juvenile::confirmed_seed_scale_reference(&environment.catalog).ok();
        let mut organism = Self::create_initial_organism();
        let world_tone = crate::world_tone::WorldToneRuntimeConfig::default_for_world(
            environment.width,
            environment.height,
        )
        .expect("default world tone must remain outside the environment bounds");
        let world_tone_cycle = crate::world_tone::WorldToneCycle::precompute(
            world_tone,
            environment.width,
            environment.height,
        )
        .expect("default world tone cycle must be valid");
        // Treat the starting usable reserve as an explicit, one-time external
        // source transaction instead of silently declaring it as the baseline.
        let initial_usable_energy = organism.usable_energy;
        organism.usable_energy = 0.0;
        let mut energy_ledger = EnergyLedger::default();
        let initial_input = ExternalInputTransaction {
            source: ExternalEnergySource::InitialCondition,
            source_available: initial_usable_energy,
            external_input: initial_usable_energy,
            usable_delta: initial_usable_energy,
            work_expended: 0.0,
            heat_dissipated: 0.0,
        };
        assert!(
            energy_ledger.settle_external_input(&mut organism.usable_energy, initial_input),
            "initial usable energy must enter through a balanced source transaction"
        );
        energy_ledger.total_usable_energy_held =
            crate::reproduction::total_usable_energy_held(std::slice::from_ref(&organism), &[]);
        energy_ledger.initial_usable_energy_held = 0.0;
        energy_ledger.total_resonant_energy_held =
            crate::reproduction::total_resonant_energy_held(std::slice::from_ref(&organism));
        energy_ledger.initial_resonant_energy_held = energy_ledger.total_resonant_energy_held;
        energy_ledger.total_structural_energy_held = organism
            .structure
            .bonds
            .iter()
            .map(|bond| bond.bond_energy)
            .sum();
        energy_ledger.initial_structural_energy_held = energy_ledger.total_structural_energy_held;
        Self {
            tick: 0,
            ticks_per_second,
            running: false,
            organisms: vec![organism],
            environment,
            active_transformations: Vec::new(),
            chemical_reaction_accumulation: std::collections::BTreeMap::new(),
            chemistry_library: crate::chemistry_library::open_default_library()
                .expect("chemistry library must open"),
            decomposing_bodies: Vec::new(),
            energy_ledger,
            next_organism_id: 2,
            next_transformation_id: 1,
            rng,
            decision_parameters: DecisionParameters::default(),
            seed_scale_reference,
            world_tone: Some(world_tone),
            world_tone_cycle: Some(world_tone_cycle),
        }
    }
    fn create_environment() -> Environment {
        let catalog = crate::resources::default_catalog();
        let width = 1000.0;
        let height = 1000.0;
        let mut field = ActiveMaterialField::new(width, height, DEFAULT_CELL_SIZE);
        crate::environmental_materials::seed_initial_landscape(&mut field, &catalog);
        crate::environmental_materials::seed_initial_resource_cloud(&mut field, &catalog);
        Environment {
            width,
            height,
            catalog,
            field,
        }
    }
    pub(crate) fn create_initial_organism() -> Organism {
        let genome = initial_genome();
        let catalog = crate::resources::default_catalog();
        let construction = crate::initial_organism_constructor::construct_valid(&catalog)
            .expect("initial organism constructor must find a valid organism");
        let mut structure = construction.structure;
        let acquired_resource_shapes = construction.acquired_resource_shapes;
        let fluid_geometry_status = construction.fluid_geometry_status;
        debug_assert_eq!(
            fluid_geometry_status
                == crate::initial_organism_constructor::FluidGeometryStatus::BobSuggestionAccepted,
            !acquired_resource_shapes.is_empty(),
            "only live-validated Bob suggestions may attach an acquired fluid shape"
        );

        let anchor = Position { x: 500.0, y: 500.0 };
        for unit in &mut structure.units {
            unit.placement.x += anchor.x;
            unit.placement.y += anchor.y;
        }

        let mut stored_material = crate::material_storage::MaterialStorage::default();
        for (name, mut placement) in construction.acquired_resource_placements {
            placement.x += anchor.x;
            placement.y += anchor.y;
            let physical = crate::physical_material::PhysicalMaterial::realized(
                crate::resources::Material::free_base(name.clone(), 1.0),
                vec![placement],
                &catalog,
            )
            .expect("constructor-validated resource must have a physical realization");
            let physical = match acquired_resource_shapes
                .iter()
                .find(|(shape_name, _)| shape_name == &name)
            {
                Some((_, shape)) => physical
                    .with_single_fluid_shape(shape.clone(), &catalog)
                    .expect("constructor-validated fluid shape must preserve resource area"),
                None => physical,
            };
            assert!(stored_material.store_physical_instance_at_owner_anchor(
                physical,
                Placement {
                    x: anchor.x,
                    y: anchor.y,
                    rotation_radians: 0.0,
                },
            ));
        }

        let construction_cost = (1.0e12 - construction.energy).max(0.0);
        let initial_energy = construction_cost + genome.reproductive_energy_allocation;

        Organism {
            id: "1".into(),
            developmental_origin: anchor.clone(),
            developmental_orientation_radians: 0.0,
            occupied_cells: vec![anchor],
            genome,
            harmonic_spectrum: crate::harmonics::ToneSpectrum::empty(),
            resonant_energy: crate::harmonics::ResonantEnergyReservoir::default(),
            experience_memory: crate::memory::ExperienceMemory::default(),
            pending_movement_experience: None,
            decision_history: crate::decision::DecisionHistory::default(),
            usable_energy: initial_energy,
            stress: 0.0,
            maintenance_debt: 0.0,
            stress_threshold: crate::state::INITIAL_STRESS_THRESHOLD,
            stored_material,
            structure,
            genome_core_identity_established: false,
            active_transformation_id: None,
            active_movement: None,
            reproductive_construction: None,
            structure_revision: 0,
            position_revision: 0,
            cached_cavity_revision: None,
            cached_cavity: None,
            cached_developmental_revision: None,
            cached_developmental_realization: None,
            peak_developmental_realization: 0.0,
            cached_harmonic_key: None,
            last_movement_attempt: None,
        }
    }
    fn record_action_experience(
        organism: &mut Organism,
        environment: &Environment,
        perceptions: &[crate::harmonics::ResonancePerception],
        action: ActionKind,
        consequence: crate::decision::ActionConsequence,
        needs: CurrentNeeds,
        material_consumed: f64,
    ) {
        let capacity = organism
            .genome_cavity_cached_ref(&environment.catalog)
            .filter(|cavity| cavity.qualifies())
            .map(crate::memory::memory_capacity);
        let Some(capacity) = capacity else {
            return;
        };
        let mut memory_consequence = crate::memory::memory_consequence_from_action(consequence);
        memory_consequence.material_consumed = material_consumed.max(0.0);
        crate::memory::record_experience(
            &mut organism.experience_memory,
            perceptions,
            action,
            memory_consequence,
            needs,
            capacity,
            organism.genome.memory_strength(),
        );
    }

    fn finalize_pending_movement_experience(
        organism: &mut Organism,
        environment: &Environment,
        acquired_amount: f64,
    ) {
        let Some(pending) = organism.pending_movement_experience.take() else {
            return;
        };
        let mut consequence = pending.consequence;
        consequence.material_acquired = acquired_amount.max(0.0);
        let capacity = organism
            .genome_cavity_cached_ref(&environment.catalog)
            .filter(|cavity| cavity.qualifies())
            .map(crate::memory::memory_capacity);
        let Some(capacity) = capacity else {
            return;
        };
        crate::memory::record_experience(
            &mut organism.experience_memory,
            &pending.perceptions,
            ActionKind::Move,
            consequence,
            pending.needs,
            capacity,
            organism.genome.memory_strength(),
        );
    }

    fn action_consequence(
        before_energy: f64,
        before_stress: f64,
        before_realization: f64,
        organism: &mut Organism,
        environment: &Environment,
    ) -> crate::decision::ActionConsequence {
        let after_realization = organism
            .developmental_realization_cached(&environment.catalog)
            .map(|realization| realization.overall)
            .unwrap_or(before_realization);
        crate::decision::ActionConsequence {
            energy_delta: organism.usable_energy - before_energy,
            stress_delta: organism.stress - before_stress,
            developmental_delta: after_realization - before_realization,
        }
    }

    pub(crate) fn transfer_contained_environmental_material(
        organism: &mut Organism,
        environment: &mut Environment,
    ) {
        // A multi-tick transformation may hold an escrowed storage index or a
        // prepared storage snapshot. Acquiring contained material while it is
        // in flight can shift a stored-BREAK rollback index or let a prepared
        // COMBINE snapshot overwrite newly acquired material. Defer acquisition
        // until the transformation resolves so the transaction's holder view
        // remains isolated for its full lifetime.
        if organism.active_transformation_id.is_some() {
            return;
        }

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
        let Ok(interior_regions) = crate::interior_geometry::find_accessible_interior_regions(
            &organism.structure,
            &environment.catalog,
        ) else {
            return;
        };
        // Gather only the affected physical objects and their exact source
        // indices. Preflight storage conversion before mutating either holder;
        // this avoids cloning the entire world field or inventory.
        let candidate_cells = environment
            .field
            .cells_intersecting_bounds(body.min_x, body.max_x, body.min_y, body.max_y);
        let mut source_indices = Vec::new();
        let mut contained = Vec::new();
        for cell_index in candidate_cells {
            let Some(cell) = environment.field.cells.get(cell_index) else {
                continue;
            };
            for (physical_index, physical) in cell.physical_materials.iter().enumerate() {
                if crate::environment::ActiveMaterialField::physical_is_fully_inside_any_region(
                    physical,
                    &interior_regions,
                    &environment.catalog,
                ) {
                    source_indices.push((cell_index, physical_index));
                    contained.push(physical.clone());
                }
            }
        }
        if contained.is_empty() {
            return;
        }

        let Some(prepared_entries) =
            crate::material_storage::MaterialStorage::prepare_physical_batch_at_owner_anchor(
                contained,
                Placement {
                    x: anchor.x,
                    y: anchor.y,
                    rotation_radians: 0.0,
                },
            )
        else {
            // The field remains untouched if any member of the batch cannot be
            // represented faithfully in storage.
            return;
        };

        // Remove from highest index to lowest so each recorded source index
        // remains valid. No other mutation can interleave within this call.
        source_indices.sort_unstable_by(|left, right| right.cmp(left));
        for (cell_index, physical_index) in source_indices {
            environment.field.cells[cell_index]
                .physical_materials
                .remove(physical_index);
            environment.field.mark_changed_at_index(cell_index);
        }
        organism
            .stored_material
            .commit_prepared_physical_batch(prepared_entries);
    }

    fn current_needs(
        organism: &mut Organism,
        environment: &Environment,
        parameters: DecisionParameters,
        developmental: Option<&crate::developmental_decision::DevelopmentalContext>,
    ) -> CurrentNeeds {
        let survival_reserve = parameters.survival_reserve.max(f64::EPSILON);
        let reserve_pressure = (1.0 - organism.usable_energy / survival_reserve).clamp(0.0, 1.0);
        let current_realization = developmental
            .map(|context| context.current_growth_fraction)
            .or_else(|| {
                organism
                    .developmental_realization_cached(&environment.catalog)
                    .map(|realization| realization.overall)
            })
            .unwrap_or(0.0)
            .clamp(0.0, 1.0);
        if organism.peak_developmental_realization <= 0.0 {
            organism.peak_developmental_realization = current_realization;
        } else {
            organism.peak_developmental_realization = organism
                .peak_developmental_realization
                .max(current_realization);
        }
        let self_maintenance_pressure =
            (organism.peak_developmental_realization - current_realization).clamp(0.0, 1.0);
        let energy_survival = (reserve_pressure * (1.0 + organism.stress.max(0.0))).clamp(0.0, 1.0);
        // Survival is first the pressure to preserve the organism's realized
        // physical self. Energy fills the remaining survival deficit rather
        // than replacing that structural pressure.
        let survival =
            self_maintenance_pressure + (1.0 - self_maintenance_pressure) * energy_survival;
        let mut development = if developmental.is_some() {
            (1.0 - current_realization).max(0.0)
        } else {
            0.0
        };
        // A developmental material mismatch contributes to continuous structural-development pressure:
        // the organism should seek material that matches its inherited
        // structural preference instead of accepting a structurally poor
        // substitute merely because one exists.
        if let Some(construction) = organism.reproductive_construction.as_ref() {
            development =
                development.max(crate::reproduction::construction_material_need_pressure(
                    construction,
                    &environment.catalog,
                ));
        }
        CurrentNeeds {
            survival,
            reproduction: if current_realization > 0.90 { 1.0 } else { 0.0 },
            development,
        }
    }
    fn action_eligibility(
        organism: &Organism,
        environment: &Environment,
        needs: CurrentNeeds,
    ) -> ActionEligibility {
        let can_combine = crate::combine_runtime::can_combine(organism, environment);
        let can_break_stored = organism.active_transformation_id.is_none()
            && organism.stored_material.entries.iter().any(|entry| {
                matches!(
                    entry,
                    crate::material_storage::StoredMaterial::Physical(instance)
                        if instance.is_realized()
                            && instance
                                .internal_connections
                                .as_ref()
                                .is_some_and(|connections| !connections.is_empty())
                )
            });
        let can_break_environment = organism.active_transformation_id.is_none()
            && crate::transformation::has_environmental_break_candidate(organism, environment);
        ActionEligibility {
            can_move: true,
            can_combine,
            can_break: (can_break_stored || can_break_environment)
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
            for (storage_index, entry) in organism.stored_material.entries.iter().enumerate() {
                let crate::material_storage::StoredMaterial::Physical(instance) = entry;
                if let Some(connections) = &instance.internal_connections {
                    for bond_index in 0..connections.len() {
                        candidates.push(ActionCandidate {
                            action: ActionKind::Break,
                            context_key: Some(format!("stored:{storage_index}:bond:{bond_index}")),
                        });
                    }
                }
            }
            candidates.extend(crate::transformation::environmental_break_candidates(
                organism,
                _environment,
            ));
        }
        if relevant(ActionKind::Combine) {
            candidates.push(ActionCandidate {
                action: ActionKind::Combine,
                context_key: None,
            });
        }
        candidates
    }

    fn expulsion_candidates(
        organism: &Organism,
        needs: CurrentNeeds,
        eligibility: ActionEligibility,
    ) -> Vec<ActionCandidate> {
        if !eligibility.permits(ActionKind::Expel)
            || !needs.any_for(ActionKind::Expel.relevant_needs())
        {
            return Vec::new();
        }
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
            })
            .collect()
    }
    #[cfg(test)]
    fn process_decomposing_bodies(&mut self) {
        let mut finished_indices = Vec::new();
        for index in 0..self.decomposing_bodies.len() {
            if self.decomposing_bodies[index].is_finished() {
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
            // Released usable energy remains with the decomposing body.
            // Distance alone is not an authorized transfer mechanism; a
            // receiver must obtain it through a real, chemistry-authorized path.
            if step.released_material.is_some() {
                finished_indices.push(index);
            }
        }
        for index in finished_indices.into_iter().rev() {
            let remaining = self.decomposing_bodies[index].energy_budget;
            if !remaining.is_finite() || remaining < 0.0 {
                // Preserve the holder for diagnosis rather than deleting an
                // invalid energy balance. Crucially, do not release its
                // material until this body's finalization can commit.
                continue;
            }
            if remaining > 0.0 {
                // A finished body cannot be removed while it still owns usable
                // energy. Until an authorized receiver/path exists, explicitly
                // settle the remainder as a sink rather than silently losing it.
                let sink = crate::energy_ledger::EnergyTransaction {
                    reason: crate::energy_ledger::EnergyReason::Decomposition,
                    potential_released: 0.0,
                    usable_delta: -remaining,
                    structural_delta: 0.0,
                    work_expended: 0.0,
                    heat_dissipated: remaining,
                };
                if !self
                    .energy_ledger
                    .settle_transaction(&mut self.decomposing_bodies[index].energy_budget, sink)
                {
                    continue;
                }
            }

            // Material release and holder removal happen only after the energy
            // sink succeeds. Otherwise a retained body would re-release the
            // same units on a later tick.
            let position = self.decomposing_bodies[index].position.clone();
            let Some(materials) =
                self.decomposing_bodies[index].release_finished_material(&self.environment.catalog)
            else {
                // Keep the holder intact if any constituent cannot be released
                // with its authoritative material and geometry.
                continue;
            };
            let deposits = materials
                .into_iter()
                .map(|material| (position.x, position.y, material))
                .collect();
            if !self.environment.field.deposit_physical_batch(deposits) {
                // A failed destination must not discard the body's remaining
                // physical ownership. Keep the finished holder for a later retry.
                continue;
            }
            self.decomposing_bodies.remove(index);
        }
    }
    fn has_intact_genome_core(
        organism: &mut Organism,
        catalog: &[crate::resources::BaseResource],
    ) -> bool {
        if let Some(cavity) = organism.genome_cavity_cached_ref(catalog) {
            return cavity.qualifies();
        }
        // The cache API currently collapses analysis errors and a missing
        // cavity. Distinguish them here so malformed geometry is not silently
        // classified as death.
        match crate::cavity::analyze_genome_cavity(&organism.structure, catalog) {
            Ok(Some(cavity)) => cavity.qualifies(),
            Ok(None) => false,
            Err(_) => true,
        }
    }

    pub(crate) fn living_population(&self) -> usize {
        self.organisms
            .iter()
            .filter(|organism| {
                match crate::cavity::analyze_genome_cavity(
                    &organism.structure,
                    &self.environment.catalog,
                ) {
                    Ok(Some(cavity)) => cavity.qualifies(),
                    Ok(None) => false,
                    // A geometry-analysis error is not proof that the core was breached.
                    Err(_) => true,
                }
            })
            .count()
    }

    pub(crate) fn step(&mut self) {
        self.tick += 1;

        let mut still_active = Vec::new();
        let mut completed = Vec::new();
        for mut transformation in self.active_transformations.drain(..) {
            if transformation.remaining_ticks > 0 {
                transformation.remaining_ticks -= 1
            }
            if should_prepare_transformation(&transformation.kind, transformation.remaining_ticks) {
                if let Some(organism) = self
                    .organisms
                    .iter_mut()
                    .find(|o| o.id == transformation.organism_id)
                {
                    if !Self::prepare_transformation(
                        &mut transformation,
                        organism,
                        &mut self.environment,
                        &mut self.energy_ledger,
                        self.seed_scale_reference,
                    ) {
                        restore_failed_transformation_material(
                            &transformation,
                            Some(organism),
                            &mut self.environment,
                        );
                        continue;
                    }
                } else {
                    if !restore_failed_transformation_material(
                        &transformation,
                        None,
                        &mut self.environment,
                    ) {
                        // Stored material has no world-space location. Keep the
                        // transaction and its escrow intact until its owner can
                        // be resolved instead of silently dropping the material.
                        still_active.push(transformation);
                    }
                    continue;
                }
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
                crate::transformation::resolve_transformation(
                    transformation,
                    organism,
                    &mut self.environment,
                    &mut self.energy_ledger,
                );
            }
        }
        // A completed physical/chemical transformation may itself breach the
        // core, so refresh the observed living set before any biological work.
        let mut living_ids: HashSet<String> = self
            .organisms
            .iter_mut()
            .filter_map(|organism| {
                Self::has_intact_genome_core(organism, &self.environment.catalog)
                    .then(|| organism.id.clone())
            })
            .collect();

        let decision_parameters = self.decision_parameters;
        // The confirmed seed scale is deterministic for the catalog. Compute it
        // once per simulation tick instead of rebuilding the calibration
        // structure for every organism.
        let seed_reference = self.seed_scale_reference;
        let world_tone_sample = self
            .world_tone_cycle
            .as_ref()
            .and_then(|cycle| cycle.sample(self.tick))
            .or_else(|| {
                self.world_tone.and_then(|config| {
                    config.sample(self.tick, self.environment.width, self.environment.height)
                })
            });
        if let Some(sample) = world_tone_sample {
            crate::world_tone_snapshot::WorldToneOccluderSnapshot::settle_living_organism_inputs(
                self,
                sample,
                &living_ids,
            );
        }
        for organism in &mut self.organisms {
            let is_living = living_ids.contains(&organism.id);
            if is_living {
                organism.apply_maintenance(&self.environment.catalog, &mut self.energy_ledger);
                crate::harmonics::update_organism_harmonics(
                    organism,
                    &self.environment,
                    world_tone_sample,
                    self.tick,
                );
                crate::memory::update_experience_memory(organism, &self.environment);
                let budding_available = seed_reference
                    .and_then(|reference| {
                        organism.developmental_realization_cached_for_reference(
                            &self.environment.catalog,
                            reference,
                        )
                    })
                    .is_some_and(|realization| realization.overall > BUDDING_AVAILABILITY_FRACTION);
                if budding_available && organism.reproductive_construction.is_none() {
                    let _ = crate::reproduction::begin_reproduction(
                        organism,
                        &mut self.rng,
                        &self.environment.catalog,
                        &mut self.energy_ledger,
                    );
                }
            }
            // Chemical reactions remain part of the physical world after
            // organism status ends; biological maintenance and memory do not.
            let chemical_operations = crate::chemical_reaction::accumulate(
                organism,
                &self.environment.catalog,
                &mut self.chemical_reaction_accumulation,
                &mut self.chemistry_library,
            );
            // Chemistry events have no artificial duration or per-organism
            // serialization. Every event that still matches the current
            // physical structure is applied immediately; conflicting events
            // naturally fail once an earlier event has changed that structure.
            for operation in chemical_operations {
                let _ = crate::chemical_reaction::resolve_preserving_accumulation(
                    operation,
                    organism,
                    &mut self.energy_ledger,
                    &mut self.chemical_reaction_accumulation,
                );
            }

            // Recheck immediately after chemistry: if a reaction breached the
            // genome core this tick, do not perform any later biological work.
            let core_intact_after_chemistry =
                Self::has_intact_genome_core(organism, &self.environment.catalog);
            if !core_intact_after_chemistry {
                living_ids.remove(&organism.id);
            }
            if core_intact_after_chemistry {
                let stored_amount_before_transfer = organism.stored_material.total_amount();
                Self::transfer_contained_environmental_material(organism, &mut self.environment);
                let acquired_amount = (organism.stored_material.total_amount()
                    - stored_amount_before_transfer)
                    .max(0.0);
                if organism.active_movement.is_none() {
                    Self::finalize_pending_movement_experience(
                        organism,
                        &self.environment,
                        acquired_amount,
                    );
                }
            }
        }
        {
            let (organisms, environment) = (&mut self.organisms, &mut self.environment);
            let mut movement_spatial_index =
                crate::movement::MovementSpatialIndex::new(organisms, environment);
            for index in 0..organisms.len() {
                if !living_ids.contains(&organisms[index].id)
                    || completed_organisms.contains(&organisms[index].id)
                {
                    continue;
                }
                let developmental = seed_reference.and_then(|reference| {
                    crate::developmental_decision::context(
                        &mut organisms[index],
                        environment,
                        reference,
                    )
                });
                let needs = Self::current_needs(
                    &mut organisms[index],
                    environment,
                    decision_parameters,
                    developmental.as_ref(),
                );
                let eligibility = Self::action_eligibility(&organisms[index], environment, needs);
                // Movement is a persistent operation. A decision starts it;
                // each later progress tick advances exactly one world unit.
                let mut movement_operation_active = false;
                {
                    let (before, rest) = organisms.split_at_mut(index);
                    let (organism, after) = rest.split_first_mut().expect("index is in organisms");

                    if let Some(mut active) = organism.active_movement.take() {
                        movement_operation_active = true;
                        let before_energy = active.before_energy;
                        let before_stress = active.before_stress;
                        let before_realization = active.before_developmental_realization;
                        let progress = Self::advance_movement(
                            before,
                            organism,
                            after,
                            &mut movement_spatial_index,
                            environment,
                            &mut self.energy_ledger,
                            self.tick,
                            &mut active,
                        );
                        match progress {
                            Ok(crate::movement::MovementProgress::Waiting)
                            | Ok(crate::movement::MovementProgress::Moved) => {
                                organism.active_movement = Some(active);
                            }
                            Ok(crate::movement::MovementProgress::Complete) => {
                                let consequence = Self::action_consequence(
                                    before_energy,
                                    before_stress,
                                    before_realization,
                                    organism,
                                    environment,
                                );
                                let move_candidate = ActionCandidate {
                                    action: ActionKind::Move,
                                    context_key: Some(format!(
                                        "distance:{:.0}",
                                        active.decision_distance
                                    )),
                                };
                                crate::decision_runtime::record_consequence(
                                    &mut organism.decision_history,
                                    &move_candidate,
                                    consequence,
                                );
                                if let Some(pending) = organism.pending_movement_experience.as_mut()
                                {
                                    pending.consequence =
                                        crate::memory::memory_consequence_from_action(consequence);
                                }
                            }
                            Err(_) => {
                                let consequence = Self::action_consequence(
                                    before_energy,
                                    before_stress,
                                    before_realization,
                                    organism,
                                    environment,
                                );
                                let move_candidate = ActionCandidate {
                                    action: ActionKind::Move,
                                    context_key: Some(format!(
                                        "distance:{:.0}",
                                        active.decision_distance
                                    )),
                                };
                                crate::decision_runtime::record_consequence(
                                    &mut organism.decision_history,
                                    &move_candidate,
                                    consequence,
                                );
                                organism.pending_movement_experience = None;
                            }
                        }
                    } else if eligibility.can_move {
                        let perceptions =
                            crate::harmonics::organism_resonance_perceptions(organism, environment);
                        let before_energy = organism.usable_energy;
                        let before_stress = organism.stress;
                        let before_realization = developmental
                            .as_ref()
                            .map(|context| context.current_growth_fraction)
                            .unwrap_or_else(|| {
                                organism
                                    .developmental_realization_cached(&environment.catalog)
                                    .map(|realization| realization.overall)
                                    .unwrap_or(0.0)
                            });
                        match Self::start_movement(
                            organism,
                            environment,
                            &mut self.rng,
                            &perceptions,
                            needs,
                        ) {
                            Ok(mut active) => {
                                active.before_energy = before_energy;
                                active.before_stress = before_stress;
                                active.before_developmental_realization = before_realization;
                                let consequence = crate::decision::ActionConsequence::default();
                                organism.pending_movement_experience =
                                    Some(crate::memory::PendingMovementExperience {
                                        perceptions,
                                        consequence: crate::memory::memory_consequence_from_action(
                                            consequence,
                                        ),
                                        needs,
                                    });
                                organism.active_movement = Some(active);
                                movement_operation_active = true;
                            }
                            Err(reason) => {
                                organism.last_movement_attempt =
                                    Some(crate::state::MovementAttemptDiagnostic {
                                        tick: self.tick,
                                        direction_x: None,
                                        direction_y: None,
                                        step: None,
                                        usable_energy: organism.usable_energy,
                                        active_transformation_id: organism.active_transformation_id,
                                        result: Err(reason),
                                        old_position: organism.occupied_cells.first().cloned(),
                                        new_position: None,
                                    });
                            }
                        }
                    }
                }
                if movement_operation_active {
                    continue;
                }
                let context = DecisionContext { needs, eligibility };
                let candidates =
                    Self::decision_candidates(&organisms[index], environment, needs, eligibility);
                let competing_indices = crate::decision_runtime::developmental_competition_indices(
                    context,
                    &organisms[index].decision_history,
                    &candidates,
                );
                let developmental_scores =
                    crate::developmental_decision::developmental_action_scores(
                        &organisms[index],
                        environment,
                        needs,
                        &candidates,
                        &competing_indices,
                        developmental.as_ref(),
                        &self.energy_ledger,
                    );
                if let Some(selected) = select_action_with_developmental_scores(
                    context,
                    &organisms[index].decision_history,
                    &candidates,
                    &developmental_scores,
                    &mut self.rng,
                ) {
                    match selected.action {
                        ActionKind::Combine => {
                            let perceptions = crate::harmonics::organism_resonance_perceptions(
                                &organisms[index],
                                environment,
                            );
                            let before_energy = organisms[index].usable_energy;
                            let before_stress = organisms[index].stress;
                            let before_realization = developmental
                                .as_ref()
                                .map(|context| context.current_growth_fraction)
                                .unwrap_or_else(|| {
                                    organisms[index]
                                        .developmental_realization_cached(&environment.catalog)
                                        .map(|realization| realization.overall)
                                        .unwrap_or(0.0)
                                });
                            let mut cache = crate::contact::ConnectionCompatibilityCache;
                            let mut ledger = self.energy_ledger;
                            let developmental_context = developmental.as_ref().map(|context| {
                                (
                                    &context.blueprint,
                                    context.origin,
                                    context.orientation,
                                    context.preferred_length,
                                )
                            });
                            let attempt = crate::combine_runtime::try_combine(
                                &mut organisms[index],
                                environment,
                                &mut cache,
                                &mut ledger,
                                developmental_context,
                            );
                            if let Some(attempt) = attempt {
                                self.energy_ledger = ledger;
                                let consequence = Self::action_consequence(
                                    before_energy,
                                    before_stress,
                                    before_realization,
                                    &mut organisms[index],
                                    environment,
                                );
                                crate::decision_runtime::record_consequence(
                                    &mut organisms[index].decision_history,
                                    &selected,
                                    consequence,
                                );
                                Self::record_action_experience(
                                    &mut organisms[index],
                                    environment,
                                    &perceptions,
                                    ActionKind::Combine,
                                    consequence,
                                    needs,
                                    attempt
                                        .environmental_source
                                        .as_ref()
                                        .map(|(_, source)| source.material.total_amount())
                                        .unwrap_or(0.0),
                                );
                            }
                        }
                        ActionKind::Break => {
                            let perceptions = crate::harmonics::organism_resonance_perceptions(
                                &organisms[index],
                                environment,
                            );
                            let before_energy = organisms[index].usable_energy;
                            let before_stress = organisms[index].stress;
                            let before_developmental_realization = developmental
                                .as_ref()
                                .map(|context| context.current_growth_fraction)
                                .unwrap_or_else(|| {
                                    organisms[index]
                                        .developmental_realization_cached(&environment.catalog)
                                        .map(|realization| realization.overall)
                                        .unwrap_or(0.0)
                                });
                            if let Some(mut transformation) =
                                crate::transformation::try_start_environmental_break(
                                    &mut organisms[index],
                                    environment,
                                    &mut self.next_transformation_id,
                                    &selected,
                                )
                                .or_else(|| {
                                    Self::try_start_transformation(
                                        &mut organisms[index],
                                        &environment.catalog,
                                        &mut self.next_transformation_id,
                                        &selected,
                                    )
                                })
                            {
                                transformation.pending_experience =
                                    Some(crate::memory::PendingTransformationExperience {
                                        perceptions,
                                        needs,
                                        before_energy,
                                        before_stress,
                                        before_developmental_realization,
                                        material_transformed: transformation
                                            .stored_material
                                            .as_ref()
                                            .map(|instance| instance.material.total_amount())
                                            .unwrap_or(0.0),
                                    });
                                self.active_transformations.push(transformation);
                            }
                        }
                        ActionKind::Expel => {
                            let perceptions = crate::harmonics::organism_resonance_perceptions(
                                &organisms[index],
                                environment,
                            );
                            let before_energy = organisms[index].usable_energy;
                            let before_stress = organisms[index].stress;
                            let before_realization = developmental
                                .as_ref()
                                .map(|context| context.current_growth_fraction)
                                .unwrap_or_else(|| {
                                    organisms[index]
                                        .developmental_realization_cached(&environment.catalog)
                                        .map(|realization| realization.overall)
                                        .unwrap_or(0.0)
                                });
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
                            let consequence = if expelled {
                                Self::action_consequence(
                                    before_energy,
                                    before_stress,
                                    before_realization,
                                    &mut organisms[index],
                                    environment,
                                )
                            } else {
                                crate::decision::ActionConsequence::default()
                            };
                            crate::decision_runtime::record_consequence(
                                &mut organisms[index].decision_history,
                                &selected,
                                consequence,
                            );
                            if expelled {
                                Self::record_action_experience(
                                    &mut organisms[index],
                                    environment,
                                    &perceptions,
                                    ActionKind::Expel,
                                    consequence,
                                    needs,
                                    0.0,
                                );
                            }
                        }
                        ActionKind::Move => unreachable!("movement is evaluated independently"),
                    }
                } else {
                    let before_energy = organisms[index].usable_energy;
                    let before_stress = organisms[index].stress;
                    let before_realization = developmental
                        .as_ref()
                        .map(|context| context.current_growth_fraction)
                        .unwrap_or_else(|| {
                            organisms[index]
                                .developmental_realization_cached(&environment.catalog)
                                .map(|realization| realization.overall)
                                .unwrap_or(0.0)
                        });
                    let expulsion_candidates =
                        Self::expulsion_candidates(&organisms[index], needs, eligibility);
                    let expulsion_competition =
                        crate::decision_runtime::developmental_competition_indices(
                            context,
                            &organisms[index].decision_history,
                            &expulsion_candidates,
                        );
                    let expulsion_scores =
                        crate::developmental_decision::developmental_action_scores(
                            &organisms[index],
                            environment,
                            needs,
                            &expulsion_candidates,
                            &expulsion_competition,
                            developmental.as_ref(),
                            &self.energy_ledger,
                        );
                    if let Some(selected) = select_action_with_developmental_scores(
                        context,
                        &organisms[index].decision_history,
                        &expulsion_candidates,
                        &expulsion_scores,
                        &mut self.rng,
                    ) {
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
                        let consequence = if expelled {
                            Self::action_consequence(
                                before_energy,
                                before_stress,
                                before_realization,
                                &mut organisms[index],
                                environment,
                            )
                        } else {
                            crate::decision::ActionConsequence::default()
                        };
                        crate::decision_runtime::record_consequence(
                            &mut organisms[index].decision_history,
                            &selected,
                            consequence,
                        );
                    }
                }
            }
        }
        let mut offspring = Vec::new();
        let mut next_organism_id = self.next_organism_id;
        for organism in &mut self.organisms {
            if !living_ids.contains(&organism.id) {
                continue;
            }
            // Construction consumes the same stored-material holder used by
            // BREAK/COMBINE transactions. Pause it while a transformation owns
            // an escrow or prepared storage snapshot; resume after resolution.
            if organism.reproductive_construction.is_some()
                && organism.active_transformation_id.is_none()
            {
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
                        self.seed_scale_reference,
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
                ) {
                    let child_id = next_organism_id.to_string();
                    if let Some(child) = crate::reproduction::finish_reproduction(
                        organism,
                        child_id,
                        &self.environment.catalog,
                        &mut self.energy_ledger,
                        self.seed_scale_reference,
                    ) {
                        next_organism_id += 1;
                        offspring.push(child);
                    }
                }
            }
        }
        self.next_organism_id = next_organism_id;
        self.organisms.extend(offspring);
        // Stress can break the core through ordinary bond-breaking chemistry.
        // A breached organism remains in the physical world; only living
        // organisms receive biological stress processing on subsequent ticks.
        for organism in &mut self.organisms {
            if living_ids.contains(&organism.id) {
                let _ = Self::apply_survival_damage(
                    organism,
                    &self.environment,
                    &mut self.energy_ledger,
                    &mut self.rng,
                );
            }
        }
        // Death is observational: breached organisms remain physical structures
        // and continue through the same ordinary chemical-reaction pass above.
        // Do not invoke the legacy DecomposingBody bond-breaking path here.

        // Keep in-flight physical/chemical transactions even if their organism's
        // core has since been breached. Dropping one here could discard reserved
        // material or energy held by the transaction. New transformations cannot
        // be initiated by breached organisms because the decision loop is gated
        // by living_ids. Existing BREAK/COMBINE transactions still settle normally.
        self.energy_ledger.total_usable_energy_held = crate::reproduction::total_usable_energy_held(
            &self.organisms,
            &self.decomposing_bodies,
        );
        self.energy_ledger.total_resonant_energy_held =
            crate::reproduction::total_resonant_energy_held(&self.organisms);
        self.energy_ledger.total_structural_energy_held =
            Self::structural_energy_held(&self.organisms, &self.decomposing_bodies);
    }
    pub(crate) fn structural_energy_held(
        organisms: &[crate::state::Organism],
        decomposing_bodies: &[crate::decomposition::DecomposingBody],
    ) -> f64 {
        fn graph_energy(structure: &crate::structure::OrganismStructure) -> f64 {
            structure.bonds.iter().map(|bond| bond.bond_energy).sum()
        }

        let organism_energy: f64 = organisms
            .iter()
            .map(|organism| {
                graph_energy(&organism.structure)
                    + organism
                        .reproductive_construction
                        .as_ref()
                        .map_or(0.0, |construction| {
                            graph_energy(&construction.developing_structure)
                        })
            })
            .sum();
        let decomposition_energy: f64 = decomposing_bodies
            .iter()
            .map(|body| graph_energy(&body.structure))
            .sum();
        organism_energy + decomposition_energy
    }
    /// Independent test oracle: recompute current stores from their owning state,
    /// rather than trusting the production ledger's cached aggregate totals.
    /// Cumulative flows and tick-zero baselines still come from the ledger because
    /// those are transaction history, not a current-state inventory.
    #[cfg(test)]
    pub(crate) fn energy_inventory_snapshot(
        &self,
    ) -> crate::energy_ledger::EnergyInventorySnapshot {
        let current_usable_energy = self
            .organisms
            .iter()
            .map(|organism| {
                organism.usable_energy
                    + organism
                        .reproductive_construction
                        .as_ref()
                        .map_or(0.0, |construction| construction.developing_energy)
            })
            .sum::<f64>()
            + self
                .decomposing_bodies
                .iter()
                .map(|body| body.energy_budget)
                .sum::<f64>();

        let current_resonant_energy = self
            .organisms
            .iter()
            .map(|organism| organism.resonant_energy.total_energy().unwrap_or(f64::NAN))
            .sum::<f64>();

        let organism_structural_energy = self
            .organisms
            .iter()
            .map(|organism| {
                let own = organism
                    .structure
                    .bonds
                    .iter()
                    .map(|bond| bond.bond_energy)
                    .sum::<f64>();
                let developing =
                    organism
                        .reproductive_construction
                        .as_ref()
                        .map_or(0.0, |construction| {
                            construction
                                .developing_structure
                                .bonds
                                .iter()
                                .map(|bond| bond.bond_energy)
                                .sum::<f64>()
                        });
                own + developing
            })
            .sum::<f64>();
        let current_structural_energy = organism_structural_energy
            + self
                .decomposing_bodies
                .iter()
                .map(|body| {
                    body.structure
                        .bonds
                        .iter()
                        .map(|bond| bond.bond_energy)
                        .sum::<f64>()
                })
                .sum::<f64>();

        crate::energy_ledger::EnergyInventorySnapshot {
            initial_usable_energy: self.energy_ledger.initial_usable_energy_held,
            current_usable_energy,
            initial_resonant_energy: self.energy_ledger.initial_resonant_energy_held,
            current_resonant_energy,
            initial_structural_energy: self.energy_ledger.initial_structural_energy_held,
            current_structural_energy,
            external_energy_input: self.energy_ledger.total_external_energy_input,
            world_tone_energy_escaped: self.energy_ledger.total_world_tone_energy_escaped,
            other_energy_released: self.energy_ledger.total_potential_energy_released,
            work_expended: self.energy_ledger.total_work_expended,
            heat_dissipated: self.energy_ledger.total_heat_dissipated,
        }
    }

    pub(crate) fn apply_survival_damage(
        organism: &mut Organism,
        environment: &Environment,
        ledger: &mut EnergyLedger,
        _rng: &mut ChaCha8Rng,
    ) -> bool {
        organism.stress *= crate::state::STRESS_DECAY_PER_TICK;
        if organism.validate_stress_state() {
            return true;
        }
        // Resonance/chemistry is the only structural damage authority; there
        // is no separate random stress-break path.
        let _ =
            crate::transformation::resolve_resonance_overload_break(organism, environment, ledger);
        match crate::cavity::analyze_genome_cavity(&organism.structure, &environment.catalog) {
            Ok(Some(cavity)) => !cavity.qualifies(),
            Ok(None) => true,
            // Geometry-analysis failure is not evidence of death.
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod decomposition_finalization_tests {
    #[test]
    fn failed_stored_break_preparation_restores_exact_malformed_escrow() {
        use crate::decision::ActionKind;
        use crate::decision_runtime::ActionCandidate;
        use crate::material_storage::StoredMaterial;
        use crate::physical_material::PhysicalMaterial;
        use crate::resources::{InternalBond, Material};
        use crate::structure::Placement;

        let mut simulation = Simulation::new(923, 10.0);
        // Isolate this test's source from the default organism inventory so
        // stored:0:bond:0 unambiguously selects the material created below.
        simulation.organisms[0].stored_material.entries.clear();
        let material = Material {
            parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
            internal_bonds: vec![InternalBond {
                part_a: 0,
                part_b: 1,
            }],
        };
        let source = PhysicalMaterial::realized(
            material,
            vec![
                Placement {
                    x: 0.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
                Placement {
                    x: 0.838,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
            ],
            &simulation.environment.catalog,
        )
        .expect("test source should realize");
        let catalog = simulation.environment.catalog.clone();
        let make_neighbor = |name: &str, x: f64| {
            PhysicalMaterial::realized(
                Material::free_base(name, 1.0),
                vec![Placement {
                    x,
                    y: 0.0,
                    rotation_radians: 0.0,
                }],
                &catalog,
            )
            .expect("single-resource storage neighbor should be realizable")
        };
        let neighbor_before = make_neighbor("Nitrogen", -2.0);
        let neighbor_after_one = make_neighbor("Phosphorus", 2.0);
        let neighbor_after_two = make_neighbor("Sulfur", 4.0);
        // Storage inserts at the front, so reverse insertion gives a source at
        // index one with a distinct physical object on either side.
        assert!(simulation.organisms[0]
            .stored_material
            .store_physical_instance(neighbor_after_two.clone()));
        assert!(simulation.organisms[0]
            .stored_material
            .store_physical_instance(neighbor_after_one.clone()));
        assert!(simulation.organisms[0]
            .stored_material
            .store_physical_instance(source));
        assert!(simulation.organisms[0]
            .stored_material
            .store_physical_instance(neighbor_before.clone()));
        let entries_before = simulation.organisms[0].stored_material.entries.clone();

        let decision = ActionCandidate {
            action: ActionKind::Break,
            context_key: Some("stored:1:bond:0".into()),
        };
        let mut next_id = 1;
        let mut transformation = Simulation::try_start_transformation(
            &mut simulation.organisms[0],
            &simulation.environment.catalog,
            &mut next_id,
            &decision,
        )
        .expect("stored BREAK should reserve source");
        transformation.decision_context_key = Some("stored:999999:bond:0".into());
        let stored = transformation.stored_material.as_mut().unwrap();
        stored.placements = None;
        let malformed = stored.clone();

        restore_failed_transformation_material(
            &transformation,
            Some(&mut simulation.organisms[0]),
            &mut simulation.environment,
        );

        let mut expected_entries = entries_before;
        expected_entries.remove(1);
        expected_entries.push(StoredMaterial::Physical(malformed));
        assert_eq!(
            simulation.organisms[0].stored_material.entries,
            expected_entries,
            "failed preparation must restore malformed escrow at its original index and preserve neighbor order"
        );
        assert_eq!(simulation.organisms[0].active_transformation_id, None);
    }

    #[test]
    fn failed_environmental_break_preparation_restores_source_at_original_cell_index() {
        use crate::physical_material::PhysicalMaterial;
        use crate::resources::{InternalBond, Material};
        use crate::state::{ActiveTransformation, TransformationKind};
        use crate::structure::Placement;

        let catalog = crate::resources::default_catalog();
        let mut environment = Environment {
            width: 1000.0,
            height: 1000.0,
            catalog: catalog.clone(),
            field: ActiveMaterialField::new(1000.0, 1000.0, crate::environment::DEFAULT_CELL_SIZE),
        };
        let make_single = |name: &str, x: f64| {
            PhysicalMaterial::realized(
                Material::free_base(name, 1.0),
                vec![Placement {
                    x,
                    y: 10.0,
                    rotation_radians: 0.0,
                }],
                &catalog,
            )
            .expect("single-resource environmental neighbor should be realizable")
        };
        let before = make_single("Nitrogen", 6.0);
        let source = PhysicalMaterial::realized(
            Material {
                parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
                internal_bonds: vec![InternalBond {
                    part_a: 0,
                    part_b: 1,
                }],
            },
            vec![
                Placement {
                    x: 10.0,
                    y: 10.0,
                    rotation_radians: 0.0,
                },
                Placement {
                    x: 10.838,
                    y: 10.0,
                    rotation_radians: 0.0,
                },
            ],
            &catalog,
        )
        .expect("bonded environmental source should be realizable");
        let after = make_single("Sulfur", 14.0);
        let cell_index = environment
            .field
            .index_for_position(10.0, 10.0)
            .expect("test source position must be in the field");
        environment.field.cells[cell_index].physical_materials =
            vec![before.clone(), after.clone()];
        let transformation = ActiveTransformation {
            id: 732,
            organism_id: "missing-environmental-break-owner".into(),
            kind: TransformationKind::Break,
            material: Material::free_base("", 0.0),
            bond: None,
            stored_material: Some(source.clone()),
            stored_bond: None,
            environmental_source: true,
            complexity: 1.0,
            duration_ticks: 1,
            remaining_ticks: 1,
            prepared_energy: None,
            pending_experience: None,
            decision_context_key: Some(format!("environment:cell:{cell_index}:material:1:bond:0")),
            prepared_structure: None,
            prepared_stored_material: None,
            prepared_usable_energy: None,
            prepared_stress: None,
            prepared_ledger: None,
            prepared_ledger_before: None,
            prepared_usable_energy_before: None,
            combine_environmental_source: None,
            combine_environmental_source_index: None,
        };

        restore_failed_transformation_material(&transformation, None, &mut environment);

        assert_eq!(
            environment.field.cells[cell_index].physical_materials,
            vec![before, source, after],
            "failed preparation must return environmental escrow to the original cell index"
        );
    }

    #[test]
    fn combine_prepares_at_completion_without_changing_break_timing() {
        use crate::state::TransformationKind;

        assert!(!super::should_prepare_transformation(
            &TransformationKind::Combine,
            1,
        ));
        assert!(super::should_prepare_transformation(
            &TransformationKind::Combine,
            0,
        ));
        assert!(super::should_prepare_transformation(
            &TransformationKind::Break,
            1,
        ));
        assert!(!super::should_prepare_transformation(
            &TransformationKind::Break,
            0,
        ));
    }

    use super::*;
    use crate::structure::{OrganismStructure, Placement, StructuralUnit};

    fn rollback_material_inventory(
        simulation: &Simulation,
    ) -> std::collections::BTreeMap<String, u64> {
        fn add(
            totals: &mut std::collections::BTreeMap<String, u64>,
            material: &crate::resources::Material,
        ) {
            for (name, amount) in &material.parts {
                assert!(
                    amount.is_finite() && *amount >= 0.0 && amount.fract().abs() <= 1e-9,
                    "rollback fixture must contain whole finite constituents"
                );
                *totals.entry(name.clone()).or_default() += amount.round() as u64;
            }
        }

        let mut totals = std::collections::BTreeMap::new();
        for organism in &simulation.organisms {
            for unit in &organism.structure.units {
                add(&mut totals, &unit.material);
            }
            for material in organism.stored_material.iter_materials() {
                add(&mut totals, material);
            }
        }
        for cell in &simulation.environment.field.cells {
            for material in &cell.materials {
                add(&mut totals, material);
            }
            for physical in &cell.physical_materials {
                add(&mut totals, &physical.material);
            }
        }
        totals
    }

    #[test]
    fn failed_stored_break_restores_material_to_organism_storage() {
        let mut simulation = Simulation::new(917, 10.0);
        for cell in &mut simulation.environment.field.cells {
            cell.materials.clear();
            cell.physical_materials.clear();
        }
        let placement = Placement {
            x: 20.0,
            y: 20.0,
            rotation_radians: 0.0,
        };
        let source = crate::physical_material::PhysicalMaterial::realized(
            crate::resources::Material::free_base("Carbon", 1.0),
            vec![placement],
            &simulation.environment.catalog,
        )
        .expect("single resource should be physically realizable");
        let amount = source.material.total_amount();
        let original_count = simulation.organisms[0].stored_material.physical_count();
        assert!(simulation.organisms[0]
            .stored_material
            .store_physical_instance(source.clone()));
        let inventory_before = rollback_material_inventory(&simulation);
        let removed = simulation.organisms[0]
            .stored_material
            .take_physical_at(original_count)
            .expect("test source should be removable from storage");
        let expected_placements = removed.placements.clone();
        let transformation = crate::state::ActiveTransformation {
            id: 999,
            organism_id: simulation.organisms[0].id.clone(),
            kind: crate::state::TransformationKind::Break,
            material: crate::resources::Material::free_base("", 0.0),
            bond: None,
            stored_material: Some(removed),
            stored_bond: None,
            environmental_source: false,
            complexity: 1.0,
            duration_ticks: 1,
            remaining_ticks: 1,
            prepared_energy: None,
            pending_experience: None,
            decision_context_key: Some("stored:0:bond:0".into()),
            prepared_structure: None,
            prepared_stored_material: None,
            prepared_usable_energy: None,
            prepared_stress: None,
            prepared_ledger: None,
            prepared_ledger_before: None,
            prepared_usable_energy_before: None,
            combine_environmental_source: None,
            combine_environmental_source_index: None,
        };

        restore_failed_transformation_material(
            &transformation,
            Some(&mut simulation.organisms[0]),
            &mut simulation.environment,
        );

        assert_eq!(
            simulation.organisms[0].stored_material.physical_count(),
            original_count + 1
        );
        assert!(
            simulation.organisms[0]
                .stored_material
                .entries
                .iter()
                .any(|entry| match entry {
                    crate::material_storage::StoredMaterial::Physical(instance) => {
                        instance.placements.as_ref() == expected_placements.as_ref()
                            && (instance.material.total_amount() - amount).abs() < 1e-12
                    }
                }),
            "failed BREAK source should be restored with its original amount"
        );
        assert_eq!(
            rollback_material_inventory(&simulation),
            inventory_before,
            "failed stored BREAK rollback must preserve total constituent ownership"
        );
    }

    #[test]
    fn initial_usable_energy_enters_through_an_explicit_source_transaction() {
        let simulation = Simulation::new(916, 10.0);
        let initial_usable_energy = simulation.organisms[0].usable_energy;

        assert!(initial_usable_energy.is_finite() && initial_usable_energy > 0.0);
        assert_eq!(simulation.energy_ledger.initial_usable_energy_held, 0.0);
        assert_eq!(
            simulation.energy_ledger.total_external_energy_input,
            initial_usable_energy
        );
        assert!(
            simulation.energy_inventory_snapshot().is_balanced(1e-9),
            "initial energy source should reconcile: {:?}",
            simulation.energy_inventory_snapshot().residual()
        );
    }

    #[test]
    fn live_world_tone_capture_settles_and_reconciles_world_energy() {
        let mut simulation = Simulation::new(918, 10.0);
        for cell in &mut simulation.environment.field.cells {
            cell.physical_materials.clear();
        }
        assert!(simulation.world_tone.is_some());

        simulation.step();

        assert!(
            simulation.energy_ledger.total_external_energy_input > 0.0,
            "live world tone should credit absorbed environmental energy"
        );
        assert!(
            simulation.energy_ledger.total_resonant_energy_held > 0.0,
            "captured world-tone energy should persist in the resonant reservoir"
        );
        assert!(
            simulation.organisms[0]
                .resonant_energy
                .total_energy()
                .is_some_and(|energy| energy > 0.0),
            "organism reservoir should retain source energy across the tick"
        );
        let inventory = simulation.energy_inventory_snapshot();
        assert!(
            inventory.is_balanced(1e-9),
            "world inventory after live sunlight capture: {:?}",
            inventory.residual()
        );
    }

    #[test]
    fn world_tone_configuration_rejects_an_orbit_inside_the_world() {
        let mut simulation = Simulation::new(917, 10.0);
        let config = crate::world_tone::WorldToneRuntimeConfig {
            parameters: crate::world_tone::WorldToneParameters {
                minimum_energy_intensity_per_tick_per_width: 1.0,
                maximum_energy_intensity_per_tick_per_width: 10.0,
                orbital_period_ticks: 2_400,
                envelope_curve: crate::world_tone::EnvelopeCurve::Cosine,
            },
            orbit_radius: 0.0,
        };

        assert!(simulation.configure_world_tone(None));
        assert!(!simulation.configure_world_tone(Some(config)));
        assert!(simulation.world_tone.is_none());
        assert!(simulation.world_tone_cycle.is_none());
    }

    #[test]
    fn initial_world_energy_inventory_uses_tick_zero_baseline() {
        let simulation = Simulation::new(914, 10.0);
        let inventory = simulation.energy_inventory_snapshot();

        // The starting usable reserve is an explicit external input, not a
        // tick-zero stored baseline. Counting both would double-count it.
        assert_eq!(inventory.initial_usable_energy, 0.0);
        assert_eq!(
            inventory.external_energy_input,
            inventory.current_usable_energy
        );
        assert_eq!(
            inventory.initial_structural_energy,
            inventory.current_structural_energy
        );
        assert!(inventory.is_balanced(1e-9));
    }

    #[test]
    fn world_energy_inventory_reconciles_across_structural_break() {
        let mut simulation = Simulation::new(917, 10.0);
        let before = simulation.energy_inventory_snapshot();
        assert!(
            before.is_balanced(1e-9),
            "initial inventory: {:?}",
            before.residual()
        );

        let organism = simulation
            .organisms
            .first()
            .expect("initial organism exists");
        let bond = *organism
            .structure
            .bonds
            .first()
            .expect("initial organism has a realized bond");
        let organism_id = organism.id.clone();

        let organism = simulation
            .organisms
            .iter_mut()
            .find(|organism| organism.id == organism_id)
            .expect("selected organism remains present");
        assert!(
            crate::transformation::settle_chemical_break_energy_with_resonance(
                organism,
                bond,
                1.0e9,
                0.0,
                0.0,
                1.0,
                &mut simulation.energy_ledger,
            )
        );

        let after = simulation.energy_inventory_snapshot();
        assert!(
            after.is_balanced(1e-9),
            "structural BREAK must reconcile the world inventory: residual={:?}",
            after.residual()
        );
        assert!(
            !simulation.organisms[0].structure.bonds.contains(&bond),
            "successful BREAK must remove the selected bond"
        );
    }

    #[test]
    fn world_energy_inventory_reconciles_across_ordinary_ticks() {
        let mut simulation = Simulation::new(916, 10.0);

        for tick in 1..=5 {
            simulation.step();
            let inventory = simulation.energy_inventory_snapshot();
            assert!(
                inventory.is_balanced(1e-9),
                "world energy inventory must reconcile after ordinary tick {tick}: residual={:?}",
                inventory.residual()
            );
        }
    }

    #[test]
    fn stress_without_retained_resonance_cannot_authorize_a_random_bond_break() {
        let mut simulation = Simulation::new(915, 10.0);
        let before = simulation.energy_inventory_snapshot();
        assert!(
            before.is_balanced(1e-9),
            "initial inventory: {:?}",
            before.residual()
        );
        let original_bonds = simulation.organisms[0].structure.bonds.clone();

        simulation.organisms[0].stress = 1.0e9;
        let _ = Simulation::apply_survival_damage(
            &mut simulation.organisms[0],
            &simulation.environment,
            &mut simulation.energy_ledger,
            &mut simulation.rng,
        );
        assert_eq!(simulation.organisms[0].structure.bonds, original_bonds);

        let after = simulation.energy_inventory_snapshot();
        assert!(
            after.is_balanced(1e-9),
            "stress bookkeeping must not create or destroy energy: {:?}",
            after.residual()
        );
    }

    #[test]
    fn invalid_finished_body_energy_holder_does_not_release_material() {
        let mut simulation = Simulation::new(913, 10.0);
        let catalog = crate::resources::default_catalog();
        let mut structure = OrganismStructure::new();
        let mut unit = StructuralUnit::new(
            "Carbon".to_string(),
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        );
        assert!(unit.realize_default_geometry(&catalog));
        structure.add_unit(unit);
        let mut body = crate::decomposition::DecomposingBody::new(
            structure,
            1.0,
            Position { x: 500.0, y: 500.0 },
        )
        .expect("test body should be valid");
        // Force the final energy settlement to reject the holder. The body
        // must remain, and its constituent material must not be deposited yet.
        body.energy_budget = f64::NAN;
        simulation.decomposing_bodies.push(body);
        let before = simulation.environment.field.total_amount();

        simulation.process_decomposing_bodies();

        assert_eq!(simulation.decomposing_bodies.len(), 1);
        assert!(simulation.decomposing_bodies[0].is_finished());
        assert_eq!(simulation.environment.field.total_amount(), before);
    }

    #[test]
    fn invalid_finished_body_position_preserves_released_material_ownership() {
        let mut simulation = Simulation::new(914, 10.0);
        let catalog = crate::resources::default_catalog();
        let mut structure = OrganismStructure::new();
        let mut unit = StructuralUnit::new(
            "Carbon".to_string(),
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        );
        assert!(unit.realize_default_geometry(&catalog));
        structure.add_unit(unit);
        let body = crate::decomposition::DecomposingBody::new(
            structure,
            0.0,
            Position {
                x: f64::NAN,
                y: 500.0,
            },
        )
        .expect("energy is valid even if the malformed position is rejected at deposit");
        simulation.decomposing_bodies.push(body);
        let before = simulation.environment.field.total_amount();

        simulation.process_decomposing_bodies();

        assert_eq!(simulation.decomposing_bodies.len(), 1);
        assert!(simulation.decomposing_bodies[0].is_finished());
        assert_eq!(simulation.decomposing_bodies[0].structure.units.len(), 1);
        assert_eq!(simulation.environment.field.total_amount(), before);
    }

    #[test]
    fn malformed_environmental_escrow_never_enters_storage_in_world_coordinates() {
        use crate::physical_material::PhysicalMaterial;
        use crate::resources::Material;
        use crate::state::ActiveTransformation;
        use crate::structure::Placement;

        let mut simulation = Simulation::new(929, 10.0);
        let mut source = PhysicalMaterial::realized(
            Material::free_base("Carbon", 1.0),
            vec![Placement {
                x: 700.0,
                y: 800.0,
                rotation_radians: 0.25,
            }],
            &simulation.environment.catalog,
        )
        .expect("test source should realize");
        source.placements = None;
        let transformation = ActiveTransformation {
            id: 811,
            organism_id: simulation.organisms[0].id.clone(),
            kind: crate::state::TransformationKind::Combine,
            material: Material::free_base("", 0.0),
            bond: None,
            stored_material: None,
            stored_bond: None,
            environmental_source: true,
            complexity: 1.0,
            duration_ticks: 1,
            remaining_ticks: 0,
            prepared_energy: None,
            pending_experience: None,
            decision_context_key: None,
            prepared_structure: None,
            prepared_stored_material: None,
            prepared_usable_energy: None,
            prepared_stress: None,
            prepared_ledger: None,
            prepared_ledger_before: None,
            prepared_usable_energy_before: None,
            combine_environmental_source: Some((usize::MAX, source.clone())),
            combine_environmental_source_index: Some(0),
        };
        simulation.organisms[0].active_transformation_id = Some(transformation.id);
        let storage_before = simulation.organisms[0].stored_material.entries.clone();

        assert!(restore_failed_transformation_material(
            &transformation,
            Some(&mut simulation.organisms[0]),
            &mut simulation.environment,
        ));
        assert_eq!(
            simulation.organisms[0].stored_material.entries,
            storage_before
        );
        assert!(
            simulation.environment.field.cells[0]
                .physical_materials
                .contains(&source),
            "unrestorable world-space escrow must remain explicitly held by the field"
        );
        assert_eq!(simulation.organisms[0].active_transformation_id, None);
    }

    #[test]
    fn environmental_escrow_fallback_converts_world_pose_to_owner_relative_storage() {
        use crate::physical_material::PhysicalMaterial;
        use crate::resources::Material;
        use crate::structure::Placement;

        let mut simulation = Simulation::new(927, 10.0);
        let source = PhysicalMaterial::realized(
            Material::free_base("Carbon", 1.0),
            vec![Placement {
                x: 700.0,
                y: 800.0,
                rotation_radians: 0.25,
            }],
            &simulation.environment.catalog,
        )
        .expect("environmental source should be physically realized");
        let owner = &mut simulation.organisms[0];
        owner.occupied_cells = vec![Position { x: 500.0, y: 500.0 }];
        assert!(
            crate::transformation::quarantine_environmental_source_in_storage(
                &source,
                owner,
                &simulation.environment,
            )
        );
        let crate::material_storage::StoredMaterial::Physical(stored) =
            &simulation.organisms[0].stored_material.entries[0];
        let placement = stored.placements.as_ref().unwrap()[0];
        let owner_origin = stored.owner_relative_origin.unwrap();
        assert!(placement.x.abs() < 1e-9 && placement.y.abs() < 1e-9);
        assert!((owner_origin.x - 200.0).abs() < 1e-9);
        assert!((owner_origin.y - 300.0).abs() < 1e-9);
        assert!((owner_origin.rotation_radians - 0.25).abs() < 1e-9);
    }
}
