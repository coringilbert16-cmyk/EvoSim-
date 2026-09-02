use crate::state::{ActiveTransformation, EnergyLedger, TransformationKind};
use crate::structure::{Bond, Placement, StructuralUnit};
use crate::transformation::resolve_transformation;

fn prepare_bonded_pair(
    sim: &mut crate::state::Simulation,
    bond_energy: f64,
    strength: f64,
) -> Bond {
    let organism = &mut sim.organisms[0];
    organism.structure.units.clear();
    organism.structure.bonds.clear();
    organism.memory.clear();
    organism.decision_history = crate::decision::DecisionHistory::default();
    organism.active_transformation_id = Some(1);
    organism.usable_energy = 10.0;

    let a = organism.structure.add_unit(StructuralUnit::new(
        "Carbon",
        Placement {
            x: 500.0,
            y: 500.0,
            rotation_radians: 0.0,
        },
    ));
    let b = organism.structure.add_unit(StructuralUnit::new(
        "Carbon",
        Placement {
            x: 501.0,
            y: 500.0,
            rotation_radians: 0.0,
        },
    ));
    let bond = Bond {
        unit_a: a,
        point_a: 0,
        unit_b: b,
        point_b: 0,
        strength,
        bond_energy,
    };
    organism.structure.add_bond(bond);
    bond
}

fn transformation(bond: Bond) -> ActiveTransformation {
    ActiveTransformation {
        id: 1,
        organism_id: "1".into(),
        kind: TransformationKind::Break,
        material: crate::resources::Material {
            parts: Vec::new(),
            bonded: true,
        },
        bond: Some(bond),
        complexity: 2.0,
        duration_ticks: 2,
        remaining_ticks: 0,
        decision_context_key: Some("bond:0".into()),
    }
}

fn ledger_balance(ledger: &EnergyLedger) -> f64 {
    ledger.total_potential_energy_released + ledger.total_usable_energy_spent
        - ledger.total_formation_energy_spent
        - ledger.total_break_energy_spent
        - ledger.total_bond_energy_created
        - ledger.total_usable_energy_gained
        - ledger.total_heat_dissipated
}

#[test]
fn break_release_regime_preserves_energy_and_breaks_bond() {
    let mut sim = crate::state::Simulation::new(41, 10.0);
    let bond = prepare_bonded_pair(&mut sim, 4.0, 0.5);
    let transformation = transformation(bond);
    let energy_before = sim.organisms[0].usable_energy;

    let (organisms, environment, ledger) = (
        &mut sim.organisms,
        &mut sim.environment,
        &mut sim.energy_ledger,
    );
    resolve_transformation(&transformation, &mut organisms[0], environment, ledger);

    let organism = &organisms[0];
    assert!(organism.structure.bonds.is_empty());
    assert_eq!(organism.structure.units.len(), 2);
    assert!(organism.usable_energy > energy_before);
    assert!(ledger.total_potential_energy_released > 0.0);
    assert!(ledger.total_break_energy_spent > 0.0);
    assert!(ledger.total_usable_energy_gained > 0.0);
    assert!(ledger.total_heat_dissipated > 0.0);
    assert!(ledger_balance(ledger).abs() < 1e-9);
}

#[test]
fn break_consume_regime_spends_organism_energy_and_preserves_energy() {
    let mut sim = crate::state::Simulation::new(43, 10.0);
    let bond = prepare_bonded_pair(&mut sim, 0.1, 0.5);
    let transformation = transformation(bond);
    let energy_before = sim.organisms[0].usable_energy;

    let (organisms, environment, ledger) = (
        &mut sim.organisms,
        &mut sim.environment,
        &mut sim.energy_ledger,
    );
    resolve_transformation(&transformation, &mut organisms[0], environment, ledger);

    let organism = &organisms[0];
    assert!(organism.structure.bonds.is_empty());
    assert_eq!(organism.structure.units.len(), 2);
    assert!(organism.usable_energy < energy_before);
    assert_eq!(ledger.total_usable_energy_gained, 0.0);
    assert_eq!(ledger.total_heat_dissipated, 0.0);
    assert!(ledger.total_break_energy_spent > ledger.total_potential_energy_released);
    assert!(ledger_balance(ledger).abs() < 1e-9);
}

