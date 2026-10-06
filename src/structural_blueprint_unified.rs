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
    /// Legacy blueprint-side reference for the minimum genome scale. Runtime
    /// genesis now uses a real temporary three-Carbon physical scaffold; this
    /// field remains only for blueprint/juvenile compatibility.
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
    /// Three Carbon reference pieces arranged as two below and one above, with
    /// flat-to-flat contact between each neighboring pair. This blueprint-side
    /// reference mirrors the runtime genesis scaffold geometry.
    pub(crate) placements: [BlueprintPlacement; 3],
    /// The three reference Carbon pieces form the triangular three-bond graph;
    /// runtime genesis realizes this graph as physical temporary material.
    pub(crate) bonds: [(usize, usize); 3],
    /// The geometric reference is derived from the actual scaffold material,
    /// rather than being independently re-derived by cavity qualification.
    /// This keeps the temporary scaffold as the single authority for the
    /// minimum genome scale.
    pub(crate) reference_area: f64,
}

impl GenomeMeasurementScaffold {
    pub(crate) fn reference_area(&self) -> f64 {
        self.reference_area
    }
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
        // Carbon is a rigid regular hexagon. The scaffold is physical:
        // neighboring Carbon pieces touch edge-to-edge without overlapping.
        // For the default hexagon, the center spacing is the apothem doubled,
        // sqrt(3) * radius. The three centers therefore form an equilateral
        // triangle whose central triangular gap is the measured reference.
        let sides = match carbon.shape.form {
            crate::resources::Form::RegularPolygon { sides, .. } => sides,
            _ => return Err("Carbon genome measurement requires a polygonal Carbon shape".into()),
        };
        if sides != 6 {
            return Err("Carbon genome measurement requires a hexagonal Carbon shape".into());
        }
        let side = 3.0_f64.sqrt() * radius;
        let circumradius = side / 3.0_f64.sqrt();
        // The three pairwise contact points form an equilateral triangle with
        // side = side / 2. Its area is the actual empty region left by the
        // temporary three-carbon scaffold.
        let gap_side = side * 0.5;
        let reference_area = 3.0_f64.sqrt() * gap_side * gap_side / 4.0;
        if !reference_area.is_finite() || reference_area <= 0.0 {
            return Err("invalid three-carbon genome reference area".into());
        }
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
            reference_area,
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
