use crate::decision::{ActionKind, OutcomeKind};
use crate::decision_runtime::ActionCandidate;
use crate::energy_ledger::{EnergyLedgerAuthority, EnergyReason, EnergyTransaction};
use crate::state::{ActiveTransformation, EnergyLedger, Environment, Organism, Simulation};
use rand::Rng;
use rand_chacha::ChaCha8Rng;

fn water_field_amount(environment: &Environment, organism: &Organism) -> f64 {
    organism
        .occupied_cells
        .first()
        .and_then(|p| environment.field.index_for_position(p.x, p.y))
        .map(|i| {
            environment.field.cells[i]
                .materials
                .iter()
                .flat_map(|m| m.parts.iter())
                .filter(|(n, _)| n == "Water")
                .map(|(_, a)| *a)
                .sum()
        })
        .unwrap_or(0.0)
}

pub(crate) fn break_energy_yield(
    a: crate::resources::ResourceProperties,
    b: crate::resources::ResourceProperties,
    water_field: f64,
    processing_efficiency: f64,
) -> Option<(f64, f64, f64)> {
    let gross = a.potential_energy + b.potential_energy;
    if !gross.is_finite() || gross < 0.0 {
        return None;
    }
    let reactivity = (crate::math::exponential_influence(crate::resources::effective_reactivity(
        a.reactivity.max(0.0),
        water_field,
    )) + crate::math::exponential_influence(
        crate::resources::effective_reactivity(b.reactivity.max(0.0), water_field),
    )) * 0.5;
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
    let candidates: Vec<usize> = organism
        .structure
        .bonds
        .iter()
        .enumerate()
        .filter_map(|(index, _)| (!genome_bonds.contains(&index)).then_some(index))
        .collect();
    candidates
}

