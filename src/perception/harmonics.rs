//! Compact harmonic/world-tone model.
//!
//! The harmonic system is deliberately spectral rather than waveform-based.
//! The world provides a reference tone of 440 cycles per simulated day; realized material structure
//! determines how that tone is filtered, coupled, damped, and enriched with
//! harmonics.

use crate::resources::{ResourceBaselines, ResourceProperties};

/// Simulation-time convention: one simulated day contains 24,000 ticks.
pub(crate) const TICKS_PER_SIMULATED_DAY: u64 = 24_000;
/// Reference carrier frequency, expressed in oscillations per simulated day.
pub(crate) const WORLD_TONE_CYCLES_PER_DAY: f64 = 440.0;
pub(crate) const MAX_SPECTRAL_COMPONENTS: usize = 4;

/// Calculate an oscillator's phase directly from simulation time.
///
/// Phase zero is the positive peak at tick zero. This is analytical and does
/// not require per-tick oscillator updates or substeps. Invalid frequencies
/// return NaN so callers can reject invalid oscillator state.
pub(crate) fn phase_at_tick(frequency_cycles_per_day: f64, tick: u64) -> f64 {
    if !frequency_cycles_per_day.is_finite() || frequency_cycles_per_day <= 0.0 {
        return f64::NAN;
    }
    let cycles = frequency_cycles_per_day * (tick as f64 / TICKS_PER_SIMULATED_DAY as f64);
    if !cycles.is_finite() {
        return f64::NAN;
    }
    (std::f64::consts::TAU * cycles.rem_euclid(1.0)).rem_euclid(std::f64::consts::TAU)
}

/// EXPERIMENTAL: absolute damping calibration for the harmonic model.
/// Reactivity determines relative damping; this constant sets the model scale.
pub(crate) const BASELINE_DAMPING_RATIO: f64 = 0.10;

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq)]
pub(crate) struct ToneComponent {
    #[serde(alias = "frequency_hz")]
    pub(crate) frequency_cycles_per_day: f64,
    pub(crate) amplitude: f64,
    pub(crate) phase_radians: f64,
}

/// Persistent energy retained by an organism's resonant structure.
///
/// This is deliberately separate from ToneSpectrum: that type represents the
/// signal currently received, while this reservoir represents energy retained
/// across ticks. Stored values are energy (amplitude squared), not amplitudes.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Default)]
pub(crate) struct ResonantEnergyReservoir {
    pub(crate) components: Vec<ResonantEnergyComponent>,
}

/// Energy held at one frequency. Phase is not stored because the reservoir
/// tracks retained energy, not the instantaneous phase of a travelling signal.
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq)]
pub(crate) struct ResonantEnergyComponent {
    #[serde(alias = "frequency_hz")]
    pub(crate) frequency_cycles_per_day: f64,
    pub(crate) energy: f64,
}

impl ResonantEnergyReservoir {
    /// Add energy at a frequency. Returns false without mutation when input is
    /// invalid or the bounded spectrum cannot represent another frequency;
    /// the caller must settle rejected energy as heat rather than discard it.
    pub(crate) fn deposit(&mut self, frequency_cycles_per_day: f64, energy: f64) -> bool {
        if self.total_energy().is_none() {
            return false;
        }
        if !frequency_cycles_per_day.is_finite()
            || frequency_cycles_per_day <= 0.0
            || !energy.is_finite()
            || energy < 0.0
        {
            return false;
        }
        if energy <= f64::EPSILON {
            return true;
        }
        if let Some(component) = self.components.iter_mut().find(|component| {
            (component.frequency_cycles_per_day / frequency_cycles_per_day)
                .ln()
                .abs()
                <= 1e-9
        }) {
            let next = component.energy + energy;
            if !next.is_finite() {
                return false;
            }
            component.energy = next;
            return true;
        }
        if self.components.len() >= MAX_SPECTRAL_COMPONENTS {
            return false;
        }
        self.components.push(ResonantEnergyComponent {
            frequency_cycles_per_day,
            energy,
        });
        self.components.sort_by(|a, b| {
            a.frequency_cycles_per_day
                .partial_cmp(&b.frequency_cycles_per_day)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        true
    }

    pub(crate) fn total_energy(&self) -> Option<f64> {
        let mut total = 0.0;
        for component in &self.components {
            if !component.frequency_cycles_per_day.is_finite()
                || component.frequency_cycles_per_day <= 0.0
                || !component.energy.is_finite()
                || component.energy < 0.0
            {
                return None;
            }
            total += component.energy;
            if !total.is_finite() {
                return None;
            }
        }
        Some(total)
    }

    /// Energy accessible to a reaction at the requested frequency, before
    /// any withdrawal. Invalid reservoir state fails closed.
    pub(crate) fn compatible_energy(&self, reaction_frequency_cycles_per_day: f64) -> f64 {
        if !reaction_frequency_cycles_per_day.is_finite()
            || reaction_frequency_cycles_per_day <= 0.0
            || self.total_energy().is_none()
        {
            return 0.0;
        }
        self.components
            .iter()
            .map(|component| {
                let distance = (component.frequency_cycles_per_day
                    / reaction_frequency_cycles_per_day)
                    .ln()
                    .abs();
                let compatibility = (-distance / SPECTRAL_MATCH_SIGMA).exp().clamp(0.0, 1.0);
                component.energy * compatibility
            })
            .sum::<f64>()
            .max(0.0)
    }

    /// Withdraw up to max_energy, weighted by logarithmic frequency
    /// compatibility with the reaction. The returned amount is actual energy
    /// removed from the reservoir. Callers must settle the same amount in the
    /// reaction transaction; no separate energy is created by matching.
    pub(crate) fn withdraw_compatible(
        &mut self,
        reaction_frequency_cycles_per_day: f64,
        max_energy: f64,
    ) -> f64 {
        if !reaction_frequency_cycles_per_day.is_finite()
            || reaction_frequency_cycles_per_day <= 0.0
            || !max_energy.is_finite()
            || max_energy <= 0.0
            || self.total_energy().is_none()
        {
            return 0.0;
        }
        let mut remaining = max_energy;
        let mut withdrawn = 0.0;
        for component in &mut self.components {
            if remaining <= f64::EPSILON {
                break;
            }
            let distance = (component.frequency_cycles_per_day / reaction_frequency_cycles_per_day)
                .ln()
                .abs();
            let compatibility = (-distance / SPECTRAL_MATCH_SIGMA).exp().clamp(0.0, 1.0);
            let available = component.energy * compatibility;
            let take = available.min(remaining);
            if take.is_finite() && take > 0.0 {
                component.energy -= take;
                remaining -= take;
                withdrawn += take;
            }
        }
        self.components
            .retain(|component| component.energy > f64::EPSILON);
        withdrawn
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Default)]
pub(crate) struct ToneSpectrum {
    pub(crate) components: Vec<ToneComponent>,
}

/// A spatially attributed environmental signal after passing through the
/// organism's existing realized resonance geometry. This does not add a
/// separate sensory system: it preserves which boundary location contributed
/// the received environmental spectrum before the cavity aggregate discards
/// that provenance.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct ResonancePerception {
    pub(crate) source_x: f64,
    pub(crate) source_y: f64,
    pub(crate) extent: f64,
    pub(crate) spectrum: ToneSpectrum,
    pub(crate) magnitude: f64,
}

impl ToneSpectrum {
    pub(crate) fn empty() -> Self {
        Self::default()
    }

    pub(crate) fn merge_from(&mut self, other: &ToneSpectrum, scale: f64) {
        add_spectrum(self, other, scale);
        self.retain_strongest();
    }

