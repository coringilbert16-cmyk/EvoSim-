//! Source identity and finite energy-accounting contracts for the isolated field work.
//!
//! This module is not connected to live simulation sources or the production
//! energy ledger. It provides the source descriptor and the single finite
//! source account used by the isolated reference solver.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct SourceId(u64);

impl SourceId {
    pub(crate) fn new(value: u64) -> Result<Self, SourceAccountError> {
        if value == 0 {
            return Err(SourceAccountError::InvalidIdentity);
        }
        Ok(Self(value))
    }

    pub(crate) fn value(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SourceProvenance {
    External { origin_id: u64 },
    Internal { owner_id: u64 },
    MathematicalTest { holder_id: u64 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SourceClass {
    MathematicalTest,
    InPlaneDistributed,
    ExternalRadiative,
    BoundaryEmitter,
    MaterialOscillator,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum SourceGeometry {
    Point { x: f64, y: f64 },
    Disk { center_x: f64, center_y: f64, radius: f64 },
    LineSegment { start_x: f64, start_y: f64, end_x: f64, end_y: f64 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TickSchedule {
    start_tick: u64,
    end_tick_exclusive: Option<u64>,
}

impl TickSchedule {
    pub(crate) fn new(
        start_tick: u64,
        end_tick_exclusive: Option<u64>,
    ) -> Result<Self, SourceAccountError> {
        if end_tick_exclusive.is_some_and(|end| end <= start_tick) {
            return Err(SourceAccountError::InvalidDescriptor);
        }
        Ok(Self { start_tick, end_tick_exclusive })
    }

    pub(crate) fn is_active(self, tick: u64) -> bool {
        tick >= self.start_tick
            && self.end_tick_exclusive.is_none_or(|end| tick < end)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum SourceWaveform {
    SinglePulse,
    Periodic { frequency_cycles_per_tick: f64, phase_cycles: f64 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum SourcePowerLaw {
    MathematicalTestOnly,
    TotalPowerPerTick(f64),
    AreaPowerDensityPerTick(f64),
    LinePowerDensityPerTick(f64),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SourceDescriptor {
    id: SourceId,
    provenance: SourceProvenance,
    class: SourceClass,
    geometry: SourceGeometry,
    schedule: TickSchedule,
    waveform: SourceWaveform,
    power_law: SourcePowerLaw,
}

impl SourceDescriptor {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        id: SourceId,
        provenance: SourceProvenance,
        class: SourceClass,
        geometry: SourceGeometry,
        schedule: TickSchedule,
        waveform: SourceWaveform,
        power_law: SourcePowerLaw,
    ) -> Result<Self, SourceAccountError> {
        let valid_provenance = match (class, provenance) {
            (SourceClass::MathematicalTest, SourceProvenance::MathematicalTest { holder_id }) => holder_id > 0,
            (SourceClass::InPlaneDistributed | SourceClass::ExternalRadiative, SourceProvenance::External { origin_id }) => origin_id > 0,
            (SourceClass::BoundaryEmitter, SourceProvenance::External { origin_id }) => origin_id > 0,
            (SourceClass::MaterialOscillator, SourceProvenance::Internal { owner_id }) => owner_id > 0,
            _ => false,
        };
        let valid_geometry = match (class, geometry) {
            (SourceClass::MathematicalTest | SourceClass::InPlaneDistributed, SourceGeometry::Disk { .. }) => true,
            (SourceClass::ExternalRadiative, SourceGeometry::Point { .. }) => true,
            (SourceClass::BoundaryEmitter, SourceGeometry::LineSegment { .. }) => true,
            (SourceClass::MaterialOscillator, SourceGeometry::Point { .. } | SourceGeometry::Disk { .. }) => true,
            _ => false,
        };
        if !valid_provenance || !valid_geometry || !valid_geometry_values(geometry) || !valid_waveform(waveform) {
            return Err(SourceAccountError::InvalidDescriptor);
        }

        let valid_power_law = match (class, power_law) {
            (SourceClass::MathematicalTest, SourcePowerLaw::MathematicalTestOnly) => true,
            (SourceClass::InPlaneDistributed, SourcePowerLaw::AreaPowerDensityPerTick(value)) => value.is_finite() && value > 0.0,
            (SourceClass::ExternalRadiative | SourceClass::MaterialOscillator, SourcePowerLaw::TotalPowerPerTick(value)) => value.is_finite() && value > 0.0,
            (SourceClass::BoundaryEmitter, SourcePowerLaw::LinePowerDensityPerTick(value)) => value.is_finite() && value > 0.0,
            _ => false,
        };
        if !valid_power_law {
            return Err(SourceAccountError::InvalidDescriptor);
        }

        Ok(Self { id, provenance, class, geometry, schedule, waveform, power_law })
    }

    pub(crate) fn id(self) -> SourceId { self.id }
    pub(crate) fn provenance(self) -> SourceProvenance { self.provenance }
    pub(crate) fn class(self) -> SourceClass { self.class }
    pub(crate) fn geometry(self) -> SourceGeometry { self.geometry }
    pub(crate) fn schedule(self) -> TickSchedule { self.schedule }
    pub(crate) fn waveform(self) -> SourceWaveform { self.waveform }
    pub(crate) fn power_law(self) -> SourcePowerLaw { self.power_law }

    pub(crate) fn validate_account(
        self,
        account: &SourceEnergyAccount,
    ) -> Result<(), SourceAccountError> {
        if self.id != account.source_id {
            return Err(SourceAccountError::IdentityMismatch);
        }
        Ok(())
    }
}

fn valid_geometry_values(geometry: SourceGeometry) -> bool {
    match geometry {
        SourceGeometry::Point { x, y } => x.is_finite() && y.is_finite(),
        SourceGeometry::Disk { center_x, center_y, radius } => {
            center_x.is_finite() && center_y.is_finite() && radius.is_finite() && radius > 0.0
        }
        SourceGeometry::LineSegment { start_x, start_y, end_x, end_y } => {
            start_x.is_finite()
                && start_y.is_finite()
                && end_x.is_finite()
                && end_y.is_finite()
                && (start_x != end_x || start_y != end_y)
        }
    }
}

fn valid_waveform(waveform: SourceWaveform) -> bool {
    match waveform {
        SourceWaveform::SinglePulse => true,
        SourceWaveform::Periodic { frequency_cycles_per_tick, phase_cycles } => {
            frequency_cycles_per_tick.is_finite()
                && frequency_cycles_per_tick > 0.0
                && phase_cycles.is_finite()
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SourceAccountError {
    InvalidIdentity,
    InvalidDescriptor,
    InvalidEnergy,
    InvalidTransfer,
    InsufficientEnergy,
    IdentityMismatch,
}

#[derive(Debug)]
pub(crate) struct SourceEnergyAccount {
    source_id: SourceId,
    initial_energy: f64,
    remaining_energy: f64,
    cumulative_field_transfer: f64,
    cumulative_replenishment: f64,
}

#[derive(Debug)]
pub(crate) struct PreparedFieldSettlement {
    source_id: SourceId,
    remaining_energy: f64,
    cumulative_field_transfer: f64,
}

/// Owns one immutable descriptor and its one mutable, non-clonable energy account.
#[derive(Debug)]
pub(crate) struct SourceInstance {
    descriptor: SourceDescriptor,
    account: SourceEnergyAccount,
}

impl SourceInstance {
    pub(crate) fn new(
        descriptor: SourceDescriptor,
        account: SourceEnergyAccount,
    ) -> Result<Self, SourceAccountError> {
        descriptor.validate_account(&account)?;
        Ok(Self { descriptor, account })
    }

    pub(crate) fn descriptor(&self) -> &SourceDescriptor {
        &self.descriptor
    }

    pub(crate) fn account(&self) -> &SourceEnergyAccount {
        &self.account
    }

    pub(crate) fn account_mut(&mut self) -> &mut SourceEnergyAccount {
        &mut self.account
    }
}

impl SourceEnergyAccount {
    pub(crate) fn new(
        source_id: SourceId,
        initial_energy: f64,
    ) -> Result<Self, SourceAccountError> {
        if !initial_energy.is_finite() || initial_energy < 0.0 {
            return Err(SourceAccountError::InvalidEnergy);
        }
        Ok(Self {
            source_id,
            initial_energy,
            remaining_energy: initial_energy,
            cumulative_field_transfer: 0.0,
            cumulative_replenishment: 0.0,
        })
    }

    pub(crate) fn source_id(&self) -> SourceId { self.source_id }
    pub(crate) fn initial_energy(&self) -> f64 { self.initial_energy }
    pub(crate) fn remaining_energy(&self) -> f64 { self.remaining_energy }
    pub(crate) fn cumulative_field_transfer(&self) -> f64 { self.cumulative_field_transfer }
    pub(crate) fn cumulative_replenishment(&self) -> f64 { self.cumulative_replenishment }

    /// Validates and stages a signed field transfer without changing the account.
    /// Positive work debits the source; negative work returns energy to it.
    pub(crate) fn prepare_field_transfer(
        &self,
        signed_transfer: f64,
    ) -> Result<PreparedFieldSettlement, SourceAccountError> {
        if !signed_transfer.is_finite() {
            return Err(SourceAccountError::InvalidTransfer);
        }
        let remaining_energy = self.remaining_energy - signed_transfer;
        let cumulative_field_transfer = self.cumulative_field_transfer + signed_transfer;
        if !remaining_energy.is_finite() || !cumulative_field_transfer.is_finite() {
            return Err(SourceAccountError::InvalidTransfer);
        }
        if remaining_energy < 0.0 {
            return Err(SourceAccountError::InsufficientEnergy);
        }
        Ok(PreparedFieldSettlement {
            source_id: self.source_id,
            remaining_energy,
            cumulative_field_transfer,
        })
    }

    /// Commits a previously validated settlement. No fallible work remains.
    pub(crate) fn commit_field_transfer(
        &mut self,
        settlement: PreparedFieldSettlement,
    ) -> Result<(), SourceAccountError> {
        if self.source_id != settlement.source_id {
            return Err(SourceAccountError::IdentityMismatch);
        }
        self.remaining_energy = settlement.remaining_energy;
        self.cumulative_field_transfer = settlement.cumulative_field_transfer;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn descriptor(id: u64) -> SourceDescriptor {
        SourceDescriptor::new(
            SourceId::new(id).unwrap(),
            SourceProvenance::MathematicalTest { holder_id: 7 },
            SourceClass::MathematicalTest,
            SourceGeometry::Disk { center_x: 2.0, center_y: 3.0, radius: 1.0 },
            TickSchedule::new(4, Some(9)).unwrap(),
            SourceWaveform::SinglePulse,
            SourcePowerLaw::MathematicalTestOnly,
        ).unwrap()
    }

    #[test]
    fn source_descriptor_preserves_identity_provenance_and_deterministic_schedule() {
        let source = descriptor(11);
        assert_eq!(source.id().value(), 11);
        assert_eq!(source.provenance(), SourceProvenance::MathematicalTest { holder_id: 7 });
        assert!(source.schedule().is_active(4));
        assert!(source.schedule().is_active(8));
        assert!(!source.schedule().is_active(9));
    }

    #[test]
    fn descriptor_rejects_invalid_geometry_power_and_provenance() {
        let id = SourceId::new(1).unwrap();
        let schedule = TickSchedule::new(0, None).unwrap();
        assert_eq!(
            SourceDescriptor::new(
                id, SourceProvenance::MathematicalTest { holder_id: 1 },
                SourceClass::MathematicalTest,
                SourceGeometry::Disk { center_x: 0.0, center_y: 0.0, radius: 0.0 },
                schedule, SourceWaveform::SinglePulse, SourcePowerLaw::MathematicalTestOnly
            ),
            Err(SourceAccountError::InvalidDescriptor)
        );
        assert_eq!(
            SourceDescriptor::new(
                id, SourceProvenance::External { origin_id: 1 },
                SourceClass::MathematicalTest,
                SourceGeometry::Point { x: 0.0, y: 0.0 },
                schedule, SourceWaveform::SinglePulse, SourcePowerLaw::MathematicalTestOnly
            ),
            Err(SourceAccountError::InvalidDescriptor)
        );
        assert_eq!(
            TickSchedule::new(5, Some(5)),
            Err(SourceAccountError::InvalidDescriptor)
        );
    }

    #[test]
    fn source_account_settles_positive_and_negative_work() {
        let mut account = SourceEnergyAccount::new(SourceId::new(2).unwrap(), 10.0).unwrap();
        let debit = account.prepare_field_transfer(3.5).unwrap();
        account.commit_field_transfer(debit).unwrap();
        assert_eq!(account.remaining_energy(), 6.5);
        assert_eq!(account.cumulative_field_transfer(), 3.5);

        let return_transfer = account.prepare_field_transfer(-1.25).unwrap();
        account.commit_field_transfer(return_transfer).unwrap();
        assert_eq!(account.remaining_energy(), 7.75);
        assert_eq!(account.cumulative_field_transfer(), 2.25);
    }

    #[test]
    fn source_account_rejects_overdraw_without_mutation() {
        let mut account = SourceEnergyAccount::new(SourceId::new(3).unwrap(), 1.0).unwrap();
        assert_eq!(
            account.prepare_field_transfer(1.5),
            Err(SourceAccountError::InsufficientEnergy)
        );
        assert_eq!(account.remaining_energy(), 1.0);
        assert_eq!(account.cumulative_field_transfer(), 0.0);
    }

    #[test]
    fn descriptor_and_account_must_share_one_source_identity() {
        let source = descriptor(4);
        let account = SourceEnergyAccount::new(SourceId::new(5).unwrap(), 2.0).unwrap();
        assert_eq!(source.validate_account(&account), Err(SourceAccountError::IdentityMismatch));
    }
}
