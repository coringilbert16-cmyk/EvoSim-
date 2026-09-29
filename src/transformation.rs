#![expect(
    dead_code,
    reason = "Staged transformation API retained for subsystem integration"
)]

use crate::decision::ActionKind;
use crate::decision_runtime::ActionCandidate;
use crate::energy_ledger::{EnergyLedgerAuthority, EnergyReason, EnergyTransaction};
use crate::state::{ActiveTransformation, EnergyLedger, Environment, Organism, Simulation};
use rand::Rng;
use rand_chacha::ChaCha8Rng;

pub(crate) fn break_energy_yield(
    a: crate::resources::ResourceProperties,
    b: crate::resources::ResourceProperties,
    processing_efficiency: f64,
) -> Option<(f64, f64, f64)> {
    let gross = a.potential_energy + b.potential_energy;
    if !gross.is_finite() || gross < 0.0 {
        return None;
    }
    let reactivity = (crate::math::exponential_influence(a.reactivity.max(0.0))
        + crate::math::exponential_influence(b.reactivity.max(0.0)))
        * 0.5;
    let cohesion =
        ((a.cohesion.clamp(0.0, 1.0) + b.cohesion.clamp(0.0, 1.0)) * 0.5).clamp(0.0, 1.0);
    let accessible = gross * reactivity;
    let cohesion_loss = accessible * cohesion * 0.5;
    let pre_processing = (accessible - cohesion_loss).max(0.0);
    let efficiency = processing_efficiency.clamp(0.0, 1.0);
    let usable = pre_processing * efficiency;
    let heat = (gross - usable).max(0.0);
    (usable.is_finite() && heat.is_finite()).then_some((gross, usable, heat))
}

fn settle_break_energy(
    organism: &mut Organism,
    bond: crate::structure::Bond,
    usable: f64,
    gross: f64,
    heat: f64,
    ledger: &mut EnergyLedger,
) -> bool {
    let mut trial_structure = organism.structure.clone();
    if trial_structure.break_matching_bond(bond).is_none() {
        return false;
    }
    let tx = EnergyTransaction {
        reason: EnergyReason::Break,
        potential_released: gross,
        usable_delta: usable,
        structural_delta: 0.0,
        heat_dissipated: heat,
    };
    if !ledger.settle_transaction(&mut organism.usable_energy, tx) {
        return false;
    }
    organism.structure = trial_structure;
    organism.mark_structure_changed();
    organism.add_transaction_stress(heat);
    true
}

