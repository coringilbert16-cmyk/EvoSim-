#![expect(dead_code, reason = "Staged API retained for subsystem integration")]
use crate::connection_geometry::WorldConnectionPoint;
use crate::physical_geometry::PhysicalGeometry;
use crate::resources::{BaseResource, Material};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::{HashMap, HashSet};
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    pub x: f64,
    pub y: f64,
    pub rotation_radians: f64,
}
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct PhysicalConstituentId(pub u64);
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct StructuralUnit {
    pub physical_id: PhysicalConstituentId,
    pub material: Material,
    pub placement: Placement,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geometry: Option<PhysicalGeometry>,
}
impl StructuralUnit {
    pub fn new(resource_name: impl Into<String>, placement: Placement) -> Self {
        Self {
            physical_id: PhysicalConstituentId(0),
            material: Material::free_base(resource_name, 1.0),
            placement,
            geometry: None,
        }
    }
    pub fn from_material(material: Material, placement: Placement) -> Option<Self> {
        if material.parts.len() != 1
            || !material.is_valid()
            || material.is_empty()
            || material.has_internal_structure()
        {
            return None;
        }
        Some(Self {
            physical_id: PhysicalConstituentId(0),
            material,
            placement,
            geometry: None,
        })
    }
    pub fn properties(
        &self,
        catalog: &[BaseResource],
    ) -> Option<crate::resources::ResourceProperties> {
        if !self.material.is_valid_for_catalog(catalog) {
            return None;
        }
        Some(self.material.weighted_properties(catalog))
    }
    fn external_geometry_resource<'a>(
        &self,
        catalog: &'a [BaseResource],
    ) -> Option<&'a BaseResource> {
        if self.material.has_internal_structure() {
            return None;
        }
        let (name, amount) = self.material.parts.first()?;
        if (*amount - 1.0).abs() > f64::EPSILON {
            return None;
        }
        catalog.iter().find(|base| base.name == *name)
    }
    pub fn shape<'a>(&'a self, catalog: &'a [BaseResource]) -> Option<&'a crate::resources::Shape> {
        if let Some(geometry) = &self.geometry {
            return Some(geometry.shape());
        }
        if !self.material.has_internal_structure() {
            let [(name, amount)] = self.material.parts.as_slice() else {
                return None;
            };
            if (*amount - 1.0).abs() > f64::EPSILON {
                return None;
            }
            return catalog
                .iter()
                .find(|base| base.name == *name)
                .map(|base| &base.shape);
        }
        self.external_geometry_resource(catalog)
            .map(|base| &base.shape)
    }
    pub fn realize_default_geometry(&mut self, catalog: &[BaseResource]) -> bool {
        if self.geometry.is_some() {
            return true;
        }
        if let [(name, amount)] = self.material.parts.as_slice() {
            if *amount > 1.0 && (*amount - amount.round()).abs() <= 1e-9 {
                let Some(resource) = catalog.iter().find(|resource| resource.name == *name) else {
                    return false;
                };
                if resource.physical_state != crate::resources::PhysicalState::Fluid
                    || !self.material.is_valid_for_catalog(catalog)
                {
                    return false;
                }
                self.geometry = Some(PhysicalGeometry::from_default(&resource.shape));
                return true;
            }
        }
        let Some(shape) = self.shape(catalog).cloned() else {
            return false;
        };
        self.geometry = Some(PhysicalGeometry::from_default(&shape));
        true
    }

    /// Re-realize a connected fluid constituent into geometry supplied by the
    /// surrounding structural context. Rigid resources cannot use this path.
    pub fn realize_fluid_geometry(
        &mut self,
        shape: crate::resources::Shape,
        catalog: &[BaseResource],
    ) -> bool {
        let Some((name, amount)) = self.material.parts.first() else {
            return false;
        };
        if self.material.parts.len() != 1
            || self.material.has_internal_structure()
            || !amount.is_finite()
            || *amount <= 0.0
        {
            return false;
        }
        let Some(resource) = catalog.iter().find(|resource| resource.name == *name) else {
            return false;
        };
        if resource.physical_state != crate::resources::PhysicalState::Fluid {
            return false;
        }
        let default_area = match &resource.shape.form {
            crate::resources::Form::Circle { radius } => std::f64::consts::PI * radius * radius,
            crate::resources::Form::Fluid { nominal_area, .. } => *nominal_area,
            _ => return false,
        };
        let realized_area = match &shape.form {
            crate::resources::Form::Fluid {
                nominal_area,
                boundary: Some(_),
                ..
            }
            | crate::resources::Form::Fluid {
                nominal_area,
                arc_boundary: Some(_),
                ..
            }
            | crate::resources::Form::Fluid {
                nominal_area,
                multi_contact_boundary: Some(_),
                ..
            } => *nominal_area,
            _ => return false,
        };
        let expected_area = default_area * *amount;
        if !expected_area.is_finite()
            || (realized_area - expected_area).abs() > 1e-9 * expected_area.max(1.0)
        {
            return false;
        }
        if self.geometry.is_none() {
            self.geometry = Some(PhysicalGeometry::from_default(&resource.shape));
        }
        let Some(geometry) = self.geometry.as_mut() else {
            return false;
        };
        geometry.replace_fluid_realization(shape)
    }
}
impl<'de> Deserialize<'de> for StructuralUnit {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct LegacyStructuralMaterial {
            material: Material,
        }
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum StoredMaterial {
            Direct(Material),
            Wrapped(LegacyStructuralMaterial),
        }
        #[derive(Deserialize)]
        struct Stored {
            #[serde(default)]
            physical_id: PhysicalConstituentId,
            material: Option<StoredMaterial>,
            resource_name: Option<String>,
            placement: Placement,
            #[serde(default)]
            geometry: Option<PhysicalGeometry>,
        }
        let s = Stored::deserialize(deserializer)?;
        let m = match s.material {
            Some(StoredMaterial::Direct(m)) => m,
            Some(StoredMaterial::Wrapped(w)) => w.material,
            None => Material::free_base(
                s.resource_name
                    .ok_or_else(|| serde::de::Error::custom("structural unit has no material"))?,
                1.0,
            ),
        };
        if !m.is_valid() || m.is_empty() {
            return Err(serde::de::Error::custom("invalid material"));
        }
        if m.parts.len() != 1 || m.has_internal_structure() {
            return Err(serde::de::Error::custom("composite material cannot be loaded as a StructuralUnit; restore its physical constituents and bonds through the organism graph"));
        }
        Ok(Self {
            physical_id: s.physical_id,
            material: m,
            placement: s.placement,
            geometry: s.geometry,
        })
    }
}
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum ConnectionEndpoint {
    Corner { point_index: usize },
    LineEndpoint { point_index: usize },
    Boundary { angle_radians: f64 },
    Fluid { x: f64, y: f64 },
}
#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
pub struct BondEndpoint {
    pub constituent_id: PhysicalConstituentId,
    pub location: ConnectionEndpoint,
}
impl BondEndpoint {
    pub fn new(constituent_id: PhysicalConstituentId, location: ConnectionEndpoint) -> Self {
        Self {
            constituent_id,
            location,
        }
    }
}
impl<'de> Deserialize<'de> for BondEndpoint {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Stored {
            constituent_id: Option<PhysicalConstituentId>,
            unit_index: Option<usize>,
            location: ConnectionEndpoint,
        }
        let s = Stored::deserialize(deserializer)?;
        let id = s.constituent_id.unwrap_or_else(|| {
            PhysicalConstituentId(s.unit_index.map(|i| u64::MAX - i as u64).unwrap_or(0))
        });
        Ok(Self {
            constituent_id: id,
            location: s.location,
        })
    }
}
impl ConnectionEndpoint {
    pub fn world_point(
        self,
        unit: &StructuralUnit,
        catalog: &[BaseResource],
    ) -> Option<WorldConnectionPoint> {
        match self {
            Self::Corner { point_index } => {
                let shape = unit.shape(catalog)?;
                crate::connection_geometry::transform_polygon_vertex(
                    shape,
                    point_index,
                    unit.placement.x,
                    unit.placement.y,
                    unit.placement.rotation_radians,
                )
            }
            Self::LineEndpoint { point_index } => {
                let shape = unit.shape(catalog)?;
                crate::connection_geometry::transform_line_endpoint(
                    shape,
                    point_index,
                    unit.placement.x,
                    unit.placement.y,
                    unit.placement.rotation_radians,
                )
            }
            Self::Boundary { angle_radians } => {
                let shape = unit.shape(catalog)?;
                let (nx, ny) = (angle_radians.cos(), angle_radians.sin());
                let point = crate::surface_geometry::boundary_point_toward(shape, nx, ny)?;
                Some(crate::connection_geometry::transform_derived_point(
                    point.x,
                    point.y,
                    point.normal_x,
                    point.normal_y,
                    unit.placement.x,
                    unit.placement.y,
                    unit.placement.rotation_radians,
                ))
            }
            Self::Fluid { x, y } => Some(crate::connection_geometry::transform_derived_point(
                x,
                y,
                0.0,
                0.0,
                unit.placement.x,
                unit.placement.y,
                unit.placement.rotation_radians,
            )),
        }
    }
    pub fn same_location(self, other: Self) -> bool {
        match (self, other) {
            (Self::Corner { point_index: a }, Self::Corner { point_index: b }) => a == b,
            (Self::LineEndpoint { point_index: a }, Self::LineEndpoint { point_index: b }) => {
                a == b
            }
            (Self::Boundary { angle_radians: a }, Self::Boundary { angle_radians: b }) => {
                (a - b).abs() <= 1e-12
            }
            (Self::Fluid { x: ax, y: ay }, Self::Fluid { x: bx, y: by }) => {
                (ax - bx).hypot(ay - by) <= 1e-12
            }
            _ => false,
        }
    }
}
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Bond {
    pub endpoint_a: BondEndpoint,
    pub endpoint_b: BondEndpoint,
    pub strength: f64,
    #[serde(default)]
    pub bond_energy: f64,
}
impl Bond {
    pub fn touches(&self, u: PhysicalConstituentId, location: ConnectionEndpoint) -> bool {
        (self.endpoint_a.constituent_id == u && self.endpoint_a.location.same_location(location))
            || (self.endpoint_b.constituent_id == u
                && self.endpoint_b.location.same_location(location))
    }
    pub fn has_same_identity(&self, o: &Bond) -> bool {
        (self.endpoint_a.constituent_id == o.endpoint_a.constituent_id
            && self
                .endpoint_a
                .location
                .same_location(o.endpoint_a.location)
            && self.endpoint_b.constituent_id == o.endpoint_b.constituent_id
            && self
                .endpoint_b
                .location
                .same_location(o.endpoint_b.location))
            || (self.endpoint_a.constituent_id == o.endpoint_b.constituent_id
                && self
                    .endpoint_a
                    .location
                    .same_location(o.endpoint_b.location)
                && self.endpoint_b.constituent_id == o.endpoint_a.constituent_id
                && self
                    .endpoint_b
                    .location
                    .same_location(o.endpoint_a.location))
    }
    pub fn is_valid(&self, connection_is_valid: impl Fn(BondEndpoint) -> bool) -> bool {
        self.endpoint_a.constituent_id.0 != 0
            && self.endpoint_b.constituent_id.0 != 0
            && self.endpoint_a.constituent_id != self.endpoint_b.constituent_id
            && self.bond_energy.is_finite()
            && self.bond_energy >= 0.0
            && connection_is_valid(self.endpoint_a)
            && connection_is_valid(self.endpoint_b)
    }
}
fn units_strictly_overlap(
    a: &StructuralUnit,
    b: &StructuralUnit,
    catalog: &[BaseResource],
) -> bool {
    let (Some(form_a), Some(form_b)) = (a.shape(catalog), b.shape(catalog)) else {
        return false;
    };
    let pa = crate::material_geometry::PlacedMaterialPart {
        part_index: 0,
        form: form_a.form.clone(),
        placement: a.placement,
    };
    let pb = crate::material_geometry::PlacedMaterialPart {
        part_index: 1,
        form: form_b.form.clone(),
        placement: b.placement,
    };
    crate::material_geometry::placed_forms_penetrate(&pa, &pb, 1e-9)
}

