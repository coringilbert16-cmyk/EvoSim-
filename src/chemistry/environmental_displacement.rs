//! Preflight evaluation for a coupled environmental displacement reaction.
//!
//! This module deliberately evaluates, but does not commit, a proposed
//! A-B + C -> A-C + B replacement. Runtime integration must supply activation
//! accumulated from C's validated physical contact with A. Keeping that signal
//! explicit prevents an unrelated BREAK followed by COMBINE from masquerading
//! as displacement.

#![expect(dead_code, reason = "Displacement API awaits live environmental scheduler integration")]

use crate::combine::{bond_strength, intrinsic_bond_potential};
use crate::contact::ConnectionPairCandidate;
use crate::energy_ledger::{EnergyLedgerAuthority, EnergyReason, ReactionEnergyBalance};
use crate::resources::{BaseResource, ResourceProperties};
use crate::structure::{Bond, BondEndpoint, OrganismStructure};

/// Energy and activation assessment for one proposed constituent replacement.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplacementPreflight {
    /// Activation supplied by the incoming constituent's contact with the anchor.
    pub(crate) incoming_activation: f64,
    /// Existing A-B bond's disruption threshold.
    pub(crate) disruption_barrier: f64,
    /// Stored potential magnitude of the existing A-B bond.
    pub(crate) old_bond_energy: f64,
    /// Stored potential magnitude of the proposed A-C bond.
    pub(crate) replacement_bond_energy: f64,
    /// Existing BREAK authority's physical disruption work requirement.
    pub(crate) disruption_work: f64,
    /// Complete signed bond-graph energy budget for the replacement.
    pub(crate) energy: ReactionEnergyBalance,
}

impl DisplacementPreflight {
    pub(crate) fn is_activated(self) -> bool {
        self.incoming_activation > 0.0
            && crate::chemistry::activated(self.incoming_activation, self.disruption_barrier)
    }

    pub(crate) fn is_energy_favorable_without_external_input(self) -> bool {
        self.energy.required_external_input <= 1e-12
    }

    pub(crate) fn can_commit(self, legitimate_external_input: f64) -> bool {
        self.is_activated() && self.energy.can_commit_with(legitimate_external_input)
    }
}

/// Accumulate one tick of activation from the incoming constituent's live
/// contact with the anchor. This mirrors environmental COMBINE's approved
/// static-potential, contact-falloff, facing-engagement, and dissipation equation.
pub(crate) fn accumulate_incoming_activation(
    anchor: ResourceProperties,
    incoming: ResourceProperties,
    candidate: ConnectionPairCandidate,
    previous: f64,
) -> Option<f64> {
    if candidate.distance > crate::combine_runtime::COMBINE_CONTACT_TOLERANCE
        || !candidate.available_a
        || !candidate.available_b
    {
        return None;
    }
    let interaction = crate::combine::chemical_interaction(
        anchor,
        incoming,
        candidate,
        crate::chemistry::CHEMICAL_K,
        crate::chemistry::CHEMICAL_D_MAX,
        crate::chemistry::CHEMICAL_CONTACT_RADIUS,
        crate::chemistry::CHEMICAL_MAX_FORCE,
    )?;
    let contact = crate::chemistry::contact_factor(
        candidate.distance,
        crate::chemistry::CHEMICAL_CONTACT_RADIUS,
    )?;
    let engagement = crate::chemistry::interface_engagement(candidate.facing)?;
    crate::chemistry::accumulate_reaction(
        previous,
        interaction.static_potential,
        contact,
        engagement,
        crate::chemistry::CHEMICAL_DISSIPATION,
    )
}

/// Fully evaluated trial topology and its independently exported physical products.
/// Nothing in this value has been committed to the live environment.
#[derive(Clone, Debug)]
pub(crate) struct DisplacementTrial {
    pub(crate) structure: OrganismStructure,
    source_units: Vec<crate::structure::StructuralUnit>,
    source_bonds: Vec<Bond>,
    pub(crate) products: Vec<crate::physical_material::PhysicalMaterial>,
    pub(crate) removed_bond: Bond,
    pub(crate) replacement_bond: Bond,
    pub(crate) energy: ReactionEnergyBalance,
    pub(crate) formation_work: f64,
    pub(crate) disruption_work: f64,
}

