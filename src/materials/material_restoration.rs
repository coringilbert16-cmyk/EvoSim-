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
/// Restore an intact stored physical material into `structure` at `origin`,
/// returning a stable rejection reason for constructor diagnostics.
///
/// The stored constituent arrangement and stored internal bond endpoints are
/// authoritative. Restoration never invents internal bonds or performs COMBINE.
pub(crate) fn restore_material_diagnostic(
    structure: &mut OrganismStructure,
    instance: &PhysicalMaterial,
    origin: Placement,
    catalog: &[BaseResource],
) -> Result<Vec<usize>, &'static str> {
    let relative = instance
        .placements
        .as_ref()
        .ok_or("missing stored constituent placements")?;
    let connections = instance
        .internal_connections
        .as_ref()
        .ok_or("missing stored internal connections")?;
    let material = &instance.material;
    if !material.is_valid() || material.parts.is_empty() || relative.len() != material.parts.len() {
        return Err("invalid material or constituent-placement count");
    }
    if connections.len() != material.internal_bonds.len()
        || connections.iter().any(|connection| {
            connection.part_a >= material.parts.len() || connection.part_b >= material.parts.len()
        })
    {
        return Err("stored internal-connection mapping does not match material bonds");
    }

    let mut trial = structure.clone();
    let mut indices = Vec::with_capacity(material.parts.len());
    for (part_index, ((name, amount), placement)) in
        material.parts.iter().zip(relative.iter()).enumerate()
    {
        if amount.fract().abs() > EPSILON || (*amount - 1.0).abs() > EPSILON {
            return Err("constituent amount is not one whole physical unit");
        }
        let resource = catalog
            .iter()
            .find(|resource| resource.name == *name)
            .ok_or("constituent resource missing from catalog")?;
        let mut unit = StructuralUnit::from_material(
            crate::resources::Material::free_base(name.clone(), *amount),
            transform_relative(origin, *placement),
        )
        .ok_or("could not create structural unit from stored constituent")?;
        if !unit.realize_default_geometry(catalog) {
            return Err("could not realize constituent default geometry");
        }
        let realized_shape = instance
            .realized_shape_for_part(part_index, catalog)
            .ok_or("stored realized shape is invalid for constituent")?;
        if realized_shape != resource.shape && !unit.realize_fluid_geometry(realized_shape, catalog)
        {
            return Err("could not restore stored realized fluid geometry");
        }
        indices.push(trial.add_unit(unit));
    }

    // A stored physical material is authoritative, but it must still be
    // geometrically self-consistent when restored. Bonded contact is allowed;
    // physical penetration is not.
    for left in 0..indices.len() {
        let left_shape = trial.units[indices[left]]
            .shape(catalog)
            .ok_or("restored constituent has no physical shape")?;
        let left_part = crate::material_geometry::PlacedMaterialPart {
            part_index: left,
            form: left_shape.form.clone(),
            placement: trial.units[indices[left]].placement,
        };
        for right in (left + 1)..indices.len() {
            let right_shape = trial.units[indices[right]]
                .shape(catalog)
                .ok_or("restored constituent has no physical shape")?;
            let right_part = crate::material_geometry::PlacedMaterialPart {
                part_index: right,
                form: right_shape.form.clone(),
                placement: trial.units[indices[right]].placement,
            };
            if crate::material_geometry::placed_forms_penetrate(&left_part, &right_part, 0.0) {
                return Err("stored constituents penetrate after origin transform");
            }
        }
    }

    for connection in connections {
        let unit_a = *indices
            .get(connection.part_a)
            .ok_or("internal bond part A is missing")?;
        let unit_b = *indices
            .get(connection.part_b)
            .ok_or("internal bond part B is missing")?;
        let id_a = trial
            .physical_id(unit_a)
            .ok_or("internal bond part A has no physical ID")?;
        let id_b = trial
            .physical_id(unit_b)
            .ok_or("internal bond part B has no physical ID")?;
        let a = trial.units[unit_a]
            .properties(catalog)
            .ok_or("internal bond part A has no properties")?;
        let b = trial.units[unit_b]
            .properties(catalog)
            .ok_or("internal bond part B has no properties")?;
        let strength = crate::combine::bond_strength(a, b);
        if !strength.is_finite() {
            return Err("restored internal bond has non-finite strength");
        }
        let bond = Bond {
            endpoint_a: BondEndpoint::new(id_a, connection.endpoint_a),
            endpoint_b: BondEndpoint::new(id_b, connection.endpoint_b),
            strength,
            // This bond predates organism admission. Restoration therefore
            // carries no new COMBINE investment or energy transaction.
            bond_energy: 0.0,
        };
        crate::contact::try_add_bond(&mut trial, bond, catalog)
            .map_err(|_| "stored internal bond endpoint failed live validation")?;
    }

    *structure = trial;
    Ok(indices)
}

/// Compatibility wrapper for callers that only need success/failure.
pub(crate) fn restore_material(
    structure: &mut OrganismStructure,
    instance: &PhysicalMaterial,
    origin: Placement,
    catalog: &[BaseResource],
) -> Option<Vec<usize>> {
    restore_material_diagnostic(structure, instance, origin, catalog).ok()
}
