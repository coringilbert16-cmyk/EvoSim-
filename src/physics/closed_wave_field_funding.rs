//! Staged adapter from the isolated closed wave field to chemistry's
//! environmental funding contract.
//!
//! This remains an isolated reference. It does not connect the field to the
//! production simulation tick or alter chemistry's structural-energy model.

use crate::closed_boundary_wave_solver::{
    ClosedBoundaryWaveField, ClosedWaveError, MatterEnergyAccount,
};
use crate::energy_ledger::{
    EnvironmentalEnergyProvider, EnvironmentalEnergyRequest, StagedEnvironmentalFunding,
};

/// A local field-energy source positioned at one interaction point.
///
/// Only one funding request may be pending at a time. Staging reserves this
/// source without changing its field state; the caller must then commit or
/// cancel before requesting another transfer.
pub(crate) struct ClosedWaveFieldFundingSource {
    field: ClosedBoundaryWaveField,
    position_x_l: f64,
    position_y_l: f64,
    pending: bool,
}

pub(crate) struct StagedClosedWaveField {
    field: ClosedBoundaryWaveField,
}

impl ClosedWaveFieldFundingSource {
    pub(crate) fn new(
        field: ClosedBoundaryWaveField,
        position_x_l: f64,
        position_y_l: f64,
    ) -> Result<Self, ClosedWaveError> {
        // Validate the interaction position once, before this source is used.
        field.query_world_at_current_time(position_x_l, position_y_l)?;
        Ok(Self {
            field,
            position_x_l,
            position_y_l,
            pending: false,
        })
    }

    pub(crate) fn field(&self) -> &ClosedBoundaryWaveField {
        &self.field
    }

    pub(crate) fn has_pending_funding(&self) -> bool {
        self.pending
    }
}

impl EnvironmentalEnergyProvider for ClosedWaveFieldFundingSource {
    type StagedSource = StagedClosedWaveField;

    fn stage_funding(
        &mut self,
        request: EnvironmentalEnergyRequest,
    ) -> Option<StagedEnvironmentalFunding<Self::StagedSource>> {
        if self.pending {
            return None;
        }

        // Stage against a private field copy. The live field remains unchanged
        // until commit_funding is called after the reaction is ready to commit.
        let mut staged_field = self.field.clone();
        let mut staged_receiver = MatterEnergyAccount::new(0.0).ok()?;
        staged_field
            .transfer_to_matter(
                self.position_x_l,
                self.position_y_l,
                request.required_external_input,
                &mut staged_receiver,
            )
            .ok()?;

        let funding = StagedEnvironmentalFunding::new(
            request,
            staged_receiver.stored_energy(),
            StagedClosedWaveField {
                field: staged_field,
            },
        )?;
        self.pending = true;
        Some(funding)
    }

    fn commit_funding(&mut self, staged_source: Self::StagedSource) {
        debug_assert!(self.pending, "field funding must be staged before commit");
        self.field = staged_source.field;
        self.pending = false;
    }

    fn cancel_funding(&mut self, _staged_source: Self::StagedSource) {
        debug_assert!(self.pending, "field funding must be staged before cancel");
        self.pending = false;
    }
}

/// One quadrature/time-bin contribution to a receiver's measured incident energy.
/// Positions are the actual boundary sample locations, not a centroid or proxy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BoundaryFundingSite {
    pub(crate) position_x_l: f64,
    pub(crate) position_y_l: f64,
    pub(crate) incident_energy: f64,
}

/// Multi-site staged funding for a complete exposed receiver boundary.
/// The caller supplies integrated incident energy at the same spatial
/// quadrature sites used by the geometry-backed flux measurement.
pub(crate) struct ClosedWaveBoundaryFundingSource {
    field: ClosedBoundaryWaveField,
    sites: Vec<BoundaryFundingSite>,
    incident_total: f64,
    pending: bool,
}

pub(crate) struct StagedClosedWaveBoundary {
    field: ClosedBoundaryWaveField,
}

impl ClosedWaveBoundaryFundingSource {
    pub(crate) fn from_measured_flux(
        field: ClosedBoundaryWaveField,
        measurement: crate::closed_wave_boundary_flux::BoundaryFluxEnergyMeasurement,
    ) -> Result<Self, ClosedWaveError> {
        Self::new(field, measurement.sites)
    }

