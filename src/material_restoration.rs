//! Restoration of an already-physical material into the organism graph.
//!
//! Restoration is deliberately distinct from COMBINE. It does not create
//! chemistry, charge energy, or establish a new external bond. It expands an
//! intact stored material into its constituent physical units and restores the
//! exact internal bonds that already belonged to that material.
use crate::physical_material::PhysicalMaterial;
use crate::resources::BaseResource;
use crate::structure::{Bond, BondEndpoint, OrganismStructure, Placement, StructuralUnit};

const EPSILON: f64 = 1e-9;

fn transform_relative(origin: Placement, relative: Placement) -> Placement {
    let (sin, cos) = origin.rotation_radians.sin_cos();
    Placement {
        x: origin.x + relative.x * cos - relative.y * sin,
        y: origin.y + relative.x * sin + relative.y * cos,
        rotation_radians: origin.rotation_radians + relative.rotation_radians,
    }
}

/// Restore an intact stored physical material into `structure` at `origin`.
///
/// The stored constituent arrangement and stored internal bond endpoints are
/// authoritative. If either is absent, restoration fails rather than deriving
/// a new physical connection from composition and geometry.
pub(crate) fn restore_material(
    structure: &mut OrganismStructure,
    instance: &PhysicalMaterial,
    origin: Placement,
    catalog: &[BaseResource],
) -> Option<Vec<usize>> {
    let relative = instance.placements.as_ref()?;
    let connections = instance.internal_connections.as_ref()?;
    let material = &instance.material;
    if !material.is_valid() || material.parts.is_empty() || relative.len() != material.parts.len() {
        return None;
    }
    if connections.len() != material.internal_bonds.len()
        || connections.iter().any(|connection| {
            connection.part_a >= material.parts.len() || connection.part_b >= material.parts.len()
        })
    {
        return None;
    }

    let mut trial = structure.clone();
    let mut indices = Vec::with_capacity(material.parts.len());
    for ((name, amount), placement) in material.parts.iter().zip(relative.iter()) {
        if amount.fract().abs() > EPSILON || (*amount - 1.0).abs() > EPSILON {
            return None;
        }
        let mut unit = StructuralUnit::from_material(
            crate::resources::Material::free_base(name.clone(), *amount),
            transform_relative(origin, *placement),
        )?;
        if !unit.realize_default_geometry(catalog) {
            return None;
        }
        indices.push(trial.add_unit(unit));
    }

    for connection in connections {
        let unit_a = *indices.get(connection.part_a)?;
        let unit_b = *indices.get(connection.part_b)?;
        let id_a = trial.physical_id(unit_a)?;
        let id_b = trial.physical_id(unit_b)?;
        let a = trial.units[unit_a].properties(catalog)?;
        let b = trial.units[unit_b].properties(catalog)?;
        let strength = crate::combine::bond_strength(a, b);
        if !strength.is_finite() {
            return None;
        }
        let bond = Bond {
            endpoint_a: BondEndpoint::new(id_a, connection.endpoint_a),
            endpoint_b: BondEndpoint::new(id_b, connection.endpoint_b),
            strength,
            // This bond predates organism admission. Restoration therefore
            // carries no new COMBINE investment or energy transaction.
            bond_energy: 0.0,
        };
        crate::contact::try_add_bond(&mut trial, bond, catalog).ok()?;
    }

    *structure = trial;
    Some(indices)
}
