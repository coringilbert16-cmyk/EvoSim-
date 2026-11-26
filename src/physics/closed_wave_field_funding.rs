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

struct StagedClosedWaveField {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn energized_field() -> ClosedBoundaryWaveField {
        let mut field = ClosedBoundaryWaveField::new(9, 9, 1.0, 0.2, 2, 2.0).unwrap();
        for y in 0..9 {
            for x in 0..9 {
                let radius_squared =
                    (x as f64 - 4.0).powi(2) + (y as f64 - 4.0).powi(2);
                let current = (-radius_squared / 8.0).exp();
                let previous = current - 0.1 * current;
                field
                    .set_initial_state(x, y, previous, current)
                    .unwrap();
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
}
