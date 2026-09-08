use crate::observation::{ObservationLevel, ObservationProjection, WorldObservation};
use crate::state::Simulation;

#[test]
fn world_observation_preserves_environmental_observation_contract() {
    let simulation = Simulation::new(1, 20.0);
    let observation = WorldObservation::from_simulation(&simulation);

    assert_eq!(observation.width, simulation.environment.width);
    assert_eq!(observation.height, simulation.environment.height);
    assert_eq!(observation.vents.len(), simulation.environment.vents.len());
    assert_eq!(
        observation.decomposing_bodies.len(),
        simulation.decomposing_bodies.len()
    );

    for observed in &observation.field {
        let cell = &simulation.environment.field.cells[observed.cell_index];
        let (x, y) = simulation
            .environment
            .field
            .cell_center(observed.cell_index);
        assert_eq!((observed.x, observed.y), (x, y));
        assert_eq!(observed.materials, cell.total_material());
    }
}

#[test]
fn world_projection_is_explicitly_world_level() {
    let simulation = Simulation::new(1, 20.0);
    let projection = ObservationProjection::world(WorldObservation::from_simulation(&simulation));

    assert_eq!(projection.context.level, ObservationLevel::World);
    assert!(projection.context.organism_ids.is_empty());
    assert!(projection.context.focused_organism_id.is_none());
}
