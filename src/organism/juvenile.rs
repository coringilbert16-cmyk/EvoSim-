//! Deterministic construction of one valid juvenile realization.
//!
//! The original viable seed realization is retained only as a physical
//! construction calibration baseline. Developmental fields, not this baseline,
//! determine descendant growth and realized architecture.
use crate::organism_viability::{validate_realized_organism, OrganismViabilityRequirements};
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

    for material in ["Carbon", "Sulfur", "Methane"] {
        if !catalog.iter().any(|resource| resource.name == material) {
            return Err(format!("catalog is missing seed {material}"));
        }
    }

    // This is the initial construction request, not a prescribed final body
    // plan. The constructor is given a substantially larger target so that it
    // has enough physical room to realize a useful cavity and surrounding
    // structure. The declared poses are spatial preferences; the constructor's
    // realized physical graph remains authoritative.
    //
    // The six-carbon inner loop supplies a physically compatible enclosed region. From one
    // point on that loop, twenty additional carbon elements extend along a
    // physically spaced outward chain. Sulfur and methane provide heterogeneous
    // peripheral material without defining an organism role or topology.
    let ring_sides = 6usize;
    let spiral_steps = 20usize;
    let carbon_radius = catalog
        .iter()
        .find(|resource| resource.name == "Carbon")
        .and_then(|resource| match resource.shape.form {
            crate::resources::Form::RegularPolygon { radius, .. } => Some(radius),
            _ => None,
        })
        .ok_or_else(|| "Carbon seed geometry is not a regular polygon".to_string())?;
    let carbon_edge_center_spacing = (3.0_f64).sqrt() * carbon_radius;
    let ring_radius =
        carbon_edge_center_spacing / (2.0 * (std::f64::consts::PI / ring_sides as f64).sin());

    let mut elements = Vec::with_capacity(ring_sides + spiral_steps + 6);

    for i in 0..ring_sides {
        let angle = std::f64::consts::TAU * i as f64 / ring_sides as f64;
        elements.push(BlueprintElement {
            material: Material::free_base("Carbon", 1.0),
            placement: BlueprintPlacement {
                x: ring_radius * angle.cos(),
                y: ring_radius * angle.sin(),
                rotation_radians: angle,
            },
        });
    }

    // Continue from the first ring element with a physically spaced outward
    // chain. A tightly wound polar spiral would put successive declared centers
    // closer than the rigid Carbon geometry allows and encourage greedy growth
    // to trap itself. These coordinates are preferences; live geometry and bond
    // transactions still determine the realized poses.
    let chain_heading = std::f64::consts::FRAC_PI_6;
    for step in 1..=spiral_steps {
        let distance = carbon_edge_center_spacing * step as f64;
        elements.push(BlueprintElement {
            material: Material::free_base("Carbon", 1.0),
            placement: BlueprintPlacement {
                x: elements[0].placement.x + distance * chain_heading.cos(),
                y: elements[0].placement.y + distance * chain_heading.sin(),
                rotation_radians: 0.0,
            },
        });
    }

    for &i in &[0usize, 1, 3, 4] {
        let angle = std::f64::consts::TAU * i as f64 / ring_sides as f64;
        let radius = ring_radius + 3.0;
        elements.push(BlueprintElement {
            material: Material::free_base("Sulfur", 1.0),
            placement: BlueprintPlacement {
                x: radius * angle.cos(),
                y: radius * angle.sin(),
                rotation_radians: angle,
            },
        });
    }

    for &i in &[2usize, 5] {
        let angle = std::f64::consts::TAU * i as f64 / ring_sides as f64;
        let radius = ring_radius + 3.0;
        elements.push(BlueprintElement {
            material: Material::free_base("Methane", 1.0),
            placement: BlueprintPlacement {
                x: radius * angle.cos(),
                y: radius * angle.sin(),
                rotation_radians: angle,
            },
        });
    }

    let mut connections = Vec::with_capacity(ring_sides + spiral_steps + 6);
    for i in 0..ring_sides {
        connections.push(BlueprintConnection {
            element_a: i,
            element_b: (i + 1) % ring_sides,
        });
    }
    // The first spiral element is the existing ring anchor (element 0).
    for step in 1..=spiral_steps {
        connections.push(BlueprintConnection {
            element_a: if step == 1 { 0 } else { ring_sides + step - 2 },
            element_b: ring_sides + step - 1,
        });
    }

    let sulfur_start = ring_sides + spiral_steps;
    for (offset, &i) in [0usize, 1, 3, 4].iter().enumerate() {
        connections.push(BlueprintConnection {
            element_a: i,
            element_b: sulfur_start + offset,
        });
    }
    let methane_start = sulfur_start + 4;
    for (offset, &i) in [2usize, 5].iter().enumerate() {
        connections.push(BlueprintConnection {
            element_a: i,
            element_b: methane_start + offset,
        });
    }

    let baseline = StructuralBlueprint::with_anchor_elements(elements, connections, vec![0])
        .with_genome_measurement(
            crate::structural_blueprint::GenomeMeasurementScaffold::three_carbon_reference(
                catalog,
            )?,
        );
    baseline.validate()?;
    Ok(baseline)
}

pub(crate) const JUVENILE_INITIAL_ENERGY_RESERVE: f64 = 16.0;
const TRIAL_ENERGY: f64 = 1.0e6;
const EPS: f64 = 1e-8;

