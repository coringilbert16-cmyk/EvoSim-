//! Primitive chemistry math.
//!
//! This module deliberately contains no resource catalogue mapping and no
//! runtime transition policy. Resource chemical positions remain a separate
//! catalog decision. The functions here implement the approved equations so
//! the migration can be tested without reusing the retired `reactivity`
//! semantics.

/// Fixed normalization ceiling for the chemical-position coordinate system.
/// The catalog currently spans 1.5..12.5, while the chemistry scale is defined
/// with headroom to 13.0.
pub const CHEMICAL_D_MAX: f64 = 13.0;

/// Nonlinear curve constant. One inverse-scale unit makes the exponent reach 1
/// at the top of the defined chemical domain.
pub const CHEMICAL_K: f64 = 1.0 / CHEMICAL_D_MAX;

/// Physical interaction radius in the shared unit geometry scale.
pub const CHEMICAL_CONTACT_RADIUS: f64 = 1.0;

/// Chemistry force unit. This is an attraction scale, not resource energy.
pub const CHEMICAL_MAX_FORCE: f64 = 1.0;
/// Characteristic distance used to convert the normalized chemistry force scale
/// into the simulation's physical energy unit. Force × distance = energy.
pub const CHEMICAL_ENERGY_DISTANCE: f64 = CHEMICAL_CONTACT_RADIUS;
/// One full normalized chemistry unit therefore represents this much physical
/// energy. With the approved scale this is 1.0 energy unit, but the conversion
/// is explicit rather than relying on identical numerical values by accident.
pub const CHEMICAL_ENERGY_PER_NORMALIZED_UNIT: f64 = CHEMICAL_MAX_FORCE * CHEMICAL_ENERGY_DISTANCE;

/// Fraction of accumulated reaction dissipated per tick when no new reaction
/// energy replaces it.
pub const CHEMICAL_DISSIPATION: f64 = 0.10;

/// Approved bounded nonlinear interaction potential for a chemical-position
/// separation.
///
/// The result is 0 at zero separation and 1 at D == d_max. Values are
/// clamped to the physical domain rather than silently extrapolated.
pub fn interaction_potential(position_a: f64, position_b: f64, k: f64, d_max: f64) -> Option<f64> {
    if !position_a.is_finite()
        || !position_b.is_finite()
        || !k.is_finite()
        || !d_max.is_finite()
        || k <= 0.0
        || d_max <= 0.0
    {
        return None;
    }

    let separation = (position_a - position_b).abs().min(d_max);
    let denominator = (k * d_max).exp() - 1.0;
    if !denominator.is_finite() || denominator <= 0.0 {
        return None;
    }

    let numerator = (k * separation).exp() - 1.0;
    Some((numerator / denominator).clamp(0.0, 1.0))
}

/// Distance/contact expression of a static chemical interaction potential.
///
/// This is zero outside the interaction radius and reaches the full static
/// potential at contact. Penetration is not represented as stronger contact.
pub fn contact_factor(distance: f64, radius: f64) -> Option<f64> {
    if !distance.is_finite() || !radius.is_finite() || radius <= 0.0 || distance < 0.0 {
        return None;
    }
    if distance > radius {
        return Some(0.0);
    }

    Some((1.0 - distance / radius).powi(2).clamp(0.0, 1.0))
}

/// Attraction expressed by the approved static-potential/contact model.
pub fn attraction(
    position_a: f64,
    position_b: f64,
    distance: f64,
    k: f64,
    d_max: f64,
    radius: f64,
    max_force: f64,
) -> Option<f64> {
    if !max_force.is_finite() || max_force < 0.0 {
        return None;
    }

    let potential = interaction_potential(position_a, position_b, k, d_max)?;
    let contact = contact_factor(distance, radius)?;
    Some(potential * max_force * contact)
}

/// Converts realized interface alignment into chemistry engagement.
///
/// Facing is supplied by the physical contact system; chemistry only bounds it
/// to the normalized engagement domain. Distance attenuation remains the job
/// of contact_factor, so this does not duplicate spatial falloff.
pub fn interface_engagement(facing: f64) -> Option<f64> {
    if !facing.is_finite() {
        return None;
    }
    Some(facing.clamp(0.0, 1.0))
}

