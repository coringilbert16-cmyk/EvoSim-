//! Genome-owned region-level architectural intent and discrete construction targets.
use crate::resources::Material;
use crate::structural_blueprint::{
    BlueprintConnection, BlueprintElement, BlueprintPlacement, StructuralBlueprint,
};
use serde::{Deserialize, Serialize};

pub const JUVENILE_LINEAR_SCALE: f64 = 0.40;

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

    /// The known-viable seed-cell construction. This is the organism's
    /// canonical juvenile realization; it is not an adult blueprint shrunk
    /// until it happens to become viable.
    pub fn juvenile_construction_target(&self) -> Result<StructuralBlueprint, String> {
        self.construction_target(JUVENILE_LINEAR_SCALE)
    }

    /// The same inherited developmental architecture at its adult extent.
    /// Growth expands realization from the canonical juvenile target toward
    /// this target rather than switching to a separately authored body plan.
    pub fn adult_construction_target(&self) -> Result<StructuralBlueprint, String> {
        self.construction_target(self.target_scale.max(JUVENILE_LINEAR_SCALE))
    }

    pub fn developmental_target(
        &self,
        requested_scale: f64,
        _catalog: &[crate::resources::BaseResource],
    ) -> Result<StructuralBlueprint, String> {
        self.validate()?;
        let requested = requested_scale.clamp(
            JUVENILE_LINEAR_SCALE,
            self.target_scale.max(JUVENILE_LINEAR_SCALE),
        );
        self.construction_target(requested)
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
        let boundary_start = add_boundary(&mut elements, &mut connections, boundary, scale);
        add_interface(
            &mut elements,
            &mut connections,
            interface,
            scale,
            boundary_start,
        );
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
) -> usize {
    let growth = scale / JUVENILE_LINEAR_SCALE;
    let hs = 1.511_858 / 2.0 * growth;
    let off = 1.677_217_5 * growth;
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
    start
}

fn add_interface(
    elements: &mut Vec<BlueprintElement>,
    connections: &mut Vec<BlueprintConnection>,
    region: &ArchitectureRegion,
    scale: f64,
    boundary_start: usize,
) {
    let growth = scale / JUVENILE_LINEAR_SCALE;
    let inner: f64 = 1.086_648 * growth;
    let outer: f64 = 1.511_858 * growth;
    let length: f64 = 0.797_884 * growth;
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
    let maps = if boundary_count == 4 {
        [0, 1, 2, 3]
    } else {
        [0, 2, 4, 6]
    };

    // The genome/core remains full size while the surrounding architecture
    // grows. A single fixed-length interface constituent cannot stretch with
    // that gap, so the developmental field increases interface material
    // composition instead. The blueprint remains region-level intent; the
    // physical graph receives the additional bonded constituents at realization.
    let anchor_width = 1.511_858;
    let anchor_height = 0.330_719;
    let anchor_center = (anchor_width + anchor_height) / 2.0;
    let boundary_half_width = 1.511_858 / 2.0 * growth;
    let boundary_half_height = 0.330_719 / 2.0;
    let boundary_offset = 1.677_217_5 * growth;
    let anchor_point = (-anchor_width / 2.0, anchor_center + anchor_height / 2.0);
    let boundary_x = if boundary_count == 4 {
        -anchor_width / 2.0
    } else {
        -boundary_half_width + anchor_width / 2.0
    };
    let boundary_point = (boundary_x, boundary_offset - boundary_half_height);
    let interface_span =
        (boundary_point.0 - anchor_point.0).hypot(boundary_point.1 - anchor_point.1);
    let segment_count = (interface_span / length).ceil().max(1.0) as usize;
    let interface_material = interface_material(region, segment_count);

    for (i, map) in maps.iter().enumerate() {
        elements.push(BlueprintElement {
            material: interface_material.clone(),
            placement: BlueprintPlacement {
                x: p[i].0,
                y: p[i].1,
                rotation_radians: p[i].2,
            },
        });
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

fn interface_material(region: &ArchitectureRegion, segment_count: usize) -> Material {
    if segment_count <= 1 {
        return region.material.clone();
    }

    let Some((name, amount)) = region.material.parts.first() else {
        return region.material.clone();
    };
    if region.material.parts.len() != 1 || !region.material.internal_bonds.is_empty() {
        return region.material.clone();
    }

    Material {
        parts: (0..segment_count)
            .map(|_| (name.clone(), *amount))
            .collect(),
        internal_bonds: (0..segment_count - 1)
            .map(|i| crate::resources::InternalBond {
                part_a: i,
                part_b: i + 1,
            })
            .collect(),
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
    fn juvenile_target_is_the_canonical_seed_cell_realization() {
        let architecture = default_architecture();
        let target = architecture.juvenile_construction_target().unwrap();
        assert_eq!(target.elements.len(), 12);
        assert_eq!(target.anchor_elements, vec![0]);
    }

    #[test]
    fn adult_target_is_expanded_from_the_same_architecture() {
        let architecture = default_architecture();
        let juvenile = architecture.juvenile_construction_target().unwrap();
        let adult = architecture.adult_construction_target().unwrap();
        assert!(adult.elements.len() > juvenile.elements.len());
        assert_eq!(adult.anchor_elements, juvenile.anchor_elements);

        let juvenile_extent = juvenile
            .elements
            .iter()
            .filter(|element| element.placement.x != 0.0 || element.placement.y != 0.0)
            .map(|element| element.placement.x.hypot(element.placement.y))
            .fold(0.0, f64::max);
        let adult_extent = adult
            .elements
            .iter()
            .filter(|element| element.placement.x != 0.0 || element.placement.y != 0.0)
            .map(|element| element.placement.x.hypot(element.placement.y))
            .fold(0.0, f64::max);
        assert!(adult_extent > juvenile_extent);
        let juvenile_interface_parts = juvenile.elements[8].material.parts.len();
        let adult_interface_parts = adult.elements[12].material.parts.len();
        assert!(adult_interface_parts > juvenile_interface_parts);
    }

    #[test]
    fn developmental_target_is_not_a_viability_search() {
        let architecture = default_architecture();
        let juvenile = architecture
            .developmental_target(JUVENILE_LINEAR_SCALE, &[])
            .unwrap();
        let canonical = architecture.juvenile_construction_target().unwrap();
        assert_eq!(juvenile.elements, canonical.elements);
        assert_eq!(juvenile.connections, canonical.connections);
    }
}
