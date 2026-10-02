//! Blueprint-free search for the first physically valid organism.
//!
//! The initial organism has no developmental blueprint, target topology,
//! shape, material recipe, or construction plan. It starts from one physical
//! unit and grows only through the same physical attachment transaction used
//! by runtime construction. The first realized state satisfying the organism
//! validity contract is accepted.

use crate::resources::{BaseResource, Material, PhysicalState};
use crate::state::EnergyLedger;
use crate::structure::Placement;

const SEARCH_ENERGY: f64 = 1.0e12;
const ACQUISITION_SAMPLES: usize = 48;
const SURFACE_CONTACT_TOLERANCE: f64 = 1.0e-8;

#[derive(Clone, Debug)]
pub(crate) struct ValidConstruction {
    pub structure: crate::structure::OrganismStructure,
    pub energy: f64,
    pub acquired_resource_placements: Vec<(String, Placement)>,
}

fn placement_fits_resource(
    resource: &BaseResource,
    region: &crate::interior_geometry::EnclosedRegion,
    catalog: &[BaseResource],
) -> Option<Placement> {
    let (min_x, max_x, min_y, max_y) = region.boundary.iter().fold(
        (
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
        ),
        |(min_x, max_x, min_y, max_y), &(x, y)| {
            (min_x.min(x), max_x.max(x), min_y.min(y), max_y.max(y))
        },
    );

    if !min_x.is_finite() || !max_x.is_finite() || !min_y.is_finite() || !max_y.is_finite() {
        return None;
    }

    // Try the region's derived interior sample first. This is an exact
    // physical containment test, so it is only a fast path; the complete
    // bounded search below remains available when the sample point is not a
    // valid placement for the resource.
    for rotation_index in 0..4 {
        let placement = Placement {
            x: region.sample_point.0,
            y: region.sample_point.1,
            rotation_radians: rotation_index as f64 * std::f64::consts::FRAC_PI_2,
        };
        let Some(physical) = crate::physical_material::PhysicalMaterial::realized(
            Material::free_base(resource.name.clone(), 1.0),
            vec![placement],
            catalog,
        ) else {
            continue;
        };
        if crate::environment::ActiveMaterialField::physical_is_fully_inside_any_region(
            &physical,
            std::slice::from_ref(region),
            catalog,
        ) {
            return Some(placement);
        }
    }

    for ix in 0..=ACQUISITION_SAMPLES {
        let x = min_x + (max_x - min_x) * ix as f64 / ACQUISITION_SAMPLES as f64;
        for iy in 0..=ACQUISITION_SAMPLES {
            let y = min_y + (max_y - min_y) * iy as f64 / ACQUISITION_SAMPLES as f64;
            for rotation_index in 0..4 {
                let placement = Placement {
                    x,
                    y,
                    rotation_radians: rotation_index as f64 * std::f64::consts::FRAC_PI_2,
                };
                let Some(physical) = crate::physical_material::PhysicalMaterial::realized(
                    Material::free_base(resource.name.clone(), 1.0),
                    vec![placement],
                    catalog,
                ) else {
                    continue;
                };
                if crate::environment::ActiveMaterialField::physical_is_fully_inside_any_region(
                    &physical,
                    std::slice::from_ref(region),
                    catalog,
                ) {
                    return Some(placement);
                }
            }
        }
    }
    None
}

fn available_acquisition_resources<'a>(catalog: &'a [BaseResource]) -> Vec<&'a BaseResource> {
    catalog
        .iter()
        .filter(|resource| resource.name != "Water")
        .filter(|resource| resource.shape.is_valid())
        .collect()
}

fn find_acquisition_set(
    candidates: &[&BaseResource],
    regions: &[crate::interior_geometry::EnclosedRegion],
    catalog: &[BaseResource],
) -> Option<Vec<(String, Placement)>> {
    for first in 0..candidates.len() {
        for second in first + 1..candidates.len() {
            for third in second + 1..candidates.len() {
                let selected = [candidates[first], candidates[second], candidates[third]];
                let mut placements = Vec::with_capacity(3);
                let mut fits = true;
                for resource in selected {
                    let Some(placement) = regions
                        .iter()
                        .find_map(|region| placement_fits_resource(resource, region, catalog))
                    else {
                        fits = false;
                        break;
                    };
                    placements.push((resource.name.clone(), placement));
                }
                if fits {
                    return Some(placements);
                }
            }
        }
    }
    None
}