    pub(crate) fn new(
        field: ClosedBoundaryWaveField,
        sites: Vec<BoundaryFundingSite>,
    ) -> Result<Self, ClosedWaveError> {
        if sites.is_empty() {
            return Err(ClosedWaveError::InvalidState);
        }
        let mut incident_total = 0.0;
        for (index, site) in sites.iter().enumerate() {
            if !site.position_x_l.is_finite()
                || !site.position_y_l.is_finite()
                || !site.incident_energy.is_finite()
                || site.incident_energy < 0.0
            {
                return Err(ClosedWaveError::InvalidState);
            }
            field.query_world_at_current_time(site.position_x_l, site.position_y_l)?;
            if sites[..index].iter().any(|previous| {
                previous.position_x_l == site.position_x_l
                    && previous.position_y_l == site.position_y_l
            }) {
                return Err(ClosedWaveError::InvalidState);
            }
            incident_total += site.incident_energy;
            if !incident_total.is_finite() {
                return Err(ClosedWaveError::InvalidState);
            }
        }
        if incident_total <= 0.0 {
            return Err(ClosedWaveError::InvalidState);
        }
        Ok(Self {
            field,
            sites,
            incident_total,
            pending: false,
        })
    }

    pub(crate) fn field(&self) -> &ClosedBoundaryWaveField {
        &self.field
    }

    pub(crate) fn has_pending_funding(&self) -> bool {
        self.pending
    }
}

impl EnvironmentalEnergyProvider for ClosedWaveBoundaryFundingSource {
    type StagedSource = StagedClosedWaveBoundary;

    fn stage_funding(
        &mut self,
        request: EnvironmentalEnergyRequest,
    ) -> Option<StagedEnvironmentalFunding<Self::StagedSource>> {
        let amount = request.required_external_input;
        if self.pending || !amount.is_finite() || amount < 0.0 || amount > self.incident_total {
            return None;
        }

        // All point-local withdrawals are applied to a private copy. Any
        // unavailable site aborts the whole stage and leaves the live field
        // untouched. The final site receives the rounding remainder so the
        // staged receiver total is exactly the requested amount.
        let mut staged_field = self.field.clone();
        let mut receiver = MatterEnergyAccount::new(0.0).ok()?;
        let mut allocated = 0.0;
        let positive_sites: Vec<_> = self
            .sites
            .iter()
            .filter(|site| site.incident_energy > 0.0)
            .collect();
        for (index, site) in positive_sites.iter().enumerate() {
            let share = if index + 1 == positive_sites.len() {
                amount - allocated
            } else {
                amount * site.incident_energy / self.incident_total
            };
            staged_field
                .transfer_to_matter(site.position_x_l, site.position_y_l, share, &mut receiver)
                .ok()?;
            allocated += share;
        }
        if (receiver.stored_energy() - amount).abs() > 1.0e-12 * amount.abs().max(1.0) {
            return None;
        }
        let funding = StagedEnvironmentalFunding::new(
            request,
            receiver.stored_energy(),
            StagedClosedWaveBoundary {
                field: staged_field,
            },
        )?;
        self.pending = true;
        Some(funding)
    }

    fn commit_funding(&mut self, staged_source: Self::StagedSource) {
        debug_assert!(
            self.pending,
            "boundary funding must be staged before commit"
        );
        self.field = staged_source.field;
        self.pending = false;
    }

