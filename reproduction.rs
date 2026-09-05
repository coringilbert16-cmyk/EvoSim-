//! Physical reproduction lifecycle.
//!
//! Reproduction does not split or copy the parent's existing structure. Once
//! an adult has accumulated enough actual unbonded material, the reproductive
//! decision commits that material to a persistent construction state. The
//! parent keeps its own structure intact. Later ticks will consume the
//! committed material through the ordinary structural construction system and
//! only create a separate organism when the developing structure reaches the
//! genetically determined juvenile threshold.

use rand_chacha::ChaCha8Rng;

use crate::resources::{BaseResource, ConnectionSites};
use crate::state::{DevelopmentStage, Organism, ReproductiveConstruction};
use crate::structure::{OrganismStructure, Placement};

const CORE_UNIT_COUNT: usize = 6;
const CORE_MATERIAL_AMOUNT: f64 = CORE_UNIT_COUNT as f64;

pub(crate) fn begin_reproduction(parent: &mut Organism, rng: &mut ChaCha8Rng) -> bool {
    if !matches!(parent.development_stage, DevelopmentStage::Adult) {
        return false;
    }
    if parent.reproductive_readiness < 1.0 - f64::EPSILON {
        return false;
    }
    if parent.reproductive_construction.is_some() {
        return false;
    }
    if parent.stored_unbonded.total_amount() + f64::EPSILON < CORE_MATERIAL_AMOUNT {
        return false;
    }

    let Some(committed_material) = parent.stored_unbonded.take(CORE_MATERIAL_AMOUNT) else {
        return false;
    };

    let mut child_genome = parent.genome.clone();
    child_genome.mutate(rng);

    parent.reproductive_readiness = 0.0;
    parent.reproductive_construction = Some(ReproductiveConstruction {
        committed_material,
        developing_structure: OrganismStructure::new(),
        child_genome,
    });
    true
}

/// Advance reproductive construction by exactly one physical structural unit.
///
/// Placement is derived from the existing connection-point geometry. The new
/// unit is positioned so one of its authored connection points exactly meets
/// an available connection point on the developing structure with opposing
/// outward normals. Bond formation remains a separate COMBINE step.
pub(crate) fn advance_construction(
    construction: &mut ReproductiveConstruction,
    catalog: &[BaseResource],
) -> bool {
    let Some((resource_name, _)) = construction
        .committed_material
        .parts
        .iter()
        .find(|(_, amount)| *amount >= 1.0 - f64::EPSILON)
        .map(|(name, amount)| (name.clone(), *amount))
    else {
        return false;
    };

    let Some(placement) = construction_placement(
        &construction.developing_structure,
        &resource_name,
        catalog,
        &construction.child_genome,
    ) else {
        return false;
    };

    if crate::structural_combine::instantiate_raw_unit(
        &mut construction.developing_structure,
        &mut construction.committed_material,
        &resource_name,
        placement,
        catalog,
    )
    .is_err()
    {
        return false;
    }

    true
}

fn construction_placement(
    structure: &OrganismStructure,
    resource_name: &str,
    catalog: &[BaseResource],
    genome: &crate::genome::Genome,
) -> Option<Placement> {
    if structure.units.is_empty() {
        return Some(Placement {
            x: 0.0,
            y: 0.0,
            rotation_radians: 0.0,
        });
    }

    let new_resource = catalog.iter().find(|base| base.name == resource_name)?;
    let ConnectionSites::Corners(new_points) = new_resource.shape.connection_sites() else {
        return None;
    };

    let compactness = genome.construction_compactness();
    let branching = genome.construction_branching();
    let centroid = structure_centroid(structure);

    let mut best: Option<(f64, Placement)> = None;
    for (unit_index, unit) in structure.units.iter().enumerate() {
        let ConnectionSites::Corners(existing_points) = unit.connection_sites(catalog)? else {
            continue;
        };
        for (existing_index, &existing_point) in existing_points.iter().enumerate() {
            if structure.connection_count(unit_index, existing_index) != 0 {
                continue;
            }

            let existing_world = crate::contact::world_connection_point(existing_point, unit);
            let existing_normal_angle = existing_world.normal_y.atan2(existing_world.normal_x);

            for (new_index, &new_point) in new_points.iter().enumerate() {
                let rotation = existing_normal_angle
                    + std::f64::consts::PI
                    - new_point.direction_radians;
                let (s, c) = rotation.sin_cos();
                let rotated_x = new_point.x * c - new_point.y * s;
                let rotated_y = new_point.x * s + new_point.y * c;
                let placement = Placement {
                    x: existing_world.x - rotated_x,
                    y: existing_world.y - rotated_y,
                    rotation_radians: rotation,
                };

                let dx = placement.x - centroid.0;
                let dy = placement.y - centroid.1;
                let distance_from_centroid = dx.hypot(dy);
                let radial_angle = dy.atan2(dx);
                let preferred_angle = (new_index as f64 + branching) * std::f64::consts::FRAC_PI_2;
                let angular_delta = angular_distance(radial_angle, preferred_angle);
                let score = distance_from_centroid * (1.0 + compactness)
                    + angular_delta * (1.0 + branching);

                if best.as_ref().map(|(current, _)| score < *current).unwrap_or(true) {
                    best = Some((score, placement));
                }
            }
        }
    }

    best.map(|(_, placement)| placement)
}

