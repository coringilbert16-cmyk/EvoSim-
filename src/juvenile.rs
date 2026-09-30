//! Deterministic construction of one valid juvenile realization.
//!
//! The original viable seed realization is retained only as a physical
//! construction calibration baseline. Developmental fields, not this baseline,
//! determine descendant growth and realized architecture.
use crate::juvenile_requirements::{validate_realized_juvenile, JuvenileViabilityRequirements};
use crate::resources::BaseResource;
use crate::state::EnergyLedger;
use crate::structural_blueprint::StructuralBlueprint;
use crate::structure::OrganismStructure;

/// Confirmed original-seed construction baseline.
///
/// This is a construction calibration artifact only. It is not serialized into
/// the genome and does not prescribe descendant topology or geometry.
pub(crate) fn confirmed_seed_baseline(
    catalog: &[BaseResource],
) -> Result<StructuralBlueprint, String> {
    use crate::resources::Material;
    use crate::structural_blueprint::{BlueprintConnection, BlueprintElement, BlueprintPlacement};

    let nitrogen = catalog
        .iter()
        .find(|resource| resource.name == "Nitrogen")
        .ok_or_else(|| "catalog is missing seed Nitrogen".to_string())?;
    let carbon = catalog
        .iter()
        .find(|resource| resource.name == "Carbon")
        .ok_or_else(|| "catalog is missing seed Carbon".to_string())?;
    let hydrogen = catalog
        .iter()
        .find(|resource| resource.name == "Hydrogen")
        .ok_or_else(|| "catalog is missing seed Hydrogen".to_string())?;
    let sulfur = catalog
        .iter()
        .find(|resource| resource.name == "Sulfur")
        .ok_or_else(|| "catalog is missing seed Sulfur".to_string())?;

    // The seed is a continuous mesh, not nested storage shells. Different rigid
    // materials are interleaved around one protected genome cavity. Any other
    // voids are ordinary consequences of the mesh and may contain environmental
    // material later.
    let _ = (carbon, hydrogen, sulfur);
    let radius = 1.65;
    let positions = [
        (0.0, radius),
        (-radius * 0.866_025_403_8, radius * 0.5),
        (-radius * 0.866_025_403_8, -radius * 0.5),
        (0.0, -radius),
        (radius * 0.866_025_403_8, -radius * 0.5),
        (radius * 0.866_025_403_8, radius * 0.5),
    ];
    let materials = [
        "Carbon", "Nitrogen", "Sulfur", "Carbon", "Nitrogen", "Hydrogen",
    ];
    let mut elements = Vec::with_capacity(positions.len());
    for ((x, y), material) in positions.into_iter().zip(materials) {
        if !catalog.iter().any(|resource| resource.name == material) {
            return Err(format!("catalog is missing seed {material}"));
        }
        elements.push(BlueprintElement {
            material: Material::free_base(material, 1.0),
            placement: BlueprintPlacement {
                x,
                y,
                rotation_radians: y.atan2(x),
            },
        });
    }

    // The ring is intentionally connected all the way around. The enclosed
    // center is the genome cavity; it is the only cavity the seed explicitly
    // creates. There is no separate storage chamber or hollow outer shell.
    let connections = (0..elements.len())
        .map(|index| BlueprintConnection {
            element_a: index,
            element_b: (index + 1) % elements.len(),
        })
        .collect::<Vec<_>>();

    let baseline = StructuralBlueprint::with_anchor_elements(elements, connections, (0..6).collect());
    baseline.validate()?;
    let _ = nitrogen;
    Ok(baseline)
}

fn largest_rigid_connection_span(catalog: &[BaseResource]) -> f64 {
    let mut points = Vec::<(f64, f64)>::new();

    for resource in catalog {
        if resource.physical_state == crate::resources::PhysicalState::Fluid {
            continue;
        }
        match &resource.shape.form {
            crate::resources::Form::Line { length } => {
                points.push((-length / 2.0, 0.0));
                points.push((length / 2.0, 0.0));
            }
            crate::resources::Form::Rectangle { width, height } => {
                let hw = width / 2.0;
                let hh = height / 2.0;
                points.extend([(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)]);
            }
            crate::resources::Form::RegularPolygon { sides, radius } => {
                for i in 0..*sides as usize {
                    let angle = i as f64 * std::f64::consts::TAU / *sides as f64;
                    points.push((radius * angle.cos(), radius * angle.sin()));
                }
            }
            crate::resources::Form::Polygon { vertices } => points.extend(vertices.iter().copied()),
            crate::resources::Form::Circle { .. } | crate::resources::Form::Fluid { .. } => {}
        }
    }

    points
        .iter()
        .enumerate()
        .flat_map(|(i, &(ax, ay))| {
            points
                .iter()
                .skip(i + 1)
                .map(move |&(bx, by)| (ax - bx).hypot(ay - by))
        })
        .fold(0.0, f64::max)
}