fn valid_construction(
    structure: &crate::structure::OrganismStructure,
    catalog: &[BaseResource],
    acquisition_candidates: &[&BaseResource],
) -> Option<Vec<(String, Placement)>> {
    // A qualifying genome cavity requires a closed physical loop. Avoid the
    // expensive cavity/interior analysis while the realized structure is still
    // necessarily acyclic.
    if structure.bonds.len() < structure.units.len() {
        return None;
    }

    let cavity = crate::cavity::analyze_genome_cavity(structure, catalog)
        .ok()
        .flatten()?;
    if !cavity.qualifies() || structure.units.len() <= cavity.boundary_units.len() {
        return None;
    }

    let regions = crate::interior_geometry::find_accessible_interior_regions(structure, catalog)
        .ok()
        .unwrap_or_default();
    if regions.is_empty() {
        return None;
    }

    let mut acquired = find_acquisition_set(acquisition_candidates, &regions, catalog)?;
    let water = catalog.iter().find(|resource| resource.name == "Water")?;
    let water_placement = regions
        .iter()
        .find_map(|region| placement_fits_resource(water, region, catalog))?;
    acquired.push((water.name.clone(), water_placement));
    Some(acquired)
}

fn score_local_growth_potential(
    structure: &crate::structure::OrganismStructure,
    trial: &crate::structure::OrganismStructure,
    new_indices: &[usize],
    excluded_existing_index: usize,
    catalog: &[BaseResource],
) -> (usize, f64) {
    let mut nearby = 0usize;
    let mut nearest_distance = f64::INFINITY;

    for &new_index in new_indices {
        let Some(new_unit) = trial.units.get(new_index) else {
            continue;
        };
        let Some(new_shape) = new_unit.shape(catalog) else {
            continue;
        };
        let new_radius = new_shape.form.bounding_radius();

        for (other_index, other) in structure.units.iter().enumerate() {
            // The anchor is already known to be bonded to the new material.
            // Count only additional nearby structure; otherwise every outward
            // chain would score as "future closure" simply because it touches
            // the unit it was just attached to.
            if other_index == excluded_existing_index {
                continue;
            }
            let Some(other_shape) = other.shape(catalog) else {
                continue;
            };
            let distance = (new_unit.placement.x - other.placement.x)
                .hypot(new_unit.placement.y - other.placement.y);
            let contact_distance =
                new_radius + other_shape.form.bounding_radius() + SURFACE_CONTACT_TOLERANCE;
            if distance <= contact_distance {
                nearby = nearby.saturating_add(1).min(2);
                nearest_distance = nearest_distance.min(distance);
                if nearby >= 2 {
                    return (2, nearest_distance);
                }
            }
        }
    }

    (nearby, nearest_distance)
}

fn close_new_physical_contacts(
    mut structure: crate::structure::OrganismStructure,
    new_indices: &[usize],
    catalog: &[BaseResource],
    mut ledger: EnergyLedger,
    mut energy: f64,
) -> (crate::structure::OrganismStructure, EnergyLedger, f64) {
    let mut cache = crate::contact::ConnectionCompatibilityCache::new();

    loop {
        let mut best: Option<(
            f64,
            usize,
            usize,
            crate::contact::ConnectionPairCandidate,
            f64,
        )> = None;

        for &new_index in new_indices {
            if new_index >= structure.units.len() {
                continue;
            }
            for other_index in 0..structure.units.len() {
                if new_index == other_index {
                    continue;
                }

                let Some(id_a) = structure.physical_id(new_index) else {
                    continue;
                };
                let Some(id_b) = structure.physical_id(other_index) else {
                    continue;
                };
                if structure.bonds.iter().any(|bond| {
                    (bond.endpoint_a.constituent_id == id_a
                        && bond.endpoint_b.constituent_id == id_b)
                        || (bond.endpoint_a.constituent_id == id_b
                            && bond.endpoint_b.constituent_id == id_a)
                }) {
                    continue;
                }

                for candidate in crate::contact::connection_pair_candidates_cached(
                    &structure,
                    new_index,
                    other_index,
                    catalog,
                    &mut cache,
                )
                .into_iter()
                .filter(|candidate| {
                    candidate.distance <= SURFACE_CONTACT_TOLERANCE
                        && candidate.available_a
                        && candidate.available_b
                }) {
                    let Some((_, _, _, investment, _)) =
                        crate::combine_runtime::selected_candidate_evaluation(
                            &structure,
                            new_index,
                            other_index,
                            candidate,
                            catalog,
                        )
                    else {
                        continue;
                    };

                    let score = candidate.facing;
                    let replace = best.as_ref().is_none_or(|current| score > current.0);
                    if replace {
                        best = Some((score, new_index, other_index, candidate, investment));
                    }
                }
            }
        }

        let Some((_, unit_a, unit_b, candidate, investment)) = best else {
            break;
        };

        let mut next = structure.clone();
        let mut next_ledger = ledger;
        let mut next_energy = energy;
        let Some(_) = crate::combine_runtime::form_selected_bond(
            &mut next,
            unit_a,
            unit_b,
            candidate,
            investment,
            catalog,
            &mut cache,
            &mut next_ledger,
            &mut next_energy,
        ) else {
            break;
        };

        structure = next;
        ledger = next_ledger;
        energy = next_energy;
    }

    (structure, ledger, energy)
}

