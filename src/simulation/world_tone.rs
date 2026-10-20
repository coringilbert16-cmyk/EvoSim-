#![expect(
    dead_code,
    reason = "Staged environmental source model awaiting simulation and energy-ledger integration"
)]
//! Parameterized model for EvoSim's single rotating world-tone source.
//!
//! This module models source position, frequency, signal level, and the energy
//! available from the source per tick as separate quantities. It does not grant
//! captured energy or mutate an energy ledger. Runtime configuration remains
//! explicit; this model does not silently choose unresolved world parameters.

use std::f64::consts::TAU;

pub(crate) const WORLD_TONE_FREQUENCY_HZ: f64 = 440.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EnvelopeCurve {
    /// Smoothly reaches zero slope at the northern and southern extrema.
    Cosine,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WorldToneParameters {
    pub(crate) minimum_signal_energy_level: f64,
    pub(crate) maximum_signal_energy_level: f64,
    pub(crate) minimum_available_energy_per_tick: f64,
    pub(crate) maximum_available_energy_per_tick: f64,
    pub(crate) orbital_period_ticks: u64,
    pub(crate) envelope_curve: EnvelopeCurve,
}

/// Explicit runtime configuration; no orbit radius or unresolved signal parameters are guessed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WorldToneRuntimeConfig {
    pub(crate) parameters: WorldToneParameters,
    pub(crate) orbit_radius: f64,
}

impl WorldToneRuntimeConfig {
    pub(crate) fn is_valid(self) -> bool {
        self.parameters.is_valid() && self.orbit_radius.is_finite() && self.orbit_radius >= 0.0
    }

    pub(crate) fn sample(
        self,
        tick: u64,
        world_width: f64,
        world_height: f64,
    ) -> Option<WorldToneSample> {
        if !self.is_valid()
            || !world_width.is_finite()
            || !world_height.is_finite()
            || world_width <= 0.0
            || world_height <= 0.0
        {
            return None;
        }
        self.parameters.sample(
            tick,
            world_width * 0.5,
            world_height * 0.5,
            self.orbit_radius,
        )
    }
}

/// Maximum table size keeps unusual, user-supplied periods from allocating
/// unbounded memory. Longer periods remain correct and use direct sampling.
pub(crate) const MAX_PRECOMPUTED_WORLD_TONE_SAMPLES: u64 = 65_536;

/// Immutable lookup table for a deterministic source cycle.
/// The table contains source output only; it does not allocate energy to receivers.
#[derive(Clone, Debug)]
pub(crate) struct WorldToneCycle {
    samples: Vec<WorldToneSample>,
}

impl WorldToneCycle {
    pub(crate) fn precompute(
        config: WorldToneRuntimeConfig,
        world_width: f64,
        world_height: f64,
    ) -> Option<Self> {
        let period = config.parameters.orbital_period_ticks;
        if !config.is_valid()
            || !world_width.is_finite()
            || !world_height.is_finite()
            || world_width <= 0.0
            || world_height <= 0.0
            || period > MAX_PRECOMPUTED_WORLD_TONE_SAMPLES
        {
            return None;
        }

        let mut samples = Vec::new();
        samples
            .try_reserve_exact(usize::try_from(period).ok()?)
            .ok()?;
        for tick in 0..period {
            samples.push(config.sample(tick, world_width, world_height)?);
        }
        Some(Self { samples })
    }

