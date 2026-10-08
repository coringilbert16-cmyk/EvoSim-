//! Local runtime chemical reaction accumulation.
use crate::contact::ConnectionCompatibilityCache;
use crate::resources::BaseResource;
use crate::state::Organism;
use crate::structure::Bond;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ChemicalBreakOperation {
    pub(crate) organism_id: String,
    pub(crate) bond: Bond,
    pub(crate) reaction_energy: f64,
    pub(crate) disruption_cost: f64,
}


fn reaction_key(bond: &Bond, interface_signature: &str) -> String {
    format!("{:?}|{:?}|{}", bond.endpoint_a, bond.endpoint_b, interface_signature)
}

pub(crate) fn accumulate(
    organism: &Organism,
    catalog: &[BaseResource],
    accumulation: &mut std::collections::BTreeMap<String, f64>,
) -> Vec<ChemicalBreakOperation> {
    if organism.structure.bonds.is_empty() {
        return Vec::new();
    }
    let current_prefix = format!("{}|rev:{}|", organism.id, organism.structure_revision);
    accumulation.retain(|key, _| !key.starts_with(&format!("{}|rev:", organism.id)) || key.starts_with(&current_prefix));
    let mut cache = ConnectionCompatibilityCache::new();
    let bonds = organism.structure.bonds.clone();
    let mut operations = Vec::new();

    for bond in bonds {
        let Some(unit_a) = organism.structure.unit_index(bond.endpoint_a.constituent_id) else { continue };
        let Some(unit_b) = organism.structure.unit_index(bond.endpoint_b.constituent_id) else { continue };
        let Some(a) = organism.structure.units.get(unit_a).and_then(|u| u.properties(catalog)) else { continue };
        let Some(b) = organism.structure.units.get(unit_b).and_then(|u| u.properties(catalog)) else { continue };
        let Some(position_a) = a.chemical_position else { continue };
        let Some(position_b) = b.chemical_position else { continue };

        let candidate = crate::contact::connection_pair_candidates_cached(
            &organism.structure, unit_a, unit_b, catalog, &mut cache,
        ).into_iter().find(|candidate| {
            (candidate.endpoint_a == bond.endpoint_a.location && candidate.endpoint_b == bond.endpoint_b.location)
                || (candidate.endpoint_a == bond.endpoint_b.location && candidate.endpoint_b == bond.endpoint_a.location)
        });
        let Some(candidate) = candidate else { continue };

        let material_a = organism.structure.units[unit_a].material.parts.first().map(|part| part.0.as_str()).unwrap_or("unknown");
        let material_b = organism.structure.units[unit_b].material.parts.first().map(|part| part.0.as_str()).unwrap_or("unknown");
        let Some(interface) = crate::geometry_reference_library::resolve_live_contact_candidate(
            material_a,
            &organism.structure.units[unit_a],
            material_b,
            &organism.structure.units[unit_b],
            candidate,
            catalog,
        ) else { continue };
        let key = format!("{}|rev:{}|{}", organism.id, organism.structure_revision, reaction_key(&bond, &interface.signature));

        let Some(potential) = crate::chemistry::interaction_potential(
            position_a, position_b, crate::chemistry::CHEMICAL_K, crate::chemistry::CHEMICAL_D_MAX,
        ) else { continue };
        let Some(contact) = crate::chemistry::contact_factor(
            candidate.distance, crate::chemistry::CHEMICAL_CONTACT_RADIUS,
        ) else { continue };
        let Some(engagement) = crate::chemistry::interface_engagement(candidate.facing) else { continue };
        let previous = accumulation.get(&key).copied().unwrap_or(0.0);
        let Some(next) = crate::chemistry::accumulate_reaction(
            previous, potential, contact, engagement, crate::chemistry::CHEMICAL_DISSIPATION,
        ) else { continue };

        if next <= 0.0 {
            accumulation.remove(&key);
            continue;
        }
        let Some(barrier) = crate::chemistry::activation_barrier_from_bond_strength(bond.strength) else { continue };

        if crate::chemistry::activated(next, barrier) {
            let Some(disruption_cost) = crate::transformation::break_work_cost(a, b, 1.0) else { continue };
            operations.push(ChemicalBreakOperation {
                organism_id: organism.id.clone(),
                bond,
                reaction_energy: next,
                disruption_cost,
            });
            accumulation.remove(&key);
        } else {
            accumulation.insert(key, next);
        }
    }
    operations
}

pub(crate) fn resolve(
    operation: ChemicalBreakOperation,
    organism: &mut Organism,
    ledger: &mut crate::state::EnergyLedger,
) -> bool {
    if organism.id != operation.organism_id {
        return false;
    }
    let processing_efficiency = organism.genome.processing_efficiency();
    let success = crate::transformation::settle_chemical_break_energy(
        organism,
        operation.bond,
        operation.reaction_energy,
        operation.disruption_cost,
        processing_efficiency,
        ledger,
    );
    // A failed event is stale or physically invalid. Its accumulation was
    // already removed when the event crossed the barrier, and the next
    // structure revision will invalidate any remaining interface state.
    success
}

