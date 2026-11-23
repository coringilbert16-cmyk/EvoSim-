//! Geometry-only validation for the compact-support radial-cosine hotspot profile.
//!
//! This module defines an incident-power-density candidate only. It does not
//! convert incident power into wave forcing, inject a field, or settle energy.

use std::f64::consts::PI;

const GAUSS_NODES: [f64; 8] = [
    -0.960_289_856_497_536_3, -0.796_666_477_413_626_7,
    -0.525_532_409_916_329_0, -0.183_434_642_495_649_8,
     0.183_434_642_495_649_8,  0.525_532_409_916_329_0,
     0.796_666_477_413_626_7,  0.960_289_856_497_536_3,
];
const GAUSS_WEIGHTS: [f64; 8] = [
    0.101_228_536_290_376_3, 0.222_381_034_453_374_5,
    0.313_706_645_877_887_3, 0.362_683_783_378_362_0,
    0.362_683_783_378_362_0, 0.313_706_645_877_887_3,
    0.222_381_034_453_374_5, 0.101_228_536_290_376_3,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HotspotProfileError {
    InvalidRadius,
    InvalidCoordinate,
    InvalidRectangle,
    InvalidGridSpacing,
    GridIndexOverflow,
}

/// Normalized compact-support radial-cosine incident-power profile.
/// Density has units L^-2; multiplying by total power P gives E/(L^2*tick).
/// This is not a field amplitude or a wave-forcing law.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RadialCosineProfile {
    radius: f64,
    normalization: f64,
}

impl RadialCosineProfile {
    pub(crate) fn new(radius: f64) -> Result<Self, HotspotProfileError> {
        if !radius.is_finite() || radius <= 0.0 {
            return Err(HotspotProfileError::InvalidRadius);
        }
        let radius_squared = radius * radius;
        let denominator = 2.0 * PI * radius_squared * (0.5 - 2.0 / (PI * PI));
        let normalization = 1.0 / denominator;
        if !radius_squared.is_finite()
            || !denominator.is_finite()
            || denominator <= 0.0
            || !normalization.is_finite()
        {
            return Err(HotspotProfileError::InvalidRadius);
        }
        Ok(Self { radius, normalization })
    }

    pub(crate) fn radius(self) -> f64 {
        self.radius
    }

    pub(crate) fn density(self, x: f64, y: f64) -> Result<f64, HotspotProfileError> {
        if !x.is_finite() || !y.is_finite() {
            return Err(HotspotProfileError::InvalidCoordinate);
        }
        let radius = x.hypot(y);
        if radius >= self.radius {
            return Ok(0.0);
        }
        Ok(self.normalization * (1.0 + (PI * radius / self.radius).cos()))
    }

    /// Exact fraction of total profile power inside a concentric radius.
    pub(crate) fn fraction_within(self, radius: f64) -> Result<f64, HotspotProfileError> {
        if !radius.is_finite() {
            return Err(HotspotProfileError::InvalidCoordinate);
        }
        if radius <= 0.0 {
            return Ok(0.0);
        }
        if radius >= self.radius {
            return Ok(1.0);
        }
        let x = radius / self.radius;
        let angle = PI * x;
        let numerator = 0.5 * x * x + x * angle.sin() / PI
            + (angle.cos() - 1.0) / (PI * PI);
        Ok(numerator / (0.5 - 2.0 / (PI * PI)))
    }

    /// Deterministic tensor-product 8-point Gauss-Legendre rectangle integral.
    /// Rectangle coordinates are relative to the source center.
    pub(crate) fn integrate_rectangle(
        self, x_min: f64, y_min: f64, x_max: f64, y_max: f64,
    ) -> Result<f64, HotspotProfileError> {
        if ![x_min, y_min, x_max, y_max].iter().all(|v| v.is_finite()) {
            return Err(HotspotProfileError::InvalidCoordinate);
        }
        if x_min >= x_max || y_min >= y_max {
            return Err(HotspotProfileError::InvalidRectangle);
        }
        let half_x = 0.5 * (x_max - x_min);
        let half_y = 0.5 * (y_max - y_min);
        let center_x = 0.5 * (x_max + x_min);
        let center_y = 0.5 * (y_max + y_min);
        let mut sum = 0.0;
        for ix in 0..8 {
            let x = center_x + half_x * GAUSS_NODES[ix];
            for iy in 0..8 {
                let y = center_y + half_y * GAUSS_NODES[iy];
                sum += GAUSS_WEIGHTS[ix] * GAUSS_WEIGHTS[iy] * self.density(x, y)?;
            }
        }
        let integral = sum * half_x * half_y;
        if !integral.is_finite() || integral < 0.0 {
            return Err(HotspotProfileError::InvalidCoordinate);
        }
        Ok(integral)
    }

