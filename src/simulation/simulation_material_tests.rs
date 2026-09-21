use crate::state::Simulation;

pub(crate) fn total_material_in_system(simulation: &Simulation) -> f64 {
    let mut total = simulation.environment.field.total_amount();
    for transformation in &simulation.active_transformations {
        total += transformation.material.total_amount();
    }
    for organism in &simulation.organisms {
        total += organism.stored_material.total_amount();
        if let Some(construction) = &organism.reproductive_construction {
            total += construction.committed_material.total_amount();
            total += construction
                .developing_structure
                .units
                .iter()
                .map(|unit| unit.material.total_amount())
                .sum::<f64>();
        }
        total += organism
            .structure
            .units
            .iter()
            .map(|unit| unit.material.total_amount())
            .sum::<f64>();
    }
    for body in &simulation.decomposing_bodies {
        total += body
            .structure
            .units
            .iter()
            .map(|unit| unit.material.total_amount())
            .sum::<f64>();
    }
    total
}
