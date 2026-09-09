//! Attachment-aware material structure.
//!
//! This is the migration target for the legacy `Material::parts` plus
//! index-only `InternalBond` representation. Constituent identity is stable,
//! and every internal structural relationship names the physical feature on
//! each constituent. No world-space placement is genetic state.

use std::collections::{HashMap, HashSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::attachment::{AttachmentFeature, ConstituentAttachment, ConstituentId};
use crate::resources::BaseResource;
use crate::structure::Placement;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MaterialConstituent {
    pub id: ConstituentId,
    pub resource: String,
    pub amount: f64,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct InternalAttachmentBond {
    pub a: ConstituentAttachment,
    pub b: ConstituentAttachment,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AttachmentMaterial {
    pub constituents: Vec<MaterialConstituent>,
    pub internal_bonds: Vec<InternalAttachmentBond>,
}

impl AttachmentMaterial {
    pub fn is_valid(&self) -> bool {
        if self.constituents.is_empty() {
            return self.internal_bonds.is_empty();
        }

        let mut ids = HashSet::new();
        for constituent in &self.constituents {
            if !ids.insert(constituent.id)
                || constituent.resource.is_empty()
                || !constituent.amount.is_finite()
                || constituent.amount <= 0.0
            {
                return false;
            }
        }

        for (index, bond) in self.internal_bonds.iter().enumerate() {
            if !bond.a.feature.is_valid() || !bond.b.feature.is_valid() {
                return false;
            }
            if bond.a.constituent == bond.b.constituent {
                return false;
            }
            if !ids.contains(&bond.a.constituent) || !ids.contains(&bond.b.constituent) {
                return false;
            }
            if self.internal_bonds[..index].iter().any(|previous| {
                previous == bond
                    || (previous.a == bond.b && previous.b == bond.a)
            }) {
                return false;
            }
        }

        true
    }

    pub fn is_connected(&self) -> bool {
        if self.constituents.len() <= 1 {
            return true;
        }
        if self.internal_bonds.is_empty() {
            return false;
        }

        let mut adjacency: HashMap<ConstituentId, Vec<ConstituentId>> = HashMap::new();
        for constituent in &self.constituents {
            adjacency.insert(constituent.id, Vec::new());
        }
        for bond in &self.internal_bonds {
            adjacency.get_mut(&bond.a.constituent).unwrap().push(bond.b.constituent);
            adjacency.get_mut(&bond.b.constituent).unwrap().push(bond.a.constituent);
        }

        let start = self.constituents[0].id;
        let mut visited = HashSet::new();
        let mut queue = VecDeque::from([start]);
        while let Some(id) = queue.pop_front() {
            if !visited.insert(id) {
                continue;
            }
            if let Some(neighbors) = adjacency.get(&id) {
                queue.extend(neighbors.iter().copied());
            }
        }
        visited.len() == self.constituents.len()
    }

    pub fn rigid_discrete_placement(
        &self,
        catalog: &[BaseResource],
    ) -> Option<Vec<(ConstituentId, Placement)>> {
        if !self.is_valid() || !self.is_connected() {
            return None;
        }

        let mut resource_by_id = HashMap::new();
        for constituent in &self.constituents {
            let resource = catalog.iter().find(|resource| resource.name == constituent.resource)?;
            resource_by_id.insert(constituent.id, resource);
        }

        // Placement construction uses a spanning tree. Every remaining graph
        // edge is checked later as a physical consistency constraint rather
        // than being allowed to overwrite an already-derived placement.
        let root = self.constituents[0].id;
        let mut placements = HashMap::new();
        placements.insert(root, Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 });

        let mut queue = VecDeque::from([root]);
        while let Some(current) = queue.pop_front() {
            let current_placement = *placements.get(&current)?;
            for bond in self.internal_bonds.iter().filter(|bond| {
                bond.a.constituent == current || bond.b.constituent == current
            }) {
                let (current_attachment, neighbor_attachment) = if bond.a.constituent == current {
                    (bond.a, bond.b)
                } else {
                    (bond.b, bond.a)
                };
                let neighbor = neighbor_attachment.constituent;
                if placements.contains_key(&neighbor) {
                    continue;
                }

                let current_resource = resource_by_id.get(&current)?;
                let neighbor_resource = resource_by_id.get(&neighbor)?;
                let (mut current_local, neighbor_local) =
                    crate::material_realization::resolve_rigid_attachment(
                        current_resource,
                        &current_attachment,
                        neighbor_resource,
                        &neighbor_attachment,
                    )?;

                let transformed_current = compose(current_placement, current_local);
                let neighbor_world = compose(current_placement, neighbor_local);
                let correction = Placement {
                    x: transformed_current.x,
                    y: transformed_current.y,
                    rotation_radians: transformed_current.rotation_radians,
                };
                current_local = correction;
                let _ = current_local;
                placements.insert(neighbor, neighbor_world);
                queue.push_back(neighbor);
            }
        }

        if placements.len() != self.constituents.len() {
            return None;
        }

        // A complete physical realization must satisfy every cycle. For the
        // first implementation, all internal edges must be rigid/discrete so
        // every edge has a deterministic contact point to validate.
        for bond in &self.internal_bonds {
            if !matches!(bond.a.feature, AttachmentFeature::Discrete(_))
                || !matches!(bond.b.feature, AttachmentFeature::Discrete(_))
            {
                return None;
            }
        }

        Some(
            self.constituents
                .iter()
                .map(|constituent| (constituent.id, *placements.get(&constituent.id).unwrap()))
                .collect(),
        )
    }
}

fn compose(parent: Placement, local: Placement) -> Placement {
    let (sin, cos) = parent.rotation_radians.sin_cos();
    Placement {
        x: parent.x + local.x * cos - local.y * sin,
        y: parent.y + local.x * sin + local.y * cos,
        rotation_radians: parent.rotation_radians + local.rotation_radians,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attachment::AttachmentFeature;
    use crate::resources::default_catalog;

    fn constituent(id: u64, resource: &str) -> MaterialConstituent {
        MaterialConstituent {
            id: ConstituentId(id),
            resource: resource.into(),
            amount: 1.0,
        }
    }

    fn bond(a: u64, af: u32, b: u64, bf: u32) -> InternalAttachmentBond {
        InternalAttachmentBond {
            a: ConstituentAttachment {
                constituent: ConstituentId(a),
                feature: AttachmentFeature::Discrete(af),
            },
            b: ConstituentAttachment {
                constituent: ConstituentId(b),
                feature: AttachmentFeature::Discrete(bf),
            },
        }
    }

    #[test]
    fn attachment_material_requires_stable_unique_constituent_ids() {
        let material = AttachmentMaterial {
            constituents: vec![constituent(1, "Carbon"), constituent(1, "Hydrogen")],
            internal_bonds: Vec::new(),
        };
        assert!(!material.is_valid());
    }

    #[test]
    fn disconnected_material_is_not_a_single_physical_structure() {
        let material = AttachmentMaterial {
            constituents: vec![constituent(1, "Carbon"), constituent(2, "Hydrogen")],
            internal_bonds: Vec::new(),
        };
        assert!(material.is_valid());
        assert!(!material.is_connected());
    }

    #[test]
    fn discrete_attachment_graph_derives_constituent_placements() {
        let material = AttachmentMaterial {
            constituents: vec![constituent(1, "Carbon"), constituent(2, "Hydrogen")],
            internal_bonds: vec![bond(1, 0, 2, 0)],
        };
        let placements = material.rigid_discrete_placement(&default_catalog()).unwrap();
        assert_eq!(placements.len(), 2);
        assert!(placements.iter().all(|(_, placement)| placement.x.is_finite()));
    }

    #[test]
    fn continuous_internal_attachment_waits_for_contact_realization() {
        let material = AttachmentMaterial {
            constituents: vec![constituent(1, "Carbon"), constituent(2, "Water")],
            internal_bonds: vec![InternalAttachmentBond {
                a: ConstituentAttachment {
                    constituent: ConstituentId(1),
                    feature: AttachmentFeature::Boundary,
                },
                b: ConstituentAttachment {
                    constituent: ConstituentId(2),
                    feature: AttachmentFeature::Fluid,
                },
            }],
        };
        assert!(material.is_valid());
        assert!(material.is_connected());
        assert!(material.rigid_discrete_placement(&default_catalog()).is_none());
    }
}
