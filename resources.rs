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
//   - Connection points mark physical locations/orientations on the
//     resource's own geometry where a *future* bonding mechanic may
//     attempt a connection. SUPERSEDED DESIGN NOTE: an earlier
//     version of this file authored connection points independently
//     per resource, explicitly NOT derived from side count. That is
//     now obsolete - the locked rule is "every physical corner is a
//     connection point," so polygonal points are now derived from
//     `Form::polygon_vertices()` (see `Shape::connection_sites()`)
//     and can never drift out of sync with the actual geometry.
//     Circle is the deliberate exception: it has a continuous
//     circumference, not a finite point list - see `ConnectionSites`.
//     Compatibility/contact rules still belong to that future
//     bonding mechanic, not to this representation.
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

/// A single connection point, always DERIVED from a resource's Form,
/// never independently authored. Position is relative to the shape's
/// own local origin; direction is the outward-facing orientation in
/// radians, computed as the angle from the shape's local origin
/// through the point itself (a deterministic, geometry-derived value,
/// not authored).
///
/// There is no independent connection-point strength. The only
/// strength value in the eventual bonding system belongs to a Bond
/// itself (formed between two points), not to a point in isolation -
/// so this type deliberately carries no such field.
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

/// Where a future bonding/contact system can find connection
/// locations on a resource's shape. Polygonal forms get exactly one
/// discrete, corner-derived point per vertex (rule: "every physical
/// corner is a connection point"). Circle is structurally different -
/// it has a continuous accessible circumference, not a finite point
/// list, so it is NOT represented as a Vec<ConnectionPoint> at all.
/// This session establishes only the representation; actually
/// resolving a contact location on a Circumference at bonding time is
/// explicitly out of scope here.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ConnectionSites {
    Corners(Vec<ConnectionPoint>),
    Circumference { radius: f64 },
}

/// The complete immutable geometric property of a resource type.
/// Connection points are no longer stored here as authored data -
/// they are derived on demand from `form` via `connection_sites()`,
/// so there is no way for a resource's connection points to drift
/// out of sync with its actual geometry.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Shape {
    pub form: Form,
}

impl Shape {
    pub fn is_valid(&self) -> bool {
        self.form.is_valid()
    }

