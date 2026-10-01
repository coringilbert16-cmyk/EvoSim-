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
    field.deposit(500.0, 500.0, make_structured(10.0));
    field.deposit(500.0, 500.0, make_structured(10.0));
    field.deposit(500.0, 500.0, make_raw("Carbon", 3.0));
    field.deposit(500.0, 500.0, make_raw("Carbon", 7.0));
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

    let first_index = first.index_for_position(first_position.0, first_position.1).unwrap();
    let second_index = second.index_for_position(second_position.0, second_position.1).unwrap();

    first.mark_changed_at_index(first_index);
    first.mark_changed_at_index(first_index);
    second.mark_changed_at_index(second_index);
    second.mark_changed_at_index(second_index);

    let first_key = first.local_revision_for_positions(
        [first_position, second_position].into_iter(),
    );
    let second_key = second.local_revision_for_positions(
        [first_position, second_position].into_iter(),
    );

    assert_ne!(first_key, second_key);
}