/// Build the proposed topology on a clone. The caller must discover the supplied
/// replacement candidate after the old A-B bond has been removed from an
/// equivalent trial graph, so the freed endpoint is available to contact search.
pub(crate) fn build_displacement_trial(
    source: &OrganismStructure,
    anchor_unit: usize,
    displaced_unit: usize,
    incoming_unit: usize,
    old_bond: Bond,
    replacement_candidate: ConnectionPairCandidate,
    investment: f64,
    incoming_activation: f64,
    reactivity_baseline: f64,
    dissipated_heat: f64,
    catalog: &[BaseResource],
) -> Option<DisplacementTrial> {
    if anchor_unit >= source.units.len()
        || displaced_unit >= source.units.len()
        || incoming_unit >= source.units.len()
        || anchor_unit == displaced_unit
        || anchor_unit == incoming_unit
        || displaced_unit == incoming_unit
    {
        return None;
    }

    let anchor_id = source.physical_id(anchor_unit)?;
    let displaced_id = source.physical_id(displaced_unit)?;
    let old_connects_expected_pair = (old_bond.endpoint_a.constituent_id == anchor_id
        && old_bond.endpoint_b.constituent_id == displaced_id)
        || (old_bond.endpoint_b.constituent_id == anchor_id
            && old_bond.endpoint_a.constituent_id == displaced_id);
    if !old_connects_expected_pair
        || !source
            .bonds
            .iter()
            .any(|bond| bond.has_same_identity(&old_bond))
    {
        return None;
    }

    let anchor = source.units[anchor_unit].properties(catalog)?;
    let displaced = source.units[displaced_unit].properties(catalog)?;
    let incoming = source.units[incoming_unit].properties(catalog)?;
    let old_strength = old_bond.strength;
    let preflight = evaluate_displacement_preflight(
        anchor,
        displaced,
        incoming,
        old_strength,
        incoming_activation,
        reactivity_baseline,
        0.0,
        dissipated_heat,
    )?;
    if !preflight.is_activated()
        || (old_bond.bond_energy - preflight.old_bond_energy).abs() > 1e-9
    {
        return None;
    }

    let mut trial = source.clone();
    trial.break_matching_bond(old_bond)?;

    // Revalidate the replacement contact and chemistry against the trial graph,
    // where the old endpoint is now genuinely available.
    let (formation, formation_work, required_investment, _) =
        crate::combine_runtime::selected_candidate_evaluation(
            &trial,
            anchor_unit,
            incoming_unit,
            replacement_candidate,
            catalog,
        )?;
    if !crate::combine::formation_succeeds(formation, investment)
        || (required_investment - investment).abs() > 1e-9
    {
        return None;
    }

    let anchor_id = trial.physical_id(anchor_unit)?;
    let incoming_id = trial.physical_id(incoming_unit)?;
    if anchor_id != source.physical_id(anchor_unit)?
        || incoming_id != source.physical_id(incoming_unit)?
    {
        return None;
    }
    let strength = bond_strength(anchor, incoming);
    let bond_energy = intrinsic_bond_potential(anchor, incoming, strength);
    if !strength.is_finite()
        || !(0.0..=1.0).contains(&strength)
        || !bond_energy.is_finite()
        || bond_energy < 0.0
    {
        return None;
    }
    let replacement_bond = Bond {
        endpoint_a: BondEndpoint::new(anchor_id, replacement_candidate.endpoint_a),
        endpoint_b: BondEndpoint::new(incoming_id, replacement_candidate.endpoint_b),
        strength,
        bond_energy,
    };
    crate::contact::try_add_bond(&mut trial, replacement_bond, catalog).ok()?;

    let reactant_structural = crate::energy_ledger::structural_potential(source)?;
    let product_structural = crate::energy_ledger::structural_potential(&trial)?;
    let disruption_work = preflight.disruption_work;
    let total_work = formation_work + disruption_work;
    let energy = ReactionEnergyBalance::calculate(
        reactant_structural,
        product_structural,
        total_work,
        dissipated_heat,
    )?;
    let products = crate::environmental_combine::physical_materials_from_structure_components(
        &trial, catalog,
    )?;
    if products.len() < 2 {
        return None;
    }

    Some(DisplacementTrial {
        structure: trial,
        source_units: source.units.clone(),
        source_bonds: source.bonds.clone(),
        products,
        removed_bond: old_bond,
        replacement_bond,
        energy,
        formation_work,
        disruption_work,
    })
}

