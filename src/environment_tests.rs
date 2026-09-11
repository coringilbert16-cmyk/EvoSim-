use super::vents::{apply_vents, valid_vent_materials, Vent};
use super::*;
use crate::resources::{InternalBond, Material};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

fn make_raw(name: &str, amount: f64) -> Material {
    Material::free_base(name, amount)
}

fn make_structured(amount: f64) -> Material {
    Material {
        parts: vec![("Carbon".into(), amount / 2.0), ("Hydrogen".into(), amount / 2.0)],
        internal_bonds: vec![InternalBond { part_a: 0, part_b: 1 }],
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
    assert_eq!(cell.materials.iter().filter(|m| m.has_internal_structure()).count(), 2);
    assert!((cell.total_amount() - 30.0).abs() < 1e-9);
    assert_eq!(cell.materials.iter().filter(|m| !m.has_internal_structure()).count(), 1);
}

#[test]
fn take_removes_up_to_available_amount_from_selected_material() {
    let mut field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
    field.deposit(50.0, 50.0, make_raw("Carbon", 4.0));
    let taken = field.take_at(50.0, 50.0, 0, 100.0).unwrap();
    assert!((taken.total_amount() - 4.0).abs() < 1e-9);
    assert!(field.cells[field.index_for_position(50.0, 50.0).unwrap()].materials.is_empty());
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

#[test]
fn diffusion_zero_fraction_is_a_noop() {
    let mut field = ActiveMaterialField::new(200.0, 200.0, 25.0);
    field.deposit(100.0, 100.0, make_raw("Methane", 10.0));
    field.diffuse_step(0.0);
    let index = field.index_for_position(100.0, 100.0).unwrap();
    assert!((field.cells[index].total_amount() - 10.0).abs() < 1e-9);
}

#[test]
fn diffusion_spreads_material_to_all_four_neighbors_from_interior_cell() {
    let mut field = ActiveMaterialField::new(200.0, 200.0, 25.0);
    field.deposit(100.0, 100.0, make_raw("Methane", 100.0));
    let center = field.index_for_position(100.0, 100.0).unwrap();
    assert_eq!(field.neighbor_indices(center).len(), 4);
    field.diffuse_step(0.2);
    for &n in &field.neighbor_indices(center) {
        assert!(field.cells[n].total_amount() > 0.0);
    }
}

#[test]
fn diffusion_preserves_structured_material_and_total_amount() {
    let mut field = ActiveMaterialField::new(500.0, 500.0, 25.0);
    field.deposit(250.0, 250.0, make_raw("Methane", 500.0));
    field.deposit(50.0, 450.0, make_raw("Carbon", 300.0));
    field.deposit(0.0, 0.0, make_structured(50.0));
    let before = field.total_amount();
    for _ in 0..200 {
        field.diffuse_step(DEFAULT_DIFFUSION_FRACTION);
    }
    assert!((before - field.total_amount()).abs() < 1e-6);
    assert!(field.cells.iter().any(|cell| cell.materials.iter().any(|m| m.has_internal_structure())));
}

#[test]
fn repeated_diffusion_spreads_material_across_the_field() {
    let mut field = ActiveMaterialField::new(200.0, 200.0, 25.0);
    field.deposit(0.0, 0.0, make_raw("Water", 640.0));
    for _ in 0..500 {
        field.diffuse_step(0.1);
    }
    assert!(field.cells.iter().filter(|c| c.total_amount() > 1e-6).count() > 1);
}

#[test]
fn vent_material_pool_contains_raw_and_structured_valid_materials() {
    let catalog = crate::resources::default_catalog();
    let materials = valid_vent_materials(&catalog);
    assert!(!materials.is_empty());
    assert!(materials.iter().all(Material::is_valid));
    assert!(materials.iter().any(|m| !m.has_internal_structure()));
    assert!(materials.iter().any(Material::has_internal_structure));
}

#[test]
fn vents_emit_directly_into_the_active_field_without_a_reservoir() {
    let mut field = ActiveMaterialField::new(200.0, 200.0, 25.0);
    let catalog = crate::resources::default_catalog();
    let mut vents = vec![Vent { x: 100.0, y: 100.0, emission_amount: 40.0, emission_interval: 0, emission_timer: 0 }];
    let mut rng = ChaCha8Rng::seed_from_u64(7);
    apply_vents(&mut field, &catalog, &mut vents, &mut rng);
    let index = field.index_for_position(100.0, 100.0).unwrap();
    assert!(field.cells[index].total_amount() > 0.0);
}

#[test]
fn vent_quantity_fluctuates_around_its_configured_average() {
    let mut field = ActiveMaterialField::new(200.0, 200.0, 25.0);
    let catalog = crate::resources::default_catalog();
    let mut vents = vec![Vent { x: 100.0, y: 100.0, emission_amount: 40.0, emission_interval: 0, emission_timer: 0 }];
    let mut rng = ChaCha8Rng::seed_from_u64(11);
    let mut amounts = Vec::new();
    for _ in 0..8 {
        let before = field.total_amount();
        apply_vents(&mut field, &catalog, &mut vents, &mut rng);
        amounts.push(field.total_amount() - before);
    }
    assert!(amounts.iter().all(|amount| *amount > 0.0));
    assert!(amounts.iter().any(|amount| (*amount - 40.0).abs() > 1e-9));
}

#[test]
fn vents_can_emit_both_atomic_and_compound_material_over_time() {
    let mut field = ActiveMaterialField::new(200.0, 200.0, 25.0);
    let catalog = crate::resources::default_catalog();
    let mut vents = vec![Vent { x: 100.0, y: 100.0, emission_amount: 40.0, emission_interval: 0, emission_timer: 0 }];
    let mut rng = ChaCha8Rng::seed_from_u64(1234);
    for _ in 0..200 {
        apply_vents(&mut field, &catalog, &mut vents, &mut rng);
    }
    let index = field.index_for_position(100.0, 100.0).unwrap();
    let materials = &field.cells[index].materials;
    assert!(materials.iter().any(|m| !m.has_internal_structure()));
    assert!(materials.iter().any(Material::has_internal_structure));
}
