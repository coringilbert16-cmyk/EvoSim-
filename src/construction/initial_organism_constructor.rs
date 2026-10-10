//! Deterministic physical construction baseline for the first valid organism.
//!
//! The current baseline has no developmental blueprint or target topology. It
//! uses a temporary deterministic Carbon scaffold only to prove the physical
//! construction, genome-cavity, and acquisition contracts. The final constructor
//! will replace this fixed scaffold with local free-form construction while
//! retaining the same physical bond authority.

use crate::resources::{BaseResource, Material, PhysicalState};
use crate::state::EnergyLedger;
use crate::structure::Placement;

const CONSTRUCTION_ENERGY: f64 = 1.0e12;
const INNER_RING_RADIUS: i32 = 3;
const OUTER_RING_RADIUS: i32 = 5;
const SPOKE_RADIUS: i32 = 4;
const SQRT_3: f64 = 1.7320508075688772935;
// Numerical guard for floating-point boundary equality; far below physical contact tolerance.
const GEOMETRY_EPSILON: f64 = 1e-9;

#[derive(Clone, Debug)]
pub(crate) struct ValidConstruction {
    pub structure: crate::structure::OrganismStructure,
    pub energy: f64,
    pub acquired_resource_placements: Vec<(String, Placement)>,
}

/// Axial hex coordinates are used only as a deterministic geometric
/// construction recipe. They are not a biological blueprint: every unit is
/// still instantiated and every connection is admitted through the normal
/// physical bond transaction.
fn axial_to_world(q: i32, r: i32) -> (f64, f64) {
    // Carbon's catalog hexagon has a vertex on the +x axis. These axial
    // basis vectors therefore match the actual tessellation of that shape.
    (
        1.5 * q as f64,
        (SQRT_3 * 0.5) * q as f64 + SQRT_3 * r as f64,
    )
}

fn hex_ring(radius: i32) -> Vec<(i32, i32)> {
    if radius <= 0 {
        return Vec::new();
    }
    let directions = [(0, 1), (1, 0), (1, -1), (0, -1), (-1, 0), (-1, 1)];
    let mut q = -radius;
    let mut r = 0;
    let mut result = Vec::with_capacity((radius * 6) as usize);
    for (dq, dr) in directions {
        for _ in 0..radius {
            result.push((q, r));
            q += dq;
            r += dr;
        }
    }
    result
}