    /// Derives this shape's connection sites purely from its Form.
    /// Circle -> its continuous circumference (no discrete points).
    /// Every other Form -> one ConnectionPoint per polygon vertex,
    /// positioned exactly at that vertex, with an outward direction
    /// computed as the angle from the local origin through the
    /// vertex (the same convention the old authored data already
    /// used for regular polygons - this just makes it automatic and
    /// universal instead of hand-authored per resource).
    pub fn connection_sites(&self) -> ConnectionSites {
        match &self.form {
            Form::Circle { radius } => ConnectionSites::Circumference { radius: *radius },
            other => {
                let vertices = other
                    .polygon_vertices()
                    .expect("non-Circle forms always resolve to a vertex list");

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
        // Carbon: hexagon. Six sides -> six connection points, one per
        // vertex, derived automatically via Shape::connection_sites()
        // (superseded note: an earlier version hand-authored only
        // three points here specifically to prove points weren't
        // derived from side count - that design is now obsolete; the
        // locked rule is exactly the opposite).
        BaseResource {
            name: "Carbon".into(),
            properties: ResourceProperties {
                mass: 1.0,
                potential_energy: 1.0,
                reactivity: 0.1,
                cohesion: 0.95,
            },
            shape: Shape {
                form: Form::RegularPolygon { sides: 6, radius: 0.6 },
            },
        },
        // Methane: triangle. Locked geometry assignment (was
        // previously a Circle, before Hydrogen was designated the
        // catalog's one and only circular resource). Three sides ->
        // three corner-derived connection points.
        BaseResource {
            name: "Methane".into(),
            properties: ResourceProperties {
                mass: 1.0,
                potential_energy: 20.0,
                reactivity: 4.0,
                cohesion: 0.1,
            },
            shape: Shape {
                form: Form::RegularPolygon { sides: 3, radius: 0.5 },
            },
        },
        // Hydrogen: the catalog's one and only circular resource
        // (locked assignment). Continuous circumference, no discrete
        // connection point list.
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
            },
        },
        // Sulfur: pentagon (locked assignment, was previously a
        // triangle before Methane took that shape). Five sides ->
        // five corner-derived connection points.
        BaseResource {
            name: "Sulfur".into(),
            properties: ResourceProperties {
                mass: 1.0,
                potential_energy: 8.0,
                reactivity: 2.0,
                cohesion: 0.4,
            },
            shape: Shape {
                form: Form::RegularPolygon { sides: 5, radius: 0.5 },
            },
        },
        // Nitrogen: an elongated rectangle (a bar/rod silhouette,
        // independent width/height rather than a regular polygon).
        // Four corners -> four corner-derived connection points (the
        // old design authored only two, at edge midpoints rather than
        // actual corners - now obsolete).
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
            },
        },
        // Phosphorus: explicit-vertex L-shape (six vertices) - the
        // concrete demonstration of the Polygon escape hatch, a
        // concave silhouette a parameterized primitive can't express.
        // Six vertices -> six corner-derived connection points,
        // including one at the concave inner corner.
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
            },
        },
        // Water: the locked long-term design is that water fills
        // accessible gaps between rigid materials (fluid/gap-filling
        // behavior) - that requires dynamic physical geometry, not a
        // fixed rigid polygon, and is explicitly NOT implemented here
        // (see report). Sulfur now occupies the pentagon, so Water
        // needs a placeholder Form purely so it remains a valid,
        // unique catalog entry in the meantime. Using a square
        // (RegularPolygon, 4 sides - distinct from Nitrogen's
        // Rectangle variant and every other resource's Form) as that
        // placeholder. This is NOT a claim about Water's eventual
        // shape - it is a temporary rigid stand-in only, chosen to
        // avoid inventing any part of the future fluid system.
        BaseResource {
            name: "Water".into(),
            properties: ResourceProperties {
                mass: 1.0,
                potential_energy: 0.0,
                reactivity: 0.0,
                cohesion: 0.5,
            },
            shape: Shape {
                form: Form::RegularPolygon { sides: 4, radius: 0.4 },
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
    fn locked_resource_geometry_assignments_are_correct() {
        // Pins down the exact locked assignments: Hydrogen=Circle,
        // Carbon=hexagon(6), Methane=triangle(3), Sulfur=pentagon(5),
        // Nitrogen=square/rectangle(4), Phosphorus=L-shaped hexagon(6).
        // Supersedes the old test, which proved the OPPOSITE for
        // Carbon (that it deliberately had fewer points than sides -
        // obsolete design).
        let catalog = default_catalog();
        let find = |name: &str| catalog.iter().find(|r| r.name == name).unwrap();

        assert!(
            matches!(find("Hydrogen").shape.form, Form::Circle { .. }),
            "Hydrogen must be the circular resource"
        );

        assert!(matches!(find("Carbon").shape.form, Form::RegularPolygon { sides: 6, .. }));
        assert!(matches!(find("Methane").shape.form, Form::RegularPolygon { sides: 3, .. }));
        assert!(matches!(find("Sulfur").shape.form, Form::RegularPolygon { sides: 5, .. }));
        assert!(matches!(find("Nitrogen").shape.form, Form::Rectangle { .. }));

        match &find("Phosphorus").shape.form {
            Form::Polygon { vertices } => {
                assert_eq!(vertices.len(), 6, "Phosphorus L-shape must have 6 vertices")
            }
            other => panic!("Phosphorus must use the explicit Polygon escape hatch, got {other:?}"),
        }

        let expected_corners = [
            ("Hydrogen", None),
            ("Carbon", Some(6)),
            ("Methane", Some(3)),
            ("Sulfur", Some(5)),
            ("Nitrogen", Some(4)),
            ("Phosphorus", Some(6)),
        ];

        for (name, expected) in expected_corners {
            match (find(name).shape.connection_sites(), expected) {
                (ConnectionSites::Circumference { .. }, None) => {}
                (ConnectionSites::Corners(points), Some(n)) => {
                    assert_eq!(points.len(), n, "{name} should have {n} connection points")
                }
                (sites, expected) => {
                    panic!("{name}: unexpected connection sites {sites:?} (expected corners={expected:?})")
                }
            }
        }
    }

    #[test]
    fn every_polygonal_resource_has_one_connection_point_per_corner() {
        for resource in default_catalog() {
            let expected_corners = match &resource.shape.form {
                Form::Circle { .. } => continue, // circle is covered separately below
                Form::Rectangle { .. } => 4,
                Form::RegularPolygon { sides, .. } => *sides as usize,
                Form::Polygon { vertices } => vertices.len(),
            };

            match resource.shape.connection_sites() {
                ConnectionSites::Corners(points) => assert_eq!(
                    points.len(),
                    expected_corners,
                    "{} should have exactly {} connection points (one per corner), got {}",
                    resource.name,
                    expected_corners,
                    points.len()
                ),
                ConnectionSites::Circumference { .. } => {
                    panic!("{} is polygonal but returned Circumference sites", resource.name)
                }
            }
        }
    }

    #[test]
    fn polygon_connection_points_correspond_to_actual_vertices() {
        for resource in default_catalog() {
            let Some(vertices) = resource.shape.form.polygon_vertices() else {
                continue; // circle has no vertex list
            };

            let ConnectionSites::Corners(points) = resource.shape.connection_sites() else {
                panic!("{} is polygonal but returned Circumference sites", resource.name);
            };

            assert_eq!(points.len(), vertices.len());

            for (point, vertex) in points.iter().zip(vertices.iter()) {
                assert_eq!(
                    (point.x, point.y),
                    *vertex,
                    "{} connection point does not match its polygon vertex exactly",
                    resource.name
                );
            }
        }
    }

    #[test]
    fn connection_points_are_valid_where_present() {
        for resource in default_catalog() {
            if let ConnectionSites::Corners(points) = resource.shape.connection_sites() {
                assert!(!points.is_empty(), "{} has a polygonal form but zero connection points", resource.name);
                for cp in &points {
                    assert!(cp.is_valid(), "{} has an invalid connection point: {:?}", resource.name, cp);
                }
            }
        }
    }

    #[test]
    fn circle_has_no_finite_connection_point_list() {
        // Confirms Circle forms resolve to Circumference, never Corners -
        // i.e. never an authored/arbitrary Vec<ConnectionPoint>.
        let circle_resources: Vec<_> = default_catalog()
            .into_iter()
            .filter(|r| matches!(r.shape.form, Form::Circle { .. }))
            .collect();

        assert!(!circle_resources.is_empty(), "expected at least one circular resource in the catalog");

        for resource in &circle_resources {
            match resource.shape.connection_sites() {
                ConnectionSites::Circumference { radius } => {
                    assert!(radius > 0.0, "{} circumference radius must be positive", resource.name);
                }
                ConnectionSites::Corners(_) => {
                    panic!("{} is a Circle but returned discrete Corners", resource.name)
                }
            }
        }
    }

    #[test]
    fn connection_point_has_no_independent_strength_field() {
        // Compile-time proof, not a runtime assertion: this exhaustive
        // struct pattern only compiles if ConnectionPoint has exactly
        // these three fields. If a `strength` (or equivalent) field is
        // ever reintroduced, this stops compiling as a deliberate
        // tripwire - the only strength value belongs to a future Bond.
        let ConnectionPoint { x: _, y: _, direction_radians: _ } =
            ConnectionPoint { x: 0.0, y: 0.0, direction_radians: 0.0 };
    }

    #[test]
    fn every_resource_has_a_unique_shape() {
        // Uses the actual Form definitions (via Form's own PartialEq)
        // rather than a hardcoded name list, so this fails automatically
        // if a future edit accidentally gives two resources the same
        // Form - including same-variant-different-parameters cases
        // like two RegularPolygons with the same side count.
        let catalog = default_catalog();

        let circle_count = catalog
            .iter()
            .filter(|r| matches!(r.shape.form, Form::Circle { .. }))
            .count();
        assert_eq!(circle_count, 1, "exactly one resource should be circular");

        for i in 0..catalog.len() {
            for j in (i + 1)..catalog.len() {
                assert_ne!(
                    catalog[i].shape.form,
                    catalog[j].shape.form,
                    "{} and {} have identical shapes: {:?}",
                    catalog[i].name,
                    catalog[j].name,
                    catalog[i].shape.form
                );
            }
        }
    }

    #[test]
    fn shape_vocabulary_is_actually_exercised() {
        let catalog = default_catalog();

        // At least one Polygon (explicit-vertex escape hatch) and a
        // spread of different RegularPolygon side counts are present.
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

            // Connection sites are derived from `form`, which was just
            // proven to round-trip correctly above - so sites computed
            // from the restored form must match sites computed from the
            // original form exactly (they're not independently stored,
            // so there's no separate serialization path to break).
            assert_eq!(restored.shape.connection_sites(), resource.shape.connection_sites());
        }
    }
}
