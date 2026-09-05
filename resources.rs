use serde::{Deserialize, Serialize};

use crate::math::{complexity, exponential_influence};

/// Immutable type properties. These never change and never evolve.
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct ResourceProperties {
    pub mass: f64,
    pub potential_energy: f64,
    pub reactivity: f64,
    pub cohesion: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BaseResource {
    pub name: String,
    pub properties: ResourceProperties,

    // Immutable geometric representation of this resource type (never
    // evolves, same status as `properties` - see Shape doc comment).
    pub shape: Shape,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Form {
    Circle { radius: f64 },
    Rectangle { width: f64, height: f64 },
    RegularPolygon { sides: u8, radius: f64 },
    Polygon { vertices: Vec<(f64, f64)> },
    Fluid { nominal_area: f64 },
}

impl Form {
    pub fn is_valid(&self) -> bool {
        match self {
            Form::Circle { radius } => radius.is_finite() && *radius > 0.0,
            Form::Rectangle { width, height } => {
                width.is_finite() && height.is_finite() && *width > 0.0 && *height > 0.0
            }
            Form::RegularPolygon { sides, radius } => {
                *sides >= 3 && radius.is_finite() && *radius > 0.0
            }
            Form::Polygon { vertices } => {
                vertices.len() >= 3 && vertices.iter().all(|(x, y)| x.is_finite() && y.is_finite())
            }
            Form::Fluid { nominal_area } => nominal_area.is_finite() && *nominal_area > 0.0,
        }
    }

    pub fn polygon_vertices(&self) -> Option<Vec<(f64, f64)>> {
        match self {
            Form::Circle { .. } | Form::Fluid { .. } => None,
            Form::Rectangle { width, height } => {
                let hw = width / 2.0;
                let hh = height / 2.0;
                Some(vec![(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)])
            }
            Form::RegularPolygon { sides, radius } => {
                let n = *sides as usize;
                Some(
                    (0..n)
                        .map(|k| {
                            let angle = (k as f64) * std::f64::consts::TAU / (*sides as f64);
                            (radius * angle.cos(), radius * angle.sin())
                        })
                        .collect(),
                )
            }
            Form::Polygon { vertices } => Some(vertices.clone()),
        }
    }

    pub fn bounding_radius(&self) -> f64 {
        match self {
            Form::Circle { radius } => *radius,
            Form::Rectangle { width, height } => {
                ((width / 2.0).powi(2) + (height / 2.0).powi(2)).sqrt()
            }
            Form::RegularPolygon { radius, .. } => *radius,
            Form::Polygon { vertices } => vertices
                .iter()
                .map(|(x, y)| (x * x + y * y).sqrt())
                .fold(0.0_f64, f64::max),
            Form::Fluid { nominal_area } => (nominal_area / std::f64::consts::PI).sqrt(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct ConnectionPoint {
    pub x: f64,
    pub y: f64,
    pub direction_radians: f64,
}

impl ConnectionPoint {
    pub fn is_valid(&self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.direction_radians.is_finite()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ConnectionSites {
    Corners(Vec<ConnectionPoint>),
    Circumference { radius: f64 },
    Undetermined,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Shape {
    pub form: Form,
}

impl Shape {
    pub fn is_valid(&self) -> bool {
        self.form.is_valid()
    }

    pub fn connection_sites(&self) -> ConnectionSites {
        match &self.form {
            Form::Circle { radius } => ConnectionSites::Circumference { radius: *radius },
            Form::Fluid { .. } => ConnectionSites::Undetermined,
            other => {
                let vertices = other
                    .polygon_vertices()
                    .expect("rigid non-Circle/Fluid forms always resolve to a vertex list");
                let points = vertices
                    .into_iter()
                    .map(|(x, y)| ConnectionPoint {
                        x,
                        y,
                        direction_radians: y.atan2(x),
                    })
                    .collect();
                ConnectionSites::Corners(points)
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct ResourceBaselines {
    pub mass: f64,
    pub potential_energy: f64,
    pub reactivity: f64,
    pub cohesion: f64,
}

impl ResourceBaselines {
    pub fn from_catalog(catalog: &[BaseResource]) -> Self {
        if catalog.is_empty() {
            return Self {
                mass: 0.0,
                potential_energy: 0.0,
                reactivity: 0.0,
                cohesion: 0.0,
            };
        }
        let count = catalog.len() as f64;
        Self {
            mass: catalog.iter().map(|r| r.properties.mass).sum::<f64>() / count,
            potential_energy: catalog
                .iter()
                .map(|r| r.properties.potential_energy)
                .sum::<f64>()
                / count,
            reactivity: catalog.iter().map(|r| r.properties.reactivity).sum::<f64>() / count,
            cohesion: catalog.iter().map(|r| r.properties.cohesion).sum::<f64>() / count,
        }
    }
}

/// One structural connection between two material constituents.
///
/// The indices refer to entries in `Material::parts`. Structure is therefore
/// part of the material itself rather than a second, parallel representation.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct InternalBond {
    pub part_a: usize,
    pub part_b: usize,
}

impl InternalBond {
    pub fn is_valid_for(&self, part_count: usize) -> bool {
        self.part_a < part_count
            && self.part_b < part_count
            && self.part_a != self.part_b
    }
}

/// A material is defined by its composition and its internal structure.
///
/// `parts` preserves constituent identity. Two constituents with the same
/// resource name remain separate entries when their structural identity must
/// be preserved; aggregation is an ecological-storage concern, not a change
/// to the physical identity of the material.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Material {
    pub parts: Vec<(String, f64)>,
    pub internal_bonds: Vec<InternalBond>,
}

impl Material {
    /// Create unstructured base stock. A collection of base-resource units is
    /// not itself a bonded structure merely because it contains multiple
    /// units of the same resource.
    pub fn free_base(name: impl Into<String>, amount: f64) -> Self {
        Self {
            parts: vec![(name.into(), amount)],
            internal_bonds: Vec::new(),
        }
    }

    /// Validate both halves of the physical material identity: composition
    /// and structure. This is the authoritative structural validity check.
    pub fn is_valid(&self) -> bool {
        if self.parts.is_empty() {
            return self.internal_bonds.is_empty();
        }

        if !self.parts.iter().all(|(name, amount)| {
            !name.is_empty() && amount.is_finite() && *amount > 0.0
        }) {
            return false;
        }

        for (i, bond) in self.internal_bonds.iter().enumerate() {
            if !bond.is_valid_for(self.parts.len()) {
                return false;
            }
            if self.internal_bonds[..i].iter().any(|previous| {
                previous == bond
                    || (previous.part_a == bond.part_b && previous.part_b == bond.part_a)
            }) {
                return false;
            }
        }

        true
    }

    /// A material has physical internal structure exactly when its structure
    /// contains at least one internal bond. No independent bonding flag exists.
    pub fn has_internal_structure(&self) -> bool {
        !self.internal_bonds.is_empty()
    }

    /// Potential energy is NOT stored on Material. It is derived on demand
    /// from immutable per-resource-type properties in the catalog, per the
    /// locked rule that potential energy is an absolute maximum tied to the
    /// resource type, not a depleting quantity carried by a material stack.
    pub fn potential_energy(&self, catalog: &[BaseResource]) -> f64 {
        self.parts
            .iter()
            .map(|(name, amount)| fresh_energy(catalog, name, *amount))
            .sum()
    }

    pub fn total_amount(&self) -> f64 {
        self.parts.iter().map(|(_, a)| *a).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.total_amount() <= 1e-12
    }

    /// BREAK is possible only for material that actually contains internal
    /// structure and has at least two units to separate.
    pub fn can_break(&self) -> bool {
        self.has_internal_structure() && self.total_amount() >= 2.0 - 1e-9
    }

    pub fn mass(&self, catalog: &[BaseResource]) -> f64 {
        self.parts
            .iter()
            .map(|(name, amount)| {
                catalog
                    .iter()
                    .find(|b| &b.name == name)
                    .map(|b| b.properties.mass * amount)
                    .unwrap_or(0.0)
            })
            .sum()
    }

    pub fn weighted_properties(&self, catalog: &[BaseResource]) -> ResourceProperties {
        let mut mass = 0.0;
        let mut pe = 0.0;
        let mut reac = 0.0;
        let mut coh = 0.0;
        let mut w = 0.0;
        for (name, amount) in &self.parts {
            if *amount <= 0.0 {
                continue;
            }
            if let Some(base) = catalog.iter().find(|b| &b.name == name) {
                w += amount;
                mass += base.properties.mass * amount;
                pe += base.properties.potential_energy * amount;
                reac += base.properties.reactivity * amount;
                coh += base.properties.cohesion * amount;
            }
        }
        if w <= 0.0 {
            return ResourceProperties {
                mass: 0.0,
                potential_energy: 0.0,
                reactivity: 0.0,
                cohesion: 0.0,
            };
        }
        ResourceProperties {
            mass: mass / w,
            potential_energy: pe / w,
            reactivity: reac / w,
            cohesion: coh / w,
        }
    }

    pub fn take(&mut self, amount: f64) -> Option<Material> {
        let total = self.total_amount();
        if amount <= 0.0 || total <= 0.0 {
            return None;
        }
        let taken = amount.min(total);
        let frac = taken / total;
        let mut parts = Vec::new();
        for (name, qty) in &mut self.parts {
            let piece = *qty * frac;
            *qty -= piece;
            parts.push((name.clone(), piece));
        }
        self.parts.retain(|(_, q)| *q > 1e-12);
        Some(Material {
            parts,
            internal_bonds: self.internal_bonds.clone(),
        })
    }
}

pub fn merge_parts(parts: &[(String, f64)]) -> Vec<(String, f64)> {
    let mut out: Vec<(String, f64)> = Vec::new();
    for (name, amount) in parts {
        if let Some(existing) = out.iter_mut().find(|(n, _)| n == name) {
            existing.1 += amount;
        } else {
            out.push((name.clone(), *amount));
        }
    }
    out.retain(|(_, a)| *a > 1e-12);
    out
}

/// Combine materials into one physical material while preserving every input
/// constituent and remapping every existing internal bond. Each additional
/// input is joined to the first constituent of the preceding input, giving
/// COMBINE an actual structural result instead of a Boolean bonding state.
pub fn combine_materials(inputs: &[Material]) -> Material {
    let mut parts = Vec::new();
    let mut internal_bonds = Vec::new();
    let mut previous_first = None;

    for material in inputs.iter().filter(|material| !material.is_empty()) {
        let offset = parts.len();
        parts.extend(material.parts.iter().cloned());

        for bond in &material.internal_bonds {
            internal_bonds.push(InternalBond {
                part_a: bond.part_a + offset,
                part_b: bond.part_b + offset,
            });
        }

        if let Some(previous) = previous_first {
            internal_bonds.push(InternalBond {
                part_a: previous,
                part_b: offset,
            });
        }
        previous_first = Some(offset);
    }

    let result = Material {
        parts,
        internal_bonds,
    };
    debug_assert!(result.is_valid());
    result
}

pub fn combine_work_cost(material: &Material, catalog: &[BaseResource], water_field: f64) -> f64 {
    let n = material.total_amount().max(2.0);
    let props = material.weighted_properties(catalog);
    let reac = exponential_influence(effective_reactivity(props.reactivity, water_field));
    let cohesion = props.cohesion.clamp(0.0, 1.0);
    let c = complexity(n);
    (c * (1.0 + cohesion) * (1.25 - reac)).max(0.2)
}

pub fn effective_reactivity(reactivity: f64, water_field: f64) -> f64 {
    reactivity / (1.0 + water_field.max(0.0))
}

pub fn property_ranges(catalog: &[BaseResource]) -> ResourceProperties {
    if catalog.is_empty() {
        return ResourceProperties {
            mass: 1.0,
            potential_energy: 1.0,
            reactivity: 1.0,
            cohesion: 1.0,
        };
    }
    let mut min_mass = f64::INFINITY;
    let mut max_mass = f64::NEG_INFINITY;
    let mut min_energy = f64::INFINITY;
    let mut max_energy = f64::NEG_INFINITY;
    let mut min_reac = f64::INFINITY;
    let mut max_reac = f64::NEG_INFINITY;
    let mut min_coh = f64::INFINITY;
    let mut max_coh = f64::NEG_INFINITY;
    for r in catalog {
        min_mass = min_mass.min(r.properties.mass);
        max_mass = max_mass.max(r.properties.mass);
        min_energy = min_energy.min(r.properties.potential_energy);
        max_energy = max_energy.max(r.properties.potential_energy);
        let er = exponential_influence(r.properties.reactivity);
        min_reac = min_reac.min(er);
        max_reac = max_reac.max(er);
        min_coh = min_coh.min(r.properties.cohesion);
        max_coh = max_coh.max(r.properties.cohesion);
    }
    ResourceProperties {
        mass: (max_mass - min_mass).max(f64::EPSILON),
        potential_energy: (max_energy - min_energy).max(f64::EPSILON),
        reactivity: (max_reac - min_reac).max(f64::EPSILON),
        cohesion: (max_coh - min_coh).max(f64::EPSILON),
    }
}

pub const NOMINAL_UNIT_AREA: f64 = 0.5;

pub fn default_catalog() -> Vec<BaseResource> {
    vec![
        BaseResource {
            name: "Carbon".into(),
            properties: ResourceProperties {
                mass: 1.0,
                potential_energy: 1.0,
                reactivity: 0.1,
                cohesion: 0.95,
            },
            shape: Shape {
                form: Form::RegularPolygon {
                    sides: 6,
                    radius: 0.438_691,
                },
            },
        },
        BaseResource {
            name: "Methane".into(),
            properties: ResourceProperties {
                mass: 1.0,
                potential_energy: 20.0,
                reactivity: 4.0,
                cohesion: 0.1,
            },
            shape: Shape {
                form: Form::RegularPolygon {
                    sides: 3,
                    radius: 0.620_403,
                },
            },
        },
        BaseResource {
            name: "Hydrogen".into(),
            properties: ResourceProperties {
                mass: 1.0,
                potential_energy: 12.0,
                reactivity: 3.0,
                cohesion: 0.05,
            },
            shape: Shape {
                form: Form::Circle { radius: 0.398_942 },
            },
        },
        BaseResource {
            name: "Sulfur".into(),
            properties: ResourceProperties {
                mass: 1.0,
                potential_energy: 8.0,
                reactivity: 2.0,
                cohesion: 0.4,
            },
            shape: Shape {
                form: Form::RegularPolygon {
                    sides: 5,
                    radius: 0.458_577,
                },
            },
        },
        BaseResource {
            name: "Nitrogen".into(),
            properties: ResourceProperties {
                mass: 1.0,
                potential_energy: 0.5,
                reactivity: 0.2,
                cohesion: 0.7,
            },
            shape: Shape {
                form: Form::Rectangle {
                    width: 1.511_858,
                    height: 0.330_719,
                },
            },
        },
        BaseResource {
            name: "Phosphorus".into(),
            properties: ResourceProperties {
                mass: 1.0,
                potential_energy: 0.8,
                reactivity: 0.3,
                cohesion: 0.6,
            },
            shape: Shape {
                form: Form::Polygon {
                    vertices: vec![
                        (-0.408_248, -0.408_248),
                        (0.408_248, -0.408_248),
                        (0.408_248, 0.0),
                        (0.0, 0.0),
                        (0.0, 0.408_248),
                        (-0.408_248, 0.408_248),
                    ],
                },
            },
        },
        BaseResource {
            name: "Water".into(),
            properties: ResourceProperties {
                mass: 1.0,
                potential_energy: 0.0,
                reactivity: 0.0,
                cohesion: 0.5,
            },
            shape: Shape {
                form: Form::Fluid {
                    nominal_area: NOMINAL_UNIT_AREA,
                },
            },
        },
    ]
}

pub fn fresh_energy(catalog: &[BaseResource], name: &str, amount: f64) -> f64 {
    catalog
        .iter()
        .find(|b| b.name == name)
        .map(|b| b.properties.potential_energy * amount)
        .unwrap_or(0.0)
}

#[cfg(test)]
mod shape_tests {
    use super::*;

    #[test]
    fn catalog_still_constructs_with_seven_resources() {
        let catalog = default_catalog();
        assert_eq!(catalog.len(), 7);
        let expected_names = [
            "Carbon",
            "Methane",
            "Hydrogen",
            "Sulfur",
            "Nitrogen",
            "Phosphorus",
            "Water",
        ];
        for name in expected_names {
            assert!(catalog.iter().any(|r| r.name == name));
        }
    }

    #[test]
    fn every_catalog_resource_has_a_valid_shape() {
        for resource in default_catalog() {
            assert!(resource.shape.is_valid());
        }
    }

    #[test]
    fn form_parameters_are_valid() {
        for resource in default_catalog() {
            assert!(resource.shape.form.is_valid());
        }
    }

    #[test]
    fn polygon_vertices_resolve_correctly_per_form() {
        for resource in default_catalog() {
            match &resource.shape.form {
                Form::Circle { .. } | Form::Fluid { .. } => {
                    assert!(resource.shape.form.polygon_vertices().is_none());
                }
                Form::Rectangle { .. } => {
                    assert_eq!(resource.shape.form.polygon_vertices().unwrap().len(), 4);
                }
                Form::RegularPolygon { sides, .. } => {
                    assert_eq!(resource.shape.form.polygon_vertices().unwrap().len(), *sides as usize);
                }
                Form::Polygon { vertices } => {
                    assert_eq!(resource.shape.form.polygon_vertices().unwrap().len(), vertices.len());
                }
            }
        }
    }

    #[test]
    fn locked_resource_geometry_assignments_are_correct() {
        let catalog = default_catalog();
        let find = |name: &str| catalog.iter().find(|r| r.name == name).unwrap();
        assert!(matches!(find("Hydrogen").shape.form, Form::Circle { .. }));
        assert!(matches!(find("Carbon").shape.form, Form::RegularPolygon { sides: 6, .. }));
        assert!(matches!(find("Methane").shape.form, Form::RegularPolygon { sides: 3, .. }));
        assert!(matches!(find("Sulfur").shape.form, Form::RegularPolygon { sides: 5, .. }));
        assert!(matches!(find("Nitrogen").shape.form, Form::Rectangle { .. }));
        assert!(matches!(
            &find("Phosphorus").shape.form,
            Form::Polygon { vertices } if vertices.len() == 6
        ));
    }

    #[test]
    fn every_polygonal_resource_has_one_connection_point_per_corner() {
        for resource in default_catalog() {
            let expected = match &resource.shape.form {
                Form::Circle { .. } | Form::Fluid { .. } => continue,
                Form::Rectangle { .. } => 4,
                Form::RegularPolygon { sides, .. } => *sides as usize,
                Form::Polygon { vertices } => vertices.len(),
            };
            match resource.shape.connection_sites() {
                ConnectionSites::Corners(points) => assert_eq!(points.len(), expected),
                other => panic!("unexpected connection sites: {other:?}"),
            }
        }
    }

    #[test]
    fn polygon_connection_points_correspond_to_actual_vertices() {
        for resource in default_catalog() {
            let Some(vertices) = resource.shape.form.polygon_vertices() else { continue };
            let ConnectionSites::Corners(points) = resource.shape.connection_sites() else { panic!("not corners") };
            assert_eq!(points.len(), vertices.len());
            for (point, vertex) in points.iter().zip(vertices.iter()) {
                assert_eq!((point.x, point.y), *vertex);
            }
        }
    }

    #[test]
    fn connection_points_are_valid_where_present() {
        for resource in default_catalog() {
            if let ConnectionSites::Corners(points) = resource.shape.connection_sites() {
                for cp in points { assert!(cp.is_valid()); }
            }
        }
    }

    #[test]
    fn circle_has_no_finite_connection_point_list() {
        let circle_resources: Vec<_> = default_catalog().into_iter().filter(|r| matches!(r.shape.form, Form::Circle { .. })).collect();
        assert_eq!(circle_resources.len(), 1);
        for resource in circle_resources {
            assert!(matches!(resource.shape.connection_sites(), ConnectionSites::Circumference { radius } if radius > 0.0));
        }
    }

    #[test]
    fn connection_point_has_no_independent_strength_field() {
        let ConnectionPoint { x: _, y: _, direction_radians: _ } = ConnectionPoint { x: 0.0, y: 0.0, direction_radians: 0.0 };
    }

    #[test]
    fn every_base_resource_unit_has_the_same_nominal_area() {
        fn polygon_area(vertices: &[(f64, f64)]) -> f64 {
            let mut sum = 0.0;
            for i in 0..vertices.len() {
                let (x1, y1) = vertices[i];
                let (x2, y2) = vertices[(i + 1) % vertices.len()];
                sum += x1 * y2 - x2 * y1;
            }
            (sum / 2.0).abs()
        }
        const EPS: f64 = 1e-4;
        for resource in default_catalog() {
            let area = match &resource.shape.form {
                Form::Circle { radius } => std::f64::consts::PI * radius * radius,
                Form::Fluid { nominal_area } => *nominal_area,
                other => polygon_area(&other.polygon_vertices().unwrap()),
            };
            assert!((area - NOMINAL_UNIT_AREA).abs() < EPS);
        }
    }

    #[test]
    fn water_is_a_fluid_with_undetermined_connection_sites() {
        let water = default_catalog().into_iter().find(|r| r.name == "Water").unwrap();
        assert!(matches!(water.shape.form, Form::Fluid { .. }));
        assert_eq!(water.shape.connection_sites(), ConnectionSites::Undetermined);
    }

    #[test]
    fn every_resource_has_a_unique_shape() {
        let catalog = default_catalog();
        assert_eq!(catalog.iter().filter(|r| matches!(r.shape.form, Form::Circle { .. })).count(), 1);
        for i in 0..catalog.len() {
            for j in (i + 1)..catalog.len() {
                assert_ne!(catalog[i].shape.form, catalog[j].shape.form);
            }
        }
    }

    #[test]
    fn shape_vocabulary_is_actually_exercised() {
        let catalog = default_catalog();
        assert!(catalog.iter().any(|r| matches!(r.shape.form, Form::Polygon { .. })));
        assert!(catalog.iter().any(|r| matches!(r.shape.form, Form::RegularPolygon { sides, .. } if sides != 6)));
    }

    #[test]
    fn serialization_round_trip_preserves_shape() {
        for resource in default_catalog() {
            let json = serde_json::to_string(&resource).unwrap();
            let restored: BaseResource = serde_json::from_str(&json).unwrap();
            assert_eq!(restored.name, resource.name);
            assert_eq!(restored.shape.form, resource.shape.form);
            assert_eq!(restored.shape.connection_sites(), resource.shape.connection_sites());
        }
    }
}
