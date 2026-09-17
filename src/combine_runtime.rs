//! Runtime COMBINE execution boundary.
//! Physics is evaluated by `combine`; this module selects a physical
//! candidate, applies the returned result, mutates structure, and settles
//! the actual energy holder through the unified ledger authority.
use crate::combine::{
    bond_strength, eligible_candidates, required_investment, ExperimentalInteraction,
    FormationEvaluation,
};
use crate::contact::ConnectionCompatibilityCache;
use crate::energy_ledger::{EnergyLedgerAuthority, EnergyReason, EnergyTransaction};
use crate::physical_material::PhysicalMaterial;
use crate::resources::{BaseResource, Material};
use crate::state::{EnergyLedger, Environment, Organism};
use crate::structure::{BondEndpoint, ConnectionEndpoint, Placement, StructuralUnit};

const EPSILON: f64 = 1e-12;
pub(crate) const COMBINE_CONTACT_TOLERANCE: f64 = 1.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CombineAttempt {
    pub unit_a: usize,
    pub unit_b: usize,
    pub endpoint_a: ConnectionEndpoint,
    pub endpoint_b: ConnectionEndpoint,
    pub work_cost: f64,
    pub energy_invested: f64,
    pub interaction_direction: f64,
    pub interaction_magnitude: f64,
    pub interaction_energy: f64,
    pub formation_threshold: f64,
    pub net_energy_change: f64,
    pub bond_strength: f64,
    pub bond_energy: f64,
}

#[derive(Clone, Copy)]
struct BondFormationRequest {
    unit_a: usize,
    unit_b: usize,
    endpoint_a: ConnectionEndpoint,
    endpoint_b: ConnectionEndpoint,
    investment: f64,
    water: f64,
}

fn water_field_amount(environment: &Environment, organism: &Organism) -> f64 {
    organism
        .occupied_cells
        .first()
        .and_then(|p| environment.field.index_for_position(p.x, p.y))
        .map(|i| {
            environment.field.cells[i]
                .materials
                .iter()
                .flat_map(|m| m.parts.iter())
                .filter(|(n, _)| n == "Water")
                .map(|(_, a)| *a)
                .sum()
        })
        .unwrap_or(0.0)
}

fn energy_requirement(investment: f64, work: f64, interaction: f64) -> Option<f64> {
    if !investment.is_finite()
        || investment < 0.0
        || !work.is_finite()
        || work < 0.0
        || !interaction.is_finite()
    {
        return None;
    }
    let required = investment + work - interaction;
    required.is_finite().then_some(required.max(0.0))
}

fn evaluate_candidate(
    structure: &crate::structure::OrganismStructure,
    ua: usize,
    ub: usize,
    candidate: crate::contact::ConnectionPairCandidate,
    catalog: &[BaseResource],
    water: f64,
) -> Option<(FormationEvaluation, ExperimentalInteraction, f64, f64, f64)> {
    if candidate.distance > COMBINE_CONTACT_TOLERANCE
        || !candidate.available_a
        || !candidate.available_b
    {
        return None;
    }
    let a = structure.units.get(ua)?.properties(catalog)?;
    let b = structure.units.get(ub)?.properties(catalog)?;
    let evaluation = crate::combine::evaluate_formation(candidate, a.cohesion, b.cohesion);
    let (interaction, work, investment) = required_investment(a, b, evaluation, water).ok()?;
    let required = energy_requirement(investment, work, interaction.signed_value)?;
    Some((evaluation, interaction, work, investment, required))
}

