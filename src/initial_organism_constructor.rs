//! Local free-form construction baseline for the first valid organism.
//!
//! Construction grows from realized physical frontier units. No fixed ring,
//! spoke, or outer-boundary topology is supplied; geometry-derived candidates
//! are admitted through the normal physical bond transaction.

use crate::resources::{BaseResource, Material, PhysicalState};
use crate::state::EnergyLedger;
use crate::structure::Placement;

const CONSTRUCTION_ENERGY: f64 = 1.0e12;
#[derive(Clone, Debug)]
pub(crate) struct ValidConstruction {
    pub structure: crate::structure::OrganismStructure,
    pub energy: f64,
    pub acquired_resource_placements: Vec<(String, Placement)>,
}

fn add_unit(
    structure: &mut crate::structure::OrganismStructure,
    resource: &BaseResource,
    center: (f64, f64),
    catalog: &[BaseResource],
) -> Result<usize, String> {
    let material = Material::free_base(resource.name.clone(), 1.0);
    let instance = crate::physical_material::PhysicalMaterial::realized(
        material,
        vec![Placement {
            x: 0.0,
            y: 0.0,
            rotation_radians: 0.0,
        }],
        catalog,
    )
    .ok_or_else(|| format!("failed to realize construction unit {}", resource.name))?;
    let indices = crate::material_restoration::restore_material(
        structure,
        &instance,
        Placement {
            x: center.0,
            y: center.1,
            rotation_radians: 0.0,
        },
        catalog,
    )
    .ok_or_else(|| format!("failed to restore construction unit {}", resource.name))?;
    indices.first().copied().ok_or_else(|| {
        format!(
            "construction unit {} restored no physical constituent",
            resource.name
        )
    })
}