    /// Integrates over physical Cartesian cell areas. Source offsets are the
    /// source-center position relative to the grid origin.
    pub(crate) fn integrate_cartesian_grid(
        self, spacing: f64, source_offset_x: f64, source_offset_y: f64,
    ) -> Result<f64, HotspotProfileError> {
        if !spacing.is_finite() || spacing <= 0.0 {
            return Err(HotspotProfileError::InvalidGridSpacing);
        }
        if !source_offset_x.is_finite() || !source_offset_y.is_finite() {
            return Err(HotspotProfileError::InvalidCoordinate);
        }
        let first_x = ((source_offset_x - self.radius) / spacing).floor();
        let last_x = ((source_offset_x + self.radius) / spacing).ceil();
        let first_y = ((source_offset_y - self.radius) / spacing).floor();
        let last_y = ((source_offset_y + self.radius) / spacing).ceil();
        if [first_x, last_x, first_y, last_y].iter()
            .any(|v| *v < isize::MIN as f64 || *v > isize::MAX as f64)
        {
            return Err(HotspotProfileError::GridIndexOverflow);
        }
        let mut total = 0.0;
        for iy in first_y as isize..last_y as isize {
            let y_min = iy as f64 * spacing - source_offset_y;
            let y_max = y_min + spacing;
            for ix in first_x as isize..last_x as isize {
                let x_min = ix as f64 * spacing - source_offset_x;
                total += self.integrate_rectangle(x_min, y_min, x_min + spacing, y_max)?;
            }
        }
        if !total.is_finite() {
            return Err(HotspotProfileError::InvalidCoordinate);
        }
        Ok(total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!((actual - expected).abs() <= tolerance,
            "actual={actual:.16e}, expected={expected:.16e}, tolerance={tolerance:.3e}");
    }

    #[test]
    fn analytic_normalization_and_radial_fraction_are_correct() {
        for radius in [0.125, 0.5, 1.0, 7.25, 100.0] {
            let profile = RadialCosineProfile::new(radius).unwrap();
            close(profile.fraction_within(radius).unwrap(), 1.0, 0.0);
            close(profile.fraction_within(0.0).unwrap(), 0.0, 0.0);
            let x: f64 = 0.5;
            let angle = PI * x;
            let expected = (0.5 * x * x + x * angle.sin() / PI
                + (angle.cos() - 1.0) / (PI * PI)) / (0.5 - 2.0 / (PI * PI));
            close(profile.fraction_within(radius * 0.5).unwrap(), expected, 1.0e-14);
        }
    }

    #[test]
    fn density_is_radial_finite_and_compactly_supported() {
        let profile = RadialCosineProfile::new(3.0).unwrap();
        let center = profile.density(0.0, 0.0).unwrap();
        assert!(center.is_finite() && center > 0.0);
        close(profile.density(1.0, 0.0).unwrap(), profile.density(0.0, 1.0).unwrap(), 0.0);
        close(profile.density(3.0, 0.0).unwrap(), 0.0, 0.0);
        close(profile.density(4.0, -2.0).unwrap(), 0.0, 0.0);
        assert!(RadialCosineProfile::new(0.0).is_err());
        assert!(RadialCosineProfile::new(f64::INFINITY).is_err());
        assert!(profile.density(f64::NAN, 0.0).is_err());
    }

    #[test]
    fn concentric_power_fraction_is_monotonic_and_bounded() {
        let profile = RadialCosineProfile::new(2.0).unwrap();
        let fractions: Vec<f64> = (0..=100).map(|i|
            profile.fraction_within(2.0 * i as f64 / 100.0).unwrap()).collect();
        assert!(fractions.windows(2).all(|pair| pair[0] <= pair[1]));
        assert!(fractions.iter().all(|v| (0.0..=1.0).contains(v)));
        close(*fractions.last().unwrap(), 1.0, 0.0);
    }

    #[test]
    fn physical_area_integral_converges_under_grid_refinement() {
        let profile = RadialCosineProfile::new(4.0).unwrap();
        let coarse = profile.integrate_cartesian_grid(2.0, 0.0, 0.0).unwrap();
        let medium = profile.integrate_cartesian_grid(1.0, 0.0, 0.0).unwrap();
        let fine = profile.integrate_cartesian_grid(0.5, 0.0, 0.0).unwrap();
        let errors = [(coarse - 1.0).abs(), (medium - 1.0).abs(), (fine - 1.0).abs()];
        assert!(errors[0] > errors[1], "errors={errors:?}");
        assert!(errors[1] > errors[2], "errors={errors:?}");
        assert!(errors[2] < 2.0e-7, "fine integral={fine}");
    }

    #[test]
    fn subcell_translation_and_grid_alignment_preserve_total_power() {
        let profile = RadialCosineProfile::new(3.0).unwrap();
        let offsets = [(0.0, 0.0), (0.25, 0.0), (0.0, 0.25),
            (0.25, 0.25), (0.499, 0.137), (-0.31, 0.42)];
        let values: Vec<f64> = offsets.iter().map(|(x, y)|
            profile.integrate_cartesian_grid(0.5, *x, *y).unwrap()).collect();
        for value in &values { close(*value, 1.0, 2.0e-7); }
        let min = values.iter().copied().fold(f64::INFINITY, f64::min);
        let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        assert!(max - min < 2.0e-7, "integrals={values:?}");
    }

    #[test]
    fn total_power_is_independent_of_profile_radius() {
        let power = 17.5;
        for radius in [0.25, 1.0, 4.0, 20.0] {
            let profile = RadialCosineProfile::new(radius).unwrap();
            let integrated = power * profile.integrate_cartesian_grid(radius / 8.0, 0.0, 0.0).unwrap();
            close(integrated, power, 2.0e-7 * power);
        }
    }

    #[test]
    fn rectangle_integration_rejects_invalid_geometry() {
        let profile = RadialCosineProfile::new(1.0).unwrap();
        assert!(profile.integrate_rectangle(0.0, 0.0, 0.0, 1.0).is_err());
        assert!(profile.integrate_rectangle(0.0, 0.0, 1.0, f64::INFINITY).is_err());
        assert!(profile.integrate_cartesian_grid(0.0, 0.0, 0.0).is_err());
    }
}
