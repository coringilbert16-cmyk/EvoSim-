//! Forward-only genesis construction for the first valid organism.
//!
//! Genesis is deliberately local. The constructor does not contain a target
//! topology, ring recipe, or future body plan. It grows physical material one
//! bond at a time using the same NFP/contact/bond machinery used by ordinary
//! construction. The qualifying realized cavity is the explicit boundary
//! between genome formation and the remainder of construction.

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

fn resource<'a>(catalog: &'a [BaseResource], name: &str) -> Option<&'a BaseResource> {
    catalog.iter().find(|resource| resource.name == name)
}

fn restore_single(
    structure: &mut crate::structure::OrganismStructure,
    resource: &BaseResource,
    placement: Placement,
    catalog: &[BaseResource],
) -> Option<usize> {
    let material = crate::physical_material::PhysicalMaterial::realized(
        Material::free_base(resource.name.clone(), 1.0),
        vec![Placement {
            x: 0.0,
            y: 0.0,
            rotation_radians: 0.0,
        }],
        catalog,
    )?;
    let indices = crate::material_restoration::restore_material(
        structure,
        &material,
        placement,
        catalog,
    )?;
    indices.first().copied()
}

fn contact_candidates(
    structure: &crate::structure::OrganismStructure,
    new_unit: usize,
    catalog: &[BaseResource],
) -> Vec<crate::contact::ConnectionPairCandidate> {
    let mut cache = crate::contact::ConnectionCompatibilityCache::new();
    let mut candidates = Vec::new();
    for existing in 0..new_unit {
        candidates.extend(
            crate::contact::connection_pair_candidates_cached(
                structure,
                existing,
                new_unit,
                catalog,
                &mut cache,
            )
            .into_iter()
            .filter(|candidate| {
                candidate.distance <= crate::combine_runtime::COMBINE_CONTACT_TOLERANCE
                    && candidate.available_a
                    && candidate.available_b
            }),
        );
    }
    candidates.sort_by(|a, b| {
        a.distance
            .partial_cmp(&b.distance)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                b.facing
                    .partial_cmp(&a.facing)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });
    candidates
}

fn commit_contact_set(
    structure: &mut crate::structure::OrganismStructure,
    new_unit: usize,
    catalog: &[BaseResource],
    ledger: &mut EnergyLedger,
    energy: &mut f64,
) -> usize {
    let mut formed = 0;
    let mut cache = crate::contact::ConnectionCompatibilityCache::new();

    // Recompute after every permanent bond because endpoint availability is
    // part of the physical state. There is no speculative future topology.
    for _ in 0..structure.units.len() {
        let candidates = contact_candidates(structure, new_unit, catalog);
        let Some(candidate) = candidates.into_iter().next() else {
            break;
        };
        // The candidate is ordered as existing -> new by contact_candidates.
        let Some(existing) = (0..new_unit).find(|index| {
            structure.units[*index].physical_id == candidate.endpoint_a.constituent_id
        }) else {
            break;
        };
        let Some((_, _, _, investment, _)) =
            crate::combine_runtime::selected_candidate_evaluation(
                structure,
                existing,
                new_unit,
                candidate.clone(),
                catalog,
            )
        else {
            break;
        };

        if crate::combine_runtime::form_selected_bond(
            structure,
            existing,
            new_unit,
            candidate,
            investment,
            catalog,
            &mut cache,
            ledger,
            energy,
        )
        .is_none()
        {
            break;
        }
        formed += 1;
    }
    formed
}

fn candidate_placements(
    structure: &crate::structure::OrganismStructure,
    resource: &BaseResource,
    catalog: &[BaseResource],
) -> Vec<Placement> {
    let mut placements = Vec::new();
    for target in 0..structure.units.len() {
        let Some(unit) = structure.units.get(target) else {
            continue;
        };
        placements.extend(crate::construction_runtime::nfp_candidate_placements(
            structure,
            resource,
            unit.placement,
            &[target],
            catalog,
        ).unwrap_or_default());
    }
    placements.sort_by(|a, b| {
        let da = structure
            .units
            .iter()
            .map(|unit| (a.x - unit.placement.x).hypot(a.y - unit.placement.y))
            .fold(f64::INFINITY, f64::min);
        let db = structure
            .units
            .iter()
            .map(|unit| (b.x - unit.placement.x).hypot(b.y - unit.placement.y))
            .fold(f64::INFINITY, f64::min);
        da.partial_cmp(&db)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                a.rotation_radians
                    .partial_cmp(&b.rotation_radians)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });
    placements.dedup_by(|a, b| {
        (a.x - b.x).abs() <= 1e-10
            && (a.y - b.y).abs() <= 1e-10
            && (a.rotation_radians - b.rotation_radians).abs() <= 1e-10
    });
    placements
}

