use crate::energy_ledger::{EnergyLedgerAuthority, EnergyReason, EnergyTransaction};
use crate::material_geometry::PlacedMaterialPart;
use crate::state::{EnergyLedger, Environment, Organism, Simulation};
use crate::structure::Placement;

const DEFAULT_MOVEMENT_EFFICIENCY: f64 = 0.8;
const MOVEMENT_BASE_STEP_DISTANCE: f64 = 4.0;
const MOVEMENT_COST_ANCHORS: [(f64, f64); 4] =
    [(2.7, 1.2), (16.0, 2.0), (64.0, 5.8), (1024.0, 65.0)];

fn mass_movement_cost(realized_mass: f64) -> f64 {
    let mass = realized_mass.max(f64::EPSILON);
    let segment = MOVEMENT_COST_ANCHORS
        .windows(2)
        .find(|pair| mass <= pair[1].0)
        .unwrap_or(&MOVEMENT_COST_ANCHORS[2..4]);
    let (mass_a, cost_a) = segment[0];
    let (mass_b, cost_b) = segment[1];
    let exponent = (cost_b / cost_a).ln() / (mass_b / mass_a).ln();
    cost_a * (mass / mass_a).powf(exponent)
}

fn movement_energy_cost(realized_mass: f64, movement_efficiency: f64) -> f64 {
    let efficiency = movement_efficiency.clamp(0.05, 1.0);
    mass_movement_cost(realized_mass) * (DEFAULT_MOVEMENT_EFFICIENCY / efficiency)
}

impl Simulation {
    pub(crate) fn update_movement(
        organisms: &mut [Organism],
        moving_index: usize,
        environment: &mut Environment,
        ledger: &mut EnergyLedger,
        tick: u64,
        rng: &mut ChaCha8Rng,
    ) -> bool {
        let (before, rest) = organisms.split_at_mut(moving_index);
        let (organism, after) = rest
            .split_first_mut()
            .expect("movement index must reference an organism");

        let old_position = organism.occupied_cells.first().cloned();
        let usable_energy = organism.usable_energy;
        let active_transformation_id = organism.active_transformation_id;
        let movement_efficiency = organism.genome.movement_efficiency();
        let realized_mass = organism.structural_mass(&environment.catalog);
        let direction =
            crate::movement_direction::movement_direction_periodic(organism, environment.height);
        let Some((x, y)) = direction else {
            organism.last_movement_attempt = Some(crate::state::MovementAttemptDiagnostic {
                tick,
                direction_x: None,
                direction_y: None,
                step: None,
                usable_energy,
                active_transformation_id,
                result: Err(crate::state::MovementFailureReason::NoDirection),
                old_position,
                new_position: None,
            });
            return false;
        };

        let Some(step) = select_movement_distance(
            organism,
            realized_mass,
            movement_efficiency,
            organism.usable_energy,
            rng,
        ) else {
            organism.last_movement_attempt = Some(crate::state::MovementAttemptDiagnostic {
                tick,
                direction_x: Some(x),
                direction_y: Some(y),
                step: None,
                usable_energy,
                active_transformation_id,
                result: Err(crate::state::MovementFailureReason::InsufficientEnergy),
                old_position,
                new_position: None,
            });
            return false;
        };

        let requested_dx = x * step;
        let requested_dy = y * step;
        let old = organism
            .occupied_cells
            .first()
            .expect("movement direction requires an occupied cell");
        let actual_dx = (old.x + requested_dx).clamp(0.0, environment.width) - old.x;
        let actual_distance = actual_dx.hypot(requested_dy);
        let cost =
            movement_energy_cost_for_distance(realized_mass, movement_efficiency, actual_distance);
        if !cost.is_finite() || organism.usable_energy + f64::EPSILON < cost {
            organism.last_movement_attempt = Some(crate::state::MovementAttemptDiagnostic {
                tick,
                direction_x: Some(x),
                direction_y: Some(y),
                step: Some(step),
                usable_energy,
                active_transformation_id,
                result: Err(crate::state::MovementFailureReason::InsufficientEnergy),
                old_position,
                new_position: None,
            });
            return false;
        }

        let moving_destination =
            organism_parts_at(organism, environment, requested_dx, requested_dy);
        if movement_blocked_by_organisms(
            &moving_destination,
            before.iter(),
            after.iter(),
            environment,
        ) || movement_blocked_by_physical_material(&moving_destination, environment)
        {
            organism.last_movement_attempt = Some(crate::state::MovementAttemptDiagnostic {
                tick,
                direction_x: Some(x),
                direction_y: Some(y),
                step: Some(step),
                usable_energy,
                active_transformation_id,
                result: Err(crate::state::MovementFailureReason::BlockedByPushChain),
                old_position,
                new_position: None,
            });
            return false;
        }

        let result = move_organism(organism, environment, requested_dx, requested_dy, cost, ledger);
        let new_position = organism.occupied_cells.first().cloned();
        organism.last_movement_attempt = Some(crate::state::MovementAttemptDiagnostic {
            tick,
            direction_x: Some(x),
            direction_y: Some(y),
            step: Some(step),
            usable_energy,
            active_transformation_id,
            result: result.clone(),
            old_position,
            new_position,
        });
        result.is_ok()
    }

