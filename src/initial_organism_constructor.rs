//! Blueprint-free search for the first physically valid organism.
//!
//! The caller supplies only the resource catalog. Candidate structural requests
//! are transient search state; none becomes inherited developmental intent.

use crate::resources::{BaseResource, Form, Material, PhysicalState};
use crate::state::EnergyLedger;
use crate::structural_blueprint::{
    BlueprintConnection, BlueprintElement, BlueprintPlacement, StructuralBlueprint,
};
use crate::structure::Placement;

const SEARCH_ENERGY: f64 = 1.0e12;
const INNER_RING_SIDES: usize = 10;
const OUTER_RING_SIDES: usize = 24;
const ACQUISITION_SAMPLES: usize = 48;

#[derive(Clone, Debug)]
pub(crate) struct ValidConstruction {
    pub structure: crate::structure::OrganismStructure,
    pub energy: f64,
    pub acquired_resource_placements: Vec<(String, Placement)>,
}

fn candidate_annulus(
    resource: &BaseResource,
    spoke_resource: &BaseResource,
    inner_sides: usize,
    outer_sides: usize,
) -> Option<StructuralBlueprint> {
    let radius = resource.shape.form.bounding_radius();
    if !radius.is_finite() || radius <= 0.0 {
        return None;
    }

    let inner_radius = radius / (std::f64::consts::PI / inner_sides as f64).sin();
    // Leave a real accessible band between the genome wall and the outer wall.
    // Two one-unit line segments bridge that band.
    let outer_radius = inner_radius + 2.0 * radius + 2.0;

    let mut elements = Vec::with_capacity(inner_sides + outer_sides + 6);
    for index in 0..inner_sides {
        let angle = std::f64::consts::TAU * index as f64 / inner_sides as f64;
        elements.push(BlueprintElement {
            material: Material::free_base(resource.name.clone(), 1.0),
            placement: BlueprintPlacement {
                x: inner_radius * angle.cos(),
                y: inner_radius * angle.sin(),
                rotation_radians: angle + std::f64::consts::FRAC_PI_2,
            },
        });
    }

    let outer_start = elements.len();
    for index in 0..outer_sides {
        let angle = std::f64::consts::TAU * index as f64 / outer_sides as f64;
        elements.push(BlueprintElement {
            material: Material::free_base(resource.name.clone(), 1.0),
            placement: BlueprintPlacement {
                x: outer_radius * angle.cos(),
                y: outer_radius * angle.sin(),
                rotation_radians: angle + std::f64::consts::FRAC_PI_2,
            },
        });
    }

    let spoke_material = Material::free_base(spoke_resource.name.clone(), 1.0);
    let spoke_start = elements.len();
    for spoke in 0..3 {
        let angle = std::f64::consts::TAU * spoke as f64 / 3.0;
        for segment in 0..2 {
            let radial_offset = radius + 0.5 + segment as f64;
            elements.push(BlueprintElement {
                material: spoke_material.clone(),
                placement: BlueprintPlacement {
                    x: (inner_radius + radial_offset) * angle.cos(),
                    y: (inner_radius + radial_offset) * angle.sin(),
                    rotation_radians: angle,
                },
            });
        }
    }

    let mut connections = Vec::with_capacity(inner_sides + outer_sides + 9);
    for index in 0..inner_sides {
        connections.push(BlueprintConnection {
            element_a: index,
            element_b: (index + 1) % inner_sides,
        });
    }
    for index in 0..outer_sides {
        connections.push(BlueprintConnection {
            element_a: outer_start + index,
            element_b: outer_start + (index + 1) % outer_sides,
        });
    }
    for spoke in 0..3 {
        let first = spoke_start + spoke * 2;
        let second = first + 1;
        let inner_anchor = (spoke * inner_sides / 3) % inner_sides;
        let outer_anchor = outer_start + (spoke * outer_sides / 3) % outer_sides;
        connections.push(BlueprintConnection {
            element_a: inner_anchor,
            element_b: first,
        });
        connections.push(BlueprintConnection {
            element_a: first,
            element_b: second,
        });
        connections.push(BlueprintConnection {
            element_a: second,
            element_b: outer_anchor,
        });
    }

    Some(StructuralBlueprint::with_anchor_elements(
        elements,
        connections,
        vec![0],
    ))
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

    let steps = ACQUISITION_SAMPLES.max(1);
    for ix in 0..=steps {
        let x = min_x + (max_x - min_x) * ix as f64 / steps as f64;
        for iy in 0..=steps {
            let y = min_y + (max_y - min_y) * iy as f64 / steps as f64;
            for rotation_index in 0..4 {
                let placement = Placement {
                    x,
                    y,
                    rotation_radians: rotation_index as f64 * std::f64::consts::FRAC_PI_2,
                };
                let physical_result = crate::physical_material::PhysicalMaterial::realized(
                    Material::free_base(resource.name.clone(), 1.0),
                    vec![placement],
                    catalog,
                );
                let Some(physical) = physical_result else {
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
    let Some(spoke_resource) = catalog.iter().find(|resource| {
        resource.physical_state == PhysicalState::Rigid
            && matches!(resource.shape.form, Form::Line { .. })
            && resource.shape.is_valid()
    }) else {
        return Err("catalog does not contain a rigid line resource for structural bridging".into());
    };

    for resource in structural_candidates {
        if !matches!(
            resource.shape.form,
            Form::Rectangle { .. } | Form::RegularPolygon { .. } | Form::Polygon { .. }
        ) {
            continue;
        }

        let Some(candidate) = candidate_annulus(
            resource,
            spoke_resource,
            INNER_RING_SIDES,
            OUTER_RING_SIDES,
        )
        else {
            continue;
        };
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

        // The genome cavity is a successful construction milestone. It is not
        // subsequently reused as ordinary storage/acquisition space.
        let Some(cavity) = crate::cavity::analyze_genome_cavity(&structure, catalog)
            .ok()
            .flatten()
        else {
            continue;
        };
        if !cavity.qualifies() {
            continue;
        }

        // Only after the genome qualifies do we test the rest of the cell for
        // ordinary accessible interior space.
        let regions =
            crate::interior_geometry::find_accessible_interior_regions(&structure, catalog)
                .ok()
                .unwrap_or_default();
        if regions.is_empty() {
            continue;
        }

        let Some(mut acquired_resource_placements) =
            find_acquisition_set(&acquisition_candidates, &regions, catalog)
        else {
            continue;
        };

        let Some(water) = catalog.iter().find(|resource| resource.name == "Water") else {
            continue;
        };
        let Some(water_placement) = regions
            .iter()
            .find_map(|region| placement_fits_resource(water, region, catalog))
        else {
            continue;
        };
        acquired_resource_placements.push((water.name.clone(), water_placement));

        return Ok(ValidConstruction {
            structure,
            energy,
            acquired_resource_placements,
        });
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
