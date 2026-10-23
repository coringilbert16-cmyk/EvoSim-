use crate::decision::ActionKind;
use crate::decision_runtime::ActionCandidate;
use crate::energy_ledger::{EnergyLedgerAuthority, EnergyReason, EnergyTransaction};
use crate::state::{ActiveTransformation, EnergyLedger, Environment, Organism, Simulation};
use crate::structure::Placement;

/// Release the intrinsic potential stored in an existing physical bond.
///
/// This is distinct from material decomposition: the constituent potential was
/// already present before the bond formed, while bond potential was explicitly
/// allocated during COMBINE. Breaking the physical bond therefore releases the
/// bond potential exactly once.
pub(crate) fn bond_break_energy_yield(
    bond_energy: f64,
    processing_efficiency: f64,
) -> Option<(f64, f64, f64)> {
    if !bond_energy.is_finite() || bond_energy < 0.0 || !processing_efficiency.is_finite() {
        return None;
    }
    let efficiency = processing_efficiency.clamp(0.0, 1.0);
    let usable = bond_energy * efficiency;
    let heat = (bond_energy - usable).max(0.0);
    (usable.is_finite() && heat.is_finite() && (bond_energy - usable - heat).abs() <= 1e-9)
        .then_some((bond_energy, usable, heat))
}

pub(crate) fn chemical_break_energy_yield(
    bond_energy: f64,
    disruption_cost: f64,
    processing_efficiency: f64,
) -> Option<(f64, f64, f64)> {
    if !bond_energy.is_finite()
        || bond_energy < 0.0
        || !disruption_cost.is_finite()
        || disruption_cost < 0.0
        || !processing_efficiency.is_finite()
    {
        return None;
    }
    let efficiency = processing_efficiency.clamp(0.0, 1.0);
    // The accumulated reaction quantity is an activation signal, not an
    // energy reservoir. The removed bond's stored energy pays disruption work
    // first; any shortfall is charged to the organism's usable-energy holder.
    let surplus = (bond_energy - disruption_cost).max(0.0);
    let deficit = (disruption_cost - bond_energy).max(0.0);
    let usable = surplus * efficiency - deficit;
    let heat = (surplus - surplus * efficiency).max(0.0);
    (usable.is_finite()
        && heat.is_finite()
        && (usable - bond_energy + disruption_cost + heat).abs() <= 1e-9)
        .then_some((bond_energy, usable, heat))
}

/// Apply a chemistry-caused BREAK after the chemistry system has crossed
/// the activation threshold. Reaction accumulation is a trigger, not an energy
/// reservoir: disruption work is paid from the removed bond's stored energy
/// first, with any shortfall debited from the organism's usable-energy holder.
/// Settle a chemistry BREAK where compatible retained resonance supplies part
/// of the disruption work. The reservoir debit is committed by the caller only
/// after this transaction succeeds; the remaining work is charged to the
/// removed bond's energy/usable-energy balance.
pub(crate) fn settle_chemical_break_energy_with_resonance(
    organism: &mut Organism,
    bond: crate::structure::Bond,
    reaction_energy: f64,
    disruption_cost: f64,
    resonant_work: f64,
    processing_efficiency: f64,
    ledger: &mut EnergyLedger,
) -> bool {
    if !resonant_work.is_finite()
        || resonant_work < 0.0
        || !disruption_cost.is_finite()
        || disruption_cost < 0.0
        || resonant_work > disruption_cost
    {
        return false;
    }
    let bond_supplied_work = disruption_cost - resonant_work;
    let next_total_work = ledger.total_work_expended + disruption_cost;
    if !next_total_work.is_finite()
        || !crate::chemistry::can_chemical_break(reaction_energy, bond_supplied_work)
    {
        return false;
    }
    let Some((gross, usable, heat)) =
        chemical_break_energy_yield(bond.bond_energy, bond_supplied_work, processing_efficiency)
    else {
        return false;
    };
    let tx = EnergyTransaction {
        reason: EnergyReason::Break,
        potential_released: 0.0,
        usable_delta: usable,
        structural_delta: -gross,
        // This balanced subtransaction charges the bond-supplied share. The
        // resonant share is added to the same cumulative work total below.
        work_expended: bond_supplied_work,
        heat_dissipated: heat,
    };
    if !settle_break_transaction(organism, bond, tx, ledger) {
        return false;
    }
    // Prevalidated above so this final ledger update cannot overflow after the
    // structure has changed. The reservoir debit is committed by the caller.
    ledger.total_work_expended = next_total_work;
    true
}

/// Shared commit path for every BREAK of a realized structural bond. The
/// candidate graph is validated before settlement; only a successful ledger
/// transaction is followed by committing the graph mutation and heat stress.
fn settle_break_transaction(
    organism: &mut Organism,
    bond: crate::structure::Bond,
    transaction: EnergyTransaction,
    ledger: &mut EnergyLedger,
) -> bool {
    let mut trial_structure = organism.structure.clone();
    if trial_structure.break_matching_bond(bond).is_none() {
        return false;
    }
    if !ledger.settle_transaction(&mut organism.usable_energy, transaction) {
        return false;
    }
    organism.structure = trial_structure;
    organism.mark_structure_changed();
    organism.add_transaction_stress(transaction.heat_dissipated);
    true
}

/// Destabilize a structural bond when the organism's retained resonance
/// exceeds the energy held by its complete bond network. The total bond energy
/// is a geometry-dependent handling capacity, not a hard reservoir cap: excess
/// energy remains stored until compatible resonance can do physical work.
pub(crate) fn resolve_resonance_overload_break(
    organism: &mut Organism,
    environment: &Environment,
    ledger: &mut EnergyLedger,
) -> bool {
    let Some(retained_energy) = organism.resonant_energy.total_energy() else {
        return false;
    };
    let mut capacity = 0.0;
    for bond in &organism.structure.bonds {
        if !bond.bond_energy.is_finite() || bond.bond_energy < 0.0 {
            return false;
        }
        capacity += bond.bond_energy;
        if !capacity.is_finite() {
            return false;
        }
    }
    if retained_energy <= capacity + 1e-9 {
        return false;
    }

    if crate::cavity::analyze_genome_cavity(&organism.structure, &environment.catalog).is_err() {
        return false;
    }
    let baselines = crate::resources::ResourceBaselines::from_catalog(&environment.catalog);
    let mut best: Option<(usize, f64, f64)> = None;
    for index in resonance_break_candidate_indices(organism, environment) {
        let Some(bond) = organism.structure.bonds.get(index).copied() else {
            continue;
        };
        let Some(barrier) = crate::chemistry::activation_barrier_from_bond_strength(bond.strength)
        else {
            continue;
        };
        if barrier <= 1e-9 {
            continue;
        }
        let Some(unit_a) = organism
            .structure
            .unit_index(bond.endpoint_a.constituent_id)
        else {
            continue;
        };
        let Some(unit_b) = organism
            .structure
            .unit_index(bond.endpoint_b.constituent_id)
        else {
            continue;
        };
        let Some(properties_a) = organism
            .structure
            .units
            .get(unit_a)
            .and_then(|unit| unit.properties(&environment.catalog))
        else {
            continue;
        };
        let Some(properties_b) = organism
            .structure
            .units
            .get(unit_b)
            .and_then(|unit| unit.properties(&environment.catalog))
        else {
            continue;
        };
        let frequency_a = crate::harmonics::natural_frequency_hz(properties_a, baselines);
        let frequency_b = crate::harmonics::natural_frequency_hz(properties_b, baselines);
        if !frequency_a.is_finite()
            || frequency_a <= 0.0
            || !frequency_b.is_finite()
            || frequency_b <= 0.0
        {
            continue;
        }
        let reaction_frequency_hz = (0.5 * frequency_a.ln() + 0.5 * frequency_b.ln()).exp();
        let compatible = organism
            .resonant_energy
            .compatible_energy(reaction_frequency_hz);
        if !reaction_frequency_hz.is_finite()
            || reaction_frequency_hz <= 0.0
            || !compatible.is_finite()
            || compatible <= 1e-9
        {
            continue;
        }
        let score = compatible / barrier;
        if !score.is_finite() {
            continue;
        }
        if best.map_or(true, |(_, _, best_score)| score > best_score) {
            best = Some((index, reaction_frequency_hz, score));
        }
    }

    let Some((target_index, reaction_frequency_hz, _)) = best else {
        return false;
    };
    let Some(target) = organism.structure.bonds.get(target_index).copied() else {
        return false;
    };
    let Some(barrier) = crate::chemistry::activation_barrier_from_bond_strength(target.strength)
    else {
        return false;
    };
    let mut trial_reservoir = organism.resonant_energy.clone();
    let resonant_work = trial_reservoir.withdraw_compatible(reaction_frequency_hz, barrier);
    if !resonant_work.is_finite() || resonant_work <= 1e-9 {
        return false;
    }
    // The overload event itself supplies the full activation signal; the
    // settlement below separately accounts how much work came from resonance.
    let reaction_energy = barrier;
    let processing_efficiency = organism.genome.processing_efficiency();
    if !settle_chemical_break_energy_with_resonance(
        organism,
        target,
        reaction_energy,
        barrier,
        resonant_work,
        processing_efficiency,
        ledger,
    ) {
        return false;
    }
    organism.resonant_energy = trial_reservoir;
    true
}

