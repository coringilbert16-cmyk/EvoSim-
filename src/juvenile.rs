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
    // Eight Carbon pieces make a sufficiently large closed neighborhood around
    // the scaffold. Sulfur and Methane are ordinary outward branches, providing
    // a mixed-material mesh without becoming part of the genome definition.
    let ring_radius = 1.146_355;
    let mut elements = Vec::with_capacity(14);

    for i in 0..8 {
        let angle = std::f64::consts::TAU * i as f64 / 8.0;
        elements.push(BlueprintElement {
            material: Material::free_base("Carbon", 1.0),
            placement: BlueprintPlacement {
                x: ring_radius * angle.cos(),
                y: ring_radius * angle.sin(),
                rotation_radians: angle,
            },
        });
    }

    for &i in &[0usize, 2, 4, 6] {
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

    for &i in &[1usize, 5] {
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

    let mut connections = Vec::with_capacity(14);
    for i in 0..8 {
        connections.push(BlueprintConnection {
            element_a: i,
            element_b: (i + 1) % 8,
        });
    }
    for (offset, &i) in [0usize, 2, 4, 6].iter().enumerate() {
        connections.push(BlueprintConnection {
            element_a: i,
            element_b: 8 + offset,
        });
    }
    for (offset, &i) in [1usize, 5].iter().enumerate() {
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
        if let Some(scaffold) = blueprint.genome_measurement.as_ref() {
            if crate::construction_runtime::placement_penetrates_genome_measurement(
                catalog
                    .iter()
                    .find(|resource| resource.name == element.material.parts[0].0)
                    .ok_or_else(|| "confirmed seed references a missing resource".to_string())?,
                unit.placement,
                scaffold,
                catalog,
            ) {
                return Err("confirmed seed penetrates its genome measurement scaffold".into());
            }
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

pub(crate) const JUVENILE_INITIAL_ENERGY_RESERVE: f64 = 16.0;
const TRIAL_ENERGY: f64 = 1.0e6;
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

    // The bond-driven constructor is the sole physical realization path.
    // Use a private trial budget to discover the actual construction cost, then
    // rerun the same solver with enough energy to preserve the biological
    // reserve. This keeps geometry, bond choice, scaffold avoidance, and cavity
    // qualification on one authoritative path.
    let mut trial_ledger = EnergyLedger::default();
    let mut trial_energy = TRIAL_ENERGY;
    let (_, _, trial_remaining) =
        blueprint.realize_with_context(catalog, &mut trial_ledger, &mut trial_energy)?;
    let required_initial_energy = TRIAL_ENERGY - trial_remaining;
    if !required_initial_energy.is_finite() || required_initial_energy < 0.0 {
        return Err("juvenile construction produced an invalid energy requirement".into());
    }

    let mut ledger = EnergyLedger::default();
    let mut energy = required_initial_energy + reserve_energy;
    let (mut structure, _, remaining_energy) =
        blueprint.realize_with_context(catalog, &mut ledger, &mut energy)?;

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

    if !remaining_energy.is_finite() || remaining_energy + EPS < reserve_energy {
        return Err(format!(
            "juvenile initialization could not preserve its reserve after Water construction: remaining={remaining_energy}"
        ));
    }
    Ok((structure, ledger, remaining_energy))
}

#[cfg(test)]
mod water_initialization_tests {
    use super::*;
    use crate::resources::default_catalog;

    #[test]
    fn initial_realization_is_a_continuous_mixed_material_mesh_with_only_genome_cavity() {
        let catalog = default_catalog();
        let blueprint = confirmed_seed_baseline(&catalog).expect("seed baseline");
        let (structure, _, _) = realize_initial(&blueprint, &catalog).expect("initial realization");
        assert!(structure.units.len() >= 6);
        let names: std::collections::HashSet<_> = structure
            .units
            .iter()
            .filter_map(|unit| unit.material.parts.first().map(|(name, _)| name.as_str()))
            .collect();
        assert!(names.len() >= 3);

        let cavity = crate::cavity::analyze_genome_cavity(&structure, &catalog)
            .expect("genome cavity analysis")
            .expect("genome cavity");
        assert!(cavity.qualifies());
        assert!(!cavity.boundary_units.is_empty());

        let accessible =
            crate::interior_geometry::find_accessible_interior_regions(&structure, &catalog)
                .expect("accessible interior analysis");
        assert!(accessible
            .iter()
            .all(|region| region.boundary_units != cavity.boundary_units));
    }
}