/// Commit a previously evaluated trial against the unchanged source graph.
/// Energy settlement is staged first; the graph, ledger, and usable holder are
/// updated only after every validation and affordability check succeeds.
pub(crate) fn commit_displacement_trial(
    source: &mut OrganismStructure,
    trial: DisplacementTrial,
    ledger: &mut crate::state::EnergyLedger,
    usable_energy: &mut f64,
) -> Option<Vec<crate::physical_material::PhysicalMaterial>> {
    if source.units != trial.source_units || source.bonds != trial.source_bonds {
        return None;
    }

    let transaction = trial.energy.settlement_transaction(EnergyReason::Combine)?;
    let mut staged_ledger = *ledger;
    let mut staged_usable_energy = *usable_energy;
    if !staged_ledger.settle_transaction(&mut staged_usable_energy, transaction) {
        return None;
    }
    let usable_delta = staged_usable_energy - *usable_energy;
    let next_total_usable = staged_ledger.total_usable_energy_held + usable_delta;
    let next_total_structural =
        staged_ledger.total_structural_energy_held + trial.energy.net_reaction_change;
    if !usable_delta.is_finite()
        || !next_total_usable.is_finite()
        || next_total_usable < 0.0
        || !next_total_structural.is_finite()
    {
        return None;
    }
    staged_ledger.total_usable_energy_held = next_total_usable;
    staged_ledger.total_structural_energy_held = next_total_structural;

    *source = trial.structure;
    *ledger = staged_ledger;
    *usable_energy = staged_usable_energy;
    Some(trial.products)
}

