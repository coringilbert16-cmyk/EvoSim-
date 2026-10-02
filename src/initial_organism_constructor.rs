//! Minimal blueprint-free constructor for the first organism.
//!
//! The initial organism has no body plan. It grows physical material through
//! the normal construction/COMBINE path and accepts the first state that
//! satisfies the actual initial-organism contract.

use crate::resources::{BaseResource, Material, PhysicalState};
use crate::state::EnergyLedger;
use crate::structure::Placement;

const SEARCH_ENERGY: f64 = 1.0e12;
const ACQUISITION_SAMPLES: usize = 48;
const CONTACT_TOLERANCE: f64 = 1.0e-8;

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

fn acquisition_candidates(catalog: &[BaseResource]) -> Vec<&BaseResource> {
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

                for resource in selected {
                    let placement = regions
                        .iter()
                        .find_map(|region| placement_fits_resource(resource, region, catalog))?;
                    placements.push((resource.name.clone(), placement));
                }

                return Some(placements);
            }
        }
    }
    None
}

fn valid_construction(
    structure: &crate::structure::OrganismStructure,
    catalog: &[BaseResource],
    acquisition: &[&BaseResource],
) -> Option<Vec<(String, Placement)>> {
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

    let mut result = find_acquisition_set(acquisition, &regions, catalog)?;
    let water = catalog.iter().find(|resource| resource.name == "Water")?;
    let water_placement = regions
        .iter()
        .find_map(|region| placement_fits_resource(water, region, catalog))?;
    result.push((water.name.clone(), water_placement));
    Some(result)
}

fn bond_exists(
    structure: &crate::structure::OrganismStructure,
    first: usize,
    second: usize,
) -> bool {
    let Some(first_id) = structure.physical_id(first) else {
        return false;
    };
    let Some(second_id) = structure.physical_id(second) else {
        return false;
    };

    structure.bonds.iter().any(|bond| {
        (bond.endpoint_a.constituent_id == first_id && bond.endpoint_b.constituent_id == second_id)
            || (bond.endpoint_a.constituent_id == second_id
                && bond.endpoint_b.constituent_id == first_id)
    })
}

