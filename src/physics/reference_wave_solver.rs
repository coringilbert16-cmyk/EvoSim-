//! Isolated reference solver extracted from PR #410 (Phase 2), preserving its
//! validated leapfrog update and discrete energy invariant. This copy is kept
//! separate from live simulation state and production energy accounting.
//!
//! Original reference implementation: PR #410, branch phase-2-independent-wave-solver.
//!
//! This module is independent of the live simulation, chemistry, movement,
//! materials, and production energy ledger. It uses a uniform Cartesian grid
//! and a second-order leapfrog update. Fixed-zero outer grid nodes are numerical
//! boundaries only; reference measurements must finish before their numerical
//! domain of dependence can reach those boundaries.

use super::source::{SourceAccountError, SourceInstance};

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
    boundary_damping: Vec<f64>,
    boundary_absorbed_energy: f64,
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
        let spatial_energy_scale = energy_normalization * spacing_squared;
        let gradient_energy_scale = energy_normalization * speed_squared;
        let wave_step_squared = speed_squared * time_step.powi(2);
        if !time_step.is_finite()
            || time_step <= 0.0
            || !spacing_squared.is_finite()
            || spacing_squared <= 0.0
            || !speed_squared.is_finite()
            || speed_squared <= 0.0
            || !spatial_energy_scale.is_finite()
            || spatial_energy_scale <= 0.0
            || !gradient_energy_scale.is_finite()
            || gradient_energy_scale <= 0.0
            || !wave_step_squared.is_finite()
            || wave_step_squared <= 0.0
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
            boundary_damping: vec![0.0; cell_count],
            boundary_absorbed_energy: 0.0,
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

    /// Energy removed by the numerical absorbing layer, recorded as
    /// computational-boundary outflow rather than silently discarded.
    pub(crate) fn boundary_absorbed_energy(&self) -> f64 {
        self.boundary_absorbed_energy
    }

    /// Configures a graded sponge layer inside the fixed-zero computational
    /// edge. This is an approximate absorber, not a perfectly matched layer:
    /// its residual reflection must be measured for the frequencies and
    /// incidence angles used by a reference experiment.
    ///
    /// The centered damping term has an exact discrete energy sink:
    /// normalization * dx^2 * sum(sigma * dt * v_mid^2).
    /// Configuration is immutable once propagation has started.
    pub(crate) fn configure_absorbing_boundary(
        &mut self,
        layer_width: usize,
        maximum_damping: f64,
    ) -> Result<(), WaveFieldError> {
        if self.substeps_elapsed != 0 {
            return Err(WaveFieldError::AlreadyStarted);
        }
        let twice_layer_width = layer_width
            .checked_mul(2)
            .ok_or(WaveFieldError::InvalidParameter)?;
        if layer_width == 0
            || twice_layer_width >= self.width.min(self.height)
            || !maximum_damping.is_finite()
            || maximum_damping <= 0.0
            || !(maximum_damping * self.time_step).is_finite()
        {
            return Err(WaveFieldError::InvalidParameter);
        }

        let side_damping = |distance: usize| {
            if distance >= layer_width {
                0.0
            } else {
                let depth = (layer_width - distance) as f64 / layer_width as f64;
                maximum_damping * depth * depth
            }
        };
        let mut damping = vec![0.0; self.current.len()];
        for y in 1..self.height - 1 {
            for x in 1..self.width - 1 {
                let value = side_damping(x)
                    + side_damping(self.width - 1 - x)
                    + side_damping(y)
                    + side_damping(self.height - 1 - y);
                if !value.is_finite() {
                    return Err(WaveFieldError::InvalidParameter);
                }
                damping[self.index(x, y)] = value;
            }
        }

        self.boundary_damping = damping;
        self.boundary_absorbed_energy = 0.0;
        Ok(())
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
    /// In a source-free, undamped control volume, the region's energy
    /// change is its net integrated boundary flux.
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
        source: &mut SourceInstance,
        displacement: &[f64],
        velocity: &[f64],
    ) -> Result<f64, WaveFieldError> {
        if self.substeps_elapsed != 0 {
            return Err(WaveFieldError::AlreadyStarted);
        }
        self.inject_profile_at_current_time(source, displacement, velocity)
    }

    /// Applies a source profile at the field's current logical time.
    ///
    /// Call only between completed substeps. The signed transfer is settled
    /// against the exact change in the discrete leapfrog invariant, so a
    /// scheduled emission can be funded at its activation time rather than
    /// being inserted into the initial condition.
    pub(crate) fn inject_profile_at_current_time(
        &mut self,
        source: &mut SourceInstance,
        displacement: &[f64],
        velocity: &[f64],
    ) -> Result<f64, WaveFieldError> {
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
            initial_velocity[index] = (self.current[index] - self.previous[index]) / self.time_step;
            next_current[index] += displacement[index];
        }
        for y in 1..self.height - 1 {
            for x in 1..self.width - 1 {
                let index = self.index(x, y);
                let damping_factor = 1.0 + 0.5 * self.boundary_damping[index] * self.time_step;
                let centered_acceleration = 0.5
                    * self.time_step
                    * self.propagation_speed.powi(2)
                    * self.laplacian(&self.current, x, y);
                initial_velocity[index] = (initial_velocity[index] + centered_acceleration)
                    / damping_factor
                    + velocity[index];
            }
        }

        let acceleration_scale = 0.5 * self.time_step.powi(2) * self.propagation_speed.powi(2);
        let mut next_previous = next_current.clone();
        for y in 1..self.height - 1 {
            for x in 1..self.width - 1 {
                let index = self.index(x, y);
                let damping_factor = 1.0 + 0.5 * self.boundary_damping[index] * self.time_step;
                next_previous[index] = next_current[index]
                    - self.time_step * damping_factor * initial_velocity[index]
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
        if !next_energy.is_finite() || next_energy < 0.0 || !signed_transfer.is_finite() {
            return Err(WaveFieldError::InvalidState);
        }
        let settlement = source
            .account()
            .prepare_field_transfer(signed_transfer)
            .map_err(|error| match error {
                SourceAccountError::InsufficientEnergy => WaveFieldError::InsufficientSourceEnergy,
                SourceAccountError::InvalidTransfer => WaveFieldError::InvalidState,
                _ => WaveFieldError::InvalidState,
            })?;

        // Both updates are now fully validated; the following assignments cannot fail.
        source
            .account_mut()
            .commit_field_transfer(settlement)
            .map_err(|_| WaveFieldError::InvalidState)?;
        self.previous = next_previous;
        self.current = next_current;
        Ok(signed_transfer)
    }

    /// Injects a compact-support radial velocity pulse as a mathematical test source.
    pub(crate) fn inject_compact_pulse(
        &mut self,
        source: &mut SourceInstance,
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

    /// Emits a compact radial velocity pulse at the current logical time.
    /// Unlike inject_compact_pulse, this is valid after propagation has begun.
    pub(crate) fn inject_compact_pulse_at_current_time(
        &mut self,
        source: &mut SourceInstance,
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
        self.inject_profile_at_current_time(source, &displacement, &velocity)
    }

    /// Advances one leapfrog substep with one prescribed acceleration profile.
    ///
    /// The profile is an acceleration term in the wave equation, not incident
    /// power density. Its signed discrete work is computed from the actual
    /// centered midpoint velocity and settled atomically against this source.
    /// The identity is exact for this centered update, including the configured
    /// diagonal sponge damping:
    ///   delta(field energy) = source work - absorbed energy.
    /// This remains an isolated mathematical forcing experiment.
    pub(crate) fn advance_forced_substep(
        &mut self,
        source: &mut SourceInstance,
        forcing: &[f64],
    ) -> Result<f64, WaveFieldError> {
        if forcing.len() != self.current.len()
            || forcing.iter().any(|value| !value.is_finite())
        {
            return Err(WaveFieldError::InvalidProfile);
        }
        for y in 0..self.height {
            for x in 0..self.width {
                if x == 0 || y == 0 || x + 1 == self.width || y + 1 == self.height {
                    if forcing[self.index(x, y)] != 0.0 {
                        return Err(WaveFieldError::InvalidProfile);
                    }
                }
            }
        }

        let dt = self.time_step;
        let wave_step_squared = self.propagation_speed.powi(2) * dt.powi(2);
        let old_energy = self.energy();
        let mut next = vec![0.0; self.current.len()];
        let mut source_work_sum = 0.0;
        let mut absorbed_sum = 0.0;

        for y in 1..self.height - 1 {
            for x in 1..self.width - 1 {
                let index = self.index(x, y);
                let damping_step = 0.5 * self.boundary_damping[index] * dt;
                next[index] = (2.0 * self.current[index]
                    - (1.0 - damping_step) * self.previous[index]
                    + wave_step_squared * self.laplacian(&self.current, x, y)
                    + dt.powi(2) * forcing[index])
                    / (1.0 + damping_step);

                let midpoint_velocity = (next[index] - self.previous[index]) / (2.0 * dt);
                source_work_sum += midpoint_velocity * forcing[index];
                absorbed_sum += self.boundary_damping[index]
                    * dt
                    * midpoint_velocity
                    * midpoint_velocity;
            }
        }

        if next.iter().any(|value| !value.is_finite()) {
            return Err(WaveFieldError::InvalidState);
        }

        let source_work =
            self.energy_normalization * dt * self.spacing.powi(2) * source_work_sum;
        let absorbed_this_step =
            self.energy_normalization * self.spacing.powi(2) * absorbed_sum;
        let next_energy = self.discrete_energy(&self.current, &next);
        let measured_change = next_energy - old_energy;
        let expected_change = source_work - absorbed_this_step;
        let accounting_tolerance = 1.0e-10
            * old_energy
                .abs()
                .max(next_energy.abs())
                .max(source_work.abs())
                .max(absorbed_this_step.abs())
                .max(1.0);
        let next_absorbed_energy = self.boundary_absorbed_energy + absorbed_this_step;

        if !source_work.is_finite()
            || !absorbed_this_step.is_finite()
            || absorbed_this_step < 0.0
            || !next_energy.is_finite()
            || next_energy < 0.0
            || !measured_change.is_finite()
            || !expected_change.is_finite()
            || (measured_change - expected_change).abs() > accounting_tolerance
            || !next_absorbed_energy.is_finite()
        {
            return Err(WaveFieldError::InvalidState);
        }

        let next_substeps_elapsed = self
            .substeps_elapsed
            .checked_add(1)
            .ok_or(WaveFieldError::CounterOverflow)?;
        let settlement = source
            .account()
            .prepare_field_transfer(source_work)
            .map_err(|error| match error {
                SourceAccountError::InsufficientEnergy => WaveFieldError::InsufficientSourceEnergy,
                SourceAccountError::InvalidTransfer => WaveFieldError::InvalidState,
                _ => WaveFieldError::InvalidState,
            })?;

        source
            .account_mut()
            .commit_field_transfer(settlement)
            .map_err(|_| WaveFieldError::InvalidState)?;
        self.previous.clone_from(&self.current);
        self.current = next;
        self.boundary_absorbed_energy = next_absorbed_energy;
        self.substeps_elapsed = next_substeps_elapsed;
        Ok(source_work)
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
        let old_energy = self.energy();
        let mut next = vec![0.0; self.current.len()];

        for y in 1..self.height - 1 {
            for x in 1..self.width - 1 {
                let index = self.index(x, y);
                let damping_step = 0.5 * self.boundary_damping[index] * self.time_step;
                next[index] = (2.0 * self.current[index]
                    - (1.0 - damping_step) * self.previous[index]
                    + wave_step_squared * self.laplacian(&self.current, x, y))
                    / (1.0 + damping_step);
            }
        }

        if next.iter().any(|value| !value.is_finite()) {
            return Err(WaveFieldError::InvalidState);
        }

        // Multiplying the centered-damping recurrence by the centered
        // velocity gives this exact discrete energy loss for each step.
        // The spatially varying damping coefficient is diagonal, so it
        // contributes no untracked cross-cell work.
        let mut absorbed_this_step = 0.0;
        for y in 1..self.height - 1 {
            for x in 1..self.width - 1 {
                let index = self.index(x, y);
                let damping = self.boundary_damping[index];
                if damping > 0.0 {
                    let midpoint_velocity =
                        (next[index] - self.previous[index]) / (2.0 * self.time_step);
                    absorbed_this_step +=
                        damping * self.time_step * midpoint_velocity * midpoint_velocity;
                }
            }
        }
        absorbed_this_step *= self.energy_normalization * self.spacing.powi(2);

        let next_energy = self.discrete_energy(&self.current, &next);
        let measured_loss = old_energy - next_energy;
        let accounting_tolerance = 1.0e-10
            * old_energy
                .abs()
                .max(next_energy.abs())
                .max(absorbed_this_step.abs())
                .max(1.0);
        let next_absorbed_energy = self.boundary_absorbed_energy + absorbed_this_step;
        if !next_energy.is_finite()
            || next_energy < 0.0
            || !absorbed_this_step.is_finite()
            || absorbed_this_step < 0.0
            || !measured_loss.is_finite()
            || measured_loss < -accounting_tolerance
            || (measured_loss - absorbed_this_step).abs() > accounting_tolerance
            || !next_absorbed_energy.is_finite()
        {
            return Err(WaveFieldError::InvalidState);
        }

        self.previous.clone_from(&self.current);
        self.current = next;
        self.boundary_absorbed_energy = next_absorbed_energy;
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
    use crate::source::SourceId;

    fn test_source(energy_budget: f64, id: u64) -> SourceInstance {
        let source_id = SourceId::new(id).expect("valid test source ID");
        let descriptor = super::super::source::SourceDescriptor::new(
            source_id,
            super::super::source::SourceProvenance::MathematicalTest { holder_id: id },
            super::super::source::SourceClass::MathematicalTest,
            super::super::source::SourceGeometry::Disk {
                center_x: 0.0,
                center_y: 0.0,
                radius: 1.0,
            },
            super::super::source::TickSchedule::new(0, None).expect("valid test schedule"),
            super::super::source::SourceWaveform::SinglePulse,
            super::super::source::SourcePowerLaw::MathematicalTestOnly,
        )
        .expect("valid mathematical test-source descriptor");
        let account = super::super::source::SourceEnergyAccount::new(source_id, energy_budget)
            .expect("valid finite test-source budget");
        SourceInstance::new(descriptor, account).expect("matching descriptor and account")
    }

    fn solver(width: usize, height: usize, spacing: f64, substeps: usize) -> WaveField {
        WaveField::new(width, height, spacing, 1.0, substeps, 1.0).expect("stable reference solver")
    }

    fn pulse(field: &mut WaveField, source: &mut SourceInstance, x: f64, y: f64, amplitude: f64) {
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
    fn prescribed_forcing_matches_exact_discrete_work_identity() {
        let mut field = solver(81, 81, 1.0, 2);
        let mut source = test_source(100.0, 101);
        let mut forcing = vec![0.0; field.width() * field.height()];
        for y in 35..=45 {
            for x in 35..=45 {
                let dx = x as f64 - 40.0;
                let dy = y as f64 - 40.0;
                forcing[field.index(x, y)] = 0.03 * (-(dx * dx + dy * dy) / 18.0).exp();
            }
        }

        let before = field.energy();
        let work = field.advance_forced_substep(&mut source, &forcing).unwrap();
        let after = field.energy();

        assert!(work > 0.0);
        assert!((after - before - work).abs() < 1.0e-10);
        assert!((source.account().cumulative_field_transfer() - work).abs() < 1.0e-12);
        assert!(
            (after + source.account().remaining_energy() - source.account().initial_energy()).abs()
                < 1.0e-10
        );
    }

    #[test]
    fn prescribed_forcing_returns_energy_when_work_is_negative() {
        let mut field = solver(41, 41, 1.0, 2);
        let mut source = test_source(0.0, 102);
        let mut forcing = vec![0.0; field.width() * field.height()];
        let center = field.index(20, 20);
        field.current[center] = 0.0;
        field.previous[center] = 1.0;
        // The existing field is moving opposite the positive applied force.
        forcing[center] = 1.0;

        let before = field.energy();
        let work = field.advance_forced_substep(&mut source, &forcing).unwrap();
        let after = field.energy();

        assert!(work < 0.0, "expected signed work to return energy, got {work}");
        assert!((after - before - work).abs() < 1.0e-10);
        assert!((source.account().remaining_energy() + work).abs() < 1.0e-10);
    }

    #[test]
    fn prescribed_forcing_accounts_for_work_and_sponge_absorption_together() {
        let mut field = solver(81, 81, 1.0, 2);
        field.configure_absorbing_boundary(12, 0.15).unwrap();
        let mut source = test_source(100.0, 103);
        let mut forcing = vec![0.0; field.width() * field.height()];
        for y in 35..=45 {
            for x in 4..=8 {
                forcing[field.index(x, y)] = 0.02;
            }
        }

        let before = field.energy();
        let absorbed_before = field.boundary_absorbed_energy();
        let work = field.advance_forced_substep(&mut source, &forcing).unwrap();
        let after = field.energy();
        let absorbed = field.boundary_absorbed_energy() - absorbed_before;

        assert!(work > 0.0);
        assert!(absorbed > 0.0, "the forced region overlaps the sponge layer");
        assert!((after - before - work + absorbed).abs() < 1.0e-10);
    }

    #[test]
    fn insufficient_forcing_budget_rolls_back_field_and_source() {
        let mut field = solver(41, 41, 1.0, 2);
        let mut source = test_source(0.0, 104);
        let mut forcing = vec![0.0; field.width() * field.height()];
        forcing[field.index(20, 20)] = 1.0;
        let previous = field.previous.clone();
        let current = field.current.clone();
        let energy = field.energy();

        assert_eq!(
            field.advance_forced_substep(&mut source, &forcing),
            Err(WaveFieldError::InsufficientSourceEnergy)
        );
        assert_eq!(field.previous, previous);
        assert_eq!(field.current, current);
        assert_eq!(field.energy(), energy);
        assert_eq!(source.account().remaining_energy(), 0.0);
        assert_eq!(source.account().cumulative_field_transfer(), 0.0);
    }

    #[test]
    fn prescribed_forcing_rejects_nonzero_fixed_boundary_values() {
        let mut field = solver(21, 21, 1.0, 2);
        let mut source = test_source(10.0, 105);
        let mut forcing = vec![0.0; field.width() * field.height()];
        forcing[field.index(0, 10)] = 1.0;
        assert_eq!(
            field.advance_forced_substep(&mut source, &forcing),
            Err(WaveFieldError::InvalidProfile)
        );
        assert_eq!(field.substeps_elapsed, 0);
        assert_eq!(field.energy(), 0.0);
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
    fn rejects_parameters_that_overflow_or_underflow_numerical_energy_scales() {
        assert_eq!(
            WaveField::new(3, 3, 1.0, 1.0e-200, 2, 1.0).err(),
            Some(WaveFieldError::InvalidParameter)
        );
        assert_eq!(
            WaveField::new(3, 3, 1.0, 2.0, 4, 1.0e308).err(),
            Some(WaveFieldError::InvalidParameter)
        );
        assert_eq!(
            WaveField::new(3, 3, 1.0e-100, 1.0, 4, 1.0e-200).err(),
            Some(WaveFieldError::InvalidParameter)
        );
    }

    #[test]
    fn field_evolution_remains_bounded_at_and_below_the_cfl_limit() {
        for (speed, substeps, use_smooth_mode) in [
            // At the exact theoretical CFL limit, the finite fixed-zero
            // grid's highest-frequency mode is near-marginal. Broadband data
            // can strongly amplify that component, so exercise a resolved
            // low-frequency eigenmode at the endpoint.
            (2.0 * std::f64::consts::FRAC_1_SQRT_2, 2, true),
            // Below the limit, also exercise broadband initial data.
            (2.0 * std::f64::consts::FRAC_1_SQRT_2, 3, false),
        ] {
            let mut field = WaveField::new(65, 65, 1.0, speed, substeps, 1.0).unwrap();
            if use_smooth_mode {
                let profile = mode_profile(65, 65, 1.0, 64.0, 3, 2);
                field.current.copy_from_slice(&profile);
                field.previous.copy_from_slice(&profile);
            } else {
                for y in 1..field.height - 1 {
                    for x in 1..field.width - 1 {
                        let phase = (x * 17 + y * 31) as f64;
                        let value = (phase * 0.173).sin() + 0.5 * (phase * 0.071).cos();
                        let index = field.index(x, y);
                        field.current[index] = value;
                        field.previous[index] = value;
                    }
                }
            }

            let initial_energy = field.energy();
            let initial_maximum = field
                .current
                .iter()
                .map(|value| value.abs())
                .fold(0.0, f64::max);
            field.advance_ticks(500).unwrap();

            let final_energy = field.energy();
            let final_maximum = field
                .current
                .iter()
                .map(|value| value.abs())
                .fold(0.0, f64::max);
            let relative_drift = (final_energy - initial_energy).abs() / initial_energy;

            assert!(
                relative_drift < 1.0e-8,
                "speed={speed}, substeps={substeps}, relative energy drift={relative_drift}"
            );
            assert!(
                final_maximum < initial_maximum * 2.0,
                "speed={speed}, substeps={substeps}, amplitude grew from {initial_maximum} to {final_maximum}"
            );
            assert!(field.current.iter().all(|value| value.is_finite()));
        }
    }

    #[test]
    fn finite_source_transfer_is_atomic_and_field_energy_is_conserved() {
        let mut field = solver(201, 201, 1.0, 2);
        let mut insufficient = test_source(0.001, 1);
        assert_eq!(
            field.inject_compact_pulse(&mut insufficient, 100.0, 100.0, 3.0, 0.5),
            Err(WaveFieldError::InsufficientSourceEnergy)
        );
        assert_eq!(field.energy(), 0.0);
        assert_eq!(insufficient.account().remaining_energy(), 0.001);
        assert_eq!(insufficient.account().cumulative_field_transfer(), 0.0);

        let mut source = test_source(10.0, 2);
        let initial_transfer = field
            .inject_compact_pulse(&mut source, 100.0, 100.0, 3.0, 0.5)
            .unwrap();
        assert!(initial_transfer > 0.0);
        assert!((field.energy() - initial_transfer).abs() < 1.0e-12);
        assert!(
            (field.energy() + source.account().remaining_energy()
                - source.account().initial_energy())
            .abs()
                < 1.0e-12
        );

        let initial_energy = field.energy();
        field.advance_ticks(40).unwrap();
        let final_energy = field.energy();
        let relative_drift = (final_energy - initial_energy).abs() / initial_energy;
        assert!(
            relative_drift < 1.0e-10,
            "relative energy drift: {relative_drift}"
        );
        assert_eq!(
            source.account().cumulative_field_transfer(),
            initial_transfer
        );
    }

    #[test]
    fn source_work_accounts_for_constructive_interference_cross_energy() {
        let mut field = solver(201, 201, 1.0, 2);
        let mut first_source = test_source(10.0, 3);
        let mut second_source = test_source(10.0, 4);

        let first_transfer = field
            .inject_compact_pulse(&mut first_source, 100.0, 100.0, 3.0, 0.5)
            .unwrap();
        let second_transfer = field
            .inject_compact_pulse(&mut second_source, 100.0, 100.0, 3.0, 0.5)
            .unwrap();

        // Identical in-phase profiles double the field amplitude, so total
        // field energy quadruples. The second source therefore supplies the
        // isolated first-pulse energy plus the full positive cross term.
        assert!(first_transfer > 0.0);
        assert!((second_transfer - 3.0 * first_transfer).abs() < 1.0e-12);
        assert!((field.energy() - 4.0 * first_transfer).abs() < 1.0e-12);
        assert!(
            (field.energy()
                + first_source.account().remaining_energy()
                + second_source.account().remaining_energy()
                - first_source.account().initial_energy()
                - second_source.account().initial_energy())
            .abs()
                < 1.0e-12
        );
        assert!(
            (first_source.account().cumulative_field_transfer() - first_transfer).abs() < 1.0e-12
        );
        assert!(
            (second_source.account().cumulative_field_transfer() - second_transfer).abs() < 1.0e-12
        );
    }

    #[test]
    fn destructive_interference_returns_field_energy_to_the_second_source() {
        let mut field = solver(201, 201, 1.0, 2);
        let mut emitter = test_source(10.0, 5);
        let mut absorber = test_source(0.0, 6);
        let mut velocity = vec![0.0; field.width() * field.height()];
        let center = field.index(100, 100);
        velocity[center] = 1.0;

        let emitted = field
            .inject_initial_profile(&mut emitter, &vec![0.0; velocity.len()], &velocity)
            .unwrap();
        assert!(emitted > 0.0);
        assert!((field.energy() - emitted).abs() < 1.0e-12);

        for value in &mut velocity {
            *value = -*value;
        }
        let returned = field
            .inject_initial_profile(&mut absorber, &vec![0.0; velocity.len()], &velocity)
            .unwrap();

        assert!((returned + emitted).abs() < 1.0e-12);
        assert!(field.energy().abs() < 1.0e-12);
        assert!(
            (emitter.account().remaining_energy() + absorber.account().remaining_energy() - 10.0)
                .abs()
                < 1.0e-12
        );
        assert!((absorber.account().remaining_energy() - emitted).abs() < 1.0e-12);
        assert!(
            (emitter.account().cumulative_field_transfer()
                + absorber.account().cumulative_field_transfer())
            .abs()
                < 1.0e-12
        );
    }

    #[test]
    fn scheduled_source_emission_begins_propagating_at_its_activation_time() {
        let mut field = solver(201, 201, 1.0, 2);
        let mut source = test_source(10.0, 7);

        field.advance_ticks(10).unwrap();
        assert_eq!(field.logical_time_ticks(), 10.0);
        let transfer = field
            .inject_compact_pulse_at_current_time(&mut source, 100.0, 100.0, 3.0, 0.5)
            .unwrap();
        assert!(transfer > 0.0);
        assert!((field.energy() - transfer).abs() < 1.0e-12);

        field.advance_ticks(10).unwrap();
        assert_eq!(field.sample(124, 100).unwrap(), 0.0);
        field.advance_ticks(10).unwrap();
        assert!(field.sample(124, 100).unwrap().abs() < 1.0e-2);
        field.advance_ticks(4).unwrap();
        assert!(field.sample(124, 100).unwrap().abs() > 1.0e-2);
        assert_eq!(field.logical_time_ticks(), 34.0);
        assert!(
            (field.energy() + source.account().remaining_energy()
                - source.account().initial_energy())
            .abs()
                < 1.0e-10
        );
        assert!((source.account().cumulative_field_transfer() - transfer).abs() < 1.0e-12);
    }

    #[test]
    fn scheduled_source_emission_preserves_centered_velocity_inside_sponge_layer() {
        let mut field = solver(201, 201, 1.0, 2);
        field.configure_absorbing_boundary(72, 0.2).unwrap();
        let mut source = test_source(10.0, 8);

        field.advance_ticks(10).unwrap();
        let transfer = field
            .inject_compact_pulse_at_current_time(&mut source, 150.0, 100.0, 3.0, 0.5)
            .unwrap();
        assert!(transfer > 0.0);

        let center = field.index(150, 100);
        let previous_at_emission = field.previous[center];
        let expected_centered_velocity = 0.5;
        let energy_after_emission = field.energy();
        field.advance_substeps(1).unwrap();

        let measured_centered_velocity =
            (field.current[center] - previous_at_emission) / (2.0 * field.time_step());
        assert!(
            (measured_centered_velocity - expected_centered_velocity).abs() < 1.0e-12,
            "requested centered velocity={}, measured={measured_centered_velocity}",
            expected_centered_velocity
        );
        assert!(
            (field.energy()
                + field.boundary_absorbed_energy()
                + source.account().remaining_energy()
                - source.account().initial_energy())
            .abs()
                < 1.0e-10
        );
        assert!(energy_after_emission > 0.0);
    }

    #[test]
    fn compact_pulse_has_finite_travel_time_and_arrives_near_distance_over_speed() {
        let mut field = solver(201, 201, 1.0, 2);
        let mut source = test_source(10.0, 9);
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
        let mut source = test_source(10.0, 10);
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
        let mut source = test_source(10.0, 11);
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
            let mut source = test_source(10.0, 12);
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
    fn standing_wave_matches_its_discrete_mode_frequency_over_many_cycles() {
        let mut field = solver(65, 65, 0.5, 4);
        let length = 32.0;
        let mode_x = 3;
        let mode_y = 2;
        let mode = mode_profile(65, 65, 0.5, length, mode_x, mode_y);
        let spatial_omega = spatial_mode_frequency(0.5, 1.0, length, mode_x, mode_y);
        let temporal_omega =
            (1.0 - 0.5 * (spatial_omega * field.time_step()).powi(2)).acos() / field.time_step();

        field.current.clone_from(&mode);
        let previous_phase = (temporal_omega * field.time_step()).cos();
        for (previous, value) in field.previous.iter_mut().zip(mode.iter()) {
            *previous = *value * previous_phase;
        }

        let initial_energy = field.energy();
        field.advance_ticks(256).unwrap();
        let time = field.logical_time_ticks();
        let expected_factor = (temporal_omega * time).cos();
        let maximum_error = field
            .current
            .iter()
            .zip(mode.iter())
            .map(|(actual, initial)| (actual - initial * expected_factor).abs())
            .fold(0.0, f64::max);
        let final_energy = field.energy();
        let relative_energy_drift = (final_energy - initial_energy).abs() / initial_energy;

        assert!(
            maximum_error < 1.0e-9,
            "standing-mode maximum error after {time} ticks: {maximum_error}"
        );
        assert!(
            relative_energy_drift < 1.0e-10,
            "standing-mode relative energy drift: {relative_energy_drift}"
        );
    }

    #[test]
    fn standing_mode_remains_undisturbed_interior_to_absorbing_edges() {
        let mut field = solver(321, 321, 1.0, 2);
        field.configure_absorbing_boundary(72, 0.2).unwrap();

        let length = 320.0;
        let mode_x = 21;
        let mode_y = 17;
        let mode = mode_profile(321, 321, 1.0, length, mode_x, mode_y);
        let spatial_omega = spatial_mode_frequency(1.0, 1.0, length, mode_x, mode_y);
        let temporal_omega =
            (1.0 - 0.5 * (spatial_omega * field.time_step()).powi(2)).acos() / field.time_step();

        field.current.clone_from(&mode);
        let previous_phase = (temporal_omega * field.time_step()).cos();
        for (previous, value) in field.previous.iter_mut().zip(mode.iter()) {
            *previous = *value * previous_phase;
        }

        field.advance_ticks(40).unwrap();
        let expected_factor = (temporal_omega * 40.0).cos();
        let mut maximum_error = 0.0_f64;
        for y in 158..=162 {
            for x in 158..=162 {
                let index = field.index(x, y);
                maximum_error =
                    maximum_error.max((field.current[index] - mode[index] * expected_factor).abs());
            }
        }

        assert!(
            maximum_error < 1.0e-10,
            "interior standing-mode error before boundary influence: {maximum_error}"
        );
        assert!(
            field.boundary_absorbed_energy() > 0.0,
            "the outer sponge should absorb energy while the interior mode remains undisturbed"
        );
    }

    #[test]
    fn empty_space_energy_remains_bounded_over_a_long_reflection_run() {
        let mut field = solver(81, 81, 1.0, 2);
        let mut source = test_source(10.0, 13);
        pulse(&mut field, &mut source, 40.0, 40.0, 0.5);

        let initial_energy = field.energy();
        field.advance_ticks(500).unwrap();
        let final_energy = field.energy();
        let relative_drift = (final_energy - initial_energy).abs() / initial_energy;

        assert!(
            relative_drift < 1.0e-9,
            "relative energy drift after 500 ticks: {relative_drift}"
        );
        assert!(
            (field.energy() + source.account().remaining_energy()
                - source.account().initial_energy())
            .abs()
                < 1.0e-10
        );
    }

    #[test]
    fn returned_field_work_is_credited_to_the_active_source() {
        let mut field = solver(81, 81, 1.0, 2);
        let mut emitter = test_source(10.0, 14);
        let mut absorber = test_source(0.0, 15);
        let emitted = field
            .inject_compact_pulse(&mut emitter, 40.0, 40.0, 3.0, 0.5)
            .unwrap();
        let returned = field
            .inject_compact_pulse(&mut absorber, 40.0, 40.0, 3.0, -0.5)
            .unwrap();

        assert!(emitted > 0.0);
        assert!((emitted + returned).abs() < 1.0e-12);
        assert!(field.energy().abs() < 1.0e-12);
        assert!((absorber.account().remaining_energy() - emitted).abs() < 1.0e-12);
    }

    #[test]
    fn computational_window_does_not_affect_interior_before_boundary_influence() {
        let mut smaller = solver(201, 201, 1.0, 2);
        let mut larger = solver(241, 241, 1.0, 2);
        let mut small_source = test_source(10.0, 16);
        let mut large_source = test_source(10.0, 17);
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
        let mut first_source = test_source(10.0, 18);
        let mut second_source = test_source(10.0, 19);
        let mut constructive_a = test_source(10.0, 20);
        let mut constructive_b = test_source(10.0, 21);
        let mut destructive_a = test_source(10.0, 22);
        let mut destructive_b = test_source(10.0, 23);

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
            let mut source = test_source(10.0, 24);
            let center_world = center as f64 * spacing;
            pulse(&mut field, &mut source, center_world, center_world, 0.5);
            field.advance_ticks(10).unwrap();

            let radius = 10.0;
            let angular_samples = 32;
            let mut samples = Vec::with_capacity(angular_samples);
            for sample in 0..angular_samples {
                let angle = std::f64::consts::TAU * sample as f64 / angular_samples as f64;
                let offset_x = (radius * angle.cos() / spacing).round() as isize;
                let offset_y = (radius * angle.sin() / spacing).round() as isize;
                let x = (center as isize + offset_x) as usize;
                let y = (center as isize + offset_y) as usize;
                samples.push(field.sample(x, y).unwrap());
            }

            let mean = samples.iter().sum::<f64>() / samples.len() as f64;
            let variance = samples
                .iter()
                .map(|value| (value - mean).powi(2))
                .sum::<f64>()
                / samples.len() as f64;
            let mean_square =
                samples.iter().map(|value| value.powi(2)).sum::<f64>() / samples.len() as f64;
            variance.sqrt() / mean_square.sqrt()
        }

        let coarse = anisotropy_error(1.0, 2);
        let medium = anisotropy_error(0.5, 4);
        let fine = anisotropy_error(0.25, 8);
        assert!(coarse > medium * 1.5, "coarse={coarse}, medium={medium}");
        assert!(medium > fine * 1.1, "medium={medium}, fine={fine}");
    }

    fn outgoing_wave_packet(
        field: &mut WaveField,
        angle: f64,
        wave_number: f64,
        center_offset: f64,
        envelope_radius: f64,
    ) -> (usize, usize, f64) {
        let center_x = (field.width / 2) as f64 * field.spacing - center_offset * angle.cos();
        let center_y = (field.height / 2) as f64 * field.spacing - center_offset * angle.sin();
        let spatial_eigenvalue = 4.0 / field.spacing.powi(2)
            * ((0.5 * wave_number * field.spacing * angle.cos())
                .sin()
                .powi(2)
                + (0.5 * wave_number * field.spacing * angle.sin())
                    .sin()
                    .powi(2));
        let spatial_omega = field.propagation_speed * spatial_eigenvalue.sqrt();
        let temporal_omega =
            (1.0 - 0.5 * (spatial_omega * field.time_step()).powi(2)).acos() / field.time_step();
        let mut envelope_norm_squared = 0.0;

        for y in 1..field.height - 1 {
            for x in 1..field.width - 1 {
                let dx = x as f64 * field.spacing - center_x;
                let dy = y as f64 * field.spacing - center_y;
                let envelope =
                    (-(dx * dx + dy * dy) / (2.0 * envelope_radius * envelope_radius)).exp();
                let phase = wave_number * (dx * angle.cos() + dy * angle.sin());
                let index = field.index(x, y);
                field.current[index] = envelope * phase.cos();
                // The previous level is the same packet one substep earlier,
                // making its carrier propagate in the selected direction.
                field.previous[index] =
                    envelope * (phase + temporal_omega * field.time_step()).cos();
                envelope_norm_squared += envelope * envelope;
            }
        }

        (
            (center_x / field.spacing).round() as usize,
            (center_y / field.spacing).round() as usize,
            envelope_norm_squared.sqrt(),
        )
    }

    fn local_amplitude_norm(
        field: &WaveField,
        center_x: usize,
        center_y: usize,
        radius: usize,
    ) -> f64 {
        let mut sum = 0.0;
        for y in center_y - radius..=center_y + radius {
            for x in center_x - radius..=center_x + radius {
                sum += field.current[field.index(x, y)].powi(2);
            }
        }
        sum.sqrt()
    }

    #[test]
    fn absorbing_boundary_accounts_for_removed_energy_without_a_hidden_sink() {
        let mut field = solver(201, 201, 1.0, 2);
        field.configure_absorbing_boundary(72, 0.2).unwrap();
        let mut source = test_source(10.0, 25);
        pulse(&mut field, &mut source, 100.0, 100.0, 0.5);
        let emitted_energy = field.energy();

        field.advance_ticks(260).unwrap();

        let remaining = field.energy();
        let absorbed = field.boundary_absorbed_energy();
        assert!(absorbed > emitted_energy * 0.99);
        assert!(
            (remaining + absorbed - emitted_energy).abs() < emitted_energy * 1.0e-9,
            "remaining={remaining}, absorbed={absorbed}, emitted={emitted_energy}"
        );
        assert!(
            (remaining + absorbed + source.account().remaining_energy()
                - source.account().initial_energy())
            .abs()
                < 1.0e-9
        );
    }

    #[test]
    fn absorbing_layer_suppresses_reflection_across_frequencies_and_angles() {
        let cases = [
            (0.0, 0.18),
            (0.0, 0.30),
            (0.0, 0.45),
            (std::f64::consts::FRAC_PI_4, 0.18),
            (std::f64::consts::FRAC_PI_4, 0.30),
            (std::f64::consts::FRAC_PI_4, 0.45),
            (75.0_f64.to_radians(), 0.18),
            (75.0_f64.to_radians(), 0.30),
            (75.0_f64.to_radians(), 0.45),
        ];

        for (angle, wave_number) in cases {
            let mut absorbing = solver(201, 201, 1.0, 2);
            let mut reflective = solver(201, 201, 1.0, 2);
            absorbing.configure_absorbing_boundary(72, 0.2).unwrap();
            let (center_x, center_y, _) =
                outgoing_wave_packet(&mut absorbing, angle, wave_number, 15.0, 9.0);
            let (reflective_x, reflective_y, _) =
                outgoing_wave_packet(&mut reflective, angle, wave_number, 15.0, 9.0);
            absorbing.advance_ticks(260).unwrap();
            reflective.advance_ticks(260).unwrap();

            let absorbed_return = local_amplitude_norm(&absorbing, center_x, center_y, 12);
            let reflected_return =
                local_amplitude_norm(&reflective, reflective_x, reflective_y, 12);
            let return_amplitude_ratio = absorbed_return / reflected_return.max(1.0e-15);
            assert!(
                return_amplitude_ratio < 0.12,
                "angle={angle}, wave_number={wave_number}, residual ratio={return_amplitude_ratio}"
            );
            let remaining_energy = absorbing.energy();
            let absorbed_energy = absorbing.boundary_absorbed_energy();
            assert!(
                absorbed_energy > 0.99 * (absorbed_energy + remaining_energy),
                "angle={angle}, wave_number={wave_number}, absorbed={absorbed_energy}, remaining={remaining_energy}"
            );
        }
    }

    #[test]
    fn absorbing_window_matches_a_larger_window_before_its_reflection_returns() {
        let mut smaller = solver(201, 201, 1.0, 2);
        let mut larger = solver(321, 321, 1.0, 2);
        smaller.configure_absorbing_boundary(72, 0.2).unwrap();
        let (small_x, small_y, initial_norm) =
            outgoing_wave_packet(&mut smaller, 0.0, 0.18, 15.0, 9.0);
        let (large_x, large_y, _) = outgoing_wave_packet(&mut larger, 0.0, 0.18, 15.0, 9.0);

        smaller.advance_ticks(260).unwrap();
        larger.advance_ticks(260).unwrap();

        let mut difference_squared = 0.0;
        for offset_y in -12isize..=12 {
            for offset_x in -12isize..=12 {
                let small = smaller
                    .sample(
                        (small_x as isize + offset_x) as usize,
                        (small_y as isize + offset_y) as usize,
                    )
                    .unwrap();
                let large = larger
                    .sample(
                        (large_x as isize + offset_x) as usize,
                        (large_y as isize + offset_y) as usize,
                    )
                    .unwrap();
                difference_squared += (small - large).powi(2);
            }
        }

        let normalized_error = difference_squared.sqrt() / initial_norm;
        assert!(
            normalized_error < 0.01,
            "absorbing-window error relative to initial packet envelope: {normalized_error}"
        );
    }

    #[test]
    fn absorbing_boundary_retains_low_reflection_when_grid_resolution_is_refined() {
        let mut absorbing = WaveField::new(241, 241, 0.5, 1.0, 4, 1.0).unwrap();
        let mut reflective = WaveField::new(241, 241, 0.5, 1.0, 4, 1.0).unwrap();
        absorbing.configure_absorbing_boundary(88, 0.2).unwrap();

        let angle = std::f64::consts::FRAC_PI_4;
        let (center_x, center_y, _) = outgoing_wave_packet(&mut absorbing, angle, 0.18, 7.5, 4.5);
        let (reflective_x, reflective_y, _) =
            outgoing_wave_packet(&mut reflective, angle, 0.18, 7.5, 4.5);

        absorbing.advance_ticks(180).unwrap();
        reflective.advance_ticks(180).unwrap();

        let absorbed_return = local_amplitude_norm(&absorbing, center_x, center_y, 24);
        let reflected_return = local_amplitude_norm(&reflective, reflective_x, reflective_y, 24);
        let return_amplitude_ratio = absorbed_return / reflected_return.max(1.0e-15);
        assert!(
            return_amplitude_ratio < 0.03,
            "refined-grid return-amplitude ratio: {return_amplitude_ratio}"
        );

        let remaining_energy = absorbing.energy();
        let absorbed_energy = absorbing.boundary_absorbed_energy();
        assert!(
            absorbed_energy > 0.99 * (absorbed_energy + remaining_energy),
            "refined-grid absorbed={absorbed_energy}, remaining={remaining_energy}"
        );
    }

    #[test]
    fn identical_runs_replay_bit_for_bit() {
        let mut left = solver(161, 161, 1.0, 2);
        let mut right = solver(161, 161, 1.0, 2);
        let mut left_source = test_source(10.0, 26);
        let mut right_source = test_source(10.0, 27);
        pulse(&mut left, &mut left_source, 80.0, 80.0, 0.5);
        pulse(&mut right, &mut right_source, 80.0, 80.0, 0.5);
        left.advance_ticks(30).unwrap();
        right.advance_substeps(60).unwrap();
        assert_eq!(left.current, right.current);
        assert_eq!(left.previous, right.previous);
        assert_eq!(left.energy().to_bits(), right.energy().to_bits());
    }
}
