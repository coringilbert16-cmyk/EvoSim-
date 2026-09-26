use crate::energy_ledger::{EnergyLedgerAuthority, EnergyReason, EnergyTransaction};
use crate::material_geometry::PlacedMaterialPart;
use crate::state::{EnergyLedger, Environment, Organism, Simulation};
use crate::structure::Placement;

const DEFAULT_MOVEMENT_EFFICIENCY: f64 = 0.8;
const MOVEMENT_BASE_STEP_DISTANCE: f64 = 4.0;
const MOVEMENT_COST_ANCHORS: [(f64, f64); 4] = [
    (2.7, 1.2),
    (16.0, 2.0),
    (64.0, 5.8),
    (1024.0, 65.0),
];

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
        organism: &mut Organism,
        environment: &mut Environment,
        other_organisms: &mut [Organism],
        ledger: &mut EnergyLedger,
        tick: u64,
    ) -> bool {
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

        // Keep the established default displacement (5.0 * 0.8 = 4.0)
        // independent from energy efficiency. Efficiency now changes the
        // energy required for the same movement rather than changing distance.
        let step = MOVEMENT_BASE_STEP_DISTANCE;
        let cost = movement_energy_cost(realized_mass, movement_efficiency);
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

        let result = Self::try_move_cell_with_reason(
            organism,
            environment,
            other_organisms,
            x * step,
            y * step,
        );
        let diagnostic_result = result.clone();
        let new_position = organism.occupied_cells.first().cloned();
        if result.is_ok() {
            let transaction = EnergyTransaction {
                reason: EnergyReason::Move,
                potential_released: 0.0,
                usable_delta: -cost,
                structural_delta: 0.0,
                heat_dissipated: cost,
            };
            if !ledger.settle_transaction(&mut organism.usable_energy, transaction) {
                organism.last_movement_attempt = Some(crate::state::MovementAttemptDiagnostic {
                    tick,
                    direction_x: Some(x),
                    direction_y: Some(y),
                    step: Some(step),
                    usable_energy,
                    active_transformation_id,
                    result: Err(crate::state::MovementFailureReason::InsufficientEnergy),
                    old_position,
                    new_position,
                });
                return false;
            }
        }
        organism.last_movement_attempt = Some(crate::state::MovementAttemptDiagnostic {
            tick,
            direction_x: Some(x),
            direction_y: Some(y),
            step: Some(step),
            usable_energy,
            active_transformation_id,
            result: diagnostic_result,
            old_position,
            new_position,
        });
        result.is_ok()
    }

    pub(crate) fn try_move_cell(
        organism: &mut Organism,
        environment: &mut Environment,
        other_organisms: &mut [Organism],
        delta_x: f64,
        delta_y: f64,
    ) -> bool {
        Self::try_move_cell_with_reason(organism, environment, other_organisms, delta_x, delta_y)
            .is_ok()
    }

    fn try_move_cell_with_reason(
        organism: &mut Organism,
        environment: &mut Environment,
        other_organisms: &mut [Organism],
        delta_x: f64,
        delta_y: f64,
    ) -> Result<(), crate::state::MovementFailureReason> {
        if !delta_x.is_finite() || !delta_y.is_finite() {
            return Err(crate::state::MovementFailureReason::NonFiniteDisplacement);
        }
        let (old_x, old_y) = organism
            .occupied_cells
            .first()
            .map(|p| (p.x, p.y))
            .ok_or(crate::state::MovementFailureReason::NoOccupiedCell)?;
        let new_x = (old_x + delta_x).clamp(0.0, environment.width);
        let new_y = wrap_y(old_y + delta_y, environment.height);
        let dx = new_x - old_x;
        let dy = new_y - old_y;
        if dx.abs() <= f64::EPSILON && dy.abs() <= f64::EPSILON {
            return Err(crate::state::MovementFailureReason::ZeroDisplacement);
        }

        let push_plan = resolve_push_chain(organism, other_organisms, environment, dx, dy)
            .ok_or(crate::state::MovementFailureReason::BlockedByPushChain)?;

        apply_push_plan(other_organisms, environment, push_plan, dx, dy);

        organism.occupied_cells[0].x = new_x;
        organism.occupied_cells[0].y = new_y;
        organism.developmental_origin.x += dx;
        organism.developmental_origin.y =
            wrap_y(organism.developmental_origin.y + dy, environment.height);
        for unit in &mut organism.structure.units {
            unit.placement.x += dx;
            unit.placement.y = wrap_y(unit.placement.y + dy, environment.height);
        }
        translate_reproductive_construction(organism, dx, dy, environment.height);
        organism.mark_position_changed();
        Ok(())
    }
}

