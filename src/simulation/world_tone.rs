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

use crate::energy_ledger::{EnergyLedgerAuthority, ExternalEnergySource, ExternalInputTransaction};
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
    pub(crate) minimum_energy_intensity_per_tick_per_width: f64,
    pub(crate) maximum_energy_intensity_per_tick_per_width: f64,
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
    /// Incoming energy intensity per tick per unit of exposed projected width, before absorption.
    pub(crate) energy_intensity_per_tick_per_width: f64,
    /// Normalized orbital phase: 0 at north, increasing eastward, in [0, 1).
    pub(crate) orbital_phase: f64,
}
/// Energy split for one material interaction with the incoming world tone.
///
/// The unabsorbed amount is not automatically reflected or lost: downstream
/// propagation, reflection, and attenuation must explicitly account for it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WorldToneInteraction {
    pub(crate) incident_energy: f64,
    pub(crate) absorbed_energy: f64,
    pub(crate) unabsorbed_energy: f64,
}

impl WorldToneSample {
    /// Energy incident on an exposed projected width during this tick.
    ///
    /// Visibility/shadowing must be applied when computing the width; this
    /// helper does not discover exposed geometry.
    pub(crate) fn incident_energy_for_projected_width(self, projected_width: f64) -> Option<f64> {
        if !self.energy_intensity_per_tick_per_width.is_finite()
            || self.energy_intensity_per_tick_per_width < 0.0
            || !projected_width.is_finite()
            || projected_width < 0.0
        {
            return None;
        }
        let energy = self.energy_intensity_per_tick_per_width * projected_width;
        energy.is_finite().then_some(energy)
    }

    /// Partition intercepted energy using a supplied material absorption
    /// fraction. This does not choose material parameters or settle a ledger.
    pub(crate) fn interact_with_absorption(
        self,
        projected_width: f64,
        absorption_fraction: f64,
    ) -> Option<WorldToneInteraction> {
        if !absorption_fraction.is_finite() || !(0.0..=1.0).contains(&absorption_fraction) {
            return None;
        }
        let incident_energy = self.incident_energy_for_projected_width(projected_width)?;
        let absorbed_energy = incident_energy * absorption_fraction;
        let unabsorbed_energy = incident_energy - absorbed_energy;
        (absorbed_energy.is_finite() && unabsorbed_energy.is_finite()).then_some(
            WorldToneInteraction {
                incident_energy,
                absorbed_energy,
                unabsorbed_energy,
            },
        )
    }
}

/// Result of settling one receiver's solar input for one tick.
/// Unabsorbed incident energy remains outside the receiver ledger; propagation
/// or reflection must be modeled separately before claiming it as another input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WorldToneCaptureResult {
    pub(crate) projected_width: f64,
    pub(crate) incident_energy: f64,
    pub(crate) absorbed_energy: f64,
    pub(crate) unabsorbed_energy: f64,
    pub(crate) usable_energy: f64,
    pub(crate) heat_dissipated: f64,
}

/// A per-tick geometry snapshot. Build it once from all relevant bodies, then
/// settle each receiver against the cached exposure so visibility is not
/// recomputed for every organism. A receiver can be settled only once per frame.
#[derive(Debug)]
pub(crate) struct WorldToneCaptureFrame {
    sample: crate::world_tone::WorldToneSample,
    projected_widths: Vec<f64>,
    /// Exposure attributed to each constituent, preserving material provenance.
    projected_widths_by_unit: Vec<Vec<(usize, f64)>>,
    settled_receivers: Vec<bool>,
}

