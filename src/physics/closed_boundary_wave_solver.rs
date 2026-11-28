//! Dense reference solver for a finite, closed 2D wave domain.
//!
//! The outer faces are reflecting (zero normal flux). The discretization is
//! cell-centred: only edges between cells inside the domain contribute to the
//! graph Laplacian. No exterior ghost cells, damping, or outflow account are
//! present. Simulation can preserve and advance this state, but this closed
//! boundary is not the production open-world field. Production activation
//! remains gated on validated outflow and whole-world energy reconciliation.

use serde::{Deserialize, Deserializer, Serialize};

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
    /// Spatial gradient synchronized to the leapfrog velocity time centre.
    pub(crate) flux_gradient_x: f64,
    pub(crate) flux_gradient_y: f64,
}

/// Combined finite-domain field inventory evaluated at the disturbance's
/// leapfrog velocity time centre. The ambient kinetic and cross terms use
/// values sampled at that same time centre.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CombinedFieldEnergy {
    pub(crate) disturbance_energy: f64,
    pub(crate) ambient_energy: f64,
    pub(crate) cross_energy: f64,
    pub(crate) total_energy: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ClosedWaveError {
    InvalidDimensions,
    InvalidParameter,
    UnstableTimeStep,
    InvalidState,
    CounterOverflow,
    InsufficientLocalFieldEnergy,
    InsufficientMatterEnergy,
    EnergySettlementFailed,
}

#[derive(Clone, Debug, Serialize)]
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
    horizontal_coupling: Vec<f64>,
    vertical_coupling: Vec<f64>,
}

#[derive(Deserialize)]
struct ClosedBoundaryWaveFieldState {
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
    horizontal_coupling: Vec<f64>,
    vertical_coupling: Vec<f64>,
}

impl<'de> Deserialize<'de> for ClosedBoundaryWaveField {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error as _;

        let state = ClosedBoundaryWaveFieldState::deserialize(deserializer)?;
        let field = Self {
            width: state.width,
            height: state.height,
            spacing: state.spacing,
            propagation_speed: state.propagation_speed,
            time_step: state.time_step,
            substeps_per_tick: state.substeps_per_tick,
            normalization: state.normalization,
            previous: state.previous,
            current: state.current,
            substeps_elapsed: state.substeps_elapsed,
            horizontal_coupling: state.horizontal_coupling,
            vertical_coupling: state.vertical_coupling,
        };
        if !field.has_valid_serialized_state() {
            return Err(D::Error::custom(
                "invalid serialized closed wave field state",
            ));
        }
        Ok(field)
    }
}

/// Grid edge orientation for a conservative internal material interface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InternalBoundaryAxis {
    /// Edge joining (x, y) to (x + 1, y).
    Horizontal,
    /// Edge joining (x, y) to (x, y + 1).
    Vertical,
}

/// Lossless interface coupling on one grid edge.
///
/// 1.0 is a transparent edge; 0.0 is a perfectly reflecting/isolating edge.
/// Intermediate values alter the symmetric edge stiffness. This is a
/// solver-level propagation property, not a material absorption coefficient.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct InternalBoundaryEdge {
    pub(crate) x: usize,
    pub(crate) y: usize,
    pub(crate) axis: InternalBoundaryAxis,
    pub(crate) coupling: f64,
}

