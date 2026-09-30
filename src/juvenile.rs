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
    let nitrogen = &nitrogen.name;

    // The juvenile starts as two concentric rigid shells. The inner shell
    // encloses the genome cavity; the larger outer shell leaves a genuinely
    // accessible chamber between them. The chamber is deliberately wide enough
    // for the largest rigid environmental resource rather than relying on a
    // resource-specific interface.
    let inner_side = 1.511_858;
    let inner_thickness = 0.330_719;
    let inner_offset = (inner_side + inner_thickness) / 2.0;

    let mut elements = Vec::new();
    elements.extend([
        BlueprintElement {
            material: Material::free_base(nitrogen, 1.0),
            placement: BlueprintPlacement {
                x: 0.0,
                y: inner_offset,
                rotation_radians: 0.0,
            },
        },
        BlueprintElement {
            material: Material::free_base(nitrogen, 1.0),
            placement: BlueprintPlacement {
                x: -inner_offset,
                y: 0.0,
                rotation_radians: std::f64::consts::FRAC_PI_2,
            },
        },
        BlueprintElement {
            material: Material::free_base(nitrogen, 1.0),
            placement: BlueprintPlacement {
                x: inner_offset,
                y: 0.0,
                rotation_radians: std::f64::consts::FRAC_PI_2,
            },
        },
        BlueprintElement {
            material: Material::free_base(nitrogen, 1.0),
            placement: BlueprintPlacement {
                x: 0.0,
                y: -inner_offset,
                rotation_radians: 0.0,
            },
        },
    ]);

    let mut connections = vec![
        BlueprintConnection {
            element_a: 0,
            element_b: 1,
        },
        BlueprintConnection {
            element_a: 0,
            element_b: 2,
        },
        BlueprintConnection {
            element_a: 1,
            element_b: 3,
        },
        BlueprintConnection {
            element_a: 2,
            element_b: 3,
        },
    ];

    // The outer shell is a regular dodecagon made from the same Nitrogen
    // rectangles. The side length is the same as the inner shell, while the
    // larger apothem leaves a genuinely accessible chamber for finite rigid
    // resources such as Methane.
    let outer_side_count = 12usize;
    let outer_side_length = inner_side;
    let outer_apothem =
        outer_side_length / (2.0 * (std::f64::consts::PI / outer_side_count as f64).tan());
    let outer_center_radius = outer_apothem + inner_thickness / 2.0;
    let outer_start = elements.len();

    for side in 0..outer_side_count {
        let normal_angle = std::f64::consts::FRAC_PI_2
            + side as f64 * (2.0 * std::f64::consts::PI / outer_side_count as f64);
        let tangent_angle = normal_angle - std::f64::consts::FRAC_PI_2;
        elements.push(BlueprintElement {
            material: Material::free_base(nitrogen, 1.0),
            placement: BlueprintPlacement {
                x: outer_center_radius * normal_angle.cos(),
                y: outer_center_radius * normal_angle.sin(),
                rotation_radians: tangent_angle,
            },
        });
    }

    for side in 0..outer_side_count {
        connections.push(BlueprintConnection {
            element_a: outer_start + side,
            element_b: outer_start + (side + 1) % outer_side_count,
        });
    }

    // Three Hydrogen segments form each symmetric bridge. The chamber is
    // deliberately wider than one Hydrogen length, so the bridge uses an
    // ordinary zig-zag chain rather than a special connector or an intake port.
    let hydrogen_length = catalog
        .iter()
        .find(|resource| resource.name == "Hydrogen")
        .and_then(|resource| match &resource.shape.form {
            crate::resources::Form::Line { length } => Some(*length),
            _ => None,
        })
        .ok_or_else(|| "catalog Hydrogen must retain its line geometry".to_string())?;

    let inner_boundary = inner_offset + inner_thickness / 2.0;
    let outer_boundary = outer_apothem;
    let shell_gap = outer_boundary - inner_boundary;
    if !shell_gap.is_finite() || shell_gap <= 0.0 {
        return Err("initial shell spacing is invalid".into());
    }

    // The final segment uses its full Hydrogen length vertically. The first
    // two segments use equal and opposite lateral offsets so the three-link
    // chain lands exactly on the outer shell without protruding through it.
    let segment_rise = (shell_gap - hydrogen_length) / 2.0;
    if !segment_rise.is_finite() || segment_rise <= 0.0 || segment_rise >= hydrogen_length {
        return Err("initial shell spacing cannot be bridged by Hydrogen".into());
    }
    let lateral_offset = (hydrogen_length.powi(2) - segment_rise.powi(2)).sqrt();
    let bridge_start = elements.len();

    // Upper chain: (0, inner_boundary) -> (offset, inner_boundary+r) ->
    // (0, inner_boundary+2r) -> (0, outer_boundary).
    let upper_bridge = [
        BlueprintPlacement {
            x: lateral_offset / 2.0,
            y: inner_boundary + segment_rise / 2.0,
            rotation_radians: segment_rise.atan2(lateral_offset),
        },
        BlueprintPlacement {
            x: lateral_offset / 2.0,
            y: inner_boundary + segment_rise * 1.5,
            rotation_radians: (-segment_rise).atan2(lateral_offset),
        },
        BlueprintPlacement {
            x: 0.0,
            y: inner_boundary + segment_rise * 2.0 + hydrogen_length / 2.0,
            rotation_radians: std::f64::consts::FRAC_PI_2,
        },
    ];

    // Mirror the same ordinary chain across the origin for the lower bridge.
    let lower_bridge = [
        BlueprintPlacement {
            x: -lateral_offset / 2.0,
            y: -(inner_boundary + segment_rise / 2.0),
            rotation_radians: -upper_bridge[0].rotation_radians,
        },
        BlueprintPlacement {
            x: -lateral_offset / 2.0,
            y: -(inner_boundary + segment_rise * 1.5),
            rotation_radians: -upper_bridge[1].rotation_radians,
        },
        BlueprintPlacement {
            x: 0.0,
            y: -(inner_boundary + segment_rise * 2.0 + hydrogen_length / 2.0),
            rotation_radians: -std::f64::consts::FRAC_PI_2,
        },
    ];

    for placement in upper_bridge.into_iter().chain(lower_bridge) {
        elements.push(BlueprintElement {
            material: Material::free_base("Hydrogen", 1.0),
            placement,
        });
    }

    connections.extend([
        BlueprintConnection {
            element_a: 0,
            element_b: bridge_start,
        },
        BlueprintConnection {
            element_a: bridge_start,
            element_b: bridge_start + 1,
        },
        BlueprintConnection {
            element_a: bridge_start + 1,
            element_b: bridge_start + 2,
        },
        BlueprintConnection {
            element_a: bridge_start + 2,
            element_b: outer_start,
        },
        BlueprintConnection {
            element_a: 3,
            element_b: bridge_start + 5,
        },
        BlueprintConnection {
            element_a: bridge_start + 5,
            element_b: bridge_start + 4,
        },
        BlueprintConnection {
            element_a: bridge_start + 4,
            element_b: bridge_start + 3,
        },
        BlueprintConnection {
            element_a: bridge_start + 3,
            element_b: outer_start + 6,
        },
    ]);

    let baseline =
        StructuralBlueprint::with_anchor_elements(elements, connections, vec![0, 1, 2, 3]);
    baseline.validate()?;
    Ok(baseline)
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

    // The confirmed seed baseline's outer shell is the twelve units after
    // the four-unit inner shell. Alternating them leaves rigid structural
    // material between permeable sections instead of making the entire boundary fluid.
    let outer_shell = 4..16;
    let mut changed = 0usize;
    for index in outer_shell.step_by(2) {
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

    // Interior Water is part of the initial physical construction. Determine its
    // one-time construction cost separately so the organism still receives the
    // complete declared juvenile energy reserve after the structure is finished.
    let mut water_trial_structure = structure.clone();
    let mut water_trial_ledger = ledger;
    let mut water_trial_energy = TRIAL_ENERGY;
    crate::interior_geometry::fill_enclosed_regions_with_water(
        &mut water_trial_structure,
        catalog,
        &mut water_trial_ledger,
        &mut water_trial_energy,
    )?;
    let water_cost = TRIAL_ENERGY - water_trial_energy;
    if !water_cost.is_finite() || water_cost < 0.0 {
        return Err("initial Water construction produced an invalid energy requirement".into());
    }
    energy += water_cost;

    crate::interior_geometry::fill_enclosed_regions_with_water(
        &mut structure,
        catalog,
        &mut ledger,
        &mut energy,
    )?;
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

        assert!(water_indices.len() >= 5);

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
        assert!(fitted_boundary >= 5);

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