impl WorldToneCaptureFrame {
    /// Discover polygonal visibility and cache each body's unique projected width.
    /// Unsupported geometry fails closed; callers should rebuild this frame next tick.
    pub(crate) fn from_geometry(
        sample: crate::world_tone::WorldToneSample,
        bodies: &[crate::organism_geometry::OrganismBodyGeometry],
    ) -> Option<Self> {
        let intervals = crate::source_exposure::visible_polygonal_intervals(
            (sample.source_x, sample.source_y),
            bodies,
        )?;
        let mut projected_widths = vec![0.0; bodies.len()];
        let mut projected_widths_by_unit = vec![Vec::new(); bodies.len()];
        for interval in intervals {
            let midpoint = (
                (interval.start.0 + interval.end.0) * 0.5,
                (interval.start.1 + interval.end.1) * 0.5,
            );
            let dx = sample.source_x - midpoint.0;
            let dy = sample.source_y - midpoint.1;
            let distance = dx.hypot(dy);
            if !distance.is_finite() || distance <= f64::EPSILON {
                return None;
            }
            let extent =
                crate::source_exposure::projected_extent(interval, (dx / distance, dy / distance))?;
            let body_width = projected_widths.get_mut(interval.body_index)?;
            *body_width += extent;
            if !body_width.is_finite() {
                return None;
            }

            let unit_widths = projected_widths_by_unit.get_mut(interval.body_index)?;
            if let Some((_, width)) = unit_widths
                .iter_mut()
                .find(|(unit_index, _)| *unit_index == interval.unit_index)
            {
                *width += extent;
                if !width.is_finite() {
                    return None;
                }
            } else {
                unit_widths.push((interval.unit_index, extent));
            }
        }
        Some(Self {
            sample,
            projected_widths,
            projected_widths_by_unit,
            settled_receivers: vec![false; bodies.len()],
        })
    }

    /// Return cached exposure for a constituent. Some(0.0) means no visible
    /// source-facing area; None means the body index is absent.
    pub(crate) fn projected_width_for_unit(
        &self,
        receiver_body_index: usize,
        unit_index: usize,
    ) -> Option<f64> {
        let units = self.projected_widths_by_unit.get(receiver_body_index)?;
        Some(
            units
                .iter()
                .find(|(candidate, _)| *candidate == unit_index)
                .map_or(0.0, |(_, width)| *width),
        )
    }

    /// Settle one receiver's absorbed input atomically. Fractions remain explicit
    /// until resource-specific absorption/retention properties are approved.
    pub(crate) fn settle_receiver(
        &mut self,
        receiver_body_index: usize,
        absorption_fraction: f64,
        usable_fraction: f64,
        holder: &mut f64,
        ledger: &mut crate::state::EnergyLedger,
    ) -> Option<WorldToneCaptureResult> {
        let settled = *self.settled_receivers.get(receiver_body_index)?;
        if settled
            || !absorption_fraction.is_finite()
            || !(0.0..=1.0).contains(&absorption_fraction)
            || !usable_fraction.is_finite()
            || !(0.0..=1.0).contains(&usable_fraction)
        {
            return None;
        }

        let projected_width = *self.projected_widths.get(receiver_body_index)?;
        let incident_energy = self
            .sample
            .incident_energy_for_projected_width(projected_width)?;
        let absorbed_energy = incident_energy * absorption_fraction;
        let unabsorbed_energy = incident_energy - absorbed_energy;
        let usable_energy = absorbed_energy * usable_fraction;
        let heat_dissipated = absorbed_energy - usable_energy;
        if [
            absorbed_energy,
            unabsorbed_energy,
            usable_energy,
            heat_dissipated,
        ]
        .iter()
        .any(|value| !value.is_finite() || *value < 0.0)
        {
            return None;
        }

        let transaction = ExternalInputTransaction {
            source: ExternalEnergySource::WorldTone,
            source_available: incident_energy,
            external_input: absorbed_energy,
            usable_delta: usable_energy,
            work_expended: 0.0,
            heat_dissipated,
        };
        if !ledger.settle_external_input(holder, transaction) {
            return None;
        }
        self.settled_receivers[receiver_body_index] = true;

        Some(WorldToneCaptureResult {
            projected_width,
            incident_energy,
            absorbed_energy,
            unabsorbed_energy,
            usable_energy,
            heat_dissipated,
        })
    }
}