/// Evaluate the coupled A-B -> A-C replacement without mutating any graph or
/// energy holder. incoming_activation must be accumulated from C's own
/// contact/engagement with A; it must not be copied from the old A-B bond.
///
/// The structural-energy terms use the same intrinsic bond-potential authority
/// as ordinary COMBINE. Physical work and heat are included once in the single
/// net reaction balance. This function does not claim to model unrepresented
/// chemical free-energy terms.
pub(crate) fn evaluate_displacement_preflight(
    anchor: ResourceProperties,
    displaced: ResourceProperties,
    incoming: ResourceProperties,
    old_bond_strength: f64,
    incoming_activation: f64,
    reactivity_baseline: f64,
    physical_work: f64,
    dissipated_heat: f64,
) -> Option<DisplacementPreflight> {
    if !incoming_activation.is_finite() || incoming_activation < 0.0 {
        return None;
    }

    let expected_old_strength = bond_strength(anchor, displaced);
    if !old_bond_strength.is_finite()
        || old_bond_strength < 0.0
        || (old_bond_strength - expected_old_strength).abs() > 1e-9
    {
        return None;
    }

    let replacement_strength = bond_strength(anchor, incoming);
    if !replacement_strength.is_finite() {
        return None;
    }

    let old_bond_energy = intrinsic_bond_potential(anchor, displaced, old_bond_strength);
    let replacement_bond_energy = intrinsic_bond_potential(anchor, incoming, replacement_strength);
    if !old_bond_energy.is_finite()
        || old_bond_energy < 0.0
        || !replacement_bond_energy.is_finite()
        || replacement_bond_energy < 0.0
    {
        return None;
    }

    let disruption_barrier = crate::chemistry::activation_barrier_from_bond_properties(
        old_bond_strength,
        anchor.reactivity,
        displaced.reactivity,
        reactivity_baseline,
    )?;

    // Existing environmental BREAK treats the activation barrier as required
    // physical disruption work, separately from the stored bond potential.
    // Include that work and the new-bond formation work in one net transaction.
    let total_work = physical_work + disruption_barrier;
    if !total_work.is_finite() {
        return None;
    }
    // Structural potential is negative for a retained stable bond. Replacing
    // one bond with another is evaluated as one complete pre/post-state change.
    let energy = ReactionEnergyBalance::calculate(
        -old_bond_energy,
        -replacement_bond_energy,
        total_work,
        dissipated_heat,
    )?;

    Some(DisplacementPreflight {
        incoming_activation,
        disruption_barrier,
        old_bond_energy,
        replacement_bond_energy,
        disruption_work: disruption_barrier,
        energy,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::default_catalog;

    fn properties(name: &str) -> ResourceProperties {
        default_catalog()
            .into_iter()
            .find(|resource| resource.name == name)
            .expect("canonical resource")
            .properties
    }

    fn catalog_reactivity_baseline() -> f64 {
        let catalog = default_catalog();
        catalog
            .iter()
            .map(|resource| resource.properties.reactivity)
            .sum::<f64>()
            / catalog.len() as f64
    }

    #[test]
    fn carbon_oxygen_can_replace_carbon_hydrogen_when_incoming_contact_activates_it() {
        let anchor = properties("Carbon");
        let displaced = properties("Hydrogen");
        let incoming = properties("Oxygen");
        let old_strength = bond_strength(anchor, displaced);
        let provisional = evaluate_displacement_preflight(
            anchor,
            displaced,
            incoming,
            old_strength,
            10.0,
            catalog_reactivity_baseline(),
            0.0,
            0.0,
        )
        .expect("finite catalog-backed preflight");

        assert!(provisional.replacement_bond_energy > provisional.old_bond_energy);
        assert!(provisional.is_activated());
        assert!(provisional.is_energy_favorable_without_external_input());
        assert!(provisional.can_commit(0.0));
        assert!(provisional.energy.residual(0.0).unwrap().abs() <= 1e-9);
    }

    #[test]
    fn weaker_carbon_nitrogen_replacement_of_carbon_oxygen_needs_real_energy_input() {
        let anchor = properties("Carbon");
        let displaced = properties("Oxygen");
        let incoming = properties("Nitrogen");
        let old_strength = bond_strength(anchor, displaced);
        let provisional = evaluate_displacement_preflight(
            anchor,
            displaced,
            incoming,
            old_strength,
            10.0,
            catalog_reactivity_baseline(),
            0.0,
            0.0,
        )
        .expect("finite catalog-backed preflight");

        assert!(provisional.replacement_bond_energy < provisional.old_bond_energy);
        assert!(provisional.is_activated());
        assert!(provisional.energy.required_external_input > 0.0);
        assert!(!provisional.can_commit(0.0));
        assert!(provisional.can_commit(provisional.energy.required_external_input));
        assert!(
            provisional
                .energy
                .residual(provisional.energy.required_external_input)
                .unwrap()
                .abs()
                <= 1e-9
        );
    }

    #[test]
    fn favorable_replacement_is_rejected_without_sufficient_incoming_activation() {
        let anchor = properties("Carbon");
        let displaced = properties("Hydrogen");
        let incoming = properties("Oxygen");
        let old_strength = bond_strength(anchor, displaced);
        let provisional = evaluate_displacement_preflight(
            anchor,
            displaced,
            incoming,
            old_strength,
            0.0,
            catalog_reactivity_baseline(),
            0.0,
            0.0,
        )
        .expect("finite catalog-backed preflight");

        assert!(provisional.is_energy_favorable_without_external_input());
        assert!(!provisional.is_activated());
        assert!(!provisional.can_commit(0.0));
    }

    #[test]
    fn invalid_or_unrelated_old_bond_strength_fails_closed() {
        let anchor = properties("Carbon");
        let displaced = properties("Nitrogen");
        let incoming = properties("Oxygen");
        assert!(evaluate_displacement_preflight(
            anchor,
            displaced,
            incoming,
            f64::NAN,
            10.0,
            catalog_reactivity_baseline(),
            0.0,
            0.0,
        )
        .is_none());
        assert!(evaluate_displacement_preflight(
            anchor,
            displaced,
            incoming,
            0.0,
            10.0,
            catalog_reactivity_baseline(),
            0.0,
            0.0,
        )
        .is_none());
    }

    fn three_participant_fixture(
        catalog: &[BaseResource],
    ) -> (OrganismStructure, Bond, ConnectionPairCandidate, f64) {
        let mut source = OrganismStructure::new();
        source.add_unit(crate::structure::StructuralUnit::new(
            "Carbon",
            crate::structure::Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));
        source.add_unit(crate::structure::StructuralUnit::new(
            "Hydrogen",
            crate::structure::Placement {
                x: 1.5,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));
        source.add_unit(crate::structure::StructuralUnit::new(
            "Oxygen",
            crate::structure::Placement {
                x: -1.577_350_269_189_625_8,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));

        let mut cache = crate::contact::ConnectionCompatibilityCache::new();
        let old_candidate =
            crate::contact::connection_pair_candidates_cached(&source, 0, 1, catalog, &mut cache)
                .into_iter()
                .find(|candidate| {
                    candidate.distance <= crate::combine_runtime::COMBINE_CONTACT_TOLERANCE
                        && candidate.available_a
                        && candidate.available_b
                })
                .expect("Carbon-Hydrogen physical contact");
        let anchor = source.units[0].properties(catalog).unwrap();
        let displaced = source.units[1].properties(catalog).unwrap();
        let strength = bond_strength(anchor, displaced);
        let old_bond = Bond {
            endpoint_a: BondEndpoint::new(source.physical_id(0).unwrap(), old_candidate.endpoint_a),
            endpoint_b: BondEndpoint::new(source.physical_id(1).unwrap(), old_candidate.endpoint_b),
            strength,
            bond_energy: intrinsic_bond_potential(anchor, displaced, strength),
        };
        crate::contact::try_add_bond(&mut source, old_bond, catalog)
            .expect("initial A-B bond should be physically valid");

        let mut replacement_trial = source.clone();
        replacement_trial.break_matching_bond(old_bond).unwrap();
        let replacement_candidate = crate::contact::connection_pair_candidates_cached(
            &replacement_trial,
            0,
            2,
            catalog,
            &mut cache,
        )
        .into_iter()
        .find(|candidate| {
            candidate.distance <= crate::combine_runtime::COMBINE_CONTACT_TOLERANCE
                && candidate.facing.is_finite()
                && candidate.facing > 0.0
                && candidate.available_a
                && candidate.available_b
        })
        .expect("freed Carbon endpoint should expose an engaged incoming Oxygen contact");
        let (_, _, investment, _) = crate::combine_runtime::selected_candidate_evaluation(
            &replacement_trial,
            0,
            2,
            replacement_candidate,
            catalog,
        )
        .expect("replacement candidate should pass ordinary COMBINE evaluation");
        (source, old_bond, replacement_candidate, investment)
    }

    #[test]
    fn incoming_activation_is_accumulated_from_the_replacement_contact() {
        let catalog = default_catalog();
        let (source, _old_bond, candidate, _investment) = three_participant_fixture(&catalog);
        let anchor = source.units[0].properties(&catalog).unwrap();
        let incoming = source.units[2].properties(&catalog).unwrap();
        let activation = accumulate_incoming_activation(anchor, incoming, candidate, 0.0)
            .expect("valid live contact should accumulate incoming influence");
        assert!(activation > 0.0);
        assert!(activation.is_finite());
    }

    #[test]
    fn trial_replacement_splits_displaced_constituent_and_preserves_source() {
        let catalog = default_catalog();
        let (source, old_bond, candidate, investment) = three_participant_fixture(&catalog);
        let source_before = source.clone();
        let trial = build_displacement_trial(
            &source,
            0,
            1,
            2,
            old_bond,
            candidate,
            investment,
            10.0,
            catalog_reactivity_baseline(),
            0.0,
            &catalog,
        )
        .expect("favorable activated replacement should produce a trial");

        assert_eq!(source.units, source_before.units);
        assert_eq!(source.bonds, source_before.bonds);
        assert_eq!(trial.structure.bonds.len(), 1);
        assert_eq!(trial.products.len(), 2);
        assert!(trial.products.iter().any(|product| {
            product.material.parts.iter().any(|part| part.0 == "Carbon")
                && product.material.parts.iter().any(|part| part.0 == "Oxygen")
                && product.material.internal_bonds.len() == 1
        }));
        assert!(trial.products.iter().any(|product| {
            product.material.parts == vec![("Hydrogen".to_owned(), 1.0)]
                && product.material.internal_bonds.is_empty()
        }));
        assert!(
            trial.disruption_work > 0.0 && trial.energy.required_external_input > 0.0,
            "a favorable replacement still has to fund the old bond's physical disruption work"
        );
        assert_eq!(trial.removed_bond, old_bond);
        let bonded_product = trial
            .products
            .iter()
            .find(|product| product.material.parts.len() == 2)
            .expect("A-C component should contain two constituents");
        assert_eq!(
            trial.replacement_bond.bond_energy,
            bonded_product.internal_connections.as_ref().unwrap()[0].bond_energy
        );
    }

    #[test]
    fn committed_trial_updates_graph_energy_and_products_together() {
        let catalog = default_catalog();
        let (mut source, old_bond, candidate, investment) = three_participant_fixture(&catalog);
        let mut ledger = crate::state::EnergyLedger {
            total_structural_energy_held: crate::energy_ledger::structural_potential(&source)
                .unwrap(),
            ..Default::default()
        };
        let trial = build_displacement_trial(
            &source,
            0,
            1,
            2,
            old_bond,
            candidate,
            investment,
            10.0,
            catalog_reactivity_baseline(),
            0.0,
            &catalog,
        )
        .expect("valid displacement trial");

        let available_energy = trial.energy.required_external_input;
        assert!(available_energy > 0.0);
        ledger.total_usable_energy_held = available_energy;
        let mut usable_energy = available_energy;
        let products =
            commit_displacement_trial(&mut source, trial, &mut ledger, &mut usable_energy)
                .expect("funded favorable trial should commit atomically");

        assert_eq!(source.bonds.len(), 1);
        assert_eq!(products.len(), 2);
        assert!(usable_energy.is_finite() && usable_energy >= 0.0);
        assert!(usable_energy < available_energy);
        assert_eq!(
            ledger.total_structural_energy_held,
            -source.bonds[0].bond_energy
        );
    }

    #[test]
    fn stale_trial_cannot_commit_or_change_energy_holders() {
        let catalog = default_catalog();
        let (mut source, old_bond, candidate, investment) = three_participant_fixture(&catalog);
        let trial = build_displacement_trial(
            &source,
            0,
            1,
            2,
            old_bond,
            candidate,
            investment,
            10.0,
            catalog_reactivity_baseline(),
            0.0,
            &catalog,
        )
        .expect("valid displacement trial");
        source.units[0].placement.x += 0.01;
        let graph_before = source.clone();
        let mut ledger = crate::state::EnergyLedger::default();
        let ledger_before = ledger;
        let mut usable_energy = 0.0;

        assert!(
            commit_displacement_trial(&mut source, trial, &mut ledger, &mut usable_energy,)
                .is_none()
        );
        assert_eq!(source.units, graph_before.units);
        assert_eq!(source.bonds, graph_before.bonds);
        assert_eq!(ledger, ledger_before);
        assert_eq!(usable_energy, 0.0);
    }

    #[test]
    fn trial_replacement_rejects_insufficient_incoming_activation_without_mutation() {
        let catalog = default_catalog();
        let (source, old_bond, candidate, investment) = three_participant_fixture(&catalog);
        let before = source.clone();
        assert!(build_displacement_trial(
            &source,
            0,
            1,
            2,
            old_bond,
            candidate,
            investment,
            0.0,
            catalog_reactivity_baseline(),
            0.0,
            &catalog,
        )
        .is_none());
        assert_eq!(source.units, before.units);
        assert_eq!(source.bonds, before.bonds);
    }
}