/// A structural activation barrier expressed in the same normalized scale as
/// accumulated reaction. Existing bond strength is the physical barrier; no
/// second arbitrary chemistry threshold is introduced.
pub fn activation_barrier_from_bond_strength(bond_strength: f64) -> Option<f64> {
    if !bond_strength.is_finite() || bond_strength < 0.0 {
        return None;
    }
    normalized_chemistry_to_energy(bond_strength.clamp(0.0, 1.0))
}

/// Convert the normalized accumulated chemistry quantity into physical energy.
///
/// The conversion is derived from the approved force and characteristic contact
/// distance rather than introducing a second tuning constant.
pub fn normalized_chemistry_to_energy(value: f64) -> Option<f64> {
    if !value.is_finite() || value < 0.0 {
        return None;
    }
    let energy = value * CHEMICAL_ENERGY_PER_NORMALIZED_UNIT;
    energy.is_finite().then_some(energy)
}

/// One tick of bounded reaction accumulation.
///
/// The returned value is never negative. Dissipation applies to accumulated
/// reaction, while the current local interaction supplies new accumulation.
pub fn accumulate_reaction(
    previous: f64,
    interaction_potential: f64,
    contact: f64,
    engagement: f64,
    dissipation: f64,
) -> Option<f64> {
    if !previous.is_finite()
        || !interaction_potential.is_finite()
        || !contact.is_finite()
        || !engagement.is_finite()
        || !dissipation.is_finite()
        || previous < 0.0
        || interaction_potential < 0.0
        || contact < 0.0
        || engagement < 0.0
        || dissipation < 0.0
    {
        return None;
    }

    let contact = contact.clamp(0.0, 1.0);
    let engagement = engagement.clamp(0.0, 1.0);
    let next = previous + interaction_potential * contact * engagement - dissipation * previous;
    Some(next.max(0.0))
}

/// Energy left after the physical work required to disrupt an existing bond.
///
/// Chemical BREAK is not a random extra failure mode: a reaction can disrupt
/// a bond only when the reaction has produced enough energy to overcome that
/// bond's physical disruption requirement. The caller supplies both quantities
/// because the chemistry model does not invent a bond-strength or activation
/// parameter.
pub fn chemical_break_surplus(reaction_energy: f64, disruption_cost: f64) -> Option<f64> {
    if !reaction_energy.is_finite()
        || !disruption_cost.is_finite()
        || reaction_energy < 0.0
        || disruption_cost < 0.0
    {
        return None;
    }

    Some((reaction_energy - disruption_cost).max(0.0))
}

/// Whether a chemical reaction has enough energy to disrupt a particular bond.
///
/// Equality is sufficient: the reaction does not need an arbitrary probability
/// or extra threshold once the physical disruption requirement has been met.
pub fn can_chemical_break(reaction_energy: f64, disruption_cost: f64) -> bool {
    chemical_break_surplus(reaction_energy, disruption_cost).is_some_and(|surplus| {
        reaction_energy >= disruption_cost && reaction_energy > 0.0 && surplus >= 0.0
    })
}

