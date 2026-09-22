//! Compact harmonic/world-tone model.
//!
//! The harmonic system is deliberately spectral rather than waveform-based.
//! The world provides a 440 Hz reference tone; realized material structure
//! determines how that tone is filtered, coupled, damped, and enriched with
//! harmonics.

use crate::resources::{ResourceBaselines, ResourceProperties};

pub(crate) const WORLD_TONE_HZ: f64 = 440.0;
pub(crate) const MAX_SPECTRAL_COMPONENTS: usize = 4;

/// EXPERIMENTAL: absolute damping calibration for the harmonic model.
/// Reactivity determines relative damping; this constant sets the model scale.
pub(crate) const BASELINE_DAMPING_RATIO: f64 = 0.10;

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
/// The absolute scale is experimental; relative damping is derived from the
/// material's reactivity relative to the catalog baseline. No new
/// damping-only resource property is introduced.
pub(crate) fn damping_ratio(properties: ResourceProperties, baselines: ResourceBaselines) -> f64 {
    let reactivity = properties.reactivity.max(0.0);
    let baseline = baselines.reactivity.max(0.0);
    if baseline <= f64::EPSILON {
        return if reactivity <= f64::EPSILON {
            0.0
        } else {
            BASELINE_DAMPING_RATIO
        };
    }
    BASELINE_DAMPING_RATIO * (reactivity / baseline)
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
    let zeta = damping.max(f64::EPSILON);
    let ratio = drive_frequency_hz / natural_frequency_hz;
    let denominator = ((1.0 - ratio * ratio).powi(2) + (2.0 * zeta * ratio).powi(2)).sqrt();
    1.0 / denominator.max(f64::EPSILON)
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
    let reactivity = reactivity.max(0.0);
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
        let amplitude = nonlinear_harmonic_amplitude(fundamental, properties.reactivity, order);
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
        if let Some(existing) = target
            .components
            .iter_mut()
            .find(|existing| (existing.frequency_hz - contribution.frequency_hz).abs() <= 1e-9)
        {
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

/// Environmental quantity scales excitation using absolute material mass.
/// This is a response scale, not a new material property.
fn environmental_mass_scale(mass: f64, baseline_mass: f64) -> f64 {
    let mass = mass.max(0.0);
    let baseline = baseline_mass.max(f64::EPSILON);
    mass / (mass + baseline)
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
            let mass = material.mass(catalog);
            let response = material_response(material.weighted_properties(catalog), baselines, 0.0);
            add_spectrum(
                &mut spectrum,
                &response,
                environmental_mass_scale(mass, baselines.mass),
            );
        }
    }
    for physical in &cell.physical_materials {
        if physical.material.is_valid() && !physical.material.is_empty() {
            let mass = physical.material.mass(catalog);
            let response = material_response(
                physical.material.weighted_properties(catalog),
                baselines,
                0.0,
            );
            add_spectrum(
                &mut spectrum,
                &response,
                environmental_mass_scale(mass, baselines.mass),
            );
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

/// Refresh the harmonic state from the organism's actual realized genome
/// cavity. A non-qualifying physical structure has no genome harmonic
/// memory surface.
pub(crate) fn update_organism_harmonics(
    organism: &mut crate::state::Organism,
    environment: &crate::state::Environment,
) {
    let boundary_units =
        crate::cavity::analyze_genome_cavity(&organism.structure, &environment.catalog)
            .ok()
            .flatten()
            .filter(|cavity| cavity.qualifies())
            .map(|cavity| cavity.boundary_units)
            .unwrap_or_default();

    organism.harmonic_spectrum = genome_cavity_spectrum(
        &organism.structure,
        &environment.catalog,
        &environment.field,
        &boundary_units,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

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

        let baseline = genome_cavity_spectrum(&structure, &catalog, &field, &cavity.boundary_units);
        let boundary = &structure.units[cavity.boundary_units[0]].placement;
        assert!(field.deposit(
            boundary.x,
            boundary.y,
            crate::resources::Material::free_base("Carbon", 1.0),
        ));
        let coupled = genome_cavity_spectrum(&structure, &catalog, &field, &cavity.boundary_units);

        let baseline_fundamental = baseline
            .components
            .iter()
            .find(|component| (component.frequency_hz - WORLD_TONE_HZ).abs() < f64::EPSILON)
            .map(|component| component.amplitude)
            .unwrap_or(0.0);
        let coupled_fundamental = coupled
            .components
            .iter()
            .find(|component| (component.frequency_hz - WORLD_TONE_HZ).abs() < f64::EPSILON)
            .map(|component| component.amplitude)
            .unwrap_or(0.0);
        assert!(coupled_fundamental > baseline_fundamental);
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

        let received = genome_cavity_spectrum(&structure, &catalog, &field, &cavity.boundary_units);
        assert!(!received.components.is_empty());

        structure.bonds.clear();
        assert!(crate::cavity::analyze_genome_cavity(&structure, &catalog)
            .unwrap()
            .is_none());
        let absent = genome_cavity_spectrum(&structure, &catalog, &field, &[]);
        assert!(absent.components.is_empty());
    }

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
        assert!(
            (natural_frequency_hz(properties(1.0, 0.0, 1.0), baselines()) - 440.0).abs() < 1e-9
        );
        assert!(natural_frequency_hz(properties(0.5, 0.0, 1.0), baselines()) > 440.0);
        assert!(natural_frequency_hz(properties(1.0, 0.0, 0.5), baselines()) < 440.0);
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
    fn material_response_is_compact_and_harmonic() {
        let spectrum = material_response(properties(1.0, 4.0, 1.0), baselines(), 0.25);
        assert!(!spectrum.components.is_empty());
        assert!(spectrum
            .components
            .iter()
            .any(|component| (component.frequency_hz - WORLD_TONE_HZ).abs() < f64::EPSILON));
        assert!(spectrum.components.iter().any(|component| {
            (component.frequency_hz - 2.0 * WORLD_TONE_HZ).abs() < f64::EPSILON
        }));
        assert!(spectrum.components.len() <= MAX_SPECTRAL_COMPONENTS);
    }
}