#[test]
fn break_neutral_regime_has_no_net_usable_energy_change() {
    let mut sim = crate::state::Simulation::new(47, 10.0);
    let organism = &mut sim.organisms[0];
    organism.structure.units.clear();
    organism.structure.bonds.clear();
    organism.usable_energy = 10.0;

    let a = organism.structure.add_unit(StructuralUnit::new(
        "Carbon",
        Placement {
            x: 500.0,
            y: 500.0,
            rotation_radians: 0.0,
        },
    ));
    let b = organism.structure.add_unit(StructuralUnit::new(
        "Carbon",
        Placement {
            x: 501.0,
            y: 500.0,
            rotation_radians: 0.0,
        },
    ));
    let strength = 0.5;
    let work = crate::structure::formation_threshold(0.95, 0.95, strength, strength) * strength;
    let bond = Bond {
        unit_a: a,
        point_a: 0,
        unit_b: b,
        point_b: 0,
        strength,
        bond_energy: work,
    };
    organism.structure.add_bond(bond);
    organism.active_transformation_id = Some(1);
    let transformation = transformation(bond);
    let energy_before = organism.usable_energy;

    let (organisms, environment, ledger) = (
        &mut sim.organisms,
        &mut sim.environment,
        &mut sim.energy_ledger,
    );
    resolve_transformation(&transformation, &mut organisms[0], environment, ledger);

    assert!(organisms[0].structure.bonds.is_empty());
    assert!((organisms[0].usable_energy - energy_before).abs() < 1e-12);
    assert_eq!(ledger.total_usable_energy_gained, 0.0);
    assert_eq!(ledger.total_usable_energy_spent, 0.0);
    assert_eq!(ledger.total_heat_dissipated, 0.0);
    assert!(ledger_balance(ledger).abs() < 1e-9);
}

#[test]
fn insufficient_break_energy_is_atomic() {
    let mut sim = crate::state::Simulation::new(53, 10.0);
    let bond = prepare_bonded_pair(&mut sim, 0.1, 0.5);
    let transformation = transformation(bond);
    sim.organisms[0].usable_energy = 0.0;
    let ledger_before = sim.energy_ledger;

    let (organisms, environment, ledger) = (
        &mut sim.organisms,
        &mut sim.environment,
        &mut sim.energy_ledger,
    );
    resolve_transformation(&transformation, &mut organisms[0], environment, ledger);

    let organism = &organisms[0];
    assert_eq!(organism.structure.bonds, vec![bond]);
    assert_eq!(organism.usable_energy, 0.0);
    assert_eq!(*ledger, ledger_before);
    assert!(organism.active_transformation_id.is_none());
}

#[test]
fn break_preserves_other_bonds_and_all_structural_units() {
    let mut sim = crate::state::Simulation::new(59, 10.0);
    let organism = &mut sim.organisms[0];
    organism.structure.units.clear();
    organism.structure.bonds.clear();
    organism.usable_energy = 10.0;

    for x in 0..3 {
        organism.structure.add_unit(StructuralUnit::new(
            "Carbon",
            Placement {
                x: 500.0 + x as f64,
                y: 500.0,
                rotation_radians: 0.0,
            },
        ));
    }
    let first = Bond {
        unit_a: 0,
        point_a: 0,
        unit_b: 1,
        point_b: 0,
        strength: 0.5,
        bond_energy: 4.0,
    };
    let second = Bond {
        unit_a: 1,
        point_a: 1,
        unit_b: 2,
        point_b: 0,
        strength: 0.5,
        bond_energy: 4.0,
    };
    organism.structure.add_bond(first);
    organism.structure.add_bond(second);
    organism.active_transformation_id = Some(1);

    let transformation = transformation(first);
    let (organisms, environment, ledger) = (
        &mut sim.organisms,
        &mut sim.environment,
        &mut sim.energy_ledger,
    );
    resolve_transformation(&transformation, &mut organisms[0], environment, ledger);

    assert_eq!(organisms[0].structure.units.len(), 3);
    assert_eq!(organisms[0].structure.bonds, vec![second]);
    assert_eq!(
        organisms[0].structure.connected_components(),
        vec![vec![0], vec![1, 2]]
    );
}

#[test]
fn break_ledger_accumulates_multiple_events() {
    let mut ledger = EnergyLedger::default();
    ledger.record_break(4.0, 0.0, 1.0, 1.2, 1.8);
    ledger.record_break(0.5, 1.5, 2.0, 0.0, 0.0);

    assert_eq!(ledger.total_potential_energy_released, 4.5);
    assert_eq!(ledger.total_break_energy_spent, 3.0);
    assert_eq!(ledger.total_usable_energy_spent, 1.5);
    assert_eq!(ledger.total_usable_energy_gained, 1.2);
    assert_eq!(ledger.total_heat_dissipated, 1.8);
    assert!(ledger_balance(&ledger).abs() < 1e-12);
}

#[test]
fn break_with_invalid_bond_state_does_not_mutate_structure() {
    let mut sim = crate::state::Simulation::new(61, 10.0);
    let mut bond = prepare_bonded_pair(&mut sim, 1.0, 0.5);
    bond.bond_energy = f64::NAN;
    sim.organisms[0].structure.bonds[0] = bond;
    let transformation = transformation(bond);

    let (organisms, environment, ledger) = (
        &mut sim.organisms,
        &mut sim.environment,
        &mut sim.energy_ledger,
    );
    resolve_transformation(&transformation, &mut organisms[0], environment, ledger);

    assert_eq!(organisms[0].structure.bonds, vec![bond]);
    assert_eq!(*ledger, EnergyLedger::default());
}