fn stress_break_candidate_indices(organism: &Organism, environment: &Environment) -> Vec<usize> {
    let genome_bonds =
        crate::cavity::analyze_genome_cavity(&organism.structure, &environment.catalog)
            .ok()
            .flatten()
            .map(|cavity| cavity.boundary_bond_indices(&organism.structure))
            .unwrap_or_default();

    // Genome bonds are protected while ordinary structural bonds remain.
    // Once those ordinary bonds are exhausted, the protected bonds become
    // stress-break candidates so structural collapse can proceed normally.
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

pub(crate) fn resolve_stress_break(
    organism: &mut Organism,
    environment: &Environment,
    ledger: &mut EnergyLedger,
    rng: &mut ChaCha8Rng,
) -> bool {
    let candidate_indices = stress_break_candidate_indices(organism, environment);
    if candidate_indices.is_empty() {
        return false;
    }
    let target_index = candidate_indices[rng.gen_range(0..candidate_indices.len())];
    let target = organism.structure.bonds[target_index];
    let Some(ia) = organism
        .structure
        .unit_index(target.endpoint_a.constituent_id)
    else {
        return false;
    };
    let Some(ib) = organism
        .structure
        .unit_index(target.endpoint_b.constituent_id)
    else {
        return false;
    };
    let Some(a) = organism
        .structure
        .units
        .get(ia)
        .and_then(|u| u.properties(&environment.catalog))
    else {
        return false;
    };
    let Some(b) = organism
        .structure
        .units
        .get(ib)
        .and_then(|u| u.properties(&environment.catalog))
    else {
        return false;
    };
    let Some((gross, usable, heat)) =
        break_energy_yield(a, b, organism.genome.processing_efficiency())
    else {
        return false;
    };
    settle_break_energy(organism, target, usable, gross, heat, ledger)
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

        let removed = organism.stored_material.entries.swap_remove(storage_index);
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
        };
        *next_id += 1;
        organism.active_transformation_id = Some(t.id);
        Some(t)
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
        for instance in &cell.physical_materials {
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
                let Some(a) = connection.endpoint_a.world_point(
                    &hypothetical.units[a_index],
                    &environment.catalog,
                ) else {
                    continue;
                };
                let Some(b) = connection.endpoint_b.world_point(
                    &hypothetical.units[b_index],
                    &environment.catalog,
                ) else {
                    continue;
                };
                if regions.iter().any(|region| {
                    region.contains_point(a.x, a.y) || region.contains_point(b.x, b.y)
                }) {
                    candidates.push(ActionCandidate {
                        action: ActionKind::Break,
                        context_key: Some(format!(
                            "environment:cell:{cell_index}:bond:{bond_index}"
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
    let Some(body) = crate::organism_geometry::OrganismBodyGeometry::from_structure(
        &organism.structure,
        &environment.catalog,
    ) else {
        return false;
    };
    environment
        .field
        .cells_intersecting_bounds(body.min_x, body.max_x, body.min_y, body.max_y)
        .into_iter()
        .any(|cell_index| {
            environment
                .field
                .cells
                .get(cell_index)
                .into_iter()
                .flat_map(|cell| cell.physical_materials.iter())
                .any(|instance| {
                    instance
                        .internal_connections
                        .as_ref()
                        .is_some_and(|connections| !connections.is_empty())
                })
        })
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
        let key = decision.context_key.as_deref()?;
        let rest = key.strip_prefix("environment:")?;
        let (cell_part, bond_part) = rest.rsplit_once(":bond:")?;
        let cell_index = cell_part.strip_prefix("cell:")?.parse::<usize>().ok()?;
        let bond_index = bond_part.parse::<usize>().ok()?;

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
        let candidates = environment
            .field
            .cells_intersecting_bounds(body.min_x, body.max_x, body.min_y, body.max_y);
        for candidate_cell in candidates {
            if candidate_cell != cell_index {
                continue;
            }
            let cell = environment.field.cells.get(cell_index)?;
            for (material_index, instance) in cell.physical_materials.iter().enumerate() {
                let Some(connections) = instance.internal_connections.as_ref() else {
                    continue;
                };
                let Some(target) = connections.get(bond_index).cloned() else {
                    continue;
                };
                if instance.placements.is_none() {
                    continue;
                }
                let mut hypothetical = organism.structure.clone();
                let indices = crate::material_restoration::restore_material(
                    &mut hypothetical,
                    instance,
                    Placement {
                        x: 0.0,
                        y: 0.0,
                        rotation_radians: 0.0,
                    },
                    &environment.catalog,
                )?;
                let Some(&a_index) = indices.get(target.part_a) else {
                    continue;
                };
                let Some(&b_index) = indices.get(target.part_b) else {
                    continue;
                };
                let endpoint_a = target.endpoint_a;
                let endpoint_b = target.endpoint_b;
                let point_a = endpoint_a.world_point(
                    &hypothetical.units[a_index],
                    &environment.catalog,
                )?;
                let point_b = endpoint_b.world_point(
                    &hypothetical.units[b_index],
                    &environment.catalog,
                )?;
                let accessible = regions.iter().any(|region| {
                    region.contains_point(point_a.x, point_a.y)
                        || region.contains_point(point_b.x, point_b.y)
                });
                if !accessible {
                    continue;
                }
                let removed = environment
                    .field
                    .cells
                    .get_mut(cell_index)?
                    .physical_materials
                    .get(material_index)?
                    .clone();
                if removed != *instance {
                    continue;
                }
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
                    combine_environmental_source: None,
                };
                *next_id += 1;
                organism.active_transformation_id = Some(t.id);
                return Some(t);
            }
        }
        None
    }

    pub(crate) fn prepare_transformation(
        transformation: &mut ActiveTransformation,
        organism: &mut Organism,
        environment: &mut Environment,
        ledger: &mut EnergyLedger,
    ) -> bool {
        if matches!(transformation.kind, crate::state::TransformationKind::Combine) {
            let Some(prepared_energy) = transformation.prepared_usable_energy else {
                organism.active_transformation_id = None;
                return false;
            };
            let Some(prepared_ledger) = transformation.prepared_ledger else {
                organism.active_transformation_id = None;
                return false;
            };
            if !prepared_energy.is_finite() || prepared_energy < 0.0 {
                organism.active_transformation_id = None;
                return false;
            }
            // Tick 2: settle the exact transaction selected on tick 1. The
            // structure and storage remain untouched until tick 3.
            organism.usable_energy = prepared_energy;
            *ledger = prepared_ledger;
            transformation.prepared_energy = Some((0.0, 0.0, 0.0));
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
        let Some(a) = stored
            .material
            .parts
            .get(target.part_a)
            .and_then(|(name, _)| {
                environment
                    .catalog
                    .iter()
                    .find(|resource| resource.name == *name)
                    .map(|r| r.properties)
            })
        else {
            organism.active_transformation_id = None;
            return false;
        };
        let Some(b) = stored
            .material
            .parts
            .get(target.part_b)
            .and_then(|(name, _)| {
                environment
                    .catalog
                    .iter()
                    .find(|resource| resource.name == *name)
                    .map(|r| r.properties)
            })
        else {
            organism.active_transformation_id = None;
            return false;
        };
        let Some((gross, usable, heat)) =
            break_energy_yield(a, b, organism.genome.processing_efficiency())
        else {
            organism.active_transformation_id = None;
            return false;
        };

        // Validate the exact bond before settling the transaction. The physical
        // structure itself is not changed until the following tick.
        if stored.break_internal_bond(target).is_none() {
            organism.active_transformation_id = None;
            return false;
        }

        let tx = EnergyTransaction {
            reason: EnergyReason::Break,
            potential_released: gross,
            usable_delta: usable,
            structural_delta: 0.0,
            heat_dissipated: heat,
        };
        if !ledger.settle_transaction(&mut organism.usable_energy, tx) {
            organism.active_transformation_id = None;
            return false;
        }

        transformation.prepared_energy = Some((gross, usable, heat));
        true
    }

    fn pending_energy_delta(
    transformation: &ActiveTransformation,
    organism: &Organism,
) -> f64 {
    transformation
        .prepared_usable_energy
        .map(|prepared| organism.usable_energy - prepared)
        .unwrap_or(0.0)
}

fn pending_stress_delta(
    transformation: &ActiveTransformation,
    organism: &Organism,
) -> f64 {
    transformation
        .prepared_stress
        .map(|prepared| organism.stress - prepared)
        .unwrap_or(0.0)
}

pub(crate) fn resolve_transformation(
        transformation: &ActiveTransformation,
        organism: &mut Organism,
        environment: &mut Environment,
        _ledger: &mut EnergyLedger,
    ) {
        if matches!(transformation.kind, crate::state::TransformationKind::Combine) {
            let (Some(structure), Some(stored_material), Some(stress)) = (
                transformation.prepared_structure.as_ref(),
                transformation.prepared_stored_material.as_ref(),
                transformation.prepared_stress,
            ) else {
                if let Some((cell_index, source)) =
                    transformation.combine_environmental_source.as_ref()
                {
                    let _ = environment
                        .field
                        .deposit(
                            source.placements
                                .as_ref()
                                .and_then(|placements| placements.first())
                                .map(|placement| placement.x)
                                .unwrap_or(0.0),
                            source.placements
                                .as_ref()
                                .and_then(|placements| placements.first())
                                .map(|placement| placement.y)
                                .unwrap_or(0.0),
                            source.clone(),
                        );
                    let _ = cell_index;
                }
                organism.active_transformation_id = None;
                return;
            };

            // Tick 3: commit the physical mutation selected on tick 1 and
            // prepared on tick 2.
            organism.structure = structure.clone();
            organism.stored_material = stored_material.clone();
            organism.stress = stress;
            organism.mark_structure_changed();
            organism.active_transformation_id = None;

            crate::decision_runtime::record_consequence(
                &mut organism.decision_history,
                &ActionCandidate {
                    action: ActionKind::Combine,
                    context_key: transformation.decision_context_key.clone(),
                },
                crate::decision::ActionConsequence {
                    energy_delta: pending_energy_delta(transformation, organism),
                    stress_delta: pending_stress_delta(transformation, organism),
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
            organism.active_transformation_id = None;
            return;
        };
        if transformation.prepared_energy.is_none() {
            organism.active_transformation_id = None;
            return;
        }
        let Some(pieces) = stored.break_internal_bond(target) else {
            organism.active_transformation_id = None;
            return;
        };

        for piece in pieces {
            if transformation.environmental_source {
                if let Some(placement) = piece
                    .placements
                    .as_ref()
                    .and_then(|placements| placements.first())
                {
                    let _ = environment.field.deposit(placement.x, placement.y, piece);
                }
            } else if !organism
                .stored_material
                .store_physical_instance(piece.clone())
            {
                if let Some(placement) = piece
                    .placements
                    .as_ref()
                    .and_then(|placements| placements.first())
                {
                    let _ = environment.field.deposit(placement.x, placement.y, piece);
                }
            }
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
}

pub(crate) fn break_work_cost(
    a: crate::resources::ResourceProperties,
    b: crate::resources::ResourceProperties,
    complexity: f64,
) -> f64 {
    crate::combine::bond_strength(a, b) * complexity.max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn break_energy_yield_is_positive_when_resources_have_reactive_potential() {
        let carbon = crate::resources::ResourceProperties {
            mass: 1.0,
            potential_energy: 1.0,
            reactivity: 1.0,
            cohesion: 0.5,
        };
        let methane = crate::resources::ResourceProperties {
            potential_energy: 20.0,
            reactivity: 4.0,
            cohesion: 0.1,
            ..carbon
        };
        let (gross, usable, heat) = break_energy_yield(carbon, methane, 0.8).unwrap();
        assert_eq!(gross, 21.0);
        assert!(usable > 0.0);
        assert!(heat >= 0.0);
        assert!((gross - usable - heat).abs() < 1e-12);
    }

    fn stress_break_test_organism() -> (
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
            experience_memory: crate::memory::ExperienceMemory::default(),
            pending_movement_experience: None,
            decision_history: crate::decision::DecisionHistory::default(),
            usable_energy: 1_000_000.0,
            stress: 0.0,
            maintenance_debt: 0.0,
            stress_threshold: crate::state::INITIAL_STRESS_THRESHOLD,
            stored_material: crate::material_storage::MaterialStorage::default(),
            structure,
            development_stage: crate::state::DevelopmentStage::Juvenile,
            active_transformation_id: None,
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
            .boundary_bond_indices(&organism.structure);
        (organism, environment, genome_bonds)
    }

    #[test]
    fn stress_break_candidates_prefer_non_genome_bonds() {
        let (organism, environment, genome_bonds) = stress_break_test_organism();
        let candidates = stress_break_candidate_indices(&organism, &environment);
        assert!(!candidates.is_empty());
        assert!(candidates.iter().all(|index| !genome_bonds.contains(index)));
    }

    #[test]
    fn stress_break_candidates_fall_back_to_genome_bonds_when_structure_is_exhausted() {
        let (mut organism, environment, genome_bonds) = stress_break_test_organism();
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
        let candidates = stress_break_candidate_indices(&organism, &environment);
        assert_eq!(candidates.len(), organism.structure.bonds.len());
        assert!(!candidates.is_empty());
    }
}