fn free_form_search(
    mut structure: crate::structure::OrganismStructure,
    catalog: &[BaseResource],
    acquisition_candidates: &[&BaseResource],
    mut ledger: EnergyLedger,
    mut energy: f64,
    nodes: &mut usize,
) -> Option<ValidConstruction> {
    // Blueprint-free construction is a forward local-growth process. A
    // complete cavity/acquisition validation is only meaningful after a new
    // bond closes a loop: adding a unit with one bond cannot create a new
    // enclosed region. This keeps the expensive cavity and acquisition
    // analysis off the ordinary acyclic growth path.
    loop {
        let rigid_resources = catalog
            .iter()
            .filter(|resource| resource.physical_state == PhysicalState::Rigid)
            .filter(|resource| resource.shape.is_valid())
            .collect::<Vec<_>>();

        let mut best: Option<(
            (usize, usize, f64, f64),
            crate::structure::OrganismStructure,
            Vec<usize>,
            EnergyLedger,
            f64,
        )> = None;

        // The frontier is geometric, not merely chronological. The newest
        // units are always included, together with older units whose bodies are
        // close enough to one of those frontier units that the next physical
        // attachment could plausibly reach them. This is the minimum local
        // information needed to permit real cavity closure without rescanning
        // the whole organism on every step.
        let max_candidate_radius = rigid_resources
            .iter()
            .map(|resource| resource.shape.form.bounding_radius())
            .fold(0.0_f64, f64::max);
        let recent_indices = if structure.units.len() <= 2 {
            (0..structure.units.len()).collect::<Vec<_>>()
        } else {
            (structure.units.len() - 2..structure.units.len()).collect::<Vec<_>>()
        };
        let mut anchor_indices = recent_indices.clone();
        for candidate_index in 0..structure.units.len() {
            if anchor_indices.contains(&candidate_index) {
                continue;
            }
            let Some(candidate_shape) = structure.units[candidate_index].shape(catalog) else {
                continue;
            };
            let candidate_radius = candidate_shape.form.bounding_radius();
            let near_frontier = recent_indices.iter().any(|&recent_index| {
                let recent = &structure.units[recent_index].placement;
                let candidate = &structure.units[candidate_index].placement;
                (recent.x - candidate.x).hypot(recent.y - candidate.y)
                    <= candidate_radius + max_candidate_radius + SURFACE_CONTACT_TOLERANCE
            });
            if near_frontier {
                anchor_indices.push(candidate_index);
            }
        }

        // Candidate materials are immutable catalog descriptions. Build their
        // one-unit physical representations once for this growth step rather
        // than reconstructing and validating the same material for every
        // anchor/material pair.
        let rigid_materials = rigid_resources
            .iter()
            .filter_map(|resource| {
                crate::physical_material::PhysicalMaterial::realized(
                    Material::free_base(resource.name.clone(), 1.0),
                    vec![Placement {
                        x: 0.0,
                        y: 0.0,
                        rotation_radians: 0.0,
                    }],
                    catalog,
                )
                .map(|material| (*resource, material))
            })
            .collect::<Vec<_>>();

        // Candidate generation is deliberately local: an attachment is tried
        // only against an existing frontier or nearby closure-capable unit.
        // No future tree, target topology, or global body-plan search is
        // constructed.
        for anchor_index in anchor_indices {
            for (_resource, material) in &rigid_materials {
                let before_nodes = *nodes;
                let Some((trial, indices, _part, _attempt, trial_ledger, trial_energy)) =
                    crate::construction_runtime::try_attach_physical_material_bond_driven(
                        &structure,
                        anchor_index,
                        &material,
                        catalog,
                        nodes,
                        &ledger,
                        energy,
                    )
                else {
                    continue;
                };
                let local_work = (*nodes).saturating_sub(before_nodes);
                // This is deliberately only a cheap geometric preference.
                // The trial has already passed the real physical bond transaction;
                // we do not need another full contact search just to rank it.
                let (future_contacts, distance) =
                    score_local_growth_potential(&structure, &trial, &indices, anchor_index, catalog);

                let score = (future_contacts, indices.len(), 0.0, -distance);
                let replace = best.as_ref().is_none_or(|current| score > current.0);
                if replace {
                    best = Some((score, trial, indices, trial_ledger, trial_energy));
                }

                // Keep the candidate accounting monotonic, but do not use it as
                // an artificial biological termination condition.
                *nodes = (*nodes).max(before_nodes.saturating_add(local_work));
            }
        }

        let Some((_, trial, indices, trial_ledger, trial_energy)) = best else {
            return None;
        };

        let bonds_before_closure = trial.bonds.len();
        let (closed_structure, closed_ledger, closed_energy) =
            close_new_physical_contacts(trial, &indices, catalog, trial_ledger, trial_energy);
        let loop_was_closed = closed_structure.bonds.len() > bonds_before_closure;

        structure = closed_structure;
        ledger = closed_ledger;
        energy = closed_energy;

        if loop_was_closed {
            if let Some(acquired_resource_placements) =
                valid_construction(&structure, catalog, acquisition_candidates)
            {
                return Some(ValidConstruction {
                    structure,
                    energy,
                    acquired_resource_placements,
                });
            }
        }
    }
}

