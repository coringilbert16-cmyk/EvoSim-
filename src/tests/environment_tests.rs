use super::*;
use crate::resources::{InternalBond, Material};

fn make_raw(name: &str, amount: f64) -> Material {
    Material::free_base(name, amount)
}

fn make_structured(amount: f64) -> Material {
    Material {
        parts: vec![
            ("Carbon".into(), amount / 2.0),
            ("Hydrogen".into(), amount / 2.0),
        ],
        internal_bonds: vec![InternalBond {
            part_a: 0,
            part_b: 1,
        }],
    }
}

#[test]
fn field_has_expected_dimensions() {
    let field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
    assert_eq!(field.width_cells, 40);
    assert_eq!(field.height_cells, 40);
    assert_eq!(field.cells.len(), 1600);
}

#[test]
fn field_starts_empty() {
    let field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
    assert_eq!(field.total_amount(), 0.0);
    assert!(field.total_material().is_empty());
}

#[test]
fn horizontal_bounds_remain_closed_while_vertical_positions_wrap() {
    let field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
    assert!(field.index_for_position(-1.0, 5.0).is_none());
    assert_eq!(
        field.index_for_position(5.0, 1000.0),
        field.index_for_position(5.0, 0.0)
    );
    assert_eq!(
        field.index_for_position(5.0, -0.1),
        field.index_for_position(5.0, 999.9)
    );
    assert!(field.index_for_position(1000.0, 5.0).is_none());
    assert!(field.index_for_position(f64::NAN, 5.0).is_none());
}

#[test]
fn vertical_neighbors_wrap_from_top_to_bottom() {
    let field = ActiveMaterialField::new(100.0, 100.0, 25.0);
    let top = field.index_for_position(37.5, 12.5).unwrap();
    let bottom = field.index_for_position(37.5, 87.5).unwrap();
    assert!(field.neighbor_indices(top).contains(&bottom));
    assert!(field.neighbor_indices(bottom).contains(&top));
}

#[test]
fn deposit_preserves_distinct_structures_and_aggregates_raw_stock() {
    let mut field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
    let catalog = crate::resources::default_catalog();
    field.deposit(500.0, 500.0, make_structured(10.0), &catalog);
    field.deposit(500.0, 500.0, make_structured(10.0), &catalog);
    field.deposit(500.0, 500.0, make_raw("Carbon", 3.0), &catalog);
    field.deposit(500.0, 500.0, make_raw("Carbon", 7.0), &catalog);
    let cell = &field.cells[field.index_for_position(500.0, 500.0).unwrap()];
    assert_eq!(cell.materials.len(), 3);
    assert_eq!(
        cell.materials
            .iter()
            .filter(|m| m.has_internal_structure())
            .count(),
        2
    );
    assert!((cell.total_amount() - 30.0).abs() < 1e-9);
    assert_eq!(
        cell.materials
            .iter()
            .filter(|m| !m.has_internal_structure())
            .count(),
        1
    );
}

#[test]
fn cells_within_radius_returns_only_cells_inside_radius() {
    let field = ActiveMaterialField::new(100.0, 100.0, 25.0);
    assert_eq!(field.cells_within_radius(37.5, 37.5, 1.0), vec![5]);
}

#[test]
fn cells_within_radius_handles_grid_edges_and_invalid_input() {
    let field = ActiveMaterialField::new(100.0, 100.0, 25.0);
    let cells = field.cells_within_radius(0.0, 0.0, 20.0);
    assert!(cells.contains(&0));
    assert!(cells.iter().all(|&i| i < field.cells.len()));
    assert!(field.cells_within_radius(f64::NAN, 0.0, 10.0).is_empty());
    assert!(field.cells_within_radius(0.0, 0.0, -1.0).is_empty());
}

#[test]
fn local_environment_revision_distinguishes_same_revision_totals_in_different_cells() {
    let mut first = ActiveMaterialField::new(100.0, 100.0, 25.0);
    let mut second = ActiveMaterialField::new(100.0, 100.0, 25.0);
    let first_position = (12.5, 12.5);
    let second_position = (37.5, 12.5);

    let first_index = first
        .index_for_position(first_position.0, first_position.1)
        .unwrap();
    let second_index = second
        .index_for_position(second_position.0, second_position.1)
        .unwrap();

    first.mark_changed_at_index(first_index);
    first.mark_changed_at_index(first_index);
    second.mark_changed_at_index(second_index);
    second.mark_changed_at_index(second_index);

    let first_key =
        first.local_revision_for_positions([first_position, second_position].into_iter());
    let second_key =
        second.local_revision_for_positions([first_position, second_position].into_iter());

    assert_ne!(first_key, second_key);
}