// This is a deterministic calibration constant for the current resource catalog.
// Callers on the simulation hot path should compute it once and reuse it.
pub(crate) fn confirmed_seed_scale_reference(
    catalog: &[BaseResource],
) -> Result<(f64, f64), String> {
    let baseline = confirmed_seed_baseline(catalog)?;
    let (structure, _, _) = realize_initial(&baseline, catalog)?;
    let mass = structure
        .units
        .iter()
        .map(|unit| unit.material.mass(catalog))
        .sum::<f64>();
    let length = structure
        .units
        .iter()
        .map(|unit| unit.placement.x.hypot(unit.placement.y))
        .fold(0.0, f64::max);
    if !mass.is_finite() || mass <= 0.0 || !length.is_finite() || length <= 0.0 {
        return Err("confirmed seed scale reference is invalid".into());
    }
    Ok((mass, length))
}

fn realize_declared_units(
    blueprint: &StructuralBlueprint,
    catalog: &[BaseResource],
) -> Result<OrganismStructure, String> {
    use crate::structure::StructuralUnit;
    let mut structure = OrganismStructure::new();
    for element in &blueprint.elements {
        let mut unit = StructuralUnit::from_material(
            element.material.clone(),
            crate::structure::Placement {
                x: element.placement.x,
                y: element.placement.y,
                rotation_radians: element.placement.rotation_radians,
            },
        )
        .ok_or_else(|| "confirmed seed contains invalid material".to_string())?;
        if !unit.realize_default_geometry(catalog) {
            return Err("confirmed seed contains unrealizable geometry".into());
        }
        structure.add_unit(unit);
    }
    Ok(structure)
}

fn form_declared_bonds(
    mut structure: OrganismStructure,
    blueprint: &StructuralBlueprint,
    catalog: &[BaseResource],
    mut energy: f64,
) -> Result<(OrganismStructure, EnergyLedger, f64), String> {
    use crate::combine_runtime::combine_specific_pair;
    use crate::contact::ConnectionCompatibilityCache;
    let mut ledger = EnergyLedger::default();
    let mut cache = ConnectionCompatibilityCache::new();
    for connection in &blueprint.connections {
        combine_specific_pair(
            &mut structure,
            connection.element_a,
            connection.element_b,
            catalog,
            &mut cache,
            &mut ledger,
            &mut energy,
        )
        .ok_or_else(|| {
            format!(
                "confirmed seed bond could not be realized: {}-{}",
                connection.element_a, connection.element_b
            )
        })?;
    }
    Ok((structure, ledger, energy))
}

/// Make the confirmed initial organism's outer interface permeable by replacing
/// alternating outer-shell constituents with fitted Water while preserving their
/// existing constituent IDs and bonds. The same realized seed persists through
/// development, so this boundary treatment is present in both juvenile and adult
/// states of the initial organism.
fn realize_initial_boundary_water(
    structure: &mut OrganismStructure,
    catalog: &[BaseResource],
) -> Result<usize, String> {
    use crate::resources::{Form, Material};
    let water = catalog
        .iter()
        .find(|resource| resource.name == "Water")
        .ok_or_else(|| "catalog is missing Water".to_string())?;

    let Form::Circle { radius } = &water.shape.form else {
        return Err("Water catalog realization must retain its default circle".into());
    };
    let nominal_area = std::f64::consts::PI * *radius * *radius;
    if !nominal_area.is_finite() || nominal_area <= 0.0 {
        return Err("Water nominal area is invalid".into());
    }

    // The confirmed seed baseline's outer shell is the ten units after
    // the four-unit inner shell. Two Water constituents are followed by two
    // rigid constituents around the membrane, producing an ordinary
    // R-R-W-W-R-R-W-W pattern rather than a special intake port.
    let water_sections = [2usize, 3, 6, 7];
    let mut changed = 0usize;
    for index in water_sections.into_iter().map(|offset| 4 + offset) {
        let Some(unit) = structure.units.get_mut(index) else {
            return Err("confirmed seed outer shell is incomplete".into());
        };
        let Some(old_shape) = unit.shape(catalog).cloned() else {
            return Err("confirmed seed outer boundary has unrealizable geometry".into());
        };
        let Some(vertices) = old_shape.form.polygon_vertices() else {
            return Err("confirmed seed outer boundary must be polygonal".into());
        };
        if vertices.len() < 3 {
            return Err("confirmed seed outer boundary polygon is degenerate".into());
        }

        let area = polygon_area(&vertices).abs();
        let amount = area / nominal_area;
        if !amount.is_finite() || amount <= 0.0 {
            return Err("confirmed seed Water boundary amount is invalid".into());
        }

        unit.material = Material::free_base("Water", amount);
        if !unit.realize_fluid_geometry(
            crate::resources::Shape {
                form: Form::Fluid {
                    nominal_area,
                    boundary: Some(vertices),
                },
            },
            catalog,
        ) {
            return Err("failed to realize fitted Water on the initial outer boundary".into());
        }
        changed += 1;
    }

    Ok(changed)
}

