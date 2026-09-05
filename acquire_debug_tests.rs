#[cfg(test)]
mod acquire_debug_tests {
    use crate::decision::{ActionKind, OutcomeKind};
    use crate::resources::Material;
    use crate::state::Simulation;

    #[test]
    fn diagnose_acquire_transfer_state() {
        let mut sim = Simulation::new(21, 10.0);
        let target = sim.environment.field.index_for_position(500.0, 500.0).unwrap();
        sim.environment
            .field
            .deposit_at_index(target, Material::free_base("Carbon", 10.0));
        sim.organisms[0]
            .decision_history
            .record(ActionKind::Move, None, OutcomeKind::Harmful);
        sim.step();
        eprintln!("stored={:?}", sim.organisms[0].stored_material.parts);
        eprintln!("history={:?}", sim.organisms[0].decision_history.entries);
        eprintln!("target={} center={:?}", target, sim.environment.field.cell_center(target));
        eprintln!("target_material={:?}", sim.environment.field.cells[target].materials);
    }

    #[test]
    fn diagnose_acquire_target_selection_state() {
        let mut sim = Simulation::new(24, 10.0);
        sim.organisms[0].occupied_cells[0].x = 500.0;
        sim.organisms[0].occupied_cells[0].y = 500.0;
        let target_a = sim.environment.field.index_for_position(500.0, 500.0).unwrap();
        let target_b = sim.environment.field.index_for_position(475.0, 500.0).unwrap();
        sim.environment
            .field
            .deposit_at_index(target_a, Material::free_base("Carbon", 5.0));
        sim.environment
            .field
            .deposit_at_index(target_b, Material::free_base("Hydrogen", 5.0));
        sim.organisms[0]
            .decision_history
            .record(ActionKind::Move, None, OutcomeKind::Harmful);
        sim.organisms[0].decision_history.record(
            ActionKind::Acquire,
            Some(format!("target:{target_a}")),
            OutcomeKind::Harmful,
        );
        sim.step();
        eprintln!("target_a={} center={:?}", target_a, sim.environment.field.cell_center(target_a));
        eprintln!("target_b={} center={:?}", target_b, sim.environment.field.cell_center(target_b));
        eprintln!("stored={:?}", sim.organisms[0].stored_material.parts);
        eprintln!("history={:?}", sim.organisms[0].decision_history.entries);
        eprintln!("a={:?}", sim.environment.field.cells[target_a].materials);
        eprintln!("b={:?}", sim.environment.field.cells[target_b].materials);
    }
}