#[test]
fn exact_fluid_arc_region_check_allows_chord_contact_but_rejects_curve_escape() {
    let family =
        crate::capillary_geometry::CapillaryContactFamily::solve(0.5, std::f64::consts::FRAC_PI_2)
            .unwrap();
    let arc = crate::capillary_geometry::CircularArcBoundary::from_flat_contact(family).unwrap();
    let form = crate::resources::Form::Fluid {
        nominal_area: 0.5,
        boundary: None,
        arc_boundary: Some(arc),
        multi_contact_boundary: None,
    };
    let inside_region = crate::interior_geometry::EnclosedRegion {
        area: 8.0,
        boundary_units: vec![],
        sample_point: (0.0, 1.0),
        boundary: vec![(-2.0, 0.0), (2.0, 0.0), (2.0, 2.0), (-2.0, 2.0)],
    };
    assert!(ActiveMaterialField::realized_form_fully_inside_region(
        &form,
        crate::structure::Placement {
            x: 0.0,
            y: 0.0,
            rotation_radians: 0.0,
        },
        &inside_region,
    ));

    let too_shallow_region = crate::interior_geometry::EnclosedRegion {
        area: 6.0,
        boundary_units: vec![],
        sample_point: (0.0, 0.0),
        boundary: vec![(-2.0, -1.0), (2.0, -1.0), (2.0, 0.4), (-2.0, 0.4)],
    };
    assert!(!ActiveMaterialField::realized_form_fully_inside_region(
        &form,
        crate::structure::Placement {
            x: 0.0,
            y: 0.0,
            rotation_radians: 0.0,
        },
        &too_shallow_region,
    ));
}

#[test]
fn physical_batch_deposit_rejects_late_invalid_destination_without_partial_commit() {
    use crate::physical_material::PhysicalMaterial;
    use crate::state::Simulation;
    use crate::structure::Placement;

    let simulation = Simulation::new(771, 10.0);
    let catalog = &simulation.environment.catalog;
    let make_carbon = |x: f64| {
        PhysicalMaterial::realized(
            Material::free_base("Carbon", 1.0),
            vec![Placement {
                x,
                y: 12.5,
                rotation_radians: 0.0,
            }],
            catalog,
        )
        .expect("carbon should be physically realizable")
    };
    let mut field = ActiveMaterialField::new(100.0, 100.0, 25.0);
    let before_revision = field.revision;
    let before_cells = field.cells.clone();

    assert!(!field.deposit_physical_batch(
        vec![
            (12.5, 12.5, make_carbon(12.5)),
            (100.0, 12.5, make_carbon(37.5)),
        ],
        catalog,
    ));

    assert_eq!(field.cells, before_cells);
    assert_eq!(field.revision, before_revision);
}

#[test]
fn physical_batch_deposit_commits_all_validated_objects() {
    use crate::physical_material::PhysicalMaterial;
    use crate::state::Simulation;
    use crate::structure::Placement;

    let simulation = Simulation::new(772, 10.0);
    let catalog = &simulation.environment.catalog;
    let make_carbon = |x: f64| {
        PhysicalMaterial::realized(
            Material::free_base("Carbon", 1.0),
            vec![Placement {
                x,
                y: 12.5,
                rotation_radians: 0.0,
            }],
            catalog,
        )
        .expect("carbon should be physically realizable")
    };
    let mut field = ActiveMaterialField::new(100.0, 100.0, 25.0);

    assert!(field.deposit_physical_batch(
        vec![
            (12.5, 12.5, make_carbon(12.5)),
            (37.5, 12.5, make_carbon(37.5)),
        ],
        catalog,
    ));

    assert_eq!(
        field
            .cells
            .iter()
            .map(|cell| cell.physical_materials.len())
            .sum::<usize>(),
        2
    );
    assert_eq!(field.revision, 2);
}

#[test]
fn physical_batch_deposit_rejects_late_malformed_object_without_partial_commit() {
    use crate::physical_material::PhysicalMaterial;
    use crate::state::Simulation;
    use crate::structure::Placement;

    let simulation = Simulation::new(773, 10.0);
    let catalog = &simulation.environment.catalog;
    let valid = PhysicalMaterial::realized(
        Material::free_base("Carbon", 1.0),
        vec![Placement {
            x: 12.5,
            y: 12.5,
            rotation_radians: 0.0,
        }],
        catalog,
    )
    .expect("carbon should be physically realizable");
    let malformed = PhysicalMaterial {
        material: Material::free_base("Carbon", 1.0),
        placements: None,
        internal_connections: None,
        owner_relative_origin: None,
        realized_shapes: None,
    };
    let mut field = ActiveMaterialField::new(100.0, 100.0, 25.0);
    let before_cells = field.cells.clone();
    let before_revision = field.revision;

    assert!(!field.deposit_physical_batch(
        vec![(12.5, 12.5, valid), (37.5, 12.5, malformed)],
        &crate::resources::default_catalog(),
    ));
    assert_eq!(field.cells, before_cells);
    assert_eq!(field.revision, before_revision);
}

