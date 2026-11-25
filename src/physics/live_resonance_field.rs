//! Persistent live spatial resonance field and its single-ledger accounting adapter.
//!
//! The disturbance is an independent wave field. The uniform ambient oscillator
//! remains owned by WorldGeneratedAmbientMode; this adapter records only the
//! disturbance energy and ambient/disturbance cross-energy so ambient inventory
//! is never counted twice.

use crate::ambient_spatial_composition::{
    AmbientSpatialComposition, ComposedFieldSample, CompositionError, DisturbanceSample,
};
use crate::adaptive_spatial_grid::{
    AdaptiveGridError, AdaptiveSpatialGrid, AdaptiveWaveField,
};
use crate::world_generated_ambient_mode::AmbientModeSample;

const PROPAGATION_SPEED: f64 = 1.0;
const FIELD_NORMALIZATION: f64 = 1.0;
const BASE_SPACING: f64 = 1.0;
const MAX_REFINEMENT_LEVEL: u8 = 2;
const INITIAL_WINDOW_BASE_CELLS: u32 = 32;
const EXPANSION_TRIGGER_MARGIN: f64 = 4.0;
const EXPANSION_PADDING_BASE_CELLS: u32 = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LiveResonanceFieldError {
    InvalidWorldDimensions,
    AdaptiveGrid(AdaptiveGridError),
    InvalidAccountingState,
}

