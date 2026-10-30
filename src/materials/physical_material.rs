//! A physically realized material instance carried through acquisition/storage.
//!
//! `Material` owns composition and the identity of pre-existing internal bonds.
//! `PhysicalMaterial` owns the complete realized arrangement needed to restore
//! those bonds without inventing connection endpoints at the organism boundary.
use crate::contact::connection_pair_candidates;
use crate::resources::{BaseResource, Material};
use crate::structure::{ConnectionEndpoint, OrganismStructure, Placement, StructuralUnit};
use serde::{Deserialize, Serialize};

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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct PhysicalMaterial {
    /// Stable deterministic identity for this physical instance. Unlike its
    /// placements, this survives environmental transport and keeps ongoing
    /// chemical encounters associated with the same reactants.
    #[serde(default)]
    pub(crate) instance_id: u64,
    pub(crate) material: Material,
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
    fn hash_identity_bytes(bytes: impl IntoIterator<Item = u8>) -> u64 {
        let hash = bytes.into_iter().fold(0xcbf29ce484222325_u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
        });
        if hash == 0 { 1 } else { hash }
    }

    fn child_instance_identity(parent: u64, members: &[usize]) -> u64 {
        let signature = format!("split:{parent}:{members:?}");
        Self::hash_identity_bytes(signature.bytes())
    }

    /// Assign an identity to a legacy instance only. Ordinary movement keeps
    /// the existing identity even as world-space placements change.
    pub(crate) fn refresh_instance_identity(&mut self) {
        if self.instance_id != 0 {
            return;
        }
        let signature = format!(
            "physical:{:?}|{:?}|{:?}|{:?}",
            self.material, self.placements, self.internal_connections, self.realized_shapes
        );
        self.instance_id = Self::hash_identity_bytes(signature.bytes());
    }

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
                        candidate.available_a && candidate.available_b && candidate.distance <= 1.0
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

        let mut instance = Self {
            instance_id: 0,
            material,
            placements: Some(placements),
            internal_connections: Some(internal_connections),
            owner_relative_origin: None,
            realized_shapes: None,
        };
        instance.refresh_instance_identity();
        Some(instance)
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
            let mut piece = Self {
                instance_id: Self::child_instance_identity(self.instance_id, &members),
                material: Material {
                    parts,
                    internal_bonds,
                },
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
            if piece.instance_id == 0 {
                piece.refresh_instance_identity();
            }
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
        Some(Self {
            instance_id: self.instance_id,
            material: self.material,
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
            instance_id: 0,
            material: Material::free_base("Water", 2.0),
            placements: Some(vec![Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }]),
            internal_connections: Some(Vec::new()),
            owner_relative_origin: None,
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
            instance_id: 0,
            material: Material::free_base("Water", 2.0),
            placements: Some(vec![Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }]),
            internal_connections: Some(Vec::new()),
            owner_relative_origin: None,
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

        let pieces = physical.break_internal_bond(&target).expect("stored bond");
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
        let target = physical.internal_connections.as_ref().unwrap()[0].clone();
        let pieces = physical.break_internal_bond(&target).expect("stored bond");
        assert_eq!(pieces.len(), 2);
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
            physical.break_internal_bond(&malformed_target).is_none(),
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
