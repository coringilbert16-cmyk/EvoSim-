#![expect(
    clippy::result_large_err,
    reason = "Physical material is intentionally returned intact on failed storage transactions"
)]
#![expect(dead_code, reason = "Staged API retained for subsystem integration")]
//! Organism material inventory.
use crate::physical_material::PhysicalMaterial;
use crate::resources::Material;
use crate::structure::Placement;
use serde::{Deserialize, Serialize};

const MATERIAL_EPSILON: f64 = 1e-12;

/// One authoritative stored-material entry. Every stored object is an actual
/// realized physical material; there is no logical/invented inventory state.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) enum StoredMaterial {
    Physical(PhysicalMaterial),
}

impl StoredMaterial {
    fn material(&self) -> &Material {
        match self {
            Self::Physical(instance) => &instance.material,
        }
    }

    fn into_physical(self) -> PhysicalMaterial {
        match self {
            Self::Physical(instance) => instance,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub(crate) struct MaterialStorage {
    #[serde(default)]
    pub(crate) entries: Vec<StoredMaterial>,
}

impl MaterialStorage {
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    pub(crate) fn total_amount(&self) -> f64 {
        self.entries
            .iter()
            .map(|entry| entry.material().total_amount())
            .sum()
    }

    pub(crate) fn first_material(&self) -> Option<Material> {
        self.entries.first().map(|entry| entry.material().clone())
    }

    pub(crate) fn iter_materials(&self) -> impl Iterator<Item = &Material> {
        self.entries.iter().map(StoredMaterial::material)
    }

    pub(crate) fn materials_snapshot(&self) -> Vec<Material> {
        self.iter_materials().cloned().collect()
    }

    pub(crate) fn drain_entries(&mut self) -> Vec<StoredMaterial> {
        std::mem::take(&mut self.entries)
    }

    pub(crate) fn physical_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|entry| matches!(entry, StoredMaterial::Physical(_)))
            .count()
    }

    fn is_discrete(material: &Material) -> bool {
        material.parts.iter().all(|(_, amount)| {
            amount.is_finite() && *amount > 0.0 && amount.fract().abs() <= MATERIAL_EPSILON
        })
    }

    /// Store an already-realized physical instance without reconstructing its
    /// geometry or internal connection endpoints. The world realization is
    /// rebased only into storage's intrinsic frame.
    pub(crate) fn store_physical_instance(
        &mut self,
        instance: PhysicalMaterial,
        catalog: &[crate::resources::BaseResource],
    ) -> bool {
        if !instance.is_realized()
            || instance.material.parts.is_empty()
            || !instance.material.is_valid_for_catalog(catalog)
            || !Self::is_discrete(&instance.material)
        {
            return false;
        }
        let Some(instance) = instance.into_intrinsic_frame() else {
            return false;
        };
        self.entries.insert(0, StoredMaterial::Physical(instance));
        true
    }

    pub(crate) fn store_physical_instance_at_owner_anchor(
        &mut self,
        instance: PhysicalMaterial,
        anchor: Placement,
        catalog: &[crate::resources::BaseResource],
    ) -> bool {
        self.try_store_physical_instance_at_owner_anchor(instance, anchor, catalog)
            .is_ok()
    }

    /// Validate and normalize already owner-relative stored objects as one batch.
    /// Unlike environmental acquisition, preserve each object's owner-relative
    /// anchor instead of deriving one from its intrinsic placement.
    pub(crate) fn prepare_physical_batch(
        instances: Vec<PhysicalMaterial>,
        catalog: &[crate::resources::BaseResource],
    ) -> Option<Vec<StoredMaterial>> {
        let mut prepared = Vec::with_capacity(instances.len());
        for instance in instances {
            if !instance.is_realized()
                || instance.material.parts.is_empty()
                || !instance.material.is_valid_for_catalog(catalog)
                || !Self::is_discrete(&instance.material)
            {
                return None;
            }
            let instance = instance.into_intrinsic_frame()?;
            prepared.push(StoredMaterial::Physical(instance));
        }
        Some(prepared)
    }

    /// Validate and convert a batch of environmental physical objects into
    /// storage entries without mutating the holder. The caller can then remove
    /// the source objects and commit these prepared entries as one transfer.
    pub(crate) fn prepare_physical_batch_at_owner_anchor(
        instances: Vec<PhysicalMaterial>,
        anchor: Placement,
        catalog: &[crate::resources::BaseResource],
    ) -> Option<Vec<StoredMaterial>> {
        if !anchor.x.is_finite() || !anchor.y.is_finite() || !anchor.rotation_radians.is_finite() {
            return None;
        }
        let mut prepared = Vec::with_capacity(instances.len());
        for instance in instances {
            let origin = instance
                .placements
                .as_ref()
                .and_then(|placements| placements.first())
                .copied()?;
            if !instance.is_realized()
                || instance.material.parts.is_empty()
                || !instance.material.is_valid_for_catalog(catalog)
                || !Self::is_discrete(&instance.material)
            {
                return None;
            }
            let owner_relative_origin = Placement {
                x: origin.x - anchor.x,
                y: origin.y - anchor.y,
                rotation_radians: origin.rotation_radians,
            };
            if !owner_relative_origin.x.is_finite()
                || !owner_relative_origin.y.is_finite()
                || !owner_relative_origin.rotation_radians.is_finite()
            {
                return None;
            }
            let mut instance = instance.into_intrinsic_frame()?;
            instance.owner_relative_origin = Some(owner_relative_origin);
            prepared.push(StoredMaterial::Physical(instance));
        }
        Some(prepared)
    }

    /// Commit entries previously validated by the batch preparation method.
    /// Reversing before front insertion preserves the old per-item insertion
    /// ordering while avoiding a clone of the existing inventory.
    pub(crate) fn commit_prepared_physical_batch(&mut self, mut prepared: Vec<StoredMaterial>) {
        prepared.reverse();
        self.entries.splice(0..0, prepared);
    }

    pub(crate) fn try_store_physical_instance_at_owner_anchor(
        &mut self,
        instance: PhysicalMaterial,
        anchor: Placement,
        catalog: &[crate::resources::BaseResource],
    ) -> Result<(), PhysicalMaterial> {
        if !anchor.x.is_finite() || !anchor.y.is_finite() || !anchor.rotation_radians.is_finite() {
            return Err(instance);
        }
        let Some(origin) = instance
            .placements
            .as_ref()
            .and_then(|placements| placements.first())
            .copied()
        else {
            return Err(instance);
        };
        if !instance.is_realized()
            || instance.material.parts.is_empty()
            || !instance.material.is_valid_for_catalog(catalog)
            || !Self::is_discrete(&instance.material)
        {
            return Err(instance);
        }
        let owner_relative_origin = Placement {
            x: origin.x - anchor.x,
            y: origin.y - anchor.y,
            rotation_radians: origin.rotation_radians,
        };
        if !owner_relative_origin.x.is_finite()
            || !owner_relative_origin.y.is_finite()
            || !owner_relative_origin.rotation_radians.is_finite()
        {
            return Err(instance);
        }
        let mut instance = instance
            .into_intrinsic_frame()
            .expect("validated physical material must have an intrinsic origin");
        instance.owner_relative_origin = Some(owner_relative_origin);
        self.entries.insert(0, StoredMaterial::Physical(instance));
        Ok(())
    }

    pub(crate) fn store_physical(
        &mut self,
        material: Material,
        placements: Vec<Placement>,
        catalog: &[crate::resources::BaseResource],
    ) -> bool {
        let Some(instance) = PhysicalMaterial::realized(material, placements, catalog) else {
            return false;
        };
        self.store_physical_instance(instance, catalog)
    }

    pub(crate) fn peek_one_unstructured(&self) -> Option<PhysicalMaterial> {
        self.entries.iter().find_map(|entry| match entry {
            StoredMaterial::Physical(instance)
                if !instance.material.has_internal_structure()
                    && !instance.material.is_empty()
                    && instance.is_realized() =>
            {
                Some(instance.clone())
            }
            _ => None,
        })
    }

    pub(crate) fn take_first_unstructured(&mut self, catalog: &[crate::resources::BaseResource]) -> Option<PhysicalMaterial> {
        let index = self.entries.iter().position(|entry| {
            let material = entry.material();
            !material.has_internal_structure()
                && !material.is_empty()
                && material.is_valid_for_catalog(catalog)
                && Self::is_discrete(material)
                && matches!(entry, StoredMaterial::Physical(instance) if instance.is_realized())
        })?;
        self.take_physical_at(index, catalog)
    }

    pub(crate) fn peek_matching_physical(&self, target: &Material) -> Option<PhysicalMaterial> {
        self.entries.iter().find_map(|entry| match entry {
            StoredMaterial::Physical(instance)
                if instance.material == *target && !instance.material.is_empty() =>
            {
                Some(instance.clone())
            }
            _ => None,
        })
    }

    pub(crate) fn take_one_unstructured_named(&mut self, name: &str, catalog: &[crate::resources::BaseResource]) -> Option<PhysicalMaterial> {
        let index = self.entries.iter().position(|entry| {
            let material = entry.material();
            !material.has_internal_structure()
                && !material.is_empty()
                && material.parts.len() == 1
                && material.parts[0].0 == name
                && (material.parts[0].1 - 1.0).abs() <= MATERIAL_EPSILON
                && material.is_valid_for_catalog(catalog)
                && Self::is_discrete(material)
                && matches!(entry, StoredMaterial::Physical(instance) if instance.is_realized())
        })?;
        Some(self.entries.swap_remove(index).into_physical())
    }

    pub(crate) fn take_matching(&mut self, target: &Material, catalog: &[crate::resources::BaseResource]) -> Option<PhysicalMaterial> {
        if !target.is_valid_for_catalog(catalog) || target.is_empty() {
            return None;
        }
        let index = self.entries.iter().position(|entry| {
            matches!(entry, StoredMaterial::Physical(instance)
                if instance.is_realized()
                    && instance.material.is_valid_for_catalog(catalog)
                    && Self::is_discrete(&instance.material)
                    && instance.material == *target
                    && !instance.material.is_empty())
        })?;
        Some(self.entries.swap_remove(index).into_physical())
    }

    pub(crate) fn take_physical_at(&mut self, index: usize, catalog: &[crate::resources::BaseResource]) -> Option<PhysicalMaterial> {
        match self.entries.get(index)? {
            StoredMaterial::Physical(instance)
                if instance.is_realized()
                    && instance.material.is_valid_for_catalog(catalog)
                    && Self::is_discrete(&instance.material) => {}
            _ => return None,
        }
        match self.entries.swap_remove(index) {
            StoredMaterial::Physical(instance) => Some(instance),
        }
    }

    pub(crate) fn take_matching_physical(
        &mut self,
        target: &Material,
        catalog: &[crate::resources::BaseResource],
    ) -> Option<PhysicalMaterial> {
        // Removal is a transfer boundary too: reject legacy/corrupt entries
        // before mutating the authoritative holder, so downstream validation
        // cannot turn a failed COMBINE into lost source material.
        if !target.is_valid_for_catalog(catalog) || target.is_empty() {
            return None;
        }
        let index = self.entries.iter().position(|entry| {
            matches!(entry, StoredMaterial::Physical(instance)
                if instance.is_realized()
                    && instance.material.is_valid_for_catalog(catalog)
                    && Self::is_discrete(&instance.material)
                    && instance.material == *target
                    && !instance.material.is_empty())
        })?;
        match self.entries.swap_remove(index) {
            StoredMaterial::Physical(instance) => Some(instance),
        }
    }

    pub(crate) fn take_unstructured(
        &mut self,
        count: usize,
        catalog: &[crate::resources::BaseResource],
    ) -> Option<Vec<PhysicalMaterial>> {
        let indices: Vec<usize> = self.entries.iter().enumerate().filter_map(|(index, entry)| {
            match entry {
                StoredMaterial::Physical(instance)
                    if !instance.material.has_internal_structure()
                        && !instance.material.is_empty()
                        && instance.is_realized()
                        && instance.material.is_valid_for_catalog(catalog)
                        && Self::is_discrete(&instance.material) => Some(index),
                _ => None,
            }
        }).take(count).collect();
        if indices.len() != count {
            return None;
        }
        let mut out = Vec::with_capacity(count);
        for index in indices.into_iter().rev() {
            out.push(self.take_physical_at(index, catalog)?);
        }
        out.reverse();
        Some(out)
    }

    pub(crate) fn count_unstructured(&self) -> usize {
        self.entries
            .iter()
            .filter(|entry| {
                !entry.material().has_internal_structure() && !entry.material().is_empty()
            })
            .count()
    }

    pub(crate) fn count_structured(&self) -> usize {
        self.entries
            .iter()
            .filter(|entry| {
                entry.material().has_internal_structure() && !entry.material().is_empty()
            })
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::{BaseResource, InternalBond, Material};

    fn compound() -> Material {
        Material {
            parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
            internal_bonds: vec![InternalBond {
                part_a: 0,
                part_b: 1,
            }],
        }
    }

    fn catalog() -> Vec<BaseResource> {
        crate::resources::default_catalog()
    }

    #[test]
    fn every_storage_entry_is_a_realized_physical_instance() {
        let mut storage = MaterialStorage::default();
        assert!(storage.store_physical(
            Material::free_base("Carbon", 1.0),
            vec![Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            &catalog(),
        ));
        assert!(storage.entries.iter().all(|entry| {
            matches!(entry, StoredMaterial::Physical(instance) if instance.is_realized())
        }));
    }

    #[test]
    fn take_matching_rejects_unknown_resource_without_removing_the_entry() {
        let mut storage = MaterialStorage::default();
        assert!(storage.store_physical(
            Material::free_base("Carbon", 1.0),
            vec![Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 }],
            &catalog(),
        ));
        let StoredMaterial::Physical(instance) = &mut storage.entries[0];
        instance.material.parts[0].0 = "Uncatalogued".to_owned();
        let before = storage.clone();

        assert!(storage
            .take_matching_physical(&Material::free_base("Uncatalogued", 1.0), &catalog())
            .is_none());
        assert_eq!(storage, before, "rejected transfer must preserve the source owner");
    }

    #[test]
    fn indexed_take_rejects_unknown_resource_without_removing_the_entry() {
        let mut storage = MaterialStorage::default();
        assert!(storage.store_physical(
            Material::free_base("Carbon", 1.0),
            vec![Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 }],
            &catalog(),
        ));
        let StoredMaterial::Physical(instance) = &mut storage.entries[0];
        instance.material.parts[0].0 = "Uncatalogued".to_owned();
        let before = storage.clone();

        assert!(storage.take_physical_at(0, &catalog()).is_none());
        assert_eq!(storage, before, "rejected indexed take must preserve ownership");
    }

    #[test]
    fn free_material_is_stored_as_discrete_units() {
        let mut storage = MaterialStorage::default();
        for x in 0..3 {
            assert!(storage.store_physical(
                Material::free_base("Carbon", 1.0),
                vec![Placement {
                    x: x as f64,
                    y: 0.0,
                    rotation_radians: 0.0,
                }],
                &catalog(),
            ));
        }
        assert_eq!(storage.len(), 3);
        assert_eq!(storage.count_unstructured(), 3);
        assert_eq!(storage.total_amount(), 3.0);
    }

    #[test]
    fn peek_does_not_consume_free_material() {
        let mut storage = MaterialStorage::default();
        assert!(storage.store_physical(
            Material::free_base("Carbon", 1.0),
            vec![Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            &catalog(),
        ));
        let peeked = storage.peek_one_unstructured().expect("stored unit");
        assert_eq!(peeked.material, Material::free_base("Carbon", 1.0));
        assert_eq!(storage.count_unstructured(), 1);
    }

    #[test]
    fn structured_material_is_stored_intact() {
        let mut storage = MaterialStorage::default();
        let m = compound();
        assert!(storage.store_physical(
            m.clone(),
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
            &catalog(),
        ));
        assert_eq!(storage.materials_snapshot(), vec![m]);
        assert_eq!(storage.count_structured(), 1);
    }

    #[test]
    fn structured_material_is_taken_intact() {
        let mut storage = MaterialStorage::default();
        let m = compound();
        assert!(storage.store_physical(
            m.clone(),
            vec![
                Placement {
                    x: 0.0,
                    y: 0.0,
                    rotation_radians: 0.0
                };
                2
            ],
            &catalog(),
        ));
        assert_eq!(
            storage.take_matching(&m, &catalog()).map(|instance| instance.material),
            Some(m.clone())
        );
        assert!(storage.is_empty());
    }

    #[test]
    fn physical_material_is_stored_in_intrinsic_frame() {
        let mut storage = MaterialStorage::default();
        let m = compound();
        let placements = vec![
            Placement {
                x: 10.0,
                y: 20.0,
                rotation_radians: 0.5,
            },
            Placement {
                x: 10.838,
                y: 20.0,
                rotation_radians: 0.75,
            },
        ];
        assert!(storage.store_physical(m.clone(), placements, &catalog()));
        assert_eq!(storage.physical_count(), 1);
        let restored = storage.take_matching_physical(&m, &catalog()).expect("stored instance");
        assert_eq!(restored.material, m);
        let intrinsic = restored.placements.expect("intrinsic realization");
        assert!(intrinsic[0].x.abs() <= 1e-12);
        assert!(intrinsic[0].y.abs() <= 1e-12);
        assert!(intrinsic[0].rotation_radians.abs() <= 1e-12);
        assert!((intrinsic[1].x.hypot(intrinsic[1].y) - 0.838).abs() <= 1e-9);
        assert!((intrinsic[1].rotation_radians - 0.25).abs() <= 1e-12);
        assert!(restored.internal_connections.is_some());
    }

    #[test]
    fn physical_single_constituent_is_stored_with_its_realization() {
        let mut storage = MaterialStorage::default();
        let m = Material::free_base("Carbon", 1.0);
        let placements = vec![Placement {
            x: 10.0,
            y: 20.0,
            rotation_radians: 0.25,
        }];
        assert!(storage.store_physical(m.clone(), placements, &catalog()));
        assert_eq!(storage.physical_count(), 1);
        let restored = storage.take_matching_physical(&m, &catalog()).expect("stored instance");
        assert_eq!(restored.material, m);
        let intrinsic = restored.placements.expect("intrinsic realization");
        assert!(intrinsic[0].x.abs() <= 1e-12);
        assert!(intrinsic[0].y.abs() <= 1e-12);
        assert!(intrinsic[0].rotation_radians.abs() <= 1e-12);
        assert!(restored.internal_connections.is_some());
    }

    #[test]
    fn storage_never_merges_independent_atoms() {
        let mut storage = MaterialStorage::default();
        assert!(storage.store_physical(
            Material::free_base("Carbon", 1.0),
            vec![Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            &catalog(),
        ));
        assert!(storage.store_physical(
            Material::free_base("Carbon", 1.0),
            vec![Placement {
                x: 1.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            &catalog(),
        ));
        assert_eq!(storage.len(), 2);
    }

    #[test]
    fn storage_never_opens_a_compound() {
        let mut storage = MaterialStorage::default();
        let m = compound();
        assert!(storage.store_physical(
            m.clone(),
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
                }
            ],
            &catalog(),
        ));
        assert!(storage.take_unstructured(1).is_none());
        assert_eq!(storage.materials_snapshot(), vec![m]);
    }

    #[test]
    fn prepared_physical_batch_preserves_owner_offsets_and_rejects_fractional_members() {
        use crate::physical_material::PhysicalMaterial;

        let catalog = catalog();
        let anchor = Placement {
            x: 20.0,
            y: 30.0,
            rotation_radians: 0.0,
        };
        let first = PhysicalMaterial::realized(
            Material::free_base("Carbon", 1.0),
            vec![Placement {
                x: 21.0,
                y: 32.0,
                rotation_radians: 0.25,
            }],
            &catalog,
        )
        .expect("first object should be realized");
        let second = PhysicalMaterial::realized(
            Material::free_base("Nitrogen", 1.0),
            vec![Placement {
                x: 24.0,
                y: 35.0,
                rotation_radians: 0.5,
            }],
            &catalog,
        )
        .expect("second object should be realized");

        let prepared = MaterialStorage::prepare_physical_batch_at_owner_anchor(
            vec![first, second],
            anchor,
            &catalog,
        )
        .expect("valid batch should prepare atomically");
        let mut storage = MaterialStorage::default();
        storage.commit_prepared_physical_batch(prepared);
        assert_eq!(storage.physical_count(), 2);
        let offsets: Vec<_> = storage
            .entries
            .iter()
            .map(|entry| match entry {
                StoredMaterial::Physical(instance) => instance.owner_relative_origin.unwrap(),
            })
            .collect();
        assert_eq!(offsets[0].x, 4.0);
        assert_eq!(offsets[0].y, 5.0);
        assert_eq!(offsets[1].x, 1.0);
        assert_eq!(offsets[1].y, 2.0);

        // Persisted/corrupt input can carry a realized-looking fractional
        // rigid constituent even though the normal constructor rejects it.
        // Batch preflight must reject this malformed later member atomically.
        let fractional = PhysicalMaterial {
            material: Material::free_base("Carbon", 1.5),
            placements: Some(vec![Placement {
                x: 1.0,
                y: 2.0,
                rotation_radians: 0.0,
            }]),
            internal_connections: Some(Vec::new()),
            owner_relative_origin: None,
            realized_shapes: None,
        };
        let valid = PhysicalMaterial::realized(
            Material::free_base("Carbon", 1.0),
            vec![Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .expect("test object should be realized");
        assert!(MaterialStorage::prepare_physical_batch_at_owner_anchor(
            vec![valid, fractional],
            anchor,
            &catalog,
        )
        .is_none());
    }

    #[test]
    fn owner_anchor_storage_rejects_non_finite_anchor_without_mutating_inventory() {
        use crate::physical_material::PhysicalMaterial;

        let instance = PhysicalMaterial::realized(
            Material::free_base("Carbon", 1.0),
            vec![Placement {
                x: 1.0,
                y: 2.0,
                rotation_radians: 0.25,
            }],
            &catalog(),
        )
        .expect("valid physical material should realize");
        let invalid_anchor = Placement {
            x: f64::NAN,
            y: 0.0,
            rotation_radians: 0.0,
        };
        assert!(
            MaterialStorage::prepare_physical_batch_at_owner_anchor(
                vec![instance.clone()],
                invalid_anchor,
                &catalog(),
            )
            .is_none(),
            "batch acquisition must reject a non-finite owner anchor"
        );

        let mut storage = MaterialStorage::default();
        assert!(
            storage
                .try_store_physical_instance_at_owner_anchor(instance, invalid_anchor, &catalog())
                .is_err(),
            "single-object acquisition must return the original source on rejection"
        );
        assert!(storage.is_empty());
    }

    #[test]
    fn unknown_resource_identity_is_rejected_at_storage_boundary() {
        let catalog = catalog();
        let mut physical = PhysicalMaterial::realized(
            Material::free_base("Carbon", 1.0),
            vec![Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .expect("Carbon should be physically realizable");
        physical.material.parts[0].0 = "Unobtainium".to_owned();

        let mut storage = MaterialStorage::default();
        assert!(!storage.store_physical_instance(physical, &catalog));
        assert!(storage.entries.is_empty());
    }

    #[test]
    fn fractional_material_is_rejected_at_storage_boundary() {
        let mut storage = MaterialStorage::default();
        assert!(!storage.store_physical(
            Material::free_base("Carbon", 1.5),
            vec![Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            &catalog(),
        ));
        assert!(storage.is_empty());
    }

    #[test]
    fn stored_split_batch_preflight_preserves_owner_relative_anchors() {
        use crate::physical_material::PhysicalMaterial;

        let mut source = PhysicalMaterial::realized(
            Material::free_base("Carbon", 1.0),
            vec![Placement {
                x: 8.0,
                y: 12.0,
                rotation_radians: 0.4,
            }],
            &catalog(),
        )
        .expect("split piece should be realized");
        source.owner_relative_origin = Some(Placement {
            x: 3.0,
            y: -2.0,
            rotation_radians: 0.7,
        });

        let prepared = MaterialStorage::prepare_physical_batch(vec![source], &catalog())
            .expect("valid stored split should prepare");
        let StoredMaterial::Physical(piece) = &prepared[0];
        let placement = piece.placements.as_ref().unwrap()[0];
        assert!(placement.x.abs() < 1e-12 && placement.y.abs() < 1e-12);
        let anchor = piece.owner_relative_origin.unwrap();
        assert_eq!(anchor.x, 3.0);
        assert_eq!(anchor.y, -2.0);
        assert_eq!(anchor.rotation_radians, 0.7);
    }
}