impl ClosedBoundaryWaveField {
    fn has_valid_serialized_state(&self) -> bool {
        if self.width < 2
            || self.height < 2
            || !self.spacing.is_finite()
            || self.spacing <= 0.0
            || !self.propagation_speed.is_finite()
            || self.propagation_speed <= 0.0
            || self.substeps_per_tick == 0
            || !self.normalization.is_finite()
            || self.normalization <= 0.0
            || !self.time_step.is_finite()
            || self.time_step <= 0.0
            || self.time_step.to_bits() != (1.0 / self.substeps_per_tick as f64).to_bits()
        {
            return false;
        }

        let Some(cell_count) = self.width.checked_mul(self.height) else {
            return false;
        };
        let Some(horizontal_count) = self
            .width
            .checked_sub(1)
            .and_then(|width| width.checked_mul(self.height))
        else {
            return false;
        };
        let Some(vertical_count) = self
            .height
            .checked_sub(1)
            .and_then(|height| height.checked_mul(self.width))
        else {
            return false;
        };
        let courant = self.propagation_speed * self.time_step / self.spacing;

        self.previous.len() == cell_count
            && self.current.len() == cell_count
            && self.horizontal_coupling.len() == horizontal_count
            && self.vertical_coupling.len() == vertical_count
            && courant.is_finite()
            && courant <= std::f64::consts::FRAC_1_SQRT_2
            && self
                .previous
                .iter()
                .chain(&self.current)
                .all(|value| value.is_finite())
            && self
                .horizontal_coupling
                .iter()
                .chain(&self.vertical_coupling)
                .all(|value| value.is_finite() && (0.0..=1.0).contains(value))
    }

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
            horizontal_coupling: vec![1.0; (width - 1) * height],
            vertical_coupling: vec![1.0; width * (height - 1)],
        })
    }

    /// Replace all internal interfaces as one deterministic geometry pass.
    ///
    /// Edges not listed remain transparent. Duplicate edges, out-of-domain
    /// edges, and invalid coefficients reject the whole update. Because the
    /// same symmetric coupling is used by propagation and the discrete energy
    /// invariant, lossless interfaces reflect/transmit without creating or
    /// destroying field energy.
    pub(crate) fn set_internal_boundaries(
        &mut self,
        edges: &[InternalBoundaryEdge],
    ) -> Result<(), ClosedWaveError> {
        let mut horizontal = vec![1.0; self.horizontal_coupling.len()];
        let mut vertical = vec![1.0; self.vertical_coupling.len()];
        let mut seen_horizontal = vec![false; horizontal.len()];
        let mut seen_vertical = vec![false; vertical.len()];

        for edge in edges {
            if !edge.coupling.is_finite() || !(0.0..=1.0).contains(&edge.coupling) {
                return Err(ClosedWaveError::InvalidParameter);
            }
            let (index, seen, values) = match edge.axis {
                InternalBoundaryAxis::Horizontal => {
                    if edge.x + 1 >= self.width || edge.y >= self.height {
                        return Err(ClosedWaveError::InvalidState);
                    }
                    let index = edge.y * (self.width - 1) + edge.x;
                    (index, &mut seen_horizontal, &mut horizontal)
                }
                InternalBoundaryAxis::Vertical => {
                    if edge.x >= self.width || edge.y + 1 >= self.height {
                        return Err(ClosedWaveError::InvalidState);
                    }
                    let index = edge.y * self.width + edge.x;
                    (index, &mut seen_vertical, &mut vertical)
                }
            };
            if seen[index] {
                return Err(ClosedWaveError::InvalidState);
            }
            seen[index] = true;
            values[index] = edge.coupling;
        }

        self.horizontal_coupling = horizontal;
        self.vertical_coupling = vertical;
        Ok(())
    }

    pub(crate) fn dimensions(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    pub(crate) fn logical_time_ticks(&self) -> f64 {
        self.substeps_elapsed as f64 / self.substeps_per_tick as f64
    }

    /// Query the field at a world-space position, in L, at its latest
    /// completed displacement time. The returned sample labels displacement
    /// and leapfrog velocity with their distinct time centres.
    ///
    /// This is a diagnostic snapshot, not an arbitrary-time query and not
    /// authorization to withdraw the reported pointwise energy density.
    pub(crate) fn query_world_at_current_time(
        &self,
        x_l: f64,
        y_l: f64,
    ) -> Result<ClosedWaveSample, ClosedWaveError> {
        if !x_l.is_finite() || !y_l.is_finite() {
            return Err(ClosedWaveError::InvalidState);
        }
        self.query(x_l / self.spacing, y_l / self.spacing)
    }

    /// Query the field at grid coordinates measured in cell-spacing units.
    /// Coordinates outside the finite domain or non-finite positions fail closed.
    /// Prefer query_world_at_current_time for chemistry-facing callers.
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
        let gradients = self.node_gradients(&self.current);
        let previous_gradients = self.node_gradients(&self.previous);
        // Leapfrog velocity is centred halfway between previous/current
        // displacement states. Use the same temporal centre for the gradient
        // in J = -K c² u_t ∇u; mixing ∇u^n with u_t^(n-1/2) biases flux.
        let midpoint_gradient_x: Vec<f64> = gradients
            .0
            .iter()
            .zip(&previous_gradients.0)
            .map(|(a, b)| 0.5 * (a + b))
            .collect();
        let midpoint_gradient_y: Vec<f64> = gradients
            .1
            .iter()
            .zip(&previous_gradients.1)
            .map(|(a, b)| 0.5 * (a + b))
            .collect();
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
        let flux_gradient_x = self.interpolate(&midpoint_gradient_x, x, y);
        let flux_gradient_y = self.interpolate(&midpoint_gradient_y, x, y);
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
            flux_x: flux_scale * flux_gradient_x,
            flux_y: flux_scale * flux_gradient_y,
            flux_gradient_x,
            flux_gradient_y,
        })
    }

    fn node_gradients(&self, values: &[f64]) -> (Vec<f64>, Vec<f64>) {
        let mut gx = vec![0.0; values.len()];
        let mut gy = vec![0.0; values.len()];
        for y in 0..self.height {
            for x in 0..self.width {
                let i = self.index(x, y);
                if x > 0 && x + 1 < self.width {
                    gx[i] = (values[i + 1] - values[i - 1]) / (2.0 * self.spacing);
                }
                if y > 0 && y + 1 < self.height {
                    gy[i] =
                        (values[i + self.width] - values[i - self.width]) / (2.0 * self.spacing);
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
                    let edge = y * (self.width - 1) + x;
                    gradient_cross += self.horizontal_coupling[edge]
                        * (self.current[next] - self.current[index])
                        * (self.previous[next] - self.previous[index]);
                }
                if y + 1 < self.height {
                    let next = index + self.width;
                    let edge = y * self.width + x;
                    gradient_cross += self.vertical_coupling[edge]
                        * (self.current[next] - self.current[index])
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

    /// Advance with first-order Mur radiation conditions on all four outer
    /// faces. The net energy change of the leapfrog invariant is accumulated as
    /// signed boundary transfer: positive is energy delivered to the void.
    fn advance_one_substep_open(
        &mut self,
        ambient_velocity_after: f64,
    ) -> Result<f64, ClosedWaveError> {
        self.substeps_elapsed
            .checked_add(1)
            .ok_or(ClosedWaveError::CounterOverflow)?;
        let courant = self.propagation_speed * self.time_step / self.spacing;
        let scale = courant.powi(2);
        let mur = (courant - 1.0) / (courant + 1.0);
        let mut next = vec![0.0; self.current.len()];
        for y in 0..self.height {
            for x in 0..self.width {
                let index = self.index(x, y);
                let center = self.current[index];
                let mut laplacian_numerator = 0.0;
                if x > 0 {
                    let edge = y * (self.width - 1) + (x - 1);
                    laplacian_numerator +=
                        self.horizontal_coupling[edge] * (self.current[index - 1] - center);
                }
                if x + 1 < self.width {
                    let edge = y * (self.width - 1) + x;
                    laplacian_numerator +=
                        self.horizontal_coupling[edge] * (self.current[index + 1] - center);
                }
                if y > 0 {
                    let edge = (y - 1) * self.width + x;
                    laplacian_numerator +=
                        self.vertical_coupling[edge] * (self.current[index - self.width] - center);
                }
                if y + 1 < self.height {
                    let edge = y * self.width + x;
                    laplacian_numerator +=
                        self.vertical_coupling[edge] * (self.current[index + self.width] - center);
                }
                next[index] = 2.0 * center - self.previous[index] + scale * laplacian_numerator;
            }
        }

        // Retain the closed stencil prediction so the cross-energy carried out
        // by the boundary condition can be measured independently.
        let closed_next = next.clone();

        // Apply the one-way characteristic condition to open faces. Corners
        // combine the two independent face predictions symmetrically.
        for y in 1..self.height - 1 {
            let left = self.index(0, y);
            let left_inner = self.index(1, y);
            next[left] = self.current[left_inner] + mur * (next[left_inner] - self.current[left]);

            let right = self.index(self.width - 1, y);
            let right_inner = self.index(self.width - 2, y);
            next[right] =
                self.current[right_inner] + mur * (next[right_inner] - self.current[right]);
        }
        for x in 1..self.width - 1 {
            let top = self.index(x, 0);
            let top_inner = self.index(x, 1);
            next[top] = self.current[top_inner] + mur * (next[top_inner] - self.current[top]);

            let bottom = self.index(x, self.height - 1);
            let bottom_inner = self.index(x, self.height - 2);
            next[bottom] =
                self.current[bottom_inner] + mur * (next[bottom_inner] - self.current[bottom]);
        }

        let corners = [
            (self.index(0, 0), self.index(1, 0), self.index(0, 1)),
            (
                self.index(self.width - 1, 0),
                self.index(self.width - 2, 0),
                self.index(self.width - 1, 1),
            ),
            (
                self.index(0, self.height - 1),
                self.index(1, self.height - 1),
                self.index(0, self.height - 2),
            ),
            (
                self.index(self.width - 1, self.height - 1),
                self.index(self.width - 2, self.height - 1),
                self.index(self.width - 1, self.height - 2),
            ),
        ];
        for (corner, horizontal_inner, vertical_inner) in corners {
            let horizontal_prediction = self.current[horizontal_inner]
                + mur * (next[horizontal_inner] - self.current[corner]);
            let vertical_prediction =
                self.current[vertical_inner] + mur * (next[vertical_inner] - self.current[corner]);
            next[corner] = 0.5 * (horizontal_prediction + vertical_prediction);
        }

        if !ambient_velocity_after.is_finite() {
            return Err(ClosedWaveError::InvalidState);
        }
        let mut boundary_velocity_difference = 0.0;
        for y in 1..self.height - 1 {
            boundary_velocity_difference += closed_next[self.index(0, y)] - next[self.index(0, y)];
            boundary_velocity_difference +=
                closed_next[self.index(self.width - 1, y)] - next[self.index(self.width - 1, y)];
        }
        for x in 1..self.width - 1 {
            boundary_velocity_difference += closed_next[self.index(x, 0)] - next[self.index(x, 0)];
            boundary_velocity_difference +=
                closed_next[self.index(x, self.height - 1)] - next[self.index(x, self.height - 1)];
        }
        for (corner, _, _) in corners {
            boundary_velocity_difference += closed_next[corner] - next[corner];
        }
        let cross_outflow = self.normalization
            * ambient_velocity_after
            * self.spacing.powi(2)
            * boundary_velocity_difference
            / self.time_step;
        if next.iter().any(|value| !value.is_finite()) || !cross_outflow.is_finite() {
            return Err(ClosedWaveError::InvalidState);
        }
        self.previous.clone_from(&self.current);
        self.current = next;
        self.substeps_elapsed += 1;
        Ok(cross_outflow)
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
                    let edge = y * (self.width - 1) + (x - 1);
                    laplacian_numerator +=
                        self.horizontal_coupling[edge] * (self.current[index - 1] - center);
                }
                if x + 1 < self.width {
                    let edge = y * (self.width - 1) + x;
                    laplacian_numerator +=
                        self.horizontal_coupling[edge] * (self.current[index + 1] - center);
                }
                if y > 0 {
                    let edge = (y - 1) * self.width + x;
                    laplacian_numerator +=
                        self.vertical_coupling[edge] * (self.current[index - self.width] - center);
                }
                if y + 1 < self.height {
                    let edge = y * self.width + x;
                    laplacian_numerator +=
                        self.vertical_coupling[edge] * (self.current[index + self.width] - center);
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

    /// Transfer a requested amount from the local field interaction into matter.
    ///
    /// The coupling is a bilinear, position-local velocity impulse. Its amplitude
    /// is solved against the exact discrete leapfrog invariant; pointwise energy
    /// density is deliberately not treated as a withdrawable balance.
    pub(crate) fn transfer_to_matter(
        &mut self,
        x_l: f64,
        y_l: f64,
        amount: f64,
        matter: &mut MatterEnergyAccount,
    ) -> Result<(), ClosedWaveError> {
        validate_transfer_amount(amount)?;
        if !matter.stored_energy.is_finite() || matter.stored_energy < 0.0 {
            return Err(ClosedWaveError::InvalidState);
        }
        if amount == 0.0 {
            return Ok(());
        }
        let credited = matter.stored_energy + amount;
        if !credited.is_finite() {
            return Err(ClosedWaveError::InvalidState);
        }
        let (quadratic, linear) = self.local_impulse_energy_coefficients(x_l, y_l)?;
        let discriminant = linear * linear - 4.0 * quadratic * amount;
        if !discriminant.is_finite() || discriminant < 0.0 || linear == 0.0 {
            return Err(ClosedWaveError::InsufficientLocalFieldEnergy);
        }
        let root = discriminant.sqrt();
        // Stable form of the smaller-magnitude root of
        // A*a^2 + B*a + amount = 0.
        let impulse = -2.0 * amount / (linear + linear.signum() * root);
        self.apply_and_verify_local_impulse(x_l, y_l, impulse, -amount)?;
        matter.stored_energy = credited;
        Ok(())
    }

    /// Release matter energy into the field through the reciprocal local impulse.
    /// The receiver is debited only after the exact field-energy increase is verified.
    pub(crate) fn release_from_matter(
        &mut self,
        x_l: f64,
        y_l: f64,
        amount: f64,
        matter: &mut MatterEnergyAccount,
    ) -> Result<(), ClosedWaveError> {
        validate_transfer_amount(amount)?;
        if !matter.stored_energy.is_finite() || matter.stored_energy < amount {
            return Err(ClosedWaveError::InsufficientMatterEnergy);
        }
        if amount == 0.0 {
            return Ok(());
        }
        let debited = matter.stored_energy - amount;
        let (quadratic, linear) = self.local_impulse_energy_coefficients(x_l, y_l)?;
        let discriminant = linear * linear + 4.0 * quadratic * amount;
        if !discriminant.is_finite() || quadratic <= 0.0 {
            return Err(ClosedWaveError::InvalidState);
        }
        let root = discriminant.sqrt();
        // Select the smaller-magnitude root, with a deterministic positive
        // branch when the local phase slope is exactly zero.
        let impulse = if linear > 0.0 {
            2.0 * amount / (linear + root)
        } else if linear < 0.0 {
            -2.0 * amount / (-linear + root)
        } else {
            (amount / quadratic).sqrt()
        };
        self.apply_and_verify_local_impulse(x_l, y_l, impulse, amount)?;
        matter.stored_energy = debited;
        Ok(())
    }

    /// Derive the exact quadratic energy response from the four-node stencil
    /// and its incident edges. No full-grid clone or energy scan is required.
    fn local_impulse_energy_coefficients(
        &self,
        x_l: f64,
        y_l: f64,
    ) -> Result<(f64, f64), ClosedWaveError> {
        let nodes = self.local_interaction_nodes(x_l, y_l)?;
        let kinetic_scale =
            2.0 * self.normalization * self.spacing.powi(2) / self.time_step.powi(2);
        let mut quadratic = 0.0;
        let mut linear = 0.0;
        for &(index, weight) in &nodes {
            if weight == 0.0 {
                continue;
            }
            let difference = self.current[index] - self.previous[index];
            linear += kinetic_scale * difference * weight;
            quadratic += kinetic_scale * weight.powi(2);
        }

        let edge_scale = 0.5 * self.normalization * self.propagation_speed.powi(2);
        let mut visited_edges = Vec::with_capacity(16);
        let mut visit_edge = |horizontal: bool,
                              edge_index: usize,
                              center_index: usize,
                              neighbor_index: usize,
                              coupling: f64,
                              center_weight: f64| {
            let edge_key = (horizontal, edge_index);
            if visited_edges.contains(&edge_key) {
                return;
            }
            visited_edges.push(edge_key);
            let neighbor_weight = nodes
                .iter()
                .find(|(index, _)| *index == neighbor_index)
                .map(|(_, weight)| *weight)
                .unwrap_or(0.0);
            let weight_difference = neighbor_weight - center_weight;
            let current_difference = self.current[neighbor_index] - self.current[center_index];
            let previous_difference = self.previous[neighbor_index] - self.previous[center_index];
            linear += edge_scale
                * coupling
                * weight_difference
                * (previous_difference - current_difference);
            quadratic -= edge_scale * coupling * weight_difference.powi(2);
        };

        for &(index, weight) in &nodes {
            if weight == 0.0 {
                continue;
            }
            let x = index % self.width;
            let y = index / self.width;
            if x + 1 < self.width {
                let edge = y * (self.width - 1) + x;
                visit_edge(
                    true,
                    edge,
                    index,
                    index + 1,
                    self.horizontal_coupling[edge],
                    weight,
                );
            }
            if x > 0 {
                let edge = y * (self.width - 1) + x - 1;
                visit_edge(
                    true,
                    edge,
                    index,
                    index - 1,
                    self.horizontal_coupling[edge],
                    weight,
                );
            }
            if y + 1 < self.height {
                let edge = y * self.width + x;
                visit_edge(
                    false,
                    edge,
                    index,
                    index + self.width,
                    self.vertical_coupling[edge],
                    weight,
                );
            }
            if y > 0 {
                let edge = (y - 1) * self.width + x;
                visit_edge(
                    false,
                    edge,
                    index,
                    index - self.width,
                    self.vertical_coupling[edge],
                    weight,
                );
            }
        }

        if !quadratic.is_finite() || quadratic <= 0.0 || !linear.is_finite() {
            return Err(ClosedWaveError::InvalidState);
        }
        Ok((quadratic, linear))
    }

    fn local_energy_contribution(&self, nodes: &[(usize, f64); 4]) -> Result<f64, ClosedWaveError> {
        let mut energy = 0.0;
        for &(index, weight) in nodes {
            if weight == 0.0 {
                continue;
            }
            let velocity = (self.current[index] - self.previous[index]) / self.time_step;
            energy += 0.5 * self.normalization * velocity.powi(2) * self.spacing.powi(2);
        }

        let mut visited_edges = Vec::with_capacity(16);
        let mut visit_edge = |horizontal: bool,
                              edge_index: usize,
                              center_index: usize,
                              neighbor_index: usize,
                              coupling: f64| {
            let edge_key = (horizontal, edge_index);
            if visited_edges.contains(&edge_key) {
                return;
            }
            visited_edges.push(edge_key);
            let current_difference = self.current[neighbor_index] - self.current[center_index];
            let previous_difference = self.previous[neighbor_index] - self.previous[center_index];
            energy += 0.5
                * self.normalization
                * self.propagation_speed.powi(2)
                * coupling
                * current_difference
                * previous_difference;
        };

        for &(index, weight) in nodes {
            if weight == 0.0 {
                continue;
            }
            let x = index % self.width;
            let y = index / self.width;
            if x + 1 < self.width {
                let edge = y * (self.width - 1) + x;
                visit_edge(true, edge, index, index + 1, self.horizontal_coupling[edge]);
            }
            if x > 0 {
                let edge = y * (self.width - 1) + x - 1;
                visit_edge(true, edge, index, index - 1, self.horizontal_coupling[edge]);
            }
            if y + 1 < self.height {
                let edge = y * self.width + x;
                visit_edge(
                    false,
                    edge,
                    index,
                    index + self.width,
                    self.vertical_coupling[edge],
                );
            }
            if y > 0 {
                let edge = (y - 1) * self.width + x;
                visit_edge(
                    false,
                    edge,
                    index,
                    index - self.width,
                    self.vertical_coupling[edge],
                );
            }
        }

        energy
            .is_finite()
            .then_some(energy)
            .ok_or(ClosedWaveError::InvalidState)
    }

    fn apply_and_verify_local_impulse(
        &mut self,
        x_l: f64,
        y_l: f64,
        impulse: f64,
        expected_delta: f64,
    ) -> Result<(), ClosedWaveError> {
        if !impulse.is_finite() {
            return Err(ClosedWaveError::InvalidState);
        }
        let nodes = self.local_interaction_nodes(x_l, y_l)?;
        let before = self.local_energy_contribution(&nodes)?;
        let original_values: Vec<_> = nodes
            .iter()
            .filter(|(_, weight)| *weight != 0.0)
            .map(|(index, _)| (*index, self.previous[*index], self.current[*index]))
            .collect();
        self.apply_local_impulse_nodes(&nodes, impulse)?;
        let after = match self.local_energy_contribution(&nodes) {
            Ok(after) => after,
            Err(error) => {
                for (index, previous, current) in original_values {
                    self.previous[index] = previous;
                    self.current[index] = current;
                }
                return Err(error);
            }
        };
        let actual_delta = after - before;
        let tolerance = 2.0e-10 * expected_delta.abs().max(1.0);
        if !actual_delta.is_finite() || (actual_delta - expected_delta).abs() > tolerance {
            for (index, previous, current) in original_values {
                self.previous[index] = previous;
                self.current[index] = current;
            }
            return Err(ClosedWaveError::EnergySettlementFailed);
        }
        Ok(())
    }

    fn local_interaction_nodes(
        &self,
        x_l: f64,
        y_l: f64,
    ) -> Result<[(usize, f64); 4], ClosedWaveError> {
        if !x_l.is_finite() || !y_l.is_finite() {
            return Err(ClosedWaveError::InvalidState);
        }
        let x = x_l / self.spacing;
        let y = y_l / self.spacing;
        if x < 0.0 || y < 0.0 || x > (self.width - 1) as f64 || y > (self.height - 1) as f64 {
            return Err(ClosedWaveError::InvalidState);
        }
        let x0 = x.floor() as usize;
        let y0 = y.floor() as usize;
        let x1 = (x0 + 1).min(self.width - 1);
        let y1 = (y0 + 1).min(self.height - 1);
        let tx = x - x0 as f64;
        let ty = y - y0 as f64;
        let raw = [
            (self.index(x0, y0), (1.0 - tx) * (1.0 - ty)),
            (self.index(x1, y0), tx * (1.0 - ty)),
            (self.index(x0, y1), (1.0 - tx) * ty),
            (self.index(x1, y1), tx * ty),
        ];
        let mut combined = [(0, 0.0); 4];
        let mut combined_count = 0;
        for (index, weight) in raw {
            if let Some((_, combined_weight)) = combined[..combined_count]
                .iter_mut()
                .find(|(existing_index, _)| *existing_index == index)
            {
                *combined_weight += weight;
            } else {
                combined[combined_count] = (index, weight);
                combined_count += 1;
            }
        }
        Ok(combined)
    }

    fn apply_local_impulse_nodes(
        &mut self,
        nodes: &[(usize, f64); 4],
        impulse: f64,
    ) -> Result<(), ClosedWaveError> {
        let mut updates = Vec::with_capacity(4);
        for (index, weight) in nodes {
            if *weight == 0.0 {
                continue;
            }
            let current = self.current[*index] + impulse * weight;
            let previous = self.previous[*index] - impulse * weight;
            if !current.is_finite() || !previous.is_finite() {
                return Err(ClosedWaveError::InvalidState);
            }
            updates.push((*index, previous, current));
        }
        for (index, previous, current) in updates {
            self.previous[index] = previous;
            self.current[index] = current;
        }
        Ok(())
    }

    fn index(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }
}

/// Minimal isolated matter-side energy account used to prove reciprocal settlement.
/// It is not a replacement for chemistry's structural-energy accounting.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MatterEnergyAccount {
    stored_energy: f64,
}

impl MatterEnergyAccount {
    pub(crate) fn new(stored_energy: f64) -> Result<Self, ClosedWaveError> {
        if !stored_energy.is_finite() || stored_energy < 0.0 {
            return Err(ClosedWaveError::InvalidState);
        }
        Ok(Self { stored_energy })
    }

    pub(crate) fn stored_energy(&self) -> f64 {
        self.stored_energy
    }
}

fn validate_transfer_amount(amount: f64) -> Result<(), ClosedWaveError> {
    if !amount.is_finite() || amount < 0.0 {
        return Err(ClosedWaveError::InvalidState);
    }
    Ok(())
}

/// Finite spatial wave field with first-order radiating outer boundaries.
///
/// The legacy advance path records disturbance-only net boundary transfer.
/// The ambient-aware path additionally records the ambient/disturbance cross
/// contribution and time-centred world-drive work. Positive outflow enters the
/// void; small signed negative increments remain auditable.
#[allow(
    dead_code,
    reason = "Validated open-boundary solver is staged for production ownership integration."
)]
#[derive(Clone, Debug, Serialize)]
pub(crate) struct OpenBoundaryWaveField {
    inner: ClosedBoundaryWaveField,
    cumulative_outflow_energy: f64,
    cumulative_combined_outflow_energy: f64,
    cumulative_world_drive_work: f64,
}

#[derive(Deserialize)]
struct OpenBoundaryWaveFieldState {
    inner: ClosedBoundaryWaveField,
    cumulative_outflow_energy: f64,
    #[serde(default)]
    cumulative_combined_outflow_energy: f64,
    #[serde(default)]
    cumulative_world_drive_work: f64,
}

impl<'de> Deserialize<'de> for OpenBoundaryWaveField {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error as _;
        let state = OpenBoundaryWaveFieldState::deserialize(deserializer)?;
        if !state.cumulative_outflow_energy.is_finite()
            || !state.cumulative_combined_outflow_energy.is_finite()
            || !state.cumulative_world_drive_work.is_finite()
        {
            return Err(D::Error::custom("invalid serialized open wave accounting"));
        }
        Ok(Self {
            inner: state.inner,
            cumulative_outflow_energy: state.cumulative_outflow_energy,
            cumulative_combined_outflow_energy: state.cumulative_combined_outflow_energy,
            cumulative_world_drive_work: state.cumulative_world_drive_work,
        })
    }
}

