//! Transfer capacity derived from physically accessible organism water.
//!
//! Permeability is not a stored material property and is not derived from an
//! abstract water pool or from the water content of the environmental
//! material. The locked model is a linear threshold/ramp/saturation function
//! of physically accessible organism water mass.
//!
//! W  = physically accessible organism water mass
//! WT = threshold water mass
//! Wmax = water mass at full permeability
//! Pmax = maximum transfer capacity
//!
//! P(W) = 0                       when W < WT
//! P(W) = linear WT -> Wmax       when WT <= W < Wmax
//! P(W) = Pmax                    when W >= Wmax

/// Calculate the transfer capacity for a physically accessible amount of
/// organism water.
///
/// The caller supplies WT, Wmax, and Pmax because those are model parameters,
/// not properties of a resource and not properties of the environmental field.
/// This function deliberately does not invent default values for them.
pub fn permeability_from_water_mass(
    water_mass: f64,
    threshold_water_mass: f64,
    full_permeability_water_mass: f64,
    maximum_transfer_capacity: f64,
) -> Option<f64> {
    if !water_mass.is_finite()
        || !threshold_water_mass.is_finite()
        || !full_permeability_water_mass.is_finite()
        || !maximum_transfer_capacity.is_finite()
        || water_mass < 0.0
        || threshold_water_mass < 0.0
        || full_permeability_water_mass < threshold_water_mass
        || maximum_transfer_capacity < 0.0
    {
        return None;
    }

    if water_mass < threshold_water_mass {
        return Some(0.0);
    }

    if water_mass >= full_permeability_water_mass {
        return Some(maximum_transfer_capacity);
    }

    let span = full_permeability_water_mass - threshold_water_mass;
    if span <= 0.0 {
        return Some(maximum_transfer_capacity);
    }

    let result = maximum_transfer_capacity
        * ((water_mass - threshold_water_mass) / span).clamp(0.0, 1.0);

    result.is_finite().then_some(result.clamp(0.0, maximum_transfer_capacity))
}

#[cfg(test)]
mod tests {
    use super::*;

    const WT: f64 = 2.0;
    const WMAX: f64 = 10.0;
    const PMAX: f64 = 4.0;

    #[test]
    fn below_threshold_has_no_transfer_capacity() {
        assert_eq!(permeability_from_water_mass(1.999, WT, WMAX, PMAX), Some(0.0));
    }

    #[test]
    fn threshold_is_the_start_of_the_linear_ramp() {
        assert_eq!(permeability_from_water_mass(WT, WT, WMAX, PMAX), Some(0.0));
    }

    #[test]
    fn midpoint_of_ramp_is_linear() {
        assert!((permeability_from_water_mass(6.0, WT, WMAX, PMAX).unwrap() - 2.0).abs() < 1e-12);
    }

    #[test]
    fn full_water_level_reaches_maximum_capacity() {
        assert_eq!(permeability_from_water_mass(WMAX, WT, WMAX, PMAX), Some(PMAX));
    }

    #[test]
    fn water_above_full_level_is_saturated() {
        assert_eq!(permeability_from_water_mass(100.0, WT, WMAX, PMAX), Some(PMAX));
    }

    #[test]
    fn zero_maximum_capacity_stays_zero() {
        assert_eq!(permeability_from_water_mass(100.0, WT, WMAX, 0.0), Some(0.0));
    }

    #[test]
    fn invalid_parameter_order_is_rejected() {
        assert!(permeability_from_water_mass(1.0, 10.0, 2.0, PMAX).is_none());
    }

    #[test]
    fn invalid_values_are_rejected() {
        assert!(permeability_from_water_mass(-1.0, WT, WMAX, PMAX).is_none());
        assert!(permeability_from_water_mass(1.0, f64::NAN, WMAX, PMAX).is_none());
        assert!(permeability_from_water_mass(1.0, WT, f64::INFINITY, PMAX).is_none());
        assert!(permeability_from_water_mass(1.0, WT, WMAX, -1.0).is_none());
    }
}