#[test]
fn logical_deposit_rejects_invalid_payload_without_claiming_success() {
    let mut field = ActiveMaterialField::new(50.0, 50.0, 25.0);
    let before_cells = field.cells.clone();
    let before_revision = field.revision;

    assert!(!field.deposit(
        12.5,
        12.5,
        make_raw("Carbon", 1.5),
        &crate::resources::default_catalog(),
    ));
    assert_eq!(field.cells, before_cells);
    assert_eq!(field.revision, before_revision);

    assert!(!field.deposit(
        12.5,
        12.5,
        Material {
            parts: Vec::new(),
            internal_bonds: Vec::new(),
        },
        &crate::resources::default_catalog(),
    ));
    assert_eq!(field.cells, before_cells);
    assert_eq!(field.revision, before_revision);

    assert!(!field.deposit(
        12.5,
        12.5,
        make_raw("UnknownResource", 1.0),
        &crate::resources::default_catalog(),
    ));
    assert_eq!(field.cells, before_cells);
    assert_eq!(field.revision, before_revision);
}

#[test]
fn logical_deposit_rejects_invalid_index_without_mutating_field() {
    let mut field = ActiveMaterialField::new(50.0, 50.0, 25.0);
    let before_cells = field.cells.clone();
    let before_revision = field.revision;

    assert!(!field.deposit_at_index(
        field.cells.len(),
        make_raw("Carbon", 1.0),
        &crate::resources::default_catalog(),
    ));
    assert_eq!(field.cells, before_cells);
    assert_eq!(field.revision, before_revision);
}

#[test]
fn logical_deposit_does_not_absorb_invalid_existing_stock() {
    let catalog = crate::resources::default_catalog();
    let mut field = ActiveMaterialField::new(50.0, 50.0, 25.0);
    field.cells[0].materials.push(make_raw("Carbon", 1e-10));
    let before_cells = field.cells.clone();
    let before_revision = field.revision;

    assert!(
        !field.deposit_at_index(0, make_raw("Carbon", 1.0), &catalog),
        "a valid deposit must not normalize corrupt fractional stock into a valid total"
    );
    assert_eq!(field.cells, before_cells);
    assert_eq!(field.revision, before_revision);
}

#[test]
fn logical_deposit_rejects_unrepresentable_merged_inventory_atomically() {
    let catalog = crate::resources::default_catalog();
    let mut field = ActiveMaterialField::new(50.0, 50.0, 25.0);
    field.cells[0]
        .materials
        .push(make_raw("Carbon", (u64::MAX as f64) - 2048.0));
    let before_cells = field.cells.clone();
    let before_revision = field.revision;

    assert!(
        !field.deposit_at_index(0, make_raw("Carbon", 4096.0), &catalog),
        "merging individually valid stock must reject an unrepresentable combined count"
    );
    assert_eq!(
        field.cells, before_cells,
        "rejected merge must preserve stock"
    );
    assert_eq!(
        field.revision, before_revision,
        "rejected merge must not invalidate field caches"
    );
}

#[test]
fn physical_deposits_reject_unknown_resource_identity_without_mutation() {
    use crate::physical_material::PhysicalMaterial;
    use crate::structure::Placement;

    let catalog = crate::resources::default_catalog();
    let mut physical = PhysicalMaterial::realized(
        make_raw("Carbon", 1.0),
        vec![Placement {
            x: 12.5,
            y: 12.5,
            rotation_radians: 0.0,
        }],
        &catalog,
    )
    .expect("carbon should be physically realizable");
    physical.material.parts[0].0 = "UnknownResource".to_owned();

    let mut field = ActiveMaterialField::new(50.0, 50.0, 25.0);
    let before_cells = field.cells.clone();
    let before_revision = field.revision;

    assert!(!field.deposit_physical_at_index(0, physical.clone(), &catalog));
    assert!(!field.deposit_physical_batch(vec![(12.5, 12.5, physical)], &catalog,));
    assert_eq!(field.cells, before_cells);
    assert_eq!(field.revision, before_revision);
}

