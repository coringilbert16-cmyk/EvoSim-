//! Standalone reference solver for the approved two-dimensional scalar wave model.
//!
//! This module is independent of the live simulation, chemistry, movement,
//! materials, and production energy ledger. It uses a uniform Cartesian grid
//! and a second-order leapfrog update. Fixed-zero outer grid nodes are numerical
//! boundaries only; reference measurements must finish before their numerical
//! domain of dependence can reach those boundaries.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WaveFieldError {
    InvalidDimensions,
    InvalidParameter,
    UnstableTimeStep,
    InvalidProfile,
    InvalidState,
    InsufficientSourceEnergy,
    AlreadyStarted,
    CounterOverflow,
}

#[derive(Clone, Debug)]
pub(crate) struct FiniteEnergySource {
    initial_budget: f64,
    available_energy: f64,
    net_energy_debit: f64,
}

impl FiniteEnergySource {
    pub(crate) fn new(energy_budget: f64) -> Result<Self, WaveFieldError> {
        if !energy_budget.is_finite() || energy_budget < 0.0 {
            return Err(WaveFieldError::InvalidParameter);
        }
        Ok(Self {
            initial_budget: energy_budget,
            available_energy: energy_budget,
            net_energy_debit: 0.0,
        })
    }

    pub(crate) fn available_energy(&self) -> f64 {
        self.available_energy
    }

    pub(crate) fn initial_budget(&self) -> f64 {
        self.initial_budget
    }

    pub(crate) fn net_energy_debit(&self) -> f64 {
        self.net_energy_debit
    }
}

#[derive(Clone, Debug)]
pub(crate) struct WaveField {
    width: usize,
    height: usize,
    spacing: f64,
    propagation_speed: f64,
    substeps_per_tick: usize,
    time_step: f64,
    energy_normalization: f64,
    previous: Vec<f64>,
    current: Vec<f64>,
    substeps_elapsed: u64,
}

impl WaveField {
    /// Creates a zero-field solver.
    ///
    /// Propagation speed is in world-distance units per logical tick.
    /// The integrator takes substeps_per_tick equal steps during each tick.
    /// The five-point/leapfrog stability condition is c * dt / dx <= 1 / sqrt(2).
    pub(crate) fn new(
        width: usize,
        height: usize,
        spacing: f64,
        propagation_speed: f64,
        substeps_per_tick: usize,
        energy_normalization: f64,
    ) -> Result<Self, WaveFieldError> {
        if width < 3 || height < 3 {
            return Err(WaveFieldError::InvalidDimensions);
        }
        if !spacing.is_finite()
            || spacing <= 0.0
            || !propagation_speed.is_finite()
            || propagation_speed <= 0.0
            || substeps_per_tick == 0
            || !energy_normalization.is_finite()
            || energy_normalization <= 0.0
        {
            return Err(WaveFieldError::InvalidParameter);
        }

        let time_step = 1.0 / substeps_per_tick as f64;
        let courant_ratio = propagation_speed * time_step / spacing;
        if !courant_ratio.is_finite() || courant_ratio > std::f64::consts::FRAC_1_SQRT_2
        {
            return Err(WaveFieldError::UnstableTimeStep);
        }

        let cell_count = width
            .checked_mul(height)
            .ok_or(WaveFieldError::InvalidDimensions)?;

        Ok(Self {
            width,
            height,
            spacing,
            propagation_speed,
            substeps_per_tick,
            time_step,
            energy_normalization,
            previous: vec![0.0; cell_count],
            current: vec![0.0; cell_count],
            substeps_elapsed: 0,
        })
    }

    pub(crate) fn width(&self) -> usize {
        self.width
    }

    pub(crate) fn height(&self) -> usize {
        self.height
    }

    pub(crate) fn spacing(&self) -> f64 {
        self.spacing
    }

    pub(crate) fn propagation_speed(&self) -> f64 {
        self.propagation_speed
    }

    pub(crate) fn time_step(&self) -> f64 {
        self.time_step
    }

    pub(crate) fn substeps_per_tick(&self) -> usize {
        self.substeps_per_tick
    }

    pub(crate) fn logical_time_ticks(&self) -> f64 {
        self.substeps_elapsed as f64 / self.substeps_per_tick as f64
    }

    pub(crate) fn sample(&self, x: usize, y: usize) -> Option<f64> {
        if x >= self.width || y >= self.height {
            return None;
        }
        Some(self.current[self.index(x, y)])
    }