fn contact_candidate(
    structure: &crate::structure::OrganismStructure,
    first: usize,
    second: usize,
    catalog: &[BaseResource],
) -> Option<crate::contact::ConnectionPairCandidate> {
    let mut cache = crate::contact::ConnectionCompatibilityCache::new();
    crate::contact::connection_pair_candidates_cached(structure, first, second, catalog, &mut cache)
        .into_iter()
        .filter(|candidate| {
            candidate.distance <= CONTACT_TOLERANCE
                && candidate.available_a
                && candidate.available_b
        })
        .max_by(|a, b| {
            a.facing
                .partial_cmp(&b.facing)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

fn commit_bond(
    structure: &mut crate::structure::OrganismStructure,
    first: usize,
    second: usize,
    catalog: &[BaseResource],
    ledger: &mut EnergyLedger,
    energy: &mut f64,
) -> bool {
    let Some(candidate) = contact_candidate(structure, first, second, catalog) else {
        return false;
    };
    let Some((_, _, _, investment, _)) = crate::combine_runtime::selected_candidate_evaluation(
        structure, first, second, candidate, catalog,
    ) else {
        return false;
    };

    let mut cache = crate::contact::ConnectionCompatibilityCache::new();
    crate::combine_runtime::form_selected_bond(
        structure, first, second, candidate, investment, catalog, &mut cache, ledger, energy,
    )
    .is_some()
}

fn try_close_with_new_unit(
    structure: &crate::structure::OrganismStructure,
    anchor: usize,
    target: usize,
    material: &crate::physical_material::PhysicalMaterial,
    catalog: &[BaseResource],
    ledger: &EnergyLedger,
    energy: f64,
) -> Option<(crate::structure::OrganismStructure, EnergyLedger, f64)> {
    if anchor == target || bond_exists(structure, anchor, target) {
        return None;
    }

    let resource = material.material.parts.first()?.0.clone();
    let resource = catalog.iter().find(|r| r.name == resource)?;
    let placements = crate::construction_runtime::candidate_placements(
        structure,
        resource,
        structure.units.get(anchor)?.placement,
        &[anchor, target],
        catalog,
    );

    for origin in placements {
        let mut trial = structure.clone();
        let indices = crate::material_restoration::restore_material_in_place(
            &mut trial, material, origin, catalog,
        )?;
        let new_index = *indices.first()?;

        let mut trial_ledger = *ledger;
        let mut trial_energy = energy;

        if !commit_bond(
            &mut trial,
            anchor,
            new_index,
            catalog,
            &mut trial_ledger,
            &mut trial_energy,
        ) {
            continue;
        }

        if !commit_bond(
            &mut trial,
            target,
            new_index,
            catalog,
            &mut trial_ledger,
            &mut trial_energy,
        ) {
            continue;
        }

        return Some((trial, trial_ledger, trial_energy));
    }

    None
}

fn attach_one(
    structure: &crate::structure::OrganismStructure,
    anchor: usize,
    materials: &[crate::physical_material::PhysicalMaterial],
    catalog: &[BaseResource],
    ledger: &EnergyLedger,
    energy: f64,
) -> Option<(crate::structure::OrganismStructure, EnergyLedger, f64)> {
    for material in materials {
        let mut nodes = 0;
        let Some((trial, _, _, _, ledger, energy)) =
            crate::construction_runtime::try_attach_physical_material_bond_driven(
                structure, anchor, material, catalog, &mut nodes, ledger, energy,
            )
        else {
            continue;
        };
        return Some((trial, ledger, energy));
    }
    None
}

fn grow_until_valid(
    mut structure: crate::structure::OrganismStructure,
    materials: &[crate::physical_material::PhysicalMaterial],
    catalog: &[BaseResource],
    acquisition: &[&BaseResource],
    mut ledger: EnergyLedger,
    mut energy: f64,
) -> Option<ValidConstruction> {
    // Every iteration does only two things:
    // 1. try to close a loop by inserting one unit between two existing units;
    // 2. if no closure exists yet, attach one unit and repeat.
    //
    // There is no score, body plan, lookahead, backtracking tree, or arbitrary
    // attempt budget. Physical validity and the organism validity contract are
    // the only authorities.
    loop {
        for anchor in 0..structure.units.len() {
            for target in 0..structure.units.len() {
                for material in materials {
                    if let Some((trial, trial_ledger, trial_energy)) = try_close_with_new_unit(
                        &structure, anchor, target, material, catalog, &ledger, energy,
                    ) {
                        structure = trial;
                        ledger = trial_ledger;
                        energy = trial_energy;

                        if let Some(acquired_resource_placements) =
                            valid_construction(&structure, catalog, acquisition)
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
        }

        let anchor = structure.units.len().saturating_sub(1);
        let (trial, trial_ledger, trial_energy) =
            attach_one(&structure, anchor, materials, catalog, &ledger, energy)?;

        structure = trial;
        ledger = trial_ledger;
        energy = trial_energy;
    }
}

pub(crate) fn construct_valid(catalog: &[BaseResource]) -> Result<ValidConstruction, String> {
    let rigid_resources = catalog
        .iter()
        .filter(|resource| resource.physical_state == PhysicalState::Rigid)
        .filter(|resource| resource.shape.is_valid())
        .collect::<Vec<_>>();

    if rigid_resources.is_empty() {
        return Err("no rigid resource can seed a physical organism".into());
    }

    let acquisition = acquisition_candidates(catalog);
    if acquisition.len() < 3 {
        return Err("catalog does not contain three non-water acquisition resources".into());
    }
    if !catalog.iter().any(|resource| resource.name == "Water") {
        return Err("catalog does not contain Water".into());
    }

    let materials = rigid_resources
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
        })
        .collect::<Vec<_>>();

    for seed in &materials {
        let mut structure = crate::structure::OrganismStructure::new();
        if crate::material_restoration::restore_material(
            &mut structure,
            seed,
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

        if let Some(result) = grow_until_valid(
            structure,
            &materials,
            catalog,
            &acquisition,
            EnergyLedger::default(),
            SEARCH_ENERGY,
        ) {
            return Ok(result);
        }
    }

    Err(
        "blueprint-free constructor could not reach a valid organism through physical growth"
            .into(),
    )
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
