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
        let _profile = crate::property_profile::measure(
            crate::property_profile::Metric::StructuralUnitProperties,
        );
        if !crate::property_profile::measure_call(
            crate::property_profile::Metric::StructuralUnitValidationSite,
            || self.material.is_valid_for_catalog(catalog),
        ) {
            return None;
        }
        Some(
            self.material
                .weighted_properties_after_catalog_validation(catalog),
        )
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
                    || !crate::property_profile::measure_call(
                        crate::property_profile::Metric::StructuralUnitGeometryValidationSite,
                        || self.material.is_valid_for_catalog(catalog),
                    )
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
                let is_line_like = match &shape.form {
                    crate::resources::Form::Line { .. } => true,
                    crate::resources::Form::Rectangle { width, height } => {
                        unit.material.parts.len() == 1
                            && unit.material.parts[0].0 == "Hydrogen"
                            && *height <= *width * 0.1 + 1e-12
                    }
                    _ => false,
                };
                if !is_line_like {
                    return None;
                }
                crate::connection_geometry::rigid_endpoint_world_point(
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
    /// Local bond integrity derived from endpoint intrinsic cohesion.
    /// Kept under the serialized `strength` name for compatibility; this is
    /// not pair adhesion, reactivity, or stored bond potential.
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
        let destination_was_empty = self.units.is_empty();
        let source_ids: HashSet<_> = source.units.iter().map(|unit| unit.physical_id).collect();
        let destination_ids: HashSet<_> = self.units.iter().map(|unit| unit.physical_id).collect();
        let source_max_id = source_ids.iter().map(|id| id.0).max().unwrap_or(0);
        let endpoint_is_finite = |endpoint: ConnectionEndpoint| match endpoint {
            ConnectionEndpoint::Boundary { angle_radians } => angle_radians.is_finite(),
            ConnectionEndpoint::Fluid { x, y } => x.is_finite() && y.is_finite(),
            ConnectionEndpoint::Corner { .. } | ConnectionEndpoint::LineEndpoint { .. } => true,
        };
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
                    || !endpoint_is_finite(bond.endpoint_a.location)
                    || !endpoint_is_finite(bond.endpoint_b.location)
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
        if destination_was_empty
            && staged.genome_constituent_ids.is_empty()
            && !source_primary.is_empty()
        {
            staged.set_genome_constituent_ids(source_primary);
        } else if !source_primary.is_empty() {
            // Appending a new organism into an existing physical aggregate must
            // not silently promote its core to the aggregate's primary genome.
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
            || selected.iter().any(|id| self.unit_index(*id).is_none())
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

    /// Atomically replace an aggregate-owned physical group and return its
    /// refreshed graph-local view. Newly allocated constituent IDs are remapped
    /// before extraction, so the returned view and authoritative aggregate agree.
    pub(crate) fn replace_subgraph_remapping_new_ids_and_extract(
        &mut self,
        old_ids: &[PhysicalConstituentId],
        replacement: &Self,
    ) -> Option<(HashMap<PhysicalConstituentId, PhysicalConstituentId>, Self)> {
        let mut staged = self.clone();
        let remapping = staged.replace_subgraph_remapping_new_ids(old_ids, replacement)?;
        let selected_ids: Vec<_> = replacement
            .physical_constituent_ids()
            .iter()
            .map(|id| remapping.get(id).copied())
            .collect::<Option<_>>()?;
        let primary_core_ids: Vec<_> = replacement
            .genome_constituent_ids
            .iter()
            .map(|id| remapping.get(id).copied())
            .collect::<Option<_>>()?;
        let refreshed_view = staged.subgraph_for_physical_ids(&selected_ids, &primary_core_ids)?;
        *self = staged;
        Some((remapping, refreshed_view))
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

    /// Build a read-only physical view of the aggregate with one complete
    /// constituent group omitted. Cross-boundary bonds are omitted from the view;
    /// the authoritative aggregate remains unchanged.
    pub(crate) fn view_without_physical_ids(
        &self,
        excluded_ids: &[PhysicalConstituentId],
    ) -> Option<Self> {
        let excluded: HashSet<_> = excluded_ids.iter().copied().collect();
        let all_ids: HashSet<_> = self.units.iter().map(|unit| unit.physical_id).collect();
        let max_id = all_ids.iter().map(|id| id.0).max().unwrap_or(0);
        let endpoint_is_finite = |endpoint: ConnectionEndpoint| match endpoint {
            ConnectionEndpoint::Boundary { angle_radians } => angle_radians.is_finite(),
            ConnectionEndpoint::Fluid { x, y } => x.is_finite() && y.is_finite(),
            ConnectionEndpoint::Corner { .. } | ConnectionEndpoint::LineEndpoint { .. } => true,
        };