    fn cancel_funding(&mut self, _staged_source: Self::StagedSource) {
        debug_assert!(
            self.pending,
            "boundary funding must be staged before cancel"
        );
        self.pending = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn energized_field() -> ClosedBoundaryWaveField {
        let mut field = ClosedBoundaryWaveField::new(9, 9, 1.0, 0.2, 2, 2.0).unwrap();
        for y in 0..9 {
            for x in 0..9 {
                let radius_squared = (x as f64 - 4.0).powi(2) + (y as f64 - 4.0).powi(2);
                let current = (-radius_squared / 8.0).exp();
                let previous = current - 0.1 * current;
                field.set_initial_state(x, y, previous, current).unwrap();
            }
        }
        field
    }

    #[test]
    fn staging_preserves_live_field_until_commit_then_debits_exact_amount() {
        let field = energized_field();
        let initial_energy = field.energy();
        let mut source = ClosedWaveFieldFundingSource::new(field, 4.0, 4.0).unwrap();
        let request = EnvironmentalEnergyRequest::new(0.01).unwrap();

        let staged = source.stage_funding(request).unwrap();
        assert!(staged.satisfies_request());
        assert_eq!(staged.transferred, 0.01);
        assert_eq!(source.field().energy(), initial_energy);
        assert!(source.has_pending_funding());

        source.commit_funding(staged.staged_source);
        assert!(!source.has_pending_funding());
        let debit = initial_energy - source.field().energy();
        assert!((debit - 0.01).abs() <= 2.0e-10);
    }

    #[test]
    fn pending_stage_prevents_duplicate_claims_and_cancel_leaves_field_unchanged() {
        let field = energized_field();
        let initial_energy = field.energy();
        let mut source = ClosedWaveFieldFundingSource::new(field, 4.0, 4.0).unwrap();
        let request = EnvironmentalEnergyRequest::new(0.01).unwrap();

        let staged = source.stage_funding(request).unwrap();
        assert!(source.stage_funding(request).is_none());
        source.cancel_funding(staged.staged_source);

        assert!(!source.has_pending_funding());
        assert_eq!(source.field().energy(), initial_energy);
        let retry = source.stage_funding(request).unwrap();
        assert!(retry.satisfies_request());
        source.commit_funding(retry.staged_source);
    }

    #[test]
    fn unavailable_local_field_energy_does_not_reserve_or_change_source() {
        let field = energized_field();
        let initial_energy = field.energy();
        let mut source = ClosedWaveFieldFundingSource::new(field, 4.0, 4.0).unwrap();
        let request = EnvironmentalEnergyRequest::new(1.0e6).unwrap();

        assert!(source.stage_funding(request).is_none());
        assert!(!source.has_pending_funding());
        assert_eq!(source.field().energy(), initial_energy);

        let retry = source
            .stage_funding(EnvironmentalEnergyRequest::new(0.01).unwrap())
            .unwrap();
        source.commit_funding(retry.staged_source);
    }
    #[test]
    fn multi_site_boundary_stage_debits_measured_energy_proportionally_and_atomically() {
        let field = energized_field();
        let initial_energy = field.energy();
        let mut source = ClosedWaveBoundaryFundingSource::new(
            field,
            vec![
                BoundaryFundingSite {
                    position_x_l: 3.5,
                    position_y_l: 4.0,
                    incident_energy: 2.0,
                },
                BoundaryFundingSite {
                    position_x_l: 4.5,
                    position_y_l: 4.0,
                    incident_energy: 1.0,
                },
            ],
        )
        .unwrap();
        let request = EnvironmentalEnergyRequest::new(0.006).unwrap();

        let staged = source.stage_funding(request).unwrap();
        assert!(staged.satisfies_request());
        assert!((staged.transferred - 0.006).abs() < 1.0e-12);
        assert_eq!(source.field().energy(), initial_energy);
        assert!(source.has_pending_funding());

        source.commit_funding(staged.staged_source);
        assert!(!source.has_pending_funding());
        assert!((initial_energy - source.field().energy() - 0.006).abs() < 2.0e-10);
    }

    #[test]
    fn multi_site_boundary_stage_cancel_and_invalid_sites_leave_live_field_unchanged() {
        let field = energized_field();
        let initial_energy = field.energy();
        let sites = vec![
            BoundaryFundingSite {
                position_x_l: 3.5,
                position_y_l: 4.0,
                incident_energy: 1.0,
            },
            BoundaryFundingSite {
                position_x_l: 4.5,
                position_y_l: 4.0,
                incident_energy: 1.0,
            },
        ];
        let mut source =
            ClosedWaveBoundaryFundingSource::new(field.clone(), sites.clone()).unwrap();
        let staged = source
            .stage_funding(EnvironmentalEnergyRequest::new(0.004).unwrap())
            .unwrap();
        assert!(source
            .stage_funding(EnvironmentalEnergyRequest::new(0.001).unwrap())
            .is_none());
        source.cancel_funding(staged.staged_source);
        assert_eq!(source.field().energy(), initial_energy);
        assert!(!source.has_pending_funding());

        let duplicate = vec![sites[0], sites[0]];
        assert_eq!(
            ClosedWaveBoundaryFundingSource::new(field.clone(), duplicate).err(),
            Some(ClosedWaveError::InvalidState)
        );
        let unavailable = vec![
            BoundaryFundingSite {
                position_x_l: 3.5,
                position_y_l: 4.0,
                incident_energy: 1.0,
            },
            BoundaryFundingSite {
                position_x_l: 100.0,
                position_y_l: 4.0,
                incident_energy: 1.0,
            },
        ];
        assert_eq!(
            ClosedWaveBoundaryFundingSource::new(field, unavailable).err(),
            Some(ClosedWaveError::InvalidState)
        );
    }
}
