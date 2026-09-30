#![expect(
    dead_code,
    reason = "Staged blueprint API retained for subsystem integration"
)]
//! Unified inherited structural blueprint authority.
//!
//! Blueprint intent is separate from physical realization. Actual constituent
//! and bond creation is delegated to the construction runtime, which delegates
//! every bond admission to COMBINE. `realize()` is a local, non-persistent
//! preview; simulation construction must use `realize_with_context()`.

use crate::resources::{BaseResource, Material};
use crate::state::EnergyLedger;
use crate::structure::OrganismStructure;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashSet;

fn default_anchor_elements() -> Vec<usize> {
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
    /// Transient construction-only genome measurement piece. It is never
    /// serialized, bonded, acquired, or retained in the organism structure.
    #[serde(skip)]
    pub(crate) genome_measurement: Option<GenomeMeasurementScaffold>,
    /// Construction anchors identify where realization may begin. They are
    /// not a biological genome definition and do not identify the genome.
    #[serde(default = "default_anchor_elements")]
    pub anchor_elements: Vec<usize>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct BlueprintElement {
    pub material: Material,
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
impl BlueprintElement {
    pub fn validate(&self) -> Result<(), String> {
        if !self.material.is_valid() {
            return Err("material is invalid".into());
        }
        if !self.placement.x.is_finite() || !self.placement.y.is_finite() {
            return Err("blueprint placement must be finite".into());
        }
        if !self.placement.rotation_radians.is_finite() {
            return Err("blueprint orientation must be finite".into());
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlueprintConnection {
    pub element_a: usize,
    pub element_b: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GenomeMeasurementScaffold {
    /// Three bonded Carbon guide pieces. The guide occupies real construction
    /// volume but has no organism units or bonds of its own.
    pub(crate) placements: [BlueprintPlacement; 3],
    /// The temporary three-carbon reference is itself a triangle: all three
    /// Carbon pieces are internally bonded, with each edge contributing to the
    /// cavity measurement.
    pub(crate) bonds: [(usize, usize); 3],
}

impl GenomeMeasurementScaffold {
    pub(crate) fn three_carbon_reference(catalog: &[BaseResource]) -> Result<Self, String> {
        let carbon = catalog
            .iter()
            .find(|resource| resource.name == "Carbon")
            .ok_or_else(|| "catalog has no Carbon resource".to_string())?;
        let radius = match carbon.shape.form {
            crate::resources::Form::RegularPolygon { radius, .. } => radius,
            _ => return Err("Carbon genome measurement requires a polygonal Carbon shape".into()),
        };
        // The measurement piece is a compact three-carbon reference, not a
        // diameter laid across the future cavity. An equilateral arrangement
        // preserves the carbon-derived spacing while keeping the temporary
        // scaffold inside the smallest intended genome cavity.
        let spacing = radius * 3.0_f64.sqrt();
        let circumradius = spacing / 3.0_f64.sqrt();
        Ok(Self {
            placements: [
                BlueprintPlacement {
                    x: 0.0,
                    y: circumradius,
                    rotation_radians: 0.0,
                },
                BlueprintPlacement {
                    x: -circumradius * (3.0_f64).sqrt() / 2.0,
                    y: -circumradius / 2.0,
                    rotation_radians: 0.0,
                },
                BlueprintPlacement {
                    x: circumradius * (3.0_f64).sqrt() / 2.0,
                    y: -circumradius / 2.0,
                    rotation_radians: 0.0,
                },
            ],
            bonds: [(0, 1), (1, 2), (2, 0)],
        })
    }
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
    pub fn new(elements: Vec<BlueprintElement>, connections: Vec<BlueprintConnection>) -> Self {
        Self {
            elements,
            connections: Self::canonical_connections(connections),
            anchor_elements: default_anchor_elements(),
            genome_measurement: None,
        }
    }

    pub fn with_anchor_elements(
        elements: Vec<BlueprintElement>,
        connections: Vec<BlueprintConnection>,
        anchor_elements: Vec<usize>,
    ) -> Self {
        Self {
            elements,
            connections: Self::canonical_connections(connections),
            anchor_elements,
            genome_measurement: None,
        }
    }

    fn canonical_connections(connections: Vec<BlueprintConnection>) -> Vec<BlueprintConnection> {
        let mut seen = HashSet::new();
        connections
            .into_iter()
            .map(BlueprintConnection::canonical)
            .filter(|c| seen.insert((c.element_a, c.element_b)))
            .collect()
    }

    pub(crate) fn with_genome_measurement(mut self, scaffold: GenomeMeasurementScaffold) -> Self {
        self.genome_measurement = Some(scaffold);
        self
    }

    pub fn is_valid(&self) -> bool {
        self.validate().is_ok()
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.elements.is_empty() {
            return Err("blueprint must contain at least one element".into());
        }
        if self.anchor_elements.is_empty() {
            return Err("blueprint must define at least one construction anchor".into());
        }
        let mut anchor_seen = vec![false; self.elements.len()];
        for &index in &self.anchor_elements {
            if index >= self.elements.len() {
                return Err("construction anchor references a missing element".into());
            }
            if anchor_seen[index] {
                return Err("construction anchors contain a duplicate element".into());
            }
            anchor_seen[index] = true;
        }
        let mut connection_seen = HashSet::new();
        for (i, e) in self.elements.iter().enumerate() {
            e.validate()
                .map_err(|error| format!("element {i}: {error}"))?;
        }
        for (i, c) in self.connections.iter().enumerate() {
            c.validate(self)
                .map_err(|error| format!("connection {i}: {error}"))?;
            if !connection_seen.insert((c.element_a, c.element_b)) {
                return Err("blueprint contains duplicate connections".into());
            }
        }
        if self.elements.len() > 1 && !self.is_connected() {
            return Err("multi-element blueprint must be connected".into());
        }
        Ok(())
    }

    /// Non-persistent physical preview. It uses a private trial energy budget
    /// solely so COMBINE can evaluate its real admission rules. No simulation
    /// ledger, organism energy, or structure is mutated by this method.
    pub fn realize(&self, catalog: &[BaseResource]) -> Result<OrganismStructure, String> {
        let mut preview = self.clone();
        preview.genome_measurement = None;
        let mut ledger = EnergyLedger::default();
        let mut preview_energy = 1.0e12;
        preview
            .realize_with_context(catalog, &mut ledger, &mut preview_energy)
            .map(|(structure, _, _)| structure)
    }

    /// Actual blueprint realization. All elements share one energy holder and
    /// one ledger; each material's internal and external bonds are admitted by
    /// the same COMBINE runtime.
    pub fn realize_with_context(
        &self,
        catalog: &[BaseResource],
        ledger: &mut EnergyLedger,
        energy: &mut f64,
    ) -> Result<(OrganismStructure, f64, f64), String> {
        self.validate()?;
        let (structure, total_heat) = crate::construction_runtime::construct_blueprint_bond_driven(
            self, catalog, ledger, energy,
        )?;
        Ok((structure, total_heat, *energy))
    }

    /// Realize this developmental blueprint from actual physical inventory.
    /// The blueprint supplies structural preference; the inventory supplies
    /// what can actually be built with. A material mismatch below the
    /// construction threshold is returned as a construction-material need.
    pub fn realize_with_materials(
        &self,
        catalog: &[BaseResource],
        available_materials: &mut crate::material_storage::MaterialStorage,
        ledger: &mut EnergyLedger,
        energy: &mut f64,
    ) -> Result<(OrganismStructure, f64, f64), String> {
        self.validate()?;
        let (structure, total_heat) =
            crate::construction_runtime::construct_blueprint_bond_driven_with_materials(
                self,
                catalog,
                available_materials,
                ledger,
                energy,
            )?;
        Ok((structure, total_heat, *energy))
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genome_measurement_scaffold_is_an_equilateral_three_bond_triangle() {
        let catalog = crate::resources::default_catalog();
        let scaffold = GenomeMeasurementScaffold::three_carbon_reference(&catalog).unwrap();

        assert_eq!(scaffold.bonds, [(0, 1), (1, 2), (2, 0)]);

        let points = scaffold.placements;
        let d01 = (points[0].x - points[1].x).hypot(points[0].y - points[1].y);
        let d12 = (points[1].x - points[2].x).hypot(points[1].y - points[2].y);
        let d20 = (points[2].x - points[0].x).hypot(points[2].y - points[0].y);

        assert!((d01 - d12).abs() <= 1e-10);
        assert!((d12 - d20).abs() <= 1e-10);
        assert!(d01 > 0.0);
    }
}