/// Ask Bob for candidate neighbor poses, then admit only a pose that is close
/// to the scaffold's intended lattice point and passes live whole-structure
/// nonpenetration and contact checks. Bob never authorizes a bond.
fn bob_validated_neighbor_placement(
    structure: &crate::structure::OrganismStructure,
    anchor_index: usize,
    resource: &BaseResource,
    intended: (f64, f64),
    library: &crate::geometry_reference_library::GeometryLibrary,
    catalog: &[BaseResource],
) -> Option<Placement> {
    let anchor = structure.units.get(anchor_index)?;
    let anchor_material = anchor.material.parts.first()?.0.as_str();
    let mut suggestions =
        library.suggest_rigid_edge_placements(anchor_material, anchor.placement, resource, catalog);
    suggestions.sort_by(|a, b| {
        let score = |placement: Placement| {
            (placement.x - intended.0).hypot(placement.y - intended.1)
                + normalize_angle_for_constructor(placement.rotation_radians).abs() * 0.25
        };
        score(a.placement)
            .partial_cmp(&score(b.placement))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    for suggestion in suggestions {
        let mut proposed = suggestion.placement;
        // Bob proposes the local interface; the scaffold still defines the
        // broad cavity topology. Only near-equivalent lattice poses are eligible.
        if (proposed.x - intended.0).hypot(proposed.y - intended.1)
            > crate::combine_runtime::COMBINE_CONTACT_TOLERANCE
            || !rotation_preserves_form_symmetry(&resource.shape.form, proposed.rotation_radians)
        {
            continue;
        }
        // Snap only the near-equivalent proposal to the exact scaffold site
        // and canonical pose. The candidate is rechecked after snapping, so
        // this cannot bypass collision, contact, or bond validation.
        proposed.x = intended.0;
        proposed.y = intended.1;
        proposed.rotation_radians = 0.0;
        let Some(instance) = crate::physical_material::PhysicalMaterial::realized(
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
        let mut trial = structure.clone();
        let Some(indices) =
            crate::material_restoration::restore_material(&mut trial, &instance, proposed, catalog)
        else {
            continue;
        };
        let Some(&candidate_index) = indices.first() else {
            continue;
        };
        let candidate = &trial.units[candidate_index];
        let Some(candidate_shape) = candidate.shape(catalog) else {
            continue;
        };
        let candidate_part = crate::material_geometry::PlacedMaterialPart {
            part_index: candidate_index,
            form: candidate_shape.form.clone(),
            placement: candidate.placement,
        };
        let mut penetrates = false;
        for (index, unit) in trial.units.iter().enumerate() {
            if indices.contains(&index) {
                continue;
            }
            let Some(shape) = unit.shape(catalog) else {
                penetrates = true;
                break;
            };
            let existing_part = crate::material_geometry::PlacedMaterialPart {
                part_index: index,
                form: shape.form.clone(),
                placement: unit.placement,
            };
            if crate::material_geometry::placed_forms_penetrate(
                &candidate_part,
                &existing_part,
                GEOMETRY_EPSILON,
            ) {
                penetrates = true;
                break;
            }
        }
        if penetrates {
            continue;
        }
        let mut cache = crate::contact::ConnectionCompatibilityCache::new();
        let touches_anchor = crate::contact::connection_pair_candidates_cached(
            &trial,
            anchor_index,
            candidate_index,
            catalog,
            &mut cache,
        )
        .into_iter()
        .any(|contact| {
            contact.distance <= crate::combine_runtime::COMBINE_CONTACT_TOLERANCE
                && contact.available_a
                && contact.available_b
        });
        if touches_anchor {
            return Some(proposed);
        }
    }
    None
}

fn normalize_angle_for_constructor(angle: f64) -> f64 {
    (angle + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU) - std::f64::consts::PI
}

/// A rotationally symmetric shape may have a different numeric pose while
/// occupying exactly the same geometry. Preserve those valid equivalent poses.
fn rotation_preserves_form_symmetry(form: &crate::resources::Form, rotation: f64) -> bool {
    if normalize_angle_for_constructor(rotation).abs() <= 0.1 {
        return true;
    }
    let Some(vertices) = form.polygon_vertices() else {
        return false;
    };
    let (s, c) = rotation.sin_cos();
    vertices.iter().all(|&(x, y)| {
        let rotated = (x * c - y * s, x * s + y * c);
        vertices
            .iter()
            .any(|&(other_x, other_y)| (rotated.0 - other_x).hypot(rotated.1 - other_y) <= 1e-7)
    })
}

fn add_unit(
    structure: &mut crate::structure::OrganismStructure,
    resource: &BaseResource,
    center: (f64, f64),
    anchor_index: Option<usize>,
    bob_library: Option<&crate::geometry_reference_library::GeometryLibrary>,
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
    let proposed_placement = anchor_index
        .and_then(|anchor| {
            bob_library.and_then(|library| {
                bob_validated_neighbor_placement(
                    structure, anchor, resource, center, library, catalog,
                )
            })
        })
        .unwrap_or(Placement {
            x: center.0,
            y: center.1,
            rotation_radians: 0.0,
        });
    let indices = crate::material_restoration::restore_material(
        structure,
        &instance,
        proposed_placement,
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

fn bond_units(
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
        let (_, _, investment, _) = crate::combine_runtime::selected_candidate_evaluation(
            structure, unit_a, unit_b, candidate, catalog,
        )
        .ok_or_else(|| format!("physical bond candidate {unit_a}-{unit_b} failed evaluation"))?;

        // Recompute candidates on the next iteration after this transaction
        // changes endpoint availability.
        crate::combine_runtime::form_selected_bond_diagnostic(
            structure, unit_a, unit_b, candidate, investment, catalog, &mut cache, ledger, energy,
        )
        .map_err(|reason| {
            format!("physical bond transaction {unit_a}-{unit_b} failed: {reason}")
        })?;

        used_a.push(endpoint_a);
        used_b.push(endpoint_b);
        bonded = true;
    }

    Ok(())
}

/// Place the fixed geometric scaffold directly, while using the ordinary
/// physical bond transaction for every permanent connection. There is no
/// speculative placement search and no recursive body-plan search.
fn construct_scaffold(
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

    let mut structure = crate::structure::OrganismStructure::new();
    // Load once. An absent/empty persistent store remains valid: Bob's API
    // derives non-persistent local suggestions and every suggestion is rechecked.
    let bob_library = crate::geometry_reference_library::open_default_library().ok();
    let mut ledger = EnergyLedger::default();
    let mut energy = CONSTRUCTION_ENERGY;

    // The inner ring closes the genome cavity. Its radius is deliberately large
    // enough that the realized enclosed area exceeds the three-Carbon minimum
    // reference; a radius-two ring leaves only a one-Carbon-scale central void.
    let inner = hex_ring(INNER_RING_RADIUS);
    let mut inner_indices = Vec::with_capacity(inner.len());
    for coordinate in inner {
        inner_indices.push(add_unit(
            &mut structure,
            carbon,
            axial_to_world(coordinate.0, coordinate.1),
            inner_indices.last().copied(),
            bob_library.as_ref(),
            catalog,
        )?);
    }
    for i in 0..inner_indices.len() {
        bond_units(
            &mut structure,
            inner_indices[i],
            inner_indices[(i + 1) % inner_indices.len()],
            catalog,
            &mut ledger,
            &mut energy,
        )?;
    }

    // The genome is a construction milestone, not a post-hoc property of the
    // completed scaffold. Nothing outside this ring is needed to qualify it.
    let cavity = crate::cavity::analyze_genome_cavity(&structure, catalog)
        .map_err(|error| format!("genome cavity analysis failed: {error}"))?
        .filter(|cavity| cavity.qualifies())
        .ok_or_else(|| {
            let pair_counts = (0..inner_indices.len())
                .map(|i| {
                    let a = structure.units[inner_indices[i]].physical_id;
                    let b = structure.units[inner_indices[(i + 1) % inner_indices.len()]].physical_id;
                    structure
                        .bonds
                        .iter()
                        .filter(|bond| {
                            (bond.endpoint_a.constituent_id == a
                                && bond.endpoint_b.constituent_id == b)
                                || (bond.endpoint_a.constituent_id == b
                                    && bond.endpoint_b.constituent_id == a)
                        })
                        .count()
                })
                .collect::<Vec<_>>();
            let regions =
                crate::interior_geometry::find_enclosed_regions(&structure, catalog);
            let max_region_area = regions
                .iter()
                .map(|region| region.area)
                .fold(0.0_f64, f64::max);
            let first_a = structure.units[inner_indices[0]].physical_id;
            let first_b = structure.units[inner_indices[1]].physical_id;
            let first_pair_bonds = structure
                .bonds
                .iter()
                .filter(|bond| {
                    (bond.endpoint_a.constituent_id == first_a
                        && bond.endpoint_b.constituent_id == first_b)
                        || (bond.endpoint_a.constituent_id == first_b
                            && bond.endpoint_b.constituent_id == first_a)
                })
                .map(|bond| (bond.endpoint_a.location, bond.endpoint_b.location))
                .collect::<Vec<_>>();
            format!(
                "inner construction phase did not produce a qualifying genome cavity: bonds={}, pair_counts={pair_counts:?}, enclosed_regions={}, max_region_area={max_region_area}, first_pair_bonds={first_pair_bonds:?}",
                structure.bonds.len(),
                regions.len()
            )
        })?;
    if cavity.boundary_units.is_empty() {
        return Err("qualifying genome cavity has no physical boundary".into());
    }

    // Build six radial supports one ring outside the cavity boundary. They remain
    // part of the structural path from the genome boundary to the outer boundary.
    let mut spokes = Vec::with_capacity(6);
    let spoke_coordinates = hex_ring(SPOKE_RADIUS);
    let outer_coordinates = hex_ring(OUTER_RING_RADIUS);
    for side in 0..6 {
        let coordinate = spoke_coordinates[side * SPOKE_RADIUS as usize];
        let inner_position = side * INNER_RING_RADIUS as usize;
        let index = add_unit(
            &mut structure,
            carbon,
            axial_to_world(coordinate.0, coordinate.1),
            Some(inner_indices[inner_position]),
            bob_library.as_ref(),
            catalog,
        )?;
        bond_units(
            &mut structure,
            index,
            inner_indices[inner_position],
            catalog,
            &mut ledger,
            &mut energy,
        )?;
        spokes.push(index);
    }

    // Grow the outer ring from the spokes. Every new unit is attached before
    // the next one is created; the final six closure bonds are ordinary
    // forward construction transactions between already realized units.
    let mut outer_indices = Vec::with_capacity(outer_coordinates.len());
    for (i, coordinate) in outer_coordinates.iter().enumerate() {
        let index = add_unit(
            &mut structure,
            carbon,
            axial_to_world(coordinate.0, coordinate.1),
            if i == 0 {
                Some(spokes[0])
            } else {
                outer_indices.last().copied()
            },
            bob_library.as_ref(),
            catalog,
        )?;
        if i == 0 {
            bond_units(
                &mut structure,
                index,
                spokes[0],
                catalog,
                &mut ledger,
                &mut energy,
            )?;
        } else {
            bond_units(
                &mut structure,
                index,
                outer_indices[i - 1],
                catalog,
                &mut ledger,
                &mut energy,
            )?;
        }
        outer_indices.push(index);
    }
    bond_units(
        &mut structure,
        outer_indices[0],
        outer_indices[outer_indices.len() - 1],
        catalog,
        &mut ledger,
        &mut energy,
    )?;

    // Connect the remaining spokes to the outer boundary. The exact radial
    // correspondence is selected from the physical contact graph rather than
    // by inventing a special construction bond.
    for side in 1..6 {
        let outer_coordinate = side * OUTER_RING_RADIUS as usize;
        bond_units(
            &mut structure,
            spokes[side],
            outer_indices[outer_coordinate],
            catalog,
            &mut ledger,
            &mut energy,
        )?;
    }

    Ok((structure, ledger, energy))
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

    let (structure, _ledger, energy) = construct_scaffold(catalog)?;
    let acquired_resource_placements =
        valid_construction(&structure, catalog, &acquisition_candidates).ok_or_else(|| {
            "deterministic construction scaffold did not satisfy viability".to_string()
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
    fn bob_neighbor_suggestions_reach_initial_constructor_physical_validation() {
        let catalog = crate::resources::default_catalog();
        let carbon = catalog
            .iter()
            .find(|resource| resource.name == "Carbon")
            .expect("default catalog includes Carbon");
        let mut structure = crate::structure::OrganismStructure::new();
        add_unit(&mut structure, carbon, (0.0, 0.0), None, None, &catalog)
            .expect("anchor Carbon should realize");
        let library = crate::geometry_reference_library::open_default_library()
            .expect("default Bob library should open, including an empty library");
        let intended = axial_to_world(1, 0);
        let raw_suggestions = library.suggest_rigid_edge_placements(
            "Carbon",
            structure.units[0].placement,
            carbon,
            &catalog,
        );
        let suggestion_poses = raw_suggestions
            .iter()
            .map(|suggestion| {
                (
                    suggestion.placement.x,
                    suggestion.placement.y,
                    suggestion.placement.rotation_radians,
                )
            })
            .collect::<Vec<_>>();
        let proposed =
            bob_validated_neighbor_placement(&structure, 0, carbon, intended, &library, &catalog)
                .unwrap_or_else(|| panic!(
                    "Bob had no suggestion accepted by live physical validation; raw poses={suggestion_poses:?}"
                ));
        assert!(
            (proposed.x - intended.0).hypot(proposed.y - intended.1)
                <= crate::combine_runtime::COMBINE_CONTACT_TOLERANCE,
            "accepted Bob pose must preserve the intended adjacent lattice site"
        );
        let instance = crate::physical_material::PhysicalMaterial::realized(
            Material::free_base(carbon.name.clone(), 1.0),
            vec![Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .expect("candidate material should realize");
        let mut trial = structure.clone();
        let indices = crate::material_restoration::restore_material(
            &mut trial, &instance, proposed, &catalog,
        )
        .expect("accepted Bob pose should restore into the trial structure");
        let candidate = *indices.first().expect("one Carbon constituent");
        for (index, unit) in trial.units.iter().enumerate() {
            if indices.contains(&index) {
                continue;
            }
            let candidate_shape = trial.units[candidate]
                .shape(&catalog)
                .expect("candidate shape");
            let existing_shape = unit.shape(&catalog).expect("existing shape");
            assert!(
                !crate::material_geometry::placed_forms_penetrate(
                    &crate::material_geometry::PlacedMaterialPart {
                        part_index: candidate,
                        form: candidate_shape.form.clone(),
                        placement: trial.units[candidate].placement,
                    },
                    &crate::material_geometry::PlacedMaterialPart {
                        part_index: index,
                        form: existing_shape.form.clone(),
                        placement: unit.placement,
                    },
                    GEOMETRY_EPSILON,
                ),
                "accepted Bob proposal must not penetrate existing structure"
            );
        }
        let mut cache = crate::contact::ConnectionCompatibilityCache::new();
        assert!(
            crate::contact::connection_pair_candidates_cached(
                &trial, 0, candidate, &catalog, &mut cache,
            )
            .into_iter()
            .any(|contact| {
                contact.distance <= crate::combine_runtime::COMBINE_CONTACT_TOLERANCE
                    && contact.available_a
                    && contact.available_b
            }),
            "Bob proposal must reach and pass live contact validation"
        );
    }

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
