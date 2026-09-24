//! Physical reproduction lifecycle.
//!
//! Reproduction owns a separate developing physical graph. The child begins
//! from one transferred anchor resource, then uses the normal COMBINE/runtime
//! path with the inherited developmental fields guiding valid opportunities.
use crate::combine_runtime::DevelopmentalContext;
use crate::energy_ledger::EnergyLedgerAuthority;
use crate::juvenile_requirements::{validate_realized_juvenile, JuvenileViabilityRequirements};
use crate::material_storage::{MaterialStorage, StoredMaterial};
use crate::resources::Material;
use crate::state::{
    DevelopmentStage, EnergyLedger, Environment, Organism, Position, ReproductiveConstruction,
};
use crate::structure::OrganismStructure;
use rand_chacha::ChaCha8Rng;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[expect(
    dead_code,
    reason = "DeadEnd is reserved for the approved P7 terminal construction condition"
)]
pub(crate) enum ConstructionStatus {
    Waiting,
    Progress,
    Ready,
    Detached,
    Dead,
    DeadEnd,
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
            if crate::material_geometry::placed_forms_overlap(&parent_part_form, &anchor_part, 0.0)
            {
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
        memory: Vec::new(),
        decision_history: crate::decision::DecisionHistory::default(),
        usable_energy: construction.developing_energy,
        stress: construction.developing_stress,
        stress_threshold: crate::state::INITIAL_STRESS_THRESHOLD,
        stored_material: construction.committed_material.clone(),
        development_stage: DevelopmentStage::Juvenile,
        active_transformation_id: None,
        reproductive_construction: None,
        structure: construction.developing_structure.clone(),
        structure_revision: 0,
        position_revision: 0,
        cached_cavity_revision: None,
        cached_cavity: None,
        cached_developmental_revision: None,
        cached_developmental_realization: None,
        cached_harmonic_key: None,
    }
}

fn store_first_available_material(
    parent_storage: &mut MaterialStorage,
    child_storage: &mut MaterialStorage,
) -> bool {
    let Some(material) = parent_storage.peek_one_unstructured() else {
        return false;
    };
    let Some(material) = parent_storage.take_matching(&material) else {
        return false;
    };
    child_storage.store(material)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NextConstructionResourceStatus {
    Available,
    Missing,
    Impossible,
}

fn next_construction_resource_status(
    child: &Organism,
    parent_storage: &MaterialStorage,
    environment: &Environment,
    ledger: &EnergyLedger,
    context: Option<DevelopmentalContext<'_>>,
) -> NextConstructionResourceStatus {
    let mut missing = false;

    for resource in &environment.catalog {
        let material = Material::free_base(resource.name.clone(), 1.0);

        let already_held = child
            .stored_material
            .materials_snapshot()
            .iter()
            .any(|held| held == &material)
            || parent_storage
                .materials_snapshot()
                .iter()
                .any(|held| held == &material);

        let mut candidate = child.clone();
        if !candidate.stored_material.store(material.clone()) {
            continue;
        }

        if try_child_construction(
            &candidate,
            &MaterialStorage::default(),
            environment,
            ledger,
            context,
        )
        .is_some()
        {
            if already_held {
                return NextConstructionResourceStatus::Available;
            }
            missing = true;
        }
    }

    if missing {
        NextConstructionResourceStatus::Missing
    } else {
        NextConstructionResourceStatus::Impossible
    }
}

fn try_child_construction(
    child: &Organism,
    parent_storage: &MaterialStorage,
    environment: &Environment,
    ledger: &EnergyLedger,
    context: Option<DevelopmentalContext<'_>>,
) -> Option<(Organism, EnergyLedger, Option<Material>)> {
    let mut candidates = Vec::new();

    for index in 0..child.stored_material.entries.len() {
        let mut candidate = child.clone();
        candidate.stored_material.entries.swap(0, index);
        candidates.push((candidate, None));
    }

    for material in parent_storage.materials_snapshot() {
        let mut candidate = child.clone();
        if !candidate.stored_material.store(material.clone()) {
            continue;
        }
        let last = candidate.stored_material.entries.len().saturating_sub(1);
        candidate.stored_material.entries.swap(0, last);
        candidates.push((candidate, Some(material)));
    }

    for (mut candidate, transferred) in candidates {
        let mut candidate_ledger = *ledger;
        let mut cache = crate::contact::ConnectionCompatibilityCache::new();
        if crate::combine_runtime::try_combine_stored_unit(
            &mut candidate,
            environment,
            &mut cache,
            &mut candidate_ledger,
            context,
        )
        .is_some()
        {
            return Some((candidate, candidate_ledger, transferred));
        }
    }
    None
}

fn preferred_length(
    genome: &crate::genome::Genome,
    catalog: &[crate::resources::BaseResource],
) -> Option<f64> {
    let (seed_mass, seed_length) = crate::juvenile::confirmed_seed_scale_reference(catalog).ok()?;
    Some(
        genome
            .developmental_blueprint
            .preferred_developmental_length(genome.adult_mass(), seed_mass, seed_length),
    )
}

fn developmental_context<'a>(
    genome: &'a crate::genome::Genome,
    origin: &Position,
    orientation: f64,
    catalog: &[crate::resources::BaseResource],
) -> Option<DevelopmentalContext<'a>> {
    Some((
        &genome.developmental_blueprint,
        (origin.x, origin.y),
        orientation,
        preferred_length(genome, catalog)?,
    ))
}

