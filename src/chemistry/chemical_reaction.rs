//! Local runtime chemical reaction accumulation.

use crate::chemistry_library::{ChemistryKey, ChemistryLibrary};
use crate::contact::ConnectionCompatibilityCache;
use crate::resources::BaseResource;
use crate::state::Organism;
use crate::structure::Bond;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ChemicalBreakOperation {
    pub(crate) organism_id: String,
    pub(crate) bond: Bond,
    pub(crate) reaction_energy: f64,
    pub(crate) disruption_cost: f64,
}

fn reaction_key(bond: &Bond, interface_signature: &str) -> String {
    format!(
        "{:?}|{:?}|{}",
        bond.endpoint_a, bond.endpoint_b, interface_signature
    )
}

fn material_identity(material: &crate::resources::Material) -> String {
    // Preserve the established base-resource keys used by the checked-in
    // chemistry cache. A one-part, one-unit material is still identified by
    // its resource name; amounts and composition remain explicit for mixtures
    // and non-unit quantities.
    if material.parts.len() == 1
        && crate::chemistry_library::quantized_amount(material.parts[0].1) == 1_000_000_000
    {
        return material.parts[0].0.clone();
    }

    let mut parts = material.parts.clone();
    parts.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
    });
    parts
        .into_iter()
        .map(|(name, amount)| {
            format!(
                "{}@{}",
                name,
                crate::chemistry_library::quantized_amount(amount)
            )
        })
        .collect::<Vec<_>>()
        .join("+")
}

