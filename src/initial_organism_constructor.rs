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
fn temporary_three_carbon_scaffold(
    catalog: &[BaseResource],
) -> Result<(crate::structure::OrganismStructure, Vec<crate::structure::PhysicalConstituentId>), String> {
    let carbon = catalog
        .iter()
        .find(|resource| resource.name == "Carbon")
        .ok_or_else(|| "catalog has no Carbon resource".to_string())?;
    let crate::resources::Form::RegularPolygon { sides, radius } = carbon.shape.form else {
        return Err("genesis scaffold requires polygonal Carbon".into());
    };
    if sides != 6 || !radius.is_finite() || radius <= 0.0 {
        return Err("genesis scaffold requires regular hexagonal Carbon".into());
    }

    // Two Carbon pieces form the bottom row. The third is centered above them.
    // All three center-to-center distances are sqrt(3) * radius, so every
    // neighboring pair meets flat-to-flat with zero penetration.
    let spacing = 3.0_f64.sqrt() * radius;
    let placements = [
        Placement { x: -spacing * 0.5, y: 0.0, rotation_radians: 0.0 },
        Placement { x:  spacing * 0.5, y: 0.0, rotation_radians: 0.0 },
        Placement { x: 0.0, y: spacing, rotation_radians: 0.0 },
    ];

    let mut structure = crate::structure::OrganismStructure::new();
    let mut ids = Vec::with_capacity(3);
    for placement in placements {
        let mut unit = crate::structure::StructuralUnit::from_material(
            Material::free_base(carbon.name.clone(), 1.0),
            placement,
        ).ok_or_else(|| "failed to create Carbon scaffold unit".to_string())?;
        if !unit.realize_default_geometry(catalog) {
            return Err("failed to realize Carbon scaffold geometry".into());
        }
        let index = structure.add_unit(unit);
        ids.push(structure.units[index].physical_id);
    }

    let contacts = [
        (0usize, 1usize, ( spacing * 0.5, 0.0), (-spacing * 0.5, 0.0)),
        (0usize, 2usize, ( spacing * 0.25, spacing * 0.5), (0.25 * spacing, -0.5 * spacing)),
        (1usize, 2usize, (-spacing * 0.25, spacing * 0.5), (0.25 * spacing, -0.5 * spacing)),
    ];

    for (a, b, local_a, local_b) in contacts {
        let properties_a = structure.units[a]
            .properties(catalog)
            .ok_or_else(|| "invalid Carbon scaffold properties".to_string())?;
        let properties_b = structure.units[b]
            .properties(catalog)
            .ok_or_else(|| "invalid Carbon scaffold properties".to_string())?;
        let bond = crate::structure::Bond {
            endpoint_a: crate::structure::BondEndpoint::new(
                ids[a],
                crate::structure::ConnectionEndpoint::BoundaryPoint {
                    x: local_a.0,
                    y: local_a.1,
                },
            ),
            endpoint_b: crate::structure::BondEndpoint::new(
                ids[b],
                crate::structure::ConnectionEndpoint::BoundaryPoint {
                    x: local_b.0,
                    y: local_b.1,
                },
            ),
            strength: crate::combine::bond_strength(properties_a, properties_b),
            bond_energy: 0.0,
        };
        crate::contact::try_add_bond(&mut structure, bond, catalog)
            .map_err(|error| format!("failed to bond Carbon genesis scaffold: {error:?}"))?;
    }

    if structure.bonds.len() != 3 {
        return Err("Carbon genesis scaffold did not form its three bonds".into());
    }
    Ok((structure, ids))
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
#[derive(Default)]
struct ConstructionEndpointOccupancy {
    occupied: std::collections::HashSet<(usize, u64, u64, u8)>,
}

impl ConstructionEndpointOccupancy {
    fn key(endpoint: &crate::structure::ConnectionEndpoint) -> (u64, u64, u8) {
        match endpoint {
            crate::structure::ConnectionEndpoint::BoundaryPoint { x, y } => {
                (x.to_bits(), y.to_bits(), 0)
            }
            crate::structure::ConnectionEndpoint::Corner { point_index } => (*point_index as u64, 0, 1),
            crate::structure::ConnectionEndpoint::LineEndpoint { point_index } => (*point_index as u64, 0, 2),
            _ => (0, 0, 3),
        }
    }

    fn insert_bond(&mut self, bond: &crate::structure::Bond, structure: &crate::structure::OrganismStructure) {
        for endpoint in [&bond.endpoint_a, &bond.endpoint_b] {
            if let Some(unit_index) = structure.unit_index(endpoint.constituent_id) {
                let (a, b, kind) = Self::key(&endpoint.location);
                self.occupied.insert((unit_index, a, b, kind));
            }
        }
    }

    fn is_occupied(&self, unit_index: usize, endpoint: &crate::structure::ConnectionEndpoint) -> bool {
        let (a, b, kind) = Self::key(endpoint);
        self.occupied.contains(&(unit_index, a, b, kind))
    }
}

fn open_construction_indices(
    structure: &crate::structure::OrganismStructure,
    catalog: &[BaseResource],
    frontier: &[usize],
    occupancy: &ConstructionEndpointOccupancy,
) -> Vec<usize> {
    // The frontier is maintained across growth steps. We only revalidate
    // indices that were already known to be capable of construction work,
    // plus the newly-created units added by the previous transaction.
    frontier
        .iter()
        .copied()
        .filter(|&index| {
            structure.units.get(index).is_some_and(|unit| {
                crate::construction_runtime::structure_unit_endpoint_options_for_frontier(unit, catalog)
                    .into_iter()
                    .any(|endpoint| !occupancy.is_occupied(index, &endpoint))
            })
        })
        .collect()
}

fn grow_one_step(
    structure: &mut crate::structure::OrganismStructure,
    catalog: &[BaseResource],
    candidates: &[(String, crate::physical_material::PhysicalMaterial)],
    frontier: &[usize],
    occupancy: &mut ConstructionEndpointOccupancy,
    spatial_index: &mut crate::construction_runtime::ConstructionSpatialIndex,
    nodes: &mut usize,
    ledger: &EnergyLedger,
    energy: f64,
) -> Option<(EnergyLedger, f64, Vec<usize>)> {
    // Genesis is intentionally local: only units with an unbonded physical
    // boundary endpoint can admit the next constituent. This is a frontier
    // search, not a rescan of every historical constituent.
    let existing_indices = open_construction_indices(structure, catalog, frontier, occupancy);

    // There is no preferred construction material here. Every valid rigid
    // resource is an equally eligible physical candidate; the first candidate
    // that satisfies exact geometry is committed immediately.
    for existing_index in existing_indices {
        for (_resource_name, instance) in candidates {
            let bonds_before = structure.bonds.len();
            let available_existing_endpoints = crate::construction_runtime::structure_unit_endpoint_options_for_frontier(
                structure.units.get(existing_index)?,
                catalog,
            )
            .into_iter()
            .filter(|endpoint| !occupancy.is_occupied(existing_index, endpoint))
            .collect::<Vec<_>>();
            if let Some((
                _indices,
                _part_index,
                _attempt,
                trial_ledger,
                trial_energy,
            )) = crate::construction_runtime::try_attach_physical_material_bond_driven_indexed(
                structure,
                existing_index,
                &available_existing_endpoints,
                instance,
                catalog,
                nodes,
                ledger,
                energy,
                Some(&spatial_index),
            ) {
                let mut next_frontier = frontier.to_vec();
                for bond in structure.bonds.iter().skip(bonds_before) {
                    occupancy.insert_bond(bond, structure);
                }
                for index in _indices {
                    if !next_frontier.contains(&index) {
                        let radius = structure.units[index]
                            .shape(catalog)
                            .map(|shape| shape.form.bounding_radius())
                            .unwrap_or(0.0);
                        spatial_index.insert(
                            index,
                            structure.units[index].placement.x,
                            structure.units[index].placement.y,
                            radius,
                        );
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
fn finalize_enclosed_scaffold(
    structure: &mut crate::structure::OrganismStructure,
    catalog: &[BaseResource],
    scaffold_ids: &[crate::structure::PhysicalConstituentId],
) -> bool {
    if scaffold_ids.is_empty() {
        return false;
    }

    // Removing the temporary scaffold must leave an actual cyclic surrounding
    // structure before we pay for full interior-geometry analysis.
    let scaffold_set = scaffold_ids.iter().copied().collect::<std::collections::HashSet<_>>();
    let remaining_units = structure
        .units
        .iter()
        .filter(|unit| !scaffold_set.contains(&unit.physical_id))
        .count();
    let remaining_bonds = structure
        .bonds
        .iter()
        .filter(|bond| {
            !scaffold_set.contains(&bond.endpoint_a.constituent_id)
                && !scaffold_set.contains(&bond.endpoint_b.constituent_id)
        })
        .count();
    if remaining_bonds < remaining_units || remaining_units < 3 {
        return false;
    }

    let mut without_scaffold = structure.clone();
    without_scaffold.remove_units_by_physical_ids(scaffold_ids);
    let Some(cavity) = crate::cavity::analyze_genome_cavity(&without_scaffold, catalog)
        .ok()
        .flatten()
        .filter(|cavity| cavity.qualifies())
    else {
        return false;
    };

    let genome_ids = cavity
        .boundary_units
        .iter()
        .filter_map(|&index| without_scaffold.physical_id(index))
        .collect::<Vec<_>>();
    if genome_ids.is_empty() {
        return false;
    }

    structure.remove_units_by_physical_ids(scaffold_ids);
    structure.set_genome_constituent_ids(genome_ids);
    true
}

fn construct_physical_organism(
    catalog: &[BaseResource],
) -> Result<(crate::structure::OrganismStructure, EnergyLedger, f64), String> {
    let (mut structure, scaffold_ids) = temporary_three_carbon_scaffold(catalog)?;
    let mut ledger = EnergyLedger::default();
    let mut energy = CONSTRUCTION_ENERGY;
    let mut nodes = 0usize;
    // Only the live construction frontier is revisited. It grows monotonically
    // as new physical constituents are committed and is pruned lazily when a
    // unit loses its remaining usable endpoints.
    let mut frontier = (0..structure.units.len()).collect::<Vec<_>>();
    let mut scaffold_active = true;
    let mut spatial_index =
        crate::construction_runtime::ConstructionSpatialIndex::new(&structure, catalog);
    let mut occupancy = ConstructionEndpointOccupancy::default();
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
        if !scaffold_active
            && acquisition_candidates.len() >= 3
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
                &mut occupancy,
                &mut spatial_index,
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

        if scaffold_active && finalize_enclosed_scaffold(&mut structure, catalog, &scaffold_ids) {
            scaffold_active = false;
            // Scaffold removal can shift unit indices. Rebuild only the derived
            // accelerators; physical IDs and realized bonds remain authoritative.
            spatial_index =
                crate::construction_runtime::ConstructionSpatialIndex::new(&structure, catalog);
            occupancy = ConstructionEndpointOccupancy::default();
            for bond in &structure.bonds {
                occupancy.insert_bond(bond, &structure);
            }
            frontier = (0..structure.units.len()).collect();
        }
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
