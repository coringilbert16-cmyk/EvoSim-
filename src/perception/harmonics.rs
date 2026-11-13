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

/// Exact visible boundary geometry associated with a reception interval.
#[derive(Clone, Copy, Debug, PartialEq)]
enum MaterialAuraBoundary {
    Straight {
        start: (f64, f64),
        end: (f64, f64),
        normal: (f64, f64),
    },
    Arc(crate::capillary_geometry::CircularArcBoundary),
}

impl MaterialAuraBoundary {
    #[allow(dead_code)] // Retained for later organism-scale occlusion geometry.
    fn normal_at(self, point: (f64, f64)) -> Option<(f64, f64)> {
        const TOLERANCE: f64 = 1e-7;
        match self {
            Self::Straight { start, end, normal } => {
                let dx = end.0 - start.0;
                let dy = end.1 - start.1;
                let length_squared = dx * dx + dy * dy;
                if !length_squared.is_finite() || length_squared <= 1e-24 {
                    return None;
                }
                let t = ((point.0 - start.0) * dx + (point.1 - start.1) * dy) / length_squared;
                if !t.is_finite() || !(-TOLERANCE..=1.0 + TOLERANCE).contains(&t) {
                    return None;
                }
                let closest = (
                    start.0 + t.clamp(0.0, 1.0) * dx,
                    start.1 + t.clamp(0.0, 1.0) * dy,
                );
                ((point.0 - closest.0).hypot(point.1 - closest.1) <= TOLERANCE).then_some(normal)
            }
            Self::Arc(arc) => {
                if !arc.center_x.is_finite()
                    || !arc.center_y.is_finite()
                    || !arc.radius.is_finite()
                    || arc.radius <= f64::EPSILON
                    || !arc.start_angle_radians.is_finite()
                    || !arc.sweep_radians.is_finite()
                    || arc.sweep_radians <= 0.0
                {
                    return None;
                }
                let dx = point.0 - arc.center_x;
                let dy = point.1 - arc.center_y;
                let radius = dx.hypot(dy);
                if !radius.is_finite() || (radius - arc.radius).abs() > TOLERANCE {
                    return None;
                }
                let angle = dy.atan2(dx);
                let offset = (angle - arc.start_angle_radians).rem_euclid(std::f64::consts::TAU);
                (offset <= arc.sweep_radians + TOLERANCE).then_some((dx / radius, dy / radius))
            }
        }
    }
}

/// One material's existing resonance aura as received at a realized
/// boundary point on another physical material.
///
/// This is a pairwise reception sample, not a force or energy transaction.
/// It preserves the receiver geometry needed for a later spatial interaction
/// calculation without inventing a second field or perception radius.
#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)] // Consumed by the staged pair-interaction implementation.
pub(crate) struct MaterialAuraReception {
    pub(crate) source_x: f64,
    pub(crate) source_y: f64,
    pub(crate) source_extent: f64,
    pub(crate) receiver_x: f64,
    pub(crate) receiver_y: f64,
    pub(crate) receiver_normal_x: f64,
    pub(crate) receiver_normal_y: f64,
    pub(crate) projected_extent: f64,
    boundary: MaterialAuraBoundary,
    /// Incident field before the receiver's oscillator response. Pair
    /// interference compares this with the receiver's own emitted field;
    /// using the responded spectrum here would apply the receiver phase lag twice.
    pub(crate) incident_spectrum: ToneSpectrum,
    /// Receiver response retained for perception/diagnostics.
    pub(crate) spectrum: ToneSpectrum,
    pub(crate) magnitude: f64,
}

/// One spatial sample of the material-origin resonance-density field.
///
/// Samples are taken on the realized boundary of a selected receiver material.
/// Density is the phase-combined, frequency-resolved incident signal intensity
/// at that location, normalized by projected boundary extent. It is a signal
/// diagnostic, not stored or spendable energy.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MaterialResonanceDensitySample {
    pub(crate) cell_x: i64,
    pub(crate) cell_y: i64,
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) components: Vec<ToneComponent>,
    pub(crate) density: f64,
}

