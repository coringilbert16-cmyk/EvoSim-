//! Inherited organism architecture.
//!
//! The genome stores region-level developmental intent. It does not store a
//! constituent list, exact bond graph, or exact juvenile body plan. A discrete
//! construction target is generated only when realization is requested.
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
        let has_boundary = self
            .regions
            .iter()
            .any(|r| matches!(r.role, ArchitectureRole::StructuralBoundary));
        let has_interface = self
            .regions
            .iter()
            .any(|r| matches!(r.role, ArchitectureRole::Interface));
        if !has_boundary || !has_interface {
            return Err("architecture requires boundary and interface regions".into());
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
                .unwrap_or(std::cmp::Ordering::Equal)
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
        add_core_region(&mut elements, &mut connections, core);
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
) {
    let offset = (1.511_858 + 0.330_719) / 2.0;
    let sx = region.center_x;
    let sy = region.center_y;
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
    let half_segment = 1.511_858 / 2.0;
    let offset = 1.677_2175;
    let sx = region.center_x;
    let sy = region.center_y;
    let start = elements.len();
    let count = if scale < 0.75 {
        1
    } else {
        ((8.0 * region.density).round() as usize).clamp(4, 8)
    };

    if count == 1 {
        elements.push(BlueprintElement {
            material: region.material.clone(),
            placement: BlueprintPlacement {
                x: sx,
                y: sy + offset,
                rotation_radians: 0.0,
            },
        });
        connections.push(BlueprintConnection {
            element_a: 0,
            element_b: start,
        });
        return;
    }

    if count <= 4 {
        let positions = [
            (sx, sy + offset, 0.0),
            (sx + offset, sy, std::f64::consts::FRAC_PI_2),
            (sx, sy - offset, 0.0),
            (sx - offset, sy, std::f64::consts::FRAC_PI_2),
        ];
        for (x, y, rotation_radians) in positions {
            elements.push(BlueprintElement {
                material: region.material.clone(),
                placement: BlueprintPlacement {
                    x,
                    y,
                    rotation_radians,
                },
            });
        }
        for i in 0..4 {
            connections.push(BlueprintConnection {
                element_a: start + i,
                element_b: start + (i + 1) % 4,
            });
        }
        return;
    }

    let positions = [
        (sx - half_segment, sy + offset, 0.0),
        (sx + half_segment, sy + offset, 0.0),
        (sx - offset, sy - half_segment, std::f64::consts::FRAC_PI_2),
        (sx - offset, sy + half_segment, std::f64::consts::FRAC_PI_2),
        (sx + offset, sy - half_segment, std::f64::consts::FRAC_PI_2),
        (sx + offset, sy + half_segment, std::f64::consts::FRAC_PI_2),
        (sx - half_segment, sy - offset, 0.0),
        (sx + half_segment, sy - offset, 0.0),
    ];
    for (x, y, rotation_radians) in positions {
        elements.push(BlueprintElement {
            material: region.material.clone(),
            placement: BlueprintPlacement {
                x,
                y,
                rotation_radians,
            },
        });
    }
    let ring = [0, 1, 5, 4, 7, 6, 2, 3];
    for i in 0..8 {
        connections.push(BlueprintConnection {
            element_a: start + ring[i],
            element_b: start + ring[(i + 1) % 8],
        });
    }
}

fn add_interface_region(
    elements: &mut Vec<BlueprintElement>,
    connections: &mut Vec<BlueprintConnection>,
    region: &ArchitectureRegion,
    scale: f64,
) {
    let inner: f64 = 1.086_648;
    let outer: f64 = 1.511_858;
    let length: f64 = 0.797_884;
    let gap = outer - inner;
    let tangent = (length * length - gap * gap).sqrt();
    let center = (inner + outer) / 2.0;
    let sx = region.center_x;
    let sy = region.center_y;
    let start = elements.len();
    let count = if scale < 0.75 {
        1
    } else {
        ((4.0 * region.density).round() as usize).clamp(2, 4)
    };
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
                x: sx - center,
                y: sy,
                rotation_radians: (-tangent).atan2(-gap),
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
    ];
    let selected = match count {
        1 => vec![cardinal[0].clone()],
        2 => vec![cardinal[0].clone(), cardinal[3].clone()],
        3 => cardinal[..3].to_vec(),
        _ => cardinal.to_vec(),
    };
    elements.extend(selected);

    let boundary_count = if scale < 0.75 {
        1
    } else {
        ((8.0 * region.density).round() as usize).clamp(4, 8)
    };
    let core_map = match count {
        1 => vec![0],
        2 => vec![0, 3],
        3 => vec![0, 1, 2],
        _ => vec![0, 1, 2, 3],
    };
    let boundary_map = match (count, boundary_count) {
        (1, 1) => vec![0],
        (2, 4) => vec![0, 2],
        (3, 4) => vec![0, 1, 2],
        (4, 4) => vec![0, 1, 2, 3],
        (4, 8) => vec![0, 2, 4, 6],
        _ => vec![0],
    };
    let boundary_start = start - boundary_count;
    for i in 0..count {
        connections.push(BlueprintConnection {
            element_a: start + i,
            element_b: boundary_start + boundary_map[i],
        });
        connections.push(BlueprintConnection {
            element_a: start + i,
            element_b: core_map[i],
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
                extent_x: 1.0,
                extent_y: 1.0,
                density: 1.0,
            },
            ArchitectureRegion {
                role: ArchitectureRole::StructuralBoundary,
                material: Material::free_base("Nitrogen", 1.0),
                center_x: 0.0,
                center_y: 0.0,
                extent_x: 1.0,
                extent_y: 1.0,
                density: 0.5,
            },
            ArchitectureRegion {
                role: ArchitectureRole::Interface,
                material: Material::free_base("Hydrogen", 1.0),
                center_x: 0.0,
                center_y: 0.0,
                extent_x: 1.0,
                extent_y: 1.0,
                density: 0.5,
            },
        ],
        relations: vec![
            ArchitectureRelation {
                region_a: 0,
                region_b: 1,
                kind: ArchitectureRelationKind::Encloses,
            },
            ArchitectureRelation {
                region_a: 1,
                region_b: 2,
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

    #[test]
    fn architecture_is_region_level() {
        let architecture = default_architecture();
        assert_eq!(architecture.regions.len(), 3);
        assert_eq!(architecture.relations.len(), 2);
        architecture.validate().unwrap();
    }

    #[test]
    fn juvenile_target_is_a_discrete_analog_not_a_scaled_body_plan() {
        let architecture = default_architecture();
        let target = architecture.construction_target(JUVENILE_LINEAR_SCALE).unwrap();
        assert_eq!(target.elements.len(), 6);
        assert!(target.elements.iter().all(|element| {
            element.placement.x.abs() < 2.0 && element.placement.y.abs() < 2.0
        }));
    }
}
