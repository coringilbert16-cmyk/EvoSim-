//! Sparse, balanced quadtree storage for the production spatial resonance field.
//!
//! The adaptive grid owns physical cell coordinates, persistent leapfrog
//! state, conservative refine/coarsen transfers, and a finite-volume wave
//! operator over coarse/fine shared faces. It remains isolated from live
//! simulation ticks and production energy accounting pending reference parity
//! and whole-system integration validation.

use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AdaptiveGridError {
    InvalidDimensions,
    InvalidParameter,
    MaximumRefinementReached,
    MissingCell,
    IncompleteChildren,
    UnbalancedRefinement,
    UnstableTimeStep,
    AlreadyStarted,
    CounterOverflow,
    InvalidState,
    UnrepresentableEnergyTransfer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct CellKey {
    /// Refinement level; zero is the configured base-cell resolution.
    pub(crate) level: u8,
    /// Integer cell coordinate at this level.
    pub(crate) x: u32,
    pub(crate) y: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AdaptiveCell {
    pub(crate) previous: f64,
    pub(crate) current: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CellBounds {
    pub(crate) min_x: f64,
    pub(crate) min_y: f64,
    pub(crate) max_x: f64,
    pub(crate) max_y: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SharedFace {
    pub(crate) first: CellKey,
    pub(crate) second: CellKey,
    /// Physical length of the shared face.
    pub(crate) length: f64,
    /// Physical distance between the two cell centers, normal to the face.
    pub(crate) center_distance: f64,
    /// Endpoints of this shared face segment in finest-grid integer coordinates.
    pub(crate) start_vertex: (u32, u32),
    pub(crate) end_vertex: (u32, u32),
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct IntegratedState {
    /// Area integral of displacement at the current leapfrog level.
    pub(crate) current_displacement: f64,
    /// Area integral of displacement at the previous leapfrog level.
    pub(crate) previous_displacement: f64,
}

#[derive(Clone, Debug)]
pub(crate) struct AdaptiveSpatialGrid {
    width: u32,
    height: u32,
    base_spacing: f64,
    max_refinement_level: u8,
    leaves: BTreeMap<CellKey, AdaptiveCell>,
}

impl AdaptiveSpatialGrid {
    pub(crate) fn new(
        width: u32,
        height: u32,
        base_spacing: f64,
        max_refinement_level: u8,
    ) -> Result<Self, AdaptiveGridError> {
        if width == 0 || height == 0 {
            return Err(AdaptiveGridError::InvalidDimensions);
        }
        if !base_spacing.is_finite() || base_spacing <= 0.0 || max_refinement_level > 20 {
            return Err(AdaptiveGridError::InvalidParameter);
        }
        let finest_scale = 1_u32
            .checked_shl(max_refinement_level as u32)
            .ok_or(AdaptiveGridError::InvalidDimensions)?;
        width
            .checked_mul(finest_scale)
            .and_then(|_| height.checked_mul(finest_scale))
            .ok_or(AdaptiveGridError::InvalidDimensions)?;
        let finest_spacing = base_spacing / finest_scale as f64;
        let world_width = width as f64 * base_spacing;
        let world_height = height as f64 * base_spacing;
        if !finest_spacing.is_finite()
            || finest_spacing <= 0.0
            || !(finest_spacing * finest_spacing).is_finite()
            || finest_spacing * finest_spacing <= 0.0
            || !world_width.is_finite()
            || !world_height.is_finite()
        {
            return Err(AdaptiveGridError::InvalidParameter);
        }
        let capacity = (width as usize)
            .checked_mul(height as usize)
            .ok_or(AdaptiveGridError::InvalidDimensions)?;
        let mut leaves = BTreeMap::new();
        for y in 0..height {
            for x in 0..width {
                leaves.insert(
                    CellKey { level: 0, x, y },
                    AdaptiveCell {
                        previous: 0.0,
                        current: 0.0,
                    },
                );
            }
        }
        debug_assert_eq!(leaves.len(), capacity);
        Ok(Self {
            width,
            height,
            base_spacing,
            max_refinement_level,
            leaves,
        })
    }

    pub(crate) fn width(&self) -> u32 {
        self.width
    }

    pub(crate) fn height(&self) -> u32 {
        self.height
    }

    pub(crate) fn base_spacing(&self) -> f64 {
        self.base_spacing
    }

    pub(crate) fn max_refinement_level(&self) -> u8 {
        self.max_refinement_level
    }

    pub(crate) fn leaf_count(&self) -> usize {
        self.leaves.len()
    }

    pub(crate) fn leaves(&self) -> impl Iterator<Item = (CellKey, AdaptiveCell)> + '_ {
        self.leaves.iter().map(|(key, state)| (*key, *state))
    }

    pub(crate) fn cell_bounds(&self, key: CellKey) -> Option<CellBounds> {
        if !self.leaves.contains_key(&key) {
            return None;
        }
        let scale = 2_u32.checked_pow(key.level as u32)? as f64;
        let spacing = self.base_spacing / scale;
        let min_x = key.x as f64 * spacing;
        let min_y = key.y as f64 * spacing;
        Some(CellBounds {
            min_x,
            min_y,
            max_x: min_x + spacing,
            max_y: min_y + spacing,
        })
    }

    pub(crate) fn set_state(
        &mut self,
        key: CellKey,
        state: AdaptiveCell,
    ) -> Result<(), AdaptiveGridError> {
        if !state.previous.is_finite() || !state.current.is_finite() {
            return Err(AdaptiveGridError::InvalidState);
        }
        let cell = self
            .leaves
            .get_mut(&key)
            .ok_or(AdaptiveGridError::MissingCell)?;
        *cell = state;
        Ok(())
    }

    /// Refines one leaf into four children. Child values inherit the parent
    /// state as piecewise-constant cell averages, preserving both area
    /// integrals exactly. Refinement is rejected if it would violate 2:1
    /// balance with an existing face neighbor.
    pub(crate) fn refine(&mut self, key: CellKey) -> Result<[CellKey; 4], AdaptiveGridError> {
        let parent = *self
            .leaves
            .get(&key)
            .ok_or(AdaptiveGridError::MissingCell)?;
        if key.level >= self.max_refinement_level {
            return Err(AdaptiveGridError::MaximumRefinementReached);
        }
        let child_level = key.level + 1;
        let child_x = key
            .x
            .checked_mul(2)
            .ok_or(AdaptiveGridError::InvalidDimensions)?;
        let child_y = key
            .y
            .checked_mul(2)
            .ok_or(AdaptiveGridError::InvalidDimensions)?;
        let children = [
            CellKey {
                level: child_level,
                x: child_x,
                y: child_y,
            },
            CellKey {
                level: child_level,
                x: child_x + 1,
                y: child_y,
            },
            CellKey {
                level: child_level,
                x: child_x,
                y: child_y + 1,
            },
            CellKey {
                level: child_level,
                x: child_x + 1,
                y: child_y + 1,
            },
        ];

        let parent_bounds = self
            .cell_bounds(key)
            .ok_or(AdaptiveGridError::MissingCell)?;
        for neighbor_key in self.leaves.keys() {
            if *neighbor_key != key
                && cells_share_face(parent_bounds, self.cell_bounds(*neighbor_key).unwrap())
                && neighbor_key.level + 1 < child_level
            {
                return Err(AdaptiveGridError::UnbalancedRefinement);
            }
        }

        self.leaves.remove(&key);
        for child in children {
            self.leaves.insert(child, parent);
        }
        Ok(children)
    }

    /// Coarsens four sibling leaves into their parent using area-weighted
    /// restriction. The parent is not created if it would violate 2:1 balance.
    pub(crate) fn coarsen(&mut self, parent: CellKey) -> Result<AdaptiveCell, AdaptiveGridError> {
        let child_level = parent
            .level
            .checked_add(1)
            .ok_or(AdaptiveGridError::IncompleteChildren)?;
        let child_x = parent
            .x
            .checked_mul(2)
            .ok_or(AdaptiveGridError::IncompleteChildren)?;
        let child_y = parent
            .y
            .checked_mul(2)
            .ok_or(AdaptiveGridError::IncompleteChildren)?;
        let children = [
            CellKey {
                level: child_level,
                x: child_x,
                y: child_y,
            },
            CellKey {
                level: child_level,
                x: child_x + 1,
                y: child_y,
            },
            CellKey {
                level: child_level,
                x: child_x,
                y: child_y + 1,
            },
            CellKey {
                level: child_level,
                x: child_x + 1,
                y: child_y + 1,
            },
        ];
        let states = children.map(|key| {
            self.leaves
                .get(&key)
                .copied()
                .ok_or(AdaptiveGridError::IncompleteChildren)
        });
        let [a, b, c, d] = states;
        let states = [a?, b?, c?, d?];
        let parent_state = AdaptiveCell {
            previous: states.iter().map(|state| state.previous).sum::<f64>() * 0.25,
            current: states.iter().map(|state| state.current).sum::<f64>() * 0.25,
        };
        if !parent_state.previous.is_finite() || !parent_state.current.is_finite() {
            return Err(AdaptiveGridError::InvalidState);
        }

        let parent_bounds = self.bounds_for_key(parent);
        for neighbor_key in self.leaves.keys() {
            if children.contains(neighbor_key) {
                continue;
            }
            if cells_share_face(parent_bounds, self.cell_bounds(*neighbor_key).unwrap())
                && neighbor_key.level > parent.level + 1
            {
                return Err(AdaptiveGridError::UnbalancedRefinement);
            }
        }

        for child in children {
            self.leaves.remove(&child);
        }
        self.leaves.insert(parent, parent_state);
        Ok(parent_state)
    }

    /// Returns the state of the leaf containing a physical position. Cell
    /// values are finite-volume averages; this is not a nearest-cell fallback.
    pub(crate) fn sample(&self, x: f64, y: f64) -> Option<AdaptiveCell> {
        if !x.is_finite()
            || !y.is_finite()
            || x < 0.0
            || y < 0.0
            || x >= self.width as f64 * self.base_spacing
            || y >= self.height as f64 * self.base_spacing
        {
            return None;
        }
        for level in (0..=self.max_refinement_level).rev() {
            let spacing = self.base_spacing / 2_u32.checked_pow(level as u32)? as f64;
            let key = CellKey {
                level,
                x: (x / spacing).floor() as u32,
                y: (y / spacing).floor() as u32,
            };
            if let Some(state) = self.leaves.get(&key) {
                return Some(*state);
            }
        }
        None
    }

    /// Enumerates each face-connected leaf pair once in deterministic key order.
    /// Face conductance for the finite-volume operator is length / center distance.
    pub(crate) fn shared_faces(&self) -> Vec<SharedFace> {
        type Segment = (u32, u32, CellKey);
        let finest_scale = 1_u32 << self.max_refinement_level;
        let finest_spacing = self.base_spacing / finest_scale as f64;
        let mut vertical_left: BTreeMap<u32, Vec<Segment>> = BTreeMap::new();
        let mut vertical_right: BTreeMap<u32, Vec<Segment>> = BTreeMap::new();
        let mut horizontal_bottom: BTreeMap<u32, Vec<Segment>> = BTreeMap::new();
        let mut horizontal_top: BTreeMap<u32, Vec<Segment>> = BTreeMap::new();

        for key in self.leaves.keys().copied() {
            let scale = 1_u32 << (self.max_refinement_level - key.level);
            let min_x = key.x * scale;
            let min_y = key.y * scale;
            let max_x = min_x + scale;
            let max_y = min_y + scale;
            vertical_left
                .entry(min_x)
                .or_default()
                .push((min_y, max_y, key));
            vertical_right
                .entry(max_x)
                .or_default()
                .push((min_y, max_y, key));
            horizontal_bottom
                .entry(min_y)
                .or_default()
                .push((min_x, max_x, key));
            horizontal_top
                .entry(max_y)
                .or_default()
                .push((min_x, max_x, key));
        }

        let mut faces = Vec::new();
        append_shared_faces(
            &vertical_right,
            &vertical_left,
            finest_spacing,
            self.max_refinement_level,
            true,
            &mut faces,
        );
        append_shared_faces(
            &horizontal_top,
            &horizontal_bottom,
            finest_spacing,
            self.max_refinement_level,
            false,
            &mut faces,
        );
        faces.sort_by_key(|face| (face.first, face.second));
        faces
    }

    /// Area integrals of the two persistent leapfrog displacement levels.
    /// Refinement and coarsening preserve these quantities to roundoff.
    pub(crate) fn integrated_state(&self) -> IntegratedState {
        let mut total = IntegratedState::default();
        for (key, state) in &self.leaves {
            let area = self.cell_area(*key);
            total.current_displacement += area * state.current;
            total.previous_displacement += area * state.previous;
        }
        total
    }

    fn bounds_for_key(&self, key: CellKey) -> CellBounds {
        let spacing = self.base_spacing / 2_u32.pow(key.level as u32) as f64;
        let min_x = key.x as f64 * spacing;
        let min_y = key.y as f64 * spacing;
        CellBounds {
            min_x,
            min_y,
            max_x: min_x + spacing,
            max_y: min_y + spacing,
        }
    }

    fn cell_area(&self, key: CellKey) -> f64 {
        let spacing = self.base_spacing / 2_u32.pow(key.level as u32) as f64;
        spacing * spacing
    }
}

/// Conservative leapfrog evolution on a fixed adaptive mesh.
///
/// Cell areas form the diagonal mass matrix. Uniform meshes retain the
/// validated two-point finite-volume stencil. Nonuniform meshes use a
/// symmetric variational multipoint stiffness operator: face-endpoint
/// subtriangles are assembled as linear finite elements and their massless
/// vertex degrees of freedom are statically condensed. This accounts for
/// tangential center offsets while retaining a symmetric positive
/// semidefinite stiffness matrix and a matching discrete energy invariant.
#[derive(Clone, Debug)]
pub(crate) struct AdaptiveWaveField {
    grid: AdaptiveSpatialGrid,
    stiffness: StiffnessMatrix,
    propagation_speed: f64,
    energy_normalization: f64,
    time_step: f64,
    substeps_per_tick: usize,
    absorbing_layer: Option<(f64, f64)>,
    absorbed_energy: f64,
    substeps_elapsed: u64,
}

impl AdaptiveWaveField {
    /// Chooses the smallest integer substep count satisfying a conservative
    /// Gershgorin CFL bound for the current adaptive mesh.
    pub(crate) fn new(
        grid: AdaptiveSpatialGrid,
        propagation_speed: f64,
        energy_normalization: f64,
    ) -> Result<Self, AdaptiveGridError> {
        if !propagation_speed.is_finite()
            || propagation_speed <= 0.0
            || !energy_normalization.is_finite()
            || energy_normalization <= 0.0
        {
            return Err(AdaptiveGridError::InvalidParameter);
        }
        for (key, state) in grid.leaves() {
            if is_computational_boundary(&grid, key)
                && (state.previous != 0.0 || state.current != 0.0)
            {
                return Err(AdaptiveGridError::InvalidState);
            }
        }

        let stiffness = stiffness_matrix(&grid);
        let maximum_row_sum = maximum_stiffness_row_sum(&grid, &stiffness);
        let required_substeps = if maximum_row_sum == 0.0 {
            1.0
        } else {
            let maximum_time_step = (4.0 / maximum_row_sum).sqrt() / propagation_speed;
            (1.0 / maximum_time_step).ceil().max(1.0)
        };
        if !required_substeps.is_finite() || required_substeps > usize::MAX as f64 {
            return Err(AdaptiveGridError::InvalidParameter);
        }
        let substeps_per_tick = required_substeps as usize;
        let time_step = 1.0 / substeps_per_tick as f64;
        if !time_step.is_finite()
            || propagation_speed * time_step * maximum_row_sum.sqrt() > 2.0 + 1.0e-12
        {
            return Err(AdaptiveGridError::UnstableTimeStep);
        }

        Ok(Self {
            grid,
            stiffness,
            propagation_speed,
            energy_normalization,
            time_step,
            substeps_per_tick,
            absorbing_layer: None,
            absorbed_energy: 0.0,
            substeps_elapsed: 0,
        })
    }

    pub(crate) fn grid(&self) -> &AdaptiveSpatialGrid {
        &self.grid
    }

    /// Refines a leaf while preserving the field's discrete leapfrog energy.
    ///
    /// The time step is held fixed. A topology change that would violate the
    /// existing CFL bound is rejected rather than silently changing the
    /// temporal energy metric.
    pub(crate) fn refine_grid(&mut self, key: CellKey) -> Result<[CellKey; 4], AdaptiveGridError> {
        self.regrid_preserving_energy(|grid| grid.refine(key))
    }

    /// Coarsens four siblings while preserving the field's discrete leapfrog
    /// energy. If the coarsened mesh has no non-uniform degrees of freedom in
    /// which to represent the removed energy, the operation is rejected.
    pub(crate) fn coarsen_grid(
        &mut self,
        parent: CellKey,
    ) -> Result<AdaptiveCell, AdaptiveGridError> {
        self.regrid_preserving_energy(|grid| grid.coarsen(parent))
    }

    fn regrid_preserving_energy<T>(
        &mut self,
        transfer: impl FnOnce(&mut AdaptiveSpatialGrid) -> Result<T, AdaptiveGridError>,
    ) -> Result<T, AdaptiveGridError> {
        let target_energy = self.energy();
        let target_integrals = self.grid.integrated_state();
        let mut candidate = self.grid.clone();
        let result = transfer(&mut candidate)?;

        for (key, state) in candidate.leaves() {
            if is_computational_boundary(&candidate, key)
                && (state.previous != 0.0 || state.current != 0.0)
            {
                return Err(AdaptiveGridError::InvalidState);
            }
        }

        // Keep the time step invariant so a topology change cannot alter the
        // kinetic term's temporal metric. Reject refinements that exceed the
        // original solver's stability envelope.
        let candidate_stiffness = stiffness_matrix(&candidate);
        let maximum_row_sum = maximum_stiffness_row_sum(&candidate, &candidate_stiffness);
        let stability_measure = self.propagation_speed * self.time_step * maximum_row_sum.sqrt();
        if !stability_measure.is_finite() || stability_measure > 2.0 + 1.0e-12 {
            return Err(AdaptiveGridError::UnstableTimeStep);
        }

        preserve_regridded_energy(
            &mut candidate,
            self.time_step,
            self.propagation_speed,
            self.energy_normalization,
            target_energy,
        )?;

        let transferred_integrals = candidate.integrated_state();
        let integral_scale = target_integrals
            .current_displacement
            .abs()
            .max(target_integrals.previous_displacement.abs())
            .max(1.0);
        let integral_tolerance = 1.0e-11 * integral_scale;
        if (transferred_integrals.current_displacement - target_integrals.current_displacement)
            .abs()
            > integral_tolerance
            || (transferred_integrals.previous_displacement
                - target_integrals.previous_displacement)
                .abs()
                > integral_tolerance
        {
            return Err(AdaptiveGridError::UnrepresentableEnergyTransfer);
        }

        self.grid = candidate;
        self.stiffness = candidate_stiffness;
        Ok(result)
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

    pub(crate) fn absorbed_energy(&self) -> f64 {
        self.absorbed_energy
    }

    pub(crate) fn sample(&self, x: f64, y: f64) -> Option<f64> {
        self.grid.sample(x, y).map(|state| state.current)
    }

    /// Configures an energy-accounted computational sponge. Outer leaf cells
    /// remain fixed-zero numerical boundary nodes; damping acts only inside
    /// the computational window and its exact discrete loss is accumulated as
    /// computational outflow. This is not a physical world boundary.
    pub(crate) fn configure_absorbing_boundary(
        &mut self,
        layer_width: f64,
        maximum_damping: f64,
    ) -> Result<(), AdaptiveGridError> {
        if self.substeps_elapsed != 0 {
            return Err(AdaptiveGridError::AlreadyStarted);
        }
        if !layer_width.is_finite()
            || layer_width <= 0.0
            || !maximum_damping.is_finite()
            || maximum_damping <= 0.0
            || !(maximum_damping * self.time_step).is_finite()
        {
            return Err(AdaptiveGridError::InvalidParameter);
        }
        self.absorbing_layer = Some((layer_width, maximum_damping));
        self.absorbed_energy = 0.0;
        Ok(())
    }

    /// Conserved discrete leapfrog energy on the current adaptive mesh.
    pub(crate) fn energy(&self) -> f64 {
        self.discrete_energy(
            &self
                .grid
                .leaves
                .iter()
                .map(|(key, state)| (*key, state.previous))
                .collect(),
            &self
                .grid
                .leaves
                .iter()
                .map(|(key, state)| (*key, state.current))
                .collect(),
        )
    }

    pub(crate) fn advance_ticks(&mut self, ticks: u64) -> Result<(), AdaptiveGridError> {
        let count = ticks
            .checked_mul(self.substeps_per_tick as u64)
            .ok_or(AdaptiveGridError::CounterOverflow)?;
        self.advance_substeps(count)
    }

    pub(crate) fn advance_substeps(&mut self, count: u64) -> Result<(), AdaptiveGridError> {
        self.substeps_elapsed
            .checked_add(count)
            .ok_or(AdaptiveGridError::CounterOverflow)?;
        for _ in 0..count {
            self.advance_one_substep()?;
        }
        Ok(())
    }

    fn advance_one_substep(&mut self) -> Result<(), AdaptiveGridError> {
        let mut face_sums: BTreeMap<CellKey, f64> =
            self.grid.leaves.keys().map(|key| (*key, 0.0)).collect();
        for (row, columns) in &self.stiffness {
            let applied = columns
                .iter()
                .map(|(column, coefficient)| coefficient * self.grid.leaves[column].current)
                .sum::<f64>();
            face_sums.insert(*row, -applied);
        }

        let dt = self.time_step;
        let speed_squared = self.propagation_speed.powi(2);
        let old_energy = self.energy();
        let mut next_current = BTreeMap::new();
        let mut absorbed_this_step = 0.0;

        for (key, state) in self.grid.leaves.iter() {
            if is_computational_boundary(&self.grid, *key) {
                next_current.insert(*key, 0.0);
                continue;
            }
            let bounds = self.grid.bounds_for_key(*key);
            let center_x = 0.5 * (bounds.min_x + bounds.max_x);
            let center_y = 0.5 * (bounds.min_y + bounds.max_y);
            let damping = self.damping_at(center_x, center_y);
            let damping_step = 0.5 * damping * dt;
            let area = self.grid.cell_area(*key);
            let next = (2.0 * state.current - (1.0 - damping_step) * state.previous
                + dt.powi(2) * speed_squared * face_sums[key] / area)
                / (1.0 + damping_step);
            if !next.is_finite() {
                return Err(AdaptiveGridError::InvalidState);
            }
            let midpoint_velocity = (next - state.previous) / (2.0 * dt);
            absorbed_this_step += area * damping * dt * midpoint_velocity.powi(2);
            next_current.insert(*key, next);
        }

        absorbed_this_step *= self.energy_normalization;
        let next_energy = self.discrete_energy(
            &self
                .grid
                .leaves
                .iter()
                .map(|(key, state)| (*key, state.current))
                .collect(),
            &next_current,
        );
        let measured_loss = old_energy - next_energy;
        let tolerance = 1.0e-10
            * old_energy
                .abs()
                .max(next_energy.abs())
                .max(absorbed_this_step.abs())
                .max(1.0);
        let next_absorbed_energy = self.absorbed_energy + absorbed_this_step;
        if !next_energy.is_finite()
            || next_energy < 0.0
            || !absorbed_this_step.is_finite()
            || absorbed_this_step < 0.0
            || !measured_loss.is_finite()
            || measured_loss < -tolerance
            || (measured_loss - absorbed_this_step).abs() > tolerance
            || !next_absorbed_energy.is_finite()
        {
            return Err(AdaptiveGridError::InvalidState);
        }
        let next_substeps_elapsed = self
            .substeps_elapsed
            .checked_add(1)
            .ok_or(AdaptiveGridError::CounterOverflow)?;

        for (key, next) in next_current {
            let state = self.grid.leaves.get_mut(&key).unwrap();
            state.previous = state.current;
            state.current = next;
        }
        self.absorbed_energy = next_absorbed_energy;
        self.substeps_elapsed = next_substeps_elapsed;
        Ok(())
    }

    fn damping_at(&self, x: f64, y: f64) -> f64 {
        let Some((layer_width, maximum_damping)) = self.absorbing_layer else {
            return 0.0;
        };
        let world_width = self.grid.width as f64 * self.grid.base_spacing;
        let world_height = self.grid.height as f64 * self.grid.base_spacing;
        let distance = x.min(world_width - x).min(y).min(world_height - y);
        if distance >= layer_width {
            0.0
        } else {
            let depth = (layer_width - distance) / layer_width;
            maximum_damping * depth * depth
        }
    }

    fn discrete_energy(
        &self,
        previous: &BTreeMap<CellKey, f64>,
        current: &BTreeMap<CellKey, f64>,
    ) -> f64 {
        discrete_energy_with_stiffness(
            &self.grid,
            &self.stiffness,
            previous,
            current,
            self.time_step,
            self.propagation_speed,
            self.energy_normalization,
        )
    }
}

/// Transfers the energy norm after conservative area projection.
///
/// In midpoint/velocity coordinates, the leapfrog invariant separates into
/// two constrained quadratic forms:
///   mᵀ K m and vᵀ (M - c² dt² K / 4) v.
/// We solve each constrained minimum, then scale only the zero-integral
/// residual. The minimum is energy-orthogonal to every admissible residual.
fn preserve_regridded_energy(
    grid: &mut AdaptiveSpatialGrid,
    time_step: f64,
    propagation_speed: f64,
    normalization: f64,
    target_energy: f64,
) -> Result<(), AdaptiveGridError> {
    let stiffness = stiffness_matrix(grid);
    let keys: Vec<_> = grid
        .leaves
        .keys()
        .copied()
        .filter(|key| !is_computational_boundary(grid, *key))
        .collect();
    if keys.is_empty() {
        return Err(AdaptiveGridError::UnrepresentableEnergyTransfer);
    }
    let areas: Vec<_> = keys.iter().map(|key| grid.cell_area(*key)).collect();
    let alpha = propagation_speed.powi(2) * time_step.powi(2) * 0.25;
    let integrals = grid.integrated_state();
    let mean_target = 0.5 * (integrals.current_displacement + integrals.previous_displacement);
    let velocity_target =
        (integrals.current_displacement - integrals.previous_displacement) / time_step;

    let mean_minimum =
        constrained_quadratic_minimizer(&keys, &areas, &stiffness, 0.0, mean_target)?;
    let velocity_minimum =
        constrained_quadratic_minimizer(&keys, &areas, &stiffness, alpha, velocity_target)?;

    let mut baseline_previous = BTreeMap::new();
    let mut baseline_current = BTreeMap::new();
    let mut projected_previous = BTreeMap::new();
    let mut projected_current = BTreeMap::new();
    for (key, state) in grid.leaves() {
        if is_computational_boundary(grid, key) {
            baseline_previous.insert(key, 0.0);
            baseline_current.insert(key, 0.0);
            projected_previous.insert(key, 0.0);
            projected_current.insert(key, 0.0);
        } else {
            let mean = mean_minimum[&key];
            let velocity = velocity_minimum[&key];
            baseline_previous.insert(key, mean - 0.5 * time_step * velocity);
            baseline_current.insert(key, mean + 0.5 * time_step * velocity);
            projected_previous.insert(key, state.previous);
            projected_current.insert(key, state.current);
        }
    }

    let minimum_energy = discrete_energy_with_stiffness(
        grid,
        &stiffness,
        &baseline_previous,
        &baseline_current,
        time_step,
        propagation_speed,
        normalization,
    );
    let projected_energy = discrete_energy_with_stiffness(
        grid,
        &stiffness,
        &projected_previous,
        &projected_current,
        time_step,
        propagation_speed,
        normalization,
    );
    let energy_scale = target_energy
        .abs()
        .max(minimum_energy.abs())
        .max(projected_energy.abs())
        .max(1.0);
    let tolerance = 1.0e-10 * energy_scale;
    let available_energy = projected_energy - minimum_energy;
    let required_energy = target_energy - minimum_energy;
    if !minimum_energy.is_finite()
        || !projected_energy.is_finite()
        || available_energy < -tolerance
        || required_energy < -tolerance
    {
        return Err(AdaptiveGridError::UnrepresentableEnergyTransfer);
    }
    let transfer_scale = if required_energy <= tolerance {
        0.0
    } else if available_energy <= tolerance {
        return Err(AdaptiveGridError::UnrepresentableEnergyTransfer);
    } else {
        (required_energy / available_energy).sqrt()
    };
    if !transfer_scale.is_finite() {
        return Err(AdaptiveGridError::UnrepresentableEnergyTransfer);
    }

    for (key, state) in grid.leaves.iter_mut() {
        state.previous = baseline_previous[key]
            + transfer_scale * (projected_previous[key] - baseline_previous[key]);
        state.current = baseline_current[key]
            + transfer_scale * (projected_current[key] - baseline_current[key]);
        if !state.previous.is_finite() || !state.current.is_finite() {
            return Err(AdaptiveGridError::InvalidState);
        }
    }
    let previous: BTreeMap<_, _> = grid
        .leaves
        .iter()
        .map(|(key, state)| (*key, state.previous))
        .collect();
    let current: BTreeMap<_, _> = grid
        .leaves
        .iter()
        .map(|(key, state)| (*key, state.current))
        .collect();
    let final_energy = discrete_energy_with_stiffness(
        grid,
        &stiffness,
        &previous,
        &current,
        time_step,
        propagation_speed,
        normalization,
    );
    if !final_energy.is_finite() || (final_energy - target_energy).abs() > tolerance {
        return Err(AdaptiveGridError::UnrepresentableEnergyTransfer);
    }
    Ok(())
}

/// Finds the minimum of xᵀ(M - alpha K)x subject to the area-integral
/// constraint. Alpha zero gives the displacement minimum; positive alpha
/// gives the leapfrog velocity minimum. Boundary cells remain fixed at zero.
fn constrained_quadratic_minimizer(
    keys: &[CellKey],
    areas: &[f64],
    stiffness: &StiffnessMatrix,
    alpha: f64,
    target_integral: f64,
) -> Result<BTreeMap<CellKey, f64>, AdaptiveGridError> {
    let count = keys.len();
    let indices: BTreeMap<_, _> = keys
        .iter()
        .enumerate()
        .map(|(i, key)| (*key, i))
        .collect();
    let mut diagonal = vec![0.0; count];
    let rhs = areas.to_vec();
    for (index, key) in keys.iter().enumerate() {
        let kii = stiffness
            .get(key)
            .and_then(|row| row.get(key))
            .copied()
            .unwrap_or(0.0);
        diagonal[index] = areas[index] - alpha * kii;
        if !diagonal[index].is_finite() || diagonal[index] <= 0.0 {
            return Err(AdaptiveGridError::UnrepresentableEnergyTransfer);
        }
    }
    let apply = |input: &[f64]| -> Vec<f64> {
        let mut output = vec![0.0; count];
        for (row_index, key) in keys.iter().enumerate() {
            let mut kx = 0.0;
            if let Some(columns) = stiffness.get(key) {
                for (column, coefficient) in columns {
                    if let Some(column_index) = indices.get(column) {
                        kx += coefficient * input[*column_index];
                    }
                }
            }
            output[row_index] = areas[row_index] * input[row_index] - alpha * kx;
        }
        output
    };

    // Jacobi-preconditioned conjugate gradients retain sparse matrix storage.
    let mut solution = vec![0.0; count];
    let mut residual = rhs.clone();
    let mut preconditioned: Vec<_> = residual
        .iter()
        .zip(&diagonal)
        .map(|(value, d)| value / d)
        .collect();
    let mut direction = preconditioned.clone();
    let mut residual_product = dot_product(&residual, &preconditioned);
    let tolerance = 1.0e-12 * dot_product(&rhs, &rhs).sqrt().max(1.0);
    let maximum_iterations = count.saturating_mul(20).max(100);
    let mut converged = dot_product(&residual, &residual).sqrt() <= tolerance;
    for _ in 0..maximum_iterations {
        if converged {
            break;
        }
        let applied = apply(&direction);
        let denominator = dot_product(&direction, &applied);
        if !denominator.is_finite() || denominator <= 0.0 {
            return Err(AdaptiveGridError::UnrepresentableEnergyTransfer);
        }
        let step = residual_product / denominator;
        for i in 0..count {
            solution[i] += step * direction[i];
            residual[i] -= step * applied[i];
        }
        if dot_product(&residual, &residual).sqrt() <= tolerance {
            converged = true;
            break;
        }
        for i in 0..count {
            preconditioned[i] = residual[i] / diagonal[i];
        }
        let next_product = dot_product(&residual, &preconditioned);
        if !next_product.is_finite() || residual_product <= 0.0 {
            return Err(AdaptiveGridError::UnrepresentableEnergyTransfer);
        }
        let beta = next_product / residual_product;
        for i in 0..count {
            direction[i] = preconditioned[i] + beta * direction[i];
        }
        residual_product = next_product;
    }
    if !converged {
        return Err(AdaptiveGridError::UnrepresentableEnergyTransfer);
    }
    let denominator = keys
        .iter()
        .enumerate()
        .map(|(i, _)| areas[i] * solution[i])
        .sum::<f64>();
    if !denominator.is_finite() || denominator <= 0.0 {
        return Err(AdaptiveGridError::UnrepresentableEnergyTransfer);
    }
    let scale = target_integral / denominator;
    if !scale.is_finite() {
        return Err(AdaptiveGridError::UnrepresentableEnergyTransfer);
    }
    Ok(keys
        .iter()
        .enumerate()
        .map(|(i, key)| (*key, scale * solution[i]))
        .collect())
}

fn dot_product(first: &[f64], second: &[f64]) -> f64 {
    first
        .iter()
        .zip(second)
        .map(|(left, right)| left * right)
        .sum()
}

type StiffnessMatrix = BTreeMap<CellKey, BTreeMap<CellKey, f64>>;

#[derive(Default)]
struct VertexStiffness {
    cell_block: BTreeMap<(CellKey, CellKey), f64>,
    cell_vertex: BTreeMap<CellKey, f64>,
    vertex_diagonal: f64,
}

fn add_stiffness_entry(matrix: &mut StiffnessMatrix, row: CellKey, column: CellKey, value: f64) {
    *matrix.entry(row).or_default().entry(column).or_default() += value;
}

fn triangle_stiffness(first: (f64, f64), second: (f64, f64), vertex: (f64, f64)) -> [[f64; 3]; 3] {
    let points = [first, second, vertex];
    let cross = (points[1].0 - points[0].0) * (points[2].1 - points[0].1)
        - (points[2].0 - points[0].0) * (points[1].1 - points[0].1);
    let area = 0.5 * cross.abs();
    let gradients = [
        [
            (points[1].1 - points[2].1) / cross,
            (points[2].0 - points[1].0) / cross,
        ],
        [
            (points[2].1 - points[0].1) / cross,
            (points[0].0 - points[2].0) / cross,
        ],
        [
            (points[0].1 - points[1].1) / cross,
            (points[1].0 - points[0].0) / cross,
        ],
    ];
    let mut result = [[0.0; 3]; 3];
    for row in 0..3 {
        for column in 0..3 {
            result[row][column] = area
                * (gradients[row][0] * gradients[column][0]
                    + gradients[row][1] * gradients[column][1]);
        }
    }
    result
}

/// Builds the symmetric stiffness matrix for the adaptive finite-volume state.
///
/// On a uniform mesh, the orthogonal two-point operator is retained exactly.
/// On a nonuniform mesh, each shared-face endpoint defines a triangle between
/// the two adjacent cell centers and that geometric vertex. Linear finite
/// elements are assembled around each vertex, then the massless vertex
/// degree of freedom is statically condensed. The resulting cell-centered
/// multipoint operator is symmetric and variational, and reproduces affine
/// fields across hanging-node interfaces.
fn stiffness_matrix(grid: &AdaptiveSpatialGrid) -> StiffnessMatrix {
    let mut matrix: StiffnessMatrix = grid
        .leaves
        .keys()
        .map(|key| (*key, BTreeMap::new()))
        .collect();
    let Some(first_key) = grid.leaves.keys().next().copied() else {
        return matrix;
    };
    let uniform_level = grid.leaves.keys().all(|key| key.level == first_key.level);

    if uniform_level {
        for face in grid.shared_faces() {
            let conductance = face.length / face.center_distance;
            add_stiffness_entry(&mut matrix, face.first, face.first, conductance);
            add_stiffness_entry(&mut matrix, face.second, face.second, conductance);
            add_stiffness_entry(&mut matrix, face.first, face.second, -conductance);
            add_stiffness_entry(&mut matrix, face.second, face.first, -conductance);
        }
        return matrix;
    }

    let finest_scale = 1_u32 << grid.max_refinement_level;
    let finest_spacing = grid.base_spacing / finest_scale as f64;
    let mut vertices: BTreeMap<(u32, u32), VertexStiffness> = BTreeMap::new();

    for face in grid.shared_faces() {
        let first_bounds = grid.bounds_for_key(face.first);
        let second_bounds = grid.bounds_for_key(face.second);
        let first_center = (
            0.5 * (first_bounds.min_x + first_bounds.max_x),
            0.5 * (first_bounds.min_y + first_bounds.max_y),
        );
        let second_center = (
            0.5 * (second_bounds.min_x + second_bounds.max_x),
            0.5 * (second_bounds.min_y + second_bounds.max_y),
        );

        for vertex_key in [face.start_vertex, face.end_vertex] {
            let vertex_position = (
                vertex_key.0 as f64 * finest_spacing,
                vertex_key.1 as f64 * finest_spacing,
            );
            let local = triangle_stiffness(first_center, second_center, vertex_position);
            let star = vertices.entry(vertex_key).or_default();

            for (local_row, row_key) in [face.first, face.second].into_iter().enumerate() {
                for (local_column, column_key) in [face.first, face.second].into_iter().enumerate()
                {
                    *star.cell_block.entry((row_key, column_key)).or_default() +=
                        local[local_row][local_column];
                }
                *star.cell_vertex.entry(row_key).or_default() += local[local_row][2];
            }
            star.vertex_diagonal += local[2][2];
        }
    }

    let world_width = grid.width * finest_scale;
    let world_height = grid.height * finest_scale;
    for (vertex_key, star) in vertices {
        for ((row, column), value) in &star.cell_block {
            add_stiffness_entry(&mut matrix, *row, *column, *value);
        }

        let on_boundary = vertex_key.0 == 0
            || vertex_key.1 == 0
            || vertex_key.0 == world_width
            || vertex_key.1 == world_height;
        if !on_boundary && star.vertex_diagonal > 0.0 {
            for (row, row_coupling) in &star.cell_vertex {
                for (column, column_coupling) in &star.cell_vertex {
                    add_stiffness_entry(
                        &mut matrix,
                        *row,
                        *column,
                        -row_coupling * column_coupling / star.vertex_diagonal,
                    );
                }
            }
        }
    }
    matrix
}

fn discrete_energy_for_grid(
    grid: &AdaptiveSpatialGrid,
    previous: &BTreeMap<CellKey, f64>,
    current: &BTreeMap<CellKey, f64>,
    time_step: f64,
    propagation_speed: f64,
    normalization: f64,
) -> f64 {
    let stiffness = stiffness_matrix(grid);
    discrete_energy_with_stiffness(
        grid,
        &stiffness,
        previous,
        current,
        time_step,
        propagation_speed,
        normalization,
    )
}

fn discrete_energy_with_stiffness(
    grid: &AdaptiveSpatialGrid,
    stiffness: &StiffnessMatrix,
    previous: &BTreeMap<CellKey, f64>,
    current: &BTreeMap<CellKey, f64>,
    time_step: f64,
    propagation_speed: f64,
    normalization: f64,
) -> f64 {
    let mut kinetic_sum = 0.0;
    for key in grid.leaves.keys() {
        let velocity = (current[key] - previous[key]) / time_step;
        kinetic_sum += grid.cell_area(*key) * velocity.powi(2);
    }
    let mut gradient_cross_sum = 0.0;
    for (row, columns) in stiffness {
        for (column, coefficient) in columns {
            gradient_cross_sum += current[row] * coefficient * previous[column];
        }
    }
    0.5 * normalization * (kinetic_sum + propagation_speed.powi(2) * gradient_cross_sum)
}

fn is_computational_boundary(grid: &AdaptiveSpatialGrid, key: CellKey) -> bool {
    let bounds = grid.bounds_for_key(key);
    let world_width = grid.width as f64 * grid.base_spacing;
    let world_height = grid.height as f64 * grid.base_spacing;
    let tolerance = 1.0e-12 * world_width.max(world_height).max(1.0);
    bounds.min_x <= tolerance
        || bounds.min_y <= tolerance
        || (bounds.max_x - world_width).abs() <= tolerance
        || (bounds.max_y - world_height).abs() <= tolerance
}

fn maximum_stiffness_row_sum(grid: &AdaptiveSpatialGrid, stiffness: &StiffnessMatrix) -> f64 {
    stiffness
        .iter()
        .filter(|(key, _)| !is_computational_boundary(grid, **key))
        .map(|(key, columns)| {
            columns.values().map(|value| value.abs()).sum::<f64>() / grid.cell_area(*key)
        })
        .fold(0.0, f64::max)
}

fn append_shared_faces(
    first_sides: &BTreeMap<u32, Vec<(u32, u32, CellKey)>>,
    second_sides: &BTreeMap<u32, Vec<(u32, u32, CellKey)>>,
    finest_spacing: f64,
    max_refinement_level: u8,
    vertical: bool,
    output: &mut Vec<SharedFace>,
) {
    for (coordinate, first_segments) in first_sides {
        let Some(second_segments) = second_sides.get(coordinate) else {
            continue;
        };
        let mut first = first_segments.clone();
        let mut second = second_segments.clone();
        first.sort_by_key(|segment| (segment.0, segment.1, segment.2));
        second.sort_by_key(|segment| (segment.0, segment.1, segment.2));
        let (mut first_index, mut second_index) = (0, 0);
        while first_index < first.len() && second_index < second.len() {
            let (first_start, first_end, first_key) = first[first_index];
            let (second_start, second_end, second_key) = second[second_index];
            let overlap_start = first_start.max(second_start);
            let overlap_end = first_end.min(second_end);
            if overlap_end > overlap_start {
                let first_width = 1_u32 << (max_refinement_level - first_key.level);
                let second_width = 1_u32 << (max_refinement_level - second_key.level);
                let center_distance = 0.5 * (first_width + second_width) as f64 * finest_spacing;
                let (first_key, second_key) = if first_key < second_key {
                    (first_key, second_key)
                } else {
                    (second_key, first_key)
                };
                let (start_vertex, end_vertex) = if vertical {
                    ((*coordinate, overlap_start), (*coordinate, overlap_end))
                } else {
                    ((overlap_start, *coordinate), (overlap_end, *coordinate))
                };
                output.push(SharedFace {
                    first: first_key,
                    second: second_key,
                    length: (overlap_end - overlap_start) as f64 * finest_spacing,
                    center_distance,
                    start_vertex,
                    end_vertex,
                });
            }
            if first_end <= second_end {
                first_index += 1;
            }
            if second_end <= first_end {
                second_index += 1;
            }
        }
    }
}

fn cells_share_face(a: CellBounds, b: CellBounds) -> bool {
    shared_face_geometry(a, b).is_some()
}

fn shared_face_geometry(a: CellBounds, b: CellBounds) -> Option<(f64, f64)> {
    let tolerance = 1.0e-12
        * a.max_x
            .abs()
            .max(a.max_y.abs())
            .max(b.max_x.abs())
            .max(b.max_y.abs())
            .max(1.0);
    let vertical_overlap = a.max_y.min(b.max_y) - a.min_y.max(b.min_y);
    let horizontal_overlap = a.max_x.min(b.max_x) - a.min_x.max(b.min_x);
    if ((a.max_x - b.min_x).abs() <= tolerance || (b.max_x - a.min_x).abs() <= tolerance)
        && vertical_overlap > tolerance
    {
        return Some((
            vertical_overlap,
            0.5 * (a.max_x - a.min_x + b.max_x - b.min_x),
        ));
    }
    if ((a.max_y - b.min_y).abs() <= tolerance || (b.max_y - a.min_y).abs() <= tolerance)
        && horizontal_overlap > tolerance
    {
        return Some((
            horizontal_overlap,
            0.5 * (a.max_y - a.min_y + b.max_y - b.min_y),
        ));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) {
        assert!(
            (a - b).abs() <= 1.0e-12 * a.abs().max(b.abs()).max(1.0),
            "{a} != {b}"
        );
    }

    #[test]
    fn refine_and_coarsen_preserve_area_integrals_of_both_time_levels() {
        let mut grid = AdaptiveSpatialGrid::new(3, 2, 2.0, 3).unwrap();
        let key = CellKey {
            level: 0,
            x: 1,
            y: 0,
        };
        grid.set_state(
            key,
            AdaptiveCell {
                previous: -0.75,
                current: 1.25,
            },
        )
        .unwrap();
        let before = grid.integrated_state();
        let children = grid.refine(key).unwrap();
        assert_eq!(children.len(), 4);
        assert_eq!(grid.leaf_count(), 9);
        let refined = grid.integrated_state();
        close(refined.current_displacement, before.current_displacement);
        close(refined.previous_displacement, before.previous_displacement);

        let restored = grid.coarsen(key).unwrap();
        assert_eq!(
            restored,
            AdaptiveCell {
                previous: -0.75,
                current: 1.25
            }
        );
        let after = grid.integrated_state();
        close(after.current_displacement, before.current_displacement);
        close(after.previous_displacement, before.previous_displacement);
        assert_eq!(grid.leaf_count(), 6);
    }

    #[test]
    fn shared_faces_partition_a_coarse_face_without_gaps_or_double_counting() {
        let mut grid = AdaptiveSpatialGrid::new(2, 1, 1.0, 2).unwrap();
        grid.refine(CellKey {
            level: 0,
            x: 0,
            y: 0,
        })
        .unwrap();
        let faces = grid.shared_faces();
        let coarse = CellKey {
            level: 0,
            x: 1,
            y: 0,
        };
        let coarse_side: f64 = faces
            .iter()
            .filter(|face| face.first == coarse || face.second == coarse)
            .filter(|face| {
                let other = if face.first == coarse {
                    face.second
                } else {
                    face.first
                };
                other.level == 1
            })
            .map(|face| face.length)
            .sum();
        close(coarse_side, 1.0);
        for face in faces
            .iter()
            .filter(|face| face.first == coarse || face.second == coarse)
        {
            close(face.center_distance, 0.75);
        }
        assert!(faces
            .iter()
            .all(|face| face.length > 0.0 && face.center_distance > 0.0));
    }

    #[test]
    fn refinement_enforces_two_to_one_neighbor_balance() {
        let mut grid = AdaptiveSpatialGrid::new(2, 1, 1.0, 3).unwrap();
        grid.refine(CellKey {
            level: 0,
            x: 0,
            y: 0,
        })
        .unwrap();
        let child = CellKey {
            level: 1,
            x: 1,
            y: 0,
        };
        assert_eq!(
            grid.refine(child),
            Err(AdaptiveGridError::UnbalancedRefinement)
        );
    }

    #[test]
    fn physical_position_queries_resolve_the_finest_containing_leaf() {
        let mut grid = AdaptiveSpatialGrid::new(2, 2, 1.0, 2).unwrap();
        let key = CellKey {
            level: 0,
            x: 0,
            y: 0,
        };
        grid.set_state(
            key,
            AdaptiveCell {
                previous: 1.0,
                current: 2.0,
            },
        )
        .unwrap();
        grid.refine(key).unwrap();
        let child = CellKey {
            level: 1,
            x: 1,
            y: 0,
        };
        grid.set_state(
            child,
            AdaptiveCell {
                previous: 3.0,
                current: 4.0,
            },
        )
        .unwrap();
        assert_eq!(
            grid.sample(0.75, 0.25),
            Some(AdaptiveCell {
                previous: 3.0,
                current: 4.0
            })
        );
        assert_eq!(grid.sample(-0.1, 0.25), None);
        assert_eq!(grid.sample(2.0, 0.25), None);
        assert_eq!(grid.sample(f64::NAN, 0.25), None);
    }

    #[test]
    fn coarsening_rejects_a_parent_that_would_break_two_to_one_balance() {
        let mut grid = AdaptiveSpatialGrid::new(2, 1, 1.0, 3).unwrap();
        let left_parent = CellKey {
            level: 0,
            x: 0,
            y: 0,
        };
        let right_parent = CellKey {
            level: 0,
            x: 1,
            y: 0,
        };
        grid.refine(left_parent).unwrap();
        grid.refine(right_parent).unwrap();
        // The right parent's leftmost child may refine to level 2 beside
        // level-1 leaves. Coarsening the left parent would then put a level-0
        // cell beside a level-2 leaf, violating 2:1 balance.
        let interface_child = CellKey {
            level: 1,
            x: 2,
            y: 0,
        };
        grid.refine(interface_child).unwrap();
        assert_eq!(
            grid.coarsen(left_parent),
            Err(AdaptiveGridError::UnbalancedRefinement)
        );
    }
    fn wave_field(refine_center: bool) -> AdaptiveWaveField {
        let mut grid = AdaptiveSpatialGrid::new(7, 7, 1.0, 2).unwrap();
        for y in 1..6 {
            for x in 1..6 {
                let dx = x as f64 - 3.0;
                let dy = y as f64 - 3.0;
                let profile = (-(dx * dx + dy * dy) / 3.0).exp();
                grid.set_state(
                    CellKey { level: 0, x, y },
                    AdaptiveCell {
                        previous: 0.03 * profile,
                        current: 0.1 * profile,
                    },
                )
                .unwrap();
            }
        }
        if refine_center {
            grid.refine(CellKey {
                level: 0,
                x: 3,
                y: 3,
            })
            .unwrap();
        }
        AdaptiveWaveField::new(grid, 0.2, 1.3).unwrap()
    }

    #[test]
    fn field_regridding_preserves_energy_and_both_area_integrals() {
        let mut field = wave_field(false);
        let initial_energy = field.energy();
        let initial_integrals = field.grid().integrated_state();

        field
            .refine_grid(CellKey {
                level: 0,
                x: 3,
                y: 3,
            })
            .unwrap();
        close(field.energy(), initial_energy);
        let refined_integrals = field.grid().integrated_state();
        close(
            refined_integrals.current_displacement,
            initial_integrals.current_displacement,
        );
        close(
            refined_integrals.previous_displacement,
            initial_integrals.previous_displacement,
        );

        field.advance_ticks(12).unwrap();
        let before_coarsen_energy = field.energy();
        let before_coarsen_integrals = field.grid().integrated_state();
        field
            .coarsen_grid(CellKey {
                level: 0,
                x: 3,
                y: 3,
            })
            .unwrap();
        close(field.energy(), before_coarsen_energy);
        let coarsened_integrals = field.grid().integrated_state();
        close(
            coarsened_integrals.current_displacement,
            before_coarsen_integrals.current_displacement,
        );
        close(
            coarsened_integrals.previous_displacement,
            before_coarsen_integrals.previous_displacement,
        );

        field.advance_ticks(12).unwrap();
        close(field.energy(), before_coarsen_energy);
    }

    #[test]
    fn coarsening_rejects_energy_that_has_no_remaining_field_degrees_of_freedom() {
        let mut grid = AdaptiveSpatialGrid::new(4, 4, 1.0, 2).unwrap();
        let parent = CellKey {
            level: 0,
            x: 1,
            y: 1,
        };
        let children = grid.refine(parent).unwrap();
        for (index, child) in children.into_iter().enumerate() {
            let sign = if index % 2 == 0 { 1.0 } else { -1.0 };
            grid.set_state(
                child,
                AdaptiveCell {
                    previous: -0.5 * sign,
                    current: sign,
                },
            )
            .unwrap();
        }
        let mut field = AdaptiveWaveField::new(grid, 0.2, 1.0).unwrap();
        let energy_before = field.energy();
        let grid_before = field.grid().clone();
        assert_eq!(
            field.coarsen_grid(parent),
            Err(AdaptiveGridError::UnrepresentableEnergyTransfer)
        );
        close(field.energy(), energy_before);
        assert_eq!(field.grid().leaf_count(), grid_before.leaf_count());
        assert_eq!(
            field.grid().integrated_state(),
            grid_before.integrated_state()
        );
    }

    #[test]
    fn adaptive_leapfrog_preserves_its_discrete_energy_across_coarse_fine_faces() {
        let mut field = wave_field(true);
        let initial_energy = field.energy();
        assert!(initial_energy > 0.0);
        field.advance_ticks(40).unwrap();
        let final_energy = field.energy();
        close(final_energy, initial_energy);
        assert_eq!(field.logical_time_ticks(), 40.0);
    }

    #[test]
    fn adaptive_leapfrog_replay_is_deterministic() {
        let mut first = wave_field(true);
        let mut second = first.clone();
        first.advance_ticks(25).unwrap();
        second.advance_ticks(25).unwrap();
        close(first.energy(), second.energy());
        for y in [1.25, 3.25, 5.25] {
            for x in [1.25, 3.25, 5.25] {
                assert_eq!(first.sample(x, y), second.sample(x, y));
            }
        }
    }

    #[test]
    fn adaptive_sponge_loss_is_reconciled_as_computational_outflow() {
        let mut grid = AdaptiveSpatialGrid::new(7, 7, 1.0, 1).unwrap();
        grid.set_state(
            CellKey {
                level: 0,
                x: 1,
                y: 3,
            },
            AdaptiveCell {
                previous: 0.0,
                current: 1.0,
            },
        )
        .unwrap();
        let mut field = AdaptiveWaveField::new(grid, 0.2, 1.0).unwrap();
        field.configure_absorbing_boundary(2.0, 1.0).unwrap();
        let initial_energy = field.energy();
        field.advance_ticks(1).unwrap();
        let final_energy = field.energy();
        assert!(field.absorbed_energy() > 0.0);
        close(initial_energy - final_energy, field.absorbed_energy());
        assert_eq!(
            field.configure_absorbing_boundary(1.0, 1.0),
            Err(AdaptiveGridError::AlreadyStarted)
        );
    }

    #[test]
    fn grid_rejects_underflowing_finest_cell_area() {
        assert_eq!(
            AdaptiveSpatialGrid::new(1, 1, 1.0e-200, 20).err(),
            Some(AdaptiveGridError::InvalidParameter)
        );
    }

    #[test]
    fn adaptive_wave_field_rejects_nonzero_computational_boundary_state() {
        let mut grid = AdaptiveSpatialGrid::new(3, 3, 1.0, 1).unwrap();
        grid.set_state(
            CellKey {
                level: 0,
                x: 0,
                y: 1,
            },
            AdaptiveCell {
                previous: 0.0,
                current: 1.0,
            },
        )
        .unwrap();
        assert_eq!(
            AdaptiveWaveField::new(grid, 1.0, 1.0).err(),
            Some(AdaptiveGridError::InvalidState)
        );
    }

    #[test]
    fn repeated_regridding_preserves_energy_and_integrals_across_wave_profiles() {
        for profile_kind in 0..4 {
            let mut grid = AdaptiveSpatialGrid::new(9, 9, 1.0, 2).unwrap();
            for y in 1..8 {
                for x in 1..8 {
                    let dx = x as f64 - 4.0;
                    let dy = y as f64 - 4.0;
                    let radius_squared = dx * dx + dy * dy;
                    let envelope = (-radius_squared / 18.0).exp();
                    let profile = match profile_kind {
                        0 => envelope,
                        1 => (-(dx * dx / 8.0 + dy * dy / 2.0)).exp(),
                        2 => (-((radius_squared.sqrt() - 2.5).powi(2)) / 1.5).exp(),
                        _ => (0.7 * dx + 0.35 * dy).sin() * envelope,
                    };
                    let previous = 0.37 * profile + 0.015 * envelope;
                    let current = profile - 0.02 * envelope;
                    grid.set_state(
                        CellKey { level: 0, x, y },
                        AdaptiveCell { previous, current },
                    )
                    .unwrap();
                }
            }

            let mut field = AdaptiveWaveField::new(grid, 0.15, 1.2).unwrap();
            let parent = CellKey {
                level: 0,
                x: 4,
                y: 4,
            };

            for _ in 0..6 {
                field.advance_ticks(2).unwrap();
                let before_refine_energy = field.energy();
                let before_refine_integrals = field.grid().integrated_state();

                field.refine_grid(parent).unwrap();
                close(field.energy(), before_refine_energy);
                let after_refine_integrals = field.grid().integrated_state();
                close(
                    after_refine_integrals.current_displacement,
                    before_refine_integrals.current_displacement,
                );
                close(
                    after_refine_integrals.previous_displacement,
                    before_refine_integrals.previous_displacement,
                );

                field.advance_ticks(2).unwrap();
                let before_coarsen_energy = field.energy();
                let before_coarsen_integrals = field.grid().integrated_state();

                field.coarsen_grid(parent).unwrap();
                close(field.energy(), before_coarsen_energy);
                let after_coarsen_integrals = field.grid().integrated_state();
                close(
                    after_coarsen_integrals.current_displacement,
                    before_coarsen_integrals.current_displacement,
                );
                close(
                    after_coarsen_integrals.previous_displacement,
                    before_coarsen_integrals.previous_displacement,
                );
            }
        }
    }

    #[test]
    fn uniformly_refined_adaptive_wave_converges_to_continuum_mode() {
        fn relative_error(refinement_level: u8) -> f64 {
            let cells = 17_u32;
            let spacing = 1.0 / 2_u32.pow(refinement_level as u32) as f64;
            // The solver fixes the outermost cell centers to zero. Their
            // physical locations move with refinement, so the analytical
            // standing-wave domain must move with them as well.
            let length = cells as f64 - spacing;
            let speed = 0.2_f64;
            let mut grid = AdaptiveSpatialGrid::new(cells, cells, 1.0, 2).unwrap();

            if refinement_level >= 1 {
                for y in 0..cells {
                    for x in 0..cells {
                        grid.refine(CellKey { level: 0, x, y }).unwrap();
                    }
                }
            }
            if refinement_level >= 2 {
                let level_one: Vec<_> = grid.leaves.keys().copied().collect();
                for key in level_one {
                    grid.refine(key).unwrap();
                }
            }

            let substeps = (0.4 / spacing).ceil() as usize;
            let dt = 1.0 / substeps as f64;
            let omega = speed * std::f64::consts::PI * std::f64::consts::SQRT_2 / length;
            let mut initial_norm = 0.0;
            for key in grid.leaves.keys().copied().collect::<Vec<_>>() {
                let bounds = grid.cell_bounds(key).unwrap();
                let center_x = 0.5 * (bounds.min_x + bounds.max_x);
                let center_y = 0.5 * (bounds.min_y + bounds.max_y);
                let mode = (std::f64::consts::PI * (center_x - 0.5 * spacing) / length).sin()
                    * (std::f64::consts::PI * (center_y - 0.5 * spacing) / length).sin();
                let boundary = is_computational_boundary(&grid, key);
                let value = if boundary { 0.0 } else { mode };
                initial_norm += grid.cell_area(key) * value.powi(2);
                grid.set_state(
                    key,
                    AdaptiveCell {
                        previous: value * (omega * dt).cos(),
                        current: value,
                    },
                )
                .unwrap();
            }

            let mut field = AdaptiveWaveField::new(grid, speed, 1.0).unwrap();
            assert_eq!(field.substeps_per_tick(), substeps);
            field.advance_ticks(4).unwrap();
            let time = field.logical_time_ticks();
            let mut error_squared = 0.0;
            for (key, state) in field.grid().leaves() {
                let bounds = field.grid().cell_bounds(key).unwrap();
                let center_x = 0.5 * (bounds.min_x + bounds.max_x);
                let center_y = 0.5 * (bounds.min_y + bounds.max_y);
                let mode = (std::f64::consts::PI * (center_x - 0.5 * spacing) / length).sin()
                    * (std::f64::consts::PI * (center_y - 0.5 * spacing) / length).sin();
                let exact = mode * (omega * time).cos();
                error_squared += field.grid().cell_area(key) * (state.current - exact).powi(2);
            }
            error_squared.sqrt() / initial_norm.sqrt()
        }

        let coarse = relative_error(0);
        let medium = relative_error(1);
        let fine = relative_error(2);
        assert!(
            medium < coarse,
            "first spatial refinement did not reduce continuum error: coarse={coarse}, medium={medium}"
        );
        assert!(
            fine < medium,
            "second spatial refinement did not reduce continuum error: medium={medium}, fine={fine}"
        );
    }

    #[test]
    fn coarse_fine_interfaces_preserve_a_linear_field_patch() {
        let width = 20_u32;
        let height = 20_u32;
        let mut grid = AdaptiveSpatialGrid::new(width, height, 1.0, 2).unwrap();

        // A level-one patch surrounded by coarse cells, with a level-two
        // interior. This exercises both coarse/fine and fine/finer interfaces.
        for y in 6..14 {
            for x in 6..14 {
                grid.refine(CellKey { level: 0, x, y }).unwrap();
            }
        }
        let inner_parents: Vec<_> = (8..12)
            .flat_map(|y| (8..12).map(move |x| CellKey { level: 0, x, y }))
            .collect();
        for parent in inner_parents {
            let children = [
                CellKey {
                    level: 1,
                    x: parent.x * 2,
                    y: parent.y * 2,
                },
                CellKey {
                    level: 1,
                    x: parent.x * 2 + 1,
                    y: parent.y * 2,
                },
                CellKey {
                    level: 1,
                    x: parent.x * 2,
                    y: parent.y * 2 + 1,
                },
                CellKey {
                    level: 1,
                    x: parent.x * 2 + 1,
                    y: parent.y * 2 + 1,
                },
            ];
            for child in children {
                grid.refine(child).unwrap();
            }
        }

        for key in grid.leaves.keys().copied().collect::<Vec<_>>() {
            let bounds = grid.cell_bounds(key).unwrap();
            let center_x = 0.5 * (bounds.min_x + bounds.max_x);
            let center_y = 0.5 * (bounds.min_y + bounds.max_y);
            let value = if is_computational_boundary(&grid, key) {
                0.0
            } else {
                center_x + 2.0 * center_y
            };
            grid.set_state(
                key,
                AdaptiveCell {
                    previous: value,
                    current: value,
                },
            )
            .unwrap();
        }

        let mut field = AdaptiveWaveField::new(grid, 0.05, 1.0).unwrap();
        let initial: Vec<_> = field
            .grid()
            .leaves()
            .filter_map(|(key, state)| {
                let bounds = field.grid().cell_bounds(key)?;
                let center_x = 0.5 * (bounds.min_x + bounds.max_x);
                let center_y = 0.5 * (bounds.min_y + bounds.max_y);
                (center_x > 2.0
                    && center_x < width as f64 - 2.0
                    && center_y > 2.0
                    && center_y < height as f64 - 2.0)
                    .then_some((key, state.current))
            })
            .collect();

        field.advance_substeps(1).unwrap();
        for (key, expected) in initial {
            let actual = field.grid().leaves.get(&key).unwrap().current;
            let scale = expected.abs().max(1.0);
            assert!(
                (actual - expected).abs() <= 1.0e-11 * scale,
                "linear patch changed at {key:?}: expected={expected}, actual={actual}"
            );
        }
    }

    #[test]
    fn wave_energy_remains_conserved_during_propagation_across_refined_interfaces() {
        let width = 40_u32;
        let height = 24_u32;
        let mut grid = AdaptiveSpatialGrid::new(width, height, 1.0, 1).unwrap();

        // A full-height refined strip creates two vertical coarse/fine
        // interfaces. The packet starts on the coarse side and crosses them.
        for y in 0..height {
            for x in 16..24 {
                grid.refine(CellKey { level: 0, x, y }).unwrap();
            }
        }

        for key in grid.leaves.keys().copied().collect::<Vec<_>>() {
            let bounds = grid.cell_bounds(key).unwrap();
            let center_x = 0.5 * (bounds.min_x + bounds.max_x);
            let center_y = 0.5 * (bounds.min_y + bounds.max_y);
            let boundary = is_computational_boundary(&grid, key);
            let envelope =
                (-((center_x - 11.0).powi(2) / 24.0 + (center_y - 12.0).powi(2) / 18.0)).exp();
            let phase = 0.55 * (center_x - 11.0);
            let current = if boundary {
                0.0
            } else {
                envelope * phase.cos()
            };
            let previous = if boundary {
                0.0
            } else {
                envelope * (phase + 0.55 * 0.15).cos()
            };
            grid.set_state(key, AdaptiveCell { previous, current })
                .unwrap();
        }

        let mut field = AdaptiveWaveField::new(grid, 0.15, 1.0).unwrap();
        let initial_energy = field.energy();
        field.advance_ticks(200).unwrap();
        let final_energy = field.energy();
        let relative_drift =
            (final_energy - initial_energy).abs() / initial_energy.abs().max(1.0e-15);
        assert!(
            relative_drift < 1.0e-9,
            "relative energy drift after crossing coarse/fine interfaces: {relative_drift}"
        );
    }
}
