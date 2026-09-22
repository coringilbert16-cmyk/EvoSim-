//! Compact harmonic/world-tone model.
//!
//! The harmonic system is deliberately spectral rather than waveform-based.
//! The world provides a 440 Hz reference tone; realized material structure
//! determines how that tone is filtered, coupled, damped, and enriched with
//! harmonics.

use crate::resources::{ResourceBaselines, ResourceProperties};

pub(crate) const WORLD_TONE_HZ: f64 = 440.0;
pub(crate) const MAX_SPECTRAL_COMPONENTS: usize = 4;
const MIN_DAMPING: f64 = 0.05;

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq)]
pub(crate) struct ToneComponent {
    pub(crate) frequency_hz: f64,
    pub(crate) amplitude: f64,
    pub(crate) phase_radians: f64,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Default)]
pub(crate) struct ToneSpectrum {
    pub(crate) components: Vec<ToneComponent>,
}

impl ToneSpectrum {
    pub(crate) fn empty() -> Self {
        Self::default()
    }

    pub(crate) fn retain_strongest(&mut self) {
        self.components.retain(|component| {
            component.frequency_hz.is_finite()
                && component.frequency_hz > 0.0
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
/// dimensionless relationship to the 440 Hz world reference.
pub(crate) fn natural_frequency_hz(
    properties: ResourceProperties,
    baselines: ResourceBaselines,
) -> f64 {
    let mass = properties.mass.max(f64::EPSILON);
    let cohesion = properties.cohesion.max(f64::EPSILON);
    let baseline_mass = baselines.mass.max(f64::EPSILON);
    let baseline_cohesion = baselines.cohesion.max(f64::EPSILON);
    let ratio = (cohesion / mass) / (baseline_cohesion / baseline_mass);
    WORLD_TONE_HZ * ratio.max(0.0).sqrt()
}

/// Reactivity supplies the existing dissipative/nonlinear response scale.
///
/// The returned value is a dimensionless damping ratio. Higher reactivity
/// produces broader, weaker resonance and stronger nonlinear harmonic content;
/// no new damping resource property is introduced.
pub(crate) fn damping_ratio(
    properties: ResourceProperties,
    baselines: ResourceBaselines,
) -> f64 {
    let reactivity = properties.reactivity.max(0.0);
    let baseline = baselines.reactivity.max(0.0);
    let normalized = if baseline <= f64::EPSILON {
        if reactivity <= f64::EPSILON { 0.0 } else { 1.0 }
    } else {
        reactivity / (reactivity + baseline)
    };
    (MIN_DAMPING + 0.45 * normalized).clamp(MIN_DAMPING, 0.5)
}

/// A compact forced-oscillator response centered on the material's natural
/// frequency. This is a spectral response, not a time-domain simulation.
pub(crate) fn resonance_response(
    drive_frequency_hz: f64,
    natural_frequency_hz: f64,
    damping: f64,
) -> f64 {
    if !drive_frequency_hz.is_finite()
        || !natural_frequency_hz.is_finite()
        || drive_frequency_hz <= 0.0
        || natural_frequency_hz <= 0.0
    {
        return 0.0;
    }
    let zeta = damping.clamp(MIN_DAMPING, 0.5);
    let ratio = drive_frequency_hz / natural_frequency_hz;
    let denominator = ((1.0 - ratio * ratio).powi(2)
        + (2.0 * zeta * ratio).powi(2))
    .sqrt();
    (1.0 / denominator.max(f64::EPSILON)).min(1.0 / (2.0 * zeta));
}

/// Reactivity-driven nonlinear content. Integer multiples are used because
/// real nonlinear oscillators commonly produce harmonic overtones.
pub(crate) fn nonlinear_harmonic_amplitude(
    fundamental_amplitude: f64,
    reactivity: f64,
    harmonic_order: usize,
) -> f64 {
    if harmonic_order < 2 || !fundamental_amplitude.is_finite() {
        return 0.0;
    }
    let nonlinear = (reactivity.max(0.0) / (1.0 + reactivity.max(0.0))).clamp(0.0, 1.0);
    fundamental_amplitude
        * 0.25
        * nonlinear.powi((harmonic_order - 1) as i32)
        / harmonic_order as f64
}

/// Generate the local spectrum produced by one realized material response to
/// the world tone. Structure-level coupling is applied by the caller through
/// the physical bond graph.
pub(crate) fn material_response(
    properties: ResourceProperties,
    baselines: ResourceBaselines,
    phase_radians: f64,
) -> ToneSpectrum {
    let natural = natural_frequency_hz(properties, baselines);
    let damping = damping_ratio(properties, baselines);
    let fundamental = resonance_response(WORLD_TONE_HZ, natural, damping);
    let mut spectrum = ToneSpectrum {
        components: vec![ToneComponent {
            frequency_hz: WORLD_TONE_HZ,
            amplitude: fundamental,
            phase_radians,
        }],
    };

    for order in 2..=MAX_SPECTRAL_COMPONENTS {
        let amplitude =
            nonlinear_harmonic_amplitude(fundamental, properties.reactivity, order);
        if amplitude > f64::EPSILON {
            spectrum.components.push(ToneComponent {
                frequency_hz: WORLD_TONE_HZ * order as f64,
                amplitude,
                phase_radians: phase_radians * order as f64,
            });
        }
    }
    spectrum
}

#[cfg(test)]
mod tests {
    use super::*;

    fn properties(mass: f64, reactivity: f64, cohesion: f64) -> ResourceProperties {
        ResourceProperties {
            mass,
            potential_energy: 1.0,
            reactivity,
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
        assert_eq!(WORLD_TONE_HZ, 440.0);
    }

    #[test]
    fn natural_frequency_follows_cohesion_over_mass() {
        assert!((natural_frequency_hz(properties(1.0, 0.0, 1.0), baselines()) - 440.0).abs() < 1e-9);
        assert!(natural_frequency_hz(properties(0.5, 0.0, 1.0), baselines()) > 440.0);
        assert!(natural_frequency_hz(properties(1.0, 0.0, 0.5), baselines()) < 440.0);
    }

    #[test]
    fn reactivity_broadens_damping_and_adds_nonlinear_content() {
        let low = damping_ratio(properties(1.0, 0.0, 1.0), baselines());
        let high = damping_ratio(properties(1.0, 4.0, 1.0), baselines());
        assert!(high > low);
        assert_eq!(nonlinear_harmonic_amplitude(1.0, 0.0, 2), 0.0);
        assert!(nonlinear_harmonic_amplitude(1.0, 4.0, 2) > 0.0);
    }

    #[test]
    fn material_response_is_compact_and_harmonic() {
        let spectrum = material_response(properties(1.0, 4.0, 1.0), baselines(), 0.25);
        assert!(!spectrum.components.is_empty());
        assert!(spectrum
            .components
            .iter()
            .any(|component| (component.frequency_hz - WORLD_TONE_HZ).abs() < f64::EPSILON));
        assert!(spectrum
            .components
            .iter()
            .any(|component| (component.frequency_hz - 2.0 * WORLD_TONE_HZ).abs() < f64::EPSILON));
        assert!(spectrum.components.len() <= MAX_SPECTRAL_COMPONENTS);
    }
}