fn developmental_linear_extent(structure: &OrganismStructure, origin: &Position) -> f64 {
    structure
        .units
        .iter()
        .map(|unit| (unit.placement.x - origin.x).hypot(unit.placement.y - origin.y))
        .fold(0.0, f64::max)
}

fn juvenile_scale_reached(
    construction: &ReproductiveConstruction,
    catalog: &[crate::resources::BaseResource],
) -> bool {
    let Some(preferred) = preferred_length(&construction.child_genome, catalog) else {
        return false;
    };
    if preferred <= 0.0 || !preferred.is_finite() {
        return false;
    }
    developmental_linear_extent(
        &construction.developing_structure,
        &construction.developmental_origin,
    ) + 1e-9
        >= 0.40 * preferred
}

fn birth_ready(
    construction: &ReproductiveConstruction,
    catalog: &[crate::resources::BaseResource],
) -> bool {
    let Ok(cavity) =
        crate::cavity::analyze_genome_cavity(&construction.developing_structure, catalog)
    else {
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
    if validate_realized_juvenile(
        &construction.developing_structure,
        catalog,
        JuvenileViabilityRequirements::default(),
    )
    .is_err()
    {
        return false;
    }
    juvenile_scale_reached(construction, catalog)
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
        memory: Vec::new(),
        decision_history: crate::decision::DecisionHistory::default(),
        usable_energy: 0.0,
        stress: 0.0,
        stress_threshold: crate::state::INITIAL_STRESS_THRESHOLD,
        stored_material: anchor_storage,
        development_stage: DevelopmentStage::Juvenile,
        active_transformation_id: None,
        reproductive_construction: None,
        structure: OrganismStructure::new(),
        structure_revision: 0,
        position_revision: 0,
        cached_cavity_revision: None,
        cached_cavity: None,
        cached_developmental_revision: None,
        cached_harmonic_key: None,
    };
    let anchor_unit_index = crate::combine_runtime::instantiate_one_unit(&mut child, catalog)?;
    if let Some(anchor_unit) = child.structure.units.get_mut(anchor_unit_index) {
        anchor_unit.placement.rotation_radians = placement.rotation_radians;
    }
    Some((child.structure, child.stored_material, anchor_unit_index))
}
pub(crate) fn total_usable_energy_held(organisms: &[Organism]) -> f64 {
    organisms
        .iter()
        .map(|organism| {
            organism.usable_energy
                + organism
                    .reproductive_construction
                    .as_ref()
                    .map_or(0.0, |construction| construction.developing_energy)
        })
        .sum()
}