    fn move_organism(
        organism: &mut Organism,
        environment: &mut Environment,
        dx: f64,
        dy: f64,
        cost: f64,
        ledger: &mut EnergyLedger,
    ) -> Result<(), crate::state::MovementFailureReason> {
        if !dx.is_finite() || !dy.is_finite() {
            return Err(crate::state::MovementFailureReason::NonFiniteDisplacement);
        }
        let (old_x, old_y) = organism
            .occupied_cells
            .first()
            .map(|p| (p.x, p.y))
            .ok_or(crate::state::MovementFailureReason::NoOccupiedCell)?;
        let new_x = (old_x + dx).clamp(0.0, environment.width);
        let new_y = wrap_y(old_y + dy, environment.height);
        let actual_dx = new_x - old_x;
        let actual_dy = new_y - old_y;
        if actual_dx.abs() <= f64::EPSILON && actual_dy.abs() <= f64::EPSILON {
            return Err(crate::state::MovementFailureReason::ZeroDisplacement);
        }
        let transaction = EnergyTransaction {
            reason: EnergyReason::Move,
            potential_released: 0.0,
            usable_delta: -cost,
            structural_delta: 0.0,
            heat_dissipated: cost,
        };
        if !ledger.settle_transaction(&mut organism.usable_energy, transaction) {
            return Err(crate::state::MovementFailureReason::InsufficientEnergy);
        }
        organism.occupied_cells[0].x = new_x;
        organism.occupied_cells[0].y = new_y;
        organism.developmental_origin.x += actual_dx;
        organism.developmental_origin.y =
            wrap_y(organism.developmental_origin.y + actual_dy, environment.height);
        for unit in &mut organism.structure.units {
            unit.placement.x += actual_dx;
            unit.placement.y = wrap_y(unit.placement.y + actual_dy, environment.height);
        }
        translate_reproductive_construction(organism, actual_dx, actual_dy, environment.height);
        organism.mark_position_changed();
        Ok(())
    }

    fn movement_blocked_by_organisms<'a>(
        moving_destination: &[PlacedMaterialPart],
        before: impl Iterator<Item = &'a Organism>,
        after: impl Iterator<Item = &'a Organism>,
        environment: &Environment,
    ) -> bool {
        before.chain(after).any(|candidate| {
            let candidate_parts = organism_parts_at(candidate, environment, 0.0, 0.0);
            parts_penetrate(moving_destination, &candidate_parts, environment.height)
        })
    }

    fn movement_blocked_by_physical_material(
        moving_destination: &[PlacedMaterialPart],
        environment: &Environment,
    ) -> bool {
        let min_x = moving_destination
            .iter()
            .map(|part| part.placement.x - part.form.bounding_radius())
            .fold(f64::INFINITY, f64::min);
        let max_x = moving_destination
            .iter()
            .map(|part| part.placement.x + part.form.bounding_radius())
            .fold(f64::NEG_INFINITY, f64::max);
        let min_y = moving_destination
            .iter()
            .map(|part| part.placement.y - part.form.bounding_radius())
            .fold(f64::INFINITY, f64::min);
        let max_y = moving_destination
            .iter()
            .map(|part| part.placement.y + part.form.bounding_radius())
            .fold(f64::NEG_INFINITY, f64::max);

        let candidate_cells = environment
            .field
            .cells_intersecting_bounds(min_x, max_x, min_y, max_y);
        candidate_cells.iter().any(|&cell_index| {
            environment.field.cells[cell_index]
                .physical_materials
                .iter()
                .filter(|physical| physical.is_realized() && !physical.material.is_empty())
                .any(|physical| {
                    let candidate_parts = physical_parts_at(physical, environment, 0.0, 0.0);
                    parts_penetrate(moving_destination, &candidate_parts, environment.height)
                })
        })
    }


    pub(crate) fn try_move_cell(
        organism: &mut Organism,
        environment: &mut Environment,
        other_organisms: &mut [Organism],
        delta_x: f64,
        delta_y: f64,
    ) -> bool {
        if !delta_x.is_finite() || !delta_y.is_finite() {
            return false;
        }
        let Some((old_x, old_y)) = organism
            .occupied_cells
            .first()
            .map(|p| (p.x, p.y))
        else {
            return false;
        };
        let new_x = (old_x + delta_x).clamp(0.0, environment.width);
        let new_y = wrap_y(old_y + delta_y, environment.height);
        let actual_dx = new_x - old_x;
        let actual_dy = new_y - old_y;
        if actual_dx.abs() <= f64::EPSILON && actual_dy.abs() <= f64::EPSILON {
            return false;
        }

        let destination = organism_parts_at(organism, environment, delta_x, delta_y);
        if movement_blocked_by_organisms(
            &destination,
            other_organisms.iter(),
            std::iter::empty(),
            environment,
        ) || movement_blocked_by_physical_material(&destination, environment)
        {
            return false;
        }

        organism.occupied_cells[0].x = new_x;
        organism.occupied_cells[0].y = new_y;
        organism.developmental_origin.x += actual_dx;
        organism.developmental_origin.y =
            wrap_y(organism.developmental_origin.y + actual_dy, environment.height);
        for unit in &mut organism.structure.units {
            unit.placement.x += actual_dx;
            unit.placement.y = wrap_y(unit.placement.y + actual_dy, environment.height);
        }
        translate_reproductive_construction(organism, actual_dx, actual_dy, environment.height);
        organism.mark_position_changed();
        true
    }

