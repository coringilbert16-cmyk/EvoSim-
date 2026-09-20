//! Physical reproduction lifecycle.
//!
//! Reproduction owns a separate developing physical graph. The child begins
//! from one transferred anchor resource, then uses the normal COMBINE/runtime
//! path with the inherited developmental fields guiding valid opportunities.
use crate::combine_runtime::DevelopmentalContext;
use crate::energy_ledger::EnergyLedgerAuthority;
use crate::juvenile_requirements::{validate_realized_juvenile, JuvenileViabilityRequirements};
use crate::material_storage::MaterialStorage;
use crate::resources::Material;
use crate::state::{
    DevelopmentStage, EnergyLedger, Environment, Organism, Position, ReproductiveConstruction,
    ResourceSense,
};
use crate::structure::OrganismStructure;
use rand_chacha::ChaCha8Rng;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ConstructionStatus {
    Waiting,
    Progress,
    Ready,
    Detached,
    Dead,
    DeadEnd,
}

pub(crate) fn parent_boundary(
    parent: &Organism,
    catalog: &[crate::resources::BaseResource],
) -> Option<(Position, f64)> {
    let center = parent.occupied_cells.first()?.clone();
    let radius =
        crate::organism_geometry::OrganismBodyGeometry::from_structure(&parent.structure, catalog)
            .map(|g| g.bounding_radius_about(center.x, center.y))
            .unwrap_or(0.0)
            .max(0.0);
    (radius.is_finite() && radius > 0.0).then_some((center, radius))
}

fn parent_child_position(
    parent: &Organism,
    anchor: &Material,
    catalog: &[crate::resources::BaseResource],
) -> Option<Position> {
    let (center, radius) = parent_boundary(parent, catalog)?;
    let anchor_name = anchor.parts.first()?.0.as_str();
    let anchor_radius = catalog
        .iter()
        .find(|r| r.name == anchor_name)
        .map(|r| r.shape.form.bounding_radius())
        .unwrap_or(1.0)
        .max(0.0);
    (anchor_radius <= radius + 1e-9).then_some(center)
}

fn child_remains_inside_parent_boundary(
    structure: &OrganismStructure,
    center: &Position,
    radius: f64,
) -> bool {
    if !radius.is_finite() || radius <= 0.0 {
        return false;
    }
    let boundary = crate::material_geometry::PlacedMaterialPart {
        part_index: 0,
        form: crate::resources::Form::Circle { radius },
        placement: crate::structure::Placement {
            x: center.x,
            y: center.y,
            rotation_radians: 0.0,
        },
    };
    structure.units.iter().any(|unit| {
        let Some(geometry) = unit.geometry.as_ref() else {
            return false;
        };
        let child = crate::material_geometry::PlacedMaterialPart {
            part_index: 0,
            form: geometry.shape().form.clone(),
            placement: unit.placement,
        };
        crate::material_geometry::placed_forms_penetrate(&child, &boundary, 0.0)
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
        resource_sense: ResourceSense {
            sensed_resources: Vec::new(),
            direction_x: 0.0,
            direction_y: 0.0,
            direction_strength: 0.0,
        },
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
    anchor: Material,
    position: Position,
    catalog: &[crate::resources::BaseResource],
) -> Option<(OrganismStructure, MaterialStorage, usize)> {
    let mut storage = MaterialStorage::default();
    if !storage.store(anchor) {
        return None;
    }
    let mut child = Organism {
        id: "developing-offspring".into(),
        developmental_origin: position.clone(),
        developmental_orientation_radians: 0.0,
        occupied_cells: vec![position],
        genome: child_genome.clone(),
        resource_sense: ResourceSense {
            sensed_resources: Vec::new(),
            direction_x: 0.0,
            direction_y: 0.0,
            direction_strength: 0.0,
        },
        memory: Vec::new(),
        decision_history: crate::decision::DecisionHistory::default(),
        usable_energy: 0.0,
        stress: 0.0,
        stress_threshold: crate::state::INITIAL_STRESS_THRESHOLD,
        stored_material: storage,
        development_stage: DevelopmentStage::Juvenile,
        active_transformation_id: None,
        reproductive_construction: None,
        structure: OrganismStructure::new(),
    };
    let anchor_unit_index = crate::combine_runtime::instantiate_one_unit(&mut child, catalog)?;
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

    let snapshot = parent.stored_material.materials_snapshot();
    for anchor in snapshot {
        let Some(position) = parent_child_position(parent, &anchor, catalog) else {
            continue;
        };
        let mut trial_storage = parent.stored_material.clone();
        let Some(transferred) = trial_storage.take_matching(&anchor) else {
            continue;
        };
        let Some((structure, child_storage, anchor_unit_index)) =
            anchor_structure(&child_genome, transferred, position.clone(), catalog)
        else {
            continue;
        };
        parent.stored_material = trial_storage;
        parent.reproductive_construction = Some(ReproductiveConstruction {
            committed_material: child_storage,
            developing_structure: structure,
            child_genome,
            developmental_origin: position,
            developmental_orientation_radians: 0.0,
            developing_stress: 0.0,
            anchor_unit_index,
            developing_energy: 0.0,
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
    parent_boundary: &(Position, f64),
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
    let Some((child, candidate_ledger, transferred)) = try_child_construction(
        &child,
        parent_storage,
        environment,
        ledger,
        context,
    ) else {
        return (ConstructionStatus::Waiting, None);
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
    construction.developing_energy = child.usable_energy;
    construction.developing_stress = child.stress;
    if !child_remains_inside_parent_boundary(
        &construction.developing_structure,
        &parent_boundary.0,
        parent_boundary.1,
    ) {
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
        resource_sense: ResourceSense {
            sensed_resources: Vec::new(),
            direction_x: 0.0,
            direction_y: 0.0,
            direction_strength: 0.0,
        },
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
        let boundary = parent_boundary(&parent, &simulation.environment.catalog).unwrap();
        assert!(structure_within_parent_boundary(
            &construction.developing_structure,
            &boundary.0,
            boundary.1,
        ));
        assert_eq!(
            construction.developmental_origin.x, boundary.0.x,
            "developing offspring must begin inside the parent's boundary"
        );
        assert_eq!(
            construction.developmental_origin.y, boundary.0.y,
            "developing offspring must begin inside the parent's boundary"
        );
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
        let boundary = parent_boundary(&parent, &environment.catalog).unwrap();
        let _ = advance_construction(
            &parent.structure,
            &mut parent.stored_material,
            &mut construction,
            &environment,
            &mut ledger,
            &mut parent.usable_energy,
            &mut simulation.rng,
            &boundary,
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
        let boundary = parent_boundary(&parent, &simulation.environment.catalog).unwrap();
        let construction = parent
            .reproductive_construction
            .as_mut()
            .expect("reproduction is active");
        for unit in &mut construction.developing_structure.units {
            unit.placement.x = boundary.0.x + boundary.1 * 0.75;
            unit.placement.y = boundary.0.y;
        }
        let inside = child_remains_inside_parent_boundary(
            &construction.developing_structure,
            &boundary.0,
            boundary.1,
        );
        assert!(inside, "crossing the boundary while material remains inside must not detach the child");
    }

    #[test]
    fn anchor_is_not_a_predefined_structural_blueprint() {
        let catalog = default_catalog();
        let genome = initial_genome();
        let anchor = Material::free_base("Carbon", 1.0);
        let position = Position { x: 0.0, y: 0.0 };
        assert!(anchor_structure(&genome, anchor, position, &catalog).is_some());
    }
}