pub(crate) fn accumulate(
    organism: &Organism,
    catalog: &[BaseResource],
    accumulation: &mut std::collections::BTreeMap<String, f64>,
    chemistry_library: &mut ChemistryLibrary,
) -> Vec<ChemicalBreakOperation> {
    if organism.structure.bonds.is_empty() {
        return Vec::new();
    }
    let current_prefix = format!("{}|rev:{}|", organism.id, organism.structure_revision);
    accumulation.retain(|key, _| {
        !key.starts_with(&format!("{}|rev:", organism.id)) || key.starts_with(&current_prefix)
    });
    let mut cache = ConnectionCompatibilityCache::new();
    let bonds = organism.structure.bonds.clone();
    let mut operations = Vec::new();

    for bond in bonds {
        let Some(unit_a) = organism
            .structure
            .unit_index(bond.endpoint_a.constituent_id)
        else {
            continue;
        };
        let Some(unit_b) = organism
            .structure
            .unit_index(bond.endpoint_b.constituent_id)
        else {
            continue;
        };
        let Some(a) = organism
            .structure
            .units
            .get(unit_a)
            .and_then(|u| u.properties(catalog))
        else {
            continue;
        };
        let Some(b) = organism
            .structure
            .units
            .get(unit_b)
            .and_then(|u| u.properties(catalog))
        else {
            continue;
        };
        let Some(position_a) = a.chemical_position else {
            continue;
        };
        let Some(position_b) = b.chemical_position else {
            continue;
        };

        let candidate = crate::contact::connection_pair_candidates_cached(
            &organism.structure,
            unit_a,
            unit_b,
            catalog,
            &mut cache,
        )
        .into_iter()
        .find(|candidate| {
            (candidate.endpoint_a == bond.endpoint_a.location
                && candidate.endpoint_b == bond.endpoint_b.location)
                || (candidate.endpoint_a == bond.endpoint_b.location
                    && candidate.endpoint_b == bond.endpoint_a.location)
        });
        let Some(candidate) = candidate else {
            continue;
        };

        let geometry_material_a = organism.structure.units[unit_a]
            .material
            .parts
            .first()
            .map(|part| part.0.as_str())
            .unwrap_or("unknown");
        let geometry_material_b = organism.structure.units[unit_b]
            .material
            .parts
            .first()
            .map(|part| part.0.as_str())
            .unwrap_or("unknown");
        let material_a = material_identity(&organism.structure.units[unit_a].material);
        let material_b = material_identity(&organism.structure.units[unit_b].material);
        let Some(interface) = crate::geometry_reference_library::resolve_live_contact_candidate(
            geometry_material_a,
            &organism.structure.units[unit_a],
            geometry_material_b,
            &organism.structure.units[unit_b],
            candidate,
            catalog,
        ) else {
            continue;
        };
        let key = format!(
            "{}|rev:{}|{}",
            organism.id,
            organism.structure_revision,
            reaction_key(&bond, &interface.signature)
        );

        let key_record = ChemistryKey::from_live_geometry(&material_a, &material_b, &interface);
        let potential = match chemistry_library.get(&key_record) {
            Some(record) => record.static_potential,
            None => {
                let Some(calculated) = crate::chemistry::interaction_potential(
                    position_a,
                    position_b,
                    crate::chemistry::CHEMICAL_K,
                    crate::chemistry::CHEMICAL_D_MAX,
                ) else {
                    continue;
                };
                match chemistry_library.get_or_insert_static_potential(key_record, calculated) {
                    Ok(Some(value)) => value,
                    Ok(None) | Err(_) => calculated,
                }
            }
        };
        let Some(contact) = crate::chemistry::contact_factor(
            candidate.distance,
            crate::chemistry::CHEMICAL_CONTACT_RADIUS,
        ) else {
            continue;
        };
        let Some(engagement) = crate::chemistry::interface_engagement(candidate.facing) else {
            continue;
        };
        let previous = accumulation.get(&key).copied().unwrap_or(0.0);
        let Some(next) = crate::chemistry::accumulate_reaction(
            previous,
            potential,
            contact,
            engagement,
            crate::chemistry::CHEMICAL_DISSIPATION,
        ) else {
            continue;
        };

        if next <= 0.0 {
            accumulation.remove(&key);
            continue;
        }
        let Some(barrier) = crate::chemistry::activation_barrier_from_bond_strength(bond.strength)
        else {
            continue;
        };

        // Accumulation remains normalized chemistry state. Only when the
        // activation threshold is crossed do we convert that state into the
        // physical energy quantity consumed by the BREAK ledger transaction.
        let Some(reaction_energy) = crate::chemistry::normalized_chemistry_to_energy(next) else {
            continue;
        };
        if crate::chemistry::activated(reaction_energy, barrier) {
            // The activation barrier is the physical disruption work for
            // this realized bond. Reuse that exact quantity; do not calculate
            // a second constituent-derived BREAK cost.
            let disruption_cost = barrier;
            operations.push(ChemicalBreakOperation {
                organism_id: organism.id.clone(),
                bond,
                reaction_energy,
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

#[cfg(test)]
mod material_identity_tests {
    use super::*;
    use crate::resources::Material;

    #[test]
    fn one_unit_base_material_keeps_legacy_resource_name() {
        assert_eq!(
            material_identity(&Material::free_base("Carbon", 1.0)),
            "Carbon"
        );
    }

    #[test]
    fn composite_and_non_unit_material_identities_keep_amounts() {
        assert_eq!(
            material_identity(&Material::free_base("Carbon", 2.0)),
            "Carbon@2000000000"
        );
        let mixed = Material {
            parts: vec![("Hydrogen".into(), 1.0), ("Carbon".into(), 1.0)],
            internal_bonds: Vec::new(),
        };
        assert_eq!(
            material_identity(&mixed),
            "Carbon@1000000000+Hydrogen@1000000000"
        );
    }
    #[test]
    fn base_contact_key_matches_checked_in_cache_identity() {
        let carbon = Material::free_base("Carbon", 1.0);
        let hydrogen = Material::free_base("Hydrogen", 1.0);
        let interface = crate::geometry_reference_library::resolve_live_contact_interface(
            "Carbon",
            crate::structure::ConnectionEndpoint::Corner { point_index: 0 },
            "Hydrogen",
            crate::structure::ConnectionEndpoint::LineEndpoint { point_index: 0 },
        );
        let key = ChemistryKey::from_live_geometry(
            material_identity(&carbon),
            material_identity(&hydrogen),
            &interface,
        );
        assert_eq!(key.schema_version, 3);
        assert_eq!(key.material_a, "Carbon");
        assert_eq!(key.material_b, "Hydrogen");
        assert_eq!(key.interface_class, "rigid_point");
        assert_eq!(
            key.interface_signature,
            "live-v1|Carbon:corner:0|Hydrogen:line:0"
        );
    }
}
