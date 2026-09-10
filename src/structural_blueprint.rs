//! Inherited structural blueprint.
//!
//! The genome specifies material composition, element frames, and which
//! elements should connect. Physical constituent geometry and bonds are
//! realized by the construction/contact systems and become the authority.

use crate::resources::{BaseResource, InternalBond, Material};
use crate::structure::{Bond, BondEndpoint, OrganismStructure};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::{HashMap, HashSet};

fn default_core_elements() -> Vec<usize> {
    vec![0]
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct BlueprintPlacement {
    pub x: f64,
    pub y: f64,
    #[serde(default)]
    pub rotation_radians: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct StructuralBlueprint {
    pub elements: Vec<BlueprintElement>,
    pub connections: Vec<BlueprintConnection>,
    #[serde(default = "default_core_elements")]
    pub core_elements: Vec<usize>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct BlueprintElement {
    pub material: Material,
    /// The rigid spatial frame in which this material is realized.
    pub placement: BlueprintPlacement,
}

impl<'de> Deserialize<'de> for BlueprintElement {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct StoredPlacement {
            x: f64,
            y: f64,
            #[serde(default)]
            rotation_radians: Option<f64>,
        }

        #[derive(Deserialize)]
        struct Stored {
            material: Material,
            placement: StoredPlacement,
        }

        let stored = Stored::deserialize(deserializer)?;
        Ok(Self {
            material: stored.material,
            placement: BlueprintPlacement {
                x: stored.placement.x,
                y: stored.placement.y,
                rotation_radians: stored.placement.rotation_radians.unwrap_or(0.0),
            },
        })
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlueprintConnection {
    pub element_a: usize,
    pub element_b: usize,
}

impl BlueprintConnection {
    pub fn canonical(self) -> Self {
        if self.element_a <= self.element_b {
            self
        } else {
            Self {
                element_a: self.element_b,
                element_b: self.element_a,
            }
        }
    }

    fn validate(&self, b: &StructuralBlueprint) -> Result<(), String> {
        if self.element_a >= b.elements.len() || self.element_b >= b.elements.len() {
            return Err("references a missing element".into());
        }
        if self.element_a == self.element_b {
            return Err("self-connections are not permitted".into());
        }
        if *self != self.canonical() {
            return Err("blueprint connections must use canonical element ordering".into());
        }
        Ok(())
    }
}

impl StructuralBlueprint {
    pub fn new(
        elements: Vec<BlueprintElement>,
        connections: Vec<BlueprintConnection>,
    ) -> Self {
        Self {
            elements,
            connections: Self::canonical_connections(connections),
            core_elements: default_core_elements(),
        }
    }

    pub fn with_core_elements(
        elements: Vec<BlueprintElement>,
        connections: Vec<BlueprintConnection>,
        core_elements: Vec<usize>,
    ) -> Self {
        Self {
            elements,
            connections: Self::canonical_connections(connections),
            core_elements,
        }
    }

    fn canonical_connections(
        connections: Vec<BlueprintConnection>,
    ) -> Vec<BlueprintConnection> {
        let mut seen = HashSet::new();
        connections
            .into_iter()
            .map(BlueprintConnection::canonical)
            .filter(|connection| seen.insert((connection.element_a, connection.element_b)))
            .collect()
    }

    pub fn is_valid(&self) -> bool {
        self.validate().is_ok()
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.elements.is_empty() {
            return Err("blueprint must contain at least one element".into());
        }
        if self.core_elements.is_empty() {
            return Err("blueprint must define a genome core".into());
        }

        let mut core_seen = vec![false; self.elements.len()];
        for &index in &self.core_elements {
            if index >= self.elements.len() {
                return Err("genome core references a missing element".into());
            }
            if core_seen[index] {
                return Err("genome core contains a duplicate element".into());
            }
            core_seen[index] = true;
        }

        let mut connection_seen = HashSet::new();
        for (index, element) in self.elements.iter().enumerate() {
            element
                .validate()
                .map_err(|error| format!("element {index}: {error}"))?;
        }
        for (index, connection) in self.connections.iter().enumerate() {
            connection
                .validate(self)
                .map_err(|error| format!("connection {index}: {error}"))?;
            if !connection_seen.insert((connection.element_a, connection.element_b)) {
                return Err("blueprint contains duplicate connections".into());
            }
        }
        if self.elements.len() > 1 && !self.is_connected() {
            return Err("multi-element blueprint must be connected".into());
        }
        if self.core_elements.len() > 1 && !self.core_is_connected() {
            return Err("genome core must be connected".into());
        }
        Ok(())
    }

    pub fn realize(&self, catalog: &[BaseResource]) -> Result<OrganismStructure, String> {
        self.validate()?;

        let mut structure = OrganismStructure::new();
        let mut realized = HashMap::<usize, Vec<usize>>::new();

        // Each material is constructed independently inside its authored rigid
        // frame. Blueprint connections are graph-level intent and are resolved
        // only after every physical element exists.
        for (index, element) in self.elements.iter().enumerate() {
            let ids = crate::construction_realization::realize_material(
                &mut structure,
                element,
                catalog,
            )?;
            apply_blueprint_orientation(&structure, &ids, element.placement, catalog)?;
            realized.insert(index, ids);
        }

        for connection in &self.connections {
            realize_connection_groups(&mut structure, &realized, *connection, catalog)
                .map_err(|error| {
                    format!(
                        "blueprint connection {}-{} failed: {error}",
                        connection.element_a, connection.element_b
                    )
                })?;
        }

        Ok(structure)
    }

    pub fn is_connected(&self) -> bool {
        if self.elements.is_empty() {
            return false;
        }
        let mut visited = vec![false; self.elements.len()];
        let mut stack = vec![0usize];
        visited[0] = true;
        while let Some(current) = stack.pop() {
            for connection in &self.connections {
                let next = if connection.element_a == current {
                    connection.element_b
                } else if connection.element_b == current {
                    connection.element_a
                } else {
                    continue;
                };
                if next < visited.len() && !visited[next] {
                    visited[next] = true;
                    stack.push(next);
                }
            }
        }
        visited.into_iter().all(|visited| visited)
    }

    fn core_is_connected(&self) -> bool {
        let core = self.core_elements.iter().copied().collect::<HashSet<_>>();
        let mut visited = HashSet::new();
        let mut stack = vec![self.core_elements[0]];
        visited.insert(self.core_elements[0]);
        while let Some(current) = stack.pop() {
            for connection in &self.connections {
                let next = if connection.element_a == current {
                    connection.element_b
                } else if connection.element_b == current {
                    connection.element_a
                } else {
                    continue;
                };
                if core.contains(&next) && visited.insert(next) {
                    stack.push(next);
                }
            }
        }
        visited.len() == core.len()
    }

    pub fn total_material_amount(&self) -> f64 {
        self.elements
            .iter()
            .map(|element| element.material.total_amount())
            .sum()
    }

    pub fn structural_mass(&self, catalog: &[BaseResource]) -> f64 {
        self.elements
            .iter()
            .map(|element| element.material.mass(catalog))
            .sum()
    }
}

fn apply_blueprint_orientation(
    structure: &OrganismStructure,
    ids: &[usize],
    placement: BlueprintPlacement,
    catalog: &[BaseResource],
) -> Result<(), String> {
    for &id in ids {
        let unit = structure
            .units
            .get(id)
            .ok_or_else(|| "orientation references a missing constituent".to_string())?;
        if (unit.placement.rotation_radians - placement.rotation_radians).abs() > 1e-12 {
            return Err("realized material does not preserve the prescribed element frame".into());
        }

        for (other_index, other) in structure.units.iter().enumerate() {
            if ids.contains(&other_index) {
                continue;
            }
            let Some(a) = unit.shape(catalog) else { continue };
            let Some(b) = other.shape(catalog) else { continue };
            let pa = crate::material_geometry::PlacedMaterialPart {
                part_index: id,
                form: a.form.clone(),
                placement: unit.placement,
            };
            let pb = crate::material_geometry::PlacedMaterialPart {
                part_index: other_index,
                form: b.form.clone(),
                placement: other.placement,
            };
            if crate::material_geometry::placed_forms_penetrate(&pa, &pb, 0.0) {
                return Err("realized material penetrates existing physical structure".into());
            }
        }
    }
    Ok(())
}

pub(crate) fn realize_connection_groups(
    structure: &mut OrganismStructure,
    realized: &HashMap<usize, Vec<usize>>,
    connection: BlueprintConnection,
    catalog: &[BaseResource],
) -> Result<f64, String> {
    let a = realized
        .get(&connection.element_a)
        .ok_or_else(|| "missing realized first blueprint element".to_string())?;
    let b = realized
        .get(&connection.element_b)
        .ok_or_else(|| "missing realized second blueprint element".to_string())?;

    for &ua in a {
        for &ub in b {
            let Some(pa) = structure.units.get(ua).and_then(|unit| unit.properties(catalog)) else {
                continue;
            };
            let Some(pb) = structure.units.get(ub).and_then(|unit| unit.properties(catalog)) else {
                continue;
            };

            let mut cache = crate::contact::ConnectionCompatibilityCache::new();
            let mut candidates = crate::contact::connection_pair_candidates_cached(
                structure,
                ua,
                ub,
                catalog,
                &mut cache,
            );
            candidates.sort_by(|left, right| {
                left.distance
                    .total_cmp(&right.distance)
                    .then_with(|| right.facing.total_cmp(&left.facing))
            });

            let id_a = structure
                .physical_id(ua)
                .ok_or_else(|| "missing first physical constituent".to_string())?;
            let id_b = structure
                .physical_id(ub)
                .ok_or_else(|| "missing second physical constituent".to_string())?;

            for candidate in candidates {
                if candidate.distance > 1e-9 {
                    continue;
                }
                let evaluation =
                    crate::combine::evaluate_formation(candidate, pa.cohesion, pb.cohesion);
                let (_, work, _) = crate::combine::required_investment(
                    pa,
                    pb,
                    evaluation,
                    0.0,
                )
                .map_err(|error| format!("formation investment failed: {error:?}"))?;
                let strength = crate::combine::bond_strength(pa, pb);
                if !strength.is_finite() || !(0.0..=1.0).contains(&strength) {
                    continue;
                }
                let bond = Bond {
                    endpoint_a: BondEndpoint::new(id_a, candidate.endpoint_a),
                    endpoint_b: BondEndpoint::new(id_b, candidate.endpoint_b),
                    strength,
                    bond_energy: 0.0,
                };
                if structure
                    .bonds
                    .iter()
                    .any(|existing| existing.has_same_identity(&bond))
                {
                    continue;
                }
                let mut trial = structure.clone();
                if crate::contact::try_add_bond(&mut trial, bond, catalog).is_ok() {
                    *structure = trial;
                    return Ok(work);
                }
            }
        }
    }

    Err("no physically admissible endpoint pair for blueprint connection".into())
}

impl BlueprintElement {
    pub fn validate(&self) -> Result<(), String> {
        if !self.material.is_valid() {
            return Err("material is invalid".into());
        }
        if !self.placement.x.is_finite()
            || !self.placement.y.is_finite()
            || !self.placement.rotation_radians.is_finite()
        {
            return Err("placement must be finite".into());
        }
        if self
            .material
            .parts
            .iter()
            .any(|(_, amount)| (*amount - 1.0).abs() > f64::EPSILON)
        {
            return Err("each blueprint constituent must represent exactly one material unit".into());
        }
        if self.material.parts.len() == 1 && !self.material.has_internal_structure() {
            return Ok(());
        }
        if !self.material.has_internal_structure() {
            return Err("multi-constituent structural material must have internal bonds".into());
        }
        if !material_structure_is_connected(&self.material) {
            return Err("internal structural material must be connected".into());
        }
        Ok(())
    }
}

fn material_structure_is_connected(material: &Material) -> bool {
    if material.parts.len() <= 1 {
        return true;
    }
    let mut visited = vec![false; material.parts.len()];
    let mut stack = vec![0usize];
    visited[0] = true;
    while let Some(current) = stack.pop() {
        for InternalBond { part_a, part_b } in &material.internal_bonds {
            let next = if *part_a == current {
                *part_b
            } else if *part_b == current {
                *part_a
            } else {
                continue;
            };
            if next < visited.len() && !visited[next] {
                visited[next] = true;
                stack.push(next);
            }
        }
    }
    visited.into_iter().all(|visited| visited)
}