    /// Conserved discrete leapfrog energy at the current half-step.
    /// The cross-time gradient term is required by this update scheme.
    pub(crate) fn energy(&self) -> f64 {
        self.discrete_energy(&self.previous, &self.current)
    }

    /// Adds an initial displacement/velocity profile and settles the exact
    /// change in discrete field energy against a finite source budget.
    ///
    /// Multiple sources may be added before the first propagation substep.
    /// Positive transfer funds the field; negative transfer returns energy
    /// to the source. Profile validation, field initialization, and settlement
    /// are atomic.
    pub(crate) fn inject_initial_profile(
        &mut self,
        source: &mut FiniteEnergySource,
        displacement: &[f64],
        velocity: &[f64],
    ) -> Result<f64, WaveFieldError> {
        if self.substeps_elapsed != 0 {
            return Err(WaveFieldError::AlreadyStarted);
        }
        if displacement.len() != self.current.len() || velocity.len() != self.current.len() {
            return Err(WaveFieldError::InvalidProfile);
        }
        if displacement
            .iter()
            .chain(velocity.iter())
            .any(|value| !value.is_finite())
        {
            return Err(WaveFieldError::InvalidProfile);
        }

        for y in 0..self.height {
            for x in 0..self.width {
                if x == 0 || y == 0 || x + 1 == self.width || y + 1 == self.height {
                    let index = self.index(x, y);
                    if displacement[index] != 0.0 || velocity[index] != 0.0 {
                        return Err(WaveFieldError::InvalidProfile);
                    }
                }
            }
        }

        let old_energy = self.energy();
        let mut next_current = self.current.clone();
        let mut initial_velocity = vec![0.0; self.current.len()];
        for index in 0..self.current.len() {
            initial_velocity[index] =
                (self.current[index] - self.previous[index]) / self.time_step + velocity[index];
            next_current[index] += displacement[index];
        }
        for y in 1..self.height - 1 {
            for x in 1..self.width - 1 {
                let index = self.index(x, y);
                initial_velocity[index] += 0.5
                    * self.time_step
                    * self.propagation_speed.powi(2)
                    * self.laplacian(&self.current, x, y);
            }
        }

        let acceleration_scale = 0.5 * self.time_step.powi(2) * self.propagation_speed.powi(2);
        let mut next_previous = next_current.clone();
        for y in 1..self.height - 1 {
            for x in 1..self.width - 1 {
                let index = self.index(x, y);
                next_previous[index] = next_current[index]
                    - self.time_step * initial_velocity[index]
                    + acceleration_scale * self.laplacian(&next_current, x, y);
            }
        }

        if next_current
            .iter()
            .chain(next_previous.iter())
            .any(|value| !value.is_finite())
        {
            return Err(WaveFieldError::InvalidProfile);
        }

        let next_energy = self.discrete_energy(&next_previous, &next_current);
        let signed_transfer = next_energy - old_energy;
        let next_available = source.available_energy - signed_transfer;
        let next_net_debit = source.net_energy_debit + signed_transfer;
        if !next_energy.is_finite()
            || next_energy < 0.0
            || !signed_transfer.is_finite()
            || !next_available.is_finite()
            || next_available < 0.0
            || !next_net_debit.is_finite()
        {
            return Err(if signed_transfer > source.available_energy {
                WaveFieldError::InsufficientSourceEnergy
            } else {
                WaveFieldError::InvalidState
            });
        }

        self.previous = next_previous;
        self.current = next_current;
        source.available_energy = next_available;
        source.net_energy_debit = next_net_debit;
        Ok(signed_transfer)
    }

    /// Injects a compact-support radial velocity pulse as a mathematical test source.
    pub(crate) fn inject_compact_pulse(
        &mut self,
        source: &mut FiniteEnergySource,
        center_x: f64,
        center_y: f64,
        support_radius: f64,
        peak_velocity: f64,
    ) -> Result<f64, WaveFieldError> {
        if !center_x.is_finite()
            || !center_y.is_finite()
            || !support_radius.is_finite()
            || support_radius <= 0.0
            || !peak_velocity.is_finite()
        {
            return Err(WaveFieldError::InvalidParameter);
        }

        let mut velocity = vec![0.0; self.current.len()];
        let radius_squared = support_radius.powi(2);
        for y in 1..self.height - 1 {
            for x in 1..self.width - 1 {
                let world_x = x as f64 * self.spacing;
                let world_y = y as f64 * self.spacing;
                let dx = world_x - center_x;
                let dy = world_y - center_y;
                let normalized_radius_squared = (dx * dx + dy * dy) / radius_squared;
                if normalized_radius_squared < 1.0 {
                    let profile = (1.0 - 1.0 / (1.0 - normalized_radius_squared)).exp();
                    velocity[self.index(x, y)] = peak_velocity * profile;
                }
            }
        }
        let displacement = vec![0.0; self.current.len()];
        self.inject_initial_profile(source, &displacement, &velocity)
    }