    pub(crate) fn sample(&self, tick: u64) -> Option<WorldToneSample> {
        self.samples
            .get(usize::try_from(tick % self.samples.len() as u64).ok()?)
            .copied()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WorldToneSample {
    pub(crate) source_x: f64,
    pub(crate) source_y: f64,
    pub(crate) frequency_hz: f64,
    /// Environmental signal level, not energy already captured by an organism.
    pub(crate) signal_energy_level: f64,
    /// Energy available for capture this tick, before organism-specific efficiency.
    pub(crate) available_energy_per_tick: f64,
    /// Normalized orbital phase: 0 at north, increasing eastward, in [0, 1).
    pub(crate) orbital_phase: f64,
}

impl WorldToneParameters {
    pub(crate) fn is_valid(self) -> bool {
        self.minimum_signal_energy_level.is_finite()
            && self.minimum_signal_energy_level >= 0.0
            && self.maximum_signal_energy_level.is_finite()
            && self.maximum_signal_energy_level > self.minimum_signal_energy_level
            && self.minimum_available_energy_per_tick.is_finite()
            && self.minimum_available_energy_per_tick >= 0.0
            && self.maximum_available_energy_per_tick.is_finite()
            && self.maximum_available_energy_per_tick > self.minimum_available_energy_per_tick
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
        let frequency_hz = WORLD_TONE_FREQUENCY_HZ;
        Some(WorldToneSample {
            source_x: center_x + orbit_radius * angle.sin(),
            source_y: center_y - orbit_radius * angle.cos(),
            frequency_hz,
            signal_energy_level: self.minimum_signal_energy_level
                + (self.maximum_signal_energy_level - self.minimum_signal_energy_level) * envelope,
            available_energy_per_tick: self.minimum_available_energy_per_tick
                + (self.maximum_available_energy_per_tick - self.minimum_available_energy_per_tick)
                    * envelope,
            orbital_phase,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parameters() -> WorldToneParameters {
        WorldToneParameters {
            minimum_signal_energy_level: 0.0,
            maximum_signal_energy_level: 8.0,
            minimum_available_energy_per_tick: 1.0,
            maximum_available_energy_per_tick: 10.0,
            orbital_period_ticks: 2_400,
            envelope_curve: EnvelopeCurve::Cosine,
        }
    }

    #[test]
    fn source_orbits_clockwise_and_returns_to_north() {
        let params = parameters();
        let north = params.sample(0, 50.0, 50.0, 10.0).unwrap();
        let east = params.sample(600, 50.0, 50.0, 10.0).unwrap();
        let south = params.sample(1_200, 50.0, 50.0, 10.0).unwrap();
        let west = params.sample(1_800, 50.0, 50.0, 10.0).unwrap();
        let next_north = params.sample(2_400, 50.0, 50.0, 10.0).unwrap();

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
    fn frequency_stays_440_while_signal_and_energy_levels_cycle() {
        let params = parameters();
        let north = params.sample(0, 0.0, 0.0, 1.0).unwrap();
        let east = params.sample(600, 0.0, 0.0, 1.0).unwrap();
        let south = params.sample(1_200, 0.0, 0.0, 1.0).unwrap();
        let west = params.sample(1_800, 0.0, 0.0, 1.0).unwrap();

        assert_eq!(north.frequency_hz, WORLD_TONE_FREQUENCY_HZ);
        assert_eq!(north.signal_energy_level, 8.0);
        assert_eq!(north.available_energy_per_tick, 10.0);
        assert_eq!(south.frequency_hz, WORLD_TONE_FREQUENCY_HZ);
        assert_eq!(south.signal_energy_level, 0.0);
        assert_eq!(south.available_energy_per_tick, 1.0);
        assert_eq!(east.frequency_hz, WORLD_TONE_FREQUENCY_HZ);
        assert_eq!(west.frequency_hz, WORLD_TONE_FREQUENCY_HZ);
        assert!((east.signal_energy_level - 4.0).abs() < 1e-12);
        assert!((west.signal_energy_level - 4.0).abs() < 1e-12);
        assert!((east.available_energy_per_tick - 5.5).abs() < 1e-12);
        assert!((west.available_energy_per_tick - 5.5).abs() < 1e-12);
    }

    #[test]
    fn runtime_configuration_uses_world_center_and_explicit_orbit_radius() {
        let config = WorldToneRuntimeConfig {
            parameters: parameters(),
            orbit_radius: 250.0,
        };
        let north = config.sample(0, 1_000.0, 800.0).unwrap();
        let east = config.sample(600, 1_000.0, 800.0).unwrap();
        assert_eq!((north.source_x, north.source_y), (500.0, 150.0));
        assert!((east.source_x - 750.0).abs() < 1e-9);
        assert!((east.source_y - 400.0).abs() < 1e-9);

        let invalid = WorldToneRuntimeConfig {
            orbit_radius: -1.0,
            ..config
        };
        assert!(invalid.sample(0, 1_000.0, 800.0).is_none());
    }

    #[test]
    fn precomputed_cycle_matches_direct_samples_for_every_tick_and_wraps() {
        let config = WorldToneRuntimeConfig {
            parameters: parameters(),
            orbit_radius: 250.0,
        };
        let cycle = WorldToneCycle::precompute(config, 1_000.0, 800.0).unwrap();
        assert_eq!(
            cycle.samples.len(),
            parameters().orbital_period_ticks as usize
        );
        for tick in 0..parameters().orbital_period_ticks {
            assert_eq!(
                cycle.sample(tick),
                config.sample(tick, 1_000.0, 800.0),
                "sample mismatch at tick {tick}"
            );
        }
        assert_eq!(
            cycle.sample(0),
            cycle.sample(parameters().orbital_period_ticks)
        );
        assert_eq!(cycle.sample(2_400), cycle.sample(4_800));
    }

    #[test]
    fn oversized_cycle_skips_precomputation_without_rejecting_source() {
        let config = WorldToneRuntimeConfig {
            parameters: WorldToneParameters {
                orbital_period_ticks: MAX_PRECOMPUTED_WORLD_TONE_SAMPLES + 1,
                ..parameters()
            },
            orbit_radius: 10.0,
        };
        assert!(config.is_valid());
        assert!(WorldToneCycle::precompute(config, 100.0, 100.0).is_none());
        assert!(config.sample(0, 100.0, 100.0).is_some());
    }

    #[test]
    fn invalid_or_unresolved_parameters_fail_closed() {
        let mut params = parameters();
        params.minimum_available_energy_per_tick = -1.0;
        assert!(params.sample(0, 0.0, 0.0, 1.0).is_none());

        params = parameters();
        params.orbital_period_ticks = 0;
        assert!(params.sample(0, 0.0, 0.0, 1.0).is_none());
        assert!(parameters().sample(0, 0.0, 0.0, -1.0).is_none());
    }
}