impl From<AdaptiveGridError> for LiveResonanceFieldError {
    fn from(value: AdaptiveGridError) -> Self {
        Self::AdaptiveGrid(value)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct LiveFieldEnergyState {
    pub(crate) disturbance_energy: f64,
    pub(crate) cross_energy: f64,
    pub(crate) total_energy: f64,
    pub(crate) cumulative_world_work: f64,
    pub(crate) cumulative_computational_outflow: f64,
    pub(crate) accounting_residual: f64,
}

#[derive(Clone, Debug)]
pub(crate) struct LiveResonanceField {
    wave: AdaptiveWaveField,
    composition: AmbientSpatialComposition,
    energy_state: LiveFieldEnergyState,
    initial_total_energy: f64,
    ambient_sample: AmbientModeSample,
}

impl LiveResonanceField {
    pub(crate) fn new(
        world_width: f64,
        world_height: f64,
        ambient_sample: AmbientModeSample,
    ) -> Result<Self, LiveResonanceFieldError> {
        if !world_width.is_finite()
            || world_width <= 0.0
            || !world_height.is_finite()
            || world_height <= 0.0
        {
            return Err(LiveResonanceFieldError::InvalidWorldDimensions);
        }
        // The disturbance window begins around the modeled world's center.
        // Its zero exterior is implicit; the window expands before any
        // nonzero disturbance can reach its computational edge.
        let half_width = 0.5 * INITIAL_WINDOW_BASE_CELLS as f64 * BASE_SPACING;
        let origin_x = 0.5 * world_width - half_width;
        let origin_y = 0.5 * world_height - half_width;
        let grid = AdaptiveSpatialGrid::new_at_origin(
            INITIAL_WINDOW_BASE_CELLS,
            INITIAL_WINDOW_BASE_CELLS,
            BASE_SPACING,
            MAX_REFINEMENT_LEVEL,
            origin_x,
            origin_y,
        )?;
        let wave = AdaptiveWaveField::new(grid, PROPAGATION_SPEED, FIELD_NORMALIZATION)?;
        let composition = AmbientSpatialComposition::new(FIELD_NORMALIZATION, PROPAGATION_SPEED)
            .map_err(|_| LiveResonanceFieldError::InvalidAccountingState)?;
        if ambient_sample.field_normalization.to_bits() != FIELD_NORMALIZATION.to_bits() {
            return Err(LiveResonanceFieldError::InvalidAccountingState);
        }
        let energy_state = LiveFieldEnergyState {
            disturbance_energy: wave.energy(),
            ..LiveFieldEnergyState::default()
        };
        let initial_total_energy = energy_state.total_energy;
        Ok(Self {
            wave,
            composition,
            energy_state,
            initial_total_energy,
            ambient_sample,
        })
    }

    pub(crate) fn energy_state(&self) -> LiveFieldEnergyState {
        self.energy_state
    }

    pub(crate) fn initial_total_energy(&self) -> f64 {
        self.initial_total_energy
    }

    pub(crate) fn sample(
        &self,
        x: f64,
        y: f64,
    ) -> Result<Option<ComposedFieldSample>, CompositionError> {
        let disturbance = match self.wave.sample_kinematics(x, y) {
            Some(sample) => DisturbanceSample {
                displacement: sample.displacement,
                velocity: sample.velocity,
                gradient_x: sample.gradient_x,
                gradient_y: sample.gradient_y,
            },
            // Outside the disturbance window the exterior state is zero.
            // A failed reconstruction inside the window is not silently treated
            // as zero; callers must handle the unavailable stencil explicitly.
            None if self.wave.grid().sample(x, y).is_none() => DisturbanceSample {
                displacement: 0.0,
                velocity: 0.0,
                gradient_x: 0.0,
                gradient_y: 0.0,
            },
            None => return Err(CompositionError::InvalidState),
        };
        self.composition
            .compose(self.ambient_sample, disturbance)
            .map(Some)
    }

    /// Advances one logical tick. The adaptive solver performs its own CFL
    /// substeps and expands its computational window before the disturbance
    /// reaches the two-cell zero guard. The ambient inventory itself remains
    /// separately accounted by WorldGeneratedAmbientMode.
    pub(crate) fn advance_one_tick(
        &mut self,
        ambient_before: AmbientModeSample,
        ambient_after: AmbientModeSample,
    ) -> Result<LiveFieldEnergyState, LiveResonanceFieldError> {
        if ambient_before.field_normalization.to_bits() != FIELD_NORMALIZATION.to_bits()
            || ambient_after.field_normalization.to_bits() != FIELD_NORMALIZATION.to_bits()
        {
            return Err(LiveResonanceFieldError::InvalidAccountingState);
        }

        let before_integrals = self.wave.grid().integrated_state();
        let time_step = self.wave.time_step();
        let before_disturbance_velocity = (before_integrals.current_displacement
            - before_integrals.previous_displacement)
            / time_step;

        self.wave.advance_ticks_with_domain_expansion(
            1,
            EXPANSION_TRIGGER_MARGIN,
            EXPANSION_PADDING_BASE_CELLS,
        )?;

        let after_integrals = self.wave.grid().integrated_state();
        let after_disturbance_velocity = (after_integrals.current_displacement
            - after_integrals.previous_displacement)
            / time_step;
        let ambient_midpoint_velocity =
            0.5 * (ambient_before.carrier_velocity + ambient_after.carrier_velocity);
        let ambient_midpoint_sample = midpoint_ambient_sample(ambient_before, ambient_after);
        let cross_energy =
            FIELD_NORMALIZATION * ambient_midpoint_velocity * after_disturbance_velocity;
        let cross_drive_work = FIELD_NORMALIZATION
            * ambient_before.carrier_acceleration
            * 0.5
            * (before_disturbance_velocity + after_disturbance_velocity);
        let disturbance_energy = self.wave.energy();
        let total_energy = disturbance_energy + cross_energy;
        let cumulative_world_work = self.energy_state.cumulative_world_work + cross_drive_work;
        let accounting_residual = total_energy - self.initial_total_energy - cumulative_world_work;

        if [
            before_disturbance_velocity,
            after_disturbance_velocity,
            ambient_midpoint_velocity,
            cross_energy,
            cross_drive_work,
            disturbance_energy,
            total_energy,
            cumulative_world_work,
            accounting_residual,
        ]
        .iter()
        .any(|value| !value.is_finite())
            || disturbance_energy < 0.0
            || !cumulative_world_work.is_finite()
        {
            return Err(LiveResonanceFieldError::InvalidAccountingState);
        }

        self.energy_state = LiveFieldEnergyState {
            disturbance_energy,
            cross_energy,
            total_energy,
            cumulative_world_work,
            // No absorber or finite-edge escape is used in the live path.
            cumulative_computational_outflow: 0.0,
            accounting_residual,
        };
        self.ambient_sample = ambient_midpoint_sample;
        Ok(self.energy_state)
    }
}

fn midpoint_ambient_sample(
    before: AmbientModeSample,
    after: AmbientModeSample,
) -> AmbientModeSample {
    AmbientModeSample {
        tick: after.tick,
        energy_density: 0.5 * (before.energy_density + after.energy_density),
        total_energy: 0.5 * (before.total_energy + after.total_energy),
        amplitude: 0.5 * (before.amplitude + after.amplitude),
        amplitude_velocity: 0.5 * (before.amplitude_velocity + after.amplitude_velocity),
        field_normalization: before.field_normalization,
        angular_frequency: 0.5 * (before.angular_frequency + after.angular_frequency),
        carrier_phase_radians: 0.5 * (before.carrier_phase_radians + after.carrier_phase_radians),
        carrier_value: 0.5 * (before.carrier_value + after.carrier_value),
        carrier_velocity: 0.5 * (before.carrier_velocity + after.carrier_velocity),
        carrier_acceleration: 0.5 * (before.carrier_acceleration + after.carrier_acceleration),
        instantaneous_oscillator_energy_density: 0.5
            * (before.instantaneous_oscillator_energy_density
                + after.instantaneous_oscillator_energy_density),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_generated_ambient_mode::{AmbientModeConfig, WorldGeneratedAmbientMode};

    fn ambient() -> WorldGeneratedAmbientMode {
        let config = AmbientModeConfig::new(250.0 * 250.0, 1.0, 0.0, 0.0).unwrap();
        WorldGeneratedAmbientMode::new(config, 0).unwrap()
    }

    #[test]
    fn live_field_initializes_with_approved_scales_and_zero_disturbance() {
        let ambient = ambient();
        let field = LiveResonanceField::new(250.0, 250.0, ambient.sample()).unwrap();
        assert_eq!(field.energy_state().disturbance_energy, 0.0);
        assert_eq!(field.energy_state().cross_energy, 0.0);
        assert_eq!(field.energy_state().total_energy, 0.0);
        assert_eq!(field.energy_state().cumulative_computational_outflow, 0.0);
        assert_eq!(field.initial_total_energy(), 0.0);
    }

    #[test]
    fn live_field_advances_deterministically_without_inventing_energy() {
        let mut ambient = ambient();
        let mut field = LiveResonanceField::new(250.0, 250.0, ambient.sample()).unwrap();
        let before = ambient.sample();
        ambient.advance_one_tick().unwrap();
        let after = ambient.sample();

        let state = field.advance_one_tick(before, after).unwrap();
        assert_eq!(state.disturbance_energy, 0.0);
        assert_eq!(state.cross_energy, 0.0);
        assert_eq!(state.total_energy, 0.0);
        assert_eq!(state.cumulative_world_work, 0.0);
        assert_eq!(state.cumulative_computational_outflow, 0.0);
        assert_eq!(state.accounting_residual, 0.0);
    }

    #[test]
    fn outside_window_queries_use_zero_disturbance_over_ambient_field() {
        let ambient = ambient();
        let field = LiveResonanceField::new(250.0, 250.0, ambient.sample()).unwrap();
        let sample = field.sample(-1000.0, -1000.0).unwrap().unwrap();
        assert_eq!(sample.disturbance_energy_density, 0.0);
        assert_eq!(sample.cross_energy_density, 0.0);
        assert_eq!(
            sample.energy_density,
            sample.ambient_oscillator_energy_density
        );
    }
}
