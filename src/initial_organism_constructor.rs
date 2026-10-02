//! Blueprint-free local assembler for the first organism.
//!
//! The initial constructor does not solve a final topology. It grows rigid
//! physical pieces from a local construction frontier, using normal geometry
//! and COMBINE/bond authority. Cavity and resource viability are evaluated
//! only after the realized structure has been assembled.

use std::collections::VecDeque;

use crate::resources::{BaseResource, Material, PhysicalState};
use crate::state::EnergyLedger;
use crate::structure::Placement;

const ASSEMBLY_TARGET: usize = 400;
const ASSEMBLY_ENERGY: f64 = 1.0e12;
const CONTACT_TOLERANCE: f64 = 0.05;

#[derive(Clone, Debug)]
pub(crate) struct ValidConstruction {
    pub structure: crate::structure::OrganismStructure,
    pub energy: f64,
    pub acquired_resource_placements: Vec<(String, Placement)>,
}

fn rigid_materials(catalog: &[BaseResource]) -> Vec<crate::physical_material::PhysicalMaterial> {
    catalog
        .iter()
        .filter(|resource| resource.physical_state == PhysicalState::Rigid)
        .filter(|resource| resource.shape.is_valid())
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
        .collect()
}

fn non_water_resources(catalog: &[BaseResource]) -> Vec<&BaseResource> {
    catalog
        .iter()
        .filter(|resource| resource.name != "Water")
        .filter(|resource| resource.shape.is_valid())
        .collect()
}

/// Check only the local neighborhood of an attachment candidate.
///
/// The constructor intentionally does not scan the whole organism. The anchor
/// and its direct bonded neighbors are the only existing geometry that can
/// affect this local assembly decision.
fn penetrates_local_neighborhood(
    structure: &crate::structure::OrganismStructure,
    candidate_shape: &crate::resources::Shape,
    candidate_placement: Placement,
    anchor: usize,
    local_neighbors: &[usize],
    catalog: &[BaseResource],
) -> bool {
    let candidate = crate::material_geometry::PlacedMaterialPart {
        part_index: 0,
        form: candidate_shape.form.clone(),
        placement: candidate_placement,
    };

    local_neighbors
        .iter()
        .copied()
        .chain(std::iter::once(anchor))
        .any(|index| {
        let Some(unit) = structure.units.get(index) else {
            return true;
        };
        let Some(shape) = unit.shape(catalog) else {
            return true;
        };
        let existing = crate::material_geometry::PlacedMaterialPart {
            part_index: index + 1,
            form: shape.form.clone(),
            placement: unit.placement,
        };
        crate::material_geometry::placed_forms_penetrate(&candidate, &existing, 0.0)
    })
}

