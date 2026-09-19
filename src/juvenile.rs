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
            0.0,
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

    let base = realize_declared_units(blueprint, catalog)?;

    let (_, _, trial_remaining) =
        form_declared_bonds(base.clone(), blueprint, catalog, TRIAL_ENERGY)?;
    let required_initial_energy = TRIAL_ENERGY - trial_remaining;
    if !required_initial_energy.is_finite() || required_initial_energy < 0.0 {
        return Err("juvenile construction produced an invalid energy requirement".into());
    }

    let mut energy = required_initial_energy + reserve_energy;
    let (structure, ledger, remaining) = form_declared_bonds(base, blueprint, catalog, energy)?;
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
    Ok((structure, ledger, energy))
}
