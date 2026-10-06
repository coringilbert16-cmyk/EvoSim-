//! Forward-only initial-organism construction from local physical geometry.
//!
//! Genesis deliberately has no fixed ring, spiral, lattice, material identity,
//! piece count, or target silhouette. One physical rigid constituent provides
//! the initial cell material; every later constituent is positioned only by
//! exact contacts with the already-realized structure. Once committed, a
//! placement and its bonds are never rolled back.

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

fn realized_single_resource(
    resource: &BaseResource,
    catalog: &[BaseResource],
) -> Option<crate::physical_material::PhysicalMaterial> {
    crate::physical_material::PhysicalMaterial::realized(
        Material::free_base(resource.name.clone(), 1.0),
        vec![Placement {
            x: 0.0,
            y: 0.0,
            rotation_radians: 0.0,
        }],
        catalog,
    )
}

fn rigid_resources<'a>(catalog: &'a [BaseResource]) -> impl Iterator<Item = &'a BaseResource> {
    catalog.iter().filter(|resource| {
        resource.physical_state == PhysicalState::Rigid && resource.shape.is_valid()
    })
}

/// Start from one real physical constituent. Catalog order is only a
/// deterministic way to obtain the first environmental material; it is not a
/// construction preference. Subsequent growth considers every rigid catalog
/// material without assigning Carbon or any other material intrinsic priority.
fn initial_seed(
    catalog: &[BaseResource],
) -> Result<(crate::structure::OrganismStructure, usize), String> {
    let seed = rigid_resources(catalog)
        .next()
        .ok_or_else(|| "catalog contains no valid rigid construction material".to_string())?;
    let instance = realized_single_resource(seed, catalog)
        .ok_or_else(|| format!("failed to realize initial {}", seed.name))?;

    let mut structure = crate::structure::OrganismStructure::new();
    let indices = crate::material_restoration::restore_material(
        &mut structure,
        &instance,
        Placement {
            x: 0.0,
            y: 0.0,
            rotation_radians: 0.0,
        },
        catalog,
    )
    .ok_or_else(|| format!("failed to restore initial {}", seed.name))?;

    let index = *indices
        .first()
        .ok_or_else(|| "initial physical material restored no constituent".to_string())?;
    Ok((structure, index))
}

