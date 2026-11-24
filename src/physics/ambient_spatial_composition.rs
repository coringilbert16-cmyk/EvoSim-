//! Energy and flux composition for the uniform ambient carrier plus a
//! propagating scalar-wave disturbance.
//!
//! These identities are the production-facing form of the already validated
//! Phase 3B.3 composition calculations. The ambient carrier is spatially
//! uniform; the disturbance gradient is therefore the gradient of the total
//! displacement. The restoring-energy term belongs to the ambient oscillator
//! degree of freedom, while the cross-energy and cross-flux terms are retained.

use crate::world_generated_ambient_mode::AmbientModeSample;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CompositionError {
    InvalidParameter,
    InvalidState,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisturbanceSample {
    pub(crate) displacement: f64,
    pub(crate) velocity: f64,
    pub(crate) gradient_x: f64,
    pub(crate) gradient_y: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ComposedFieldSample {
    pub(crate) displacement: f64,
    pub(crate) velocity: f64,
    pub(crate) energy_density: f64,
    pub(crate) disturbance_energy_density: f64,
    pub(crate) ambient_oscillator_energy_density: f64,
    pub(crate) cross_energy_density: f64,
    pub(crate) flux_x: f64,
    pub(crate) flux_y: f64,
    /// Signed world-drive power density for the additive field representation.
    pub(crate) world_drive_power_density: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AmbientSpatialComposition {
    field_normalization: f64,
    propagation_speed: f64,
}

impl AmbientSpatialComposition {
    pub(crate) fn new(
        field_normalization: f64,
        propagation_speed: f64,
    ) -> Result<Self, CompositionError> {
        if !field_normalization.is_finite()
            || field_normalization <= 0.0
            || !propagation_speed.is_finite()
            || propagation_speed <= 0.0
        {
            return Err(CompositionError::InvalidParameter);
        }
        Ok(Self {
            field_normalization,
            propagation_speed,
        })
    }

    pub(crate) fn compose(
        self,
        ambient: AmbientModeSample,
        disturbance: DisturbanceSample,
    ) -> Result<ComposedFieldSample, CompositionError> {
        let inputs = [
            ambient.carrier_value,
            ambient.carrier_velocity,
            ambient.carrier_acceleration,
            ambient.angular_frequency,
            disturbance.displacement,
            disturbance.velocity,
            disturbance.gradient_x,
            disturbance.gradient_y,
        ];
        if inputs.iter().any(|value| !value.is_finite()) || ambient.angular_frequency <= 0.0 {
            return Err(CompositionError::InvalidState);
        }

        let velocity = ambient.carrier_velocity + disturbance.velocity;
        let displacement = ambient.carrier_value + disturbance.displacement;
        let gradient_squared = disturbance.gradient_x.powi(2) + disturbance.gradient_y.powi(2);
        let speed_squared = self.propagation_speed.powi(2);
        let normalization = self.field_normalization;

        let disturbance_energy_density =
            0.5 * normalization * (disturbance.velocity.powi(2) + speed_squared * gradient_squared);
        let ambient_oscillator_energy_density = 0.5
            * normalization
            * (ambient.carrier_velocity.powi(2)
                + ambient.angular_frequency.powi(2) * ambient.carrier_value.powi(2));
        let cross_energy_density = normalization * ambient.carrier_velocity * disturbance.velocity;
        let energy_density =
            disturbance_energy_density + ambient_oscillator_energy_density + cross_energy_density;

        let flux_scale = -normalization * speed_squared * velocity;
        let flux_x = flux_scale * disturbance.gradient_x;
        let flux_y = flux_scale * disturbance.gradient_y;
        let world_drive_power_density = normalization
            * (ambient.carrier_acceleration * velocity
                + ambient.angular_frequency.powi(2)
                    * ambient.carrier_value
                    * ambient.carrier_velocity);

        if [
            displacement,
            velocity,
            gradient_squared,
            disturbance_energy_density,
            ambient_oscillator_energy_density,
            cross_energy_density,
            energy_density,
            flux_x,
            flux_y,
            world_drive_power_density,
        ]
        .iter()
        .any(|value| !value.is_finite())
            || energy_density < 0.0
        {
            return Err(CompositionError::InvalidState);
        }

        Ok(ComposedFieldSample {
            displacement,
            velocity,
            energy_density,
            disturbance_energy_density,
            ambient_oscillator_energy_density,
            cross_energy_density,
            flux_x,
            flux_y,
            world_drive_power_density,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_generated_ambient_mode::{AmbientModeConfig, WorldGeneratedAmbientMode};

    fn ambient_sample() -> AmbientModeSample {
        let config = AmbientModeConfig::new(1.0, 1.0, 0.0, 0.0).unwrap();
        WorldGeneratedAmbientMode::new(config, 317)
            .unwrap()
            .sample()
    }

    #[test]
    fn combined_energy_retains_the_cross_term() {
        let ambient = ambient_sample();
        let disturbance = DisturbanceSample {
            displacement: 0.25,
            velocity: -0.37,
            gradient_x: 0.21,
            gradient_y: -0.13,
        };
        let composition = AmbientSpatialComposition::new(1.7, 1.2).unwrap();
        let sample = composition.compose(ambient, disturbance).unwrap();

        let expected_cross = 1.7 * ambient.carrier_velocity * disturbance.velocity;
        assert!((sample.cross_energy_density - expected_cross).abs() < 1.0e-12);
        assert!(
            (sample.energy_density
                - sample.disturbance_energy_density
                - sample.ambient_oscillator_energy_density
                - expected_cross)
                .abs()
                < 1.0e-12
        );
    }

    #[test]
    fn combined_flux_retains_the_ambient_cross_flux() {
        let ambient = ambient_sample();
        let disturbance = DisturbanceSample {
            displacement: 0.0,
            velocity: 0.42,
            gradient_x: -0.18,
            gradient_y: 0.31,
        };
        let normalization = 2.0;
        let speed = 1.3;
        let sample = AmbientSpatialComposition::new(normalization, speed)
            .unwrap()
            .compose(ambient, disturbance)
            .unwrap();

        let expected_x = -normalization
            * speed.powi(2)
            * (disturbance.velocity + ambient.carrier_velocity)
            * disturbance.gradient_x;
        let expected_y = -normalization
            * speed.powi(2)
            * (disturbance.velocity + ambient.carrier_velocity)
            * disturbance.gradient_y;
        assert!((sample.flux_x - expected_x).abs() < 1.0e-12);
        assert!((sample.flux_y - expected_y).abs() < 1.0e-12);
    }

    #[test]
    fn world_drive_power_includes_disturbance_work() {
        let ambient = ambient_sample();
        let disturbance = DisturbanceSample {
            displacement: 0.0,
            velocity: 0.6,
            gradient_x: 0.0,
            gradient_y: 0.0,
        };
        let normalization = 1.4;
        let sample = AmbientSpatialComposition::new(normalization, 1.0)
            .unwrap()
            .compose(ambient, disturbance)
            .unwrap();

        let expected = normalization
            * (ambient.carrier_acceleration * (ambient.carrier_velocity + disturbance.velocity)
                + ambient.angular_frequency.powi(2)
                    * ambient.carrier_value
                    * ambient.carrier_velocity);
        assert!((sample.world_drive_power_density - expected).abs() < 1.0e-12);
    }

    #[test]
    fn invalid_field_parameters_are_rejected() {
        assert_eq!(
            AmbientSpatialComposition::new(0.0, 1.0),
            Err(CompositionError::InvalidParameter)
        );
        assert_eq!(
            AmbientSpatialComposition::new(1.0, f64::NAN),
            Err(CompositionError::InvalidParameter)
        );
    }
}
