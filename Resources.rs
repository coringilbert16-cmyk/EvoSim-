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
 
// ============================================================
// SHAPE (immutable geometric property)
// ============================================================
//
// Shape sits alongside `properties` as a second immutable, per-type
// resource property. It does not evolve - organisms evolve genome,
// behavior, and process, never a fundamental resource's own physical
// geometry (same status as mass/potential_energy/reactivity/cohesion).
//
// REDESIGN NOTE: the first version of this type was a fixed 3-variant
// enum (Circle/Rectangle/Triangle) with a separate `Dimensions`
// bounding-box struct. That capped the vocabulary at three shapes and
// stored a bounding box that duplicated information already implied
// by the primitive's own parameters. This version instead uses a
// compact *parameterized* primitive system - each Form variant carries
// exactly the numbers needed to reconstruct its own geometry, nothing
// more - plus a `Polygon` variant as an explicit-vertex escape hatch
// for anything not expressible as a circle/rectangle/regular polygon
// (an L-shape, a scalene triangle, a cross, etc). This is the
// "compact parameterized primitive system plus a polygon
// representation for irregular forms" approach: a small vocabulary of
// primitives, each with free parameters (radius, side count, explicit
// vertices), produces a very large space of distinct silhouettes
// without a general-purpose mesh/geometry engine.
//
// This remains deliberately minimal, not a physics/geometry engine:
//   - 2D only. Future 3D emerges by stacking 2D layers, not by adding
//     a third spatial dimension to this representation - so nothing
//     here stores a z-axis, but nothing here prevents a future
//     compound/stacked representation from wrapping several Shapes
//     with layer/depth info added at that (separate) level.
//   - `form` + `connection_points` only. No roughness, flexibility,
//     texture, density fields, mesh data, or surface chemistry -
//     those are explicitly out of scope.
//   - No position/rotation is stored on Shape itself. A base resource
//     type doesn't have "a" position or orientation in the world -
//     only an *instance* placed into a future compound/organism
//     structure will, and that placement (position, rotation, later
//     layer/depth for stacking) belongs to that future compound
//     representation, not to this immutable per-type definition.
//   - Connection points are NOT universal snap points, and their
//     count/position is NEVER derived from the primitive's side
//     count or curvature - they are always explicitly authored per
//     resource. They mark immutable physical locations/orientations
//     on the resource's own geometry where a *future* COMBINE
//     mechanic may attempt a connection. Compatibility rules belong
//     to that future mechanic, not to this representation.
// ============================================================
 
