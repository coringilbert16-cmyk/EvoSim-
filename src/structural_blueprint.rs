//! Inherited structural blueprint.
use crate::attachment::{AttachmentFeature, BlueprintAttachment, BlueprintAttachmentTarget, BlueprintElementId};
use crate::resources::{BaseResource, Material};
use crate::structure::{Bond, BondEndpoint, ConnectionEndpoint, OrganismStructure, Placement, StructuralUnit};
use serde::{Deserialize, Serialize};

fn default_core_elements() -> Vec<BlueprintElementId> { vec![BlueprintElementId(1)] }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct StructuralBlueprint {
    pub elements: Vec<BlueprintElement>,
    pub connections: Vec<BlueprintConnection>,
    #[serde(default = "default_core_elements")]
    pub core_elements: Vec<BlueprintElementId>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BlueprintElement {
    pub id: BlueprintElementId,
    pub material: Material,
    pub placement: Placement,
}

/// A structural relationship between two inherited physical attachment regions.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlueprintConnection {
    pub a: BlueprintAttachment,
    pub b: BlueprintAttachment,
}

impl StructuralBlueprint {
    pub fn new(elements: Vec<BlueprintElement>, connections: Vec<BlueprintConnection>) -> Self {
        Self { elements, connections, core_elements: default_core_elements() }
    }

    pub fn with_core_elements(elements: Vec<BlueprintElement>, connections: Vec<BlueprintConnection>, core_elements: Vec<BlueprintElementId>) -> Self {
        Self { elements, connections, core_elements }
    }

    pub fn is_valid(&self) -> bool { self.validate().is_ok() }

    fn element_index(&self, id: BlueprintElementId) -> Option<usize> { self.elements.iter().position(|e| e.id == id) }

    pub fn validate(&self) -> Result<(), String> {
        if self.elements.is_empty() { return Err("blueprint must contain at least one element".into()); }
        if self.core_elements.is_empty() { return Err("blueprint must define a genome core".into()); }
        let mut ids = std::collections::HashSet::new();
        for (i, e) in self.elements.iter().enumerate() {
            if !ids.insert(e.id) { return Err(format!("duplicate blueprint element id at element {i}")); }
            e.validate().map_err(|x| format!("element {i}: {x}"))?;
        }
        for &id in &self.core_elements { if self.element_index(id).is_none() { return Err("genome core references a missing element".into()); } }
        for (i, c) in self.connections.iter().enumerate() { c.validate(self).map_err(|x| format!("connection {i}: {x}"))?; }
        if self.elements.len() > 1 && !self.is_connected() { return Err("multi-element blueprint must be connected".into()); }
        if self.core_elements.len() > 1 && !self.core_is_connected() { return Err("genome core must be connected".into()); }
        Ok(())
    }

    pub fn realize(&self, catalog: &[BaseResource]) -> Result<OrganismStructure, String> {
        self.validate()?;
        let mut s = OrganismStructure::new();
        for e in &self.elements {
            s.add_unit(StructuralUnit::from_material(e.material.clone(), e.placement).ok_or_else(|| "invalid blueprint structural material".to_string())?);
        }
        for c in &self.connections { realize_connection(self, &mut s, *c, catalog).map_err(|e| format!("connection {c:?}: {e}"))?; }
        Ok(s)
    }

    pub fn is_connected(&self) -> bool {
        if self.elements.is_empty() { return false; }
        let mut seen = std::collections::HashSet::new();
        let mut stack = vec![self.elements[0].id];
        seen.insert(self.elements[0].id);
        while let Some(cur) = stack.pop() {
            for c in &self.connections {
                let next = if c.a.element == cur { c.b.element } else if c.b.element == cur { c.a.element } else { continue };
                if self.element_index(next).is_some() && seen.insert(next) { stack.push(next); }
            }
        }
        seen.len() == self.elements.len()
    }

    fn core_is_connected(&self) -> bool {
        let core = self.core_elements.iter().copied().collect::<std::collections::HashSet<_>>();
        let mut seen = std::collections::HashSet::new();
        let mut stack = vec![self.core_elements[0]];
        seen.insert(self.core_elements[0]);
        while let Some(cur) = stack.pop() {
            for c in &self.connections {
                let next = if c.a.element == cur { c.b.element } else if c.b.element == cur { c.a.element } else { continue };
                if core.contains(&next) && seen.insert(next) { stack.push(next); }
            }
        }
        seen.len() == core.len()
    }

    pub fn total_material_amount(&self) -> f64 { self.elements.iter().map(|e| e.material.total_amount()).sum() }
    pub fn structural_mass(&self, catalog: &[BaseResource]) -> f64 { self.elements.iter().map(|e| e.material.mass(catalog)).sum() }
}

fn validate_attachment_target(element: &BlueprintElement, attachment: BlueprintAttachment) -> Result<(), String> {
    match attachment.target {
        BlueprintAttachmentTarget::Assembly => Ok(()),
        BlueprintAttachmentTarget::Constituent(id) => {
            let Some(structure) = element.material.structure() else { return Err("constituent attachment requires structured material".into()); };
            if structure.constituents.iter().any(|c| c.id == id) { Ok(()) } else { Err("attachment references a missing material constituent".into()) }
        }
    }
}

