use crate::state::{Environment, Organism};
use crate::structure::Placement;

pub(crate) fn expel_physical_material(
    organism: &mut Organism,
    environment: &mut Environment,
    storage_index: usize,
) -> bool {
    let Some(origin) = organism.occupied_cells.first() else {
        return false;
    };
    let mut direction = crate::movement_direction::movement_direction_periodic(organism, environment.height);
    if direction.is_none() {
        let Some(stored) = organism.stored_material.entries.get(storage_index) else {
            return false;
        };
        let crate::material_storage::StoredMaterial::Physical(instance) = stored else {
            return false;
        };
        let Some(relative) = instance.owner_relative_origin else {
            return false;
        };
        let dx = relative.x;
        let dy = relative.y;
        let magnitude = dx.hypot(dy);
        if magnitude <= f64::EPSILON {
            return false;
        }
        direction = Some((dx / magnitude, dy / magnitude));
    }
    let Some(direction) = direction else {
        return false;
    };
    let Some(body) = crate::organism_geometry::OrganismBodyGeometry::from_structure(
        &organism.structure,
        &environment.catalog,
    ) else {
        return false;
    };
    let Some(mut physical) = organism.stored_material.take_physical_at(storage_index) else {
        return false;
    };
    let Some(placements) = physical.placements.as_ref() else {
        let _ = organism.stored_material.store_physical_instance(physical);
        return false;
    };
    if placements.len() != physical.material.parts.len() {
        let _ = organism.stored_material.store_physical_instance(physical);
        return false;
    }

    let mut body_support = f64::NEG_INFINITY;
    for part in &body.parts {
        let (sin, cos) = part.rotation_radians.sin_cos();
        let local_dx = direction.0 * cos + direction.1 * sin;
        let local_dy = -direction.0 * sin + direction.1 * cos;
        let Some(boundary) = crate::surface_geometry::boundary_point_toward(
            &crate::resources::Shape {
                form: part.form.clone(),
            },
            local_dx,
            local_dy,
        ) else {
            continue;
        };
        let world_x = boundary.x * cos - boundary.y * sin + part.x;
        let world_y = boundary.x * sin + boundary.y * cos + part.y;
        let projection = (world_x - origin.x) * direction.0 + (world_y - origin.y) * direction.1;
        body_support = body_support.max(projection);
    }
    if !body_support.is_finite() {
        let _ = organism.stored_material.store_physical_instance(physical);
        return false;
    }

    let owner_offset_projection = physical
        .owner_relative_origin
        .map(|origin| origin.x * direction.0 + origin.y * direction.1)
        .unwrap_or(0.0);
    let mut material_near = f64::INFINITY;
    for (index, placement) in placements.iter().enumerate() {
        let Some((name, _)) = physical.material.parts.get(index) else {
            let _ = organism.stored_material.store_physical_instance(physical);
            return false;
        };
        let Some(resource) = environment.catalog.iter().find(|r| r.name == *name) else {
            let _ = organism.stored_material.store_physical_instance(physical);
            return false;
        };
        let extent = resource.shape.form.bounding_radius();
        let projection =
            owner_offset_projection + placement.x * direction.0 + placement.y * direction.1
                - extent;
        material_near = material_near.min(projection);
    }
    if !material_near.is_finite() {
        let _ = organism.stored_material.store_physical_instance(physical);
        return false;
    }

    let translation = body_support - material_near + f64::EPSILON;
    if !translation.is_finite() || translation <= 0.0 {
        let _ = organism.stored_material.store_physical_instance(physical);
        return false;
    }
    let dx = direction.0 * translation;
    let dy = direction.1 * translation;
    let relative_origin = physical.owner_relative_origin.unwrap_or(Placement {
        x: 0.0,
        y: 0.0,
        rotation_radians: 0.0,
    });
    let (sin, cos) = relative_origin.rotation_radians.sin_cos();
    let first_placement = placements
        .first()
        .expect("validated placements are non-empty");
    let first_local_x = first_placement.x * cos - first_placement.y * sin;
    let first_local_y = first_placement.x * sin + first_placement.y * cos;
    let destination_x = origin.x + relative_origin.x + first_local_x + dx;
    let destination_y =
        (origin.y + relative_origin.y + first_local_y + dy).rem_euclid(environment.height);
    let Some(index) = environment
        .field
        .index_for_position(destination_x, destination_y)
    else {
        let _ = organism.stored_material.store_physical_instance(physical);
        return false;
    };
    if let Some(world_placements) = physical.placements.as_mut() {
        for placement in world_placements {
            let local_x = placement.x * cos - placement.y * sin;
            let local_y = placement.x * sin + placement.y * cos;
            placement.x = origin.x + relative_origin.x + local_x + dx;
            placement.y =
                (origin.y + relative_origin.y + local_y + dy).rem_euclid(environment.height);
            placement.rotation_radians += relative_origin.rotation_radians;
        }
    }
    environment.field.deposit_physical_at_index(index, physical)
}