fn placement_fits_resource(
    resource: &BaseResource,
    region: &crate::interior_geometry::EnclosedRegion,
    catalog: &[BaseResource],
) -> Option<Placement> {
    // Acquisition placement is a physical containment check, not a body-plan
    // coordinate. Try the region's own sample point and the center-biased
    // alternatives already implied by the region geometry.
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
    // A qualifying cavity requires a closed cycle in the connected physical
    // boundary. Before bonds can reach the unit count there is therefore no
    // possible enclosed region to analyze. Keep the expensive global cavity
    // analysis out of the early frontier-growth loop.
    if structure.bonds.len() < structure.units.len() {
        return None;
    }
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

/// Grow one constituent at a time from the current realized boundary.
///
/// The constructor does not generate a target topology. Each candidate material
/// is tried through the exact physical attachment path, which derives valid
/// rotations from actual boundary features. A successful transaction is
/// immediately committed; a failed candidate is discarded without mutating the
/// organism. No committed step is backtracked.
fn open_construction_indices(
    structure: &crate::structure::OrganismStructure,
    catalog: &[BaseResource],
    frontier: &[usize],
) -> Vec<usize> {
    // The frontier is maintained across growth steps. We only revalidate
    // indices that were already known to be capable of construction work,
    // plus the newly-created units added by the previous transaction.
    frontier
        .iter()
        .copied()
        .filter(|&index| {
            structure.units.get(index).is_some_and(|unit| {
                !crate::construction_runtime::construction_frontier_endpoints(
                    unit,
                    structure,
                    catalog,
                )
                .is_empty()
            })
        })
        .collect()
}

fn grow_one_step(
    structure: &mut crate::structure::OrganismStructure,
    catalog: &[BaseResource],
    candidates: &[(String, crate::physical_material::PhysicalMaterial)],
    frontier: &[usize],
    nodes: &mut usize,
    ledger: &EnergyLedger,
    energy: f64,
) -> Option<(EnergyLedger, f64, Vec<usize>)> {
    // Genesis is intentionally local: only units with an unbonded physical
    // boundary endpoint can admit the next constituent. This is a frontier
    // search, not a rescan of every historical constituent.
    let existing_indices = open_construction_indices(structure, catalog, frontier);

    // There is no preferred construction material here. Every valid rigid
    // resource is an equally eligible physical candidate; the first candidate
    // that satisfies exact geometry is committed immediately.
    for existing_index in existing_indices {
        for (_resource_name, instance) in candidates {
            if let Some((
                _indices,
                _part_index,
                _attempt,
                trial_ledger,
                trial_energy,
            )) = crate::construction_runtime::try_attach_physical_material_bond_driven(
                structure,
                existing_index,
                instance,
                catalog,
                nodes,
                ledger,
                energy,
            ) {
                let mut next_frontier = frontier.to_vec();
                for index in _indices {
                    if !next_frontier.contains(&index) {
                        next_frontier.push(index);
                    }
                }
                return Some((trial_ledger, trial_energy, next_frontier));
            }
        }
    }

    None
}

/// Construct the first viable organism by local physical growth.
///
/// The cavity analyzer remains independent: a cavity qualifies because the
/// realized geometry satisfies the genome rules, not because this constructor
/// declares a particular region to be the genome. Growth continues after the
/// first qualifying cavity until the ordinary initial viability contract is
/// satisfied.
fn construct_physical_organism(
    catalog: &[BaseResource],
) -> Result<(crate::structure::OrganismStructure, EnergyLedger, f64), String> {
    let (mut structure, _seed_index) = initial_seed(catalog)?;
    let mut ledger = EnergyLedger::default();
    let mut energy = CONSTRUCTION_ENERGY;
    let mut nodes = 0usize;
    // Only the live construction frontier is revisited. It grows monotonically
    // as new physical constituents are committed and is pruned lazily when a
    // unit loses its remaining usable endpoints.
    let mut frontier = vec![0usize];
    // Physical material realizations are immutable candidate geometry during
    // genesis. Build them once rather than rebuilding the same catalog-derived
    // shapes on every frontier-growth step.
    let construction_candidates = rigid_resources(catalog)
        .filter_map(|resource| {
            Some((
                resource.name.clone(),
                realized_single_resource(resource, catalog)?,
            ))
        })
        .collect::<Vec<_>>();

    loop {
        let acquisition_candidates = available_acquisition_resources(catalog);
        if acquisition_candidates.len() >= 3
            && catalog.iter().any(|resource| resource.name == "Water")
            && valid_construction(&structure, catalog, &acquisition_candidates).is_some()
        {
            return Ok((structure, ledger, energy));
        }

        let Some((next_ledger, next_energy, next_frontier)) =
            grow_one_step(
                &mut structure,
                catalog,
                &construction_candidates,
                &frontier,
                &mut nodes,
                &ledger,
                energy,
            )
        else {
            let cavity = crate::cavity::analyze_genome_cavity(&structure, catalog)
                .ok()
                .flatten()
                .is_some_and(|cavity| cavity.qualifies());
            return Err(format!(
                "forward physical construction reached a state with no valid local growth step: units={}, bonds={}, genome_cavity={}, placement_nodes={nodes}",
                structure.units.len(),
                structure.bonds.len(),
                cavity
            ));
        };

        ledger = next_ledger;
        energy = next_energy;
        frontier = next_frontier;
    }
}

pub(crate) fn construct_valid(catalog: &[BaseResource]) -> Result<ValidConstruction, String> {
    let acquisition_candidates = available_acquisition_resources(catalog);
    if acquisition_candidates.len() < 3 {
        return Err("catalog does not contain three non-water acquisition resources".into());
    }
    if !catalog.iter().any(|resource| resource.name == "Water") {
        return Err("catalog does not contain Water".into());
    }

    let (structure, _ledger, energy) = construct_physical_organism(catalog)?;
    let acquired_resource_placements =
        valid_construction(&structure, catalog, &acquisition_candidates).ok_or_else(|| {
            "forward physical construction did not satisfy initial viability".to_string()
        })?;

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