fn structure_centroid(structure: &OrganismStructure) -> (f64, f64) {
    let count = structure.units.len() as f64;
    let x = structure.units.iter().map(|u| u.placement.x).sum::<f64>() / count;
    let y = structure.units.iter().map(|u| u.placement.y).sum::<f64>() / count;
    (x, y)
}

fn angular_distance(a: f64, b: f64) -> f64 {
    let delta = (a - b).rem_euclid(std::f64::consts::TAU);
    delta.min(std::f64::consts::TAU - delta)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use crate::decision::DecisionHistory;
    use crate::genome::initial_genome;
    use crate::resources::Material;
    use crate::state::{Position, ResourceSense};

    fn adult_parent(material_amount: f64) -> Organism {
        Organism {
            id: "parent".into(),
            occupied_cells: vec![Position { x: 50.0, y: 50.0 }],
            genome: initial_genome(),
            resource_sense: ResourceSense { sensed_resources: Vec::new(), direction_x: 0.0, direction_y: 0.0, direction_strength: 0.0 },
            memory: Vec::new(),
            decision_history: DecisionHistory::default(),
            usable_energy: 10.0,
            stress: 0.0,
            stored_unbonded: Material::free_base("Carbon", material_amount),
            structure: OrganismStructure::new(),
            development_stage: DevelopmentStage::Adult,
            age: 10,
            reproductive_readiness: 1.0,
            active_transformation_id: None,
            reproductive_construction: None,
        }
    }

    #[test]
    fn reproduction_commits_real_material_without_touching_parent_structure() {
        let mut parent = adult_parent(CORE_MATERIAL_AMOUNT + 2.0);
        let mut rng = ChaCha8Rng::seed_from_u64(7);
        assert!(begin_reproduction(&mut parent, &mut rng));
        assert!(parent.structure.units.is_empty());
        assert!(parent.structure.bonds.is_empty());
        assert_eq!(parent.stored_unbonded.total_amount(), 2.0);
        assert_eq!(parent.reproductive_readiness, 0.0);
        let construction = parent.reproductive_construction.as_ref().unwrap();
        assert_eq!(construction.committed_material.total_amount(), CORE_UNIT_COUNT as f64);
        assert!(!construction.committed_material.has_internal_structure());
        assert!(construction.developing_structure.units.is_empty());
    }

    #[test]
    fn construction_consumes_one_real_unit_per_step() {
        let catalog = crate::resources::default_catalog();
        let mut parent = adult_parent(CORE_MATERIAL_AMOUNT);
        let mut rng = ChaCha8Rng::seed_from_u64(7);
        assert!(begin_reproduction(&mut parent, &mut rng));
        let construction = parent.reproductive_construction.as_mut().unwrap();
        assert!(advance_construction(construction, &catalog));
        assert_eq!(construction.developing_structure.units.len(), 1);
        assert_eq!(construction.committed_material.total_amount(), 5.0);
        assert_eq!(parent.structure.units.len(), 0);
        assert!(advance_construction(construction, &catalog));
        assert_eq!(construction.developing_structure.units.len(), 2);
        assert_eq!(construction.committed_material.total_amount(), 4.0);
    }

    #[test]
    fn construction_places_new_unit_at_real_contact_geometry() {
        let catalog = crate::resources::default_catalog();
        let mut parent = adult_parent(CORE_MATERIAL_AMOUNT);
        let mut rng = ChaCha8Rng::seed_from_u64(7);
        assert!(begin_reproduction(&mut parent, &mut rng));
        let construction = parent.reproductive_construction.as_mut().unwrap();
        assert!(advance_construction(construction, &catalog));
        assert!(advance_construction(construction, &catalog));

        let candidates = crate::contact::contacting_connection_pair_candidates(
            &construction.developing_structure,
            0,
            1,
            &catalog,
            1.0,
            0.0,
        );
        assert!(!candidates.is_empty());
        assert!(candidates.iter().any(|candidate| candidate.distance <= 1e-12));
        assert!(candidates.iter().any(|candidate| candidate.facing >= 1.0 - 1e-12));
    }

    #[test]
    fn construction_stops_when_no_full_unit_remains() {
        let catalog = crate::resources::default_catalog();
        let mut parent = adult_parent(CORE_MATERIAL_AMOUNT);
        let mut rng = ChaCha8Rng::seed_from_u64(7);
        assert!(begin_reproduction(&mut parent, &mut rng));
        let construction = parent.reproductive_construction.as_mut().unwrap();
        for _ in 0..CORE_UNIT_COUNT {
            assert!(advance_construction(construction, &catalog));
        }
        assert!(!advance_construction(construction, &catalog));
        assert!(construction.committed_material.is_empty());
        assert_eq!(construction.developing_structure.units.len(), CORE_UNIT_COUNT);
    }
}