#[derive(Serialize, Clone, Debug)]
pub struct PhysicalConstituentGraph {
    pub units: Vec<StructuralUnit>,
    pub bonds: Vec<Bond>,
    /// Stable physical constituents that established the primary genome cavity.
    /// Structural membership is derived from connectivity to these IDs.
    #[serde(default)]
    genome_constituent_ids: Vec<PhysicalConstituentId>,
    /// Additional independently identified genome cores in a shared physical
    /// aggregate. These are metadata only: each constituent still exists once
    /// in `units`, and bonds still refer to its single stable physical ID.
    #[serde(default)]
    additional_genome_constituent_groups: Vec<Vec<PhysicalConstituentId>>,
    #[serde(default)]
    next_constituent_id: u64,
}
impl<'de> Deserialize<'de> for PhysicalConstituentGraph {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Stored {
            units: Vec<StructuralUnit>,
            bonds: Vec<Bond>,
            #[serde(default)]
            genome_constituent_ids: Vec<PhysicalConstituentId>,
            #[serde(default)]
            additional_genome_constituent_groups: Vec<Vec<PhysicalConstituentId>>,
            #[serde(default)]
            next_constituent_id: u64,
        }
        let mut s = Stored::deserialize(deserializer)?;
        let mut used = HashSet::new();
        let mut next = s.next_constituent_id.max(1);
        for unit in &mut s.units {
            let id = unit.physical_id;
            if id.0 == 0 || used.contains(&id) {
                while next == 0 || used.contains(&PhysicalConstituentId(next)) {
                    next = next.checked_add(1).ok_or_else(|| {
                        serde::de::Error::custom("physical constituent ID space exhausted")
                    })?
                }
                unit.physical_id = PhysicalConstituentId(next);
                used.insert(unit.physical_id);
                next = next.saturating_add(1)
            } else {
                used.insert(id);
            }
        }
        if let Some(max_id) = used.iter().map(|id| id.0).max() {
            next = next.max(max_id.saturating_add(1)).max(1)
        }
        for bond in &mut s.bonds {
            for endpoint in [&mut bond.endpoint_a, &mut bond.endpoint_b] {
                if endpoint.constituent_id.0 >= u64::MAX.saturating_sub(s.units.len() as u64) {
                    let legacy_index = (u64::MAX - endpoint.constituent_id.0) as usize;
                    if let Some(id) = s.units.get(legacy_index).map(|u| u.physical_id) {
                        endpoint.constituent_id = id
                    } else {
                        return Err(serde::de::Error::custom(
                            "bond references missing legacy unit",
                        ));
                    }
                }
            }
            if !s
                .units
                .iter()
                .any(|u| u.physical_id == bond.endpoint_a.constituent_id)
                || !s
                    .units
                    .iter()
                    .any(|u| u.physical_id == bond.endpoint_b.constituent_id)
            {
                return Err(serde::de::Error::custom(
                    "bond references missing physical constituent",
                ));
            }
        }
        let mut genome_constituent_ids: Vec<_> = s
            .genome_constituent_ids
            .into_iter()
            .filter(|id| id.0 != 0 && used.contains(id))
            .collect();
        genome_constituent_ids.sort_unstable_by_key(|id| id.0);
        genome_constituent_ids.dedup();
        let primary_ids: HashSet<_> = genome_constituent_ids.iter().copied().collect();
        let mut assigned_ids = primary_ids;
        let additional_genome_constituent_groups = s
            .additional_genome_constituent_groups
            .into_iter()
            .map(|group| {
                let mut cleaned: Vec<_> = group
                    .into_iter()
                    .filter(|id| id.0 != 0 && used.contains(id) && assigned_ids.insert(*id))
                    .collect();
                cleaned.sort_unstable_by_key(|id| id.0);
                cleaned
            })
            .filter(|group| !group.is_empty())
            .collect();
        Ok(Self {
            units: s.units,
            bonds: s.bonds,
            genome_constituent_ids,
            additional_genome_constituent_groups,
            next_constituent_id: next,
        })
    }
}
impl PhysicalConstituentGraph {
    pub fn new() -> Self {
        Self {
            units: Vec::new(),
            bonds: Vec::new(),
            genome_constituent_ids: Vec::new(),
            additional_genome_constituent_groups: Vec::new(),
            next_constituent_id: 1,
        }
    }
    pub(crate) fn push_bond_unchecked(&mut self, b: Bond) -> usize {
        self.bonds.push(b);
        self.bonds.len() - 1
    }
    pub fn add_unit(&mut self, mut u: StructuralUnit) -> usize {
        let index = self.units.len();
        let id = PhysicalConstituentId(self.next_constituent_id);
        self.next_constituent_id = self.next_constituent_id.saturating_add(1);
        u.physical_id = id;
        self.units.push(u);
        index
    }

    /// Append another physical graph without aliasing its graph-local IDs.
    ///
    /// The source graph is validated and the complete result is staged on a
    /// clone before this graph is changed. Geometry/contact eligibility is the
    /// caller's responsibility; this operation only preserves physical
    /// constituents, bond endpoints/energy, and genome-core identity metadata.
    /// The destination's primary core stays primary; the source's primary core
    /// becomes an additional core in the combined aggregate.
    pub(crate) fn append_graph_remapping_ids(
        &mut self,
        source: &Self,
    ) -> Option<HashMap<PhysicalConstituentId, PhysicalConstituentId>> {
        let source_ids: HashSet<_> = source.units.iter().map(|unit| unit.physical_id).collect();
        let destination_ids: HashSet<_> = self.units.iter().map(|unit| unit.physical_id).collect();
        let source_max_id = source_ids.iter().map(|id| id.0).max().unwrap_or(0);
        let destination_max_id = destination_ids.iter().map(|id| id.0).max().unwrap_or(0);
        if source_ids.len() != source.units.len()
            || source_ids.contains(&PhysicalConstituentId(0))
            || source.next_constituent_id <= source_max_id
            || destination_ids.len() != self.units.len()
            || destination_ids.contains(&PhysicalConstituentId(0))
            || self.next_constituent_id <= destination_max_id
            || self
                .next_constituent_id
                .checked_add(source.units.len() as u64)
                .is_none()
        {
            return None;
        }
        let valid_bonds = |bonds: &[Bond], ids: &HashSet<PhysicalConstituentId>| {
            !bonds.iter().any(|bond| {
                bond.endpoint_a.constituent_id == bond.endpoint_b.constituent_id
                    || !ids.contains(&bond.endpoint_a.constituent_id)
                    || !ids.contains(&bond.endpoint_b.constituent_id)
                    || !bond.strength.is_finite()
                    || !(0.0..=1.0).contains(&bond.strength)
                    || !bond.bond_energy.is_finite()
                    || bond.bond_energy < 0.0
            })
        };
        if !valid_bonds(&self.bonds, &destination_ids) || !valid_bonds(&source.bonds, &source_ids) {
            return None;
        }
        let valid_core_metadata =
            |primary_ids: &[PhysicalConstituentId],
             groups: &[Vec<PhysicalConstituentId>],
             ids: &HashSet<PhysicalConstituentId>| {
                let primary: HashSet<_> = primary_ids.iter().copied().collect();
                if primary.len() != primary_ids.len() || !primary.is_subset(ids) {
                    return false;
                }
                let mut assigned = primary;
                groups.iter().all(|group| {
                    let group_ids: HashSet<_> = group.iter().copied().collect();
                    let valid = !group.is_empty()
                        && group_ids.len() == group.len()
                        && group_ids.is_subset(ids)
                        && assigned.is_disjoint(&group_ids);
                    if valid {
                        assigned.extend(group_ids);
                    }
                    valid
                })
            };
        if !valid_core_metadata(
            &self.genome_constituent_ids,
            &self.additional_genome_constituent_groups,
            &destination_ids,
        ) || !valid_core_metadata(
            &source.genome_constituent_ids,
            &source.additional_genome_constituent_groups,
            &source_ids,
        ) {
            return None;
        }

        let mut staged = self.clone();
        let mut remap = HashMap::with_capacity(source.units.len());
        for unit in &source.units {
            let old_id = unit.physical_id;
            let new_index = staged.add_unit(unit.clone());
            let new_id = staged.physical_id(new_index)?;
            if remap.insert(old_id, new_id).is_some() {
                return None;
            }
        }
        for bond in &source.bonds {
            let mut remapped = *bond;
            remapped.endpoint_a.constituent_id = *remap.get(&bond.endpoint_a.constituent_id)?;
            remapped.endpoint_b.constituent_id = *remap.get(&bond.endpoint_b.constituent_id)?;
            staged.bonds.push(remapped);
        }

        let remap_group = |ids: &[PhysicalConstituentId]| -> Option<Vec<PhysicalConstituentId>> {
            ids.iter().map(|id| remap.get(id).copied()).collect()
        };
        let source_primary = remap_group(&source.genome_constituent_ids)?;
        let source_additional = source
            .additional_genome_constituent_groups
            .iter()
            .map(|group| remap_group(group))
            .collect::<Option<Vec<_>>>()?;
        if staged.genome_constituent_ids.is_empty() && !source_primary.is_empty() {
            staged.set_genome_constituent_ids(source_primary);
        } else if !source_primary.is_empty() {
            staged.add_genome_constituent_group(source_primary);
        }
        for group in source_additional {
            staged.add_genome_constituent_group(group);
        }

        *self = staged;
        Some(remap)
    }
    /// Extract a graph-local view of selected physical constituents without
    /// changing their stable IDs. Only bonds wholly inside the selection are
    /// included; cross-boundary bonds remain owned by the source aggregate.
    /// The caller supplies the selected core identity to promote to primary
    /// identity in the view (normally the developing organism's core group).
    pub(crate) fn subgraph_for_physical_ids(
        &self,
        ids: &[PhysicalConstituentId],
        primary_core_ids: &[PhysicalConstituentId],
    ) -> Option<Self> {
        let source_ids: HashSet<_> = self.units.iter().map(|unit| unit.physical_id).collect();
        let source_max_id = source_ids.iter().map(|id| id.0).max().unwrap_or(0);
        let selected: HashSet<_> = ids.iter().copied().collect();
        let primary: HashSet<_> = primary_core_ids.iter().copied().collect();
        if ids.is_empty()
            || source_ids.len() != self.units.len()
            || source_ids.contains(&PhysicalConstituentId(0))
            || self.next_constituent_id == 0
            || self.next_constituent_id <= source_max_id
            || self.bonds.iter().any(|bond| {
                !source_ids.contains(&bond.endpoint_a.constituent_id)
                    || !source_ids.contains(&bond.endpoint_b.constituent_id)
                    || !bond.strength.is_finite()
                    || !(0.0..=1.0).contains(&bond.strength)
                    || !bond.bond_energy.is_finite()
                    || bond.bond_energy < 0.0
            })
            || selected.len() != ids.len()
            || selected.contains(&PhysicalConstituentId(0))
            || !selected.is_subset(&source_ids)
            || primary.len() != primary_core_ids.len()
            || primary.iter().any(|id| !selected.contains(id))
        {
            return None;
        }

        let source_primary: HashSet<_> = self.genome_constituent_ids.iter().copied().collect();
        let mut source_core_ids = source_primary.clone();
        let source_core_metadata_valid = source_primary.len() == self.genome_constituent_ids.len()
            && source_primary.is_subset(&source_ids)
            && self
                .additional_genome_constituent_groups
                .iter()
                .all(|group| {
                    let group_ids: HashSet<_> = group.iter().copied().collect();
                    let valid = !group.is_empty()
                        && group_ids.len() == group.len()
                        && group_ids.is_subset(&source_ids)
                        && source_core_ids.is_disjoint(&group_ids);
                    if valid {
                        source_core_ids.extend(group_ids);
                    }
                    valid
                });
        if !source_core_metadata_valid {
            return None;
        }
        if !source_primary.is_disjoint(&selected)
            && (source_primary != primary || !source_primary.is_subset(&selected))
        {
            return None;
        }

        let mut retained_groups = Vec::new();
        let mut primary_identity_found = primary.is_empty() || source_primary == primary;
        for group in &self.additional_genome_constituent_groups {
            let group_ids: HashSet<_> = group.iter().copied().collect();
            if group_ids.is_disjoint(&selected) {
                continue;
            }
            if !group_ids.is_subset(&selected) {
                return None;
            }
            if group_ids == primary {
                primary_identity_found = true;
            } else if !group_ids.is_disjoint(&primary) {
                return None;
            } else {
                retained_groups.push(group.clone());
            }
        }
        if !primary_identity_found {
            return None;
        }

        let max_id = selected.iter().map(|id| id.0).max().unwrap_or(0);
        if self.next_constituent_id <= max_id || self.next_constituent_id == 0 {
            return None;
        }
        let mut graph = Self::new();
        graph.units = self
            .units
            .iter()
            .filter(|unit| selected.contains(&unit.physical_id))
            .cloned()
            .collect();
        graph.bonds = self
            .bonds
            .iter()
            .filter(|bond| {
                selected.contains(&bond.endpoint_a.constituent_id)
                    && selected.contains(&bond.endpoint_b.constituent_id)
            })
            .copied()
            .collect();
        graph.genome_constituent_ids = primary_core_ids.to_vec();
        graph.genome_constituent_ids.sort_unstable_by_key(|id| id.0);
        graph.additional_genome_constituent_groups = retained_groups;
        graph.next_constituent_id = self.next_constituent_id;
        Some(graph)
    }

