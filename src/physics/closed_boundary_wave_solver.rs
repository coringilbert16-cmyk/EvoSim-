//! Isolated dense reference solver for a finite, closed 2D wave domain.
//!
//! The outer faces are reflecting (zero normal flux). The discretization is
//! cell-centred: only edges between cells inside the domain contribute to the
//! graph Laplacian. No exterior ghost cells, fixed-zero edge nodes, damping,
//! or outflow account are present. This is deliberately independent of the
//! production simulation tick, chemistry, materials, and energy ledger.

/// Deterministic local reconstruction of the discrete field at a world position.
/// Spatial derivatives are node-centred finite differences, interpolated with
/// the same bilinear weights as displacement and velocity. Reflecting faces
/// impose zero normal gradient. Energy and flux are pointwise continuum
/// diagnostics; their spatial sum is not the leapfrog invariant returned by energy().
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ClosedWaveSample {
    pub(crate) position_x: f64,
    pub(crate) position_y: f64,
    pub(crate) displacement_time_ticks: f64,
    pub(crate) velocity_time_ticks: f64,
    pub(crate) displacement: f64,
    pub(crate) time_derivative: f64,
    pub(crate) gradient_x: f64,
    pub(crate) gradient_y: f64,
    pub(crate) energy_density: f64,
    pub(crate) flux_x: f64,
    pub(crate) flux_y: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ClosedWaveError {
    InvalidDimensions,
    InvalidParameter,
    UnstableTimeStep,
    InvalidState,
    CounterOverflow,
}

#[derive(Clone, Debug)]
pub(crate) struct ClosedBoundaryWaveField {
    width: usize,
    height: usize,
    spacing: f64,
    propagation_speed: f64,
    time_step: f64,
    substeps_per_tick: usize,
    normalization: f64,
    previous: Vec<f64>,
    current: Vec<f64>,
    substeps_elapsed: u64,
}

impl ClosedBoundaryWaveField {
    pub(crate) fn new(
        width: usize,
        height: usize,
        spacing: f64,
        propagation_speed: f64,
        substeps_per_tick: usize,
        normalization: f64,
    ) -> Result<Self, ClosedWaveError> {
        if width < 2 || height < 2 {
            return Err(ClosedWaveError::InvalidDimensions);
        }
        if !spacing.is_finite()
            || spacing <= 0.0
            || !propagation_speed.is_finite()
            || propagation_speed <= 0.0
            || substeps_per_tick == 0
            || !normalization.is_finite()
            || normalization <= 0.0
        {
            return Err(ClosedWaveError::InvalidParameter);
        }
        let cell_count = width
            .checked_mul(height)
            .ok_or(ClosedWaveError::InvalidDimensions)?;
        let time_step = 1.0 / substeps_per_tick as f64;
        let courant = propagation_speed * time_step / spacing;
        if !time_step.is_finite()
            || !courant.is_finite()
            || courant > std::f64::consts::FRAC_1_SQRT_2
        {
            return Err(ClosedWaveError::UnstableTimeStep);
        }
        Ok(Self {
            width,
            height,
            spacing,
            propagation_speed,
            time_step,
            substeps_per_tick,
            normalization,
            previous: vec![0.0; cell_count],
            current: vec![0.0; cell_count],
            substeps_elapsed: 0,
        })
    }

    pub(crate) fn dimensions(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    pub(crate) fn logical_time_ticks(&self) -> f64 {
        self.substeps_elapsed as f64 / self.substeps_per_tick as f64
    }

    /// Query the field at grid coordinates measured in cell-spacing units.
    /// Coordinates outside the finite domain or non-finite positions fail closed.
    pub(crate) fn query(&self, x: f64, y: f64) -> Result<ClosedWaveSample, ClosedWaveError> {
        if !x.is_finite()
            || !y.is_finite()
            || x < 0.0
            || y < 0.0
            || x > (self.width - 1) as f64
            || y > (self.height - 1) as f64
        {
            return Err(ClosedWaveError::InvalidState);
        }
        let gradients = self.node_gradients();
        let displacement = self.interpolate(&self.current, x, y);
        let velocity_nodes: Vec<f64> = self
            .current
            .iter()
            .zip(&self.previous)
            .map(|(now, before)| (now - before) / self.time_step)
            .collect();
        let time_derivative = self.interpolate(&velocity_nodes, x, y);
        let gradient_x = self.interpolate(&gradients.0, x, y);
        let gradient_y = self.interpolate(&gradients.1, x, y);
        let energy_density = 0.5
            * self.normalization
            * (time_derivative.powi(2)
                + self.propagation_speed.powi(2) * (gradient_x.powi(2) + gradient_y.powi(2)));
        let flux_scale = -self.normalization * self.propagation_speed.powi(2) * time_derivative;
        Ok(ClosedWaveSample {
            position_x: x * self.spacing,
            position_y: y * self.spacing,
            // Leapfrog velocity is centred between the previous/current states.
            displacement_time_ticks: self.substeps_elapsed as f64 / self.substeps_per_tick as f64,
            velocity_time_ticks: (self.substeps_elapsed as f64 - 0.5)
                / self.substeps_per_tick as f64,
            displacement,
            time_derivative,
            gradient_x,
            gradient_y,
            energy_density,
            flux_x: flux_scale * gradient_x,
            flux_y: flux_scale * gradient_y,
        })
    }

    fn node_gradients(&self) -> (Vec<f64>, Vec<f64>) {
        let mut gx = vec![0.0; self.current.len()];
        let mut gy = vec![0.0; self.current.len()];
        for y in 0..self.height {
            for x in 0..self.width {
                let i = self.index(x, y);
                if x > 0 && x + 1 < self.width {
                    gx[i] = (self.current[i + 1] - self.current[i - 1]) / (2.0 * self.spacing);
                }
                if y > 0 && y + 1 < self.height {
                    gy[i] = (self.current[i + self.width] - self.current[i - self.width])
                        / (2.0 * self.spacing);
                }
            }
        }
        (gx, gy)
    }

    fn interpolate(&self, values: &[f64], x: f64, y: f64) -> f64 {
        let x0 = x.floor() as usize;
        let y0 = y.floor() as usize;
        let x1 = (x0 + 1).min(self.width - 1);
        let y1 = (y0 + 1).min(self.height - 1);
        let tx = x - x0 as f64;
        let ty = y - y0 as f64;
        let a = values[self.index(x0, y0)] * (1.0 - tx) + values[self.index(x1, y0)] * tx;
        let b = values[self.index(x0, y1)] * (1.0 - tx) + values[self.index(x1, y1)] * tx;
        a * (1.0 - ty) + b * ty
    }

    pub(crate) fn sample(&self, x: usize, y: usize) -> Option<f64> {
        (x < self.width && y < self.height).then(|| self.current[self.index(x, y)])
    }

    /// Set one cell's initial leapfrog state. Initialization is closed once
    /// the first propagation substep has been taken.
    pub(crate) fn set_initial_state(
        &mut self,
        x: usize,
        y: usize,
        previous: f64,
        current: f64,
    ) -> Result<(), ClosedWaveError> {
        if self.substeps_elapsed != 0 {
            return Err(ClosedWaveError::InvalidState);
        }
        if x >= self.width || y >= self.height || !previous.is_finite() || !current.is_finite() {
            return Err(ClosedWaveError::InvalidState);
        }
        let index = self.index(x, y);
        self.previous[index] = previous;
        self.current[index] = current;
        Ok(())
    }

    /// Discrete leapfrog invariant: kinetic energy plus energy on internal
    /// cell-to-cell edges. There are no edges crossing the physical boundary.
    pub(crate) fn energy(&self) -> f64 {
        let mut kinetic = 0.0;
        for index in 0..self.current.len() {
            let velocity = (self.current[index] - self.previous[index]) / self.time_step;
            kinetic += velocity * velocity;
        }

        let mut gradient_cross = 0.0;
        for y in 0..self.height {
            for x in 0..self.width {
                let index = self.index(x, y);
                if x + 1 < self.width {
                    let next = index + 1;
                    gradient_cross += (self.current[next] - self.current[index])
                        * (self.previous[next] - self.previous[index]);
                }
                if y + 1 < self.height {
                    let next = index + self.width;
                    gradient_cross += (self.current[next] - self.current[index])
                        * (self.previous[next] - self.previous[index]);
                }
            }
        }

        0.5 * self.normalization
            * (kinetic * self.spacing.powi(2) + self.propagation_speed.powi(2) * gradient_cross)
    }

    pub(crate) fn advance_ticks(&mut self, ticks: u64) -> Result<(), ClosedWaveError> {
        let steps = ticks
            .checked_mul(self.substeps_per_tick as u64)
            .ok_or(ClosedWaveError::CounterOverflow)?;
        self.advance_substeps(steps)
    }

    pub(crate) fn advance_substeps(&mut self, steps: u64) -> Result<(), ClosedWaveError> {
        self.substeps_elapsed
            .checked_add(steps)
            .ok_or(ClosedWaveError::CounterOverflow)?;
        for _ in 0..steps {
            self.advance_one_substep()?;
        }
        Ok(())
    }

    fn advance_one_substep(&mut self) -> Result<(), ClosedWaveError> {
        let scale = (self.propagation_speed * self.time_step / self.spacing).powi(2);
        let mut next = vec![0.0; self.current.len()];
        for y in 0..self.height {
            for x in 0..self.width {
                let index = self.index(x, y);
                let center = self.current[index];
                let mut laplacian_numerator = 0.0;
                if x > 0 {
                    laplacian_numerator += self.current[index - 1] - center;
                }
                if x + 1 < self.width {
                    laplacian_numerator += self.current[index + 1] - center;
                }
                if y > 0 {
                    laplacian_numerator += self.current[index - self.width] - center;
                }
                if y + 1 < self.height {
                    laplacian_numerator += self.current[index + self.width] - center;
                }
                next[index] = 2.0 * center - self.previous[index] + scale * laplacian_numerator;
            }
        }
        if next.iter().any(|value| !value.is_finite()) {
            return Err(ClosedWaveError::InvalidState);
        }
        self.previous.clone_from(&self.current);
        self.current = next;
        self.substeps_elapsed += 1;
        Ok(())
    }

    fn index(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f64, expected: f64, tolerance: f64) {
        let scale = actual.abs().max(expected.abs()).max(1.0);
        assert!(
            (actual - expected).abs() <= tolerance * scale,
            "{actual} != {expected} (tolerance {tolerance})"
        );
    }

    #[test]
    fn constant_field_is_stationary_at_all_reflecting_boundaries() {
        let mut field = ClosedBoundaryWaveField::new(17, 11, 1.0, 1.0, 2, 1.0).unwrap();
        for y in 0..11 {
            for x in 0..17 {
                field.set_initial_state(x, y, 2.5, 2.5).unwrap();
            }
        }
        let initial_energy = field.energy();
        field.advance_ticks(200).unwrap();
        for y in 0..11 {
            for x in 0..17 {
                close(field.sample(x, y).unwrap(), 2.5, 1.0e-12);
            }
        }
        close(field.energy(), initial_energy, 1.0e-12);
        close(field.logical_time_ticks(), 200.0, 1.0e-12);
    }

    #[test]
    fn closed_domain_energy_is_conserved_for_edge_and_corner_incident_waves() {
        for (source_x, source_y) in [(1, 5), (8, 1), (15, 9), (8, 5)] {
            let mut field = ClosedBoundaryWaveField::new(17, 11, 1.0, 0.2, 2, 1.0).unwrap();
            for y in 0..11 {
                for x in 0..17 {
                    let radius_squared =
                        (x as f64 - source_x as f64).powi(2) + (y as f64 - source_y as f64).powi(2);
                    let displacement = (-radius_squared / 5.0).exp();
                    field
                        .set_initial_state(x, y, displacement * 0.98, displacement)
                        .unwrap();
                }
            }
            let initial_energy = field.energy();
            assert!(initial_energy.is_finite() && initial_energy > 0.0);
            field.advance_ticks(500).unwrap();
            close(field.energy(), initial_energy, 2.0e-10);
            assert!(field.sample(0, 0).unwrap().is_finite());
        }
    }

    #[test]
    fn reflecting_boundary_does_not_pin_edge_cells_to_zero() {
        let mut field = ClosedBoundaryWaveField::new(9, 9, 1.0, 0.2, 2, 1.0).unwrap();
        field.set_initial_state(1, 4, 0.8, 1.0).unwrap();
        let initial_edge = field.sample(0, 4).unwrap();
        field.advance_ticks(1).unwrap();
        assert_ne!(field.sample(0, 4).unwrap(), initial_edge);
        assert!(field.sample(0, 4).unwrap().is_finite());
    }

    #[test]
    fn query_reconstructs_local_energy_flux_and_reflecting_gradient() {
        let mut field = ClosedBoundaryWaveField::new(7, 7, 0.5, 0.25, 2, 3.0).unwrap();
        for y in 0..7 {
            for x in 0..7 {
                let value = x as f64 * 0.5 + y as f64 * 0.25;
                field.set_initial_state(x, y, value - 0.1, value).unwrap();
            }
        }
        let sample = field.query(3.0, 3.0).unwrap();
        close(sample.displacement, 2.25, 1.0e-12);
        close(sample.gradient_x, 1.0, 1.0e-12);
        close(sample.gradient_y, 0.5, 1.0e-12);
        close(sample.time_derivative, 0.2, 1.0e-12);
        close(
            sample.energy_density,
            0.5 * 3.0 * (0.04 + 0.0625 * 1.25),
            1.0e-12,
        );
        close(sample.flux_x, -3.0 * 0.0625 * 0.2, 1.0e-12);
        let edge = field.query(0.0, 3.0).unwrap();
        close(edge.gradient_x, 0.0, 1.0e-12);
        assert!(edge.flux_x.abs() < 1.0e-12);
        assert_eq!(field.query(-0.01, 2.0), Err(ClosedWaveError::InvalidState));
        assert_eq!(
            field.query(f64::NAN, 2.0),
            Err(ClosedWaveError::InvalidState)
        );
    }

    #[test]
    fn invalid_initial_state_and_unstable_steps_are_rejected() {
        assert_eq!(
            ClosedBoundaryWaveField::new(1, 8, 1.0, 1.0, 2, 1.0).unwrap_err(),
            ClosedWaveError::InvalidDimensions
        );
        assert_eq!(
            ClosedBoundaryWaveField::new(8, 8, 1.0, 1.0, 1, 1.0).unwrap_err(),
            ClosedWaveError::UnstableTimeStep
        );
        let mut field = ClosedBoundaryWaveField::new(8, 8, 1.0, 0.2, 2, 1.0).unwrap();
        assert_eq!(
            field.set_initial_state(8, 0, 0.0, 0.0),
            Err(ClosedWaveError::InvalidState)
        );
        assert_eq!(
            field.set_initial_state(0, 0, f64::NAN, 0.0),
            Err(ClosedWaveError::InvalidState)
        );
    }
}
