//! Isolated reference model for the world-generated uniform ambient resonance mode.
//!
//! This module models only the spatially uniform ambient resonance component. Its
//! cycle-averaged stored energy density follows the approved daily cosine
//! envelope, while its carrier phase advances at 440 cycles per 24,000-tick
//! day. Positive changes are explicit world generation; negative changes are
//! explicit world withdrawal. No sun object, external oscillator, or second
//! energy reservoir is represented here.
//!
//! Local disturbance modes remain governed by the existing nondispersive
//! scalar wave equation. This module does not modify or replace that solver and
//! is not yet connected to production materials or chemistry.

use std::{f64::consts::TAU, sync::Arc};

pub(crate) const AMBIENT_DAY_TICKS: u64 = 24_000;
pub(crate) const AMBIENT_CARRIER_CYCLES_PER_DAY: u64 = 440;
pub(crate) const AMBIENT_MIN_ENERGY_DENSITY: f64 = 1.0;
pub(crate) const AMBIENT_MAX_ENERGY_DENSITY: f64 = 10.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AmbientModeError {
    InvalidParameter,
    InvalidState,
    CounterOverflow,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AmbientModeConfig {
    world_area: f64,
    field_normalization: f64,
    day_phase_offset_cycles: f64,
    carrier_phase_offset_cycles: f64,
}

impl AmbientModeConfig {
    /// Creates the reference ambient mode for a finite world-accounting area.
    ///
    /// world_area is an accounting footprint only, not a propagation boundary.
    /// field_normalization is the positive K in the oscillator relation
    /// e = K * omega^2 * amplitude^2 / 2.
    pub(crate) fn new(
        world_area: f64,
        field_normalization: f64,
        day_phase_offset_cycles: f64,
        carrier_phase_offset_cycles: f64,
    ) -> Result<Self, AmbientModeError> {
        let config = Self {
            world_area,
            field_normalization,
            day_phase_offset_cycles,
            carrier_phase_offset_cycles,
        };
        if !config.is_valid() {
            return Err(AmbientModeError::InvalidParameter);
        }
        Ok(config)
    }

    fn is_valid(self) -> bool {
        self.world_area.is_finite()
            && self.world_area > 0.0
            && self.field_normalization.is_finite()
            && self.field_normalization > 0.0
            && self.day_phase_offset_cycles.is_finite()
            && self.carrier_phase_offset_cycles.is_finite()
    }

    fn energy_density_at(self, tick: u64) -> f64 {
        let day_tick = tick % AMBIENT_DAY_TICKS;
        let day_cycles = day_tick as f64 / AMBIENT_DAY_TICKS as f64 + self.day_phase_offset_cycles;
        let envelope = 0.5 * (1.0 + (TAU * day_cycles).cos());
        AMBIENT_MIN_ENERGY_DENSITY
            + (AMBIENT_MAX_ENERGY_DENSITY - AMBIENT_MIN_ENERGY_DENSITY) * envelope
    }

    fn carrier_phase_at(self, tick: u64) -> f64 {
        let day_tick = tick % AMBIENT_DAY_TICKS;
        let carrier_cycles = day_tick as f64 * AMBIENT_CARRIER_CYCLES_PER_DAY as f64
            / AMBIENT_DAY_TICKS as f64
            + self.carrier_phase_offset_cycles;
        TAU * carrier_cycles.rem_euclid(1.0)
    }

    fn sample_at(
        self,
        tick: u64,
        trajectory: &AmbientModeTrajectory,
    ) -> Result<AmbientModeSample, AmbientModeError> {
        let energy_density = self.energy_density_at(tick);
        let total_energy = energy_density * self.world_area;
        let angular_frequency =
            TAU * AMBIENT_CARRIER_CYCLES_PER_DAY as f64 / AMBIENT_DAY_TICKS as f64;
        let day_tick = (tick % AMBIENT_DAY_TICKS) as usize;
        let amplitude = trajectory.amplitude[day_tick];
        let amplitude_velocity = trajectory.amplitude_velocity[day_tick];
        let amplitude_acceleration = trajectory.amplitude_acceleration[day_tick];
        let carrier_phase_radians = self.carrier_phase_at(tick);
        let cosine = carrier_phase_radians.cos();
        let sine = carrier_phase_radians.sin();
        let carrier_value = amplitude * cosine;
        let carrier_velocity = amplitude_velocity * cosine - amplitude * angular_frequency * sine;
        let carrier_acceleration = (amplitude_acceleration
            - amplitude * angular_frequency.powi(2))
            * cosine
            - 2.0 * amplitude_velocity * angular_frequency * sine;
        let instantaneous_oscillator_energy_density = 0.5
            * self.field_normalization
            * (carrier_velocity.powi(2) + angular_frequency.powi(2) * carrier_value.powi(2));

        if [
            energy_density,
            total_energy,
            angular_frequency,
            amplitude,
            carrier_phase_radians,
            carrier_value,
        ]
        .iter()
        .any(|value| !value.is_finite())
            || energy_density < AMBIENT_MIN_ENERGY_DENSITY
            || energy_density > AMBIENT_MAX_ENERGY_DENSITY
            || total_energy <= 0.0
            || !carrier_velocity.is_finite()
            || !carrier_acceleration.is_finite()
            || !instantaneous_oscillator_energy_density.is_finite()
            || amplitude <= 0.0
        {
            return Err(AmbientModeError::InvalidState);
        }

        Ok(AmbientModeSample {
            tick,
            energy_density,
            total_energy,
            amplitude,
            angular_frequency,
            carrier_phase_radians,
            carrier_value,
            carrier_velocity,
            carrier_acceleration,
            instantaneous_oscillator_energy_density,
        })
    }
}


/// Periodic amplitude trajectory whose carrier-averaged total oscillator energy
/// (including the slow-envelope kinetic term) matches the approved 1–10 schedule.
#[derive(Clone, Debug)]
struct AmbientModeTrajectory {
    amplitude: Vec<f64>,
    amplitude_velocity: Vec<f64>,
    amplitude_acceleration: Vec<f64>,
}

impl AmbientModeTrajectory {
    fn build(config: AmbientModeConfig) -> Result<Self, AmbientModeError> {
        let count = AMBIENT_DAY_TICKS as usize;
        let angular_frequency =
            TAU * AMBIENT_CARRIER_CYCLES_PER_DAY as f64 / AMBIENT_DAY_TICKS as f64;
        let denominator = config.field_normalization * angular_frequency.powi(2);
        let mut amplitude: Vec<f64> = (0..count)
            .map(|tick| {
                (2.0 * config.energy_density_at(tick as u64) / denominator).sqrt()
            })
            .collect();

        // Four periodic fixed-point corrections reproduce the reference
        // calculation: E = K*Omega^2*a^2/2 + K*(a')^2/4.
        for _ in 0..4 {
            let mut corrected = Vec::with_capacity(count);
            for tick in 0..count {
                let amplitude_velocity = Self::first_derivative(&amplitude, tick);
                let target = config.energy_density_at(tick as u64);
                let remaining = target
                    - 0.25 * config.field_normalization * amplitude_velocity.powi(2);
                if !remaining.is_finite() || remaining <= 0.0 {
                    return Err(AmbientModeError::InvalidState);
                }
                corrected.push((2.0 * remaining / denominator).sqrt());
            }
            amplitude = corrected;
        }

        let amplitude_velocity: Vec<f64> = (0..count)
            .map(|tick| Self::first_derivative(&amplitude, tick))
            .collect();
        let amplitude_acceleration: Vec<f64> = (0..count)
            .map(|tick| Self::second_derivative(&amplitude, tick))
            .collect();

        if amplitude
            .iter()
            .chain(amplitude_velocity.iter())
            .chain(amplitude_acceleration.iter())
            .any(|value| !value.is_finite())
            || amplitude.iter().any(|value| *value <= 0.0)
        {
            return Err(AmbientModeError::InvalidState);
        }

        Ok(Self {
            amplitude,
            amplitude_velocity,
            amplitude_acceleration,
        })
    }

    fn first_derivative(values: &[f64], index: usize) -> f64 {
        let count = values.len();
        let previous_two = values[(index + count - 2) % count];
        let previous = values[(index + count - 1) % count];
        let next = values[(index + 1) % count];
        let next_two = values[(index + 2) % count];
        (previous_two - 8.0 * previous + 8.0 * next - next_two) / 12.0
    }

    fn second_derivative(values: &[f64], index: usize) -> f64 {
        let count = values.len();
        let previous_two = values[(index + count - 2) % count];
        let previous = values[(index + count - 1) % count];
        let current = values[index];
        let next = values[(index + 1) % count];
        let next_two = values[(index + 2) % count];
        (-next_two + 16.0 * next - 30.0 * current + 16.0 * previous - previous_two) / 12.0
    }
}

/// Read-only state of the uniform ambient mode at one logical tick.
///
/// energy_density is the mode's cycle-averaged oscillator energy per area.
/// carrier_value is the signed carrier displacement. The daily envelope
/// changes stored mode energy through explicit world exchange, not by changing
/// the carrier frequency.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AmbientModeSample {
    pub(crate) tick: u64,
    pub(crate) energy_density: f64,
    pub(crate) total_energy: f64,
    pub(crate) amplitude: f64,
    pub(crate) angular_frequency: f64,
    pub(crate) carrier_phase_radians: f64,
    pub(crate) carrier_value: f64,
    /// Exact time derivative of the ambient carrier displacement.
    pub(crate) carrier_velocity: f64,
    /// Exact second time derivative of the ambient carrier displacement.
    pub(crate) carrier_acceleration: f64,
    /// Instantaneous oscillator energy density; energy_density remains the
    /// carrier-averaged target schedule used by the world inventory ledger.
    pub(crate) instantaneous_oscillator_energy_density: f64,
}

