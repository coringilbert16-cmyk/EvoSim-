#[cfg(test)]
mod tests {
    use crate::resources::Material;
    use crate::state::{EnergyLedger, Simulation};
    use crate::structure::{Bond, Placement, StructuralUnit};

    fn add_test_break_bond(sim: &mut Simulation) {
        let a = sim.organisms[0].structure.add_unit(StructuralUnit::new(
            "Carbon",
            Placement { x: 500.0, y: 500.0, rotation_radians: 0.0 },
        ));
        let b = sim.organisms[0].structure.add_unit(StructuralUnit::new(
            "Methane",
            Placement { x: 501.0, y: 500.0, rotation_radians: 0.0 },
        ));
        sim.organisms[0].structure.add_bond(Bond {
            unit_a: a, point_a: 0, unit_b: b, point_b: 0, strength: 0.8, bond_energy: 12.5,
        });
    }

    #[test]
    fn diagnose_break_inputs_before_resolution() {
        let mut sim = Simulation::new(7, 10.0);
        add_test_break_bond(&mut sim);

        sim.step();
        let transformation = sim.active_transformations[0].clone();
        assert_eq!(transformation.remaining_ticks, 2);

        sim.step();
        assert_eq!(transformation.remaining_ticks, 2);
        assert_eq!(sim.active_transformations[0].remaining_ticks, 1);

        let target = transformation.bond.expect("break transformation should retain target bond");
        let current = sim.organisms[0].structure.bonds[0];
        assert!(current.has_same_identity(&target), "target={target:?} current={current:?}");
        assert_eq!(sim.organisms[0].id, transformation.organism_id);

        let mut ledger = EnergyLedger::default();
        Simulation::resolve_transformation(
            &transformation,
            &mut sim.organisms[0],
            &mut sim.environment,
            &mut ledger,
        );
        assert!(sim.organisms[0].structure.bonds.is_empty(), "target={target:?} current_after={:?}", sim.organisms[0].structure.bonds);
        assert!((sim.organisms[0].usable_energy - 12.5).abs() < 1e-12);
        let _ = Material { parts: Vec::new(), bonded: true };
    }
}
