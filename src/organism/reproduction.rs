//! Physical reproduction lifecycle.
//!
//! Reproduction keeps the developing member as a stable-ID view into the
//! parent's authoritative physical aggregate. Growth and division stage graph
//! changes transactionally; the view is not a second owner of those constituents.
use crate::combine_runtime::DevelopmentalContext;
use crate::construction_material_selection::ConstructionMaterialDecision;
use crate::contact::ConnectionCompatibilityCache;
use crate::energy_ledger::EnergyLedgerAuthority;
use crate::material_storage::MaterialStorage;
use crate::organism_viability::{
    validate_realized_organism_for_core, OrganismViabilityRequirements,
};
use crate::resources::Material;
use crate::state::{EnergyLedger, Environment, Organism, Position, ReproductiveConstruction};
use crate::structure::OrganismStructure;
use rand_chacha::ChaCha8Rng;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ConstructionStatus {
    Waiting,
    Progress,
    Ready,
    Detached,
    Dead,
}

pub(crate) fn parent_body_geometry(
    parent: &Organism,
    catalog: &[crate::resources::BaseResource],
) -> Option<crate::organism_geometry::OrganismBodyGeometry> {
    crate::organism_geometry::OrganismBodyGeometry::from_structure(&parent.structure, catalog)
}

fn parent_child_position(
    parent: &Organism,
    anchor: &Material,
    catalog: &[crate::resources::BaseResource],
) -> Option<crate::structure::Placement> {
    let body = parent_body_geometry(parent, catalog)?;
    let anchor_name = anchor.parts.first()?.0.as_str();
    let anchor_resource = catalog.iter().find(|r| r.name == anchor_name)?;

    for parent_part in &body.parts {
        let anchor_reference = crate::structure::Placement {
            x: parent_part.x,
            y: parent_part.y,
            rotation_radians: parent_part.rotation_radians,
        };
        for candidate in crate::construction_runtime::candidate_placements(
            &parent.structure,
            anchor_resource,
            anchor_reference,
            &[parent_part.unit_index],
            catalog,
        ) {
            let anchor_part = crate::material_geometry::PlacedMaterialPart {
                part_index: usize::MAX,
                form: anchor_resource.shape.form.clone(),
                placement: candidate,
            };
            let parent_part_form = crate::material_geometry::PlacedMaterialPart {
                part_index: parent_part.unit_index,
                form: parent_part.form.clone(),
                placement: crate::structure::Placement {
                    x: parent_part.x,
                    y: parent_part.y,
                    rotation_radians: parent_part.rotation_radians,
                },
            };
            // `placed_forms_overlap` intentionally includes boundary touch,
            // so it cannot by itself distinguish a valid attachment from
            // interpenetrating matter. Require actual boundary contact with
            // the intended parent part and reject penetration against every
            // realized parent part before using this as the construction anchor.
            let touches_target = crate::material_geometry::placed_forms_boundary_contact(
                &parent_part_form,
                &anchor_part,
                0.0,
            );
            let penetrates_parent = body.parts.iter().any(|part| {
                let existing = crate::material_geometry::PlacedMaterialPart {
                    part_index: part.unit_index,
                    form: part.form.clone(),
                    placement: crate::structure::Placement {
                        x: part.x,
                        y: part.y,
                        rotation_radians: part.rotation_radians,
                    },
                };
                crate::material_geometry::placed_forms_penetrate(&existing, &anchor_part, 0.0)
            });
            if touches_target && !penetrates_parent {
                return Some(candidate);
            }
        }
    }
    None
}

fn child_intersects_realized_parent_region(
    structure: &OrganismStructure,
    parent_body: &crate::organism_geometry::OrganismBodyGeometry,
) -> bool {
    child_remains_within_realized_parent_boundary(structure, parent_body)
}

fn child_remains_within_realized_parent_boundary(
    structure: &OrganismStructure,
    parent_body: &crate::organism_geometry::OrganismBodyGeometry,
) -> bool {
    structure.units.iter().any(|unit| {
        let Some(geometry) = unit.geometry.as_ref() else {
            return false;
        };
        let child = crate::material_geometry::PlacedMaterialPart {
            part_index: 0,
            form: geometry.shape().form.clone(),
            placement: unit.placement,
        };
        parent_body.parts.iter().any(|parent_part| {
            let parent_part = crate::material_geometry::PlacedMaterialPart {
                part_index: parent_part.unit_index,
                form: parent_part.form.clone(),
                placement: crate::structure::Placement {
                    x: parent_part.x,
                    y: parent_part.y,
                    rotation_radians: parent_part.rotation_radians,
                },
            };
            crate::material_geometry::placed_forms_overlap(&parent_part, &child, 0.0)
        })
    })
}

fn parent_child_in_contact(parent: &OrganismStructure, child: &OrganismStructure) -> bool {
    parent.units.iter().any(|parent_unit| {
        let Some(parent_geometry) = parent_unit.geometry.as_ref() else {
            return false;
        };
        let parent_part = crate::material_geometry::PlacedMaterialPart {
            part_index: 0,
            form: parent_geometry.shape().form.clone(),
            placement: parent_unit.placement,
        };
        child.units.iter().any(|child_unit| {
            let Some(child_geometry) = child_unit.geometry.as_ref() else {
                return false;
            };
            let child_part = crate::material_geometry::PlacedMaterialPart {
                part_index: 0,
                form: child_geometry.shape().form.clone(),
                placement: child_unit.placement,
            };
            crate::material_geometry::placed_forms_overlap(&parent_part, &child_part, 0.0)
        })
    })
}

