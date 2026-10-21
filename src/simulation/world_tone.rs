#![allow(dead_code)] // Lower-level receiver APIs remain useful for focused energy-flow tests.
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
    /// Default 440 Hz environmental source for a world with the supplied bounds.
    /// Its orbit radius is derived from the world dimensions so the source remains
    /// outside the full rectangle rather than relying on a fixed scene size.
    pub(crate) fn default_for_world(world_width: f64, world_height: f64) -> Option<Self> {
        let config = Self {
            parameters: WorldToneParameters {
                minimum_energy_intensity_per_tick_per_width: 1.0,
                maximum_energy_intensity_per_tick_per_width: 10.0,
                orbital_period_ticks: 2_400,
                envelope_curve: EnvelopeCurve::Cosine,
            },
            orbit_radius: world_width.max(world_height),
        };
        config.sample(0, world_width, world_height).map(|_| config)
    }

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

        // The source must remain outside the entire rectangular world, not
        // merely outside its center or one edge. A circular orbit whose radius
        // exceeds half the world diagonal is strictly outside all four corners.
        let minimum_outside_radius = 0.5 * world_width.hypot(world_height);
        if !minimum_outside_radius.is_finite() || self.orbit_radius <= minimum_outside_radius {
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
    /// Signal strength, expressed as energy per tick per unit of exposed projected width.
    /// This is the source budget before material interaction, not energy already captured.
    pub(crate) energy_intensity_per_tick_per_width: f64,
    /// Normalized orbital phase: 0 at north, increasing eastward, in [0, 1).
    pub(crate) orbital_phase: f64,
}
/// Energy split for one material interaction with the incoming world tone.
///
/// The unabsorbed amount remains in the continuing signal budget; it is neither
/// lost nor credited again. Any onward propagation must explicitly account for
/// attenuation and frequency-composition changes. This model does not use literal
/// reflection or bouncing rays.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WorldToneInteraction {
    pub(crate) incident_energy: f64,
    pub(crate) absorbed_energy: f64,
    pub(crate) unabsorbed_energy: f64,
}

impl WorldToneSample {
    /// Reject manually constructed samples that violate the source contract.
    ///
    /// Runtime-configured samples already satisfy these bounds; checking again
    /// at the capture boundary prevents test, deserialized, or future internal
    /// callers from crediting energy from an invalid source state.
    pub(crate) fn is_valid(self) -> bool {
        self.source_x.is_finite()
            && self.source_y.is_finite()
            && self.frequency_hz.is_finite()
            && self.frequency_hz > 0.0
            && self.energy_intensity_per_tick_per_width.is_finite()
            && (1.0..=10.0).contains(&self.energy_intensity_per_tick_per_width)
            && self.orbital_phase.is_finite()
            && (0.0..1.0).contains(&self.orbital_phase)
    }

    /// Energy incident on an exposed projected width during this tick.
    ///
    /// Visibility/shadowing must be applied when computing the width; this
    /// helper does not discover exposed geometry.
    pub(crate) fn incident_energy_for_projected_width(self, projected_width: f64) -> Option<f64> {
        if !self.energy_intensity_per_tick_per_width.is_finite()
            || !(1.0..=10.0).contains(&self.energy_intensity_per_tick_per_width)
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
/// Unabsorbed incident energy remains in the continuing signal budget and must
/// not be credited as a new source when it reaches downstream material. The
/// intended onward interaction may shift frequency; it is not literal reflection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WorldToneCaptureResult {
    pub(crate) projected_width: f64,
    pub(crate) incident_energy: f64,
    pub(crate) absorbed_energy: f64,
    pub(crate) unabsorbed_energy: f64,
    pub(crate) usable_energy: f64,
    pub(crate) heat_dissipated: f64,
}

/// Energy response of one exposed material constituent to the current source sample.
/// Signal strength is the energy budget; material damping partitions it between
/// absorbed energy and the remaining signal. The remainder is not a second source.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WorldToneSurfaceResponse {
    pub(crate) projected_width: f64,
    pub(crate) incident_energy: f64,
    pub(crate) absorption_fraction: f64,
    pub(crate) absorbed_by_damping: f64,
    pub(crate) unabsorbed: f64,
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
    /// Per-constituent claims are tracked separately from aggregate-body claims.
    settled_units: Vec<Vec<usize>>,
}

impl WorldToneCaptureFrame {
    /// Discover polygonal visibility and cache each body's unique projected width.
    /// Unsupported geometry fails closed; callers should rebuild this frame next tick.
    pub(crate) fn from_geometry(
        sample: crate::world_tone::WorldToneSample,
        bodies: &[crate::organism_geometry::OrganismBodyGeometry],
    ) -> Option<Self> {
        let receiver_body_indices: Vec<_> = (0..bodies.len()).collect();
        Self::from_geometry_for_receivers(sample, bodies, &receiver_body_indices)
    }

