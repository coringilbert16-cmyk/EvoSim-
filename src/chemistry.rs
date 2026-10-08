//! Primitive chemistry math.
//!
//! This module deliberately contains no resource catalogue mapping and no
//! runtime transition policy. Resource chemical positions remain a separate
//! catalog decision. The functions here implement the approved equations so
//! the migration can be tested without reusing the retired `reactivity`
//! semantics.

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
    fn interaction_potential_is_bounded_and_symmetric() {
        let a = interaction_potential(2.0, 12.0, 0.5, 13.0).unwrap();
        let b = interaction_potential(12.0, 2.0, 0.5, 13.0).unwrap();
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
    fn activation_is_barrier_based() {
        assert!(!activated(0.99, 1.0));
        assert!(activated(1.0, 1.0));
        assert!(activated(1.5, 1.0));
    }
}
