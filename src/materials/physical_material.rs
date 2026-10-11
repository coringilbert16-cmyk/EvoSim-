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
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct PhysicalMaterial {
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
    pub(crate) fn realized(
        material: Material,
        placements: Vec<Placement>,
        catalog: &[BaseResource],
    ) -> Option<Self> {
        if !material.is_valid()
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
                return None;
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
            });
        }

        Some(Self {
            material,
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
        if (*amount - 1.0).abs() > 1e-9 {
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
                arc_boundary: Some(_), ..} => *nominal_area,
            _ => return None,
        };
        if (shape_area - default_area).abs() > 1e-9 * default_area.max(1.0) {
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
        let (name, _) = self.material.parts.get(part_index)?;
        let resource = catalog.iter().find(|resource| resource.name == *name)?;
        let shape = match self.realized_shapes.as_ref() {
            Some(shapes) if shapes.len() == self.material.parts.len() => shapes.get(part_index)?,
            Some(_) => return None,
            None => &resource.shape,
        };
        if !shape.is_valid() {
            return None;
        }
        if shape != &resource.shape {
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
                    boundary: Some(_),
                    ..
                }
                | crate::resources::Form::Fluid {
                    nominal_area,
                    arc_boundary: Some(_),
                    ..
                } => *nominal_area,
                _ => return None,
            };
            if (shape_area - default_area).abs() > 1e-9 * default_area.max(1.0) {
                return None;
            }
        }
        Some(shape.clone())
    }

    /// Break one pre-existing internal bond and return the resulting physically
    /// disconnected pieces. The bond itself must belong to this stored material.
    pub(crate) fn break_internal_bond(&self, target: &PhysicalMaterialBond) -> Option<Vec<Self>> {
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
            pieces.push(Self {
                material: Material {
                    parts,
                    internal_bonds,
                },
                placements: Some(piece_placements),
                internal_connections: Some(internal_connections),
                owner_relative_origin: self.owner_relative_origin,
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

    fn remapped_bond(bond: &PhysicalMaterialBond, remap: &[usize]) -> PhysicalMaterialBond {
        PhysicalMaterialBond {
            part_a: remap[bond.part_a],
            endpoint_a: bond.endpoint_a,
            part_b: remap[bond.part_b],
            endpoint_b: bond.endpoint_b,
        }
    }

    pub(crate) fn is_realized(&self) -> bool {
        self.placements.is_some()
            && self.internal_connections.is_some()
            && self.realized_shapes.as_ref().map_or(true, |shapes| {
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
}