    /// Build one world occlusion frame but solve exact exposure only for the
    /// selected receiver bodies. All supplied bodies remain potential occluders.
    pub(crate) fn from_geometry_for_receivers(
        sample: crate::world_tone::WorldToneSample,
        bodies: &[crate::organism_geometry::OrganismBodyGeometry],
        receiver_body_indices: &[usize],
    ) -> Option<Self> {
        if !sample.is_valid()
            || bodies
                .iter()
                .any(|body| body.contains_point(sample.source_x, sample.source_y))
        {
            return None;
        }

        let phase = std::f64::consts::TAU * sample.orbital_phase;
        let toward_source = (phase.sin(), -phase.cos());
        let projected_widths_by_unit =
            crate::source_exposure::parallel_projected_widths_by_body_for_targets(
                toward_source,
                bodies,
                receiver_body_indices,
            )?;
        let projected_widths: Vec<f64> = projected_widths_by_unit
            .iter()
            .map(|units| units.iter().map(|(_, width)| *width).sum())
            .collect();

        if projected_widths.iter().any(|width| !width.is_finite())
            || projected_widths_by_unit
                .iter()
                .flatten()
                .any(|(_, width)| !width.is_finite())
        {
            return None;
        }

        Some(Self {
            sample,
            projected_widths,
            projected_widths_by_unit,
            settled_receivers: vec![false; bodies.len()],
            settled_units: vec![Vec::new(); bodies.len()],
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
        units
            .iter()
            .find(|(candidate, _)| *candidate == unit_index)
            .map(|(_, width)| *width)
    }

    /// Evaluate one constituent using its existing catalog-normalized material
    /// resonance and damping properties. This is a read-only response: callers
    /// must not credit energy until the absorbed/reflected/usable split is settled.
    pub(crate) fn surface_response_for_unit(
        &self,
        receiver_body_index: usize,
        unit_index: usize,
        properties: crate::resources::ResourceProperties,
        baselines: crate::resources::ResourceBaselines,
    ) -> Option<WorldToneSurfaceResponse> {
        let projected_width = self.projected_width_for_unit(receiver_body_index, unit_index)?;
        let incident_energy = self
            .sample
            .incident_energy_for_projected_width(projected_width)?;
        let natural_frequency = crate::harmonics::natural_frequency_hz(properties, baselines);
        let damping = crate::harmonics::damping_ratio(properties, baselines);
        let absorption_fraction = crate::harmonics::resonant_absorption_fraction(
            self.sample.frequency_hz,
            natural_frequency,
            damping,
        );
        let absorbed_by_damping = incident_energy * absorption_fraction;
        let unabsorbed = incident_energy - absorbed_by_damping;
        if [absorption_fraction, absorbed_by_damping, unabsorbed]
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
        {
            return None;
        }
        Some(WorldToneSurfaceResponse {
            projected_width,
            incident_energy,
            absorption_fraction,
            absorbed_by_damping,
            unabsorbed,
        })
    }
    /// Settle one constituent's response using an explicit downstream usable fraction.
    /// This prevents unlike materials on the same body from being forced into one
    /// averaged absorption response. The caller must supply the retention decision;
    /// this method deliberately does not invent one.
    pub(crate) fn settle_surface_response(
        &mut self,
        receiver_body_index: usize,
        unit_index: usize,
        properties: crate::resources::ResourceProperties,
        baselines: crate::resources::ResourceBaselines,
        usable_fraction: f64,
        holder: &mut f64,
        ledger: &mut crate::state::EnergyLedger,
    ) -> Option<WorldToneCaptureResult> {
        if *self.settled_receivers.get(receiver_body_index)?
            || !usable_fraction.is_finite()
            || !(0.0..=1.0).contains(&usable_fraction)
            || self
                .settled_units
                .get(receiver_body_index)?
                .contains(&unit_index)
        {
            return None;
        }
        let response =
            self.surface_response_for_unit(receiver_body_index, unit_index, properties, baselines)?;
        let usable_energy = response.absorbed_by_damping * usable_fraction;
        let heat_dissipated = response.absorbed_by_damping - usable_energy;
        if !usable_energy.is_finite()
            || !heat_dissipated.is_finite()
            || usable_energy < 0.0
            || heat_dissipated < 0.0
        {
            return None;
        }
        let transaction = ExternalInputTransaction {
            source: ExternalEnergySource::WorldTone,
            source_available: response.incident_energy,
            external_input: response.absorbed_by_damping,
            usable_delta: usable_energy,
            work_expended: 0.0,
            heat_dissipated,
        };
        // Validate the diagnostic accumulator before mutating the holder or
        // ledger, so an overflow cannot leave a partially settled interaction.
        let next_unabsorbed = ledger.total_world_tone_energy_unabsorbed + response.unabsorbed;
        if !next_unabsorbed.is_finite() || next_unabsorbed < 0.0 {
            return None;
        }
        if !ledger.settle_external_input(holder, transaction) {
            return None;
        }
        ledger.total_world_tone_energy_unabsorbed = next_unabsorbed;
        self.settled_units
            .get_mut(receiver_body_index)?
            .push(unit_index);
        Some(WorldToneCaptureResult {
            projected_width: response.projected_width,
            incident_energy: response.incident_energy,
            absorbed_energy: response.absorbed_by_damping,
            unabsorbed_energy: response.unabsorbed,
            usable_energy,
            heat_dissipated,
        })
    }
    /// Settle one organism constituent using the existing material spectrum.
    /// The material response determines the intercepted resonant-energy budget;
    /// its damping ratio partitions that response into retained resonance and
    /// heat. Energy outside the response continues as signal. No genome-selected
    /// efficiency or arbitrary retention coefficient is introduced.
    pub(crate) fn settle_organism_surface_response(
        &mut self,
        receiver_body_index: usize,
        unit_index: usize,
        properties: crate::resources::ResourceProperties,
        baselines: crate::resources::ResourceBaselines,
        organism: &mut crate::state::Organism,
        ledger: &mut crate::state::EnergyLedger,
    ) -> Option<WorldToneCaptureResult> {
        if *self.settled_receivers.get(receiver_body_index)?
            || self
                .settled_units
                .get(receiver_body_index)?
                .contains(&unit_index)
        {
            return None;
        }
        let response =
            self.surface_response_for_unit(receiver_body_index, unit_index, properties, baselines)?;

        // Determine how much incident energy the existing material response
        // represents before damping partitions it into storage and heat.
        let incident_amplitude = response.incident_energy.sqrt();
        let response_spectrum = crate::harmonics::material_response(properties, baselines, 0.0);
        let mut response_energy: f64 = 0.0;
        for component in &response_spectrum.components {
            let amplitude = component.amplitude * incident_amplitude;
            let energy = amplitude * amplitude;
            if !energy.is_finite() || energy < 0.0 {
                return None;
            }
            response_energy += energy;
            if !response_energy.is_finite() {
                return None;
            }
        }
        response_energy = response_energy.min(response.incident_energy);

        // Damping acts on the material's resonant response, not on the entire
        // incident signal. The response budget not dissipated by damping is
        // available for persistent frequency-resolved resonance.
        let damping = crate::harmonics::damping_ratio(properties, baselines);
        if !damping.is_finite() || damping < 0.0 {
            return None;
        }
        let damping_heat = response_energy * damping.clamp(0.0, 1.0);
        let retained_budget = response_energy - damping_heat;
        let incoming_signal = crate::harmonics::ToneSpectrum {
            components: vec![crate::harmonics::ToneComponent {
                frequency_hz: self.sample.frequency_hz,
                amplitude: incident_amplitude,
                phase_radians: 0.0,
            }],
        };
        let spectrum = crate::harmonics::transform_signal_to_energy_budget(
            properties,
            baselines,
            &incoming_signal,
            retained_budget,
        )?;

        // Deposit the whole retained spectrum atomically. If the bounded
        // reservoir cannot represent it, dissipate the entire response as heat.
        let mut reservoir = organism.resonant_energy.clone();
        let mut deposit_ok = true;
        for component in &spectrum.components {
            let energy = component.amplitude * component.amplitude;
            if energy > f64::EPSILON && !reservoir.deposit(component.frequency_hz, energy) {
                deposit_ok = false;
                break;
            }
        }
        let retained_energy = if deposit_ok { retained_budget } else { 0.0 };
        let heat_dissipated = if deposit_ok {
            damping_heat
        } else {
            response_energy
        };
        let unabsorbed_energy = response.incident_energy - response_energy;
        if !retained_energy.is_finite()
            || !heat_dissipated.is_finite()
            || !unabsorbed_energy.is_finite()
            || retained_energy < 0.0
            || heat_dissipated < 0.0
            || unabsorbed_energy < -1e-9
        {
            return None;
        }
        let unabsorbed_energy = unabsorbed_energy.max(0.0);
        let next_unabsorbed = ledger.total_world_tone_energy_unabsorbed + unabsorbed_energy;
        if !next_unabsorbed.is_finite() || next_unabsorbed < 0.0 {
            return None;
        }
        let transaction = ExternalInputTransaction {
            source: ExternalEnergySource::WorldTone,
            source_available: response.incident_energy,
            external_input: response_energy,
            usable_delta: retained_energy,
            work_expended: 0.0,
            heat_dissipated,
        };
        let usable_before = organism.usable_energy;
        if !ledger.settle_external_input(&mut organism.usable_energy, transaction) {
            return None;
        }
        // The transaction temporarily credits the usable holder; move that exact
        // retained amount into the reservoir so inventory categories stay exclusive.
        organism.usable_energy = usable_before;
        if deposit_ok {
            organism.resonant_energy = reservoir;
        }
        ledger.total_world_tone_energy_unabsorbed = next_unabsorbed;
        self.settled_units
            .get_mut(receiver_body_index)?
            .push(unit_index);

        Some(WorldToneCaptureResult {
            projected_width: response.projected_width,
            incident_energy: response.incident_energy,
            absorbed_energy: response_energy,
            unabsorbed_energy,
            usable_energy: retained_energy,
            heat_dissipated,
        })
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
            || !self.settled_units.get(receiver_body_index)?.is_empty()
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
        // Keep unabsorbed incident energy visible in the ledger's throughput
        // diagnostics without misclassifying it as stored energy or heat.
        let next_unabsorbed = ledger.total_world_tone_energy_unabsorbed + unabsorbed_energy;
        if !next_unabsorbed.is_finite() || next_unabsorbed < 0.0 {
            return None;
        }
        if !ledger.settle_external_input(holder, transaction) {
            return None;
        }
        ledger.total_world_tone_energy_unabsorbed = next_unabsorbed;
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
        self.minimum_energy_intensity_per_tick_per_width.is_finite()
            && self.minimum_energy_intensity_per_tick_per_width >= 1.0
            && self.maximum_energy_intensity_per_tick_per_width.is_finite()
            && self.maximum_energy_intensity_per_tick_per_width <= 10.0
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
            minimum_energy_intensity_per_tick_per_width: 1.0,
            maximum_energy_intensity_per_tick_per_width: 10.0,
            orbital_period_ticks: 2_400,
            envelope_curve: EnvelopeCurve::Cosine,
        }
    }

    fn circle_body(
        unit_index: usize,
        x: f64,
        y: f64,
        radius: f64,
    ) -> crate::organism_geometry::OrganismBodyGeometry {
        crate::organism_geometry::OrganismBodyGeometry {
            parts: vec![crate::organism_geometry::PlacedForm {
                unit_index,
                form: crate::resources::Form::Circle { radius },
                x,
                y,
                rotation_radians: 0.0,
            }],
            min_x: x - radius,
            max_x: x + radius,
            min_y: y - radius,
            max_y: y + radius,
        }
    }

    // Parallel-ray projection of the exposed x=4 face spanning y=[-1, 1].
    fn expected_front_face_projected_width() -> f64 {
        2.0
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
    fn frequency_stays_440_while_signal_strength_tracks_energy_intensity() {
        let params = parameters();
        let north = params.sample(0, 0.0, 0.0, 1.0).unwrap();
        let east = params.sample(600, 0.0, 0.0, 1.0).unwrap();
        let south = params.sample(1_200, 0.0, 0.0, 1.0).unwrap();
        let west = params.sample(1_800, 0.0, 0.0, 1.0).unwrap();

        assert_eq!(north.frequency_hz, WORLD_TONE_FREQUENCY_HZ);
        assert_eq!(north.energy_intensity_per_tick_per_width, 10.0);
        assert_eq!(south.frequency_hz, WORLD_TONE_FREQUENCY_HZ);
        assert_eq!(south.energy_intensity_per_tick_per_width, 1.0);
        assert_eq!(east.frequency_hz, WORLD_TONE_FREQUENCY_HZ);
        assert_eq!(west.frequency_hz, WORLD_TONE_FREQUENCY_HZ);
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
    fn invalid_manual_world_tone_samples_fail_before_geometry_is_cached() {
        let valid = parameters().sample(0, 0.0, 0.0, 1.0).unwrap();
        assert!(valid.is_valid());
        for sample in [
            WorldToneSample {
                frequency_hz: 0.0,
                ..valid
            },
            WorldToneSample {
                frequency_hz: f64::NAN,
                ..valid
            },
            WorldToneSample {
                orbital_phase: 1.0,
                ..valid
            },
            WorldToneSample {
                source_x: f64::INFINITY,
                ..valid
            },
        ] {
            assert!(!sample.is_valid());
        }
    }

    #[test]
    fn incident_energy_rejects_out_of_contract_intensity_even_for_manual_samples() {
        let valid = parameters().sample(0, 0.0, 0.0, 1.0).unwrap();
        assert_eq!(valid.incident_energy_for_projected_width(1.0), Some(10.0));

        for intensity in [0.0, 0.999, 10.001, 11.0, f64::NAN, f64::INFINITY] {
            let sample = WorldToneSample {
                energy_intensity_per_tick_per_width: intensity,
                ..valid
            };
            assert_eq!(
                sample.incident_energy_for_projected_width(1.0),
                None,
                "out-of-contract intensity {intensity:?} must not be credited"
            );
        }
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
            orbit_radius: 700.0,
        };
        let north = config.sample(0, 1_000.0, 800.0).unwrap();
        let east = config.sample(600, 1_000.0, 800.0).unwrap();
        assert_eq!((north.source_x, north.source_y), (500.0, -300.0));
        assert!((east.source_x - 1_200.0).abs() < 1e-9);
        assert!((east.source_y - 400.0).abs() < 1e-9);

        let invalid = WorldToneRuntimeConfig {
            orbit_radius: -1.0,
            ..config
        };
        assert!(invalid.sample(0, 1_000.0, 800.0).is_none());

        let inside_world = WorldToneRuntimeConfig {
            orbit_radius: 250.0,
            ..config
        };
        assert!(
            inside_world.sample(0, 1_000.0, 800.0).is_none(),
            "the source orbit must clear the entire world rectangle, including corners"
        );
    }

    #[test]
    fn precomputed_cycle_matches_direct_samples_for_every_tick_and_wraps() {
        let config = WorldToneRuntimeConfig {
            parameters: parameters(),
            orbit_radius: 700.0,
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
            orbit_radius: 100.0,
        };
        assert!(config.is_valid());
        assert!(WorldToneCycle::precompute(config, 100.0, 100.0).is_none());
        assert!(config.sample(0, 100.0, 100.0).is_some());
    }

    #[test]
    fn energy_intensity_is_constrained_to_the_approved_one_to_ten_range() {
        let mut params = parameters();
        params.minimum_energy_intensity_per_tick_per_width = 0.999;
        assert!(
            !params.is_valid(),
            "intensity below 1 is outside the approved range"
        );

        params = parameters();
        params.maximum_energy_intensity_per_tick_per_width = 10.001;
        assert!(
            !params.is_valid(),
            "intensity above 10 is outside the approved range"
        );

        params = parameters();
        assert!(params.is_valid(), "the exact 1-to-10 range is valid");
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
    fn standalone_circle_uses_parallel_projection_without_polygon_approximation() {
        use crate::organism_geometry::{OrganismBodyGeometry, PlacedForm};
        use crate::resources::Form;

        let body = OrganismBodyGeometry {
            parts: vec![PlacedForm {
                unit_index: 7,
                form: Form::Circle { radius: 1.25 },
                x: 5.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            min_x: 3.75,
            max_x: 6.25,
            min_y: -1.25,
            max_y: 1.25,
        };
        let sample = crate::world_tone::WorldToneSample {
            source_x: -5.0,
            source_y: 0.0,
            frequency_hz: 440.0,
            energy_intensity_per_tick_per_width: 10.0,
            orbital_phase: 0.75,
        };
        let expected = 2.5;
        let frame = WorldToneCaptureFrame::from_geometry(sample, &[body])
            .expect("an isolated, externally illuminated circle has parallel-ray exposure");
        assert!(expected > 0.0 && expected <= 2.5 + 1e-12);
        assert!((frame.projected_widths[0] - expected).abs() < 0.02);
        assert!((frame.projected_width_for_unit(0, 7).unwrap() - expected).abs() < 0.02);
    }

    #[test]
    fn standalone_legacy_fluid_uses_parallel_projection_of_nominal_circle() {
        use crate::organism_geometry::{OrganismBodyGeometry, PlacedForm};
        use crate::resources::Form;

        let area = 0.5;
        let radius = (area / std::f64::consts::PI).sqrt();
        let body = OrganismBodyGeometry {
            parts: vec![PlacedForm {
                unit_index: 3,
                form: Form::Fluid {
                    nominal_area: area,
                    boundary: None,
                    arc_boundary: None,
                    multi_contact_boundary: None,
                },
                x: 5.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            min_x: 5.0 - radius,
            max_x: 5.0 + radius,
            min_y: -radius,
            max_y: radius,
        };
        let sample = crate::world_tone::WorldToneSample {
            source_x: -5.0,
            source_y: 0.0,
            frequency_hz: 440.0,
            energy_intensity_per_tick_per_width: 10.0,
            orbital_phase: 0.75,
        };
        let expected = 2.0 * radius;
        let frame = WorldToneCaptureFrame::from_geometry(sample, &[body])
            .expect("legacy circular fluid has a parallel-ray realization");
        assert!((frame.projected_width_for_unit(0, 3).unwrap() - expected).abs() < 0.02);
    }

    #[test]
    fn standalone_circular_fluid_arc_uses_parallel_projection() {
        use crate::capillary_geometry::CircularArcBoundary;
        use crate::organism_geometry::{OrganismBodyGeometry, PlacedForm};
        use crate::resources::Form;

        let arc = CircularArcBoundary {
            center_x: 0.0,
            center_y: 0.0,
            radius: 2.0,
            start_angle_radians: 3.0 * std::f64::consts::FRAC_PI_4,
            sweep_radians: std::f64::consts::FRAC_PI_2,
        };
        let area = arc.segment_area().unwrap();
        let body = OrganismBodyGeometry {
            parts: vec![PlacedForm {
                unit_index: 2,
                form: Form::Fluid {
                    nominal_area: area,
                    boundary: None,
                    arc_boundary: Some(arc),
                    multi_contact_boundary: None,
                },
                x: 5.0,
                y: 0.0,
                rotation_radians: 0.25,
            }],
            min_x: 3.0,
            max_x: 7.0,
            min_y: -2.0,
            max_y: 2.0,
        };
        let sample = crate::world_tone::WorldToneSample {
            source_x: -5.0,
            source_y: 0.0,
            frequency_hz: 440.0,
            energy_intensity_per_tick_per_width: 10.0,
            orbital_phase: 0.75,
        };
        let frame = WorldToneCaptureFrame::from_geometry(sample, &[body])
            .expect("an isolated exact circular free boundary can be integrated analytically");
        let expected_world_arc = CircularArcBoundary {
            center_x: 5.0,
            center_y: 0.0,
            start_angle_radians: arc.start_angle_radians + 0.25,
            ..arc
        };
        let expected =
            crate::source_exposure::parallel_projected_arc_extent(expected_world_arc, (-1.0, 0.0))
                .unwrap();
        assert!((frame.projected_widths[0] - expected).abs() < 0.02);
        assert!((frame.projected_width_for_unit(0, 2).unwrap() - expected).abs() < 0.02);
    }

    #[test]
    fn mixed_frame_supports_a_circle_receiver_shadowed_by_a_polygon() {
        use crate::organism_geometry::{OrganismBodyGeometry, PlacedForm};
        use crate::resources::Form;

        let circle = OrganismBodyGeometry {
            parts: vec![PlacedForm {
                unit_index: 0,
                form: Form::Circle { radius: 1.0 },
                x: 5.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            min_x: 4.0,
            max_x: 6.0,
            min_y: -1.0,
            max_y: 1.0,
        };
        let blocker = OrganismBodyGeometry {
            parts: vec![PlacedForm {
                unit_index: 0,
                form: Form::Rectangle {
                    width: 1.0,
                    height: 1.0,
                },
                x: 2.5,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            min_x: 2.0,
            max_x: 3.0,
            min_y: -0.5,
            max_y: 0.5,
        };
        let sample = crate::world_tone::WorldToneSample {
            source_x: -5.0,
            source_y: 0.0,
            frequency_hz: 440.0,
            energy_intensity_per_tick_per_width: 10.0,
            orbital_phase: 0.75,
        };
        let frame = WorldToneCaptureFrame::from_geometry(sample, &[circle.clone(), blocker])
            .expect("mixed curved receivers and polygonal occluders are supported");
        let unoccluded = WorldToneCaptureFrame::from_geometry(sample, &[circle])
            .expect("isolated circle visibility is supported");
        assert!(frame.projected_widths[0] > 0.0);
        assert!(frame.projected_widths[0] < unoccluded.projected_widths[0]);
    }

    #[test]
    fn mixed_frame_supports_a_straight_receiver_shadowed_by_a_circle() {
        use crate::organism_geometry::{OrganismBodyGeometry, PlacedForm};
        use crate::resources::Form;

        let sample = crate::world_tone::WorldToneSample {
            source_x: -5.0,
            source_y: 0.0,
            frequency_hz: 440.0,
            energy_intensity_per_tick_per_width: 10.0,
            orbital_phase: 0.75,
        };
        let blocker = circle_body(0, 0.0, 0.0, 0.5);
        let receiver = OrganismBodyGeometry {
            parts: vec![PlacedForm {
                unit_index: 1,
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
        let frame = WorldToneCaptureFrame::from_geometry(sample, &[blocker, receiver.clone()])
            .expect("circle blockers split straight receiver edges analytically");
        let unoccluded = WorldToneCaptureFrame::from_geometry(sample, &[receiver])
            .expect("isolated straight receiver is supported");
        assert!(frame.projected_widths[1] > 0.0);
        assert!(frame.projected_widths[1] < unoccluded.projected_widths[0]);
    }

    #[test]
    fn curved_only_frame_fully_shadows_circle_behind_circle() {
        let sample = crate::world_tone::WorldToneSample {
            source_x: -5.0,
            source_y: 0.0,
            frequency_hz: 440.0,
            energy_intensity_per_tick_per_width: 10.0,
            orbital_phase: 0.75,
        };
        let blocker = circle_body(0, 0.0, 0.0, 1.0);
        let target = circle_body(0, 5.0, 0.0, 1.0);
        let frame = WorldToneCaptureFrame::from_geometry(sample, &[blocker, target])
            .expect("circle boundaries share one parallel-ray field");
        assert!((frame.projected_widths[0] - 2.0).abs() < 0.02);
        assert_eq!(frame.projected_widths[1], 0.0);
    }

    #[test]
    fn curved_only_frame_preserves_partial_circle_shadow() {
        let sample = crate::world_tone::WorldToneSample {
            source_x: -5.0,
            source_y: 0.0,
            frequency_hz: 440.0,
            energy_intensity_per_tick_per_width: 10.0,
            orbital_phase: 0.75,
        };
        let blocker = circle_body(0, 0.0, 0.7, 0.5);
        let target = circle_body(0, 5.0, 0.0, 1.0);
        let frame = WorldToneCaptureFrame::from_geometry(sample, &[blocker, target])
            .expect("circle boundaries share one parallel-ray field");
        assert!(frame.projected_widths[1] > 0.0);
        assert!(frame.projected_widths[1] < 2.0);
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
            energy_intensity_per_tick_per_width: 10.0,
            orbital_phase: 0.75,
        };
        let mut frame = WorldToneCaptureFrame::from_geometry(sample, &[receiver])
            .expect("valid polygonal receiver should produce exposure frame");
        let expected_width = expected_front_face_projected_width();
        let mut holder = 1.0;
        let mut ledger = crate::state::EnergyLedger::default();
        let result = frame
            .settle_receiver(0, 0.5, 0.8, &mut holder, &mut ledger)
            .expect("valid receiver should capture");
        assert!((result.projected_width - expected_width).abs() < 1e-8);
        assert!((frame.projected_width_for_unit(0, 0).unwrap() - expected_width).abs() < 1e-8);
        assert!((result.incident_energy - 10.0 * expected_width).abs() < 1e-7);
        assert!((result.absorbed_energy - 5.0 * expected_width).abs() < 1e-7);
        assert!((result.unabsorbed_energy - 5.0 * expected_width).abs() < 1e-7);
        assert!((result.usable_energy - 4.0 * expected_width).abs() < 1e-7);
        assert!((result.heat_dissipated - expected_width).abs() < 1e-7);
        assert!((holder - (1.0 + 4.0 * expected_width)).abs() < 1e-7);
        assert!((ledger.total_external_energy_input - 5.0 * expected_width).abs() < 1e-7);
        assert!((ledger.total_world_tone_energy_unabsorbed - 5.0 * expected_width).abs() < 1e-7);
        assert!((ledger.total_usable_energy_gained - 4.0 * expected_width).abs() < 1e-7);
        assert!((ledger.total_heat_dissipated - expected_width).abs() < 1e-7);
    }

    #[test]
    fn world_tone_capture_preserves_surface_exposure_by_constituent() {
        use crate::organism_geometry::{OrganismBodyGeometry, PlacedForm};
        use crate::resources::Form;

        let body = OrganismBodyGeometry {
            parts: vec![
                PlacedForm {
                    unit_index: 0,
                    form: Form::Rectangle {
                        width: 2.0,
                        height: 2.0,
                    },
                    x: 5.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
                PlacedForm {
                    unit_index: 1,
                    form: Form::Rectangle {
                        width: 2.0,
                        height: 2.0,
                    },
                    x: 7.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
            ],
            min_x: 4.0,
            max_x: 8.0,
            min_y: -1.0,
            max_y: 1.0,
        };
        let sample = crate::world_tone::WorldToneSample {
            source_x: -5.0,
            source_y: 0.0,
            frequency_hz: 440.0,
            energy_intensity_per_tick_per_width: 10.0,
            orbital_phase: 0.75,
        };
        let expected_width = expected_front_face_projected_width();
        let frame = WorldToneCaptureFrame::from_geometry(sample, &[body]).unwrap();

        assert!((frame.projected_width_for_unit(0, 0).unwrap() - expected_width).abs() < 1e-7);
        assert_eq!(frame.projected_width_for_unit(0, 1), Some(0.0));
        assert_eq!(frame.projected_width_for_unit(0, 99), None);
        assert_eq!(frame.projected_width_for_unit(99, 0), None);
        assert!((frame.projected_widths[0] - expected_width).abs() < 1e-7);
    }

    #[test]
    fn surface_response_uses_existing_material_resonance_for_energy_split() {
        use crate::organism_geometry::{OrganismBodyGeometry, PlacedForm};
        use crate::resources::{Form, ResourceBaselines, ResourceProperties};
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
            energy_intensity_per_tick_per_width: 10.0,
            orbital_phase: 0.75,
        };
        let expected_width = expected_front_face_projected_width();
        let frame = WorldToneCaptureFrame::from_geometry(sample, &[receiver]).unwrap();
        let properties = ResourceProperties {
            mass: 1.0,
            potential_energy: 1.0,
            reactivity: 1.0,
            chemical_position: Some(7.0),
            cohesion: 1.0,
        };
        let baselines = ResourceBaselines {
            mass: 1.0,
            potential_energy: 1.0,
            reactivity: 1.0,
            cohesion: 1.0,
        };
        let response = frame
            .surface_response_for_unit(0, 0, properties, baselines)
            .unwrap();
        assert!((response.projected_width - expected_width).abs() < 1e-7);
        assert!((response.incident_energy - 10.0 * expected_width).abs() < 1e-7);
        assert!((response.absorption_fraction - 1.0).abs() < 1e-7);
        assert!((response.absorbed_by_damping - 10.0 * expected_width).abs() < 1e-7);
        assert!(response.unabsorbed.abs() < 1e-7);
        assert!(
            (response.absorbed_by_damping + response.unabsorbed - response.incident_energy).abs()
                < 1e-7
        );
    }
    #[test]
    fn surface_settlement_uses_per_material_response_and_prevents_double_credit() {
        use crate::organism_geometry::{OrganismBodyGeometry, PlacedForm};
        use crate::resources::{Form, ResourceBaselines, ResourceProperties};
        let body = OrganismBodyGeometry {
            parts: vec![
                PlacedForm {
                    unit_index: 0,
                    form: Form::Rectangle {
                        width: 2.0,
                        height: 2.0,
                    },
                    x: 5.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
                PlacedForm {
                    unit_index: 1,
                    form: Form::Rectangle {
                        width: 2.0,
                        height: 2.0,
                    },
                    x: 7.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
            ],
            min_x: 4.0,
            max_x: 8.0,
            min_y: -1.0,
            max_y: 1.0,
        };
        let sample = crate::world_tone::WorldToneSample {
            source_x: -5.0,
            source_y: 0.0,
            frequency_hz: 440.0,
            energy_intensity_per_tick_per_width: 10.0,
            orbital_phase: 0.75,
        };
        let expected_width = expected_front_face_projected_width();
        let mut frame = WorldToneCaptureFrame::from_geometry(sample, &[body]).unwrap();
        let properties = ResourceProperties {
            mass: 1.0,
            potential_energy: 1.0,
            reactivity: 1.0,
            chemical_position: Some(7.0),
            cohesion: 1.0,
        };
        let baselines = ResourceBaselines {
            mass: 1.0,
            potential_energy: 1.0,
            reactivity: 1.0,
            cohesion: 1.0,
        };
        let mut holder = 0.0;
        let mut ledger = crate::state::EnergyLedger::default();
        let result = frame
            .settle_surface_response(0, 0, properties, baselines, 0.75, &mut holder, &mut ledger)
            .unwrap();
        assert!((result.incident_energy - 10.0 * expected_width).abs() < 1e-7);
        assert!((result.absorbed_energy - 10.0 * expected_width).abs() < 1e-7);
        assert!((result.usable_energy - 7.5 * expected_width).abs() < 1e-7);
        assert!((result.heat_dissipated - 2.5 * expected_width).abs() < 1e-7);
        assert!((holder - 7.5 * expected_width).abs() < 1e-7);
        assert!((ledger.total_external_energy_input - 10.0 * expected_width).abs() < 1e-7);
        assert!((ledger.total_usable_energy_gained - 7.5 * expected_width).abs() < 1e-7);
        assert!((ledger.total_heat_dissipated - 2.5 * expected_width).abs() < 1e-7);
        assert!(frame
            .settle_surface_response(0, 0, properties, baselines, 0.75, &mut holder, &mut ledger,)
            .is_none());
        assert!(frame
            .settle_receiver(0, 1.0, 1.0, &mut holder, &mut ledger)
            .is_none());
        assert!((holder - 7.5 * expected_width).abs() < 1e-7);
    }
    #[test]
    fn partial_absorption_accounts_for_remaining_signal_without_double_credit() {
        use crate::organism_geometry::{OrganismBodyGeometry, PlacedForm};
        use crate::resources::{Form, ResourceBaselines, ResourceProperties};

        let body = OrganismBodyGeometry {
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
            energy_intensity_per_tick_per_width: 10.0,
            orbital_phase: 0.75,
        };
        let properties = ResourceProperties {
            mass: 1.0,
            potential_energy: 1.0,
            reactivity: 1.0,
            chemical_position: Some(7.0),
            cohesion: 4.0,
        };
        let baselines = ResourceBaselines {
            mass: 1.0,
            potential_energy: 1.0,
            reactivity: 1.0,
            cohesion: 1.0,
        };
        let mut frame = WorldToneCaptureFrame::from_geometry(sample, &[body]).unwrap();
        let mut holder = 0.0;
        let mut ledger = crate::state::EnergyLedger::default();

        let result = frame
            .settle_surface_response(0, 0, properties, baselines, 0.75, &mut holder, &mut ledger)
            .unwrap();

        assert!(result.absorbed_energy > 0.0);
        assert!(result.unabsorbed_energy > 0.0);
        assert!(
            (result.incident_energy - result.absorbed_energy - result.unabsorbed_energy).abs()
                < 1e-9
        );
        assert!(
            (result.absorbed_energy - result.usable_energy - result.heat_dissipated).abs() < 1e-9
        );
        assert!((holder - result.usable_energy).abs() < 1e-9);
        assert!((ledger.total_external_energy_input - result.absorbed_energy).abs() < 1e-9);
        assert!((ledger.total_heat_dissipated - result.heat_dissipated).abs() < 1e-9);
        assert!(
            (ledger.total_world_tone_energy_unabsorbed - result.unabsorbed_energy).abs() < 1e-9
        );
    }

    #[test]
    fn organism_surface_settlement_conserves_resonance_heat_and_remaining_signal() {
        use crate::organism_geometry::{OrganismBodyGeometry, PlacedForm};
        use crate::resources::{Form, ResourceBaselines, ResourceProperties};
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
            energy_intensity_per_tick_per_width: 10.0,
            orbital_phase: 0.75,
        };
        let mut frame = WorldToneCaptureFrame::from_geometry(sample, &[receiver]).unwrap();
        let properties = ResourceProperties {
            mass: 1.0,
            potential_energy: 1.0,
            reactivity: 1.0,
            chemical_position: Some(7.0),
            cohesion: 1.0,
        };
        let baselines = ResourceBaselines {
            mass: 1.0,
            potential_energy: 1.0,
            reactivity: 1.0,
            cohesion: 1.0,
        };
        let mut simulation = crate::state::Simulation::new(74, 1.0);
        let organism = &mut simulation.organisms[0];
        let initial_usable = organism.usable_energy;
        let initial_resonant = organism.resonant_energy.total_energy().unwrap();
        let mut ledger = crate::state::EnergyLedger::default();

        let result = frame
            .settle_organism_surface_response(0, 0, properties, baselines, organism, &mut ledger)
            .expect("valid organism surface response should settle");

        // Damping is heat; the material response is retained as resonance,
        // and only the remaining incident budget stays in the continuing signal.
        assert!(result.usable_energy > 0.0);
        assert!(result.heat_dissipated > 0.0);
        assert!(
            (result.incident_energy
                - result.usable_energy
                - result.heat_dissipated
                - result.unabsorbed_energy)
                .abs()
                < 1e-7
        );
        assert!(
            (result.absorbed_energy - result.usable_energy - result.heat_dissipated).abs() < 1e-7
        );
        // Captured resonance is transferred out of usable energy and counted once.
        assert!((organism.usable_energy - initial_usable).abs() < 1e-7);
        assert!(
            (organism.resonant_energy.total_energy().unwrap()
                - initial_resonant
                - result.usable_energy)
                .abs()
                < 1e-7
        );
        assert!((ledger.total_external_energy_input - result.absorbed_energy).abs() < 1e-7);
        assert!((ledger.total_usable_energy_gained - result.usable_energy).abs() < 1e-7);
        assert!((ledger.total_heat_dissipated - result.heat_dissipated).abs() < 1e-7);
        assert!(
            (ledger.total_world_tone_energy_unabsorbed - result.unabsorbed_energy).abs() < 1e-7
        );
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
            energy_intensity_per_tick_per_width: 10.0,
            orbital_phase: 0.75,
        };
        let expected_width = expected_front_face_projected_width();
        let mut frame = WorldToneCaptureFrame::from_geometry(sample, &[receiver]).unwrap();
        let mut holder = 0.0;
        let mut ledger = crate::state::EnergyLedger::default();
        assert!(frame
            .settle_receiver(0, 1.0, 1.0, &mut holder, &mut ledger)
            .is_some());
        assert!(frame
            .settle_receiver(0, 1.0, 1.0, &mut holder, &mut ledger)
            .is_none());
        assert!((holder - 10.0 * expected_width).abs() < 1e-7);
        assert!((ledger.total_external_energy_input - 10.0 * expected_width).abs() < 1e-7);
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
            energy_intensity_per_tick_per_width: 10.0,
            orbital_phase: 0.75,
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
            energy_intensity_per_tick_per_width: 10.0,
            orbital_phase: 0.75,
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