fn polygon_area(vertices: &[(f64, f64)]) -> f64 {
    vertices
        .iter()
        .enumerate()
        .map(|(i, &(x1, y1))| {
            let (x2, y2) = vertices[(i + 1) % vertices.len()];
            x1 * y2 - y1 * x2
        })
        .sum::<f64>()
        * 0.5
}

pub(crate) const JUVENILE_INITIAL_ENERGY_RESERVE: f64 = 16.0;
const TRIAL_ENERGY: f64 = 1.0e12;
const EPS: f64 = 1e-8;

/// Realize a construction/calibration baseline through the authoritative physical path.
#[allow(dead_code)]
pub(crate) fn realize_initial(
    blueprint: &StructuralBlueprint,
    catalog: &[BaseResource],
) -> Result<(OrganismStructure, EnergyLedger, f64), String> {
    realize_initial_with_reserve(blueprint, catalog, JUVENILE_INITIAL_ENERGY_RESERVE)
}

pub(crate) fn realize_initial_with_reserve(
    blueprint: &StructuralBlueprint,
    catalog: &[BaseResource],
    reserve_energy: f64,
) -> Result<(OrganismStructure, EnergyLedger, f64), String> {
    if !blueprint.is_valid() {
        return Err("juvenile construction target is invalid".into());
    }
    if !reserve_energy.is_finite() || reserve_energy <= 0.0 {
        return Err("juvenile energy reserve must be finite and positive".into());
    }

    let base = realize_declared_units(blueprint, catalog)?;

    let (_, _, trial_remaining) =
        form_declared_bonds(base.clone(), blueprint, catalog, TRIAL_ENERGY)?;
    let required_initial_energy = TRIAL_ENERGY - trial_remaining;
    if !required_initial_energy.is_finite() || required_initial_energy < 0.0 {
        return Err("juvenile construction produced an invalid energy requirement".into());
    }

    let mut energy = required_initial_energy + reserve_energy;
    let (structure, mut ledger, remaining) = form_declared_bonds(base, blueprint, catalog, energy)?;
    energy = remaining;

    if !energy.is_finite() || energy + EPS < reserve_energy {
        return Err(format!(
            "juvenile initialization could not preserve its reserve: remaining={energy}"
        ));
    }
    validate_realized_juvenile(
        &structure,
        catalog,
        JuvenileViabilityRequirements::default(),
    )?;
    let cavity = crate::cavity::analyze_genome_cavity(&structure, catalog)?.ok_or_else(|| {
        "juvenile realization has no qualifying physical genome cavity".to_string()
    })?;
    let genome_ids = cavity
        .boundary_units
        .iter()
        .filter_map(|&index| structure.physical_id(index))
        .collect::<Vec<_>>();
    let mut structure = structure;
    structure.set_genome_constituent_ids(genome_ids);
    realize_initial_boundary_water(&mut structure, catalog)?;

    // The initial boundary water is ordinary realized structure. Interior Water is
    // no longer synthesized into enclosed regions; environmental physical material
    // enters through the realized boundary and is acquired only after containment.
    if !energy.is_finite() || energy + EPS < reserve_energy {
        return Err(format!(
            "juvenile initialization could not preserve its reserve after Water construction: remaining={energy}"
        ));
    }
    Ok((structure, ledger, energy))
}

#[cfg(test)]
mod water_initialization_tests {
    use super::*;
    use crate::resources::{default_catalog, PhysicalState};

    #[test]
    fn initial_realization_contains_fitted_boundary_water_and_internal_water() {
        let catalog = default_catalog();
        let blueprint = confirmed_seed_baseline(&catalog).expect("seed baseline");
        let (structure, _, _) = realize_initial(&blueprint, &catalog).expect("initial realization");

        let water_indices: Vec<_> = structure
            .units
            .iter()
            .enumerate()
            .filter(|(_, unit)| {
                unit.material
                    .parts
                    .first()
                    .map(|(name, _)| name == "Water")
                    .unwrap_or(false)
            })
            .map(|(index, _)| index)
            .collect();

        assert!(!water_indices.is_empty());

        let fitted_boundary = water_indices
            .iter()
            .filter(|&&index| {
                matches!(
                    structure.units[index]
                        .shape(&catalog)
                        .map(|shape| &shape.form),
                    Some(crate::resources::Form::Fluid {
                        boundary: Some(_),
                        ..
                    })
                )
            })
            .count();
        assert_eq!(fitted_boundary, water_indices.len());

        let water_resource = catalog
            .iter()
            .find(|resource| resource.name == "Water")
            .expect("Water resource");
        assert_eq!(water_resource.physical_state, PhysicalState::Fluid);

        let genome = crate::cavity::analyze_genome_cavity(&structure, &catalog)
            .expect("genome cavity analysis")
            .expect("genome cavity");
        let accessible =
            crate::interior_geometry::find_accessible_interior_regions(&structure, &catalog)
                .expect("accessible interior analysis");

        assert!(accessible
            .iter()
            .all(|region| region.boundary_units != genome.boundary_units));
    }
}