#[derive(Default)]
struct PushPlan {
    organisms: Vec<usize>,
    physical: Vec<(usize, usize)>,
}

fn resolve_push_chain(
    moving: &Organism,
    other_organisms: &[Organism],
    environment: &Environment,
    dx: f64,
    dy: f64,
) -> Option<PushPlan> {
    let mut organism_visited = vec![false; other_organisms.len()];
    let mut physical_visited = std::collections::HashSet::new();
    let physical_keys: Vec<(usize, usize)> = environment
        .field
        .cells
        .iter()
        .enumerate()
        .flat_map(|(cell_index, cell)| {
            (0..cell.physical_materials.len())
                .map(move |material_index| (cell_index, material_index))
        })
        .collect();
    let moving_destination = organism_parts_at(moving, environment, dx, dy);
    let mut plan = PushPlan::default();
    if push_blockers_for_parts(
        &moving_destination,
        other_organisms,
        environment,
        &physical_keys,
        dx,
        dy,
        &mut organism_visited,
        &mut physical_visited,
        &mut plan,
    ) {
        Some(plan)
    } else {
        None
    }
}

fn push_blockers_for_parts(
    moving_destination: &[PlacedMaterialPart],
    other_organisms: &[Organism],
    environment: &Environment,
    physical_keys: &[(usize, usize)],
    dx: f64,
    dy: f64,
    organism_visited: &mut [bool],
    physical_visited: &mut std::collections::HashSet<(usize, usize)>,
    plan: &mut PushPlan,
) -> bool {
    for index in 0..other_organisms.len() {
        if organism_visited[index] {
            continue;
        }
        let candidate = &other_organisms[index];
        let candidate_parts = organism_parts_at(candidate, environment, 0.0, 0.0);
        if !parts_penetrate(moving_destination, &candidate_parts, environment.height) {
            continue;
        }
        if !can_translate_organism(candidate, environment, dx, dy) {
            return false;
        }
        let destination = organism_parts_at(candidate, environment, dx, dy);
        organism_visited[index] = true;
        if !push_blockers_for_parts(
            &destination,
            other_organisms,
            environment,
            physical_keys,
            dx,
            dy,
            organism_visited,
            physical_visited,
            plan,
        ) {
            return false;
        }
        plan.organisms.push(index);
    }

    for &(cell_index, material_index) in physical_keys {
        let key = (cell_index, material_index);
        if physical_visited.contains(&key) {
            continue;
        }
        let Some(candidate) = environment
            .field
            .cells
            .get(cell_index)
            .and_then(|cell| cell.physical_materials.get(material_index))
        else {
            continue;
        };
        if !candidate.is_realized() || candidate.material.is_empty() {
            continue;
        }
        let candidate_parts = physical_parts_at(candidate, environment, 0.0, 0.0);
        if !parts_penetrate(moving_destination, &candidate_parts, environment.height) {
            continue;
        }
        if !can_translate_physical(candidate, environment, dx, dy) {
            return false;
        }
        let destination = physical_parts_at(candidate, environment, dx, dy);
        physical_visited.insert(key);
        if !push_blockers_for_parts(
            &destination,
            other_organisms,
            environment,
            physical_keys,
            dx,
            dy,
            organism_visited,
            physical_visited,
            plan,
        ) {
            return false;
        }
        plan.physical.push(key);
    }
    true
}

fn apply_push_plan(
    other_organisms: &mut [Organism],
    environment: &mut Environment,
    mut plan: PushPlan,
    dx: f64,
    dy: f64,
) {
    for index in plan.organisms.drain(..) {
        translate_organism(&mut other_organisms[index], dx, dy, environment.height);
    }

    plan.physical.sort_unstable_by(|a, b| b.cmp(a));
    let mut pushed = Vec::with_capacity(plan.physical.len());
    for (cell_index, material_index) in plan.physical {
        let physical = environment.field.cells[cell_index]
            .physical_materials
            .remove(material_index);
        let mut physical = physical;
        translate_physical(&mut physical, dx, dy, environment.height);
        pushed.push(physical);
    }
    if !pushed.is_empty() {
        environment.field.revision = environment.field.revision.wrapping_add(1);
    }
    for physical in pushed {
        if let Some(placement) = physical
            .placements
            .as_ref()
            .and_then(|placements| placements.first())
        {
            if let Some(index) = environment
                .field
                .index_for_position(placement.x, placement.y)
            {
                environment.field.cells[index]
                    .physical_materials
                    .push(physical);
            }
        }
    }
}