fn try_local_continuation(
    structure: &crate::structure::OrganismStructure,
    resource: &BaseResource,
    catalog: &[BaseResource],
    ledger: &EnergyLedger,
    energy: f64,
) -> Option<(
    crate::structure::OrganismStructure,
    EnergyLedger,
    f64,
    usize,
    bool,
    f64,
)> {
    let mut best = None;

    for placement in candidate_placements(structure, resource, catalog) {
        let mut trial = structure.clone();
        let new_unit = restore_single(&mut trial, resource, placement, catalog)?;
        if crate::construction_runtime::placed_unit_overlaps(
            &trial,
            &trial.units[new_unit],
            &[new_unit],
            catalog,
        ) {
            continue;
        }

        let mut trial_ledger = *ledger;
        let mut trial_energy = energy;
        let formed = commit_contact_set(
            &mut trial,
            new_unit,
            catalog,
            &mut trial_ledger,
            &mut trial_energy,
        );
        if formed == 0 {
            continue;
        }

        let cavity_qualifies = crate::cavity::analyze_genome_cavity(&trial, catalog)
            .ok()
            .flatten()
            .is_some_and(|cavity| cavity.qualifies());

        let nearest = structure
            .units
            .iter()
            .map(|unit| {
                (placement.x - unit.placement.x)
                    .hypot(placement.y - unit.placement.y)
            })
            .fold(f64::INFINITY, f64::min);

        if best.as_ref().is_none_or(
            |current: &(crate::structure::OrganismStructure, EnergyLedger, f64, usize, bool, f64)| {
                (cavity_qualifies, formed, -nearest)
                    > (current.4, current.3, -current.5)
            },
        )
        {
            best = Some((
                trial,
                trial_ledger,
                trial_energy,
                new_unit,
                cavity_qualifies,
                nearest,
            ));
        }
    }

    best
}

/// Grow until the realized physical graph first contains a qualifying genome
/// cavity. Every committed unit is attached locally and permanently. No target
/// topology or future placement is consulted.
fn construct_until_genome(
    carbon: &BaseResource,
    catalog: &[BaseResource],
    ledger: &mut EnergyLedger,
    energy: &mut f64,
) -> Result<crate::structure::OrganismStructure, String> {
    let mut structure = crate::structure::OrganismStructure::new();
    let first = restore_single(
        &mut structure,
        carbon,
        Placement {
            x: 0.0,
            y: 0.0,
            rotation_radians: 0.0,
        },
        catalog,
    )
    .ok_or_else(|| "failed to realize genesis starting material".to_string())?;

    let mut current_ledger = *ledger;
    let mut current_energy = *energy;
    let _ = first;

    loop {
        if crate::cavity::analyze_genome_cavity(&structure, catalog)
            .map_err(|error| format!("genome cavity analysis failed: {error}"))?
            .is_some_and(|cavity| cavity.qualifies())
        {
            *ledger = current_ledger;
            *energy = current_energy;
            return Ok(structure);
        }

        let Some((next, next_ledger, next_energy, _, _, _)) =
            try_local_continuation(&structure, carbon, catalog, &current_ledger, current_energy)
        else {
            return Err(
                "genesis constructor has no locally valid physical continuation before genome formation"
                    .into(),
            );
        };

        structure = next;
        current_ledger = next_ledger;
        current_energy = next_energy;
    }
}

fn placement_fits_resource(
    resource: &BaseResource,
    region: &crate::interior_geometry::EnclosedRegion,
    catalog: &[BaseResource],
) -> Option<Placement> {
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
    let water = resource(catalog, "Water")?;
    let water_placement = regions
        .iter()
        .find_map(|region| placement_fits_resource(water, region, catalog))?;
    acquired.push((water.name.clone(), water_placement));
    Some(acquired)
}

fn finish_after_genome(
    mut structure: crate::structure::OrganismStructure,
    catalog: &[BaseResource],
    ledger: &mut EnergyLedger,
    energy: &mut f64,
) -> Result<(crate::structure::OrganismStructure, f64, Vec<(String, Placement)>), String> {
    let acquisition_candidates = available_acquisition_resources(catalog);

    loop {
        if let Some(acquired) = valid_construction(&structure, catalog, &acquisition_candidates) {
            return Ok((structure, *energy, acquired));
        }

        let Some((next, next_ledger, next_energy, _, _, _)) =
            try_local_continuation(
                &structure,
                resource(catalog, "Carbon")
                    .ok_or_else(|| "catalog lacks Carbon for genesis continuation".to_string())?,
                catalog,
                ledger,
                *energy,
            )
        else {
            return Err(
                "genesis constructor has no locally valid continuation after genome formation"
                    .into(),
            );
        };
        structure = next;
        *ledger = next_ledger;
        *energy = next_energy;
    }
}

pub(crate) fn construct_valid(catalog: &[BaseResource]) -> Result<ValidConstruction, String> {
    let carbon = catalog
        .iter()
        .find(|resource| {
            resource.name == "Carbon"
                && resource.physical_state == PhysicalState::Rigid
                && resource.shape.is_valid()
        })
        .ok_or_else(|| "catalog lacks valid rigid Carbon genesis material".to_string())?;

    let mut ledger = EnergyLedger::default();
    let mut energy = CONSTRUCTION_ENERGY;
    let structure = construct_until_genome(carbon, catalog, &mut ledger, &mut energy)?;
    let (structure, energy, acquired_resource_placements) =
        finish_after_genome(structure, catalog, &mut ledger, &mut energy)?;

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
