//! Physical reproduction lifecycle.
//!
//! Reproduction owns a separate developing physical graph. The child begins
//! from one transferred anchor resource, then uses the normal COMBINE/runtime
//! path with the inherited developmental fields guiding valid opportunities.
use crate::combine_runtime::DevelopmentalContext;
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

fn developing_organism(construction: &ReproductiveConstruction) -> Organism {
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
        stored_material: construction.committed_material.clone(),
        active_transformation_id: None,
        active_movement: None,
        reproductive_construction: None,
        structure: construction.developing_structure.clone(),
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
    match crate::construction_material_selection::select_construction_material(
        &construction.committed_material,
        preferred,
        catalog,
    ) {
        Ok(crate::construction_material_selection::ConstructionMaterialDecision::Need {
            ..
        }) => 1.0,
        Ok(crate::construction_material_selection::ConstructionMaterialDecision::Selected {
            ..
        }) => 0.0,
        Err(_) => 1.0,
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
            .map(|decision| {
                matches!(
                    decision,
                    crate::construction_material_selection::ConstructionMaterialDecision::Selected { .. }
                )
            })
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
    developmental_linear_extent(
        &construction.developing_structure,
        &construction.developmental_origin,
    ) + 1e-9
        >= 0.80 * preferred
}

fn developing_membership_is_valid(construction: &ReproductiveConstruction) -> bool {
    let actual = construction.developing_structure.physical_constituent_ids();
    if actual.len() != construction.developing_member_ids.len() {
        return false;
    }
    let actual_ids: std::collections::HashSet<_> = actual.iter().copied().collect();
    let recorded_ids: std::collections::HashSet<_> =
        construction.developing_member_ids.iter().copied().collect();
    actual_ids.len() == actual.len()
        && recorded_ids.len() == construction.developing_member_ids.len()
        && actual_ids == recorded_ids
}

fn birth_ready_with_reference(
    construction: &ReproductiveConstruction,
    catalog: &[crate::resources::BaseResource],
    seed_reference: Option<(f64, f64)>,
) -> bool {
    if !developing_membership_is_valid(construction) {
        return false;
    }
    let Some(anchor_id) = construction
        .developing_structure
        .physical_id(construction.anchor_unit_index)
    else {
        return false;
    };
    let Ok(cavity) = crate::cavity::analyze_genome_cavity_for_core(
        &construction.developing_structure,
        catalog,
        &[anchor_id],
    ) else {
        return false;
    };
    let Some(cavity) = cavity else {
        return false;
    };
    if !cavity
        .boundary_units
        .contains(&construction.anchor_unit_index)
    {
        return false;
    }
    if validate_realized_organism_for_core(
        &construction.developing_structure,
        catalog,
        OrganismViabilityRequirements::default(),
        &[anchor_id],
    )
    .is_err()
    {
        return false;
    }
    budding_scale_reached_with_reference(construction, catalog, seed_reference)
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
pub(crate) fn total_usable_energy_held(
    organisms: &[Organism],
    decomposing_bodies: &[crate::decomposition::DecomposingBody],
) -> f64 {
    let organism_energy: f64 = organisms
        .iter()
        .map(|organism| {
            organism.usable_energy
                + organism
                    .reproductive_construction
                    .as_ref()
                    .map_or(0.0, |construction| construction.developing_energy)
        })
        .sum();
    let decomposition_energy: f64 = decomposing_bodies
        .iter()
        .map(|body| body.energy_budget)
        .sum();
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
    _ledger: &mut EnergyLedger,
) -> bool {
    // Construction consumes a parent-held physical anchor. Do not mutate
    // that holder while a multi-tick BREAK/COMBINE transaction owns an escrow
    // index or a prepared storage snapshot for the same organism.
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

    // No resource type is reserved as a reproductive anchor. The first
    // offspring core is selected from whatever parent-held material can
    // actually be instantiated by the physical construction runtime.
    // Structured logical material remains intact; an already-realized
    // structured object may be used directly.
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

        // The anchor becomes part of the developing physical graph immediately.
        // Give the constructor the exact physical instance selected from the parent;
        // instantiate_one_unit consumes it through the normal material-restoration
        // path, so the child does not retain a duplicate inventory copy.
        let mut trial_storage = MaterialStorage::default();
        trial_storage
            .entries
            .push(crate::material_storage::StoredMaterial::Physical(
                instance.clone(),
            ));
        let Some((structure, child_storage, anchor_unit_index)) =
            anchor_structure(&child_genome, trial_storage, placement, catalog)
        else {
            continue;
        };
        let developing_member_ids = structure.physical_constituent_ids();

        let mut parent_trial = parent.stored_material.clone();
        if parent_trial.take_physical_at(storage_index).is_none() {
            continue;
        }

        parent.stored_material = parent_trial;
        parent.reproductive_construction = Some(ReproductiveConstruction {
            committed_material: child_storage,
            developing_structure: structure,
            developing_member_ids,
            child_genome: child_genome.clone(),
            developmental_origin: Position {
                x: placement.x,
                y: placement.y,
            },
            developmental_orientation_radians: 0.0,
            developing_stress: 0.0,
            anchor_unit_index,
            developing_energy: 0.0,
            developing_maintenance_debt: 0.0,
            needs_space: false,
        });
        return true;
    }
    false
}

fn commit_candidate_attachment(
    parent_structure: &OrganismStructure,
    parent_storage: &mut MaterialStorage,
    construction: &mut ReproductiveConstruction,
    environment: &Environment,
    ledger: &mut EnergyLedger,
    parent_body: &crate::organism_geometry::OrganismBodyGeometry,
    child: Organism,
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
    if let Some(parent_index) = transferred {
        if parent_storage.take_physical_at(parent_index).is_none() {
            return ConstructionStatus::Waiting;
        }
    }
    *ledger = candidate_ledger;
    construction.committed_material = child.stored_material;
    construction.developing_member_ids = child.structure.physical_constituent_ids();
    construction.developing_structure = child.structure;
    construction.needs_space = false;
    construction.developing_energy = child.usable_energy;
    construction.developing_stress = child.stress;

    if birth_ready_with_reference(construction, &environment.catalog, seed_reference) {
        ConstructionStatus::Ready
    } else if candidate_units > before_units {
        ConstructionStatus::Progress
    } else {
        ConstructionStatus::Waiting
    }
}

pub(crate) fn advance_construction(
    parent_structure: &OrganismStructure,
    parent_storage: &mut MaterialStorage,
    construction: &mut ReproductiveConstruction,
    environment: &Environment,
    ledger: &mut EnergyLedger,
    parent_energy: &mut f64,
    _rng: &mut ChaCha8Rng,
    parent_body: &crate::organism_geometry::OrganismBodyGeometry,
    seed_reference: Option<(f64, f64)>,
) -> (ConstructionStatus, Option<f64>) {
    // Validate the persisted developing holder before any new parent-funded
    // reserve transfer. A non-finite construction must not drain its parent on
    // every subsequent tick while it remains attached for diagnosis.
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

    if reserve_energy > construction.developing_energy {
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
    let mut child = developing_organism(construction);
    child.apply_maintenance(&environment.catalog, ledger);
    let dead = child.validate_stress_state();
    if !dead {
        let _ = crate::transformation::resolve_resonance_overload_break(
            &mut child,
            environment,
            ledger,
        );
    }
    construction.developing_energy = child.usable_energy;
    construction.developing_stress = child.stress;
    construction.developing_maintenance_debt = child.maintenance_debt;
    construction.developing_structure = child.structure.clone();
    construction.developing_member_ids = construction.developing_structure.physical_constituent_ids();
    if dead {
        return (ConstructionStatus::Dead, None);
    }

    if birth_ready_with_reference(construction, &environment.catalog, seed_reference) {
        return (ConstructionStatus::Ready, None);
    }

    if !parent_child_in_contact(parent_structure, &construction.developing_structure) {
        return (ConstructionStatus::Detached, None);
    }

    let genome_qualified = crate::cavity::analyze_genome_cavity(
        &construction.developing_structure,
        &environment.catalog,
    )
    .ok()
    .flatten()
    .is_some_and(|cavity| {
        cavity
            .boundary_units
            .contains(&construction.anchor_unit_index)
    });
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
    _ledger: &mut EnergyLedger,
    seed_reference: Option<(f64, f64)>,
) -> Option<Organism> {
    let construction = parent.reproductive_construction.take()?;
    let ready = birth_ready_with_reference(&construction, catalog, seed_reference);
    // Construction may only produce an offspring after the physical genome is
    // validated and ready. Detached/dead/non-ready states are not a successful
    // division transaction; retain the parent-owned construction unchanged.
    if !ready {
        parent.reproductive_construction = Some(construction);
        return None;
    }
    let child_position = construction.developmental_origin.clone();
    Some(Organism {
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
        usable_energy: construction.developing_energy,
        stress: construction.developing_stress,
        maintenance_debt: construction.developing_maintenance_debt,
        stress_threshold: crate::state::INITIAL_STRESS_THRESHOLD,
        stored_material: construction.committed_material,
        genome_core_identity_established: false,
        active_transformation_id: None,
        active_movement: None,
        reproductive_construction: None,
        structure: construction.developing_structure,
        structure_revision: 0,
        position_revision: 0,
        cached_cavity_revision: None,
        cached_cavity: None,
        cached_developmental_revision: None,
        cached_developmental_realization: None,
        peak_developmental_realization: 0.0,
        cached_harmonic_key: None,
        last_movement_attempt: None,
    })
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
            developing_energy: 0.0,
            developing_maintenance_debt: 0.0,
            needs_space: false,
        };
        let parent_structure = OrganismStructure::new();
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
            &parent_structure,
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
            developing_energy: f64::NAN,
            developing_maintenance_debt: 0.0,
            needs_space: false,
        };
        let parent_structure = OrganismStructure::new();
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
            &parent_structure,
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
            developing_energy: 0.0,
            developing_maintenance_debt: 0.0,
            needs_space: false,
        };
        assert!(preferred > 0.0);
        assert!(!budding_scale_reached_with_reference(
            &construction,
            &catalog,
            None
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
        assert!(developing_membership_is_valid(&construction));

        construction.developing_member_ids.clear();
        assert!(!developing_membership_is_valid(&construction));

        construction.developing_member_ids =
            construction.developing_structure.physical_constituent_ids();
        let first_id = construction.developing_member_ids[0];
        construction.developing_member_ids.push(first_id);
        assert!(!developing_membership_is_valid(&construction));
    }

    #[test]
    fn anchor_starts_a_separate_physical_child_graph() {
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
        assert!(!construction.developing_structure.units.is_empty());
        assert_eq!(
            construction.developing_member_ids,
            construction.developing_structure.physical_constituent_ids(),
            "construction membership must be tracked by stable physical IDs"
        );
        assert_eq!(
            construction.developing_member_ids.iter().collect::<std::collections::HashSet<_>>().len(),
            construction.developing_member_ids.len(),
            "a physical constituent may only belong to the developing member once"
        );
        assert!(construction.anchor_unit_index < construction.developing_structure.units.len());
        assert_eq!(
            format!("{:?}", parent.structure),
            format!("{:?}", parent_structure)
        );
        let body = parent_body_geometry(&parent, &simulation.environment.catalog).unwrap();
        assert!(child_remains_within_realized_parent_boundary(
            &construction.developing_structure,
            &body,
        ));

        let anchor_unit = &construction.developing_structure.units[construction.anchor_unit_index];
        let anchor_part = crate::material_geometry::PlacedMaterialPart {
            part_index: construction.anchor_unit_index,
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
    fn developing_offspring_receives_persistent_energy_from_parent() {
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
        let environment = simulation.environment.clone();
        let body = parent_body_geometry(&parent, &environment.catalog).unwrap();
        let _ = advance_construction(
            &parent.structure,
            &mut parent.stored_material,
            &mut construction,
            &environment,
            &mut ledger,
            &mut parent.usable_energy,
            &mut simulation.rng,
            &body,
            None,
        );
        assert!(construction.developing_energy > 0.0);
        assert!(parent.usable_energy < before_parent_energy);
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
            &parent.structure,
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

        let mut child = developing_organism(&construction);
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

        let parent_structure = OrganismStructure::new();
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
        let ledger_before = ledger;
        let mut candidate_ledger = ledger;
        candidate_ledger.total_heat_dissipated = 9.0;
        let units_before = construction.developing_structure.units.clone();

        let status = commit_candidate_attachment(
            &parent_structure,
            &mut parent_storage,
            &mut construction,
            &environment,
            &mut ledger,
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
            .as_mut()
            .expect("reproduction is active");
        let inside_part = body.parts[0].clone();
        let outside_x = body.max_x + 100.0;
        for (index, unit) in construction
            .developing_structure
            .units
            .iter_mut()
            .enumerate()
        {
            if index == 0 {
                unit.placement.x = inside_part.x;
                unit.placement.y = inside_part.y;
            } else {
                unit.placement.x = outside_x;
                unit.placement.y = inside_part.y;
            }
        }
        let inside = child_remains_within_realized_parent_boundary(
            &construction.developing_structure,
            &body,
        );
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
            .as_mut()
            .expect("reproduction is active");
        for unit in &mut construction.developing_structure.units {
            unit.placement.x = body.max_x + 100.0;
            unit.placement.y = body.max_y + 100.0;
        }
        assert!(!child_remains_within_realized_parent_boundary(
            &construction.developing_structure,
            &body,
        ));
    }

    #[test]
    fn loss_of_parent_contact_is_detachment() {
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
            .as_mut()
            .expect("reproduction is active");
        for unit in &mut construction.developing_structure.units {
            unit.placement.x = parent.occupied_cells[0].x + 2_000.0;
            unit.placement.y = parent.occupied_cells[0].y;
        }
        assert!(!parent_child_in_contact(
            &parent.structure,
            &construction.developing_structure,
        ));
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
        let before_units = construction.developing_structure.units.len();
        let body = parent_body_geometry(&parent, &simulation.environment.catalog).unwrap();
        let environment = simulation.environment.clone();

        let status = advance_construction(
            &parent.structure,
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
        assert_eq!(construction.developing_structure.units.len(), before_units);
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
        let body = parent_body_geometry(&parent, &simulation.environment.catalog).unwrap();
        let environment = simulation.environment.clone();

        let first_status = advance_construction(
            &parent.structure,
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

        let before_units = construction.developing_structure.units.len();
        let before_bonds = construction.developing_structure.bonds.len();

        assert!(parent.store_material(Material::free_base("Carbon", 1.0)));

        let second_status = advance_construction(
            &parent.structure,
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
        assert!(construction.developing_structure.units.len() > before_units);
        assert!(construction.developing_structure.bonds.len() > before_bonds);
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
            .developing_structure
            .units
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
        assert_eq!(retained.developing_structure.units.len(), original_units);
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