    /// Partition one complete attached genome aggregate out of this graph.
    ///
    /// The selected constituents keep their stable physical IDs in the returned
    /// child graph. Bonds wholly inside the selection move with the child; bonds
    /// wholly outside remain with the parent; cross-boundary bonds are removed
    /// from both graphs and their stored energy is returned for explicit ledger
    /// settlement by the caller. No graph state changes unless both resulting
    /// graphs can be constructed and validated.
    pub(crate) fn partition_subgraph_for_division(
        &mut self,
        selected_ids: &[PhysicalConstituentId],
        child_primary_core_ids: &[PhysicalConstituentId],
    ) -> Option<(Self, f64)> {
        let selected: HashSet<_> = selected_ids.iter().copied().collect();
        let child_primary: HashSet<_> = child_primary_core_ids.iter().copied().collect();
        let parent_primary: HashSet<_> = self.genome_constituent_ids.iter().copied().collect();
        if selected_ids.is_empty()
            || selected.len() != selected_ids.len()
            || child_primary.len() != child_primary_core_ids.len()
            || !child_primary.is_subset(&selected)
            || !parent_primary.is_disjoint(&selected)
            || child_primary_core_ids.is_empty()
            || !self
                .additional_genome_constituent_groups
                .iter()
                .any(|group| group.iter().copied().collect::<HashSet<_>>() == child_primary)
            || self
                .additional_genome_constituent_groups
                .iter()
                .any(|group| {
                    let group_ids: HashSet<_> = group.iter().copied().collect();
                    !group_ids.is_disjoint(&selected) && !group_ids.is_subset(&selected)
                })
        {
            return None;
        }

        // Build the offspring view first. This validates source IDs, bonds,
        // complete core groups, and the requested child-core identity.
        let child = self.subgraph_for_physical_ids(selected_ids, child_primary_core_ids)?;
        // Division creates exactly one new organism. A selected graph that
        // still contains another genome identity is not a valid offspring.
        if !child.additional_genome_constituent_groups.is_empty() {
            return None;
        }
        let source_ids: HashSet<_> = self.units.iter().map(|unit| unit.physical_id).collect();
        let crossing_energy = self
            .bonds
            .iter()
            .filter(|bond| {
                selected.contains(&bond.endpoint_a.constituent_id)
                    != selected.contains(&bond.endpoint_b.constituent_id)
            })
            .map(|bond| bond.bond_energy)
            .sum::<f64>();
        if !crossing_energy.is_finite() || crossing_energy < 0.0 {
            return None;
        }

        let mut parent = self.clone();
        parent
            .units
            .retain(|unit| !selected.contains(&unit.physical_id));
        parent.bonds.retain(|bond| {
            !selected.contains(&bond.endpoint_a.constituent_id)
                && !selected.contains(&bond.endpoint_b.constituent_id)
        });
        parent
            .additional_genome_constituent_groups
            .retain(|group| group.iter().all(|id| !selected.contains(id)));
        let parent_ids = parent.physical_constituent_ids();
        if parent.units.is_empty()
            || parent_primary.is_empty()
            || !parent_primary.is_subset(&source_ids)
            || !parent_primary.is_subset(&parent_ids.iter().copied().collect())
            || parent
                .subgraph_for_physical_ids(&parent_ids, &self.genome_constituent_ids)
                .is_none()
        {
            return None;
        }

        *self = parent;
        Some((child, crossing_energy))
    }

    /// Atomically replace one attached subgraph while retaining the
    /// aggregate's cross-boundary bonds and the destination's primary genome.
    /// Existing selected IDs may be preserved by the replacement; any new IDs
    /// must be unused in the aggregate. The caller owns geometry/contact checks.
    /// Replace a selected physical subgraph while assigning aggregate-global IDs
    /// to any newly developed constituents. Existing selected IDs are preserved;
    /// IDs allocated by a transient construction graph are never allowed to alias
    /// unrelated constituents already owned by this aggregate.
    pub(crate) fn replace_subgraph_remapping_new_ids(
        &mut self,
        old_ids: &[PhysicalConstituentId],
        replacement: &Self,
    ) -> Option<HashMap<PhysicalConstituentId, PhysicalConstituentId>> {
        let selected: HashSet<_> = old_ids.iter().copied().collect();
        if old_ids.is_empty()
            || selected.len() != old_ids.len()
            || selected.contains(&PhysicalConstituentId(0))
            || selected
                .iter()
                .any(|id| self.unit_index(*id).is_none())
        {
            return None;
        }

        let mut staged_replacement = replacement.clone();
        let replacement_ids: HashSet<_> = staged_replacement
            .units
            .iter()
            .map(|unit| unit.physical_id)
            .collect();
        if replacement_ids.len() != staged_replacement.units.len()
            || replacement_ids.contains(&PhysicalConstituentId(0))
        {
            return None;
        }

        let mut next_id = self.next_constituent_id;
        if next_id == 0 {
            return None;
        }
        let mut remapping = HashMap::new();
        for unit in &mut staged_replacement.units {
            let old_id = unit.physical_id;
            let new_id = if selected.contains(&old_id) {
                old_id
            } else {
                let assigned = PhysicalConstituentId(next_id);
                next_id = next_id.checked_add(1)?;
                assigned
            };
            remapping.insert(old_id, new_id);
            unit.physical_id = new_id;
        }
        let remap = |id: &mut PhysicalConstituentId| -> Option<()> {
            *id = *remapping.get(id)?;
            Some(())
        };
        for bond in &mut staged_replacement.bonds {
            remap(&mut bond.endpoint_a.constituent_id)?;
            remap(&mut bond.endpoint_b.constituent_id)?;
        }
        for id in &mut staged_replacement.genome_constituent_ids {
            remap(id)?;
        }
        for group in &mut staged_replacement.additional_genome_constituent_groups {
            for id in group {
                remap(id)?;
            }
        }
        staged_replacement.next_constituent_id = next_id;

        let mut staged_aggregate = self.clone();
        staged_aggregate.replace_subgraph_for_physical_ids(old_ids, &staged_replacement)?;
        *self = staged_aggregate;
        Some(remapping)
    }

    pub(crate) fn replace_subgraph_for_physical_ids(
        &mut self,
        old_ids: &[PhysicalConstituentId],
        replacement: &Self,
    ) -> Option<()> {
        let old: HashSet<_> = old_ids.iter().copied().collect();
        let destination_ids: HashSet<_> = self.units.iter().map(|unit| unit.physical_id).collect();
        let replacement_ids: HashSet<_> = replacement
            .units
            .iter()
            .map(|unit| unit.physical_id)
            .collect();
        let destination_max_id = destination_ids.iter().map(|id| id.0).max().unwrap_or(0);
        let replacement_max_id = replacement_ids.iter().map(|id| id.0).max().unwrap_or(0);
        let destination_primary_ids: HashSet<_> =
            self.genome_constituent_ids.iter().copied().collect();
        let mut destination_core_ids = destination_primary_ids.clone();
        let destination_core_metadata_valid = destination_primary_ids.len()
            == self.genome_constituent_ids.len()
            && destination_primary_ids.is_subset(&destination_ids)
            && self
                .additional_genome_constituent_groups
                .iter()
                .all(|group| {
                    let group_ids: HashSet<_> = group.iter().copied().collect();
                    let valid = !group.is_empty()
                        && group_ids.len() == group.len()
                        && group_ids.is_subset(&destination_ids)
                        && destination_core_ids.is_disjoint(&group_ids);
                    if valid {
                        destination_core_ids.extend(group_ids);
                    }
                    valid
                });
        if old_ids.is_empty()
            || old.len() != old_ids.len()
            || old.contains(&PhysicalConstituentId(0))
            || old.iter().any(|id| !destination_ids.contains(id))
            || destination_ids.len() != self.units.len()
            || destination_ids.contains(&PhysicalConstituentId(0))
            || self.next_constituent_id == 0
            || self.next_constituent_id <= destination_max_id
            || !destination_core_metadata_valid
            || self.bonds.iter().any(|bond| {
                bond.endpoint_a.constituent_id == bond.endpoint_b.constituent_id
                    || !destination_ids.contains(&bond.endpoint_a.constituent_id)
                    || !destination_ids.contains(&bond.endpoint_b.constituent_id)
                    || !bond.strength.is_finite()
                    || !(0.0..=1.0).contains(&bond.strength)
                    || !bond.bond_energy.is_finite()
                    || bond.bond_energy < 0.0
            })
            || replacement_ids.len() != replacement.units.len()
            || replacement_ids.contains(&PhysicalConstituentId(0))
            || replacement.next_constituent_id == 0
            || replacement.next_constituent_id <= replacement_max_id
            || replacement_ids.iter().any(|id| {
                !old.contains(id)
                    && (destination_ids.contains(id) || id.0 < self.next_constituent_id)
            })
            || replacement.units.iter().any(|unit| {
                !unit.placement.x.is_finite()
                    || !unit.placement.y.is_finite()
                    || !unit.placement.rotation_radians.is_finite()
            })
            || replacement.bonds.iter().any(|bond| {
                bond.endpoint_a.constituent_id == bond.endpoint_b.constituent_id
                    || !replacement_ids.contains(&bond.endpoint_a.constituent_id)
                    || !replacement_ids.contains(&bond.endpoint_b.constituent_id)
                    || !bond.strength.is_finite()
                    || !(0.0..=1.0).contains(&bond.strength)
                    || !bond.bond_energy.is_finite()
                    || bond.bond_energy < 0.0
            })
        {
            return None;
        }

        // A primary genome cannot be partially replaced by an attached graph.
        let primary_ids = destination_primary_ids;
        if !primary_ids.is_disjoint(&old) {
            return None;
        }
        for group in &self.additional_genome_constituent_groups {
            let group_ids: HashSet<_> = group.iter().copied().collect();
            if !group_ids.is_disjoint(&old) && !group_ids.is_subset(&old) {
                return None;
            }
        }
        // Preserve cross-boundary bonds only when the replacement still owns
        // the selected endpoint; otherwise the replacement would leave a
        // dangling physical bond in the aggregate.
        if self.bonds.iter().any(|bond| {
            let a_selected = old.contains(&bond.endpoint_a.constituent_id);
            let b_selected = old.contains(&bond.endpoint_b.constituent_id);
            a_selected != b_selected
                && !(if a_selected {
                    replacement_ids.contains(&bond.endpoint_a.constituent_id)
                } else {
                    replacement_ids.contains(&bond.endpoint_b.constituent_id)
                })
        }) {
            return None;
        }

        let replacement_primary: HashSet<_> =
            replacement.genome_constituent_ids.iter().copied().collect();
        if replacement_primary.len() != replacement.genome_constituent_ids.len()
            || !replacement_primary.is_subset(&replacement_ids)
        {
            return None;
        }
        let mut replacement_core_ids = replacement_primary.clone();
        for group in &replacement.additional_genome_constituent_groups {
            let group_ids: HashSet<_> = group.iter().copied().collect();
            if group.is_empty()
                || group_ids.len() != group.len()
                || !group_ids.is_subset(&replacement_ids)
                || !replacement_core_ids.is_disjoint(&group_ids)
            {
                return None;
            }
            replacement_core_ids.extend(group_ids);
        }

        let mut staged = self.clone();
        staged.units.retain(|unit| !old.contains(&unit.physical_id));
        staged.units.extend(replacement.units.iter().cloned());
        staged.bonds.retain(|bond| {
            !(old.contains(&bond.endpoint_a.constituent_id)
                && old.contains(&bond.endpoint_b.constituent_id))
        });
        staged.bonds.extend(replacement.bonds.iter().copied());
        staged
            .additional_genome_constituent_groups
            .retain(|group| group.iter().all(|id| !old.contains(id)));
        if staged.genome_constituent_ids.is_empty()
            && !replacement.genome_constituent_ids.is_empty()
        {
            staged.set_genome_constituent_ids(replacement.genome_constituent_ids.clone());
        } else if !replacement.genome_constituent_ids.is_empty() {
            staged.add_genome_constituent_group(replacement.genome_constituent_ids.clone());
        }
        for group in &replacement.additional_genome_constituent_groups {
            staged.add_genome_constituent_group(group.iter().copied());
        }
        let max_id = staged
            .units
            .iter()
            .map(|unit| unit.physical_id.0)
            .max()
            .unwrap_or(0);
        staged.next_constituent_id = staged
            .next_constituent_id
            .max(replacement.next_constituent_id)
            .max(max_id.saturating_add(1))
            .max(1);
        if staged
            .units
            .iter()
            .map(|unit| unit.physical_id)
            .collect::<HashSet<_>>()
            .len()
            != staged.units.len()
            || staged.bonds.iter().any(|bond| {
                !staged
                    .units
                    .iter()
                    .any(|unit| unit.physical_id == bond.endpoint_a.constituent_id)
                    || !staged
                        .units
                        .iter()
                        .any(|unit| unit.physical_id == bond.endpoint_b.constituent_id)
            })
        {
            return None;
        }
        *self = staged;
        Some(())
    }

