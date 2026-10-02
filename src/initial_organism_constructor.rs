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
// This is a computational search guard, not a biological requirement. It is
// deliberately expressed as a unit-search depth so no body-plan size is baked
// into organism validity.
const MAX_FREE_FORM_UNITS: usize = 32;
const ACQUISITION_SAMPLES: usize = 48;

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

fn free_form_search(
    structure: crate::structure::OrganismStructure,
    catalog: &[BaseResource],
    acquisition_candidates: &[&BaseResource],
    ledger: EnergyLedger,
    energy: f64,
    depth: usize,
) -> Option<ValidConstruction> {
    if let Some(acquired_resource_placements) =
        valid_construction(&structure, catalog, acquisition_candidates)
    {
        return Some(ValidConstruction {
            structure,
            energy,
            acquired_resource_placements,
        });
    }

    if depth >= MAX_FREE_FORM_UNITS {
        return None;
    }

    let rigid_resources = catalog
        .iter()
        .filter(|resource| resource.physical_state == PhysicalState::Rigid)
        .filter(|resource| resource.shape.is_valid());

    // No target unit, angle, topology, or material is prescribed here. Each
    // branch asks the physical construction machinery whether this material can
    // form one valid new bond to one currently realized unit.
    for anchor_index in 0..structure.units.len() {
        for resource in rigid_resources.clone() {
            let Some(material) = crate::physical_material::PhysicalMaterial::realized(
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

            let mut nodes = 0usize;
            let Some((next_structure, _indices, _part, _attempt, next_ledger, next_energy)) =
                crate::construction_runtime::try_attach_physical_material_bond_driven(
                    &structure,
                    anchor_index,
                    &material,
                    catalog,
                    &mut nodes,
                    &ledger,
                    energy,
                )
            else {
                continue;
            };

            if let Some(result) = free_form_search(
                next_structure,
                catalog,
                acquisition_candidates,
                next_ledger,
                next_energy,
                depth + 1,
            ) {
                return Some(result);
            }
        }
    }

    None
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

        if let Some(result) = free_form_search(
            structure,
            catalog,
            &acquisition_candidates,
            EnergyLedger::default(),
            SEARCH_ENERGY,
            1,
        ) {
            return Ok(result);
        }
    }

    Err("blueprint-free constructor found no physically valid organism within the physical search budget".into())
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
