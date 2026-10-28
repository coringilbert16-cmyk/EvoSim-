#![expect(dead_code, reason = "Staged API retained for subsystem integration")]
use crate::energy_ledger::{EnergyLedgerAuthority, EnergyReason};
use crate::physical_material::PhysicalMaterial;
use crate::state::{EnergyLedger, Environment, Organism, Position};
use crate::structure::OrganismStructure;

#[derive(Clone, Debug)]
pub(crate) struct DecomposingBody {
    pub(crate) structure: OrganismStructure,
    pub(crate) energy_budget: f64,
    pub(crate) position: Position,
}

impl DecomposingBody {
    pub(crate) fn new(
        structure: OrganismStructure,
        energy_budget: f64,
        position: Position,
    ) -> Option<Self> {
        if !energy_budget.is_finite() || energy_budget < 0.0 {
            return None;
        }
        Some(Self {
            structure,
            energy_budget,
            position,
        })
    }

    pub(crate) fn is_finished(&self) -> bool {
        self.structure.bonds.is_empty()
    }

    pub(crate) fn release_finished_material(
        &mut self,
        catalog: &[crate::resources::BaseResource],
    ) -> Option<Vec<PhysicalMaterial>> {
        if !self.is_finished() {
            return Some(Vec::new());
        }
        self.structure
            .units
            .iter()
            .map(|unit| realize_released_unit(unit, catalog))
            .collect()
    }
}

pub(crate) fn realize_released_unit(
    unit: &crate::structure::StructuralUnit,
    catalog: &[crate::resources::BaseResource],
) -> Option<PhysicalMaterial> {
    if unit.material.parts.len() == 1 {
        let (name, amount) = unit.material.parts.first()?;
        let resource = catalog.iter().find(|resource| resource.name == *name)?;
        if resource.physical_state == crate::resources::PhysicalState::Fluid {
            let shape = unit.shape(catalog)?.clone();
            if *amount > 1.0 || matches!(shape.form, crate::resources::Form::Fluid { .. }) {
                return PhysicalMaterial::realized_with_single_fluid_shape(
                    unit.material.clone(),
                    unit.placement,
                    shape,
                    catalog,
                );
            }
        }
    }
    PhysicalMaterial::realized(unit.material.clone(), vec![unit.placement], catalog)
}

pub(crate) struct DecompositionStep {
    pub(crate) net_energy: f64,
    pub(crate) potential_energy: f64,
    pub(crate) heat: f64,
    pub(crate) released_material: Option<Vec<PhysicalMaterial>>,
}

pub(crate) fn resolve_one_bond_with_ledger(
    body: &mut DecomposingBody,
    environment: &Environment,
    ledger: &mut EnergyLedger,
) -> Option<DecompositionStep> {
    let target = *body.structure.bonds.first()?;
    // Decomposition BREAK raises structural potential by the bond's
    // dissociation-energy magnitude. It must be funded by the body's own
    // legitimate energy budget; constituent type maxima are not energy stores.
    let mut trial_structure = body.structure.clone();
    trial_structure.break_matching_bond(target)?;
    let released_material = if trial_structure.bonds.is_empty() {
        Some(
            trial_structure
                .units
                .iter()
                .map(|unit| realize_released_unit(unit, &environment.catalog))
                .collect::<Option<Vec<_>>>()?,
        )
    } else {
        None
    };

    let before = body.energy_budget;
    let (gross, _usable, heat) =
        crate::transformation::bond_break_energy_yield(target.bond_energy, 1.0)?;
    let before_potential = crate::energy_ledger::structural_potential(&body.structure)?;
    let after_potential = crate::energy_ledger::structural_potential(&trial_structure)?;
    let structural_delta = after_potential - before_potential;
    let delta_tolerance = 1e-9 * gross.abs().max(1.0);
    if (structural_delta - gross).abs() > delta_tolerance {
        return None;
    }
    let transaction =
        crate::energy_ledger::ReactionEnergyBalance::calculate(0.0, structural_delta, 0.0, heat)?
            .settlement_transaction(EnergyReason::Decomposition)?;
    if !ledger.settle_transaction(&mut body.energy_budget, transaction) {
        return None;
    }
    let net = body.energy_budget - before;
    body.structure = trial_structure;
    Some(DecompositionStep {
        net_energy: net,
        potential_energy: gross,
        heat,
        released_material,
    })
}

