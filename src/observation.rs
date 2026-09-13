//! Thin browser-facing projection of authoritative simulation state.
//!
//! The microscope does not invent simulation state. Physical state is exposed
//! in the same coordinate system used by the simulation; numerical/logical
//! state is exposed as data for the diagnostic sidebar.
use serde::{Deserialize, Serialize};

use crate::resources::{Form, Material};
use crate::state::{ActiveTransformation, EnergyLedger, Organism, Simulation};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SeedObservation {
    pub(crate) tick: u64,
    pub(crate) running: bool,
    pub(crate) ticks_per_second: f64,
    pub(crate) world_width: f64,
    pub(crate) world_height: f64,
    pub(crate) field_cell_size: f64,
    pub(crate) seed: Organism,
    pub(crate) structure: SeedStructureObservation,
    pub(crate) field: Vec<FieldCellObservation>,
    pub(crate) vents: Vec<PointObservation>,
    pub(crate) decomposing_bodies: Vec<PointObservation>,
    pub(crate) active_transformations: Vec<ActiveTransformation>,
    pub(crate) energy_ledger: EnergyLedger,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SeedStructureObservation {
    pub(crate) units: Vec<SeedUnitObservation>,
    pub(crate) bonds: Vec<SeedBondObservation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SeedUnitObservation {
    pub(crate) unit_index: usize,
    pub(crate) material: Material,
    pub(crate) form: Option<Form>,
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) rotation_radians: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SeedBondObservation {
    pub(crate) unit_a: usize,
    pub(crate) unit_b: usize,
    pub(crate) strength: f64,
    pub(crate) bond_energy: f64,
    pub(crate) endpoint_a: Option<PointObservation>,
    pub(crate) endpoint_b: Option<PointObservation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct FieldCellObservation {
    pub(crate) cell_index: usize,
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) materials: Vec<Material>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub(crate) struct PointObservation {
    pub(crate) x: f64,
    pub(crate) y: f64,
}

impl SeedObservation {
    pub(crate) fn from_simulation(simulation: &Simulation) -> Option<Self> {
        let seed = simulation.organisms.first()?.clone();
        let seed_position = seed.occupied_cells.first().copied()?;
        let catalog = &simulation.environment.catalog;

        let units = seed
            .structure
            .units
            .iter()
            .enumerate()
            .map(|(unit_index, unit)| SeedUnitObservation {
                unit_index,
                material: unit.material.clone(),
                form: unit.shape(catalog).map(|shape| shape.form.clone()),
                x: unit.placement.x + seed_position.x,
                y: unit.placement.y + seed_position.y,
                rotation_radians: unit.placement.rotation_radians,
            })
            .collect();

        let bonds = seed
            .structure
            .bonds
            .iter()
            .filter_map(|bond| {
                let ia = seed.structure.unit_index(bond.endpoint_a.constituent_id)?;
                let ib = seed.structure.unit_index(bond.endpoint_b.constituent_id)?;
                let endpoint_a = seed
                    .structure
                    .units
                    .get(ia)
                    .filter(|unit| unit.geometry.is_some())
                    .and_then(|unit| bond.endpoint_a.location.world_point(unit, catalog))
                    .map(|p| PointObservation {
                        x: p.x + seed_position.x,
                        y: p.y + seed_position.y,
                    });
                let endpoint_b = seed
                    .structure
                    .units
                    .get(ib)
                    .filter(|unit| unit.geometry.is_some())
                    .and_then(|unit| bond.endpoint_b.location.world_point(unit, catalog))
                    .map(|p| PointObservation {
                        x: p.x + seed_position.x,
                        y: p.y + seed_position.y,
                    });
                Some(SeedBondObservation {
                    unit_a: ia,
                    unit_b: ib,
                    strength: bond.strength,
                    bond_energy: bond.bond_energy,
                    endpoint_a,
                    endpoint_b,
                })
            })
            .collect();

        let field = simulation
            .environment
            .field
            .cells
            .iter()
            .enumerate()
            .filter(|(_, cell)| !cell.materials.is_empty())
            .map(|(cell_index, cell)| {
                let (x, y) = simulation.environment.field.cell_center(cell_index);
                FieldCellObservation {
                    cell_index,
                    x,
                    y,
                    materials: cell.materials.clone(),
                }
            })
            .collect();

        Some(Self {
            tick: simulation.tick,
            running: simulation.running,
            ticks_per_second: simulation.ticks_per_second,
            world_width: simulation.environment.width,
            world_height: simulation.environment.height,
            field_cell_size: simulation.environment.field.cell_size,
            seed: seed.clone(),
            structure: SeedStructureObservation { units, bonds },
            field,
            vents: simulation
                .environment
                .vents
                .iter()
                .map(|vent| PointObservation { x: vent.x, y: vent.y })
                .collect(),
            decomposing_bodies: simulation
                .decomposing_bodies
                .iter()
                .map(|body| PointObservation {
                    x: body.position.x,
                    y: body.position.y,
                })
                .collect(),
            active_transformations: simulation
                .active_transformations
                .iter()
                .filter(|transformation| transformation.organism_id == seed.id)
                .cloned()
                .collect(),
            energy_ledger: simulation.energy_ledger,
        })
    }
}
