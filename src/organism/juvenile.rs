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
    let ring_radius =
        carbon_edge_center_spacing / (2.0 * (std::f64::consts::PI / ring_sides as f64).sin());

    let mut elements = Vec::with_capacity(ring_sides + chain_steps + 6);

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

    let mut connections = Vec::with_capacity(ring_sides + chain_steps + 6);
    for i in 0..ring_sides {
        connections.push(BlueprintConnection {
            element_a: i,
            element_b: (i + 1) % ring_sides,
        });
    }
    // The first spiral element is the existing ring anchor (element 0).
    for step in 1..=chain_steps {
        connections.push(BlueprintConnection {
            element_a: if step == 1 { 0 } else { ring_sides + step - 2 },
            element_b: ring_sides + step - 1,
        });
    }

    let sulfur_start = ring_sides + chain_steps;
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
