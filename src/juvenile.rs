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

    // This is the initial construction request, not a prescribed final body
    // plan. The constructor is given a substantially larger target so that it
    // has enough physical room to realize a useful cavity and surrounding
    // structure. The declared poses are spatial preferences; the constructor's
    // realized physical graph remains authoritative.
    //
    // The ten-carbon inner loop supplies a simple enclosed region. From one
    // point on that loop, twenty additional carbon elements wind outward as a
    // loose spiral. Sulfur and methane provide a small amount of heterogeneous
    // peripheral material without defining an organism role or topology.
    let ring_sides = 10usize;
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

    // Continue from the first ring element into an outward spiral. The
    // constructor may rotate/translate each piece as needed to obtain a valid
    // physical bond; these coordinates express the requested spatial tendency.
    let spiral_angle_step = 0.35_f64;
    let spiral_radius_step = 0.10_f64;
    for step in 1..=spiral_steps {
        let angle = spiral_angle_step * step as f64;
        let radius = ring_radius + spiral_radius_step * step as f64;
        elements.push(BlueprintElement {
            material: Material::free_base("Carbon", 1.0),
            placement: BlueprintPlacement {
                x: radius * angle.cos(),
                y: radius * angle.sin(),
                rotation_radians: angle,
            },
        });
    }

    for &i in &[0usize, 3, 6, 9] {
        let angle = std::f64::consts::TAU * i as f64 / 10.0;
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

    for &i in &[2usize, 7] {
        let angle = std::f64::consts::TAU * i as f64 / 10.0;
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
    for (offset, &i) in [0usize, 3, 6, 9].iter().enumerate() {
        connections.push(BlueprintConnection {
            element_a: i,
            element_b: sulfur_start + offset,
        });
    }
    let methane_start = sulfur_start + 4;
    for (offset, &i) in [2usize, 7].iter().enumerate() {
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
        return Err("juvenile construction target is invalid".into());
    }
    if !reserve_energy.is_finite() || reserve_energy <= 0.0 {
        return Err("juvenile energy reserve must be finite and positive".into());
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