#[allow(
    dead_code,
    reason = "Validated open-boundary solver is staged for production ownership integration."
)]
impl OpenBoundaryWaveField {
    pub(crate) fn new(
        width: usize,
        height: usize,
        spacing: f64,
        propagation_speed: f64,
        substeps_per_tick: usize,
        normalization: f64,
    ) -> Result<Self, ClosedWaveError> {
        Ok(Self {
            inner: ClosedBoundaryWaveField::new(
                width,
                height,
                spacing,
                propagation_speed,
                substeps_per_tick,
                normalization,
            )?,
            cumulative_outflow_energy: 0.0,
            cumulative_combined_outflow_energy: 0.0,
            cumulative_world_drive_work: 0.0,
        })
    }

    pub(crate) fn dimensions(&self) -> (usize, usize) {
        self.inner.dimensions()
    }

    pub(crate) fn logical_time_ticks(&self) -> f64 {
        self.inner.logical_time_ticks()
    }

    /// The leapfrog velocity is centred half a substep before the latest
    /// displacement state. Ambient carrier inputs for combined inventory must
    /// be sampled at this exact logical time.
    pub(crate) fn velocity_time_ticks(&self) -> f64 {
        (self.inner.substeps_elapsed as f64 - 0.5) / self.inner.substeps_per_tick as f64
    }

