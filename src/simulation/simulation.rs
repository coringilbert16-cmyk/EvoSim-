#![expect(
    dead_code,
    reason = "Organism-specific runtime is intentionally dormant during the chemistry-first phase"
)]
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::collections::HashSet;

use crate::energy_ledger::{EnergyLedgerAuthority, ExternalEnergySource, ExternalInputTransaction};

use crate::decision::{ActionEligibility, ActionKind, CurrentNeeds, DecisionParameters};
use crate::decision_runtime::ActionCandidate;
use crate::environment::{ActiveMaterialField, DEFAULT_CELL_SIZE};
use crate::genome::initial_genome;
use crate::state::{EnergyLedger, Environment, Organism, Position, Simulation};
use crate::structure::Placement;

fn checked_next_organism_id(current: u64) -> Option<u64> {
    current.checked_add(1)
}

fn should_prepare_transformation(
    kind: &crate::state::TransformationKind,
    remaining_ticks: u64,
) -> bool {
    match kind {
        crate::state::TransformationKind::Combine => remaining_ticks == 0,
        _ => remaining_ticks == 1,
    }
}

fn whole_tick_profiling_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var_os("EVOSIM_PROFILE_TICK").is_some())
}

fn environmental_material_identity_set(environment: &Environment) -> HashSet<u64> {
    environment
        .field
        .cells
        .iter()
        .flat_map(|cell| cell.physical_materials.iter())
        .map(|material| material.environmental_identity)
        .filter(|identity| *identity != 0)
        .collect()
}

fn environmental_new_materials(
    environment: &Environment,
    previous_identities: &HashSet<u64>,
) -> Vec<crate::physical_material::PhysicalMaterial> {
    environment
        .field
        .cells
        .iter()
        .flat_map(|cell| cell.physical_materials.iter())
        .filter(|material| {
            material.environmental_identity != 0
                && !previous_identities.contains(&material.environmental_identity)
        })
        .cloned()
        .collect()
}

fn environmental_lifecycle_records(
    tick: u64,
    reaction_id: String,
    reaction: &str,
    parents: &[crate::physical_material::PhysicalMaterial],
    products: &[crate::physical_material::PhysicalMaterial],
) -> Vec<crate::state::EnvironmentalMoleculeLifecycleEvent> {
    let parent_ids: Vec<u64> = parents
        .iter()
        .map(|material| material.environmental_identity)
        .filter(|identity| *identity != 0)
        .collect();
    let mut records = Vec::with_capacity(parent_ids.len() + products.len());

    // Emit product creation before parent retirement. The reaction is atomic,
    // but this ordering lets the analyzer validate that every lineage parent
    // was alive when its child was recorded, including same-tick reactions.
    for product in products {
        if product.environmental_identity == 0 {
            continue;
        }
        records.push(crate::state::EnvironmentalMoleculeLifecycleEvent {
            event: "molecular_lifecycle".to_string(),
            action: "created".to_string(),
            tick,
            molecule_id: product.environmental_identity,
            parent_ids: parent_ids.clone(),
            reaction_id: reaction_id.clone(),
            reaction: reaction.to_string(),
            molecule: Some(serde_json::to_value(product).unwrap_or(serde_json::Value::Null)),
        });
    }
    for parent in parents {
        if parent.environmental_identity == 0 {
            continue;
        }
        records.push(crate::state::EnvironmentalMoleculeLifecycleEvent {
            event: "molecular_lifecycle".to_string(),
            action: "destroyed".to_string(),
            tick,
            molecule_id: parent.environmental_identity,
            parent_ids: Vec::new(),
            reaction_id: reaction_id.clone(),
            reaction: reaction.to_string(),
            molecule: Some(serde_json::to_value(parent).unwrap_or(serde_json::Value::Null)),
        });
    }
    records
}

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

