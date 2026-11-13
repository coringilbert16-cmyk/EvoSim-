//! A physically realized material instance carried through acquisition/storage.
//!
//! `Material` owns composition and the identity of pre-existing internal bonds.
//! `PhysicalMaterial` owns the complete realized arrangement needed to restore
//! those bonds without inventing connection endpoints at the organism boundary.
use crate::contact::connection_pair_candidates;
use crate::resources::{BaseResource, Material};
use crate::structure::{ConnectionEndpoint, OrganismStructure, Placement, StructuralUnit};
use serde::{Deserialize, Serialize};

// Allows only accumulated floating-point noise at the unit-length bond boundary.
const INTERNAL_BOND_DISTANCE_EPSILON: f64 = 1e-12;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct PhysicalMaterialBond {
    pub(crate) part_a: usize,
    pub(crate) endpoint_a: ConnectionEndpoint,
    pub(crate) part_b: usize,
    pub(crate) endpoint_b: ConnectionEndpoint,
    /// Positive dissociation-energy magnitude allocated when this bond forms.
    /// Legacy bonds deserialize to zero because their past energy allocation is unknown.
    #[serde(default)]
    pub(crate) bond_energy: f64,
}

fn connection_endpoint_is_finite(endpoint: ConnectionEndpoint) -> bool {
    match endpoint {
        ConnectionEndpoint::Corner { .. } | ConnectionEndpoint::LineEndpoint { .. } => true,
        ConnectionEndpoint::Boundary { angle_radians } => angle_radians.is_finite(),
        ConnectionEndpoint::Fluid { x, y } => x.is_finite() && y.is_finite(),
    }
}

