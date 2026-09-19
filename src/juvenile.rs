//! Deterministic construction of one valid juvenile realization.
//!
//! The genome owns architectural intent. This module asks the genome for a
//! developmental construction target, realizes it through the unified
//! construction runtime, and validates the resulting physical structure
//! against the juvenile viability contract.
use crate::genome::Genome;
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
    use crate::structural_blueprint::{
        BlueprintConnection, BlueprintElement, BlueprintPlacement,
    };

    let resource = catalog
        .iter()
        .find(|resource| resource.name == "Carbon")
        .or_else(|| catalog.first())
        .ok_or_else(|| "catalog contains no seed material".to_string())?;
    let radius = resource.shape.form.bounding_radius().max(1e-6);
    let mut elements = Vec::new();
    let mut connections = Vec::new();
    for i in 0..4 {
        let angle = i as f64 * std::f64::consts::FRAC_PI_2;
        elements.push(BlueprintElement {
            material: Material::free_base(&resource.name, 1.0),
            placement: BlueprintPlacement {
                x: radius * 1.5 * angle.cos(),
                y: radius * 1.5 * angle.sin(),
                rotation_radians: angle,
            },
        });
        connections.push(BlueprintConnection {
            element_a: i,
            element_b: (i + 1) % 4,
        });
    }
    for i in 0..8 {
        let parent = i % 4;
        let angle = parent as f64 * std::f64::consts::FRAC_PI_2
            + (i / 4) as f64 * std::f64::consts::FRAC_PI_4;
        let child = elements.len();
        elements.push(BlueprintElement {
            material: Material::free_base(&resource.name, 1.0),
            placement: BlueprintPlacement {
                x: 3.0 * radius * angle.cos(),
                y: 3.0 * radius * angle.sin(),
                rotation_radians: angle,
            },
        });
        connections.push(BlueprintConnection {
            element_a: parent,
            element_b: child,
        });
    }
    let baseline = StructuralBlueprint::with_anchor_elements(elements, connections, vec![0]);
    baseline.validate()?;
    Ok(baseline)
}

pub(crate) fn confirmed_seed_scale_reference(
    catalog: &[BaseResource],
) -> Result<(f64, f64), String> {
    let baseline = confirmed_seed_baseline(catalog)?;
    let structure = baseline.realize(catalog)?;
    let mass = structure.structural_mass(catalog);
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
