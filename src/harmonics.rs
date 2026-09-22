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

    pub(crate) fn merge_from(&mut self, other: &ToneSpectrum, scale: f64) {
        add_spectrum(self, other, scale);
        self.retain_strongest();
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
    (1.0 / denominator.max(f64::EPSILON)).min(1.0 / (2.0 * zeta))
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
        frequency_hz: a.frequency_hz,
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
            frequency_hz: component.frequency_hz,
            amplitude: component.amplitude * scale,
            phase_radians: component.phase_radians,
        };
        if let Some(existing) = target.components.iter_mut().find(|existing| {
            (existing.frequency_hz - contribution.frequency_hz).abs() <= 1e-9
        }) {
            *existing = combine_component(*existing, contribution);
        } else {
            target.components.push(contribution);
        }
    }
}

fn realized_unit_spectra(
    structure: &crate::structure::OrganismStructure,
    catalog: &[crate::resources::BaseResource],
) -> Vec<ToneSpectrum> {
    let baselines = ResourceBaselines::from_catalog(catalog);
    let mut local = Vec::with_capacity(structure.units.len());

    for unit in &structure.units {
        let Some(properties) = unit.properties(catalog) else {
            local.push(ToneSpectrum::empty());
            continue;
        };
        local.push(material_response(properties, baselines, 0.0));
    }

    let mut received = local.clone();
    for bond in &structure.bonds {
        let Some(a) = structure.unit_index(bond.endpoint_a.constituent_id) else {
            continue;
        };
        let Some(b) = structure.unit_index(bond.endpoint_b.constituent_id) else {
            continue;
        };
        let coupling = bond.strength.clamp(0.0, 1.0);
        add_spectrum(&mut received[a], &local[b], coupling);
        add_spectrum(&mut received[b], &local[a], coupling);
    }
    received
}

/// Generate the spectrum produced by the actual realized organism graph.
///
/// Every realized unit receives the analytical world tone through its own
/// material response. Existing physical bonds then transmit a portion of the
/// response between their actual physical endpoints. No blueprint, genome
/// target, or abstract sensor participates in this calculation.
pub(crate) fn realized_structure_spectrum(
    structure: &crate::structure::OrganismStructure,
    catalog: &[crate::resources::BaseResource],
) -> ToneSpectrum {
    let received = realized_unit_spectra(structure, catalog);
    let mut spectrum = ToneSpectrum::empty();
    for unit_spectrum in &received {
        add_spectrum(&mut spectrum, unit_spectrum, 1.0);
    }
    spectrum.retain_strongest();
    spectrum
}

/// The physical genome cavity receives the spectrum present at its realized
/// boundary. Boundary membership comes only from the actual cavity analysis;
/// it is never inferred from a blueprint or a hard-coded genome core.
fn environmental_spectrum_at_position(
    field: &crate::environment::ActiveMaterialField,
    catalog: &[crate::resources::BaseResource],
    x: f64,
    y: f64,
) -> ToneSpectrum {
    let Some(index) = field.index_for_position(x, y) else {
        return ToneSpectrum::empty();
    };
    let cell = &field.cells[index];
    let baselines = ResourceBaselines::from_catalog(catalog);
    let mut spectrum = ToneSpectrum::empty();

    for material in &cell.materials {
        if material.is_valid() && !material.is_empty() {
            let response = material_response(material.weighted_properties(catalog), baselines, 0.0);
            add_spectrum(&mut spectrum, &response, 1.0);
        }
    }
    for physical in &cell.physical_materials {
        if physical.material.is_valid() && !physical.material.is_empty() {
            let response =
                material_response(physical.material.weighted_properties(catalog), baselines, 0.0);
            add_spectrum(&mut spectrum, &response, 1.0);
        }
    }
    spectrum.retain_strongest();
    spectrum
}

pub(crate) fn genome_cavity_spectrum(
    structure: &crate::structure::OrganismStructure,
    catalog: &[crate::resources::BaseResource],
    field: &crate::environment::ActiveMaterialField,
    boundary_units: &[usize],
) -> ToneSpectrum {
    if boundary_units.is_empty() {
        return ToneSpectrum::empty();
    }
    let received = realized_unit_spectra(structure, catalog);
    let mut spectrum = ToneSpectrum::empty();
    let mut count = 0usize;

    for &unit_index in boundary_units {
        let Some(unit) = structure.units.get(unit_index) else {
            continue;
        };
        let Some(unit_spectrum) = received.get(unit_index) else {
            continue;
        };
        let environmental =
            environmental_spectrum_at_position(field, catalog, unit.placement.x, unit.placement.y);
        let mut coupled = unit_spectrum.clone();
        coupled.merge_from(&environmental, 1.0);
        add_spectrum(&mut spectrum, &coupled, 1.0 / boundary_units.len() as f64);
        count += 1;
    }

    if count == 0 {
        return ToneSpectrum::empty();
    }
    spectrum.retain_strongest();
    spectrum
}



impl crate::state::Simulation {
    /// Refresh the harmonic state from the organism's actual realized genome
    /// cavity. A non-qualifying physical structure has no genome harmonic
    /// memory surface.
    pub(crate) fn update_organism_harmonics(
        organism: &mut crate::state::Organism,
        environment: &crate::state::Environment,
    ) {
        let boundary_units = crate::cavity::analyze_genome_cavity(
            &organism.structure,
            &environment.catalog,
        )
        .ok()
        .flatten()
        .filter(|cavity| cavity.qualifies())
        .map(|cavity| cavity.boundary_units)
        .unwrap_or_default();

        organism.harmonic_spectrum =
            genome_cavity_spectrum(&organism.structure, &environment.catalog, &boundary_units);
    }
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
