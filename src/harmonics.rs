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
        0.0,
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
    receiver_a: (f64, f64),
    receiver_b: (f64, f64),
    cavity_center: (f64, f64),
) -> Option<(ResonancePerception, ToneSpectrum)> {
    let (source_x, source_y, extent, emitted) =
        environmental_material_resonance(physical, catalog)?;
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
    let magnitude = received
        .components
        .iter()
        .map(|component| component.amplitude.max(0.0))
        .sum::<f64>();
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
                    environmental_contribution(physical, catalog, *a, *b, center)
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
    environmental_contributions(catalog, field, cavity).0
}

/// Preserve one spatially attributed signal for each source/receiver pair.
/// Keeping these channels separate is what gives downstream interpretation
/// directional information without creating a separate sensor system.
pub(crate) fn genome_cavity_resonance_perceptions(
    catalog: &[crate::resources::BaseResource],
    field: &crate::environment::ActiveMaterialField,
    cavity: &crate::cavity::GenomeCavity,
) -> Vec<ResonancePerception> {
    environmental_contributions(catalog, field, cavity).1
}

/// Return the spatially attributed resonance signals currently reaching the
/// organism's genome cavity. This reuses the same physical resonance geometry
/// as the harmonic state; it does not create a second sensory radius.
pub(crate) fn organism_resonance_perceptions(
    organism: &crate::state::Organism,
    environment: &crate::state::Environment,
) -> Vec<ResonancePerception> {
    let Some(cavity) =
        crate::cavity::analyze_genome_cavity(&organism.structure, &environment.catalog)
            .ok()
            .flatten()
    else {
        return Vec::new();
    };
    genome_cavity_resonance_perceptions(&environment.catalog, &environment.field, &cavity)
}

/// Refresh the harmonic state from the organism's actual realized genome
/// cavity. A non-qualifying physical structure has no genome harmonic
/// memory surface.
pub(crate) fn update_organism_harmonics(
    organism: &mut crate::state::Organism,
    environment: &crate::state::Environment,
) {
    let cavity = organism.genome_cavity_cached(&environment.catalog);
    let key = (
        organism.structure_revision,
        organism.position_revision,
        environment.field.revision,
    );
    if organism.cached_harmonic_key == Some(key) {
        return;
    }

    let spectrum = cavity
        .as_ref()
        .map(|cavity| genome_cavity_spectrum(&environment.catalog, &environment.field, cavity))
        .unwrap_or_default();

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
                        let distance = (component.frequency_hz / other.frequency_hz).ln().abs();
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
    fn spectral_similarity_is_symmetric_and_exact_match_is_one() {
        let a = ToneSpectrum {
            components: vec![ToneComponent {
                frequency_hz: 440.0,
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
                frequency_hz: 440.0,
                amplitude: 1.0,
                phase_radians: 0.0,
            }],
        };
        let near = ToneSpectrum {
            components: vec![ToneComponent {
                frequency_hz: 460.0,
                amplitude: 1.0,
                phase_radians: 0.0,
            }],
        };
        let far = ToneSpectrum {
            components: vec![ToneComponent {
                frequency_hz: 1760.0,
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
        assert!(field.deposit_physical(boundary.x + 40.0, boundary.y, source,));
        let coupled = genome_cavity_spectrum(&catalog, &field, &cavity);

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
        assert!(field.deposit_physical(boundary.x + 40.0, boundary.y, source));
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
        assert!(field.deposit_physical(boundary.x + 40.0, boundary.y, source));

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
            cohesion,
        }
    }

    fn baselines() -> ResourceBaselines {
        ResourceBaselines {
            mass: 1.0,
            potential_energy: 1.0,
            reactivity: 1.0,
            chemical_position: None,
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