/// One exact change in the ambient mode's stored energy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AmbientModeTransfer {
    pub(crate) from_tick: u64,
    pub(crate) to_tick: u64,
    pub(crate) energy_before: f64,
    pub(crate) energy_after: f64,
    /// Positive means world generation; negative means world withdrawal.
    pub(crate) signed_world_exchange: f64,
    pub(crate) generated_energy: f64,
    pub(crate) withdrawn_energy: f64,
}

/// Deterministic uniform ambient resonance mode used by the live simulation.
#[derive(Clone, Debug)]
pub(crate) struct WorldGeneratedAmbientMode {
    config: AmbientModeConfig,
    trajectory: Arc<AmbientModeTrajectory>,
    tick: u64,
    current: AmbientModeSample,
    cumulative_generated_energy: f64,
    cumulative_withdrawn_energy: f64,
}

impl WorldGeneratedAmbientMode {
    /// Initializes the mode at initial_tick. Its initial energy is opening
    /// inventory, not a generated transfer; subsequent changes are accounted
    /// explicitly by advance_one_tick.
    pub(crate) fn new(
        config: AmbientModeConfig,
        initial_tick: u64,
    ) -> Result<Self, AmbientModeError> {
        if !config.is_valid() {
            return Err(AmbientModeError::InvalidParameter);
        }
        let trajectory = Arc::new(AmbientModeTrajectory::build(config)?);
        let current = config.sample_at(initial_tick, &trajectory)?;
        Ok(Self {
            config,
            trajectory,
            tick: initial_tick,
            current,
            cumulative_generated_energy: 0.0,
            cumulative_withdrawn_energy: 0.0,
        })
    }

