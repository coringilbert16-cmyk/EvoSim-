//! Phase 3D isolated spectral reconstruction of incident boundary flux.
//!
//! Estimator only: no geometry discovery, material calibration, field debit,
//! or chemistry connection.
//!
//! Samples are uniformly spaced over one periodic DFT window. For each
//! boundary quadrature location, synchronized velocity and normal-gradient
//! traces are transformed, their full complex coefficient convolution is
//! formed (including every frequency-pair cross term), and signed flux is
//! reconstructed before applying max(0, -q). Composite trapezoidal time
//! integration is used for both spectral and direct reference totals. Callers
//! declare an upper input-frequency bound whose doubled product bandwidth
//! remains below Nyquist.

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BoundaryTrace<'a> {
    pub times: &'a [f64],
    pub normal_velocity: &'a [f64],
    pub normal_gradient: &'a [f64],
    pub boundary_weight: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SpectralFluxEstimate {
    pub incident_energy: f64,
    pub direct_reference_energy: f64,
    pub reconciliation_residual: f64,
    pub max_signed_flux_error: f64,
}

#[derive(Clone, Copy, Debug, Default)]
struct Complex {
    re: f64,
    im: f64,
}

impl Complex {
    fn add(self, rhs: Self) -> Self {
        Self {
            re: self.re + rhs.re,
            im: self.im + rhs.im,
        }
    }

    fn mul(self, rhs: Self) -> Self {
        Self {
            re: self.re * rhs.re - self.im * rhs.im,
            im: self.re * rhs.im + self.im * rhs.re,
        }
    }

    fn scale(self, factor: f64) -> Self {
        Self {
            re: self.re * factor,
            im: self.im * factor,
        }
    }
}