/// Approximate planar intrinsic inertia using the same realized-shape
/// convention used by environmental transport. The result is independent of
/// the flow mobility/response model and is suitable for Newtonian rotation.
pub(crate) fn intrinsic_moment_of_inertia(form: &crate::resources::Form, mass: f64) -> f64 {
    use crate::resources::Form;
    match form {
        Form::Circle { radius } => 0.5 * mass * radius * radius,
        Form::Line { length } => mass * length * length / 12.0,
        Form::Rectangle { width, height } => mass * (width * width + height * height) / 12.0,
        Form::RegularPolygon { radius, .. } => 0.5 * mass * radius * radius,
        Form::Polygon { vertices } => {
            let mean_radius_sq = vertices.iter().map(|(x, y)| x * x + y * y).sum::<f64>()
                / vertices.len().max(1) as f64;
            0.5 * mass * mean_radius_sq
        }
        Form::Fluid { nominal_area, .. } => {
            let radius_sq = nominal_area.max(0.0) / std::f64::consts::PI;
            0.5 * mass * radius_sq
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RigidBodyMassProperties {
    pub(crate) mass: f64,
    pub(crate) center_x: f64,
    pub(crate) center_y: f64,
    pub(crate) moment_of_inertia: f64,
}

/// Persistent Newtonian state for one realized elemental or molecular body.
/// Linear velocity is the center-of-mass velocity in world axes; angular
/// velocity is radians per physics-time unit. Environmental flow does not
/// modify this state. Legacy serialized materials begin at rest.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct InertialMotion {
    pub(crate) velocity_x: f64,
    pub(crate) velocity_y: f64,
    pub(crate) angular_velocity: f64,
}

impl InertialMotion {
    pub(crate) fn is_finite(self) -> bool {
        self.velocity_x.is_finite()
            && self.velocity_y.is_finite()
            && self.angular_velocity.is_finite()
    }

    /// Apply a constant net force and torque over an explicitly supplied
    /// physics-time interval. No drag or damping is applied. The caller owns
    /// the timestep and the authoritative mass/inertia calculation.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "Newtonian runtime integration follows in this PR")
    )]
    pub(crate) fn advanced(
        self,
        force_x: f64,
        force_y: f64,
        torque: f64,
        mass: f64,
        moment_of_inertia: f64,
        delta_time: f64,
    ) -> Option<Self> {
        if !self.is_finite()
            || ![
                force_x,
                force_y,
                torque,
                mass,
                moment_of_inertia,
                delta_time,
            ]
            .iter()
            .all(|value| value.is_finite())
            || mass <= 0.0
            || moment_of_inertia <= 0.0
            || delta_time < 0.0
        {
            return None;
        }
        let next = Self {
            velocity_x: self.velocity_x + force_x / mass * delta_time,
            velocity_y: self.velocity_y + force_y / mass * delta_time,
            angular_velocity: self.angular_velocity + torque / moment_of_inertia * delta_time,
        };
        next.is_finite().then_some(next)
    }

    /// Kinetic energy for the supplied authoritative mass and inertia.
    pub(crate) fn kinetic_energy(self, mass: f64, moment_of_inertia: f64) -> Option<f64> {
        if !self.is_finite()
            || !mass.is_finite()
            || !moment_of_inertia.is_finite()
            || mass <= 0.0
            || moment_of_inertia < 0.0
        {
            return None;
        }
        let energy = 0.5
            * (mass
                * self
                    .velocity_x
                    .mul_add(self.velocity_x, self.velocity_y * self.velocity_y)
                + moment_of_inertia * self.angular_velocity * self.angular_velocity);
        (energy.is_finite() && energy >= 0.0).then_some(energy)
    }

    /// Velocity of a point offset from the center of mass.
    pub(crate) fn velocity_at_offset(self, offset_x: f64, offset_y: f64) -> Option<(f64, f64)> {
        if !self.is_finite() || !offset_x.is_finite() || !offset_y.is_finite() {
            return None;
        }
        Some((
            self.velocity_x - self.angular_velocity * offset_y,
            self.velocity_y + self.angular_velocity * offset_x,
        ))
    }

    /// Express the same physical velocity in axes rotated by the supplied angle.
    pub(crate) fn rotated_axes(self, angle: f64) -> Option<Self> {
        if !self.is_finite() || !angle.is_finite() {
            return None;
        }
        let (sin, cos) = angle.sin_cos();
        Some(Self {
            velocity_x: self.velocity_x * cos - self.velocity_y * sin,
            velocity_y: self.velocity_x * sin + self.velocity_y * cos,
            angular_velocity: self.angular_velocity,
        })
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct PhysicalMaterial {
    pub(crate) material: Material,
    /// Persistent rigid-body momentum state; unaffected by ambient flow.
    #[serde(default)]
    pub(crate) inertial_motion: InertialMotion,
    /// Stable identity for environmental chemistry; preserved by transport.
    /// Zero means not yet assigned to an active field instance.
    #[serde(default)]
    pub(crate) environmental_identity: u64,
    pub(crate) placements: Option<Vec<Placement>>,
    #[serde(default)]
    pub(crate) internal_connections: Option<Vec<PhysicalMaterialBond>>,
    #[serde(default)]
    pub(crate) owner_relative_origin: Option<Placement>,
    /// Optional constituent-local shapes for physically realized deformable material.
    /// Legacy stored materials omit this field and continue to use catalog shapes.
    #[serde(default)]
    pub(crate) realized_shapes: Option<Vec<crate::resources::Shape>>,
}

impl PhysicalMaterial {
    pub(crate) fn realized(
        material: Material,
        placements: Vec<Placement>,
        catalog: &[BaseResource],
    ) -> Option<Self> {
        Self::realized_inner(material, placements, catalog, false)
    }

    /// Realize one fluid constituent with its authoritative physical shape.
    /// This is required for aggregated fluid counts and also preserves a
    /// single fluid's deformed shape when it enters storage.
    pub(crate) fn realized_with_single_fluid_shape(
        material: Material,
        placement: Placement,
        shape: crate::resources::Shape,
        catalog: &[BaseResource],
    ) -> Option<Self> {
        let mut instance = Self::realized_inner(material, vec![placement], catalog, true)?;
        instance = instance.with_single_fluid_shape(shape, catalog)?;
        Some(instance)
    }

    fn realized_inner(
        material: Material,
        placements: Vec<Placement>,
        catalog: &[BaseResource],
        allow_aggregate_fluid: bool,
    ) -> Option<Self> {
        if !material.is_valid_for_catalog(catalog)
            || material.parts.is_empty()
            || placements.len() != material.parts.len()
        {
            return None;
        }
        if placements.iter().any(|placement| {
            !placement.x.is_finite()
                || !placement.y.is_finite()
                || !placement.rotation_radians.is_finite()
        }) {
            return None;
        }

        let mut structure = OrganismStructure::new();
        for ((name, amount), placement) in material.parts.iter().zip(placements.iter()) {
            if (*amount - 1.0).abs() > 1e-9 {
                let resource = catalog.iter().find(|resource| resource.name == *name)?;
                if material.parts.len() != 1
                    || material.has_internal_structure()
                    || resource.physical_state != crate::resources::PhysicalState::Fluid
                    || !allow_aggregate_fluid
                    || *amount < 1.0
                    || (*amount - amount.round()).abs() > 1e-9
                {
                    return None;
                }
            }
            let mut unit =
                StructuralUnit::from_material(Material::free_base(name.clone(), 1.0), *placement)?;
            if !unit.realize_default_geometry(catalog) {
                return None;
            }
            structure.add_unit(unit);
        }

        let mut internal_connections = Vec::with_capacity(material.internal_bonds.len());
        for bond in &material.internal_bonds {
            // `Material::InternalBond` records which constituents are bonded,
            // while the physical endpoints are derived from their realization.
            // More than one endpoint pairing can be geometrically valid for a
            // symmetric realization. Preserve one deterministic physical choice
            // rather than rejecting an otherwise valid material.
            let candidate =
                connection_pair_candidates(&structure, bond.part_a, bond.part_b, catalog)
                    .into_iter()
                    .find(|candidate| {
                        candidate.available_a
                            && candidate.available_b
                            && candidate.distance <= 1.0 + INTERNAL_BOND_DISTANCE_EPSILON
                    })?;
            internal_connections.push(PhysicalMaterialBond {
                part_a: bond.part_a,
                endpoint_a: candidate.endpoint_a,
                part_b: bond.part_b,
                endpoint_b: candidate.endpoint_b,
                // Composition-only historical bonds have no authoritative stored energy.
                bond_energy: 0.0,
            });
        }

        Some(Self {
            material,
            inertial_motion: InertialMotion::default(),
            environmental_identity: 0,
            placements: Some(placements),
            internal_connections: Some(internal_connections),
            owner_relative_origin: None,
            realized_shapes: None,
        })
    }

    /// Attach an exact fluid shape to a single-part acquired fluid material.
    ///
    /// This first bridge is intentionally limited to one free fluid constituent:
    /// bonded composite endpoint geometry must be recomputed transactionally before
    /// arbitrary constituent shape overrides can be supported.
    pub(crate) fn with_single_fluid_shape(
        mut self,
        shape: crate::resources::Shape,
        catalog: &[BaseResource],
    ) -> Option<Self> {
        if self.material.parts.len() != 1
            || !self.material.internal_bonds.is_empty()
            || !shape.is_valid()
        {
            return None;
        }
        let (name, amount) = self.material.parts.first()?;
        if !amount.is_finite() || *amount < 1.0 || (*amount - amount.round()).abs() > 1e-9 {
            return None;
        }
        let resource = catalog.iter().find(|resource| resource.name == *name)?;
        if resource.physical_state != crate::resources::PhysicalState::Fluid {
            return None;
        }
        let default_area = match &resource.shape.form {
            crate::resources::Form::Circle { radius } => std::f64::consts::PI * radius * radius,
            crate::resources::Form::Fluid { nominal_area, .. } => *nominal_area,
            _ => return None,
        };
        let shape_area = match &shape.form {
            crate::resources::Form::Fluid {
                nominal_area,
                boundary: None,
                arc_boundary: Some(_),
                ..
            }
            | crate::resources::Form::Fluid {
                nominal_area,
                multi_contact_boundary: Some(_),
                ..
            } => *nominal_area,
            _ => return None,
        };
        let expected_area = default_area * *amount;
        if !expected_area.is_finite()
            || (shape_area - expected_area).abs() > 1e-9 * expected_area.max(1.0)
        {
            return None;
        }
        self.realized_shapes = Some(vec![shape]);
        Some(self)
    }

    /// Resolve the stored physical shape for one constituent, validating any
    /// override against the catalog's material state and conserved fluid area.
    /// Derive rigid-body mass, center of mass, and planar moment of inertia
    /// from constituent masses and their authoritative realized geometry.
    /// A vertical period is supplied only for worlds with periodic Y geometry.
    /// Propose one Newtonian rigid-body step under a constant net force and
    /// torque. Translation and rotation use midpoint velocity, so a constant
    /// acceleration advances the pose consistently with the updated momentum.
    /// This is a pure staged proposal: callers must validate collision/path
    /// constraints and settle the before/after kinetic and pair-potential energy
    /// changes before committing the returned material.
    pub(crate) fn advanced_under_force(
        &self,
        force_x: f64,
        force_y: f64,
        torque: f64,
        delta_time: f64,
        catalog: &[BaseResource],
        vertical_period: Option<f64>,
    ) -> Option<Self> {
        if !self.is_realized()
            || !delta_time.is_finite()
            || delta_time < 0.0
            || vertical_period.is_some_and(|period| !period.is_finite() || period <= 0.0)
        {
            return None;
        }
        let mass_properties = self.mass_properties(catalog, vertical_period)?;
        let next_motion = self.inertial_motion.advanced(
            force_x,
            force_y,
            torque,
            mass_properties.mass,
            mass_properties.moment_of_inertia,
            delta_time,
        )?;
        let dx = 0.5 * (self.inertial_motion.velocity_x + next_motion.velocity_x) * delta_time;
        let dy = 0.5 * (self.inertial_motion.velocity_y + next_motion.velocity_y) * delta_time;
        let angle = 0.5
            * (self.inertial_motion.angular_velocity + next_motion.angular_velocity)
            * delta_time;
        if ![dx, dy, angle].iter().all(|value| value.is_finite()) {
            return None;
        }

        let (sin, cos) = angle.sin_cos();
        let mut advanced = self.clone();
        let placements = advanced.placements.as_mut()?;
        for placement in placements {
            let offset_x = placement.x - mass_properties.center_x;
            let raw_offset_y = placement.y - mass_properties.center_y;
            let offset_y = vertical_period.map_or(raw_offset_y, |period| {
                (raw_offset_y + period * 0.5).rem_euclid(period) - period * 0.5
            });
            let rotated_x = offset_x * cos - offset_y * sin;
            let rotated_y = offset_x * sin + offset_y * cos;
            placement.x = mass_properties.center_x + dx + rotated_x;
            let next_y = mass_properties.center_y + dy + rotated_y;
            placement.y = vertical_period.map_or(next_y, |period| next_y.rem_euclid(period));
            placement.rotation_radians =
                (placement.rotation_radians + angle).rem_euclid(std::f64::consts::TAU);
            if !placement.x.is_finite()
                || !placement.y.is_finite()
                || !placement.rotation_radians.is_finite()
            {
                return None;
            }
        }
        advanced.inertial_motion = next_motion;
        advanced.is_realized().then_some(advanced)
    }

    pub(crate) fn mass_properties(
        &self,
        catalog: &[BaseResource],
        vertical_period: Option<f64>,
    ) -> Option<RigidBodyMassProperties> {
        if !self.is_realized()
            || !self.material.is_valid_for_catalog(catalog)
            || vertical_period.is_some_and(|period| !period.is_finite() || period <= 0.0)
        {
            return None;
        }
        let placements = self.placements.as_ref()?;
        if placements.len() != self.material.parts.len() {
            return None;
        }
        let origin = *placements.first()?;
        let mut constituents = Vec::with_capacity(placements.len());
        let mut total_mass = 0.0;
        let mut weighted_x = 0.0;
        let mut weighted_y = 0.0;
        for (index, ((name, amount), placement)) in
            self.material.parts.iter().zip(placements).enumerate()
        {
            let resource = catalog.iter().find(|resource| resource.name == *name)?;
            let part_mass = resource.properties.mass * *amount;
            if !part_mass.is_finite() || part_mass <= 0.0 {
                return None;
            }
            let dx = placement.x - origin.x;
            let raw_dy = placement.y - origin.y;
            let dy = vertical_period.map_or(raw_dy, |period| {
                (raw_dy + period * 0.5).rem_euclid(period) - period * 0.5
            });
            let shape = self.realized_shape_for_part(index, catalog)?;
            total_mass += part_mass;
            weighted_x += dx * part_mass;
            weighted_y += dy * part_mass;
            constituents.push((part_mass, dx, dy, shape));
        }
        if !total_mass.is_finite() || total_mass <= 0.0 {
            return None;
        }
        let center_offset_x = weighted_x / total_mass;
        let center_offset_y = weighted_y / total_mass;
        let mut moment_of_inertia = 0.0;
        for (part_mass, dx, dy, shape) in constituents {
            let rx = dx - center_offset_x;
            let ry = dy - center_offset_y;
            moment_of_inertia += part_mass * rx.mul_add(rx, ry * ry)
                + intrinsic_moment_of_inertia(&shape.form, part_mass);
        }
        let center_y = vertical_period.map_or(origin.y + center_offset_y, |period| {
            (origin.y + center_offset_y).rem_euclid(period)
        });
        let properties = RigidBodyMassProperties {
            mass: total_mass,
            center_x: origin.x + center_offset_x,
            center_y,
            moment_of_inertia,
        };
        (properties.mass.is_finite()
            && properties.center_x.is_finite()
            && properties.center_y.is_finite()
            && properties.moment_of_inertia.is_finite()
            && properties.moment_of_inertia > 0.0)
            .then_some(properties)
    }

    /// Translational plus rotational kinetic energy from authoritative
    /// realized mass properties.
    pub(crate) fn kinetic_energy(
        &self,
        catalog: &[BaseResource],
        vertical_period: Option<f64>,
    ) -> Option<f64> {
        let properties = self.mass_properties(catalog, vertical_period)?;
        self.inertial_motion
            .kinetic_energy(properties.mass, properties.moment_of_inertia)
    }

    pub(crate) fn realized_shape_for_part(
        &self,
        part_index: usize,
        catalog: &[BaseResource],
    ) -> Option<crate::resources::Shape> {
        if !self.material.is_valid_for_catalog(catalog) {
            return None;
        }
        let (name, amount) = self.material.parts.get(part_index)?;
        let resource = catalog.iter().find(|resource| resource.name == *name)?;
        let shape = match self.realized_shapes.as_ref() {
            Some(shapes) if shapes.len() == self.material.parts.len() => shapes.get(part_index)?,
            Some(_) => return None,
            None => &resource.shape,
        };
        if !shape.is_valid() {
            return None;
        }
        if shape != &resource.shape
            && resource.physical_state != crate::resources::PhysicalState::Fluid
        {
            return None;
        }
        if resource.physical_state == crate::resources::PhysicalState::Fluid {
            let default_area = match &resource.shape.form {
                crate::resources::Form::Circle { radius } => std::f64::consts::PI * radius * radius,
                crate::resources::Form::Fluid { nominal_area, .. } => *nominal_area,
                _ => return None,
            };
            let shape_area = match &shape.form {
                crate::resources::Form::Circle { radius } => std::f64::consts::PI * radius * radius,
                crate::resources::Form::Fluid { nominal_area, .. } => *nominal_area,
                _ => return None,
            };
            let expected_area = default_area * *amount;
            if !expected_area.is_finite()
                || (shape_area - expected_area).abs() > 1e-9 * expected_area.max(1.0)
            {
                return None;
            }
        }
        Some(shape.clone())
    }

    /// Break one pre-existing internal bond and return the resulting physically
    /// disconnected pieces. The bond itself must belong to this stored material.
    pub(crate) fn break_internal_bond(&self, target: &PhysicalMaterialBond) -> Option<Vec<Self>> {
        // Persisted active transformations can reach completion without passing
        // through the ordinary source-selection gate. Revalidate the whole
        // object so malformed bond topology cannot be split into new objects.
        if !self.is_realized() {
            return None;
        }
        let placements = self.placements.as_ref()?;
        let connections = self.internal_connections.as_ref()?;
        if self
            .realized_shapes
            .as_ref()
            .is_some_and(|shapes| shapes.len() != self.material.parts.len())
        {
            return None;
        }
        let target_index = connections.iter().position(|bond| bond == target)?;

        let mut adjacency = vec![Vec::new(); self.material.parts.len()];
        for (index, bond) in connections.iter().enumerate() {
            if index == target_index {
                continue;
            }
            if bond.part_a >= adjacency.len() || bond.part_b >= adjacency.len() {
                return None;
            }
            adjacency[bond.part_a].push(bond.part_b);
            adjacency[bond.part_b].push(bond.part_a);
        }

        let mut components = Vec::<Vec<usize>>::new();
        let mut seen = vec![false; adjacency.len()];
        for start in 0..adjacency.len() {
            if seen[start] {
                continue;
            }
            let mut stack = vec![start];
            let mut members = Vec::new();
            seen[start] = true;
            while let Some(current) = stack.pop() {
                members.push(current);
                for &next in &adjacency[current] {
                    if !seen[next] {
                        seen[next] = true;
                        stack.push(next);
                    }
                }
            }
            members.sort_unstable();
            components.push(members);
        }

        let mut pieces = Vec::with_capacity(components.len());
        for members in components {
            let mut remap = vec![usize::MAX; self.material.parts.len()];
            let mut parts = Vec::with_capacity(members.len());
            let mut piece_placements = Vec::with_capacity(members.len());
            for (new_index, &old_index) in members.iter().enumerate() {
                remap[old_index] = new_index;
                parts.push(self.material.parts[old_index].clone());
                piece_placements.push(*placements.get(old_index)?);
            }
            let mut internal_bonds = Vec::new();
            let mut internal_connections = Vec::new();
            for bond in connections {
                if remap.get(bond.part_a).copied() != Some(usize::MAX)
                    && remap.get(bond.part_b).copied() != Some(usize::MAX)
                {
                    internal_bonds.push(crate::resources::InternalBond {
                        part_a: remap[bond.part_a],
                        part_b: remap[bond.part_b],
                    });
                    internal_connections.push(Self::remapped_bond(bond, &remap));
                }
            }
            // A split component may begin at a different constituent than
            // the original stored object. Move its owner-relative anchor by
            // that constituent's source-frame offset, rotated into owner space.
            // Keep the original owner anchor absent for environmental material.
            let owner_relative_origin = self.owner_relative_origin.map(|origin| {
                let first = piece_placements[0];
                let (sin, cos) = origin.rotation_radians.sin_cos();
                Placement {
                    x: origin.x + first.x * cos - first.y * sin,
                    y: origin.y + first.x * sin + first.y * cos,
                    rotation_radians: origin.rotation_radians + first.rotation_radians,
                }
            });
            pieces.push(Self {
                material: Material {
                    parts,
                    internal_bonds,
                },
                inertial_motion: InertialMotion::default(),
                environmental_identity: 0,
                placements: Some(piece_placements),
                internal_connections: Some(internal_connections),
                owner_relative_origin,
                realized_shapes: self.realized_shapes.as_ref().map(|shapes| {
                    members
                        .iter()
                        .map(|&old_index| shapes[old_index].clone())
                        .collect()
                }),
            });
        }
        Some(pieces)
    }

    /// Split a free environmental molecule while preserving its rigid-body
    /// momentum across the resulting independent material pieces. Owner-relative
    /// stored material must use the separate legacy storage path.
    #[allow(dead_code)] // Used by environmental BREAK when the movement transaction is enabled.
    pub(crate) fn break_environmental_internal_bond(
        &self,
        target: &PhysicalMaterialBond,
        catalog: &[BaseResource],
        vertical_period: Option<f64>,
    ) -> Option<Vec<Self>> {
        // Persisted active transformations can reach completion without passing
        // through the ordinary source-selection gate. Revalidate the whole
        // object so malformed bond topology cannot be split into new objects.
        if !self.is_realized() || self.owner_relative_origin.is_some() {
            return None;
        }
        let source_mass_properties = self.mass_properties(catalog, vertical_period)?;
        let placements = self.placements.as_ref()?;
        let connections = self.internal_connections.as_ref()?;
        if self
            .realized_shapes
            .as_ref()
            .is_some_and(|shapes| shapes.len() != self.material.parts.len())
        {
            return None;
        }
        let target_index = connections.iter().position(|bond| bond == target)?;

        let mut adjacency = vec![Vec::new(); self.material.parts.len()];
        for (index, bond) in connections.iter().enumerate() {
            if index == target_index {
                continue;
            }
            if bond.part_a >= adjacency.len() || bond.part_b >= adjacency.len() {
                return None;
            }
            adjacency[bond.part_a].push(bond.part_b);
            adjacency[bond.part_b].push(bond.part_a);
        }

        let mut components = Vec::<Vec<usize>>::new();
        let mut seen = vec![false; adjacency.len()];
        for start in 0..adjacency.len() {
            if seen[start] {
                continue;
            }
            let mut stack = vec![start];
            let mut members = Vec::new();
            seen[start] = true;
            while let Some(current) = stack.pop() {
                members.push(current);
                for &next in &adjacency[current] {
                    if !seen[next] {
                        seen[next] = true;
                        stack.push(next);
                    }
                }
            }
            members.sort_unstable();
            components.push(members);
        }

        let mut pieces = Vec::with_capacity(components.len());
        for members in components {
            let mut remap = vec![usize::MAX; self.material.parts.len()];
            let mut parts = Vec::with_capacity(members.len());
            let mut piece_placements = Vec::with_capacity(members.len());
            for (new_index, &old_index) in members.iter().enumerate() {
                remap[old_index] = new_index;
                parts.push(self.material.parts[old_index].clone());
                piece_placements.push(*placements.get(old_index)?);
            }
            let mut internal_bonds = Vec::new();
            let mut internal_connections = Vec::new();
            for bond in connections {
                if remap.get(bond.part_a).copied() != Some(usize::MAX)
                    && remap.get(bond.part_b).copied() != Some(usize::MAX)
                {
                    internal_bonds.push(crate::resources::InternalBond {
                        part_a: remap[bond.part_a],
                        part_b: remap[bond.part_b],
                    });
                    internal_connections.push(Self::remapped_bond(bond, &remap));
                }
            }
            // A split component may begin at a different constituent than
            // the original stored object. Move its owner-relative anchor by
            // that constituent's source-frame offset, rotated into owner space.
            // Keep the original owner anchor absent for environmental material.
            let owner_relative_origin = self.owner_relative_origin.map(|origin| {
                let first = piece_placements[0];
                let (sin, cos) = origin.rotation_radians.sin_cos();
                Placement {
                    x: origin.x + first.x * cos - first.y * sin,
                    y: origin.y + first.x * sin + first.y * cos,
                    rotation_radians: origin.rotation_radians + first.rotation_radians,
                }
            });
            let mut piece = Self {
                material: Material {
                    parts,
                    internal_bonds,
                },
                environmental_identity: 0,
                inertial_motion: self.inertial_motion,
                placements: Some(piece_placements),
                internal_connections: Some(internal_connections),
                owner_relative_origin,
                realized_shapes: self.realized_shapes.as_ref().map(|shapes| {
                    members
                        .iter()
                        .map(|&old_index| shapes[old_index].clone())
                        .collect()
                }),
            };
            let piece_mass_properties = piece.mass_properties(catalog, vertical_period)?;
            let offset_x = piece_mass_properties.center_x - source_mass_properties.center_x;
            let raw_offset_y = piece_mass_properties.center_y - source_mass_properties.center_y;
            let offset_y = vertical_period.map_or(raw_offset_y, |period| {
                (raw_offset_y + period * 0.5).rem_euclid(period) - period * 0.5
            });
            let (velocity_x, velocity_y) = self
                .inertial_motion
                .velocity_at_offset(offset_x, offset_y)?;
            piece.inertial_motion = InertialMotion {
                velocity_x,
                velocity_y,
                angular_velocity: self.inertial_motion.angular_velocity,
            };
            pieces.push(piece);
        }
        Some(pieces)
    }

    fn remapped_bond(bond: &PhysicalMaterialBond, remap: &[usize]) -> PhysicalMaterialBond {
        PhysicalMaterialBond {
            part_a: remap[bond.part_a],
            endpoint_a: bond.endpoint_a,
            part_b: remap[bond.part_b],
            endpoint_b: bond.endpoint_b,
            bond_energy: bond.bond_energy,
        }
    }

    pub(crate) fn is_realized(&self) -> bool {
        let Some(placements) = self.placements.as_ref() else {
            return false;
        };
        let Some(connections) = self.internal_connections.as_ref() else {
            return false;
        };
        if placements.is_empty()
            || !self.inertial_motion.is_finite()
            || !self.material.is_valid()
            || placements.len() != self.material.parts.len()
            || placements.iter().any(|placement| {
                !placement.x.is_finite()
                    || !placement.y.is_finite()
                    || !placement.rotation_radians.is_finite()
            })
            || self.owner_relative_origin.is_some_and(|origin| {
                !origin.x.is_finite()
                    || !origin.y.is_finite()
                    || !origin.rotation_radians.is_finite()
            })
            || connections.len() != self.material.internal_bonds.len()
            || connections.iter().any(|connection| {
                connection.part_a >= placements.len()
                    || connection.part_b >= placements.len()
                    || !connection_endpoint_is_finite(connection.endpoint_a)
                    || !connection_endpoint_is_finite(connection.endpoint_b)
                    || !connection.bond_energy.is_finite()
                    || connection.bond_energy < 0.0
            })
            || self
                .material
                .internal_bonds
                .iter()
                .zip(connections)
                .any(|(bond, connection)| {
                    !((bond.part_a == connection.part_a && bond.part_b == connection.part_b)
                        || (bond.part_a == connection.part_b && bond.part_b == connection.part_a))
                })
        {
            return false;
        }
        self.realized_shapes.as_ref().map_or(true, |shapes| {
            shapes.len() == self.material.parts.len()
                && shapes.iter().all(crate::resources::Shape::is_valid)
        })
    }

    /// Re-express this realization in a deterministic local frame anchored on
    /// its first constituent. This changes only coordinate representation; all
    /// relative positions, orientations, geometry, and endpoint identities are
    /// preserved. World position is supplied later by the owning context.
    pub(crate) fn into_intrinsic_frame(self) -> Option<Self> {
        let placements = self.placements?;
        let origin = *placements.first()?;
        let (sin, cos) = origin.rotation_radians.sin_cos();
        let rebased = placements
            .into_iter()
            .map(|placement| {
                let dx = placement.x - origin.x;
                let dy = placement.y - origin.y;
                Placement {
                    x: dx * cos + dy * sin,
                    y: -dx * sin + dy * cos,
                    rotation_radians: placement.rotation_radians - origin.rotation_radians,
                }
            })
            .collect();
        let inertial_motion = self
            .inertial_motion
            .rotated_axes(-origin.rotation_radians)?;
        Some(Self {
            material: self.material,
            inertial_motion,
            environmental_identity: self.environmental_identity,
            placements: Some(rebased),
            internal_connections: self.internal_connections,
            owner_relative_origin: self.owner_relative_origin,
            realized_shapes: self.realized_shapes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::{InternalBond, Material};

    #[test]
    fn newtonian_step_integrates_force_into_velocity_and_pose() {
        let catalog = crate::resources::default_catalog();
        let physical = PhysicalMaterial::realized(
            Material::free_base("Carbon", 1.0),
            vec![Placement {
                x: 3.0,
                y: 4.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .expect("carbon should realize");
        let mass = physical.mass_properties(&catalog, None).unwrap().mass;
        let advanced = physical
            .advanced_under_force(2.0, 0.0, 0.0, 0.5, &catalog, None)
            .expect("finite force should produce a staged Newtonian step");
        let expected_velocity = 2.0 / mass * 0.5;
        let expected_displacement = 0.5 * expected_velocity * 0.5;
        assert!((advanced.inertial_motion.velocity_x - expected_velocity).abs() < 1e-12);
        assert!(
            (advanced.placements.as_ref().unwrap()[0].x - (3.0 + expected_displacement)).abs()
                < 1e-12
        );
        assert_eq!(advanced.placements.as_ref().unwrap()[0].y, 4.0);
        assert_eq!(physical.inertial_motion, InertialMotion::default());
        assert_eq!(physical.placements.as_ref().unwrap()[0].x, 3.0);
    }

    #[test]
    fn newtonian_torque_rotates_about_the_realized_center_of_mass() {
        let catalog = crate::resources::default_catalog();
        let physical = PhysicalMaterial::realized(
            Material {
                parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
                internal_bonds: Vec::new(),
            },
            vec![
                Placement {
                    x: 0.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
                Placement {
                    x: 2.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
            ],
            &catalog,
        )
        .expect("composite should realize");
        let before = physical.mass_properties(&catalog, None).unwrap();
        let advanced = physical
            .advanced_under_force(0.0, 0.0, 2.0, 0.5, &catalog, None)
            .expect("finite torque should produce a staged rigid-body step");
        let after = advanced.mass_properties(&catalog, None).unwrap();
        assert!((after.center_x - before.center_x).abs() < 1e-12);
        assert!((after.center_y - before.center_y).abs() < 1e-12);
        assert!(advanced.inertial_motion.angular_velocity > 0.0);
        assert!(advanced.placements.as_ref().unwrap()[1].y > 0.0);
        assert_eq!(physical.inertial_motion, InertialMotion::default());
    }

    #[test]
    fn newtonian_step_preserves_momentum_without_force_and_wraps_vertical_pose() {
        let catalog = crate::resources::default_catalog();
        let mut physical = PhysicalMaterial::realized(
            Material::free_base("Carbon", 1.0),
            vec![Placement {
                x: 1.0,
                y: 9.8,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .expect("carbon should realize");
        physical.inertial_motion.velocity_x = 2.0;
        physical.inertial_motion.velocity_y = 1.0;
        physical.inertial_motion.angular_velocity = 0.25;
        let advanced = physical
            .advanced_under_force(0.0, 0.0, 0.0, 0.5, &catalog, Some(10.0))
            .expect("force-free motion should preserve momentum");
        assert_eq!(advanced.inertial_motion, physical.inertial_motion);
        let placement = advanced.placements.as_ref().unwrap()[0];
        assert!((placement.x - 2.0).abs() < 1e-12);
        assert!((placement.y - 0.3).abs() < 1e-12);
        assert!((placement.rotation_radians - 0.125).abs() < 1e-12);
    }

    #[test]
    fn newtonian_step_rejects_invalid_force_without_mutating_source() {
        let catalog = crate::resources::default_catalog();
        let physical = PhysicalMaterial::realized(
            Material::free_base("Carbon", 1.0),
            vec![Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .expect("carbon should realize");
        assert!(physical
            .advanced_under_force(f64::NAN, 0.0, 0.0, 1.0, &catalog, None)
            .is_none());
        assert_eq!(physical.inertial_motion, InertialMotion::default());
        assert_eq!(physical.placements.as_ref().unwrap()[0].x, 0.0);
    }

    #[test]
    fn mass_properties_use_constituent_mass_and_realized_geometry() {
        let catalog = crate::resources::default_catalog();
        let physical = PhysicalMaterial::realized(
            Material {
                parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
                internal_bonds: Vec::new(),
            },
            vec![
                Placement {
                    x: 0.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
                Placement {
                    x: 2.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
            ],
            &catalog,
        )
        .expect("unbonded composite should realize");
        let properties = physical
            .mass_properties(&catalog, None)
            .expect("realized geometry should have mass properties");
        assert!((properties.mass - 1.25).abs() < 1e-12);
        assert!((properties.center_x - 0.4).abs() < 1e-12);
        assert_eq!(properties.center_y, 0.0);
        let carbon_form = &catalog
            .iter()
            .find(|resource| resource.name == "Carbon")
            .unwrap()
            .shape
            .form;
        let hydrogen_form = &catalog
            .iter()
            .find(|resource| resource.name == "Hydrogen")
            .unwrap()
            .shape
            .form;
        let expected_inertia = 0.8
            + intrinsic_moment_of_inertia(carbon_form, 1.0)
            + intrinsic_moment_of_inertia(hydrogen_form, 0.25);
        assert!((properties.moment_of_inertia - expected_inertia).abs() < 1e-12);
    }

    #[test]
    fn mass_properties_respect_periodic_vertical_geometry() {
        let catalog = crate::resources::default_catalog();
        let physical = PhysicalMaterial::realized(
            Material {
                parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
                internal_bonds: Vec::new(),
            },
            vec![
                Placement {
                    x: 0.0,
                    y: 0.1,
                    rotation_radians: 0.0,
                },
                Placement {
                    x: 0.0,
                    y: 99.9,
                    rotation_radians: 0.0,
                },
            ],
            &catalog,
        )
        .expect("unbonded composite should realize");
        let properties = physical
            .mass_properties(&catalog, Some(100.0))
            .expect("periodic geometry should have mass properties");
        assert!((properties.center_y - 0.06).abs() < 1e-12);
        assert!(physical.mass_properties(&catalog, Some(0.0)).is_none());
    }

    #[test]
    fn inertial_motion_integrates_force_and_torque_without_drag() {
        let initial = InertialMotion {
            velocity_x: 2.0,
            velocity_y: -1.0,
            angular_velocity: 0.5,
        };
        let next = initial
            .advanced(4.0, -2.0, 3.0, 2.0, 1.5, 0.25)
            .expect("finite Newtonian update should succeed");
        assert_eq!(next.velocity_x, 2.5);
        assert_eq!(next.velocity_y, -1.25);
        assert_eq!(next.angular_velocity, 1.0);

        let coasting = next
            .advanced(0.0, 0.0, 0.0, 2.0, 1.5, 10.0)
            .expect("unforced motion should persist");
        assert_eq!(coasting, next);
    }

    #[test]
    fn inertial_motion_kinetic_energy_includes_translation_and_rotation() {
        let motion = InertialMotion {
            velocity_x: 3.0,
            velocity_y: 4.0,
            angular_velocity: 2.0,
        };
        assert_eq!(motion.kinetic_energy(2.0, 0.5), Some(26.0));
        assert_eq!(
            InertialMotion::default().kinetic_energy(2.0, 0.5),
            Some(0.0)
        );
    }

    #[test]
    fn inertial_motion_rejects_invalid_dynamics_inputs() {
        let motion = InertialMotion::default();
        assert!(motion.advanced(1.0, 0.0, 0.0, 0.0, 1.0, 1.0).is_none());
        assert!(motion.advanced(1.0, 0.0, 0.0, 1.0, 0.0, 1.0).is_none());
        assert!(motion.advanced(1.0, 0.0, 0.0, 1.0, 1.0, f64::NAN).is_none());
        assert!(motion.kinetic_energy(-1.0, 1.0).is_none());
    }

    #[test]
    fn intrinsic_frame_conversion_preserves_center_of_mass_velocity() {
        let motion = InertialMotion {
            velocity_x: 1.0,
            velocity_y: 2.0,
            angular_velocity: 3.0,
        };
        let local = motion.rotated_axes(-std::f64::consts::FRAC_PI_2).unwrap();
        assert!((local.velocity_x - 2.0).abs() < 1e-12);
        assert!((local.velocity_y + 1.0).abs() < 1e-12);
        assert_eq!(local.angular_velocity, motion.angular_velocity);
    }

    #[test]
    fn internal_bond_boundary_allows_rounding_noise_but_not_real_separation() {
        let catalog = crate::resources::default_catalog();
        let realize_pair = |separation: f64| {
            PhysicalMaterial::realized(
                Material {
                    parts: vec![("Hydrogen".into(), 1.0), ("Hydrogen".into(), 1.0)],
                    internal_bonds: vec![InternalBond {
                        part_a: 0,
                        part_b: 1,
                    }],
                },
                vec![
                    Placement {
                        x: 0.0,
                        y: 0.0,
                        rotation_radians: 0.0,
                    },
                    Placement {
                        x: separation,
                        y: 0.0,
                        rotation_radians: 0.0,
                    },
                ],
                &catalog,
            )
        };

        assert!(
            realize_pair(2.0 + 5e-13).is_some(),
            "sub-picometer numerical excess at the unit bond boundary should be accepted"
        );
        assert!(
            realize_pair(2.0 + 5e-10).is_none(),
            "a genuine geometric excess must still be rejected"
        );
    }

    #[test]
    fn realized_fluid_shape_area_scales_with_material_amount() {
        let catalog = crate::resources::default_catalog();
        let water = catalog
            .iter()
            .find(|resource| resource.name == "Water")
            .unwrap();
        let unit_area = match &water.shape.form {
            crate::resources::Form::Circle { radius } => std::f64::consts::PI * radius * radius,
            _ => panic!("default Water shape should have a measurable area"),
        };
        let merged_area = 2.0 * unit_area;
        let family = crate::capillary_geometry::CapillaryContactFamily::solve(
            merged_area,
            std::f64::consts::FRAC_PI_2,
        )
        .unwrap();
        let arc =
            crate::capillary_geometry::CircularArcBoundary::from_flat_contact(family).unwrap();
        let shape = crate::resources::Shape {
            form: crate::resources::Form::Fluid {
                nominal_area: merged_area,
                boundary: None,
                arc_boundary: Some(arc),
                multi_contact_boundary: None,
            },
        };
        let physical = PhysicalMaterial {
            material: Material::free_base("Water", 2.0),
            inertial_motion: InertialMotion::default(),
            placements: Some(vec![Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }]),
            internal_connections: Some(Vec::new()),
            owner_relative_origin: None,
            environmental_identity: 0,
            realized_shapes: Some(vec![shape.clone()]),
        };

        assert_eq!(physical.realized_shape_for_part(0, &catalog), Some(shape));

        let unit_family = crate::capillary_geometry::CapillaryContactFamily::solve(
            unit_area,
            std::f64::consts::FRAC_PI_2,
        )
        .unwrap();
        let unit_arc =
            crate::capillary_geometry::CircularArcBoundary::from_flat_contact(unit_family).unwrap();
        let underfilled_shape = crate::resources::Shape {
            form: crate::resources::Form::Fluid {
                nominal_area: unit_area,
                boundary: None,
                arc_boundary: Some(unit_arc),
                multi_contact_boundary: None,
            },
        };
        let underfilled = PhysicalMaterial {
            material: Material::free_base("Water", 2.0),
            inertial_motion: InertialMotion::default(),
            placements: Some(vec![Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }]),
            internal_connections: Some(Vec::new()),
            owner_relative_origin: None,
            environmental_identity: 0,
            realized_shapes: Some(vec![underfilled_shape]),
        };
        assert_eq!(underfilled.realized_shape_for_part(0, &catalog), None);
    }

    #[test]
    fn aggregate_fluid_requires_and_preserves_an_authoritative_shape() {
        let catalog = crate::resources::default_catalog();
        let water = catalog
            .iter()
            .find(|resource| resource.name == "Water")
            .unwrap();
        let unit_area = match &water.shape.form {
            crate::resources::Form::Circle { radius } => std::f64::consts::PI * radius * radius,
            _ => panic!("default Water shape should have a measurable area"),
        };
        let merged_area = 2.0 * unit_area;
        let family = crate::capillary_geometry::CapillaryContactFamily::solve(
            merged_area,
            std::f64::consts::FRAC_PI_2,
        )
        .unwrap();
        let arc =
            crate::capillary_geometry::CircularArcBoundary::from_flat_contact(family).unwrap();
        let shape = crate::resources::Shape {
            form: crate::resources::Form::Fluid {
                nominal_area: merged_area,
                boundary: None,
                arc_boundary: Some(arc),
                multi_contact_boundary: None,
            },
        };
        let material = Material::free_base("Water", 2.0);
        let placement = Placement {
            x: 0.0,
            y: 0.0,
            rotation_radians: 0.0,
        };

        assert!(
            PhysicalMaterial::realized(material.clone(), vec![placement], &catalog).is_none(),
            "aggregate fluid cannot be stored with a one-unit default shape"
        );
        let physical = PhysicalMaterial::realized_with_single_fluid_shape(
            material,
            placement,
            shape.clone(),
            &catalog,
        )
        .expect("aggregate fluid with conserved-area geometry should be storable");
        assert_eq!(physical.realized_shape_for_part(0, &catalog), Some(shape));
    }

    #[test]
    fn breaking_stored_compound_preserves_rotated_owner_relative_component_offsets() {
        let catalog = crate::resources::default_catalog();
        let material = Material {
            parts: vec![
                ("Carbon".into(), 1.0),
                ("Hydrogen".into(), 1.0),
                ("Carbon".into(), 1.0),
            ],
            internal_bonds: vec![
                InternalBond {
                    part_a: 0,
                    part_b: 1,
                },
                InternalBond {
                    part_a: 1,
                    part_b: 2,
                },
            ],
        };
        let owner_rotation = 0.7_f64;
        let physical = PhysicalMaterial::realized(
            material,
            vec![
                Placement {
                    x: 0.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
                Placement {
                    x: 0.838,
                    y: 0.0,
                    rotation_radians: 0.25,
                },
                Placement {
                    x: 1.676,
                    y: 0.0,
                    rotation_radians: 0.5,
                },
            ],
            &catalog,
        )
        .expect("stored compound should be realizable");
        let mut physical = physical;
        physical.owner_relative_origin = Some(Placement {
            x: 3.0,
            y: -2.0,
            rotation_radians: owner_rotation,
        });
        let target = physical.internal_connections.as_ref().unwrap()[0].clone();

        let pieces = physical
            .break_internal_bond(&target, &catalog, None)
            .expect("stored bond");
        assert_eq!(pieces.len(), 2);
        let offset = 0.838_f64;
        let expected = Placement {
            x: 3.0 + offset * owner_rotation.cos(),
            y: -2.0 + offset * owner_rotation.sin(),
            rotation_radians: owner_rotation + 0.25,
        };
        let actual = pieces[1]
            .owner_relative_origin
            .expect("second component retains its distinct owner-relative anchor");
        assert!((actual.x - expected.x).abs() <= 1e-12);
        assert!((actual.y - expected.y).abs() <= 1e-12);
        assert!((actual.rotation_radians - expected.rotation_radians).abs() <= 1e-12);
        assert_eq!(
            pieces[0].owner_relative_origin,
            physical.owner_relative_origin,
            "the component anchored at the original first constituent keeps the original owner offset"
        );
    }

    #[test]
    fn breaking_internal_bond_splits_only_the_stored_material() {
        let catalog = crate::resources::default_catalog();
        let material = Material {
            parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
            internal_bonds: vec![InternalBond {
                part_a: 0,
                part_b: 1,
            }],
        };
        let physical = PhysicalMaterial::realized(
            material,
            vec![
                Placement {
                    x: 0.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
                Placement {
                    x: 0.838,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
            ],
            &catalog,
        )
        .expect("stored compound should be realizable");
        let mut physical = physical;
        physical.inertial_motion = InertialMotion {
            velocity_x: 1.25,
            velocity_y: -0.5,
            angular_velocity: 0.75,
        };
        let source_mass_properties = physical.mass_properties(&catalog, None).unwrap();
        let expected_motion = physical.inertial_motion;
        let target = physical.internal_connections.as_ref().unwrap()[0].clone();
        let pieces = physical
            .break_internal_bond(&target, &catalog, None)
            .expect("stored bond");
        assert_eq!(pieces.len(), 2);
        let source_kinetic_energy = expected_motion
            .kinetic_energy(
                source_mass_properties.mass,
                source_mass_properties.moment_of_inertia,
            )
            .unwrap();
        let mut pieces_kinetic_energy = 0.0;
        for piece in &pieces {
            let properties = piece.mass_properties(&catalog, None).unwrap();
            let expected = expected_motion
                .velocity_at_offset(
                    properties.center_x - source_mass_properties.center_x,
                    properties.center_y - source_mass_properties.center_y,
                )
                .unwrap();
            assert!((piece.inertial_motion.velocity_x - expected.0).abs() < 1e-12);
            assert!((piece.inertial_motion.velocity_y - expected.1).abs() < 1e-12);
            assert_eq!(
                piece.inertial_motion.angular_velocity,
                expected_motion.angular_velocity
            );
            pieces_kinetic_energy += piece
                .inertial_motion
                .kinetic_energy(properties.mass, properties.moment_of_inertia)
                .unwrap();
        }
        assert!(
            (pieces_kinetic_energy - source_kinetic_energy).abs() < 1e-12,
            "splitting a rigid body without an impulse must conserve total kinetic energy"
        );
        assert!(pieces
            .iter()
            .all(|piece| piece.material.internal_bonds.is_empty()));
        assert_eq!(
            pieces
                .iter()
                .map(|piece| piece.material.parts.len())
                .sum::<usize>(),
            2
        );
    }
    #[test]
    fn exact_fluid_shape_override_survives_physical_material_serialization() {
        let catalog = crate::resources::default_catalog();
        let water = catalog
            .iter()
            .find(|resource| resource.name == "Water")
            .unwrap();
        let area = match &water.shape.form {
            crate::resources::Form::Circle { radius } => std::f64::consts::PI * radius * radius,
            _ => panic!("default Water shape should have a measurable area"),
        };
        let family = crate::capillary_geometry::CapillaryContactFamily::solve(
            area,
            std::f64::consts::FRAC_PI_2,
        )
        .unwrap();
        let arc =
            crate::capillary_geometry::CircularArcBoundary::from_flat_contact(family).unwrap();
        let shape = crate::resources::Shape {
            form: crate::resources::Form::Fluid {
                nominal_area: area,
                boundary: None,
                arc_boundary: Some(arc),
                multi_contact_boundary: None,
            },
        };
        let instance = PhysicalMaterial::realized(
            Material::free_base("Water", 1.0),
            vec![Placement {
                x: 4.0,
                y: 5.0,
                rotation_radians: 0.3,
            }],
            &catalog,
        )
        .unwrap()
        .with_single_fluid_shape(shape.clone(), &catalog)
        .expect("a conserved-area exact fluid shape should be storable");

        assert_eq!(instance.realized_shape_for_part(0, &catalog), Some(shape));
        let encoded = serde_json::to_string(&instance).unwrap();
        let decoded: PhysicalMaterial = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, instance);
        assert!(decoded.realized_shape_for_part(0, &catalog).is_some());
    }

    #[test]
    fn exact_multi_contact_fluid_shape_survives_material_round_trip() {
        let catalog = crate::resources::default_catalog();
        let water = catalog
            .iter()
            .find(|resource| resource.name == "Water")
            .unwrap();
        let area = match &water.shape.form {
            crate::resources::Form::Circle { radius } => std::f64::consts::PI * radius * radius,
            _ => panic!("default Water shape should have a measurable area"),
        };
        let half = (area.sqrt()) * 0.5;
        let angle = std::f64::consts::FRAC_PI_2;
        let boundary = crate::capillary_geometry::MultiContactFluidBoundary {
            nominal_area: area,
            pieces: vec![
                crate::capillary_geometry::FluidBoundaryPiece::SolidContact {
                    start_x: -half,
                    start_y: -half,
                    end_x: half,
                    end_y: -half,
                    contact_angle_radians: angle,
                },
                crate::capillary_geometry::FluidBoundaryPiece::SolidContact {
                    start_x: half,
                    start_y: -half,
                    end_x: half,
                    end_y: half,
                    contact_angle_radians: angle,
                },
                crate::capillary_geometry::FluidBoundaryPiece::SolidContact {
                    start_x: half,
                    start_y: half,
                    end_x: -half,
                    end_y: half,
                    contact_angle_radians: angle,
                },
                crate::capillary_geometry::FluidBoundaryPiece::SolidContact {
                    start_x: -half,
                    start_y: half,
                    end_x: -half,
                    end_y: -half,
                    contact_angle_radians: angle,
                },
            ],
        };
        let shape = crate::resources::Shape {
            form: crate::resources::Form::Fluid {
                nominal_area: area,
                boundary: None,
                arc_boundary: None,
                multi_contact_boundary: Some(boundary),
            },
        };
        assert!(shape.form.is_valid());
        let instance = PhysicalMaterial::realized(
            Material::free_base("Water", 1.0),
            vec![Placement {
                x: 2.0,
                y: -1.0,
                rotation_radians: 0.4,
            }],
            &catalog,
        )
        .unwrap()
        .with_single_fluid_shape(shape.clone(), &catalog)
        .expect("multi-contact fluid shape should retain conserved area");
        assert_eq!(
            instance.realized_shape_for_part(0, &catalog),
            Some(shape.clone())
        );
        let encoded = serde_json::to_string(&instance).unwrap();
        let decoded: PhysicalMaterial = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded.realized_shape_for_part(0, &catalog), Some(shape));
    }

    #[test]
    fn restoring_stored_water_preserves_exact_arc_geometry() {
        let catalog = crate::resources::default_catalog();
        let water = catalog
            .iter()
            .find(|resource| resource.name == "Water")
            .unwrap();
        let area = match &water.shape.form {
            crate::resources::Form::Circle { radius } => std::f64::consts::PI * radius * radius,
            _ => panic!("default Water shape should have a measurable area"),
        };
        let family = crate::capillary_geometry::CapillaryContactFamily::solve(
            area,
            std::f64::consts::FRAC_PI_2,
        )
        .unwrap();
        let arc =
            crate::capillary_geometry::CircularArcBoundary::from_flat_contact(family).unwrap();
        let shape = crate::resources::Shape {
            form: crate::resources::Form::Fluid {
                nominal_area: area,
                boundary: None,
                arc_boundary: Some(arc),
                multi_contact_boundary: None,
            },
        };
        let instance = PhysicalMaterial::realized(
            Material::free_base("Water", 1.0),
            vec![Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .unwrap()
        .with_single_fluid_shape(shape.clone(), &catalog)
        .unwrap();
        let mut structure = OrganismStructure::new();
        let indices = crate::material_restoration::restore_material(
            &mut structure,
            &instance,
            Placement {
                x: 10.0,
                y: 20.0,
                rotation_radians: 0.5,
            },
            &catalog,
        )
        .expect("stored exact fluid shape should restore");

        assert_eq!(indices.len(), 1);
        assert_eq!(structure.units[indices[0]].shape(&catalog), Some(&shape));
        assert_eq!(structure.units[indices[0]].placement.x, 10.0);
        assert_eq!(structure.units[indices[0]].placement.y, 20.0);
    }
    #[test]
    fn realized_state_rejects_non_finite_placement_coordinates() {
        let catalog = crate::resources::default_catalog();
        let mut physical = PhysicalMaterial::realized(
            Material::free_base("Carbon", 1.0),
            vec![Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .expect("test material should realize");
        physical.placements.as_mut().unwrap()[0].x = f64::NAN;
        assert!(!physical.is_realized());
    }

    #[test]
    fn realized_state_rejects_placement_count_mismatches() {
        let catalog = crate::resources::default_catalog();
        let material = Material {
            parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
            internal_bonds: Vec::new(),
        };
        let valid = PhysicalMaterial::realized(
            material,
            vec![
                Placement {
                    x: 0.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
                Placement {
                    x: 1.5,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
            ],
            &catalog,
        )
        .expect("valid unbonded composite should realize");

        let mut missing = valid.clone();
        missing.placements.as_mut().unwrap().pop();
        assert!(
            !missing.is_realized(),
            "a persisted composite missing a constituent placement must fail closed"
        );

        let mut extra = valid;
        extra.placements.as_mut().unwrap().push(Placement {
            x: 3.0,
            y: 0.0,
            rotation_radians: 0.0,
        });
        assert!(
            !extra.is_realized(),
            "a persisted composite with an extra placement must fail closed"
        );
    }

    #[test]
    fn realized_state_rejects_invalid_persisted_material_composition() {
        let catalog = crate::resources::default_catalog();
        let mut physical = PhysicalMaterial::realized(
            Material::free_base("Carbon", 1.0),
            vec![Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .expect("valid single constituent should realize");
        physical.material.parts[0].1 = 1.5;
        assert!(
            !physical.is_realized(),
            "fractional persisted rigid constituent must not count as realized geometry"
        );
    }

    #[test]
    fn realized_state_rejects_non_finite_persisted_connection_endpoints() {
        let catalog = crate::resources::default_catalog();
        let mut physical = PhysicalMaterial::realized(
            Material {
                parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
                internal_bonds: vec![crate::resources::InternalBond {
                    part_a: 0,
                    part_b: 1,
                }],
            },
            vec![
                Placement {
                    x: 0.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
                Placement {
                    x: 0.838,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
            ],
            &catalog,
        )
        .expect("valid bonded composite should realize");
        physical.internal_connections.as_mut().unwrap()[0].endpoint_a =
            ConnectionEndpoint::Boundary {
                angle_radians: f64::NAN,
            };
        assert!(
            !physical.is_realized(),
            "non-finite persisted endpoint metadata must fail closed"
        );
    }

    #[test]
    fn realized_state_rejects_connections_that_disagree_with_material_bond_topology() {
        let catalog = crate::resources::default_catalog();
        let mut physical = PhysicalMaterial::realized(
            Material {
                parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
                internal_bonds: vec![InternalBond {
                    part_a: 0,
                    part_b: 1,
                }],
            },
            vec![
                Placement {
                    x: 0.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
                Placement {
                    x: 0.838,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
            ],
            &catalog,
        )
        .expect("test composite should realize");
        let connection = &mut physical.internal_connections.as_mut().unwrap()[0];
        connection.part_a = 0;
        connection.part_b = 0;
        let malformed_target = physical.internal_connections.as_ref().unwrap()[0].clone();

        assert!(
            !physical.is_realized(),
            "persisted connection endpoints must agree with declared material topology"
        );
        assert!(
            physical
                .break_internal_bond(&malformed_target, &catalog, None)
                .is_none(),
            "stored BREAK completion must fail closed on malformed persisted topology"
        );
    }

    #[test]
    fn realized_state_rejects_non_finite_owner_relative_origin() {
        let catalog = crate::resources::default_catalog();
        let valid = PhysicalMaterial::realized(
            Material::free_base("Carbon", 1.0),
            vec![Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .expect("valid single constituent should realize");

        for origin in [
            Placement {
                x: f64::NAN,
                y: 0.0,
                rotation_radians: 0.0,
            },
            Placement {
                x: 0.0,
                y: f64::INFINITY,
                rotation_radians: 0.0,
            },
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: f64::NEG_INFINITY,
            },
        ] {
            let mut malformed = valid.clone();
            malformed.owner_relative_origin = Some(origin);
            assert!(
                !malformed.is_realized(),
                "non-finite owner-relative coordinates must fail closed"
            );
        }
    }

    #[test]
    fn realized_state_rejects_connections_outside_placement_indices() {
        let catalog = crate::resources::default_catalog();
        let mut physical = PhysicalMaterial::realized(
            Material {
                parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
                internal_bonds: vec![InternalBond {
                    part_a: 0,
                    part_b: 1,
                }],
            },
            vec![
                Placement {
                    x: 0.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
                Placement {
                    x: 1.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
            ],
            &catalog,
        )
        .expect("test composite should realize");
        physical.internal_connections.as_mut().unwrap()[0].part_b = 2;
        assert!(!physical.is_realized());
    }
}