    /// Compose using the live ambient mode sampled at the exact leapfrog
    /// velocity time centre. This is the safe entry point for callers.
    pub(crate) fn combined_energy(
        &self,
        ambient: &crate::world_generated_ambient_mode::WorldGeneratedAmbientMode,
    ) -> Result<CombinedFieldEnergy, ClosedWaveError> {
        let sample = ambient
            .sample_at_time(self.velocity_time_ticks())
            .map_err(|_| ClosedWaveError::InvalidState)?;
        self.combined_energy_at_velocity_center(
            sample.carrier_velocity,
            sample.instantaneous_field_energy_density,
            sample.field_normalization,
        )
    }

    /// Compose the discrete disturbance invariant with ambient wave kinetic
    /// energy and the cross term at the same leapfrog velocity time centre.
    ///
    /// The ambient carrier velocity and kinetic energy density must be
    /// sampled at velocity_time_ticks(). The ambient mode is uniform, so its
    /// wave kinetic energy is multiplied by the finite accounting area.
    /// The cross term is integrated against the discrete disturbance velocity
    /// using the same cell-area quadrature as the solver's kinetic invariant.
    pub(crate) fn combined_energy_at_velocity_center(
        &self,
        ambient_carrier_velocity: f64,
        ambient_energy_density: f64,
        ambient_field_normalization: f64,
    ) -> Result<CombinedFieldEnergy, ClosedWaveError> {
        if !ambient_carrier_velocity.is_finite()
            || !ambient_energy_density.is_finite()
            || ambient_energy_density < 0.0
            || !ambient_field_normalization.is_finite()
            || ambient_field_normalization.to_bits() != self.inner.normalization.to_bits()
        {
            return Err(ClosedWaveError::InvalidState);
        }

        let cell_area = self.inner.spacing.powi(2);
        let accounting_area = self.inner.width as f64 * self.inner.height as f64 * cell_area;
        let disturbance_velocity_integral = self
            .inner
            .current
            .iter()
            .zip(&self.inner.previous)
            .map(|(current, previous)| (current - previous) / self.inner.time_step)
            .sum::<f64>()
            * cell_area;
        let disturbance_energy = self.inner.energy();
        let ambient_energy = ambient_energy_density * accounting_area;
        let cross_energy =
            ambient_field_normalization * ambient_carrier_velocity * disturbance_velocity_integral;
        let total_energy = disturbance_energy + ambient_energy + cross_energy;

        if [
            accounting_area,
            disturbance_velocity_integral,
            disturbance_energy,
            ambient_energy,
            cross_energy,
            total_energy,
        ]
        .iter()
        .any(|value| !value.is_finite())
            || disturbance_energy < 0.0
            || total_energy < 0.0
        {
            return Err(ClosedWaveError::InvalidState);
        }

        Ok(CombinedFieldEnergy {
            disturbance_energy,
            ambient_energy,
            cross_energy,
            total_energy,
        })
    }

