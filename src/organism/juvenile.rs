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
    // Two Carbon pieces per side form a sealed hexagonal wall large enough
    // for the established three-Carbon cavity reference. The chain and
    // heterogeneous peripheral materials remain independent branches.
    let ring_sides = 6usize;
    let ring_pieces_per_side = 2usize;
    let ring_piece_count = ring_sides * ring_pieces_per_side;
    let chain_steps = 20usize;
    let carbon_radius = catalog
        .iter()
        .find(|resource| resource.name == "Carbon")
        .and_then(|resource| match resource.shape.form {
            crate::resources::Form::RegularPolygon { radius, .. } => Some(radius),
            _ => None,
        })
        .ok_or_else(|| "Carbon seed geometry is not a regular polygon".to_string())?;
    let carbon_edge_center_spacing = (3.0_f64).sqrt() * carbon_radius;
    let ring_radius = carbon_edge_center_spacing * ring_pieces_per_side as f64;

    let mut elements = Vec::with_capacity(ring_piece_count + chain_steps + 6);

    for i in 0..ring_sides {
        let angle = std::f64::consts::TAU * i as f64 / ring_sides as f64;
        let next_angle =
            std::f64::consts::TAU * (i + 1) as f64 / ring_sides as f64;
        let position_angle = angle - std::f64::consts::FRAC_PI_6;
        let next_position_angle = next_angle - std::f64::consts::FRAC_PI_6;
        let vertex = (
            ring_radius * position_angle.cos(),
            ring_radius * position_angle.sin(),
        );
        let next_vertex = (
            ring_radius * next_position_angle.cos(),
            ring_radius * next_position_angle.sin(),
        );
        let midpoint = (
            (vertex.0 + next_vertex.0) * 0.5,
            (vertex.1 + next_vertex.1) * 0.5,
        );
        for (x, y) in [vertex, midpoint] {
            elements.push(BlueprintElement {
                material: Material::free_base("Carbon", 1.0),
                placement: BlueprintPlacement {
                    x,
                    y,
                    rotation_radians: 0.0,
                },
            });
        }
    }

    // Continue from the first ring element with a physically spaced outward
    // chain. A tightly wound polar spiral would put successive declared centers
    // closer than the rigid Carbon geometry allows and encourage greedy growth
    // to trap itself. These coordinates are preferences; live geometry and bond
    // transactions still determine the realized poses.
    let chain_heading = std::f64::consts::FRAC_PI_6;
    for step in 1..=chain_steps {
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
        let position_angle = angle - std::f64::consts::FRAC_PI_6;
        let radius = ring_radius + 3.0;
        elements.push(BlueprintElement {
            material: Material::free_base("Sulfur", 1.0),
            placement: BlueprintPlacement {
                x: radius * position_angle.cos(),
                y: radius * position_angle.sin(),
                rotation_radians: angle,
            },
        });
    }

    for &i in &[2usize, 5] {
        let angle = std::f64::consts::TAU * i as f64 / ring_sides as f64;
        let position_angle = angle - std::f64::consts::FRAC_PI_6;
        let radius = ring_radius + 3.0;
        elements.push(BlueprintElement {
            material: Material::free_base("Methane", 1.0),
            placement: BlueprintPlacement {
                x: radius * position_angle.cos(),
                y: radius * position_angle.sin(),
                rotation_radians: angle,
            },
        });
    }

    let mut connections = Vec::with_capacity(ring_piece_count + chain_steps + 6);
    for i in 0..ring_piece_count {
        connections.push(BlueprintConnection {
            element_a: i,
            element_b: (i + 1) % ring_piece_count,
        });
    }
    // The first chain element attaches to the ring's vertex at element 0.
    for step in 1..=chain_steps {
        connections.push(BlueprintConnection {
            element_a: if step == 1 { 0 } else { ring_piece_count + step - 2 },
            element_b: ring_piece_count + step - 1,
        });
    }

    let sulfur_start = ring_piece_count + chain_steps;
    for (offset, &ring_side_index) in [0usize, 1, 3, 4].iter().enumerate() {
        connections.push(BlueprintConnection {
            element_a: ring_side_index * ring_pieces_per_side,
            element_b: sulfur_start + offset,
        });
    }
    let methane_start = sulfur_start + 4;
    for (offset, &ring_side_index) in [2usize, 5].iter().enumerate() {
        connections.push(BlueprintConnection {
            element_a: ring_side_index * ring_pieces_per_side,
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
    fn confirmed_seed_ring_uses_two_face_to_face_carbon_pieces_per_side() {
        let catalog = crate::resources::default_catalog();
        let blueprint = confirmed_seed_baseline(&catalog).expect("seed baseline should be valid");
        let ring_sides = 6usize;
        let ring_piece_count = ring_sides * 2;
        let carbon_radius = catalog
            .iter()
            .find(|resource| resource.name == "Carbon")
            .and_then(|resource| match resource.shape.form {
                crate::resources::Form::RegularPolygon { radius, .. } => Some(radius),
                _ => None,
            })
            .expect("Carbon should be a regular polygon");
        let expected_spacing = 3.0_f64.sqrt() * carbon_radius;

        for index in 0..ring_piece_count {
            let next = (index + 1) % ring_piece_count;
            let a = blueprint.elements[index].placement;
            let b = blueprint.elements[next].placement;
            let distance = (a.x - b.x).hypot(a.y - b.y);
            assert!(
                (distance - expected_spacing).abs() < 1e-9,
                "ring edge {index}-{next} has center spacing {distance}, expected {expected_spacing}"
            );
            assert!(
                (a.rotation_radians - b.rotation_radians).abs() < 1e-9,
                "edge-adjacent Carbon pieces should have aligned rotations"
            );
            assert!(
                blueprint.connections.iter().any(|connection| {
                    (connection.element_a == index && connection.element_b == next)
                        || (connection.element_a == next && connection.element_b == index)
                }),
                "ring edge {index}-{next} must be connected"
            );
        }

        let chain_start = ring_piece_count;
        for step in 1..=20 {
            let previous = if step == 1 { 0 } else { chain_start + step - 2 };
            let current = chain_start + step - 1;
            let a = blueprint.elements[previous].placement;
            let b = blueprint.elements[current].placement;
            let distance = (a.x - b.x).hypot(a.y - b.y);
            assert!(
                (distance - expected_spacing).abs() < 1e-9,
                "outward chain step {step} has center spacing {distance}, expected {expected_spacing}"
            );
        }

        // The outward chain may touch a ring piece at an unbonded boundary,
        // but it must never penetrate any ring constituent.
        let carbon_form = catalog
            .iter()
            .find(|resource| resource.name == "Carbon")
            .expect("Carbon should exist")
            .shape
            .form
            .clone();
        for chain_index in ring_piece_count..(ring_piece_count + 20) {
            let chain = blueprint.elements[chain_index].placement;
            for ring_index in 1..ring_piece_count {
                let ring = blueprint.elements[ring_index].placement;
                let chain_part = crate::material_geometry::PlacedMaterialPart {
                    part_index: chain_index,
                    form: carbon_form.clone(),
                    placement: crate::structure::Placement {
                        x: chain.x,
                        y: chain.y,
                        rotation_radians: chain.rotation_radians,
                    },
                };
                let ring_part = crate::material_geometry::PlacedMaterialPart {
                    part_index: ring_index,
                    form: carbon_form.clone(),
                    placement: crate::structure::Placement {
                        x: ring.x,
                        y: ring.y,
                        rotation_radians: ring.rotation_radians,
                    },
                };
                assert!(
                    !crate::material_geometry::placed_forms_penetrate(
                        &chain_part,
                        &ring_part,
                        crate::material_geometry::GEOMETRY_EPSILON,
                    ),
                    "outward chain element {chain_index} penetrates ring element {ring_index}"
                );
            }
        }
    }

    #[test]
    fn methane_attachments_follow_the_six_hexagon_vertex_angles() {
        let catalog = crate::resources::default_catalog();
        let blueprint = confirmed_seed_baseline(&catalog).expect("seed baseline should be valid");
        let ring_sides = 6usize;
        let ring_pieces_per_side = 2usize;
        let ring_piece_count = ring_sides * ring_pieces_per_side;
        let methane_start = ring_piece_count + 20 + 4;

        for (offset, ring_index) in [2usize, 5].into_iter().enumerate() {
            let methane = blueprint.elements[methane_start + offset].placement;
            let actual_angle = methane.y.atan2(methane.x);
            let expected_angle = std::f64::consts::TAU * ring_index as f64 / ring_sides as f64
                - std::f64::consts::FRAC_PI_6;
            let angular_error = (actual_angle - expected_angle)
                .rem_euclid(std::f64::consts::TAU)
                .min((expected_angle - actual_angle).rem_euclid(std::f64::consts::TAU));
            assert!(
                angular_error < 1e-9,
                "methane attachment at ring index {ring_index} has angle {actual_angle}, expected {expected_angle}"
            );
            let ring_anchor_index = ring_index * ring_pieces_per_side;
            assert!(
                blueprint.connections.iter().any(|connection| {
                    (connection.element_a == ring_anchor_index
                        && connection.element_b == methane_start + offset)
                        || (connection.element_b == ring_anchor_index
                            && connection.element_a == methane_start + offset)
                }),
                "methane attachment must connect to ring vertex {ring_anchor_index}"
            );
        }
    }
}