fn bond_units_legacy(
    structure: &mut crate::structure::OrganismStructure,
    unit_a: usize,
    unit_b: usize,
    catalog: &[BaseResource],
    ledger: &mut EnergyLedger,
    energy: &mut f64,
) -> Result<(), String> {
    let mut bonded = false;
    let mut used_a = Vec::new();
    let mut used_b = Vec::new();

    // A shared wall segment is sealed by at most two endpoint bonds. Recompute
    // the physical candidate graph after each commit so endpoint availability
    // cannot become stale after the first permanent bond.
    for _ in 0..2 {
        let mut cache = crate::contact::ConnectionCompatibilityCache::new();
        let candidates = crate::contact::connection_pair_candidates_cached(
            structure, unit_a, unit_b, catalog, &mut cache,
        )
        .into_iter()
        .filter(|candidate| {
            candidate.distance <= crate::combine_runtime::COMBINE_CONTACT_TOLERANCE
                && candidate.available_a
                && candidate.available_b
        })
        .collect::<Vec<_>>();

        let candidate = candidates
            .iter()
            .filter(|candidate| {
                matches!(
                    candidate.endpoint_a,
                    crate::structure::ConnectionEndpoint::Corner { .. }
                ) && matches!(
                    candidate.endpoint_b,
                    crate::structure::ConnectionEndpoint::Corner { .. }
                ) && !used_a.contains(&candidate.endpoint_a)
                    && !used_b.contains(&candidate.endpoint_b)
            })
            .min_by(|a, b| {
                a.distance
                    .partial_cmp(&b.distance)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned()
            .or_else(|| {
                if bonded {
                    None
                } else {
                    candidates.into_iter().max_by(|a, b| {
                        a.distance
                            .partial_cmp(&b.distance)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                }
            });

        let Some(candidate) = candidate else {
            if bonded {
                break;
            }
            return Err(format!(
                "no physical contact between construction units {unit_a} and {unit_b}"
            ));
        };

        let endpoint_a = candidate.endpoint_a.clone();
        let endpoint_b = candidate.endpoint_b.clone();
        let (_, _, _, investment, _) = crate::combine_runtime::selected_candidate_evaluation(
            structure, unit_a, unit_b, candidate, catalog,
        )
        .ok_or_else(|| format!("physical bond candidate {unit_a}-{unit_b} failed evaluation"))?;

        // Recompute candidates on the next iteration after this transaction
        // changes endpoint availability.
        crate::combine_runtime::form_selected_bond(
            structure, unit_a, unit_b, candidate, investment, catalog, &mut cache, ledger, energy,
        )
        .ok_or_else(|| format!("physical bond transaction {unit_a}-{unit_b} failed"))?;

        used_a.push(endpoint_a);
        used_b.push(endpoint_b);
        bonded = true;
    }

    Ok(())
}

/// Place the fixed geometric scaffold directly, while using the ordinary
/// physical bond transaction for every permanent connection. There is no
/// speculative placement search and no recursive body-plan search.
/// Build the initial body by local geometric growth.
///
/// No target outline, ring, spoke, or blueprint is supplied. Each step starts
/// from a realized frontier unit, derives candidate placements from its actual
/// connection geometry, rejects only true penetration, commits explicit bonds,
/// and checks whether the realized structure has produced a qualifying bonded
/// cavity. Incidental contact alone never becomes a bond.
fn construct_free_form(
    catalog: &[BaseResource],
) -> Result<(crate::structure::OrganismStructure, EnergyLedger, f64), String> {
    let carbon = catalog
        .iter()
        .find(|resource| {
            resource.name == "Carbon"
                && resource.physical_state == PhysicalState::Rigid
                && resource.shape.is_valid()
        })
        .ok_or_else(|| "catalog lacks valid rigid Carbon geometry".to_string())?;

    let carbon_index = catalog
        .iter()
        .position(|resource| resource.name == carbon.name)
        .ok_or_else(|| "Carbon is missing from the construction catalog".to_string())?;
    let construction_catalog = crate::construction_catalog::ConstructionCatalog::build(catalog);

    let mut structure = crate::structure::OrganismStructure::new();
    let mut ledger = EnergyLedger::default();
    let mut energy = CONSTRUCTION_ENERGY;
    let seed = add_unit(&mut structure, carbon, (0.0, 0.0), catalog)?;
    let mut frontier = vec![seed];

    for _step in 0..256 {
        let snapshot = structure.clone();
        let mut best: Option<(
            i32,
            f64,
            crate::structure::OrganismStructure,
            EnergyLedger,
            f64,
            usize,
        )> = None;

        for &anchor_index in &frontier {
            let Some(anchor_unit) = snapshot.units.get(anchor_index) else {
                continue;
            };
            let anchor = anchor_unit.placement;

            for placement in construction_catalog
                .placements_for_pair(carbon_index, carbon_index, anchor)
                .into_iter()
                .skip(1)
            {
                let instance = crate::physical_material::PhysicalMaterial::realized(
                    Material::free_base(carbon.name.clone(), 1.0),
                    vec![Placement {
                        x: 0.0,
                        y: 0.0,
                        rotation_radians: 0.0,
                    }],
                    catalog,
                )
                .ok_or_else(|| "Carbon construction material has invalid geometry".to_string())?;

                let mut trial = snapshot.clone();
                let Some(indices) = crate::material_restoration::restore_material(
                    &mut trial, &instance, placement, catalog,
                ) else {
                    continue;
                };
                let Some(new_index) = indices.first().copied() else {
                    continue;
                };

                if crate::construction_runtime::placed_unit_overlaps(
                    &trial,
                    &trial.units[new_index],
                    &indices,
                    catalog,
                ) {
                    continue;
                }

                let mut contacts = Vec::<(usize, crate::contact::ConnectionPairCandidate)>::new();

                for existing_index in 0..new_index {
                    let mut cache = crate::contact::ConnectionCompatibilityCache::new();
                    if let Some(candidate) = crate::contact::connection_pair_candidates_cached(
                        &trial,
                        existing_index,
                        new_index,
                        catalog,
                        &mut cache,
                    )
                    .into_iter()
                    .filter(|candidate| {
                        candidate.distance <= crate::combine_runtime::COMBINE_CONTACT_TOLERANCE
                            && candidate.available_a
                            && candidate.available_b
                    })
                    .min_by(|a, b| {
                        a.distance
                            .partial_cmp(&b.distance)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    }) {
                        contacts.push((existing_index, candidate));
                    }
                }

                let Some((anchor_target, anchor_candidate)) = contacts
                    .iter()
                    .find(|(index, _)| *index == anchor_index)
                    .cloned()
                else {
                    continue;
                };

                let Some((_, _, _, investment, _)) =
                    crate::combine_runtime::selected_candidate_evaluation(
                        &trial,
                        anchor_target,
                        new_index,
                        anchor_candidate,
                        catalog,
                    )
                else {
                    continue;
                };

                let mut trial_ledger = ledger.clone();
                let mut trial_energy = energy;
                let mut cache = crate::contact::ConnectionCompatibilityCache::new();

                if crate::combine_runtime::form_selected_bond(
                    &mut trial,
                    anchor_target,
                    new_index,
                    anchor_candidate,
                    investment,
                    catalog,
                    &mut cache,
                    &mut trial_ledger,
                    &mut trial_energy,
                )
                .is_none()
                {
                    continue;
                }

                // Recompute after every bond so endpoint availability is
                // authoritative. When two units physically share a boundary,
                // allow up to two explicit corner bonds: one for each endpoint
                // of that shared contact. This is local bond formation, not a
                // global rule that all physical contacts must be bonded.
                for (existing_index, _) in contacts.iter().copied() {
                    let attempts = if existing_index == anchor_index { 1 } else { 2 };
                    for _ in 0..attempts {
                        let mut cache = crate::contact::ConnectionCompatibilityCache::new();
                        let candidates = crate::contact::connection_pair_candidates_cached(
                            &trial,
                            existing_index,
                            new_index,
                            catalog,
                            &mut cache,
                        )
                        .into_iter()
                        .filter(|candidate| {
                            candidate.distance <= crate::combine_runtime::COMBINE_CONTACT_TOLERANCE
                                && candidate.available_a
                                && candidate.available_b
                        })
                        .collect::<Vec<_>>();

                        let candidate = candidates
                            .iter()
                            .filter(|candidate| {
                                matches!(
                                    candidate.endpoint_a,
                                    crate::structure::ConnectionEndpoint::Corner { .. }
                                ) && matches!(
                                    candidate.endpoint_b,
                                    crate::structure::ConnectionEndpoint::Corner { .. }
                                )
                            })
                            .min_by(|a, b| {
                                a.distance
                                    .partial_cmp(&b.distance)
                                    .unwrap_or(std::cmp::Ordering::Equal)
                            })
                            .copied()
                            .or_else(|| {
                                candidates.into_iter().min_by(|a, b| {
                                    a.distance
                                        .partial_cmp(&b.distance)
                                        .unwrap_or(std::cmp::Ordering::Equal)
                                })
                            });

                        let Some(candidate) = candidate else {
                            break;
                        };

                        let Some((_, _, _, investment, _)) =
                            crate::combine_runtime::selected_candidate_evaluation(
                                &trial,
                                existing_index,
                                new_index,
                                candidate,
                                catalog,
                            )
                        else {
                            break;
                        };

                        if crate::combine_runtime::form_selected_bond(
                            &mut trial,
                            existing_index,
                            new_index,
                            candidate,
                            investment,
                            catalog,
                            &mut cache,
                            &mut trial_ledger,
                            &mut trial_energy,
                        )
                        .is_none()
                        {
                            break;
                        }
                    }
                }

                let contact_count = contacts.len() as i32;
                let cavity_bonus = crate::cavity::analyze_genome_cavity(&trial, catalog)
                    .ok()
                    .flatten()
                    .map(|cavity| (cavity.area * 1000.0) as i32)
                    .unwrap_or(0);

                // Grow outward with single-contact candidates, but once a
                // larger local body exists, allow multi-contact candidates to
                // close loops. No global topology is prescribed.
                let loop_preference = if snapshot.units.len() >= 6 {
                    contact_count * 1000
                } else {
                    -contact_count * 1000
                };
                let score = loop_preference + cavity_bonus;
                let distance = (placement.x - anchor.x).hypot(placement.y - anchor.y);

                if best
                    .as_ref()
                    .is_none_or(|(best_score, best_distance, _, _, _, _)| {
                        score > *best_score || (score == *best_score && distance < *best_distance)
                    })
                {
                    best = Some((
                        score,
                        distance,
                        trial,
                        trial_ledger,
                        trial_energy,
                        new_index,
                    ));
                }
            }
        }

        let Some((_, _, next_structure, next_ledger, next_energy, new_index)) = best else {
            return Err(format!(
                "free-form constructor reached a geometric dead end after {} units",
                structure.units.len()
            ));
        };

        structure = next_structure;
        ledger = next_ledger;
        energy = next_energy;
        frontier.push(new_index);

        if let Some(cavity) = crate::cavity::analyze_genome_cavity(&structure, catalog)
            .map_err(|error| format!("genome cavity analysis failed: {error}"))?
        {
            if cavity.qualifies()
                && structure.units.len() > cavity.boundary_units.len()
                && valid_construction(
                    &structure,
                    catalog,
                    &available_acquisition_resources(catalog),
                )
                .is_some()
            {
                return Ok((structure, ledger, energy));
            }
        }
    }

    Err("free-form constructor exhausted local geometric growth before satisfying viability".into())
}

fn placement_fits_resource(
    resource: &BaseResource,
    region: &crate::interior_geometry::EnclosedRegion,
    catalog: &[BaseResource],
) -> Option<Placement> {
    // The scaffold deliberately creates broad accessible chambers. Test the
    // region's own sample point first, then a small deterministic set of
    // nearby points. This is a bounded geometric check, not a raster search.
    let points = [
        region.sample_point,
        (region.sample_point.0 * 0.75, region.sample_point.1 * 0.75),
        (region.sample_point.0 * 0.5, region.sample_point.1 * 0.5),
        (region.sample_point.0 * 1.25, region.sample_point.1 * 1.25),
    ];
    for &(x, y) in &points {
        if !region.contains_point(x, y) {
            continue;
        }
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
    let cavity = crate::cavity::analyze_genome_cavity(structure, catalog)
        .ok()
        .flatten()?;
    if !cavity.qualifies() || structure.units.len() <= cavity.boundary_units.len() {
        return None;
    }

    let regions =
        crate::interior_geometry::find_accessible_interior_regions(structure, catalog).ok()?;
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

pub(crate) fn construct_valid(catalog: &[BaseResource]) -> Result<ValidConstruction, String> {
    let acquisition_candidates = available_acquisition_resources(catalog);
    if acquisition_candidates.len() < 3 {
        return Err("catalog does not contain three non-water acquisition resources".into());
    }
    if !catalog.iter().any(|resource| resource.name == "Water") {
        return Err("catalog does not contain Water".into());
    }

    let (structure, _ledger, energy) = construct_free_form(catalog)?;
    let acquired_resource_placements =
        valid_construction(&structure, catalog, &acquisition_candidates)
            .ok_or_else(|| "free-form construction did not satisfy viability".to_string())?;

    Ok(ValidConstruction {
        structure,
        energy,
        acquired_resource_placements,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_form_constructor_produces_a_valid_organism() {
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