/// Try one new physical piece against one frontier anchor.
///
/// The only speculative state is the single newly restored piece and its
/// immediate bond(s). A failed candidate is discarded by truncating those
/// additions; no whole-organism clone or future branch is created.
fn attach_local_piece(
    structure: &mut crate::structure::OrganismStructure,
    anchor: usize,
    local_neighbors: &[usize],
    material: &crate::physical_material::PhysicalMaterial,
    cache: &mut crate::contact::ConnectionCompatibilityCache,
    catalog: &[BaseResource],
    ledger: &mut EnergyLedger,
    energy: &mut f64,
) -> Option<(usize, Option<usize>)> {
    let resource_name = material.material.parts.first()?.0.as_str();
    let resource = catalog
        .iter()
        .find(|candidate| candidate.name == resource_name)?;
    let anchor_placement = structure.units.get(anchor)?.placement;

    // Initial construction uses only the local rigid-to-rigid geometry authority.
    // No developmental construction-runtime search is involved here.
    let Some(anchor_unit) = structure.units.get(anchor) else {
        return None;
    };
    let Some(anchor_shape) = anchor_unit.shape(catalog) else {
        return None;
    };
    let origins = crate::rigid_boundary::surface_alignment_placements(
        &anchor_shape,
        anchor_placement,
        &resource.shape,
        Placement {
            x: 0.0,
            y: 0.0,
            rotation_radians: 0.0,
        },
    );

    for origin in origins {
        if penetrates_local_neighborhood(
            structure,
            &resource.shape,
            origin,
            anchor,
            local_neighbors,
            catalog,
        ) {
            continue;
        }

        let previous_units = structure.units.len();
        let previous_bonds = structure.bonds.len();
        let previous_ledger = *ledger;
        let previous_energy = *energy;

        let Some(indices) = crate::material_restoration::restore_material_in_place(
            structure, material, origin, catalog,
        ) else {
            continue;
        };
        let Some(new_index) = indices.first().copied() else {
            structure.units.truncate(previous_units);
            structure.bonds.truncate(previous_bonds);
            *ledger = previous_ledger;
            *energy = previous_energy;
            continue;
        };

        let candidates = crate::contact::connection_pair_candidates_cached(
            structure, anchor, new_index, catalog, cache,
        );

        let mut bonded = false;
        for candidate in candidates {
            if candidate.distance > CONTACT_TOLERANCE
                || !candidate.available_a
                || !candidate.available_b
            {
                continue;
            }
            let Some((_, _, _, investment, _)) =
                crate::combine_runtime::selected_candidate_evaluation(
                    structure, anchor, new_index, candidate, catalog,
                )
            else {
                continue;
            };
            if let Some(attempt) = crate::combine_runtime::form_selected_bond_in_place(
                structure, anchor, new_index, candidate, investment, catalog, cache, ledger, energy,
            ) {
                let id_a = structure.units[anchor].physical_id;
                let id_b = structure.units[new_index].physical_id;
                cache.record_bond(
                    id_a,
                    attempt.endpoint_a,
                    id_b,
                    attempt.endpoint_b,
                    attempt.bond_energy,
                );
                bonded = true;
                break;
            }
        }

        if !bonded {
            structure.units.truncate(previous_units);
            structure.bonds.truncate(previous_bonds);
            *ledger = previous_ledger;
            *energy = previous_energy;
            continue;
        }

        // A new piece may also contact a direct neighbor of the anchor. If the
        // physical contact is already present, take that second local bond
        // immediately. This is how closed local structures can emerge without
        // a global closure solver.
        let mut closed_target = None;
        for &target in local_neighbors {
            let candidates = crate::contact::connection_pair_candidates_cached(
                structure, target, new_index, catalog, &mut cache,
            );
            let mut closed = false;
            for candidate in candidates {
                if candidate.distance > CONTACT_TOLERANCE
                    || !candidate.available_a
                    || !candidate.available_b
                {
                    continue;
                }
                let Some((_, _, _, investment, _)) =
                    crate::combine_runtime::selected_candidate_evaluation(
                        structure, target, new_index, candidate, catalog,
                    )
                else {
                    continue;
                };
                if let Some(attempt) = crate::combine_runtime::form_selected_bond_in_place(
                    structure, target, new_index, candidate, investment, catalog, cache, ledger,
                    energy,
                ) {
                    let id_a = structure.units[target].physical_id;
                    let id_b = structure.units[new_index].physical_id;
                    cache.record_bond(
                        id_a,
                        attempt.endpoint_a,
                        id_b,
                        attempt.endpoint_b,
                        attempt.bond_energy,
                    );
                    closed = true;
                    closed_target = Some(target);
                    break;
                }
            }
            if closed {
                break;
            }
        }

        return Some((new_index, closed_target));
    }

    None
}

