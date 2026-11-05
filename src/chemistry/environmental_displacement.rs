//! Preflight evaluation for a coupled environmental displacement reaction.
//!
//! This module deliberately evaluates, but does not commit, a proposed
//! A-B + C -> A-C + B replacement. Runtime integration must supply activation
//! accumulated from C's validated physical contact with A. Keeping that signal
//! explicit prevents an unrelated BREAK followed by COMBINE from masquerading
//! as displacement.

use crate::combine::{bond_strength, intrinsic_bond_potential};
use crate::energy_ledger::ReactionEnergyBalance;
use crate::resources::ResourceProperties;

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
        crate::chemistry::activated(self.incoming_activation, self.disruption_barrier)
    }

    pub(crate) fn is_energy_favorable_without_external_input(self) -> bool {
        self.energy.required_external_input <= 1e-12
    }

    pub(crate) fn can_commit(self, legitimate_external_input: f64) -> bool {
        self.is_activated() && self.energy.can_commit_with(legitimate_external_input)
    }
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
    let replacement_bond_energy =
        intrinsic_bond_potential(anchor, incoming, replacement_strength);
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
    fn carbon_oxygen_can_replace_carbon_nitrogen_when_incoming_contact_activates_it() {
        let anchor = properties("Carbon");
        let displaced = properties("Nitrogen");
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
        let displaced = properties("Nitrogen");
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
}