pub(crate) fn begin_reproduction(
    parent: &mut Organism,
    rng: &mut ChaCha8Rng,
    catalog: &[crate::resources::BaseResource],
    _ledger: &mut EnergyLedger,
) -> bool {
    if !matches!(parent.development_stage, DevelopmentStage::Adult)
        || parent.reproductive_construction.is_some()
    {
        return false;
    }
    let mut child_genome = parent.genome.clone();
    child_genome.mutate(rng);
    if child_genome.developmental_blueprint.validate().is_err()
        || !child_genome.juvenile_energy_reserve.is_finite()
        || child_genome.juvenile_energy_reserve <= 0.0
    {
        return false;
    }

    // No resource type is reserved as a reproductive anchor. The first
    // offspring core is selected from whatever parent-held material can
    // actually be instantiated by the physical construction runtime.
    // Structured logical material remains intact; an already-realized
    // structured object may be used directly.
    for entry in parent.stored_material.entries.clone() {
        let anchor = match &entry {
            StoredMaterial::Logical(material) if !material.has_internal_structure() => material.clone(),
            StoredMaterial::Physical(instance) => instance.material.clone(),
            StoredMaterial::Logical(_) => continue,
        };
        let Some(placement) = parent_child_position(parent, &anchor, catalog) else {
            continue;
        };

        let mut trial_storage = MaterialStorage::default();
        trial_storage.entries.push(entry.clone());
        let Some((structure, child_storage, anchor_unit_index)) =
            anchor_structure(&child_genome, trial_storage, placement, catalog)
        else {
            continue;
        };

        let mut parent_trial = parent.stored_material.clone();
        let removed = match &entry {
            StoredMaterial::Logical(material) => parent_trial.take_matching(material).is_some(),
            StoredMaterial::Physical(instance) => parent_trial
                .take_matching_physical(&instance.material)
                .is_some(),
        };
        if !removed {
            continue;
        }

        parent.stored_material = parent_trial;
        parent.reproductive_construction = Some(ReproductiveConstruction {
            committed_material: child_storage,
            developing_structure: structure,
            child_genome: child_genome.clone(),
            developmental_origin: Position {
                x: placement.x,
                y: placement.y,
            },
            developmental_orientation_radians: 0.0,
            developing_stress: 0.0,
            anchor_unit_index,
            developing_energy: 0.0,
            needs_space: false,
        });
        return true;
    }
    false
}

pub(crate) fn advance_construction(
    parent_structure: &OrganismStructure,
    parent_storage: &mut MaterialStorage,
    construction: &mut ReproductiveConstruction,
    environment: &Environment,
    ledger: &mut EnergyLedger,
    parent_energy: &mut f64,
    rng: &mut ChaCha8Rng,
    parent_body: &crate::organism_geometry::OrganismBodyGeometry,
) -> (ConstructionStatus, Option<f64>) {
    let reserve_energy = construction.child_genome.juvenile_energy_reserve;
    if reserve_energy.is_finite() && reserve_energy > construction.developing_energy {
        let transfer =
            (reserve_energy - construction.developing_energy).min(parent_energy.max(0.0));
        if transfer > 0.0 {
            let _ = ledger.transfer(parent_energy, &mut construction.developing_energy, transfer);
        }
    }

    let mut child = developing_organism(construction);
    child.apply_maintenance(&environment.catalog, ledger);
    let dead = child.apply_stress_damage(environment, ledger, rng);
    construction.developing_energy = child.usable_energy;
    construction.developing_stress = child.stress;
    construction.developing_structure = child.structure.clone();
    if dead {
        return (ConstructionStatus::Dead, None);
    }

    if birth_ready(construction, &environment.catalog) {
        return (ConstructionStatus::Ready, None);
    }

    if !parent_child_in_contact(parent_structure, &construction.developing_structure) {
        return (ConstructionStatus::Detached, None);
    }

    if child.stored_material.is_empty()
        && !store_first_available_material(parent_storage, &mut child.stored_material)
    {
        return (ConstructionStatus::Waiting, None);
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
            NextConstructionResourceStatus::Impossible => {
                construction.needs_space = false;
                return (ConstructionStatus::DeadEnd, None);
            }
        }
    };

    if let Some(material) = transferred {
        let mut parent_trial = parent_storage.clone();
        if parent_trial.take_matching(&material).is_none() {
            return (ConstructionStatus::Waiting, None);
        }
        *parent_storage = parent_trial;
    }
    *ledger = candidate_ledger;
    construction.committed_material = child.stored_material;
    construction.developing_structure = child.structure;
    construction.needs_space = false;
    construction.developing_energy = child.usable_energy;
    construction.developing_stress = child.stress;
    if !child_intersects_realized_parent_region(&construction.developing_structure, parent_body) {
        return (ConstructionStatus::Detached, None);
    }
    if !parent_child_in_contact(parent_structure, &construction.developing_structure) {
        return (ConstructionStatus::Detached, None);
    }
    if birth_ready(construction, &environment.catalog) {
        return (ConstructionStatus::Ready, None);
    }
    if construction.developing_structure.units.len() > before_units {
        (ConstructionStatus::Progress, None)
    } else {
        (ConstructionStatus::Waiting, None)
    }
}