    /// Remove temporary physical constituents and every bond touching them.
    /// Bonds identify constituents by stable physical ID, so remaining units do
    /// not need index remapping.
    pub(crate) fn remove_units_by_physical_ids(&mut self, ids: &[PhysicalConstituentId]) {
        if ids.is_empty() {
            return;
        }
        let doomed: HashSet<PhysicalConstituentId> = ids.iter().copied().collect();
        self.units
            .retain(|unit| !doomed.contains(&unit.physical_id));
        self.bonds.retain(|bond| {
            !doomed.contains(&bond.endpoint_a.constituent_id)
                && !doomed.contains(&bond.endpoint_b.constituent_id)
        });
        self.genome_constituent_ids
            .retain(|id| !doomed.contains(id));
        for group in &mut self.additional_genome_constituent_groups {
            group.retain(|id| !doomed.contains(id));
        }
        self.additional_genome_constituent_groups
            .retain(|group| !group.is_empty());
    }

    /// Returns true when every unit in `candidate` is already represented by
    /// the same stable physical identity, material, and world pose in this graph.
    /// This lets aggregate owners expose transient subgraph views without making
    /// movement or world-tone consumers count those constituents twice.
    pub(crate) fn contains_exact_physical_subgraph(&self, candidate: &Self) -> bool {
        if candidate.units.is_empty() {
            return false;
        }
        let mut seen = HashSet::new();
        candidate.units.iter().all(|unit| {
            unit.physical_id.0 != 0
                && seen.insert(unit.physical_id)
                && self
                    .unit_index(unit.physical_id)
                    .is_some_and(|index| self.units[index] == *unit)
        })
    }

    pub fn physical_id(&self, unit_index: usize) -> Option<PhysicalConstituentId> {
        self.units.get(unit_index).map(|u| u.physical_id)
    }
    pub fn unit_index(&self, id: PhysicalConstituentId) -> Option<usize> {
        self.units.iter().position(|u| u.physical_id == id)
    }
    pub fn physical_constituent_ids(&self) -> Vec<PhysicalConstituentId> {
        self.units.iter().map(|u| u.physical_id).collect()
    }
    fn endpoint_index(&self, e: BondEndpoint) -> Option<usize> {
        self.unit_index(e.constituent_id)
    }
    pub fn is_valid_bond(&self, b: &Bond, c: &[BaseResource]) -> bool {
        let Some(a) = self.endpoint_index(b.endpoint_a) else {
            return false;
        };
        let Some(d) = self.endpoint_index(b.endpoint_b) else {
            return false;
        };
        if !b.is_valid(|e| {
            self.endpoint_index(e)
                .and_then(|i| e.location.world_point(self.units.get(i)?, c))
                .is_some()
        }) {
            return false;
        }
        if units_strictly_overlap(&self.units[a], &self.units[d], c) {
            return false;
        }
        let Some(pa) = self.units[a].properties(c) else {
            return false;
        };
        let Some(pb) = self.units[d].properties(c) else {
            return false;
        };
        let strength = crate::combine::bond_strength(pa, pb);
        strength.is_finite() && (0.0..=1.0).contains(&strength)
    }
    /// Stable physical IDs that established the qualifying genome cavity.
    pub fn genome_constituent_ids(&self) -> &[PhysicalConstituentId] {
        &self.genome_constituent_ids
    }

    /// Persist the actual physical constituents forming a qualifying genome
    /// cavity. No material recipe, topology, unit index, or first-unit rule is
    /// introduced; identity is carried by the stable physical IDs.
    pub fn set_genome_constituent_ids(
        &mut self,
        ids: impl IntoIterator<Item = PhysicalConstituentId>,
    ) {
        let mut ids: Vec<_> = ids.into_iter().filter(|id| id.0 != 0).collect();
        ids.sort_unstable_by_key(|id| id.0);
        ids.dedup();
        ids.retain(|id| self.unit_index(*id).is_some());
        let primary_ids: HashSet<_> = ids.iter().copied().collect();
        for group in &mut self.additional_genome_constituent_groups {
            group.retain(|id| !primary_ids.contains(id));
        }
        self.additional_genome_constituent_groups
            .retain(|group| !group.is_empty());
        self.genome_constituent_ids = ids;
    }

    pub fn clear_genome_constituent_ids(&mut self) {
        self.genome_constituent_ids.clear();
    }

    /// Additional genome-core groups in this same physical aggregate. The
    /// primary core remains available through `genome_constituent_ids` for
    /// compatibility with existing single-organism callers.
    pub fn additional_genome_constituent_groups(&self) -> &[Vec<PhysicalConstituentId>] {
        &self.additional_genome_constituent_groups
    }

    /// Register a distinct genome core without duplicating any physical unit.
    /// IDs already assigned to the primary or another core are ignored.
    pub fn add_genome_constituent_group(
        &mut self,
        ids: impl IntoIterator<Item = PhysicalConstituentId>,
    ) {
        let mut assigned: HashSet<_> = self.genome_constituent_ids.iter().copied().collect();
        for group in &self.additional_genome_constituent_groups {
            assigned.extend(group.iter().copied());
        }
        let mut group: Vec<_> = ids
            .into_iter()
            .filter(|id| id.0 != 0 && self.unit_index(*id).is_some() && assigned.insert(*id))
            .collect();
        group.sort_unstable_by_key(|id| id.0);
        group.dedup();
        if !group.is_empty() {
            self.additional_genome_constituent_groups.push(group);
        }
    }

    pub fn clear_additional_genome_constituent_groups(&mut self) {
        self.additional_genome_constituent_groups.clear();
    }

    /// The structural body is every bonded physical component that contains
    /// non-fluid material. Genome-boundary IDs are never required for structural
    /// membership, so the body can reorganize as the realized genome cavity changes.
    pub fn structural_unit_indices(&self, catalog: &[BaseResource]) -> Vec<usize> {
        let mut structural = Vec::new();
        for component in self.connected_components() {
            if component.len() < 2
                || !component.iter().any(|&index| {
                    self.units[index]
                        .material
                        .parts
                        .first()
                        .and_then(|(name, _)| {
                            catalog.iter().find(|resource| resource.name == *name)
                        })
                        .is_some_and(|resource| {
                            resource.physical_state != crate::resources::PhysicalState::Fluid
                        })
                })
            {
                continue;
            }
            structural.extend(component);
        }
        structural.sort_unstable();
        structural
    }
    pub fn genome_connected(&self, unit_index: usize) -> bool {
        if unit_index >= self.units.len()
            || (self.genome_constituent_ids.is_empty()
                && self.additional_genome_constituent_groups.is_empty())
        {
            return false;
        }
        self.connected_component_containing(unit_index)
            .into_iter()
            .any(|index| {
                let id = self.units[index].physical_id;
                self.genome_constituent_ids.contains(&id)
                    || self
                        .additional_genome_constituent_groups
                        .iter()
                        .any(|group| group.contains(&id))
            })
    }

    pub fn direct_neighbor_indices(&self, unit_index: usize) -> Vec<usize> {
        let Some(id) = self.physical_id(unit_index) else {
            return Vec::new();
        };
        let mut neighbors = Vec::new();
        for bond in &self.bonds {
            let other = if bond.endpoint_a.constituent_id == id {
                bond.endpoint_b.constituent_id
            } else if bond.endpoint_b.constituent_id == id {
                bond.endpoint_a.constituent_id
            } else {
                continue;
            };
            if let Some(index) = self.unit_index(other) {
                if !neighbors.contains(&index) {
                    neighbors.push(index);
                }
            }
        }
        neighbors.sort_unstable();
        neighbors
    }

    pub fn component_contains_nonfluid(&self, unit_index: usize, catalog: &[BaseResource]) -> bool {
        self.connected_component_containing(unit_index)
            .into_iter()
            .any(|index| {
                self.units[index]
                    .material
                    .parts
                    .first()
                    .and_then(|(name, _)| catalog.iter().find(|resource| resource.name == *name))
                    .is_some_and(|resource| {
                        resource.physical_state != crate::resources::PhysicalState::Fluid
                    })
            })
    }

    pub fn is_structurally_qualified(&self, unit_index: usize, catalog: &[BaseResource]) -> bool {
        if unit_index >= self.units.len()
            || self.connected_component_containing(unit_index).len() < 2
        {
            return false;
        }
        let Some((name, _)) = self
            .units
            .get(unit_index)
            .and_then(|unit| unit.material.parts.first())
        else {
            return false;
        };
        let Some(resource) = catalog.iter().find(|resource| resource.name == *name) else {
            return false;
        };
        if resource.physical_state != crate::resources::PhysicalState::Fluid {
            return true;
        }
        self.component_contains_nonfluid(unit_index, catalog)
    }

    pub fn connected_component_containing(&self, start: usize) -> Vec<usize> {
        if start >= self.units.len() {
            return Vec::new();
        }
        let mut adjacency = vec![Vec::<usize>::new(); self.units.len()];
        for bond in &self.bonds {
            let (Some(a), Some(b)) = (
                self.unit_index(bond.endpoint_a.constituent_id),
                self.unit_index(bond.endpoint_b.constituent_id),
            ) else {
                continue;
            };
            adjacency[a].push(b);
            adjacency[b].push(a);
        }
        let mut visited = vec![false; self.units.len()];
        let mut stack = vec![start];
        visited[start] = true;
        let mut component = Vec::new();
        while let Some(unit) = stack.pop() {
            component.push(unit);
            for &next in &adjacency[unit] {
                if !visited[next] {
                    visited[next] = true;
                    stack.push(next);
                }
            }
        }
        component.sort_unstable();
        component
    }