impl WorldToneParameters {
    pub(crate) fn is_valid(self) -> bool {
        self.minimum_signal_energy_level.is_finite()
            && self.minimum_signal_energy_level >= 0.0
            && self.maximum_signal_energy_level.is_finite()
            && self.maximum_signal_energy_level > self.minimum_signal_energy_level
            && self.minimum_energy_intensity_per_tick_per_width.is_finite()
            && self.minimum_energy_intensity_per_tick_per_width >= 0.0
            && self.maximum_energy_intensity_per_tick_per_width.is_finite()
            && self.maximum_energy_intensity_per_tick_per_width
                > self.minimum_energy_intensity_per_tick_per_width
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
            energy_intensity_per_tick_per_width: self.minimum_energy_intensity_per_tick_per_width
                + (self.maximum_energy_intensity_per_tick_per_width
                    - self.minimum_energy_intensity_per_tick_per_width)
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
            minimum_energy_intensity_per_tick_per_width: 1.0,
            maximum_energy_intensity_per_tick_per_width: 10.0,
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
        assert_eq!(north.energy_intensity_per_tick_per_width, 10.0);
        assert_eq!(south.frequency_hz, WORLD_TONE_FREQUENCY_HZ);
        assert_eq!(south.signal_energy_level, 0.0);
        assert_eq!(south.energy_intensity_per_tick_per_width, 1.0);
        assert_eq!(east.frequency_hz, WORLD_TONE_FREQUENCY_HZ);
        assert_eq!(west.frequency_hz, WORLD_TONE_FREQUENCY_HZ);
        assert!((east.signal_energy_level - 4.0).abs() < 1e-12);
        assert!((west.signal_energy_level - 4.0).abs() < 1e-12);
        assert!((east.energy_intensity_per_tick_per_width - 5.5).abs() < 1e-12);
        assert!((west.energy_intensity_per_tick_per_width - 5.5).abs() < 1e-12);
    }

    #[test]
    fn incident_energy_scales_with_exposed_projected_width() {
        let params = parameters();
        let peak = params.sample(0, 0.0, 0.0, 1.0).unwrap();
        assert_eq!(peak.energy_intensity_per_tick_per_width, 10.0);
        assert_eq!(peak.incident_energy_for_projected_width(1.0), Some(10.0));
        assert_eq!(peak.incident_energy_for_projected_width(0.5), Some(5.0));

        let minimum = params.sample(1_200, 0.0, 0.0, 1.0).unwrap();
        assert_eq!(minimum.energy_intensity_per_tick_per_width, 1.0);
        assert_eq!(minimum.incident_energy_for_projected_width(0.5), Some(0.5));
    }

    #[test]
    fn material_interaction_partitions_incident_energy_without_creating_energy() {
        let sample = parameters().sample(0, 0.0, 0.0, 1.0).unwrap();
        let interaction = sample.interact_with_absorption(1.0, 0.3).unwrap();
        assert_eq!(interaction.incident_energy, 10.0);
        assert!((interaction.absorbed_energy - 3.0).abs() < 1e-12);
        assert!((interaction.unabsorbed_energy - 7.0).abs() < 1e-12);
        assert!(
            (interaction.absorbed_energy + interaction.unabsorbed_energy
                - interaction.incident_energy)
                .abs()
                < 1e-12
        );
    }

    #[test]
    fn material_interaction_accepts_zero_and_full_absorption() {
        let sample = parameters().sample(0, 0.0, 0.0, 1.0).unwrap();
        let none = sample.interact_with_absorption(0.5, 0.0).unwrap();
        assert_eq!(none.absorbed_energy, 0.0);
        assert_eq!(none.unabsorbed_energy, 5.0);
        let all = sample.interact_with_absorption(0.5, 1.0).unwrap();
        assert_eq!(all.absorbed_energy, 5.0);
        assert_eq!(all.unabsorbed_energy, 0.0);
    }

    #[test]
    fn material_interaction_rejects_invalid_absorption_fraction() {
        let sample = parameters().sample(0, 0.0, 0.0, 1.0).unwrap();
        assert!(sample.interact_with_absorption(1.0, -0.1).is_none());
        assert!(sample.interact_with_absorption(1.0, 1.1).is_none());
        assert!(sample.interact_with_absorption(1.0, f64::NAN).is_none());
    }