/// Grow a connected structure through a FIFO frontier.
///
/// Each successful attachment adds the new piece to the frontier. Frontier
/// growth is local: adding one piece does not enumerate all existing units or
/// all future structures.
fn assemble_local(
    mut structure: crate::structure::OrganismStructure,
    materials: &[crate::physical_material::PhysicalMaterial],
    catalog: &[BaseResource],
) -> Option<(crate::structure::OrganismStructure, EnergyLedger, f64)> {
    let mut frontier = VecDeque::new();
    let mut adjacency: Vec<Vec<usize>> = Vec::new();
    if !structure.units.is_empty() {
        frontier.push_back(0);
        adjacency.push(Vec::new());
    }

    let mut ledger = EnergyLedger::default();
    let mut energy = ASSEMBLY_ENERGY;
    let mut cache = crate::contact::ConnectionCompatibilityCache::new_complete();

    while structure.units.len() < ASSEMBLY_TARGET {
        let frontier_len = frontier.len();
        if frontier_len == 0 {
            return None;
        }

        let mut attached = false;
        for _ in 0..frontier_len {
            let Some(anchor) = frontier.pop_front() else {
                break;
            };

            for material in materials.iter().cycle().take(materials.len()) {
                if let Some((new_index, closed_target)) = attach_local_piece(
                    &mut structure,
                    anchor,
                    &adjacency[anchor],
                    material,
                    &mut cache,
                    catalog,
                    &mut ledger,
                    &mut energy,
                ) {
                    adjacency.push(vec![anchor]);
                    adjacency[anchor].push(new_index);
                    if let Some(target) = closed_target {
                        adjacency[new_index].push(target);
                        adjacency[target].push(new_index);
                    }
                    frontier.push_back(anchor);
                    frontier.push_back(new_index);
                    attached = true;
                    break;
                }
            }

            if attached {
                break;
            }
        }

        if !attached {
            return None;
        }
    }

    Some((structure, ledger, energy))
}

fn placement_fits_resource(
    resource: &BaseResource,
    region: &crate::interior_geometry::EnclosedRegion,
    catalog: &[BaseResource],
) -> Option<Placement> {
    // Acquisition placement is deliberately a completion-time check. It is
    // not part of local construction and therefore never runs during assembly.
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

    let samples = 12;
    for ix in 0..=samples {
        let x = min_x + (max_x - min_x) * ix as f64 / samples as f64;
        for iy in 0..=samples {
            let y = min_y + (max_y - min_y) * iy as f64 / samples as f64;
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

fn find_acquisition_set(
    candidates: &[&BaseResource],
    regions: &[crate::interior_geometry::EnclosedRegion],
    catalog: &[BaseResource],
) -> Option<Vec<(String, Placement)>> {
    // Viability is allowed to choose any three distinct non-Water resource
    // categories. The catalog is tiny, so enumerate only the combinations of
    // three here, after assembly; this is not construction search.
    for first in 0..candidates.len() {
        for second in (first + 1)..candidates.len() {
            for third in (second + 1)..candidates.len() {
                let selected = [candidates[first], candidates[second], candidates[third]];
                let mut result = Vec::with_capacity(3);
                let mut viable = true;

                for resource in selected {
                    let Some(placement) = regions
                        .iter()
                        .find_map(|region| placement_fits_resource(resource, region, catalog))
                    else {
                        viable = false;
                        break;
                    };
                    result.push((resource.name.clone(), placement));
                }

                if viable {
                    return Some(result);
                }
            }
        }
    }

    None
}

fn validate_completed_structure(
    structure: &crate::structure::OrganismStructure,
    catalog: &[BaseResource],
    acquisition: &[&BaseResource],
) -> Option<Vec<(String, Placement)>> {
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

pub(crate) fn construct_valid(catalog: &[BaseResource]) -> Result<ValidConstruction, String> {
    let materials = rigid_materials(catalog);
    if materials.is_empty() {
        return Err("no rigid resource can seed a physical organism".into());
    }

    let acquisition = non_water_resources(catalog);
    if acquisition.len() < 3 {
        return Err("catalog does not contain three non-water acquisition resources".into());
    }
    if !catalog.iter().any(|resource| resource.name == "Water") {
        return Err("catalog does not contain Water".into());
    }

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

        if let Some((structure, _ledger, energy)) = assemble_local(structure, &materials, catalog) {
            if let Some(acquired_resource_placements) =
                validate_completed_structure(&structure, catalog, &acquisition)
            {
                return Ok(ValidConstruction {
                    structure,
                    energy,
                    acquired_resource_placements,
                });
            }
        }
    }

    Err("local frontier assembler did not realize a viable organism".into())
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