pub(crate) fn harvestable_decomposition_energy(
    organisms: &[Organism],
    position: &Position,
) -> Option<usize> {
    organisms
        .iter()
        .enumerate()
        .filter_map(|(index, organism)| {
            let organism_position = organism.occupied_cells.first()?;
            let distance =
                (organism_position.x - position.x).hypot(organism_position.y - position.y);
            Some((index, distance))
        })
        .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(index, _)| index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::default_catalog;
    use crate::structure::{OrganismStructure, Placement, StructuralUnit};

    fn minimal_realized_structure() -> OrganismStructure {
        let catalog = default_catalog();
        let mut structure = OrganismStructure::new();
        let mut unit = StructuralUnit::new(
            "Carbon".to_string(),
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        );
        assert!(unit.realize_default_geometry(&catalog));
        structure.add_unit(unit);
        structure
    }

    #[test]
    fn retains_structure_budget_and_position() {
        let structure = minimal_realized_structure();
        let body = DecomposingBody::new(structure, 4.0, Position { x: 2.0, y: 3.0 }).unwrap();
        assert_eq!(body.structure.units.len(), 1);
        assert_eq!(body.energy_budget, 4.0);
        assert_eq!(body.position, Position { x: 2.0, y: 3.0 });
    }

    #[test]
    fn release_finished_material_preserves_aggregate_fluid_shape() {
        let catalog = default_catalog();
        let placement = Placement {
            x: 0.0,
            y: 0.0,
            rotation_radians: 0.0,
        };
        let mut water_unit = StructuralUnit::from_material(
            crate::resources::Material::free_base("Water", 2.0),
            placement,
        )
        .unwrap();
        assert!(water_unit.realize_default_geometry(&catalog));
        let water_resource = catalog
            .iter()
            .find(|resource| resource.name == "Water")
            .unwrap();
        let unit_area = match &water_resource.shape.form {
            crate::resources::Form::Circle { radius } => std::f64::consts::PI * radius * radius,
            _ => panic!("default Water shape should have a measurable area"),
        };
        let merged_area = 2.0 * unit_area;
        let family = crate::capillary_geometry::CapillaryContactFamily::solve(
            merged_area,
            std::f64::consts::FRAC_PI_2,
        )
        .unwrap();
        let arc =
            crate::capillary_geometry::CircularArcBoundary::from_flat_contact(family).unwrap();
        let shape = crate::resources::Shape {
            form: crate::resources::Form::Fluid {
                nominal_area: merged_area,
                boundary: None,
                arc_boundary: Some(arc),
                multi_contact_boundary: None,
            },
        };
        assert!(water_unit.realize_fluid_geometry(shape.clone(), &catalog));
        let mut structure = OrganismStructure::new();
        structure.add_unit(water_unit);
        let mut body = DecomposingBody::new(structure, 0.0, Position { x: 0.0, y: 0.0 }).unwrap();

        let released = body
            .release_finished_material(&catalog)
            .expect("all finished material should retain a valid physical realization");
        assert_eq!(released.len(), 1);
        assert_eq!(
            released[0].realized_shape_for_part(0, &catalog),
            Some(shape)
        );
    }

    #[test]
    fn zero_bond_structure_is_finished() {
        let structure = minimal_realized_structure();
        let body = DecomposingBody::new(structure, 0.0, Position { x: 0.0, y: 0.0 }).unwrap();
        assert!(body.is_finished());
    }

    #[test]
    fn finished_release_rejects_aggregate_fluid_without_exact_geometry() {
        let catalog = default_catalog();
        let mut water = StructuralUnit::from_material(
            crate::resources::Material::free_base("Water", 2.0),
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        )
        .unwrap();
        assert!(water.realize_default_geometry(&catalog));
        let mut structure = OrganismStructure::new();
        structure.add_unit(water);
        let mut body = DecomposingBody::new(structure, 0.0, Position { x: 0.0, y: 0.0 }).unwrap();

        assert!(
            body.release_finished_material(&catalog).is_none(),
            "invalid aggregate fluid must not be silently dropped during release"
        );
    }
    #[test]
    fn rejected_decomposition_settlement_preserves_body_and_ledger() {
        use crate::structure::{Bond, BondEndpoint, ConnectionEndpoint};

        let catalog = default_catalog();
        let mut structure = OrganismStructure::new();
        let mut first = StructuralUnit::new(
            "Carbon".to_string(),
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        );
        let mut second = StructuralUnit::new(
            "Carbon".to_string(),
            Placement {
                x: 1.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        );
        assert!(first.realize_default_geometry(&catalog));
        assert!(second.realize_default_geometry(&catalog));
        structure.add_unit(first);
        structure.add_unit(second);
        let first_id = structure.units[0].physical_id;
        let second_id = structure.units[1].physical_id;
        structure.push_bond_unchecked(Bond {
            endpoint_a: BondEndpoint::new(first_id, ConnectionEndpoint::Corner { point_index: 0 }),
            endpoint_b: BondEndpoint::new(second_id, ConnectionEndpoint::Corner { point_index: 3 }),
            strength: 1.0,
            bond_energy: 10.0,
        });

        let mut body = DecomposingBody::new(structure, 100.0, Position { x: 5.0, y: 7.0 })
            .expect("decomposing body should be valid");
        let bonds_before = body.structure.bonds.clone();
        let units_before = body.structure.units.clone();
        let energy_before = body.energy_budget;
        // Force rejection deterministically; adding a small increment to
        // f64::MAX can round away and is not a reliable overflow fixture.
        let mut ledger = EnergyLedger {
            total_heat_dissipated: f64::NAN,
            ..EnergyLedger::default()
        };
        let ledger_before = ledger;

        let environment = Environment {
            width: 1000.0,
            height: 1000.0,
            chemistry_usable_energy: 0.0,
            resonant_energy: Default::default(),
            catalog,
            field: crate::environment::ActiveMaterialField::new(
                1000.0,
                1000.0,
                crate::environment::DEFAULT_CELL_SIZE,
            ),
        };
        assert!(
            resolve_one_bond_with_ledger(&mut body, &environment, &mut ledger).is_none(),
            "rejected ledger settlement must reject the decomposition step"
        );
        assert_eq!(body.structure.bonds, bonds_before);
        assert_eq!(body.structure.units, units_before);
        assert_eq!(body.energy_budget, energy_before);
        assert!(ledger.total_heat_dissipated.is_nan());
        assert_eq!(
            ledger.total_structural_energy_delta,
            ledger_before.total_structural_energy_delta
        );
        assert_eq!(
            ledger.total_work_expended,
            ledger_before.total_work_expended
        );
        assert_eq!(
            ledger.total_potential_energy_released,
            ledger_before.total_potential_energy_released
        );
    }
}
