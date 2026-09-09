use serde::{Deserialize, Serialize};
use crate::resources::{BaseResource, Material};
use crate::structure::{Bond, OrganismStructure, Placement, StructuralUnit};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BlueprintElement {
    pub material: Material,
    pub placement: Placement,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlueprintConnection {
    pub element_a: usize,
    pub point_a: usize,
    pub element_b: usize,
    pub point_b: usize,
}

impl BlueprintConnection {
    fn validate(&self, b: &StructuralBlueprint) -> Result<(), String> {
        if self.element_a >= b.elements.len() || self.element_b >= b.elements.len() {
            return Err("connection references an invalid element".into());
        }
        if self.element_a == self.element_b {
            return Err("connection cannot join an element to itself".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct StructuralBlueprint {
    pub elements: Vec<BlueprintElement>,
    pub connections: Vec<BlueprintConnection>,
    #[serde(default)]
    pub core_elements: Vec<usize>,
}

impl StructuralBlueprint {
    pub fn validate(&self) -> Result<(), String> {
        for (i, e) in self.elements.iter().enumerate() {
            e.material
                .is_valid()
                .then_some(())
                .ok_or_else(|| format!("element {i}: invalid structural material"))?;
        }
        for (i, c) in self.connections.iter().enumerate() {
            c.validate(self)
                .map_err(|x| format!("connection {i}: {x}"))?;
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
        let mut s = OrganismStructure::new();
        for e in &self.elements {
            s.add_unit(
                StructuralUnit::from_material(e.material.clone(), e.placement)
                    .ok_or_else(|| "invalid blueprint structural material".to_string())?,
            );
        }
        for c in &self.connections {
            let a = s
                .connection_site(
                    crate::structure::ConnectionSiteRef {
                        unit_index: c.element_a,
                        point_index: c.point_a,
                    },
                    catalog,
                )
                .ok_or_else(|| format!("connection {c:?} references an invalid first site"))?;
            let b = s
                .connection_site(
                    crate::structure::ConnectionSiteRef {
                        unit_index: c.element_b,
                        point_index: c.point_b,
                    },
                    catalog,
                )
                .ok_or_else(|| format!("connection {c:?} references an invalid second site"))?;
            if !crate::contact::connection_points_contact(
                a,
                &s.units[c.element_a],
                b,
                &s.units[c.element_b],
                1e-9,
                1.0 - 1e-9,
            ) {
                return Err(format!("connection {c:?} does not realize as physical contact"));
            }
            let pa = s.units[c.element_a]
                .properties(catalog)
                .ok_or_else(|| "missing catalog properties for first connection endpoint".to_string())?;
            let pb = s.units[c.element_b]
                .properties(catalog)
                .ok_or_else(|| "missing catalog properties for second connection endpoint".to_string())?;
            let strength = crate::combine::bond_strength(pa, pb);
            if !strength.is_finite() || !(0.0..=1.0).contains(&strength) {
                return Err("connection produced invalid intrinsic bond strength".into());
            }
            // Use the non-cached candidate enumeration here. Blueprint units may
            // be composite materials, which intentionally have no single
            // resource_name and therefore cannot use the runtime cache key.
            let mut cache = crate::contact::ConnectionCompatibilityCache::new();
            let candidate = crate::contact::connection_pair_candidates(
                &s,
                c.element_a,
                c.element_b,
                catalog,
            )
            .into_iter()
            .find(|candidate| {
                candidate.point_a == c.point_a && candidate.point_b == c.point_b
            })
            .ok_or_else(|| format!("connection {c:?} has no valid formation candidate"))?;
            let evaluation = crate::combine::evaluate_formation(candidate, pa.cohesion, pb.cohesion);
            if !evaluation.threshold.is_finite() || evaluation.threshold <= 0.0 {
                return Err("connection produced invalid COMBINE formation investment".into());
            }
            s.add_bond(Bond {
                unit_a: c.element_a,
                point_a: c.point_a,
                unit_b: c.element_b,
                point_b: c.point_b,
                strength,
                bond_energy: evaluation.threshold,
            });
        }
        Ok(s)
    }

    pub fn is_connected(&self) -> bool {
        if self.elements.is_empty() {
            return false;
        }
        let mut v = vec![false; self.elements.len()];
        let mut stack = vec![0usize];
        v[0] = true;
        while let Some(cur) = stack.pop() {
            for c in &self.connections {
                let next = if c.element_a == cur {
                    Some(c.element_b)
                } else if c.element_b == cur {
                    Some(c.element_a)
                } else {
                    None
                };
                if let Some(next) = next {
                    if !v[next] {
                        v[next] = true;
                        stack.push(next);
                    }
                }
            }
        }
        v.into_iter().all(|x| x)
    }

    pub fn core_is_connected(&self) -> bool {
        if self.core_elements.is_empty() {
            return true;
        }
        let set: std::collections::HashSet<usize> = self.core_elements.iter().copied().collect();
        let mut seen = std::collections::HashSet::new();
        let mut stack = vec![self.core_elements[0]];
        while let Some(cur) = stack.pop() {
            if !seen.insert(cur) {
                continue;
            }
            for c in &self.connections {
                let next = if c.element_a == cur {
                    Some(c.element_b)
                } else if c.element_b == cur {
                    Some(c.element_a)
                } else {
                    None
                };
                if let Some(next) = next {
                    if set.contains(&next) && !seen.contains(&next) {
                        stack.push(next);
                    }
                }
            }
        }
        seen.len() == set.len()
    }

    pub fn total_material_amount(&self) -> f64 {
        self.elements.iter().map(|e| e.material.total_amount()).sum()
    }
}
