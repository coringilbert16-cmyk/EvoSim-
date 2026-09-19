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

    // This is the historically confirmed viable juvenile construction used as
    // the physical seed calibration. It is not inherited and does not define
    // descendant topology. The original viable realization uses the catalog's
    // rectangular Nitrogen material to form a four-unit enclosure, with the
    // remaining juvenile units extending from one boundary.
    let resource = catalog
        .iter()
        .find(|resource| {
            matches!(
                resource.shape.form,
                crate::resources::Form::Rectangle { .. }
            )
        })
        .or_else(|| catalog.first())
        .ok_or_else(|| "catalog contains no seed material".to_string())?;

    let ring = match &resource.shape.form {
        crate::resources::Form::Rectangle { width, height } => {
            let radius = (width + height) * 0.5;
            vec![
                BlueprintPlacement {
                    x: 0.0,
                    y: radius,
                    rotation_radians: 0.0,
                },
                BlueprintPlacement {
                    x: radius,
                    y: 0.0,
                    rotation_radians: std::f64::consts::FRAC_PI_2,
                },
                BlueprintPlacement {
                    x: 0.0,
                    y: -radius,
                    rotation_radians: 0.0,
                },
                BlueprintPlacement {
                    x: -radius,
                    y: 0.0,
                    rotation_radians: std::f64::consts::FRAC_PI_2,
                },
            ]
        }
        _ => {
            let vertices =
                resource.shape.form.polygon_vertices().ok_or_else(|| {
                    "seed resource has no constructible polygon geometry".to_string()
                })?;
            let radius = vertices
                .iter()
                .map(|(x, y)| x.hypot(*y))
                .fold(0.0, f64::max);
            if radius <= 0.0 {
                return Err("seed resource has invalid geometry".into());
            }
            let ring_radius = radius / (std::f64::consts::PI / 4.0).sin();
            (0..4)
                .map(|i| {
                    let angle = i as f64 * std::f64::consts::FRAC_PI_2;
                    BlueprintPlacement {
                        x: ring_radius * angle.cos(),
                        y: ring_radius * angle.sin(),
                        rotation_radians: angle + std::f64::consts::FRAC_PI_2,
                    }
                })
                .collect()
        }
    };

    let mut elements = ring
        .iter()
        .copied()
        .map(|placement| BlueprintElement {
            material: Material::free_base(&resource.name, 1.0),
            placement,
        })
        .collect::<Vec<_>>();
    let mut connections = Vec::with_capacity(12);
    for i in 0..4 {
        connections.push(BlueprintConnection {
            element_a: i,
            element_b: (i + 1) % 4,
        });
    }

    let outward_length = match &resource.shape.form {
        crate::resources::Form::Rectangle { height, .. } => *height,
        _ => resource.shape.form.bounding_radius().max(1e-6),
    };
    let first = ring[0];
    let norm = first.x.hypot(first.y).max(1e-9);
    let direction = (first.x / norm, first.y / norm);
    let mut previous = first;
    for _ in 0..8 {
        let placement = BlueprintPlacement {
            x: previous.x + direction.0 * outward_length.max(1e-6),
            y: previous.y + direction.1 * outward_length.max(1e-6),
            rotation_radians: first.rotation_radians,
        };
        let parent = elements.len() - 1;
        elements.push(BlueprintElement {
            material: Material::free_base(&resource.name, 1.0),
            placement,
        });
        connections.push(BlueprintConnection {
            element_a: parent,
            element_b: parent + 1,
        });
        previous = placement;
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