    pub fn connected_components(&self) -> Vec<Vec<usize>> {
        let mut adjacency = vec![Vec::<usize>::new(); self.units.len()];
        for b in &self.bonds {
            let (Some(a), Some(d)) = (
                self.unit_index(b.endpoint_a.constituent_id),
                self.unit_index(b.endpoint_b.constituent_id),
            ) else {
                continue;
            };
            adjacency[a].push(d);
            adjacency[d].push(a)
        }
        let mut visited = vec![false; self.units.len()];
        let mut out = Vec::new();
        for start in 0..self.units.len() {
            if visited[start] {
                continue;
            }
            let mut stack = vec![start];
            visited[start] = true;
            let mut component = Vec::new();
            while let Some(u) = stack.pop() {
                component.push(u);
                for &next in &adjacency[u] {
                    if !visited[next] {
                        visited[next] = true;
                        stack.push(next);
                    }
                }
            }
            component.sort_unstable();
            out.push(component)
        }
        out
    }
    pub fn connection_load(
        &self,
        u: usize,
        location: ConnectionEndpoint,
        _c: &[BaseResource],
    ) -> f64 {
        let Some(id) = self.physical_id(u) else {
            return 0.0;
        };
        self.bonds
            .iter()
            .filter(|b| b.touches(id, location))
            .map(|b| crate::combine::experimental_bond_strength(b.bond_energy))
            .sum()
    }
    pub fn break_bond(&mut self, i: usize) -> Option<Bond> {
        if i < self.bonds.len() {
            Some(self.bonds.remove(i))
        } else {
            None
        }
    }
    pub fn break_matching_bond(&mut self, t: Bond) -> Option<Bond> {
        let i = self.bonds.iter().position(|b| b.has_same_identity(&t))?;
        self.break_bond(i)
    }
}
pub type OrganismStructure = PhysicalConstituentGraph;
pub fn formation_threshold(a: f64, b: f64, la: f64, lb: f64) -> f64 {
    let la = la.max(0.0);
    let lb = lb.max(0.0);
    ((a + b) / 2.0) * (1.0 + la.sqrt() + lb.sqrt())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_fluid_resources_can_take_context_fitting_geometry() {
        let catalog = crate::resources::default_catalog();
        let mut water = StructuralUnit::new(
            "Water",
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        );
        assert!(water.realize_default_geometry(&catalog));
        let fitted = crate::resources::Shape {
            form: crate::resources::Form::Fluid {
                nominal_area: crate::resources::NOMINAL_UNIT_AREA,
                boundary: Some(vec![(-0.5, 0.0), (0.0, 0.5), (0.5, 0.0), (0.0, -0.5)]),
                arc_boundary: None,
                multi_contact_boundary: None,
            },
        };
        assert!(water.realize_fluid_geometry(fitted.clone(), &catalog));
        assert_eq!(water.geometry.as_ref().unwrap().shape(), &fitted);

        let mut bulk_water = StructuralUnit::from_material(
            Material::free_base("Water", 4.0),
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        )
        .unwrap();
        let bulk_fitted = crate::resources::Shape {
            form: crate::resources::Form::Fluid {
                nominal_area: 2.0,
                boundary: Some(vec![(-1.0, 0.0), (0.0, 1.0), (1.0, 0.0), (0.0, -1.0)]),
                arc_boundary: None,
                multi_contact_boundary: None,
            },
        };
        assert!(!water.realize_fluid_geometry(bulk_fitted.clone(), &catalog));
        assert_eq!(water.geometry.as_ref().unwrap().shape(), &fitted);
        assert!(bulk_water.realize_fluid_geometry(bulk_fitted.clone(), &catalog));
        assert_eq!(bulk_water.geometry.as_ref().unwrap().shape(), &bulk_fitted);

        let half = 2.0_f64.sqrt() / 2.0;
        let multi_contact = crate::resources::Shape {
            form: crate::resources::Form::Fluid {
                nominal_area: 2.0,
                boundary: None,
                arc_boundary: None,
                multi_contact_boundary: Some(
                    crate::capillary_geometry::MultiContactFluidBoundary {
                        nominal_area: 2.0,
                        pieces: vec![
                            crate::capillary_geometry::FluidBoundaryPiece::SolidContact {
                                start_x: -half,
                                start_y: -half,
                                end_x: half,
                                end_y: -half,
                                contact_angle_radians: std::f64::consts::FRAC_PI_2,
                            },
                            crate::capillary_geometry::FluidBoundaryPiece::SolidContact {
                                start_x: half,
                                start_y: -half,
                                end_x: half,
                                end_y: half,
                                contact_angle_radians: std::f64::consts::FRAC_PI_2,
                            },
                            crate::capillary_geometry::FluidBoundaryPiece::SolidContact {
                                start_x: half,
                                start_y: half,
                                end_x: -half,
                                end_y: half,
                                contact_angle_radians: std::f64::consts::FRAC_PI_2,
                            },
                            crate::capillary_geometry::FluidBoundaryPiece::SolidContact {
                                start_x: -half,
                                start_y: half,
                                end_x: -half,
                                end_y: -half,
                                contact_angle_radians: std::f64::consts::FRAC_PI_2,
                            },
                        ],
                    },
                ),
            },
        };
        assert!(bulk_water.realize_fluid_geometry(multi_contact.clone(), &catalog));
        assert_eq!(
            bulk_water.geometry.as_ref().unwrap().shape(),
            &multi_contact
        );

        let mut carbon = StructuralUnit::new(
            "Carbon",
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        );
        assert!(carbon.realize_default_geometry(&catalog));
        assert!(!carbon.realize_fluid_geometry(fitted, &catalog));
    }

    #[test]
    fn append_graph_remaps_local_ids_bonds_energy_and_core_groups_atomically() {
        let placement = Placement {
            x: 0.0,
            y: 0.0,
            rotation_radians: 0.0,
        };
        let mut parent = OrganismStructure::new();
        let parent_unit = parent.add_unit(StructuralUnit::new("Carbon", placement));
        let parent_id = parent.physical_id(parent_unit).unwrap();
        parent.set_genome_constituent_ids([parent_id]);

        let mut developing = OrganismStructure::new();
        let first = developing.add_unit(StructuralUnit::new("Nitrogen", placement));
        let second = developing.add_unit(StructuralUnit::new("Carbon", placement));
        let first_id = developing.physical_id(first).unwrap();
        let second_id = developing.physical_id(second).unwrap();
        developing.set_genome_constituent_ids([first_id]);
        developing.add_genome_constituent_group([second_id]);
        developing.push_bond_unchecked(Bond {
            endpoint_a: BondEndpoint::new(
                first_id,
                ConnectionEndpoint::Boundary { angle_radians: 0.0 },
            ),
            endpoint_b: BondEndpoint::new(
                second_id,
                ConnectionEndpoint::Boundary {
                    angle_radians: std::f64::consts::PI,
                },
            ),
            strength: 0.75,
            bond_energy: 2.5,
        });

        let remap = parent.append_graph_remapping_ids(&developing).unwrap();
        let mapped_first = remap[&first_id];
        let mapped_second = remap[&second_id];
        assert_ne!(mapped_first, first_id);
        assert_ne!(mapped_second, second_id);
        assert_eq!(parent.units.len(), 3);
        assert_eq!(parent.bonds.len(), 1);
        assert_eq!(parent.bonds[0].endpoint_a.constituent_id, mapped_first);
        assert_eq!(parent.bonds[0].endpoint_b.constituent_id, mapped_second);
        assert_eq!(parent.bonds[0].bond_energy, 2.5);
        assert_eq!(parent.genome_constituent_ids(), &[parent_id]);
        assert!(parent
            .additional_genome_constituent_groups()
            .contains(&vec![mapped_first]));
        assert!(parent
            .additional_genome_constituent_groups()
            .contains(&vec![mapped_second]));
        assert_eq!(
            parent
                .physical_constituent_ids()
                .into_iter()
                .collect::<HashSet<_>>()
                .len(),
            3
        );
    }

    #[test]
    fn subgraph_preserves_stable_ids_and_only_internal_bonds() {
        let placement = Placement {
            x: 0.0,
            y: 0.0,
            rotation_radians: 0.0,
        };
        let mut aggregate = OrganismStructure::new();
        let parent_index = aggregate.add_unit(StructuralUnit::new("Carbon", placement));
        let parent_id = aggregate.physical_id(parent_index).unwrap();
        aggregate.set_genome_constituent_ids([parent_id]);

        let child_a = aggregate.add_unit(StructuralUnit::new("Nitrogen", placement));
        let child_b = aggregate.add_unit(StructuralUnit::new("Carbon", placement));
        let child_a_id = aggregate.physical_id(child_a).unwrap();
        let child_b_id = aggregate.physical_id(child_b).unwrap();
        aggregate.add_genome_constituent_group([child_a_id]);
        let endpoint =
            |id| BondEndpoint::new(id, ConnectionEndpoint::Boundary { angle_radians: 0.0 });
        aggregate.push_bond_unchecked(Bond {
            endpoint_a: endpoint(child_a_id),
            endpoint_b: endpoint(child_b_id),
            strength: 0.75,
            bond_energy: 2.5,
        });
        aggregate.push_bond_unchecked(Bond {
            endpoint_a: endpoint(parent_id),
            endpoint_b: endpoint(child_a_id),
            strength: 0.5,
            bond_energy: 1.25,
        });

        let subgraph = aggregate
            .subgraph_for_physical_ids(&[child_a_id, child_b_id], &[child_a_id])
            .expect("valid child subset");
        assert_eq!(
            subgraph.physical_constituent_ids(),
            vec![child_a_id, child_b_id]
        );
        assert_eq!(subgraph.bonds.len(), 1);
        assert_eq!(subgraph.bonds[0].bond_energy, 2.5);
        assert_eq!(subgraph.genome_constituent_ids(), &[child_a_id]);
        assert!(subgraph.additional_genome_constituent_groups().is_empty());

        let mut extended = subgraph;
        let added = extended.add_unit(StructuralUnit::new("Sulfur", placement));
        assert_eq!(extended.physical_id(added), Some(PhysicalConstituentId(4)));
        assert_eq!(aggregate.units.len(), 3);
    }

    #[test]
    fn subgraph_replacement_preserves_cross_boundary_bonds_and_rejects_dangling_replacements() {
        let placement = Placement {
            x: 0.0,
            y: 0.0,
            rotation_radians: 0.0,
        };
        let mut aggregate = OrganismStructure::new();
        let parent_index = aggregate.add_unit(StructuralUnit::new("Carbon", placement));
        let parent_id = aggregate.physical_id(parent_index).unwrap();
        aggregate.set_genome_constituent_ids([parent_id]);

        let child_core_index = aggregate.add_unit(StructuralUnit::new("Nitrogen", placement));
        let child_body_index = aggregate.add_unit(StructuralUnit::new("Carbon", placement));
        let child_core_id = aggregate.physical_id(child_core_index).unwrap();
        let child_body_id = aggregate.physical_id(child_body_index).unwrap();
        aggregate.add_genome_constituent_group([child_core_id]);
        let endpoint =
            |id| BondEndpoint::new(id, ConnectionEndpoint::Boundary { angle_radians: 0.0 });
        aggregate.push_bond_unchecked(Bond {
            endpoint_a: endpoint(parent_id),
            endpoint_b: endpoint(child_core_id),
            strength: 0.5,
            bond_energy: 1.25,
        });
        aggregate.push_bond_unchecked(Bond {
            endpoint_a: endpoint(child_core_id),
            endpoint_b: endpoint(child_body_id),
            strength: 0.75,
            bond_energy: 2.5,
        });

        let selected_ids = [child_core_id, child_body_id];
        let mut replacement = aggregate
            .subgraph_for_physical_ids(&selected_ids, &[child_core_id])
            .expect("valid attached subgraph");
        replacement.units[1].placement.x = 1.0;
        aggregate
            .replace_subgraph_for_physical_ids(&selected_ids, &replacement)
            .expect("replacement retains the crossing-bond endpoint");

        assert_eq!(
            aggregate.physical_constituent_ids(),
            vec![parent_id, child_core_id, child_body_id]
        );
        assert_eq!(aggregate.bonds.len(), 2);
        assert!(aggregate.bonds.iter().any(|bond| {
            bond.endpoint_a.constituent_id == parent_id
                && bond.endpoint_b.constituent_id == child_core_id
                && bond.bond_energy == 1.25
        }));
        assert!(aggregate.bonds.iter().any(|bond| {
            bond.endpoint_a.constituent_id == child_core_id
                && bond.endpoint_b.constituent_id == child_body_id
                && bond.bond_energy == 2.5
        }));

        let before_rejection = aggregate.clone();
        replacement
            .units
            .retain(|unit| unit.physical_id != child_core_id);
        assert!(aggregate
            .replace_subgraph_for_physical_ids(&selected_ids, &replacement)
            .is_none());
        assert_eq!(aggregate.units, before_rejection.units);
        assert_eq!(aggregate.bonds, before_rejection.bonds);
        assert_eq!(
            aggregate.genome_constituent_ids,
            before_rejection.genome_constituent_ids
        );
        assert_eq!(
            aggregate.additional_genome_constituent_groups,
            before_rejection.additional_genome_constituent_groups
        );
        assert_eq!(
            aggregate.next_constituent_id,
            before_rejection.next_constituent_id
        );
    }

    #[test]
    fn division_partition_preserves_ids_and_returns_cross_boundary_bond_energy() {
        let placement = Placement {
            x: 0.0,
            y: 0.0,
            rotation_radians: 0.0,
        };
        let mut aggregate = OrganismStructure::new();
        let parent_index = aggregate.add_unit(StructuralUnit::new("Carbon", placement));
        let parent_id = aggregate.physical_id(parent_index).unwrap();
        aggregate.set_genome_constituent_ids([parent_id]);
        let child_core_index = aggregate.add_unit(StructuralUnit::new("Nitrogen", placement));
        let child_core_id = aggregate.physical_id(child_core_index).unwrap();
        let child_body_index = aggregate.add_unit(StructuralUnit::new("Sulfur", placement));
        let child_body_id = aggregate.physical_id(child_body_index).unwrap();
        aggregate.add_genome_constituent_group([child_core_id]);
        let endpoint =
            |id| BondEndpoint::new(id, ConnectionEndpoint::Boundary { angle_radians: 0.0 });
        aggregate.push_bond_unchecked(Bond {
            endpoint_a: endpoint(parent_id),
            endpoint_b: endpoint(child_core_id),
            strength: 0.5,
            bond_energy: 1.25,
        });
        aggregate.push_bond_unchecked(Bond {
            endpoint_a: endpoint(child_core_id),
            endpoint_b: endpoint(child_body_id),
            strength: 0.75,
            bond_energy: 2.5,
        });

        let (child, released_energy) = aggregate
            .partition_subgraph_for_division(&[child_core_id, child_body_id], &[child_core_id])
            .expect("valid aggregate division");
        assert_eq!(released_energy, 1.25);
        assert_eq!(aggregate.physical_constituent_ids(), vec![parent_id]);
        assert!(aggregate.bonds.is_empty());
        assert_eq!(
            child.physical_constituent_ids(),
            vec![child_core_id, child_body_id]
        );
        assert_eq!(child.bonds.len(), 1);
        assert_eq!(child.bonds[0].bond_energy, 2.5);
        assert_eq!(child.genome_constituent_ids(), &[child_core_id]);
        assert!(child.additional_genome_constituent_groups().is_empty());
    }

    #[test]
    fn division_partition_settles_all_crossing_bond_energy_and_preserves_internal_bonds() {
        let placement = Placement {
            x: 0.0,
            y: 0.0,
            rotation_radians: 0.0,
        };
        let mut aggregate = OrganismStructure::new();
        let parent_core_index = aggregate.add_unit(StructuralUnit::new("Carbon", placement));
        let parent_core_id = aggregate.physical_id(parent_core_index).unwrap();
        aggregate.set_genome_constituent_ids([parent_core_id]);
        let parent_body_index = aggregate.add_unit(StructuralUnit::new("Sulfur", placement));
        let parent_body_id = aggregate.physical_id(parent_body_index).unwrap();
        let child_core_index = aggregate.add_unit(StructuralUnit::new("Nitrogen", placement));
        let child_core_id = aggregate.physical_id(child_core_index).unwrap();
        let child_body_index = aggregate.add_unit(StructuralUnit::new("Phosphorus", placement));
        let child_body_id = aggregate.physical_id(child_body_index).unwrap();
        aggregate.add_genome_constituent_group([child_core_id]);
        let endpoint =
            |id| BondEndpoint::new(id, ConnectionEndpoint::Boundary { angle_radians: 0.0 });
        aggregate.push_bond_unchecked(Bond {
            endpoint_a: endpoint(parent_core_id),
            endpoint_b: endpoint(parent_body_id),
            strength: 0.8,
            bond_energy: 4.0,
        });
        aggregate.push_bond_unchecked(Bond {
            endpoint_a: endpoint(parent_body_id),
            endpoint_b: endpoint(child_core_id),
            strength: 0.5,
            bond_energy: 1.25,
        });
        aggregate.push_bond_unchecked(Bond {
            endpoint_a: endpoint(parent_core_id),
            endpoint_b: endpoint(child_body_id),
            strength: 0.4,
            bond_energy: 2.0,
        });
        aggregate.push_bond_unchecked(Bond {
            endpoint_a: endpoint(child_core_id),
            endpoint_b: endpoint(child_body_id),
            strength: 0.7,
            bond_energy: 2.5,
        });
        let next_id_before = aggregate.next_constituent_id;

        let (child, crossing_energy) = aggregate
            .partition_subgraph_for_division(&[child_core_id, child_body_id], &[child_core_id])
            .expect("valid aggregate division");

        assert_eq!(crossing_energy, 3.25);
        assert_eq!(
            aggregate.physical_constituent_ids(),
            vec![parent_core_id, parent_body_id]
        );
        assert_eq!(aggregate.bonds.len(), 1);
        assert_eq!(aggregate.bonds[0].bond_energy, 4.0);
        assert_eq!(aggregate.next_constituent_id, next_id_before);
        assert_eq!(
            child.physical_constituent_ids(),
            vec![child_core_id, child_body_id]
        );
        assert_eq!(child.bonds.len(), 1);
        assert_eq!(child.bonds[0].bond_energy, 2.5);
        assert_eq!(child.genome_constituent_ids(), &[child_core_id]);
        assert!(child.additional_genome_constituent_groups().is_empty());
    }

    #[test]
    fn division_partition_rejects_partial_core_without_mutating_aggregate() {
        let placement = Placement {
            x: 0.0,
            y: 0.0,
            rotation_radians: 0.0,
        };
        let mut aggregate = OrganismStructure::new();
        let parent_index = aggregate.add_unit(StructuralUnit::new("Carbon", placement));
        let parent_id = aggregate.physical_id(parent_index).unwrap();
        aggregate.set_genome_constituent_ids([parent_id]);
        let child_a_index = aggregate.add_unit(StructuralUnit::new("Nitrogen", placement));
        let child_a_id = aggregate.physical_id(child_a_index).unwrap();
        let child_b_index = aggregate.add_unit(StructuralUnit::new("Sulfur", placement));
        let child_b_id = aggregate.physical_id(child_b_index).unwrap();
        aggregate.add_genome_constituent_group([child_a_id, child_b_id]);
        let before = aggregate.clone();

        assert!(aggregate
            .partition_subgraph_for_division(&[child_a_id], &[child_a_id])
            .is_none());
        assert_eq!(aggregate.units, before.units);
        assert_eq!(aggregate.bonds, before.bonds);
        assert_eq!(
            aggregate.genome_constituent_ids,
            before.genome_constituent_ids
        );
        assert_eq!(
            aggregate.additional_genome_constituent_groups,
            before.additional_genome_constituent_groups
        );
        assert_eq!(aggregate.next_constituent_id, before.next_constituent_id);
    }

    #[test]
    fn division_partition_rejects_partial_secondary_core_group_atomically() {
        let placement = Placement {
            x: 0.0,
            y: 0.0,
            rotation_radians: 0.0,
        };
        let mut aggregate = OrganismStructure::new();
        let parent_index = aggregate.add_unit(StructuralUnit::new("Carbon", placement));
        let parent_id = aggregate.physical_id(parent_index).unwrap();
        aggregate.set_genome_constituent_ids([parent_id]);
        let child_core_index = aggregate.add_unit(StructuralUnit::new("Nitrogen", placement));
        let child_core_id = aggregate.physical_id(child_core_index).unwrap();
        let child_body_index = aggregate.add_unit(StructuralUnit::new("Sulfur", placement));
        let child_body_id = aggregate.physical_id(child_body_index).unwrap();
        let other_core_a_index = aggregate.add_unit(StructuralUnit::new("Hydrogen", placement));
        let other_core_a_id = aggregate.physical_id(other_core_a_index).unwrap();
        let other_core_b_index = aggregate.add_unit(StructuralUnit::new("Phosphorus", placement));
        let other_core_b_id = aggregate.physical_id(other_core_b_index).unwrap();
        aggregate.add_genome_constituent_group([child_core_id]);
        aggregate.add_genome_constituent_group([other_core_a_id, other_core_b_id]);
        let before = aggregate.clone();

        assert!(aggregate
            .partition_subgraph_for_division(
                &[child_core_id, child_body_id, other_core_a_id],
                &[child_core_id]
            )
            .is_none());
        assert_eq!(aggregate.units, before.units);
        assert_eq!(aggregate.bonds, before.bonds);
        assert_eq!(
            aggregate.genome_constituent_ids,
            before.genome_constituent_ids
        );
        assert_eq!(
            aggregate.additional_genome_constituent_groups,
            before.additional_genome_constituent_groups
        );
    }

    #[test]
    fn subgraph_rejects_partial_core_and_invalid_ids() {
        let placement = Placement {
            x: 0.0,
            y: 0.0,
            rotation_radians: 0.0,
        };
        let mut graph = OrganismStructure::new();
        let a = graph.add_unit(StructuralUnit::new("Carbon", placement));
        let b = graph.add_unit(StructuralUnit::new("Nitrogen", placement));
        let a_id = graph.physical_id(a).unwrap();
        let b_id = graph.physical_id(b).unwrap();
        graph.add_genome_constituent_group([a_id, b_id]);

        assert!(graph.subgraph_for_physical_ids(&[a_id], &[a_id]).is_none());
        assert!(graph
            .subgraph_for_physical_ids(&[a_id, PhysicalConstituentId(999)], &[])
            .is_none());
        assert!(graph
            .subgraph_for_physical_ids(&[a_id, a_id], &[])
            .is_none());

        let mut invalid_next_id = graph.clone();
        invalid_next_id.next_constituent_id = 2;
        assert!(invalid_next_id
            .subgraph_for_physical_ids(&[a_id, b_id], &[])
            .is_none());
    }

    #[test]
    fn subgraph_rejects_empty_or_overlapping_genome_groups() {
        let placement = Placement {
            x: 0.0,
            y: 0.0,
            rotation_radians: 0.0,
        };
        let mut empty_group_graph = OrganismStructure::new();
        let parent = empty_group_graph.add_unit(StructuralUnit::new("Carbon", placement));
        let parent_id = empty_group_graph.physical_id(parent).unwrap();
        empty_group_graph.set_genome_constituent_ids([parent_id]);
        let child = empty_group_graph.add_unit(StructuralUnit::new("Nitrogen", placement));
        let child_id = empty_group_graph.physical_id(child).unwrap();
        empty_group_graph
            .additional_genome_constituent_groups
            .push(Vec::new());
        assert!(empty_group_graph
            .subgraph_for_physical_ids(&[child_id], &[])
            .is_none());

        let mut overlapping_group_graph = OrganismStructure::new();
        let parent = overlapping_group_graph.add_unit(StructuralUnit::new("Carbon", placement));
        let parent_id = overlapping_group_graph.physical_id(parent).unwrap();
        overlapping_group_graph.set_genome_constituent_ids([parent_id]);
        let child = overlapping_group_graph.add_unit(StructuralUnit::new("Nitrogen", placement));
        let child_id = overlapping_group_graph.physical_id(child).unwrap();
        overlapping_group_graph
            .additional_genome_constituent_groups
            .push(vec![parent_id]);
        assert!(overlapping_group_graph
            .subgraph_for_physical_ids(&[child_id], &[])
            .is_none());
    }

    #[test]
    fn replacing_subgraph_preserves_cross_bonds_and_updates_only_attached_members() {
        let placement = Placement {
            x: 0.0,
            y: 0.0,
            rotation_radians: 0.0,
        };
        let mut aggregate = OrganismStructure::new();
        let parent_index = aggregate.add_unit(StructuralUnit::new("Carbon", placement));
        let parent_id = aggregate.physical_id(parent_index).unwrap();
        aggregate.set_genome_constituent_ids([parent_id]);
        let child_a = aggregate.add_unit(StructuralUnit::new("Nitrogen", placement));
        let child_b = aggregate.add_unit(StructuralUnit::new("Carbon", placement));
        let child_a_id = aggregate.physical_id(child_a).unwrap();
        let child_b_id = aggregate.physical_id(child_b).unwrap();
        aggregate.add_genome_constituent_group([child_a_id]);
        let endpoint =
            |id| BondEndpoint::new(id, ConnectionEndpoint::Boundary { angle_radians: 0.0 });
        aggregate.push_bond_unchecked(Bond {
            endpoint_a: endpoint(child_a_id),
            endpoint_b: endpoint(child_b_id),
            strength: 0.75,
            bond_energy: 2.5,
        });
        aggregate.push_bond_unchecked(Bond {
            endpoint_a: endpoint(parent_id),
            endpoint_b: endpoint(child_a_id),
            strength: 0.5,
            bond_energy: 1.25,
        });

        let mut replacement = aggregate
            .subgraph_for_physical_ids(&[child_a_id, child_b_id], &[child_a_id])
            .unwrap();
        replacement.bonds[0].bond_energy = 3.5;
        let new_index = replacement.add_unit(StructuralUnit::new("Sulfur", placement));
        let new_id = replacement.physical_id(new_index).unwrap();
        replacement.push_bond_unchecked(Bond {
            endpoint_a: endpoint(child_b_id),
            endpoint_b: endpoint(new_id),
            strength: 0.6,
            bond_energy: 0.75,
        });

        assert!(aggregate
            .replace_subgraph_for_physical_ids(&[child_a_id, child_b_id], &replacement)
            .is_some());
        assert_eq!(aggregate.units.len(), 4);
        assert_eq!(aggregate.genome_constituent_ids(), &[parent_id]);
        assert!(aggregate
            .additional_genome_constituent_groups()
            .contains(&vec![child_a_id]));
        assert!(aggregate.bonds.iter().any(|bond| {
            bond.endpoint_a.constituent_id == parent_id
                && bond.endpoint_b.constituent_id == child_a_id
                && (bond.bond_energy - 1.25).abs() < f64::EPSILON
        }));
        assert!(aggregate.bonds.iter().any(|bond| {
            bond.endpoint_a.constituent_id == child_a_id
                && bond.endpoint_b.constituent_id == child_b_id
                && (bond.bond_energy - 3.5).abs() < f64::EPSILON
        }));
        assert!(aggregate.bonds.iter().any(|bond| {
            bond.endpoint_a.constituent_id == child_b_id && bond.endpoint_b.constituent_id == new_id
        }));
    }

    #[test]
    fn replacing_subgraph_rejects_empty_replacement_genome_group_atomically() {
        let placement = Placement {
            x: 0.0,
            y: 0.0,
            rotation_radians: 0.0,
        };
        let mut aggregate = OrganismStructure::new();
        let parent = aggregate.add_unit(StructuralUnit::new("Carbon", placement));
        let parent_id = aggregate.physical_id(parent).unwrap();
        aggregate.set_genome_constituent_ids([parent_id]);
        let child = aggregate.add_unit(StructuralUnit::new("Nitrogen", placement));
        let child_id = aggregate.physical_id(child).unwrap();
        let mut replacement = aggregate
            .subgraph_for_physical_ids(&[child_id], &[])
            .unwrap();
        replacement
            .additional_genome_constituent_groups
            .push(Vec::new());

        let before_units = aggregate.units.clone();
        let before_bonds = aggregate.bonds.clone();
        let before_primary = aggregate.genome_constituent_ids.clone();
        let before_groups = aggregate.additional_genome_constituent_groups.clone();

        assert!(aggregate
            .replace_subgraph_for_physical_ids(&[child_id], &replacement)
            .is_none());
        assert_eq!(aggregate.units, before_units);
        assert_eq!(aggregate.bonds, before_bonds);
        assert_eq!(aggregate.genome_constituent_ids, before_primary);
        assert_eq!(
            aggregate.additional_genome_constituent_groups,
            before_groups
        );
    }

    #[test]
    fn replacing_subgraph_rejects_dangling_destination_bond_atomically() {
        let placement = Placement {
            x: 0.0,
            y: 0.0,
            rotation_radians: 0.0,
        };
        let mut aggregate = OrganismStructure::new();
        let parent = aggregate.add_unit(StructuralUnit::new("Carbon", placement));
        let parent_id = aggregate.physical_id(parent).unwrap();
        aggregate.set_genome_constituent_ids([parent_id]);
        let child = aggregate.add_unit(StructuralUnit::new("Nitrogen", placement));
        let child_id = aggregate.physical_id(child).unwrap();
        // Capture a valid replacement before injecting the malformed bond.
        // The destination validation is what this test is intended to exercise.
        let replacement = aggregate
            .subgraph_for_physical_ids(&[child_id], &[])
            .unwrap();
        aggregate.push_bond_unchecked(Bond {
            endpoint_a: BondEndpoint::new(
                child_id,
                ConnectionEndpoint::Boundary { angle_radians: 0.0 },
            ),
            endpoint_b: BondEndpoint::new(
                crate::structure::PhysicalConstituentId(u64::MAX),
                ConnectionEndpoint::Boundary { angle_radians: 0.0 },
            ),
            strength: 0.5,
            bond_energy: 1.0,
        });

        let before_units = aggregate.units.clone();
        let before_bonds = aggregate.bonds.clone();
        let before_primary = aggregate.genome_constituent_ids.clone();
        let before_groups = aggregate.additional_genome_constituent_groups.clone();

        assert!(aggregate
            .replace_subgraph_for_physical_ids(&[child_id], &replacement)
            .is_none());
        assert_eq!(aggregate.units, before_units);
        assert_eq!(aggregate.bonds, before_bonds);
        assert_eq!(aggregate.genome_constituent_ids, before_primary);
        assert_eq!(
            aggregate.additional_genome_constituent_groups,
            before_groups
        );
    }

    #[test]
    fn replacing_subgraph_rejects_lost_cross_bond_endpoint_atomically() {
        let placement = Placement {
            x: 0.0,
            y: 0.0,
            rotation_radians: 0.0,
        };
        let mut aggregate = OrganismStructure::new();
        let parent = aggregate.add_unit(StructuralUnit::new("Carbon", placement));
        let parent_id = aggregate.physical_id(parent).unwrap();
        aggregate.set_genome_constituent_ids([parent_id]);
        let child_a = aggregate.add_unit(StructuralUnit::new("Nitrogen", placement));
        let child_b = aggregate.add_unit(StructuralUnit::new("Carbon", placement));
        let child_a_id = aggregate.physical_id(child_a).unwrap();
        let child_b_id = aggregate.physical_id(child_b).unwrap();
        aggregate.add_genome_constituent_group([child_a_id]);
        aggregate.push_bond_unchecked(Bond {
            endpoint_a: BondEndpoint::new(
                parent_id,
                ConnectionEndpoint::Boundary { angle_radians: 0.0 },
            ),
            endpoint_b: BondEndpoint::new(
                child_a_id,
                ConnectionEndpoint::Boundary { angle_radians: 0.0 },
            ),
            strength: 0.5,
            bond_energy: 1.25,
        });
        let replacement = aggregate
            .subgraph_for_physical_ids(&[child_b_id], &[])
            .unwrap();
        let before_units = aggregate.units.clone();
        let before_bonds = aggregate.bonds.clone();
        let before_groups = aggregate.additional_genome_constituent_groups.clone();

        assert!(aggregate
            .replace_subgraph_for_physical_ids(&[child_a_id, child_b_id], &replacement)
            .is_none());
        assert_eq!(aggregate.units, before_units);
        assert_eq!(aggregate.bonds, before_bonds);
        assert_eq!(
            aggregate.additional_genome_constituent_groups,
            before_groups
        );
    }

    #[test]
    fn append_graph_rejects_empty_source_genome_group_without_mutating_destination() {
        let placement = Placement {
            x: 0.0,
            y: 0.0,
            rotation_radians: 0.0,
        };
        let mut destination = OrganismStructure::new();
        destination.add_unit(StructuralUnit::new("Carbon", placement));
        let before_units = destination.units.clone();
        let before_bonds = destination.bonds.clone();

        let mut source = OrganismStructure::new();
        let parent = source.add_unit(StructuralUnit::new("Nitrogen", placement));
        let parent_id = source.physical_id(parent).unwrap();
        source.set_genome_constituent_ids([parent_id]);
        source.additional_genome_constituent_groups.push(Vec::new());

        assert!(destination.append_graph_remapping_ids(&source).is_none());
        assert_eq!(destination.units, before_units);
        assert_eq!(destination.bonds, before_bonds);
    }

    #[test]
    fn append_graph_rejects_invalid_destination_ids_without_mutating_it() {
        let placement = Placement {
            x: 0.0,
            y: 0.0,
            rotation_radians: 0.0,
        };
        let mut destination = OrganismStructure::new();
        let first = destination.add_unit(StructuralUnit::new("Carbon", placement));
        let second = destination.add_unit(StructuralUnit::new("Nitrogen", placement));
        destination.units[second].physical_id = destination.units[first].physical_id;
        let original_units = destination.units.clone();
        let original_bonds = destination.bonds.clone();

        let mut source = OrganismStructure::new();
        source.add_unit(StructuralUnit::new("Sulfur", placement));

        assert!(destination.append_graph_remapping_ids(&source).is_none());
        assert_eq!(destination.units, original_units);
        assert_eq!(destination.bonds, original_bonds);
    }

    #[test]
    fn append_graph_rejects_dangling_bond_without_mutating_destination() {
        let placement = Placement {
            x: 0.0,
            y: 0.0,
            rotation_radians: 0.0,
        };
        let mut destination = OrganismStructure::new();
        destination.add_unit(StructuralUnit::new("Carbon", placement));
        let original_units = destination.units.clone();
        let original_bonds = destination.bonds.clone();

        let mut invalid_source = OrganismStructure::new();
        let source_unit = invalid_source.add_unit(StructuralUnit::new("Carbon", placement));
        let source_id = invalid_source.physical_id(source_unit).unwrap();
        invalid_source.push_bond_unchecked(Bond {
            endpoint_a: BondEndpoint::new(
                source_id,
                ConnectionEndpoint::Boundary { angle_radians: 0.0 },
            ),
            endpoint_b: BondEndpoint::new(
                PhysicalConstituentId(999),
                ConnectionEndpoint::Boundary { angle_radians: 1.0 },
            ),
            strength: 0.5,
            bond_energy: 1.0,
        });

        assert!(destination
            .append_graph_remapping_ids(&invalid_source)
            .is_none());
        assert_eq!(destination.units, original_units);
        assert_eq!(destination.bonds, original_bonds);
    }

    #[test]
    fn structural_membership_follows_realized_bonded_components() {
        let catalog = crate::resources::default_catalog();
        let mut s = OrganismStructure::new();
        let a = s.add_unit(StructuralUnit::new(
            "Carbon",
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));
        let b = s.add_unit(StructuralUnit::new(
            "Carbon",
            Placement {
                x: 2.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));
        let ida = s.physical_id(a).unwrap();
        let idb = s.physical_id(b).unwrap();
        assert!(s.structural_unit_indices(&catalog).is_empty());
        s.push_bond_unchecked(Bond {
            endpoint_a: BondEndpoint::new(ida, ConnectionEndpoint::Boundary { angle_radians: 0.0 }),
            endpoint_b: BondEndpoint::new(
                idb,
                ConnectionEndpoint::Boundary {
                    angle_radians: std::f64::consts::PI,
                },
            ),
            strength: 0.5,
            bond_energy: 1.0,
        });
        assert_eq!(s.structural_unit_indices(&catalog), vec![a, b]);
        s.set_genome_constituent_ids([idb]);
        assert_eq!(s.structural_unit_indices(&catalog), vec![a, b]);
    }

    fn connect(s: &mut OrganismStructure, a: usize, b: usize) {
        let ida = s.physical_id(a).unwrap();
        let idb = s.physical_id(b).unwrap();
        s.push_bond_unchecked(Bond {
            endpoint_a: BondEndpoint::new(ida, ConnectionEndpoint::Boundary { angle_radians: 0.0 }),
            endpoint_b: BondEndpoint::new(
                idb,
                ConnectionEndpoint::Boundary {
                    angle_radians: std::f64::consts::PI,
                },
            ),
            strength: 0.5,
            bond_energy: 1.0,
        });
    }

    fn water_chain_structure(names: &[&str]) -> (OrganismStructure, Vec<usize>) {
        let mut s = OrganismStructure::new();
        let mut units = Vec::new();
        for (index, name) in names.iter().enumerate() {
            units.push(s.add_unit(StructuralUnit::new(
                *name,
                Placement {
                    x: index as f64,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
            )));
        }
        for index in 0..units.len().saturating_sub(1) {
            connect(&mut s, units[index], units[index + 1]);
        }
        let genome_id = s.physical_id(units[0]).unwrap();
        s.set_genome_constituent_ids([genome_id]);
        (s, units)
    }

    #[test]
    fn water_structural_matrix_follows_component_nonfluid_rule() {
        let catalog = crate::resources::default_catalog();

        let (s, u) = water_chain_structure(&["Carbon", "Water"]);
        assert!(s.is_structurally_qualified(u[1], &catalog));

        let (s, u) = water_chain_structure(&["Carbon", "Water", "Water"]);
        assert!(s.is_structurally_qualified(u[1], &catalog));
        assert!(s.is_structurally_qualified(u[2], &catalog));

        let (s, u) = water_chain_structure(&["Carbon", "Water", "Water", "Water", "Carbon"]);
        assert!(s.is_structurally_qualified(u[1], &catalog));
        assert!(s.is_structurally_qualified(u[2], &catalog));
        assert!(s.is_structurally_qualified(u[3], &catalog));
        assert!(s.is_structurally_qualified(u[4], &catalog));

        let (s, u) = water_chain_structure(&["Carbon", "Water", "Carbon", "Water", "Carbon"]);
        assert!(s.is_structurally_qualified(u[1], &catalog));
        assert!(s.is_structurally_qualified(u[3], &catalog));
    }

    #[test]
    fn component_nonfluid_member_qualifies_fluid_water() {
        let catalog = crate::resources::default_catalog();
        let mut s = OrganismStructure::new();
        let genome = s.add_unit(StructuralUnit::new(
            "Carbon",
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));
        let water = s.add_unit(StructuralUnit::new(
            "Water",
            Placement {
                x: 1.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));
        let gid = s.physical_id(genome).unwrap();
        let wid = s.physical_id(water).unwrap();
        s.set_genome_constituent_ids([gid]);
        s.push_bond_unchecked(Bond {
            endpoint_a: BondEndpoint::new(gid, ConnectionEndpoint::Boundary { angle_radians: 0.0 }),
            endpoint_b: BondEndpoint::new(wid, ConnectionEndpoint::Fluid { x: 0.0, y: 0.0 }),
            strength: 0.5,
            bond_energy: 1.0,
        });
        assert!(s.is_structurally_qualified(water, &catalog));
    }

    #[test]
    fn water_water_chain_qualifies_when_component_contains_nonfluid() {
        let catalog = crate::resources::default_catalog();
        let mut s = OrganismStructure::new();
        let genome = s.add_unit(StructuralUnit::new(
            "Carbon",
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));
        let w1 = s.add_unit(StructuralUnit::new(
            "Water",
            Placement {
                x: 1.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));
        let w2 = s.add_unit(StructuralUnit::new(
            "Water",
            Placement {
                x: 2.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));
        let gid = s.physical_id(genome).unwrap();
        let w1id = s.physical_id(w1).unwrap();
        let w2id = s.physical_id(w2).unwrap();
        s.set_genome_constituent_ids([gid]);
        s.push_bond_unchecked(Bond {
            endpoint_a: BondEndpoint::new(gid, ConnectionEndpoint::Boundary { angle_radians: 0.0 }),
            endpoint_b: BondEndpoint::new(w1id, ConnectionEndpoint::Fluid { x: 0.0, y: 0.0 }),
            strength: 0.5,
            bond_energy: 1.0,
        });
        s.push_bond_unchecked(Bond {
            endpoint_a: BondEndpoint::new(w1id, ConnectionEndpoint::Fluid { x: 0.0, y: 0.0 }),
            endpoint_b: BondEndpoint::new(w2id, ConnectionEndpoint::Fluid { x: 0.0, y: 0.0 }),
            strength: 0.5,
            bond_energy: 1.0,
        });
        assert!(s.is_structurally_qualified(w1, &catalog));
        assert!(s.is_structurally_qualified(w2, &catalog));
    }

    #[test]
    fn physical_constituents_receive_stable_ids() {
        let mut s = OrganismStructure::new();
        let a = s.add_unit(StructuralUnit::new(
            "Carbon",
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));
        let b = s.add_unit(StructuralUnit::new(
            "Hydrogen",
            Placement {
                x: 1.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));
        assert_ne!(s.physical_id(a), s.physical_id(b));
    }
    #[test]
    fn aggregate_recognizes_transient_subgraph_by_stable_identity_material_and_pose() {
        let pose = Placement {
            x: 2.0,
            y: 3.0,
            rotation_radians: 0.25,
        };
        let mut aggregate = OrganismStructure::new();
        let index = aggregate.add_unit(StructuralUnit::new("Carbon", pose));
        let id = aggregate.physical_id(index).unwrap();

        let mut view = OrganismStructure::new();
        let mut unit = StructuralUnit::new("Carbon", pose);
        unit.physical_id = id;
        view.units.push(unit);
        assert!(aggregate.contains_exact_physical_subgraph(&view));

        view.units[0].placement.x += 0.25;
        assert!(!aggregate.contains_exact_physical_subgraph(&view));
        view.units[0].placement = pose;
        view.units[0].material = Material::free_base("Water", 1.0);
        assert!(!aggregate.contains_exact_physical_subgraph(&view));
    }

    #[test]
    fn genome_constituent_ids_round_trip_through_serialization() {
        let mut s = OrganismStructure::new();
        let unit = s.add_unit(StructuralUnit::new(
            "Carbon",
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));
        let id = s.physical_id(unit).unwrap();
        s.set_genome_constituent_ids([id]);
        let encoded = serde_json::to_string(&s).unwrap();
        let decoded: OrganismStructure = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded.genome_constituent_ids(), &[id]);
        assert!(decoded.additional_genome_constituent_groups().is_empty());
    }

    #[test]
    fn aggregate_genome_core_groups_round_trip_without_duplicating_units() {
        let mut s = OrganismStructure::new();
        let first = s.add_unit(StructuralUnit::new(
            "Carbon",
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));
        let second = s.add_unit(StructuralUnit::new(
            "Carbon",
            Placement {
                x: 2.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));
        let first_id = s.physical_id(first).unwrap();
        let second_id = s.physical_id(second).unwrap();
        s.set_genome_constituent_ids([first_id]);
        s.add_genome_constituent_group([second_id, second_id, first_id]);
        assert_eq!(s.units.len(), 2);
        assert_eq!(s.additional_genome_constituent_groups(), &[vec![second_id]]);
        let encoded = serde_json::to_string(&s).unwrap();
        let decoded: OrganismStructure = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded.genome_constituent_ids(), &[first_id]);
        assert_eq!(
            decoded.additional_genome_constituent_groups(),
            &[vec![second_id]]
        );
        assert_eq!(decoded.units.len(), 2);

        let mut pruned = decoded.clone();
        pruned.remove_units_by_physical_ids(&[second_id]);
        assert!(pruned.additional_genome_constituent_groups().is_empty());
        assert_eq!(pruned.units.len(), 1);

        let mut reassigned = decoded;
        reassigned.set_genome_constituent_ids([first_id, second_id]);
        assert!(reassigned.additional_genome_constituent_groups().is_empty());
    }

    #[test]
    fn each_aggregate_genome_core_is_independently_recognized() {
        let mut structure = OrganismStructure::new();
        let primary = structure.add_unit(StructuralUnit::new(
            "Carbon",
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));
        let additional = structure.add_unit(StructuralUnit::new(
            "Carbon",
            Placement {
                x: 3.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));
        let primary_id = structure.physical_id(primary).unwrap();
        let additional_id = structure.physical_id(additional).unwrap();
        structure.set_genome_constituent_ids([primary_id]);
        structure.add_genome_constituent_group([additional_id]);

        assert!(structure.genome_connected(primary));
        assert!(structure.genome_connected(additional));
    }

    #[test]
    fn bond_identity_uses_physical_ids() {
        let a = PhysicalConstituentId(11);
        let b = PhysicalConstituentId(22);
        let x = Bond {
            endpoint_a: BondEndpoint::new(a, ConnectionEndpoint::Corner { point_index: 0 }),
            endpoint_b: BondEndpoint::new(b, ConnectionEndpoint::Corner { point_index: 1 }),
            strength: 0.5,
            bond_energy: 1.0,
        };
        let y = Bond {
            endpoint_a: BondEndpoint::new(a, ConnectionEndpoint::Corner { point_index: 0 }),
            endpoint_b: BondEndpoint::new(b, ConnectionEndpoint::Corner { point_index: 1 }),
            strength: 0.5,
            bond_energy: 1.0,
        };
        assert!(x.has_same_identity(&y));
    }
    #[test]
    fn structural_unit_serialization_uses_material_directly() {
        let unit = StructuralUnit::new(
            "Carbon",
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        );
        let encoded = serde_json::to_string(&unit).unwrap();
        assert!(encoded.contains("\"material\":{\"parts\""));
        let decoded: StructuralUnit = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded.material, unit.material);
    }
    #[test]
    fn legacy_structural_material_wrapper_deserializes() {
        let encoded = r#"{"physical_id":0,"material":{"material":{"parts":[["Carbon",1.0]],"internal_bonds":[]}},"placement":{"x":0.0,"y":0.0,"rotation_radians":0.0}}"#;
        let decoded: StructuralUnit = serde_json::from_str(encoded).unwrap();
        assert_eq!(decoded.material, Material::free_base("Carbon", 1.0));
        assert!(decoded.geometry.is_none());
    }
    #[test]
    fn composite_material_is_rejected_as_structural_material() {
        let material = Material {
            parts: vec![("Carbon".into(), 2.0), ("Hydrogen".into(), 3.0)],
            internal_bonds: vec![],
        };
        assert!(StructuralUnit::from_material(
            material,
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0
            }
        )
        .is_none());
    }
}