    pub(crate) fn sample(&self) -> AmbientModeSample {
        self.current
    }

    /// Total world generation since this mode's opening inventory.
    #[cfg(test)]
    pub(crate) fn cumulative_generated_energy(&self) -> f64 {
        self.cumulative_generated_energy
    }

    /// Total world withdrawal since this mode's opening inventory.
    #[cfg(test)]
    pub(crate) fn cumulative_withdrawn_energy(&self) -> f64 {
        self.cumulative_withdrawn_energy
    }

    /// Advances exactly one logical tick and accounts for the complete change
    /// in the mode's stored energy. State and counters are committed together.
    pub(crate) fn advance_one_tick(&mut self) -> Result<AmbientModeTransfer, AmbientModeError> {
        let next_tick = self
            .tick
            .checked_add(1)
            .ok_or(AmbientModeError::CounterOverflow)?;
        let next = self.config.sample_at(next_tick, &self.trajectory)?;
        let signed_world_exchange = next.total_energy - self.current.total_energy;
        let generated_energy = signed_world_exchange.max(0.0);
        let withdrawn_energy = (-signed_world_exchange).max(0.0);
        let next_generated = self.cumulative_generated_energy + generated_energy;
        let next_withdrawn = self.cumulative_withdrawn_energy + withdrawn_energy;

        if [
            signed_world_exchange,
            generated_energy,
            withdrawn_energy,
            next_generated,
            next_withdrawn,
        ]
        .iter()
        .any(|value| !value.is_finite())
        {
            return Err(AmbientModeError::InvalidState);
        }

        let transfer = AmbientModeTransfer {
            from_tick: self.tick,
            to_tick: next_tick,
            energy_before: self.current.total_energy,
            energy_after: next.total_energy,
            signed_world_exchange,
            generated_energy,
            withdrawn_energy,
        };

        self.tick = next_tick;
        self.current = next;
        self.cumulative_generated_energy = next_generated;
        self.cumulative_withdrawn_energy = next_withdrawn;
        Ok(transfer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mode_at(tick: u64, area: f64) -> WorldGeneratedAmbientMode {
        let config = AmbientModeConfig::new(area, 2.5, 0.0, 0.0).unwrap();
        WorldGeneratedAmbientMode::new(config, tick).unwrap()
    }

    #[test]
    fn daily_cosine_envelope_spans_one_to_ten_over_24000_ticks() {
        let mut mode = mode_at(0, 12.0);
        assert_eq!(mode.sample().energy_density, 10.0);

        for _ in 0..6_000 {
            mode.advance_one_tick().unwrap();
        }
        assert!((mode.sample().energy_density - 5.5).abs() < 1.0e-12);

        for _ in 6_000..12_000 {
            mode.advance_one_tick().unwrap();
        }
        assert!((mode.sample().energy_density - 1.0).abs() < 1.0e-12);

        for _ in 12_000..18_000 {
            mode.advance_one_tick().unwrap();
        }
        assert!((mode.sample().energy_density - 5.5).abs() < 1.0e-12);

        for _ in 18_000..24_000 {
            mode.advance_one_tick().unwrap();
        }
        assert!((mode.sample().energy_density - 10.0).abs() < 1.0e-12);
    }

    #[test]
    fn carrier_advances_at_440_cycles_per_simulated_day() {
        let mode = mode_at(0, 1.0);
        assert_eq!(mode.sample().angular_frequency, TAU * 440.0 / 24_000.0);
        let mut quarter_day = mode;
        for _ in 0..6_000 {
            quarter_day.advance_one_tick().unwrap();
        }
        // 440 * 1/4 day = 110 complete carrier cycles.
        assert!(quarter_day.sample().carrier_phase_radians.abs() < 1.0e-10);
    }

    #[test]
    fn oscillator_amplitude_matches_the_declared_energy_density() {
        let mode = mode_at(0, 3.0);
        let sample = mode.sample();
        let reconstructed = 0.5 * 2.5 * sample.angular_frequency.powi(2) * sample.amplitude.powi(2)
            + 0.25 * 2.5 * sample.amplitude_velocity.powi(2);
        assert!((reconstructed - sample.energy_density).abs() < 1.0e-12);
        assert!((sample.total_energy - sample.energy_density * 3.0).abs() < 1.0e-12);
    }

    #[test]
    fn daily_energy_change_is_reconciled_as_world_generation_and_withdrawal() {
        let area = 7.5;
        let mut mode = mode_at(0, area);
        let initial_energy = mode.sample().total_energy;
        let mut total_generated = 0.0;
        let mut total_withdrawn = 0.0;

        for _ in 0..AMBIENT_DAY_TICKS {
            let transfer = mode.advance_one_tick().unwrap();
            total_generated += transfer.generated_energy;
            total_withdrawn += transfer.withdrawn_energy;
            assert!(
                (transfer.energy_before + transfer.signed_world_exchange - transfer.energy_after)
                    .abs()
                    <= 1.0e-12
            );
        }

        assert!((mode.sample().total_energy - initial_energy).abs() < 1.0e-10);
        assert!((total_generated - 9.0 * area).abs() < 1.0e-9);
        assert!((total_withdrawn - 9.0 * area).abs() < 1.0e-9);
        assert!((mode.cumulative_generated_energy() - total_generated).abs() < 1.0e-10);
        assert!((mode.cumulative_withdrawn_energy() - total_withdrawn).abs() < 1.0e-10);
        assert!((total_generated - total_withdrawn).abs() < 1.0e-10);
    }

    #[test]
    fn invalid_configuration_is_rejected() {
        assert_eq!(
            AmbientModeConfig::new(0.0, 1.0, 0.0, 0.0),
            Err(AmbientModeError::InvalidParameter)
        );
        assert_eq!(
            AmbientModeConfig::new(1.0, f64::NAN, 0.0, 0.0),
            Err(AmbientModeError::InvalidParameter)
        );
        assert_eq!(
            AmbientModeConfig::new(1.0, 1.0, f64::INFINITY, 0.0),
            Err(AmbientModeError::InvalidParameter)
        );
    }

    #[test]
    fn replay_is_bitwise_deterministic() {
        let mut left = mode_at(0, 2.0);
        let mut right = mode_at(0, 2.0);
        for _ in 0..5_000 {
            left.advance_one_tick().unwrap();
            right.advance_one_tick().unwrap();
        }
        assert_eq!(left.sample(), right.sample());
        assert_eq!(
            left.cumulative_generated_energy().to_bits(),
            right.cumulative_generated_energy().to_bits()
        );
        assert_eq!(
            left.cumulative_withdrawn_energy().to_bits(),
            right.cumulative_withdrawn_energy().to_bits()
        );
    }
}