    pub(crate) fn advance_substeps(&mut self, count: u64) -> Result<(), WaveFieldError> {
        self.substeps_elapsed
            .checked_add(count)
            .ok_or(WaveFieldError::CounterOverflow)?;
        for _ in 0..count {
            self.advance_one_substep()?;
        }
        Ok(())
    }

    pub(crate) fn advance_ticks(&mut self, ticks: u64) -> Result<(), WaveFieldError> {
        let count = ticks
            .checked_mul(self.substeps_per_tick as u64)
            .ok_or(WaveFieldError::CounterOverflow)?;
        self.advance_substeps(count)
    }

    fn advance_one_substep(&mut self) -> Result<(), WaveFieldError> {
        let ratio = self.propagation_speed * self.time_step / self.spacing;
        let ratio_squared = ratio * ratio;
        let mut next = vec![0.0; self.current.len()];

        for y in 1..self.height - 1 {
            for x in 1..self.width - 1 {
                let index = self.index(x, y);
                next[index] = 2.0 * self.current[index] - self.previous[index]
                    + ratio_squared * self.laplacian(&self.current, x, y);
            }
        }

        if next.iter().any(|value| !value.is_finite()) {
            return Err(WaveFieldError::InvalidState);
        }
        let next_energy = self.discrete_energy(&self.current, &next);
        if !next_energy.is_finite() || next_energy < 0.0 {
            return Err(WaveFieldError::InvalidState);
        }

        self.previous.clone_from(&self.current);
        self.current = next;
        self.substeps_elapsed += 1;
        Ok(())
    }

    fn laplacian(&self, values: &[f64], x: usize, y: usize) -> f64 {
        let index = self.index(x, y);
        (values[index - 1]
            + values[index + 1]
            + values[index - self.width]
            + values[index + self.width]
            - 4.0 * values[index])
            / self.spacing.powi(2)
    }

    fn discrete_energy(&self, previous: &[f64], current: &[f64]) -> f64 {
        let mut kinetic_sum = 0.0;
        for index in 0..current.len() {
            let velocity = (current[index] - previous[index]) / self.time_step;
            kinetic_sum += velocity * velocity;
        }

        let mut gradient_cross_sum = 0.0;
        for y in 0..self.height {
            for x in 0..self.width - 1 {
                let index = self.index(x, y);
                let next = index + 1;
                gradient_cross_sum +=
                    (current[next] - current[index]) * (previous[next] - previous[index]);
            }
        }
        for y in 0..self.height - 1 {
            for x in 0..self.width {
                let index = self.index(x, y);
                let next = index + self.width;
                gradient_cross_sum += (current[next] - current[index])
                    * (previous[next] - previous[index]);
            }
        }

        0.5 * self.energy_normalization
            * (kinetic_sum * self.spacing.powi(2)
                + self.propagation_speed.powi(2) * gradient_cross_sum)
    }

