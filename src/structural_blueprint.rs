//! Inherited structural blueprint.
use crate::attachment::{AttachmentFeature, BlueprintAttachment, BlueprintElementId};
use crate::resources::{BaseResource, Material};
use crate::structure::{Bond, BondEndpoint, ConnectionEndpoint, OrganismStructure, Placement, StructuralUnit};
use serde::{Deserialize, Serialize};

fn default_core_elements() -> Vec<usize> { vec![0] }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct StructuralBlueprint {
    pub elements: Vec<BlueprintElement>,
    pub connections: Vec<BlueprintConnection>,
    #[serde(default = "default_core_elements")]
    pub core_elements: Vec<usize>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BlueprintElement {
    pub material: Material,
    pub placement: Placement,
}

/// A structural relationship between two inherited physical attachment regions.
///
/// The blueprint stores stable element identity plus attachment feature identity;
/// it does not store a socket number or a bond-capacity count. Discrete features
/// refer to immutable geometry of the referenced material. Boundary and Fluid
/// features remain continuous and are resolved by the physical contact solver.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlueprintConnection {
    pub a: BlueprintAttachment,
    pub b: BlueprintAttachment,
}

impl StructuralBlueprint {
    pub fn new(elements: Vec<BlueprintElement>, connections: Vec<BlueprintConnection>) -> Self {
        Self { elements, connections, core_elements: default_core_elements() }
    }

    pub fn with_core_elements(
        elements: Vec<BlueprintElement>,
        connections: Vec<BlueprintConnection>,
        core_elements: Vec<usize>,
    ) -> Self {
        Self { elements, connections, core_elements }
    }

    pub fn is_valid(&self) -> bool { self.validate().is_ok() }

    pub fn validate(&self) -> Result<(), String> {
        if self.elements.is_empty() { return Err("blueprint must contain at least one element".into()); }
        if self.core_elements.is_empty() { return Err("blueprint must define a genome core".into()); }
        let mut seen = vec![false; self.elements.len()];
        for &i in &self.core_elements {
            if i >= self.elements.len() || seen[i] { return Err("invalid genome core".into()); }
            seen[i] = true;
        }
        for (i, e) in self.elements.iter().enumerate() {
            e.validate().map_err(|x| format!("element {i}: {x}"))?;
        }
        for (i, c) in self.connections.iter().enumerate() {
            c.validate(self).map_err(|x| format!("connection {i}: {x}"))?;
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
            s.add_unit(StructuralUnit::from_material(e.material.clone(), e.placement)
                .ok_or_else(|| "invalid blueprint structural material".to_string())?);
        }
        for c in &self.connections {
            realize_connection(&mut s, *c, catalog)
                .map_err(|e| format!("connection {c:?}: {e}"))?;
        }
        Ok(s)
    }

    pub fn is_connected(&self) -> bool {
        if self.elements.is_empty() { return false; }
        let mut v = vec![false; self.elements.len()];
        let mut stack = vec![0];
        v[0] = true;
        while let Some(cur) = stack.pop() {
            for c in &self.connections {
                let n = if c.a.element.0 as usize == cur {
                    c.b.element.0 as usize
                } else if c.b.element.0 as usize == cur {
                    c.a.element.0 as usize
                } else {
                    continue;
                };
                if n < v.len() && !v[n] { v[n] = true; stack.push(n); }
            }
        }
        v.into_iter().all(|x| x)
    }

    fn core_is_connected(&self) -> bool {
        let core = self.core_elements.iter().copied().collect::<std::collections::HashSet<_>>();
        let mut seen = std::collections::HashSet::new();
        let mut stack = vec![self.core_elements[0]];
        seen.insert(self.core_elements[0]);
        while let Some(cur) = stack.pop() {
            for c in &self.connections {
                let n = if c.a.element.0 as usize == cur {
                    c.b.element.0 as usize
                } else if c.b.element.0 as usize == cur {
                    c.a.element.0 as usize
                } else {
                    continue;
                };
                if core.contains(&n) && seen.insert(n) { stack.push(n); }
            }
        }
        seen.len() == core.len()
    }

    pub fn total_material_amount(&self) -> f64 {
        self.elements.iter().map(|e| e.material.total_amount()).sum()
    }

    pub fn structural_mass(&self, catalog: &[BaseResource]) -> f64 {
        self.elements.iter().map(|e| e.material.mass(catalog)).sum()
    }
}

fn blueprint_element_index(attachment: BlueprintAttachment) -> usize {
    attachment.element.0 as usize
}

fn runtime_endpoint(attachment: BlueprintAttachment) -> Result<ConnectionEndpoint, String> {
    match attachment.feature {
        AttachmentFeature::Discrete(point_index) => Ok(ConnectionEndpoint::Corner { point_index: point_index as usize }),
        AttachmentFeature::Boundary => Err("continuous Boundary blueprint attachment requires physical contact resolution".into()),
        AttachmentFeature::Fluid => Err("continuous Fluid blueprint attachment requires physical contact resolution".into()),
    }
}

