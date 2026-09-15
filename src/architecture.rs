//! Inherited organism architecture.
//!
//! The genome stores region-level developmental intent, not a constituent list
//! or a juvenile body plan. Discrete construction targets are generated at the
//! moment they are needed and are validated against the real physical runtime.
use crate::resources::Material;
use crate::structural_blueprint::{
    BlueprintConnection, BlueprintElement, BlueprintPlacement, StructuralBlueprint,
};
use serde::{Deserialize, Serialize};
pub const JUVENILE_LINEAR_SCALE: f64 = 0.40;
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ArchitectureRole {
    GenomeCore,
    StructuralBoundary,
    Interface,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ArchitectureRegion {
    pub role: ArchitectureRole,
    pub material: Material,
    pub center_x: f64,
    pub center_y: f64,
    pub extent_x: f64,
    pub extent_y: f64,
    pub density: f64,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum ArchitectureRelationKind {
    Encloses,
    Interfaces,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ArchitectureRelation {
    pub region_a: usize,
    pub region_b: usize,
    pub kind: ArchitectureRelationKind,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct OrganismArchitecture {
    pub regions: Vec<ArchitectureRegion>,
    pub relations: Vec<ArchitectureRelation>,
    pub genome_region: usize,
    pub target_scale: f64,
}
impl OrganismArchitecture {
    pub fn validate(&self) -> Result<(), String> {
        if self.regions.is_empty() || self.genome_region >= self.regions.len() {
            return Err("architecture has no valid genome region".into());
        }
        if !self.target_scale.is_finite() || self.target_scale <= 0.0 {
            return Err("architecture target scale must be positive and finite".into());
        }
        for region in &self.regions {
            if !region.center_x.is_finite()
                || !region.center_y.is_finite()
                || !region.extent_x.is_finite()
                || region.extent_x <= 0.0
                || !region.extent_y.is_finite()
                || region.extent_y <= 0.0
                || !region.density.is_finite()
                || region.density <= 0.0
                || !region.material.is_valid()
            {
                return Err("architecture region is invalid".into());
            }
        }
        for relation in &self.relations {
            if relation.region_a >= self.regions.len()
                || relation.region_b >= self.regions.len()
                || relation.region_a == relation.region_b
            {
                return Err("architecture relation is invalid".into());
            }
        }
        Ok(())
    }
    pub fn adult_construction_target(&self) -> Result<StructuralBlueprint, String> {
        self.construction_target(1.0)
    }
    pub fn developmental_target(
        &self,
        requested_scale: f64,
        catalog: &[crate::resources::BaseResource],
    ) -> Result<StructuralBlueprint, String> {
        self.validate()?;
        let requested = requested_scale.clamp(JUVENILE_LINEAR_SCALE, 1.0);
        let mut candidates = Vec::new();
        for step in 0..=12 {
            candidates.push((requested + step as f64 * 0.05).min(1.0));
        }
        for step in 1..=12 {
            candidates.push((requested - step as f64 * 0.05).max(JUVENILE_LINEAR_SCALE));
        }
        candidates.sort_by(|a, b| {
            (a - requested)
                .abs()
                .partial_cmp(&(b - requested).abs())
                .unwrap()
        });
        candidates.dedup_by(|a, b| (*a - *b).abs() < 1e-12);
        let mut last_error = "no viable developmental target".to_string();
        for scale in candidates {
            let target = self.construction_target(scale)?;
            match target.realize(catalog) {
                Ok(structure)
                    if crate::juvenile_requirements::validate_realized_juvenile(
                        &structure,
                        catalog,
                        &target.core_elements,
                        crate::juvenile_requirements::JuvenileViabilityRequirements::default(),
                    )
                    .is_ok() =>
                {
                    return Ok(target)
                }
                Ok(_) => last_error = format!("scale {scale:.2} failed juvenile viability"),
                Err(error) => last_error = format!("scale {scale:.2} failed realization: {error}"),
            }
        }
        Err(last_error)
    }
    fn construction_target(&self, scale: f64) -> Result<StructuralBlueprint, String> {
        self.validate()?;
        let core = self
            .regions
            .get(self.genome_region)
            .ok_or("missing genome region")?;
        if !matches!(core.role, ArchitectureRole::GenomeCore) {
            return Err("genome_region must identify GenomeCore".into());
        }
        let boundary = self
            .regions
            .iter()
            .position(|r| matches!(r.role, ArchitectureRole::StructuralBoundary))
            .ok_or("architecture requires a structural boundary")?;
        let interface = self
            .regions
            .iter()
            .position(|r| matches!(r.role, ArchitectureRole::Interface))
            .ok_or("architecture requires an interface")?;
        let mut elements = Vec::new();
        let mut connections = Vec::new();
        add_core_region(&mut elements, &mut connections, core, scale);
        add_boundary_region(
            &mut elements,
            &mut connections,
            &self.regions[boundary],
            scale,
        );
        add_interface_region(
            &mut elements,
            &mut connections,
            &self.regions[interface],
            scale,
        );
        let target =
            StructuralBlueprint::with_core_elements(elements, connections, vec![0, 1, 2, 3]);
        target.validate()?;
        Ok(target)
    }
}
fn add_core_region(
    elements: &mut Vec<BlueprintElement>,
    connections: &mut Vec<BlueprintConnection>,
    region: &ArchitectureRegion,
    scale: f64,
) {
    let offset = (1.511_858 + 0.330_719) / 2.0;
    let sx = region.center_x * scale;
    let sy = region.center_y * scale;
    elements.extend([
        BlueprintElement {
            material: region.material.clone(),
            placement: BlueprintPlacement {
                x: sx,
                y: sy + offset,
                rotation_radians: 0.0,
            },
        },
        BlueprintElement {
            material: region.material.clone(),
            placement: BlueprintPlacement {
                x: sx - offset,
                y: sy,
                rotation_radians: std::f64::consts::FRAC_PI_2,
            },
        },
        BlueprintElement {
            material: region.material.clone(),
            placement: BlueprintPlacement {
                x: sx + offset,
                y: sy,
                rotation_radians: std::f64::consts::FRAC_PI_2,
            },
        },
        BlueprintElement {
            material: region.material.clone(),
            placement: BlueprintPlacement {
                x: sx,
                y: sy - offset,
                rotation_radians: 0.0,
            },
        },
    ]);
    connections.extend([
        BlueprintConnection {
            element_a: 0,
            element_b: 1,
        },
        BlueprintConnection {
            element_a: 0,
            element_b: 2,
        },
        BlueprintConnection {
            element_a: 1,
            element_b: 3,
        },
        BlueprintConnection {
            element_a: 2,
            element_b: 3,
        },
    ]);
}
fn add_boundary_region(
    elements: &mut Vec<BlueprintElement>,
    connections: &mut Vec<BlueprintConnection>,
    region: &ArchitectureRegion,
    scale: f64,
) {
    let half = 1.511_858 / 2.0;
    let offset = 1.677_2175;
    let sx = region.center_x * scale;
    let sy = region.center_y * scale;
    let start = elements.len();
    let count = ((8.0 * region.density * scale.sqrt()).round() as usize).clamp(4, 8);
    if count <= 4 {
        elements.extend([
            BlueprintElement {
                material: region.material.clone(),
                placement: BlueprintPlacement {
                    x: sx,
                    y: sy + offset,
                    rotation_radians: 0.0,
                },
            },
            BlueprintElement {
                material: region.material.clone(),
                placement: BlueprintPlacement {
                    x: sx + offset,
                    y: sy,
                    rotation_radians: std::f64::consts::FRAC_PI_2,
                },
            },
            BlueprintElement {
                material: region.material.clone(),
                placement: BlueprintPlacement {
                    x: sx,
                    y: sy - offset,
                    rotation_radians: 0.0,
                },
            },
            BlueprintElement {
                material: region.material.clone(),
                placement: BlueprintPlacement {
                    x: sx - offset,
                    y: sy,
                    rotation_radians: std::f64::consts::FRAC_PI_2,
                },
            },
        ]);
        connections.extend([
            BlueprintConnection {
                element_a: start,
                element_b: start + 1,
            },
            BlueprintConnection {
                element_a: start + 1,
                element_b: start + 2,
            },
            BlueprintConnection {
                element_a: start + 2,
                element_b: start + 3,
            },
            BlueprintConnection {
                element_a: start + 3,
                element_b: start,
            },
        ]);
    } else {
        elements.extend([
            BlueprintElement {
                material: region.material.clone(),
                placement: BlueprintPlacement {
                    x: sx - half,
                    y: sy + offset,
                    rotation_radians: 0.0,
                },
            },
            BlueprintElement {
                material: region.material.clone(),
                placement: BlueprintPlacement {
                    x: sx + half,
                    y: sy + offset,
                    rotation_radians: 0.0,
                },
            },
            BlueprintElement {
                material: region.material.clone(),
                placement: BlueprintPlacement {
                    x: sx - offset,
                    y: sy - half,
                    rotation_radians: std::f64::consts::FRAC_PI_2,
                },
            },
            BlueprintElement {
                material: region.material.clone(),
                placement: BlueprintPlacement {
                    x: sx - offset,
                    y: sy + half,
                    rotation_radians: std::f64::consts::FRAC_PI_2,
                },
            },
            BlueprintElement {
                material: region.material.clone(),
                placement: BlueprintPlacement {
                    x: sx + offset,
                    y: sy - half,
                    rotation_radians: std::f64::consts::FRAC_PI_2,
                },
            },
            BlueprintElement {
                material: region.material.clone(),
                placement: BlueprintPlacement {
                    x: sx + offset,
                    y: sy + half,
                    rotation_radians: std::f64::consts::FRAC_PI_2,
                },
            },
            BlueprintElement {
                material: region.material.clone(),
                placement: BlueprintPlacement {
                    x: sx - half,
                    y: sy - offset,
                    rotation_radians: 0.0,
                },
            },
            BlueprintElement {
                material: region.material.clone(),
                placement: BlueprintPlacement {
                    x: sx + half,
                    y: sy - offset,
                    rotation_radians: 0.0,
                },
            },
        ]);
        connections.extend([
            BlueprintConnection {
                element_a: start,
                element_b: start + 1,
            },
            BlueprintConnection {
                element_a: start + 1,
                element_b: start + 5,
            },
            BlueprintConnection {
                element_a: start + 5,
                element_b: start + 4,
            },
            BlueprintConnection {
                element_a: start + 4,
                element_b: start + 7,
            },
            BlueprintConnection {
                element_a: start + 7,
                element_b: start + 6,
            },
            BlueprintConnection {
                element_a: start + 6,
                element_b: start + 2,
            },
            BlueprintConnection {
                element_a: start + 2,
                element_b: start + 3,
            },
            BlueprintConnection {
                element_a: start + 3,
                element_b: start,
            },
        ]);
    }
}
fn add_interface_region(
    elements: &mut Vec<BlueprintElement>,
    connections: &mut Vec<BlueprintConnection>,
    region: &ArchitectureRegion,
    scale: f64,
) {
    let inner = 1.086_648;
    let outer = 1.511_858;
    let length: f64 = 0.797_884;
    let gap = outer - inner;
    let tangent = (length * length - gap * gap).sqrt();
    let center = (inner + outer) / 2.0;
    let sx = region.center_x * scale;
    let sy = region.center_y * scale;
    let start = elements.len();
    let count = ((4.0 * region.density * scale.sqrt()).round() as usize).clamp(2, 4);
    let cardinal = [
        BlueprintElement {
            material: region.material.clone(),
            placement: BlueprintPlacement {
                x: sx,
                y: sy + center,
                rotation_radians: gap.atan2(-tangent),
            },
        },
        BlueprintElement {
            material: region.material.clone(),
            placement: BlueprintPlacement {
                x: sx + center,
                y: sy,
                rotation_radians: tangent.atan2(gap),
            },
        },
        BlueprintElement {
            material: region.material.clone(),
            placement: BlueprintPlacement {
                x: sx,
                y: sy - center,
                rotation_radians: (-gap).atan2(tangent),
            },
        },
        BlueprintElement {
            material: region.material.clone(),
            placement: BlueprintPlacement {
                x: sx - center,
                y: sy,
                rotation_radians: (-tangent).atan2(-gap),
            },
        },
    ];
    elements.extend(cardinal.into_iter().take(count));
    let boundary_count = if count == 2 { 4 } else { 8 };
    let (core_map, boundary_map): (&[usize], &[usize]) = if count == 2 {
        (&[0, 3], &[0, 2])
    } else {
        (&[0, 1, 2, 3], &[0, 2, 4, 6])
    };
    for i in 0..count {
        connections.push(BlueprintConnection {
            element_a: core_map[i],
            element_b: start + i,
        });
        connections.push(BlueprintConnection {
            element_a: start - boundary_count + boundary_map[i],
            element_b: start + i,
        });
    }
}
pub fn default_architecture() -> OrganismArchitecture {
    OrganismArchitecture {
        regions: vec![
            ArchitectureRegion {
                role: ArchitectureRole::GenomeCore,
                material: Material::free_base("Nitrogen", 1.0),
                center_x: 0.0,
                center_y: 0.0,
                extent_x: 1.84,
                extent_y: 1.84,
                density: 1.0,
            },
            ArchitectureRegion {
                role: ArchitectureRole::StructuralBoundary,
                material: Material::free_base("Nitrogen", 1.0),
                center_x: 0.0,
                center_y: 0.0,
                extent_x: 3.35,
                extent_y: 3.35,
                density: 1.0,
            },
            ArchitectureRegion {
                role: ArchitectureRole::Interface,
                material: Material::free_base("Hydrogen", 1.0),
                center_x: 0.0,
                center_y: 0.0,
                extent_x: 2.17,
                extent_y: 2.17,
                density: 1.0,
            },
        ],
        relations: vec![
            ArchitectureRelation {
                region_a: 1,
                region_b: 0,
                kind: ArchitectureRelationKind::Encloses,
            },
            ArchitectureRelation {
                region_a: 2,
                region_b: 0,
                kind: ArchitectureRelationKind::Interfaces,
            },
            ArchitectureRelation {
                region_a: 2,
                region_b: 1,
                kind: ArchitectureRelationKind::Interfaces,
            },
        ],
        genome_region: 0,
        target_scale: 1.0,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::default_catalog;
    #[test]
    fn architecture_is_region_level() {
        let a = default_architecture();
        assert_eq!(a.regions.len(), 3);
        assert!(a.validate().is_ok());
    }
    #[test]
    fn juvenile_target_is_derived_at_runtime() {
        let a = default_architecture();
        let target = a
            .developmental_target(JUVENILE_LINEAR_SCALE, &default_catalog())
            .unwrap();
        assert!(target.is_valid());
        assert_eq!(target.core_elements.len(), 4);
    }
}