fn environmental_break_local_resonance_sample(
    simulation: &Simulation,
    operation: &crate::chemical_reaction::EnvironmentalChemicalBreakOperation,
) -> serde_json::Value {
    let mut restored = crate::structure::OrganismStructure::new();
    let origin = Placement {
        x: 0.0,
        y: 0.0,
        rotation_radians: 0.0,
    };
    let endpoint_points = crate::physical_material::restore_material(
        &mut restored,
        &operation.source_material,
        origin,
        &simulation.environment.catalog,
    )
    .and_then(|indices| {
        let unit_a = restored.units.get(*indices.get(operation.bond.part_a)?)?;
        let unit_b = restored.units.get(*indices.get(operation.bond.part_b)?)?;
        let point_a = crate::contact::endpoint_world_point(
            operation.bond.endpoint_a,
            unit_a,
            &simulation.environment.catalog,
        )?;
        let point_b = crate::contact::endpoint_world_point(
            operation.bond.endpoint_b,
            unit_b,
            &simulation.environment.catalog,
        )?;
        Some([(point_a.x, point_a.y), (point_b.x, point_b.y)])
    });

    let local_spectra = endpoint_points.as_ref().and_then(|points| {
        crate::world_tone_snapshot::world_material_resonance_spectra_at_boundary_points(
            simulation,
            operation.source_material.environmental_identity,
            points,
        )
    });

    // This is a read-only preview of the exact compatible-resonance withdrawal
    // used by settlement. It is deliberately distinct from instantaneous local
    // signal reception and does not mutate the live reservoir.
    let compatible_resonance_available = simulation
        .environment
        .resonant_energy
        .compatible_energy(operation.reaction_frequency_cycles_per_day);
    let mut staged_reservoir = simulation.environment.resonant_energy.clone();
    let resonance_withdrawal_preview = staged_reservoir.withdraw_compatible(
        operation.reaction_frequency_cycles_per_day,
        operation.disruption_cost,
    );
    let required_usable_energy = operation.bond.bond_energy
        + (operation.disruption_cost - resonance_withdrawal_preview.min(operation.disruption_cost));
    let usable_energy = simulation.environment.chemistry_usable_energy;
    let usable_energy_shortfall = (required_usable_energy - usable_energy).max(0.0);

    let endpoint_samples = match (endpoint_points, local_spectra) {
        (Some(points), Some(spectra)) if points.len() == spectra.len() => Some(
            points
                .iter()
                .zip(spectra.iter())
                .enumerate()
                .map(|(index, (point, spectrum))| {
                    let signal_intensity = spectrum
                        .components
                        .iter()
                        .map(|component| component.amplitude * component.amplitude)
                        .sum::<f64>();
                    serde_json::json!({
                        "endpoint": if index == 0 { "a" } else { "b" },
                        "point": [point.0, point.1],
                        "source_component_count": spectrum.components.len(),
                        "signal_intensity": signal_intensity,
                        "components": &spectrum.components
                    })
                })
                .collect::<Vec<_>>(),
        ),
        _ => None,
    };

    serde_json::json!({
        "event": "environmental_local_resonance_sample",
        "tick": simulation.tick,
        "material_identity": operation.source_material.environmental_identity,
        "bond_index": operation.bond_index,
        "reaction_frequency_cycles_per_day": operation.reaction_frequency_cycles_per_day,
        "local_resonance_similarity": operation.local_resonance_similarity,
        "activation_signal": operation.activation_signal,
        "disruption_cost": operation.disruption_cost,
        "stored_bond_energy": operation.bond.bond_energy,
        "chemistry_usable_energy_before": usable_energy,
        "environment_resonant_energy_before": simulation.environment.resonant_energy.total_energy(),
        "compatible_resonance_available": compatible_resonance_available,
        "compatible_resonance_withdrawal_preview": resonance_withdrawal_preview,
        "required_usable_energy_after_resonance": required_usable_energy,
        "usable_energy_shortfall": usable_energy_shortfall,
        "funding_sufficient_before_attempt": usable_energy_shortfall <= 1e-9,
        "endpoint_geometry_available": endpoint_points.is_some(),
        "local_spectrum_available": endpoint_samples.is_some(),
        "endpoint_samples": endpoint_samples
    })
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
        let environment = Self::create_environment(seed);
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
        energy_ledger.total_structural_energy_held =
            crate::reproduction::total_structural_energy_held(std::slice::from_ref(&organism), &[])
                + Self::environmental_structural_energy_held(&environment);
        energy_ledger.initial_structural_energy_held = energy_ledger.total_structural_energy_held;
        let hotspot = crate::hotspot::RadialHotspot::right_bend_for_world(
            environment.width,
            environment.height,
        );
        let ambient_config = crate::world_generated_ambient_mode::AmbientModeConfig::new(
            environment.width * environment.height,
            1.0,
            0.0,
            0.0,
        )
        .expect("approved ambient energy configuration must be valid");
        let ambient_energy =
            crate::world_generated_ambient_mode::WorldGeneratedAmbientMode::new(ambient_config, 0)
                .expect("ambient energy must initialize at tick zero");
        let opening_ambient_energy = ambient_energy.sample().total_energy;
        energy_ledger.total_ambient_energy_held = opening_ambient_energy;
        energy_ledger.initial_ambient_energy_held = opening_ambient_energy;
        let resonance_field = crate::live_resonance_field::LiveResonanceField::new(
            environment.width,
            environment.height,
            ambient_energy
                .sample_midpoint_before_current()
                .expect("initial ambient carrier midpoint must be available"),
        )
        .expect("live spatial resonance field must initialize");
        let initial_spatial_field_energy = resonance_field.initial_total_energy();
        energy_ledger.total_spatial_disturbance_energy_held =
            resonance_field.energy_state().disturbance_energy;
        energy_ledger.total_spatial_cross_energy_held = resonance_field.energy_state().cross_energy;
        energy_ledger.total_spatial_field_energy_held = resonance_field.energy_state().total_energy;
        energy_ledger.initial_spatial_field_energy_held = initial_spatial_field_energy;
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
            hotspot,
            environmental_chemistry_diagnostics: Default::default(),
            environmental_chemistry_events: Vec::new(),
            environmental_local_resonance_samples: Vec::new(),
            record_local_resonance_diagnostics: false,
            record_environmental_molecular_history: false,
            flow_field: crate::environment_flow::FlowFieldConfig::default(),
            ambient_energy,
            resonance_field,
        }
    }
    fn create_environment(seed: u64) -> Environment {
        let catalog = crate::resources::default_catalog();
        let width = 250.0;
        let height = 250.0;
        let mut field = ActiveMaterialField::new(width, height, DEFAULT_CELL_SIZE);
        crate::environment_chemistry_experiment::seed_randomized_resources(
            &mut field, &catalog, seed, width, height,
        );
        Environment {
            width,
            height,
            catalog,
            field,
            chemistry_usable_energy: 0.0,
            resonant_energy: Default::default(),
        }
    }
    pub(crate) fn create_initial_organism() -> Organism {
        let genome = initial_genome();
        let catalog = crate::resources::default_catalog();
        let construction = crate::initial_organism_constructor::construct_valid(&catalog)
            .expect("initial organism constructor must find a valid organism");
        let mut structure = construction.structure;
        let anchor = Position { x: 125.0, y: 125.0 };
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
            assert!(stored_material.store_physical_instance_at_owner_anchor(
                physical,
                Placement {
                    x: anchor.x,
                    y: anchor.y,
                    rotation_radians: 0.0,
                },
                &catalog,
            ));
        }

        // The constructor's large energy allowance is a computation budget for
        // realizing the initial geometry, not a physical source of organism energy.
        // Only the genome's explicit initial allocation enters the live world.
        let initial_energy = genome.reproductive_energy_allocation;

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
                &environment.catalog,
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
        let stress_input = organism.stress.max(0.0) * crate::state::STRESS_INPUT_SCALE;
        let energy_survival = (reserve_pressure * (1.0 + stress_input)).clamp(0.0, 1.0);
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
                    &organism.stored_material,
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
        // A multi-tick transformation owns a staged physical/energy snapshot.
        // Do not start movement or another material action against that snapshot.
        if organism.active_transformation_id.is_some() {
            return ActionEligibility {
                can_move: false,
                can_combine: false,
                can_break: false,
                can_expel: false,
            };
        }
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
            // Stage energy dissipation without mutating the live holder or
            // ledger. Material release and energy settlement are one ownership
            // transition: either both commit, or the body remains unchanged.
            let mut staged_ledger = self.energy_ledger;
            let mut staged_energy = remaining;
            if remaining > 0.0 {
                let sink = crate::energy_ledger::EnergyTransaction {
                    reason: crate::energy_ledger::EnergyReason::Decomposition,
                    potential_released: 0.0,
                    usable_delta: -remaining,
                    structural_delta: 0.0,
                    work_expended: 0.0,
                    heat_dissipated: remaining,
                };
                if !staged_ledger.settle_transaction(&mut staged_energy, sink) {
                    continue;
                }
            }

            let position = self.decomposing_bodies[index].position.clone();
            let Some(materials) =
                self.decomposing_bodies[index].release_finished_material(&self.environment.catalog)
            else {
                // Keep the holder and live ledger intact if any constituent
                // cannot be released with its authoritative material/geometry.
                continue;
            };
            let deposits = materials
                .into_iter()
                .map(|material| (position.x, position.y, material))
                .collect();
            if !self
                .environment
                .field
                .deposit_physical_batch(deposits, &self.environment.catalog)
            {
                // A failed destination commits neither material nor energy.
                continue;
            }
            self.decomposing_bodies[index].energy_budget = staged_energy;
            self.energy_ledger = staged_ledger;
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

    /// Advances the approved ambient energy envelope as part of the live simulation clock.
    /// Signed changes are booked separately from receiver/chemistry transactions.
    fn advance_ambient_energy(&mut self) {
        let ambient_before = self.ambient_energy.sample();
        let transfer = self
            .ambient_energy
            .advance_one_tick()
            .expect("ambient energy must advance deterministically by one tick");
        let ambient_midpoint = self
            .ambient_energy
            .sample_midpoint_before_current()
            .expect("ambient carrier midpoint must be available after advancing");
        self.energy_ledger.total_ambient_energy_generated += transfer.generated_energy;
        self.energy_ledger.total_ambient_energy_withdrawn += transfer.withdrawn_energy;
        self.energy_ledger.total_ambient_energy_held = transfer.energy_after;

        let spatial_state = self
            .resonance_field
            .advance_one_tick(ambient_before, ambient_midpoint)
            .expect("spatial resonance field must advance and account deterministically");
        self.energy_ledger.total_spatial_disturbance_energy_held = spatial_state.disturbance_energy;
        self.energy_ledger.total_spatial_cross_energy_held = spatial_state.cross_energy;
        self.energy_ledger.total_spatial_field_energy_held = spatial_state.total_energy;
        self.energy_ledger.total_spatial_field_world_work = spatial_state.cumulative_world_work;
        self.energy_ledger.total_spatial_field_computational_outflow =
            spatial_state.cumulative_computational_outflow;
        self.energy_ledger.total_spatial_field_accounting_residual =
            spatial_state.accounting_residual;
    }

    /// Samples the composed ambient-plus-disturbance field at an actual world position.
    /// Outside the current computational window, the disturbance uses its validated
    /// zero-extension state while the uniform ambient carrier remains present.
    pub(crate) fn sample_resonance_field_at(
        &self,
        x: f64,
        y: f64,
    ) -> Result<
        crate::ambient_spatial_composition::ComposedFieldSample,
        crate::ambient_spatial_composition::CompositionError,
    > {
        self.resonance_field
            .sample(x, y)?
            .ok_or(crate::ambient_spatial_composition::CompositionError::InvalidState)
    }

    pub(crate) fn step(&mut self) {
        self.step_with_transport_reuse::<true>();
    }

    pub(crate) fn step_with_transport_reuse<const REUSE_SINGLETON_CENTER_VELOCITY: bool>(
        &mut self,
    ) {
        // Biology is dormant while the simulator establishes chemistry-first
        // behavior. Preserve every biological holder, but remove it from the
        // active tick so no lifecycle, organism-owned reaction, decision,
        // movement, reproduction, maintenance, or death rule can execute.
        // Environmental material remains in place and continues through the
        // ordinary physical and chemical tick below.
        let dormant_organisms = std::mem::take(&mut self.organisms);
        let dormant_transformations = std::mem::take(&mut self.active_transformations);
        let dormant_decomposing_bodies = std::mem::take(&mut self.decomposing_bodies);

        self.step_environmental_chemistry_with_transport_reuse::<REUSE_SINGLETON_CENTER_VELOCITY>();

        self.organisms = dormant_organisms;
        self.active_transformations = dormant_transformations;
        self.decomposing_bodies = dormant_decomposing_bodies;
        // Reconcile once, after the environmental tick and after every dormant
        // holder has been restored to the authoritative inventory.
        self.reconcile_energy_inventory_holders();
    }

    fn step_environmental_chemistry_with_transport_reuse<
        const REUSE_SINGLETON_CENTER_VELOCITY: bool,
    >(
        &mut self,
    ) {
        self.environmental_chemistry_diagnostics =
            crate::state::EnvironmentalChemistryTickDiagnostics {
                tick: self.tick,
                ..Default::default()
            };
        self.environmental_chemistry_events.clear();
        self.environmental_local_resonance_samples.clear();
        self.tick += 1;
        self.advance_ambient_energy();
        self.environmental_chemistry_diagnostics.tick = self.tick;

        // Whole-tick profiling is opt-in and samples one tick in ten. The
        // ordinary simulation path does not create timers or collect phase data.
        let profile_tick = whole_tick_profiling_enabled() && self.tick % 10 == 0;
        let tick_profile_started = profile_tick.then(std::time::Instant::now);
        let mut profile_phase_started = tick_profile_started;
        let mut profile_phases: Vec<(&str, u128)> = if profile_tick {
            Vec::with_capacity(16)
        } else {
            Vec::new()
        };
        let mut finish_profile_phase = |name: &'static str| {
            if let Some(started) = profile_phase_started.take() {
                let now = std::time::Instant::now();
                profile_phases.push((name, now.duration_since(started).as_nanos()));
                profile_phase_started = Some(now);
            }
        };

        // Organism and in-flight biological holders are detached by the
        // outer tick boundary. Keep the living-owner set empty so universal
        // environmental material input cannot route energy into biology.
        let living_ids: HashSet<String> = HashSet::new();

        if profile_tick {
            finish_profile_phase("tick_prelude");
        }

        // Ambient transport runs before environmental reaction discovery. Flow is
        // allowed to change contacts and interrupt activation naturally; chemistry
        // rules are not relaxed to guarantee any particular reaction.
        crate::environment_flow::transport_field_with_config::<REUSE_SINGLETON_CENTER_VELOCITY>(
            &mut self.environment.field,
            &self.environment.catalog,
            self.environment.width,
            self.environment.height,
            self.flow_field,
        );
        if profile_tick {
            finish_profile_phase("ambient_transport");
        }

        // Resonance movement runs after ambient transport and before reaction
        // discovery, so chemistry sees the updated environmental contacts.
        crate::environmental_movement::apply_environmental_resonance_movement(
            &mut self.environment.field,
            &self.environment.catalog,
            self.tick,
        );
        if profile_tick {
            finish_profile_phase("resonance_movement");
        }

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
            crate::world_tone_snapshot::WorldToneRecipientSnapshot::settle_uniform_material_inputs(
                self,
                sample,
                &living_ids,
            );
        }
        if profile_tick {
            finish_profile_phase("world_tone_environment_settlement");
        }
        if let Some(hotspot) = self.hotspot {
            self.environmental_chemistry_diagnostics
                .hotspot_settlement_succeeded = Some(crate::hotspot::settle_environment_inputs(
                hotspot,
                self.tick,
                &mut self.environment,
                &mut self.energy_ledger,
            ));
        }
        if profile_tick {
            finish_profile_phase("hotspot_settlement");
        }

        // Environmental chemistry is a first-class owner and runs without
        // organisms. The chemistry phases below operate on the field directly.
        let mut break_discovery = crate::state::EnvironmentalBreakDiscoveryDiagnostics::default();
        // Build the material-origin field once per tick and share it across
        // environmental chemistry discovery.
        // A controlled A/B run can disable only local activation influence. Stored
        // resonance funding and all other environmental chemistry remain unchanged.
        let local_resonance_world =
            if std::env::var_os("EVOSIM_DISABLE_LOCAL_RESONANCE_ACTIVATION").is_some() {
                None
            } else {
                crate::world_tone_snapshot::LocalResonanceWorldSnapshot::from_simulation(self)
            };
        self.environment.field.ensure_environmental_bond_index();
        if profile_tick {
            finish_profile_phase("resonance_snapshot_and_bond_index");
        }
        // Keep bounded discovery shares for COMBINE and displacement. BREAK
        // uses the complete bond index after both reaction phases resolve.
        let mut environmental_combine_discovery_remaining =
            crate::chemical_reaction::MAX_REACTION_CANDIDATE_INSPECTIONS_PER_TICK / 8;
        self.environmental_chemistry_diagnostics.combine_search =
            crate::state::EnvironmentalCombineSearchDiagnostics::default();
        let environmental_combine_operations =
            crate::environmental_combine::accumulate_environmental_with_diagnostics(
                &self.environment,
                &mut self.chemical_reaction_accumulation,
                &mut self.chemistry_library,
                self.tick,
                &mut environmental_combine_discovery_remaining,
                local_resonance_world.as_ref(),
                &mut self.environmental_chemistry_diagnostics.combine_search,
            );
        self.environmental_chemistry_diagnostics
            .combine_ready_candidates = environmental_combine_operations.len();
        self.environmental_chemistry_diagnostics
            .combine_discovery_work_remaining = environmental_combine_discovery_remaining;

        if profile_tick {
            finish_profile_phase("environmental_combine_discovery");
        }

        // Environmental COMBINE receives its own bounded share of the field's
        // work budget. It consumes the same shared reaction ledger and preserves
        // progress if a source snapshot became stale before commit.
        let mut environmental_combine_work_remaining =
            crate::chemical_reaction::MAX_REACTION_REALIZATIONS_PER_TICK / 8;
        let environmental_combine_operations =
            crate::environmental_combine::schedule_environmental_operations(
                environmental_combine_operations,
                self.tick,
                &mut environmental_combine_work_remaining,
                &mut self.chemical_reaction_accumulation,
            );
        self.environmental_chemistry_diagnostics.combine_scheduled =
            environmental_combine_operations.len();
        self.environmental_chemistry_diagnostics
            .combine_realization_work_remaining = environmental_combine_work_remaining;
        for operation in environmental_combine_operations {
            let source_cells = vec![operation.cell_a, operation.cell_b];
            let source_material_indices = vec![operation.material_a, operation.material_b];
            let source_materials = vec![
                operation.source_a.material.parts.clone(),
                operation.source_b.material.parts.clone(),
            ];
            let source_material_ids = [
                operation.source_a.environmental_identity,
                operation.source_b.environmental_identity,
            ]
            .into_iter()
            .map(|identity| (identity != 0).then_some(identity))
            .collect();
            let reaction_id = format!(
                "{}-{}",
                self.tick,
                self.environmental_chemistry_events.len()
            );
            let molecular_history_enabled = self.record_environmental_molecular_history;
            let parent_materials = if molecular_history_enabled {
                vec![operation.source_a.clone(), operation.source_b.clone()]
            } else {
                Vec::new()
            };
            let previous_identities = if molecular_history_enabled {
                environmental_material_identity_set(&self.environment)
            } else {
                HashSet::new()
            };
            let work_required = operation.investment;
            let accumulated_reaction = Some(operation.accumulated_reaction);
            let product_cell = operation.cell_a;
            let product_material_index = operation.material_a;
            let energy_settlement =
                crate::environmental_combine::resolve_environmental_preserving_accumulation_with_settlement(
                    operation,
                    &mut self.environment,
                    &mut self.energy_ledger,
                    &mut self.chemical_reaction_accumulation,
                );
            let committed = energy_settlement.is_some();
            let energy_settlement = energy_settlement.unwrap_or_default();
            if committed {
                self.environmental_chemistry_diagnostics.combine_committed += 1;
            } else {
                self.environmental_chemistry_diagnostics.combine_rejected += 1;
            }
            let product = if committed {
                self.environment
                    .field
                    .cells
                    .get(product_cell)
                    .and_then(|cell| cell.physical_materials.get(product_material_index))
            } else {
                None
            };
            let product_materials = if committed && molecular_history_enabled {
                environmental_new_materials(&self.environment, &previous_identities)
            } else {
                Vec::new()
            };
            let molecular_lifecycle = if committed && molecular_history_enabled {
                environmental_lifecycle_records(
                    self.tick,
                    reaction_id.clone(),
                    "COMBINE",
                    &parent_materials,
                    &product_materials,
                )
            } else {
                Vec::new()
            };
            let product_material = product.map(|material| material.material.parts.clone());
            let product_internal_bond_count =
                product.map(|material| material.internal_connections.as_ref().map_or(0, Vec::len));
            let mut post_commit_source_cell_materials = Vec::new();
            if committed {
                for cell_index in source_cells.iter().copied() {
                    if let Some(cell) = self.environment.field.cells.get(cell_index) {
                        post_commit_source_cell_materials.extend(
                            cell.physical_materials
                                .iter()
                                .map(|material| material.material.parts.clone()),
                        );
                    }
                }
            }
            self.environmental_chemistry_events.push(
                crate::state::EnvironmentalChemistryEvent {
                    reaction_id,
                    tick: self.tick,
                    reaction: "COMBINE".to_string(),
                    source_cells,
                    source_material_indices,
                    source_material_ids,
                    source_materials,
                    work_required,
                    reaction_frequency_hz: None,
                    activation_signal: None,
                    accumulated_reaction,
                    broken_bond_index: None,
                    outcome: if committed { "committed" } else { "rejected" }.to_string(),
                    failure_reason: (!committed).then(|| {
                        "resolver rejected stale, invalid, or unfunded settlement; detailed reason is not yet exposed".to_string()
                    }),
                    energy_settlement,
                    product_material,
                    product_internal_bond_count,
                    post_commit_source_cell_materials,
                    molecular_lifecycle,
                },
            );
        }

        if profile_tick {
            finish_profile_phase("environmental_combine_resolution");
        }

        let mut environmental_displacement_discovery_remaining =
            crate::chemical_reaction::MAX_REACTION_CANDIDATE_INSPECTIONS_PER_TICK / 8;
        let mut displacement_discovery =
            crate::state::EnvironmentalDisplacementDiscoveryDiagnostics::default();
        let environmental_displacement_operations =
            crate::environmental_displacement::accumulate_environmental_with_resonance(
                &self.environment,
                &mut self.chemical_reaction_accumulation,
                self.tick,
                &mut environmental_displacement_discovery_remaining,
                local_resonance_world.as_ref(),
                &mut displacement_discovery,
            );
        self.environmental_chemistry_diagnostics
            .displacement_discovery = displacement_discovery;
        self.environmental_chemistry_diagnostics
            .displacement_ready_candidates = environmental_displacement_operations.len();
        self.environmental_chemistry_diagnostics
            .displacement_discovery_work_remaining = environmental_displacement_discovery_remaining;

        if profile_tick {
            finish_profile_phase("environmental_displacement_discovery");
        }

        let mut environmental_displacement_work_remaining =
            crate::chemical_reaction::MAX_REACTION_REALIZATIONS_PER_TICK / 8;
        let environmental_displacement_operations =
            crate::environmental_displacement::schedule_environmental_operations(
                environmental_displacement_operations,
                self.tick,
                &mut environmental_displacement_work_remaining,
                &mut self.chemical_reaction_accumulation,
            );
        self.environmental_chemistry_diagnostics
            .displacement_scheduled = environmental_displacement_operations.len();
        self.environmental_chemistry_diagnostics
            .displacement_realization_work_remaining = environmental_displacement_work_remaining;
        for operation in environmental_displacement_operations {
            let source_cells = vec![operation.cell_a, operation.cell_b];
            let source_material_indices = vec![operation.material_a, operation.material_b];
            let source_materials = vec![
                operation.source_a.material.parts.clone(),
                operation.source_b.material.parts.clone(),
            ];
            let source_material_ids = [
                operation.source_a.environmental_identity,
                operation.source_b.environmental_identity,
            ]
            .into_iter()
            .map(|identity| (identity != 0).then_some(identity))
            .collect();
            let reaction_id = format!(
                "{}-{}",
                self.tick,
                self.environmental_chemistry_events.len()
            );
            let molecular_history_enabled = self.record_environmental_molecular_history;
            let parent_materials = if molecular_history_enabled {
                vec![operation.source_a.clone(), operation.source_b.clone()]
            } else {
                Vec::new()
            };
            let previous_identities = if molecular_history_enabled {
                environmental_material_identity_set(&self.environment)
            } else {
                HashSet::new()
            };
            let work_required = operation.work_required;
            let reaction_frequency_hz =
                Some(operation.reaction_frequency_cycles_per_day / 86_400.0);
            let activation_signal = Some(operation.activation_signal);
            let accumulated_reaction = Some(operation.accumulated_reaction);
            let broken_bond_index = Some(operation.bond_index);
            let resolution =
                crate::environmental_displacement::resolve_environmental_preserving_accumulation(
                    operation,
                    &mut self.environment,
                    &mut self.energy_ledger,
                    &mut self.chemical_reaction_accumulation,
                );
            let energy_settlement = match &resolution {
                Ok(settlement) => settlement.clone(),
                Err(_) => crate::state::EnvironmentalEnergySettlement::default(),
            };
            let committed = resolution.is_ok();
            let product_materials = if committed && molecular_history_enabled {
                environmental_new_materials(&self.environment, &previous_identities)
            } else {
                Vec::new()
            };
            let molecular_lifecycle = if committed && molecular_history_enabled {
                environmental_lifecycle_records(
                    self.tick,
                    reaction_id.clone(),
                    "DISPLACEMENT",
                    &parent_materials,
                    &product_materials,
                )
            } else {
                Vec::new()
            };
            let failure_reason = resolution.err().map(str::to_string);
            if committed {
                self.environmental_chemistry_diagnostics
                    .displacement_committed += 1;
            } else {
                self.environmental_chemistry_diagnostics
                    .displacement_rejected += 1;
            }
            let mut post_commit_source_cell_materials = Vec::new();
            if committed {
                for cell_index in source_cells.iter().copied() {
                    if let Some(cell) = self.environment.field.cells.get(cell_index) {
                        post_commit_source_cell_materials.extend(
                            cell.physical_materials
                                .iter()
                                .map(|material| material.material.parts.clone()),
                        );
                    }
                }
            }
            self.environmental_chemistry_events
                .push(crate::state::EnvironmentalChemistryEvent {
                    reaction_id,
                    tick: self.tick,
                    reaction: "DISPLACEMENT".to_string(),
                    source_cells,
                    source_material_indices,
                    source_material_ids,
                    source_materials,
                    work_required,
                    reaction_frequency_hz,
                    activation_signal,
                    accumulated_reaction,
                    broken_bond_index,
                    outcome: if committed { "committed" } else { "rejected" }.to_string(),
                    failure_reason,
                    energy_settlement,
                    product_material: None,
                    product_internal_bond_count: None,
                    post_commit_source_cell_materials,
                    molecular_lifecycle,
                });
        }

        if profile_tick {
            finish_profile_phase("environmental_displacement_resolution");
        }

        // BREAK discovery runs after transport, world input, COMBINE, and
        // displacement have completed. Rebuild its local-resonance snapshot
        // from the final environmental geometry before checking each bond once.
        let break_local_resonance_snapshot_started = std::time::Instant::now();
        let break_local_resonance_world =
            if std::env::var_os("EVOSIM_DISABLE_LOCAL_RESONANCE_ACTIVATION").is_some() {
                None
            } else {
                crate::world_tone_snapshot::LocalResonanceWorldSnapshot::from_simulation(self)
            };
        break_discovery.local_resonance_snapshot_build_nanoseconds =
            break_local_resonance_snapshot_started
                .elapsed()
                .as_nanos()
                .min(u64::MAX as u128) as u64;
        let mut environmental_break_discovery_remaining = 0;
        self.environment.field.ensure_environmental_bond_index();
        let environmental_break_operations =
            crate::chemical_reaction::accumulate_environmental_with_diagnostics(
                &self.environment,
                &mut self.chemical_reaction_accumulation,
                &mut self.chemistry_library,
                self.tick,
                &mut environmental_break_discovery_remaining,
                break_local_resonance_world.as_ref(),
                &mut break_discovery,
            );
        self.environmental_chemistry_diagnostics.break_discovery = break_discovery;
        self.environmental_chemistry_diagnostics
            .break_ready_candidates = environmental_break_operations.len();
        self.environmental_chemistry_diagnostics
            .break_discovery_work_remaining = environmental_break_discovery_remaining;
        if profile_tick {
            finish_profile_phase("environmental_break_discovery");
        }

        let mut environmental_break_work_remaining =
            crate::chemical_reaction::MAX_REACTION_REALIZATIONS_PER_TICK / 4;
        let environmental_break_operations =
            crate::chemical_reaction::schedule_environmental_operations(
                environmental_break_operations,
                self.tick,
                &mut environmental_break_work_remaining,
                &mut self.chemical_reaction_accumulation,
            );
        self.environmental_chemistry_diagnostics.break_scheduled =
            environmental_break_operations.len();
        self.environmental_chemistry_diagnostics
            .break_realization_work_remaining = environmental_break_work_remaining;
        for operation in environmental_break_operations {
            // A bounded sample every 50 ticks captures actual scheduled BREAK
            // locations without placing the full-world geometric sampler on
            // the ordinary tick path or multiplying its cost by candidate count.
            if self.record_local_resonance_diagnostics
                && self.tick % 50 == 0
                && self.environmental_local_resonance_samples.is_empty()
            {
                self.environmental_local_resonance_samples
                    .push(environmental_break_local_resonance_sample(self, &operation));
            }
            let source_cell = operation.cell_index;
            let source_material_index = operation.material_index;
            let source_materials = vec![operation.source_material.material.parts.clone()];
            let source_material_ids = vec![(operation.source_material.environmental_identity != 0)
                .then_some(operation.source_material.environmental_identity)];
            let reaction_id = format!(
                "{}-{}",
                self.tick,
                self.environmental_chemistry_events.len()
            );
            let molecular_history_enabled = self.record_environmental_molecular_history;
            let parent_materials = if molecular_history_enabled {
                vec![operation.source_material.clone()]
            } else {
                Vec::new()
            };
            let previous_identities = if molecular_history_enabled {
                environmental_material_identity_set(&self.environment)
            } else {
                HashSet::new()
            };
            let work_required = operation.disruption_cost;
            let reaction_frequency_hz =
                Some(operation.reaction_frequency_cycles_per_day / 86_400.0);
            let activation_signal = Some(operation.activation_signal);
            let accumulated_reaction = Some(operation.accumulated_reaction);
            let broken_bond_index = Some(operation.bond_index);
            let resolution =
                crate::chemical_reaction::resolve_environmental_preserving_accumulation(
                    operation,
                    &mut self.environment,
                    &mut self.energy_ledger,
                    &mut self.chemical_reaction_accumulation,
                );
            let energy_settlement = match &resolution {
                Ok(settlement) => settlement.clone(),
                Err(_) => crate::state::EnvironmentalEnergySettlement::default(),
            };
            let committed = resolution.is_ok();
            let product_materials = if committed && molecular_history_enabled {
                environmental_new_materials(&self.environment, &previous_identities)
            } else {
                Vec::new()
            };
            let molecular_lifecycle = if committed && molecular_history_enabled {
                environmental_lifecycle_records(
                    self.tick,
                    reaction_id.clone(),
                    "BREAK",
                    &parent_materials,
                    &product_materials,
                )
            } else {
                Vec::new()
            };
            let failure_reason = resolution.err().map(str::to_string);
            if committed {
                self.environmental_chemistry_diagnostics.break_committed += 1;
            } else {
                self.environmental_chemistry_diagnostics.break_rejected += 1;
            }
            let post_commit_source_cell_materials = if committed {
                self.environment
                    .field
                    .cells
                    .get(source_cell)
                    .map(|cell| {
                        cell.physical_materials
                            .iter()
                            .map(|material| material.material.parts.clone())
                            .collect()
                    })
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
            self.environmental_chemistry_events
                .push(crate::state::EnvironmentalChemistryEvent {
                    reaction_id,
                    tick: self.tick,
                    reaction: "BREAK".to_string(),
                    source_cells: vec![source_cell],
                    source_material_indices: vec![source_material_index],
                    source_material_ids,
                    source_materials,
                    work_required,
                    reaction_frequency_hz,
                    activation_signal,
                    accumulated_reaction,
                    broken_bond_index,
                    outcome: if committed { "committed" } else { "rejected" }.to_string(),
                    failure_reason,
                    energy_settlement,
                    product_material: None,
                    product_internal_bond_count: None,
                    post_commit_source_cell_materials,
                    molecular_lifecycle,
                });
        }

        if profile_tick {
            finish_profile_phase("environmental_break_resolution");
        }

        // Organism-owned BREAK/COMBINE, uptake, decisions, movement,
        // reproduction, survival, and dead-holder release are intentionally
        // absent from the chemistry tick. Their implementations remain
        // available for later redesign against universal material ownership.
        self.environmental_chemistry_diagnostics
            .accumulation_entries = self.chemical_reaction_accumulation.len();

        if profile_tick {
            finish_profile_phase("chemistry_tick_finalization");
            drop(finish_profile_phase);
            let total_ns = tick_profile_started.map_or(0, |started| started.elapsed().as_nanos());
            eprintln!(
                "SIMULATION_TICK_PROFILE tick={} total_ns={} phases={:?}",
                self.tick, total_ns, profile_phases
            );
        }
    }
    /// Transfer a dead, fully disconnected organism into environmental material.
    ///
    /// A core breach alone never moves material. Release is permitted only after
    /// shared chemistry has removed every graph bond and every non-graph holder
    /// that would otherwise be orphaned is empty. The field insertion and energy
    /// sink are staged so a rejected transfer leaves the organism authoritative.
    fn release_finished_dead_organisms(&mut self, living_ids: &HashSet<String>) {
        let mut release_indices = Vec::new();
        for (index, organism) in self.organisms.iter().enumerate() {
            if living_ids.contains(&organism.id)
                || self
                    .active_transformations
                    .iter()
                    .any(|transformation| transformation.organism_id == organism.id)
                || !organism.structure.bonds.is_empty()
                || organism.active_transformation_id.is_some()
                || organism.reproductive_construction.is_some()
                || !organism.stored_material.entries.is_empty()
                || organism.active_movement.is_some()
            {
                continue;
            }
            let Some(resonant_energy) = organism.resonant_energy.total_energy() else {
                continue;
            };
            if !resonant_energy.is_finite() || resonant_energy < 0.0 {
                continue;
            }
            if !organism.usable_energy.is_finite() || organism.usable_energy < 0.0 {
                continue;
            }
            if organism
                .structure
                .units
                .iter()
                .any(|unit| unit.shape(&self.environment.catalog).is_none())
            {
                continue;
            }
            release_indices.push(index);
        }

        for index in release_indices.into_iter().rev() {
            let organism = &self.organisms[index];
            let organism_id = organism.id.clone();
            let Some(resonant_energy) = organism.resonant_energy.total_energy() else {
                continue;
            };
            if !resonant_energy.is_finite() || resonant_energy < 0.0 {
                continue;
            }
            let Some(materials) = organism
                .structure
                .units
                .iter()
                .map(|unit| {
                    crate::decomposition::realize_released_unit(unit, &self.environment.catalog)
                })
                .collect::<Option<Vec<_>>>()
            else {
                continue;
            };
            let deposits = materials
                .into_iter()
                .zip(&organism.structure.units)
                .map(|(material, unit)| (unit.placement.x, unit.placement.y, material))
                .collect();

            let mut staged_ledger = self.energy_ledger;
            let mut staged_energy = organism.usable_energy;
            if staged_energy > 0.0 {
                let sink = crate::energy_ledger::EnergyTransaction {
                    reason: crate::energy_ledger::EnergyReason::Decomposition,
                    potential_released: 0.0,
                    usable_delta: -staged_energy,
                    structural_delta: 0.0,
                    work_expended: 0.0,
                    heat_dissipated: staged_energy,
                };
                if !staged_ledger.settle_transaction(&mut staged_energy, sink) {
                    continue;
                }
            }
            // Once the dead graph has no bonds or outstanding holders, any
            // remaining retained resonance is dissipated as heat in the same
            // finalization transaction. It is never silently discarded or
            // converted into usable chemical energy.
            let next_resonant = staged_ledger.total_resonant_energy_held - resonant_energy;
            let next_heat = staged_ledger.total_heat_dissipated + resonant_energy;
            if !next_resonant.is_finite()
                || next_resonant < 0.0
                || !next_heat.is_finite()
                || resonant_energy > staged_ledger.total_resonant_energy_held + 1e-9
            {
                continue;
            }
            staged_ledger.total_resonant_energy_held = next_resonant.max(0.0);
            staged_ledger.total_heat_dissipated = next_heat;
            if !self
                .environment
                .field
                .deposit_physical_batch(deposits, &self.environment.catalog)
            {
                continue;
            }
            self.energy_ledger = staged_ledger;
            self.organisms.remove(index);

            // Activation progress is not physical material or energy. Once
            // the dead graph has been atomically released, discard its stale
            // graph-reaction progress so a later owner cannot inherit it.
            let organism_prefix = format!("{}|rev:", organism_id);
            self.chemical_reaction_accumulation
                .retain(|key, _| !key.starts_with(&organism_prefix));
        }
    }

    pub(crate) fn structural_energy_held(
        organisms: &[crate::state::Organism],
        decomposing_bodies: &[crate::decomposition::DecomposingBody],
    ) -> f64 {
        crate::reproduction::total_structural_energy_held(organisms, decomposing_bodies)
    }

    /// Reconcile all authoritative energy holders after a tick. This is
    /// inventory bookkeeping only; it does not invoke biological behavior.
    fn reconcile_energy_inventory_holders(&mut self) {
        self.energy_ledger.total_usable_energy_held = crate::reproduction::total_usable_energy_held(
            &self.organisms,
            &self.decomposing_bodies,
        ) + self.environment.chemistry_usable_energy;
        self.energy_ledger.total_resonant_energy_held =
            crate::reproduction::total_resonant_energy_held(&self.organisms)
                + self
                    .environment
                    .resonant_energy
                    .total_energy()
                    .unwrap_or(f64::NAN);
        self.energy_ledger.total_structural_energy_held =
            Self::structural_energy_held(&self.organisms, &self.decomposing_bodies)
                + Self::environmental_structural_energy_held(&self.environment);
    }

    /// Signed potential stored in realized physical bonds owned by the field.
    /// Old field bonds deserialize with zero energy rather than inventing a source.
    fn environmental_structural_energy_held(environment: &Environment) -> f64 {
        environment
            .field
            .cells
            .iter()
            .flat_map(|cell| cell.physical_materials.iter())
            .map(|material| {
                material
                    .internal_connections
                    .as_ref()
                    .map(|bonds| bonds.iter().map(|bond| -bond.bond_energy).sum::<f64>())
                    .unwrap_or(f64::NAN)
            })
            .sum()
    }
    /// Independent diagnostic oracle: recompute current stores from their owning state,
    /// rather than trusting the production ledger's cached aggregate totals.
    /// Cumulative flows and tick-zero baselines still come from the ledger because
    /// those are transaction history, not a current-state inventory.
    pub(crate) fn energy_inventory_snapshot(
        &self,
    ) -> crate::energy_ledger::EnergyInventorySnapshot {
        let current_usable_energy = crate::reproduction::total_usable_energy_held(
            &self.organisms,
            &self.decomposing_bodies,
        ) + self.environment.chemistry_usable_energy;
        let current_resonant_energy =
            crate::reproduction::total_resonant_energy_held(&self.organisms)
                + self
                    .environment
                    .resonant_energy
                    .total_energy()
                    .unwrap_or(f64::NAN);
        let current_structural_energy =
            crate::reproduction::total_structural_energy_held(
                &self.organisms,
                &self.decomposing_bodies,
            ) + Self::environmental_structural_energy_held(&self.environment);

        crate::energy_ledger::EnergyInventorySnapshot {
            initial_usable_energy: self.energy_ledger.initial_usable_energy_held,
            current_usable_energy,
            initial_resonant_energy: self.energy_ledger.initial_resonant_energy_held,
            current_resonant_energy,
            initial_structural_energy: self.energy_ledger.initial_structural_energy_held,
            current_structural_energy,
            initial_pair_interaction_potential: self
                .energy_ledger
                .initial_pair_interaction_potential_held,
            current_pair_interaction_potential: self
                .energy_ledger
                .total_pair_interaction_potential_held,
            initial_ambient_energy: self.energy_ledger.initial_ambient_energy_held,
            current_ambient_energy: self.ambient_energy.sample().total_energy,
            initial_spatial_field_energy: self.energy_ledger.initial_spatial_field_energy_held,
            current_spatial_field_energy: self.resonance_field.energy_state().total_energy,
            ambient_energy_generated: self.energy_ledger.total_ambient_energy_generated,
            ambient_energy_withdrawn: self.energy_ledger.total_ambient_energy_withdrawn,
            spatial_field_world_work: self.energy_ledger.total_spatial_field_world_work,
            spatial_field_computational_outflow: self
                .energy_ledger
                .total_spatial_field_computational_outflow,
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
            .store_physical_instance(neighbor_after_two.clone(), &simulation.environment.catalog));
        assert!(simulation.organisms[0]
            .stored_material
            .store_physical_instance(neighbor_after_one.clone(), &simulation.environment.catalog));
        assert!(simulation.organisms[0]
            .stored_material
            .store_physical_instance(source, &simulation.environment.catalog));
        assert!(simulation.organisms[0]
            .stored_material
            .store_physical_instance(neighbor_before.clone(), &simulation.environment.catalog));
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
        expected_entries[1] = StoredMaterial::Physical(malformed);
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
            chemistry_usable_energy: 0.0,
            resonant_energy: Default::default(),
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
            decision_context_key: Some("corrupted-after-reservation".into()),
            prepared_structure: None,
            prepared_stored_material: None,
            prepared_usable_energy: None,
            prepared_stress: None,
            prepared_ledger: None,
            prepared_ledger_before: None,
            prepared_usable_energy_before: None,
            combine_environmental_source: None,
            stored_source_index: None,
            environmental_source_location: Some((cell_index, 1)),
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

    fn prepare_bondless_dead_fixture(simulation: &mut Simulation) {
        let mut structure = OrganismStructure::new();
        let mut unit = StructuralUnit::new(
            "Carbon".to_string(),
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        );
        assert!(unit.realize_default_geometry(&simulation.environment.catalog));
        structure.add_unit(unit);

        let organism = &mut simulation.organisms[0];
        organism.structure = structure;
        organism.stored_material.entries.clear();
        organism.reproductive_construction = None;
        organism.active_transformation_id = None;
        organism.active_movement = None;
        organism.usable_energy = 0.0;
    }

    #[test]
    fn bondless_dead_organism_releases_material_after_holder_checks() {
        let mut simulation = Simulation::new(0xD3AD, 10.0);
        prepare_bondless_dead_fixture(&mut simulation);
        let organism_id = simulation.organisms[0].id.clone();
        let stale_progress = format!("{}|rev:0|stale-reaction", organism_id);
        simulation
            .chemical_reaction_accumulation
            .insert(stale_progress.clone(), 0.75);
        simulation
            .chemical_reaction_accumulation
            .insert("unrelated-owner|rev:0|reaction".into(), 0.25);
        let field_before = simulation.environment.field.total_amount();
        let living = HashSet::new();

        simulation.release_finished_dead_organisms(&living);

        assert!(!simulation
            .chemical_reaction_accumulation
            .contains_key(&stale_progress));
        assert_eq!(
            simulation
                .chemical_reaction_accumulation
                .get("unrelated-owner|rev:0|reaction"),
            Some(&0.25),
            "releasing one dead graph must not clear another owner's progress"
        );
        assert!(simulation.organisms.is_empty());
        assert_eq!(
            simulation.environment.field.total_amount(),
            field_before + 1.0,
            "the disconnected unit must be deposited exactly once"
        );
    }

    #[test]
    fn dead_bonded_graph_remains_owned_until_shared_chemistry_breaks_its_bonds() {
        let mut simulation = Simulation::new(0xD3B4, 10.0);
        assert!(
            !simulation.organisms[0].structure.bonds.is_empty(),
            "default fixture should contain bonded physical material"
        );
        let field_before = simulation.environment.field.total_amount();
        let bonds_before = simulation.organisms[0].structure.bonds.clone();
        let living = HashSet::new();

        simulation.release_finished_dead_organisms(&living);

        assert_eq!(simulation.organisms.len(), 1);
        assert_eq!(simulation.organisms[0].structure.bonds, bonds_before);
        assert_eq!(
            simulation.environment.field.total_amount(),
            field_before,
            "death must not bypass chemistry by releasing a still-bonded graph"
        );
    }

    #[test]
    fn organism_owned_bonded_graph_is_dormant_during_chemistry_ticks() {
        let mut simulation = Simulation::new(0xD3B5, 10.0);
        // Keep the normal world configuration. Death disables biological
        // activity; it must not install a special decomposition reaction.
        for cell in &mut simulation.environment.field.cells {
            cell.physical_materials.clear();
        }

        let owner_id = simulation.organisms[0].id.clone();
        let bonds_before = simulation.organisms[0].structure.bonds.clone();
        let units_before = simulation.organisms[0].structure.units.clone();
        simulation.organisms[0]
            .structure
            .clear_genome_constituent_ids();
        simulation.organisms[0].genome_core_identity_established = true;
        simulation.organisms[0].stored_material.entries.clear();
        simulation.organisms[0].reproductive_construction = None;
        simulation.organisms[0].active_transformation_id = None;
        simulation.organisms[0].active_movement = None;
        simulation.organisms[0].mark_structure_changed();

        assert!(!simulation.organisms[0].structure.bonds.is_empty());
        let field_before = simulation.environment.field.total_amount();
        let structural_amount_before = simulation.organisms[0]
            .structure
            .units
            .iter()
            .flat_map(|unit| unit.material.parts.iter())
            .map(|(_, amount)| *amount)
            .sum::<f64>();
        let material_before = field_before + structural_amount_before;

        // The chemistry-only tick must not run organism-owned BREAK discovery,
        // lifecycle checks, or dead-material release.
        simulation.step();

        let structural_amount_after = simulation
            .organisms
            .iter()
            .filter(|organism| organism.id == owner_id)
            .flat_map(|organism| organism.structure.units.iter())
            .flat_map(|unit| unit.material.parts.iter())
            .map(|(_, amount)| *amount)
            .sum::<f64>();
        let material_after = simulation.environment.field.total_amount() + structural_amount_after;
        assert!(
            (material_after - material_before).abs() <= 1e-9,
            "chemistry-only ticks must preserve total physical material: before={material_before}, after={material_after}"
        );
        assert_eq!(simulation.organisms.len(), 1);
        assert_eq!(simulation.organisms[0].structure.units, units_before);
        assert_eq!(simulation.organisms[0].structure.bonds, bonds_before);
        assert_eq!(simulation.environment.field.total_amount(), field_before);
        assert!(
            simulation.energy_inventory_snapshot().is_balanced(1e-8),
            "chemistry-only tick must conserve world energy: {:?}",
            simulation.energy_inventory_snapshot().residual()
        );
    }

    #[test]
    fn bondless_organism_graph_remains_dormant_during_chemistry_ticks() {
        let mut simulation = Simulation::new(0xD3B0, 10.0);
        prepare_bondless_dead_fixture(&mut simulation);
        let field_before = simulation.environment.field.total_amount();

        let organism_units_before = simulation.organisms[0].structure.units.clone();
        simulation.step();

        assert_eq!(simulation.organisms.len(), 1);
        assert_eq!(
            simulation.organisms[0].structure.units,
            organism_units_before
        );
        assert_eq!(
            simulation.environment.field.total_amount(),
            field_before,
            "biology-disabled ticks must not release organism-owned material"
        );
    }

    #[test]
    fn dead_release_settles_remaining_usable_energy_as_heat() {
        let mut simulation = Simulation::new(0xD3B3, 10.0);
        prepare_bondless_dead_fixture(&mut simulation);
        simulation.organisms[0].usable_energy = 2.0;
        let field_before = simulation.environment.field.total_amount();
        let heat_before = simulation.energy_ledger.total_heat_dissipated;

        let living = HashSet::new();
        simulation.release_finished_dead_organisms(&living);

        assert!(simulation.organisms.is_empty());
        assert_eq!(
            simulation.environment.field.total_amount(),
            field_before + 1.0
        );
        assert_eq!(
            simulation.energy_ledger.total_heat_dissipated,
            heat_before + 2.0
        );
    }

    #[test]
    fn failed_dead_material_release_preserves_organism_energy_and_field() {
        let mut simulation = Simulation::new(0xD3AE, 10.0);
        prepare_bondless_dead_fixture(&mut simulation);
        simulation.organisms[0].usable_energy = 2.0;
        simulation.organisms[0].structure.units[0].placement.x = f64::MAX;
        let field_before = simulation.environment.field.total_amount();
        let ledger_before = simulation.energy_ledger;
        let living = HashSet::new();

        simulation.release_finished_dead_organisms(&living);

        assert_eq!(simulation.organisms.len(), 1);
        assert_eq!(simulation.organisms[0].usable_energy, 2.0);
        assert_eq!(simulation.environment.field.total_amount(), field_before);
        assert_eq!(simulation.energy_ledger, ledger_before);
    }

    #[test]
    fn failed_dead_release_energy_settlement_preserves_organism_and_field() {
        let mut simulation = Simulation::new(0xD3B6, 10.0);
        prepare_bondless_dead_fixture(&mut simulation);
        simulation.organisms[0].usable_energy = f64::MAX;
        // Force the staged heat sink to overflow so settlement rejects before
        // the field batch can commit.
        simulation.energy_ledger.total_heat_dissipated = f64::MAX;
        let field_before = simulation.environment.field.total_amount();
        let ledger_before = simulation.energy_ledger;
        let living = HashSet::new();

        simulation.release_finished_dead_organisms(&living);

        assert_eq!(simulation.organisms.len(), 1);
        assert_eq!(simulation.organisms[0].usable_energy, f64::MAX);
        assert_eq!(simulation.environment.field.total_amount(), field_before);
        assert_eq!(simulation.energy_ledger, ledger_before);
    }

    #[test]
    fn dead_organism_with_unsettled_holders_is_not_released() {
        let mut simulation = Simulation::new(0xD3AF, 10.0);
        prepare_bondless_dead_fixture(&mut simulation);
        assert!(simulation.organisms[0]
            .store_material(crate::resources::Material::free_base("Hydrogen", 1.0)));
        let field_before = simulation.environment.field.total_amount();
        let living = HashSet::new();

        simulation.release_finished_dead_organisms(&living);

        assert_eq!(simulation.organisms.len(), 1);
        assert_eq!(simulation.environment.field.total_amount(), field_before);
        assert_eq!(simulation.organisms[0].stored_material.total_amount(), 1.0);
    }

    #[test]
    fn dead_organism_dissipates_retained_resonance_as_heat_on_final_release() {
        let mut simulation = Simulation::new(0xD3B2, 10.0);
        prepare_bondless_dead_fixture(&mut simulation);
        assert!(simulation.organisms[0].resonant_energy.deposit(440.0, 1.0));
        // The fixture replaces the seed organism with a one-unit graph, so
        // rebase the independent inventory oracle around that deliberate setup.
        // The manually seeded resonance is initial stored energy, not new input.
        let structural = Simulation::structural_energy_held(
            &simulation.organisms,
            &simulation.decomposing_bodies,
        );
        simulation.energy_ledger.initial_usable_energy_held = 0.0;
        simulation.energy_ledger.initial_resonant_energy_held = 1.0;
        simulation.energy_ledger.initial_structural_energy_held = structural;
        simulation.energy_ledger.total_usable_energy_held =
            crate::reproduction::total_usable_energy_held(
                &simulation.organisms,
                &simulation.decomposing_bodies,
            );
        simulation.energy_ledger.total_resonant_energy_held = 1.0;
        simulation.energy_ledger.total_structural_energy_held = structural;
        simulation.energy_ledger.total_external_energy_input = 0.0;
        simulation.energy_ledger.total_potential_energy_released = 0.0;
        simulation.energy_ledger.total_work_expended = 0.0;
        simulation.energy_ledger.total_heat_dissipated = 0.0;
        simulation.energy_ledger.total_world_tone_energy_escaped = 0.0;
        let field_before = simulation.environment.field.total_amount();
        let heat_before = simulation.energy_ledger.total_heat_dissipated;

        let living = HashSet::new();
        simulation.release_finished_dead_organisms(&living);

        assert!(simulation.organisms.is_empty());
        assert_eq!(
            simulation.environment.field.total_amount(),
            field_before + 1.0
        );
        assert_eq!(
            simulation.energy_ledger.total_heat_dissipated,
            heat_before + 1.0
        );
        assert_eq!(simulation.energy_ledger.total_resonant_energy_held, 0.0);
        assert!(
            simulation.energy_inventory_snapshot().is_balanced(1e-9),
            "final release must account retained resonance as heat: {:?}",
            simulation.energy_inventory_snapshot().residual()
        );
    }

    #[test]
    fn dead_organism_with_owner_transformation_in_flight_is_not_released() {
        let mut simulation = Simulation::new(0xD3B1, 10.0);
        prepare_bondless_dead_fixture(&mut simulation);
        let owner_id = simulation.organisms[0].id.clone();
        simulation
            .active_transformations
            .push(crate::state::ActiveTransformation {
                id: 991,
                organism_id: owner_id,
                kind: crate::state::TransformationKind::Break,
                material: crate::resources::Material::free_base("Carbon", 1.0),
                bond: None,
                stored_material: None,
                stored_bond: None,
                environmental_source: false,
                complexity: 1.0,
                duration_ticks: 1,
                remaining_ticks: 1,
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
                combine_environmental_source: None,
                stored_source_index: None,
                environmental_source_location: None,
                combine_environmental_source_index: None,
            });
        let field_before = simulation.environment.field.total_amount();

        let living = HashSet::new();
        simulation.release_finished_dead_organisms(&living);

        assert_eq!(simulation.organisms.len(), 1);
        assert_eq!(simulation.environment.field.total_amount(), field_before);
    }

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
            if let Some(construction) = organism.reproductive_construction.as_ref() {
                assert!(
                    construction.is_aggregate_owned_by(&organism.structure),
                    "material inventory cannot reconcile detached construction ownership"
                );
                // Stable IDs are metadata, not a second material owner.
                for material in construction.committed_material.iter_materials() {
                    add(&mut totals, material);
                }
            }
        }
        // In-flight BREAK/environmental COMBINE sources are authoritative
        // holders while removed from storage or the world field. Do not count
        // prepared COMBINE snapshots: they intentionally duplicate live state.
        for transformation in &simulation.active_transformations {
            if let Some(source) = transformation.stored_material.as_ref() {
                add(&mut totals, &source.material);
            }
            if let Some((_, source)) = transformation.combine_environmental_source.as_ref() {
                add(&mut totals, &source.material);
            }
        }
        for body in &simulation.decomposing_bodies {
            for unit in &body.structure.units {
                add(&mut totals, &unit.material);
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
    fn rollback_material_inventory_counts_aggregate_owned_construction_once() {
        let mut simulation = Simulation::new(992, 20.0);
        let before = rollback_material_inventory(&simulation);
        let organism = &mut simulation.organisms[0];
        let developing_structure = organism.structure.clone();
        let developing_member_ids = developing_structure.physical_constituent_ids();
        let anchor_physical_id = developing_structure.physical_id(0);

        organism.reproductive_construction = Some(crate::state::ReproductiveConstruction {
            committed_material: crate::material_storage::MaterialStorage::default(),
            developing_structure,
            developing_member_ids,
            child_genome: organism.genome.clone(),
            developmental_origin: organism.developmental_origin.clone(),
            developmental_orientation_radians: organism.developmental_orientation_radians,
            developing_stress: 0.0,
            anchor_unit_index: 0,
            anchor_physical_id,
            developing_energy: 0.0,
            developing_maintenance_debt: 0.0,
            needs_space: false,
        });

        assert_eq!(
            rollback_material_inventory(&simulation),
            before,
            "the aggregate-owned developing view must not count the same constituents twice"
        );
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
            .store_physical_instance(source.clone(), &simulation.environment.catalog));
        let inventory_before = rollback_material_inventory(&simulation);
        let removed = simulation.organisms[0]
            .stored_material
            .take_physical_at(original_count, &simulation.environment.catalog)
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
            stored_source_index: None,
            environmental_source_location: None,
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
    fn failed_stored_break_preparation_restores_escrow_through_live_tick() {
        use crate::physical_material::PhysicalMaterial;
        use crate::resources::{InternalBond, Material};
        use crate::structure::Placement;

        let mut simulation = Simulation::new(931, 10.0);
        for cell in &mut simulation.environment.field.cells {
            cell.materials.clear();
            cell.physical_materials.clear();
        }
        // Isolate the source index from the default organism inventory.
        simulation.organisms[0].stored_material.entries.clear();
        // Model a breached owner: the in-flight transaction must still return
        // its escrow even though biological decisions will no longer run.
        simulation.organisms[0].structure = crate::structure::OrganismStructure::new();
        let catalog = simulation.environment.catalog.clone();
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
                    x: 20.0,
                    y: 20.0,
                    rotation_radians: 0.0,
                },
                Placement {
                    x: 20.838,
                    y: 20.0,
                    rotation_radians: 0.0,
                },
            ],
            &catalog,
        )
        .expect("bonded stored source should be physically realizable");
        assert!(simulation.organisms[0]
            .stored_material
            .store_physical_instance(source, &simulation.environment.catalog));
        let inventory_before = rollback_material_inventory(&simulation);

        let decision = crate::decision_runtime::ActionCandidate {
            action: crate::decision::ActionKind::Break,
            context_key: Some("stored:0:bond:0".into()),
        };
        let mut next_id = 1;
        let mut transformation = Simulation::try_start_transformation(
            &mut simulation.organisms[0],
            &catalog,
            &mut next_id,
            &decision,
        )
        .expect("stored bond should reserve for BREAK");
        transformation.stored_bond = None;
        transformation.decision_context_key = Some("corrupted-after-reservation".into());
        transformation.duration_ticks = 2;
        transformation.remaining_ticks = 2;
        simulation.active_transformations.push(transformation);
        assert_eq!(
            rollback_material_inventory(&simulation),
            inventory_before,
            "reserved source must remain counted while held by an in-flight transformation"
        );

        // Organism-owned transactions are dormant in chemistry-only ticks.
        // Their escrow and ownership metadata must remain untouched.
        let transformation_id = simulation.active_transformations[0].id;
        let owner_id = simulation.organisms[0].active_transformation_id;
        simulation.step();

        assert_eq!(simulation.active_transformations.len(), 1);
        assert_eq!(simulation.active_transformations[0].id, transformation_id);
        assert_eq!(simulation.organisms[0].active_transformation_id, owner_id);
        assert_eq!(
            rollback_material_inventory(&simulation),
            inventory_before,
            "a dormant transaction must preserve all constituent ownership"
        );
        assert_eq!(simulation.organisms[0].stored_material.physical_count(), 0);
    }

    #[test]
    fn organism_id_reservation_rejects_exhaustion_without_wrapping() {
        assert_eq!(checked_next_organism_id(u64::MAX), None);
        assert_eq!(checked_next_organism_id(u64::MAX - 1), Some(u64::MAX));
    }

    #[test]
    fn initial_usable_energy_enters_through_an_explicit_source_transaction() {
        let simulation = Simulation::new(916, 10.0);
        let initial_usable_energy = simulation.organisms[0].usable_energy;
        let declared_initial_allocation = simulation.organisms[0]
            .genome
            .reproductive_energy_allocation;

        assert!(initial_usable_energy.is_finite() && initial_usable_energy > 0.0);
        assert_eq!(
            initial_usable_energy, declared_initial_allocation,
            "constructor computation allowance must not become physical organism energy"
        );
        assert_eq!(simulation.energy_ledger.initial_usable_energy_held, 0.0);
        assert_eq!(
            simulation.energy_ledger.total_external_energy_input,
            initial_usable_energy
        );
        assert!(
            simulation
                .energy_inventory_snapshot()
                .is_balanced_without_creation(1e-9),
            "initial energy source should reconcile: {:?}",
            simulation.energy_inventory_snapshot().residual()
        );
    }

    #[test]
    fn active_transformation_suppresses_new_movement_and_material_actions() {
        let simulation = Simulation::new(942, 10.0);
        let mut organism = simulation.organisms[0].clone();
        organism.active_transformation_id = Some(91);

        let eligibility = Simulation::action_eligibility(
            &organism,
            &simulation.environment,
            CurrentNeeds {
                survival: 0.0,
                reproduction: 0.0,
                development: 0.0,
            },
        );

        assert!(!eligibility.can_move);
        assert!(!eligibility.can_combine);
        assert!(!eligibility.can_break);
        assert!(!eligibility.can_expel);
    }

    #[test]
    fn live_world_tone_capture_settles_environmental_material_and_reconciles_energy() {
        let mut simulation = Simulation::new(918, 10.0);
        assert!(simulation.world_tone.is_some());
        assert!(
            simulation
                .environment
                .field
                .cells
                .iter()
                .any(|cell| !cell.physical_materials.is_empty()),
            "fixture must contain environmental physical material"
        );

        let external_before = simulation.energy_ledger.total_external_energy_input;
        simulation.step();

        assert!(
            simulation.energy_ledger.total_external_energy_input > external_before,
            "live world tone should credit new absorbed environmental energy"
        );
        // World-tone capture occurs before ordinary chemistry in this tick.
        // Captured resonance may therefore be spent immediately by a valid
        // reaction; the post-tick reservoir is not required to stay positive.
        // The independent inventory below verifies that captured input remains
        // accounted for whether it is retained, converted, or dissipated.
        let inventory = simulation.energy_inventory_snapshot();
        assert!(
            inventory.is_balanced_without_creation(1e-9),
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
        assert_eq!(inventory.initial_ambient_energy, inventory.current_ambient_energy);
        assert_eq!(inventory.initial_spatial_field_energy, 0.0);
        assert_eq!(inventory.current_spatial_field_energy, 0.0);
        assert_eq!(
            inventory.external_energy_input,
            inventory.current_usable_energy
        );
        assert_eq!(
            inventory.initial_structural_energy,
            inventory.current_structural_energy
        );
        assert!(inventory.is_balanced_without_creation(1e-9));
    }

    #[test]
    fn world_energy_inventory_reconciles_across_structural_break() {
        let mut simulation = Simulation::new(917, 10.0);
        let before = simulation.energy_inventory_snapshot();
        assert!(
            before.is_balanced_without_creation(1e-9),
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
                0.0,
                &mut simulation.energy_ledger,
            )
        );

        let after = simulation.energy_inventory_snapshot();
        assert!(
            after.is_balanced_without_creation(1e-9),
            "structural BREAK must reconcile the world inventory: residual={:?}",
            after.residual()
        );
        assert!(
            !simulation.organisms[0].structure.bonds.contains(&bond),
            "successful BREAK must remove the selected bond"
        );
    }

    #[test]
    fn world_energy_inventory_reconciles_world_tone_and_structural_break_together() {
        let mut simulation = Simulation::new(918, 10.0);
        for cell in &mut simulation.environment.field.cells {
            cell.physical_materials.clear();
        }

        let sample = simulation
            .world_tone_cycle
            .as_ref()
            .and_then(|cycle| cycle.sample(simulation.tick))
            .or_else(|| {
                simulation.world_tone.and_then(|config| {
                    config.sample(
                        simulation.tick,
                        simulation.environment.width,
                        simulation.environment.height,
                    )
                })
            })
            .expect("world-tone source must have a sample at tick zero");
        let catalog = &simulation.environment.catalog;
        let living_ids = simulation
            .organisms
            .iter_mut()
            .filter_map(|organism| {
                Simulation::has_intact_genome_core(organism, catalog).then(|| organism.id.clone())
            })
            .collect::<std::collections::HashSet<_>>();
        let external_before = simulation.energy_ledger.total_external_energy_input;
        crate::world_tone_snapshot::WorldToneOccluderSnapshot::settle_living_organism_inputs(
            &mut simulation,
            sample,
            &living_ids,
        );
        assert!(
            simulation.energy_ledger.total_external_energy_input > external_before,
            "the live world-tone source must contribute before the reaction"
        );
        let after_capture = simulation.energy_inventory_snapshot();
        assert!(
            after_capture.is_balanced_without_creation(1e-9),
            "inventory after world-tone capture: {:?}",
            after_capture.residual()
        );

        let (organism_id, bond) = {
            let organism = simulation
                .organisms
                .first()
                .expect("initial organism exists");
            (
                organism.id.clone(),
                *organism
                    .structure
                    .bonds
                    .first()
                    .expect("initial organism has a realized bond"),
            )
        };
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
                0.0,
                &mut simulation.energy_ledger,
            ),
            "structural BREAK must settle after world-tone input"
        );

        let after_break = simulation.energy_inventory_snapshot();
        assert!(
            after_break.is_balanced_without_creation(1e-9),
            "world-tone input and structural BREAK must reconcile together: {:?}",
            after_break.residual()
        );
        assert!(
            !simulation
                .organisms
                .iter()
                .find(|organism| organism.id == organism_id)
                .expect("selected organism remains present")
                .structure
                .bonds
                .contains(&bond),
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
                inventory.is_balanced_without_creation(1e-9),
                "world energy inventory must reconcile after ordinary tick {tick}: residual={:?}, inventory={inventory:?}",
                inventory.residual()
            );
        }
    }

    #[test]
    fn world_inventory_rejects_unowned_non_structural_release_in_simulation() {
        let mut simulation = Simulation::new(914, 10.0);
        let inventory_before = simulation.energy_inventory_snapshot();
        let ledger_before = simulation.energy_ledger;
        let usable_before = simulation.organisms[0].usable_energy;
        let structure_before = simulation.organisms[0].structure.clone();

        // The arithmetic balances, but this transaction has no named physical
        // source holder to debit. Rejection must leave both energy and structure
        // unchanged, not merely leave the ledger counters untouched.
        let transaction = crate::energy_ledger::EnergyTransaction {
            reason: crate::energy_ledger::EnergyReason::Transfer,
            potential_released: 1.0,
            usable_delta: 0.0,
            structural_delta: 0.0,
            work_expended: 0.0,
            heat_dissipated: 1.0,
        };
        assert!(!simulation
            .energy_ledger
            .settle_transaction(&mut simulation.organisms[0].usable_energy, transaction,));

        assert_eq!(simulation.organisms[0].usable_energy, usable_before);
        assert_eq!(
            simulation.organisms[0].structure.units,
            structure_before.units
        );
        assert_eq!(
            simulation.organisms[0].structure.bonds,
            structure_before.bonds
        );
        assert_eq!(simulation.energy_ledger, ledger_before);
        assert_eq!(simulation.energy_inventory_snapshot(), inventory_before);
    }

    #[test]
    fn stress_without_retained_resonance_cannot_authorize_a_random_bond_break() {
        let mut simulation = Simulation::new(915, 10.0);
        let before = simulation.energy_inventory_snapshot();
        assert!(
            before.is_balanced_without_creation(1e-9),
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
            after.is_balanced_without_creation(1e-9),
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
            1.0,
            Position {
                x: f64::NAN,
                y: 500.0,
            },
        )
        .expect("energy is valid even if the malformed position is rejected at deposit");
        simulation.decomposing_bodies.push(body);
        let before = simulation.environment.field.total_amount();
        let before_ledger = simulation.energy_ledger;

        simulation.process_decomposing_bodies();

        assert_eq!(simulation.decomposing_bodies.len(), 1);
        assert!(simulation.decomposing_bodies[0].is_finished());
        assert_eq!(simulation.decomposing_bodies[0].structure.units.len(), 1);
        assert_eq!(simulation.decomposing_bodies[0].energy_budget, 1.0);
        assert_eq!(simulation.energy_ledger, before_ledger);
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
            stored_source_index: None,
            environmental_source_location: None,
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

    #[test]
    fn breached_organism_retains_storage_and_construction_without_biological_progress() {
        use crate::resources::Material;

        let mut simulation = Simulation::new(0xD34D, 10.0);
        // Remove the external source so this test measures holder continuity,
        // not a legitimate world-tone input during the tick.
        simulation.world_tone = None;
        simulation.world_tone_cycle = None;

        simulation.organisms[0].stored_material.entries.clear();
        assert!(simulation.organisms[0].store_material(Material::free_base("Carbon", 1.0)));
        assert!(simulation.organisms[0].store_material(Material::free_base("Hydrogen", 1.0)));

        let mut ledger = simulation.energy_ledger;
        assert!(crate::reproduction::begin_reproduction(
            &mut simulation.organisms[0],
            &mut simulation.rng,
            &simulation.environment.catalog,
            &mut ledger,
        ));
        simulation.energy_ledger = ledger;

        // Give construction a committed physical-material holder using a real
        // transfer. Aggregate-owned construction energy remains with the parent.
        let organism = &mut simulation.organisms[0];
        let construction = organism
            .reproductive_construction
            .as_mut()
            .expect("construction started");
        let committed = organism
            .stored_material
            .take_physical_at(0, &simulation.environment.catalog)
            .expect("second stored constituent remains");
        construction
            .committed_material
            .entries
            .push(crate::material_storage::StoredMaterial::Physical(committed));
        let construction = simulation.organisms[0]
            .reproductive_construction
            .as_ref()
            .expect("construction remains parent-owned");
        assert_eq!(construction.developing_energy, 0.0);
        assert!(construction.developing_structure.units.is_empty());
        assert!(!construction.committed_material.is_empty());
        let developing_member_ids = construction.developing_member_ids.clone();
        assert!(developing_member_ids
            .iter()
            .all(|id| simulation.organisms[0].structure.unit_index(*id).is_some()));

        // Breach the parent's established core while retaining the physical
        // aggregate, including construction members. Parent-owned construction
        // must not need a duplicate graph to survive the breach.
        let all_ids = simulation.organisms[0].structure.physical_constituent_ids();
        let non_developing_ids = all_ids
            .into_iter()
            .filter(|id| !developing_member_ids.contains(id))
            .collect::<Vec<_>>();
        assert!(!non_developing_ids.is_empty());
        simulation.organisms[0]
            .structure
            .remove_units_by_physical_ids(&non_developing_ids);
        simulation.organisms[0]
            .structure
            .clear_genome_constituent_ids();
        simulation.organisms[0].genome_core_identity_established = true;
        simulation.organisms[0].mark_structure_changed();

        let material_before = rollback_material_inventory(&simulation);
        let parent_energy_before = simulation.organisms[0].usable_energy;
        let parent_storage_before = simulation.organisms[0].stored_material.entries.clone();
        let construction_before = {
            let construction = simulation.organisms[0]
                .reproductive_construction
                .as_ref()
                .expect("construction remains attached");
            (
                construction.developing_energy,
                construction.developing_stress,
                construction.developing_maintenance_debt,
                construction.developing_structure.clone(),
                construction.committed_material.entries.clone(),
                construction.needs_space,
            )
        };
        let inventory_before = simulation.energy_inventory_snapshot();

        // This regression isolates biological holder retention from the
        // default environment's independent external hotspot input.
        simulation.hotspot = None;
        simulation.step();

        assert_eq!(rollback_material_inventory(&simulation), material_before);
        assert_eq!(simulation.organisms.len(), 1);
        assert!(simulation.decomposing_bodies.is_empty());
        assert_eq!(simulation.organisms[0].usable_energy, parent_energy_before);
        assert_eq!(
            simulation.organisms[0].stored_material.entries,
            parent_storage_before
        );
        let construction = simulation.organisms[0]
            .reproductive_construction
            .as_ref()
            .expect("breached holder remains owned by the organism");
        assert_eq!(construction.developing_energy, construction_before.0);
        assert_eq!(construction.developing_stress, construction_before.1);
        assert_eq!(
            construction.developing_maintenance_debt,
            construction_before.2
        );
        assert_eq!(
            construction.developing_structure.units,
            construction_before.3.units
        );
        assert_eq!(
            construction.developing_structure.bonds,
            construction_before.3.bonds
        );
        assert!(construction.developing_structure.units.is_empty());
        assert!(developing_member_ids
            .iter()
            .all(|id| simulation.organisms[0].structure.unit_index(*id).is_some()));
        assert_eq!(
            construction.committed_material.entries,
            construction_before.4
        );
        assert_eq!(construction.needs_space, construction_before.5);

        let inventory_after = simulation.energy_inventory_snapshot();
        assert_eq!(
            inventory_after.current_usable_energy,
            inventory_before.current_usable_energy
        );
        assert_eq!(
            inventory_after.current_resonant_energy,
            inventory_before.current_resonant_energy
        );
        assert_eq!(
            inventory_after.current_structural_energy,
            inventory_before.current_structural_energy
        );
    }

    #[test]
    fn aggregate_owned_construction_is_counted_once_in_energy_inventory() {
        let mut simulation = Simulation::new(953, 20.0);
        let organism = &mut simulation.organisms[0];
        organism.usable_energy = 7.0;
        let aggregate = organism.structure.clone();
        let member_ids = aggregate.physical_constituent_ids();
        organism.reproductive_construction = Some(crate::state::ReproductiveConstruction {
            committed_material: crate::material_storage::MaterialStorage::default(),
            developing_structure: aggregate,
            developing_member_ids: member_ids,
            child_genome: organism.genome.clone(),
            developmental_origin: organism.developmental_origin.clone(),
            developmental_orientation_radians: organism.developmental_orientation_radians,
            developing_stress: 0.0,
            anchor_unit_index: 0,
            anchor_physical_id: organism.structure.physical_id(0),
            developing_energy: 0.0,
            developing_maintenance_debt: 0.0,
            needs_space: false,
        });

        let expected_structural_energy = organism
            .structure
            .bonds
            .iter()
            .map(|bond| -bond.bond_energy)
            .sum::<f64>();
        let inventory = simulation.energy_inventory_snapshot();

        assert_eq!(inventory.current_usable_energy, 7.0);
        assert_eq!(
            inventory.current_structural_energy,
            expected_structural_energy
        );
        assert_eq!(
            Simulation::structural_energy_held(&simulation.organisms, &[]),
            expected_structural_energy
        );
    }
}