fn form_bond(
    structure: &mut crate::structure::OrganismStructure,
    request: BondFormationRequest,
    catalog: &[BaseResource],
    cache: &mut ConnectionCompatibilityCache,
    ledger: &mut EnergyLedger,
    energy: &mut f64,
) -> Option<CombineAttempt> {
    let BondFormationRequest {
        unit_a: ua,
        unit_b: ub,
        endpoint_a,
        endpoint_b,
        investment,
        water,
    } = request;
    if ua >= structure.units.len() || ub >= structure.units.len() || ua == ub {
        return None;
    }
    let id_a = structure.physical_id(ua)?;
    let id_b = structure.physical_id(ub)?;
    let candidate =
        crate::contact::connection_pair_candidates_cached(structure, ua, ub, catalog, cache)
            .into_iter()
            .find(|c| {
                c.endpoint_a == endpoint_a
                    && c.endpoint_b == endpoint_b
                    && c.distance <= COMBINE_CONTACT_TOLERANCE
                    && c.available_a
                    && c.available_b
            })?;
    let a = structure.units[ua].properties(catalog)?;
    let b = structure.units[ub].properties(catalog)?;
    let evaluation = crate::combine::evaluate_formation(candidate, a.cohesion, b.cohesion);
    if !crate::combine::formation_succeeds(evaluation, investment) {
        return None;
    }
    let (interaction, work, threshold) = required_investment(a, b, evaluation, water).ok()?;
    if (threshold - investment).abs() > EPSILON || interaction.signed_value < 0.0 {
        return None;
    }
    let strength = bond_strength(a, b);
    if !strength.is_finite() {
        return None;
    }
    let mut trial_structure = structure.clone();
    let bond = crate::structure::Bond {
        endpoint_a: BondEndpoint::new(id_a, endpoint_a),
        endpoint_b: BondEndpoint::new(id_b, endpoint_b),
        strength,
        bond_energy: investment,
    };
    crate::contact::try_add_bond(&mut trial_structure, bond, catalog).ok()?;
    let before = *energy;
    let transaction = EnergyTransaction {
        reason: EnergyReason::Combine,
        potential_released: interaction.signed_value,
        usable_delta: interaction.signed_value - investment - work,
        structural_delta: investment,
        heat_dissipated: work,
    };
    if !ledger.settle_transaction(energy, transaction) {
        *energy = before;
        return None;
    }
    let net = *energy - before;
    *structure = trial_structure;
    Some(CombineAttempt {
        unit_a: ua,
        unit_b: ub,
        endpoint_a,
        endpoint_b,
        work_cost: work,
        energy_invested: investment,
        interaction_direction: interaction.direction,
        interaction_magnitude: interaction.magnitude,
        interaction_energy: interaction.signed_value,
        formation_threshold: threshold,
        net_energy_change: net,
        bond_strength: strength,
        bond_energy: investment,
    })
}

fn physical_material_candidate(
    material: &Material,
    placement: Placement,
    catalog: &[BaseResource],
) -> Option<StructuralUnit> {
    if !material.is_valid() || material.is_empty() || material.parts.len() != 1 {
        return None;
    }
    let (name, amount) = material.parts.first()?;
    if (*amount - 1.0).abs() > EPSILON || material.has_internal_structure() {
        return None;
    }
    let mut unit =
        StructuralUnit::from_material(Material::free_base(name.clone(), 1.0), placement)?;
    if !unit.realize_default_geometry(catalog) {
        return None;
    }
    Some(unit)
}

fn instantiate_realized_single(
    structure: &mut crate::structure::OrganismStructure,
    instance: &PhysicalMaterial,
) -> Option<usize> {
    let placements = instance.placements.as_ref()?;
    if placements.len() != 1 || instance.material.parts.len() != 1 {
        return None;
    }
    let unit = StructuralUnit::from_material(instance.material.clone(), placements[0])?;
    Some(structure.add_unit(unit))
}

pub(crate) fn instantiate_one_unit(
    organism: &mut Organism,
    catalog: &[BaseResource],
) -> Option<usize> {
    let material = organism.stored_material.materials.first()?.clone();
    let (x, y) = organism
        .occupied_cells
        .first()
        .map(|p| (p.x, p.y))
        .unwrap_or((0.0, 0.0));
    if material.has_internal_structure() {
        let instance = organism.stored_material.peek_matching_physical(&material)?;
        if !instance.is_realized() {
            return None;
        }
        let mut trial = organism.structure.clone();
        let indices = crate::material_restoration::restore_material(
            &mut trial,
            &instance,
            Placement {
                x,
                y,
                rotation_radians: 0.0,
            },
            catalog,
        )?;
        organism.stored_material.take_matching_physical(&material)?;
        organism.structure = trial;
        return indices.first().copied();
    }
    let unit = if let Some(instance) = organism.stored_material.peek_matching_physical(&material) {
        if instance.is_realized() {
            let mut trial = organism.structure.clone();
            let translated = instance.translated(Placement {
                x,
                y,
                rotation_radians: 0.0,
            })?;
            let index = instantiate_realized_single(&mut trial, &translated)?;
            organism.stored_material.take_matching_physical(&material)?;
            organism.structure = trial;
            return Some(index);
        }
        physical_material_candidate(
            &material,
            Placement {
                x,
                y,
                rotation_radians: 0.0,
            },
            catalog,
        )?
    } else {
        physical_material_candidate(
            &material,
            Placement {
                x,
                y,
                rotation_radians: 0.0,
            },
            catalog,
        )?
    };
    organism.stored_material.take_matching(&material)?;
    Some(organism.structure.add_unit(unit))
}

// The remainder of this module contains the existing candidate search and
// COMBINE execution paths.