    pub(crate) fn energy(&self) -> f64 {
        self.inner.energy()
    }

    pub(crate) fn cumulative_outflow_energy(&self) -> f64 {
        self.cumulative_outflow_energy
    }

    /// Signed net energy delivered to the void by the composed ambient and
    /// disturbance field, including their cross-energy contribution.
    pub(crate) fn cumulative_combined_outflow_energy(&self) -> f64 {
        self.cumulative_combined_outflow_energy
    }

    /// Signed world work inferred from the combined discrete balance:
    /// field-energy change plus combined boundary outflow.
    pub(crate) fn cumulative_world_drive_work(&self) -> f64 {
        self.cumulative_world_drive_work
    }

    pub(crate) fn sample(&self, x: usize, y: usize) -> Option<f64> {
        self.inner.sample(x, y)
    }

    pub(crate) fn set_initial_state(
        &mut self,
        x: usize,
        y: usize,
        previous: f64,
        current: f64,
    ) -> Result<(), ClosedWaveError> {
        self.inner.set_initial_state(x, y, previous, current)
    }

    pub(crate) fn query_world_at_current_time(
        &self,
        x_l: f64,
        y_l: f64,
    ) -> Result<ClosedWaveSample, ClosedWaveError> {
        self.inner.query_world_at_current_time(x_l, y_l)
    }

    pub(crate) fn transfer_to_matter(
        &mut self,
        x_l: f64,
        y_l: f64,
        amount: f64,
        matter: &mut MatterEnergyAccount,
    ) -> Result<(), ClosedWaveError> {
        self.inner.transfer_to_matter(x_l, y_l, amount, matter)
    }

    pub(crate) fn release_from_matter(
        &mut self,
        x_l: f64,
        y_l: f64,
        amount: f64,
        matter: &mut MatterEnergyAccount,
    ) -> Result<(), ClosedWaveError> {
        self.inner.release_from_matter(x_l, y_l, amount, matter)
    }

    pub(crate) fn advance_ticks(&mut self, ticks: u64) -> Result<(), ClosedWaveError> {
        let steps = ticks
            .checked_mul(self.inner.substeps_per_tick as u64)
            .ok_or(ClosedWaveError::CounterOverflow)?;
        self.advance_substeps(steps)
    }

    pub(crate) fn advance_substeps(&mut self, steps: u64) -> Result<(), ClosedWaveError> {
        self.inner
            .substeps_elapsed
            .checked_add(steps)
            .ok_or(ClosedWaveError::CounterOverflow)?;
        let energy_before = self.inner.energy();
        for _ in 0..steps {
            self.inner.advance_one_substep_open(0.0)?;
        }
        let energy_after = self.inner.energy();
        let net_outflow = energy_before - energy_after;
        let next_outflow = self.cumulative_outflow_energy + net_outflow;
        if !net_outflow.is_finite() || !next_outflow.is_finite() {
            return Err(ClosedWaveError::InvalidState);
        }
        self.cumulative_outflow_energy = next_outflow;
        Ok(())
    }