#[cfg(test)]
mod live_ambient_energy_contract_tests {
    use super::Simulation;

    #[test]
    fn simulation_exposes_composed_field_queries_at_world_positions() {
        let simulation = Simulation::new(42, 10.0);
        let world_sample = simulation
            .sample_resonance_field_at(125.0, 125.0)
            .expect("world-position field query should succeed");
        let exterior_sample = simulation
            .sample_resonance_field_at(-1_000.0, -1_000.0)
            .expect("zero-extended exterior query should succeed");

        assert_eq!(world_sample.disturbance_energy_density, 0.0);
        assert_eq!(world_sample.cross_energy_density, 0.0);
        assert_eq!(
            world_sample.energy_density,
            world_sample.ambient_oscillator_energy_density
        );
        assert_eq!(exterior_sample.disturbance_energy_density, 0.0);
        assert_eq!(exterior_sample.cross_energy_density, 0.0);
        assert_eq!(
            exterior_sample.energy_density,
            exterior_sample.ambient_oscillator_energy_density
        );
    }

    #[test]
    fn nonzero_live_field_reconciles_through_the_independent_world_inventory() {
        let mut simulation = Simulation::new(42, 10.0);
        simulation
            .resonance_field
            .set_initial_cell_state_for_test(
                crate::adaptive_spatial_grid::CellKey {
                    level: 0,
                    x: 16,
                    y: 16,
                },
                0.0,
                0.5,
            );

        // Treat the seeded disturbance as part of the test's opening inventory,
        // not as energy created during the measured interval.
        let opening_field = simulation.resonance_field.energy_state();
        simulation.energy_ledger.initial_spatial_field_energy_held = opening_field.total_energy;
        simulation.energy_ledger.total_spatial_disturbance_energy_held =
            opening_field.disturbance_energy;
        simulation.energy_ledger.total_spatial_cross_energy_held = opening_field.cross_energy;
        simulation.energy_ledger.total_spatial_field_energy_held = opening_field.total_energy;

        for _ in 0..20 {
            simulation.advance_ambient_energy();
        }

        let inventory = simulation.energy_inventory_snapshot();
        assert!(inventory.current_spatial_field_energy > 0.0);
        assert!(inventory.spatial_field_world_work.abs() > 0.0);
        assert!(
            inventory.is_balanced(1.0e-5),
            "nonzero live field must reconcile with ambient world work: {:?}",
            inventory.residual()
        );
    }

