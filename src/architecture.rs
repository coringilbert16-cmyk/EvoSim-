//! Genome-owned region-level architectural intent and discrete construction targets.
use crate::resources::Material;
use crate::structural_blueprint::{
    BlueprintConnection, BlueprintElement, BlueprintPlacement, StructuralBlueprint,
};
use serde::{Deserialize, Serialize};

/// Juveniles inherit the complete adult developmental field. Developmental\n/// stage changes how much of that field is realized, not the blueprint itself.\npub const JUVENILE_LINEAR_SCALE: f64 = 1.0;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ArchitectureRole {
    ConstructionAnchor,
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
    pub anchor_region: usize,
    pub target_scale: f64,
}

impl OrganismArchitecture {
    pub fn validate(&self) -> Result<(), String> {
        if self.regions.is_empty() || self.anchor_region >= self.regions.len() {
            return Err("architecture has no valid construction anchor region".into());
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
        if !self
            .regions
            .iter()
            .any(|r| matches!(r.role, ArchitectureRole::StructuralBoundary))
            || !self
                .regions
                .iter()
                .any(|r| matches!(r.role, ArchitectureRole::Interface))
        {
            return Err("architecture requires boundary and interface regions".into());
        }
        if !matches!(
            self.regions[self.anchor_region].role,
            ArchitectureRole::ConstructionAnchor
        ) {
            return Err("anchor_region must identify ConstructionAnchor".into());
        }
        Ok(())
    }

    pub fn adult_construction_target(&self) -> Result<StructuralBlueprint, String> {
        self.construction_target(self.target_scale.clamp(JUVENILE_LINEAR_SCALE, 1.0))
    }

    pub fn developmental_target(
        &self,
        requested_scale: f64,
        catalog: &[crate::resources::BaseResource],
    ) -> Result<StructuralBlueprint, String> {
        self.validate()?;
        let requested = requested_scale.clamp(JUVENILE_LINEAR_SCALE, 1.0);
        let mut candidates = vec![requested];
        for step in 1..=12 {
            candidates.push((requested + step as f64 * 0.05).min(1.0));
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
                        crate::juvenile_requirements::JuvenileViabilityRequirements::default(),
                    )
                    .is_ok() =>
                {
                    return Ok(target);
                }
                Ok(_) => last_error = format!("scale {scale:.2} failed juvenile viability"),
                Err(error) => last_error = format!("scale {scale:.2} failed realization: {error}"),
            }
        }
        Err(last_error)
    }

    fn construction_target(&self, scale: f64) -> Result<StructuralBlueprint, String> {
        self.validate()?;
        let anchor = &self.regions[self.anchor_region];
        let boundary = self
            .regions
            .iter()
            .find(|r| matches!(r.role, ArchitectureRole::StructuralBoundary))
            .ok_or("architecture requires a structural boundary")?;
        let interface = self
            .regions
            .iter()
            .find(|r| matches!(r.role, ArchitectureRole::Interface))
            .ok_or("architecture requires an interface")?;
        let mut elements = Vec::new();
        let mut connections = Vec::new();
        add_anchor(&mut elements, &mut connections, anchor);
        add_boundary(&mut elements, &mut connections, boundary, scale);
        add_interface(&mut elements, &mut connections, interface, scale);
        let target = StructuralBlueprint::with_anchor_elements(elements, connections, vec![0]);
        target.validate()?;
        Ok(target)
    }
}

fn add_anchor(
    elements: &mut Vec<BlueprintElement>,
    connections: &mut Vec<BlueprintConnection>,
    region: &ArchitectureRegion,
) {
    let d = (1.511_858 + 0.330_719) / 2.0;
    for (x, y, rotation_radians) in [
        (region.center_x, region.center_y + d, 0.0),
        (
            region.center_x - d,
            region.center_y,
            std::f64::consts::FRAC_PI_2,
        ),
        (
            region.center_x + d,
            region.center_y,
            std::f64::consts::FRAC_PI_2,
        ),
        (region.center_x, region.center_y - d, 0.0),
    ] {
        elements.push(BlueprintElement {
            material: region.material.clone(),
            placement: BlueprintPlacement {
                x,
                y,
                rotation_radians,
            },
        });
    }
    for (a, b) in [(0, 1), (0, 2), (1, 3), (2, 3)] {
        connections.push(BlueprintConnection {
            element_a: a,
            element_b: b,
        });
    }
}