/// Build a local resonance-density heat map over one realized receiver.
///
/// Every other supplied material is considered as a possible emitter. Other
/// materials do not occlude chemistry-scale resonance; receiver geometry is
/// used only to choose spatially meaningful boundary sample locations. The
/// spatial grid is diagnostic sampling only: reaction code must use continuous
/// physical samples, never make eligibility depend on this grid.
#[allow(dead_code)] // Diagnostic field sampler; reaction integration follows energy-provenance validation.
pub(crate) fn material_resonance_density_on_receiver(
    materials: &[crate::physical_material::PhysicalMaterial],
    source_indices: &[usize],
    receiver_index: usize,
    catalog: &[crate::resources::BaseResource],
    tick: u64,
    cell_size: f64,
) -> Option<Vec<MaterialResonanceDensitySample>> {
    if !cell_size.is_finite() || cell_size <= f64::EPSILON {
        return None;
    }
    let receiver = materials.get(receiver_index)?;
    let mut seen_sources = std::collections::BTreeSet::new();
    if source_indices.iter().any(|index| {
        *index >= materials.len() || *index == receiver_index || !seen_sources.insert(*index)
    }) {
        return None;
    }

    // First average repeated boundary patches from each source within a map
    // cell. This prevents a finely segmented receiver boundary from counting
    // the same source field multiple times merely because it has more patches.
    let mut source_cells =
        std::collections::BTreeMap::<(i64, i64, usize), Vec<(ToneComponent, usize)>>::new();

    for &source_index in source_indices {
        let source = materials.get(source_index)?;
        // Receiver geometry supplies useful map coordinates, but chemistry
        // resonance itself is sampled through the unoccluded point-field API.
        // In particular, no third material can hide a source from this map.
        let receptions = material_aura_receptions(source, receiver, catalog, tick)?;
        for reception in receptions {
            if !reception.receiver_x.is_finite() || !reception.receiver_y.is_finite() {
                return None;
            }
            let cell_x_f = (reception.receiver_x / cell_size).floor();
            let cell_y_f = (reception.receiver_y / cell_size).floor();
            if !cell_x_f.is_finite()
                || !cell_y_f.is_finite()
                || cell_x_f < i64::MIN as f64
                || cell_x_f > i64::MAX as f64
                || cell_y_f < i64::MIN as f64
                || cell_y_f > i64::MAX as f64
            {
                return None;
            }
            let key = (cell_x_f as i64, cell_y_f as i64, source_index);
            let spectra = material_resonance_spectra_at_boundary_points(
                materials,
                &[source_index],
                receiver_index,
                &[(reception.receiver_x, reception.receiver_y)],
                catalog,
                tick,
            )?;
            let spectrum = spectra.first()?;
            let samples = source_cells.entry(key).or_default();
            for component in &spectrum.components {
                if !component.frequency_cycles_per_day.is_finite()
                    || component.frequency_cycles_per_day <= 0.0
                    || !component.amplitude.is_finite()
                    || component.amplitude < 0.0
                    || !component.phase_radians.is_finite()
                {
                    return None;
                }
                if let Some((existing, count)) = samples.iter_mut().find(|(existing, _)| {
                    (existing.frequency_cycles_per_day - component.frequency_cycles_per_day).abs()
                        <= 1e-9
                }) {
                    *existing = combine_component(*existing, *component);
                    *count += 1;
                } else {
                    samples.push((*component, 1));
                }
            }
        }
    }

    let mut cells = std::collections::BTreeMap::<(i64, i64), Vec<ToneComponent>>::new();
    for ((cell_x, cell_y, _source_index), source_components) in source_cells {
        let target = cells.entry((cell_x, cell_y)).or_default();
        for (component, sample_count) in source_components {
            let averaged = ToneComponent {
                amplitude: component.amplitude / sample_count as f64,
                ..component
            };
            if let Some(existing) = target.iter_mut().find(|existing| {
                (existing.frequency_cycles_per_day - averaged.frequency_cycles_per_day).abs()
                    <= 1e-9
            }) {
                *existing = combine_component(*existing, averaged);
            } else {
                target.push(averaged);
            }
        }
    }

    let mut output = Vec::with_capacity(cells.len());
    for ((cell_x, cell_y), mut components) in cells {
        components.sort_by(|a, b| {
            a.frequency_cycles_per_day
                .partial_cmp(&b.frequency_cycles_per_day)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let density = components
            .iter()
            .map(|component| component.amplitude * component.amplitude)
            .sum::<f64>();
        if !density.is_finite() {
            return None;
        }
        output.push(MaterialResonanceDensitySample {
            cell_x,
            cell_y,
            x: (cell_x as f64 + 0.5) * cell_size,
            y: (cell_y as f64 + 0.5) * cell_size,
            components,
            density,
        });
    }
    output.sort_by(|a, b| {
        a.cell_y
            .cmp(&b.cell_y)
            .then_with(|| a.cell_x.cmp(&b.cell_x))
    });
    Some(output)
}

/// One signed frequency contribution to a material pair's interaction
/// potential. Signed energy is required so PR 4 can settle attractive and
/// repulsive work against the matching retained-resonance frequency.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ResonancePairPotentialComponent {
    pub(crate) frequency_cycles_per_day: f64,
    pub(crate) energy: f64,
}

/// Translation force on one participant in a resonance interaction.
///
/// The order returned by the multi-body gradient is source, receiver, then
/// occluders in caller-supplied order. Forces are all derived from the same
/// frequency-resolved symmetric potential.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct ResonanceBodyForce {
    pub(crate) x: f64,
    pub(crate) y: f64,
    /// Torque about the body's center of mass from the same potential gradient.
    pub(crate) torque: f64,
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

/// Phase lag of a damped, driven harmonic oscillator's displacement response.
///
/// For frequency ratio r = drive / natural and damping ratio ζ, the
/// steady-state transfer function has denominator
/// (1 - r²) + i(2ζr). Its argument is the lag, so the response phase is
/// the drive phase minus this value. This is the standard linear oscillator
/// result; it introduces no propagation delay or arbitrary phase offset.
pub(crate) fn oscillator_phase_lag(
    drive_frequency_cycles_per_day: f64,
    natural_frequency_cycles_per_day: f64,
    damping: f64,
) -> f64 {
    if !drive_frequency_cycles_per_day.is_finite()
        || !natural_frequency_cycles_per_day.is_finite()
        || !damping.is_finite()
        || drive_frequency_cycles_per_day <= 0.0
        || natural_frequency_cycles_per_day <= 0.0
        || damping < 0.0
    {
        return 0.0;
    }

    let ratio = drive_frequency_cycles_per_day / natural_frequency_cycles_per_day;
    if !ratio.is_finite() || ratio <= 0.0 {
        return 0.0;
    }

    let zeta = damping.max(f64::EPSILON);
    (2.0 * zeta * ratio).atan2(1.0 - ratio * ratio)
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
    let response_phase =
        phase_radians - oscillator_phase_lag(drive_frequency_cycles_per_day, natural, damping);
    let mut spectrum = ToneSpectrum {
        components: vec![ToneComponent {
            frequency_cycles_per_day: drive_frequency_cycles_per_day,
            amplitude: fundamental,
            phase_radians: response_phase,
        }],
    };

    for order in 2..=MAX_SPECTRAL_COMPONENTS {
        let amplitude = nonlinear_harmonic_amplitude(fundamental, properties.reactivity, order);
        if amplitude > f64::EPSILON {
            spectrum.components.push(ToneComponent {
                frequency_cycles_per_day: drive_frequency_cycles_per_day * order as f64,
                amplitude,
                phase_radians: response_phase * order as f64,
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

/// Cross-energy contributed by same-frequency components when two
/// spectra are superposed as the model's existing complex phasors.
///
/// Different frequencies have no cycle-averaged cross term. This returns
/// only the interference term (combined energy minus the two separate
/// energies); it is not by itself a mechanical potential or force.
#[allow(dead_code)] // Consumed by the staged pair-interaction implementation.
pub(crate) fn spectrum_cross_energy(a: &ToneSpectrum, b: &ToneSpectrum) -> Option<f64> {
    let components = spectrum_cross_energy_components(a, b)?;
    let energy = components
        .iter()
        .map(|component| component.energy)
        .sum::<f64>();
    energy.is_finite().then_some(energy)
}

/// Preserve the cross-energy term by frequency so conservative work can
/// be withdrawn from or returned to a compatible resonance component.
fn spectrum_cross_energy_components(
    a: &ToneSpectrum,
    b: &ToneSpectrum,
) -> Option<Vec<ResonancePairPotentialComponent>> {
    spectrum_energy(a)?;
    spectrum_energy(b)?;
    let mut components = Vec::<ResonancePairPotentialComponent>::new();
    for left in &a.components {
        for right in &b.components {
            if (left.frequency_cycles_per_day - right.frequency_cycles_per_day).abs() > 1e-9 {
                continue;
            }
            let energy = 2.0
                * left.amplitude
                * right.amplitude
                * (left.phase_radians - right.phase_radians).cos();
            if !energy.is_finite() {
                return None;
            }
            if let Some(existing) = components.iter_mut().find(|component| {
                (component.frequency_cycles_per_day - left.frequency_cycles_per_day).abs() <= 1e-9
            }) {
                existing.energy += energy;
                if !existing.energy.is_finite() {
                    return None;
                }
            } else {
                components.push(ResonancePairPotentialComponent {
                    frequency_cycles_per_day: left.frequency_cycles_per_day,
                    energy,
                });
            }
        }
    }
    components.sort_by(|a, b| {
        a.frequency_cycles_per_day
            .partial_cmp(&b.frequency_cycles_per_day)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Some(components)
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

/// Tick-zero convenience retained for tests that characterize the static initial condition.
#[cfg(test)]
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
fn environmental_material_geometry(
    physical: &crate::physical_material::PhysicalMaterial,
) -> Option<(f64, f64, f64)> {
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
    let extent = placements
        .iter()
        .map(|placement| (placement.x - source_x).hypot(placement.y - source_y))
        .fold(0.0_f64, f64::max);
    if !source_x.is_finite() || !source_y.is_finite() || !extent.is_finite() {
        return None;
    }
    Some((source_x, source_y, extent))
}

fn environmental_material_resonance(
    physical: &crate::physical_material::PhysicalMaterial,
    catalog: &[crate::resources::BaseResource],
    tick: u64,
) -> Option<(f64, f64, f64, ToneSpectrum)> {
    let (source_x, source_y, extent) = environmental_material_geometry(physical)?;
    let baselines = ResourceBaselines::from_catalog(catalog);
    let spectrum = material_response(
        physical.material.weighted_properties(catalog),
        baselines,
        phase_at_tick(WORLD_TONE_CYCLES_PER_DAY, tick),
    );
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

/// Receive one realized physical material's aura at another realized
/// physical material's actual exposed constituent boundaries.
///
/// The source uses the same aggregate oscillator and mass scaling as the
/// environmental aura. Receiver intervals, normals, and occlusion use the
/// shared realized-shape visibility geometry for straight and curved boundaries.
/// Distance attenuation remains the existing continuous 1/(1 + distance) law.
/// The receiver's own oscillator response is applied after reception.
///
/// Each sample carries its exact projected boundary extent. Amplitude scales
/// with the square root of that extent because the model's signal energy is
/// amplitude squared. These samples are not yet a pair potential or force.
#[allow(dead_code)] // Staged for the force/potential work in this PR.
pub(crate) fn material_aura_receptions(
    source: &crate::physical_material::PhysicalMaterial,
    receiver: &crate::physical_material::PhysicalMaterial,
    catalog: &[crate::resources::BaseResource],
    tick: u64,
) -> Option<Vec<MaterialAuraReception>> {
    material_aura_receptions_with_occluders(source, receiver, &[], catalog, tick)
}

/// Pairwise reception with all other physical bodies retained as occluders.
/// The caller supplies only bodies distinct from source and receiver; they
/// block visibility but are not themselves sampled as receivers.
pub(crate) fn material_aura_receptions_with_occluders(
    source: &crate::physical_material::PhysicalMaterial,
    receiver: &crate::physical_material::PhysicalMaterial,
    occluders: &[crate::physical_material::PhysicalMaterial],
    catalog: &[crate::resources::BaseResource],
    tick: u64,
) -> Option<Vec<MaterialAuraReception>> {
    // This API consumes world-resolved placements. Stored materials carrying
    // an owner-relative origin must be transformed by their owner first.
    if source.owner_relative_origin.is_some()
        || receiver.owner_relative_origin.is_some()
        || occluders
            .iter()
            .any(|material| material.owner_relative_origin.is_some())
    {
        return None;
    }
    if std::ptr::eq(source, receiver) {
        return Some(Vec::new());
    }
    let receiver_geometry =
        crate::organism_geometry::OrganismBodyGeometry::from_physical_material(receiver, catalog)?;
    let source_geometry =
        crate::organism_geometry::OrganismBodyGeometry::from_physical_material(source, catalog)?;
    let source_placements = source.placements.as_ref()?;
    if source_placements.is_empty() || source_placements.len() != source.material.parts.len() {
        return None;
    }
    let source_count = source_placements.len() as f64;
    let source_x = source_placements
        .iter()
        .map(|placement| placement.x)
        .sum::<f64>()
        / source_count;
    let source_y = source_placements
        .iter()
        .map(|placement| placement.y)
        .sum::<f64>()
        / source_count;
    if !source_x.is_finite() || !source_y.is_finite() {
        return None;
    }
    // Every source-to-receiver-boundary segment lies inside this AABB. A body
    // outside it cannot occlude any sampled ray, so retain only geometries
    // whose bounds intersect the conservative envelope. No relevant blocker
    // is truncated and no distance cutoff is introduced.
    let receiver_envelope = [
        (receiver_geometry.min_x, receiver_geometry.min_y),
        (receiver_geometry.min_x, receiver_geometry.max_y),
        (receiver_geometry.max_x, receiver_geometry.min_y),
        (receiver_geometry.max_x, receiver_geometry.max_y),
    ];

    // Keep the receiver first as the only target. Other bodies remain exact
    // occluders; the source itself is excluded because its aura originates
    // inside its own geometry.
    let mut receiver_bodies = Vec::with_capacity(1 + occluders.len());
    receiver_bodies.push(receiver_geometry);
    for occluder in occluders {
        if std::ptr::eq(occluder, source) || std::ptr::eq(occluder, receiver) {
            continue;
        }
        let geometry = crate::organism_geometry::OrganismBodyGeometry::from_physical_material(
            occluder, catalog,
        )?;
        if geometry_bounds_may_occlude_source_points(
            (source_x, source_y),
            &receiver_envelope,
            &geometry,
        ) {
            receiver_bodies.push(geometry);
        }
    }

    material_aura_receptions_with_geometry(
        source,
        receiver,
        &source_geometry,
        &receiver_bodies,
        catalog,
        tick,
    )
}

/// Conservative AABB broad phase for possible occluders between a point
/// source and the sampled receiver endpoints. Every source-to-endpoint segment
/// lies inside the envelope of the source and requested points.
fn geometry_bounds_may_occlude_source_points(
    source: (f64, f64),
    points: &[(f64, f64)],
    candidate: &crate::organism_geometry::OrganismBodyGeometry,
) -> bool {
    let mut min_x = source.0;
    let mut max_x = source.0;
    let mut min_y = source.1;
    let mut max_y = source.1;
    for &(x, y) in points {
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    !(candidate.max_x < min_x
        || candidate.min_x > max_x
        || candidate.max_y < min_y
        || candidate.min_y > max_y)
}

fn receiver_has_straight_boundary(
    receiver: &crate::organism_geometry::OrganismBodyGeometry,
) -> bool {
    receiver
        .parts
        .iter()
        .any(|part| part.form.polygon_vertices().is_some())
}

fn receiver_has_curved_boundary(receiver: &crate::organism_geometry::OrganismBodyGeometry) -> bool {
    receiver.parts.iter().any(|part| match &part.form {
        crate::resources::Form::Circle { .. } => true,
        crate::resources::Form::Fluid {
            boundary: None,
            arc_boundary: Some(_),
            ..
        } => true,
        crate::resources::Form::Fluid {
            boundary: None,
            arc_boundary: None,
            multi_contact_boundary: Some(multi),
            ..
        } => multi.pieces.iter().any(|piece| {
            matches!(
                piece,
                crate::capillary_geometry::FluidBoundaryPiece::FreeArc(_)
            )
        }),
        crate::resources::Form::Fluid {
            boundary: None,
            arc_boundary: None,
            multi_contact_boundary: None,
            ..
        } => true,
        _ => false,
    })
}

/// Evaluate one source against prebuilt world geometry. The receiver must be
/// the first body in `receiver_bodies`; all remaining bodies are occluders.
fn material_aura_receptions_with_geometry(
    source: &crate::physical_material::PhysicalMaterial,
    receiver: &crate::physical_material::PhysicalMaterial,
    source_geometry: &crate::organism_geometry::OrganismBodyGeometry,
    receiver_bodies: &[crate::organism_geometry::OrganismBodyGeometry],
    catalog: &[crate::resources::BaseResource],
    tick: u64,
) -> Option<Vec<MaterialAuraReception>> {
    if source.owner_relative_origin.is_some()
        || receiver.owner_relative_origin.is_some()
        || receiver_bodies.is_empty()
        || source_geometry.parts.is_empty()
    {
        return None;
    }
    let (source_x, source_y, source_extent, emitted) =
        environmental_material_resonance(source, catalog, tick)?;

    let target_body_indices = [0];
    let receiver_geometry = receiver_bodies.first()?;
    let straight_intervals = if receiver_has_straight_boundary(receiver_geometry) {
        crate::source_exposure::visible_polygonal_intervals_for_targets(
            (source_x, source_y),
            receiver_bodies,
            &target_body_indices,
        )?
    } else {
        Vec::new()
    };
    let curved_intervals = if receiver_has_curved_boundary(receiver_geometry) {
        crate::source_exposure::visible_circular_arc_intervals_for_targets(
            (source_x, source_y),
            receiver_bodies,
            &target_body_indices,
        )?
    } else {
        Vec::new()
    };

    let baselines = ResourceBaselines::from_catalog(catalog);
    let receiver_properties = receiver.material.weighted_properties(catalog);
    let source_mass_scale = environmental_mass_scale(source.material.mass(catalog), baselines.mass);
    if source_mass_scale <= f64::EPSILON {
        return Some(Vec::new());
    }

    let mut receptions = Vec::with_capacity(straight_intervals.len() + curved_intervals.len());
    let mut receive_at = |receiver_x: f64,
                          receiver_y: f64,
                          normal_x: f64,
                          normal_y: f64,
                          projected_extent: f64,
                          boundary: MaterialAuraBoundary| {
        if !projected_extent.is_finite() || projected_extent <= f64::EPSILON {
            return Some(());
        }
        let distance = (source_x - receiver_x).hypot(source_y - receiver_y);
        if !distance.is_finite() || distance <= f64::EPSILON {
            return Some(());
        }
        let attenuation = source_mass_scale * projected_extent.sqrt() / (1.0 + distance);
        if !attenuation.is_finite() || attenuation <= f64::EPSILON {
            return Some(());
        }
        let mut incident = ToneSpectrum::empty();
        add_spectrum(&mut incident, &emitted, attenuation);
        incident.retain_strongest();
        let mut received = material_response_spectrum(receiver_properties, baselines, &incident)?;
        received.retain_strongest();
        let magnitude = spectrum_magnitude(&received);
        // Geometric reception is independent of whether this material can
        // respond to the incident field. Preserve the raw incident sample
        // for pair interaction even when (for example) cohesion is zero
        // and the receiver's own response spectrum is empty.
        if spectrum_magnitude(&incident) > f64::EPSILON {
            receptions.push(MaterialAuraReception {
                source_x,
                source_y,
                source_extent,
                receiver_x,
                receiver_y,
                receiver_normal_x: normal_x,
                receiver_normal_y: normal_y,
                projected_extent,
                boundary,
                incident_spectrum: incident,
                spectrum: received,
                magnitude,
            });
        }
        Some(())
    };

    for interval in straight_intervals {
        let projected_extent =
            crate::source_exposure::point_source_projected_extent(interval, (source_x, source_y))?;
        let receiver_x = (interval.start.0 + interval.end.0) * 0.5;
        let receiver_y = (interval.start.1 + interval.end.1) * 0.5;
        receive_at(
            receiver_x,
            receiver_y,
            interval.outward_normal.0,
            interval.outward_normal.1,
            projected_extent,
            MaterialAuraBoundary::Straight {
                start: interval.start,
                end: interval.end,
                normal: interval.outward_normal,
            },
        )?;
    }

    for interval in curved_intervals {
        let projected_extent = crate::source_exposure::point_source_projected_arc_extent(
            interval.arc,
            (source_x, source_y),
        )?;
        let midpoint_angle = interval.arc.start_angle_radians + interval.arc.sweep_radians * 0.5;
        let (normal_y, normal_x) = midpoint_angle.sin_cos();
        let receiver_x = interval.arc.center_x + interval.arc.radius * normal_x;
        let receiver_y = interval.arc.center_y + interval.arc.radius * normal_y;
        receive_at(
            receiver_x,
            receiver_y,
            normal_x,
            normal_y,
            projected_extent,
            MaterialAuraBoundary::Arc(interval.arc),
        )?;
    }

    Some(receptions)
}

/// One tick-local emitter prepared for repeated chemistry-field queries.
/// Source position, mass scaling, and emitted spectrum are shared by every
/// candidate sample during the tick instead of being rebuilt per reaction.
#[derive(Clone, Debug)]
pub(crate) struct MaterialResonanceEmitter {
    source_index: usize,
    source_x: f64,
    source_y: f64,
    mass_scale: f64,
    emitted: ToneSpectrum,
}

/// Prepare the unoccluded material-origin emitters once for a simulation tick.
pub(crate) fn material_resonance_emitters_at_tick(
    materials: &[crate::physical_material::PhysicalMaterial],
    source_indices: &[usize],
    catalog: &[crate::resources::BaseResource],
    tick: u64,
) -> Option<Vec<MaterialResonanceEmitter>> {
    let mut seen_sources = std::collections::BTreeSet::new();
    if source_indices
        .iter()
        .any(|index| *index >= materials.len() || !seen_sources.insert(*index))
    {
        return None;
    }
    let baselines = ResourceBaselines::from_catalog(catalog);
    let phase = phase_at_tick(WORLD_TONE_CYCLES_PER_DAY, tick);
    // Environmental inventories contain many independent objects made from
    // the same material composition. Their emitted spectrum and mass response
    // are composition/tick dependent, not position dependent. Prepare that
    // response once per distinct composition and reuse it across the objects.
    let mut response_cache =
        std::collections::HashMap::<Vec<(String, u64)>, (f64, ToneSpectrum)>::new();
    let mut emitters = Vec::with_capacity(source_indices.len());
    for &source_index in source_indices {
        let source = materials.get(source_index)?;
        if !source.material.is_valid() || source.material.is_empty() {
            return None;
        }
        let placements = source.placements.as_ref()?;
        if placements.len() != source.material.parts.len() || placements.is_empty() {
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
        if !source_x.is_finite() || !source_y.is_finite() {
            return None;
        }

        let signature: Vec<_> = source
            .material
            .parts
            .iter()
            .map(|(name, amount)| (name.clone(), amount.to_bits()))
            .collect();
        let (mass_scale, emitted) = response_cache.entry(signature).or_insert_with(|| {
            let mass_scale =
                environmental_mass_scale(source.material.mass(catalog), baselines.mass);
            let emitted = material_response(
                source.material.weighted_properties(catalog),
                baselines,
                phase,
            );
            (mass_scale, emitted)
        });
        if !mass_scale.is_finite() || *mass_scale < 0.0 {
            return None;
        }
        if *mass_scale > f64::EPSILON {
            emitters.push(MaterialResonanceEmitter {
                source_index,
                source_x,
                source_y,
                mass_scale: *mass_scale,
                emitted: emitted.clone(),
            });
        }
    }
    Some(emitters)
}

/// Evaluate prepared emitters at exact points around one receiver. Chemistry
/// reception remains unoccluded and independent of receiver boundary geometry.
/// Apply the receiver response directly to a scaled prepared emitter.
///
/// The scaled incident spectrum has the same component ordering as the emitted
/// spectrum, so it is unnecessary to allocate an intermediate ToneSpectrum
/// merely to multiply every amplitude before applying the response.
fn material_response_spectrum_from_scaled_emitter(
    properties: ResourceProperties,
    baselines: ResourceBaselines,
    emitted: &ToneSpectrum,
    scale: f64,
) -> Option<ToneSpectrum> {
    if !scale.is_finite() || scale <= f64::EPSILON {
        return None;
    }
    let incoming_energy = spectrum_energy(emitted)? * scale * scale;
    if !incoming_energy.is_finite() {
        return None;
    }
    let mut transformed = ToneSpectrum::empty();
    for component in &emitted.components {
        if component.amplitude <= f64::EPSILON {
            continue;
        }
        let response = material_response_to_drive(
            properties,
            baselines,
            component.frequency_cycles_per_day,
            component.amplitude * scale,
            component.phase_radians,
        );
        transformed.merge_from(&response, 1.0);
    }
    bound_spectrum_energy(&mut transformed, incoming_energy);
    Some(transformed)
}

pub(crate) fn material_resonance_spectra_from_emitters(
    emitters: &[MaterialResonanceEmitter],
    receiver: &crate::physical_material::PhysicalMaterial,
    receiver_index: usize,
    points: &[(f64, f64)],
    catalog: &[crate::resources::BaseResource],
) -> Option<Vec<ToneSpectrum>> {
    if points.is_empty()
        || points
            .iter()
            .any(|point| !point.0.is_finite() || !point.1.is_finite())
        || !receiver.material.is_valid()
        || receiver.material.is_empty()
    {
        return None;
    }
    let baselines = ResourceBaselines::from_catalog(catalog);
    let receiver_properties = receiver.material.weighted_properties(catalog);
    let mut fields = vec![ToneSpectrum::empty(); points.len()];
    for emitter in emitters
        .iter()
        .filter(|emitter| emitter.source_index != receiver_index)
    {
        for (point_index, point) in points.iter().enumerate() {
            let distance = (emitter.source_x - point.0).hypot(emitter.source_y - point.1);
            if !distance.is_finite() {
                return None;
            }
            let attenuation = emitter.mass_scale / (1.0 + distance);
            if !attenuation.is_finite() || attenuation <= f64::EPSILON {
                continue;
            }
            let mut received = material_response_spectrum_from_scaled_emitter(
                receiver_properties,
                baselines,
                &emitter.emitted,
                attenuation,
            )?;
            received.retain_strongest();
            add_spectrum(&mut fields[point_index], &received, 1.0);
        }
    }
    for field in &mut fields {
        field.retain_strongest();
    }
    Some(fields)
}

/// Compatibility entry point for callers that need a one-off field query.
/// Repeated reaction queries should prepare emitters once and reuse them.
pub(crate) fn material_resonance_spectra_at_boundary_points(
    materials: &[crate::physical_material::PhysicalMaterial],
    source_indices: &[usize],
    receiver_index: usize,
    points: &[(f64, f64)],
    catalog: &[crate::resources::BaseResource],
    tick: u64,
) -> Option<Vec<ToneSpectrum>> {
    let receiver = materials.get(receiver_index)?;
    if source_indices.contains(&receiver_index) {
        return None;
    }
    let emitters = material_resonance_emitters_at_tick(materials, source_indices, catalog, tick)?;
    material_resonance_spectra_from_emitters(&emitters, receiver, receiver_index, points, catalog)
}

/// Single-point compatibility wrapper for unoccluded chemistry-scale reception.
#[allow(dead_code)] // Direct query retained for focused geometry tests and callers.
pub(crate) fn material_resonance_spectrum_at_boundary_point(
    materials: &[crate::physical_material::PhysicalMaterial],
    source_indices: &[usize],
    receiver_index: usize,
    point: (f64, f64),
    catalog: &[crate::resources::BaseResource],
    tick: u64,
) -> Option<ToneSpectrum> {
    material_resonance_spectra_at_boundary_points(
        materials,
        source_indices,
        receiver_index,
        &[point],
        catalog,
        tick,
    )?
    .into_iter()
    .next()
}

/// Integrate the interference between an incoming aura and the receiver's
/// own emitted aura over the receiver's exposed boundary samples.
///
/// The incident spectrum is intentionally used here, not the receiver-
/// responded spectrum: the receiver's own emission already contains its
/// oscillator phase lag. Comparing against the responded spectrum would apply
/// that lag a second time and erase the expected same-material phase relation.
#[allow(dead_code)] // Staged for PR 4 movement and energy consumers.
fn directional_resonance_pair_potential_components(
    source: &crate::physical_material::PhysicalMaterial,
    receiver: &crate::physical_material::PhysicalMaterial,
    catalog: &[crate::resources::BaseResource],
    tick: u64,
) -> Option<Vec<ResonancePairPotentialComponent>> {
    directional_resonance_pair_potential_components_with_occluders(
        source,
        receiver,
        &[],
        catalog,
        tick,
    )
}

#[allow(dead_code)] // Staged for PR 4 movement and energy consumers.
fn directional_resonance_pair_potential_components_with_occluders(
    source: &crate::physical_material::PhysicalMaterial,
    receiver: &crate::physical_material::PhysicalMaterial,
    occluders: &[crate::physical_material::PhysicalMaterial],
    catalog: &[crate::resources::BaseResource],
    tick: u64,
) -> Option<Vec<ResonancePairPotentialComponent>> {
    let receptions =
        material_aura_receptions_with_occluders(source, receiver, occluders, catalog, tick)?;
    if receptions.is_empty() {
        return Some(Vec::new());
    }

    let (receiver_x, receiver_y, _, receiver_emission) =
        environmental_material_resonance(receiver, catalog, tick)?;
    let baselines = ResourceBaselines::from_catalog(catalog);
    let receiver_mass_scale =
        environmental_mass_scale(receiver.material.mass(catalog), baselines.mass);
    if !receiver_mass_scale.is_finite() || receiver_mass_scale < 0.0 {
        return None;
    }

    let mut potential = Vec::<ResonancePairPotentialComponent>::new();
    for sample in receptions {
        let local_distance = (sample.receiver_x - receiver_x).hypot(sample.receiver_y - receiver_y);
        if !local_distance.is_finite() {
            return None;
        }
        // Both fields are amplitudes, so the same square-root projected
        // extent is applied to the receiver's own boundary field. The
        // incident spectrum already contains its matching extent factor.
        let local_scale =
            receiver_mass_scale * sample.projected_extent.sqrt() / (1.0 + local_distance);
        if !local_scale.is_finite() {
            return None;
        }
        let mut local_field = ToneSpectrum::empty();
        add_spectrum(&mut local_field, &receiver_emission, local_scale);
        local_field.retain_strongest();
        for contribution in
            spectrum_cross_energy_components(&sample.incident_spectrum, &local_field)?
        {
            if let Some(existing) = potential.iter_mut().find(|component| {
                (component.frequency_cycles_per_day - contribution.frequency_cycles_per_day).abs()
                    <= 1e-9
            }) {
                existing.energy += contribution.energy;
                if !existing.energy.is_finite() {
                    return None;
                }
            } else {
                potential.push(contribution);
            }
        }
    }
    potential.sort_by(|a, b| {
        a.frequency_cycles_per_day
            .partial_cmp(&b.frequency_cycles_per_day)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Some(potential)
}

/// One symmetric resonance-derived pair potential for an isolated pair,
/// retained by frequency so energy settlement can use the same spectral
/// compatibility as the reservoir. If third-party bodies can occlude the pair,
/// use the occluder-aware variant instead.
#[allow(dead_code)] // Staged for PR 4 frequency-resolved energy settlement.
pub(crate) fn resonance_pair_potential_components(
    a: &crate::physical_material::PhysicalMaterial,
    b: &crate::physical_material::PhysicalMaterial,
    catalog: &[crate::resources::BaseResource],
    tick: u64,
) -> Option<Vec<ResonancePairPotentialComponent>> {
    resonance_pair_potential_components_with_occluders(a, b, &[], catalog, tick)
}

/// Frequency-resolved symmetric pair potential with third-party physical
/// bodies blocking the source-to-receiver boundary paths.
#[allow(dead_code)] // Staged for PR 4 frequency-resolved energy settlement.
pub(crate) fn resonance_pair_potential_components_with_occluders(
    a: &crate::physical_material::PhysicalMaterial,
    b: &crate::physical_material::PhysicalMaterial,
    occluders: &[crate::physical_material::PhysicalMaterial],
    catalog: &[crate::resources::BaseResource],
    tick: u64,
) -> Option<Vec<ResonancePairPotentialComponent>> {
    if std::ptr::eq(a, b) {
        return Some(Vec::new());
    }
    let a_to_b = directional_resonance_pair_potential_components_with_occluders(
        a, b, occluders, catalog, tick,
    )?;
    let b_to_a = directional_resonance_pair_potential_components_with_occluders(
        b, a, occluders, catalog, tick,
    )?;
    let mut symmetric = Vec::<ResonancePairPotentialComponent>::new();
    for component in a_to_b.iter().chain(b_to_a.iter()) {
        if let Some(existing) = symmetric.iter_mut().find(|existing| {
            (existing.frequency_cycles_per_day - component.frequency_cycles_per_day).abs() <= 1e-9
        }) {
            existing.energy += 0.5 * component.energy;
            if !existing.energy.is_finite() {
                return None;
            }
        } else {
            symmetric.push(ResonancePairPotentialComponent {
                frequency_cycles_per_day: component.frequency_cycles_per_day,
                energy: 0.5 * component.energy,
            });
        }
    }
    symmetric.sort_by(|a, b| {
        a.frequency_cycles_per_day
            .partial_cmp(&b.frequency_cycles_per_day)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Some(symmetric)
}

/// Scalar view of the isolated-pair potential. PR 4 should use the
/// frequency-resolved form when settling energy into/out of resonance stores.
/// If third-party bodies can occlude the pair, use the occluder-aware variant.
#[allow(dead_code)] // Staged for PR 4 movement and energy consumers.
pub(crate) fn resonance_pair_potential(
    a: &crate::physical_material::PhysicalMaterial,
    b: &crate::physical_material::PhysicalMaterial,
    catalog: &[crate::resources::BaseResource],
    tick: u64,
) -> Option<f64> {
    resonance_pair_potential_with_occluders(a, b, &[], catalog, tick)
}

#[allow(dead_code)] // Staged for PR 4 movement and energy consumers.
pub(crate) fn resonance_pair_potential_with_occluders(
    a: &crate::physical_material::PhysicalMaterial,
    b: &crate::physical_material::PhysicalMaterial,
    occluders: &[crate::physical_material::PhysicalMaterial],
    catalog: &[crate::resources::BaseResource],
    tick: u64,
) -> Option<f64> {
    let components =
        resonance_pair_potential_components_with_occluders(a, b, occluders, catalog, tick)?;
    let potential = components
        .iter()
        .map(|component| component.energy)
        .sum::<f64>();
    potential.is_finite().then_some(potential)
}

#[allow(dead_code)] // Used by the staged numerical-gradient force implementation.
pub(crate) fn rotated_physical_material(
    physical: &crate::physical_material::PhysicalMaterial,
    angle: f64,
    catalog: &[crate::resources::BaseResource],
) -> Option<crate::physical_material::PhysicalMaterial> {
    if !angle.is_finite() {
        return None;
    }
    let properties = physical.mass_properties(catalog, None)?;
    let (sin, cos) = angle.sin_cos();
    let mut rotated = physical.clone();
    let placements = rotated.placements.as_mut()?;
    for placement in placements {
        let dx = placement.x - properties.center_x;
        let dy = placement.y - properties.center_y;
        placement.x = properties.center_x + dx * cos - dy * sin;
        placement.y = properties.center_y + dx * sin + dy * cos;
        placement.rotation_radians += angle;
        if !placement.x.is_finite()
            || !placement.y.is_finite()
            || !placement.rotation_radians.is_finite()
        {
            return None;
        }
    }
    Some(rotated)
}

fn translated_physical_material(
    physical: &crate::physical_material::PhysicalMaterial,
    dx: f64,
    dy: f64,
) -> Option<crate::physical_material::PhysicalMaterial> {
    if !dx.is_finite() || !dy.is_finite() {
        return None;
    }
    let mut translated = physical.clone();
    let placements = translated.placements.as_mut()?;
    for placement in placements {
        placement.x += dx;
        placement.y += dy;
        if !placement.x.is_finite() || !placement.y.is_finite() {
            return None;
        }
    }
    Some(translated)
}

/// Isolated-pair force derived from the spatial gradient of the single
/// symmetric potential. The caller must ensure no third-party body can occlude
/// the pair; use the occluder-aware variant otherwise. The centered difference
/// is a numerical derivative only, not an additional physical force law.
#[allow(dead_code)] // Staged for PR 4 movement and chemistry-force consumers.
pub(crate) fn resonance_pair_force(
    a: &crate::physical_material::PhysicalMaterial,
    b: &crate::physical_material::PhysicalMaterial,
    catalog: &[crate::resources::BaseResource],
    tick: u64,
) -> Option<((f64, f64), (f64, f64))> {
    resonance_pair_force_with_occluders(a, b, &[], catalog, tick)
}

#[allow(dead_code)] // Staged for PR 4 movement and chemistry-force consumers.
pub(crate) fn resonance_pair_force_with_occluders(
    a: &crate::physical_material::PhysicalMaterial,
    b: &crate::physical_material::PhysicalMaterial,
    occluders: &[crate::physical_material::PhysicalMaterial],
    catalog: &[crate::resources::BaseResource],
    tick: u64,
) -> Option<((f64, f64), (f64, f64))> {
    // A blocker makes this a three-body potential: translating only B while
    // assigning the opposite gradient to A would violate the actual gradient
    // with respect to A. Until the blocker also receives its derived force,
    // fail closed rather than claim a false action/reaction pair.
    if !occluders.is_empty() {
        return None;
    }
    const DIFFERENCE_STEP: f64 = 1.0e-4;
    let potential = resonance_pair_potential_with_occluders(a, b, occluders, catalog, tick)?;
    let b_plus_x = translated_physical_material(b, DIFFERENCE_STEP, 0.0)?;
    let b_minus_x = translated_physical_material(b, -DIFFERENCE_STEP, 0.0)?;
    let b_plus_y = translated_physical_material(b, 0.0, DIFFERENCE_STEP)?;
    let b_minus_y = translated_physical_material(b, 0.0, -DIFFERENCE_STEP)?;

    let gradient_x =
        (resonance_pair_potential_with_occluders(a, &b_plus_x, occluders, catalog, tick)?
            - resonance_pair_potential_with_occluders(a, &b_minus_x, occluders, catalog, tick)?)
            / (2.0 * DIFFERENCE_STEP);
    let gradient_y =
        (resonance_pair_potential_with_occluders(a, &b_plus_y, occluders, catalog, tick)?
            - resonance_pair_potential_with_occluders(a, &b_minus_y, occluders, catalog, tick)?)
            / (2.0 * DIFFERENCE_STEP);
    if !gradient_x.is_finite() || !gradient_y.is_finite() || !potential.is_finite() {
        return None;
    }

    // F_b = -grad_b(U); translational invariance gives F_a = -F_b.
    let force_on_b = (-gradient_x, -gradient_y);
    let force_on_a = (gradient_x, gradient_y);
    Some((force_on_a, force_on_b))
}

/// Derive the net force and center-of-mass torque on every body participating
/// in an occluded resonance pair. Translation and rigid rotation are both
/// differentiated from the same occlusion-aware potential, preserving the
/// complete multi-body gradient instead of assigning an unbalanced opposite
/// force to the source/receiver pair alone.
#[allow(dead_code)] // Staged for integration with environmental rigid-body movement.
pub(crate) fn resonance_multi_body_forces_with_occluders(
    a: &crate::physical_material::PhysicalMaterial,
    b: &crate::physical_material::PhysicalMaterial,
    occluders: &[crate::physical_material::PhysicalMaterial],
    catalog: &[crate::resources::BaseResource],
    tick: u64,
) -> Option<Vec<ResonanceBodyForce>> {
    if std::ptr::eq(a, b) {
        return None;
    }
    const DIFFERENCE_STEP: f64 = 1.0e-4;
    let potential = |source: &crate::physical_material::PhysicalMaterial,
                     receiver: &crate::physical_material::PhysicalMaterial,
                     blockers: &[crate::physical_material::PhysicalMaterial]| {
        resonance_pair_potential_with_occluders(source, receiver, blockers, catalog, tick)
    };
    let baseline = potential(a, b, occluders)?;
    if !baseline.is_finite() {
        return None;
    }

    // Reuse the blocker vectors across finite-difference samples. Cloning
    // the complete occluder inventory for every blocker and axis made this
    // otherwise local derivative allocate quadratically in blocker count.
    let mut plus_blockers = occluders.to_vec();
    let mut minus_blockers = occluders.to_vec();
    // Only blockers whose bounds intersect the source-to-receiver envelope
    // (expanded by the finite-difference step) can affect this pair's visibility
    // gradient. Keep every such blocker; distant bodies have an exact zero
    // derivative and do not need four additional potential evaluations.
    let (source_x, source_y, _, _) = environmental_material_resonance(a, catalog, tick)?;
    let receiver_geometry =
        crate::organism_geometry::OrganismBodyGeometry::from_physical_material(b, catalog)?;
    let step_points = [
        (source_x - DIFFERENCE_STEP, source_y - DIFFERENCE_STEP),
        (source_x - DIFFERENCE_STEP, source_y + DIFFERENCE_STEP),
        (source_x + DIFFERENCE_STEP, source_y - DIFFERENCE_STEP),
        (source_x + DIFFERENCE_STEP, source_y + DIFFERENCE_STEP),
        (
            receiver_geometry.min_x - DIFFERENCE_STEP,
            receiver_geometry.min_y - DIFFERENCE_STEP,
        ),
        (
            receiver_geometry.min_x - DIFFERENCE_STEP,
            receiver_geometry.max_y + DIFFERENCE_STEP,
        ),
        (
            receiver_geometry.max_x + DIFFERENCE_STEP,
            receiver_geometry.min_y - DIFFERENCE_STEP,
        ),
        (
            receiver_geometry.max_x + DIFFERENCE_STEP,
            receiver_geometry.max_y + DIFFERENCE_STEP,
        ),
    ];
    let blocker_can_affect_gradient = occluders
        .iter()
        .map(|blocker| {
            let geometry = crate::organism_geometry::OrganismBodyGeometry::from_physical_material(
                blocker, catalog,
            )?;
            Some(geometry_bounds_may_occlude_source_points(
                (source_x, source_y),
                &step_points,
                &geometry,
            ))
        })
        .collect::<Option<Vec<_>>>()?;

    let mut forces = Vec::with_capacity(occluders.len() + 2);
    for participant in 0..occluders.len() + 2 {
        let mut derivatives = [0.0; 3];
        for (axis, derivative) in derivatives.iter_mut().take(2).enumerate() {
            if participant >= 2 && !blocker_can_affect_gradient[participant - 2] {
                *derivative = 0.0;
                continue;
            }
            let (dx, dy) = if axis == 0 {
                (DIFFERENCE_STEP, 0.0)
            } else {
                (0.0, DIFFERENCE_STEP)
            };
            let (plus, minus) = if participant == 0 {
                let shifted_plus = translated_physical_material(a, dx, dy)?;
                let shifted_minus = translated_physical_material(a, -dx, -dy)?;
                (
                    potential(&shifted_plus, b, occluders)?,
                    potential(&shifted_minus, b, occluders)?,
                )
            } else if participant == 1 {
                let shifted_plus = translated_physical_material(b, dx, dy)?;
                let shifted_minus = translated_physical_material(b, -dx, -dy)?;
                (
                    potential(a, &shifted_plus, occluders)?,
                    potential(a, &shifted_minus, occluders)?,
                )
            } else {
                let blocker_index = participant - 2;
                plus_blockers[blocker_index] =
                    translated_physical_material(&occluders[blocker_index], dx, dy)?;
                minus_blockers[blocker_index] =
                    translated_physical_material(&occluders[blocker_index], -dx, -dy)?;
                (
                    potential(a, b, &plus_blockers)?,
                    potential(a, b, &minus_blockers)?,
                )
            };
            *derivative = -(plus - minus) / (2.0 * DIFFERENCE_STEP);
            if !derivative.is_finite() {
                return None;
            }
        }
        let angular_step = DIFFERENCE_STEP;
        if participant >= 2 && !blocker_can_affect_gradient[participant - 2] {
            // A body outside the complete source-to-receiver ray envelope
            // cannot change this pair potential under rigid rotation either.
            // Its exact angular derivative is zero; do not rotate unrelated
            // world bodies merely to rediscover that zero.
            derivatives[2] = 0.0;
        } else {
            let (plus, minus) = if participant == 0 {
                let shifted_plus = rotated_physical_material(a, angular_step, catalog)?;
                let shifted_minus = rotated_physical_material(a, -angular_step, catalog)?;
                (
                    potential(&shifted_plus, b, occluders)?,
                    potential(&shifted_minus, b, occluders)?,
                )
            } else if participant == 1 {
                let shifted_plus = rotated_physical_material(b, angular_step, catalog)?;
                let shifted_minus = rotated_physical_material(b, -angular_step, catalog)?;
                (
                    potential(a, &shifted_plus, occluders)?,
                    potential(a, &shifted_minus, occluders)?,
                )
            } else {
                let blocker_index = participant - 2;
                let shifted_plus =
                    rotated_physical_material(&occluders[blocker_index], angular_step, catalog)?;
                let shifted_minus =
                    rotated_physical_material(&occluders[blocker_index], -angular_step, catalog)?;
                plus_blockers[blocker_index] = shifted_plus;
                minus_blockers[blocker_index] = shifted_minus;
                (
                    potential(a, b, &plus_blockers)?,
                    potential(a, b, &minus_blockers)?,
                )
            };
            derivatives[2] = -(plus - minus) / (2.0 * angular_step);
        }
        if !derivatives[2].is_finite() {
            return None;
        }
        if participant >= 2 {
            let blocker_index = participant - 2;
            plus_blockers[blocker_index] = occluders[blocker_index].clone();
            minus_blockers[blocker_index] = occluders[blocker_index].clone();
        }
        forces.push(ResonanceBodyForce {
            x: derivatives[0],
            y: derivatives[1],
            torque: derivatives[2],
        });
    }
    let total_force = forces
        .iter()
        .fold((0.0, 0.0), |sum, force| (sum.0 + force.x, sum.1 + force.y));
    let scale = forces
        .iter()
        .map(|force| force.x.abs().max(force.y.abs()))
        .fold(1.0_f64, f64::max);
    if !total_force.0.is_finite()
        || !total_force.1.is_finite()
        || total_force.0.abs() > scale * 1e-5
        || total_force.1.abs() > scale * 1e-5
    {
        return None;
    }
    Some(forces)
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

#[cfg(test)]
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
#[cfg(test)]
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

/// Convert a received local spectrum into a reaction activation influence.
///
/// Frequency similarity alone is scale-invariant and would let an arbitrarily
/// weak distant signal activate a reaction as strongly as a nearby one. Keep
/// frequency matching separate from field strength: the received RMS amplitude
/// is measured against the model's unit-amplitude reference drive and saturates
/// at one. This remains a dimensionless signal influence, never usable energy.
pub(crate) fn spectral_activation_influence(received: &ToneSpectrum, target: &ToneSpectrum) -> f64 {
    let frequency_match = spectral_similarity(received, target);
    // The chemistry field sums contributions from many materials, so its
    // magnitude can legitimately exceed one. A smooth bounded response keeps
    // stronger signals more influential without allowing unbounded activation.
    let magnitude = spectrum_magnitude(received);
    let signal_strength = magnitude / (1.0 + magnitude);
    (frequency_match * signal_strength).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaled_emitter_response_matches_the_existing_scaled_spectrum_path() {
        let emitted = ToneSpectrum {
            components: vec![
                ToneComponent {
                    frequency_cycles_per_day: 440.0,
                    amplitude: 0.8,
                    phase_radians: 0.3,
                },
                ToneComponent {
                    frequency_cycles_per_day: 880.0,
                    amplitude: 0.2,
                    phase_radians: 0.6,
                },
            ],
        };
        let scale = 0.37;
        let properties = properties(1.0, 4.0, 1.0);
        let baselines = baselines();

        let mut incident = ToneSpectrum::empty();
        add_spectrum(&mut incident, &emitted, scale);
        incident.retain_strongest();
        let mut expected = material_response_spectrum(properties, baselines, &incident).unwrap();
        expected.retain_strongest();

        let mut actual =
            material_response_spectrum_from_scaled_emitter(properties, baselines, &emitted, scale)
                .unwrap();
        actual.retain_strongest();

        assert_eq!(actual.components.len(), expected.components.len());
        for (actual, expected) in actual.components.iter().zip(&expected.components) {
            assert!(
                (actual.frequency_cycles_per_day - expected.frequency_cycles_per_day).abs()
                    <= 1e-12
            );
            assert!((actual.amplitude - expected.amplitude).abs() <= 1e-12);
            assert!((actual.phase_radians - expected.phase_radians).abs() <= 1e-12);
        }
    }

    #[test]
    fn reaction_activation_influence_preserves_received_signal_strength() {
        let target = ToneSpectrum {
            components: vec![ToneComponent {
                frequency_cycles_per_day: 440.0,
                amplitude: 1.0,
                phase_radians: 0.0,
            }],
        };
        let received = |amplitude| ToneSpectrum {
            components: vec![ToneComponent {
                frequency_cycles_per_day: 440.0,
                amplitude,
                phase_radians: 0.0,
            }],
        };

        let unit = spectral_activation_influence(&received(1.0), &target);
        let weak = spectral_activation_influence(&received(0.1), &target);
        let distant = spectral_activation_influence(&received(0.01), &target);
        let strong = spectral_activation_influence(&received(5.0), &target);

        assert!((unit - 0.5).abs() < 1e-12);
        assert!((weak - (0.1_f64 / 1.1)).abs() < 1e-12);
        assert!((distant - (0.01_f64 / 1.01)).abs() < 1e-12);
        assert!((strong - (5.0_f64 / 6.0)).abs() < 1e-12);
        assert!(strong > unit && unit > weak && weak > distant);
    }

    #[test]
    fn resonance_occluder_broad_phase_keeps_endpoint_path_bounds_and_discards_disjoint_bounds() {
        let endpoint_points = [(4.0, 0.0), (4.0, 1.0)];
        let crossing = crate::organism_geometry::OrganismBodyGeometry {
            parts: Vec::new(),
            min_x: 2.0,
            max_x: 3.0,
            min_y: 0.25,
            max_y: 0.75,
        };
        let outside_y = crate::organism_geometry::OrganismBodyGeometry {
            parts: Vec::new(),
            min_x: 1.0,
            max_x: 3.0,
            min_y: 2.0,
            max_y: 3.0,
        };
        let outside_x = crate::organism_geometry::OrganismBodyGeometry {
            parts: Vec::new(),
            min_x: 5.0,
            max_x: 6.0,
            min_y: 0.25,
            max_y: 0.75,
        };

        assert!(geometry_bounds_may_occlude_source_points(
            (0.0, 0.0),
            &endpoint_points,
            &crossing,
        ));
        assert!(!geometry_bounds_may_occlude_source_points(
            (0.0, 0.0),
            &endpoint_points,
            &outside_y,
        ));
        assert!(!geometry_bounds_may_occlude_source_points(
            (0.0, 0.0),
            &endpoint_points,
            &outside_x,
        ));
    }

    #[test]
    fn resonance_visibility_runs_only_for_boundary_types_present_on_receiver() {
        let polygon = crate::organism_geometry::OrganismBodyGeometry {
            parts: vec![crate::organism_geometry::PlacedForm {
                unit_index: 0,
                form: crate::resources::Form::Rectangle {
                    width: 1.0,
                    height: 1.0,
                },
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            min_x: -0.5,
            max_x: 0.5,
            min_y: -0.5,
            max_y: 0.5,
        };
        let circle = crate::organism_geometry::OrganismBodyGeometry {
            parts: vec![crate::organism_geometry::PlacedForm {
                unit_index: 0,
                form: crate::resources::Form::Circle { radius: 0.5 },
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            min_x: -0.5,
            max_x: 0.5,
            min_y: -0.5,
            max_y: 0.5,
        };
        let line = crate::organism_geometry::OrganismBodyGeometry {
            parts: vec![crate::organism_geometry::PlacedForm {
                unit_index: 0,
                form: crate::resources::Form::Line { length: 1.0 },
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            min_x: -0.5,
            max_x: 0.5,
            min_y: 0.0,
            max_y: 0.0,
        };

        assert!(receiver_has_straight_boundary(&polygon));
        assert!(!receiver_has_curved_boundary(&polygon));
        assert!(!receiver_has_straight_boundary(&circle));
        assert!(receiver_has_curved_boundary(&circle));
        assert!(!receiver_has_straight_boundary(&line));
        assert!(!receiver_has_curved_boundary(&line));
    }

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
    fn spectrum_cross_energy_tracks_phase_and_ignores_distinct_frequencies() {
        let make = |phase_radians: f64, frequency_cycles_per_day: f64| ToneSpectrum {
            components: vec![ToneComponent {
                frequency_cycles_per_day,
                amplitude: 0.5,
                phase_radians,
            }],
        };
        let in_phase = spectrum_cross_energy(&make(0.0, 440.0), &make(0.0, 440.0))
            .expect("valid in-phase spectra should be accepted");
        let opposite = spectrum_cross_energy(&make(0.0, 440.0), &make(std::f64::consts::PI, 440.0))
            .expect("valid opposite-phase spectra should be accepted");
        let quadrature =
            spectrum_cross_energy(&make(0.0, 440.0), &make(std::f64::consts::FRAC_PI_2, 440.0))
                .expect("valid quadrature spectra should be accepted");
        let different_frequency = spectrum_cross_energy(&make(0.0, 440.0), &make(0.0, 441.0))
            .expect("valid distinct-frequency spectra should be accepted");

        assert!((in_phase - 0.5).abs() < 1e-12);
        assert!((opposite + 0.5).abs() < 1e-12);
        assert!(quadrature.abs() < 1e-12);
        assert!(different_frequency.abs() < 1e-12);
    }

    #[test]
    fn spectrum_cross_energy_is_symmetric_and_fails_closed_for_invalid_input() {
        let a = ToneSpectrum {
            components: vec![ToneComponent {
                frequency_cycles_per_day: 440.0,
                amplitude: 0.7,
                phase_radians: 0.3,
            }],
        };
        let b = ToneSpectrum {
            components: vec![ToneComponent {
                frequency_cycles_per_day: 440.0,
                amplitude: 0.2,
                phase_radians: -0.4,
            }],
        };
        let ab = spectrum_cross_energy(&a, &b).expect("valid spectra should be accepted");
        let ba = spectrum_cross_energy(&b, &a).expect("valid spectra should be accepted");
        assert!((ab - ba).abs() < 1e-12);

        let invalid = ToneSpectrum {
            components: vec![ToneComponent {
                frequency_cycles_per_day: 440.0,
                amplitude: f64::NAN,
                phase_radians: 0.0,
            }],
        };
        assert!(spectrum_cross_energy(&a, &invalid).is_none());
    }

    #[test]
    fn material_aura_reception_uses_realized_geometry_and_is_reciprocal() {
        let catalog = crate::resources::default_catalog();
        let make_material = |x| {
            crate::physical_material::PhysicalMaterial::realized(
                crate::resources::Material::free_base("Carbon", 1.0),
                vec![crate::structure::Placement {
                    x,
                    y: 0.0,
                    rotation_radians: 0.0,
                }],
                &catalog,
            )
            .expect("carbon should have a valid realized physical shape")
        };
        let left = make_material(0.0);
        let right = make_material(4.0);
        let farther = make_material(8.0);

        let left_to_right = material_aura_receptions(&left, &right, &catalog, 0)
            .expect("valid pair should produce aura reception");
        let right_to_left = material_aura_receptions(&right, &left, &catalog, 0)
            .expect("reverse pair should produce aura reception");
        let left_to_farther = material_aura_receptions(&left, &farther, &catalog, 0)
            .expect("farther pair should produce attenuated aura reception");

        assert!(!left_to_right.is_empty());
        assert!(!right_to_left.is_empty());
        assert!(!left_to_farther.is_empty());
        assert!(left_to_right.iter().all(|sample| {
            sample.receiver_x.is_finite()
                && sample.receiver_y.is_finite()
                && (sample.receiver_normal_x.powi(2) + sample.receiver_normal_y.powi(2) - 1.0).abs()
                    < 1e-6
        }));
        assert!(right_to_left.iter().all(|sample| {
            sample.receiver_x.is_finite()
                && sample.receiver_y.is_finite()
                && (sample.receiver_normal_x.powi(2) + sample.receiver_normal_y.powi(2) - 1.0).abs()
                    < 1e-6
        }));

        let total_received_energy = |samples: &[MaterialAuraReception]| {
            samples
                .iter()
                .map(|sample| {
                    spectrum_energy(&sample.spectrum).expect("received spectrum is valid")
                })
                .sum::<f64>()
        };
        let near_energy = total_received_energy(&left_to_right);
        let reverse_energy = total_received_energy(&right_to_left);
        let far_energy = total_received_energy(&left_to_farther);
        assert!(near_energy > far_energy);
        assert!(
            (near_energy - reverse_energy).abs() < 1e-9,
            "identical materials at equal separation should receive reciprocal energy"
        );
        assert!(left_to_right
            .iter()
            .all(|sample| sample.projected_extent > 0.0));
        assert!(left_to_right.iter().all(|sample| {
            sample
                .spectrum
                .components
                .iter()
                .all(|component| component.phase_radians.is_finite())
        }));
        assert!(material_aura_receptions(&left, &left, &catalog, 0)
            .expect("self reception is a valid empty result")
            .is_empty());
    }

    #[test]
    fn continuous_boundary_geometry_resolves_straight_and_curved_normals() {
        let straight = MaterialAuraBoundary::Straight {
            start: (0.0, 0.0),
            end: (2.0, 0.0),
            normal: (0.0, 1.0),
        };
        assert_eq!(straight.normal_at((1.0, 0.0)), Some((0.0, 1.0)));
        assert_eq!(straight.normal_at((1.0, 0.1)), None);

        let arc = MaterialAuraBoundary::Arc(crate::capillary_geometry::CircularArcBoundary {
            center_x: 0.0,
            center_y: 0.0,
            radius: 2.0,
            start_angle_radians: 0.0,
            sweep_radians: std::f64::consts::FRAC_PI_2,
        });
        let diagonal = 2.0_f64.sqrt();
        let normal = arc
            .normal_at((diagonal, diagonal))
            .expect("point on the exposed arc should have a normal");
        assert!((normal.0 - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-12);
        assert!((normal.1 - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-12);
        assert_eq!(arc.normal_at((0.0, 2.1)), None);
        assert_eq!(arc.normal_at((-2.0, 0.0)), None);
    }

    #[test]
    fn chemistry_scale_resonance_query_is_grid_free_and_unoccluded() {
        let catalog = crate::resources::default_catalog();
        let make_material = |name: &str, x, y| {
            crate::physical_material::PhysicalMaterial::realized(
                crate::resources::Material::free_base(name, 1.0),
                vec![crate::structure::Placement {
                    x,
                    y,
                    rotation_radians: 0.0,
                }],
                &catalog,
            )
            .expect("material should have a valid realized physical shape")
        };
        let source = make_material("Carbon", 0.0, 0.0);
        let receiver = make_material("Carbon", 8.0, 0.0);
        let materials = vec![source, receiver];
        let receptions = material_aura_receptions_with_occluders(
            &materials[0],
            &materials[1],
            &materials,
            &catalog,
            0,
        )
        .expect("valid physical arrangement should produce boundary receptions");
        let reception = receptions
            .iter()
            .find(|sample| {
                sample
                    .boundary
                    .normal_at((sample.receiver_x, sample.receiver_y))
                    .is_some_and(|normal| normal.0 < -0.5)
            })
            .expect("an exposed boundary facing the source should exist");
        let point = (reception.receiver_x, reception.receiver_y);

        let first =
            material_resonance_spectrum_at_boundary_point(&materials, &[0], 1, point, &catalog, 0)
                .expect("a finite reaction endpoint should be queryable");
        let repeated =
            material_resonance_spectrum_at_boundary_point(&materials, &[0], 1, point, &catalog, 0)
                .expect("the continuous query should be deterministic");
        assert_eq!(first, repeated);
        let paired = material_resonance_spectra_at_boundary_points(
            &materials,
            &[0],
            1,
            &[point, point],
            &catalog,
            0,
        )
        .expect("paired endpoint sampling should accept finite reaction points");
        assert_eq!(paired, vec![first.clone(), first.clone()]);
        assert!(
            !first.components.is_empty(),
            "the local resonance field should reach the reaction endpoint"
        );

        let off_boundary = (point.0 + 0.25, point.1 + 0.25);
        let outside = material_resonance_spectrum_at_boundary_point(
            &materials,
            &[0],
            1,
            off_boundary,
            &catalog,
            0,
        )
        .expect("any finite chemistry endpoint should be queryable");
        assert!(
            !outside.components.is_empty(),
            "chemistry-scale resonance is not restricted to a material boundary"
        );

        let blocked_materials = vec![
            materials[0].clone(),
            materials[1].clone(),
            make_material("Nitrogen", 4.0, 0.0),
        ];
        let unoccluded = material_resonance_spectrum_at_boundary_point(
            &blocked_materials,
            &[0],
            1,
            point,
            &catalog,
            0,
        )
        .expect("third-party geometry must not be needed for chemistry-scale reception");
        assert_eq!(
            unoccluded, first,
            "small chemistry materials do not cast resonance shadows"
        );
    }

    #[test]
    fn resonance_density_heatmap_matches_continuous_midpoint_sample() {
        let catalog = crate::resources::default_catalog();
        let make_material = |x| {
            crate::physical_material::PhysicalMaterial::realized(
                crate::resources::Material::free_base("Carbon", 1.0),
                vec![crate::structure::Placement {
                    x,
                    y: 0.0,
                    rotation_radians: 0.0,
                }],
                &catalog,
            )
            .expect("material should have a valid realized physical shape")
        };
        let materials = vec![make_material(0.0), make_material(8.0)];
        let receptions = material_aura_receptions_with_occluders(
            &materials[0],
            &materials[1],
            &materials,
            &catalog,
            0,
        )
        .expect("valid physical arrangement should produce boundary receptions");
        let reception = receptions
            .iter()
            .find(|sample| sample.receiver_normal_x < -0.5)
            .expect("the receiver should have a visible boundary facing the source");
        let point = (reception.receiver_x, reception.receiver_y);
        let continuous =
            material_resonance_spectrum_at_boundary_point(&materials, &[0], 1, point, &catalog, 0)
                .expect("the exact midpoint should be a valid continuous sample");
        let heatmap =
            material_resonance_density_on_receiver(&materials, &[0], 1, &catalog, 0, 0.01)
                .expect("valid receiver geometry should produce heat-map samples");
        let cell_x = (point.0 / 0.01).floor() as i64;
        let cell_y = (point.1 / 0.01).floor() as i64;
        let cell = heatmap
            .iter()
            .find(|sample| sample.cell_x == cell_x && sample.cell_y == cell_y)
            .expect("the selected boundary midpoint should have a heat-map cell");
        assert_eq!(cell.components.len(), continuous.components.len());
        for expected in &continuous.components {
            let actual = cell
                .components
                .iter()
                .find(|component| {
                    (component.frequency_cycles_per_day - expected.frequency_cycles_per_day).abs()
                        <= 1e-9
                })
                .expect("heat-map and continuous samples should preserve each frequency");
            assert!(
                (actual.amplitude - expected.amplitude).abs() <= 1e-10,
                "heat-map amplitude should match the continuous midpoint sample"
            );
            assert!(
                (actual.phase_radians - expected.phase_radians).abs() <= 1e-10,
                "heat-map phase should match the continuous midpoint sample"
            );
        }
    }

    #[test]
    fn resonance_density_phasors_reinforce_cancel_and_keep_frequencies_separate() {
        let component = |frequency_cycles_per_day, amplitude, phase_radians| ToneComponent {
            frequency_cycles_per_day,
            amplitude,
            phase_radians,
        };
        let reinforced = combine_component(component(440.0, 0.5, 0.0), component(440.0, 0.5, 0.0));
        assert!((reinforced.amplitude - 1.0).abs() < 1e-12);

        let cancelled = combine_component(
            component(440.0, 0.5, 0.0),
            component(440.0, 0.5, std::f64::consts::PI),
        );
        assert!(cancelled.amplitude < 1e-12);

        let mut spectrum = ToneSpectrum::empty();
        add_spectrum(
            &mut spectrum,
            &ToneSpectrum {
                components: vec![component(440.0, 0.5, 0.0)],
            },
            1.0,
        );
        add_spectrum(
            &mut spectrum,
            &ToneSpectrum {
                components: vec![component(880.0, 0.5, 0.0)],
            },
            1.0,
        );
        assert_eq!(spectrum.components.len(), 2);
        assert!((spectrum.components[0].frequency_cycles_per_day - 440.0).abs() < 1e-9);
        assert!((spectrum.components[1].frequency_cycles_per_day - 880.0).abs() < 1e-9);
    }

    #[test]
    fn local_resonance_density_ignores_occlusion_but_respects_source_distance() {
        let catalog = crate::resources::default_catalog();
        let make_material = |name: &str, x, y| {
            crate::physical_material::PhysicalMaterial::realized(
                crate::resources::Material::free_base(name, 1.0),
                vec![crate::structure::Placement {
                    x,
                    y,
                    rotation_radians: 0.0,
                }],
                &catalog,
            )
            .expect("material should have a valid realized physical shape")
        };
        let receiver = make_material("Carbon", 8.0, 0.0);
        let nearby_source = make_material("Carbon", 0.0, 0.0);
        let distant_source = make_material("Carbon", -12.0, 0.0);
        let blocker = make_material("Nitrogen", 4.0, 0.0);

        let density_sum = |materials: &[crate::physical_material::PhysicalMaterial]| {
            material_resonance_density_on_receiver(materials, &[0], 1, &catalog, 0, 1.0)
                .expect("valid physical arrangement should be sampled")
                .iter()
                .map(|sample| sample.density)
                .sum::<f64>()
        };

        let clear = density_sum(&[nearby_source.clone(), receiver.clone()]);
        let intervened = density_sum(&[nearby_source.clone(), receiver.clone(), blocker]);
        let farther = density_sum(&[distant_source, receiver]);
        assert!(clear > 0.0);
        assert!(
            (intervened - clear).abs() <= 1e-9 * clear.max(1.0),
            "third-party materials must not occlude chemistry-scale resonance"
        );
        assert!(
            farther < clear,
            "moving the source farther away should reduce received local density"
        );
    }

    #[test]
    fn local_resonance_density_is_frequency_resolved_and_spatially_attributed() {
        let catalog = crate::resources::default_catalog();
        let make_material = |x| {
            crate::physical_material::PhysicalMaterial::realized(
                crate::resources::Material::free_base("Carbon", 1.0),
                vec![crate::structure::Placement {
                    x,
                    y: 0.0,
                    rotation_radians: 0.0,
                }],
                &catalog,
            )
            .expect("carbon should have a valid realized physical shape")
        };
        let materials = vec![make_material(0.0), make_material(4.0), make_material(8.0)];
        let density =
            material_resonance_density_on_receiver(&materials, &[0, 2], 1, &catalog, 0, 1.0)
                .expect("valid realized materials should produce a local density sample");

        assert!(!density.is_empty());
        assert!(density.iter().all(|sample| {
            sample.x.is_finite()
                && sample.y.is_finite()
                && sample.density.is_finite()
                && sample.density > 0.0
                && !sample.components.is_empty()
                && sample.components.iter().all(|component| {
                    component.frequency_cycles_per_day.is_finite()
                        && component.frequency_cycles_per_day > 0.0
                        && component.amplitude.is_finite()
                        && component.phase_radians.is_finite()
                })
        }));
        assert!(density
            .windows(2)
            .all(|pair| { (pair[0].cell_y, pair[0].cell_x) <= (pair[1].cell_y, pair[1].cell_x) }));
    }

    #[test]
    fn local_resonance_density_rejects_invalid_grid_and_unresolved_materials() {
        let catalog = crate::resources::default_catalog();
        let material = crate::physical_material::PhysicalMaterial::realized(
            crate::resources::Material::free_base("Carbon", 1.0),
            vec![crate::structure::Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .expect("carbon should have a valid realized physical shape");
        assert!(material_resonance_density_on_receiver(
            std::slice::from_ref(&material),
            &[],
            0,
            &catalog,
            0,
            0.0
        )
        .is_none());
        assert!(
            material_resonance_density_on_receiver(&[material], &[], 1, &catalog, 0, 1.0).is_none()
        );
    }

    #[test]
    fn circular_water_boundary_contributes_to_pair_reception() {
        let catalog = crate::resources::default_catalog();
        let source = crate::physical_material::PhysicalMaterial::realized(
            crate::resources::Material::free_base("Carbon", 1.0),
            vec![crate::structure::Placement {
                x: -4.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .expect("carbon should have a valid realized physical shape");
        let receiver = crate::physical_material::PhysicalMaterial::realized(
            crate::resources::Material::free_base("Water", 1.0),
            vec![crate::structure::Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .expect("water should have a valid realized circular physical shape");

        let receptions = material_aura_receptions(&source, &receiver, &catalog, 0)
            .expect("circular receiver should support aura reception");
        assert!(
            !receptions.is_empty(),
            "the exposed circular boundary should produce curved reception samples"
        );
        assert!(receptions.iter().all(|sample| {
            !sample.incident_spectrum.components.is_empty()
                && sample.spectrum.components.is_empty()
                && sample.magnitude == 0.0
        }), "zero-cohesion Water should retain geometric incident samples without inventing a material response");
        assert!(receptions.iter().all(|sample| {
            sample.receiver_x.is_finite()
                && sample.receiver_y.is_finite()
                && sample.projected_extent.is_finite()
                && sample.projected_extent > 0.0
        }));
        let potential = resonance_pair_potential(&source, &receiver, &catalog, 0)
            .expect("circular pair should have a finite interaction potential");
        assert!(potential.is_finite());
    }

    #[test]
    fn pair_potential_remains_finite_at_contact_and_shallow_overlap() {
        let catalog = crate::resources::default_catalog();
        let make_carbon = |x| {
            crate::physical_material::PhysicalMaterial::realized(
                crate::resources::Material::free_base("Carbon", 1.0),
                vec![crate::structure::Placement {
                    x,
                    y: 0.0,
                    rotation_radians: 0.0,
                }],
                &catalog,
            )
            .expect("carbon should have a valid realized physical shape")
        };
        let source = make_carbon(0.0);

        for separation in [2.0, 1.9] {
            let receiver = make_carbon(separation);
            let potential = resonance_pair_potential(&source, &receiver, &catalog, 0)
                .expect("contact and shallow overlap should remain a valid pair geometry");
            assert!(
                potential.is_finite(),
                "pair potential must remain finite at separation {separation}"
            );
        }
    }

    #[test]
    fn pair_reception_rejects_unresolved_owner_relative_materials() {
        let catalog = crate::resources::default_catalog();
        let make_material = |x| {
            crate::physical_material::PhysicalMaterial::realized(
                crate::resources::Material::free_base("Carbon", 1.0),
                vec![crate::structure::Placement {
                    x,
                    y: 0.0,
                    rotation_radians: 0.0,
                }],
                &catalog,
            )
            .expect("carbon should have a valid realized physical shape")
        };
        let source = make_material(0.0);
        let mut stored_receiver = make_material(4.0);
        stored_receiver.owner_relative_origin = Some(crate::structure::Placement {
            x: 100.0,
            y: 50.0,
            rotation_radians: 0.0,
        });

        assert!(material_aura_receptions(&source, &stored_receiver, &catalog, 0).is_none());
    }

    #[test]
    fn resonance_pair_potential_is_symmetric_and_derives_balanced_force() {
        let catalog = crate::resources::default_catalog();
        let make_material = |x, y| {
            crate::physical_material::PhysicalMaterial::realized(
                crate::resources::Material::free_base("Carbon", 1.0),
                vec![crate::structure::Placement {
                    x,
                    y,
                    rotation_radians: 0.0,
                }],
                &catalog,
            )
            .expect("carbon should have a valid realized physical shape")
        };
        let left = make_material(0.0, 0.0);
        let near = make_material(4.0, 0.0);
        let far = make_material(8.0, 0.0);

        let near_ab = resonance_pair_potential(&left, &near, &catalog, 0)
            .expect("valid near pair should have a potential");
        let near_ba = resonance_pair_potential(&near, &left, &catalog, 0)
            .expect("reversed near pair should have a potential");
        let far_energy = resonance_pair_potential(&left, &far, &catalog, 0)
            .expect("valid far pair should have a potential");
        assert!(
            near_ab > 0.0,
            "in-phase material fields should add positive cross-energy"
        );
        assert!((near_ab - near_ba).abs() < 1e-10);
        assert!(
            near_ab > far_energy,
            "the positive pair potential should decay with separation"
        );

        let shifted_left = translated_physical_material(&left, -13.0, 7.0)
            .expect("left material should translate");
        let shifted_near = translated_physical_material(&near, -13.0, 7.0)
            .expect("right material should translate");
        let translated_energy = resonance_pair_potential(&shifted_left, &shifted_near, &catalog, 0)
            .expect("translated pair should have a potential");
        assert!((near_ab - translated_energy).abs() < 1e-9);

        let (force_on_left, force_on_near) = resonance_pair_force(&left, &near, &catalog, 0)
            .expect("valid pair should have a gradient-derived force");
        assert!(
            force_on_near.0 > 0.0,
            "positive decaying potential should repel"
        );
        assert!(
            force_on_near.1.abs() < 1e-6,
            "the symmetric horizontal pair has no vertical force"
        );
        assert!((force_on_left.0 + force_on_near.0).abs() < 1e-12);
        assert!((force_on_left.1 + force_on_near.1).abs() < 1e-12);
    }

    #[test]
    fn stationary_pair_potential_is_invariant_as_shared_phase_clock_advances() {
        let catalog = crate::resources::default_catalog();
        let make_material = |x| {
            crate::physical_material::PhysicalMaterial::realized(
                crate::resources::Material::free_base("Carbon", 1.0),
                vec![crate::structure::Placement {
                    x,
                    y: 0.0,
                    rotation_radians: 0.0,
                }],
                &catalog,
            )
            .expect("carbon should have a valid realized physical shape")
        };
        let left = make_material(0.0);
        let right = make_material(4.0);

        let initial = resonance_pair_potential_components(&left, &right, &catalog, 0)
            .expect("initial pair should have frequency-resolved potential");
        let advanced = resonance_pair_potential_components(&left, &right, &catalog, 137)
            .expect("advanced pair should have frequency-resolved potential");

        assert_eq!(initial.len(), advanced.len());
        for (before, after) in initial.iter().zip(&advanced) {
            assert!(
                (before.frequency_cycles_per_day - after.frequency_cycles_per_day).abs() < 1e-10
            );
            assert!(
                (before.energy - after.energy).abs() < 1e-9,
                "shared phase advance must not change a stationary pair's potential"
            );
        }
    }

    #[test]
    fn distant_non_occluding_body_does_not_change_pair_potential() {
        let catalog = crate::resources::default_catalog();
        let make_material = |name: &str, x, y| {
            crate::physical_material::PhysicalMaterial::realized(
                crate::resources::Material::free_base(name, 1.0),
                vec![crate::structure::Placement {
                    x,
                    y,
                    rotation_radians: 0.0,
                }],
                &catalog,
            )
            .expect("material should have a valid realized physical shape")
        };
        let source = make_material("Carbon", 0.0, 0.0);
        let receiver = make_material("Carbon", 8.0, 0.0);
        let distant = make_material("Nitrogen", 50.0, 50.0);
        let baseline = resonance_pair_potential_components(&source, &receiver, &catalog, 0)
            .expect("unoccluded pair potential should be valid");
        let with_distant = resonance_pair_potential_components_with_occluders(
            &source,
            &receiver,
            std::slice::from_ref(&distant),
            &catalog,
            0,
        )
        .expect("distant body should be excluded by the conservative occluder broad phase");
        assert_eq!(with_distant, baseline);
    }

    #[test]
    fn occluded_pair_does_not_claim_an_unbalanced_two_body_force() {
        let catalog = crate::resources::default_catalog();
        let make_material = |name: &str, x| {
            crate::physical_material::PhysicalMaterial::realized(
                crate::resources::Material::free_base(name, 1.0),
                vec![crate::structure::Placement {
                    x,
                    y: 0.0,
                    rotation_radians: 0.0,
                }],
                &catalog,
            )
            .expect("material should have a valid realized physical shape")
        };
        let source = make_material("Carbon", 0.0);
        let receiver = make_material("Carbon", 8.0);
        let blocker = make_material("Nitrogen", 4.0);

        assert!(resonance_pair_force_with_occluders(
            &source,
            &receiver,
            std::slice::from_ref(&blocker),
            &catalog,
            0,
        )
        .is_none());
    }

    #[test]
    fn occluded_resonance_gradient_assigns_force_to_blocker_and_balances_all_bodies() {
        let catalog = crate::resources::default_catalog();
        let make_material = |name: &str, x, y| {
            crate::physical_material::PhysicalMaterial::realized(
                crate::resources::Material::free_base(name, 1.0),
                vec![crate::structure::Placement {
                    x,
                    y,
                    rotation_radians: 0.0,
                }],
                &catalog,
            )
            .expect("material should have a valid realized physical shape")
        };
        let source = make_material("Carbon", 0.0, 0.0);
        let receiver = make_material("Carbon", 8.0, 0.0);
        let blocker = make_material("Nitrogen", 4.0, 0.35);
        let second_blocker = make_material("Hydrogen", 4.5, -0.4);
        let distant_blocker = make_material("Sulfur", 50.0, 50.0);
        let blockers = vec![blocker, second_blocker, distant_blocker];
        let forces =
            resonance_multi_body_forces_with_occluders(&source, &receiver, &blockers, &catalog, 0)
                .expect("the full multi-body gradient should be finite and balanced");
        assert_eq!(forces.len(), 5);
        assert!(
            forces[2..]
                .iter()
                .any(|force| force.x.abs() + force.y.abs() > 1e-8),
            "occluding bodies must receive their own derived forces"
        );
        assert!(
            forces
                .iter()
                .all(|force| force.x.is_finite() && force.y.is_finite()),
            "reused blocker snapshots must not leak one participant's perturbation into another"
        );
        assert_eq!(
            forces[4],
            ResonanceBodyForce::default(),
            "a body outside the expanded ray envelope has an exact zero gradient"
        );
        let sum_x = forces.iter().map(|force| force.x).sum::<f64>();
        let sum_y = forces.iter().map(|force| force.y).sum::<f64>();
        assert!(sum_x.abs() < 1e-5);
        assert!(sum_y.abs() < 1e-5);

        let bodies = [&source, &receiver, &blockers[0], &blockers[1], &blockers[2]];
        let total_angular_momentum_derivative = bodies
            .iter()
            .zip(&forces)
            .map(|(body, force)| {
                let center = body.mass_properties(&catalog, None).unwrap();
                center.center_x * force.y - center.center_y * force.x + force.torque
            })
            .sum::<f64>();
        assert!(
            total_angular_momentum_derivative.abs() < 1e-4,
            "the complete rotational and translational gradient must conserve angular momentum"
        );
        assert!(forces.iter().all(|force| force.torque.is_finite()));
    }

    #[test]
    fn asymmetric_occluder_receives_resonance_torque() {
        let catalog = crate::resources::default_catalog();
        let make_material = |name: &str, x, y| {
            crate::physical_material::PhysicalMaterial::realized(
                crate::resources::Material::free_base(name, 1.0),
                vec![crate::structure::Placement {
                    x,
                    y,
                    rotation_radians: 0.0,
                }],
                &catalog,
            )
            .expect("material should realize")
        };
        let source = make_material("Carbon", 0.0, 0.0);
        let receiver = make_material("Carbon", 8.0, 0.0);
        let asymmetric_occluder = crate::physical_material::PhysicalMaterial::realized(
            crate::resources::Material {
                parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
                internal_bonds: Vec::new(),
            },
            vec![
                crate::structure::Placement {
                    x: 3.8,
                    y: -0.45,
                    rotation_radians: 0.0,
                },
                crate::structure::Placement {
                    x: 4.7,
                    y: 0.95,
                    rotation_radians: 0.0,
                },
            ],
            &catalog,
        )
        .expect("asymmetric occluder should realize");
        let forces = resonance_multi_body_forces_with_occluders(
            &source,
            &receiver,
            std::slice::from_ref(&asymmetric_occluder),
            &catalog,
            0,
        )
        .expect("asymmetric multi-body gradient should be valid");
        assert!(
            forces[2].torque.abs() > 1e-8,
            "rotating an asymmetric occluder changes its visibility contribution and must produce torque"
        );
    }

    #[test]
    fn different_material_phase_response_can_produce_attraction() {
        let catalog = crate::resources::default_catalog();
        let carbon = crate::physical_material::PhysicalMaterial::realized(
            crate::resources::Material::free_base("Carbon", 1.0),
            vec![crate::structure::Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .expect("carbon should have a valid realized physical shape");
        let hydrogen = crate::physical_material::PhysicalMaterial::realized(
            crate::resources::Material::free_base("Hydrogen", 1.0),
            vec![crate::structure::Placement {
                x: 4.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .expect("hydrogen should have a valid realized physical shape");

        let potential = resonance_pair_potential(&carbon, &hydrogen, &catalog, 0)
            .expect("valid unlike-material pair should have a potential");
        assert!(
            potential < 0.0,
            "the unlike oscillator phases should produce a negative cross-energy"
        );
        let (_, force_on_hydrogen) = resonance_pair_force(&carbon, &hydrogen, &catalog, 0)
            .expect("valid unlike-material pair should have a gradient-derived force");
        assert!(
            force_on_hydrogen.0 < 0.0,
            "a negative decaying potential should attract the receiver toward the source"
        );
    }

    #[test]
    fn quadrature_material_fields_produce_a_neutral_pair_potential() {
        let mut catalog = crate::resources::default_catalog();
        for resource in &mut catalog {
            resource.properties.mass = 1.0;
            resource.properties.cohesion = 1.0;
            resource.properties.reactivity = 1.0;
        }
        let carbon = catalog
            .iter_mut()
            .find(|resource| resource.name == "Carbon")
            .expect("default catalog contains Carbon");
        carbon.properties.cohesion = 2.0;
        carbon.properties.reactivity = 0.0;
        let hydrogen = catalog
            .iter_mut()
            .find(|resource| resource.name == "Hydrogen")
            .expect("default catalog contains Hydrogen");
        hydrogen.properties.cohesion = 7.0 / 6.0;

        let source = crate::physical_material::PhysicalMaterial::realized(
            crate::resources::Material::free_base("Carbon", 1.0),
            vec![crate::structure::Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .expect("carbon should have a valid realized physical shape");
        let receiver = crate::physical_material::PhysicalMaterial::realized(
            crate::resources::Material::free_base("Hydrogen", 1.0),
            vec![crate::structure::Placement {
                x: 4.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .expect("hydrogen should have a valid realized physical shape");

        let potential = resonance_pair_potential(&source, &receiver, &catalog, 0)
            .expect("valid quadrature pair should have a potential");
        assert!(
            potential.abs() < 1e-8,
            "quadrature fields should contribute no cycle-averaged cross-energy"
        );
        let (force_on_source, force_on_receiver) =
            resonance_pair_force(&source, &receiver, &catalog, 0)
                .expect("valid quadrature pair should have a force");
        assert!(force_on_source.0.abs() < 1e-6);
        assert!(force_on_source.1.abs() < 1e-6);
        assert!(force_on_receiver.0.abs() < 1e-6);
        assert!(force_on_receiver.1.abs() < 1e-6);
    }

    #[test]
    fn third_party_material_occludes_resonance_pair_potential() {
        let catalog = crate::resources::default_catalog();
        let make_material = |name: &str, x, y| {
            crate::physical_material::PhysicalMaterial::realized(
                crate::resources::Material::free_base(name, 1.0),
                vec![crate::structure::Placement {
                    x,
                    y,
                    rotation_radians: 0.0,
                }],
                &catalog,
            )
            .expect("material should have a valid realized physical shape")
        };
        let source = make_material("Carbon", 0.0, 0.0);
        let receiver = make_material("Carbon", 8.0, 0.0);
        let blocker = make_material("Nitrogen", 4.0, 0.0);

        let unobstructed = resonance_pair_potential(&source, &receiver, &catalog, 0)
            .expect("unobstructed pair should have a potential");
        let occluded = resonance_pair_potential_with_occluders(
            &source,
            &receiver,
            std::slice::from_ref(&blocker),
            &catalog,
            0,
        )
        .expect("occluded pair should have a potential");
        assert!(unobstructed > 0.0);
        assert!(
            occluded < unobstructed,
            "a third-party physical body must reduce exposed resonance reception"
        );
    }

    #[test]
    fn pair_potential_tracks_full_partial_and_clear_visibility_transitions() {
        let catalog = crate::resources::default_catalog();
        let make_material = |name: &str, x, y| {
            crate::physical_material::PhysicalMaterial::realized(
                crate::resources::Material::free_base(name, 1.0),
                vec![crate::structure::Placement {
                    x,
                    y,
                    rotation_radians: 0.0,
                }],
                &catalog,
            )
            .expect("material should have a valid realized physical shape")
        };
        let source = make_material("Carbon", 0.0, 0.0);
        let receiver = make_material("Carbon", 8.0, 0.0);
        let unobstructed = resonance_pair_potential(&source, &receiver, &catalog, 0)
            .expect("unobstructed pair should have a potential");

        let centered_blocker = make_material("Nitrogen", 4.0, 0.0);
        let offset_blocker = make_material("Nitrogen", 4.0, 0.35);
        let clear_blocker = make_material("Nitrogen", 4.0, 5.0);
        let centered = resonance_pair_potential_with_occluders(
            &source,
            &receiver,
            std::slice::from_ref(&centered_blocker),
            &catalog,
            0,
        )
        .expect("centered blocker should produce a finite potential");
        let partial = resonance_pair_potential_with_occluders(
            &source,
            &receiver,
            std::slice::from_ref(&offset_blocker),
            &catalog,
            0,
        )
        .expect("offset blocker should produce a finite potential");
        let clear = resonance_pair_potential_with_occluders(
            &source,
            &receiver,
            std::slice::from_ref(&clear_blocker),
            &catalog,
            0,
        )
        .expect("nonblocking body should produce a finite potential");

        assert!(centered >= 0.0 && partial >= 0.0 && clear >= 0.0);
        assert!(
            centered < partial,
            "partially exposed receiver should capture more than a fully shadowed receiver"
        );
        assert!(
            partial < clear,
            "clearer receiver visibility should increase the pair potential"
        );
        assert!(
            (clear - unobstructed).abs() < 1e-9,
            "a body outside the source-receiver visibility path must not change the pair potential"
        );
    }

    #[test]
    fn oscillator_phase_lag_tracks_below_at_and_above_resonance() {
        let natural = 440.0;
        let damping = 0.15;
        let below = oscillator_phase_lag(220.0, natural, damping);
        let at = oscillator_phase_lag(440.0, natural, damping);
        let above = oscillator_phase_lag(880.0, natural, damping);

        assert!(below > 0.0 && below < std::f64::consts::FRAC_PI_2);
        assert!((at - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
        assert!(above > std::f64::consts::FRAC_PI_2 && above < std::f64::consts::PI);
    }

    #[test]
    fn oscillator_phase_lag_changes_with_damping() {
        let low = oscillator_phase_lag(352.0, 440.0, 0.05);
        let high = oscillator_phase_lag(352.0, 440.0, 0.5);
        assert!(high > low);
    }

    #[test]
    fn material_response_applies_lag_to_fundamental_and_generated_harmonics() {
        let input_phase = 0.7;
        let spectrum = material_response(properties(1.0, 1.0, 1.0), baselines(), input_phase);
        let expected_fundamental =
            input_phase - oscillator_phase_lag(440.0, 440.0, BASELINE_DAMPING_RATIO);

        let fundamental = spectrum
            .components
            .iter()
            .find(|component| (component.frequency_cycles_per_day - 440.0).abs() < 1e-9)
            .expect("fundamental response should be present");
        assert!((fundamental.phase_radians - expected_fundamental).abs() < 1e-12);

        for component in spectrum
            .components
            .iter()
            .filter(|component| component.frequency_cycles_per_day > 440.0)
        {
            let order = component.frequency_cycles_per_day / 440.0;
            assert!((component.phase_radians - expected_fundamental * order).abs() < 1e-12);
        }
    }

    #[test]
    fn successive_material_responses_accumulate_oscillator_phase_lag() {
        let incoming = ToneSpectrum {
            components: vec![ToneComponent {
                frequency_cycles_per_day: 440.0,
                amplitude: 1.0,
                phase_radians: 0.2,
            }],
        };
        let first = material_response_spectrum(properties(1.0, 1.0, 1.0), baselines(), &incoming)
            .expect("first material response should be valid");
        let second = material_response_spectrum(properties(1.0, 1.0, 1.0), baselines(), &first)
            .expect("second material response should be valid");

        let fundamental = second
            .components
            .iter()
            .find(|component| (component.frequency_cycles_per_day - 440.0).abs() < 1e-9)
            .expect("fundamental should remain in the transmitted spectrum");
        let one_material_lag = oscillator_phase_lag(440.0, 440.0, BASELINE_DAMPING_RATIO);
        let expected_phase = 0.2 - 2.0 * one_material_lag;
        assert!((fundamental.phase_radians - expected_phase).abs() < 1e-12);
    }

    #[test]
    fn each_incoming_frequency_receives_its_own_oscillator_phase_lag() {
        let incoming = ToneSpectrum {
            components: vec![
                ToneComponent {
                    frequency_cycles_per_day: 440.0,
                    amplitude: 0.5,
                    phase_radians: 0.15,
                },
                ToneComponent {
                    frequency_cycles_per_day: 713.0,
                    amplitude: 0.4,
                    phase_radians: -0.4,
                },
                ToneComponent {
                    frequency_cycles_per_day: 997.0,
                    amplitude: 0.3,
                    phase_radians: 1.1,
                },
            ],
        };
        let material = properties(1.0, 1.0, 1.0);
        let baseline = baselines();
        let natural = natural_frequency_cycles_per_day(material, baseline);
        let damping = damping_ratio(material, baseline);
        let output = material_response_spectrum(material, baseline, &incoming)
            .expect("valid mixed-frequency input should produce a response");

        for incident in &incoming.components {
            let response = output
                .components
                .iter()
                .find(|component| {
                    (component.frequency_cycles_per_day - incident.frequency_cycles_per_day).abs()
                        < 1e-9
                })
                .unwrap_or_else(|| {
                    panic!(
                        "fundamental response at {} cycles/day should be retained",
                        incident.frequency_cycles_per_day
                    )
                });
            let expected_phase = incident.phase_radians
                - oscillator_phase_lag(incident.frequency_cycles_per_day, natural, damping);
            assert!(
                (response.phase_radians.cos() - expected_phase.cos()).abs() < 1e-10
                    && (response.phase_radians.sin() - expected_phase.sin()).abs() < 1e-10,
                "incoming frequency {} must receive its own phase lag",
                incident.frequency_cycles_per_day
            );
        }
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
        let properties = properties(1.0, 4.0, 1.0);
        let baselines = baselines();
        let natural_frequency = natural_frequency_cycles_per_day(properties, baselines);
        let damping = damping_ratio(properties, baselines);
        let fundamental_lag = oscillator_phase_lag(drive_frequency, natural_frequency, damping);

        for component in &spectrum.components {
            let harmonic_order = component.frequency_cycles_per_day / drive_frequency;
            let expected_phase = phase_at_tick(component.frequency_cycles_per_day, tick)
                - harmonic_order * fundamental_lag;
            assert!(
                (component.phase_radians.cos() - expected_phase.cos()).abs() < 1e-10
                    && (component.phase_radians.sin() - expected_phase.sin()).abs() < 1e-10,
                "harmonic at {} cycles/day did not follow the shared clock with its derived oscillator lag",
                component.frequency_cycles_per_day
            );
        }
    }
}
