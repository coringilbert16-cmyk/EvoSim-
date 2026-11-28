use serde::{Deserialize, Serialize};

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
    /// Spatial gradient synchronized to the leapfrog velocity time centre.
    pub(crate) flux_gradient_x: f64,
    pub(crate) flux_gradient_y: f64,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
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
    /// Number of completed logical simulation ticks represented by this field.
    pub(crate) fn logical_time_ticks(&self) -> f64 {
        self.substeps_elapsed as f64 / self.substeps_per_tick as f64
    }

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

    fn local_impulse_energy_coefficients(
        &self,
        x_l: f64,
        y_l: f64,
    ) -> Result<(f64, f64), ClosedWaveError> {
        let nodes = self.local_interaction_nodes(x_l, y_l)?;
        let baseline = self.energy();
        let mut plus = self.clone();
        let mut minus = self.clone();
        plus.apply_local_impulse_nodes(&nodes, 1.0)?;
        minus.apply_local_impulse_nodes(&nodes, -1.0)?;
        let plus_delta = plus.energy() - baseline;
        let minus_delta = minus.energy() - baseline;
        let quadratic = 0.5 * (plus_delta + minus_delta);
        let linear = 0.5 * (plus_delta - minus_delta);
        if !quadratic.is_finite() || quadratic <= 0.0 || !linear.is_finite() {
            return Err(ClosedWaveError::InvalidState);
        }
        Ok((quadratic, linear))
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
        let before = self.clone();
        self.apply_local_impulse_nodes(&nodes, impulse)?;
        let actual_delta = self.energy() - before.energy();
        let tolerance = 2.0e-10 * expected_delta.abs().max(1.0);
        if !actual_delta.is_finite() || (actual_delta - expected_delta).abs() > tolerance {
            *self = before;
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
        Ok([
            (self.index(x0, y0), (1.0 - tx) * (1.0 - ty)),
            (self.index(x1, y0), tx * (1.0 - ty)),
            (self.index(x0, y1), (1.0 - tx) * ty),
            (self.index(x1, y1), tx * ty),
        ])
    }

    fn apply_local_impulse_nodes(
        &mut self,
        nodes: &[(usize, f64); 4],
        impulse: f64,
    ) -> Result<(), ClosedWaveError> {
        let mut updates = Vec::with_capacity(4);
        for (index, weight) in nodes {
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
}
