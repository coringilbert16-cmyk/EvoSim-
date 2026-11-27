//! Passive, local chemical-structure response to incident wave-field energy.
//!
//! This is an isolated accounting kernel. It does not sample geometry, estimate
//! spectra, advance the field, or connect to chemistry. Its inputs must come
//! from a future validated integration of incident normal flux over the exact
//! exposed material boundary.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SpectralIncidentEnergy {
    /// Frequency in cycles per logical simulation tick.
    pub(crate) frequency_cycles_per_tick: f64,
    /// Nonnegative incident energy assigned to this spectral component.
    /// Components are an analysis of one time-domain field, not separate stores.
    pub(crate) incident_energy: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ChemicalBoundaryResponse {
    /// Fraction of incident energy directly absorbed as reaction work.
    /// This must be zero when no eligible reaction is actively coupled.
    pub(crate) reaction_work_absorptance: f64,
    /// Fraction remaining in the field as reflected energy.
    pub(crate) reflectance: f64,
    /// Fraction continuing through the material as transmitted energy.
    pub(crate) transmittance: f64,
    /// Fraction remaining in the field as scattered energy.
    pub(crate) scatterance: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct ChemicalBoundaryEnergyPartition {
    pub(crate) incident: f64,
    pub(crate) reaction_work: f64,
    pub(crate) reflected: f64,
    pub(crate) transmitted: f64,
    pub(crate) scattered: f64,
}

impl ChemicalBoundaryResponse {
    fn is_valid(self) -> bool {
        let values = [
            self.reaction_work_absorptance,
            self.reflectance,
            self.transmittance,
            self.scatterance,
        ];
        values
            .iter()
            .all(|value| value.is_finite() && (0.0..=1.0).contains(value))
            && (values.iter().sum::<f64>() - 1.0).abs() <= 1.0e-12
    }
}

/// One measured or explicitly supplied constitutive response point. The active
/// and inactive curves are separate because reaction-work absorption must not
/// be silently redirected when no reaction is eligible.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MaterialResponseKnot {
    pub(crate) frequency_cycles_per_tick: f64,
    pub(crate) active_reaction: ChemicalBoundaryResponse,
    pub(crate) inactive_reaction: ChemicalBoundaryResponse,
}

/// A bounded, piecewise-linear frequency response. Values outside the
/// calibrated frequency interval fail closed; no extrapolation is performed.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MaterialResponseCalibration {
    knots: Vec<MaterialResponseKnot>,
}

impl MaterialResponseCalibration {
    pub(crate) fn new(knots: Vec<MaterialResponseKnot>) -> Option<Self> {
        if knots.is_empty() {
            return None;
        }
        let mut previous_frequency = None;
        for knot in &knots {
            let frequency = knot.frequency_cycles_per_tick;
            if !frequency.is_finite()
                || frequency.is_sign_negative()
                || frequency < 0.0
                || previous_frequency.is_some_and(|previous| frequency <= previous)
                || !knot.active_reaction.is_valid()
                || !knot.inactive_reaction.is_valid()
                || knot.inactive_reaction.reaction_work_absorptance != 0.0
            {
                return None;
            }
            previous_frequency = Some(frequency);
        }
        Some(Self { knots })
    }

    pub(crate) fn response_at(
        &self,
        frequency_cycles_per_tick: f64,
        reaction_active: bool,
    ) -> Option<ChemicalBoundaryResponse> {
        if !frequency_cycles_per_tick.is_finite()
            || frequency_cycles_per_tick.is_sign_negative()
            || frequency_cycles_per_tick < 0.0
        {
            return None;
        }
        let first = self.knots.first()?;
        let last = self.knots.last()?;
        if frequency_cycles_per_tick < first.frequency_cycles_per_tick
            || frequency_cycles_per_tick > last.frequency_cycles_per_tick
        {
            return None;
        }

        let response = |knot: &MaterialResponseKnot| {
            if reaction_active {
                knot.active_reaction
            } else {
                knot.inactive_reaction
            }
        };
        match self.knots.binary_search_by(|knot| {
            knot.frequency_cycles_per_tick
                .total_cmp(&frequency_cycles_per_tick)
        }) {
            Ok(index) => Some(response(&self.knots[index])),
            Err(upper_index) => {
                let lower = &self.knots[upper_index - 1];
                let upper = &self.knots[upper_index];
                let fraction = (frequency_cycles_per_tick - lower.frequency_cycles_per_tick)
                    / (upper.frequency_cycles_per_tick - lower.frequency_cycles_per_tick);
                let lower_response = response(lower);
                let upper_response = response(upper);
                let interpolate = |a: f64, b: f64| a + fraction * (b - a);
                let result = ChemicalBoundaryResponse {
                    reaction_work_absorptance: interpolate(
                        lower_response.reaction_work_absorptance,
                        upper_response.reaction_work_absorptance,
                    ),
                    reflectance: interpolate(
                        lower_response.reflectance,
                        upper_response.reflectance,
                    ),
                    transmittance: interpolate(
                        lower_response.transmittance,
                        upper_response.transmittance,
                    ),
                    scatterance: interpolate(
                        lower_response.scatterance,
                        upper_response.scatterance,
                    ),
                };
                result.is_valid().then_some(result)
            }
        }
    }
}