fn add_boundary(
    elements: &mut Vec<BlueprintElement>,
    connections: &mut Vec<BlueprintConnection>,
    region: &ArchitectureRegion,
    scale: f64,
) {
    let hs = 1.511_858 / 2.0;
    let off = 1.677_217_5;
    let start = elements.len();
    let x = region.center_x;
    let y = region.center_y;
    let count = ((8.0 * region.density * scale).round() as usize).clamp(4, 8);
    let positions = if count <= 4 {
        vec![
            (x, y + off, 0.0),
            (x + off, y, std::f64::consts::FRAC_PI_2),
            (x, y - off, 0.0),
            (x - off, y, std::f64::consts::FRAC_PI_2),
        ]
    } else {
        vec![
            (x - hs, y + off, 0.0),
            (x + hs, y + off, 0.0),
            (x - off, y - hs, std::f64::consts::FRAC_PI_2),
            (x - off, y + hs, std::f64::consts::FRAC_PI_2),
            (x + off, y - hs, std::f64::consts::FRAC_PI_2),
            (x + off, y + hs, std::f64::consts::FRAC_PI_2),
            (x - hs, y - off, 0.0),
            (x + hs, y - off, 0.0),
        ]
    };
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
    if count <= 4 {
        for i in 0..4 {
            connections.push(BlueprintConnection {
                element_a: start + i,
                element_b: start + (i + 1) % 4,
            });
        }
    } else {
        let ring = [0, 1, 5, 4, 7, 6, 2, 3];
        for i in 0..8 {
            connections.push(BlueprintConnection {
                element_a: start + ring[i],
                element_b: start + ring[(i + 1) % 8],
            });
        }
    }
}

fn add_interface(
    elements: &mut Vec<BlueprintElement>,
    connections: &mut Vec<BlueprintConnection>,
    region: &ArchitectureRegion,
    scale: f64,
) {
    let inner: f64 = 1.086_648;
    let outer: f64 = 1.511_858;
    let length: f64 = 0.797_884;
    let gap = outer - inner;
    let tangent: f64 = (length * length - gap * gap).sqrt();
    let center = (inner + outer) / 2.0;
    let start = elements.len();
    let p = [
        (
            region.center_x,
            region.center_y + center,
            gap.atan2(-tangent),
        ),
        (
            region.center_x - center,
            region.center_y,
            (-tangent).atan2(-gap),
        ),
        (
            region.center_x + center,
            region.center_y,
            tangent.atan2(gap),
        ),
        (
            region.center_x,
            region.center_y - center,
            (-gap).atan2(tangent),
        ),
    ];
    for (x, y, rotation_radians) in p {
        elements.push(BlueprintElement {
            material: region.material.clone(),
            placement: BlueprintPlacement {
                x,
                y,
                rotation_radians,
            },
        });
    }
    let boundary_count = ((8.0 * region.density * scale).round() as usize).clamp(4, 8);
    let boundary_start = start - boundary_count;
    let maps = if boundary_count == 4 {
        [0, 1, 2, 3]
    } else {
        [0, 2, 4, 6]
    };
    for (i, map) in maps.iter().enumerate() {
        connections.push(BlueprintConnection {
            element_a: start + i,
            element_b: boundary_start + *map,
        });
        connections.push(BlueprintConnection {
            element_a: start + i,
            element_b: i,
        });
    }
}

pub fn default_architecture() -> OrganismArchitecture {
    OrganismArchitecture {
        regions: vec![
            ArchitectureRegion {
                role: ArchitectureRole::ConstructionAnchor,
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
                density: 1.0,
            },
            ArchitectureRegion {
                role: ArchitectureRole::Interface,
                material: Material::free_base("Hydrogen", 1.0),
                center_x: 0.0,
                center_y: 0.0,
                extent_x: 1.0,
                extent_y: 1.0,
                density: 1.0,
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
        anchor_region: 0,
        target_scale: 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn architecture_is_region_level() {
        let architecture = default_architecture();
        assert!(architecture.validate().is_ok());
        assert_eq!(architecture.regions.len(), 3);
    }

    #[test]
    fn juvenile_target_preserves_the_full_adult_developmental_field() {
        let architecture = default_architecture();
        let target = architecture.construction_target(1.0).unwrap();
        assert_eq!(target.elements.len(), 16);
        assert!(target
            .elements
            .iter()
            .all(|element| { element.placement.x.abs() < 2.0 && element.placement.y.abs() < 2.0 }));
        assert_eq!(target.anchor_elements, vec![0]);
    }
}
