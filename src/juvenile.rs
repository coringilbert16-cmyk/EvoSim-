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

    // Recovered from the last confirmed-good pre-P6 seed realization. This is
    // a construction/scale calibration artifact only: it is not serialized,
    // inherited, or used as descendant topology authority.
    let nitrogen = catalog
        .iter()
        .find(|resource| resource.name == "Nitrogen")
        .ok_or_else(|| "catalog is missing the confirmed seed Nitrogen material".to_string())?;
    let nitrogen = &nitrogen.name;

    // Original four-unit inner shell.
    let side = 1.511_858;
    let thickness = 0.330_719;
    let offset = (side + thickness) / 2.0;
    let mut elements = vec![
        BlueprintElement {
            material: Material::free_base(nitrogen, 1.0),
            placement: BlueprintPlacement {
                x: 0.0,
                y: offset,
                rotation_radians: 0.0,
            },
        },
        BlueprintElement {
            material: Material::free_base(nitrogen, 1.0),
            placement: BlueprintPlacement {
                x: -offset,
                y: 0.0,
                rotation_radians: std::f64::consts::FRAC_PI_2,
            },
        },
        BlueprintElement {
            material: Material::free_base(nitrogen, 1.0),
            placement: BlueprintPlacement {
                x: offset,
                y: 0.0,
                rotation_radians: std::f64::consts::FRAC_PI_2,
            },
        },
        BlueprintElement {
            material: Material::free_base(nitrogen, 1.0),
            placement: BlueprintPlacement {
                x: 0.0,
                y: -offset,
                rotation_radians: 0.0,
            },
        },
    ];
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

    // Original eight-unit outer shell.
    let half_segment = side / 2.0;
    let outer_offset = 1.677_2175;
    let start = elements.len();
    elements.extend([
        BlueprintElement {
            material: Material::free_base(nitrogen, 1.0),
            placement: BlueprintPlacement {
                x: -half_segment,
                y: outer_offset,
                rotation_radians: 0.0,
            },
        },
        BlueprintElement {
            material: Material::free_base(nitrogen, 1.0),
            placement: BlueprintPlacement {
                x: half_segment,
                y: outer_offset,
                rotation_radians: 0.0,
            },
        },
        BlueprintElement {
            material: Material::free_base(nitrogen, 1.0),
            placement: BlueprintPlacement {
                x: -outer_offset,
                y: -half_segment,
                rotation_radians: std::f64::consts::FRAC_PI_2,
            },
        },
        BlueprintElement {
            material: Material::free_base(nitrogen, 1.0),
            placement: BlueprintPlacement {
                x: -outer_offset,
                y: half_segment,
                rotation_radians: std::f64::consts::FRAC_PI_2,
            },
        },
        BlueprintElement {
            material: Material::free_base(nitrogen, 1.0),
            placement: BlueprintPlacement {
                x: outer_offset,
                y: -half_segment,
                rotation_radians: std::f64::consts::FRAC_PI_2,
            },
        },
        BlueprintElement {
            material: Material::free_base(nitrogen, 1.0),
            placement: BlueprintPlacement {
                x: outer_offset,
                y: half_segment,
                rotation_radians: std::f64::consts::FRAC_PI_2,
            },
        },
        BlueprintElement {
            material: Material::free_base(nitrogen, 1.0),
            placement: BlueprintPlacement {
                x: -half_segment,
                y: -outer_offset,
                rotation_radians: 0.0,
            },
        },
        BlueprintElement {
            material: Material::free_base(nitrogen, 1.0),
            placement: BlueprintPlacement {
                x: half_segment,
                y: -outer_offset,
                rotation_radians: 0.0,
            },
        },
    ]);
    connections.extend([
        BlueprintConnection {
            element_a: start,
            element_b: start + 1,
        },
        BlueprintConnection {
            element_a: start + 1,
            element_b: start + 5,
        },
        BlueprintConnection {
            element_a: start + 5,
            element_b: start + 4,
        },
        BlueprintConnection {
            element_a: start + 4,
            element_b: start + 7,
        },
        BlueprintConnection {
            element_a: start + 7,
            element_b: start + 6,
        },
        BlueprintConnection {
            element_a: start + 6,
            element_b: start + 2,
        },
        BlueprintConnection {
            element_a: start + 2,
            element_b: start + 3,
        },
        BlueprintConnection {
            element_a: start + 3,
            element_b: start,
        },
    ]);

    // Original four Hydrogen interface connectors.
    let inner_outer = 1.086_648;
    let outer_inner = 1.511_858;
    let length: f64 = 0.797_884;
    let gap = outer_inner - inner_outer;
    let tangent = (length * length - gap * gap).sqrt();
    let center = (inner_outer + outer_inner) / 2.0;
    let start = elements.len();
    elements.extend([
        BlueprintElement {
            material: Material::free_base("Hydrogen", 1.0),
            placement: BlueprintPlacement {
                x: 0.0,
                y: center,
                rotation_radians: gap.atan2(-tangent),
            },
        },
        BlueprintElement {
            material: Material::free_base("Hydrogen", 1.0),
            placement: BlueprintPlacement {
                x: -center,
                y: 0.0,
                rotation_radians: (-tangent).atan2(-gap),
            },
        },
        BlueprintElement {
            material: Material::free_base("Hydrogen", 1.0),
            placement: BlueprintPlacement {
                x: center,
                y: 0.0,
                rotation_radians: tangent.atan2(gap),
            },
        },
        BlueprintElement {
            material: Material::free_base("Hydrogen", 1.0),
            placement: BlueprintPlacement {
                x: 0.0,
                y: -center,
                rotation_radians: (-gap).atan2(tangent),
            },
        },
    ]);
    connections.extend([
        BlueprintConnection {
            element_a: 0,
            element_b: start,
        },
        BlueprintConnection {
            element_a: 4,
            element_b: start,
        },
        BlueprintConnection {
            element_a: 1,
            element_b: start + 1,
        },
        BlueprintConnection {
            element_a: 6,
            element_b: start + 1,
        },
        BlueprintConnection {
            element_a: 2,
            element_b: start + 2,
        },
        BlueprintConnection {
            element_a: 8,
            element_b: start + 2,
        },
        BlueprintConnection {
            element_a: 3,
            element_b: start + 3,
        },
        BlueprintConnection {
            element_a: 10,
            element_b: start + 3,
        },
    ]);

    let baseline =
        StructuralBlueprint::with_anchor_elements(elements, connections, vec![0, 1, 2, 3]);
    baseline.validate()?;
    Ok(baseline)
}

