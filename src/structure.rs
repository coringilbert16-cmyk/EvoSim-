#![expect(dead_code, reason = "Staged API retained for subsystem integration")]
use crate::connection_geometry::WorldConnectionPoint;
use crate::physical_geometry::PhysicalGeometry;
use crate::resources::{BaseResource, Material};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashSet;
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
        if !self.material.is_valid()
            || !self
                .material
                .parts
                .iter()
                .all(|(name, _)| catalog.iter().any(|base| base.name == *name))
        {
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
        let Some(shape) = self.shape(catalog).cloned() else {
            return false;
        };
        self.geometry = Some(PhysicalGeometry::from_default(&shape));
        true
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
                let crate::resources::Form::Circle { radius } = unit.shape(catalog)?.form else {
                    return None;
                };
                let (nx, ny) = (angle_radians.cos(), angle_radians.sin());
                Some(crate::connection_geometry::transform_derived_point(
                    radius * nx,
                    radius * ny,
                    nx,
                    ny,
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
    if !crate::material_geometry::placed_forms_overlap(&pa, &pb, 0.0) {
        return false;
    }
    let dx = b.placement.x - a.placement.x;
    let dy = b.placement.y - a.placement.y;
    let distance = dx.hypot(dy);
    let (sx, sy) = if distance > 1e-12 {
        (dx / distance, dy / distance)
    } else {
        (1.0, 0.0)
    };
    let scale = pa
        .form
        .bounding_radius()
        .max(pb.form.bounding_radius())
        .max(1.0);
    let epsilon = 1e-8 * scale;
    let shifted = crate::material_geometry::PlacedMaterialPart {
        part_index: 0,
        form: pa.form.clone(),
        placement: Placement {
            x: a.placement.x - sx * epsilon,
            y: a.placement.y - sy * epsilon,
            rotation_radians: a.placement.rotation_radians,
        },
    };
    crate::material_geometry::placed_forms_overlap(&shifted, &pb, 0.0)
}
#[derive(Serialize, Clone, Debug)]
pub struct PhysicalConstituentGraph {
    pub units: Vec<StructuralUnit>,
    pub bonds: Vec<Bond>,
    /// Stable physical constituents that established the genome cavity.
    /// Structural membership is derived from connectivity to these IDs.
    #[serde(default)]
    genome_constituent_ids: Vec<PhysicalConstituentId>,
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
        Ok(Self {
            units: s.units,
            bonds: s.bonds,
            genome_constituent_ids: s
                .genome_constituent_ids
                .into_iter()
                .filter(|id| id.0 != 0 && used.contains(id))
                .collect(),
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
    pub fn set_genome_constituent_ids(&mut self, ids: impl IntoIterator<Item = PhysicalConstituentId>) {
        let mut ids: Vec<_> = ids.filter(|id| id.0 != 0).collect();
        ids.sort_unstable_by_key(|id| id.0);
        ids.dedup();
        ids.retain(|id| self.unit_index(*id).is_some());
        self.genome_constituent_ids = ids;
    }

    pub fn clear_genome_constituent_ids(&mut self) {
        self.genome_constituent_ids.clear();
    }

    /// The structural body is the connected component containing the persisted
    /// physical genome constituents. Disconnected material is not structural
    /// merely because it remains inside the organism boundary.
    pub fn structural_unit_indices(&self, catalog: &[BaseResource]) -> Vec<usize> {
        let Some(&genome_id) = self.genome_constituent_ids.first() else {
            return Vec::new();
        };
        let Some(start) = self.unit_index(genome_id) else {
            return Vec::new();
        };
        self.connected_component_containing(start)
    }

    pub fn genome_connected(&self, unit_index: usize) -> bool {
        if unit_index >= self.units.len() || self.genome_constituent_ids.is_empty() {
            return false;
        }
        self.connected_component_containing(unit_index)
            .into_iter()
            .any(|index| self.genome_constituent_ids.contains(&self.units[index].physical_id))
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

    pub fn has_direct_nonfluid_neighbor(
        &self,
        unit_index: usize,
        catalog: &[BaseResource],
    ) -> bool {
        self.direct_neighbor_indices(unit_index).into_iter().any(|neighbor| {
            let is_genome = self
                .genome_constituent_ids
                .contains(&self.units[neighbor].physical_id);
            is_genome || self.units[neighbor]
                .material
                .parts
                .first()
                .and_then(|(name, _)| catalog.iter().find(|resource| resource.name == *name))
                .is_some_and(|resource| {
                    resource.physical_state != crate::resources::PhysicalState::Fluid
                })
        })
    }

    pub fn is_structurally_qualified(
        &self,
        unit_index: usize,
        catalog: &[BaseResource],
    ) -> bool {
        if !self.genome_connected(unit_index) {
            return false;
        }
        let Some((name, _)) = self.units.get(unit_index).and_then(|unit| unit.material.parts.first()) else {
            return false;
        };
        let Some(resource) = catalog.iter().find(|resource| resource.name == *name) else {
            return false;
        };
        if resource.physical_state != crate::resources::PhysicalState::Fluid {
            return true;
        }
        self.has_direct_nonfluid_neighbor(unit_index, catalog)
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
    fn structural_membership_uses_persisted_physical_genome_ids() {
        let mut s = OrganismStructure::new();
        let a = s.add_unit(StructuralUnit::new("Carbon", Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 }));
        let b = s.add_unit(StructuralUnit::new("Carbon", Placement { x: 2.0, y: 0.0, rotation_radians: 0.0 }));
        let ida = s.physical_id(a).unwrap();
        let idb = s.physical_id(b).unwrap();
        s.set_genome_constituent_ids([ida]);
        assert_eq!(s.structural_unit_indices(&crate::resources::default_catalog()), vec![a]);
        assert!(s.genome_connected(a));
        assert!(!s.genome_connected(b));
        s.push_bond_unchecked(Bond {
            endpoint_a: BondEndpoint::new(ida, ConnectionEndpoint::Boundary { angle_radians: 0.0 }),
            endpoint_b: BondEndpoint::new(idb, ConnectionEndpoint::Boundary { angle_radians: std::f64::consts::PI }),
            strength: 0.5,
            bond_energy: 1.0,
        });
        assert_eq!(s.structural_unit_indices(&crate::resources::default_catalog()), vec![a, b]);
        assert!(s.genome_connected(b));
    }

    #[test]
    fn direct_nonfluid_neighbor_qualifies_fluid_water() {
        let catalog = crate::resources::default_catalog();
        let mut s = OrganismStructure::new();
        let genome = s.add_unit(StructuralUnit::new("Carbon", Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 }));
        let water = s.add_unit(StructuralUnit::new("Water", Placement { x: 1.0, y: 0.0, rotation_radians: 0.0 }));
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
    fn water_water_bond_does_not_qualify_second_water() {
        let catalog = crate::resources::default_catalog();
        let mut s = OrganismStructure::new();
        let genome = s.add_unit(StructuralUnit::new("Carbon", Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 }));
        let w1 = s.add_unit(StructuralUnit::new("Water", Placement { x: 1.0, y: 0.0, rotation_radians: 0.0 }));
        let w2 = s.add_unit(StructuralUnit::new("Water", Placement { x: 2.0, y: 0.0, rotation_radians: 0.0 }));
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
        assert!(!s.is_structurally_qualified(w2, &catalog));
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
    fn genome_constituent_ids_round_trip_through_serialization() {
        let mut s = OrganismStructure::new();
        let unit = s.add_unit(StructuralUnit::new(
            "Carbon",
            Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 },
        ));
        let id = s.physical_id(unit).unwrap();
        s.set_genome_constituent_ids([id]);
        let encoded = serde_json::to_string(&s).unwrap();
        let decoded: OrganismStructure = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded.genome_constituent_ids(), &[id]);
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