pub(crate) fn resolve_stress_break(
    organism: &mut Organism,
    environment: &Environment,
    ledger: &mut EnergyLedger,
    rng: &mut ChaCha8Rng,
) -> bool {
    let candidate_indices = stress_break_candidate_indices(organism, environment);
    let Some(&target_index) = candidate_indices.get(rng.gen_range(0..candidate_indices.len()))
    else {
        return false;
    };
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
    let Some((gross, usable, heat)) = break_energy_yield(
        a,
        b,
        water_field_amount(environment, organism),
        organism.genome.processing_efficiency(),
    ) else {
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
        let mut stored_material = None;
        let mut stored_bond = None;
        let mut environmental_material_id = None;
        let mut environmental_bond_index = None;

        if let Some(rest) = key.strip_prefix("stored:") {
            let (storage_index, bond_part) = rest.split_once(":bond:")?;
            let storage_index = storage_index.parse::<usize>().ok()?;
            let bond_index = bond_part.parse::<usize>().ok()?;
            let entry = organism.stored_material.entries.get(storage_index)?;
            let physical = match entry {
                crate::material_storage::StoredMaterial::Physical(instance)
                    if instance.is_realized() =>
                {
                    instance.clone()
                }
                _ => return None,
            };
            stored_bond = Some(
                physical
                    .internal_connections
                    .as_ref()?
                    .get(bond_index)?
                    .clone(),
            );
            let removed = organism.stored_material.entries.swap_remove(storage_index);
            stored_material = match removed {
                crate::material_storage::StoredMaterial::Physical(instance) => Some(instance),
            };
        } else if let Some(rest) = key.strip_prefix("environment:") {
            let (material_part, bond_part) = rest.split_once(":bond:")?;
            environmental_material_id = Some(material_part.parse::<u64>().ok()?);
            environmental_bond_index = Some(bond_part.parse::<usize>().ok()?);
        } else {
            return None;
        }

        let complexity = crate::math::complexity(2.0);
        let duration = 1_u64.max(complexity.ceil() as u64);
        let t = ActiveTransformation {
            id: *next_id,
            organism_id: organism.id.clone(),
            kind: crate::state::TransformationKind::Break,
            bond: None,
            stored_material,
            stored_bond,
            environmental_material_id,
            environmental_bond_index,
            complexity,
            duration_ticks: duration,
            remaining_ticks: duration,
            decision_context_key: decision.context_key.clone(),
        };
        *next_id += 1;
        organism.active_transformation_id = Some(t.id);
        Some(t)
    }

    pub(crate) fn resolve_transformation(
        transformation: &ActiveTransformation,
        organism: &mut Organism,
        environment: &mut Environment,
        ledger: &mut EnergyLedger,
    ) {
        let (stored, target, environmental_id) =
            if let Some(stored) = transformation.stored_material.as_ref() {
                let Some(target) = transformation.stored_bond.as_ref() else {
                    organism.active_transformation_id = None;
                    return;
                };
                (stored.clone(), target.clone(), None)
            } else if let (Some(material_id), Some(bond_index)) = (
                transformation.environmental_material_id,
                transformation.environmental_bond_index,
            ) {
                let Some((cell_index, material_index)) =
                    environment.field.find_physical_material(material_id)
                else {
                    organism.active_transformation_id = None;
                    return;
                };
                let Some(physical) = environment
                    .field
                    .cells
                    .get(cell_index)
                    .and_then(|cell| cell.physical_materials.get(material_index))
                    .filter(|physical| physical.is_realized())
                    .cloned()
                else {
                    organism.active_transformation_id = None;
                    return;
                };
                let Some(target) = physical
                    .internal_connections
                    .as_ref()
                    .and_then(|connections| connections.get(bond_index))
                    .cloned()
                else {
                    organism.active_transformation_id = None;
                    return;
                };
                (physical, target, Some(material_id))
            } else {
                organism.active_transformation_id = None;
                return;
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
                    .map(|resource| resource.properties)
            })
        else {
            organism.active_transformation_id = None;
            return;
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
                    .map(|resource| resource.properties)
            })
        else {
            organism.active_transformation_id = None;
            return;
        };
        let Some((gross, usable, heat)) = break_energy_yield(
            a,
            b,
            water_field_amount(environment, organism),
            organism.genome.processing_efficiency(),
        ) else {
            organism.active_transformation_id = None;
            return;
        };

        if let Some(material_id) = environmental_id {
            let Some(removed) = environment.field.remove_physical_material(material_id) else {
                organism.active_transformation_id = None;
                return;
            };
            let Some(pieces) = removed.break_internal_bond(&target) else {
                let _ = environment.field.deposit_physical(
                    removed
                        .placements
                        .as_ref()
                        .and_then(|p| p.first())
                        .map(|p| p.x)
                        .unwrap_or(0.0),
                    removed
                        .placements
                        .as_ref()
                        .and_then(|p| p.first())
                        .map(|p| p.y)
                        .unwrap_or(0.0),
                    removed,
                );
                organism.active_transformation_id = None;
                return;
            };
            let tx = EnergyTransaction {
                reason: EnergyReason::Break,
                potential_released: gross,
                usable_delta: usable,
                structural_delta: 0.0,
                heat_dissipated: heat,
            };
            if !ledger.settle_transaction(&mut organism.usable_energy, tx) {
                let first = stored.placements.as_ref().and_then(|p| p.first()).copied();
                let _ = environment.field.deposit_physical(
                    first.map(|p| p.x).unwrap_or(0.0),
                    first.map(|p| p.y).unwrap_or(0.0),
                    removed,
                );
                organism.active_transformation_id = None;
                return;
            }
            for piece in pieces {
                if let Some(placement) = piece
                    .placements
                    .as_ref()
                    .and_then(|placements| placements.first())
                {
                    let _ = environment
                        .field
                        .deposit_physical(placement.x, placement.y, piece);
                }
            }
            organism.add_transaction_stress(heat);
            organism.active_transformation_id = None;
            crate::decision_runtime::record_outcome(
                &mut organism.decision_history,
                &ActionCandidate {
                    action: ActionKind::Break,
                    context_key: transformation.decision_context_key.clone(),
                },
                if usable > f64::EPSILON {
                    OutcomeKind::Beneficial
                } else if heat > f64::EPSILON {
                    OutcomeKind::Harmful
                } else {
                    OutcomeKind::Neutral
                },
            );
            return;
        }

        let Some(pieces) = stored.break_internal_bond(&target) else {
            organism.active_transformation_id = None;
            return;
        };
        let tx = EnergyTransaction {
            reason: EnergyReason::Break,
            potential_released: gross,
            usable_delta: usable,
            structural_delta: 0.0,
            heat_dissipated: heat,
        };
        if !ledger.settle_transaction(&mut organism.usable_energy, tx) {
            organism.active_transformation_id = None;
            return;
        }
        for piece in pieces {
            if !organism
                .stored_material
                .store_physical_instance(piece.clone())
            {
                if let Some(placement) = piece
                    .placements
                    .as_ref()
                    .and_then(|placements| placements.first())
                {
                    let _ = environment
                        .field
                        .deposit_physical(placement.x, placement.y, piece);
                }
            }
        }
        organism.add_transaction_stress(heat);
        organism.active_transformation_id = None;
        crate::decision_runtime::record_outcome(
            &mut organism.decision_history,
            &ActionCandidate {
                action: ActionKind::Break,
                context_key: transformation.decision_context_key.clone(),
            },
            if usable > f64::EPSILON {
                OutcomeKind::Beneficial
            } else if heat > f64::EPSILON {
                OutcomeKind::Harmful
            } else {
                OutcomeKind::Neutral
            },
        );
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
        let (gross, usable, heat) = break_energy_yield(carbon, methane, 0.0, 0.8).unwrap();
        assert_eq!(gross, 21.0);
        assert!(usable > 0.0);
        assert!(heat >= 0.0);
        assert!((gross - usable - heat).abs() < 1e-12);
    }

    #[test]
    fn stress_break_candidates_exclude_genome_boundary_bonds() {
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
            memory: Vec::new(),
            decision_history: crate::decision::DecisionHistory::default(),
            usable_energy: 1_000_000.0,
            stress: 0.0,
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
        let candidates = stress_break_candidate_indices(&organism, &environment);
        assert!(!candidates.is_empty());
        let genome_bonds = crate::cavity::analyze_genome_cavity(&organism.structure, &catalog)
            .unwrap()
            .unwrap()
            .boundary_bond_indices(&organism.structure);
        assert!(candidates.iter().all(|index| !genome_bonds.contains(index)));
    }
}