fn resonance_break_candidate_indices(organism: &Organism, environment: &Environment) -> Vec<usize> {
    let genome_bonds =
        crate::cavity::analyze_genome_cavity(&organism.structure, &environment.catalog)
            .ok()
            .flatten()
            .map(|cavity| cavity.boundary_bond_indices(&organism.structure, &environment.catalog))
            .unwrap_or_default();

    // Genome bonds are protected while ordinary structural bonds remain.
    // Once ordinary bonds are exhausted, compatible resonance may destabilize
    // a genome-boundary bond through the same chemistry pathway.
    let ordinary_candidates: Vec<usize> = organism
        .structure
        .bonds
        .iter()
        .enumerate()
        .filter_map(|(index, _)| (!genome_bonds.contains(&index)).then_some(index))
        .collect();
    if !ordinary_candidates.is_empty() {
        return ordinary_candidates;
    }

    genome_bonds
}

impl Simulation {
    pub(crate) fn try_start_transformation(
        organism: &mut Organism,
        _catalog: &[crate::resources::BaseResource],
        next_id: &mut u64,
        decision: &ActionCandidate,
    ) -> Option<ActiveTransformation> {
        if decision.action != ActionKind::Break || organism.active_transformation_id.is_some() {
            return None;
        }
        let next_id_after = (*next_id).checked_add(1)?;
        let key = decision.context_key.as_deref()?;
        let rest = key.strip_prefix("stored:")?;
        let (storage_index, bond_part) = rest.split_once(":bond:")?;
        let storage_index = storage_index.parse::<usize>().ok()?;
        let bond_index = bond_part.parse::<usize>().ok()?;

        let removed = organism.stored_material.entries.get(storage_index)?;
        let bond_count = match removed {
            crate::material_storage::StoredMaterial::Physical(instance)
                if instance.is_realized() =>
            {
                instance.internal_connections.as_ref()?.len()
            }
            _ => return None,
        };
        if bond_index >= bond_count {
            return None;
        }

        // Preserve the relative order of the other entries so a failed operation can
        // restore the source at its exact original index.
        let removed = organism.stored_material.entries.remove(storage_index);
        let crate::material_storage::StoredMaterial::Physical(stored_material) = removed;
        let stored_bond = stored_material
            .internal_connections
            .as_ref()?
            .get(bond_index)?
            .clone();

        let complexity = crate::math::complexity(2.0);
        let duration = 1_u64.max(complexity.ceil() as u64);
        let t = ActiveTransformation {
            id: *next_id,
            organism_id: organism.id.clone(),
            kind: crate::state::TransformationKind::Break,
            material: crate::resources::Material::free_base("", 0.0),
            bond: None,
            stored_material: Some(stored_material),
            stored_bond: Some(stored_bond),
            environmental_source: false,
            complexity,
            duration_ticks: duration,
            remaining_ticks: duration,
            prepared_energy: None,
            pending_experience: None,
            decision_context_key: decision.context_key.clone(),
            prepared_structure: None,
            prepared_stored_material: None,
            prepared_usable_energy: None,
            prepared_stress: None,
            prepared_ledger: None,
            prepared_ledger_before: None,
            prepared_usable_energy_before: None,
            combine_environmental_source: None,
            stored_source_index: Some(storage_index),
            environmental_source_location: None,
            combine_environmental_source_index: None,
        };
        *next_id = next_id_after;
        organism.active_transformation_id = Some(t.id);
        Some(t)
    }
}