pub(crate) fn finish_reproduction(
    parent: &mut Organism,
    child_id: String,
    catalog: &[crate::resources::BaseResource],
    _ledger: &mut EnergyLedger,
) -> Option<Organism> {
    let construction = parent.reproductive_construction.take()?;
    let ready = birth_ready(&construction, catalog);
    if !ready && construction.developing_structure.units.is_empty() {
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
        memory: Vec::new(),
        decision_history: crate::decision::DecisionHistory::default(),
        usable_energy: construction.developing_energy,
        stress: construction.developing_stress,
        stress_threshold: crate::state::INITIAL_STRESS_THRESHOLD,
        stored_material: construction.committed_material,
        development_stage: DevelopmentStage::Juvenile,
        active_transformation_id: None,
        reproductive_construction: None,
        structure: construction.developing_structure,
        structure_revision: 0,
        position_revision: 0,
        cached_cavity_revision: None,
        cached_cavity: None,
        cached_developmental_revision: None,
        cached_developmental_realization: None,
        cached_harmonic_key: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genome::initial_genome;
    use crate::resources::{default_catalog, Material};
    use crate::state::Simulation;

    #[test]
    fn developmental_scale_uses_approved_forty_percent_linear_target() {
        let catalog = default_catalog();
        let genome = initial_genome();
        let origin = Position { x: 0.0, y: 0.0 };
        let preferred = preferred_length(&genome, &catalog).unwrap();
        let construction = ReproductiveConstruction {
            committed_material: MaterialStorage::default(),
            developing_structure: OrganismStructure::new(),
            child_genome: genome,
            developmental_origin: origin,
            developmental_orientation_radians: 0.0,
            developing_stress: 0.0,
            anchor_unit_index: 0,
            developing_energy: 0.0,
            needs_space: false,
        };
        assert!(preferred > 0.0);
        assert!(!juvenile_scale_reached(&construction, &catalog));
    }

    #[test]
    fn reserve_requirement_is_genome_defined() {
        let mut storage = MaterialStorage::default();
        let genome = initial_genome();
        assert!(storage.store(genome.juvenile_reserve.clone()));
        assert!(storage.take_matching(&genome.juvenile_reserve).is_some());
        assert!(genome.juvenile_energy_reserve > 0.0);
    }

    #[test]
    fn anchor_starts_a_separate_physical_child_graph() {
        let mut simulation = Simulation::new(7, 20.0);
        let mut parent = simulation.organisms.remove(0);
        parent.development_stage = DevelopmentStage::Adult;
        let parent_structure = parent.structure.clone();
        let mut ledger = EnergyLedger::default();
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
    }

    #[test]
    fn developing_offspring_receives_persistent_energy_from_parent() {
        let mut simulation = Simulation::new(11, 20.0);
        let mut parent = simulation.organisms.remove(0);
        parent.development_stage = DevelopmentStage::Adult;
        let mut ledger = EnergyLedger::default();
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
        );
        assert!(construction.developing_energy > 0.0);
        assert!(parent.usable_energy < before_parent_energy);
    }

    #[test]
    fn crossing_parent_boundary_is_detachment_not_rejection() {
        let mut simulation = Simulation::new(17, 20.0);
        let mut parent = simulation.organisms.remove(0);
        parent.development_stage = DevelopmentStage::Adult;
        let mut ledger = EnergyLedger::default();
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
            "crossing the boundary while material remains inside must not detach the child"
        );
    }

    #[test]
    fn fully_outside_parent_boundary_is_detached() {
        let mut simulation = Simulation::new(23, 20.0);
        let mut parent = simulation.organisms.remove(0);
        parent.development_stage = DevelopmentStage::Adult;
        let mut ledger = EnergyLedger::default();
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
        parent.development_stage = DevelopmentStage::Adult;
        let mut ledger = EnergyLedger::default();
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
        assert!(storage.store(anchor));
        assert!(anchor_structure(&genome, storage, placement, &catalog).is_some());
    }
}