/// A small, deliberately limited vocabulary of 2D primitives. Each
/// variant is parameterized rather than fixed-size, so the same small
/// enum produces a large range of distinct silhouettes:
///   - `RegularPolygon { sides, radius }` alone covers triangles,
///     squares, pentagons, hexagons, etc. - side count is a free
///     parameter, not a separate variant per polygon type.
///   - `Polygon { vertices }` is the explicit escape hatch for any
///     silhouette the parameterized primitives can't express - an
///     L-shape, a scalene triangle, a cross/branching form, a
///     trapezoid, and so on all just become a vertex list under this
///     one variant, rather than one enum variant each.
///
/// Extend this enum only if a genuinely new *primitive family* is
/// needed (not a new named shape - those belong under `Polygon`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Form {
    /// A true circle - kept distinct from the polygon primitives
    /// since it has a curved boundary a renderer draws natively
    /// rather than tessellating.
    Circle { radius: f64 },
 
    /// An axis-aligned rectangle centered on the shape's local
    /// origin. Independent width/height (unlike `RegularPolygon`)
    /// makes this the natural primitive for elongated bar/rod forms.
    Rectangle { width: f64, height: f64 },
 
    /// A regular (equal-sided, equal-angled) polygon with `sides`
    /// vertices at distance `radius` from the local origin, first
    /// vertex fixed at angle 0 (along +x) by convention, remaining
    /// vertices spaced by 2*PI/sides. `sides` is a free parameter:
    /// 3 -> triangle, 5 -> pentagon, 6 -> hexagon, etc.
    RegularPolygon { sides: u8, radius: f64 },
 
    /// An explicit, immutable list of vertices (local coordinates,
    /// wound consistently) describing an irregular silhouette that
    /// the parameterized primitives above can't express - concave
    /// forms (L-shapes, crosses), non-regular polygons (a scalene
    /// triangle), trapezoids, etc. This is the deliberate escape
    /// hatch so the enum doesn't need one variant per named shape.
    Polygon { vertices: Vec<(f64, f64)> },
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
                vertices.len() >= 3
                    && vertices.iter().all(|(x, y)| x.is_finite() && y.is_finite())
            }
        }
    }
 
    /// Resolves this Form into actual 2D vertices, local to the
    /// shape's own origin - the concrete mechanism that lets "many of
    /// these forms ultimately resolve into actual 2D vertices/polygons
    /// without requiring a separate geometry engine for every shape."
    /// `Circle` has no finite vertex list (a renderer draws it
    /// natively as a circle, not a tessellated polygon), so it
    /// returns `None`; every polygonal form returns `Some(vertices)`.
    pub fn polygon_vertices(&self) -> Option<Vec<(f64, f64)>> {
        match self {
            Form::Circle { .. } => None,
 
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
}
 
/// A single immutable connection point on a resource's shape.
/// Position is relative to the shape's own local origin, direction is
/// the outward-facing orientation in radians, and strength is a
/// property of this specific point (per design decision #9 - not a
/// separate global resource property). Always explicitly authored -
/// never derived from the Form's side count or curvature.
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct ConnectionPoint {
    pub x: f64,
    pub y: f64,
    pub direction_radians: f64,
    pub strength: f64,
}
 
impl ConnectionPoint {
    pub fn is_valid(&self) -> bool {
        self.x.is_finite()
            && self.y.is_finite()
            && self.direction_radians.is_finite()
            && self.strength.is_finite()
            && self.strength >= 0.0
    }
}
 
/// The complete immutable geometric property of a resource type.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Shape {
    pub form: Form,
    pub connection_points: Vec<ConnectionPoint>,
}
 
impl Shape {
    pub fn is_valid(&self) -> bool {
        self.form.is_valid() && self.connection_points.iter().all(ConnectionPoint::is_valid)
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
    /// Average of each property across base resource *types*, not abundance.
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
            reactivity: catalog
                .iter()
                .map(|r| r.properties.reactivity)
                .sum::<f64>()
                / count,
            cohesion: catalog.iter().map(|r| r.properties.cohesion).sum::<f64>() / count,
        }
    }
}
 
/// A stack of material in the world or in an organism.
///
/// `bonded == false`: uncombined base stock (a pile of carbon is still
/// uncombined; it cannot BREAK).
/// `bonded == true`: result of COMBINE. BREAK is legal iff total units ≥ 2.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Material {
    pub parts: Vec<(String, f64)>,
    pub bonded: bool,
}
 