    pub(crate) fn retain_strongest(&mut self) {
        self.components.retain(|component| {
            component.frequency_cycles_per_day.is_finite()
                && component.frequency_cycles_per_day > 0.0
                && component.amplitude.is_finite()
                && component.amplitude > f64::EPSILON
                && component.phase_radians.is_finite()
        });
        self.components.sort_by(|a, b| {
            b.amplitude
                .partial_cmp(&a.amplitude)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        self.components.truncate(MAX_SPECTRAL_COMPONENTS);
    }
}

/// Natural frequency uses the oscillator relationship f ~ sqrt(k/m).
///
/// EvoSim does not introduce a new stiffness property: cohesion supplies the
/// existing material's restoring/coupling tendency and mass supplies inertia.
/// Catalog-average cohesion/mass provides the normalization that anchors the
/// dimensionless relationship to the 440 cycles per simulated day world reference.
pub(crate) fn natural_frequency_cycles_per_day(
    properties: ResourceProperties,
    baselines: ResourceBaselines,
) -> f64 {
    let values = [
        properties.mass,
        properties.cohesion,
        baselines.mass,
        baselines.cohesion,
    ];
    if values
        .iter()
        .any(|value| !value.is_finite() || *value <= 0.0)
    {
        return f64::NAN;
    }
    let ratio = (properties.cohesion / properties.mass) / (baselines.cohesion / baselines.mass);
    if !ratio.is_finite() || ratio <= 0.0 {
        return f64::NAN;
    }
    let frequency = WORLD_TONE_CYCLES_PER_DAY * ratio.sqrt();
    if frequency.is_finite() && frequency > 0.0 {
        frequency
    } else {
        f64::NAN
    }
}

/// Reactivity supplies the existing dissipative/nonlinear response scale.
///
/// The absolute scale is experimental; relative damping is derived from the
/// material's reactivity relative to the catalog baseline. No new
/// damping-only resource property is introduced.
pub(crate) fn damping_ratio(properties: ResourceProperties, baselines: ResourceBaselines) -> f64 {
    let reactivity = properties.reactivity;
    let baseline = baselines.reactivity;
    if !reactivity.is_finite() || reactivity < 0.0 || !baseline.is_finite() || baseline < 0.0 {
        return f64::NAN;
    }
    if baseline <= f64::EPSILON {
        return if reactivity <= f64::EPSILON {
            0.0
        } else {
            BASELINE_DAMPING_RATIO
        };
    }
    let damping = BASELINE_DAMPING_RATIO * (reactivity / baseline);
    if damping.is_finite() && damping >= 0.0 {
        damping
    } else {
        f64::NAN
    }
}

/// A compact forced-oscillator response centered on the material's natural
/// frequency. This is a spectral response, not a time-domain simulation.
pub(crate) fn resonance_response(
    drive_frequency_cycles_per_day: f64,
    natural_frequency_cycles_per_day: f64,
    damping: f64,
) -> f64 {
    if !drive_frequency_cycles_per_day.is_finite()
        || !natural_frequency_cycles_per_day.is_finite()
        || !damping.is_finite()
        || damping < 0.0
        || drive_frequency_cycles_per_day <= 0.0
        || natural_frequency_cycles_per_day <= 0.0
    {
        return 0.0;
    }
    let zeta = damping.max(f64::EPSILON);
    let ratio = drive_frequency_cycles_per_day / natural_frequency_cycles_per_day;
    let denominator = ((1.0 - ratio * ratio).powi(2) + (2.0 * zeta * ratio).powi(2)).sqrt();
    1.0 / denominator.max(f64::EPSILON)
}

/// Convert wave amplitude to normalized wave energy.
///
/// The harmonic model uses a dimensionless normalization where energy is
/// proportional to amplitude squared. This is the explicit boundary between
/// spectral signal values and the energy ledger; callers must not treat raw
/// amplitudes as energy.
pub(crate) fn wave_energy_from_amplitude(amplitude: f64) -> f64 {
    if !amplitude.is_finite() || amplitude <= 0.0 {
        return 0.0;
    }
    let energy = amplitude * amplitude;
    if energy.is_finite() {
        energy
    } else {
        0.0
    }
}

/// Bound a response spectrum to its incident energy budget.
///
/// Resonance gain is a steady-state displacement ratio, not a license to create
/// outgoing wave energy. When that gain or nonlinear harmonics exceed the
/// available budget, scale the whole spectrum uniformly so its summed
/// amplitude-squared energy does not exceed the incident budget. Invalid or
/// unrepresentable spectra fail closed.
fn bound_spectrum_energy(spectrum: &mut ToneSpectrum, energy_budget: f64) {
    if !energy_budget.is_finite() || energy_budget <= 0.0 {
        spectrum.components.clear();
        return;
    }

    let mut total_energy = 0.0;
    for component in &spectrum.components {
        if !component.amplitude.is_finite() || component.amplitude < 0.0 {
            spectrum.components.clear();
            return;
        }
        total_energy += component.amplitude * component.amplitude;
        if !total_energy.is_finite() {
            spectrum.components.clear();
            return;
        }
    }

    if total_energy > energy_budget && total_energy > 0.0 {
        let scale = (energy_budget / total_energy).sqrt();
        if !scale.is_finite() {
            spectrum.components.clear();
            return;
        }
        for component in &mut spectrum.components {
            component.amplitude *= scale;
        }
    }
}

/// Convert normalized wave energy to its corresponding non-negative amplitude.
pub(crate) fn wave_amplitude_from_energy(energy: f64) -> f64 {
    if !energy.is_finite() || energy <= 0.0 {
        return 0.0;
    }
    energy.sqrt()
}

/// Return the equivalent amplitude of a multi-frequency spectrum.
///
/// Distinct frequency components contribute energy additively, so their
/// combined magnitude is the square root of summed amplitude-squared energy,
/// not the arithmetic sum of amplitudes.
fn spectrum_magnitude(spectrum: &ToneSpectrum) -> f64 {
    let mut energy = 0.0;
    for component in &spectrum.components {
        if !component.amplitude.is_finite() || component.amplitude < 0.0 {
            return 0.0;
        }
        energy += component.amplitude * component.amplitude;
        if !energy.is_finite() {
            return 0.0;
        }
    }
    energy.sqrt()
}

/// Dimensionless fraction of incident wave energy dissipated by damping.
///
/// The oscillator response is an amplitude ratio. Damping couples that
/// response to the dissipative channel with amplitude factor 2*zeta*r, where
/// r is drive/natural frequency. Squaring the resulting damped amplitude gives
/// the dissipated-energy fraction. This is derived from the wave-amplitude
/// model, rather than maintained as a separate energy equation.
pub(crate) fn resonant_absorption_fraction(
    drive_frequency_cycles_per_day: f64,
    natural_frequency_cycles_per_day: f64,
    damping: f64,
) -> f64 {
    if !drive_frequency_cycles_per_day.is_finite()
        || !natural_frequency_cycles_per_day.is_finite()
        || !damping.is_finite()
        || drive_frequency_cycles_per_day <= 0.0
        || natural_frequency_cycles_per_day <= 0.0
        || damping <= 0.0
    {
        return 0.0;
    }
    let ratio = drive_frequency_cycles_per_day / natural_frequency_cycles_per_day;
    if !ratio.is_finite() {
        return 0.0;
    }
    let response_amplitude = resonance_response(
        drive_frequency_cycles_per_day,
        natural_frequency_cycles_per_day,
        damping,
    );
    let dissipative_amplitude = 2.0 * damping * ratio * response_amplitude;
    let fraction = wave_energy_from_amplitude(dissipative_amplitude);
    if fraction.is_finite() {
        fraction.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// Reactivity-driven nonlinear content. Integer multiples are used because
/// real nonlinear oscillators commonly produce harmonic overtones.
pub(crate) fn nonlinear_harmonic_amplitude(
    fundamental_amplitude: f64,
    reactivity: f64,
    harmonic_order: usize,
) -> f64 {
    if harmonic_order < 2
        || !fundamental_amplitude.is_finite()
        || fundamental_amplitude < 0.0
        || !reactivity.is_finite()
        || reactivity < 0.0
    {
        return 0.0;
    }
    let nonlinear = if reactivity <= f64::EPSILON {
        0.0
    } else {
        reactivity / (reactivity + 1.0)
    };
    fundamental_amplitude * nonlinear.powi((harmonic_order - 1) as i32) / harmonic_order as f64
}

/// Generate the local spectrum produced by one realized material response to
/// the world tone. Structure-level coupling is applied by the caller through
/// the physical bond graph.
pub(crate) fn material_response(
    properties: ResourceProperties,
    baselines: ResourceBaselines,
    phase_radians: f64,
) -> ToneSpectrum {
    material_response_to_drive(
        properties,
        baselines,
        WORLD_TONE_CYCLES_PER_DAY,
        1.0,
        phase_radians,
    )
}

/// Apply the existing material resonance model to a supplied environmental
/// drive. The signal level scales response; it is not energy captured by the
/// receiver and must never be credited to an energy ledger by this function.
fn material_response_to_drive(
    properties: ResourceProperties,
    baselines: ResourceBaselines,
    drive_frequency_cycles_per_day: f64,
    signal_level: f64,
    phase_radians: f64,
) -> ToneSpectrum {
    if !drive_frequency_cycles_per_day.is_finite()
        || drive_frequency_cycles_per_day <= 0.0
        || !signal_level.is_finite()
        || signal_level <= 0.0
        || !properties.mass.is_finite()
        || properties.mass <= 0.0
        || !properties.cohesion.is_finite()
        || properties.cohesion <= 0.0
        || !properties.reactivity.is_finite()
        || properties.reactivity < 0.0
        || !baselines.mass.is_finite()
        || baselines.mass <= 0.0
        || !baselines.cohesion.is_finite()
        || baselines.cohesion <= 0.0
        || !baselines.reactivity.is_finite()
        || baselines.reactivity < 0.0
    {
        return ToneSpectrum::empty();
    }
    let natural = natural_frequency_cycles_per_day(properties, baselines);
    let damping = damping_ratio(properties, baselines);
    let fundamental =
        resonance_response(drive_frequency_cycles_per_day, natural, damping) * signal_level;
    let mut spectrum = ToneSpectrum {
        components: vec![ToneComponent {
            frequency_cycles_per_day: drive_frequency_cycles_per_day,
            amplitude: fundamental,
            phase_radians,
        }],
    };

    for order in 2..=MAX_SPECTRAL_COMPONENTS {
        let amplitude = nonlinear_harmonic_amplitude(fundamental, properties.reactivity, order);
        if amplitude > f64::EPSILON {
            spectrum.components.push(ToneComponent {
                frequency_cycles_per_day: drive_frequency_cycles_per_day * order as f64,
                amplitude,
                phase_radians: phase_radians * order as f64,
            });
        }
    }
    spectrum.retain_strongest();
    bound_spectrum_energy(&mut spectrum, wave_energy_from_amplitude(signal_level));
    spectrum
}

/// Transform a continuing environmental signal through one material response.
///
/// The harmonic model determines relative frequency composition. The caller
/// supplies the remaining physical energy budget after capture and dissipation;
/// amplitudes are normalized so the outgoing spectrum carries exactly that
/// budget under E = A². This is onward transmission, not a new emitted source
/// and not a literal reflected ray.
fn spectrum_energy(spectrum: &ToneSpectrum) -> Option<f64> {
    let mut energy = 0.0;
    for component in &spectrum.components {
        if !component.frequency_cycles_per_day.is_finite()
            || component.frequency_cycles_per_day <= 0.0
            || !component.amplitude.is_finite()
            || component.amplitude < 0.0
            || !component.phase_radians.is_finite()
        {
            return None;
        }
        energy += component.amplitude * component.amplitude;
        if !energy.is_finite() {
            return None;
        }
    }
    Some(energy)
}

/// Apply the existing material resonance and nonlinear response to every
/// component of an incoming spectrum. The returned response keeps its natural
/// response amplitude, but total energy cannot exceed the incoming spectrum.
pub(crate) fn material_response_spectrum(
    properties: ResourceProperties,
    baselines: ResourceBaselines,
    incoming: &ToneSpectrum,
) -> Option<ToneSpectrum> {
    let incoming_energy = spectrum_energy(incoming)?;
    let mut transformed = ToneSpectrum::empty();
    for component in &incoming.components {
        if component.amplitude <= f64::EPSILON {
            continue;
        }
        let response = material_response_to_drive(
            properties,
            baselines,
            component.frequency_cycles_per_day,
            component.amplitude,
            component.phase_radians,
        );
        transformed.merge_from(&response, 1.0);
    }
    bound_spectrum_energy(&mut transformed, incoming_energy);
    Some(transformed)
}

/// Transform a continuing environmental signal through one material response.
///
/// The harmonic model determines relative frequency composition. The caller
/// supplies the remaining physical energy budget after capture and dissipation;
/// amplitudes are normalized so the outgoing spectrum carries exactly that
/// budget under E = A². This is onward transmission, not a new emitted source
/// and not a literal reflected ray.
pub(crate) fn transform_signal_to_energy_budget(
    properties: ResourceProperties,
    baselines: ResourceBaselines,
    incoming: &ToneSpectrum,
    outgoing_energy_budget: f64,
) -> Option<ToneSpectrum> {
    let incoming_energy = spectrum_energy(incoming)?;
    if !outgoing_energy_budget.is_finite()
        || outgoing_energy_budget < 0.0
        || outgoing_energy_budget > incoming_energy + 1e-9
    {
        return None;
    }
    if outgoing_energy_budget <= f64::EPSILON {
        return Some(ToneSpectrum::empty());
    }

    let mut transformed = material_response_spectrum(properties, baselines, incoming)?;
    let transformed_energy = spectrum_energy(&transformed)?;
    if !transformed_energy.is_finite() || transformed_energy <= f64::EPSILON {
        return None;
    }

    // Keep the strongest harmonic components first, then normalize the
    // remaining spectrum to the conserved remainder. Normalizing before this
    // filter could lose energy when weak components are removed.
    transformed.retain_strongest();
    let retained_energy = spectrum_energy(&transformed)?;
    if !retained_energy.is_finite() || retained_energy <= f64::EPSILON {
        return None;
    }
    let scale = (outgoing_energy_budget / retained_energy).sqrt();
    if !scale.is_finite() || scale <= 0.0 {
        return None;
    }
    // Bias the first scale very slightly downward. If floating-point rounding
    // still lands above the budget, the correction loop below only scales down.
    let scale = scale * (1.0 - 2.0 * f64::EPSILON);
    for component in &mut transformed.components {
        component.amplitude *= scale;
    }
    let mut final_energy = spectrum_energy(&transformed)?;
    for _ in 0..4 {
        if final_energy <= outgoing_energy_budget {
            break;
        }
        let correction =
            (outgoing_energy_budget / final_energy).sqrt() * (1.0 - 2.0 * f64::EPSILON);
        if !correction.is_finite() || correction <= 0.0 {
            return None;
        }
        for component in &mut transformed.components {
            component.amplitude *= correction;
        }
        final_energy = spectrum_energy(&transformed)?;
    }
    if !final_energy.is_finite()
        || final_energy > outgoing_energy_budget
        || (final_energy - outgoing_energy_budget).abs() > 1e-9 * outgoing_energy_budget.max(1.0)
    {
        return None;
    }
    Some(transformed)
}

/// Receive the rotating world tone through the organism's actual cavity-wall
/// segments. This is signal perception only: it neither mutates organism energy
/// nor records energy capture. Each wall segment uses its realized material
/// properties and the same outward-normal directional coupling as environmental
/// material reception.
pub(crate) fn world_tone_cavity_contribution(
    catalog: &[crate::resources::BaseResource],
    structure: &crate::structure::OrganismStructure,
    cavity: &crate::cavity::GenomeCavity,
    sample: crate::world_tone::WorldToneSample,
) -> (ToneSpectrum, Vec<ResonancePerception>) {
    world_tone_cavity_contribution_at_tick(catalog, structure, cavity, sample, 0)
}

pub(crate) fn world_tone_cavity_contribution_at_tick(
    catalog: &[crate::resources::BaseResource],
    structure: &crate::structure::OrganismStructure,
    cavity: &crate::cavity::GenomeCavity,
    sample: crate::world_tone::WorldToneSample,
    tick: u64,
) -> (ToneSpectrum, Vec<ResonancePerception>) {
    let segments = cavity.boundary_segments();
    if segments.is_empty() || sample.energy_intensity_per_tick_per_width <= f64::EPSILON {
        return (ToneSpectrum::empty(), Vec::new());
    }
    let center = cavity_center(cavity);
    let baselines = ResourceBaselines::from_catalog(catalog);
    let mut spectrum = ToneSpectrum::empty();
    let mut perceptions = Vec::new();

    for (a, b, unit_index) in segments {
        let Some(unit) = structure.units.get(unit_index) else {
            continue;
        };
        let Some(properties) = unit.properties(catalog) else {
            continue;
        };
        let rx = (a.0 + b.0) * 0.5;
        let ry = (a.1 + b.1) * 0.5;
        let dx = sample.source_x - rx;
        let dy = sample.source_y - ry;
        let distance = dx.hypot(dy);
        if !distance.is_finite() || distance <= f64::EPSILON {
            continue;
        }

        let edge_x = b.0 - a.0;
        let edge_y = b.1 - a.1;
        let edge_length = edge_x.hypot(edge_y);
        if !edge_length.is_finite() || edge_length <= f64::EPSILON {
            continue;
        }
        let mut normal_x = -edge_y / edge_length;
        let mut normal_y = edge_x / edge_length;
        if normal_x * (rx - center.0) + normal_y * (ry - center.1) < 0.0 {
            normal_x = -normal_x;
            normal_y = -normal_y;
        }
        let directional_gain = (normal_x * dx / distance + normal_y * dy / distance).max(0.0);
        if directional_gain <= f64::EPSILON {
            continue;
        }
        let attenuation = directional_gain / (1.0 + distance);
        let mut received = material_response_to_drive(
            properties,
            baselines,
            sample.frequency_cycles_per_day,
            wave_amplitude_from_energy(sample.energy_intensity_per_tick_per_width * attenuation),
            phase_at_tick(sample.frequency_cycles_per_day, tick),
        );
        received.retain_strongest();
        let magnitude = spectrum_magnitude(&received);
        if magnitude <= f64::EPSILON {
            continue;
        }
        add_spectrum(&mut spectrum, &received, 1.0);
        perceptions.push(ResonancePerception {
            source_x: sample.source_x,
            source_y: sample.source_y,
            extent: 0.0,
            spectrum: received,
            magnitude,
        });
    }
    spectrum.retain_strongest();
    (spectrum, perceptions)
}

/// Combine two spectral components at the same frequency as phasors.
///
/// Frequency is the physical identity of a component. Amplitude and phase
/// combine through the existing harmonic representation rather than creating
/// a new time-domain simulation.
fn combine_component(a: ToneComponent, b: ToneComponent) -> ToneComponent {
    let ax = a.amplitude * a.phase_radians.cos();
    let ay = a.amplitude * a.phase_radians.sin();
    let bx = b.amplitude * b.phase_radians.cos();
    let by = b.amplitude * b.phase_radians.sin();
    let x = ax + bx;
    let y = ay + by;
    ToneComponent {
        frequency_cycles_per_day: a.frequency_cycles_per_day,
        amplitude: x.hypot(y),
        phase_radians: y.atan2(x),
    }
}

fn add_spectrum(target: &mut ToneSpectrum, source: &ToneSpectrum, scale: f64) {
    if !scale.is_finite() || scale <= f64::EPSILON {
        return;
    }
    for component in &source.components {
        let contribution = ToneComponent {
            frequency_cycles_per_day: component.frequency_cycles_per_day,
            amplitude: component.amplitude * scale,
            phase_radians: component.phase_radians,
        };
        if let Some(existing) = target.components.iter_mut().find(|existing| {
            (existing.frequency_cycles_per_day - contribution.frequency_cycles_per_day).abs()
                <= 1e-9
        }) {
            *existing = combine_component(*existing, contribution);
        } else {
            target.components.push(contribution);
        }
    }
}

/// Environmental quantity scales excitation using absolute material mass.
/// This is a response scale, not a new material property.
fn environmental_mass_scale(mass: f64, baseline_mass: f64) -> f64 {
    let mass = mass.max(0.0);
    let baseline = baseline_mass.max(f64::EPSILON);
    mass / (mass + baseline)
}

/// A realized environmental material emits one spectrum as a whole material.
/// Composition is evaluated through the material's aggregate properties; the
/// material is never split into independent environmental sensory sources.
fn environmental_material_resonance(
    physical: &crate::physical_material::PhysicalMaterial,
    catalog: &[crate::resources::BaseResource],
    tick: u64,
) -> Option<(f64, f64, f64, ToneSpectrum)> {
    if !physical.material.is_valid() || physical.material.is_empty() {
        return None;
    }
    let placements = physical.placements.as_ref()?;
    if placements.len() != physical.material.parts.len() || placements.is_empty() {
        return None;
    }

    let mut source_x = 0.0;
    let mut source_y = 0.0;
    for placement in placements {
        source_x += placement.x;
        source_y += placement.y;
    }
    let count = placements.len() as f64;
    source_x /= count;
    source_y /= count;

    let baselines = ResourceBaselines::from_catalog(catalog);
    let spectrum = material_response(
        physical.material.weighted_properties(catalog),
        baselines,
        phase_at_tick(WORLD_TONE_CYCLES_PER_DAY, tick),
    );
    let extent = placements
        .iter()
        .map(|placement| (placement.x - source_x).hypot(placement.y - source_y))
        .fold(0.0_f64, f64::max);
    Some((source_x, source_y, extent, spectrum))
}

/// Calculate one directional environmental contribution at one realized
/// genome-cavity boundary segment. The segment itself is the receiver: its
/// physical orientation is the antenna geometry, while the source remains the
/// complete environmental material.
fn environmental_contribution(
    physical: &crate::physical_material::PhysicalMaterial,
    catalog: &[crate::resources::BaseResource],
    tick: u64,
    receiver_a: (f64, f64),
    receiver_b: (f64, f64),
    cavity_center: (f64, f64),
) -> Option<(ResonancePerception, ToneSpectrum)> {
    let (source_x, source_y, extent, emitted) =
        environmental_material_resonance(physical, catalog, tick)?;
    let baselines = ResourceBaselines::from_catalog(catalog);
    let mass_scale = environmental_mass_scale(physical.material.mass(catalog), baselines.mass);

    let rx = (receiver_a.0 + receiver_b.0) * 0.5;
    let ry = (receiver_a.1 + receiver_b.1) * 0.5;
    let dx = source_x - rx;
    let dy = source_y - ry;
    let distance = dx.hypot(dy);
    if !distance.is_finite() || distance <= f64::EPSILON {
        return None;
    }

    let edge_x = receiver_b.0 - receiver_a.0;
    let edge_y = receiver_b.1 - receiver_a.1;
    let edge_length = edge_x.hypot(edge_y);
    if edge_length <= f64::EPSILON {
        return None;
    }

    let mut normal_x = -edge_y / edge_length;
    let mut normal_y = edge_x / edge_length;
    let away_x = rx - cavity_center.0;
    let away_y = ry - cavity_center.1;
    if normal_x * away_x + normal_y * away_y < 0.0 {
        normal_x = -normal_x;
        normal_y = -normal_y;
    }

    let incoming_x = dx / distance;
    let incoming_y = dy / distance;
    let directional_gain = (normal_x * incoming_x + normal_y * incoming_y).max(0.0);
    if directional_gain <= f64::EPSILON {
        return None;
    }

    // No hard perception radius is introduced. Distance attenuates the aura
    // continuously, while the receiver's realized boundary geometry controls
    // directional coupling.
    let distance_scale = 1.0 / (1.0 + distance);
    let scale = mass_scale * directional_gain * distance_scale;
    if scale <= f64::EPSILON {
        return None;
    }

    let mut received = ToneSpectrum::empty();
    add_spectrum(&mut received, &emitted, scale);
    let magnitude = spectrum_magnitude(&received);
    if magnitude <= f64::EPSILON {
        return None;
    }

    Some((
        ResonancePerception {
            source_x,
            source_y,
            extent,
            spectrum: received.clone(),
            magnitude,
        },
        received,
    ))
}

fn cavity_center(cavity: &crate::cavity::GenomeCavity) -> (f64, f64) {
    let segments = cavity.boundary_segments();
    if segments.is_empty() {
        return (0.0, 0.0);
    }
    let mut x = 0.0;
    let mut y = 0.0;
    let mut count = 0.0;
    for (a, b, _) in segments {
        x += a.0 + b.0;
        y += a.1 + b.1;
        count += 2.0;
    }
    (x / count, y / count)
}

fn environmental_contributions(
    catalog: &[crate::resources::BaseResource],
    field: &crate::environment::ActiveMaterialField,
    cavity: &crate::cavity::GenomeCavity,
    tick: u64,
) -> (ToneSpectrum, Vec<ResonancePerception>) {
    let segments = cavity.boundary_segments();
    if segments.is_empty() {
        return (ToneSpectrum::empty(), Vec::new());
    }
    let center = cavity_center(cavity);
    let mut spectrum = ToneSpectrum::empty();
    let mut perceptions = Vec::new();

    for cell in &field.cells {
        if cell.physical_materials.is_empty() {
            continue;
        }
        for physical in &cell.physical_materials {
            for (a, b, _) in &segments {
                if let Some((perception, contribution)) =
                    environmental_contribution(physical, catalog, tick, *a, *b, center)
                {
                    add_spectrum(&mut spectrum, &contribution, 1.0);
                    perceptions.push(perception);
                }
            }
        }
    }
    spectrum.retain_strongest();
    (spectrum, perceptions)
}

pub(crate) fn genome_cavity_spectrum(
    catalog: &[crate::resources::BaseResource],
    field: &crate::environment::ActiveMaterialField,
    cavity: &crate::cavity::GenomeCavity,
) -> ToneSpectrum {
    genome_cavity_spectrum_at_tick(catalog, field, cavity, 0)
}

pub(crate) fn genome_cavity_spectrum_at_tick(
    catalog: &[crate::resources::BaseResource],
    field: &crate::environment::ActiveMaterialField,
    cavity: &crate::cavity::GenomeCavity,
    tick: u64,
) -> ToneSpectrum {
    environmental_contributions(catalog, field, cavity, tick).0
}

/// Preserve one spatially attributed signal for each source/receiver pair.
/// Keeping these channels separate is what gives downstream interpretation
/// directional information without creating a separate sensor system.
pub(crate) fn genome_cavity_resonance_perceptions(
    catalog: &[crate::resources::BaseResource],
    field: &crate::environment::ActiveMaterialField,
    cavity: &crate::cavity::GenomeCavity,
) -> Vec<ResonancePerception> {
    genome_cavity_resonance_perceptions_at_tick(catalog, field, cavity, 0)
}

pub(crate) fn genome_cavity_resonance_perceptions_at_tick(
    catalog: &[crate::resources::BaseResource],
    field: &crate::environment::ActiveMaterialField,
    cavity: &crate::cavity::GenomeCavity,
    tick: u64,
) -> Vec<ResonancePerception> {
    environmental_contributions(catalog, field, cavity, tick).1
}

/// Return the spatially attributed resonance signals currently reaching the
/// organism's genome cavity. This reuses the same physical resonance geometry
/// as the harmonic state; it does not create a second sensory radius.
pub(crate) fn organism_resonance_perceptions(
    organism: &crate::state::Organism,
    environment: &crate::state::Environment,
    tick: u64,
) -> Vec<ResonancePerception> {
    let Some(cavity) =
        crate::cavity::analyze_genome_cavity(&organism.structure, &environment.catalog)
            .ok()
            .flatten()
    else {
        return Vec::new();
    };
    genome_cavity_resonance_perceptions_at_tick(
        &environment.catalog,
        &environment.field,
        &cavity,
        tick,
    )
}

/// Refresh the harmonic state from the organism's actual realized genome
/// cavity. A non-qualifying physical structure has no genome harmonic
/// memory surface.
pub(crate) fn update_organism_harmonics(
    organism: &mut crate::state::Organism,
    environment: &crate::state::Environment,
    world_tone_sample: Option<crate::world_tone::WorldToneSample>,
    tick: u64,
) {
    let cavity = organism.genome_cavity_cached(&environment.catalog);
    let key = (
        organism.structure_revision,
        organism.position_revision,
        environment.field.revision,
        if world_tone_sample.is_some() {
            tick.wrapping_add(1)
        } else {
            0
        },
    );
    if organism.cached_harmonic_key == Some(key) {
        return;
    }

    let mut spectrum = cavity
        .as_ref()
        .map(|cavity| {
            genome_cavity_spectrum_at_tick(&environment.catalog, &environment.field, cavity, tick)
        })
        .unwrap_or_default();

    if let (Some(sample), Some(cavity)) = (world_tone_sample, cavity.as_ref()) {
        let (world_spectrum, _) = world_tone_cavity_contribution_at_tick(
            &environment.catalog,
            &organism.structure,
            cavity,
            sample,
            tick,
        );
        spectrum.merge_from(&world_spectrum, 1.0);
    }

    organism.harmonic_spectrum = spectrum;
    organism.cached_harmonic_key = Some(key);
}

/// EXPERIMENTAL: logarithmic spectral matching width. Smaller values require
/// frequencies to be closer before they are treated as similar.
pub(crate) const SPECTRAL_MATCH_SIGMA: f64 = 0.25;

/// Return a symmetric continuous similarity ratio in [0, 1].
///
/// Frequency distance is logarithmic so a doubling and halving are treated
/// symmetrically. Amplitude weights the contribution of each perceived
/// component; phase is intentionally ignored because recognition is based on
/// the received spectral pattern rather than instantaneous oscillator phase.
pub(crate) fn spectral_similarity(a: &ToneSpectrum, b: &ToneSpectrum) -> f64 {
    if a.components.is_empty() || b.components.is_empty() {
        return 0.0;
    }
    fn directional(from: &ToneSpectrum, to: &ToneSpectrum) -> f64 {
        let total = from
            .components
            .iter()
            .map(|component| component.amplitude.max(0.0))
            .sum::<f64>();
        if total <= f64::EPSILON {
            return 0.0;
        }
        from.components
            .iter()
            .map(|component| {
                let best = to
                    .components
                    .iter()
                    .map(|other| {
                        let distance = (component.frequency_cycles_per_day
                            / other.frequency_cycles_per_day)
                            .ln()
                            .abs();
                        (-distance / SPECTRAL_MATCH_SIGMA).exp()
                    })
                    .fold(0.0_f64, f64::max);
                component.amplitude.max(0.0) * best
            })
            .sum::<f64>()
            / total
    }

    ((directional(a, b) + directional(b, a)) * 0.5).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_clock_starts_at_peak_and_advances_reference_tone_by_6_point_6_degrees_per_tick() {
        assert_eq!(phase_at_tick(WORLD_TONE_CYCLES_PER_DAY, 0), 0.0);
        let expected = std::f64::consts::TAU * 440.0 / 24_000.0;
        assert!((phase_at_tick(WORLD_TONE_CYCLES_PER_DAY, 1) - expected).abs() < 1e-12);
        assert!((phase_at_tick(WORLD_TONE_CYCLES_PER_DAY, 1).to_degrees() - 6.6).abs() < 1e-10);
    }

    #[test]
    fn phase_clock_wraps_after_the_reference_tone_completes_its_daily_cycles() {
        assert!(phase_at_tick(WORLD_TONE_CYCLES_PER_DAY, TICKS_PER_SIMULATED_DAY).abs() < 1e-10);
        assert!(
            phase_at_tick(WORLD_TONE_CYCLES_PER_DAY, 2 * TICKS_PER_SIMULATED_DAY).abs() < 1e-10
        );
    }

    #[test]
    fn phase_clock_scales_with_each_material_frequency_without_tick_updates() {
        let tick = 137_u64;
        let base = phase_at_tick(WORLD_TONE_CYCLES_PER_DAY, tick);
        let double = phase_at_tick(2.0 * WORLD_TONE_CYCLES_PER_DAY, tick);
        let expected = (2.0 * base).rem_euclid(std::f64::consts::TAU);
        assert!((double - expected).abs() < 1e-10);
    }

    #[test]
    fn phase_clock_rejects_invalid_frequency() {
        assert!(phase_at_tick(0.0, 1).is_nan());
        assert!(phase_at_tick(f64::NAN, 1).is_nan());
        assert!(phase_at_tick(f64::INFINITY, 1).is_nan());
    }

    #[test]
    fn legacy_serialized_frequency_fields_load_as_cycles_per_day() {
        let tone_json = r#"{"frequency_hz":440.0,"amplitude":1.0,"phase_radians":0.0}"#;
        let tone = serde_json::from_str::<ToneComponent>(tone_json)
            .expect("legacy tone frequency field should deserialize");
        assert_eq!(tone.frequency_cycles_per_day, 440.0);

        let retained_json = r#"{"frequency_hz":440.0,"energy":2.0}"#;
        let retained = serde_json::from_str::<ResonantEnergyComponent>(retained_json)
            .expect("legacy retained-energy frequency field should deserialize");
        assert_eq!(retained.frequency_cycles_per_day, 440.0);
    }

    #[test]
    fn retained_resonance_is_distinct_from_current_signal_and_conserves_energy() {
        let mut reservoir = ResonantEnergyReservoir::default();
        assert!(reservoir.deposit(440.0, 3.0));
        assert!(reservoir.deposit(440.0, 2.0));
        assert!((reservoir.total_energy().unwrap() - 5.0).abs() < 1e-12);
        assert_eq!(reservoir.components.len(), 1);
    }

    #[test]
    fn retained_resonance_rejects_unrepresentable_frequency_without_losing_energy() {
        let mut reservoir = ResonantEnergyReservoir::default();
        for (frequency, energy) in [(110.0, 1.0), (220.0, 2.0), (440.0, 3.0), (880.0, 4.0)] {
            assert!(reservoir.deposit(frequency, energy));
        }
        assert!(!reservoir.deposit(1760.0, 5.0));
        assert!((reservoir.total_energy().unwrap() - 10.0).abs() < 1e-12);
    }

    #[test]
    fn compatible_resonance_query_is_frequency_weighted_without_mutation() {
        let mut reservoir = ResonantEnergyReservoir::default();
        assert!(reservoir.deposit(440.0, 2.0));
        assert!(reservoir.deposit(1760.0, 2.0));
        let original = reservoir.clone();
        assert!(reservoir.compatible_energy(440.0) > 2.0);
        assert!(reservoir.compatible_energy(440.0) < 4.0);
        assert_eq!(reservoir, original);
    }

    #[test]
    fn invalid_retained_resonance_state_fails_closed_without_mutation() {
        let mut reservoir = ResonantEnergyReservoir {
            components: vec![ResonantEnergyComponent {
                frequency_cycles_per_day: 440.0,
                energy: f64::NAN,
            }],
        };
        let original = reservoir.clone();
        assert!(!reservoir.deposit(440.0, 1.0));
        assert_eq!(reservoir.withdraw_compatible(440.0, 1.0), 0.0);
        assert_eq!(reservoir.components.len(), original.components.len());
        assert_eq!(
            reservoir.components[0].frequency_cycles_per_day,
            original.components[0].frequency_cycles_per_day
        );
        assert!(reservoir.components[0].energy.is_nan());
    }

    #[test]
    fn retained_resonance_withdrawal_is_frequency_weighted_and_bounded() {
        let mut reservoir = ResonantEnergyReservoir::default();
        assert!(reservoir.deposit(440.0, 4.0));
        assert!(reservoir.deposit(1760.0, 4.0));
        let withdrawn = reservoir.withdraw_compatible(440.0, 2.0);
        assert!(withdrawn > 0.0 && withdrawn <= 2.0);
        assert!((reservoir.total_energy().unwrap() - (8.0 - withdrawn)).abs() < 1e-12);
        assert_eq!(reservoir.withdraw_compatible(440.0, f64::NAN), 0.0);
    }

    #[test]
    fn wave_amplitude_energy_conversion_round_trips() {
        for energy in [0.0, 0.25, 1.0, 5.0, 10.0, 1_000.0] {
            let amplitude = wave_amplitude_from_energy(energy);
            assert!((wave_energy_from_amplitude(amplitude) - energy).abs() < 1e-10);
        }
        assert_eq!(wave_energy_from_amplitude(f64::NAN), 0.0);
        assert_eq!(wave_amplitude_from_energy(f64::INFINITY), 0.0);
    }

    #[test]
    fn damped_energy_fraction_is_squared_wave_response() {
        let drive = 660.0;
        let natural = 440.0;
        let damping = 0.15;
        let ratio = drive / natural;
        let response = resonance_response(drive, natural, damping);
        let dissipative_amplitude = 2.0 * damping * ratio * response;
        let expected = wave_energy_from_amplitude(dissipative_amplitude);
        assert!((resonant_absorption_fraction(drive, natural, damping) - expected).abs() < 1e-12);
    }

    #[test]
    fn resonant_absorption_fraction_is_bounded_and_peaks_at_resonance() {
        let damping = 0.2;
        let resonant = resonant_absorption_fraction(440.0, 440.0, damping);
        let below = resonant_absorption_fraction(220.0, 440.0, damping);
        let above = resonant_absorption_fraction(880.0, 440.0, damping);
        assert!((resonant - 1.0).abs() < 1e-12);
        assert!((0.0..=1.0).contains(&below));
        assert!((0.0..=1.0).contains(&above));
        assert!(resonant > below);
        assert!(resonant > above);
    }

    #[test]
    fn resonant_absorption_fraction_rejects_invalid_inputs() {
        assert_eq!(resonant_absorption_fraction(0.0, 440.0, 0.2), 0.0);
        assert_eq!(resonant_absorption_fraction(440.0, f64::NAN, 0.2), 0.0);
        assert_eq!(resonant_absorption_fraction(440.0, 440.0, 0.0), 0.0);
        assert_eq!(
            resonant_absorption_fraction(440.0, 440.0, f64::INFINITY),
            0.0
        );
    }

    #[test]
    fn spectral_similarity_is_symmetric_and_exact_match_is_one() {
        let a = ToneSpectrum {
            components: vec![ToneComponent {
                frequency_cycles_per_day: 440.0,
                amplitude: 1.0,
                phase_radians: 0.0,
            }],
        };
        let b = a.clone();
        assert!((spectral_similarity(&a, &b) - 1.0).abs() < 1e-12);
        assert!((spectral_similarity(&a, &b) - spectral_similarity(&b, &a)).abs() < 1e-12);
    }

    #[test]
    fn spectral_similarity_falls_with_log_frequency_distance() {
        let a = ToneSpectrum {
            components: vec![ToneComponent {
                frequency_cycles_per_day: 440.0,
                amplitude: 1.0,
                phase_radians: 0.0,
            }],
        };
        let near = ToneSpectrum {
            components: vec![ToneComponent {
                frequency_cycles_per_day: 460.0,
                amplitude: 1.0,
                phase_radians: 0.0,
            }],
        };
        let far = ToneSpectrum {
            components: vec![ToneComponent {
                frequency_cycles_per_day: 1760.0,
                amplitude: 1.0,
                phase_radians: 0.0,
            }],
        };
        assert!(spectral_similarity(&a, &near) > spectral_similarity(&a, &far));
    }

    #[test]
    fn genome_cavity_receives_environmental_material_at_realized_boundary() {
        let catalog = crate::resources::default_catalog();
        let blueprint = crate::juvenile::confirmed_seed_baseline(&catalog).unwrap();
        let (mut structure, _, _) = crate::juvenile::realize_initial(&blueprint, &catalog).unwrap();
        for unit in &mut structure.units {
            unit.placement.x += 500.0;
            unit.placement.y += 500.0;
        }
        let cavity = crate::cavity::analyze_genome_cavity(&structure, &catalog)
            .unwrap()
            .expect("confirmed seed must contain a genome cavity");
        let mut field = crate::environment::ActiveMaterialField::new(
            1000.0,
            1000.0,
            crate::environment::DEFAULT_CELL_SIZE,
        );

        let baseline = genome_cavity_spectrum(&catalog, &field, &cavity);
        let boundary = &structure.units[cavity.boundary_units[0]].placement;
        let source = crate::physical_material::PhysicalMaterial::realized(
            crate::resources::Material::free_base("Carbon", 1.0),
            vec![crate::structure::Placement {
                x: boundary.x + 40.0,
                y: boundary.y,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .unwrap();
        assert!(field.deposit_physical(boundary.x + 40.0, boundary.y, source, &catalog));
        let coupled = genome_cavity_spectrum(&catalog, &field, &cavity);

        let baseline_fundamental = baseline
            .components
            .iter()
            .find(|component| {
                (component.frequency_cycles_per_day - WORLD_TONE_CYCLES_PER_DAY).abs()
                    < f64::EPSILON
            })
            .map(|component| component.amplitude)
            .unwrap_or(0.0);
        let coupled_fundamental = coupled
            .components
            .iter()
            .find(|component| {
                (component.frequency_cycles_per_day - WORLD_TONE_CYCLES_PER_DAY).abs()
                    < f64::EPSILON
            })
            .map(|component| component.amplitude)
            .unwrap_or(0.0);
        assert!(coupled_fundamental > baseline_fundamental);
    }

    #[test]
    fn rotating_world_tone_is_filtered_by_cavity_wall_and_keeps_source_location() {
        let catalog = crate::resources::default_catalog();
        let blueprint = crate::juvenile::confirmed_seed_baseline(&catalog).unwrap();
        let (mut structure, _, _) = crate::juvenile::realize_initial(&blueprint, &catalog).unwrap();
        for unit in &mut structure.units {
            unit.placement.x += 500.0;
            unit.placement.y += 500.0;
        }
        let cavity = crate::cavity::analyze_genome_cavity(&structure, &catalog)
            .unwrap()
            .expect("confirmed seed must contain a genome cavity");
        let north = crate::world_tone::WorldToneSample {
            source_x: 500.0,
            source_y: 0.0,
            frequency_cycles_per_day: 440.0,
            energy_intensity_per_tick_per_width: 10.0,
            orbital_phase: 0.0,
        };
        let (north_spectrum, north_perceptions) =
            world_tone_cavity_contribution(&catalog, &structure, &cavity, north);
        assert!(!north_spectrum.components.is_empty());
        assert!(!north_perceptions.is_empty());
        assert!(north_perceptions.iter().all(|perception| {
            perception.source_x == north.source_x
                && perception.source_y == north.source_y
                && perception.magnitude > 0.0
        }));
        assert!(north_spectrum.components.iter().any(|component| (component
            .frequency_cycles_per_day
            - 440.0)
            .abs()
            < 1e-9));

        let south = crate::world_tone::WorldToneSample {
            source_x: 500.0,
            source_y: 1000.0,
            frequency_cycles_per_day: 110.0,
            energy_intensity_per_tick_per_width: 1.0,
            orbital_phase: 0.5,
        };
        let (south_spectrum, south_perceptions) =
            world_tone_cavity_contribution(&catalog, &structure, &cavity, south);
        assert!(!south_spectrum.components.is_empty());
        assert!(!south_perceptions.is_empty());
    }

    #[test]
    fn harmonic_reception_depends_on_qualifying_realized_cavity() {
        let catalog = crate::resources::default_catalog();
        let blueprint = crate::juvenile::confirmed_seed_baseline(&catalog).unwrap();
        let (mut structure, _, _) = crate::juvenile::realize_initial(&blueprint, &catalog).unwrap();
        let cavity = crate::cavity::analyze_genome_cavity(&structure, &catalog)
            .unwrap()
            .expect("confirmed seed must contain a genome cavity");
        let field = crate::environment::ActiveMaterialField::new(
            1000.0,
            1000.0,
            crate::environment::DEFAULT_CELL_SIZE,
        );

        let received = genome_cavity_spectrum(&catalog, &field, &cavity);
        assert!(received.components.is_empty());

        let boundary = &structure.units[cavity.boundary_units[0]].placement;
        let source = crate::physical_material::PhysicalMaterial::realized(
            crate::resources::Material::free_base("Carbon", 1.0),
            vec![crate::structure::Placement {
                x: boundary.x + 40.0,
                y: boundary.y,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .unwrap();
        let mut field = field;
        assert!(field.deposit_physical(boundary.x + 40.0, boundary.y, source, &catalog));
        let received = genome_cavity_spectrum(&catalog, &field, &cavity);
        assert!(!received.components.is_empty());

        structure.bonds.clear();
        assert!(crate::cavity::analyze_genome_cavity(&structure, &catalog)
            .unwrap()
            .is_none());
        let absent = ToneSpectrum::empty();
        assert!(absent.components.is_empty());
    }

    #[test]
    fn resonance_perception_preserves_environmental_source_location() {
        let catalog = crate::resources::default_catalog();
        let blueprint = crate::juvenile::confirmed_seed_baseline(&catalog).unwrap();
        let (mut structure, _, _) = crate::juvenile::realize_initial(&blueprint, &catalog).unwrap();
        for unit in &mut structure.units {
            unit.placement.x += 500.0;
            unit.placement.y += 500.0;
        }
        let cavity = crate::cavity::analyze_genome_cavity(&structure, &catalog)
            .unwrap()
            .expect("confirmed seed must contain a genome cavity");
        let mut field = crate::environment::ActiveMaterialField::new(
            1000.0,
            1000.0,
            crate::environment::DEFAULT_CELL_SIZE,
        );
        let boundary = &structure.units[cavity.boundary_units[0]].placement;
        let source = crate::physical_material::PhysicalMaterial::realized(
            crate::resources::Material::free_base("Carbon", 1.0),
            vec![crate::structure::Placement {
                x: boundary.x + 40.0,
                y: boundary.y,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .unwrap();
        assert!(field.deposit_physical(boundary.x + 40.0, boundary.y, source, &catalog));

        let perceptions = genome_cavity_resonance_perceptions(&catalog, &field, &cavity);
        assert!(!perceptions.is_empty());
        assert!(perceptions.iter().any(|perception| {
            (perception.source_x - (boundary.x + 40.0)).abs() < f64::EPSILON
                && (perception.source_y - boundary.y).abs() < f64::EPSILON
                && perception.magnitude > 0.0
        }));
    }

    fn properties(mass: f64, reactivity: f64, cohesion: f64) -> ResourceProperties {
        ResourceProperties {
            mass,
            potential_energy: 1.0,
            reactivity,
            chemical_position: None,
            cohesion,
        }
    }

    fn baselines() -> ResourceBaselines {
        ResourceBaselines {
            mass: 1.0,
            potential_energy: 1.0,
            reactivity: 1.0,
            cohesion: 1.0,
        }
    }

    #[test]
    fn world_reference_is_440_hz() {
        assert_eq!(WORLD_TONE_CYCLES_PER_DAY, 440.0);
    }

    #[test]
    fn natural_frequency_follows_cohesion_over_mass() {
        assert!(
            (natural_frequency_cycles_per_day(properties(1.0, 0.0, 1.0), baselines()) - 440.0)
                .abs()
                < 1e-9
        );
        assert!(natural_frequency_cycles_per_day(properties(0.5, 0.0, 1.0), baselines()) > 440.0);
        assert!(natural_frequency_cycles_per_day(properties(1.0, 0.0, 0.5), baselines()) < 440.0);
    }

    #[test]
    fn resonance_math_fails_closed_for_invalid_material_properties() {
        assert!(
            natural_frequency_cycles_per_day(properties(f64::NAN, 0.0, 1.0), baselines()).is_nan()
        );
        assert!(
            natural_frequency_cycles_per_day(properties(1.0, 0.0, f64::INFINITY), baselines())
                .is_nan()
        );
        assert!(damping_ratio(properties(1.0, f64::NAN, 1.0), baselines()).is_nan());
        assert!(damping_ratio(
            properties(1.0, 1.0, 1.0),
            ResourceBaselines {
                reactivity: f64::INFINITY,
                ..baselines()
            }
        )
        .is_nan());
        assert_eq!(resonance_response(440.0, 440.0, f64::NAN), 0.0);
        assert_eq!(nonlinear_harmonic_amplitude(1.0, f64::NAN, 2), 0.0);
        assert!(
            material_response(properties(1.0, f64::NAN, 1.0), baselines(), 0.0,)
                .components
                .is_empty()
        );
    }

    #[test]
    fn reactivity_sets_relative_damping_and_nonlinear_content() {
        let baseline = damping_ratio(properties(1.0, 1.0, 1.0), baselines());
        let high = damping_ratio(properties(1.0, 4.0, 1.0), baselines());
        assert!((baseline - BASELINE_DAMPING_RATIO).abs() < 1e-12);
        assert!((high - 4.0 * BASELINE_DAMPING_RATIO).abs() < 1e-12);
        assert_eq!(damping_ratio(properties(1.0, 0.0, 1.0), baselines()), 0.0);
        assert_eq!(nonlinear_harmonic_amplitude(1.0, 0.0, 2), 0.0);
        assert!((nonlinear_harmonic_amplitude(1.0, 1.0, 2) - 0.25).abs() < 1e-12);
        assert!(nonlinear_harmonic_amplitude(1.0, 4.0, 2) > 0.0);
    }

    #[test]
    fn environmental_mass_scale_is_quantity_sensitive() {
        assert!((environmental_mass_scale(1.0, 1.0) - 0.5).abs() < 1e-12);
        assert!(environmental_mass_scale(10.0, 1.0) > environmental_mass_scale(1.0, 1.0));
        assert!(environmental_mass_scale(0.1, 1.0) < environmental_mass_scale(1.0, 1.0));
    }

    #[test]
    fn spectrum_magnitude_is_root_sum_square_of_component_amplitudes() {
        let spectrum = ToneSpectrum {
            components: vec![
                ToneComponent {
                    frequency_cycles_per_day: 440.0,
                    amplitude: 0.6,
                    phase_radians: 0.0,
                },
                ToneComponent {
                    frequency_cycles_per_day: 880.0,
                    amplitude: 0.8,
                    phase_radians: 0.0,
                },
            ],
        };
        assert!((spectrum_magnitude(&spectrum) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn resonant_response_and_harmonics_cannot_exceed_incident_energy() {
        let incident_amplitude = 0.75;
        let spectrum = material_response_to_drive(
            properties(1.0, 4.0, 1.0),
            baselines(),
            440.0,
            incident_amplitude,
            0.0,
        );
        let spectral_energy: f64 = spectrum
            .components
            .iter()
            .map(|component| wave_energy_from_amplitude(component.amplitude))
            .sum();
        let incident_energy = wave_energy_from_amplitude(incident_amplitude);
        assert!(spectral_energy <= incident_energy + 1e-12);
        assert!(spectrum
            .components
            .iter()
            .any(
                |component| (component.frequency_cycles_per_day - 2.0 * 440.0).abs() < f64::EPSILON
            ));
    }

    #[test]
    fn unrepresentable_spectral_energy_fails_closed() {
        let mut spectrum = ToneSpectrum {
            components: vec![ToneComponent {
                frequency_cycles_per_day: 440.0,
                amplitude: f64::MAX,
                phase_radians: 0.0,
            }],
        };
        bound_spectrum_energy(&mut spectrum, 1.0);
        assert!(spectrum.components.is_empty());
    }

    #[test]
    fn transformed_secondary_signal_preserves_budget_and_material_harmonics() {
        let input = ToneSpectrum {
            components: vec![ToneComponent {
                frequency_cycles_per_day: 440.0,
                amplitude: 2.0,
                phase_radians: 0.0,
            }],
        };
        let output =
            transform_signal_to_energy_budget(properties(1.0, 2.0, 1.0), baselines(), &input, 2.5)
                .expect("valid residual budget should produce a transformed spectrum");
        let energy: f64 = output
            .components
            .iter()
            .map(|component| component.amplitude * component.amplitude)
            .sum();
        assert!((energy - 2.5).abs() < 1e-9);
        assert!(output
            .components
            .iter()
            .any(|component| (component.frequency_cycles_per_day - 880.0).abs() < 1e-9));
        assert!(transform_signal_to_energy_budget(
            properties(1.0, 2.0, 1.0),
            baselines(),
            &input,
            4.1,
        )
        .is_none());
    }

    #[test]
    fn transformed_secondary_signal_never_exceeds_budget_after_rounding() {
        let input = ToneSpectrum {
            components: vec![
                ToneComponent {
                    frequency_cycles_per_day: 440.0,
                    amplitude: 1.7,
                    phase_radians: 0.0,
                },
                ToneComponent {
                    frequency_cycles_per_day: 660.0,
                    amplitude: 1.3,
                    phase_radians: 0.2,
                },
                ToneComponent {
                    frequency_cycles_per_day: 880.0,
                    amplitude: 1.4,
                    phase_radians: -0.1,
                },
            ],
        };
        for budget in [0.1, 0.3, 0.7, 1.1, 1.7] {
            let output = transform_signal_to_energy_budget(
                properties(1.0, 2.0, 1.0),
                baselines(),
                &input,
                budget,
            )
            .expect("valid budget should produce a bounded spectrum");
            let energy = spectrum_energy(&output).expect("transformed spectrum is valid");
            assert!(
                energy <= budget,
                "transformed energy {energy} exceeded requested budget {budget}"
            );
            assert!(
                (budget - energy).abs() <= 1e-9 * budget.max(1.0),
                "downward rounding correction exceeded tolerance: budget={budget}, energy={energy}"
            );
        }
    }

    #[test]
    fn material_response_is_compact_and_harmonic() {
        let spectrum = material_response(properties(1.0, 4.0, 1.0), baselines(), 0.25);
        assert!(!spectrum.components.is_empty());
        assert!(spectrum
            .components
            .iter()
            .any(
                |component| (component.frequency_cycles_per_day - WORLD_TONE_CYCLES_PER_DAY).abs()
                    < f64::EPSILON
            ));
        assert!(spectrum.components.iter().any(|component| {
            (component.frequency_cycles_per_day - 2.0 * WORLD_TONE_CYCLES_PER_DAY).abs()
                < f64::EPSILON
        }));
        assert!(spectrum.components.len() <= MAX_SPECTRAL_COMPONENTS);
    }

    #[test]
    fn every_generated_harmonic_advances_from_the_shared_phase_clock() {
        let tick = 137;
        let drive_frequency = WORLD_TONE_CYCLES_PER_DAY;
        let drive_phase = phase_at_tick(drive_frequency, tick);
        let spectrum = material_response(properties(1.0, 4.0, 1.0), baselines(), drive_phase);

        assert!(!spectrum.components.is_empty());
        for component in &spectrum.components {
            let expected_phase = phase_at_tick(component.frequency_cycles_per_day, tick);
            assert!(
                (component.phase_radians.cos() - expected_phase.cos()).abs() < 1e-10
                    && (component.phase_radians.sin() - expected_phase.sin()).abs() < 1e-10,
                "harmonic at {} cycles/day did not follow the shared clock",
                component.frequency_cycles_per_day
            );
        }
    }
}
