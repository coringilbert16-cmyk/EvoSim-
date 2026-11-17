//! One bounded, resonance-driven movement pass for environmental materials.

use crate::environment::ActiveMaterialField;
use crate::physical_material::PhysicalMaterial;
use crate::resources::BaseResource;

/// Apply one symmetric resonance movement update to free environmental objects.
///
/// Pair spectra are prepared once and every unordered pair is evaluated once.
/// Accepted translations use the field's existing atomic geometry/placement
/// transaction, which preserves each complete physical object.
pub(crate) fn apply_environmental_resonance_movement(
    field: &mut ActiveMaterialField,
    catalog: &[BaseResource],
    tick: u64,
) -> usize {
    field.assign_missing_material_identities();

    let mut identities = Vec::new();
    let mut materials: Vec<&PhysicalMaterial> = Vec::new();
    for material in field
        .cells
        .iter()
        .flat_map(|cell| cell.physical_materials.iter())
    {
        if material.environmental_identity == 0
            || material.owner_relative_origin.is_some()
            || !material.is_realized()
            || !material.material.is_valid_for_catalog(catalog)
            || material.material.is_empty()
        {
            continue;
        }
        identities.push(material.environmental_identity);
        materials.push(material);
    }

    // Opt-in phase timing keeps normal simulation runs free of profiling output.
    let profile_movement =
        std::env::var("EVOSIM_PROFILE_MOVEMENT").is_ok_and(|value| value == "1" || value == "true");

    let resonance_started = std::time::Instant::now();
    let Some(displacements) =
        crate::harmonics::environmental_resonance_displacements(&materials, catalog, tick)
    else {
        if profile_movement {
            eprintln!(
                "MOVEMENT_PROFILE tick={tick} phase=resonance elapsed_ns={} objects={}",
                resonance_started.elapsed().as_nanos(),
                materials.len()
            );
        }
        return 0;
    };
    let resonance_elapsed = resonance_started.elapsed();
    drop(materials);

    let proposals = identities
        .into_iter()
        .zip(displacements)
        .map(|(identity, (dx, dy))| (identity, dx, dy))
        .collect::<Vec<_>>();

    let placement_started = std::time::Instant::now();
    let moved = field.move_physical_materials_by_identity(&proposals, catalog);
    let placement_elapsed = placement_started.elapsed();

    if profile_movement {
        eprintln!(
            "MOVEMENT_PROFILE tick={tick} objects={} proposals={} moved={} resonance_ns={} placement_ns={}",
            proposals.len(),
            proposals.iter().filter(|(_, dx, dy)| *dx != 0.0 || *dy != 0.0).count(),
            moved,
            resonance_elapsed.as_nanos(),
            placement_elapsed.as_nanos()
        );
    }

    moved
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_environmental_movement_translates_whole_objects() {
        let catalog = crate::resources::default_catalog();
        let mut field = ActiveMaterialField::new(100.0, 100.0, 25.0);
        let make_carbon = |x| {
            PhysicalMaterial::realized(
                crate::resources::Material::free_base("Carbon", 1.0),
                vec![crate::structure::Placement {
                    x,
                    y: 50.0,
                    rotation_radians: 0.0,
                }],
                &catalog,
            )
            .expect("carbon should be a valid realized material")
        };
        assert!(field.deposit_physical(40.0, 50.0, make_carbon(40.0), &catalog));
        assert!(field.deposit_physical(50.0, 50.0, make_carbon(50.0), &catalog));
        field.assign_missing_material_identities();

        let before = field
            .cells
            .iter()
            .flat_map(|cell| cell.physical_materials.iter())
            .map(|material| {
                (
                    material.environmental_identity,
                    material.material.clone(),
                    material.placements.clone().unwrap(),
                )
            })
            .collect::<Vec<_>>();

        let moved = apply_environmental_resonance_movement(&mut field, &catalog, 0);
        assert_eq!(moved, 2);

        for (identity, material, placements) in before {
            let after = field
                .cells
                .iter()
                .flat_map(|cell| cell.physical_materials.iter())
                .find(|candidate| candidate.environmental_identity == identity)
                .expect("movement must preserve the material identity");
            assert_eq!(after.material, material);
            assert_ne!(after.placements.as_ref().unwrap(), &placements);
        }
    }

    fn make_carbon(catalog: &[BaseResource], x: f64, y: f64) -> PhysicalMaterial {
        PhysicalMaterial::realized(
            crate::resources::Material::free_base("Carbon", 1.0),
            vec![crate::structure::Placement {
                x,
                y,
                rotation_radians: 0.0,
            }],
            catalog,
        )
        .expect("carbon should be a valid realized material")
    }

    #[test]
    fn rejected_boundary_and_collision_moves_leave_the_field_unchanged() {
        let catalog = crate::resources::default_catalog();
        let mut field = ActiveMaterialField::new(100.0, 100.0, 25.0);
        assert!(field.deposit_physical(
            20.0,
            50.0,
            make_carbon(&catalog, 20.0, 50.0),
            &catalog
        ));
        assert!(field.deposit_physical(
            40.0,
            50.0,
            make_carbon(&catalog, 40.0, 50.0),
            &catalog
        ));
        field.assign_missing_material_identities();

        let identity = field
            .cells
            .iter()
            .flat_map(|cell| cell.physical_materials.iter())
            .find(|material| material.placements.as_ref().unwrap()[0].x == 20.0)
            .unwrap()
            .environmental_identity;
        let before_cells = field.cells.clone();
        let before_revision = field.revision;
        let before_cell_revisions = field.cell_revisions.clone();
        let before_amount = field.total_amount();
        let before_material = field.total_material();

        // The translated object would extend beyond the right field boundary.
        assert_eq!(
            field.move_physical_materials_by_identity(&[(identity, 100.0, 0.0)], &catalog),
            0
        );
        assert_eq!(field.cells, before_cells);
        assert_eq!(field.revision, before_revision);
        assert_eq!(field.cell_revisions, before_cell_revisions);

        // The translated object would penetrate the stationary object at x=40.
        assert_eq!(
            field.move_physical_materials_by_identity(&[(identity, 20.0, 0.0)], &catalog),
            0
        );
        assert_eq!(field.cells, before_cells);
        assert_eq!(field.revision, before_revision);
        assert_eq!(field.cell_revisions, before_cell_revisions);
        assert_eq!(field.total_amount(), before_amount);
        assert_eq!(field.total_material(), before_material);
    }

    #[test]
    fn moving_a_bonded_composite_preserves_its_complete_physical_identity() {
        use crate::resources::{InternalBond, Material};

        let catalog = crate::resources::default_catalog();
        let composite = Material {
            parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
            internal_bonds: vec![InternalBond { part_a: 0, part_b: 1 }],
        };
        let physical = PhysicalMaterial::realized(
            composite,
            vec![
                crate::structure::Placement {
                    x: 30.0,
                    y: 50.0,
                    rotation_radians: 0.0,
                },
                crate::structure::Placement {
                    x: 31.8,
                    y: 50.0,
                    rotation_radians: 0.0,
                },
            ],
            &catalog,
        )
        .expect("the bonded carbon-hydrogen composite should be realizable");
        let mut field = ActiveMaterialField::new(100.0, 100.0, 25.0);
        assert!(field.deposit_physical(30.0, 50.0, physical, &catalog));
        field.assign_missing_material_identities();

        let before = field.cells.iter()
            .flat_map(|cell| cell.physical_materials.iter())
            .next()
            .unwrap()
            .clone();
        let before_amount = field.total_amount();
        let before_material = field.total_material();

        assert_eq!(
            field.move_physical_materials_by_identity(
                &[(before.environmental_identity, 2.0, 3.0)],
                &catalog,
            ),
            1
        );

        let after_locations = field
            .cells
            .iter()
            .enumerate()
            .flat_map(|(cell_index, cell)| {
                cell.physical_materials
                    .iter()
                    .map(move |material| (cell_index, material))
            })
            .collect::<Vec<_>>();
        assert_eq!(after_locations.len(), 1);
        let (cell_index, after) = after_locations[0];
        assert_eq!(after.environmental_identity, before.environmental_identity);
        assert_eq!(after.material, before.material);
        assert_eq!(after.internal_connections, before.internal_connections);
        assert_eq!(after.realized_shapes, before.realized_shapes);
        assert_eq!(after.owner_relative_origin, before.owner_relative_origin);
        let before_placements = before.placements.as_ref().unwrap();
        let after_placements = after.placements.as_ref().unwrap();
        assert_eq!(after_placements.len(), before_placements.len());
        for (old, new) in before_placements.iter().zip(after_placements) {
            assert!((new.x - (old.x + 2.0)).abs() < 1e-12);
            assert!((new.y - (old.y + 3.0)).abs() < 1e-12);
            assert_eq!(new.rotation_radians, old.rotation_radians);
        }
        assert_eq!(
            field.index_for_position(after_placements[0].x, after_placements[0].y),
            Some(cell_index)
        );
        assert_eq!(field.total_amount(), before_amount);
        assert_eq!(field.total_material(), before_material);
    }
}
