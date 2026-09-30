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

    for material in ["Carbon", "Sulfur", "Methane"] {
        if !catalog.iter().any(|resource| resource.name == material) {
            return Err(format!("catalog is missing seed {material}"));
        }
    }

    // This is only the deterministic starting target used to calibrate a
    // viable juvenile. It deliberately does not encode a square, a fixed core,
    // or a sacred body plan. The constructor is expected to rotate and translate
    // these pieces around the temporary three-carbon measurement scaffold until
    // the physical bonds and cavity are simultaneously satisfied.
    //
    // Ten Carbon pieces form the smallest regular Carbon ring that can
    // physically enclose the three-Carbon measurement scaffold without overlap.
    // Eight Carbon pieces cannot satisfy both requirements with the catalog's
    // fixed Carbon geometry: their bonded center spacing fixes the ring radius
    // too tightly for the scaffold to fit inside. Sulfur and Methane remain
    // ordinary outward branches and are not part of the genome definition.
    let ring_sides = 10usize;
    let carbon_radius = catalog
        .iter()
        .find(|resource| resource.name == "Carbon")
        .and_then(|resource| match resource.shape.form {
            crate::resources::Form::RegularPolygon { radius, .. } => Some(radius),
            _ => None,
        })
        .ok_or_else(|| "Carbon seed geometry is not a regular polygon".to_string())?;
    let ring_radius =
        (2.0 * carbon_radius) / (2.0 * (std::f64::consts::PI / ring_sides as f64).sin());
    let mut elements = Vec::with_capacity(ring_sides + 6);

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

    for &i in &[0usize, 2, 5, 7] {
        let angle = std::f64::consts::TAU * i as f64 / 8.0;
        let radius = ring_radius + 0.95;
        elements.push(BlueprintElement {
            material: Material::free_base("Sulfur", 1.0),
            placement: BlueprintPlacement {
                x: radius * angle.cos(),
                y: radius * angle.sin(),
                rotation_radians: angle,
            },
        });
    }

    for &i in &[1usize, 6] {
        let angle = std::f64::consts::TAU * i as f64 / 8.0;
        let radius = ring_radius + 1.05;
        elements.push(BlueprintElement {
            material: Material::free_base("Methane", 1.0),
            placement: BlueprintPlacement {
                x: radius * angle.cos(),
                y: radius * angle.sin(),
                rotation_radians: angle,
            },
        });
    }

    let mut connections = Vec::with_capacity(ring_sides + 6);
    for i in 0..ring_sides {
        connections.push(BlueprintConnection {
            element_a: i,
            element_b: (i + 1) % ring_sides,
        });
    }
    for (offset, &i) in [0usize, 3, 6, 9].iter().enumerate() {
        connections.push(BlueprintConnection {
            element_a: i,
            element_b: 8 + offset,
        });
    }
    for (offset, &i) in [1usize, 7].iter().enumerate() {
        connections.push(BlueprintConnection {
            element_a: i,
            element_b: 12 + offset,
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