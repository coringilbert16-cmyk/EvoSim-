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

/// Recover the material origin from one constituent's absolute placement.
///
/// restore_material applies the relative placement as a rigid transform:
/// absolute = origin composed with relative. This helper is the exact inverse,
/// allowing configuration-space placement to operate on constituent geometry
/// while restoration still receives the material-level origin.
pub(crate) fn origin_for_relative_placement(
    absolute: Placement,
    relative: Placement,
) -> Placement {
    let origin_rotation = absolute.rotation_radians - relative.rotation_radians;
    let (sin, cos) = origin_rotation.sin_cos();
    Placement {
        x: absolute.x - (relative.x * cos - relative.y * sin),
        y: absolute.y - (relative.x * sin + relative.y * cos),
        rotation_radians: origin_rotation,
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

    // A stored physical material is authoritative, but it must still be
    // geometrically self-consistent when restored. Bonded contact is allowed;
    // physical penetration is not.
    for left in 0..indices.len() {
        let Some(left_shape) = trial.units[indices[left]].shape(catalog) else {
            return None;
        };
        let left_part = crate::material_geometry::PlacedMaterialPart {
            part_index: left,
            form: left_shape.form.clone(),
            placement: trial.units[indices[left]].placement,
        };
        for right in (left + 1)..indices.len() {
            let Some(right_shape) = trial.units[indices[right]].shape(catalog) else {
                return None;
            };
            let right_part = crate::material_geometry::PlacedMaterialPart {
                part_index: right,
                form: right_shape.form.clone(),
                placement: trial.units[indices[right]].placement,
            };
            if crate::material_geometry::placed_forms_penetrate(&left_part, &right_part, 0.0) {
                return None;
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
        let base_strength = crate::combine::bond_strength(a, b);
        if !base_strength.is_finite() {
            return None;
        }
        let feature_a = crate::contact::contact_feature_measurement(
            &trial.units[unit_a],
            connection.endpoint_a,
            catalog,
        )?;
        let feature_b = crate::contact::contact_feature_measurement(
            &trial.units[unit_b],
            connection.endpoint_b,
            catalog,
        )?;
        let contact_factor = feature_a
            .feature
            .bond_strength_factor(feature_b.feature)
            .unwrap_or(1.0);
        let strength = base_strength * contact_factor;
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


#[cfg(test)]
mod tests {
    use super::origin_for_relative_placement;
    use crate::structure::Placement;

    #[test]
    fn origin_for_relative_placement_is_exact_inverse() {
        let origin = Placement {
            x: 3.25,
            y: -1.75,
            rotation_radians: 0.73,
        };
        let relative = Placement {
            x: 1.4,
            y: -0.8,
            rotation_radians: -0.31,
        };
        let absolute = {
            let (sin, cos) = origin.rotation_radians.sin_cos();
            Placement {
                x: origin.x + relative.x * cos - relative.y * sin,
                y: origin.y + relative.x * sin + relative.y * cos,
                rotation_radians: origin.rotation_radians + relative.rotation_radians,
            }
        };
        let recovered = origin_for_relative_placement(absolute, relative);
        assert!((recovered.x - origin.x).abs() < 1e-12);
        assert!((recovered.y - origin.y).abs() < 1e-12);
        assert!((recovered.rotation_radians - origin.rotation_radians).abs() < 1e-12);
    }
}