pub(crate) fn environmental_break_candidates(
    organism: &Organism,
    environment: &Environment,
) -> Vec<ActionCandidate> {
    let Ok(regions) = crate::interior_geometry::find_accessible_interior_regions(
        &organism.structure,
        &environment.catalog,
    ) else {
        return Vec::new();
    };
    if regions.is_empty() {
        return Vec::new();
    }
    let Some(body) = crate::organism_geometry::OrganismBodyGeometry::from_structure(
        &organism.structure,
        &environment.catalog,
    ) else {
        return Vec::new();
    };

    let cells = environment
        .field
        .cells_intersecting_bounds(body.min_x, body.max_x, body.min_y, body.max_y);
    let mut candidates = Vec::new();

    for cell_index in cells {
        let Some(cell) = environment.field.cells.get(cell_index) else {
            continue;
        };
        for (material_index, instance) in cell.physical_materials.iter().enumerate() {
            let Some(connections) = instance.internal_connections.as_ref() else {
                continue;
            };
            let Some(placements) = instance.placements.as_ref() else {
                continue;
            };
            if placements.len() != instance.material.parts.len() {
                continue;
            }

            let mut hypothetical = organism.structure.clone();
            let Some(indices) = crate::material_restoration::restore_material(
                &mut hypothetical,
                instance,
                crate::structure::Placement {
                    x: 0.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
                &environment.catalog,
            ) else {
                continue;
            };

            for (bond_index, connection) in connections.iter().enumerate() {
                let Some(&a_index) = indices.get(connection.part_a) else {
                    continue;
                };
                let Some(&b_index) = indices.get(connection.part_b) else {
                    continue;
                };
                let Some(a) = connection
                    .endpoint_a
                    .world_point(&hypothetical.units[a_index], &environment.catalog)
                else {
                    continue;
                };
                let Some(b) = connection
                    .endpoint_b
                    .world_point(&hypothetical.units[b_index], &environment.catalog)
                else {
                    continue;
                };
                if regions.iter().any(|region| {
                    region.contains_point(a.x, a.y) || region.contains_point(b.x, b.y)
                }) {
                    candidates.push(ActionCandidate {
                        action: ActionKind::Break,
                        context_key: Some(format!(
                            "environment:cell:{cell_index}:material:{material_index}:bond:{bond_index}"
                        )),
                    });
                }
            }
        }
    }
    candidates
}

pub(crate) fn has_environmental_break_candidate(
    organism: &Organism,
    environment: &Environment,
) -> bool {
    // Eligibility must use the same endpoint-access rule as candidate
    // generation. Merely finding a bonded environmental component near the
    // organism is not enough: at least one endpoint of the target bond must
    // actually lie inside accessible organism interior.
    !environmental_break_candidates(organism, environment).is_empty()
}

fn parse_environmental_break_context_key(key: &str) -> Option<(usize, usize, usize)> {
    let rest = key.strip_prefix("environment:")?;
    let (cell_material_part, bond_part) = rest.rsplit_once(":bond:")?;
    let (cell_part, material_part) = cell_material_part.rsplit_once(":material:")?;
    Some((
        cell_part.strip_prefix("cell:")?.parse().ok()?,
        material_part.parse().ok()?,
        bond_part.parse().ok()?,
    ))
}

fn environmental_break_source_at(
    environment: &Environment,
    cell_index: usize,
    material_index: usize,
    bond_index: usize,
) -> Option<(
    crate::physical_material::PhysicalMaterial,
    crate::physical_material::PhysicalMaterialBond,
)> {
    let instance = environment
        .field
        .cells
        .get(cell_index)?
        .physical_materials
        .get(material_index)?;
    let target = instance
        .internal_connections
        .as_ref()?
        .get(bond_index)?
        .clone();
    Some((instance.clone(), target))
}

pub(crate) fn try_start_environmental_break(
    organism: &mut Organism,
    environment: &mut Environment,
    next_id: &mut u64,
    decision: &ActionCandidate,
) -> Option<ActiveTransformation> {
    if decision.action != ActionKind::Break || organism.active_transformation_id.is_some() {
        return None;
    }
    let next_id_after = (*next_id).checked_add(1)?;
    let key = decision.context_key.as_deref()?;
    let (cell_index, material_index, bond_index) = parse_environmental_break_context_key(key)?;

    let regions = crate::interior_geometry::find_accessible_interior_regions(
        &organism.structure,
        &environment.catalog,
    )
    .ok()?;
    if regions.is_empty() {
        return None;
    }
    let body = crate::organism_geometry::OrganismBodyGeometry::from_structure(
        &organism.structure,
        &environment.catalog,
    )?;
    let candidate_cells = environment
        .field
        .cells_intersecting_bounds(body.min_x, body.max_x, body.min_y, body.max_y);
    if !candidate_cells.contains(&cell_index) {
        return None;
    }

    let (instance, target) =
        environmental_break_source_at(environment, cell_index, material_index, bond_index)?;
    let accessible = {
        let placements = instance.placements.as_ref()?;
        if placements.len() != instance.material.parts.len() {
            return None;
        }

        let mut hypothetical = organism.structure.clone();
        let indices = crate::material_restoration::restore_material(
            &mut hypothetical,
            &instance,
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
            &environment.catalog,
        )?;
        let &a_index = indices.get(target.part_a)?;
        let &b_index = indices.get(target.part_b)?;
        let point_a = target
            .endpoint_a
            .world_point(&hypothetical.units[a_index], &environment.catalog)?;
        let point_b = target
            .endpoint_b
            .world_point(&hypothetical.units[b_index], &environment.catalog)?;
        regions.iter().any(|region| {
            region.contains_point(point_a.x, point_a.y)
                || region.contains_point(point_b.x, point_b.y)
        })
    };
    if !accessible {
        return None;
    }

    let removed = environment
        .field
        .cells
        .get(cell_index)?
        .physical_materials
        .get(material_index)?
        .clone();
    environment.field.cells[cell_index]
        .physical_materials
        .remove(material_index);
    environment.field.mark_changed_at_index(cell_index);

    let complexity = crate::math::complexity(2.0);
    let duration = 1_u64.max(complexity.ceil() as u64);
    let t = ActiveTransformation {
        id: *next_id,
        organism_id: organism.id.clone(),
        kind: crate::state::TransformationKind::Break,
        material: crate::resources::Material::free_base("", 0.0),
        bond: None,
        stored_material: Some(removed),
        stored_bond: Some(target),
        environmental_source: true,
        complexity,
        duration_ticks: duration,
        remaining_ticks: duration,
        prepared_energy: None,
        pending_experience: None,
        decision_context_key: decision.context_key.clone(),
        prepared_structure: None,
        prepared_stored_material: None,
        prepared_usable_energy: None,
        prepared_stress: None,
        prepared_ledger: None,
        prepared_ledger_before: None,
        prepared_usable_energy_before: None,
        combine_environmental_source: None,
        stored_source_index: None,
        environmental_source_location: Some((cell_index, material_index)),
        combine_environmental_source_index: None,
    };
    *next_id = next_id_after;
    organism.active_transformation_id = Some(t.id);
    Some(t)
}

impl Simulation {
    pub(crate) fn prepare_transformation(
        transformation: &mut ActiveTransformation,
        organism: &mut Organism,
        environment: &mut Environment,
        ledger: &mut EnergyLedger,
        seed_scale_reference: Option<(f64, f64)>,
    ) -> bool {
        if matches!(
            transformation.kind,
            crate::state::TransformationKind::Combine
        ) {
            // Resolve and commit on the same completion tick. Preparing a
            // replacement graph one tick early would leave it stale while
            // chemical or stress-driven BREAK can still mutate the live graph.
            let mut trial_organism = organism.clone();
            // Candidate COMBINE only touches material in the organism's local
            // field bounds. Keep the trial environment sparse so a large
            // persistent resource field is not cloned for every transformation.
            let mut trial_environment = Environment {
                width: environment.width,
                height: environment.height,
                catalog: environment.catalog.clone(),
                field: crate::environment::ActiveMaterialField::new(
                    environment.width,
                    environment.height,
                    environment.field.cell_size,
                ),
            };
            if let Some(body) = crate::organism_geometry::OrganismBodyGeometry::from_structure(
                &organism.structure,
                &environment.catalog,
            ) {
                for cell_index in environment
                    .field
                    .cells_intersecting_bounds(body.min_x, body.max_x, body.min_y, body.max_y)
                {
                    if let (Some(source), Some(target)) = (
                        environment.field.cells.get(cell_index),
                        trial_environment.field.cells.get_mut(cell_index),
                    ) {
                        target.physical_materials = source.physical_materials.clone();
                    }
                }
            }
            let original_ledger = *ledger;
            let mut trial_ledger = original_ledger;
            let mut cache = crate::contact::ConnectionCompatibilityCache;
            let developmental_context = seed_scale_reference.and_then(|reference| {
                crate::developmental_decision::context(
                    &mut trial_organism,
                    &trial_environment,
                    reference,
                )
            });
            let developmental = developmental_context.as_ref().map(|context| {
                (
                    &context.blueprint,
                    context.origin,
                    context.orientation,
                    context.preferred_length,
                )
            });
            let Some(_attempt) = crate::combine_runtime::try_combine(
                &mut trial_organism,
                &mut trial_environment,
                &mut cache,
                &mut trial_ledger,
                developmental,
            ) else {
                organism.active_transformation_id = None;
                return false;
            };

            let prepared_energy = trial_organism.usable_energy - organism.usable_energy;
            let prepared_stress = trial_organism.stress - organism.stress;
            if !prepared_energy.is_finite() || !prepared_stress.is_finite() {
                organism.active_transformation_id = None;
                return false;
            }
            let ledger_delta = EnergyLedger {
                total_potential_energy_released: trial_ledger.total_potential_energy_released
                    - original_ledger.total_potential_energy_released,
                total_structural_energy_delta: trial_ledger.total_structural_energy_delta
                    - original_ledger.total_structural_energy_delta,
                total_external_energy_input: trial_ledger.total_external_energy_input
                    - original_ledger.total_external_energy_input,
                total_world_tone_energy_unabsorbed: trial_ledger.total_world_tone_energy_unabsorbed
                    - original_ledger.total_world_tone_energy_unabsorbed,
                total_world_tone_energy_escaped: trial_ledger.total_world_tone_energy_escaped
                    - original_ledger.total_world_tone_energy_escaped,
                total_usable_energy_gained: trial_ledger.total_usable_energy_gained
                    - original_ledger.total_usable_energy_gained,
                total_work_expended: trial_ledger.total_work_expended
                    - original_ledger.total_work_expended,
                total_heat_dissipated: trial_ledger.total_heat_dissipated
                    - original_ledger.total_heat_dissipated,
                total_usable_energy_held: trial_ledger.total_usable_energy_held
                    - original_ledger.total_usable_energy_held,
                total_resonant_energy_held: trial_ledger.total_resonant_energy_held
                    - original_ledger.total_resonant_energy_held,
                // This value is a transaction delta, not a live inventory snapshot.
                initial_usable_energy_held: 0.0,
                initial_resonant_energy_held: 0.0,
                total_structural_energy_held: 0.0,
                initial_structural_energy_held: 0.0,
                // This is a transaction delta; the source-once guard is unchanged.
                initial_condition_energy_settled: original_ledger.initial_condition_energy_settled,
            };
            if let Some((cell_index, source)) = _attempt.environmental_source.as_ref() {
                let Some(cell) = environment.field.cells.get_mut(*cell_index) else {
                    organism.active_transformation_id = None;
                    return false;
                };
                let Some(index) = cell
                    .physical_materials
                    .iter()
                    .position(|candidate| candidate == source)
                else {
                    organism.active_transformation_id = None;
                    return false;
                };
                cell.physical_materials.remove(index);
                environment.field.mark_changed_at_index(*cell_index);
                transformation.environmental_source = true;
                transformation.combine_environmental_source = Some((*cell_index, source.clone()));
                transformation.combine_environmental_source_index = Some(index);
            }
            transformation.prepared_structure = Some(trial_organism.structure);
            transformation.prepared_stored_material = Some(trial_organism.stored_material);
            transformation.prepared_usable_energy = Some(prepared_energy);
            transformation.prepared_stress = Some(prepared_stress);
            transformation.prepared_ledger = Some(ledger_delta);
            transformation.prepared_ledger_before = Some(original_ledger);
            transformation.prepared_usable_energy_before = Some(organism.usable_energy);

            let before_energy = organism.usable_energy;
            let before_stress = organism.stress;
            organism.usable_energy += prepared_energy;
            ledger.total_potential_energy_released += ledger_delta.total_potential_energy_released;
            ledger.total_structural_energy_delta += ledger_delta.total_structural_energy_delta;
            ledger.total_external_energy_input += ledger_delta.total_external_energy_input;
            ledger.total_world_tone_energy_unabsorbed +=
                ledger_delta.total_world_tone_energy_unabsorbed;
            ledger.total_world_tone_energy_escaped += ledger_delta.total_world_tone_energy_escaped;
            ledger.total_usable_energy_gained += ledger_delta.total_usable_energy_gained;
            ledger.total_work_expended += ledger_delta.total_work_expended;
            ledger.total_heat_dissipated += ledger_delta.total_heat_dissipated;
            ledger.total_usable_energy_held += ledger_delta.total_usable_energy_held;
            ledger.total_resonant_energy_held += ledger_delta.total_resonant_energy_held;
            transformation.prepared_energy = Some((before_energy, before_stress, 0.0));
            return true;
        }
        let Some(stored) = transformation.stored_material.as_ref() else {
            organism.active_transformation_id = None;
            return false;
        };
        let Some(target) = transformation.stored_bond.as_ref() else {
            organism.active_transformation_id = None;
            return false;
        };
        // Stored environmental compounds retain their bond endpoints, but
        // no energy was allocated when those pre-existing bonds were created.
        // Breaking one therefore changes connectivity only; deriving a fresh
        // energy credit from resource properties would create energy.
        // Validate the exact bond before settling the transaction. The physical
        // structure itself is not changed until the following tick.
        if stored.break_internal_bond(target).is_none() {
            organism.active_transformation_id = None;
            return false;
        }

        let before_energy = organism.usable_energy;
        let tx = EnergyTransaction {
            reason: EnergyReason::Break,
            potential_released: 0.0,
            usable_delta: 0.0,
            structural_delta: 0.0,
            work_expended: 0.0,
            heat_dissipated: 0.0,
        };
        if !ledger.settle_transaction(&mut organism.usable_energy, tx) {
            organism.active_transformation_id = None;
            return false;
        }

        // Preserve the usual completion marker without claiming that stored
        // material released usable energy or heat.
        transformation.prepared_energy = Some((before_energy, 0.0, 0.0));
        true
    }
}

/// Return an escrowed stored-material BREAK source to its authoritative holder
/// when completion cannot commit. The active ID guards against restoring the
/// same escrow twice if a stale completion is delivered.
pub(crate) fn original_stored_source_index(transformation: &ActiveTransformation) -> Option<usize> {
    if let Some(index) = transformation.stored_source_index {
        return Some(index);
    }
    let key = transformation.decision_context_key.as_deref()?;
    let rest = key.strip_prefix("stored:")?;
    let (index, _) = rest.split_once(":bond:")?;
    index.parse().ok()
}

pub(crate) fn original_environmental_source_location(
    transformation: &ActiveTransformation,
) -> Option<(usize, usize)> {
    if let Some(location) = transformation.environmental_source_location {
        return Some(location);
    }
    let key = transformation.decision_context_key.as_deref()?;
    let rest = key.strip_prefix("environment:")?;
    let (cell_material_part, bond_part) = rest.rsplit_once(":bond:")?;
    let _bond_index = bond_part.parse::<usize>().ok()?;
    let (cell_part, material_part) = cell_material_part.rsplit_once(":material:")?;
    let cell_index = cell_part.strip_prefix("cell:")?.parse::<usize>().ok()?;
    let material_index = material_part.parse::<usize>().ok()?;
    Some((cell_index, material_index))
}

/// Restore environmental BREAK escrow to the exact field cell and list position
/// from which it was reserved. Returning false lets the caller preserve the source
/// in another explicit holder if a malformed operation has lost its location key.
pub(crate) fn restore_environmental_break_source(
    transformation: &ActiveTransformation,
    environment: &mut Environment,
) -> bool {
    if !transformation.environmental_source {
        return false;
    }
    let Some(source) = transformation.stored_material.as_ref() else {
        return false;
    };
    let Some((cell_index, material_index)) = original_environmental_source_location(transformation)
    else {
        return false;
    };
    {
        let Some(cell) = environment.field.cells.get_mut(cell_index) else {
            return false;
        };
        if material_index > cell.physical_materials.len() {
            return false;
        }
        cell.physical_materials
            .insert(material_index, source.clone());
    }
    environment.field.mark_changed_at_index(cell_index);
    true
}

/// Preserve a world-space environmental source in an organism's storage
/// when its original field location is no longer usable. Convert its geometry
/// to the storage's intrinsic frame and retain its owner-relative anchor; never
/// place world coordinates directly into the intrinsic storage representation.
pub(crate) fn retain_environmental_escrow_in_field(
    source: &crate::physical_material::PhysicalMaterial,
    environment: &mut Environment,
) {
    let deposited = source
        .placements
        .as_ref()
        .and_then(|placements| placements.first())
        .is_some_and(|placement| {
            environment
                .field
                .deposit(placement.x, placement.y, source.clone())
        });
    if !deposited && !environment.field.cells.is_empty() {
        // Last-resort explicit world-field escrow. Never reinterpret a
        // world-space placement as an owner-relative storage coordinate.
        environment.field.cells[0]
            .physical_materials
            .push(source.clone());
        environment.field.mark_changed_at_index(0);
    }
}

pub(crate) fn quarantine_or_retain_environmental_source(
    source: &crate::physical_material::PhysicalMaterial,
    organism: &mut Organism,
    environment: &mut Environment,
) {
    if !quarantine_environmental_source_in_storage(source, organism, environment) {
        retain_environmental_escrow_in_field(source, environment);
    }
}

pub(crate) fn quarantine_environmental_source_in_storage(
    source: &crate::physical_material::PhysicalMaterial,
    organism: &mut Organism,
    _environment: &Environment,
) -> bool {
    let anchor = organism
        .occupied_cells
        .first()
        .cloned()
        .unwrap_or(crate::state::Position { x: 0.0, y: 0.0 });
    let Some(prepared) =
        crate::material_storage::MaterialStorage::prepare_physical_batch_at_owner_anchor(
            vec![source.clone()],
            Placement {
                x: anchor.x,
                y: anchor.y,
                rotation_radians: 0.0,
            },
        )
    else {
        return false;
    };
    organism
        .stored_material
        .commit_prepared_physical_batch(prepared);
    true
}

fn restore_failed_stored_break_source(
    transformation: &ActiveTransformation,
    organism: &mut Organism,
    environment: &mut Environment,
) {
    if organism.active_transformation_id != Some(transformation.id) {
        return;
    }
    if let Some(source) = transformation.stored_material.as_ref() {
        if transformation.environmental_source {
            if !restore_environmental_break_source(transformation, environment) {
                // Never put world-space coordinates directly into storage.
                quarantine_or_retain_environmental_source(source, organism, environment);
            }
        } else {
            // Stored BREAK escrow originated at this exact storage index. Since
            // reservation preserves the remaining entry order, reinserting here
            // restores the previous holder contents when the transaction fails.
            let entry = crate::material_storage::StoredMaterial::Physical(source.clone());
            if let Some(index) = original_stored_source_index(transformation)
                .filter(|index| *index <= organism.stored_material.entries.len())
            {
                organism.stored_material.entries.insert(index, entry);
            } else {
                // Corrupt/missing index metadata cannot support exact positional
                // rollback. Preserve the escrow without claiming the old order.
                organism.stored_material.entries.push(entry);
            }
        }
    }
    organism.active_transformation_id = None;
}

/// Reverse a staged COMBINE settlement when its physical commit state is
/// incomplete. Deltas are applied independently because serialized/corrupt
/// prepared state may have lost one optional field.
pub(crate) fn restore_combine_environmental_source(
    transformation: &ActiveTransformation,
    environment: &mut Environment,
) -> bool {
    let Some((cell_index, source)) = transformation.combine_environmental_source.as_ref() else {
        return false;
    };
    let Some(cell) = environment.field.cells.get_mut(*cell_index) else {
        return false;
    };
    let Some(insertion_index) = transformation.combine_environmental_source_index else {
        return false;
    };
    if insertion_index > cell.physical_materials.len() {
        return false;
    }
    cell.physical_materials
        .insert(insertion_index, source.clone());
    environment.field.mark_changed_at_index(*cell_index);
    true
}

fn rollback_prepared_combine(
    usable_energy: &mut f64,
    ledger: &mut EnergyLedger,
    usable_energy_before: Option<f64>,
    energy_delta: Option<f64>,
    ledger_before: Option<&EnergyLedger>,
    ledger_delta: Option<&EnergyLedger>,
) {
    if let Some(before) = usable_energy_before {
        *usable_energy = before;
    } else if let Some(delta) = energy_delta {
        // Compatibility for transformations persisted before before-state
        // snapshots were introduced. Newly prepared transactions use exact restore.
        *usable_energy -= delta;
    }

    if let Some(before) = ledger_before {
        *ledger = *before;
    } else if let Some(delta) = ledger_delta {
        // Legacy serialized transactions have only the staged delta available.
        ledger.total_potential_energy_released -= delta.total_potential_energy_released;
        ledger.total_structural_energy_delta -= delta.total_structural_energy_delta;
        ledger.total_external_energy_input -= delta.total_external_energy_input;
        ledger.total_world_tone_energy_unabsorbed -= delta.total_world_tone_energy_unabsorbed;
        ledger.total_world_tone_energy_escaped -= delta.total_world_tone_energy_escaped;
        ledger.total_usable_energy_gained -= delta.total_usable_energy_gained;
        ledger.total_work_expended -= delta.total_work_expended;
        ledger.total_heat_dissipated -= delta.total_heat_dissipated;
        ledger.total_usable_energy_held -= delta.total_usable_energy_held;
        ledger.total_resonant_energy_held -= delta.total_resonant_energy_held;
    }
}

fn pending_energy_delta(transformation: &ActiveTransformation, organism: &Organism) -> f64 {
    transformation
        .prepared_energy
        .map(|(before, _, _)| organism.usable_energy - before)
        .unwrap_or(0.0)
}

fn pending_stress_delta(transformation: &ActiveTransformation, organism: &Organism) -> f64 {
    transformation
        .prepared_energy
        .map(|(_, before, _)| organism.stress - before)
        .unwrap_or(0.0)
}

pub(crate) fn resolve_transformation(
    transformation: &ActiveTransformation,
    organism: &mut Organism,
    environment: &mut Environment,
    ledger: &mut EnergyLedger,
) {
    if matches!(
        transformation.kind,
        crate::state::TransformationKind::Combine
    ) {
        let (Some(structure), Some(stored_material), Some(stress)) = (
            transformation.prepared_structure.as_ref(),
            transformation.prepared_stored_material.as_ref(),
            transformation.prepared_stress,
        ) else {
            // Preparation settles the energy transaction before the physical
            // mutation is committed. If the prepared physical state is missing,
            // reverse that staged settlement as well as restoring any reserved
            // environmental source; otherwise a failed COMBINE can spend energy
            // and ledger-account it without ever creating the bond.
            rollback_prepared_combine(
                &mut organism.usable_energy,
                ledger,
                transformation.prepared_usable_energy_before,
                transformation.prepared_usable_energy,
                transformation.prepared_ledger_before.as_ref(),
                transformation.prepared_ledger.as_ref(),
            );
            if !restore_combine_environmental_source(transformation, environment) {
                // Convert world geometry before using storage as the fallback holder.
                if let Some((_, source)) = transformation.combine_environmental_source.as_ref() {
                    quarantine_or_retain_environmental_source(source, organism, environment);
                }
            }
            organism.active_transformation_id = None;
            return;
        };

        // Commit the prepared physical mutation on the same completion
        // tick as its energy transaction, before chemistry or stress can run.
        organism.structure = structure.clone();
        organism.stored_material = stored_material.clone();
        if stress.is_finite() {
            organism.stress += stress;
        }
        organism.mark_structure_changed();
        organism.active_transformation_id = None;

        let energy_delta = pending_energy_delta(transformation, organism);
        let stress_delta = pending_stress_delta(transformation, organism);
        crate::decision_runtime::record_consequence(
            &mut organism.decision_history,
            &ActionCandidate {
                action: ActionKind::Combine,
                context_key: transformation.decision_context_key.clone(),
            },
            crate::decision::ActionConsequence {
                energy_delta,
                stress_delta,
                developmental_delta: 0.0,
            },
        );

        if let Some(pending) = transformation.pending_experience.as_ref() {
            let after_developmental_realization = organism
                .developmental_realization_cached(&environment.catalog)
                .map(|realization| realization.overall)
                .unwrap_or(pending.before_developmental_realization);
            let consequence = crate::memory::MemoryConsequence {
                energy_delta: organism.usable_energy - pending.before_energy,
                stress_delta: organism.stress - pending.before_stress,
                developmental_delta: after_developmental_realization
                    - pending.before_developmental_realization,
                material_transformed: pending.material_transformed,
                ..Default::default()
            };
            let capacity = organism
                .genome_cavity_cached_ref(&environment.catalog)
                .filter(|cavity| cavity.qualifies())
                .map(crate::memory::memory_capacity);
            if let Some(capacity) = capacity {
                crate::memory::record_experience(
                    &mut organism.experience_memory,
                    &pending.perceptions,
                    ActionKind::Combine,
                    consequence,
                    pending.needs,
                    capacity,
                    organism.genome.memory_strength(),
                );
            }
        }
        return;
    }
    let Some(stored) = transformation.stored_material.as_ref() else {
        organism.active_transformation_id = None;
        return;
    };
    let Some(target) = transformation.stored_bond.as_ref() else {
        restore_failed_stored_break_source(transformation, organism, environment);
        return;
    };
    if transformation.prepared_energy.is_none() {
        restore_failed_stored_break_source(transformation, organism, environment);
        return;
    }
    let Some(pieces) = stored.break_internal_bond(target) else {
        restore_failed_stored_break_source(transformation, organism, environment);
        return;
    };

    if transformation.environmental_source {
        // Validate every split-piece destination first, then commit the batch.
        // This keeps the split atomic without cloning the entire world field.
        let mut deposits = Vec::with_capacity(pieces.len());
        for piece in pieces {
            let Some(placement) = piece
                .placements
                .as_ref()
                .and_then(|placements| placements.first())
            else {
                restore_failed_stored_break_source(transformation, organism, environment);
                return;
            };
            deposits.push((placement.x, placement.y, piece));
        }
        if !environment.field.deposit_physical_batch(deposits) {
            restore_failed_stored_break_source(transformation, organism, environment);
            return;
        }
    } else {
        // Stored source pieces remain in intrinsic coordinates. Validate the
        // complete split before touching the holder, then commit the prepared
        // entries without cloning unrelated inventory.
        let Some(prepared) =
            crate::material_storage::MaterialStorage::prepare_physical_batch(pieces)
        else {
            restore_failed_stored_break_source(transformation, organism, environment);
            return;
        };
        organism
            .stored_material
            .commit_prepared_physical_batch(prepared);
    }
    let (_, usable, heat) = transformation.prepared_energy.unwrap();
    organism.add_transaction_stress(heat);
    organism.active_transformation_id = None;
    crate::decision_runtime::record_consequence(
        &mut organism.decision_history,
        &ActionCandidate {
            action: ActionKind::Break,
            context_key: transformation.decision_context_key.clone(),
        },
        crate::decision::ActionConsequence {
            energy_delta: usable,
            stress_delta: heat,
            developmental_delta: 0.0,
        },
    );

    if let Some(pending) = transformation.pending_experience.as_ref() {
        let after_developmental_realization = organism
            .developmental_realization_cached(&environment.catalog)
            .map(|realization| realization.overall)
            .unwrap_or(pending.before_developmental_realization);
        let consequence = crate::memory::MemoryConsequence {
            energy_delta: organism.usable_energy - pending.before_energy,
            stress_delta: organism.stress - pending.before_stress,
            developmental_delta: after_developmental_realization
                - pending.before_developmental_realization,
            material_transformed: pending.material_transformed,
            ..Default::default()
        };
        let capacity = organism
            .genome_cavity_cached_ref(&environment.catalog)
            .filter(|cavity| cavity.qualifies())
            .map(crate::memory::memory_capacity);
        if let Some(capacity) = capacity {
            crate::memory::record_experience(
                &mut organism.experience_memory,
                &pending.perceptions,
                ActionKind::Break,
                consequence,
                pending.needs,
                capacity,
                organism.genome.memory_strength(),
            );
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_prepared_combine_rolls_back_energy_and_every_ledger_delta() {
        let mut usable_energy = 13.0;
        let mut ledger = EnergyLedger {
            total_potential_energy_released: 2.0,
            total_structural_energy_delta: 5.0,
            total_external_energy_input: 3.0,
            total_world_tone_energy_unabsorbed: 9.0,
            total_world_tone_energy_escaped: 13.0,
            total_usable_energy_gained: 7.0,
            total_work_expended: 11.0,
            total_heat_dissipated: 17.0,
            total_usable_energy_held: 19.0,
            total_resonant_energy_held: 23.0,
            ..EnergyLedger::default()
        };
        let delta = EnergyLedger {
            total_potential_energy_released: 1.0,
            total_structural_energy_delta: 2.0,
            total_external_energy_input: 1.0,
            total_world_tone_energy_unabsorbed: 2.0,
            total_world_tone_energy_escaped: 4.0,
            total_usable_energy_gained: 3.0,
            total_work_expended: 5.0,
            total_heat_dissipated: 7.0,
            total_usable_energy_held: 11.0,
            total_resonant_energy_held: 6.0,
            ..EnergyLedger::default()
        };

        rollback_prepared_combine(
            &mut usable_energy,
            &mut ledger,
            None,
            Some(5.0),
            None,
            Some(&delta),
        );

        assert_eq!(usable_energy, 8.0);
        assert_eq!(ledger.total_potential_energy_released, 1.0);
        assert_eq!(ledger.total_structural_energy_delta, 3.0);
        assert_eq!(ledger.total_external_energy_input, 2.0);
        assert_eq!(ledger.total_world_tone_energy_unabsorbed, 7.0);
        assert_eq!(ledger.total_world_tone_energy_escaped, 9.0);
        assert_eq!(ledger.total_usable_energy_gained, 4.0);
        assert_eq!(ledger.total_work_expended, 6.0);
        assert_eq!(ledger.total_heat_dissipated, 10.0);
        assert_eq!(ledger.total_usable_energy_held, 8.0);
        assert_eq!(ledger.total_resonant_energy_held, 17.0);
    }

    #[test]
    fn failed_prepared_combine_restores_exact_energy_and_ledger_snapshot() {
        let before_energy = 12.345678901234;
        let before_ledger = EnergyLedger {
            total_potential_energy_released: 1.25,
            total_structural_energy_delta: 2.5,
            total_external_energy_input: 3.75,
            initial_condition_energy_settled: true,
            total_world_tone_energy_unabsorbed: 4.125,
            total_world_tone_energy_escaped: 5.25,
            total_usable_energy_gained: 6.5,
            total_work_expended: 7.75,
            total_heat_dissipated: 8.875,
            total_usable_energy_held: 9.25,
            total_resonant_energy_held: 10.5,
            initial_resonant_energy_held: 11.75,
            initial_usable_energy_held: 12.875,
            total_structural_energy_held: 13.25,
            initial_structural_energy_held: 14.5,
        };
        let mut usable_energy = before_energy + 0.1;
        let mut ledger = EnergyLedger {
            total_potential_energy_released: before_ledger.total_potential_energy_released + 0.2,
            total_structural_energy_delta: before_ledger.total_structural_energy_delta - 0.3,
            total_external_energy_input: before_ledger.total_external_energy_input + 0.4,
            initial_condition_energy_settled: false,
            total_world_tone_energy_unabsorbed: before_ledger.total_world_tone_energy_unabsorbed
                + 0.5,
            total_world_tone_energy_escaped: before_ledger.total_world_tone_energy_escaped + 0.6,
            total_usable_energy_gained: before_ledger.total_usable_energy_gained + 0.7,
            total_work_expended: before_ledger.total_work_expended + 0.8,
            total_heat_dissipated: before_ledger.total_heat_dissipated + 0.9,
            total_usable_energy_held: before_ledger.total_usable_energy_held + 1.0,
            total_resonant_energy_held: before_ledger.total_resonant_energy_held + 1.1,
            initial_resonant_energy_held: 99.0,
            initial_usable_energy_held: 98.0,
            total_structural_energy_held: 97.0,
            initial_structural_energy_held: 96.0,
        };

        rollback_prepared_combine(
            &mut usable_energy,
            &mut ledger,
            Some(before_energy),
            Some(0.1),
            Some(&before_ledger),
            None,
        );

        assert_eq!(usable_energy, before_energy);
        assert_eq!(ledger, before_ledger);
    }

    #[test]
    fn failed_prepared_combine_restores_energy_ledger_and_environmental_source_together() {
        let catalog = crate::resources::default_catalog();
        let (mut environment, cell_index, before, source, after) =
            environmental_break_test_inventory(&catalog);
        environment.field.cells[cell_index].physical_materials =
            vec![before.clone(), after.clone()];

        let mut simulation = crate::state::Simulation::new(931, 10.0);
        let mut organism = simulation.organisms.remove(0);
        let before_energy = 12.345678901234;
        organism.usable_energy = before_energy + 2.5;
        organism.stress = 4.75;
        organism.active_transformation_id = Some(733);

        let before_ledger = EnergyLedger::default();
        let ledger_delta = EnergyLedger {
            total_work_expended: 2.0,
            total_heat_dissipated: 3.0,
            total_external_energy_input: 5.0,
            ..EnergyLedger::default()
        };
        let mut staged_ledger = ledger_delta;

        let transformation = ActiveTransformation {
            id: 733,
            organism_id: organism.id.clone(),
            kind: crate::state::TransformationKind::Combine,
            material: crate::resources::Material::free_base("", 0.0),
            bond: None,
            stored_material: None,
            stored_bond: None,
            environmental_source: true,
            complexity: 1.0,
            duration_ticks: 1,
            remaining_ticks: 0,
            prepared_energy: Some((before_energy, organism.stress, 0.0)),
            pending_experience: None,
            decision_context_key: None,
            prepared_structure: None,
            prepared_stored_material: Some(organism.stored_material.clone()),
            prepared_usable_energy: Some(2.5),
            prepared_stress: Some(0.75),
            prepared_ledger: Some(ledger_delta),
            prepared_ledger_before: Some(before_ledger),
            prepared_usable_energy_before: Some(before_energy),
            combine_environmental_source: Some((cell_index, source.clone())),
            stored_source_index: None,
            environmental_source_location: None,
            combine_environmental_source_index: Some(1),
        };

        resolve_transformation(
            &transformation,
            &mut organism,
            &mut environment,
            &mut staged_ledger,
        );

        assert_eq!(organism.usable_energy, before_energy);
        assert_eq!(
            organism.stress, 4.75,
            "failed COMBINE must not apply staged stress"
        );
        assert_eq!(staged_ledger, before_ledger);
        assert_eq!(
            environment.field.cells[cell_index].physical_materials,
            vec![before, source, after],
            "failed COMBINE must restore the source at its original index"
        );
        assert_eq!(organism.active_transformation_id, None);
    }

    #[test]
    fn bond_break_releases_only_stored_bond_potential() {
        let (gross, usable, heat) = bond_break_energy_yield(4.0, 0.5).unwrap();
        assert_eq!(gross, 4.0);
        assert_eq!(usable, 2.0);
        assert_eq!(heat, 2.0);
        assert!((gross - usable - heat).abs() < 1e-12);
    }

    #[test]
    fn bond_break_rejects_non_finite_processing_efficiency() {
        assert!(bond_break_energy_yield(4.0, f64::NAN).is_none());
        assert!(bond_break_energy_yield(4.0, f64::INFINITY).is_none());
        assert!(bond_break_energy_yield(4.0, f64::NEG_INFINITY).is_none());
    }

    #[test]
    fn failed_stored_break_completion_restores_escrowed_source() {
        let catalog = crate::resources::default_catalog();
        let mut organism = crate::state::Organism {
            id: "stored-break-failure-test".into(),
            developmental_origin: crate::state::Position { x: 0.0, y: 0.0 },
            developmental_orientation_radians: 0.0,
            occupied_cells: Vec::new(),
            genome: crate::genome::initial_genome(),
            harmonic_spectrum: crate::harmonics::ToneSpectrum::empty(),
            resonant_energy: crate::harmonics::ResonantEnergyReservoir::default(),
            experience_memory: crate::memory::ExperienceMemory::default(),
            pending_movement_experience: None,
            decision_history: crate::decision::DecisionHistory::default(),
            usable_energy: 100.0,
            stress: 0.0,
            maintenance_debt: 0.0,
            stress_threshold: crate::state::INITIAL_STRESS_THRESHOLD,
            stored_material: crate::material_storage::MaterialStorage::default(),
            structure: crate::structure::OrganismStructure::new(),
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
        };
        let material = crate::resources::Material {
            parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
            internal_bonds: vec![crate::resources::InternalBond {
                part_a: 0,
                part_b: 1,
            }],
        };
        let source = crate::physical_material::PhysicalMaterial::realized(
            material,
            vec![
                crate::structure::Placement {
                    x: 0.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
                crate::structure::Placement {
                    x: 0.838,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
            ],
            &catalog,
        )
        .expect("internally bonded physical material should be realizable");
        let make_neighbor = |name: &str, x: f64| {
            crate::physical_material::PhysicalMaterial::realized(
                crate::resources::Material::free_base(name, 1.0),
                vec![crate::structure::Placement {
                    x,
                    y: 0.0,
                    rotation_radians: 0.0,
                }],
                &catalog,
            )
            .expect("single-resource storage neighbor should be realizable")
        };
        let before = make_neighbor("Nitrogen", -2.0);
        let after_one = make_neighbor("Phosphorus", 2.0);
        let after_two = make_neighbor("Sulfur", 4.0);
        // Storage inserts at the front, so reverse insertion gives the
        // intended holder order: before, source, after_one, after_two.
        assert!(organism
            .stored_material
            .store_physical_instance(after_two.clone()));
        assert!(organism
            .stored_material
            .store_physical_instance(after_one.clone()));
        assert!(organism
            .stored_material
            .store_physical_instance(source.clone()));
        assert!(organism
            .stored_material
            .store_physical_instance(before.clone()));
        let entries_before = organism.stored_material.entries.clone();

        let decision = crate::decision_runtime::ActionCandidate {
            action: crate::decision::ActionKind::Break,
            context_key: Some("stored:1:bond:0".into()),
        };
        let mut next_id = 1;
        let mut transformation =
            Simulation::try_start_transformation(&mut organism, &catalog, &mut next_id, &decision)
                .expect("stored internal bond should start BREAK");
        assert_eq!(
            organism.stored_material.entries,
            vec![
                entries_before[0].clone(),
                entries_before[2].clone(),
                entries_before[3].clone(),
            ],
            "reserving the middle source must preserve the remaining entry order"
        );
        transformation.stored_bond = None;
        // Rollback must use the captured source index, not mutable decision metadata.
        transformation.decision_context_key = Some("corrupted-after-reservation".into());

        let mut environment = crate::state::Environment {
            width: 1000.0,
            height: 1000.0,
            catalog,
            field: crate::environment::ActiveMaterialField::new(
                1000.0,
                1000.0,
                crate::environment::DEFAULT_CELL_SIZE,
            ),
        };
        let mut ledger = crate::state::EnergyLedger::default();
        resolve_transformation(
            &transformation,
            &mut organism,
            &mut environment,
            &mut ledger,
        );

        assert_eq!(
            organism.stored_material.entries, entries_before,
            "failed stored BREAK must restore the source at its exact original index"
        );
        assert_eq!(organism.active_transformation_id, None);

        let mut legacy_transaction = transformation.clone();
        legacy_transaction.stored_source_index = None;
        legacy_transaction.decision_context_key = Some("stored:1:bond:0".into());
        assert_eq!(
            original_stored_source_index(&legacy_transaction),
            Some(1),
            "older serialized transactions must retain the decision-key fallback"
        );

        let mut legacy_json = serde_json::to_value(&transformation).unwrap();
        let fields = legacy_json.as_object_mut().unwrap();
        fields.remove("stored_source_index");
        fields.remove("environmental_source_location");
        let decoded: ActiveTransformation = serde_json::from_value(legacy_json).unwrap();
        assert_eq!(decoded.stored_source_index, None);
        assert_eq!(decoded.environmental_source_location, None);
    }

    #[test]
    fn stored_break_rejects_exhausted_transformation_ids_without_reserving_source() {
        use crate::physical_material::PhysicalMaterial;
        use crate::resources::{InternalBond, Material};
        use crate::structure::Placement;

        let catalog = crate::resources::default_catalog();
        let mut organism = environmental_break_test_organism("exhausted-break-id", None);
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
            &catalog,
        )
        .expect("bonded source should be physically realizable");
        assert!(organism.stored_material.store_physical_instance(source));
        let before = organism.stored_material.entries.clone();
        let decision = ActionCandidate {
            action: ActionKind::Break,
            context_key: Some("stored:0:bond:0".into()),
        };
        let mut next_id = u64::MAX;

        let started =
            Simulation::try_start_transformation(&mut organism, &catalog, &mut next_id, &decision);
        assert!(started.is_none());
        assert_eq!(organism.stored_material.entries, before);
        assert_eq!(organism.active_transformation_id, None);
        assert_eq!(next_id, u64::MAX);
    }

    #[test]
    fn environmental_break_rejects_exhausted_transformation_ids_without_removing_source() {
        use crate::physical_material::PhysicalMaterial;
        use crate::resources::{InternalBond, Material};
        use crate::structure::Placement;

        let (mut organism, mut environment, _) = resonance_break_test_organism();
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
            &environment.catalog,
        )
        .expect("bonded environmental source should be physically realizable");
        let cell_index = environment
            .field
            .index_for_position(0.0, 0.0)
            .expect("source origin must be in the field");
        environment.field.cells[cell_index]
            .physical_materials
            .push(source);
        let candidate = environmental_break_candidates(&organism, &environment)
            .into_iter()
            .next()
            .expect("fixture must expose an accessible environmental BREAK candidate");
        assert_eq!(
            parse_environmental_break_context_key(candidate.context_key.as_deref().unwrap()),
            Some((cell_index, 0, 0))
        );
        let materials_before = environment.field.cells[cell_index].physical_materials.clone();
        let mut next_id = u64::MAX;

        let started =
            try_start_environmental_break(&mut organism, &mut environment, &mut next_id, &candidate);
        assert!(started.is_none());
        assert_eq!(
            environment.field.cells[cell_index].physical_materials,
            materials_before
        );
        assert_eq!(organism.active_transformation_id, None);
        assert_eq!(next_id, u64::MAX);
    }

    fn environmental_break_test_organism(
        id: &str,
        active_transformation_id: Option<u64>,
    ) -> Organism {
        Organism {
            id: id.into(),
            developmental_origin: crate::state::Position { x: 0.0, y: 0.0 },
            developmental_orientation_radians: 0.0,
            occupied_cells: Vec::new(),
            genome: crate::genome::initial_genome(),
            harmonic_spectrum: crate::harmonics::ToneSpectrum::empty(),
            resonant_energy: crate::harmonics::ResonantEnergyReservoir::default(),
            experience_memory: crate::memory::ExperienceMemory::default(),
            pending_movement_experience: None,
            decision_history: crate::decision::DecisionHistory::default(),
            usable_energy: 100.0,
            stress: 0.0,
            maintenance_debt: 0.0,
            stress_threshold: crate::state::INITIAL_STRESS_THRESHOLD,
            stored_material: crate::material_storage::MaterialStorage::default(),
            structure: crate::structure::OrganismStructure::new(),
            genome_core_identity_established: false,
            active_transformation_id,
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

    fn environmental_break_test_inventory(
        catalog: &[crate::resources::BaseResource],
    ) -> (
        Environment,
        usize,
        crate::physical_material::PhysicalMaterial,
        crate::physical_material::PhysicalMaterial,
        crate::physical_material::PhysicalMaterial,
    ) {
        use crate::physical_material::PhysicalMaterial;
        use crate::resources::{InternalBond, Material};
        use crate::structure::Placement;

        let mut environment = Environment {
            width: 1000.0,
            height: 1000.0,
            catalog: catalog.to_vec(),
            field: crate::environment::ActiveMaterialField::new(
                1000.0,
                1000.0,
                crate::environment::DEFAULT_CELL_SIZE,
            ),
        };
        let make_single = |name: &str, x: f64| {
            PhysicalMaterial::realized(
                Material::free_base(name, 1.0),
                vec![Placement {
                    x,
                    y: 10.0,
                    rotation_radians: 0.0,
                }],
                catalog,
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
            catalog,
        )
        .expect("bonded environmental source should be realizable");
        let after = make_single("Sulfur", 14.0);
        let cell_index = environment
            .field
            .index_for_position(10.0, 10.0)
            .expect("test source position must be in the field");
        environment.field.cells[cell_index].physical_materials =
            vec![before.clone(), after.clone()];
        (environment, cell_index, before, source, after)
    }

    #[test]
    fn failed_combine_restores_environmental_source_at_original_list_index() {
        let catalog = crate::resources::default_catalog();
        let (mut environment, cell_index, before, source, after) =
            environmental_break_test_inventory(&catalog);
        let transformation = ActiveTransformation {
            id: 733,
            organism_id: "combine-source-rollback-test".into(),
            kind: crate::state::TransformationKind::Combine,
            material: crate::resources::Material::free_base("", 0.0),
            bond: None,
            stored_material: None,
            stored_bond: None,
            environmental_source: false,
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
            combine_environmental_source: Some((cell_index, source.clone())),
            stored_source_index: None,
            environmental_source_location: None,
            combine_environmental_source_index: Some(1),
        };

        assert!(restore_combine_environmental_source(
            &transformation,
            &mut environment
        ));
        assert_eq!(
            environment.field.cells[cell_index].physical_materials,
            vec![before, source, after],
            "failed COMBINE must restore escrow to the exact source list position"
        );

        let mut invalid_index = transformation.clone();
        invalid_index.combine_environmental_source_index = Some(usize::MAX);
        let before_invalid_restore = environment.field.cells[cell_index]
            .physical_materials
            .clone();
        assert!(
            !restore_combine_environmental_source(&invalid_index, &mut environment),
            "corrupt source indices must fail closed instead of silently clamping"
        );
        assert_eq!(
            environment.field.cells[cell_index].physical_materials, before_invalid_restore,
            "a rejected exact restore must not mutate the environmental inventory"
        );

        let mut missing_index = transformation.clone();
        missing_index.combine_environmental_source_index = None;
        assert!(
            !restore_combine_environmental_source(&missing_index, &mut environment),
            "missing source indices must not be mistaken for exact restoration"
        );
        assert_eq!(
            environment.field.cells[cell_index].physical_materials, before_invalid_restore,
            "missing-index recovery must leave the environment unchanged"
        );
    }

    #[test]
    fn failed_environmental_break_context_selects_exact_material_and_bond_indices() {
        let catalog = crate::resources::default_catalog();
        let (mut environment, cell_index, before, source, after) =
            environmental_break_test_inventory(&catalog);
        environment.field.cells[cell_index].physical_materials =
            vec![before.clone(), source.clone(), after.clone()];
        let decision_key = format!("environment:cell:{cell_index}:material:1:bond:0");
        let (parsed_cell, parsed_material, parsed_bond) =
            parse_environmental_break_context_key(&decision_key)
                .expect("valid decision context should parse");
        let (selected, selected_bond) =
            environmental_break_source_at(&environment, parsed_cell, parsed_material, parsed_bond)
                .expect("context key should resolve the indexed environmental source");
        assert_eq!(selected, source);
        assert_eq!(
            selected_bond,
            selected.internal_connections.as_ref().unwrap()[0]
        );
        assert_eq!(
            environment.field.cells[cell_index].physical_materials,
            vec![before, source, after],
            "resolving a decision context must not mutate the environmental inventory"
        );
    }

    #[test]
    fn failed_environmental_break_split_destination_restores_original_source_atomically() {
        let catalog = crate::resources::default_catalog();
        let mut organism =
            environmental_break_test_organism("environmental-break-split-test", Some(732));
        let (mut environment, cell_index, before, mut source, after) =
            environmental_break_test_inventory(&catalog);
        // Preserve a valid original bond record but make one split piece's
        // destination invalid, so the first staged deposit must also roll back.
        source.placements.as_mut().unwrap()[1].x = environment.width + 100.0;
        let original_source = source.clone();
        let target = source.internal_connections.as_ref().unwrap()[0].clone();
        environment.field.cells[cell_index].physical_materials =
            vec![before.clone(), after.clone()];
        let transformation = ActiveTransformation {
            id: 732,
            organism_id: organism.id.clone(),
            kind: crate::state::TransformationKind::Break,
            material: crate::resources::Material::free_base("", 0.0),
            bond: None,
            stored_material: Some(source),
            stored_bond: Some(target),
            environmental_source: true,
            complexity: 1.0,
            duration_ticks: 1,
            remaining_ticks: 0,
            prepared_energy: Some((organism.usable_energy, 0.0, 0.0)),
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
            stored_source_index: None,
            environmental_source_location: None,
            combine_environmental_source_index: None,
        };

        resolve_transformation(
            &transformation,
            &mut organism,
            &mut environment,
            &mut EnergyLedger::default(),
        );

        assert_eq!(
            environment.field.cells[cell_index].physical_materials,
            vec![before, original_source, after],
            "a rejected split-piece destination must restore the unsplit source at its original index"
        );
        assert!(organism.stored_material.is_empty());
        assert_eq!(organism.active_transformation_id, None);
    }

    #[test]
    fn failed_environmental_break_completion_restores_source_at_original_cell_index() {
        let catalog = crate::resources::default_catalog();
        let mut organism =
            environmental_break_test_organism("environmental-break-completion-test", Some(731));
        let (mut environment, cell_index, before, source, after) =
            environmental_break_test_inventory(&catalog);
        let transformation = ActiveTransformation {
            id: 731,
            organism_id: organism.id.clone(),
            kind: crate::state::TransformationKind::Break,
            material: crate::resources::Material::free_base("", 0.0),
            bond: None,
            stored_material: Some(source.clone()),
            stored_bond: None,
            environmental_source: true,
            complexity: 1.0,
            duration_ticks: 1,
            remaining_ticks: 0,
            prepared_energy: None,
            pending_experience: None,
            decision_context_key: Some("environment:cell:999:material:999:bond:0".into()),
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

        resolve_transformation(
            &transformation,
            &mut organism,
            &mut environment,
            &mut EnergyLedger::default(),
        );

        assert_eq!(
            environment.field.cells[cell_index].physical_materials,
            vec![before, source, after],
            "failed environmental BREAK must restore its source at its original cell index"
        );
        assert!(organism.stored_material.is_empty());
        assert_eq!(organism.active_transformation_id, None);

        let mut invalid_location = transformation.clone();
        invalid_location.environmental_source_location = None;
        invalid_location.decision_context_key =
            Some(format!("environment:cell:{cell_index}:material:99:bond:0"));
        let before_invalid_restore = environment.field.cells[cell_index]
            .physical_materials
            .clone();
        assert!(
            !restore_environmental_break_source(&invalid_location, &mut environment),
            "corrupt environmental BREAK indices must fail closed"
        );
        assert_eq!(
            environment.field.cells[cell_index].physical_materials, before_invalid_restore,
            "a rejected exact restore must not alter the environmental inventory"
        );
    }

    #[test]
    fn stored_material_break_does_not_credit_unallocated_bond_energy() {
        // Keep this accounting regression independent of the calibrated
        // developmental constructor; it needs a holder and storage, not a genome.
        let catalog = crate::resources::default_catalog();
        let mut organism = crate::state::Organism {
            id: "stored-break-accounting-test".into(),
            developmental_origin: crate::state::Position { x: 0.0, y: 0.0 },
            developmental_orientation_radians: 0.0,
            occupied_cells: Vec::new(),
            genome: crate::genome::initial_genome(),
            harmonic_spectrum: crate::harmonics::ToneSpectrum::empty(),
            resonant_energy: crate::harmonics::ResonantEnergyReservoir::default(),
            experience_memory: crate::memory::ExperienceMemory::default(),
            pending_movement_experience: None,
            decision_history: crate::decision::DecisionHistory::default(),
            usable_energy: 100.0,
            stress: 0.0,
            maintenance_debt: 0.0,
            stress_threshold: crate::state::INITIAL_STRESS_THRESHOLD,
            stored_material: crate::material_storage::MaterialStorage::default(),
            structure: crate::structure::OrganismStructure::new(),
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
        };
        let mut environment = crate::state::Environment {
            width: 1000.0,
            height: 1000.0,
            catalog: catalog.clone(),
            field: crate::environment::ActiveMaterialField::new(
                1000.0,
                1000.0,
                crate::environment::DEFAULT_CELL_SIZE,
            ),
        };
        let material = crate::resources::Material {
            parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
            internal_bonds: vec![crate::resources::InternalBond {
                part_a: 0,
                part_b: 1,
            }],
        };
        let physical = crate::physical_material::PhysicalMaterial::realized(
            material,
            vec![
                crate::structure::Placement {
                    x: 0.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
                crate::structure::Placement {
                    x: 0.838,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
            ],
            &catalog,
        )
        .expect("internally bonded physical material should be realizable");
        assert!(organism.stored_material.store_physical_instance(physical));

        let decision = crate::decision_runtime::ActionCandidate {
            action: crate::decision::ActionKind::Break,
            context_key: Some("stored:0:bond:0".into()),
        };
        let mut next_id = 1;
        let mut transformation =
            Simulation::try_start_transformation(&mut organism, &catalog, &mut next_id, &decision)
                .expect("stored internal bond should be a BREAK candidate");
        let before_energy = organism.usable_energy;
        let mut ledger = crate::state::EnergyLedger::default();
        let before_ledger = ledger;

        assert!(Simulation::prepare_transformation(
            &mut transformation,
            &mut organism,
            &mut environment,
            &mut ledger,
            None,
        ));
        assert_eq!(organism.usable_energy, before_energy);
        assert_eq!(
            transformation.prepared_energy,
            Some((before_energy, 0.0, 0.0))
        );
        assert_eq!(
            ledger.total_potential_energy_released,
            before_ledger.total_potential_energy_released
        );
        assert_eq!(
            ledger.total_external_energy_input,
            before_ledger.total_external_energy_input
        );
        assert_eq!(
            ledger.total_work_expended,
            before_ledger.total_work_expended
        );
        assert_eq!(
            ledger.total_heat_dissipated,
            before_ledger.total_heat_dissipated
        );
        assert_eq!(
            ledger.total_structural_energy_delta,
            before_ledger.total_structural_energy_delta
        );
    }

    #[test]
    fn chemical_break_energy_comes_from_the_removed_bond_not_the_reaction_signal() {
        let (structural_energy, usable, heat) =
            chemical_break_energy_yield(10.0, 4.0, 0.5).unwrap();
        assert!((structural_energy - 10.0).abs() < 1e-12);
        assert!((usable - 3.0).abs() < 1e-12);
        assert!((heat - 3.0).abs() < 1e-12);
        assert!((usable - structural_energy + 4.0 + heat).abs() < 1e-12);
    }

    #[test]
    fn chemical_break_charges_a_work_shortfall_to_the_usable_holder() {
        let (structural_energy, usable, heat) = chemical_break_energy_yield(4.0, 5.0, 0.5).unwrap();
        assert!((structural_energy - 4.0).abs() < 1e-12);
        assert!((usable + 1.0).abs() < 1e-12);
        assert_eq!(heat, 0.0);
        assert!((usable - structural_energy + 5.0 + heat).abs() < 1e-12);
    }

    #[test]
    fn compatible_resonance_reduces_bond_supplied_work_without_energy_creation() {
        let bond_energy = 4.0;
        let total_disruption_work = 5.0;
        let resonant_work = 2.0;
        let bond_supplied_work = total_disruption_work - resonant_work;
        let (structural_energy, usable, heat) =
            chemical_break_energy_yield(bond_energy, bond_supplied_work, 0.5).unwrap();

        assert!((structural_energy - bond_energy).abs() < 1e-12);
        assert!((usable - 0.5).abs() < 1e-12);
        assert!((heat - 0.5).abs() < 1e-12);
        // Bond energy plus the reservoir debit exactly pays usable gain,
        // dissipated heat, and the full physical disruption work.
        let residual = -structural_energy - resonant_work + usable + heat + total_disruption_work;
        assert!(residual.abs() < 1e-12);
    }

    fn resonance_break_test_organism() -> (
        crate::state::Organism,
        crate::state::Environment,
        Vec<usize>,
    ) {
        let genome = crate::genome::initial_genome();
        let catalog = crate::resources::default_catalog();
        let blueprint = crate::juvenile::confirmed_seed_baseline(&catalog).unwrap();
        let (structure, _, _) = crate::juvenile::realize_initial(&blueprint, &catalog).unwrap();
        let organism = crate::state::Organism {
            id: "test".into(),
            developmental_origin: crate::state::Position { x: 0.0, y: 0.0 },
            developmental_orientation_radians: 0.0,
            occupied_cells: vec![crate::state::Position { x: 0.0, y: 0.0 }],
            genome,
            harmonic_spectrum: crate::harmonics::ToneSpectrum::empty(),
            resonant_energy: crate::harmonics::ResonantEnergyReservoir::default(),
            experience_memory: crate::memory::ExperienceMemory::default(),
            pending_movement_experience: None,
            decision_history: crate::decision::DecisionHistory::default(),
            usable_energy: 1_000_000.0,
            stress: 0.0,
            maintenance_debt: 0.0,
            stress_threshold: crate::state::INITIAL_STRESS_THRESHOLD,
            stored_material: crate::material_storage::MaterialStorage::default(),
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
        };
        let environment = crate::state::Environment {
            width: 1000.0,
            height: 1000.0,
            catalog: catalog.clone(),
            field: crate::environment::ActiveMaterialField::new(
                1000.0,
                1000.0,
                crate::environment::DEFAULT_CELL_SIZE,
            ),
        };
        let genome_bonds = crate::cavity::analyze_genome_cavity(&organism.structure, &catalog)
            .unwrap()
            .unwrap()
            .boundary_bond_indices(&organism.structure, &catalog);
        (organism, environment, genome_bonds)
    }

    #[test]
    fn resonance_overload_breaks_a_compatible_bond_only_above_structural_capacity() {
        let mut simulation = crate::state::Simulation::new(918, 10.0);
        let mut organism = simulation.organisms.remove(0);
        let environment = simulation.environment.clone();
        let capacity: f64 = organism
            .structure
            .bonds
            .iter()
            .map(|bond| bond.bond_energy)
            .sum();
        assert!(capacity.is_finite() && capacity > 0.0);
        assert!(organism.resonant_energy.deposit(440.0, capacity * 0.5));
        let original_bond_count = organism.structure.bonds.len();
        let mut ledger = crate::state::EnergyLedger::default();
        assert!(!resolve_resonance_overload_break(
            &mut organism,
            &environment,
            &mut ledger,
        ));
        assert_eq!(organism.structure.bonds.len(), original_bond_count);

        organism.resonant_energy = crate::harmonics::ResonantEnergyReservoir::default();
        assert!(organism.resonant_energy.deposit(440.0, capacity + 1.0));
        let before_resonance = organism.resonant_energy.total_energy().unwrap();
        assert!(resolve_resonance_overload_break(
            &mut organism,
            &environment,
            &mut ledger,
        ));
        assert_eq!(organism.structure.bonds.len(), original_bond_count - 1);
        assert!(organism.resonant_energy.total_energy().unwrap() < before_resonance);
    }

    #[test]
    fn resonance_overload_break_reconciles_world_inventory() {
        let mut simulation = crate::state::Simulation::new(926, 10.0);
        let capacity: f64 = simulation.organisms[0]
            .structure
            .bonds
            .iter()
            .map(|bond| bond.bond_energy)
            .sum();
        assert!(capacity.is_finite() && capacity > 0.0);
        assert!(simulation.organisms[0]
            .resonant_energy
            .deposit(440.0, capacity + 1.0));
        let seeded_resonance = simulation.organisms[0]
            .resonant_energy
            .total_energy()
            .expect("seeded resonance should have finite energy");
        // This reservoir is an explicit test starting condition, not new
        // world-tone input. Use it as the independent inventory baseline.
        simulation.energy_ledger.initial_resonant_energy_held = seeded_resonance;
        simulation.energy_ledger.total_resonant_energy_held = seeded_resonance;
        let before = simulation.energy_inventory_snapshot();
        assert!(
            before.is_balanced(1e-9),
            "initial overload inventory: {:?}",
            before.residual()
        );

        assert!(resolve_resonance_overload_break(
            &mut simulation.organisms[0],
            &simulation.environment,
            &mut simulation.energy_ledger,
        ));
        let after = simulation.energy_inventory_snapshot();
        assert!(
            after.is_balanced(1e-9),
            "resonance-overload BREAK must reconcile the world inventory: residual={:?}",
            after.residual()
        );
        assert!(after.current_resonant_energy < before.current_resonant_energy);
        assert!(after.current_structural_energy < before.current_structural_energy);
    }

    #[test]
    fn resonance_overload_without_frequency_compatible_energy_preserves_structure() {
        let mut simulation = crate::state::Simulation::new(918, 10.0);
        let mut organism = simulation.organisms.remove(0);
        let environment = simulation.environment.clone();
        let capacity: f64 = organism
            .structure
            .bonds
            .iter()
            .map(|bond| bond.bond_energy)
            .sum();
        assert!(capacity.is_finite() && capacity > 0.0);
        assert!(organism.resonant_energy.deposit(1.0e300, capacity + 1.0));
        let original_bonds = organism.structure.bonds.clone();
        let original_energy = organism.resonant_energy.total_energy().unwrap();
        let mut ledger = crate::state::EnergyLedger::default();

        assert!(!resolve_resonance_overload_break(
            &mut organism,
            &environment,
            &mut ledger,
        ));
        assert_eq!(organism.structure.bonds.len(), original_bonds.len());
        assert_eq!(
            organism.resonant_energy.total_energy().unwrap(),
            original_energy
        );
    }

    #[test]
    fn resonance_break_candidates_prefer_non_genome_bonds() {
        let (organism, environment, genome_bonds) = resonance_break_test_organism();
        let candidates = resonance_break_candidate_indices(&organism, &environment);
        assert!(!candidates.is_empty());
        assert!(candidates.iter().all(|index| !genome_bonds.contains(index)));
    }

    #[test]
    fn resonance_break_candidates_fall_back_to_genome_bonds_when_structure_is_exhausted() {
        let (mut organism, environment, genome_bonds) = resonance_break_test_organism();
        assert!(!genome_bonds.is_empty());
        let protected_bonds: Vec<_> = genome_bonds
            .iter()
            .filter_map(|&index| organism.structure.bonds.get(index).copied())
            .collect();
        organism.structure.bonds.retain(|bond| {
            protected_bonds
                .iter()
                .any(|candidate| candidate.has_same_identity(bond))
        });
        let candidates = resonance_break_candidate_indices(&organism, &environment);
        assert_eq!(candidates.len(), organism.structure.bonds.len());
        assert!(!candidates.is_empty());
    }
}
