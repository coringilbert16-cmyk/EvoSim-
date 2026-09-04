use super::settling::{apply_settling, DEFAULT_SETTLING_FRACTION, DEFAULT_SETTLING_INTERVAL_TICKS};
use super::vents::{apply_vents, Vent};
use super::*;
use crate::resources::Material;

fn make_bonded(name: &str, amount: f64) -> Material {
    Material {
        parts: vec![(name.to_string(), amount)],
        bonded: true,
    }
}
fn make_unbonded(name: &str, amount: f64) -> Material {
    Material {
        parts: vec![(name.to_string(), amount)],
        bonded: false,
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
fn deposit_and_query_bonded_and_unbonded_independently() {
    let mut field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
    field.deposit(500.0, 500.0, make_bonded("Methane", 10.0));
    field.deposit(500.0, 500.0, make_unbonded("Carbon", 3.0));
    let cell = &field.cells[field.index_for_position(500.0, 500.0).unwrap()];
    assert!((cell.bonded.total_amount() - 10.0).abs() < 1e-9);
    assert!((cell.unbonded.total_amount() - 3.0).abs() < 1e-9);
}
#[test]
fn deposit_merges_same_resource_into_existing_stack() {
    let mut field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
    field.deposit(10.0, 10.0, make_bonded("Methane", 5.0));
    field.deposit(10.0, 10.0, make_bonded("Methane", 7.0));
    field.deposit(10.0, 10.0, make_bonded("Hydrogen", 2.0));
    let cell = &field.cells[field.index_for_position(10.0, 10.0).unwrap()];
    assert_eq!(cell.bonded.parts.len(), 2);
    assert!((cell.bonded.total_amount() - 14.0).abs() < 1e-9);
}
#[test]
fn take_removes_up_to_available_amount_and_no_more() {
    let mut field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
    field.deposit(50.0, 50.0, make_bonded("Carbon", 4.0));
    let taken = field.take_at(50.0, 50.0, true, 100.0).unwrap();
    assert!((taken.total_amount() - 4.0).abs() < 1e-9);
    assert!(
        field.cells[field.index_for_position(50.0, 50.0).unwrap()]
            .bonded
            .total_amount()
            < 1e-9
    );
}
#[test]
fn take_from_wrong_stack_does_not_touch_the_other() {
    let mut field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
    field.deposit(50.0, 50.0, make_bonded("Carbon", 4.0));
    field.deposit(50.0, 50.0, make_unbonded("Carbon", 9.0));
    let taken = field.take_at(50.0, 50.0, true, 4.0).unwrap();
    assert!((taken.total_amount() - 4.0).abs() < 1e-9);
    assert!(
        (field.cells[field.index_for_position(50.0, 50.0).unwrap()]
            .unbonded
            .total_amount()
            - 9.0)
            .abs()
            < 1e-9
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
fn diffusion_zero_fraction_is_a_noop() {
    let mut field = ActiveMaterialField::new(200.0, 200.0, 25.0);
    field.deposit(100.0, 100.0, make_bonded("Methane", 10.0));
    field.diffuse_step(0.0);
    assert!(
        (field.cells[field.index_for_position(100.0, 100.0).unwrap()]
            .bonded
            .total_amount()
            - 10.0)
            .abs()
            < 1e-9
    );
}
#[test]
fn diffusion_spreads_material_to_all_four_neighbors_from_interior_cell() {
    let mut field = ActiveMaterialField::new(200.0, 200.0, 25.0);
    field.deposit(100.0, 100.0, make_bonded("Methane", 100.0));
    let center = field.index_for_position(100.0, 100.0).unwrap();
    assert_eq!(field.neighbor_indices(center).len(), 4);
    field.diffuse_step(0.2);
    for &n in &field.neighbor_indices(center) {
        assert!(field.cells[n].bonded.total_amount() > 0.0);
    }
}
#[test]
fn diffusion_conserves_total_mass_over_many_steps() {
    let mut field = ActiveMaterialField::new(500.0, 500.0, 25.0);
    field.deposit(250.0, 250.0, make_bonded("Methane", 500.0));
    field.deposit(50.0, 450.0, make_unbonded("Carbon", 300.0));
    field.deposit(0.0, 0.0, make_bonded("Hydrogen", 50.0));
    let before = field.total_amount();
    for _ in 0..200 {
        field.diffuse_step(DEFAULT_DIFFUSION_FRACTION);
    }
    assert!((before - field.total_amount()).abs() < 1e-6);
}
#[test]
fn diffusion_conserves_mass_per_resource_type_not_just_total() {
    let mut field = ActiveMaterialField::new(300.0, 300.0, 25.0);
    field.deposit(150.0, 150.0, make_bonded("Methane", 200.0));
    field.deposit(150.0, 150.0, make_bonded("Carbon", 80.0));
    for _ in 0..50 {
        field.diffuse_step(0.1);
    }
    let totals = field.total_material();
    assert!((totals.iter().find(|(n, _)| n == "Methane").unwrap().1 - 200.0).abs() < 1e-6);
    assert!((totals.iter().find(|(n, _)| n == "Carbon").unwrap().1 - 80.0).abs() < 1e-6);
}
#[test]
fn corner_cell_diffusion_conserves_mass_with_only_two_neighbors() {
    let mut field = ActiveMaterialField::new(200.0, 200.0, 25.0);
    field.deposit(0.0, 0.0, make_bonded("Sulfur", 40.0));
    let corner = field.index_for_position(0.0, 0.0).unwrap();
    assert_eq!(field.neighbor_indices(corner).len(), 2);
    let before = field.total_amount();
    for _ in 0..30 {
        field.diffuse_step(0.15);
    }
    assert!((before - field.total_amount()).abs() < 1e-6);
}
#[test]
fn repeated_diffusion_eventually_spreads_material_across_the_field() {
    let mut field = ActiveMaterialField::new(200.0, 200.0, 25.0);
    field.deposit(0.0, 0.0, make_bonded("Water", 640.0));
    for _ in 0..500 {
        field.diffuse_step(0.1);
    }
    assert!(
        field
            .cells
            .iter()
            .filter(|c| c.bonded.total_amount() > 1e-6)
            .count()
            > 1
    );
}

fn field_and_reservoir() -> (ActiveMaterialField, DeepReservoir) {
    let field = ActiveMaterialField::new(1000.0, 1000.0, DEFAULT_CELL_SIZE);
    let reservoir = DeepReservoir::new_matching_field(&field, DEFAULT_RESERVOIR_BLOCK_SIZE);
    (field, reservoir)
}
#[test]
fn reservoir_grid_is_coarser_than_field_and_spatially_aligned() {
    let (field, reservoir) = field_and_reservoir();
    assert_eq!(reservoir.width_cells, 8);
    assert_eq!(reservoir.height_cells, 8);
    assert!(reservoir.cells.len() < field.cells.len());
}
#[test]
fn seeding_distributes_total_evenly_and_conserves_it() {
    let (_, mut reservoir) = field_and_reservoir();
    reservoir.seed_uniform("Carbon", 6400.0);
    assert!((reservoir.total_amount() - 6400.0).abs() < 1e-6);
    assert!((reservoir.cells[0].amount_of("Carbon") - 100.0).abs() < 1e-9);
}
#[test]
fn vent_draws_only_from_its_own_region_not_a_global_pool() {
    let (mut field, mut reservoir) = field_and_reservoir();
    let region_a_amount = 50.0;
    let region_b_amount = 999.0;
    let idx_a = reservoir
        .reservoir_index_for_field_index(&field, field.index_for_position(10.0, 10.0).unwrap());
    let idx_b = reservoir
        .reservoir_index_for_field_index(&field, field.index_for_position(900.0, 900.0).unwrap());
    reservoir.cells[idx_a].add("Methane", region_a_amount);
    reservoir.cells[idx_b].add("Methane", region_b_amount);
    let mut vents = vec![
        Vent {
            x: 10.0,
            y: 10.0,
            composition: vec![("Methane".into(), 1.0)],
            emission_amount: 200.0,
            emission_interval: 0,
            emission_timer: 0,
        },
        Vent {
            x: 900.0,
            y: 900.0,
            composition: vec![("Methane".into(), 1.0)],
            emission_amount: 10.0,
            emission_interval: 0,
            emission_timer: 0,
        },
    ];
    apply_vents(&mut field, &mut reservoir, &mut vents);
    assert!(reservoir.cells[idx_a].amount_of("Methane") < 1e-9);
    let field_idx_a = field.index_for_position(10.0, 10.0).unwrap();
    assert!((field.cells[field_idx_a].unbonded.total_amount() - region_a_amount).abs() < 1e-9);
    assert_eq!(field.cells[field_idx_a].bonded.total_amount(), 0.0);
    assert!(
        (reservoir.cells[idx_b].amount_of("Methane") - (region_b_amount - 10.0)).abs()
            < 1e-9
    );
}
#[test]
fn vent_draw_is_indiscriminate_across_unified_reservoir_stock() {
    let (mut field, mut reservoir) = field_and_reservoir();
    let field_index = field.index_for_position(500.0, 500.0).unwrap();
    let reservoir_index = reservoir.reservoir_index_for_field_index(&field, field_index);
    reservoir.cells[reservoir_index].add("Carbon", 20.0);
    reservoir.cells[reservoir_index].add("Carbon", 80.0);
    assert!((reservoir.cells[reservoir_index].amount_of("Carbon") - 100.0).abs() < 1e-9);
    let mut vents = vec![Vent {
        x: 500.0,
        y: 500.0,
        composition: vec![("Carbon".into(), 1.0)],
        emission_amount: 50.0,
        emission_interval: 0,
        emission_timer: 0,
    }];
    apply_vents(&mut field, &mut reservoir, &mut vents);
    assert!(field.cells[field_index].bonded.total_amount() < 1e-9);
    assert!((field.cells[field_index].unbonded.total_amount() - 50.0).abs() < 1e-9);
    assert!((reservoir.cells[reservoir_index].amount_of("Carbon") - 50.0).abs() < 1e-9);
}
#[test]
fn vent_releases_unified_reservoir_material_as_unbonded_active_material() {
    let (mut field, mut reservoir) = field_and_reservoir();
    let field_index = field.index_for_position(500.0, 500.0).unwrap();
    let reservoir_index = reservoir.reservoir_index_for_field_index(&field, field_index);
    reservoir.cells[reservoir_index].add("Carbon", 100.0);
    let mut vents = vec![Vent {
        x: 500.0,
        y: 500.0,
        composition: vec![("Carbon".into(), 1.0)],
        emission_amount: 30.0,
        emission_interval: 0,
        emission_timer: 0,
    }];
    apply_vents(&mut field, &mut reservoir, &mut vents);
    assert!((field.cells[field_index].unbonded.total_amount() - 30.0).abs() < 1e-9);
    assert!(field.cells[field_index].bonded.total_amount() < 1e-9);
    assert!((reservoir.cells[reservoir_index].amount_of("Carbon") - 70.0).abs() < 1e-9);
}
#[test]
fn vent_releases_any_unified_reservoir_stock_as_unbonded_active_material() {
    let (mut field, mut reservoir) = field_and_reservoir();
    let field_index = field.index_for_position(500.0, 500.0).unwrap();
    let reservoir_index = reservoir.reservoir_index_for_field_index(&field, field_index);
    reservoir.cells[reservoir_index].add("Methane", 50.0);
    let mut vents = vec![Vent {
        x: 500.0,
        y: 500.0,
        composition: vec![("Methane".into(), 1.0)],
        emission_amount: 20.0,
        emission_interval: 0,
        emission_timer: 0,
    }];
    apply_vents(&mut field, &mut reservoir, &mut vents);
    assert!((field.cells[field_index].unbonded.total_amount() - 20.0).abs() < 1e-9);
    assert!(field.cells[field_index].bonded.total_amount() < 1e-9);
    assert!((reservoir.cells[reservoir_index].amount_of("Methane") - 30.0).abs() < 1e-9);
}
#[test]
fn venting_conserves_total_material_reservoir_plus_field() {
    let (mut field, mut reservoir) = field_and_reservoir();
    reservoir.seed_uniform("Carbon", 5000.0);
    let mut vents = vec![Vent {
        x: 250.0,
        y: 250.0,
        composition: vec![("Carbon".into(), 1.0)],
        emission_amount: 30.0,
        emission_interval: 2,
        emission_timer: 0,
    }];
    let before = reservoir.total_amount() + field.total_amount();
    for _ in 0..50 {
        apply_vents(&mut field, &mut reservoir, &mut vents);
    }
    assert!((before - (reservoir.total_amount() + field.total_amount())).abs() < 1e-6);
}
#[test]
fn settling_drains_both_bonded_and_unbonded_stacks() {
    let (mut field, mut reservoir) = field_and_reservoir();
    let idx = field.index_for_position(500.0, 500.0).unwrap();
    field.deposit_at_index(
        idx,
        Material {
            parts: vec![("Carbon".into(), 100.0)],
            bonded: true,
        },
    );
    field.deposit_at_index(
        idx,
        Material {
            parts: vec![("Carbon".into(), 40.0)],
            bonded: false,
        },
    );
    for _ in 0..20 {
        apply_settling(&mut field, &mut reservoir, DEFAULT_SETTLING_FRACTION);
    }
    assert!(field.cells[idx].bonded.total_amount() < 100.0);
    assert!(field.cells[idx].unbonded.total_amount() < 40.0);
    assert!(reservoir.total_amount() > 0.0);
}
#[test]
fn settling_merges_bonded_and_unbonded_material_into_unified_reservoir() {
    let (mut field, mut reservoir) = field_and_reservoir();
    let field_index = field.index_for_position(500.0, 500.0).unwrap();
    let reservoir_index = reservoir.reservoir_index_for_field_index(&field, field_index);
    field.deposit_at_index(
        field_index,
        Material {
            parts: vec![("Sulfur".into(), 200.0)],
            bonded: true,
        },
    );
    field.deposit_at_index(
        field_index,
        Material {
            parts: vec![("Sulfur".into(), 100.0)],
            bonded: false,
        },
    );
    for _ in 0..50 {
        apply_settling(&mut field, &mut reservoir, DEFAULT_SETTLING_FRACTION);
    }
    assert!(reservoir.cells[reservoir_index].amount_of("Sulfur") > 0.0);
    assert!(field.cells[field_index].bonded.total_amount() < 200.0);
    assert!(field.cells[field_index].unbonded.total_amount() < 100.0);
}
#[test]
fn settled_material_can_be_re_released_by_a_vent_as_unbonded_active_material() {
    let (mut field, mut reservoir) = field_and_reservoir();
    let field_index = field.index_for_position(500.0, 500.0).unwrap();
    let reservoir_index = reservoir.reservoir_index_for_field_index(&field, field_index);
    field.deposit_at_index(
        field_index,
        Material {
            parts: vec![("Nitrogen".into(), 500.0)],
            bonded: true,
        },
    );
    for _ in 0..500 {
        apply_settling(&mut field, &mut reservoir, 0.05);
    }
    assert!(field.cells[field_index].bonded.total_amount() < 1.0);
    let reservoir_before = reservoir.cells[reservoir_index].amount_of("Nitrogen");
    assert!(reservoir_before > 400.0);
    let mut vents = vec![Vent {
        x: 500.0,
        y: 500.0,
        composition: vec![("Nitrogen".into(), 1.0)],
        emission_amount: 50.0,
        emission_interval: 0,
        emission_timer: 0,
    }];
    apply_vents(&mut field, &mut reservoir, &mut vents);
    assert!((field.cells[field_index].unbonded.total_amount() - 50.0).abs() < 1e-6);
    assert!(field.cells[field_index].bonded.total_amount() < 1e-9);
    assert!(
        (reservoir.cells[reservoir_index].amount_of("Nitrogen")
            - (reservoir_before - 50.0))
            .abs()
            < 1e-6
    );
}
#[test]
fn settling_conserves_total_material_field_plus_reservoir() {
    let (mut field, mut reservoir) = field_and_reservoir();
    let idx = field.index_for_position(500.0, 500.0).unwrap();
    field.deposit_at_index(
        idx,
        Material {
            parts: vec![("Water".into(), 300.0)],
            bonded: false,
        },
    );
    field.deposit_at_index(
        idx,
        Material {
            parts: vec![("Hydrogen".into(), 150.0)],
            bonded: true,
        },
    );
    let before = field.total_amount() + reservoir.total_amount();
    for _ in 0..100 {
        apply_settling(&mut field, &mut reservoir, DEFAULT_SETTLING_FRACTION);
    }
    assert!((before - (field.total_amount() + reservoir.total_amount())).abs() < 1e-6);
}
#[test]
fn full_environment_loop_conserves_material_over_many_ticks() {
    let (mut field, mut reservoir) = field_and_reservoir();
    reservoir.seed_uniform("Carbon", 20_000.0);
    reservoir.seed_uniform("Methane", 10_000.0);
    reservoir.seed_uniform("Water", 15_000.0);
    let mut vents = vec![
        Vent {
            x: 250.0,
            y: 250.0,
            composition: vec![("Carbon".into(), 0.5), ("Methane".into(), 0.5)],
            emission_amount: 40.0,
            emission_interval: 5,
            emission_timer: 0,
        },
        Vent {
            x: 750.0,
            y: 750.0,
            composition: vec![("Water".into(), 1.0)],
            emission_amount: 20.0,
            emission_interval: 8,
            emission_timer: 0,
        },
    ];
    let before = field.total_amount() + reservoir.total_amount();
    for tick in 0..2000u64 {
        apply_vents(&mut field, &mut reservoir, &mut vents);
        field.diffuse_step(DEFAULT_DIFFUSION_FRACTION);
        if tick % DEFAULT_SETTLING_INTERVAL_TICKS == 0 {
            apply_settling(&mut field, &mut reservoir, DEFAULT_SETTLING_FRACTION);
        }
    }
    assert!((before - (field.total_amount() + reservoir.total_amount())).abs() < 1e-4);
}
