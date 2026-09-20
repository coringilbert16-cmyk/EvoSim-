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
    Dead,
}

fn parent_child_position(
    parent: &Organism,
    anchor: &Material,
    catalog: &[crate::resources::BaseResource],
) -> Option<Position> {
    let parent_position = parent.occupied_cells.first()?.clone();
    let parent_radius =
        crate::organism_geometry::OrganismBodyGeometry::from_structure(&parent.structure, catalog)
            .ok()
            .map(|g| g.bounding_radius_about(0.0, 0.0))
            .unwrap_or(1.0)
            .max(0.0);
    let anchor_name = anchor.parts.first()?.0.as_str();
    let anchor_radius = catalog
        .iter()
        .find(|r| r.name == anchor_name)
        .map(|r| r.shape.form.bounding_radius())
        .unwrap_or(1.0)
        .max(0.0);
    Some(Position {
        x: parent_position.x + parent_radius + anchor_radius + 1.0,
        y: parent_position.y,
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

fn preferred_length(
    genome: &crate::genome::Genome,
    catalog: &[crate::resources::BaseResource],
) -> Option<f64> {
    let (seed_mass, seed_length) =
        crate::juvenile::confirmed_seed_scale_reference(catalog).ok()?;
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
        let Some(position) = parent_child_position(parent, &anchor, catalog) else { continue };
        let mut trial_storage = parent.stored_material.clone();
        let Some(transferred) = trial_storage.take_matching(&anchor) else { continue };
        let Some((structure, child_storage)) = anchor_structure(parent, &child_genome, transferred, position.clone(), catalog) else { continue };
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
    parent_storage: &mut MaterialStorage,
    construction: &mut ReproductiveConstruction,
    environment: &Environment,
    ledger: &mut EnergyLedger,
    parent_energy: &mut f64,
    rng: &mut ChaCha8Rng,
) -> (ConstructionStatus, Option<f64>) {
    let reserve_energy = construction.child_genome.juvenile_energy_reserve;
    if reserve_energy.is_finite() && reserve_energy > construction.developing_energy {
        let transfer =
            (reserve_energy - construction.developing_energy).min(parent_energy.max(0.0));
        if transfer > 0.0 {
            let _ = ledger.transfer(
                parent_energy,
                &mut construction.developing_energy,
                transfer,
            );
        }
    }

    let mut child = developing_organism(construction);
    child.apply_maintenance(&environment.catalog, ledger);
    let dead = child.apply_stress_damage(environment, ledger, rng);
    construction.developing_energy = child.usable_energy;
    construction.developing_stress = child.stress;
    construction.developing_structure = child.structure;
    if dead {
        return (ConstructionStatus::Dead, None);
    }

    if birth_ready(construction, &environment.catalog) {
        return (ConstructionStatus::Ready, None);
    }

    if child.stored_material.is_empty() {
        if !store_first_available_material(parent_storage, &mut child.stored_material) {
            return (ConstructionStatus::Waiting, None);
        }
    }

    let genome_qualified = crate::cavity::analyze_genome_cavity(
        &construction.developing_structure,
        &environment.catalog,
    )
    .ok()
    .flatten()
    .is_some_and(|cavity| cavity.boundary_units.contains(&construction.anchor_unit_index));
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
    let mut cache = crate::contact::ConnectionCompatibilityCache::new();
    let before_units = child.structure.units.len();
    let before_storage = child.stored_material.clone();
    let result = crate::combine_runtime::try_combine_stored_unit(
        &mut child,
        environment,
        &mut cache,
        ledger,
        Some(context),
    );
    if result.is_some() {
        construction.committed_material = child.stored_material;
        construction.developing_structure = child.structure;
        construction.developing_energy = child.usable_energy;
        construction.developing_stress = child.stress;
        if birth_ready(construction, &environment.catalog) {
            return (ConstructionStatus::Ready, None);
        }
        return (
            ConstructionStatus::Progress,
            result.map(|attempt| attempt.work_cost),
        );
    }

    // Preserve material if the COMBINE attempt was not physically valid. If
    // other parent material is available, the next tick can try a different
    // material. A blocked structure with existing bonds remains alive because
    // BREAK/reorganization is still a valid future operation; no false dead
    // end is declared here.
    construction.committed_material = before_storage;
    if before_units == 0 {
        (ConstructionStatus::Dead, None)
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

    #[test]
    fn developmental_scale_uses_approved_forty_percent_linear_target() {
        let catalog = default_catalog();
        let genome = initial_genome();
        let origin = Position { x: 0.0, y: 0.0 };
        let preferred = preferred_length(&genome, &catalog).unwrap();
        let mut construction = ReproductiveConstruction {
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
        construction.developing_structure = OrganismStructure::new();
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
    fn child_genome_is_mutated_before_construction() {
        let parent = initial_genome();
        let child = parent.clone();
        assert!(child.developmental_blueprint.validate().is_ok());
        assert!(child.juvenile_energy_reserve.is_finite());
    }

    #[test]
    fn anchor_is_not_a_predefined_structural_blueprint() {
        let catalog = default_catalog();
        let genome = initial_genome();
        let anchor = Material::free_base("Carbon", 1.0);
        let position = Position { x: 0.0, y: 0.0 };
        let result = anchor_structure(&genome, anchor, position, &catalog);
        assert!(result.is_some());
    }
}