pub(crate) fn realize_connection(
    structure: &mut OrganismStructure,
    connection: BlueprintConnection,
    catalog: &[BaseResource],
) -> Result<f64, String> {
    let a = blueprint_element_index(connection.a);
    let b = blueprint_element_index(connection.b);
    if a >= structure.units.len() || b >= structure.units.len() || a == b {
        return Err("invalid blueprint connection elements".into());
    }
    let endpoint_a = runtime_endpoint(connection.a)?;
    let endpoint_b = runtime_endpoint(connection.b)?;
    let ConnectionEndpoint::Corner { point_index: point_a } = endpoint_a else {
        unreachable!();
    };
    let ConnectionEndpoint::Corner { point_index: point_b } = endpoint_b else {
        unreachable!();
    };
    let _ = structure.connection_site(crate::structure::ConnectionSiteRef { unit_index: a, point_index: point_a }, catalog)
        .ok_or_else(|| "invalid first connection site".to_string())?;
    let _ = structure.connection_site(crate::structure::ConnectionSiteRef { unit_index: b, point_index: point_b }, catalog)
        .ok_or_else(|| "invalid second connection site".to_string())?;
    let pa = structure.units[a].properties(catalog).ok_or_else(|| "missing first endpoint properties".to_string())?;
    let pb = structure.units[b].properties(catalog).ok_or_else(|| "missing second endpoint properties".to_string())?;
    let mut cache = crate::contact::ConnectionCompatibilityCache::new();
    let candidate = crate::contact::connection_pair_candidates_cached(structure, a, b, catalog, &mut cache)
        .into_iter()
        .find(|c| c.endpoint_a == endpoint_a && c.endpoint_b == endpoint_b)
        .ok_or_else(|| "connection endpoints are not physically resolvable".to_string())?;
    let evaluation = crate::combine::evaluate_formation(candidate, pa.cohesion, pb.cohesion);
    let (_, work, _) = crate::combine::required_investment(pa, pb, evaluation, 0.0)
        .map_err(|e| format!("formation investment failed: {e:?}"))?;
    let strength = crate::combine::bond_strength(pa, pb);
    if !strength.is_finite() || !(0.0..=1.0).contains(&strength) {
        return Err("invalid intrinsic bond strength".into());
    }
    crate::contact::try_add_bond(structure, Bond {
        endpoint_a: BondEndpoint { unit_index: a, location: endpoint_a },
        endpoint_b: BondEndpoint { unit_index: b, location: endpoint_b },
        strength,
        bond_energy: 0.0,
    }, catalog).map_err(|e| e.to_string())?;
    Ok(work)
}

impl BlueprintElement {
    pub fn validate(&self) -> Result<(), String> {
        if !self.material.is_valid() { return Err("material is invalid".into()); }
        if !self.placement.x.is_finite() || !self.placement.y.is_finite() || !self.placement.rotation_radians.is_finite() {
            return Err("placement must be finite".into());
        }
        if !self.material.is_structured() {
            let c = self.material.composition();
            if c.len() != 1 || (c[0].amount - 1.0).abs() > f64::EPSILON {
                return Err("unstructured blueprint element must represent exactly one material unit".into());
            }
        } else if !self.material.is_connected() {
            return Err("internal structural material must be connected".into());
        }
        Ok(())
    }
}

impl BlueprintConnection {
    fn validate(&self, blueprint: &StructuralBlueprint) -> Result<(), String> {
        for attachment in [self.a, self.b] {
            let index = blueprint_element_index(attachment);
            if index >= blueprint.elements.len() {
                return Err("references a missing element".into());
            }
            if !attachment.feature.is_valid() {
                return Err("invalid attachment feature".into());
            }
            if let AttachmentFeature::Discrete(point_index) = attachment.feature {
                let material = &blueprint.elements[index].material;
                if !material.is_structured() {
                    let component = material.composition().first().ok_or_else(|| "missing blueprint material".to_string())?;
                    if component.amount != 1.0 {
                        return Err("discrete attachment requires one physical material unit".into());
                    }
                }
                // Actual discrete-feature existence is checked against immutable resource geometry during realization.
                let _ = point_index;
            }
        }
        if self.a.element == self.b.element { return Err("self-connections are not permitted".into()); }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attachment::AttachmentFeature;

    #[test]
    fn blueprint_connection_uses_physical_attachment_identity() {
        let connection = BlueprintConnection {
            a: BlueprintAttachment { element: BlueprintElementId(4), feature: AttachmentFeature::Discrete(2) },
            b: BlueprintAttachment { element: BlueprintElementId(9), feature: AttachmentFeature::Boundary },
        };
        assert_eq!(connection.a.element, BlueprintElementId(4));
        assert_eq!(connection.a.feature, AttachmentFeature::Discrete(2));
        assert_eq!(connection.b.feature, AttachmentFeature::Boundary);
    }
}