fn developing_organism(
    construction: &ReproductiveConstruction,
    structure: OrganismStructure,
) -> Organism {
    Organism {
        id: "developing-offspring".into(),
        developmental_origin: construction.developmental_origin.clone(),
        developmental_orientation_radians: construction.developmental_orientation_radians,
        occupied_cells: vec![construction.developmental_origin.clone()],
        genome: construction.child_genome.clone(),
        harmonic_spectrum: crate::harmonics::ToneSpectrum::empty(),
        resonant_energy: crate::harmonics::ResonantEnergyReservoir::default(),
        experience_memory: crate::memory::ExperienceMemory::default(),
        pending_movement_experience: None,
        decision_history: crate::decision::DecisionHistory::default(),
        usable_energy: construction.developing_energy,
        stress: construction.developing_stress,
        maintenance_debt: construction.developing_maintenance_debt,
        stress_threshold: crate::state::INITIAL_STRESS_THRESHOLD,
        stored_material: if construction.is_aggregate_owned_by(&structure) {
            // Aggregate-owned construction draws exclusively from the parent’s
            // storage. Do not materialize a second inventory on its transient view.
            MaterialStorage::default()
        } else {
            construction.committed_material.clone()
        },
        active_transformation_id: None,
        active_movement: None,
        reproductive_construction: None,
        structure,
        genome_core_identity_established: false,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NextConstructionResourceStatus {
    Available,
    Missing,
    NoFit,
}

pub(crate) fn construction_material_need_pressure(
    construction: &ReproductiveConstruction,
    parent_storage: &MaterialStorage,
    catalog: &[crate::resources::BaseResource],
) -> f64 {
    let blueprint = &construction.child_genome.developmental_blueprint;
    let x = construction.developmental_origin.x;
    let y = construction.developmental_origin.y;
    let preferred = blueprint
        .material_preferences
        .iter()
        .max_by(|a, b| {
            a.evaluate(x, y)
                .partial_cmp(&b.evaluate(x, y))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|field| field.resource_name.as_str());
    let Some(preferred) = preferred else {
        return 1.0;
    };

    // Aggregate-owned construction draws from the parent's storage; the
    // committed-material holder is only legacy/child-held inventory. Pressure
    // should request preferred material only when neither owner can supply it.
    let has_preferred_material = |storage: &MaterialStorage| {
        matches!(
            crate::construction_material_selection::select_construction_material(
                storage, preferred, catalog,
            ),
            Ok(ConstructionMaterialDecision::Selected { .. })
        )
    };
    if has_preferred_material(&construction.committed_material)
        || has_preferred_material(parent_storage)
    {
        0.0
    } else {
        1.0
    }
}

fn construction_preferred_resource(child: &Organism) -> Option<&str> {
    let blueprint = &child.genome.developmental_blueprint;
    blueprint
        .material_preferences
        .iter()
        .max_by(|a, b| {
            a.evaluate(child.developmental_origin.x, child.developmental_origin.y)
                .partial_cmp(
                    &b.evaluate(child.developmental_origin.x, child.developmental_origin.y),
                )
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|field| field.resource_name.as_str())
}

fn try_child_construction(
    child: &Organism,
    parent_storage: &MaterialStorage,
    environment: &Environment,
    ledger: &EnergyLedger,
    _context: Option<DevelopmentalContext<'_>>,
) -> Option<(Organism, EnergyLedger, Option<usize>)> {
    let preferred = construction_preferred_resource(child)?;
    let mut nodes = 0usize;

    let child_candidates =
        crate::construction_material_selection::rank_available_construction_materials(
            &child.stored_material,
            preferred,
            &environment.catalog,
        )
        .ok()?
        .into_iter()
        .filter(|(_, _, score)| {
            *score >= crate::construction_material_selection::MIN_CONSTRUCTION_MATERIAL_MATCH
        });

    for (storage_index, _, _) in child_candidates {
        let crate::material_storage::StoredMaterial::Physical(instance) =
            child.stored_material.entries.get(storage_index)?;

        for existing_index in 0..child.structure.units.len() {
            let candidate_ledger = *ledger;
            let Some((
                trial_structure,
                _indices,
                _part_index,
                _attempt,
                candidate_ledger,
                candidate_energy,
            )) = crate::construction_runtime::try_attach_physical_material_bond_driven(
                &child.structure,
                existing_index,
                instance,
                &environment.catalog,
                &mut nodes,
                &candidate_ledger,
                child.usable_energy,
            )
            else {
                continue;
            };

            let mut candidate = child.clone();
            candidate.stored_material.take_physical_at(storage_index)?;
            candidate.structure = trial_structure;
            candidate.usable_energy = candidate_energy;
            return Some((candidate, candidate_ledger, None));
        }
    }

    let parent_candidates =
        crate::construction_material_selection::rank_available_construction_materials(
            parent_storage,
            preferred,
            &environment.catalog,
        )
        .ok()?
        .into_iter()
        .filter(|(_, _, score)| {
            *score >= crate::construction_material_selection::MIN_CONSTRUCTION_MATERIAL_MATCH
        });

    for (parent_index, _, _) in parent_candidates {
        let crate::material_storage::StoredMaterial::Physical(instance) =
            parent_storage.entries.get(parent_index)?;

        for existing_index in 0..child.structure.units.len() {
            let candidate_ledger = *ledger;
            let Some((
                trial_structure,
                _indices,
                _part_index,
                _attempt,
                candidate_ledger,
                candidate_energy,
            )) = crate::construction_runtime::try_attach_physical_material_bond_driven(
                &child.structure,
                existing_index,
                instance,
                &environment.catalog,
                &mut nodes,
                &candidate_ledger,
                child.usable_energy,
            )
            else {
                continue;
            };

            let mut candidate = child.clone();
            candidate.structure = trial_structure;
            candidate.usable_energy = candidate_energy;
            return Some((candidate, candidate_ledger, Some(parent_index)));
        }
    }

    None
}

fn next_construction_resource_status(
    child: &Organism,
    parent_storage: &MaterialStorage,
    environment: &Environment,
    ledger: &EnergyLedger,
    context: Option<DevelopmentalContext<'_>>,
) -> NextConstructionResourceStatus {
    let preferred = construction_preferred_resource(child);
    let has_acceptable_physical_material = preferred.is_some_and(|preferred| {
        let acceptable = |storage: &MaterialStorage| {
            crate::construction_material_selection::select_construction_material(
                storage,
                preferred,
                &environment.catalog,
            )
            .map(|decision| matches!(decision, ConstructionMaterialDecision::Selected { .. }))
            .unwrap_or(false)
        };
        acceptable(&child.stored_material) || acceptable(parent_storage)
    });
    if !has_acceptable_physical_material {
        return NextConstructionResourceStatus::Missing;
    }

    if try_child_construction(child, parent_storage, environment, ledger, context).is_some() {
        NextConstructionResourceStatus::Available
    } else {
        NextConstructionResourceStatus::NoFit
    }
}

fn preferred_length(
    genome: &crate::genome::Genome,
    catalog: &[crate::resources::BaseResource],
) -> Option<f64> {
    let seed_reference = crate::juvenile::confirmed_seed_scale_reference(catalog).ok()?;
    preferred_length_with_reference(genome, seed_reference)
}

fn preferred_length_with_reference(
    genome: &crate::genome::Genome,
    seed_reference: (f64, f64),
) -> Option<f64> {
    let (seed_mass, seed_length) = seed_reference;
    Some(
        genome
            .developmental_blueprint
            .preferred_developmental_length(genome.preferred_mass(), seed_mass, seed_length),
    )
}

fn developmental_context<'a>(
    genome: &'a crate::genome::Genome,
    origin: &Position,
    orientation: f64,
    catalog: &[crate::resources::BaseResource],
    seed_reference: Option<(f64, f64)>,
) -> Option<DevelopmentalContext<'a>> {
    let preferred = seed_reference
        .and_then(|reference| preferred_length_with_reference(genome, reference))
        .or_else(|| preferred_length(genome, catalog))?;
    Some((
        &genome.developmental_blueprint,
        (origin.x, origin.y),
        orientation,
        preferred,
    ))
}

fn developmental_linear_extent(structure: &OrganismStructure, origin: &Position) -> f64 {
    structure
        .units
        .iter()
        .map(|unit| (unit.placement.x - origin.x).hypot(unit.placement.y - origin.y))
        .fold(0.0, f64::max)
}

fn budding_scale_reached_with_reference(
    construction: &ReproductiveConstruction,
    structure: &OrganismStructure,
    catalog: &[crate::resources::BaseResource],
    seed_reference: Option<(f64, f64)>,
) -> bool {
    let preferred = seed_reference
        .and_then(|reference| {
            preferred_length_with_reference(&construction.child_genome, reference)
        })
        .or_else(|| preferred_length(&construction.child_genome, catalog));
    let Some(preferred) = preferred else {
        return false;
    };
    if preferred <= 0.0 || !preferred.is_finite() {
        return false;
    }
    developmental_linear_extent(structure, &construction.developmental_origin) + 1e-9
        >= 0.80 * preferred
}

fn developing_membership_is_valid(
    construction: &ReproductiveConstruction,
    structure: &OrganismStructure,
) -> bool {
    let actual = structure.physical_constituent_ids();
    if actual.len() != construction.developing_member_ids.len() {
        return false;
    }
    let actual_ids: std::collections::HashSet<_> = actual.iter().copied().collect();
    let recorded_ids: std::collections::HashSet<_> =
        construction.developing_member_ids.iter().copied().collect();
    actual_ids.len() == actual.len()
        && recorded_ids.len() == construction.developing_member_ids.len()
        && actual_ids == recorded_ids
        && construction
            .anchor_physical_id
            .is_none_or(|anchor_id| actual_ids.contains(&anchor_id))
}

fn birth_ready_with_reference(
    construction: &ReproductiveConstruction,
    parent_structure: &OrganismStructure,
    catalog: &[crate::resources::BaseResource],
    seed_reference: Option<(f64, f64)>,
) -> bool {
    // Readiness is evaluated against a transient view of the authoritative
    // aggregate whenever the offspring's members are already parent-owned.
    // The persisted graph is only the compatibility path for detached legacy
    // constructions and is never the source of truth for attached members.
    let structure = if construction.is_aggregate_owned_by(parent_structure) {
        let Some(anchor_id) = construction.anchor_physical_id else {
            return false;
        };
        let Some(view) = parent_structure
            .subgraph_for_physical_ids(&construction.developing_member_ids, &[anchor_id])
        else {
            return false;
        };
        view
    } else {
        construction.developing_structure.clone()
    };
    if !developing_membership_is_valid(construction, &structure) {
        return false;
    }
    let Some(anchor_id) = construction
        .anchor_physical_id
        .filter(|id| structure.unit_index(*id).is_some())
        .or_else(|| structure.physical_id(construction.anchor_unit_index))
    else {
        return false;
    };
    let Some(anchor_index) = structure.unit_index(anchor_id) else {
        return false;
    };
    let Ok(cavity) =
        crate::cavity::analyze_genome_cavity_for_core(&structure, catalog, &[anchor_id])
    else {
        return false;
    };
    let Some(cavity) = cavity else {
        return false;
    };
    if !cavity.boundary_units.contains(&anchor_index) {
        return false;
    }
    if validate_realized_organism_for_core(
        &structure,
        catalog,
        OrganismViabilityRequirements::default(),
        &[anchor_id],
    )
    .is_err()
    {
        return false;
    }
    budding_scale_reached_with_reference(construction, &structure, catalog, seed_reference)
}

fn anchor_structure(
    child_genome: &crate::genome::Genome,
    anchor_storage: MaterialStorage,
    placement: crate::structure::Placement,
    catalog: &[crate::resources::BaseResource],
) -> Option<(OrganismStructure, MaterialStorage, usize)> {
    let mut child = Organism {
        id: "developing-offspring".into(),
        developmental_origin: Position {
            x: placement.x,
            y: placement.y,
        },
        developmental_orientation_radians: 0.0,
        occupied_cells: vec![Position {
            x: placement.x,
            y: placement.y,
        }],
        genome: child_genome.clone(),
        harmonic_spectrum: crate::harmonics::ToneSpectrum::empty(),
        resonant_energy: crate::harmonics::ResonantEnergyReservoir::default(),
        experience_memory: crate::memory::ExperienceMemory::default(),
        pending_movement_experience: None,
        decision_history: crate::decision::DecisionHistory::default(),
        usable_energy: 0.0,
        stress: 0.0,
        maintenance_debt: 0.0,
        stress_threshold: crate::state::INITIAL_STRESS_THRESHOLD,
        stored_material: anchor_storage,
        genome_core_identity_established: false,
        active_transformation_id: None,
        active_movement: None,
        reproductive_construction: None,
        structure: OrganismStructure::new(),
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
    let anchor_unit_index = crate::combine_runtime::instantiate_one_unit(&mut child, catalog)?;
    if let Some(anchor_unit) = child.structure.units.get_mut(anchor_unit_index) {
        anchor_unit.placement.rotation_radians = placement.rotation_radians;
    }
    Some((child.structure, child.stored_material, anchor_unit_index))
}

pub(crate) fn construction_usable_energy_held(organism: &Organism) -> f64 {
    let Some(construction) = organism.reproductive_construction.as_ref() else {
        return 0.0;
    };

    if construction.is_aggregate_owned_by(&organism.structure) {
        // Aggregate-owned construction has no independent energy holder.
        // A stale nonzero cache signals ambiguous ownership; do not silently
        // count or discard it.
        if construction.developing_energy == 0.0 {
            0.0
        } else {
            f64::NAN
        }
    } else if construction.has_any_member_in(&organism.structure) {
        // Partial physical ownership cannot be reconciled by counting either
        // holder opportunistically.
        f64::NAN
    } else {
        construction.developing_energy
    }
}

pub(crate) fn total_usable_energy_held(
    organisms: &[Organism],
    decomposing_bodies: &[crate::decomposition::DecomposingBody],
) -> f64 {
    let organism_energy: f64 = organisms
        .iter()
        .map(|organism| organism.usable_energy + construction_usable_energy_held(organism))
        .sum();
    let decomposition_energy: f64 = decomposing_bodies
        .iter()
        .map(|body| body.energy_budget)
        .sum();
    organism_energy + decomposition_energy
}

pub(crate) fn total_structural_energy_held(
    organisms: &[Organism],
    decomposing_bodies: &[crate::decomposition::DecomposingBody],
) -> f64 {
    let organism_energy = organisms
        .iter()
        .map(|organism| {
            let own_energy = organism
                .structure
                .bonds
                .iter()
                .map(|bond| bond.bond_energy)
                .sum::<f64>();
            let construction_energy =
                organism
                    .reproductive_construction
                    .as_ref()
                    .map_or(0.0, |construction| {
                        if construction.is_aggregate_owned_by(&organism.structure) {
                            0.0
                        } else if construction.has_any_member_in(&organism.structure) {
                            f64::NAN
                        } else {
                            construction
                                .developing_structure
                                .bonds
                                .iter()
                                .map(|bond| bond.bond_energy)
                                .sum::<f64>()
                        }
                    });
            own_energy + construction_energy
        })
        .sum::<f64>();
    let decomposition_energy = decomposing_bodies
        .iter()
        .map(|body| {
            body.structure
                .bonds
                .iter()
                .map(|bond| bond.bond_energy)
                .sum::<f64>()
        })
        .sum::<f64>();
    organism_energy + decomposition_energy
}

pub(crate) fn total_resonant_energy_held(organisms: &[Organism]) -> f64 {
    organisms
        .iter()
        .map(|organism| organism.resonant_energy.total_energy().unwrap_or(f64::NAN))
        .sum()
}

pub(crate) fn begin_reproduction(
    parent: &mut Organism,
    rng: &mut ChaCha8Rng,
    catalog: &[crate::resources::BaseResource],
    ledger: &mut EnergyLedger,
) -> bool {
    if parent.active_transformation_id.is_some() || parent.reproductive_construction.is_some() {
        return false;
    }
    let mut child_genome = parent.genome.clone();
    child_genome.mutate(rng);
    if child_genome.developmental_blueprint.validate().is_err()
        || !child_genome.reproductive_energy_allocation.is_finite()
        || child_genome.reproductive_energy_allocation <= 0.0
    {
        return false;
    }

    for (storage_index, entry) in parent
        .stored_material
        .entries
        .clone()
        .into_iter()
        .enumerate()
    {
        let crate::material_storage::StoredMaterial::Physical(instance) = &entry;
        let anchor = instance.material.clone();
        let Some(placement) = parent_child_position(parent, &anchor, catalog) else {
            continue;
        };
        let mut trial_storage = MaterialStorage::default();
        trial_storage
            .entries
            .push(crate::material_storage::StoredMaterial::Physical(
                instance.clone(),
            ));
        let Some((mut anchor_graph, mut child_storage, anchor_unit_index)) =
            anchor_structure(&child_genome, trial_storage, placement, catalog)
        else {
            continue;
        };
        let Some(anchor_physical_id) = anchor_graph.physical_id(anchor_unit_index) else {
            continue;
        };
        anchor_graph.set_genome_constituent_ids(vec![anchor_physical_id]);
        let anchor_ids = anchor_graph.physical_constituent_ids();
        if anchor_ids.is_empty() || !anchor_ids.contains(&anchor_physical_id) {
            continue;
        }

        // Stage the merge, physical COMBINE bond, energy/ledger settlement, and
        // storage removal. The live parent changes only after every step validates.
        let mut staged_structure = parent.structure.clone();
        let parent_unit_count = staged_structure.units.len();
        let Some(remapping) = staged_structure.append_graph_remapping_ids(&anchor_graph) else {
            continue;
        };
        let Some(mapped_anchor_id) = remapping.get(&anchor_physical_id).copied() else {
            continue;
        };
        let Some(mapped_anchor_ids) = anchor_ids
            .iter()
            .map(|id| remapping.get(id).copied())
            .collect::<Option<Vec<_>>>()
        else {
            continue;
        };

        let mut committed: Option<(OrganismStructure, EnergyLedger, f64)> = None;
        'contacts: for parent_unit in 0..parent_unit_count {
            for anchor_unit in parent_unit_count..staged_structure.units.len() {
                let mut cache = ConnectionCompatibilityCache::new();
                let candidates = crate::contact::connection_pair_candidates_cached(
                    &staged_structure,
                    parent_unit,
                    anchor_unit,
                    catalog,
                    &mut cache,
                );
                for candidate in candidates {
                    if candidate.distance > crate::combine_runtime::COMBINE_CONTACT_TOLERANCE
                        || !candidate.available_a
                        || !candidate.available_b
                    {
                        continue;
                    }
                    let Some((_, _, investment, _)) =
                        crate::combine_runtime::selected_candidate_evaluation(
                            &staged_structure,
                            parent_unit,
                            anchor_unit,
                            candidate,
                            catalog,
                        )
                    else {
                        continue;
                    };
                    let mut bond_trial = staged_structure.clone();
                    let mut ledger_trial = *ledger;
                    let mut energy_trial = parent.usable_energy;
                    let mut bond_cache = ConnectionCompatibilityCache::new();
                    if crate::combine_runtime::form_selected_bond(
                        &mut bond_trial,
                        parent_unit,
                        anchor_unit,
                        candidate,
                        investment,
                        catalog,
                        &mut bond_cache,
                        &mut ledger_trial,
                        &mut energy_trial,
                    )
                    .is_some()
                    {
                        committed = Some((bond_trial, ledger_trial, energy_trial));
                        break 'contacts;
                    }
                }
            }
        }
        let Some((staged_structure, staged_ledger, staged_energy)) = committed else {
            continue;
        };
        let Some(developing_structure) =
            staged_structure.subgraph_for_physical_ids(&mapped_anchor_ids, &[mapped_anchor_id])
        else {
            continue;
        };
        let mut parent_trial_storage = parent.stored_material.clone();
        if parent_trial_storage
            .take_physical_at(storage_index)
            .is_none()
        {
            continue;
        }
        let Some(refreshed_anchor_index) = developing_structure.unit_index(mapped_anchor_id) else {
            continue;
        };

        // Any unconsumed anchor material remains in the parent’s one inventory;
        // the aggregate-owned construction is only a stable-ID view.
        parent_trial_storage.entries.extend(child_storage.drain_entries());

        *ledger = staged_ledger;
        parent.usable_energy = staged_energy;
        parent.structure = staged_structure;
        parent.stored_material = parent_trial_storage;
        parent.reproductive_construction = Some(ReproductiveConstruction {
            committed_material: MaterialStorage::default(),
            developing_member_ids: mapped_anchor_ids,
            // The anchor is already part of the parent's physical aggregate.
            // Keep the persisted graph empty; aggregate-owned working graphs
            // are derived transiently from stable physical IDs.
            developing_structure: OrganismStructure::new(),
            child_genome: child_genome.clone(),
            developmental_origin: Position {
                x: placement.x,
                y: placement.y,
            },
            developmental_orientation_radians: 0.0,
            developing_stress: 0.0,
            anchor_unit_index: refreshed_anchor_index,
            anchor_physical_id: Some(mapped_anchor_id),
            developing_energy: 0.0,
            developing_maintenance_debt: 0.0,
            needs_space: false,
        });
        return true;
    }
    false
}

fn commit_candidate_attachment(
    parent_structure: &mut OrganismStructure,
    parent_storage: &mut MaterialStorage,
    construction: &mut ReproductiveConstruction,
    environment: &Environment,
    ledger: &mut EnergyLedger,
    parent_energy: &mut f64,
    parent_body: &crate::organism_geometry::OrganismBodyGeometry,
    mut child: Organism,
    candidate_ledger: EnergyLedger,
    transferred: Option<usize>,
    before_units: usize,
    seed_reference: Option<(f64, f64)>,
) -> ConstructionStatus {
    // Validate before consuming a parent-owned material entry or publishing
    // the candidate ledger/graph. A rejected placement must not partially
    // commit the attachment transaction.
    if !child_intersects_realized_parent_region(&child.structure, parent_body)
        || !parent_child_in_contact(parent_structure, &child.structure)
    {
        return ConstructionStatus::Detached;
    }

    let candidate_units = child.structure.units.len();
    let aggregate_owned = construction.is_aggregate_owned_by(parent_structure);
    if aggregate_owned && (!child.usable_energy.is_finite() || child.usable_energy < 0.0) {
        return ConstructionStatus::Waiting;
    }
    // When construction is already represented inside the parent aggregate,
    // stage the aggregate replacement before consuming material or publishing
    // the candidate ledger. Keep only stable IDs as the aggregate-owned view;
    // the cached graph is retained solely for detached legacy construction.
    let staged_aggregate = if construction.is_aggregate_owned_by(parent_structure) {
        let mut staged = parent_structure.clone();
        let old_ids = construction.developing_member_ids.clone();
        let Some((_, refreshed_view)) =
            staged.replace_subgraph_remapping_new_ids_and_extract(&old_ids, &child.structure)
        else {
            return ConstructionStatus::Waiting;
        };
        Some((staged, refreshed_view))
    } else {
        None
    };
    if let Some(parent_index) = transferred {
        if parent_storage.take_physical_at(parent_index).is_none() {
            return ConstructionStatus::Waiting;
        }
    }
    if let Some((staged, refreshed_view)) = staged_aggregate {
        *parent_structure = staged;
        child.structure = refreshed_view;
    }
    *ledger = candidate_ledger;
    if aggregate_owned {
        *parent_energy = child.usable_energy;
    }
    if aggregate_owned {
        // Any leftover candidate material remains owned by the parent inventory,
        // not by a second construction-specific storage holder.
        parent_storage
            .entries
            .extend(child.stored_material.drain_entries());
        construction.committed_material = MaterialStorage::default();
    } else {
        construction.committed_material = child.stored_material;
    }
    construction.developing_member_ids = child.structure.physical_constituent_ids();
    construction.developing_structure = if aggregate_owned {
        OrganismStructure::new()
    } else {
        child.structure
    };
    construction.needs_space = false;
    construction.developing_energy = if aggregate_owned {
        0.0
    } else {
        child.usable_energy
    };
    construction.developing_stress = child.stress;

    if birth_ready_with_reference(
        construction,
        parent_structure,
        &environment.catalog,
        seed_reference,
    ) {
        ConstructionStatus::Ready
    } else if candidate_units > before_units {
        ConstructionStatus::Progress
    } else {
        ConstructionStatus::Waiting
    }
}

pub(crate) fn advance_construction(
    parent_structure: &mut OrganismStructure,
    parent_storage: &mut MaterialStorage,
    construction: &mut ReproductiveConstruction,
    environment: &Environment,
    ledger: &mut EnergyLedger,
    parent_energy: &mut f64,
    _rng: &mut ChaCha8Rng,
    parent_body: &crate::organism_geometry::OrganismBodyGeometry,
    seed_reference: Option<(f64, f64)>,
) -> (ConstructionStatus, Option<f64>) {
    // Stable IDs, not the cached developing graph, determine membership once
    // the child has been admitted into the parent's physical aggregate. Rebuild
    // the working view before validation or chemistry so stale cached geometry
    // cannot become a second physical authority.
    let any_ids_present_in_parent = construction.has_any_member_in(parent_structure);
    let aggregate_owned = construction.is_aggregate_owned_by(parent_structure);
    if any_ids_present_in_parent && !aggregate_owned {
        return (ConstructionStatus::Dead, None);
    }
    let working_structure = if aggregate_owned {
        let Some(anchor_id) = construction.anchor_physical_id else {
            return (ConstructionStatus::Dead, None);
        };
        let Some(view) = parent_structure
            .subgraph_for_physical_ids(&construction.developing_member_ids, &[anchor_id])
        else {
            return (ConstructionStatus::Dead, None);
        };
        view
    } else {
        construction.developing_structure.clone()
    };

    // Validate the authoritative transient view before any new parent-funded
    // reserve transfer. A non-finite construction must not drain its parent.
    if !developing_membership_is_valid(construction, &working_structure) {
        return (ConstructionStatus::Dead, None);
    }
    if aggregate_owned && !construction.committed_material.is_empty() {
        // Migrate serialized legacy inventory into the parent's authoritative
        // storage before advancing the aggregate-owned construction.
        parent_storage
            .entries
            .extend(construction.committed_material.drain_entries());
    }
    let reserve_energy = construction.child_genome.reproductive_energy_allocation;
    // Validate all persisted scalar accounting state before reserve transfer,
    // maintenance settlement, or candidate construction can mutate it.
    // Comparisons against NaN otherwise silently skip the reserve branch and
    // allow invalid energy/debt to flow into the live construction path.
    if !construction.developing_stress.is_finite()
        || !crate::state::INITIAL_STRESS_THRESHOLD.is_finite()
        || !construction.developing_energy.is_finite()
        || construction.developing_energy < 0.0
        || !construction.developing_maintenance_debt.is_finite()
        || construction.developing_maintenance_debt < 0.0
        || !reserve_energy.is_finite()
        || reserve_energy < 0.0
        || !parent_energy.is_finite()
        || *parent_energy < 0.0
    {
        return (ConstructionStatus::Dead, None);
    }

    if !aggregate_owned && reserve_energy > construction.developing_energy {
        let transfer =
            (reserve_energy - construction.developing_energy).min(parent_energy.max(0.0));
        if transfer > 0.0
            && !ledger.transfer(parent_energy, &mut construction.developing_energy, transfer)
        {
            // A rejected reserve transfer must not let construction, maintenance,
            // or structural chemistry advance without the intended energy move.
            return (ConstructionStatus::Waiting, None);
        }
    }

    // Rebuild after the reserve transfer so the child snapshot contains the
    // newly transferred energy rather than overwriting it with a stale balance.
    let original_working_structure = working_structure.clone();
    let mut child = developing_organism(construction, working_structure);
    if aggregate_owned {
        child.usable_energy = *parent_energy;
    }
    let mut staged_ledger = *ledger;
    // An aggregate-owned developing core is not a second living organism.
    // The parent tick already applies maintenance and structural chemistry to
    // the complete authoritative aggregate; repeating those operations on a
    // subgraph would charge energy and process bonds twice. Keep this staged
    // view for growth/readiness only. The legacy detached path retains its own
    // maintenance until it is migrated into aggregate ownership.
    let dead = if aggregate_owned {
        child.validate_stress_state()
    } else {
        child.apply_maintenance(&environment.catalog, &mut staged_ledger);
        let dead = child.validate_stress_state();
        if !dead {
            let _ = crate::transformation::resolve_resonance_overload_break(
                &mut child,
                environment,
                &mut staged_ledger,
            );
        }
        dead
    };
    // Maintenance and resonance chemistry may alter the construction graph
    // (including bonds). If its members are aggregate-owned, commit those
    // changes transactionally into the parent before publishing the refreshed
    // construction view or continuing growth.
    if construction.is_aggregate_owned_by(parent_structure)
        && (child.structure.units != original_working_structure.units
            || child.structure.bonds != original_working_structure.bonds)
    {
        // Maintenance commonly changes only scalar energy/debt state. Do not
        // replace an unchanged aggregate subgraph: replacement validates core
        // group boundaries and can reject a no-op view even though no geometry
        // or bond update needs committing.
        let old_ids = construction.developing_member_ids.clone();
        let Some((_, refreshed_view)) = parent_structure
            .replace_subgraph_remapping_new_ids_and_extract(&old_ids, &child.structure)
        else {
            return (ConstructionStatus::Dead, None);
        };
        child.structure = refreshed_view;
    }
    *ledger = staged_ledger;
    if aggregate_owned {
        *parent_energy = child.usable_energy;
        construction.developing_energy = 0.0;
    } else {
        construction.developing_energy = child.usable_energy;
    }
    construction.developing_stress = child.stress;
    construction.developing_maintenance_debt = child.maintenance_debt;
    construction.developing_member_ids = child.structure.physical_constituent_ids();
    construction.developing_structure = if aggregate_owned {
        OrganismStructure::new()
    } else {
        child.structure.clone()
    };
    if dead {
        return (ConstructionStatus::Dead, None);
    }

    if birth_ready_with_reference(
        construction,
        parent_structure,
        &environment.catalog,
        seed_reference,
    ) {
        return (ConstructionStatus::Ready, None);
    }

    if !parent_child_in_contact(parent_structure, &child.structure) {
        return (ConstructionStatus::Detached, None);
    }

    let anchor_index = construction
        .anchor_physical_id
        .and_then(|id| child.structure.unit_index(id))
        .unwrap_or(construction.anchor_unit_index);
    let genome_qualified =
        crate::cavity::analyze_genome_cavity(&child.structure, &environment.catalog)
            .ok()
            .flatten()
            .is_some_and(|cavity| cavity.boundary_units.contains(&anchor_index));
    let context = if genome_qualified {
        developmental_context(
            &construction.child_genome,
            &construction.developmental_origin,
            construction.developmental_orientation_radians,
            &environment.catalog,
            seed_reference,
        )
    } else {
        // Genome construction is deliberately performed through the normal
        // physical solver without a second genome blueprint. Developmental
        // fields become active guidance only after a qualifying physical
        // genome cavity exists.
        None
    };
    let before_units = child.structure.units.len();
    let Some((child, candidate_ledger, transferred)) =
        try_child_construction(&child, parent_storage, environment, ledger, context)
    else {
        match next_construction_resource_status(
            &child,
            parent_storage,
            environment,
            ledger,
            context,
        ) {
            NextConstructionResourceStatus::Missing => {
                construction.needs_space = false;
                return (ConstructionStatus::Waiting, None);
            }
            NextConstructionResourceStatus::Available => {
                construction.needs_space = true;
                return (ConstructionStatus::Waiting, None);
            }
            NextConstructionResourceStatus::NoFit => {
                // The inventory is real but none of the current physical
                // candidates can make this bond. Keep the unfinished bond
                // pending; new material may make it solvable later.
                construction.needs_space = false;
                return (ConstructionStatus::Waiting, None);
            }
        }
    };

    let status = commit_candidate_attachment(
        parent_structure,
        parent_storage,
        construction,
        environment,
        ledger,
        parent_energy,
        parent_body,
        child,
        candidate_ledger,
        transferred,
        before_units,
        seed_reference,
    );
    (status, None)
}

pub(crate) fn finish_reproduction(
    parent: &mut Organism,
    child_id: String,
    catalog: &[crate::resources::BaseResource],
    ledger: &mut EnergyLedger,
    seed_reference: Option<(f64, f64)>,
) -> Option<Organism> {
    let construction = parent.reproductive_construction.take()?;
    // Stable IDs establish ownership; readiness and division derive transient
    // views from the authoritative parent graph instead of refreshing a second
    // persisted copy. Reject partially admitted membership rather than falling
    // back to the detached legacy graph.
    let any_ids_present_in_parent = construction.has_any_member_in(&parent.structure);
    let aggregate_owned = construction.is_aggregate_owned_by(&parent.structure);
    if any_ids_present_in_parent && !aggregate_owned {
        parent.reproductive_construction = Some(construction);
        return None;
    }
    if !birth_ready_with_reference(&construction, &parent.structure, catalog, seed_reference) {
        parent.reproductive_construction = Some(construction);
        return None;
    }

    // Aggregate-owned construction can divide only by atomically partitioning
    // the authoritative parent graph. A transient child view is never promoted
    // to a second owner. Legacy, not-yet-migrated constructions retain their
    // existing behavior until they are admitted into the aggregate.
    let (child_structure, child_energy, staged_ledger, staged_parent_commit) = if aggregate_owned {
        let mut staged_parent_structure = parent.structure.clone();
        let Some(anchor_id) = construction.anchor_physical_id else {
            parent.reproductive_construction = Some(construction);
            return None;
        };
        let Some(developing_view) = parent
            .structure
            .subgraph_for_physical_ids(&construction.developing_member_ids, &[anchor_id])
        else {
            parent.reproductive_construction = Some(construction);
            return None;
        };
        let child_core_ids = developing_view.genome_constituent_ids().to_vec();
        let Some((child_structure, crossing_bond_energy)) = staged_parent_structure
            .partition_subgraph_for_division(&construction.developing_member_ids, &child_core_ids)
        else {
            parent.reproductive_construction = Some(construction);
            return None;
        };

        // Division is one publication boundary: neither resulting organism may
        // become visible unless both post-split structures independently satisfy
        // the full viability contract for their own established genome core.
        // Validate the staged parent as well as the offspring before settling
        // boundary-bond energy or mutating the live parent.
        let parent_core_ids = parent.structure.genome_constituent_ids().to_vec();
        if parent_core_ids.is_empty()
            || validate_realized_organism_for_core(
                &staged_parent_structure,
                catalog,
                OrganismViabilityRequirements::default(),
                &parent_core_ids,
            )
            .is_err()
            || child_core_ids.is_empty()
            || validate_realized_organism_for_core(
                &child_structure,
                catalog,
                OrganismViabilityRequirements::default(),
                &child_core_ids,
            )
            .is_err()
        {
            parent.reproductive_construction = Some(construction);
            return None;
        }

        // Bonds severed at the division boundary release their stored energy
        // exactly once. The release is a structural debit and usable-energy
        // credit, not a second external chemical source.
        let mut staged_ledger = *ledger;
        let mut combined_energy = parent.usable_energy;
        if crossing_bond_energy > 0.0 {
            let transaction = crate::energy_ledger::EnergyTransaction {
                reason: crate::energy_ledger::EnergyReason::Break,
                potential_released: 0.0,
                usable_delta: crossing_bond_energy,
                structural_delta: -crossing_bond_energy,
                work_expended: 0.0,
                heat_dissipated: 0.0,
            };
            if !staged_ledger.settle_transaction(&mut combined_energy, transaction) {
                parent.reproductive_construction = Some(construction);
                return None;
            }
        }

        let structural_mass = |structure: &OrganismStructure| -> Option<f64> {
            let mass = structure
                .units
                .iter()
                .map(|unit| unit.properties(catalog).map(|properties| properties.mass))
                .collect::<Option<Vec<_>>>()?
                .into_iter()
                .sum::<f64>();
            (mass.is_finite() && mass > 0.0).then_some(mass)
        };
        let (Some(parent_mass), Some(child_mass)) = (
            structural_mass(&staged_parent_structure),
            structural_mass(&child_structure),
        ) else {
            parent.reproductive_construction = Some(construction);
            return None;
        };
        let total_mass = parent_mass + child_mass;
        if !total_mass.is_finite() || total_mass <= 0.0 {
            parent.reproductive_construction = Some(construction);
            return None;
        }
        let child_share = combined_energy * (child_mass / total_mass);
        let parent_share = combined_energy - child_share;
        if !child_share.is_finite()
            || !parent_share.is_finite()
            || child_share < 0.0
            || parent_share < 0.0
        {
            parent.reproductive_construction = Some(construction);
            return None;
        }
        (
            child_structure,
            child_share,
            staged_ledger,
            Some((staged_parent_structure, parent_share)),
        )
    } else {
        (
            construction.developing_structure.clone(),
            construction.developing_energy,
            *ledger,
            None,
        )
    };

    let child_position = construction.developmental_origin.clone();
    let child = Organism {
        id: child_id,
        developmental_origin: child_position.clone(),
        developmental_orientation_radians: construction.developmental_orientation_radians,
        occupied_cells: vec![child_position],
        genome: construction.child_genome,
        harmonic_spectrum: crate::harmonics::ToneSpectrum::empty(),
        resonant_energy: crate::harmonics::ResonantEnergyReservoir::default(),
        experience_memory: crate::memory::ExperienceMemory::default(),
        pending_movement_experience: None,
        decision_history: crate::decision::DecisionHistory::default(),
        usable_energy: child_energy,
        stress: construction.developing_stress,
        maintenance_debt: construction.developing_maintenance_debt,
        stress_threshold: crate::state::INITIAL_STRESS_THRESHOLD,
        stored_material: construction.committed_material,
        genome_core_identity_established: false,
        active_transformation_id: None,
        active_movement: None,
        reproductive_construction: None,
        structure: child_structure,
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
    // Publish the parent graph, parent energy, and ledger only after the
    // complete child object has been assembled. Every rejection above leaves
    // the original parent and ledger untouched.
    if let Some((staged_parent_structure, parent_energy)) = staged_parent_commit {
        parent.structure = staged_parent_structure;
        parent.mark_structure_changed();
        parent.usable_energy = parent_energy;
    }
    *ledger = staged_ledger;
    Some(child)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genome::initial_genome;
    use crate::resources::{default_catalog, Material};
    use crate::state::Simulation;
    use rand::SeedableRng;

    #[test]
    fn reproduction_cannot_mutate_storage_while_transformation_is_active() {
        use rand::RngCore;

        let mut simulation = Simulation::new(941, 10.0);
        let mut parent = simulation.organisms.remove(0);
        parent.stored_material.entries.clear();
        assert!(parent.store_material(Material::free_base("Carbon", 1.0)));
        parent.active_transformation_id = Some(73);

        let original_entries = parent.stored_material.entries.clone();
        let original_energy = parent.usable_energy;
        let original_genome = serde_json::to_value(&parent.genome).unwrap();
        let mut rng = ChaCha8Rng::seed_from_u64(0xC0FFEE);
        let mut expected_rng = rng.clone();
        let mut ledger = EnergyLedger::default();
        let original_ledger = ledger;

        assert!(!begin_reproduction(
            &mut parent,
            &mut rng,
            &simulation.environment.catalog,
            &mut ledger,
        ));

        assert_eq!(parent.stored_material.entries, original_entries);
        assert_eq!(parent.usable_energy, original_energy);
        assert_eq!(
            serde_json::to_value(&parent.genome).unwrap(),
            original_genome
        );
        assert!(parent.reproductive_construction.is_none());
        assert_eq!(parent.active_transformation_id, Some(73));
        assert_eq!(ledger, original_ledger);
        assert_eq!(rng.next_u64(), expected_rng.next_u64());
    }

    #[test]
    fn serialized_aggregate_construction_keeps_its_persisted_graph_empty() {
        let construction = ReproductiveConstruction {
            committed_material: MaterialStorage::default(),
            developing_structure: OrganismStructure::new(),
            developing_member_ids: Vec::new(),
            child_genome: initial_genome(),
            developmental_origin: Position { x: 0.0, y: 0.0 },
            developmental_orientation_radians: 0.0,
            developing_stress: 0.0,
            anchor_unit_index: 0,
            anchor_physical_id: None,
            developing_energy: 0.0,
            developing_maintenance_debt: 0.0,
            needs_space: false,
        };

        let serialized =
            serde_json::to_value(construction).expect("reproductive construction should serialize");
        let graph = serialized
            .get("developing_structure")
            .expect("detached-legacy compatibility graph remains serializable");
        assert_eq!(graph["units"].as_array().unwrap().len(), 0);
        assert_eq!(graph["bonds"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn invalid_developing_holder_does_not_draw_parent_reserve_again() {
        use crate::environment::{ActiveMaterialField, DEFAULT_CELL_SIZE};
        use crate::resources::default_catalog;
        use crate::state::Environment;

        let catalog = default_catalog();
        let environment = Environment {
            width: 1000.0,
            height: 1000.0,
            catalog: catalog.clone(),
            field: ActiveMaterialField::new(1000.0, 1000.0, DEFAULT_CELL_SIZE),
        };
        let mut construction = ReproductiveConstruction {
            committed_material: MaterialStorage::default(),
            developing_structure: OrganismStructure::new(),
            developing_member_ids: vec![],
            child_genome: initial_genome(),
            developmental_origin: Position { x: 0.0, y: 0.0 },
            developmental_orientation_radians: 0.0,
            developing_stress: f64::NAN,
            anchor_unit_index: 0,
            anchor_physical_id: None,
            developing_energy: 0.0,
            developing_maintenance_debt: 0.0,
            needs_space: false,
        };
        let mut parent_structure = OrganismStructure::new();
        let mut parent_storage = MaterialStorage::default();
        let mut ledger = EnergyLedger::default();
        let ledger_before = ledger;
        let mut parent_energy = 25.0;
        let mut rng = ChaCha8Rng::seed_from_u64(1);
        let body = crate::organism_geometry::OrganismBodyGeometry {
            parts: Vec::new(),
            min_x: 0.0,
            max_x: 0.0,
            min_y: 0.0,
            max_y: 0.0,
        };

        let (status, stress) = advance_construction(
            &mut parent_structure,
            &mut parent_storage,
            &mut construction,
            &environment,
            &mut ledger,
            &mut parent_energy,
            &mut rng,
            &body,
            None,
        );

        assert_eq!(status, ConstructionStatus::Dead);
        assert_eq!(stress, None);
        assert_eq!(parent_energy, 25.0);
        assert_eq!(ledger, ledger_before);
        assert!(parent_storage.is_empty());
        assert!(construction.developing_stress.is_nan());
    }

    #[test]
    fn invalid_developing_energy_does_not_draw_parent_reserve_or_mutate_ledger() {
        use crate::environment::{ActiveMaterialField, DEFAULT_CELL_SIZE};
        use crate::state::Environment;

        let catalog = default_catalog();
        let environment = Environment {
            width: 1000.0,
            height: 1000.0,
            catalog: catalog.clone(),
            field: ActiveMaterialField::new(1000.0, 1000.0, DEFAULT_CELL_SIZE),
        };
        let mut construction = ReproductiveConstruction {
            committed_material: MaterialStorage::default(),
            developing_structure: OrganismStructure::new(),
            developing_member_ids: vec![],
            child_genome: initial_genome(),
            developmental_origin: Position { x: 0.0, y: 0.0 },
            developmental_orientation_radians: 0.0,
            developing_stress: 0.0,
            anchor_unit_index: 0,
            anchor_physical_id: None,
            developing_energy: f64::NAN,
            developing_maintenance_debt: 0.0,
            needs_space: false,
        };
        let mut parent_structure = OrganismStructure::new();
        let mut parent_storage = MaterialStorage::default();
        let mut ledger = EnergyLedger::default();
        let ledger_before = ledger;
        let mut parent_energy = 25.0;
        let mut rng = ChaCha8Rng::seed_from_u64(2);
        let body = crate::organism_geometry::OrganismBodyGeometry {
            parts: Vec::new(),
            min_x: 0.0,
            max_x: 0.0,
            min_y: 0.0,
            max_y: 0.0,
        };

        let status = advance_construction(
            &mut parent_structure,
            &mut parent_storage,
            &mut construction,
            &environment,
            &mut ledger,
            &mut parent_energy,
            &mut rng,
            &body,
            None,
        )
        .0;

        assert_eq!(status, ConstructionStatus::Dead);
        assert_eq!(parent_energy, 25.0);
        assert_eq!(ledger, ledger_before);
        assert!(parent_storage.is_empty());
        assert!(construction.developing_energy.is_nan());
    }

    #[test]
    fn developmental_scale_requires_eighty_percent_linear_target() {
        let catalog = default_catalog();
        let genome = initial_genome();
        let origin = Position { x: 0.0, y: 0.0 };
        let preferred = preferred_length(&genome, &catalog).unwrap();
        let construction = ReproductiveConstruction {
            committed_material: MaterialStorage::default(),
            developing_structure: OrganismStructure::new(),
            developing_member_ids: vec![],
            child_genome: genome,
            developmental_origin: origin,
            developmental_orientation_radians: 0.0,
            developing_stress: 0.0,
            anchor_unit_index: 0,
            anchor_physical_id: None,
            developing_energy: 0.0,
            developing_maintenance_debt: 0.0,
            needs_space: false,
        };
        assert!(preferred > 0.0);
        assert!(!budding_scale_reached_with_reference(
            &construction,
            &construction.developing_structure,
            &catalog,
            None,
        ));
    }

    #[test]
    fn developing_membership_validation_rejects_missing_or_duplicate_ids() {
        let mut simulation = Simulation::new(7, 20.0);
        let mut parent = simulation.organisms.remove(0);
        assert!(parent.store_material(Material::free_base("Carbon", 1.0)));
        let mut ledger = EnergyLedger::default();
        assert!(begin_reproduction(
            &mut parent,
            &mut simulation.rng,
            &simulation.environment.catalog,
            &mut ledger,
        ));
        let mut construction = parent.reproductive_construction.take().unwrap();
        let anchor_id = construction.anchor_physical_id.unwrap();
        let transient_view = parent
            .structure
            .subgraph_for_physical_ids(&construction.developing_member_ids, &[anchor_id])
            .unwrap();
        assert!(construction.developing_structure.units.is_empty());
        assert!(developing_membership_is_valid(
            &construction,
            &transient_view
        ));

        construction.developing_member_ids.clear();
        assert!(!developing_membership_is_valid(
            &construction,
            &transient_view
        ));

        construction.developing_member_ids = transient_view.physical_constituent_ids();
        let first_id = construction.developing_member_ids[0];
        construction.developing_member_ids.push(first_id);
        assert!(!developing_membership_is_valid(
            &construction,
            &transient_view
        ));
    }

    #[test]
    fn aggregate_owned_growth_uses_transient_view_from_stable_ids() {
        let mut simulation = Simulation::new(708, 20.0);
        let mut parent = simulation.organisms.remove(0);
        assert!(parent.store_material(Material::free_base("Carbon", 1.0)));
        let mut ledger = EnergyLedger::default();
        assert!(begin_reproduction(
            &mut parent,
            &mut simulation.rng,
            &simulation.environment.catalog,
            &mut ledger,
        ));

        let construction = parent.reproductive_construction.as_mut().unwrap();
        assert!(construction.is_aggregate_owned_by(&parent.structure));
        let member_id = construction.developing_member_ids[0];
        let aggregate_index = parent.structure.unit_index(member_id).unwrap();
        let aggregate_pose = parent.structure.units[aggregate_index].placement;
        assert!(
            construction.developing_structure.units.is_empty(),
            "aggregate-owned construction must not persist a duplicate physical graph"
        );
        assert!(construction.is_aggregate_owned_by(&parent.structure));

        let transient_view = parent
            .structure
            .subgraph_for_physical_ids(&construction.developing_member_ids, &[member_id])
            .expect("stable IDs must produce a transient view");
        let view_index = transient_view.unit_index(member_id).unwrap();
        assert_eq!(
            transient_view.units[view_index].placement, aggregate_pose,
            "the transient view must use the authoritative aggregate pose"
        );
    }

    #[test]
    fn finish_reproduction_readiness_ignores_stale_cached_graph() {
        let mut simulation = Simulation::new(709, 20.0);
        let mut parent = simulation.organisms.remove(0);
        assert!(parent.store_material(Material::free_base("Carbon", 1.0)));
        let mut ledger = EnergyLedger::default();
        assert!(begin_reproduction(
            &mut parent,
            &mut simulation.rng,
            &simulation.environment.catalog,
            &mut ledger,
        ));

        let construction = parent.reproductive_construction.as_ref().unwrap();
        assert!(construction.is_aggregate_owned_by(&parent.structure));
        let member_id = construction.developing_member_ids[0];
        let aggregate_index = parent.structure.unit_index(member_id).unwrap();
        let aggregate_pose = parent.structure.units[aggregate_index].placement;
        assert!(construction.developing_structure.units.is_empty());

        // A failed division attempt must preserve every authoritative holder,
        // not merely restore the construction metadata after taking it.
        let structure_before = parent.structure.clone();
        let stored_material_before = parent.stored_material.clone();
        let usable_energy_before = parent.usable_energy;
        let ledger_before = ledger;
        let member_ids_before = construction.developing_member_ids.clone();
        let committed_material_before = construction.committed_material.clone();

        assert!(finish_reproduction(
            &mut parent,
            "child".to_owned(),
            &simulation.environment.catalog,
            &mut ledger,
            None,
        )
        .is_none());

        assert_eq!(parent.structure.units, structure_before.units);
        assert_eq!(parent.structure.bonds, structure_before.bonds);
        assert_eq!(
            parent.structure.genome_constituent_ids(),
            structure_before.genome_constituent_ids()
        );
        assert_eq!(parent.stored_material, stored_material_before);
        assert_eq!(parent.usable_energy, usable_energy_before);
        assert_eq!(ledger, ledger_before);

        let retained = parent.reproductive_construction.as_ref().unwrap();
        assert_eq!(retained.developing_member_ids, member_ids_before);
        assert_eq!(retained.committed_material, committed_material_before);
        assert!(
            retained.developing_structure.units.is_empty(),
            "aggregate-owned readiness must leave the duplicate graph empty"
        );
        let aggregate_index = parent.structure.unit_index(member_id).unwrap();
        assert_eq!(
            parent.structure.units[aggregate_index].placement, aggregate_pose,
            "the parent aggregate remains the physical authority"
        );
        assert!(retained.is_aggregate_owned_by(&parent.structure));
    }

    #[test]
    fn anchor_is_admitted_into_parent_aggregate() {
        let mut simulation = Simulation::new(7, 20.0);
        let mut parent = simulation.organisms.remove(0);
        let parent_structure = parent.structure.clone();
        let mut ledger = EnergyLedger::default();
        assert!(parent.store_material(Material::free_base("Carbon", 1.0)));
        assert!(begin_reproduction(
            &mut parent,
            &mut simulation.rng,
            &simulation.environment.catalog,
            &mut ledger,
        ));
        let construction = parent
            .reproductive_construction
            .as_ref()
            .expect("reproduction is active");
        assert!(
            construction.developing_structure.units.is_empty(),
            "the persisted graph must be empty after admission into the parent aggregate"
        );
        let transient_view = parent
            .structure
            .subgraph_for_physical_ids(
                &construction.developing_member_ids,
                &[construction.anchor_physical_id.unwrap()],
            )
            .expect("stable IDs reconstruct the admitted anchor view");
        assert_eq!(
            construction.developing_member_ids,
            transient_view.physical_constituent_ids(),
            "construction membership must be tracked by stable physical IDs"
        );
        assert_eq!(
            construction
                .developing_member_ids
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len(),
            construction.developing_member_ids.len(),
            "a physical constituent may only belong to the developing member once"
        );
        assert_eq!(
            construction.anchor_physical_id,
            transient_view.physical_id(
                transient_view
                    .unit_index(construction.anchor_physical_id.unwrap())
                    .unwrap()
            ),
            "the anchor is tracked by stable physical identity"
        );
        assert!(construction.is_aggregate_owned_by(&parent.structure));
        assert!(parent.structure.units.len() > parent_structure.units.len());
        assert!(
            parent.structure.bonds.iter().any(|bond| {
                let a = construction
                    .developing_member_ids
                    .contains(&bond.endpoint_a.constituent_id);
                let b = construction
                    .developing_member_ids
                    .contains(&bond.endpoint_b.constituent_id);
                a != b
            }),
            "anchor admission must create a real bond to the parent aggregate"
        );
        let body = crate::organism_geometry::OrganismBodyGeometry::from_structure(
            &parent_structure,
            &simulation.environment.catalog,
        )
        .unwrap();
        assert!(child_remains_within_realized_parent_boundary(
            &transient_view,
            &body,
        ));

        let anchor_index = transient_view
            .unit_index(construction.anchor_physical_id.unwrap())
            .unwrap();
        let anchor_unit = &transient_view.units[anchor_index];
        let anchor_part = crate::material_geometry::PlacedMaterialPart {
            part_index: anchor_index,
            form: anchor_unit
                .shape(&simulation.environment.catalog)
                .expect("anchor has a catalog shape")
                .form
                .clone(),
            placement: anchor_unit.placement,
        };
        assert!(
            body.parts.iter().any(|part| {
                let parent_part = crate::material_geometry::PlacedMaterialPart {
                    part_index: part.unit_index,
                    form: part.form.clone(),
                    placement: crate::structure::Placement {
                        x: part.x,
                        y: part.y,
                        rotation_radians: part.rotation_radians,
                    },
                };
                crate::material_geometry::placed_forms_boundary_contact(
                    &parent_part,
                    &anchor_part,
                    0.0,
                )
            }),
            "anchor must contact the parent at a physical boundary"
        );
        assert!(
            body.parts.iter().all(|part| {
                let parent_part = crate::material_geometry::PlacedMaterialPart {
                    part_index: part.unit_index,
                    form: part.form.clone(),
                    placement: crate::structure::Placement {
                        x: part.x,
                        y: part.y,
                        rotation_radians: part.rotation_radians,
                    },
                };
                !crate::material_geometry::placed_forms_penetrate(&parent_part, &anchor_part, 0.0)
            }),
            "anchor must not interpenetrate any parent constituent"
        );
    }

    #[test]
    fn aggregate_owned_construction_keeps_energy_in_parent_holder() {
        let mut simulation = Simulation::new(11, 20.0);
        let mut parent = simulation.organisms.remove(0);
        let mut ledger = EnergyLedger::default();
        assert!(parent.store_material(Material::free_base("Carbon", 1.0)));
        assert!(begin_reproduction(
            &mut parent,
            &mut simulation.rng,
            &simulation.environment.catalog,
            &mut ledger,
        ));
        let mut construction = parent.reproductive_construction.take().unwrap();
        let before_parent_energy = parent.usable_energy;
        assert!(construction.developing_structure.units.is_empty());
        let environment = simulation.environment.clone();
        let body = parent_body_geometry(&parent, &environment.catalog).unwrap();
        let _ = advance_construction(
            &mut parent.structure,
            &mut parent.stored_material,
            &mut construction,
            &environment,
            &mut ledger,
            &mut parent.usable_energy,
            &mut simulation.rng,
            &body,
            None,
        );
        assert!(construction.is_aggregate_owned_by(&parent.structure));
        assert_eq!(construction.developing_energy, 0.0);
        assert_eq!(
            parent.usable_energy, before_parent_energy,
            "an attached developing view must not settle a second maintenance/chemistry debit"
        );
        assert_eq!(
            total_usable_energy_held(&[parent.clone()], &[]),
            parent.usable_energy
        );
    }

    #[test]
    fn invalid_parent_energy_fails_closed_without_mutating_construction() {
        let mut simulation = Simulation::new(912, 20.0);
        let mut parent = simulation.organisms.remove(0);
        let mut ledger = EnergyLedger::default();
        assert!(parent.store_material(Material::free_base("Carbon", 1.0)));
        assert!(begin_reproduction(
            &mut parent,
            &mut simulation.rng,
            &simulation.environment.catalog,
            &mut ledger,
        ));
        let mut construction = parent.reproductive_construction.take().unwrap();
        let original_energy = construction.developing_energy;
        let original_structure = format!("{:?}", construction.developing_structure);
        let original_stress = construction.developing_stress;
        let original_debt = construction.developing_maintenance_debt;
        let original_storage = parent.stored_material.clone();
        // Invalid parent energy must fail closed before any reserve transfer or
        // construction mutation. The persisted developing state remains attached
        // so the invalid holder can be diagnosed rather than partially advanced.
        parent.usable_energy = f64::INFINITY;
        let environment = simulation.environment.clone();
        let body = parent_body_geometry(&parent, &environment.catalog).unwrap();

        let (status, _) = advance_construction(
            &mut parent.structure,
            &mut parent.stored_material,
            &mut construction,
            &environment,
            &mut ledger,
            &mut parent.usable_energy,
            &mut simulation.rng,
            &body,
            None,
        );

        assert_eq!(status, ConstructionStatus::Dead);
        assert_eq!(parent.usable_energy, f64::INFINITY);
        assert_eq!(construction.developing_energy, original_energy);
        assert_eq!(
            format!("{:?}", construction.developing_structure),
            original_structure
        );
        assert_eq!(construction.developing_stress, original_stress);
        assert_eq!(construction.developing_maintenance_debt, original_debt);
        assert_eq!(parent.stored_material.entries, original_storage.entries);
    }

    #[test]
    fn rejected_parent_material_attachment_keeps_source_and_developing_graph_atomic() {
        use crate::material_storage::StoredMaterial;
        use crate::physical_material::PhysicalMaterial;
        use crate::structure::{Placement, StructuralUnit};

        let catalog = default_catalog();
        let mut original_unit = StructuralUnit::new(
            "Carbon",
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        );
        assert!(original_unit.realize_default_geometry(&catalog));
        let mut original_structure = OrganismStructure::new();
        original_structure.add_unit(original_unit);

        let mut construction = ReproductiveConstruction {
            committed_material: MaterialStorage::default(),
            developing_structure: original_structure.clone(),
            developing_member_ids: vec![],
            child_genome: initial_genome(),
            developmental_origin: Position { x: 0.0, y: 0.0 },
            developmental_orientation_radians: 0.0,
            developing_stress: 0.0,
            anchor_unit_index: 0,
            anchor_physical_id: None,
            developing_energy: 10.0,
            developing_maintenance_debt: 0.0,
            needs_space: false,
        };
        let source = PhysicalMaterial::realized(
            Material::free_base("Carbon", 1.0),
            vec![Placement {
                x: 100.0,
                y: 100.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .expect("parent-held source must be a valid physical constituent");
        let mut parent_storage = MaterialStorage::default();
        parent_storage
            .entries
            .push(StoredMaterial::Physical(source));
        let stored_before = parent_storage.entries.clone();

        let mut child =
            developing_organism(&construction, construction.developing_structure.clone());
        let mut candidate_unit = StructuralUnit::new(
            "Carbon",
            Placement {
                x: 2.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        );
        assert!(candidate_unit.realize_default_geometry(&catalog));
        child.structure.add_unit(candidate_unit);

        let mut parent_structure = OrganismStructure::new();
        let rejecting_body = crate::organism_geometry::OrganismBodyGeometry {
            parts: Vec::new(),
            min_x: 0.0,
            max_x: 0.0,
            min_y: 0.0,
            max_y: 0.0,
        };
        let environment = Environment {
            width: 1000.0,
            height: 1000.0,
            catalog: catalog.clone(),
            field: crate::environment::ActiveMaterialField::new(
                1000.0,
                1000.0,
                crate::environment::DEFAULT_CELL_SIZE,
            ),
        };
        let mut ledger = EnergyLedger::default();
        let mut parent_energy = 20.0;
        let ledger_before = ledger;
        let mut candidate_ledger = ledger;
        candidate_ledger.total_heat_dissipated = 9.0;
        let units_before = construction.developing_structure.units.clone();

        let status = commit_candidate_attachment(
            &mut parent_structure,
            &mut parent_storage,
            &mut construction,
            &environment,
            &mut ledger,
            &mut parent_energy,
            &rejecting_body,
            child,
            candidate_ledger,
            Some(0),
            units_before.len(),
            None,
        );

        assert_eq!(status, ConstructionStatus::Detached);
        assert_eq!(parent_storage.entries, stored_before);
        assert_eq!(construction.developing_structure.units, units_before);
        assert_eq!(ledger, ledger_before);
    }

    #[test]
    fn crossing_parent_boundary_does_not_detach_while_contact_remains() {
        let mut simulation = Simulation::new(17, 20.0);
        let mut parent = simulation.organisms.remove(0);
        let mut ledger = EnergyLedger::default();
        assert!(parent.store_material(Material::free_base("Carbon", 1.0)));
        assert!(begin_reproduction(
            &mut parent,
            &mut simulation.rng,
            &simulation.environment.catalog,
            &mut ledger,
        ));
        let body = parent_body_geometry(&parent, &simulation.environment.catalog).unwrap();
        let construction = parent
            .reproductive_construction
            .as_ref()
            .expect("reproduction is active");
        assert!(construction.developing_structure.units.is_empty());
        let member_ids = construction.developing_member_ids.clone();
        let anchor_id = construction.anchor_physical_id.unwrap();
        let inside_part = body.parts[0].clone();
        let outside_x = body.max_x + 100.0;
        let anchor_index = parent.structure.unit_index(anchor_id).unwrap();
        let member_indices = member_ids
            .iter()
            .filter_map(|id| parent.structure.unit_index(*id))
            .collect::<Vec<_>>();
        for index in member_indices {
            let unit = &mut parent.structure.units[index];
            if index == anchor_index {
                unit.placement.x = inside_part.x;
                unit.placement.y = inside_part.y;
            } else {
                unit.placement.x = outside_x;
                unit.placement.y = inside_part.y;
            }
        }
        let transient_view = parent
            .structure
            .subgraph_for_physical_ids(&member_ids, &[anchor_id])
            .unwrap();
        let inside = child_remains_within_realized_parent_boundary(&transient_view, &body);
        assert!(
            inside,
            "crossing the boundary while contact remains must not detach the child"
        );
    }

    #[test]
    fn fully_outside_parent_boundary_is_detached() {
        let mut simulation = Simulation::new(23, 20.0);
        let mut parent = simulation.organisms.remove(0);
        let mut ledger = EnergyLedger::default();
        assert!(parent.store_material(Material::free_base("Carbon", 1.0)));
        assert!(begin_reproduction(
            &mut parent,
            &mut simulation.rng,
            &simulation.environment.catalog,
            &mut ledger,
        ));
        let body = parent_body_geometry(&parent, &simulation.environment.catalog).unwrap();
        let construction = parent
            .reproductive_construction
            .as_ref()
            .expect("reproduction is active");
        let member_ids = construction.developing_member_ids.clone();
        let anchor_id = construction.anchor_physical_id.unwrap();
        let member_indices = member_ids
            .iter()
            .filter_map(|id| parent.structure.unit_index(*id))
            .collect::<Vec<_>>();
        for index in member_indices {
            let unit = &mut parent.structure.units[index];
            unit.placement.x = body.max_x + 100.0;
            unit.placement.y = body.max_y + 100.0;
        }
        let transient_view = parent
            .structure
            .subgraph_for_physical_ids(&member_ids, &[anchor_id])
            .unwrap();
        assert!(!child_remains_within_realized_parent_boundary(
            &transient_view,
            &body
        ));
    }

    #[test]
    fn aggregate_owned_member_pose_is_read_from_parent_graph() {
        let mut simulation = Simulation::new(29, 20.0);
        let mut parent = simulation.organisms.remove(0);
        let mut ledger = EnergyLedger::default();
        assert!(parent.store_material(Material::free_base("Carbon", 1.0)));
        assert!(begin_reproduction(
            &mut parent,
            &mut simulation.rng,
            &simulation.environment.catalog,
            &mut ledger,
        ));
        let construction = parent
            .reproductive_construction
            .as_ref()
            .expect("reproduction is active");
        let member_ids = construction.developing_member_ids.clone();
        let anchor_id = construction.anchor_physical_id.unwrap();
        let outside = Position {
            x: parent.occupied_cells[0].x + 2_000.0,
            y: parent.occupied_cells[0].y,
        };
        let member_indices = member_ids
            .iter()
            .filter_map(|id| parent.structure.unit_index(*id))
            .collect::<Vec<_>>();
        for index in member_indices {
            let unit = &mut parent.structure.units[index];
            unit.placement.x = outside.x;
            unit.placement.y = outside.y;
        }
        let transient_view = parent
            .structure
            .subgraph_for_physical_ids(&member_ids, &[anchor_id])
            .unwrap();
        assert_eq!(
            transient_view.units[transient_view.unit_index(anchor_id).unwrap()]
                .placement
                .x,
            outside.x,
            "the transient view must track authoritative aggregate geometry"
        );
        assert!(construction.developing_structure.units.is_empty());
    }

    #[test]
    fn construction_waits_when_all_physical_inventory_is_exhausted() {
        let mut simulation = Simulation::new(31, 20.0);
        let mut parent = simulation.organisms.remove(0);
        let mut ledger = EnergyLedger::default();

        // Isolate this inventory-exhaustion scenario from the constructor's
        // evolving seed stock: this test supplies exactly one available object.
        parent.stored_material.entries.clear();
        assert!(parent.store_material(Material::free_base("Carbon", 1.0)));
        assert!(begin_reproduction(
            &mut parent,
            &mut simulation.rng,
            &simulation.environment.catalog,
            &mut ledger,
        ));
        assert!(parent.stored_material.is_empty());

        let mut construction = parent
            .reproductive_construction
            .take()
            .expect("reproduction is active");
        let anchor_id = construction.anchor_physical_id.unwrap();
        let mut transient_view = parent
            .structure
            .subgraph_for_physical_ids(&construction.developing_member_ids, &[anchor_id])
            .unwrap();
        assert!(developing_membership_is_valid(
            &construction,
            &transient_view
        ));
        assert!(construction.developing_structure.units.is_empty());
        assert!(construction.is_aggregate_owned_by(&parent.structure));
        assert_eq!(construction.developing_energy, 0.0);
        let before_units = transient_view.units.len();
        let body = parent_body_geometry(&parent, &simulation.environment.catalog).unwrap();
        let environment = simulation.environment.clone();

        let status = advance_construction(
            &mut parent.structure,
            &mut parent.stored_material,
            &mut construction,
            &environment,
            &mut ledger,
            &mut parent.usable_energy,
            &mut simulation.rng,
            &body,
            None,
        )
        .0;

        assert_eq!(status, ConstructionStatus::Waiting);
        transient_view = parent
            .structure
            .subgraph_for_physical_ids(&construction.developing_member_ids, &[anchor_id])
            .unwrap();
        assert_eq!(transient_view.units.len(), before_units);
        assert!(construction.developing_structure.units.is_empty());
        assert!(parent.stored_material.is_empty());
    }

    #[test]
    fn waiting_construction_resumes_from_same_graph_when_material_arrives() {
        let mut simulation = Simulation::new(37, 20.0);
        let mut parent = simulation.organisms.remove(0);
        let mut ledger = EnergyLedger::default();

        assert!(parent.store_material(Material::free_base("Carbon", 1.0)));
        assert!(begin_reproduction(
            &mut parent,
            &mut simulation.rng,
            &simulation.environment.catalog,
            &mut ledger,
        ));

        let mut construction = parent
            .reproductive_construction
            .take()
            .expect("reproduction is active");
        let anchor_id = construction.anchor_physical_id.unwrap();
        let transient_view = parent
            .structure
            .subgraph_for_physical_ids(&construction.developing_member_ids, &[anchor_id])
            .unwrap();
        assert!(developing_membership_is_valid(
            &construction,
            &transient_view
        ));
        assert!(construction.developing_structure.units.is_empty());
        assert!(construction.is_aggregate_owned_by(&parent.structure));
        assert_eq!(construction.developing_energy, 0.0);
        let body = parent_body_geometry(&parent, &simulation.environment.catalog).unwrap();
        let environment = simulation.environment.clone();

        let first_status = advance_construction(
            &mut parent.structure,
            &mut parent.stored_material,
            &mut construction,
            &environment,
            &mut ledger,
            &mut parent.usable_energy,
            &mut simulation.rng,
            &body,
            None,
        )
        .0;
        assert_eq!(first_status, ConstructionStatus::Waiting);

        let before_view = parent
            .structure
            .subgraph_for_physical_ids(&construction.developing_member_ids, &[anchor_id])
            .unwrap();
        let before_units = before_view.units.len();
        let before_bonds = before_view.bonds.len();

        let developing_child = developing_organism(&construction, before_view);
        let newly_available_resource = construction_preferred_resource(&developing_child)
            .expect("the developing genome has a preferred construction resource")
            .to_owned();
        assert!(parent.store_material(Material::free_base(&newly_available_resource, 1.0)));

        let second_status = advance_construction(
            &mut parent.structure,
            &mut parent.stored_material,
            &mut construction,
            &environment,
            &mut ledger,
            &mut parent.usable_energy,
            &mut simulation.rng,
            &body,
            None,
        )
        .0;

        assert!(
            matches!(
                second_status,
                ConstructionStatus::Progress
                    | ConstructionStatus::Ready
                    | ConstructionStatus::Detached
            ),
            "new physical material must resume construction, got {second_status:?}"
        );
        let refreshed_view = parent
            .structure
            .subgraph_for_physical_ids(&construction.developing_member_ids, &[anchor_id])
            .unwrap();
        assert!(refreshed_view.units.len() > before_units);
        assert!(refreshed_view.bonds.len() > before_bonds);
        assert!(construction.developing_structure.units.is_empty());
    }

    #[test]
    fn non_ready_construction_cannot_create_an_offspring() {
        let mut simulation = Simulation::new(41, 20.0);
        let mut parent = simulation.organisms.remove(0);
        let mut ledger = EnergyLedger::default();
        assert!(parent.store_material(Material::free_base("Carbon", 1.0)));
        assert!(begin_reproduction(
            &mut parent,
            &mut simulation.rng,
            &simulation.environment.catalog,
            &mut ledger,
        ));
        let original_units = parent
            .reproductive_construction
            .as_ref()
            .unwrap()
            .developing_member_ids
            .len();
        assert!(original_units > 0);

        let child = finish_reproduction(
            &mut parent,
            "should-not-exist".to_owned(),
            &simulation.environment.catalog,
            &mut ledger,
            None,
        );

        assert!(child.is_none());
        let retained = parent
            .reproductive_construction
            .as_ref()
            .expect("non-ready construction remains parent-owned");
        assert!(retained.developing_structure.units.is_empty());
        assert_eq!(retained.developing_member_ids.len(), original_units);
    }

    #[test]
    fn shared_aggregate_ownership_predicate_ignores_stale_cached_pose() {
        use crate::structure::{Placement, StructuralUnit};

        let catalog = default_catalog();
        let mut unit = StructuralUnit::new(
            "Carbon",
            Placement {
                x: 1.0,
                y: 2.0,
                rotation_radians: 0.0,
            },
        );
        assert!(unit.realize_default_geometry(&catalog));
        let mut aggregate = OrganismStructure::new();
        aggregate.add_unit(unit);
        let construction = ReproductiveConstruction {
            committed_material: MaterialStorage::default(),
            developing_structure: OrganismStructure::new(),
            developing_member_ids: aggregate.physical_constituent_ids(),
            child_genome: initial_genome(),
            developmental_origin: Position { x: 1.0, y: 2.0 },
            developmental_orientation_radians: 0.0,
            developing_stress: 0.0,
            anchor_unit_index: 0,
            anchor_physical_id: aggregate.physical_id(0),
            developing_energy: 0.0,
            developing_maintenance_debt: 0.0,
            needs_space: false,
        };

        assert!(construction.is_aggregate_owned_by(&aggregate));
        assert!(
            construction.developing_structure.units.is_empty(),
            "aggregate-owned construction must not persist a duplicate graph"
        );
        assert!(
            construction.is_aggregate_owned_by(&aggregate),
            "the ownership predicate is based on stable IDs in the aggregate"
        );

        let mut missing_member = aggregate.clone();
        missing_member.units.clear();
        assert!(
            !construction.is_aggregate_owned_by(&missing_member),
            "stable IDs absent from the authoritative aggregate are not aggregate-owned"
        );
    }

    #[test]
    fn aggregate_owned_construction_with_stale_energy_invalidates_inventory() {
        let mut simulation = Simulation::new(947, 20.0);
        let mut parent = simulation.organisms.remove(0);
        parent.usable_energy = 17.0;

        let aggregate = parent.structure.clone();
        let member_ids = aggregate.physical_constituent_ids();
        parent.reproductive_construction = Some(ReproductiveConstruction {
            committed_material: MaterialStorage::default(),
            developing_structure: OrganismStructure::new(),
            developing_member_ids: member_ids,
            child_genome: parent.genome.clone(),
            developmental_origin: parent.developmental_origin.clone(),
            developmental_orientation_radians: parent.developmental_orientation_radians,
            developing_stress: 0.0,
            anchor_unit_index: 0,
            anchor_physical_id: parent.structure.physical_id(0),
            developing_energy: 11.0,
            developing_maintenance_debt: 0.0,
            needs_space: false,
        });

        let held = total_usable_energy_held(&[parent], &[]);
        assert!(
            held.is_nan(),
            "a stale second energy balance must invalidate inventory instead of being silently ignored"
        );
    }

    #[test]
    fn anchor_is_not_a_predefined_structural_blueprint() {
        let catalog = default_catalog();
        let genome = initial_genome();
        let anchor = Material::free_base("Carbon", 1.0);
        let placement = crate::structure::Placement {
            x: 0.0,
            y: 0.0,
            rotation_radians: 0.0,
        };
        let mut storage = MaterialStorage::default();
        assert!(storage.store_physical(anchor, vec![placement], &catalog));
        assert!(anchor_structure(&genome, storage, placement, &catalog).is_some());
    }
}
