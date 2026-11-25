//! Persistent live spatial resonance field and its single-ledger accounting adapter.
//!
//! The disturbance is an independent wave field. The uniform ambient oscillator
//! remains owned by WorldGeneratedAmbientMode; this adapter records only the
//! disturbance energy and ambient/disturbance cross-energy so ambient inventory
//! is never counted twice.

use crate::adaptive_spatial_grid::{AdaptiveGridError, AdaptiveSpatialGrid, AdaptiveWaveField};
use crate::ambient_spatial_composition::{
    AmbientSpatialComposition, ComposedFieldSample, CompositionError, DisturbanceSample,
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
        Self::new_with_window_cells(
            world_width,
            world_height,
            ambient_sample,
            INITIAL_WINDOW_BASE_CELLS,
        )
    }

    #[cfg(test)]
    pub(crate) fn new_with_test_window(
        world_width: f64,
        world_height: f64,
        ambient_sample: AmbientModeSample,
        window_cells: u32,
    ) -> Result<Self, LiveResonanceFieldError> {
        Self::new_with_window_cells(world_width, world_height, ambient_sample, window_cells)
    }

    fn new_with_window_cells(
        world_width: f64,
        world_height: f64,
        ambient_sample: AmbientModeSample,
        window_cells: u32,
    ) -> Result<Self, LiveResonanceFieldError> {
        if !world_width.is_finite()
            || world_width <= 0.0
            || !world_height.is_finite()
            || world_height <= 0.0
            || window_cells < 4
        {
            return Err(LiveResonanceFieldError::InvalidWorldDimensions);
        }
        // The disturbance window begins around the modeled world's center.
        // Its zero exterior is implicit; the production window expands before
        // any nonzero disturbance can reach its computational edge.
        let half_width = 0.5 * window_cells as f64 * BASE_SPACING;
        let origin_x = 0.5 * world_width - half_width;
        let origin_y = 0.5 * world_height - half_width;
        let grid = AdaptiveSpatialGrid::new_at_origin(
            window_cells,
            window_cells,
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

    pub(crate) fn substep_time_step(&self) -> f64 {
        self.wave.time_step()
    }

    pub(crate) fn set_initial_ambient_sample(
        &mut self,
        ambient_sample: AmbientModeSample,
    ) -> Result<(), LiveResonanceFieldError> {
        if ambient_sample.field_normalization.to_bits() != FIELD_NORMALIZATION.to_bits() {
            return Err(LiveResonanceFieldError::InvalidAccountingState);
        }
        let integrals = self.wave.grid().integrated_state();
        let disturbance_velocity = (integrals.current_displacement
            - integrals.previous_displacement)
            / self.wave.time_step();
        let disturbance_energy = self.wave.energy();
        let cross_energy = FIELD_NORMALIZATION * ambient_sample.carrier_velocity * disturbance_velocity;
        let total_energy = disturbance_energy + cross_energy;
        if !total_energy.is_finite() || disturbance_energy < 0.0 {
            return Err(LiveResonanceFieldError::InvalidAccountingState);
        }
        self.ambient_sample = ambient_sample;
        self.energy_state.disturbance_energy = disturbance_energy;
        self.energy_state.cross_energy = cross_energy;
        self.energy_state.total_energy = total_energy;
        self.initial_total_energy = total_energy;
        Ok(())
    }

    /// Expands the zero-disturbance exterior into the computational window
    /// before a caller establishes an initial condition at an arbitrary point.
    pub(crate) fn ensure_position_covered(
        &mut self,
        x: f64,
        y: f64,
    ) -> Result<bool, AdaptiveGridError> {
        self.wave
            .ensure_position_covered(x, y, 4.0, EXPANSION_PADDING_BASE_CELLS)
    }

    pub(crate) fn sample(
        &self,
        x: f64,
        y: f64,
    ) -> Result<Option<ComposedFieldSample>, CompositionError> {
        if !x.is_finite() || !y.is_finite() {
            return Err(CompositionError::InvalidParameter);
        }
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

    #[cfg(test)]
    pub(crate) fn set_initial_cell_state_for_test(
        &mut self,
        key: crate::adaptive_spatial_grid::CellKey,
        previous: f64,
        current: f64,
    ) {
        self.wave
            .set_initial_state(
                key,
                crate::adaptive_spatial_grid::AdaptiveCell { previous, current },
            )
            .unwrap();
        let integrals = self.wave.grid().integrated_state();
        let disturbance_velocity = (integrals.current_displacement
            - integrals.previous_displacement)
            / self.wave.time_step();
        let disturbance_energy = self.wave.energy();
        let cross_energy =
            FIELD_NORMALIZATION * self.ambient_sample.carrier_velocity * disturbance_velocity;
        let total_energy = disturbance_energy + cross_energy;
        self.energy_state = LiveFieldEnergyState {
            disturbance_energy,
            cross_energy,
            total_energy,
            ..LiveFieldEnergyState::default()
        };
        self.initial_total_energy = total_energy;
    }

    /// Seeds a resolved standing-wave mode that satisfies the fixed-zero
    /// computational boundary exactly. This lets long accounting tests use a
    /// compact window without mistaking boundary truncation for ambient work.
    #[cfg(test)]
    pub(crate) fn set_initial_standing_mode_for_test(&mut self, amplitude: f64) {
        assert!(amplitude.is_finite());
        let width = self.wave.grid().width();
        let height = self.wave.grid().height();
        assert!(width >= 4 && height >= 4);
        let keys: Vec<_> = self.wave.grid().leaves().map(|(key, _)| key).collect();
        for key in keys {
            let value = if key.level == 0
                && key.x > 0
                && key.y > 0
                && key.x + 1 < width
                && key.y + 1 < height
            {
                let x_phase = std::f64::consts::PI * key.x as f64 / (width - 1) as f64;
                let y_phase = std::f64::consts::PI * key.y as f64 / (height - 1) as f64;
                amplitude * x_phase.sin() * y_phase.sin()
            } else {
                0.0
            };
            self.wave
                .set_initial_state(
                    key,
                    crate::adaptive_spatial_grid::AdaptiveCell {
                        previous: value,
                        current: value,
                    },
                )
                .expect("standing-wave test state must satisfy fixed-zero boundaries");
        }
        let integrals = self.wave.grid().integrated_state();
        let disturbance_velocity = (integrals.current_displacement
            - integrals.previous_displacement)
            / self.wave.time_step();
        let disturbance_energy = self.wave.energy();
        let cross_energy =
            FIELD_NORMALIZATION * self.ambient_sample.carrier_velocity * disturbance_velocity;
        let total_energy = disturbance_energy + cross_energy;
        self.energy_state = LiveFieldEnergyState {
            disturbance_energy,
            cross_energy,
            total_energy,
            ..LiveFieldEnergyState::default()
        };
        self.initial_total_energy = total_energy;
    }

    /// Advances one logical tick. The adaptive solver performs its own CFL
    /// substeps and expands its computational window before the disturbance
    /// reaches the two-cell zero guard. The ambient inventory itself remains
    /// separately accounted by WorldGeneratedAmbientMode.
    pub(crate) fn advance_one_tick(
        &mut self,
        ambient_start: AmbientModeSample,
        ambient_midpoint: AmbientModeSample,
        ambient_end: AmbientModeSample,
    ) -> Result<LiveFieldEnergyState, LiveResonanceFieldError> {
        self.advance_one_tick_with_domain_policy(ambient_start, ambient_midpoint, ambient_end, true)
    }

    pub(crate) fn advance_one_tick_with_domain_policy(
        &mut self,
        ambient_start: AmbientModeSample,
        ambient_midpoint: AmbientModeSample,
        ambient_end: AmbientModeSample,
        expand_domain: bool,
    ) -> Result<LiveFieldEnergyState, LiveResonanceFieldError> {
        if ambient_start.field_normalization.to_bits() != FIELD_NORMALIZATION.to_bits()
            || ambient_midpoint.field_normalization.to_bits() != FIELD_NORMALIZATION.to_bits()
            || ambient_end.field_normalization.to_bits() != FIELD_NORMALIZATION.to_bits()
        {
            return Err(LiveResonanceFieldError::InvalidAccountingState);
        }

        let before_integrals = self.wave.grid().integrated_state();
        let time_step = self.wave.time_step();
        let before_disturbance_velocity = (before_integrals.current_displacement
            - before_integrals.previous_displacement)
            / time_step;

        if expand_domain {
            self.wave.advance_ticks_with_domain_expansion(
                1,
                EXPANSION_TRIGGER_MARGIN,
                EXPANSION_PADDING_BASE_CELLS,
            )?;
        } else {
            self.wave.advance_ticks(1)?;
        }

        let after_integrals = self.wave.grid().integrated_state();
        let after_disturbance_velocity = (after_integrals.current_displacement
            - after_integrals.previous_displacement)
            / time_step;
        let ambient_end_velocity = ambient_end.carrier_velocity;
        let cross_energy =
            FIELD_NORMALIZATION * ambient_end_velocity * after_disturbance_velocity;
        let midpoint_disturbance_velocity =
            0.5 * (before_disturbance_velocity + after_disturbance_velocity);
        // Simpson integration is aligned to the actual leapfrog velocity
        // timestamps: t - dt/2, t + 1/2 - dt/2, and t + 1 - dt/2.
        // The ambient samples are supplied at those same three times.
        let cross_drive_work = FIELD_NORMALIZATION
            * (ambient_start.carrier_acceleration * before_disturbance_velocity
                + 4.0 * ambient_midpoint.carrier_acceleration * midpoint_disturbance_velocity
                + ambient_end.carrier_acceleration * after_disturbance_velocity)
            / 6.0;
        let disturbance_energy = self.wave.energy();
        let total_energy = disturbance_energy + cross_energy;
        let cumulative_world_work = self.energy_state.cumulative_world_work + cross_drive_work;
        let accounting_residual = total_energy - self.initial_total_energy - cumulative_world_work;

        if [
            before_disturbance_velocity,
            after_disturbance_velocity,
            ambient_end_velocity,
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
        self.ambient_sample = ambient_end;
        Ok(self.energy_state)
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
        let initial_midpoint = ambient.sample_midpoint_before_current().unwrap();
        let field = LiveResonanceField::new(250.0, 250.0, initial_midpoint).unwrap();
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
        let half_substep = 0.5 * field.substep_time_step();
        let initial_sample = ambient.sample_at_offset(-half_substep).unwrap();
        field.set_initial_ambient_sample(initial_sample).unwrap();
        ambient.advance_one_tick().unwrap();
        let start = ambient.sample_at_offset(-1.0 - half_substep).unwrap();
        let midpoint = ambient.sample_at_offset(-0.5 - half_substep).unwrap();
        let end = ambient.sample_at_offset(-half_substep).unwrap();

        let state = field.advance_one_tick(start, midpoint, end).unwrap();
        assert_eq!(state.disturbance_energy, 0.0);
        assert_eq!(state.cross_energy, 0.0);
        assert_eq!(state.total_energy, 0.0);
        assert_eq!(state.cumulative_world_work, 0.0);
        assert_eq!(state.cumulative_computational_outflow, 0.0);
        assert_eq!(state.accounting_residual, 0.0);
    }

    #[test]
    fn live_field_can_expand_to_cover_an_arbitrary_source_position() {
        let ambient = ambient();
        let initial_midpoint = ambient.sample_midpoint_before_current().unwrap();
        let mut field = LiveResonanceField::new(250.0, 250.0, initial_midpoint).unwrap();
        assert!(field.ensure_position_covered(150.0, 100.0).unwrap());
        assert!(field.wave.sample(150.0, 100.0).is_some());
        assert!(field.wave.grid().origin().0 < 109.0);
        assert!(field.wave.grid().origin().1 < 109.0);
    }

    #[test]
    fn nonzero_disturbance_reconciles_cross_energy_across_domain_expansion() {
        let config = AmbientModeConfig::new(250.0 * 250.0, 1.0, 0.0, 0.0).unwrap();
        let mut ambient = WorldGeneratedAmbientMode::new(config, 100).unwrap();
        let mut field = LiveResonanceField::new(250.0, 250.0, ambient.sample()).unwrap();
        let half_substep = 0.5 * field.substep_time_step();
        let initial_sample = ambient.sample_at_offset(-half_substep).unwrap();
        field.set_initial_ambient_sample(initial_sample).unwrap();
        field.set_initial_cell_state_for_test(
            crate::adaptive_spatial_grid::CellKey {
                level: 0,
                x: 16,
                y: 16,
            },
            0.0,
            0.5,
        );

        let mut maximum_residual: f64 = 0.0;
        let mut final_state = None;
        for _ in 0..20 {
            ambient.advance_one_tick().unwrap();
            let ambient_start = ambient.sample_at_offset(-1.0 - half_substep).unwrap();
            let ambient_midpoint = ambient.sample_at_offset(-0.5 - half_substep).unwrap();
            let ambient_end = ambient.sample_at_offset(-half_substep).unwrap();
            let state = field
                .advance_one_tick(ambient_start, ambient_midpoint, ambient_end)
                .unwrap();
            maximum_residual = maximum_residual.max(state.accounting_residual.abs());
            final_state = Some(state);
        }
        let state = final_state.expect("the test advances twenty logical ticks");

        assert!(state.disturbance_energy > 0.0);
        assert!(state.cross_energy.abs() > 0.0);
        assert!(state.cumulative_world_work.abs() > 0.0);
        assert!(field.wave.grid().width() > INITIAL_WINDOW_BASE_CELLS);
        assert!(
            maximum_residual < 1.0e-5 * state.total_energy.abs().max(1.0),
            "maximum nonzero composed-field balance residual: {maximum_residual}"
        );
    }

    #[test]
    fn field_queries_reject_non_finite_world_positions() {
        let ambient = ambient();
        let field = LiveResonanceField::new(250.0, 250.0, ambient.sample()).unwrap();

        assert_eq!(
            field.sample(f64::NAN, 0.0),
            Err(CompositionError::InvalidParameter)
        );
        assert_eq!(
            field.sample(0.0, f64::INFINITY),
            Err(CompositionError::InvalidParameter)
        );
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