pub(crate) fn construct_valid(catalog: &[BaseResource]) -> Result<ValidConstruction, String> {
    let structural_candidates = catalog
        .iter()
        .filter(|resource| resource.physical_state == PhysicalState::Rigid)
        .filter(|resource| resource.shape.is_valid())
        .collect::<Vec<_>>();
    if structural_candidates.is_empty() {
        return Err("no rigid resource can seed a physical organism".into());
    }

    let acquisition_candidates = available_acquisition_resources(catalog);
    if acquisition_candidates.len() < 3 {
        return Err("catalog does not contain three non-water acquisition resources".into());
    }
    if !catalog.iter().any(|resource| resource.name == "Water") {
        return Err("catalog does not contain Water".into());
    }

    for resource in structural_candidates {
        let Some(seed) = crate::physical_material::PhysicalMaterial::realized(
            Material::free_base(resource.name.clone(), 1.0),
            vec![Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            catalog,
        ) else {
            continue;
        };
        let mut structure = crate::structure::OrganismStructure::new();
        if crate::material_restoration::restore_material(
            &mut structure,
            &seed,
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
            catalog,
        )
        .is_none()
        {
            continue;
        }

        let mut nodes = 0usize;
        if let Some(result) = free_form_search(
            structure,
            catalog,
            &acquisition_candidates,
            EnergyLedger::default(),
            SEARCH_ENERGY,
            &mut nodes,
        ) {
            return Ok(result);
        }
    }

    Err("blueprint-free constructor exhausted its physically reachable growth candidates before producing a valid organism".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blueprint_free_constructor_produces_a_valid_organism() {
        let catalog = crate::resources::default_catalog();
        let result = construct_valid(&catalog).expect("constructor should find a valid organism");
        assert!(!result.structure.units.is_empty());
        assert!(!result.structure.bonds.is_empty());
        let cavity = crate::cavity::analyze_genome_cavity(&result.structure, &catalog)
            .expect("genome analysis should succeed");
        assert!(cavity.is_some_and(|cavity| cavity.qualifies()));
        assert_eq!(result.acquired_resource_placements.len(), 4);
    }

    #[test]
    fn acquisition_requirement_is_binary_for_each_selected_resource() {
        let catalog = crate::resources::default_catalog();
        let result = construct_valid(&catalog).expect("constructor should find a valid organism");
        let names = result
            .acquired_resource_placements
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>();
        assert!(names.contains(&"Water"));
        assert_eq!(names.len(), 4);
    }
}