    /// Advance while reconciling the ambient carrier, spatial disturbance,
    /// cross-energy, and radiating-boundary transfer at every leapfrog step.
    /// The signed world-drive term is integrated from K * u_ambient,tt *
    /// (u_ambient,t + u_disturbance,t) over the accounting area and substep.
    pub(crate) fn advance_substeps_with_ambient(
        &mut self,
        steps: u64,
        ambient: &crate::world_generated_ambient_mode::WorldGeneratedAmbientMode,
    ) -> Result<(), ClosedWaveError> {
        self.inner
            .substeps_elapsed
            .checked_add(steps)
            .ok_or(ClosedWaveError::CounterOverflow)?;
        let mut disturbance_outflow = 0.0;
        let mut combined_outflow = 0.0;
        let mut world_drive_work = 0.0;

        for _ in 0..steps {
            let energy_before = self.inner.energy();
            let old_disturbance_velocity_integral = self
                .inner
                .current
                .iter()
                .zip(&self.inner.previous)
                .map(|(current, previous)| (current - previous) / self.inner.time_step)
                .sum::<f64>()
                * self.inner.spacing.powi(2);
            let ambient_before = ambient
                .sample_at_time(self.velocity_time_ticks())
                .map_err(|_| ClosedWaveError::InvalidState)?;
            let next_velocity_time =
                (self.inner.substeps_elapsed as f64 + 0.5) / self.inner.substeps_per_tick as f64;
            let ambient_after = ambient
                .sample_at_time(next_velocity_time)
                .map_err(|_| ClosedWaveError::InvalidState)?;
            let cross_outflow = self
                .inner
                .advance_one_substep_open(ambient_after.carrier_velocity)?;
            let step_disturbance_outflow = energy_before - self.inner.energy();
            let step_combined_outflow = step_disturbance_outflow + cross_outflow;
            let new_disturbance_velocity_integral = self
                .inner
                .current
                .iter()
                .zip(&self.inner.previous)
                .map(|(current, previous)| (current - previous) / self.inner.time_step)
                .sum::<f64>()
                * self.inner.spacing.powi(2);
            let disturbance_velocity_integral =
                0.5 * (old_disturbance_velocity_integral + new_disturbance_velocity_integral);
            let ambient_world_area = ambient_before.total_energy / ambient_before.energy_density;
            let ambient_velocity_change =
                ambient_after.carrier_velocity - ambient_before.carrier_velocity;
            let mean_ambient_velocity =
                0.5 * (ambient_before.carrier_velocity + ambient_after.carrier_velocity);
            let step_world_drive_work = self.inner.normalization
                * ambient_velocity_change
                * (mean_ambient_velocity * ambient_world_area + disturbance_velocity_integral);

            if [
                step_disturbance_outflow,
                step_combined_outflow,
                disturbance_velocity_integral,
                ambient_world_area,
                step_world_drive_work,
            ]
            .iter()
            .any(|value| !value.is_finite())
            {
                return Err(ClosedWaveError::InvalidState);
            }
            disturbance_outflow += step_disturbance_outflow;
            combined_outflow += step_combined_outflow;
            world_drive_work += step_world_drive_work;
        }

        let next_disturbance_outflow = self.cumulative_outflow_energy + disturbance_outflow;
        let next_combined_outflow = self.cumulative_combined_outflow_energy + combined_outflow;
        let next_world_drive_work = self.cumulative_world_drive_work + world_drive_work;
        if [
            next_disturbance_outflow,
            next_combined_outflow,
            next_world_drive_work,
        ]
        .iter()
        .any(|value| !value.is_finite())
        {
            return Err(ClosedWaveError::InvalidState);
        }
        self.cumulative_outflow_energy = next_disturbance_outflow;
        self.cumulative_combined_outflow_energy = next_combined_outflow;
        self.cumulative_world_drive_work = next_world_drive_work;
        Ok(())
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
    fn local_impulse_coefficients_match_full_grid_energy_at_interior_and_boundaries() {
        let field = energized_field();
        for (x, y) in [(4.25, 4.5), (0.0, 0.0), (8.0, 8.0), (0.0, 4.5)] {
            let nodes = field.local_interaction_nodes(x, y).unwrap();
            let baseline = field.energy();
            let mut plus = field.clone();
            let mut minus = field.clone();
            plus.apply_local_impulse_nodes(&nodes, 1.0).unwrap();
            minus.apply_local_impulse_nodes(&nodes, -1.0).unwrap();
            let plus_delta = plus.energy() - baseline;
            let minus_delta = minus.energy() - baseline;
            let local_before = field.local_energy_contribution(&nodes).unwrap();
            let local_after = plus.local_energy_contribution(&nodes).unwrap();
            close(local_after - local_before, plus_delta, 2.0e-12);
            let expected_quadratic = 0.5 * (plus_delta + minus_delta);
            let expected_linear = 0.5 * (plus_delta - minus_delta);
            let (quadratic, linear) = field.local_impulse_energy_coefficients(x, y).unwrap();
            close(quadratic, expected_quadratic, 2.0e-12);
            close(linear, expected_linear, 2.0e-12);
        }
    }

    #[test]
    fn boundary_local_transfer_debits_the_exact_requested_energy() {
        let mut field = energized_field();
        let mut matter = MatterEnergyAccount::new(0.0).unwrap();
        let before = field.energy();
        field
            .transfer_to_matter(0.0, 0.0, 0.000001, &mut matter)
            .unwrap();
        close(before - field.energy(), 0.000001, 2.0e-10);
        close(matter.stored_energy(), 0.000001, 1.0e-12);
    }

    #[test]
    fn field_to_matter_transfer_debits_exactly_what_matter_receives() {
        let mut field = energized_field();
        let mut matter = MatterEnergyAccount::new(1.25).unwrap();
        let field_before = field.energy();
        let matter_before = matter.stored_energy();

        field
            .transfer_to_matter(4.0, 4.0, 0.01, &mut matter)
            .unwrap();

        close(field_before - field.energy(), 0.01, 2.0e-10);
        close(matter.stored_energy() - matter_before, 0.01, 1.0e-12);
        close(
            field.energy() + matter.stored_energy(),
            field_before + matter_before,
            2.0e-10,
        );
    }

    #[test]
    fn matter_to_field_release_is_reciprocal_and_conservative() {
        let mut field = energized_field();
        let mut matter = MatterEnergyAccount::new(0.75).unwrap();
        let field_before = field.energy();
        let matter_before = matter.stored_energy();

        field
            .release_from_matter(4.0, 4.0, 0.02, &mut matter)
            .unwrap();

        close(field.energy() - field_before, 0.02, 2.0e-10);
        close(matter_before - matter.stored_energy(), 0.02, 1.0e-12);
        close(
            field.energy() + matter.stored_energy(),
            field_before + matter_before,
            2.0e-10,
        );
    }

    #[test]
    fn failed_or_overlapping_transfers_cannot_double_spend_field_energy() {
        let mut field = energized_field();
        let mut first = MatterEnergyAccount::new(0.0).unwrap();
        let mut second = MatterEnergyAccount::new(0.0).unwrap();
        field
            .transfer_to_matter(4.0, 4.0, 0.01, &mut first)
            .unwrap();

        let field_after_first = field.energy();
        let second_before = second.stored_energy();
        let first_before = first.stored_energy();
        let result = field.transfer_to_matter(4.0, 4.0, 1.0e6, &mut second);
        assert_eq!(result, Err(ClosedWaveError::InsufficientLocalFieldEnergy));
        close(field.energy(), field_after_first, 1.0e-12);
        close(second.stored_energy(), second_before, 1.0e-12);
        close(first.stored_energy(), first_before, 1.0e-12);

        // A second receiver at the same interaction site can only draw from
        // the field state that remains after the first committed transaction.
        let total_before = field.energy() + first.stored_energy() + second.stored_energy();
        field
            .transfer_to_matter(4.0, 4.0, 0.005, &mut second)
            .unwrap();
        close(
            field.energy() + first.stored_energy() + second.stored_energy(),
            total_before,
            2.0e-10,
        );
    }

    #[test]
    fn invalid_transfer_and_insufficient_matter_leave_both_sides_unchanged() {
        let mut field = energized_field();
        let mut matter = MatterEnergyAccount::new(0.01).unwrap();
        let field_before = field.energy();
        let matter_before = matter.stored_energy();
        assert_eq!(
            field.transfer_to_matter(f64::NAN, 4.0, 0.1, &mut matter),
            Err(ClosedWaveError::InvalidState)
        );
        assert_eq!(
            field.release_from_matter(4.0, 4.0, 0.02, &mut matter),
            Err(ClosedWaveError::InsufficientMatterEnergy)
        );
        assert_eq!(field.energy(), field_before);
        assert_eq!(matter.stored_energy(), matter_before);
        assert_eq!(
            MatterEnergyAccount::new(f64::NAN),
            Err(ClosedWaveError::InvalidState)
        );
    }

    #[test]
    fn lossless_internal_interfaces_preserve_the_discrete_energy_invariant() {
        let mut field = energized_field();
        let mut edges = Vec::new();
        for y in 0..8 {
            edges.push(InternalBoundaryEdge {
                x: 3,
                y,
                axis: InternalBoundaryAxis::Horizontal,
                coupling: 0.0,
            });
        }
        field.set_internal_boundaries(&edges).unwrap();
        let initial_energy = field.energy();
        field.advance_ticks(120).unwrap();
        close(field.energy(), initial_energy, 2.0e-10);
    }

    #[test]
    fn opaque_internal_wall_prevents_field_propagation_across_it() {
        let mut field = ClosedBoundaryWaveField::new(9, 5, 1.0, 0.2, 2, 1.0).unwrap();
        field.set_initial_state(2, 2, 0.0, 1.0).unwrap();
        let wall: Vec<_> = (0..5)
            .map(|y| InternalBoundaryEdge {
                x: 3,
                y,
                axis: InternalBoundaryAxis::Horizontal,
                coupling: 0.0,
            })
            .collect();
        field.set_internal_boundaries(&wall).unwrap();
        field.advance_ticks(20).unwrap();
        for y in 0..5 {
            for x in 4..9 {
                close(field.sample(x, y).unwrap(), 0.0, 1.0e-12);
            }
        }
        assert!(field.sample(3, 2).unwrap().abs() > 0.0);
    }

    #[test]
    fn invalid_or_duplicate_internal_edges_reject_the_entire_update() {
        let mut field = energized_field();
        let before = field.energy();
        let invalid = [
            InternalBoundaryEdge {
                x: 2,
                y: 2,
                axis: InternalBoundaryAxis::Horizontal,
                coupling: 0.5,
            },
            InternalBoundaryEdge {
                x: 2,
                y: 2,
                axis: InternalBoundaryAxis::Horizontal,
                coupling: 0.25,
            },
        ];
        assert_eq!(
            field.set_internal_boundaries(&invalid),
            Err(ClosedWaveError::InvalidState)
        );
        close(field.energy(), before, 1.0e-12);
        assert_eq!(
            field.set_internal_boundaries(&[InternalBoundaryEdge {
                x: 2,
                y: 2,
                axis: InternalBoundaryAxis::Vertical,
                coupling: f64::NAN,
            }]),
            Err(ClosedWaveError::InvalidParameter)
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
    fn world_space_query_uses_length_units_and_reports_both_time_centres() {
        let mut field = ClosedBoundaryWaveField::new(7, 7, 0.5, 0.25, 2, 3.0).unwrap();
        for y in 0..7 {
            for x in 0..7 {
                let value = x as f64 * 0.5 + y as f64 * 0.25;
                field.set_initial_state(x, y, value - 0.1, value).unwrap();
            }
        }

        let initial = field.query_world_at_current_time(1.5, 1.5).unwrap();
        close(initial.position_x, 1.5, 1.0e-12);
        close(initial.position_y, 1.5, 1.0e-12);
        close(initial.displacement_time_ticks, 0.0, 1.0e-12);
        close(initial.velocity_time_ticks, -0.25, 1.0e-12);

        field.advance_ticks(3).unwrap();
        let later = field.query_world_at_current_time(1.5, 1.5).unwrap();
        close(later.displacement_time_ticks, 3.0, 1.0e-12);
        close(later.velocity_time_ticks, 2.75, 1.0e-12);
        assert_eq!(
            field.query_world_at_current_time(-0.01, 1.5),
            Err(ClosedWaveError::InvalidState)
        );
        assert_eq!(
            field.query_world_at_current_time(f64::INFINITY, 1.5),
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
    fn initialize_outgoing_packet(
        open: &mut OpenBoundaryWaveField,
        closed: &mut ClosedBoundaryWaveField,
        angle: f64,
    ) {
        let (width, height) = open.dimensions();
        let c = 0.2;
        let dt = 0.5;
        let source_x = 20.0;
        let source_y = 50.0;
        let (sin_angle, cos_angle) = angle.sin_cos();
        for y in 0..height {
            for x in 0..width {
                let dx = x as f64 - source_x;
                let dy = y as f64 - source_y;
                let along = dx * cos_angle + dy * sin_angle;
                let across = -dx * sin_angle + dy * cos_angle;
                let envelope = (-along.powi(2) / (2.0 * 8.0_f64.powi(2))
                    - across.powi(2) / (2.0 * 18.0_f64.powi(2)))
                .exp();
                let displacement = envelope * (0.45 * along).cos();
                let spatial_derivative =
                    -along / 64.0 * displacement - 0.45 * envelope * (0.45 * along).sin();
                let time_derivative = -c * spatial_derivative;
                let previous = displacement - dt * time_derivative;
                open.set_initial_state(x, y, previous, displacement)
                    .unwrap();
                closed
                    .set_initial_state(x, y, previous, displacement)
                    .unwrap();
            }
        }
    }

    #[test]
    fn radiating_boundary_accounts_for_energy_leaving_the_finite_domain() {
        let mut open = OpenBoundaryWaveField::new(101, 101, 1.0, 0.2, 2, 1.0).unwrap();
        let mut closed = ClosedBoundaryWaveField::new(101, 101, 1.0, 0.2, 2, 1.0).unwrap();
        initialize_outgoing_packet(&mut open, &mut closed, 0.0);
        let initial_energy = open.energy();
        assert!(initial_energy > 0.0);

        open.advance_substeps(1_500).unwrap();

        assert!(open.cumulative_outflow_energy() > 0.0);
        close(
            initial_energy,
            open.energy() + open.cumulative_outflow_energy(),
            2.0e-10,
        );
        assert_eq!(open.logical_time_ticks(), 750.0);
        assert_eq!(closed.logical_time_ticks(), 0.0);
    }

    #[test]
    fn first_order_radiating_boundary_suppresses_normal_and_oblique_reflections() {
        for angle in [0.0_f64, 0.25, 0.5, std::f64::consts::FRAC_PI_4] {
            let mut open = OpenBoundaryWaveField::new(101, 101, 1.0, 0.2, 2, 1.0).unwrap();
            let mut closed = ClosedBoundaryWaveField::new(101, 101, 1.0, 0.2, 2, 1.0).unwrap();
            initialize_outgoing_packet(&mut open, &mut closed, angle);
            let (sin_angle, cos_angle) = angle.sin_cos();
            let probe_x = (20.0 + 48.0 * cos_angle).round() as usize;
            let probe_y = (50.0 + 48.0 * sin_angle).round() as usize;
            let mut open_trace = Vec::with_capacity(300);
            let mut closed_trace = Vec::with_capacity(300);
            for _ in 0..300 {
                open.advance_substeps(5).unwrap();
                closed.advance_substeps(5).unwrap();
                open_trace.push(open.sample(probe_x, probe_y).unwrap().abs());
                closed_trace.push(closed.sample(probe_x, probe_y).unwrap().abs());
            }
            let early_open = open_trace[60..130].iter().copied().fold(0.0_f64, f64::max);
            let late_open = open_trace[180..260].iter().copied().fold(0.0_f64, f64::max);
            let early_closed = closed_trace[60..130]
                .iter()
                .copied()
                .fold(0.0_f64, f64::max);
            let late_closed = closed_trace[180..260]
                .iter()
                .copied()
                .fold(0.0_f64, f64::max);
            assert!(early_open > 0.1, "incident packet did not reach probe");
            assert!(
                late_open / early_open < 0.05,
                "open-boundary reflection exceeded 5% at angle {angle}: {}",
                late_open / early_open
            );
            assert!(
                late_closed / early_closed > 0.05,
                "closed reference did not retain its reflected wave at angle {angle}"
            );
        }
    }

    #[test]
    fn combined_inventory_includes_ambient_kinetic_and_leapfrog_cross_energy() {
        let mut open = OpenBoundaryWaveField::new(2, 2, 1.0, 0.2, 2, 2.0).unwrap();
        for y in 0..2 {
            for x in 0..2 {
                open.set_initial_state(x, y, 0.0, 0.5).unwrap();
            }
        }

        let combined = open
            .combined_energy_at_velocity_center(0.25, 3.0, 2.0)
            .unwrap();

        assert!((combined.disturbance_energy - 4.0).abs() < 1.0e-12);
        assert!((combined.ambient_energy - 12.0).abs() < 1.0e-12);
        assert!((combined.cross_energy - 2.0).abs() < 1.0e-12);
        assert!((combined.total_energy - 18.0).abs() < 1.0e-12);
        assert_eq!(open.velocity_time_ticks(), -0.25);
    }

    #[test]
    fn combined_inventory_samples_ambient_at_leapfrog_velocity_time() {
        let open = OpenBoundaryWaveField::new(3, 3, 1.0, 0.2, 4, 2.0).unwrap();
        let config =
            crate::world_generated_ambient_mode::AmbientModeConfig::new(9.0, 2.0, 0.0, 0.0)
                .unwrap();
        let ambient =
            crate::world_generated_ambient_mode::WorldGeneratedAmbientMode::new(config, 0).unwrap();
        let expected = ambient.sample_at_time(open.velocity_time_ticks()).unwrap();
        let combined = open.combined_energy(&ambient).unwrap();

        assert!(
            (combined.ambient_energy - expected.instantaneous_field_energy_density * 9.0).abs()
                < 1.0e-12
        );
        assert_eq!(open.velocity_time_ticks(), -0.125);
    }

    #[test]
    fn ambient_advance_reconciles_combined_inventory_with_drive_and_outflow() {
        let mut open = OpenBoundaryWaveField::new(9, 9, 1.0, 0.2, 2, 2.0).unwrap();
        for y in 0..9 {
            for x in 0..9 {
                let radius_squared = (x as f64 - 4.0).powi(2) + (y as f64 - 4.0).powi(2);
                let current = (-radius_squared / 8.0).exp();
                let previous = current - 0.1 * current;
                open.set_initial_state(x, y, previous, current).unwrap();
            }
        }
        let config =
            crate::world_generated_ambient_mode::AmbientModeConfig::new(81.0, 2.0, 0.0, 0.0)
                .unwrap();
        let ambient =
            crate::world_generated_ambient_mode::WorldGeneratedAmbientMode::new(config, 0).unwrap();
        let initial_combined = open.combined_energy(&ambient).unwrap();
        assert!(initial_combined.cross_energy.abs() > 1.0e-12);

        open.advance_substeps_with_ambient(4, &ambient).unwrap();

        let final_combined = open.combined_energy(&ambient).unwrap();
        let combined_outflow = open.cumulative_combined_outflow_energy();
        let world_drive_work = open.cumulative_world_drive_work();
        let conservation_residual = final_combined.total_energy - initial_combined.total_energy
            + combined_outflow
            - world_drive_work;
        assert!(
            conservation_residual.abs() < 2.0e-3,
            "combined disturbance/ambient balance residual was {conservation_residual}"
        );
        assert!(combined_outflow.abs() > 1.0e-12);
        assert!(world_drive_work.is_finite());

        let mut ambient_only = OpenBoundaryWaveField::new(3, 3, 1.0, 0.2, 2, 2.0).unwrap();
        let ambient_only_config =
            crate::world_generated_ambient_mode::AmbientModeConfig::new(9.0, 2.0, 0.0, 0.0)
                .unwrap();
        let ambient_only_mode =
            crate::world_generated_ambient_mode::WorldGeneratedAmbientMode::new(
                ambient_only_config,
                0,
            )
            .unwrap();
        let ambient_before = ambient_only
            .combined_energy(&ambient_only_mode)
            .unwrap()
            .total_energy;
        ambient_only
            .advance_substeps_with_ambient(4, &ambient_only_mode)
            .unwrap();
        let ambient_after = ambient_only
            .combined_energy(&ambient_only_mode)
            .unwrap()
            .total_energy;
        close(
            ambient_after - ambient_before,
            ambient_only.cumulative_world_drive_work(),
            2.0e-3,
        );
        close(
            ambient_only.cumulative_combined_outflow_energy(),
            0.0,
            1.0e-12,
        );
    }

    #[test]
    fn combined_inventory_rejects_mismatched_ambient_normalization() {
        let open = OpenBoundaryWaveField::new(2, 2, 1.0, 0.2, 2, 2.0).unwrap();
        assert_eq!(
            open.combined_energy_at_velocity_center(0.0, 1.0, 3.0),
            Err(ClosedWaveError::InvalidState)
        );
    }

    #[test]
    fn open_boundary_outflow_and_field_state_survive_serialization() {
        let mut open = OpenBoundaryWaveField::new(17, 13, 1.0, 0.2, 2, 1.0).unwrap();
        open.set_initial_state(1, 6, 0.8, 1.0).unwrap();
        open.advance_ticks(30).unwrap();
        assert!(open.cumulative_outflow_energy() > 0.0);
        let restored: OpenBoundaryWaveField =
            serde_json::from_str(&serde_json::to_string(&open).unwrap()).unwrap();
        close(restored.energy(), open.energy(), 1.0e-12);
        close(
            restored.cumulative_outflow_energy(),
            open.cumulative_outflow_energy(),
            1.0e-12,
        );
        assert_eq!(restored.logical_time_ticks(), open.logical_time_ticks());
    }
}
