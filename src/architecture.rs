//! Inherited organism architecture.
//!
//! The genome stores region-level developmental intent. Exact constituents and
//! bonds are generated only as transient construction candidates and are then
//! realized by the physical construction solver.
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
        // Rigid constituents cannot be continuously shrunk. Search discrete
        // realizations around the requested scale and keep the nearest viable one.
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
                    return Ok(target);
                }
                Ok(_) => last_error = format!("scale {scale:.2} failed juvenile viability"),
                Err(error) => {
                    last_error = format!("scale {scale:.2} failed realization: {error}")
                }
            }
        }
        Err(last_error)
    }

    fn construction_target(&self, _scale: f64) -> Result<StructuralBlueprint, String> {
        self.validate()?;
        let core = &self.regions[self.genome_region];
        if !matches!(core.role, ArchitectureRole::GenomeCore) {
            return Err("genome_region must identify GenomeCore".into());
        }
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
        add_core(&mut elements, &mut connections, core);
        add_boundary(&mut elements, &mut connections, boundary);
        add_interface(&mut elements, &mut connections, interface);
        let target =
            StructuralBlueprint::with_core_elements(elements, connections, vec![0, 1, 2, 3]);
        target.validate()?;
        Ok(target)
    }
}

fn add_core(
    elements: &mut Vec<BlueprintElement>,
    connections: &mut Vec<BlueprintConnection>,
    region: &ArchitectureRegion,
) {
    let d = (1.511_858 + 0.330_719) / 2.0;
    let x = region.center_x;
    let y = region.center_y;
    for (px, py, r) in [
        (x, y + d, 0.0),
        (x - d, y, std::f64::consts::FRAC_PI_2),
        (x + d, y, std::f64::consts::FRAC_PI_2),
        (x, y - d, 0.0),
    ] {
        elements.push(BlueprintElement {
            material: region.material.clone(),
            placement: BlueprintPlacement {
                x: px,
                y: py,
                rotation_radians: r,
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
) {
    let hs = 1.511_858 / 2.0;
    let off = 1.677_2175;
    let start = elements.len();
    let x = region.center_x;
    let y = region.center_y;
    let count = ((8.0 * region.density).round() as usize).clamp(4, 8);
    if count <= 4 {
        for (px, py, r) in [
            (x, y + off, 0.0),
            (x + off, y, std::f64::consts::FRAC_PI_2),
            (x, y - off, 0.0),
            (x - off, y, std::f64::consts::FRAC_PI_2),
        ] {
            elements.push(BlueprintElement {
                material: region.material.clone(),
                placement: BlueprintPlacement {
                    x: px,
                    y: py,
                    rotation_radians: r,
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
    let p = [
        (x - hs, y + off, 0.0),
        (x + hs, y + off, 0.0),
        (x - off, y - hs, std::f64::consts::FRAC_PI_2),
        (x - off, y + hs, std::f64::consts::FRAC_PI_2),
        (x + off, y - hs, std::f64::consts::FRAC_PI_2),
        (x + off, y + hs, std::f64::consts::FRAC_PI_2),
        (x - hs, y - off, 0.0),
        (x + hs, y - off, 0.0),
    ];
    for (px, py, r) in p {
        elements.push(BlueprintElement {
            material: region.material.clone(),
            placement: BlueprintPlacement {
                x: px,
                y: py,
                rotation_radians: r,
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

fn add_interface(
    elements: &mut Vec<BlueprintElement>,
    connections: &mut Vec<BlueprintConnection>,
    region: &ArchitectureRegion,
) {
    let inner = 1.086_648;
    let outer = 1.511_858;
    let length = 0.797_884;
    let gap = outer - inner;
    let tangent = (length * length - gap * gap).sqrt();
    let center = (inner + outer) / 2.0;
    let x = region.center_x;
    let y = region.center_y;
    let start = elements.len();
    let p = [
        (x, y + center, gap.atan2(-tangent)),
        (x - center, y, (-tangent).atan2(-gap)),
        (x + center, y, tangent.atan2(gap)),
        (x, y - center, (-gap).atan2(tangent)),
    ];
    for (px, py, r) in p {
        elements.push(BlueprintElement {
            material: region.material.clone(),
            placement: BlueprintPlacement {
                x: px,
                y: py,
                rotation_radians: r,
            },
        });
    }
    let boundary_count = ((8.0 * region.density).round() as usize).clamp(4, 8);
    let boundary_start = start - boundary_count;
    let maps = if boundary_count == 4 {
        [0, 1, 2, 3]
    } else {
        [0, 2, 4, 6]
    };
    for i in 0..4 {
        connections.push(BlueprintConnection {
            element_a: start + i,
            element_b: boundary_start + maps[i],
        });
        connections.push(BlueprintConnection {
            element_a: start + i,
            element_b: [0, 1, 2, 3][i],
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
        genome_region: 0,
        target_scale: 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn architecture_is_region_level() {
        let a = default_architecture();
        assert!(a.validate().is_ok());
        assert_eq!(a.regions.len(), 3);
    }

    #[test]
    fn juvenile_target_is_a_discrete_analog_not_a_scaled_body_plan() {
        let a = default_architecture();
        let t = a.construction_target(JUVENILE_LINEAR_SCALE).unwrap();
        assert_eq!(t.elements.len(), 16);
        assert!(t
            .elements
            .iter()
            .all(|e| e.placement.x.abs() < 2.0 && e.placement.y.abs() < 2.0));
    }
}
