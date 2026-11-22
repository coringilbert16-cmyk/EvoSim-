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
        let spacing_squared = spacing.powi(2);
        let speed_squared = propagation_speed.powi(2);
        if !time_step.is_finite()
            || time_step <= 0.0
            || !spacing_squared.is_finite()
            || spacing_squared <= 0.0
            || !speed_squared.is_finite()
        {
            return Err(WaveFieldError::InvalidParameter);
        }
        let courant_ratio = propagation_speed * time_step / spacing;
        if !courant_ratio.is_finite() || courant_ratio > std::f64::consts::FRAC_1_SQRT_2 {
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

    /// Discrete energy assigned to an inclusive rectangular control volume.
    /// Edge potential energy is split evenly between its endpoint cells, so
    /// a region and its complement sum exactly to the global field energy.
    /// In source-free empty space, the region's energy change is its net
    /// integrated boundary flux.
    pub(crate) fn energy_in_region(
        &self,
        x_min: usize,
        y_min: usize,
        x_max: usize,
        y_max: usize,
    ) -> Option<f64> {
        if x_min > x_max || y_min > y_max || x_max >= self.width || y_max >= self.height {
            return None;
        }

        let mut kinetic_sum = 0.0;
        for y in y_min..=y_max {
            for x in x_min..=x_max {
                let index = self.index(x, y);
                let velocity = (self.current[index] - self.previous[index]) / self.time_step;
                kinetic_sum += velocity * velocity;
            }
        }

        let mut gradient_cross_sum = 0.0;
        let horizontal_start = x_min.saturating_sub(1);
        let horizontal_end = x_max.min(self.width - 2);
        for y in y_min..=y_max {
            for x in horizontal_start..=horizontal_end {
                let index = self.index(x, y);
                let next = index + 1;
                let left_inside = x >= x_min;
                let right_inside = x < x_max;
                let weight = 0.5 * (left_inside as u8 + right_inside as u8) as f64;
                gradient_cross_sum += weight
                    * (self.current[next] - self.current[index])
                    * (self.previous[next] - self.previous[index]);
            }
        }

        let vertical_start = y_min.saturating_sub(1);
        let vertical_end = y_max.min(self.height - 2);
        for y in vertical_start..=vertical_end {
            for x in x_min..=x_max {
                let index = self.index(x, y);
                let next = index + self.width;
                let top_inside = y >= y_min;
                let bottom_inside = y < y_max;
                let weight = 0.5 * (top_inside as u8 + bottom_inside as u8) as f64;
                gradient_cross_sum += weight
                    * (self.current[next] - self.current[index])
                    * (self.previous[next] - self.previous[index]);
            }
        }

        Some(
            0.5 * self.energy_normalization
                * (kinetic_sum * self.spacing.powi(2)
                    + self.propagation_speed.powi(2) * gradient_cross_sum),
        )
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
        let maximum_x = (self.width - 1) as f64 * self.spacing;
        let maximum_y = (self.height - 1) as f64 * self.spacing;
        let radius_squared = support_radius.powi(2);
        if !center_x.is_finite()
            || !center_y.is_finite()
            || !support_radius.is_finite()
            || support_radius <= 0.0
            || !radius_squared.is_finite()
            || !peak_velocity.is_finite()
            || !maximum_x.is_finite()
            || !maximum_y.is_finite()
            || center_x - support_radius <= 0.0
            || center_y - support_radius <= 0.0
            || center_x + support_radius >= maximum_x
            || center_y + support_radius >= maximum_y
        {
            return Err(WaveFieldError::InvalidParameter);
        }

        let mut velocity = vec![0.0; self.current.len()];
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
        let wave_step_squared = self.propagation_speed.powi(2) * self.time_step.powi(2);
        let mut next = vec![0.0; self.current.len()];

        for y in 1..self.height - 1 {
            for x in 1..self.width - 1 {
                let index = self.index(x, y);
                next[index] = 2.0 * self.current[index] - self.previous[index]
                    + wave_step_squared * self.laplacian(&self.current, x, y);
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
                gradient_cross_sum +=
                    (current[next] - current[index]) * (previous[next] - previous[index]);
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

    fn spatial_mode_frequency(
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
        assert!(WaveField::new(10, 10, 1.0, 2.0 * std::f64::consts::FRAC_1_SQRT_2, 2, 1.0).is_ok());
        assert_eq!(
            WaveField::new(
                10,
                10,
                1.0,
                2.0 * std::f64::consts::FRAC_1_SQRT_2 * 1.0001,
                2,
                1.0
            )
            .err(),
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

        field.advance_ticks(10).unwrap();
        let before_arrival = field.sample(124, 100).unwrap();
        assert!(
            before_arrival.abs() < 1.0e-2,
            "wave arrived too early: {before_arrival}"
        );

        field.advance_ticks(4).unwrap();
        let after_arrival = field.sample(124, 100).unwrap();
        assert!(
            after_arrival.abs() > 1.0e-2,
            "wave had not reached receiver after 24 ticks: {after_arrival}"
        );
        assert!((field.logical_time_ticks() - 24.0).abs() < f64::EPSILON);
    }

    fn regional_energy_for_pair(
        field: &WaveField,
        previous: &[f64],
        current: &[f64],
        region: (usize, usize, usize, usize),
    ) -> f64 {
        let (x_min, y_min, x_max, y_max) = region;
        let mut kinetic_sum = 0.0;
        for y in y_min..=y_max {
            for x in x_min..=x_max {
                let index = field.index(x, y);
                let velocity = (current[index] - previous[index]) / field.time_step;
                kinetic_sum += velocity * velocity;
            }
        }

        let mut gradient_cross_sum = 0.0;
        let horizontal_start = x_min.saturating_sub(1);
        let horizontal_end = x_max.min(field.width - 2);
        for y in y_min..=y_max {
            for x in horizontal_start..=horizontal_end {
                let index = field.index(x, y);
                let left_inside = x >= x_min;
                let right_inside = x < x_max;
                let weight = 0.5 * (left_inside as u8 + right_inside as u8) as f64;
                gradient_cross_sum += weight
                    * (current[index + 1] - current[index])
                    * (previous[index + 1] - previous[index]);
            }
        }
        let vertical_start = y_min.saturating_sub(1);
        let vertical_end = y_max.min(field.height - 2);
        for y in vertical_start..=vertical_end {
            for x in x_min..=x_max {
                let index = field.index(x, y);
                let top_inside = y >= y_min;
                let bottom_inside = y < y_max;
                let weight = 0.5 * (top_inside as u8 + bottom_inside as u8) as f64;
                gradient_cross_sum += weight
                    * (current[index + field.width] - current[index])
                    * (previous[index + field.width] - previous[index]);
            }
        }
        0.5 * field.energy_normalization
            * (kinetic_sum * field.spacing.powi(2)
                + field.propagation_speed.powi(2) * gradient_cross_sum)
    }

    // Discrete local conservation for the leapfrog scheme. For each edge
    // crossing the control-volume boundary, the regional energy increment
    // contributed by that edge is
    //
    //   (normalization * c^2 / 4) * (q_j - q_i) * (d_i + d_j),
    //
    // where q is the field at the old integer time and d = q^(n+1)-q^(n-1).
    // This follows by subtracting the two-time-level energy expressions and
    // applying the five-point update plus discrete summation by parts. It
    // uses only boundary-edge field samples, not total or complementary energy.
    fn independently_integrated_boundary_energy_change(
        field: &WaveField,
        region: (usize, usize, usize, usize),
        previous: &[f64],
        current: &[f64],
        next: &[f64],
    ) -> f64 {
        let (x_min, y_min, x_max, y_max) = region;
        let mut edge_sum = 0.0;
        let mut add_edge = |inside: usize, outside: usize| {
            let gradient = current[outside] - current[inside];
            let centered_change =
                (next[inside] - previous[inside]) + (next[outside] - previous[outside]);
            edge_sum += gradient * centered_change;
        };

        for y in y_min..=y_max {
            if x_min > 0 {
                add_edge(field.index(x_min, y), field.index(x_min - 1, y));
            }
            if x_max + 1 < field.width {
                add_edge(field.index(x_max, y), field.index(x_max + 1, y));
            }
        }
        for x in x_min..=x_max {
            if y_min > 0 {
                add_edge(field.index(x, y_min), field.index(x, y_min - 1));
            }
            if y_max + 1 < field.height {
                add_edge(field.index(x, y_max), field.index(x, y_max + 1));
            }
        }

        0.25 * field.energy_normalization * field.propagation_speed.powi(2) * edge_sum
    }

    #[test]
    fn regional_energy_change_closes_independently_integrated_boundary_flux() {
        let mut field = solver(201, 201, 1.0, 2);
        let mut source = FiniteEnergySource::new(10.0).unwrap();
        pulse(&mut field, &mut source, 100.0, 100.0, 0.5);

        let region = (95, 95, 105, 105);
        let initial_region_energy = field
            .energy_in_region(region.0, region.1, region.2, region.3)
            .unwrap();
        assert!(field.energy_in_region(10, 10, 9, 20).is_none());

        let mut independently_integrated_flux = 0.0;
        let mut largest_step_residual: f64 = 0.0;
        for _ in 0..48 {
            let previous = field.previous.clone();
            let current = field.current.clone();
            field.advance_substeps(1).unwrap();
            let next = field.current.clone();

            let measured_change = field
                .energy_in_region(region.0, region.1, region.2, region.3)
                .unwrap()
                - regional_energy_for_pair(&field, &previous, &current, region);
            let boundary_change = independently_integrated_boundary_energy_change(
                &field, region, &previous, &current, &next,
            );
            largest_step_residual =
                largest_step_residual.max((measured_change - boundary_change).abs());
            independently_integrated_flux += boundary_change;
        }

        let final_region_energy = field
            .energy_in_region(region.0, region.1, region.2, region.3)
            .unwrap();
        let measured_net_change = final_region_energy - initial_region_energy;
        assert!(independently_integrated_flux < 0.0);
        assert!(
            largest_step_residual < 1.0e-10,
            "largest per-step local conservation residual: {largest_step_residual}"
        );
        assert!(
            (measured_net_change - independently_integrated_flux).abs() < 1.0e-10,
            "regional change={measured_net_change}, integrated boundary transfer={independently_integrated_flux}"
        );
        assert!(field.energy().is_finite());
    }

    #[test]
    fn incoming_boundary_flux_closes_for_an_off_center_rectangular_region() {
        let mut field = solver(201, 201, 1.0, 2);
        let mut source = FiniteEnergySource::new(10.0).unwrap();
        pulse(&mut field, &mut source, 80.0, 100.0, 0.5);

        // The source support is entirely outside this non-square control volume.
        let region = (90, 95, 100, 105);
        let initial_region_energy = field
            .energy_in_region(region.0, region.1, region.2, region.3)
            .unwrap();
        assert_eq!(initial_region_energy, 0.0);

        let mut integrated_boundary_transfer = 0.0;
        let mut largest_step_residual: f64 = 0.0;
        for _ in 0..12 {
            let previous = field.previous.clone();
            let current = field.current.clone();
            field.advance_substeps(1).unwrap();
            let next = field.current.clone();

            let measured_change = field
                .energy_in_region(region.0, region.1, region.2, region.3)
                .unwrap()
                - regional_energy_for_pair(&field, &previous, &current, region);
            let boundary_transfer = independently_integrated_boundary_energy_change(
                &field, region, &previous, &current, &next,
            );
            largest_step_residual =
                largest_step_residual.max((measured_change - boundary_transfer).abs());
            integrated_boundary_transfer += boundary_transfer;
        }

        let final_region_energy = field
            .energy_in_region(region.0, region.1, region.2, region.3)
            .unwrap();
        let measured_net_change = final_region_energy - initial_region_energy;
        assert!(
            measured_net_change > 0.0,
            "the region should receive wave energy"
        );
        assert!(integrated_boundary_transfer > 0.0);
        assert!(
            largest_step_residual < 1.0e-10,
            "largest per-step local conservation residual: {largest_step_residual}"
        );
        assert!(
            (measured_net_change - integrated_boundary_transfer).abs() < 1.0e-10,
            "regional change={measured_net_change}, integrated boundary transfer={integrated_boundary_transfer}"
        );
    }

    #[test]
    fn independently_integrated_flux_closes_across_region_sizes_and_orientations() {
        let regions = [
            (90, 95, 100, 105),
            (90, 97, 100, 103),
            (92, 90, 98, 110),
            (88, 94, 96, 106),
            (94, 94, 106, 106),
        ];

        for region in regions {
            let mut field = solver(241, 241, 1.0, 2);
            let mut source = FiniteEnergySource::new(10.0).unwrap();
            pulse(&mut field, &mut source, 80.0, 100.0, 0.5);

            let initial_region_energy = field
                .energy_in_region(region.0, region.1, region.2, region.3)
                .unwrap();
            let mut integrated_boundary_transfer = 0.0;
            let mut largest_step_residual: f64 = 0.0;

            for _ in 0..48 {
                let previous = field.previous.clone();
                let current = field.current.clone();
                field.advance_substeps(1).unwrap();
                let next = field.current.clone();

                let measured_change = field
                    .energy_in_region(region.0, region.1, region.2, region.3)
                    .unwrap()
                    - regional_energy_for_pair(&field, &previous, &current, region);
                let boundary_transfer = independently_integrated_boundary_energy_change(
                    &field, region, &previous, &current, &next,
                );
                largest_step_residual =
                    largest_step_residual.max((measured_change - boundary_transfer).abs());
                integrated_boundary_transfer += boundary_transfer;
            }

            let final_region_energy = field
                .energy_in_region(region.0, region.1, region.2, region.3)
                .unwrap();
            let measured_net_change = final_region_energy - initial_region_energy;

            assert!(
                largest_step_residual < 1.0e-10,
                "region={region:?}, largest local conservation residual={largest_step_residual}"
            );
            assert!(
                (measured_net_change - integrated_boundary_transfer).abs() < 1.0e-10,
                "region={region:?}, regional change={measured_net_change}, integrated flux={integrated_boundary_transfer}"
            );
            assert!(
                integrated_boundary_transfer.abs() > 1.0e-12,
                "region={region:?} should exchange measurable energy across its boundary"
            );
        }
    }

    #[test]
    fn returned_field_work_is_credited_to_the_active_source() {
        let mut field = solver(81, 81, 1.0, 2);
        let mut emitter = FiniteEnergySource::new(10.0).unwrap();
        let mut absorber = FiniteEnergySource::new(0.0).unwrap();
        let emitted = field
            .inject_compact_pulse(&mut emitter, 40.0, 40.0, 3.0, 0.5)
            .unwrap();
        let returned = field
            .inject_compact_pulse(&mut absorber, 40.0, 40.0, 3.0, -0.5)
            .unwrap();

        assert!(emitted > 0.0);
        assert!((emitted + returned).abs() < 1.0e-12);
        assert!(field.energy().abs() < 1.0e-12);
        assert!((absorber.available_energy() - emitted).abs() < 1.0e-12);
    }

    #[test]
    fn computational_window_does_not_affect_interior_before_boundary_influence() {
        let mut smaller = solver(201, 201, 1.0, 2);
        let mut larger = solver(241, 241, 1.0, 2);
        let mut small_source = FiniteEnergySource::new(10.0).unwrap();
        let mut large_source = FiniteEnergySource::new(10.0).unwrap();
        pulse(&mut smaller, &mut small_source, 100.0, 100.0, 0.5);
        pulse(&mut larger, &mut large_source, 120.0, 120.0, 0.5);
        smaller.advance_ticks(40).unwrap();
        larger.advance_ticks(40).unwrap();

        for offset_y in -20isize..=20 {
            for offset_x in -20isize..=20 {
                let small = smaller
                    .sample(
                        (100isize + offset_x) as usize,
                        (100isize + offset_y) as usize,
                    )
                    .unwrap();
                let large = larger
                    .sample(
                        (120isize + offset_x) as usize,
                        (120isize + offset_y) as usize,
                    )
                    .unwrap();
                assert!((small - large).abs() < 1.0e-12);
            }
        }
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

        let spatial_omega_a = spatial_mode_frequency(spacing, 1.0, length, 1, 1);
        let spatial_omega_b = spatial_mode_frequency(spacing, 1.0, length, 2, 1);
        let omega_a = (1.0 - 0.5 * (spatial_omega_a * dt).powi(2)).acos() / dt;
        let omega_b = (1.0 - 0.5 * (spatial_omega_b * dt).powi(2)).acos() / dt;
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
            let omega = spatial_mode_frequency(spacing, 1.0, length, 1, 1);
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
    fn radial_wavefront_isotropy_improves_under_grid_refinement() {
        fn anisotropy_error(spacing: f64, substeps: usize) -> f64 {
            let domain_length = 100.0;
            let intervals = (domain_length / spacing) as usize;
            let width = intervals + 1;
            let center = intervals / 2;
            let mut field = solver(width, width, spacing, substeps);
            let mut source = FiniteEnergySource::new(10.0).unwrap();
            let center_world = center as f64 * spacing;
            pulse(&mut field, &mut source, center_world, center_world, 0.5);
            field.advance_ticks(10).unwrap();

            let axial_offset = (10.0 / spacing).round() as usize;
            let off_axis_x = (6.0 / spacing).round() as usize;
            let off_axis_y = (8.0 / spacing).round() as usize;
            let axial = field.sample(center + axial_offset, center).unwrap();
            let off_axis = field
                .sample(center + off_axis_x, center + off_axis_y)
                .unwrap();
            (axial - off_axis).abs() / axial.abs().max(off_axis.abs())
        }

        let coarse = anisotropy_error(1.0, 2);
        let medium = anisotropy_error(0.5, 4);
        let fine = anisotropy_error(0.25, 8);
        assert!(coarse > medium * 5.0, "coarse={coarse}, medium={medium}");
        assert!(medium > fine * 1.5, "medium={medium}, fine={fine}");
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
        right.advance_substeps(60).unwrap();
        assert_eq!(left.current, right.current);
        assert_eq!(left.previous, right.previous);
        assert_eq!(left.energy().to_bits(), right.energy().to_bits());
    }
}
