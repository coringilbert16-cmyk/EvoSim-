use crate::resources::{BaseResource, Material};
use crate::structure::OrganismStructure;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlueprintPlacement {
    pub x: f64,
    pub y: f64,
    pub rotation_radians: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BlueprintElement {
    pub material: Material,
    pub placement: BlueprintPlacement,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BlueprintConnection {
    pub element_a: usize,
    pub element_b: usize,
}

#[derive(Clone, Debug, Default)]
pub struct StructuralBlueprint {
    pub elements: Vec<BlueprintElement>,
    pub connections: Vec<BlueprintConnection>,
    pub core_elements: Vec<usize>,
}

impl StructuralBlueprint {
    pub fn realize(
        &self,
        structure: &OrganismStructure,
        catalog: &[BaseResource],
        ledger: &mut crate::state::EnergyLedger,
        energy: &mut f64,
    ) -> Result<(OrganismStructure, f64), String> {
        let mut structure = structure.clone();
        let mut realized: HashMap<usize, Vec<usize>> = HashMap::new();
        let mut total_heat = 0.0;

        for index in 0..self.elements.len() {
            let external = self
                .connections
                .iter()
                .filter_map(|connection| {
                    let neighbor = if connection.element_a == index {
                        connection.element_b
                    } else if connection.element_b == index {
                        connection.element_a
                    } else {
                        return None;
                    };
                    realized.get(&neighbor).cloned()
                })
                .collect::<Vec<_>>();
            let (ids, heat) = crate::construction_runtime::realize_material_with_context(
                &mut structure,
                &self.elements[index],
                catalog,
                ledger,
                energy,
                &external,
            )?;
            validate_element_contact(&structure, &ids, &external, catalog)?;
            realized.insert(index, ids);
            total_heat += heat;
        }

        Ok((structure, total_heat))
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
        if self.core_elements.is_empty() {
            return false;
        }
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

fn validate_element_contact(
    structure: &OrganismStructure,
    ids: &[usize],
    neighbors: &[Vec<usize>],
    catalog: &[BaseResource],
) -> Result<(), String> {
    for &id in ids {
        if structure.units.get(id).is_none() {
            return Err("realized material references a missing constituent".to_string());
        }
    }
    for group in neighbors {
        if !ids.iter().any(|&a| {
            group.iter().any(|&b| {
                crate::contact::contacting_connection_pair_candidates(
                    structure, a, b, catalog, 1.0, 0.0,
                )
                .iter()
                .any(|candidate| candidate.available_a && candidate.available_b)
            })
        }) {
            return Err(
                "realized material has no physical contact with a prescribed neighbor".into(),
            );
        }
    }
    Ok(())
}