/// Compatibility entry point for callers that create the initial organism or
/// exercise the original-seed calibration. The realization itself is now
/// performed exclusively by the forward-only bond-driven constructor.
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
        return Err("initial construction target is invalid".into());
    }
    if !reserve_energy.is_finite() || reserve_energy <= 0.0 {
        return Err("initial reproductive energy allocation must be finite and positive".into());
    }

    let mut trial_ledger = EnergyLedger::default();
    let mut trial_energy = TRIAL_ENERGY;
    crate::construction_runtime::construct_blueprint_bond_driven(
        blueprint,
        catalog,
        &mut trial_ledger,
        &mut trial_energy,
    )?;
    let required_initial_energy = TRIAL_ENERGY - trial_energy;
    if !required_initial_energy.is_finite() || required_initial_energy < 0.0 {
        return Err("juvenile construction produced an invalid energy requirement".into());
    }

    let mut ledger = EnergyLedger::default();
    let mut energy = required_initial_energy + reserve_energy;
    let (mut structure, _) = crate::construction_runtime::construct_blueprint_bond_driven(
        blueprint,
        catalog,
        &mut ledger,
        &mut energy,
    )?;
    let remaining_energy = energy;

    if !remaining_energy.is_finite() || remaining_energy + EPS < reserve_energy {
        return Err(format!(
            "juvenile initialization could not preserve its reserve: remaining={remaining_energy}"
        ));
    }

    validate_realized_organism(
        &structure,
        catalog,
        OrganismViabilityRequirements::default(),
    )?;

    let cavity = crate::cavity::analyze_genome_cavity(&structure, catalog)?.ok_or_else(|| {
        "juvenile realization has no qualifying physical genome cavity".to_string()
    })?;
    let genome_ids = cavity
        .boundary_units
        .iter()
        .filter_map(|&index| structure.physical_id(index))
        .collect::<Vec<_>>();
    structure.set_genome_constituent_ids(genome_ids);

    Ok((structure, ledger, remaining_energy))
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

#[cfg(test)]
mod seed_geometry_tests {
    use super::*;

    #[test]
    fn confirmed_seed_ring_uses_compatible_hexagonal_edge_spacing() {
        let catalog = crate::resources::default_catalog();
        let blueprint = confirmed_seed_baseline(&catalog).expect("seed baseline should be valid");
        let ring_sides = 6usize;
        let carbon_radius = catalog
            .iter()
            .find(|resource| resource.name == "Carbon")
            .and_then(|resource| match resource.shape.form {
                crate::resources::Form::RegularPolygon { radius, .. } => Some(radius),
                _ => None,
            })
            .expect("Carbon should be a regular polygon");
        let expected_spacing = 3.0_f64.sqrt() * carbon_radius;

        let spiral_start = ring_sides;
        for step in 1..=20 {
            let previous = if step == 1 {
                0
            } else {
                spiral_start + step - 2
            };
            let current = spiral_start + step - 1;
            let a = blueprint.elements[previous].placement;
            let b = blueprint.elements[current].placement;
            let distance = (a.x - b.x).hypot(a.y - b.y);
            assert!(
                (distance - expected_spacing).abs() < 1e-9,
                "outward chain step {step} has center spacing {distance}, expected {expected_spacing}"
            );
        }

        // The outward chain may share the anchor at element 0, but every other
        // ring element must remain clear of every chain element. This guards
        // against replacing the old tight spiral with a differently crossing path.
        for chain_index in ring_sides..(ring_sides + 20) {
            let chain = blueprint.elements[chain_index].placement;
            for ring_index in 1..ring_sides {
                let ring = blueprint.elements[ring_index].placement;
                let distance = (chain.x - ring.x).hypot(chain.y - ring.y);
                assert!(
                    distance > expected_spacing * 1.3,
                    "outward chain element {chain_index} approaches ring element {ring_index}: {distance}"
                );
            }
        }

        for index in 0..ring_sides {
            let next = (index + 1) % ring_sides;
            let a = blueprint.elements[index].placement;
            let b = blueprint.elements[next].placement;
            let distance = (a.x - b.x).hypot(a.y - b.y);
            assert!(
                (distance - expected_spacing).abs() < 1e-9,
                "ring edge {index}-{next} has center spacing {distance}, expected {expected_spacing}"
            );

            let rotation_delta =
                (b.rotation_radians - a.rotation_radians).rem_euclid(std::f64::consts::TAU);
            assert!(
                (rotation_delta - std::f64::consts::TAU / ring_sides as f64).abs() < 1e-9,
                "ring rotations should advance by one sixth-turn"
            );

            assert!(
                blueprint.connections.iter().any(|connection| {
                    (connection.element_a == index && connection.element_b == next)
                        || (connection.element_a == next && connection.element_b == index)
                }),
                "ring edge {index}-{next} must be connected"
            );
        }
    }

    #[test]
    fn methane_attachments_follow_the_six_piece_ring_angles() {
        let catalog = crate::resources::default_catalog();
        let blueprint = confirmed_seed_baseline(&catalog).expect("seed baseline should be valid");
        let ring_sides = 6usize;
        let methane_start = ring_sides + 20 + 4;

        for (offset, ring_index) in [2usize, 5].into_iter().enumerate() {
            let methane = blueprint.elements[methane_start + offset].placement;
            let actual_angle = methane.y.atan2(methane.x);
            let expected_angle = std::f64::consts::TAU * ring_index as f64 / ring_sides as f64;
            let angular_error = (actual_angle - expected_angle)
                .rem_euclid(std::f64::consts::TAU)
                .min((expected_angle - actual_angle).rem_euclid(std::f64::consts::TAU));
            assert!(
                angular_error < 1e-9,
                "methane attachment at ring index {ring_index} has angle {actual_angle}, expected {expected_angle}"
            );
            assert!(blueprint.connections.iter().any(|connection| {
                (connection.element_a == ring_index && connection.element_b == methane_start + offset)
                    || (connection.element_b == ring_index && connection.element_a == methane_start + offset)
            }), "methane attachment must connect to ring element {ring_index}");
        }
    }
}