fn runtime_endpoint(blueprint: &StructuralBlueprint, attachment: BlueprintAttachment) -> Result<ConnectionEndpoint, String> {
    let element = blueprint.element_index(attachment.element).ok_or_else(|| "invalid blueprint element".to_string()).and_then(|i| blueprint.elements.get(i).ok_or_else(|| "invalid blueprint element index".to_string()))?;
    validate_attachment_target(element, attachment)?;
    match (attachment.target, attachment.feature) {
        (BlueprintAttachmentTarget::Assembly, AttachmentFeature::Discrete(point_index)) => Ok(ConnectionEndpoint::Corner { point_index: point_index as usize }),
        (BlueprintAttachmentTarget::Assembly, AttachmentFeature::Boundary) => Err("continuous Boundary assembly attachment requires physical contact resolution".into()),
        (BlueprintAttachmentTarget::Assembly, AttachmentFeature::Fluid) => Err("continuous Fluid assembly attachment requires physical contact resolution".into()),
        (BlueprintAttachmentTarget::Constituent(_), AttachmentFeature::Discrete(_)) => Err("constituent discrete blueprint attachment requires constituent-aware physical resolution".into()),
        (BlueprintAttachmentTarget::Constituent(_), AttachmentFeature::Boundary) => Err("constituent Boundary blueprint attachment requires physical contact resolution".into()),
        (BlueprintAttachmentTarget::Constituent(_), AttachmentFeature::Fluid) => Err("constituent Fluid blueprint attachment requires physical contact resolution".into()),
    }
}

pub(crate) fn realize_connection(blueprint: &StructuralBlueprint, structure: &mut OrganismStructure, connection: BlueprintConnection, catalog: &[BaseResource]) -> Result<f64, String> {
    let a = blueprint.element_index(connection.a.element).ok_or_else(|| "invalid first blueprint element".to_string())?;
    let b = blueprint.element_index(connection.b.element).ok_or_else(|| "invalid second blueprint element".to_string())?;
    if a == b { return Err("self-connections are not permitted".into()); }
    let endpoint_a = runtime_endpoint(blueprint, connection.a)?;
    let endpoint_b = runtime_endpoint(blueprint, connection.b)?;
    let ConnectionEndpoint::Corner { point_index: point_a } = endpoint_a else { unreachable!() };
    let ConnectionEndpoint::Corner { point_index: point_b } = endpoint_b else { unreachable!() };
    let _ = structure.connection_site(crate::structure::ConnectionSiteRef { unit_index: a, point_index: point_a }, catalog).ok_or_else(|| "invalid first connection site".to_string())?;
    let _ = structure.connection_site(crate::structure::ConnectionSiteRef { unit_index: b, point_index: point_b }, catalog).ok_or_else(|| "invalid second connection site".to_string())?;
    let pa = structure.units[a].properties(catalog).ok_or_else(|| "missing first endpoint properties".to_string())?;
    let pb = structure.units[b].properties(catalog).ok_or_else(|| "missing second endpoint properties".to_string())?;
    let mut cache = crate::contact::ConnectionCompatibilityCache::new();
    let candidate = crate::contact::connection_pair_candidates_cached(structure, a, b, catalog, &mut cache).into_iter().find(|c| c.endpoint_a == endpoint_a && c.endpoint_b == endpoint_b).ok_or_else(|| "connection endpoints are not physically resolvable".to_string())?;
    let evaluation = crate::combine::evaluate_formation(candidate, pa.cohesion, pb.cohesion);
    let (_, work, _) = crate::combine::required_investment(pa, pb, evaluation, 0.0).map_err(|e| format!("formation investment failed: {e:?}"))?;
    let strength = crate::combine::bond_strength(pa, pb);
    if !strength.is_finite() || !(0.0..=1.0).contains(&strength) { return Err("invalid intrinsic bond strength".into()); }
    crate::contact::try_add_bond(structure, Bond { endpoint_a: BondEndpoint { unit_index: a, location: endpoint_a }, endpoint_b: BondEndpoint { unit_index: b, location: endpoint_b }, strength, bond_energy: 0.0 }, catalog).map_err(|e| e.to_string())?;
    Ok(work)
}

impl BlueprintElement {
    pub fn validate(&self) -> Result<(), String> {
        if !self.material.is_valid() { return Err("material is invalid".into()); }
        if !self.placement.x.is_finite() || !self.placement.y.is_finite() || !self.placement.rotation_radians.is_finite() { return Err("placement must be finite".into()); }
        if !self.material.is_structured() {
            let c = self.material.composition();
            if c.len() != 1 || (c[0].amount - 1.0).abs() > f64::EPSILON { return Err("unstructured blueprint element must represent exactly one material unit".into()); }
        } else if !self.material.is_connected() { return Err("internal structural material must be connected".into()); }
        Ok(())
    }
}

impl BlueprintConnection {
    fn validate(&self, blueprint: &StructuralBlueprint) -> Result<(), String> {
        for attachment in [self.a, self.b] {
            let element = blueprint.element_index(attachment.element).and_then(|i| blueprint.elements.get(i)).ok_or_else(|| "references a missing element".to_string())?;
            if !attachment.feature.is_valid() { return Err("invalid attachment feature".into()); }
            validate_attachment_target(element, attachment)?;
            if let (BlueprintAttachmentTarget::Constituent(id), AttachmentFeature::Discrete(feature)) = (attachment.target, attachment.feature) {
                let structure = element.material.structure().ok_or_else(|| "constituent attachment requires structured material".to_string())?;
                let constituent = structure.constituents.iter().find(|c| c.id == id).ok_or_else(|| "attachment references a missing material constituent".to_string())?;
                let _ = (constituent, feature);
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
    fn blueprint_connection_distinguishes_constituent_and_assembly_targets() {
        let connection = BlueprintConnection {
            a: BlueprintAttachment { element: BlueprintElementId(4), target: BlueprintAttachmentTarget::Constituent(crate::attachment::ConstituentId(7)), feature: AttachmentFeature::Discrete(2) },
            b: BlueprintAttachment { element: BlueprintElementId(9), target: BlueprintAttachmentTarget::Assembly, feature: AttachmentFeature::Boundary },
        };
        assert_ne!(connection.a.target, connection.b.target);
    }
}