pub(crate) fn confirmed_seed_scale_reference(
    catalog: &[BaseResource],
) -> Result<(f64, f64), String> {
    let baseline = confirmed_seed_baseline(catalog)?;
    let structure = baseline.realize(catalog)?;
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

pub(crate) const JUVENILE_INITIAL_ENERGY_RESERVE: f64 = 16.0;
const TRIAL_ENERGY: f64 = 1.0e12;
const EPS: f64 = 1e-8;

/// Realize the juvenile target generated from inherited architecture.
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

    // First realization is a non-persistent energy requirement calculation.
    // The actual physical assembly is still performed by the same unified
    // blueprint/construction/COMBINE path used elsewhere.
    let mut trial_ledger = EnergyLedger::default();
    let mut trial_energy = TRIAL_ENERGY;
    blueprint
        .realize_with_context(catalog, &mut trial_ledger, &mut trial_energy)
        .map_err(|error| format!("juvenile construction target could not be realized: {error}"))?;
    let required_initial_energy = TRIAL_ENERGY - trial_energy;
    if !required_initial_energy.is_finite() || required_initial_energy < 0.0 {
        return Err("juvenile construction produced an invalid energy requirement".into());
    }

    let mut ledger = EnergyLedger::default();
    let mut energy = required_initial_energy + reserve_energy;
    let (structure, _) = blueprint
        .realize_with_context(catalog, &mut ledger, &mut energy)
        .map_err(|error| format!("juvenile construction target could not be realized: {error}"))?;

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
    Ok((structure, ledger, energy))
}