impl Material {
    pub fn free_base(name: impl Into<String>, amount: f64) -> Self {
        Self {
            parts: vec![(name.into(), amount)],
            bonded: false,
        }
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
 
    pub fn can_break(&self) -> bool {
        self.bonded && self.total_amount() >= 2.0 - 1e-9
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
            bonded: self.bonded,
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
 
pub fn combine_materials(inputs: &[Material]) -> Material {
    let mut parts = Vec::new();
    for mat in inputs {
        parts.extend(mat.parts.iter().cloned());
    }
    Material {
        parts: merge_parts(&parts),
        bonded: true,
    }
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
 
pub fn default_catalog() -> Vec<BaseResource> {
    vec![
        // Carbon: high cohesion (0.95) -> a hexagon, chosen specifically
        // because hexagonal packing creates useful, varied structural
        // possibilities for future construction mechanics. Six sides
        // does NOT mean six connection points - three are explicitly
        // placed at alternating edge midpoints (not vertices), leaving
        // the other three edges bare. That asymmetry (relative to the
        // shape's own 6-fold symmetry) is deliberate: it's an explicit
        // authoring choice, never derived from side count.
        {
            let radius = 0.6_f64;
            // Apothem (center-to-edge-midpoint distance) for a regular
            // hexagon: radius * cos(PI / sides).
            let apothem = radius * (std::f64::consts::PI / 6.0).cos();
            let angle_a = std::f64::consts::PI / 6.0; // 30 deg
            let angle_b = 5.0 * std::f64::consts::PI / 6.0; // 150 deg
            let angle_c = 3.0 * std::f64::consts::PI / 2.0; // 270 deg
            BaseResource {
                name: "Carbon".into(),
                properties: ResourceProperties {
                    mass: 1.0,
                    potential_energy: 1.0,
                    reactivity: 0.1,
                    cohesion: 0.95,
                },
                shape: Shape {
                    form: Form::RegularPolygon { sides: 6, radius },
                    connection_points: vec![
                        ConnectionPoint { x: apothem * angle_a.cos(), y: apothem * angle_a.sin(), direction_radians: angle_a, strength: 0.9 },
                        ConnectionPoint { x: apothem * angle_b.cos(), y: apothem * angle_b.sin(), direction_radians: angle_b, strength: 0.9 },
                        ConnectionPoint { x: apothem * angle_c.cos(), y: apothem * angle_c.sin(), direction_radians: angle_c, strength: 0.9 },
                    ],
                },
            }
        },
        // Methane: reactive, low cohesion (0.1) -> a small circle with
        // three connection points at deliberately irregular (not evenly
        // spaced) angles - a round, unstable-feeling blob rather than a
        // tidy symmetric one.
        {
            let radius = 0.5_f64;
            let angle_a = 20.0_f64.to_radians();
            let angle_b = 160.0_f64.to_radians();
            let angle_c = 260.0_f64.to_radians();
            BaseResource {
                name: "Methane".into(),
                properties: ResourceProperties {
                    mass: 1.0,
                    potential_energy: 20.0,
                    reactivity: 4.0,
                    cohesion: 0.1,
                },
                shape: Shape {
                    form: Form::Circle { radius },
                    connection_points: vec![
                        ConnectionPoint { x: radius * angle_a.cos(), y: radius * angle_a.sin(), direction_radians: angle_a, strength: 0.2 },
                        ConnectionPoint { x: radius * angle_b.cos(), y: radius * angle_b.sin(), direction_radians: angle_b, strength: 0.2 },
                        ConnectionPoint { x: radius * angle_c.cos(), y: radius * angle_c.sin(), direction_radians: angle_c, strength: 0.2 },
                    ],
                },
            }
        },
        // Hydrogen: smallest, weakest circle - a single connection
        // point, a plain terminal/cap-like piece.
        BaseResource {
            name: "Hydrogen".into(),
            properties: ResourceProperties {
                mass: 1.0,
                potential_energy: 12.0,
                reactivity: 3.0,
                cohesion: 0.05,
            },
            shape: Shape {
                form: Form::Circle { radius: 0.3 },
                connection_points: vec![
                    ConnectionPoint { x: 0.3, y: 0.0, direction_radians: 0.0, strength: 0.15 },
                ],
            },
        },
        // Sulfur: moderate cohesion (0.4) -> an equilateral triangle
        // (RegularPolygon with 3 sides). Only two of its three vertices
        // carry a connection point, again to make clear that point
        // count is never implied by side count - the third vertex is
        // deliberately bare.
        {
            let radius = 0.5_f64;
            let angle_a = 0.0_f64;
            let angle_b = 2.0 * std::f64::consts::PI / 3.0; // 120 deg
            BaseResource {
                name: "Sulfur".into(),
                properties: ResourceProperties {
                    mass: 1.0,
                    potential_energy: 8.0,
                    reactivity: 2.0,
                    cohesion: 0.4,
                },
                shape: Shape {
                    form: Form::RegularPolygon { sides: 3, radius },
                    connection_points: vec![
                        ConnectionPoint { x: radius * angle_a.cos(), y: radius * angle_a.sin(), direction_radians: angle_a, strength: 0.4 },
                        ConnectionPoint { x: radius * angle_b.cos(), y: radius * angle_b.sin(), direction_radians: angle_b, strength: 0.4 },
                    ],
                },
            }
        },
        // Nitrogen: cohesion 0.7 -> an elongated rectangle (a bar/rod
        // silhouette, independent width/height rather than a regular
        // polygon), with one connection point at each end.
        BaseResource {
            name: "Nitrogen".into(),
            properties: ResourceProperties {
                mass: 1.0,
                potential_energy: 0.5,
                reactivity: 0.2,
                cohesion: 0.7,
            },
            shape: Shape {
                form: Form::Rectangle { width: 1.6, height: 0.35 },
                connection_points: vec![
                    ConnectionPoint { x: 0.8, y: 0.0, direction_radians: 0.0, strength: 0.65 },
                    ConnectionPoint { x: -0.8, y: 0.0, direction_radians: std::f64::consts::PI, strength: 0.65 },
                ],
            },
        },
        // Phosphorus: cohesion 0.6 -> an explicit-vertex L-shape. This
        // is the concrete demonstration of the Polygon escape hatch:
        // a concave, asymmetric silhouette that a parameterized
        // primitive can't express. Two connection points sit on its
        // outer edges (moderate strength); the third sits in the
        // concave inner corner - a deliberately awkward, partially
        // enclosed attachment site (weaker strength), which is the
        // kind of "difficult attachment configuration" this
        // representation is meant to make possible.
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
                        (-0.5, -0.5),
                        (0.5, -0.5),
                        (0.5, 0.0),
                        (0.0, 0.0),
                        (0.0, 0.5),
                        (-0.5, 0.5),
                    ],
                },
                connection_points: vec![
                    ConnectionPoint { x: 0.5, y: -0.25, direction_radians: 0.0, strength: 0.55 },
                    ConnectionPoint { x: -0.25, y: 0.5, direction_radians: std::f64::consts::FRAC_PI_2, strength: 0.55 },
                    ConnectionPoint { x: 0.0, y: 0.0, direction_radians: std::f64::consts::FRAC_PI_4, strength: 0.3 },
                ],
            },
        },
        // Water: diluent, cohesion 0.5 -> a pentagon with two
        // connection points at non-adjacent vertices, set apart at a
        // bent angle (a nod to water's familiar bent silhouette, not a
        // claim of real chemistry).
        {
            let radius = 0.4_f64;
            let angle_a = std::f64::consts::FRAC_PI_2; // 90 deg
            let angle_b = 210.0_f64.to_radians();
            BaseResource {
                name: "Water".into(),
                properties: ResourceProperties {
                    mass: 1.0,
                    potential_energy: 0.0,
                    reactivity: 0.0,
                    cohesion: 0.5,
                },
                shape: Shape {
                    form: Form::RegularPolygon { sides: 5, radius },
                    connection_points: vec![
                        ConnectionPoint { x: radius * angle_a.cos(), y: radius * angle_a.sin(), direction_radians: angle_a, strength: 0.5 },
                        ConnectionPoint { x: radius * angle_b.cos(), y: radius * angle_b.sin(), direction_radians: angle_b, strength: 0.5 },
                    ],
                },
            }
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
 
// ============================================================
// TESTS - Shape geometry representation only.
//
// Scoped deliberately narrow per this task: shape validity,
// serialization round-trip, and catalog construction. No COMBINE
// tests - COMBINE is not implemented in this task.
// ============================================================
 
#[cfg(test)]
mod shape_tests {
    use super::*;
 
    #[test]
    fn catalog_still_constructs_with_seven_resources() {
        let catalog = default_catalog();
        assert_eq!(catalog.len(), 7);
 
        let expected_names = [
            "Carbon", "Methane", "Hydrogen", "Sulfur", "Nitrogen", "Phosphorus", "Water",
        ];
        for name in expected_names {
            assert!(
                catalog.iter().any(|r| r.name == name),
                "expected catalog to contain {name}"
            );
        }
    }
 
    #[test]
    fn every_catalog_resource_has_a_valid_shape() {
        for resource in default_catalog() {
            assert!(
                resource.shape.is_valid(),
                "{} has an invalid shape: {:?}",
                resource.name,
                resource.shape
            );
        }
    }
 
    #[test]
    fn form_parameters_are_valid() {
        for resource in default_catalog() {
            assert!(
                resource.shape.form.is_valid(),
                "{} has invalid form parameters: {:?}",
                resource.name,
                resource.shape.form
            );
        }
    }
 
    #[test]
    fn polygon_vertices_resolve_correctly_per_form() {
        for resource in default_catalog() {
            match &resource.shape.form {
                Form::Circle { .. } => {
                    // Circles are rendered natively, not tessellated -
                    // no finite vertex list is expected.
                    assert!(
                        resource.shape.form.polygon_vertices().is_none(),
                        "{} (Circle) should not resolve to vertices",
                        resource.name
                    );
                }
                Form::Rectangle { .. } => {
                    let vertices = resource
                        .shape
                        .form
                        .polygon_vertices()
                        .expect("Rectangle must resolve to vertices");
                    assert_eq!(vertices.len(), 4, "{} rectangle must have 4 vertices", resource.name);
                }
                Form::RegularPolygon { sides, .. } => {
                    let vertices = resource
                        .shape
                        .form
                        .polygon_vertices()
                        .expect("RegularPolygon must resolve to vertices");
                    assert_eq!(
                        vertices.len(),
                        *sides as usize,
                        "{} regular polygon vertex count must match side count",
                        resource.name
                    );
                }
                Form::Polygon { vertices } => {
                    let resolved = resource
                        .shape
                        .form
                        .polygon_vertices()
                        .expect("Polygon must resolve to vertices");
                    assert_eq!(
                        resolved.len(),
                        vertices.len(),
                        "{} polygon vertex count must be preserved",
                        resource.name
                    );
                }
            }
        }
    }
 
    #[test]
    fn carbon_is_a_hexagon_with_explicit_non_derived_connection_points() {
        let catalog = default_catalog();
        let carbon = catalog.iter().find(|r| r.name == "Carbon").unwrap();
 
        match carbon.shape.form {
            Form::RegularPolygon { sides, .. } => assert_eq!(sides, 6, "Carbon must be a hexagon"),
            _ => panic!("Carbon must use RegularPolygon"),
        }
 
        // Six-sided shape, but connection point count is NOT six -
        // proving point count is explicitly authored, not derived from
        // side count.
        assert_ne!(carbon.shape.connection_points.len(), 6);
    }
 
    #[test]
    fn connection_points_are_valid_and_present() {
        for resource in default_catalog() {
            assert!(
                !resource.shape.connection_points.is_empty(),
                "{} has no connection points",
                resource.name
            );
            for cp in &resource.shape.connection_points {
                assert!(
                    cp.is_valid(),
                    "{} has an invalid connection point: {:?}",
                    resource.name,
                    cp
                );
                assert!(
                    cp.strength >= 0.0,
                    "{} connection point strength must be non-negative",
                    resource.name
                );
            }
        }
    }
 
    #[test]
    fn different_resources_have_different_shapes() {
        let catalog = default_catalog();
 
        let carbon = catalog.iter().find(|r| r.name == "Carbon").unwrap();
        let hydrogen = catalog.iter().find(|r| r.name == "Hydrogen").unwrap();
 
        // Different form variant entirely (RegularPolygon vs Circle).
        assert_ne!(carbon.shape.form, hydrogen.shape.form);
 
        // Different connection point counts across the catalog in
        // general (spot-check a few, not exhaustive).
        assert_ne!(
            carbon.shape.connection_points.len(),
            hydrogen.shape.connection_points.len()
        );
 
        // Spot-check that the vocabulary is actually being exercised:
        // at least one Polygon (explicit-vertex escape hatch) and at
        // least one RegularPolygon with a side count other than
        // Carbon's are present in the catalog.
        assert!(
            catalog
                .iter()
                .any(|r| matches!(r.shape.form, Form::Polygon { .. })),
            "expected at least one explicit-vertex Polygon form in the catalog"
        );
        assert!(
            catalog.iter().any(|r| matches!(
                r.shape.form,
                Form::RegularPolygon { sides, .. } if sides != 6
            )),
            "expected a RegularPolygon with a side count other than Carbon's hexagon"
        );
    }
 
    #[test]
    fn serialization_round_trip_preserves_shape() {
        for resource in default_catalog() {
            let json = serde_json::to_string(&resource).expect("serialize BaseResource");
            let restored: BaseResource =
                serde_json::from_str(&json).expect("deserialize BaseResource");
 
            assert_eq!(restored.name, resource.name);
 
            // Form parameters preserved (Circle/Rectangle/RegularPolygon
            // carry exact literal parameters with no trig involved, so
            // these compare bit-exact via PartialEq; Polygon vertices
            // are also plain literals here).
            match (&restored.shape.form, &resource.shape.form) {
                (Form::Circle { radius: a }, Form::Circle { radius: b }) => {
                    assert_eq!(a, b);
                }
                (
                    Form::Rectangle { width: aw, height: ah },
                    Form::Rectangle { width: bw, height: bh },
                ) => {
                    assert_eq!(aw, bw);
                    assert_eq!(ah, bh);
                }
                (
                    Form::RegularPolygon { sides: a_sides, radius: a_radius },
                    Form::RegularPolygon { sides: b_sides, radius: b_radius },
                ) => {
                    assert_eq!(a_sides, b_sides);
                    assert_eq!(a_radius, b_radius);
                }
                (Form::Polygon { vertices: a }, Form::Polygon { vertices: b }) => {
                    assert_eq!(a, b);
                }
                (a, b) => panic!("{} form variant changed across round-trip: {:?} vs {:?}", resource.name, a, b),
            }
 
            assert_eq!(
                restored.shape.connection_points.len(),
                resource.shape.connection_points.len()
            );
            for (a, b) in restored
                .shape
                .connection_points
                .iter()
                .zip(resource.shape.connection_points.iter())
            {
                // Epsilon comparison, not bit-exact equality: some
                // connection points are derived from sin()/cos() (see
                // Carbon/Water), and JSON's decimal text round-trip is
                // not guaranteed bit-exact for every irrational-derived
                // f64 - this is a property of text serialization, not
                // of the Shape representation itself.
                const EPS: f64 = 1e-9;
                assert!((a.x - b.x).abs() < EPS, "x mismatch: {} vs {}", a.x, b.x);
                assert!((a.y - b.y).abs() < EPS, "y mismatch: {} vs {}", a.y, b.y);
                assert!(
                    (a.direction_radians - b.direction_radians).abs() < EPS,
                    "direction mismatch: {} vs {}",
                    a.direction_radians,
                    b.direction_radians
                );
                assert!(
                    (a.strength - b.strength).abs() < EPS,
                    "strength mismatch: {} vs {}",
                    a.strength,
                    b.strength
                );
            }
        }
    }
}
 
