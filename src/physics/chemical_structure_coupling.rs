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
        values.iter().all(|value| value.is_finite() && (0.0..=1.0).contains(value))
            && (values.iter().sum::<f64>() - 1.0).abs() <= 1.0e-12
    }
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
        let next_work =
            result.reaction_work + incident * response.reaction_work_absorptance;
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

    let accounted =
        result.reaction_work + result.reflected + result.transmitted + result.scattered;
    (accounted.is_finite() && (accounted - result.incident).abs() <= 1.0e-10)
        .then_some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(work: f64, reflected: f64, transmitted: f64, scattered: f64) -> ChemicalBoundaryResponse {
        ChemicalBoundaryResponse {
            reaction_work_absorptance: work,
            reflectance: reflected,
            transmittance: transmitted,
            scatterance: scattered,
        }
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
        assert_eq!(partition.reflected + partition.transmitted + partition.scattered, 5.0);
    }

    #[test]
    fn invalid_response_or_incident_energy_fails_closed() {
        let incident = SpectralIncidentEnergy {
            frequency_cycles_per_tick: 0.1,
            incident_energy: 5.0,
        };
        assert!(partition_chemical_boundary_energy(&[(
            incident,
            response(0.5, 0.5, 0.5, 0.0)
        )])
        .is_none());
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
