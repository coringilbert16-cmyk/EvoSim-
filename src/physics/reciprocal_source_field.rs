//! Isolated Candidate B: a reciprocal source oscillator coupled to the wave field.
//!
//! This is a closed mathematical comparator, not a physical hotspot/sun model.
//! The oscillator exchanges energy with the field through an explicit coupling
//! potential. It is intentionally disconnected from live sources, materials,
//! chemistry, movement, and the production energy ledger.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReciprocalModelError {
    InvalidDimensions,
    InvalidParameter,
    InvalidProfile,
    UnstableTimeStep,
    InvalidState,
    InsufficientSourceEnergy,
    SourceAccountRejected,
    CounterOverflow,
}

#[derive(Clone, Debug)]
pub(crate) struct ReciprocalSourceField {
    width: usize,
    height: usize,
    spacing: f64,
    propagation_speed: f64,
    time_step: f64,
    field_normalization: f64,
    source_mass: f64,
    source_frequency: f64,
    coupling_strength: f64,
    profile: Vec<f64>,
    previous_field: Vec<f64>,
    current_field: Vec<f64>,
    previous_source: f64,
    current_source: f64,
    steps_elapsed: u64,
}

impl ReciprocalSourceField {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        width: usize,
        height: usize,
        spacing: f64,
        propagation_speed: f64,
        time_step: f64,
        field_normalization: f64,
        source_mass: f64,
        source_frequency: f64,
        coupling_strength: f64,
        normalized_profile: Vec<f64>,
        initial_source_displacement: f64,
    ) -> Result<Self, ReciprocalModelError> {
        if width < 3 || height < 3 {
            return Err(ReciprocalModelError::InvalidDimensions);
        }
        if !spacing.is_finite()
            || spacing <= 0.0
            || !propagation_speed.is_finite()
            || propagation_speed <= 0.0
            || !time_step.is_finite()
            || time_step <= 0.0
            || !field_normalization.is_finite()
            || field_normalization <= 0.0
            || !source_mass.is_finite()
            || source_mass <= 0.0
            || !source_frequency.is_finite()
            || source_frequency <= 0.0
            || !coupling_strength.is_finite()
            || coupling_strength < 0.0
            || !initial_source_displacement.is_finite()
        {
            return Err(ReciprocalModelError::InvalidParameter);
        }

        let cell_count = width
            .checked_mul(height)
            .ok_or(ReciprocalModelError::InvalidDimensions)?;
        if normalized_profile.len() != cell_count
            || normalized_profile
                .iter()
                .any(|value| !value.is_finite() || *value < 0.0)
        {
            return Err(ReciprocalModelError::InvalidProfile);
        }

        for y in 0..height {
            for x in 0..width {
                if (x == 0 || y == 0 || x + 1 == width || y + 1 == height)
                    && normalized_profile[y * width + x] != 0.0
                {
                    return Err(ReciprocalModelError::InvalidProfile);
                }
            }
        }

        let area = spacing.powi(2);
        let profile_integral: f64 = normalized_profile.iter().sum::<f64>() * area;
        if !profile_integral.is_finite() || (profile_integral - 1.0).abs() > 1.0e-10 {
            return Err(ReciprocalModelError::InvalidProfile);
        }

        // A conservative Gershgorin-style bound for the largest eigenvalue of
        // the mass-normalized coupled stiffness operator. Central differences
        // require dt^2 * lambda_max < 4 for linear stability.
        let profile_norm = normalized_profile
            .iter()
            .map(|value| value.powi(2))
            .sum::<f64>();
        let maximum_eigenvalue_bound = 8.0 * propagation_speed.powi(2) / area
            + source_frequency.powi(2)
            + coupling_strength * (1.0 / source_mass + area * profile_norm / field_normalization);
        let stability_number = time_step.powi(2) * maximum_eigenvalue_bound;
        if !maximum_eigenvalue_bound.is_finite()
            || !stability_number.is_finite()
            || stability_number >= 4.0
        {
            return Err(ReciprocalModelError::UnstableTimeStep);
        }

        Ok(Self {
            width,
            height,
            spacing,
            propagation_speed,
            time_step,
            field_normalization,
            source_mass,
            source_frequency,
            coupling_strength,
            profile: normalized_profile,
            previous_field: vec![0.0; cell_count],
            current_field: vec![0.0; cell_count],
            previous_source: initial_source_displacement,
            current_source: initial_source_displacement,
            steps_elapsed: 0,
        })
    }

    pub(crate) fn steps_elapsed(&self) -> u64 {
        self.steps_elapsed
    }

    pub(crate) fn field_sample(&self, x: usize, y: usize) -> Option<f64> {
        if x >= self.width || y >= self.height {
            return None;
        }
        Some(self.current_field[y * self.width + x])
    }

    pub(crate) fn source_displacement(&self) -> f64 {
        self.current_source
    }

    pub(crate) fn field_energy(&self) -> f64 {
        self.field_energy_components().0
    }

    pub(crate) fn source_energy(&self) -> f64 {
        self.field_energy_components().1
    }

    pub(crate) fn coupling_energy(&self) -> f64 {
        self.field_energy_components().2
    }

    /// Cross-time discrete Hamiltonian for the centered coupled recurrence.
    /// The coupling potential is included explicitly, not hidden in either
    /// the field or oscillator inventory.
    pub(crate) fn total_energy(&self) -> f64 {
        let (field, source, coupling) = self.field_energy_components();
        field + source + coupling
    }

    pub(crate) fn step(&mut self) -> Result<(), ReciprocalModelError> {
        self.step_with_external_force(0.0).map(|_| ())
    }

    /// Advances the reciprocal source/field system with an explicit external
    /// generalized force applied to the source oscillator. Returns the exact
    /// discrete external work for this centered-difference step.
    ///
    /// The environmental account must settle this signed work atomically with
    /// the state update. This method does not own or replenish that account.
    pub(crate) fn step_with_external_force(
        &mut self,
        external_force: f64,
    ) -> Result<f64, ReciprocalModelError> {
        if !external_force.is_finite() {
            return Err(ReciprocalModelError::InvalidParameter);
        }
        let old_previous_source = self.previous_source;
        let area = self.spacing.powi(2);
        let interaction_now = self.source_average(&self.current_field);
        let source_gap = self.current_source - interaction_now;
        let source_acceleration = -self.source_frequency.powi(2) * self.current_source
            - self.coupling_strength / self.source_mass * source_gap
            + external_force / self.source_mass;

        let mut next_field = self.current_field.clone();
        for y in 1..self.height - 1 {
            for x in 1..self.width - 1 {
                let index = self.index(x, y);
                let laplacian = (self.current_field[index - 1]
                    + self.current_field[index + 1]
                    + self.current_field[index - self.width]
                    + self.current_field[index + self.width]
                    - 4.0 * self.current_field[index])
                    / area;
                let coupling_acceleration = self.coupling_strength / self.field_normalization
                    * source_gap
                    * self.profile[index];
                let acceleration =
                    self.propagation_speed.powi(2) * laplacian + coupling_acceleration;
                next_field[index] = 2.0 * self.current_field[index] - self.previous_field[index]
                    + self.time_step.powi(2) * acceleration;
            }
        }

        let next_source = 2.0 * self.current_source - self.previous_source
            + self.time_step.powi(2) * source_acceleration;
        if !next_source.is_finite() || next_field.iter().any(|value| !value.is_finite()) {
            return Err(ReciprocalModelError::InvalidState);
        }
        let next_step = self
            .steps_elapsed
            .checked_add(1)
            .ok_or(ReciprocalModelError::CounterOverflow)?;

        let discrete_external_work = 0.5 * external_force * (next_source - old_previous_source);
        if !discrete_external_work.is_finite() {
            return Err(ReciprocalModelError::InvalidState);
        }

        self.previous_field = std::mem::replace(&mut self.current_field, next_field);
        self.previous_source = self.current_source;
        self.current_source = next_source;
        self.steps_elapsed = next_step;
        Ok(discrete_external_work)
    }

    /// Atomically settles externally driven oscillator work against one finite
    /// environmental source account. The trial state is computed separately;
    /// an invalid or underfunded transfer leaves both the model and account
    /// unchanged. This clone-based staging is intentionally isolated and is
    /// not a production performance implementation.
    pub(crate) fn step_with_environmental_source(
        &mut self,
        source: &mut SourceInstance,
        external_force: f64,
    ) -> Result<f64, ReciprocalModelError> {
        let mut trial = self.clone();
        let signed_work = trial.step_with_external_force(external_force)?;
        let settlement = source
            .account()
            .prepare_field_transfer(signed_work)
            .map_err(|error| match error {
                SourceAccountError::InsufficientEnergy => {
                    ReciprocalModelError::InsufficientSourceEnergy
                }
                _ => ReciprocalModelError::SourceAccountRejected,
            })?;

        source
            .account_mut()
            .commit_field_transfer(settlement)
            .map_err(|_| ReciprocalModelError::SourceAccountRejected)?;
        *self = trial;
        Ok(signed_work)
    }

    fn field_energy_components(&self) -> (f64, f64, f64) {
        let area = self.spacing.powi(2);
        let mut field_kinetic = 0.0;
        for index in 0..self.current_field.len() {
            let velocity =
                (self.current_field[index] - self.previous_field[index]) / self.time_step;
            field_kinetic += velocity.powi(2);
        }

        let mut gradient_cross = 0.0;
        for y in 0..self.height {
            for x in 0..self.width {
                let index = self.index(x, y);
                if x + 1 < self.width {
                    let current = self.current_field[index] - self.current_field[index + 1];
                    let previous = self.previous_field[index] - self.previous_field[index + 1];
                    gradient_cross += current * previous;
                }
                if y + 1 < self.height {
                    let current =
                        self.current_field[index] - self.current_field[index + self.width];
                    let previous =
                        self.previous_field[index] - self.previous_field[index + self.width];
                    gradient_cross += current * previous;
                }
            }
        }

        let field = 0.5
            * self.field_normalization
            * (area * field_kinetic + self.propagation_speed.powi(2) * gradient_cross);
        let source_velocity = (self.current_source - self.previous_source) / self.time_step;
        let source = 0.5
            * self.source_mass
            * (source_velocity.powi(2)
                + self.source_frequency.powi(2) * self.current_source * self.previous_source);
        let current_gap = self.current_source - self.source_average(&self.current_field);
        let previous_gap = self.previous_source - self.source_average(&self.previous_field);
        let coupling = 0.5 * self.coupling_strength * current_gap * previous_gap;
        (field, source, coupling)
    }

    fn source_average(&self, field: &[f64]) -> f64 {
        self.spacing.powi(2)
            * self
                .profile
                .iter()
                .zip(field)
                .map(|(weight, value)| weight * value)
                .sum::<f64>()
    }

    fn index(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn normalized_disk_profile(width: usize, height: usize, spacing: f64) -> Vec<f64> {
        let mut profile = vec![0.0; width * height];
        let center_x = width / 2;
        let center_y = height / 2;
        let mut indices = Vec::new();
        for y in center_y - 1..=center_y + 1 {
            for x in center_x - 1..=center_x + 1 {
                indices.push(y * width + x);
            }
        }
        let value = 1.0 / (indices.len() as f64 * spacing.powi(2));
        for index in indices {
            profile[index] = value;
        }
        profile
    }

    fn normalized_gaussian_profile(width: usize, height: usize, spacing: f64) -> Vec<f64> {
        let center = (width as f64 - 1.0) * spacing * 0.5;
        let sigma = 0.25_f64;
        let mut profile = Vec::with_capacity(width * height);
        for y in 0..height {
            for x in 0..width {
                let px = x as f64 * spacing;
                let py = y as f64 * spacing;
                let radius_squared = (px - center).powi(2) + (py - center).powi(2);
                profile.push((-radius_squared / (2.0 * sigma.powi(2))).exp());
            }
        }
        for y in 0..height {
            for x in 0..width {
                if x == 0 || y == 0 || x + 1 == width || y + 1 == height {
                    profile[y * width + x] = 0.0;
                }
            }
        }
        let integral = profile.iter().sum::<f64>() * spacing.powi(2);
        for value in &mut profile {
            *value /= integral;
        }
        profile
    }

    fn refined_model(cells: usize, spacing: f64, time_step: f64) -> ReciprocalSourceField {
        ReciprocalSourceField::new(
            cells,
            cells,
            spacing,
            1.0,
            time_step,
            1.0,
            2.0,
            1.5,
            0.2,
            normalized_gaussian_profile(cells, cells, spacing),
            1.0,
        )
        .unwrap()
    }

    fn model(frequency: f64, coupling: f64, time_step: f64) -> ReciprocalSourceField {
        let width = 15;
        let height = 15;
        let spacing = 0.2;
        ReciprocalSourceField::new(
            width,
            height,
            spacing,
            1.0,
            time_step,
            1.0,
            2.0,
            frequency,
            coupling,
            normalized_disk_profile(width, height, spacing),
            1.0,
        )
        .unwrap()
    }

    #[test]
    fn reciprocal_source_field_conserves_total_energy_over_many_periods() {
        let mut system = model(1.5, 0.2, 0.005);
        let initial_energy = system.total_energy();
        for _ in 0..20_000 {
            system.step().unwrap();
        }
        let final_energy = system.total_energy();
        let scale = initial_energy.abs().max(final_energy.abs()).max(1.0);
        assert!(
            (final_energy - initial_energy).abs() <= 2.0e-9 * scale,
            "reciprocal total-energy residual: {}",
            final_energy - initial_energy
        );
        assert!(system.field_energy() > 1.0e-6);
        assert!(system.coupling_energy().abs() > 1.0e-8);
        assert_eq!(system.steps_elapsed(), 20_000);
    }

    #[test]
    fn zero_coupling_keeps_field_and_source_independent() {
        let mut system = model(1.5, 0.0, 0.005);
        let initial_source_energy = system.source_energy();
        for _ in 0..2_000 {
            system.step().unwrap();
        }
        assert_eq!(system.field_energy(), 0.0);
        assert!((system.source_energy() - initial_source_energy).abs() < 1.0e-10);
        assert_eq!(system.coupling_energy(), 0.0);
    }

    #[test]
    fn reciprocal_model_rejects_unstable_time_step() {
        let width = 15;
        let height = 15;
        let spacing = 0.2;
        let result = ReciprocalSourceField::new(
            width,
            height,
            spacing,
            1.0,
            1.0,
            1.0,
            2.0,
            1.5,
            0.2,
            normalized_disk_profile(width, height, spacing),
            1.0,
        );
        assert_eq!(result.err(), Some(ReciprocalModelError::UnstableTimeStep));
    }

    #[test]
    fn externally_driven_reciprocal_model_reconciles_discrete_work() {
        let mut system = model(1.5, 0.2, 0.005);
        let initial_total = system.total_energy();
        let mut cumulative_work = 0.0;
        for step in 0..4_000 {
            let phase = step as f64 * 0.005 * 0.7;
            let force = 0.08 * phase.sin();
            cumulative_work += system.step_with_external_force(force).unwrap();
            let residual = system.total_energy() - initial_total - cumulative_work;
            assert!(
                residual.abs() <= 2.0e-9 * initial_total.abs().max(1.0),
                "external-drive energy residual at step {step}: {residual}"
            );
        }
        assert!(cumulative_work.is_finite());
        assert!(system.field_energy() > 0.0);
    }

    fn mathematical_source(id: u64, initial_energy: f64) -> SourceInstance {
        use super::super::source::{
            SourceClass, SourceDescriptor, SourceEnergyAccount, SourceGeometry, SourceId,
            SourcePowerLaw, SourceProvenance, SourceWaveform, TickSchedule,
        };

        let source_id = SourceId::new(id).unwrap();
        let descriptor = SourceDescriptor::new(
            source_id,
            SourceProvenance::MathematicalTest { holder_id: id },
            SourceClass::MathematicalTest,
            SourceGeometry::Disk {
                center_x: 1.0,
                center_y: 1.0,
                radius: 0.5,
            },
            TickSchedule::new(0, None).unwrap(),
            SourceWaveform::SinglePulse,
            SourcePowerLaw::MathematicalTestOnly,
        )
        .unwrap();
        SourceInstance::new(
            descriptor,
            SourceEnergyAccount::new(source_id, initial_energy).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn environmental_source_settlement_reconciles_and_debits_once() {
        let mut system = model(1.5, 0.2, 0.005);
        let mut source = mathematical_source(71, 10.0);
        let initial_total = system.total_energy();
        let work = system
            .step_with_environmental_source(&mut source, 5.0)
            .unwrap();

        assert!(work > 0.0);
        assert!((system.total_energy() - initial_total - work).abs() < 1.0e-10);
        assert!((source.account().remaining_energy() - (10.0 - work)).abs() < 1.0e-12);
        assert!((source.account().cumulative_field_transfer() - work).abs() < 1.0e-12);
    }

    #[test]
    fn environmental_source_overdraw_rolls_back_model_and_account() {
        let mut system = model(1.5, 0.2, 0.005);
        let before = system.clone();
        let mut source = mathematical_source(72, 0.0);
        let account_before = source.account().remaining_energy();
        let result = system.step_with_environmental_source(&mut source, 5.0);

        assert_eq!(result, Err(ReciprocalModelError::InsufficientSourceEnergy));
        assert_eq!(system.steps_elapsed, before.steps_elapsed);
        assert_eq!(system.previous_field, before.previous_field);
        assert_eq!(system.current_field, before.current_field);
        assert_eq!(
            system.previous_source.to_bits(),
            before.previous_source.to_bits()
        );
        assert_eq!(
            system.current_source.to_bits(),
            before.current_source.to_bits()
        );
        assert_eq!(source.account().remaining_energy(), account_before);
        assert_eq!(source.account().cumulative_field_transfer(), 0.0);
    }

    #[test]
    fn reciprocal_model_rejects_non_finite_external_drive_without_mutation() {
        let mut system = model(1.5, 0.2, 0.005);
        let before = system.clone();
        assert_eq!(
            system.step_with_external_force(f64::NAN),
            Err(ReciprocalModelError::InvalidParameter)
        );
        assert_eq!(system.steps_elapsed, before.steps_elapsed);
        assert_eq!(system.previous_field, before.previous_field);
        assert_eq!(system.current_field, before.current_field);
        assert_eq!(
            system.previous_source.to_bits(),
            before.previous_source.to_bits()
        );
        assert_eq!(
            system.current_source.to_bits(),
            before.current_source.to_bits()
        );
    }

    #[test]
    fn reciprocal_model_replay_is_bitwise_deterministic() {
        let mut left = model(1.5, 0.2, 0.005);
        let mut right = model(1.5, 0.2, 0.005);
        for _ in 0..4_000 {
            left.step().unwrap();
            right.step().unwrap();
        }
        assert_eq!(left.steps_elapsed, right.steps_elapsed);
        assert_eq!(left.previous_field.len(), right.previous_field.len());
        for (a, b) in left.previous_field.iter().zip(&right.previous_field) {
            assert_eq!(a.to_bits(), b.to_bits());
        }
        for (a, b) in left.current_field.iter().zip(&right.current_field) {
            assert_eq!(a.to_bits(), b.to_bits());
        }
        assert_eq!(
            left.previous_source.to_bits(),
            right.previous_source.to_bits()
        );
        assert_eq!(
            left.current_source.to_bits(),
            right.current_source.to_bits()
        );
        assert_eq!(
            left.total_energy().to_bits(),
            right.total_energy().to_bits()
        );
    }

    #[test]
    fn reciprocal_model_energy_exchange_changes_both_inventories() {
        let mut system = model(1.5, 0.2, 0.005);
        let initial_field_energy = system.field_energy();
        let initial_source_energy = system.source_energy();
        for _ in 0..3_000 {
            system.step().unwrap();
        }
        assert!(system.field_energy() > initial_field_energy);
        assert!((system.source_energy() - initial_source_energy).abs() > 1.0e-5);
    }

    #[test]
    fn reciprocal_model_spatial_refinement_converges_at_fixed_time_step() {
        let duration = 1.0_f64;
        let configurations = [(17_usize, 0.2_f64), (33, 0.1), (65, 0.05)];
        let mut source_displacements = Vec::new();

        for (cells, spacing) in configurations {
            let time_step = 0.001;
            let mut system = refined_model(cells, spacing, time_step);
            let steps = (duration / time_step).round() as usize;
            for _ in 0..steps {
                system.step().unwrap();
            }
            source_displacements.push(system.source_displacement());
        }

        let reference = source_displacements[2];
        let coarse_error = (source_displacements[0] - reference).abs();
        let medium_error = (source_displacements[1] - reference).abs();
        assert!(
            medium_error < coarse_error,
            "spatial refinement did not reduce source-displacement error: coarse={coarse_error}, medium={medium_error}"
        );
    }

    #[test]
    fn reciprocal_model_temporal_refinement_converges_at_fixed_grid() {
        let duration = 1.0_f64;
        let cells = 33;
        let spacing = 0.1;
        let time_steps = [0.004_f64, 0.002, 0.001, 0.0005];
        let mut source_displacements = Vec::new();

        for time_step in time_steps {
            let mut system = refined_model(cells, spacing, time_step);
            let steps = (duration / time_step).round() as usize;
            for _ in 0..steps {
                system.step().unwrap();
            }
            source_displacements.push(system.source_displacement());
        }

        let reference = source_displacements[3];
        let coarse_error = (source_displacements[0] - reference).abs();
        let medium_error = (source_displacements[1] - reference).abs();
        let fine_error = (source_displacements[2] - reference).abs();
        assert!(
            medium_error < coarse_error && fine_error < medium_error,
            "temporal refinement did not monotonically reduce source-displacement error: coarse={coarse_error}, medium={medium_error}, fine={fine_error}"
        );
    }

    #[test]
    fn reciprocal_model_parameter_sweep_preserves_energy_and_exchanges_energy() {
        let duration = 8.0_f64;
        let frequencies = [0.75_f64, 1.5, 3.0];
        let couplings = [0.05_f64, 0.2, 0.8];
        let time_steps = [0.0025_f64, 0.005, 0.01];

        for frequency in frequencies {
            for coupling in couplings {
                for time_step in time_steps {
                    let mut system = model(frequency, coupling, time_step);
                    let initial_total = system.total_energy();
                    let initial_source = system.source_energy();
                    let steps = (duration / time_step).round() as usize;

                    for _ in 0..steps {
                        system.step().unwrap();
                    }

                    let final_total = system.total_energy();
                    let scale = initial_total.abs().max(final_total.abs()).max(1.0);
                    let residual = (final_total - initial_total).abs() / scale;
                    assert!(
                        residual <= 1.0e-8,
                        "energy drift for frequency={frequency}, coupling={coupling}, dt={time_step}: {residual}"
                    );
                    assert!(
                        system.field_energy() > 0.0,
                        "no field energy for frequency={frequency}, coupling={coupling}, dt={time_step}"
                    );
                    assert!(
                        (system.source_energy() - initial_source).abs() > 1.0e-8,
                        "no source energy exchange for frequency={frequency}, coupling={coupling}, dt={time_step}"
                    );
                    assert!(system.coupling_energy().is_finite());
                    assert_eq!(system.steps_elapsed(), steps as u64);
                }
            }
        }
    }

    #[test]
    fn candidate_a_replay_of_candidate_b_coupling_matches_field_evolution() {
        use super::super::reference_wave_solver::WaveField;
        use super::super::source::{
            SourceClass, SourceDescriptor, SourceEnergyAccount, SourceGeometry, SourceId,
            SourceInstance, SourcePowerLaw, SourceProvenance, SourceWaveform, TickSchedule,
        };

        let cells = 33;
        let spacing = 0.2;
        let time_step = 0.005;
        let substeps_per_tick = (1.0 / time_step) as usize;
        let mut candidate_b = refined_model(cells, spacing, time_step);
        let mut candidate_a =
            WaveField::new(cells, cells, spacing, 1.0, substeps_per_tick, 1.0).unwrap();

        let source_id = SourceId::new(9001).unwrap();
        let descriptor = SourceDescriptor::new(
            source_id,
            SourceProvenance::MathematicalTest { holder_id: 9001 },
            SourceClass::MathematicalTest,
            SourceGeometry::Disk {
                center_x: (cells as f64 - 1.0) * spacing * 0.5,
                center_y: (cells as f64 - 1.0) * spacing * 0.5,
                radius: 0.5,
            },
            TickSchedule::new(0, None).unwrap(),
            SourceWaveform::SinglePulse,
            SourcePowerLaw::MathematicalTestOnly,
        )
        .unwrap();
        let account = SourceEnergyAccount::new(source_id, 100.0).unwrap();
        let mut prescribed_source = SourceInstance::new(descriptor, account).unwrap();

        let initial_b_energy = candidate_b.total_energy();
        let mut maximum_field_difference: f64 = 0.0;
        let mut signed_work = 0.0;
        let steps = 2_000;

        for _ in 0..steps {
            // Candidate A receives Candidate B's instantaneous reciprocal
            // coupling acceleration as a prescribed force. This deliberately
            // replays the same force history; it does not make the source
            // models physically equivalent or give Candidate A an oscillator.
            let gap =
                candidate_b.current_source - candidate_b.source_average(&candidate_b.current_field);
            let forcing: Vec<f64> = candidate_b
                .profile
                .iter()
                .map(|weight| {
                    candidate_b.coupling_strength / candidate_b.field_normalization * gap * weight
                })
                .collect();

            let work = candidate_a
                .advance_forced_substep(&mut prescribed_source, &forcing)
                .unwrap();
            signed_work += work;
            candidate_b.step().unwrap();

            for y in 0..cells {
                for x in 0..cells {
                    let index = y * cells + x;
                    let difference = (candidate_a.sample(x, y).unwrap()
                        - candidate_b.current_field[index])
                        .abs();
                    maximum_field_difference = maximum_field_difference.max(difference);
                }
            }
        }

        let field_energy_difference = (candidate_a.energy() - candidate_b.field_energy()).abs();
        let replayed_work_difference = (signed_work - candidate_b.field_energy()).abs();
        let candidate_b_energy_residual = (candidate_b.total_energy() - initial_b_energy).abs();

        assert!(
            maximum_field_difference <= 1.0e-12,
            "matched forcing did not reproduce Candidate B field: {maximum_field_difference}"
        );
        assert!(
            field_energy_difference <= 1.0e-10,
            "field energy mismatch: {field_energy_difference}"
        );
        assert!(
            replayed_work_difference <= 1.0e-10,
            "Candidate A signed work did not match field energy: {replayed_work_difference}"
        );
        assert!(
            candidate_b_energy_residual <= 1.0e-8,
            "Candidate B total-energy residual: {candidate_b_energy_residual}"
        );
        assert_eq!(
            candidate_a.logical_time_ticks(),
            steps as f64 / substeps_per_tick as f64
        );
        assert_eq!(candidate_b.steps_elapsed(), steps as u64);
        assert!(candidate_b.source_energy().is_finite());
        assert!(candidate_b.coupling_energy().is_finite());
    }
}
