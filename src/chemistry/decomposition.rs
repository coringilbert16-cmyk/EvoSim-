#![expect(dead_code, reason = "Staged API retained for subsystem integration")]
use crate::energy_ledger::{EnergyLedgerAuthority, EnergyReason, EnergyTransaction};
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
    ) -> Vec<PhysicalMaterial> {
        if !self.is_finished() {
            return Vec::new();
        }
        self.structure
            .units
            .iter()
            .filter_map(|unit| realize_released_unit(unit, catalog))
            .collect()
    }
}

fn realize_released_unit(
    unit: &crate::structure::StructuralUnit,
    catalog: &[crate::resources::BaseResource],
) -> Option<PhysicalMaterial> {
    if unit.material.parts.len() == 1 {
        let (name, _) = unit.material.parts.first()?;
        let resource = catalog.iter().find(|resource| resource.name == *name)?;
        if resource.physical_state == crate::resources::PhysicalState::Fluid {
            return PhysicalMaterial::realized_with_single_fluid_shape(
                unit.material.clone(),
                unit.placement,
                unit.shape(catalog)?.clone(),
                catalog,
            );
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
    // A decomposition step still breaks an existing physical bond. Its
    // energy source is therefore the bond's stored intrinsic potential, not
    // the constituent material potential (which was never consumed by
    // formation).
    let mut trial_structure = body.structure.clone();
    trial_structure.break_matching_bond(target)?;
    let released_material = if trial_structure.bonds.is_empty() {
        Some(
            trial_structure
                .units
                .iter()
                .filter_map(|unit| realize_released_unit(unit, &environment.catalog))
                .collect(),
        )
    } else {
        None
    };

    let before = body.energy_budget;
    let (gross, usable, heat) =
        crate::transformation::bond_break_energy_yield(target.bond_energy, 1.0)?;
    let transaction = EnergyTransaction {
        reason: EnergyReason::Decomposition,
        potential_released: 0.0,
        usable_delta: usable,
        structural_delta: -gross,
        work_expended: 0.0,
        heat_dissipated: heat,
    };
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
        let mut water = StructuralUnit::from_material(
            crate::resources::Material::free_base("Water", 2.0),
            placement,
        )
        .unwrap();
        assert!(water.realize_default_geometry(&catalog));
        let unit_area = match &catalog.iter().find(|resource| resource.name == "Water").unwrap().shape.form {
            crate::resources::Form::Circle { radius } => std::f64::consts::PI * radius * radius,
            _ => panic!("default Water shape should have a measurable area"),
        };
        let merged_area = 2.0 * unit_area;
        let family = crate::capillary_geometry::CapillaryContactFamily::solve(
            merged_area,
            std::f64::consts::FRAC_PI_2,
        )
        .unwrap();
        let arc = crate::capillary_geometry::CircularArcBoundary::from_flat_contact(family).unwrap();
        let shape = crate::resources::Shape {
            form: crate::resources::Form::Fluid {
                nominal_area: merged_area,
                boundary: None,
                arc_boundary: Some(arc),
                multi_contact_boundary: None,
            },
        };
        assert!(water.realize_fluid_geometry(shape.clone(), &catalog));
        let mut structure = OrganismStructure::new();
        structure.add_unit(water);
        let mut body = DecomposingBody::new(
            structure,
            0.0,
            Position { x: 0.0, y: 0.0 },
        )
        .unwrap();

        let released = body.release_finished_material(&catalog);
        assert_eq!(released.len(), 1);
        assert_eq!(released[0].realized_shape_for_part(0, &catalog), Some(shape));
    }

    #[test]
    fn zero_bond_structure_is_finished() {
        let structure = minimal_realized_structure();
        let body = DecomposingBody::new(structure, 0.0, Position { x: 0.0, y: 0.0 }).unwrap();
        assert!(body.is_finished());
    }
}
