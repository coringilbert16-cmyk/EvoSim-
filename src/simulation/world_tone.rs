#![expect(
    dead_code,
    reason = "Staged environmental source model awaiting simulation and energy-ledger integration"
)]
//! Parameterized model for EvoSim's single rotating world-tone source.
//!
//! This module deliberately models signal position, frequency, and signal-energy
//! level only. It does not grant energy or mutate an energy ledger. The orbital
//! period, minimum frequency, signal-energy scale, and envelope curve must be
//! supplied by the caller; no unresolved world parameter is silently chosen.

use std::f64::consts::TAU;

pub(crate) const MAX_WORLD_TONE_FREQUENCY_HZ: f64 = 440.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EnvelopeCurve {
    /// Smoothly reaches zero slope at the northern and southern extrema.
    Cosine,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WorldToneParameters {
    pub(crate) minimum_frequency_hz: f64,
    pub(crate) minimum_signal_energy_level: f64,
    pub(crate) maximum_signal_energy_level: f64,
    pub(crate) orbital_period_ticks: u64,
    pub(crate) envelope_curve: EnvelopeCurve,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WorldToneSample {
    pub(crate) source_x: f64,
    pub(crate) source_y: f64,
    pub(crate) frequency_hz: f64,
    /// Environmental signal level, not energy already captured by an organism.
    pub(crate) signal_energy_level: f64,
    /// Normalized orbital phase: 0 at north, increasing eastward, in [0, 1).
    pub(crate) orbital_phase: f64,
}

impl WorldToneParameters {
    pub(crate) fn is_valid(self) -> bool {
        self.minimum_frequency_hz.is_finite()
            && self.minimum_frequency_hz > 0.0
            && self.minimum_frequency_hz <= MAX_WORLD_TONE_FREQUENCY_HZ
            && self.minimum_signal_energy_level.is_finite()
            && self.minimum_signal_energy_level >= 0.0
            && self.maximum_signal_energy_level.is_finite()
            && self.maximum_signal_energy_level > self.minimum_signal_energy_level
            && self.orbital_period_ticks > 0
    }

    /// Sample the source at a simulation tick around the supplied world center.
    /// The caller supplies the orbit radius so world geometry is not guessed here.
    pub(crate) fn sample(
        self,
        tick: u64,
        center_x: f64,
        center_y: f64,
        orbit_radius: f64,
    ) -> Option<WorldToneSample> {
        if !self.is_valid()
            || !center_x.is_finite()
            || !center_y.is_finite()
            || !orbit_radius.is_finite()
            || orbit_radius < 0.0
        {
            return None;
        }

        let orbital_phase =
            (tick % self.orbital_period_ticks) as f64 / self.orbital_period_ticks as f64;
        let angle = TAU * orbital_phase;
        let envelope = match self.envelope_curve {
            EnvelopeCurve::Cosine => (1.0 + angle.cos()) * 0.5,
        };
        let frequency_hz = self.minimum_frequency_hz
            + (MAX_WORLD_TONE_FREQUENCY_HZ - self.minimum_frequency_hz) * envelope;
        Some(WorldToneSample {
            source_x: center_x + orbit_radius * angle.sin(),
            source_y: center_y - orbit_radius * angle.cos(),
            frequency_hz,
            signal_energy_level: self.minimum_signal_energy_level
                + (self.maximum_signal_energy_level - self.minimum_signal_energy_level) * envelope,
            orbital_phase,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parameters() -> WorldToneParameters {
        WorldToneParameters {
            minimum_frequency_hz: 110.0,
            minimum_signal_energy_level: 0.0,
            maximum_signal_energy_level: 8.0,
            orbital_period_ticks: 100,
            envelope_curve: EnvelopeCurve::Cosine,
        }
    }

    #[test]
    fn source_orbits_clockwise_and_returns_to_north() {
        let params = parameters();
        let north = params.sample(0, 50.0, 50.0, 10.0).unwrap();
        let east = params.sample(25, 50.0, 50.0, 10.0).unwrap();
        let south = params.sample(50, 50.0, 50.0, 10.0).unwrap();
        let west = params.sample(75, 50.0, 50.0, 10.0).unwrap();
        let next_north = params.sample(100, 50.0, 50.0, 10.0).unwrap();

        assert_eq!((north.source_x, north.source_y), (50.0, 40.0));
        assert!((east.source_x - 60.0).abs() < 1e-9);
        assert!((east.source_y - 50.0).abs() < 1e-9);
        assert!((south.source_x - 50.0).abs() < 1e-9);
        assert!((south.source_y - 60.0).abs() < 1e-9);
        assert!((west.source_x - 40.0).abs() < 1e-9);
        assert!((west.source_y - 50.0).abs() < 1e-9);
        assert_eq!(north, next_north);
    }

    #[test]
    fn frequency_and_signal_level_co_vary_but_remain_separate_outputs() {
        let params = parameters();
        let north = params.sample(0, 0.0, 0.0, 1.0).unwrap();
        let east = params.sample(25, 0.0, 0.0, 1.0).unwrap();
        let south = params.sample(50, 0.0, 0.0, 1.0).unwrap();
        let west = params.sample(75, 0.0, 0.0, 1.0).unwrap();

        assert_eq!(north.frequency_hz, 440.0);
        assert_eq!(north.signal_energy_level, 8.0);
        assert_eq!(south.frequency_hz, 110.0);
        assert_eq!(south.signal_energy_level, 0.0);
        assert!(north.frequency_hz > east.frequency_hz);
        assert!(east.frequency_hz > south.frequency_hz);
        assert!(south.frequency_hz < west.frequency_hz);
        assert!(west.frequency_hz < north.frequency_hz);
        assert_eq!(east.signal_energy_level, 4.0);
        assert_eq!(west.signal_energy_level, 4.0);
    }

    #[test]
    fn invalid_or_unresolved_parameters_fail_closed() {
        let mut params = parameters();
        params.minimum_frequency_hz = 0.0;
        assert!(params.sample(0, 0.0, 0.0, 1.0).is_none());

        params = parameters();
        params.orbital_period_ticks = 0;
        assert!(params.sample(0, 0.0, 0.0, 1.0).is_none());
        assert!(parameters().sample(0, 0.0, 0.0, -1.0).is_none());
    }
}