fn organism_parts_at(
    organism: &Organism,
    environment: &Environment,
    dx: f64,
    dy: f64,
) -> Vec<PlacedMaterialPart> {
    organism
        .structure
        .units
        .iter()
        .filter_map(|unit| {
            let shape = unit.shape(&environment.catalog)?;
            Some(PlacedMaterialPart {
                part_index: 0,
                form: shape.form.clone(),
                placement: Placement {
                    x: unit.placement.x + dx,
                    y: unit.placement.y + dy,
                    rotation_radians: unit.placement.rotation_radians,
                },
            })
        })
        .collect()
}

fn physical_parts_at(
    physical: &crate::physical_material::PhysicalMaterial,
    environment: &Environment,
    dx: f64,
    dy: f64,
) -> Vec<PlacedMaterialPart> {
    let Some(placements) = &physical.placements else {
        return Vec::new();
    };
    physical
        .material
        .parts
        .iter()
        .zip(placements.iter())
        .enumerate()
        .filter_map(|(part_index, ((name, amount), placement))| {
            if (*amount - 1.0).abs() > 1e-9 {
                return None;
            }
            let base = environment.catalog.iter().find(|b| b.name == *name)?;
            Some(PlacedMaterialPart {
                part_index,
                form: base.shape.form.clone(),
                placement: Placement {
                    x: placement.x + dx,
                    y: placement.y + dy,
                    rotation_radians: placement.rotation_radians,
                },
            })
        })
        .collect()
}

fn parts_penetrate(
    a: &[PlacedMaterialPart],
    b: &[PlacedMaterialPart],
    environment_height: f64,
) -> bool {
    a.iter().any(|part_a| {
        b.iter().any(|part_b| {
            if crate::material_geometry::placed_forms_penetrate(part_a, part_b, 0.0) {
                return true;
            }
            // The active field is vertically periodic. Test the two wrapped
            // images needed to detect contact across the seam without making
            // the seam itself a physical wall.
            if environment_height <= 0.0 {
                return false;
            }
            let mut wrapped = part_b.clone();
            wrapped.placement.y += environment_height;
            if crate::material_geometry::placed_forms_penetrate(part_a, &wrapped, 0.0) {
                return true;
            }
            wrapped.placement.y -= 2.0 * environment_height;
            crate::material_geometry::placed_forms_penetrate(part_a, &wrapped, 0.0)
        })
    })
}

fn wrap_y(y: f64, height: f64) -> f64 {
    if height > 0.0 {
        y.rem_euclid(height)
    } else {
        y
    }
}

fn can_translate_physical(
    physical: &crate::physical_material::PhysicalMaterial,
    environment: &Environment,
    dx: f64,
    dy: f64,
) -> bool {
    let parts = physical_parts_at(physical, environment, dx, dy);
    !parts.is_empty()
        && parts.iter().all(|part| {
            let radius = part.form.bounding_radius();
            let x = part.placement.x;
            let y = part.placement.y;
            x.is_finite() && y.is_finite() && x - radius >= 0.0 && x + radius <= environment.width
        })
}

fn translate_physical(
    physical: &mut crate::physical_material::PhysicalMaterial,
    dx: f64,
    dy: f64,
    environment_height: f64,
) {
    if let Some(placements) = physical.placements.as_mut() {
        for placement in placements {
            placement.x += dx;
            placement.y = wrap_y(placement.y + dy, environment_height);
        }
    }
}

fn can_translate_organism(
    organism: &Organism,
    environment: &Environment,
    dx: f64,
    dy: f64,
) -> bool {
    organism.structure.units.iter().all(|unit| {
        let Some(shape) = unit.shape(&environment.catalog) else {
            return false;
        };
        let radius = shape.form.bounding_radius();
        let x = unit.placement.x + dx;
        let y = unit.placement.y + dy;
        x.is_finite() && y.is_finite() && x - radius >= 0.0 && x + radius <= environment.width
    })
}

