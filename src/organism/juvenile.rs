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
/// This is the established 12-piece, corner-free nitrogen cavity seed. It is a
/// calibration fixture only; developmental fields do not prescribe descendant
/// topology or geometry.
pub(crate) fn confirmed_seed_baseline(
    catalog: &[BaseResource],
) -> Result<StructuralBlueprint, String> {
    use crate::resources::Material;
    use crate::structural_blueprint::{BlueprintConnection, BlueprintElement, BlueprintPlacement};

    if !catalog.iter().any(|resource| resource.name == "Nitrogen") {
        return Err("catalog is missing seed Nitrogen".to_string());
    }

    // The seed is the 5x5 perimeter with its four non-wall corners omitted:
    // twelve square Nitrogen pieces make a sealed cavity without synthetic
    // core material or an unrelated carbon chain.
    let mut elements = Vec::with_capacity(12);
    let mut grid = [[None; 5]; 5];
    for (y, row) in grid.iter_mut().enumerate() {
        for (x, cell) in row.iter_mut().enumerate() {
            if (x == 0 || x == 4) && (y == 0 || y == 4) {
                continue;
            }
            if x != 0 && x != 4 && y != 0 && y != 4 {
                continue;
            }
            let index = elements.len();
            elements.push(BlueprintElement {
                material: Material::free_base("Nitrogen", 1.0),
                placement: BlueprintPlacement {
                    x: x as f64 - 2.0,
                    y: y as f64 - 2.0,
                    rotation_radians: 0.0,
                },
            });
            *cell = Some(index);
        }
    }

    let mut connections = Vec::with_capacity(12);
    for (y, row) in grid.iter().enumerate() {
        for (x, slot) in row.iter().enumerate() {
            let Some(a) = *slot else { continue };
            for (nx, ny) in [(x + 1, y), (x, y + 1)] {
                if let Some(b) = grid
                    .get(ny)
                    .and_then(|neighbor_row| neighbor_row.get(nx))
                    .copied()
                    .flatten()
                {
                    connections.push(BlueprintConnection {
                        element_a: a,
                        element_b: b,
                    });
                }
            }
        }
    }

    // Side groups meet at the four omitted corners. These diagonal contacts
    // keep the ring physically connected without adding non-wall constituents.
    for ((ax, ay), (bx, by)) in [
        ((1, 0), (0, 1)),
        ((3, 0), (4, 1)),
        ((0, 3), (1, 4)),
        ((4, 3), (3, 4)),
    ] {
        connections.push(BlueprintConnection {
            element_a: grid[ay][ax].expect("corner-adjacent side piece exists"),
            element_b: grid[by][bx].expect("corner-adjacent side piece exists"),
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

fn is_canonical_nitrogen_seed(blueprint: &StructuralBlueprint) -> bool {
    blueprint.elements.len() == 12
        && blueprint.connections.len() == 12
        && blueprint.elements.iter().all(|element| {
            element.material.parts.len() == 1
                && element.material.parts[0].0 == "Nitrogen"
                && element.material.parts[0].1 == 1.0
        })
        && blueprint.elements.iter().all(|element| {
            let x = element.placement.x;
            let y = element.placement.y;
            (x.abs() == 2.0 && y.abs() < 2.0) || (y.abs() == 2.0 && x.abs() < 2.0)
        })
}

fn construct_calibration_structure(
    blueprint: &StructuralBlueprint,
    catalog: &[BaseResource],
    ledger: &mut EnergyLedger,
    energy: &mut f64,
) -> Result<OrganismStructure, String> {
    if is_canonical_nitrogen_seed(blueprint) {
        crate::initial_organism_constructor::build_nitrogen_ring_seed(catalog, ledger, energy)
    } else {
        crate::construction_runtime::construct_blueprint_bond_driven(
            blueprint, catalog, ledger, energy,
        )
        .map(|(structure, _)| structure)
    }
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
    construct_calibration_structure(blueprint, catalog, &mut trial_ledger, &mut trial_energy)?;
    let required_initial_energy = TRIAL_ENERGY - trial_energy;
    if !required_initial_energy.is_finite() || required_initial_energy < 0.0 {
        return Err("juvenile construction produced an invalid energy requirement".into());
    }

    let mut ledger = EnergyLedger::default();
    let mut energy = required_initial_energy + reserve_energy;
    let mut structure =
        construct_calibration_structure(blueprint, catalog, &mut ledger, &mut energy)?;
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
    fn confirmed_seed_is_the_established_twelve_piece_nitrogen_ring() {
        let catalog = crate::resources::default_catalog();
        let blueprint = confirmed_seed_baseline(&catalog).expect("nitrogen seed should be valid");
        assert_eq!(blueprint.elements.len(), 12);
        assert_eq!(blueprint.connections.len(), 12);
        assert!(blueprint.elements.iter().all(|element| {
            element.material.parts.len() == 1
                && element.material.parts[0].0 == "Nitrogen"
                && element.material.parts[0].1 == 1.0
        }));
        assert!(blueprint.elements.iter().all(|element| {
            let x = element.placement.x;
            let y = element.placement.y;
            (x.abs() == 2.0 && y.abs() < 2.0) || (y.abs() == 2.0 && x.abs() < 2.0)
        }));
    }
}