    #[test]
    fn incident_energy_rejects_invalid_projected_width() {
        let sample = parameters().sample(0, 0.0, 0.0, 1.0).unwrap();
        assert_eq!(sample.incident_energy_for_projected_width(-0.1), None);
        assert_eq!(sample.incident_energy_for_projected_width(f64::NAN), None);
        assert_eq!(
            sample.incident_energy_for_projected_width(f64::INFINITY),
            None
        );
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
        params.minimum_energy_intensity_per_tick_per_width = -1.0;
        assert!(params.sample(0, 0.0, 0.0, 1.0).is_none());

        params = parameters();
        params.orbital_period_ticks = 0;
        assert!(params.sample(0, 0.0, 0.0, 1.0).is_none());
        assert!(parameters().sample(0, 0.0, 0.0, -1.0).is_none());
    }
    #[test]
    fn world_tone_capture_uses_visible_receiver_geometry_and_settles_balanced_input() {
        use crate::organism_geometry::{OrganismBodyGeometry, PlacedForm};
        use crate::resources::Form;
        let receiver = OrganismBodyGeometry {
            parts: vec![PlacedForm {
                unit_index: 0,
                form: Form::Rectangle {
                    width: 2.0,
                    height: 2.0,
                },
                x: 5.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            min_x: 4.0,
            max_x: 6.0,
            min_y: -1.0,
            max_y: 1.0,
        };
        let sample = crate::world_tone::WorldToneSample {
            source_x: -5.0,
            source_y: 0.0,
            frequency_hz: 440.0,
            signal_energy_level: 8.0,
            energy_intensity_per_tick_per_width: 10.0,
            orbital_phase: 0.0,
        };
        let mut frame = WorldToneCaptureFrame::from_geometry(sample, &[receiver])
            .expect("valid polygonal receiver should produce exposure frame");
        let mut holder = 1.0;
        let mut ledger = crate::state::EnergyLedger::default();
        let result = frame
            .settle_receiver(0, 0.5, 0.8, &mut holder, &mut ledger)
            .expect("valid receiver should capture");
        assert!((result.projected_width - 2.0).abs() < 1e-8);
        assert!((frame.projected_width_for_unit(0, 0).unwrap() - 2.0).abs() < 1e-8);
        assert!((result.incident_energy - 20.0).abs() < 1e-7);
        assert!((result.absorbed_energy - 10.0).abs() < 1e-7);
        assert!((result.unabsorbed_energy - 10.0).abs() < 1e-7);
        assert!((result.usable_energy - 8.0).abs() < 1e-7);
        assert!((result.heat_dissipated - 2.0).abs() < 1e-7);
        assert!((holder - 9.0).abs() < 1e-7);
        assert!((ledger.total_external_energy_input - 10.0).abs() < 1e-7);
        assert!((ledger.total_usable_energy_gained - 8.0).abs() < 1e-7);
        assert!((ledger.total_heat_dissipated - 2.0).abs() < 1e-7);
    }

    #[test]
    fn world_tone_capture_preserves_surface_exposure_by_constituent() {
        use crate::organism_geometry::{OrganismBodyGeometry, PlacedForm};
        use crate::resources::Form;

        let body = OrganismBodyGeometry {
            parts: vec![
                PlacedForm {
                    unit_index: 0,
                    form: Form::Rectangle { width: 2.0, height: 2.0 },
                    x: 5.0, y: 0.0, rotation_radians: 0.0,
                },
                PlacedForm {
                    unit_index: 1,
                    form: Form::Rectangle { width: 2.0, height: 2.0 },
                    x: 7.0, y: 0.0, rotation_radians: 0.0,
                },
            ],
            min_x: 4.0, max_x: 8.0, min_y: -1.0, max_y: 1.0,
        };
        let sample = crate::world_tone::WorldToneSample {
            source_x: -5.0, source_y: 0.0, frequency_hz: 440.0,
            signal_energy_level: 8.0, energy_intensity_per_tick_per_width: 10.0,
            orbital_phase: 0.0,
        };
        let frame = WorldToneCaptureFrame::from_geometry(sample, &[body]).unwrap();

        assert!((frame.projected_width_for_unit(0, 0).unwrap() - 2.0).abs() < 1e-7);
        assert_eq!(frame.projected_width_for_unit(0, 1), Some(0.0));
        assert_eq!(frame.projected_width_for_unit(0, 99), Some(0.0));
        assert_eq!(frame.projected_width_for_unit(99, 0), None);
        assert!((frame.projected_widths[0] - 2.0).abs() < 1e-7);
    }
    #[test]
    fn world_tone_capture_frame_prevents_duplicate_receiver_credit() {
        use crate::organism_geometry::{OrganismBodyGeometry, PlacedForm};
        use crate::resources::Form;
        let receiver = OrganismBodyGeometry {
            parts: vec![PlacedForm {
                unit_index: 0,
                form: Form::Rectangle {
                    width: 2.0,
                    height: 2.0,
                },
                x: 5.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            min_x: 4.0,
            max_x: 6.0,
            min_y: -1.0,
            max_y: 1.0,
        };
        let sample = crate::world_tone::WorldToneSample {
            source_x: -5.0,
            source_y: 0.0,
            frequency_hz: 440.0,
            signal_energy_level: 8.0,
            energy_intensity_per_tick_per_width: 10.0,
            orbital_phase: 0.0,
        };
        let mut frame = WorldToneCaptureFrame::from_geometry(sample, &[receiver]).unwrap();
        let mut holder = 0.0;
        let mut ledger = crate::state::EnergyLedger::default();
        assert!(frame
            .settle_receiver(0, 1.0, 1.0, &mut holder, &mut ledger)
            .is_some());
        assert!(frame
            .settle_receiver(0, 1.0, 1.0, &mut holder, &mut ledger)
            .is_none());
        assert!((holder - 20.0).abs() < 1e-7);
        assert!((ledger.total_external_energy_input - 20.0).abs() < 1e-7);
    }

    #[test]
    fn world_tone_capture_does_not_credit_a_different_shadowed_body() {
        use crate::organism_geometry::{OrganismBodyGeometry, PlacedForm};
        use crate::resources::Form;
        let body = |x| OrganismBodyGeometry {
            parts: vec![PlacedForm {
                unit_index: 0,
                form: Form::Rectangle {
                    width: 2.0,
                    height: 2.0,
                },
                x,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            min_x: x - 1.0,
            max_x: x + 1.0,
            min_y: -1.0,
            max_y: 1.0,
        };
        let bodies = [body(5.0), body(10.0)];
        let sample = crate::world_tone::WorldToneSample {
            source_x: -5.0,
            source_y: 0.0,
            frequency_hz: 440.0,
            signal_energy_level: 8.0,
            energy_intensity_per_tick_per_width: 10.0,
            orbital_phase: 0.0,
        };
        let mut frame = WorldToneCaptureFrame::from_geometry(sample, &bodies).unwrap();
        let mut holder = 1.0;
        let mut ledger = crate::state::EnergyLedger::default();
        let result = frame
            .settle_receiver(1, 1.0, 1.0, &mut holder, &mut ledger)
            .unwrap();
        assert_eq!(result.projected_width, 0.0);
        assert_eq!(result.incident_energy, 0.0);
        assert_eq!(result.absorbed_energy, 0.0);
        assert_eq!(holder, 1.0);
        assert_eq!(ledger.total_external_energy_input, 0.0);
    }

    #[test]
    fn world_tone_capture_rejects_invalid_fractions_without_credit_or_claim() {
        use crate::organism_geometry::{OrganismBodyGeometry, PlacedForm};
        use crate::resources::Form;
        let receiver = OrganismBodyGeometry {
            parts: vec![PlacedForm {
                unit_index: 0,
                form: Form::Rectangle {
                    width: 2.0,
                    height: 2.0,
                },
                x: 5.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            min_x: 4.0,
            max_x: 6.0,
            min_y: -1.0,
            max_y: 1.0,
        };
        let sample = crate::world_tone::WorldToneSample {
            source_x: -5.0,
            source_y: 0.0,
            frequency_hz: 440.0,
            signal_energy_level: 8.0,
            energy_intensity_per_tick_per_width: 10.0,
            orbital_phase: 0.0,
        };
        let mut frame = WorldToneCaptureFrame::from_geometry(sample, &[receiver]).unwrap();
        let mut holder = 1.0;
        let mut ledger = crate::state::EnergyLedger::default();
        assert!(frame
            .settle_receiver(0, f64::NAN, 0.8, &mut holder, &mut ledger)
            .is_none());
        assert_eq!(holder, 1.0);
        assert_eq!(ledger.total_external_energy_input, 0.0);
        assert!(frame
            .settle_receiver(0, 0.5, 0.8, &mut holder, &mut ledger)
            .is_some());
    }
}