    fn index(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solver(width: usize, height: usize, spacing: f64, substeps: usize) -> WaveField {
        WaveField::new(width, height, spacing, 1.0, substeps, 1.0).expect("stable reference solver")
    }

    fn pulse(
        field: &mut WaveField,
        source: &mut FiniteEnergySource,
        x: f64,
        y: f64,
        amplitude: f64,
    ) {
        field
            .inject_compact_pulse(source, x, y, 3.0, amplitude)
            .expect("finite-budget pulse injection");
    }

    fn mode_profile(
        width: usize,
        height: usize,
        spacing: f64,
        length: f64,
        mode_x: usize,
        mode_y: usize,
    ) -> Vec<f64> {
        let mut profile = vec![0.0; width * height];
        for y in 1..height - 1 {
            for x in 1..width - 1 {
                let px = x as f64 * spacing;
                let py = y as f64 * spacing;
                profile[y * width + x] = (std::f64::consts::PI * mode_x as f64 * px / length).sin()
                    * (std::f64::consts::PI * mode_y as f64 * py / length).sin();
            }
        }
        profile
    }

    fn discrete_mode_frequency(
        spacing: f64,
        speed: f64,
        length: f64,
        mode_x: usize,
        mode_y: usize,
    ) -> f64 {
        let kx = std::f64::consts::PI * mode_x as f64 / length;
        let ky = std::f64::consts::PI * mode_y as f64 / length;
        let eigenvalue = 4.0 / spacing.powi(2)
            * ((0.5 * kx * spacing).sin().powi(2) + (0.5 * ky * spacing).sin().powi(2));
        speed * eigenvalue.sqrt()
    }

    #[test]
    fn rejects_invalid_grids_and_time_steps_outside_the_stability_bound() {
        assert_eq!(
            WaveField::new(2, 10, 1.0, 1.0, 2, 1.0).err(),
            Some(WaveFieldError::InvalidDimensions)
        );
        assert_eq!(
            WaveField::new(10, 10, 1.0, 1.0, 1, 1.0).err(),
            Some(WaveFieldError::UnstableTimeStep)
        );
        assert!(WaveField::new(10, 10, 1.0, 1.4, 2, 1.0).is_ok());
        assert_eq!(
            WaveField::new(10, 10, 1.0, 1.44, 2, 1.0).err(),
            Some(WaveFieldError::UnstableTimeStep)
        );
    }

    #[test]
    fn finite_source_transfer_is_atomic_and_field_energy_is_conserved() {
        let mut field = solver(201, 201, 1.0, 2);
        let mut insufficient = FiniteEnergySource::new(0.001).unwrap();
        assert_eq!(
            field.inject_compact_pulse(&mut insufficient, 100.0, 100.0, 3.0, 0.5),
            Err(WaveFieldError::InsufficientSourceEnergy)
        );
        assert_eq!(field.energy(), 0.0);
        assert_eq!(insufficient.available_energy(), 0.001);
        assert_eq!(insufficient.net_energy_debit(), 0.0);

        let mut source = FiniteEnergySource::new(10.0).unwrap();
        let initial_transfer = field
            .inject_compact_pulse(&mut source, 100.0, 100.0, 3.0, 0.5)
            .unwrap();
        assert!(initial_transfer > 0.0);
        assert!((field.energy() - initial_transfer).abs() < 1.0e-12);
        assert!(
            (field.energy() + source.available_energy() - source.initial_budget()).abs() < 1.0e-12
        );

        let initial_energy = field.energy();
        field.advance_ticks(40).unwrap();
        let final_energy = field.energy();
        let relative_drift = (final_energy - initial_energy).abs() / initial_energy;
        assert!(
            relative_drift < 1.0e-10,
            "relative energy drift: {relative_drift}"
        );
        assert_eq!(source.net_energy_debit(), initial_transfer);
    }

    #[test]
    fn compact_pulse_has_finite_travel_time_and_arrives_near_distance_over_speed() {
        let mut field = solver(201, 201, 1.0, 2);
        let mut source = FiniteEnergySource::new(10.0).unwrap();
        pulse(&mut field, &mut source, 100.0, 100.0, 0.5);

        field.advance_ticks(10).unwrap();
        assert_eq!(field.sample(124, 100).unwrap(), 0.0);

        field.advance_ticks(14).unwrap();
        let after_arrival = field.sample(124, 100).unwrap();
        assert!(
            after_arrival.abs() > 1.0e-2,
            "wave had not reached receiver after 24 ticks: {after_arrival}"
        );
        assert!((field.logical_time_ticks() - 24.0).abs() < f64::EPSILON);
    }

    #[test]
    fn two_sources_superpose_constructively_and_destructively() {
        let mut first = solver(241, 241, 1.0, 2);
        let mut second = solver(241, 241, 1.0, 2);
        let mut constructive = solver(241, 241, 1.0, 2);
        let mut destructive = solver(241, 241, 1.0, 2);
        let mut first_source = FiniteEnergySource::new(10.0).unwrap();
        let mut second_source = FiniteEnergySource::new(10.0).unwrap();
        let mut constructive_a = FiniteEnergySource::new(10.0).unwrap();
        let mut constructive_b = FiniteEnergySource::new(10.0).unwrap();
        let mut destructive_a = FiniteEnergySource::new(10.0).unwrap();
        let mut destructive_b = FiniteEnergySource::new(10.0).unwrap();

        pulse(&mut first, &mut first_source, 80.0, 120.0, 0.5);
        pulse(&mut second, &mut second_source, 160.0, 120.0, 0.5);
        pulse(&mut constructive, &mut constructive_a, 80.0, 120.0, 0.5);
        pulse(&mut constructive, &mut constructive_b, 160.0, 120.0, 0.5);
        pulse(&mut destructive, &mut destructive_a, 80.0, 120.0, 0.5);
        pulse(&mut destructive, &mut destructive_b, 160.0, 120.0, -0.5);

        first.advance_ticks(40).unwrap();
        second.advance_ticks(40).unwrap();
        constructive.advance_ticks(40).unwrap();
        destructive.advance_ticks(40).unwrap();

        let a = first.sample(120, 120).unwrap();
        let b = second.sample(120, 120).unwrap();
        let sum = constructive.sample(120, 120).unwrap();
        let difference = destructive.sample(120, 120).unwrap();
        assert!((sum - (a + b)).abs() < 1.0e-12);
        assert!(sum.abs() > 1.0e-3);
        assert!(difference.abs() < 1.0e-12);
    }

    #[test]
    fn one_field_preserves_two_simultaneous_mode_frequencies_and_phases() {
        let mut field = solver(65, 65, 0.5, 4);
        let spacing = field.spacing();
        let dt = field.time_step();
        let length = 32.0;
        let first = mode_profile(65, 65, spacing, length, 1, 1);
        let second = mode_profile(65, 65, spacing, length, 2, 1);
        let mut displacement = vec![0.0; first.len()];
        for index in 0..displacement.len() {
            displacement[index] = 0.7 * first[index] + 0.3 * second[index];
        }

        let omega_a = discrete_mode_frequency(spacing, 1.0, length, 1, 1);
        let omega_b = discrete_mode_frequency(spacing, 1.0, length, 2, 1);
        field.current.clone_from(&displacement);
        for index in 0..field.previous.len() {
            field.previous[index] = 0.7 * first[index] * (omega_a * dt).cos()
                + 0.3 * second[index] * (omega_b * dt).cos();
        }

        field.advance_ticks(12).unwrap();
        let time = field.logical_time_ticks();
        let x = 24;
        let y = 20;
        let expected = 0.7 * first[y * 65 + x] * (omega_a * time).cos()
            + 0.3 * second[y * 65 + x] * (omega_b * time).cos();
        assert!((field.sample(x, y).unwrap() - expected).abs() < 1.0e-10);
    }

    #[test]
    fn spatial_and_time_refinement_show_second_order_convergence() {
        fn error(spacing: f64, substeps: usize) -> f64 {
            let length = 32.0;
            let intervals = (length / spacing) as usize;
            let width = intervals + 1;
            let mut field = solver(width, width, spacing, substeps);
            let mode = mode_profile(width, width, spacing, length, 1, 1);
            let dt = field.time_step();
            let omega = discrete_mode_frequency(spacing, 1.0, length, 1, 1);
            field.current.clone_from(&mode);
            for index in 0..field.previous.len() {
                field.previous[index] = mode[index] - 0.5 * dt * dt * omega.powi(2) * mode[index];
            }
            field.advance_ticks(2).unwrap();
            let continuum_omega = std::f64::consts::PI * std::f64::consts::SQRT_2 / length;
            let expected_factor = (continuum_omega * 2.0).cos();
            field
                .current
                .iter()
                .zip(mode.iter())
                .map(|(actual, initial)| (actual - initial * expected_factor).abs())
                .fold(0.0, f64::max)
        }

        let coarse = error(1.0, 2);
        let medium = error(0.5, 4);
        let fine = error(0.25, 8);
        assert!(coarse > medium * 3.5, "coarse={coarse}, medium={medium}");
        assert!(medium > fine * 3.5, "medium={medium}, fine={fine}");
    }

    #[test]
    fn identical_runs_replay_bit_for_bit() {
        let mut left = solver(161, 161, 1.0, 2);
        let mut right = solver(161, 161, 1.0, 2);
        let mut left_source = FiniteEnergySource::new(10.0).unwrap();
        let mut right_source = FiniteEnergySource::new(10.0).unwrap();
        pulse(&mut left, &mut left_source, 80.0, 80.0, 0.5);
        pulse(&mut right, &mut right_source, 80.0, 80.0, 0.5);
        left.advance_ticks(30).unwrap();
        right.advance_ticks(30).unwrap();
        assert_eq!(left.current, right.current);
        assert_eq!(left.previous, right.previous);
        assert_eq!(left.energy().to_bits(), right.energy().to_bits());
    }
}
