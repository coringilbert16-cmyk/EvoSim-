use super::*;
use crate::resources::{InternalBond, Material};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

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
fn out_of_bounds_position_is_none() {
    let field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
    assert!(field.index_for_position(-1.0, 5.0).is_none());
    assert!(field.index_for_position(5.0, 1000.0).is_none());
    assert!(field.index_for_position(1000.0, 5.0).is_none());
    assert!(field.index_for_position(f64::NAN, 5.0).is_none());
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
fn take_removes_up_to_available_amount_from_selected_material() {
    let mut field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
    field.deposit(50.0, 50.0, make_raw("Carbon", 4.0));
    let taken = field.take_at(50.0, 50.0, 0, 100.0).unwrap();
    assert!((taken.total_amount() - 4.0).abs() < 1e-9);
    assert!(field.cells[field.index_for_position(50.0, 50.0).unwrap()]
        .materials
        .is_empty());
}

#[test]
fn take_from_one_material_does_not_touch_another() {
    let mut field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
    field.deposit(50.0, 50.0, make_raw("Carbon", 4.0));
    field.deposit(50.0, 50.0, make_structured(9.0));
    let taken = field.take_at(50.0, 50.0, 0, 4.0).unwrap();
    assert!((taken.total_amount() - 4.0).abs() < 1e-9);
    let cell = &field.cells[field.index_for_position(50.0, 50.0).unwrap()];
    assert_eq!(cell.materials.len(), 1);
    assert!((cell.materials[0].total_amount() - 9.0).abs() < 1e-9);
    assert!(cell.materials[0].has_internal_structure());
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
