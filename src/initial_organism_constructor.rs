//! Blueprint-free search for the first physically valid organism.
//!
//! The caller supplies only the resource catalog. Candidate structural requests
//! are transient search state; none becomes inherited developmental intent.

use crate::resources::{BaseResource, Form, Material, PhysicalState};
use crate::state::{EnergyLedger, Organism};
use crate::structural_blueprint::{
    BlueprintConnection, BlueprintElement, BlueprintPlacement, StructuralBlueprint,
};
use crate::structure::Placement;

const SEARCH_ENERGY: f64 = 1.0e12;
const MIN_RING_SIDES: usize = 3;
const MAX_RING_SIDES: usize = 24;
const ACQUISITION_SAMPLES: usize = 48;

#[derive(Clone, Debug)]
pub(crate) struct ValidConstruction {
    pub structure: crate::structure::OrganismStructure,
    pub energy: f64,
    pub acquired_resource_placements: Vec<(String, Placement)>,
}

fn candidate_ring(
    resource: &BaseResource,
    sides: usize,
) -> StructuralBlueprint {
    let radius = resource.shape.form.bounding_radius();
    let ring_radius = radius / (std::f64::consts::PI / sides as f64).sin();
    let elements = (0..sides)
        .map(|index| {
            let angle = std::f64::consts::TAU * index as f64 / sides as f64;
            BlueprintElement {
                material: Material::free_base(resource.name.clone(), 1.0),
                placement: BlueprintPlacement {
                    x: ring_radius * angle.cos(),
                    y: ring_radius * angle.sin(),
                    rotation_radians: angle + std::f64::consts::FRAC_PI_2,
                },
            }
        })
        .collect::<Vec<_>>();
    let connections = (0..sides)
        .map(|index| BlueprintConnection {
            element_a: index,
            element_b: (index + 1) % sides,
        })
        .collect();
    StructuralBlueprint::with_anchor_elements(elements, connections, vec![0])
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
            (
                min_x.min(x),
                max_x.max(x),
                min_y.min(y),
                max_y.max(y),
            )
        },
    );

    if !min_x.is_finite() || !max_x.is_finite() || !min_y.is_finite() || !max_y.is_finite() {
        return None;
    }

    let steps = ACQUISITION_SAMPLES.max(1);
    for ix in 0..=steps {
        let x = min_x + (max_x - min_x) * ix as f64 / steps as f64;
        for iy in 0..=steps {
            let y = min_y + (max_y - min_y) * iy as f64 / steps as f64;
            for rotation_index in 0..4 {
                let placement = Placement {
                    x,
                    y,
                    rotation_radians: rotation_index as f64
                        * std::f64::consts::FRAC_PI_2,
                };
                let Ok_or_none = crate::physical_material::PhysicalMaterial::realized(
                    Material::free_base(resource.name.clone(), 1.0),
                    vec![placement],
                    catalog,
                );
                let Some(physical) = Ok_or_none else {
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

fn required_acquisition_resources<'a>(
    catalog: &'a [BaseResource],
) -> Vec<&'a BaseResource> {
    catalog
        .iter()
        .filter(|resource| resource.name != "Water")
        .filter(|resource| resource.shape.is_valid())
        .take(3)
        .collect()
}

pub(crate) fn construct_valid(
    catalog: &[BaseResource],
) -> Result<ValidConstruction, String> {
    let structural_candidates = catalog
        .iter()
        .filter(|resource| resource.physical_state == PhysicalState::Rigid)
        .filter(|resource| resource.shape.is_valid())
        .collect::<Vec<_>>();

    if structural_candidates.is_empty() {
        return Err("no rigid resource can seed a physical organism".into());
    }

    let required_resources = required_acquisition_resources(catalog);
    if required_resources.len() < 3 {
        return Err("catalog does not contain three non-water acquisition resources".into());
    }

    for resource in structural_candidates {
        if !matches!(
            resource.shape.form,
            Form::Rectangle { .. }
                | Form::RegularPolygon { .. }
                | Form::Polygon { .. }
        ) {
            continue;
        }

        for sides in MIN_RING_SIDES..=MAX_RING_SIDES {
            let candidate = candidate_ring(resource, sides);
            if !candidate.is_valid() {
                continue;
            }

            let mut ledger = EnergyLedger::default();
            let mut energy = SEARCH_ENERGY;
            let Ok((structure, _heat)) =
                crate::construction_runtime::construct_blueprint_bond_driven(
                    &candidate,
                    catalog,
                    &mut ledger,
                    &mut energy,
                )
            else {
                continue;
            };

            let Some(cavity) =
                crate::cavity::analyze_genome_cavity(&structure, catalog).ok().flatten()
            else {
                continue;
            };
            if !cavity.qualifies() {
                continue;
            }

            let regions = crate::interior_geometry::find_accessible_interior_regions(
                &structure,
                catalog,
            )
            .ok()
            .unwrap_or_default();
            if regions.is_empty() {
                continue;
            }

            let mut acquired_resource_placements = Vec::new();
            let mut acquisition_failed = false;
            for required in required_resources.iter().copied() {
                let placement = regions
                    .iter()
                    .find_map(|region| placement_fits_resource(required, region, catalog));
                let Some(placement) = placement else {
                    acquisition_failed = true;
                    break;
                };
                acquired_resource_placements.push((required.name.clone(), placement));
            }

            let Some(water) = catalog.iter().find(|resource| resource.name == "Water") else {
                continue;
            };
            let water_placement = regions
                .iter()
                .find_map(|region| placement_fits_resource(water, region, catalog));
            let Some(water_placement) = water_placement else {
                acquisition_failed = true;
                continue;
            };
            acquired_resource_placements.push((water.name.clone(), water_placement));

            if acquisition_failed {
                continue;
            }

            return Ok(ValidConstruction {
                structure,
                energy,
                acquired_resource_placements,
            });
        }
    }

    Err("blueprint-free constructor found no physically valid organism".into())
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
        let cavity =
            crate::cavity::analyze_genome_cavity(&result.structure, &catalog)
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