#[test]
fn physical_split_rejects_stale_source_without_mutating_field() {
    use crate::physical_material::PhysicalMaterial;
    use crate::state::Simulation;
    use crate::structure::Placement;

    let simulation = Simulation::new(773, 10.0);
    let catalog = &simulation.environment.catalog;
    let source = PhysicalMaterial::realized(
        make_structured(2.0),
        vec![
            Placement {
                x: 12.5,
                y: 12.5,
                rotation_radians: 0.0,
            },
            Placement {
                x: 14.3,
                y: 12.5,
                rotation_radians: 0.0,
            },
        ],
        catalog,
    )
    .expect("test composite should be physically realizable");
    let wrong_source = PhysicalMaterial::realized(
        Material::free_base("Carbon", 1.0),
        vec![Placement {
            x: 12.5,
            y: 12.5,
            rotation_radians: 0.0,
        }],
        catalog,
    )
    .expect("carbon should be physically realizable");
    let mut field = ActiveMaterialField::new(100.0, 100.0, 25.0);
    let cell = field.index_for_position(12.5, 12.5).unwrap();
    assert!(field.deposit_physical_at_index(cell, source, catalog));
    let before_cells = field.cells.clone();
    let before_revision = field.revision;

    assert!(!field.split_physical_at_index(
        cell,
        0,
        &wrong_source,
        vec![wrong_source.clone()],
        catalog,
    ));
    assert_eq!(field.cells, before_cells);
    assert_eq!(field.revision, before_revision);
}

#[test]
fn physical_split_replaces_source_only_after_every_piece_destination_is_valid() {
    use crate::physical_material::PhysicalMaterial;
    use crate::state::Simulation;
    use crate::structure::Placement;

    let simulation = Simulation::new(774, 10.0);
    let catalog = &simulation.environment.catalog;
    let source = PhysicalMaterial::realized(
        make_structured(2.0),
        vec![
            Placement {
                x: 12.5,
                y: 12.5,
                rotation_radians: 0.0,
            },
            Placement {
                x: 14.3,
                y: 12.5,
                rotation_radians: 0.0,
            },
        ],
        catalog,
    )
    .expect("test composite should be physically realizable");
    let carbon = PhysicalMaterial::realized(
        Material::free_base("Carbon", 1.0),
        vec![Placement {
            x: 12.5,
            y: 12.5,
            rotation_radians: 0.0,
        }],
        catalog,
    )
    .expect("carbon should be physically realizable");
    let hydrogen = PhysicalMaterial::realized(
        Material::free_base("Hydrogen", 1.0),
        vec![Placement {
            x: 14.3,
            y: 12.5,
            rotation_radians: 0.0,
        }],
        catalog,
    )
    .expect("hydrogen should be physically realizable");
    let mut field = ActiveMaterialField::new(100.0, 100.0, 25.0);
    let cell = field.index_for_position(12.5, 12.5).unwrap();
    assert!(field.deposit_physical_at_index(cell, source.clone(), catalog));
    let before_revision = field.revision;

    // A late out-of-bounds piece rejects the entire operation before source
    // removal; then a fully valid split replaces the source in one commit.
    let invalid_hydrogen = PhysicalMaterial::realized(
        Material::free_base("Hydrogen", 1.0),
        vec![Placement {
            x: 100.0,
            y: 12.5,
            rotation_radians: 0.0,
        }],
        catalog,
    )
    .expect("hydrogen geometry itself should be valid");
    assert!(!field.split_physical_at_index(
        cell,
        0,
        &source,
        vec![carbon.clone(), invalid_hydrogen],
        catalog,
    ));
    assert_eq!(field.cells[cell].physical_materials, vec![source.clone()]);
    assert_eq!(field.revision, before_revision);

    assert!(field.split_physical_at_index(cell, 0, &source, vec![carbon, hydrogen], catalog,));
    assert_eq!(
        field
            .cells
            .iter()
            .map(|cell| cell.physical_materials.len())
            .sum::<usize>(),
        2
    );
    assert!(field.revision > before_revision);
}

#[test]
fn bounded_bounds_query_limits_allocation_and_rotates_across_large_ranges() {
    let field = ActiveMaterialField::new(100.0, 100.0, 10.0);
    let mut visited = std::collections::BTreeSet::new();

    for tick_window in 0..15 {
        let cells = field.cells_intersecting_bounds_limited(
            0.0,
            99.0,
            -100_000.0,
            100_000.0,
            7,
            tick_window * 7,
        );
        assert!(cells.len() <= 7, "each bounded query must respect its cap");
        visited.extend(cells);
    }

    assert_eq!(
        visited.len(),
        100,
        "rotating bounded queries must eventually visit every cell in the range"
    );
}

#[test]
fn bounded_bounds_query_matches_full_query_for_a_complete_window() {
    let field = ActiveMaterialField::new(100.0, 100.0, 10.0);
    let full = field.cells_intersecting_bounds(10.0, 40.0, 80.0, 120.0);
    let bounded = field.cells_intersecting_bounds_limited(10.0, 40.0, 80.0, 120.0, full.len(), 0);

    assert_eq!(bounded, full);
}