/// Estimate incident energy from synchronized boundary traces.
///
/// max_frequency is the caller-declared highest physical frequency in cycles
/// per project-time unit. Windows where twice this frequency reaches Nyquist
/// fail closed. The caller is responsible for supplying a truthful band limit.
pub(crate) fn estimate_incident_energy(
    traces: &[BoundaryTrace<'_>],
    stiffness: f64,
    wave_speed: f64,
    max_frequency: f64,
) -> Option<SpectralFluxEstimate> {
    if traces.is_empty()
        || !stiffness.is_finite()
        || stiffness <= 0.0
        || !wave_speed.is_finite()
        || wave_speed <= 0.0
        || !max_frequency.is_finite()
        || max_frequency < 0.0
    {
        return None;
    }

    let mut spectral_total = 0.0;
    let mut direct_total = 0.0;
    let mut max_error: f64 = 0.0;

    for trace in traces {
        let count = trace.times.len();
        if count < 4
            || trace.normal_velocity.len() != count
            || trace.normal_gradient.len() != count
            || !trace.boundary_weight.is_finite()
            || trace.boundary_weight <= 0.0
        {
            return None;
        }

        let dt = trace.times[1] - trace.times[0];
        if !dt.is_finite() || dt <= 0.0 || 2.0 * max_frequency >= 0.5 / dt {
            return None;
        }
        let time_tolerance = 1.0e-9 * dt.max(1.0);
        for index in 0..count {
            if !trace.times[index].is_finite()
                || !trace.normal_velocity[index].is_finite()
                || !trace.normal_gradient[index].is_finite()
                || (index > 0
                    && ((trace.times[index] - trace.times[index - 1]) - dt).abs() > time_tolerance)
            {
                return None;
            }
        }

        let velocity_spectrum = forward_dft(trace.normal_velocity);
        let gradient_spectrum = forward_dft(trace.normal_gradient);
        let flux_spectrum = pairwise_flux_spectrum(
            &velocity_spectrum,
            &gradient_spectrum,
            stiffness * wave_speed * wave_speed,
        );
        let reconstructed_flux = inverse_dft_real(&flux_spectrum);

        let mut spectral_power = Vec::with_capacity(count);
        let mut direct_power = Vec::with_capacity(count);
        for index in 0..count {
            let direct_flux = -stiffness
                * wave_speed
                * wave_speed
                * trace.normal_velocity[index]
                * trace.normal_gradient[index];
            let reconstructed = reconstructed_flux[index];
            if !direct_flux.is_finite() || !reconstructed.is_finite() {
                return None;
            }
            max_error = max_error.max((reconstructed - direct_flux).abs());
            // Apply the incoming clamp only after complete signed-flux reconstruction.
            spectral_power.push((-reconstructed).max(0.0));
            direct_power.push((-direct_flux).max(0.0));
        }

        let mut spectral_energy = 0.0;
        let mut direct_energy = 0.0;
        for index in 0..count - 1 {
            spectral_energy += 0.5 * (spectral_power[index] + spectral_power[index + 1]) * dt;
            direct_energy += 0.5 * (direct_power[index] + direct_power[index + 1]) * dt;
        }
        spectral_total += spectral_energy * trace.boundary_weight;
        direct_total += direct_energy * trace.boundary_weight;
        if !spectral_total.is_finite() || !direct_total.is_finite() || !max_error.is_finite() {
            return None;
        }
    }

    Some(SpectralFluxEstimate {
        incident_energy: spectral_total,
        direct_reference_energy: direct_total,
        reconciliation_residual: spectral_total - direct_total,
        max_signed_flux_error: max_error,
    })
}

fn forward_dft(samples: &[f64]) -> Vec<Complex> {
    let count = samples.len();
    (0..count)
        .map(|frequency| {
            let mut coefficient = Complex::default();
            for (sample_index, sample) in samples.iter().enumerate() {
                let angle =
                    -std::f64::consts::TAU * frequency as f64 * sample_index as f64 / count as f64;
                coefficient = coefficient.add(
                    Complex {
                        re: angle.cos(),
                        im: angle.sin(),
                    }
                    .scale(*sample / count as f64),
                );
            }
            coefficient
        })
        .collect()
}

/// Full pairwise coefficient product. Modulo indexing is the DFT circular
/// convolution and retains conjugate terms and every cross-frequency pair.
fn pairwise_flux_spectrum(velocity: &[Complex], gradient: &[Complex], scale: f64) -> Vec<Complex> {
    let count = velocity.len();
    let mut flux = vec![Complex::default(); count];
    for (velocity_frequency, velocity_coefficient) in velocity.iter().enumerate() {
        for (gradient_frequency, gradient_coefficient) in gradient.iter().enumerate() {
            let output_frequency = (velocity_frequency + gradient_frequency) % count;
            flux[output_frequency] = flux[output_frequency].add(
                velocity_coefficient
                    .mul(*gradient_coefficient)
                    .scale(-scale),
            );
        }
    }
    flux
}

fn inverse_dft_real(coefficients: &[Complex]) -> Vec<f64> {
    let count = coefficients.len();
    (0..count)
        .map(|sample_index| {
            let mut value = Complex::default();
            for (frequency, coefficient) in coefficients.iter().enumerate() {
                let angle =
                    std::f64::consts::TAU * frequency as f64 * sample_index as f64 / count as f64;
                value = value.add(coefficient.mul(Complex {
                    re: angle.cos(),
                    im: angle.sin(),
                }));
            }
            value.re
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sinusoid(count: usize, bin: usize, phase: f64) -> Vec<f64> {
        (0..count)
            .map(|index| {
                (std::f64::consts::TAU * bin as f64 * index as f64 / count as f64 + phase).sin()
            })
            .collect()
    }

    fn times(count: usize, dt: f64) -> Vec<f64> {
        (0..count).map(|index| index as f64 * dt).collect()
    }

    #[test]
    fn single_tone_reconstructs_signed_flux_and_incident_integral() {
        let count = 64;
        let t = times(count, 0.01);
        let velocity = sinusoid(count, 3, 0.2);
        let gradient = sinusoid(count, 3, -0.4);
        let result = estimate_incident_energy(
            &[BoundaryTrace {
                times: &t,
                normal_velocity: &velocity,
                normal_gradient: &gradient,
                boundary_weight: 2.0,
            }],
            1.5,
            2.0,
            4.0,
        )
        .unwrap();
        assert!(result.max_signed_flux_error < 1.0e-11);
        assert!(result.reconciliation_residual.abs() < 1.0e-11);
        assert!(result.incident_energy > 0.0);
    }

    #[test]
    fn two_tone_phase_changes_are_preserved_by_pairwise_cross_terms() {
        let count = 128;
        let t = times(count, 0.01);
        let velocity: Vec<_> = (0..count)
            .map(|index| {
                let phase = std::f64::consts::TAU * 5.0 * index as f64 / count as f64;
                phase.sin() + 0.7 * (2.0 * phase + 0.3).sin()
            })
            .collect();
        let gradient_a: Vec<_> = (0..count)
            .map(|index| {
                let phase = std::f64::consts::TAU * 5.0 * index as f64 / count as f64;
                (phase + 0.1).sin() + 0.7 * (2.0 * phase + 0.3).sin()
            })
            .collect();
        let gradient_b: Vec<_> = (0..count)
            .map(|index| {
                let phase = std::f64::consts::TAU * 5.0 * index as f64 / count as f64;
                (phase + 1.1).sin() + 0.7 * (2.0 * phase + 0.3).sin()
            })
            .collect();
        let estimate = |gradient: &[f64]| {
            estimate_incident_energy(
                &[BoundaryTrace {
                    times: &t,
                    normal_velocity: &velocity,
                    normal_gradient: gradient,
                    boundary_weight: 1.0,
                }],
                1.0,
                1.0,
                8.0,
            )
            .unwrap()
        };
        let first = estimate(&gradient_a);
        let shifted = estimate(&gradient_b);
        assert!(first.max_signed_flux_error < 1.0e-10);
        assert!(shifted.max_signed_flux_error < 1.0e-10);
        assert!((first.incident_energy - shifted.incident_energy).abs() > 1.0e-3);
    }

    #[test]
    fn incoming_clamp_follows_complete_signed_flux_reconstruction() {
        let count = 96;
        let t = times(count, 0.02);
        let velocity = sinusoid(count, 7, 0.0);
        let gradient = sinusoid(count, 7, 0.0);
        let estimate = estimate_incident_energy(
            &[BoundaryTrace {
                times: &t,
                normal_velocity: &velocity,
                normal_gradient: &gradient,
                boundary_weight: 0.75,
            }],
            2.0,
            1.25,
            8.0,
        )
        .unwrap();
        assert!(estimate.incident_energy > 0.0);
        assert!(estimate.reconciliation_residual.abs() < 1.0e-10);
        assert!(estimate.max_signed_flux_error < 1.0e-10);
    }

    #[test]
    fn rejects_nonuniform_underresolved_and_invalid_windows() {
        let velocity = [0.0, 1.0, 0.0, -1.0, 0.0, 1.0, 0.0, -1.0];
        let gradient = velocity;
        let regular = times(8, 0.1);
        let mut irregular = regular.clone();
        irregular[4] += 0.01;
        let irregular_trace = BoundaryTrace {
            times: &irregular,
            normal_velocity: &velocity,
            normal_gradient: &gradient,
            boundary_weight: 1.0,
        };
        let regular_trace = BoundaryTrace {
            times: &regular,
            normal_velocity: &velocity,
            normal_gradient: &gradient,
            boundary_weight: 1.0,
        };
        assert!(estimate_incident_energy(&[irregular_trace], 1.0, 1.0, 1.0).is_none());
        assert!(estimate_incident_energy(&[regular_trace], 1.0, 1.0, 5.0).is_none());
        // The product of two band-limited traces can reach twice the input band.
        assert!(estimate_incident_energy(&[regular_trace], 1.0, 1.0, 3.0).is_none());
        assert!(estimate_incident_energy(&[regular_trace], 1.0, 1.0, f64::NAN).is_none());
    }

    #[test]
    fn temporal_refinement_reduces_incident_integral_difference() {
        let estimate = |count: usize| {
            let duration = 1.0;
            let t: Vec<_> = (0..count)
                .map(|index| index as f64 * duration / count as f64)
                .collect();
            let velocity: Vec<_> = t
                .iter()
                .map(|time| (std::f64::consts::TAU * 5.0 * time).sin())
                .collect();
            let gradient: Vec<_> = t
                .iter()
                .map(|time| (std::f64::consts::TAU * 5.0 * time + 0.37).sin())
                .collect();
            estimate_incident_energy(
                &[BoundaryTrace {
                    times: &t,
                    normal_velocity: &velocity,
                    normal_gradient: &gradient,
                    boundary_weight: 1.0,
                }],
                1.0,
                1.0,
                5.0,
            )
            .unwrap()
            .incident_energy
        };
        let coarse = estimate(32);
        let refined = estimate(64);
        let finer = estimate(128);
        assert!(
            (refined - finer).abs() < (coarse - refined).abs(),
            "temporal refinement did not reduce incident-integral change: coarse={coarse}, refined={refined}, finer={finer}"
        );
    }

    #[test]
    fn repeated_estimates_are_deterministic_and_boundary_weights_add() {
        let count = 32;
        let t = times(count, 0.02);
        let velocity = sinusoid(count, 2, 0.4);
        let gradient = sinusoid(count, 2, -0.2);
        let trace = BoundaryTrace {
            times: &t,
            normal_velocity: &velocity,
            normal_gradient: &gradient,
            boundary_weight: 0.5,
        };
        let first = estimate_incident_energy(&[trace, trace], 1.0, 1.0, 3.0).unwrap();
        let second = estimate_incident_energy(&[trace, trace], 1.0, 1.0, 3.0).unwrap();
        assert_eq!(first, second);
        let single = estimate_incident_energy(&[trace], 1.0, 1.0, 3.0).unwrap();
        assert!((first.incident_energy - 2.0 * single.incident_energy).abs() < 1.0e-12);
    }

    #[test]
    fn incident_integral_converges_to_independent_analytic_reference() {
        // For u_t = partial_n u = sin(2*pi*f*t), q = -sin^2(2*pi*f*t),
        // so the incoming power is sin^2 and its exact integral on [0, T] is
        // T/2 - sin(4*pi*f*T)/(8*pi*f).
        let frequency = 3.1;
        let duration = 1.0;
        let exact = duration / 2.0
            - (std::f64::consts::TAU * 2.0 * frequency * duration).sin()
                / (8.0 * std::f64::consts::PI * frequency);
        let estimate = |count: usize| {
            let dt = duration / (count - 1) as f64;
            let t: Vec<_> = (0..count).map(|index| index as f64 * dt).collect();
            let signal: Vec<_> = t
                .iter()
                .map(|time| (std::f64::consts::TAU * frequency * time).sin())
                .collect();
            estimate_incident_energy(
                &[BoundaryTrace {
                    times: &t,
                    normal_velocity: &signal,
                    normal_gradient: &signal,
                    boundary_weight: 1.0,
                }],
                1.0,
                1.0,
                frequency,
            )
            .unwrap()
            .incident_energy
        };

        let coarse = estimate(33);
        let refined = estimate(65);
        let fine = estimate(129);
        assert!(
            (refined - exact).abs() < (coarse - exact).abs(),
            "first refinement did not approach analytic integral: coarse={coarse}, refined={refined}, exact={exact}"
        );
        assert!(
            (fine - exact).abs() < (refined - exact).abs(),
            "second refinement did not approach analytic integral: refined={refined}, fine={fine}, exact={exact}"
        );
        assert!(
            (fine - exact).abs() < 2.0e-4,
            "fine estimate did not meet analytic accuracy target: fine={fine}, exact={exact}"
        );
    }

    #[test]
    fn spatial_quadrature_refinement_converges_to_analytic_boundary_integral() {
        let count = 129;
        let duration = 1.0;
        let frequency = 3.1;
        let dt = duration / (count - 1) as f64;
        let t: Vec<_> = (0..count).map(|index| index as f64 * dt).collect();
        let base_signal: Vec<_> = t
            .iter()
            .map(|time| (std::f64::consts::TAU * frequency * time).sin())
            .collect();
        let temporal_exact = duration / 2.0
            - (std::f64::consts::TAU * 2.0 * frequency * duration).sin()
                / (8.0 * std::f64::consts::PI * frequency);
        let exact = temporal_exact / 3.0;

        let spatial_estimate = |segments: usize| {
            let traces: Vec<_> = (0..segments)
                .map(|segment| {
                    let midpoint = (segment as f64 + 0.5) / segments as f64;
                    let scaled_signal: Vec<_> =
                        base_signal.iter().map(|sample| sample * midpoint).collect();
                    (scaled_signal, 1.0 / segments as f64)
                })
                .collect();
            let scaled: Vec<_> = traces
                .iter()
                .map(|(signal, weight)| BoundaryTrace {
                    times: &t,
                    normal_velocity: signal,
                    normal_gradient: signal,
                    boundary_weight: *weight,
                })
                .collect();
            estimate_incident_energy(&scaled, 1.0, 1.0, frequency)
                .unwrap()
                .incident_energy
        };

        let coarse = spatial_estimate(4);
        let refined = spatial_estimate(8);
        let fine = spatial_estimate(16);
        assert!(
            (refined - exact).abs() < (coarse - exact).abs(),
            "first spatial refinement did not approach analytic integral: coarse={coarse}, refined={refined}, exact={exact}"
        );
        assert!(
            (fine - exact).abs() < (refined - exact).abs(),
            "second spatial refinement did not approach analytic integral: refined={refined}, fine={fine}, exact={exact}"
        );
    }
}
