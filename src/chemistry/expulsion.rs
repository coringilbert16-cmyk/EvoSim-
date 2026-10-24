use crate::physical_material::PhysicalMaterial;
use crate::state::{Environment, Organism};
use crate::structure::Placement;

fn expulsion_shape_for_part(
    physical: &crate::physical_material::PhysicalMaterial,
    part_index: usize,
    catalog: &[crate::resources::BaseResource],
) -> Option<crate::resources::Shape> {
    // Stored fluid geometry is authoritative. Falling back to the catalog shape
    // here can place a deformed stored fluid as if it had never deformed.
    physical.realized_shape_for_part(part_index, catalog)
}

pub(crate) fn expel_physical_material(
    organism: &mut Organism,
    environment: &mut Environment,
    storage_index: usize,
) -> bool {
    expel_physical_material_with_deposit(
        organism,
        environment,
        storage_index,
        |environment, index, physical| environment.field.deposit_physical_at_index(index, physical),
    )
}

/// Execute expulsion with an injectable destination commit boundary. Production
/// uses the real field insertion; tests can reject that final commit after all
/// geometry preflight has passed, exercising the source-restoration branch.
fn expel_physical_material_with_deposit<F>(
    organism: &mut Organism,
    environment: &mut Environment,
    storage_index: usize,
    deposit: F,
) -> bool
where
    F: FnOnce(&mut Environment, usize, PhysicalMaterial) -> bool,
{
    let Some(origin) = organism.occupied_cells.first() else {
        return false;
    };
    let mut direction = if organism.experience_memory.spatial.is_empty() {
        None
    } else {
        crate::movement_direction::movement_direction_periodic(organism, environment.height, &[])
    };
    if direction.is_none() {
        let Some(stored) = organism.stored_material.entries.get(storage_index) else {
            return false;
        };
        let crate::material_storage::StoredMaterial::Physical(instance) = stored;
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
    let Some(crate::material_storage::StoredMaterial::Physical(stored)) =
        organism.stored_material.entries.get(storage_index)
    else {
        return false;
    };
    // Validate and compute the destination from a clone while the original
    // remains authoritative in storage. Removing the source before validation
    // makes every early return a potential material-loss path.
    let original = stored.clone();
    let mut physical = original.clone();
    let Some(placements) = physical.placements.as_ref() else {
        return false;
    };
    if placements.len() != physical.material.parts.len() {
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
        return false;
    }

    let owner_offset_projection = physical
        .owner_relative_origin
        .map(|origin| origin.x * direction.0 + origin.y * direction.1)
        .unwrap_or(0.0);
    let mut material_near = f64::INFINITY;
    for (index, placement) in placements.iter().enumerate() {
        let Some(shape) = expulsion_shape_for_part(&physical, index, &environment.catalog) else {
            return false;
        };
        let relative_origin = physical.owner_relative_origin.unwrap_or(Placement {
            x: 0.0,
            y: 0.0,
            rotation_radians: 0.0,
        });
        let total_rotation = relative_origin.rotation_radians + placement.rotation_radians;
        let (sin, cos) = total_rotation.sin_cos();
        let local_dx = direction.0 * cos + direction.1 * sin;
        let local_dy = -direction.0 * sin + direction.1 * cos;
        let Some(boundary) =
            crate::surface_geometry::boundary_point_toward(&shape, local_dx, local_dy)
        else {
            return false;
        };
        let boundary_projection = boundary.x * direction.0 * cos - boundary.y * direction.0 * sin
            + boundary.x * direction.1 * sin
            + boundary.y * direction.1 * cos;
        let center_projection = placement.x * (direction.0 * cos - direction.1 * sin)
            + placement.y * (direction.0 * sin + direction.1 * cos);
        let projection = owner_offset_projection + center_projection + boundary_projection;
        material_near = material_near.min(projection);
    }
    if !material_near.is_finite() {
        return false;
    }

    let translation = body_support - material_near + f64::EPSILON;
    if !translation.is_finite() || translation <= 0.0 {
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
    // All geometry checks are complete. Remove the authoritative source
    // only now, and restore the exact original entry if the destination rejects
    // the transformed instance.
    // Preserve inventory order during rollback. The ordinary take helper
    // uses swap_remove, which would reorder an unrelated last entry if the
    // destination rejects this expulsion.
    let Some(crate::material_storage::StoredMaterial::Physical(current)) =
        organism.stored_material.entries.get(storage_index)
    else {
        return false;
    };
    if !current.is_realized() {
        return false;
    }
    let crate::material_storage::StoredMaterial::Physical(removed) =
        organism.stored_material.entries.remove(storage_index);
    if deposit(environment, index, physical) {
        true
    } else {
        organism.stored_material.entries.insert(
            storage_index,
            crate::material_storage::StoredMaterial::Physical(removed),
        );
        false
    }
}

#[cfg(test)]
mod tests {
    use super::{expel_physical_material, expel_physical_material_with_deposit};
    use crate::material_storage::StoredMaterial;
    use crate::physical_material::PhysicalMaterial;
    use crate::resources::Material;
    use crate::state::Simulation;
    use crate::structure::Placement;

    #[test]
    fn expulsion_boundary_uses_authoritative_realized_fluid_shape() {
        let catalog = crate::resources::default_catalog();
        let water = catalog
            .iter()
            .find(|resource| resource.name == "Water")
            .unwrap();
        let area = match &water.shape.form {
            crate::resources::Form::Circle { radius } => std::f64::consts::PI * radius * radius,
            _ => panic!("default Water shape should have a measurable area"),
        };
        let family = crate::capillary_geometry::CapillaryContactFamily::solve(
            area,
            std::f64::consts::FRAC_PI_2,
        )
        .unwrap();
        let arc =
            crate::capillary_geometry::CircularArcBoundary::from_flat_contact(family).unwrap();
        let realized_shape = crate::resources::Shape {
            form: crate::resources::Form::Fluid {
                nominal_area: area,
                boundary: None,
                arc_boundary: Some(arc),
                multi_contact_boundary: None,
            },
        };
        let physical = PhysicalMaterial::realized(
            Material::free_base("Water", 1.0),
            vec![Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .unwrap()
        .with_single_fluid_shape(realized_shape.clone(), &catalog)
        .expect("valid conserved-area fluid shape should be retained");

        assert_eq!(
            super::expulsion_shape_for_part(&physical, 0, &catalog),
            Some(realized_shape),
            "expulsion must use the stored fluid's authoritative realized boundary"
        );
    }

    #[test]
    fn expulsion_preserves_deformed_fluid_shape_through_environment_transfer() {
        let mut simulation = Simulation::new(929, 10.0);
        for cell in &mut simulation.environment.field.cells {
            cell.physical_materials.clear();
        }
        let catalog = simulation.environment.catalog.clone();
        let water = catalog
            .iter()
            .find(|resource| resource.name == "Water")
            .unwrap();
        let area = match &water.shape.form {
            crate::resources::Form::Circle { radius } => std::f64::consts::PI * radius * radius,
            _ => panic!("default Water shape should have a measurable area"),
        };
        let family = crate::capillary_geometry::CapillaryContactFamily::solve(
            area,
            std::f64::consts::FRAC_PI_2,
        )
        .unwrap();
        let arc =
            crate::capillary_geometry::CircularArcBoundary::from_flat_contact(family).unwrap();
        let realized_shape = crate::resources::Shape {
            form: crate::resources::Form::Fluid {
                nominal_area: area,
                boundary: None,
                arc_boundary: Some(arc),
                multi_contact_boundary: None,
            },
        };
        let mut physical = PhysicalMaterial::realized(
            Material::free_base("Water", 1.0),
            vec![Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .unwrap()
        .with_single_fluid_shape(realized_shape.clone(), &catalog)
        .expect("valid conserved-area fluid shape should be retained");
        physical.owner_relative_origin = Some(Placement {
            x: 1.0,
            y: 0.0,
            rotation_radians: 0.0,
        });
        simulation.organisms[0]
            .stored_material
            .entries
            .push(StoredMaterial::Physical(physical));
        let storage_index = simulation.organisms[0].stored_material.entries.len() - 1;

        assert!(expel_physical_material(
            &mut simulation.organisms[0],
            &mut simulation.environment,
            storage_index,
        ));
        assert_eq!(
            simulation.organisms[0].stored_material.entries.len(),
            storage_index,
            "successful expulsion must remove exactly the source entry"
        );
        let deposited: Vec<_> = simulation
            .environment
            .field
            .cells
            .iter()
            .flat_map(|cell| cell.physical_materials.iter())
            .filter(|material| {
                material
                    .material
                    .parts
                    .first()
                    .is_some_and(|part| part.0 == "Water")
            })
            .collect();
        assert_eq!(
            deposited.len(),
            1,
            "the transferred fluid must remain a single object"
        );
        assert_eq!(
            deposited[0].realized_shape_for_part(0, &catalog),
            Some(realized_shape),
            "expulsion must not replace the realized fluid boundary with the catalog default"
        );
    }

    #[test]
    fn rejected_destination_restores_expulsion_source_and_neighbor_order() {
        let mut simulation = Simulation::new(927, 10.0);
        for cell in &mut simulation.environment.field.cells {
            cell.physical_materials.clear();
        }
        let catalog = simulation.environment.catalog.clone();
        let make = |name: &str, x: f64| {
            PhysicalMaterial::realized(
                Material::free_base(name, 1.0),
                vec![Placement {
                    x,
                    y: 0.0,
                    rotation_radians: 0.0,
                }],
                &catalog,
            )
            .expect("test material should realize")
        };
        let mut source = make("Carbon", 0.0);
        source.owner_relative_origin = Some(Placement {
            x: 1.0,
            y: 0.0,
            rotation_radians: 0.0,
        });
        let original = vec![
            StoredMaterial::Physical(make("Nitrogen", -1.0)),
            StoredMaterial::Physical(source),
            StoredMaterial::Physical(make("Phosphorus", 1.0)),
            StoredMaterial::Physical(make("Sulfur", 2.0)),
        ];
        simulation.organisms[0].stored_material.entries = original.clone();

        assert!(!expel_physical_material_with_deposit(
            &mut simulation.organisms[0],
            &mut simulation.environment,
            1,
            |_, _, _| false,
        ));
        assert_eq!(
            simulation.organisms[0].stored_material.entries,
            original,
            "a rejected destination commit must restore the exact source and preserve neighbor order"
        );
        assert!(
            simulation
                .environment
                .field
                .cells
                .iter()
                .all(|cell| cell.physical_materials.is_empty()),
            "a rejected destination must not leave a partial field insertion"
        );
    }

    #[test]
    fn malformed_stored_material_remains_owned_when_expulsion_validation_fails() {
        let mut simulation = Simulation::new(923, 10.0);
        let mut physical = PhysicalMaterial::realized(
            Material::free_base("Carbon", 1.0),
            vec![Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            &simulation.environment.catalog,
        )
        .expect("test material should realize");
        physical.owner_relative_origin = Some(Placement {
            x: 1.0,
            y: 0.0,
            rotation_radians: 0.0,
        });
        // Simulate malformed persisted state that the expulsion path must fail
        // closed on without deleting the source from its authoritative holder.
        physical.placements = None;
        simulation.organisms[0]
            .stored_material
            .entries
            .push(StoredMaterial::Physical(physical));
        let storage_index = simulation.organisms[0].stored_material.entries.len() - 1;
        let original_entries = simulation.organisms[0].stored_material.entries.clone();

        assert!(!expel_physical_material(
            &mut simulation.organisms[0],
            &mut simulation.environment,
            storage_index,
        ));
        assert_eq!(
            simulation.organisms[0].stored_material.entries, original_entries,
            "failed expulsion validation must not remove or rewrite the stored source"
        );
    }

    #[test]
    fn missing_destination_index_preserves_expulsion_source_and_neighbor_order() {
        let mut simulation = Simulation::new(928, 10.0);
        let catalog = simulation.environment.catalog.clone();
        let make = |name: &str, x: f64| {
            PhysicalMaterial::realized(
                Material::free_base(name, 1.0),
                vec![Placement {
                    x,
                    y: 0.0,
                    rotation_radians: 0.0,
                }],
                &catalog,
            )
            .expect("test material should realize")
        };
        let before = make("Nitrogen", 0.0);
        let water = catalog
            .iter()
            .find(|resource| resource.name == "Water")
            .unwrap();
        let area = match &water.shape.form {
            crate::resources::Form::Circle { radius } => std::f64::consts::PI * radius * radius,
            _ => panic!("default Water shape should have a measurable area"),
        };
        let family = crate::capillary_geometry::CapillaryContactFamily::solve(
            area,
            std::f64::consts::FRAC_PI_2,
        )
        .unwrap();
        let arc =
            crate::capillary_geometry::CircularArcBoundary::from_flat_contact(family).unwrap();
        let deformed_shape = crate::resources::Shape {
            form: crate::resources::Form::Fluid {
                nominal_area: area,
                boundary: None,
                arc_boundary: Some(arc),
                multi_contact_boundary: None,
            },
        };
        let mut source = PhysicalMaterial::realized(
            Material::free_base("Water", 1.0),
            vec![Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .unwrap()
        .with_single_fluid_shape(deformed_shape, &catalog)
        .expect("valid deformed fluid should be storable");
        source.owner_relative_origin = Some(Placement {
            x: 1.0,
            y: 0.0,
            rotation_radians: 0.0,
        });
        let after_one = make("Phosphorus", 0.0);
        let after_two = make("Sulfur", 0.0);
        simulation.organisms[0].stored_material.entries = vec![
            StoredMaterial::Physical(before),
            StoredMaterial::Physical(source),
            StoredMaterial::Physical(after_one),
            StoredMaterial::Physical(after_two),
        ];
        let original = simulation.organisms[0].stored_material.entries.clone();
        let storage_index = 1;
        // Force destination lookup to fail before source removal. This checks
        // the preflight guarantee; deposit-time rollback is not directly
        // injectable because deposit_physical_at_index validates synchronously.
        simulation.environment.field.cells.clear();

        assert!(!expel_physical_material(
            &mut simulation.organisms[0],
            &mut simulation.environment,
            storage_index,
        ));
        assert_eq!(simulation.organisms[0].stored_material.entries, original);
    }
}
