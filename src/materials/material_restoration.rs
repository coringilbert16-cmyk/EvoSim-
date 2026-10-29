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

fn is_finite_placement(placement: Placement) -> bool {
    placement.x.is_finite() && placement.y.is_finite() && placement.rotation_radians.is_finite()
}

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
///
/// This helper also stages candidate graphs, so it validates the restored
/// material's own geometry and connections but does not impose a destination-wide
/// overlap policy. Callers must validate new-versus-existing geometry before
/// committing a candidate into a nonempty structure.
pub(crate) fn restore_material(
    structure: &mut OrganismStructure,
    instance: &PhysicalMaterial,
    origin: Placement,
    catalog: &[BaseResource],
) -> Option<Vec<usize>> {
    macro_rules! reject { () => {{ eprintln!("restore_material rejected at line {}", line!()); reject!(); }}; }
    let relative = instance.placements.as_ref()?;
    let connections = instance.internal_connections.as_ref()?;
    let material = &instance.material;
    if !is_finite_placement(origin)
        || relative
            .iter()
            .any(|placement| !is_finite_placement(*placement))
        || !material.is_valid_for_catalog(catalog)
        || material.parts.is_empty()
        || relative.len() != material.parts.len()
    {
        reject!();
    }
    if connections.len() != material.internal_bonds.len()
        || connections.iter().any(|connection| {
            connection.part_a >= material.parts.len() || connection.part_b >= material.parts.len()
        })
        || connections
            .iter()
            .zip(&material.internal_bonds)
            .any(|(connection, bond)| {
                !((connection.part_a == bond.part_a && connection.part_b == bond.part_b)
                    || (connection.part_a == bond.part_b && connection.part_b == bond.part_a))
            })
    {
        reject!();
    }

    let mut trial = structure.clone();
    let mut indices = Vec::with_capacity(material.parts.len());
    for (part_index, ((name, amount), placement)) in
        material.parts.iter().zip(relative.iter()).enumerate()
    {
        let resource = catalog.iter().find(|resource| resource.name == *name)?;
        if (*amount).fract().abs() > EPSILON
            || *amount < 1.0
            || ((*amount - 1.0).abs() > EPSILON
                && (material.parts.len() != 1
                    || material.has_internal_structure()
                    || resource.physical_state != crate::resources::PhysicalState::Fluid))
        {
            reject!();
        }
        let transformed = transform_relative(origin, *placement);
        if !is_finite_placement(transformed) {
            reject!();
        }
        let mut unit = StructuralUnit::from_material(
            crate::resources::Material::free_base(name.clone(), *amount),
            transformed,
        )?;
        if !unit.realize_default_geometry(catalog) {
            reject!();
        }
        let realized_shape = instance.realized_shape_for_part(part_index, catalog)?;
        if realized_shape != resource.shape && !unit.realize_fluid_geometry(realized_shape, catalog)
        {
            reject!();
        }
        indices.push(trial.add_unit(unit));
    }

    // A stored physical material is authoritative, but it must still be
    // geometrically self-consistent when restored. Bonded contact is allowed;
    // physical penetration is not.
    for (left, &left_index) in indices.iter().enumerate() {
        let left_shape = trial.units[left_index].shape(catalog)?;
        let left_part = crate::material_geometry::PlacedMaterialPart {
            part_index: left,
            form: left_shape.form.clone(),
            placement: trial.units[left_index].placement,
        };
        for (right, &right_index) in indices.iter().enumerate().skip(left + 1) {
            let right_shape = trial.units[right_index].shape(catalog)?;
            let right_part = crate::material_geometry::PlacedMaterialPart {
                part_index: right,
                form: right_shape.form.clone(),
                placement: trial.units[right_index].placement,
            };
            if crate::material_geometry::placed_forms_penetrate(&left_part, &right_part, 0.0) {
                reject!();
            }
        }
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
            reject!();
        }
        let bond = Bond {
            endpoint_a: BondEndpoint::new(id_a, connection.endpoint_a),
            endpoint_b: BondEndpoint::new(id_b, connection.endpoint_b),
            strength,
            // Preserve energy already allocated to this physical bond.
            // Restoration is not a new reaction and therefore charges nothing.
            bond_energy: connection.bond_energy,
        };
        if let Err(reason) = crate::contact::try_add_bond(&mut trial, bond, catalog) { eprintln!("restore_material bond insertion rejected at line {}: {reason}", line!()); return None; }
    }

    *structure = trial;
    Some(indices)
}