    #[test]
    fn production_ambient_tick_updates_ambient_and_spatial_ledgers_together() {
        let mut simulation = Simulation::new(42, 10.0);
        let opening = simulation.energy_ledger.total_ambient_energy_held;

        assert!((simulation.ambient_energy.sample().energy_density - 10.0).abs() < 1.0e-12);
        for _ in 0..32 {
            simulation.advance_ambient_energy();
        }

        let ambient = simulation.ambient_energy.sample();
        let ledger = &simulation.energy_ledger;
        assert_eq!(ledger.total_ambient_energy_held, ambient.total_energy);
        assert!((opening + ledger.total_ambient_energy_generated
            - ledger.total_ambient_energy_withdrawn
            - ledger.total_ambient_energy_held)
            .abs()
            < 1.0e-7);

        let spatial = simulation.resonance_field.energy_state();
        assert_eq!(ledger.total_spatial_disturbance_energy_held, spatial.disturbance_energy);
        assert_eq!(ledger.total_spatial_cross_energy_held, spatial.cross_energy);
        assert_eq!(ledger.total_spatial_field_energy_held, spatial.total_energy);
        assert_eq!(ledger.total_spatial_field_world_work, spatial.cumulative_world_work);
        assert_eq!(
            ledger.total_spatial_field_computational_outflow,
            spatial.cumulative_computational_outflow
        );
        assert_eq!(
            ledger.total_spatial_field_accounting_residual,
            spatial.accounting_residual
        );
        assert!(spatial.accounting_residual.abs() < 1.0e-9);
        let inventory = simulation.energy_inventory_snapshot();
        assert!(
            inventory.is_balanced_without_creation(1.0e-7),
            "live ambient and spatial-field energy must reconcile in the independent world inventory: {:?}",
            inventory.residual()
        );
    }

    #[test]
    fn ambient_phase_uses_simulation_ticks_not_runtime_pacing() {
        let mut slow = Simulation::new(42, 1.0);
        let mut fast = Simulation::new(42, 900.0);
        // The complete daily envelope is covered by the standalone ambient-mode
        // test. This integration test only needs enough live ticks to prove that
        // runtime pacing does not alter logical-tick advancement.
        for _ in 0..32 {
            slow.advance_ambient_energy();
            fast.advance_ambient_energy();
        }
        assert_eq!(slow.ambient_energy.sample(), fast.ambient_energy.sample());
        assert_eq!(
            slow.energy_ledger.total_ambient_energy_generated,
            fast.energy_ledger.total_ambient_energy_generated
        );
        assert_eq!(
            slow.energy_ledger.total_ambient_energy_withdrawn,
            fast.energy_ledger.total_ambient_energy_withdrawn
        );
    }
}