/// Whether accumulated interaction has reached the calculated activation
/// barrier. The barrier itself is deliberately supplied by the caller because
/// it depends on physical structure and interface state.
pub fn activated(accumulated_reaction: f64, barrier: f64) -> bool {
    accumulated_reaction.is_finite()
        && barrier.is_finite()
        && accumulated_reaction >= 0.0
        && barrier >= 0.0
        && accumulated_reaction >= barrier
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approved_parameters_define_one_fixed_chemistry_scale() {
        assert_eq!(CHEMICAL_D_MAX, 13.0);
        assert!((CHEMICAL_K * CHEMICAL_D_MAX - 1.0).abs() < 1e-12);
        assert_eq!(CHEMICAL_CONTACT_RADIUS, 1.0);
        assert_eq!(CHEMICAL_MAX_FORCE, 1.0);
        assert_eq!(CHEMICAL_DISSIPATION, 0.10);
    }

    #[test]
    fn interaction_potential_is_bounded_and_symmetric() {
        let a = interaction_potential(2.0, 12.0, CHEMICAL_K, CHEMICAL_D_MAX).unwrap();
        let b = interaction_potential(12.0, 2.0, CHEMICAL_K, CHEMICAL_D_MAX).unwrap();
        assert!((a - b).abs() < 1e-12);
        assert!((0.0..=1.0).contains(&a));
    }

    #[test]
    fn identical_positions_have_no_chemical_separation() {
        assert_eq!(interaction_potential(7.0, 7.0, 0.5, 13.0), Some(0.0));
    }

    #[test]
    fn maximum_separation_reaches_full_static_potential() {
        assert_eq!(interaction_potential(1.0, 14.0, 0.5, 13.0), Some(1.0));
    }

    #[test]
    fn contact_factor_grows_toward_contact() {
        let far = contact_factor(1.0, 1.0).unwrap();
        let near = contact_factor(0.5, 1.0).unwrap();
        let contact = contact_factor(0.0, 1.0).unwrap();
        assert_eq!(far, 0.0);
        assert!(near > far);
        assert_eq!(contact, 1.0);
    }

    #[test]
    fn attraction_is_maximal_at_contact() {
        let near = attraction(1.0, 14.0, 0.5, 0.5, 13.0, 1.0, 10.0).unwrap();
        let contact = attraction(1.0, 14.0, 0.0, 0.5, 13.0, 1.0, 10.0).unwrap();
        assert!(contact > near);
        assert!((contact - 10.0).abs() < 1e-12);
    }

    #[test]
    fn static_potential_does_not_depend_on_distance() {
        let far = interaction_potential(1.0, 14.0, 0.5, 13.0).unwrap();
        let contact = interaction_potential(1.0, 14.0, 0.5, 13.0).unwrap();
        assert_eq!(far, contact);
    }

    #[test]
    fn interface_engagement_uses_realized_facing() {
        assert_eq!(interface_engagement(1.0), Some(1.0));
        assert_eq!(interface_engagement(0.5), Some(0.5));
        assert_eq!(interface_engagement(-1.0), Some(0.0));
    }

    #[test]
    fn normalized_chemistry_has_explicit_energy_conversion() {
        assert_eq!(CHEMICAL_ENERGY_DISTANCE, 1.0);
        assert_eq!(CHEMICAL_ENERGY_PER_NORMALIZED_UNIT, 1.0);
        assert_eq!(normalized_chemistry_to_energy(0.75), Some(0.75));
    }

    #[test]
    fn activation_barrier_is_existing_bond_strength() {
        assert_eq!(activation_barrier_from_bond_strength(0.75), Some(0.75));
        assert_eq!(activation_barrier_from_bond_strength(2.0), Some(1.0));
    }

    #[test]
    fn reaction_accumulates_only_from_engaged_contact() {
        let no_contact = accumulate_reaction(0.0, 1.0, 0.0, 1.0, 0.0).unwrap();
        let engaged = accumulate_reaction(0.0, 1.0, 1.0, 1.0, 0.0).unwrap();
        assert_eq!(no_contact, 0.0);
        assert_eq!(engaged, 1.0);
    }

    #[test]
    fn reaction_dissipates_without_new_interaction() {
        let next = accumulate_reaction(1.0, 0.0, 1.0, 1.0, 0.25).unwrap();
        assert!((next - 0.75).abs() < 1e-12);
    }

    #[test]
    fn chemical_break_requires_enough_reaction_energy() {
        assert_eq!(chemical_break_surplus(4.0, 5.0), Some(0.0));
        assert!(!can_chemical_break(4.0, 5.0));
        assert!(can_chemical_break(5.0, 5.0));
        assert_eq!(chemical_break_surplus(8.0, 5.0), Some(3.0));
    }

    #[test]
    fn activation_is_barrier_based() {
        assert!(!activated(0.99, 1.0));
        assert!(activated(1.0, 1.0));
        assert!(activated(1.5, 1.0));
    }
}