/// Apply a bounded material calibration to the incident spectral analysis.
/// This performs accounting only: it neither changes the field nor settles a
/// reaction. A missing/out-of-range coefficient fails closed.
pub(crate) fn partition_calibrated_incident_energy(
    components: &[SpectralIncidentEnergy],
    calibration: &MaterialResponseCalibration,
    reaction_active: bool,
) -> Option<ChemicalBoundaryEnergyPartition> {
    let mut resolved = Vec::with_capacity(components.len());
    for component in components {
        let response =
            calibration.response_at(component.frequency_cycles_per_tick, reaction_active)?;
        resolved.push((*component, response));
    }
    partition_chemical_boundary_energy(&resolved)
}

/// Partition already-integrated incident energy for one material response.
///
/// Frequency-dependent coefficients are supplied per component by the
/// constitutive model; this function deliberately invents no material values.
/// The spectral components must reconstruct the incident energy without
/// dropping cross-frequency terms. A future field/geometry adapter must derive
/// incident energy from max(0, -J·n) integrated over exposed boundary length
/// and interaction time before calling this accounting kernel.
pub(crate) fn partition_chemical_boundary_energy(
    components: &[(SpectralIncidentEnergy, ChemicalBoundaryResponse)],
) -> Option<ChemicalBoundaryEnergyPartition> {
    let mut result = ChemicalBoundaryEnergyPartition::default();
    for (component, response) in components {
        if !component.frequency_cycles_per_tick.is_finite()
            || component.frequency_cycles_per_tick < 0.0
            || !component.incident_energy.is_finite()
            || component.incident_energy < 0.0
            || !response.is_valid()
        {
            return None;
        }

        let incident = component.incident_energy;
        let next_incident = result.incident + incident;
        let next_work = result.reaction_work + incident * response.reaction_work_absorptance;
        let next_reflected = result.reflected + incident * response.reflectance;
        let next_transmitted = result.transmitted + incident * response.transmittance;
        let next_scattered = result.scattered + incident * response.scatterance;
        if [
            next_incident,
            next_work,
            next_reflected,
            next_transmitted,
            next_scattered,
        ]
        .iter()
        .any(|value| !value.is_finite())
        {
            return None;
        }
        result = ChemicalBoundaryEnergyPartition {
            incident: next_incident,
            reaction_work: next_work,
            reflected: next_reflected,
            transmitted: next_transmitted,
            scattered: next_scattered,
        };
    }

    let accounted = result.reaction_work + result.reflected + result.transmitted + result.scattered;
    let tolerance = 1.0e-10 * result.incident.max(1.0);
    (accounted.is_finite() && (accounted - result.incident).abs() <= tolerance).then_some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(
        work: f64,
        reflected: f64,
        transmitted: f64,
        scattered: f64,
    ) -> ChemicalBoundaryResponse {
        ChemicalBoundaryResponse {
            reaction_work_absorptance: work,
            reflectance: reflected,
            transmittance: transmitted,
            scatterance: scattered,
        }
    }

    fn idealized_test_calibration() -> MaterialResponseCalibration {
        // Test fixtures only. These values are not asserted to describe any
        // real substance and must never become production material defaults.
        MaterialResponseCalibration::new(vec![
            MaterialResponseKnot {
                frequency_cycles_per_tick: 1.0,
                active_reaction: response(0.2, 0.3, 0.3, 0.2),
                inactive_reaction: response(0.0, 0.4, 0.35, 0.25),
            },
            MaterialResponseKnot {
                frequency_cycles_per_tick: 3.0,
                active_reaction: response(0.4, 0.2, 0.2, 0.2),
                inactive_reaction: response(0.0, 0.2, 0.5, 0.3),
            },
        ])
        .unwrap()
    }

    #[test]
    fn material_response_interpolates_each_frequency_channel_and_conserves() {
        let calibration = idealized_test_calibration();
        let active = calibration.response_at(2.0, true).unwrap();
        let expected_active = response(0.3, 0.25, 0.25, 0.2);
        assert!(
            (active.reaction_work_absorptance - expected_active.reaction_work_absorptance).abs()
                < 1.0e-12
        );
        assert!((active.reflectance - expected_active.reflectance).abs() < 1.0e-12);
        assert!((active.transmittance - expected_active.transmittance).abs() < 1.0e-12);
        assert!((active.scatterance - expected_active.scatterance).abs() < 1.0e-12);

        let inactive = calibration.response_at(2.0, false).unwrap();
        let expected_inactive = response(0.0, 0.3, 0.425, 0.275);
        assert!(
            (inactive.reaction_work_absorptance - expected_inactive.reaction_work_absorptance)
                .abs()
                < 1.0e-12
        );
        assert!((inactive.reflectance - expected_inactive.reflectance).abs() < 1.0e-12);
        assert!((inactive.transmittance - expected_inactive.transmittance).abs() < 1.0e-12);
        assert!((inactive.scatterance - expected_inactive.scatterance).abs() < 1.0e-12);
    }

    #[test]
    fn calibrated_partition_uses_distinct_active_and_inactive_response_curves() {
        let calibration = idealized_test_calibration();
        let components = [
            SpectralIncidentEnergy {
                frequency_cycles_per_tick: 1.0,
                incident_energy: 10.0,
            },
            SpectralIncidentEnergy {
                frequency_cycles_per_tick: 3.0,
                incident_energy: 10.0,
            },
        ];

        let active = partition_calibrated_incident_energy(&components, &calibration, true).unwrap();
        assert_eq!(active.incident, 20.0);
        assert_eq!(active.reaction_work, 6.0);
        assert_eq!(active.reflected, 5.0);
        assert_eq!(active.transmitted, 5.0);
        assert_eq!(active.scattered, 4.0);

        let inactive =
            partition_calibrated_incident_energy(&components, &calibration, false).unwrap();
        assert_eq!(inactive.reaction_work, 0.0);
        assert_eq!(inactive.reflected, 6.0);
        assert_eq!(inactive.transmitted, 8.5);
        assert_eq!(inactive.scattered, 5.5);
    }

    #[test]
    fn calibration_rejects_invalid_knots_and_does_not_extrapolate() {
        assert!(MaterialResponseCalibration::new(Vec::new()).is_none());
        assert!(MaterialResponseCalibration::new(vec![MaterialResponseKnot {
            frequency_cycles_per_tick: 2.0,
            active_reaction: response(0.25, 0.25, 0.25, 0.25),
            inactive_reaction: response(0.1, 0.3, 0.3, 0.3),
        },])
        .is_none());
        assert!(MaterialResponseCalibration::new(vec![
            MaterialResponseKnot {
                frequency_cycles_per_tick: 2.0,
                active_reaction: response(0.25, 0.25, 0.25, 0.25),
                inactive_reaction: response(0.0, 0.3, 0.3, 0.4),
            },
            MaterialResponseKnot {
                frequency_cycles_per_tick: 1.0,
                active_reaction: response(0.25, 0.25, 0.25, 0.25),
                inactive_reaction: response(0.0, 0.3, 0.3, 0.4),
            },
        ])
        .is_none());

        let calibration = idealized_test_calibration();
        assert!(calibration.response_at(0.99, true).is_none());
        assert!(calibration.response_at(3.01, false).is_none());
        assert!(calibration.response_at(f64::NAN, true).is_none());
        assert!(calibration.response_at(-0.0, true).is_none());
    }

    #[test]
    fn frequency_dependent_response_partitions_energy_without_creating_or_losing_it() {
        let components = [
            (
                SpectralIncidentEnergy {
                    frequency_cycles_per_tick: 440.0 / 24_000.0,
                    incident_energy: 8.0,
                },
                response(0.25, 0.25, 0.25, 0.25),
            ),
            (
                SpectralIncidentEnergy {
                    frequency_cycles_per_tick: 880.0 / 24_000.0,
                    incident_energy: 4.0,
                },
                response(0.5, 0.25, 0.125, 0.125),
            ),
        ];

        let partition = partition_chemical_boundary_energy(&components).unwrap();
        assert_eq!(partition.incident, 12.0);
        assert_eq!(partition.reaction_work, 4.0);
        assert_eq!(partition.reflected, 3.0);
        assert_eq!(partition.transmitted, 2.5);
        assert_eq!(partition.scattered, 2.5);
        assert_eq!(
            partition.reaction_work
                + partition.reflected
                + partition.transmitted
                + partition.scattered,
            partition.incident
        );
    }

    #[test]
    fn no_active_reaction_means_no_energy_is_absorbed_as_reaction_work() {
        let components = [(
            SpectralIncidentEnergy {
                frequency_cycles_per_tick: 0.1,
                incident_energy: 5.0,
            },
            response(0.0, 0.4, 0.3, 0.3),
        )];

        let partition = partition_chemical_boundary_energy(&components).unwrap();
        assert_eq!(partition.reaction_work, 0.0);
        assert_eq!(
            partition.reflected + partition.transmitted + partition.scattered,
            5.0
        );
    }

    #[test]
    fn invalid_response_or_incident_energy_fails_closed() {
        let incident = SpectralIncidentEnergy {
            frequency_cycles_per_tick: 0.1,
            incident_energy: 5.0,
        };
        assert!(
            partition_chemical_boundary_energy(&[(incident, response(0.5, 0.5, 0.5, 0.0))])
                .is_none()
        );
        assert!(partition_chemical_boundary_energy(&[(
            SpectralIncidentEnergy {
                incident_energy: f64::NAN,
                ..incident
            },
            response(0.25, 0.25, 0.25, 0.25)
        )])
        .is_none());
    }

    #[test]
    fn empty_interaction_has_zero_energy_partition() {
        assert_eq!(
            partition_chemical_boundary_energy(&[]),
            Some(ChemicalBoundaryEnergyPartition::default())
        );
    }
}