fn translate_reproductive_construction(
    organism: &mut Organism,
    dx: f64,
    dy: f64,
    environment_height: f64,
) {
    if let Some(construction) = organism.reproductive_construction.as_mut() {
        construction.developmental_origin.x += dx;
        construction.developmental_origin.y =
            wrap_y(construction.developmental_origin.y + dy, environment_height);
        for unit in &mut construction.developing_structure.units {
            unit.placement.x += dx;
            unit.placement.y = wrap_y(unit.placement.y + dy, environment_height);
        }
    }
}

fn translate_organism(organism: &mut Organism, dx: f64, dy: f64, environment_height: f64) {
    organism.developmental_origin.x += dx;
    organism.developmental_origin.y =
        wrap_y(organism.developmental_origin.y + dy, environment_height);
    for point in &mut organism.occupied_cells {
        point.x += dx;
        point.y = wrap_y(point.y + dy, environment_height);
    }
    for unit in &mut organism.structure.units {
        unit.placement.x += dx;
        unit.placement.y = wrap_y(unit.placement.y + dy, environment_height);
    }
    translate_reproductive_construction(organism, dx, dy, environment_height);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn movement_direction(organism: &Organism) -> Option<(f64, f64)> {
        crate::movement_direction::movement_direction_periodic(organism, 0.0)
    }

    fn empty_environment(simulation: &Simulation) -> Environment {
        let mut environment = simulation.environment.clone();
        for cell in &mut environment.field.cells {
            cell.materials.clear();
            cell.physical_materials.clear();
        }
        environment
    }

    #[test]
    fn movement_cost_matches_reference_curve_at_default_efficiency() {
        let cases = [
            (2.7, 1.2),
            (4.0, 1.34),
            (8.0, 1.64),
            (16.0, 2.0),
            (32.0, 3.4),
            (64.0, 5.8),
            (128.0, 10.6),
            (256.0, 19.2),
            (512.0, 35.2),
            (1024.0, 65.0),
        ];
        for (mass, expected) in cases {
            let actual = movement_energy_cost(mass, DEFAULT_MOVEMENT_EFFICIENCY);
            assert!(
                (actual - expected).abs() < 0.06,
                "mass {mass}: expected {expected}, got {actual}"
            );
        }
    }

    #[test]
    fn movement_cost_increases_with_realized_mass() {
        let masses = [2.7, 4.0, 8.0, 16.0, 32.0, 64.0, 128.0, 256.0];
        for pair in masses.windows(2) {
            assert!(
                movement_energy_cost(pair[1], DEFAULT_MOVEMENT_EFFICIENCY)
                    > movement_energy_cost(pair[0], DEFAULT_MOVEMENT_EFFICIENCY)
            );
        }
    }

    #[test]
    fn movement_efficiency_changes_energy_cost_not_distance() {
        assert!(
            movement_energy_cost(16.0, 1.0) < movement_energy_cost(16.0, 0.8)
        );
        assert!(
            movement_energy_cost(16.0, 0.5) > movement_energy_cost(16.0, 0.8)
        );
        assert_eq!(MOVEMENT_BASE_STEP_DISTANCE, 4.0);
    }

    #[test]
    fn movement_cost_is_finite_for_nonnegative_mass_and_valid_efficiency() {
        for mass in [0.0, 2.7, 16.0, 1024.0, 1.0e12] {
            for efficiency in [0.05, 0.8, 1.0] {
                assert!(movement_energy_cost(mass, efficiency).is_finite());
            }
        }
    }

    #[test]
    fn movement_direction_uses_learned_memory_experience() {
        let simulation = Simulation::new(7, 20.0);
        let mut organism = simulation.organisms[0].clone();
        organism.memory.push(crate::state::MemoryPoint {
            x: organism.occupied_cells[0].x,
            y: organism.occupied_cells[0].y - 20.0,
            strength: 1.0,
            spectrum: crate::harmonics::ToneSpectrum::empty(),
            outcome: None,
        });
        let (x, y) = movement_direction(&organism).expect("direction should exist");
        assert!(x.abs() < f64::EPSILON);
        assert!(y < 0.0);
    }

    #[test]
    fn movement_direction_without_inputs_gets_soft_random_push() {
        let simulation = Simulation::new(7, 20.0);
        let organism = simulation.organisms[0].clone();
        let (x, y) = movement_direction(&organism).expect("initial direction should exist");
        let magnitude = (x * x + y * y).sqrt();
        assert!((magnitude - 1.0).abs() < 1e-12);

        let mut other = organism.clone();
        other.id = "2".into();
        let other_direction = movement_direction(&other).expect("other organism should move");
        assert_ne!((x, y), other_direction);
    }

    #[test]
    fn move_translates_developing_offspring_with_parent() {
        let mut simulation = Simulation::new(7, 20.0);
        let mut environment = empty_environment(&simulation);
        let mut organism = simulation.organisms.remove(0);
        organism.development_stage = crate::state::DevelopmentStage::Adult;
        let mut ledger = crate::state::EnergyLedger::default();
        assert!(crate::reproduction::begin_reproduction(
            &mut organism,
            &mut simulation.rng,
            &simulation.environment.catalog,
            &mut ledger,
        ));
        let before = organism
            .reproductive_construction
            .as_ref()
            .unwrap()
            .developmental_origin
            .clone();
        assert!(Simulation::try_move_cell(
            &mut organism,
            &mut environment,
            &mut [],
            12.0,
            -7.0,
        ));
        let after = &organism
            .reproductive_construction
            .as_ref()
            .unwrap()
            .developmental_origin;
        assert!((after.x - before.x - 12.0).abs() < 1e-9);
        assert!((after.y - before.y + 7.0).abs() < 1e-9);
    }

    #[test]
    fn move_translates_anchor_and_structure() {
        let simulation = Simulation::new(7, 20.0);
        let mut environment = empty_environment(&simulation);
        let mut organism = simulation.organisms[0].clone();
        let anchor = organism.occupied_cells[0].clone();
        let placements: Vec<_> = organism
            .structure
            .units
            .iter()
            .map(|u| (u.placement.x, u.placement.y))
            .collect();
        assert!(Simulation::try_move_cell(
            &mut organism,
            &mut environment,
            &mut [],
            12.0,
            -7.0
        ));
        assert!((organism.occupied_cells[0].x - anchor.x - 12.0).abs() < 1e-9);
        assert!((organism.occupied_cells[0].y - anchor.y + 7.0).abs() < 1e-9);
        for (unit, (x, y)) in organism.structure.units.iter().zip(placements) {
            assert!((unit.placement.x - x - 12.0).abs() < 1e-9);
            assert!((unit.placement.y - y + 7.0).abs() < 1e-9);
        }
    }

    #[test]
    fn blocked_zero_move_and_nonfinite_move_are_rejected() {
        let simulation = Simulation::new(7, 20.0);
        let mut environment = simulation.environment.clone();
        let mut organism = simulation.organisms[0].clone();
        organism.occupied_cells[0].x = 0.0;
        organism.occupied_cells[0].y = 0.0;
        assert!(!Simulation::try_move_cell(
            &mut organism,
            &mut environment,
            &mut [],
            0.0,
            0.0
        ));
        assert!(!Simulation::try_move_cell(
            &mut organism,
            &mut environment,
            &mut [],
            f64::NAN,
            1.0
        ));
    }

    #[test]
    fn movement_pushes_realized_physical_material() {
        let simulation = Simulation::new(7, 20.0);
        let mut environment = empty_environment(&simulation);
        let mut organism = simulation.organisms[0].clone();
        let x = organism.structure.units[0].placement.x;
        let y = organism.structure.units[0].placement.y;
        let placement = crate::structure::Placement {
            x: x + 6.0,
            y,
            rotation_radians: 0.0,
        };
        let physical = crate::physical_material::PhysicalMaterial::realized(
            crate::resources::Material {
                parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
                internal_bonds: vec![crate::resources::InternalBond {
                    part_a: 0,
                    part_b: 1,
                }],
            },
            vec![
                placement,
                crate::structure::Placement {
                    x: x + 6.0,
                    y,
                    rotation_radians: 0.0,
                },
            ],
            &environment.catalog,
        )
        .expect("realized composite should be valid");
        let index = environment
            .field
            .index_for_position(placement.x, placement.y)
            .expect("physical material must be in bounds");
        assert!(environment.field.deposit_physical_at_index(index, physical));
        assert!(Simulation::try_move_cell(
            &mut organism,
            &mut environment,
            &mut [],
            5.0,
            0.0
        ));
        let moved_index = environment
            .field
            .index_for_position(x + 11.0, y)
            .expect("pushed material must remain in bounds");
        let pushed = environment.field.cells[moved_index]
            .physical_materials
            .first()
            .expect("pushed material must remain in the field");
        assert_eq!(pushed.placements.as_ref().unwrap()[0].x, x + 11.0);
    }

    #[test]
    fn movement_pushes_another_organism_atomically() {
        let simulation = Simulation::new(7, 20.0);
        let mut environment = empty_environment(&simulation);
        let mut organism = simulation.organisms[0].clone();
        let mut blocker = simulation.organisms[0].clone();
        blocker.id = "pushed".to_string();
        let x = organism.structure.units[0].placement.x;
        let y = organism.structure.units[0].placement.y;
        for unit in &mut blocker.structure.units {
            unit.placement.x = x + 6.0;
            unit.placement.y = y;
        }
        let mut others = vec![blocker];
        let blocker_before = others[0].structure.units[0].placement.x;
        assert!(Simulation::try_move_cell(
            &mut organism,
            &mut environment,
            &mut others,
            5.0,
            0.0
        ));
        assert!((organism.structure.units[0].placement.x - (x + 5.0)).abs() < 1e-9);
        assert!((others[0].structure.units[0].placement.x - (blocker_before + 5.0)).abs() < 1e-9);
    }

    #[test]
    fn movement_propagates_an_organism_push_chain_atomically() {
        let simulation = Simulation::new(7, 20.0);
        let mut environment = empty_environment(&simulation);
        let mut organism = simulation.organisms[0].clone();
        let mut first = simulation.organisms[0].clone();
        let mut second = simulation.organisms[0].clone();
        first.id = "first".to_string();
        second.id = "second".to_string();
        let x = organism.structure.units[0].placement.x;
        let y = organism.structure.units[0].placement.y;
        for unit in &mut first.structure.units {
            unit.placement.x = x + 6.0;
            unit.placement.y = y;
        }
        for unit in &mut second.structure.units {
            unit.placement.x = x + 10.0;
            unit.placement.y = y;
        }
        let first_before = first.structure.units[0].placement.x;
        let second_before = second.structure.units[0].placement.x;
        let mut others = vec![first, second];
        assert!(Simulation::try_move_cell(
            &mut organism,
            &mut environment,
            &mut others,
            5.0,
            0.0
        ));
        assert_eq!(organism.structure.units[0].placement.x, x + 5.0);
        assert_eq!(others[0].structure.units[0].placement.x, first_before + 5.0);
        assert_eq!(
            others[1].structure.units[0].placement.x,
            second_before + 5.0
        );
    }

    #[test]
    fn movement_pushes_realized_physical_material_and_reindexes_it() {
        let simulation = Simulation::new(7, 20.0);
        let mut environment = empty_environment(&simulation);
        let mut organism = simulation.organisms[0].clone();
        let x = organism.structure.units[0].placement.x;
        let y = organism.structure.units[0].placement.y;
        let placement = crate::structure::Placement {
            x: x + 6.0,
            y,
            rotation_radians: 0.0,
        };
        let physical = crate::physical_material::PhysicalMaterial::realized(
            crate::resources::Material::free_base("Carbon", 1.0),
            vec![placement],
            &environment.catalog,
        )
        .expect("single carbon should have a valid physical realization");
        let original_index = environment
            .field
            .index_for_position(placement.x, placement.y)
            .expect("physical material must be in bounds");
        assert!(environment
            .field
            .deposit_physical_at_index(original_index, physical));
        assert!(Simulation::try_move_cell(
            &mut organism,
            &mut environment,
            &mut [],
            5.0,
            0.0
        ));
        let moved_x = x + 11.0;
        let moved_index = environment
            .field
            .index_for_position(moved_x, y)
            .expect("pushed material must remain in bounds");
        let physical = environment.field.cells[moved_index]
            .physical_materials
            .first()
            .expect("pushed physical material must be reindexed");
        assert_eq!(physical.placements.as_ref().unwrap()[0].x, moved_x);
    }

    #[test]
    fn touching_another_organism_does_not_block_movement() {
        let simulation = Simulation::new(7, 20.0);
        let mut environment = empty_environment(&simulation);
        let mut organism = simulation.organisms[0].clone();
        let mut blocker = simulation.organisms[0].clone();
        blocker.id = "touching".to_string();
        let x = organism.structure.units[0].placement.x;
        let y = organism.structure.units[0].placement.y;
        for unit in &mut blocker.structure.units {
            unit.placement.x = x + 10.0;
            unit.placement.y = y;
        }
        assert!(Simulation::try_move_cell(
            &mut organism,
            &mut environment,
            &mut [blocker],
            5.0,
            0.0
        ));
    }

    #[test]
    fn failed_push_chain_is_atomic_for_all_affected_objects() {
        let simulation = Simulation::new(7, 20.0);
        let mut environment = empty_environment(&simulation);
        let mut organism = simulation.organisms[0].clone();
        let mut first = simulation.organisms[0].clone();
        let mut second = simulation.organisms[0].clone();
        first.id = "first".to_string();
        second.id = "second".to_string();
        let x = environment.width - 17.0;
        let y = organism.structure.units[0].placement.y;
        for unit in &mut organism.structure.units {
            unit.placement.x = x;
            unit.placement.y = y;
        }
        organism.occupied_cells[0].x = x;
        organism.occupied_cells[0].y = y;
        for unit in &mut first.structure.units {
            unit.placement.x = x + 6.0;
            unit.placement.y = y;
        }
        for unit in &mut second.structure.units {
            unit.placement.x = x + 12.0;
            unit.placement.y = y;
        }
        let organism_before = organism.structure.clone();
        let first_before = first.structure.clone();
        let second_before = second.structure.clone();
        let mut others = vec![first, second];
        assert!(!Simulation::try_move_cell(
            &mut organism,
            &mut environment,
            &mut others,
            5.0,
            0.0
        ));
        assert_eq!(organism.structure.units, organism_before.units);
        assert_eq!(organism.structure.bonds, organism_before.bonds);
        assert_eq!(others[0].structure.units, first_before.units);
        assert_eq!(others[0].structure.bonds, first_before.bonds);
        assert_eq!(others[1].structure.units, second_before.units);
        assert_eq!(others[1].structure.bonds, second_before.bonds);
    }

    #[test]
    fn moving_realized_material_preserves_intrinsic_realization() {
        let simulation = Simulation::new(7, 20.0);
        let mut environment = empty_environment(&simulation);
        let mut organism = simulation.organisms[0].clone();
        let x = organism.structure.units[0].placement.x;
        let y = organism.structure.units[0].placement.y;
        let material = crate::resources::Material {
            parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
            internal_bonds: vec![crate::resources::InternalBond {
                part_a: 0,
                part_b: 1,
            }],
        };
        let placements = vec![
            crate::structure::Placement {
                x: x + 4.0,
                y,
                rotation_radians: 0.0,
            },
            crate::structure::Placement {
                x: x + 5.0,
                y,
                rotation_radians: 0.25,
            },
        ];
        let physical = crate::physical_material::PhysicalMaterial::realized(
            material,
            placements.clone(),
            &environment.catalog,
        )
        .expect("realized bonded material should be valid");
        let original = physical.clone();
        let original_index = environment
            .field
            .index_for_position(placements[0].x, placements[0].y)
            .expect("material must be in bounds");
        assert!(environment
            .field
            .deposit_physical_at_index(original_index, physical));
        assert!(Simulation::try_move_cell(
            &mut organism,
            &mut environment,
            &mut [],
            5.0,
            0.0
        ));
        let moved_index = environment
            .field
            .index_for_position(x + 9.0, y)
            .expect("material must remain in bounds");
        let moved = environment.field.cells[moved_index]
            .physical_materials
            .iter()
            .find(|candidate| candidate.material.parts == original.material.parts)
            .expect("realized material should remain present");
        assert_eq!(moved.material, original.material);
        assert_eq!(moved.internal_connections, original.internal_connections);
        let moved_placements = moved.placements.as_ref().expect("placements must remain");
        let original_placements = original.placements.as_ref().expect("placements must exist");
        assert_eq!(moved_placements.len(), original_placements.len());
        for (moved, original) in moved_placements.iter().zip(original_placements) {
            assert!((moved.x - original.x - 5.0).abs() < 1e-9);
            assert!((moved.y - original.y).abs() < 1e-9);
            assert_eq!(moved.rotation_radians, original.rotation_radians);
        }
    }
}
