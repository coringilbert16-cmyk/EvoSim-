#![expect(dead_code, reason = "Staged API retained for subsystem integration")]
use serde::{Deserialize, Serialize};

/// Immutable physical state of a resource type. Geometry describes what the
/// material currently occupies; state describes whether that geometry may
/// deform without changing its composition.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PhysicalState {
    #[default]
    Rigid,
    Fluid,
}

/// Immutable type properties. These never change and never evolve.
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct ResourceProperties {
    pub mass: f64,
    pub potential_energy: f64,
    pub reactivity: f64,
    /// Ordered chemical-spectrum position on the approved chemical spectrum.
    /// This is explicit catalog data and must never be inferred from retired
    /// `reactivity` values.
    #[serde(default)]
    pub chemical_position: Option<f64>,
    pub cohesion: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BaseResource {
    pub name: String,
    pub properties: ResourceProperties,
    #[serde(default)]
    pub physical_state: PhysicalState,

    // Immutable default geometric representation of this resource type.
    // Individual physical constituents may realize different current geometry.
    pub shape: Shape,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Form {
    Circle {
        radius: f64,
    },
    Line {
        length: f64,
    },
    Rectangle {
        width: f64,
        height: f64,
    },
    RegularPolygon {
        sides: u8,
        radius: f64,
    },
    Polygon {
        vertices: Vec<(f64, f64)>,
    },
    /// Legacy serialized form retained only for migration compatibility.
    /// New resource definitions must express fluidity through `PhysicalState`.
    Fluid {
        nominal_area: f64,
        #[serde(default)]
        boundary: Option<Vec<(f64, f64)>>,
        /// Exact free interface. Mutually exclusive with the legacy polygon boundary.
        #[serde(default)]
        arc_boundary: Option<crate::capillary_geometry::CircularArcBoundary>,
        /// Exact piecewise boundary for simultaneous multiple solid contacts.
        #[serde(default)]
        multi_contact_boundary: Option<crate::capillary_geometry::MultiContactFluidBoundary>,
    },
}

fn polygon_geometry_is_valid(vertices: &[(f64, f64)]) -> bool {
    const EPS: f64 = 1e-12;

    if vertices.len() < 3 || !vertices.iter().all(|(x, y)| x.is_finite() && y.is_finite()) {
        return false;
    }

    let mut area2 = 0.0;
    for i in 0..vertices.len() {
        let (x1, y1) = vertices[i];
        let (x2, y2) = vertices[(i + 1) % vertices.len()];
        area2 += x1 * y2 - x2 * y1;
    }
    if !area2.is_finite() || area2.abs() <= EPS {
        return false;
    }

    fn orientation(a: (f64, f64), b: (f64, f64), c: (f64, f64)) -> f64 {
        (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
    }

    fn on_segment(a: (f64, f64), b: (f64, f64), p: (f64, f64)) -> bool {
        const EPS: f64 = 1e-12;
        orientation(a, b, p).abs() <= EPS
            && p.0 >= a.0.min(b.0) - EPS
            && p.0 <= a.0.max(b.0) + EPS
            && p.1 >= a.1.min(b.1) - EPS
            && p.1 <= a.1.max(b.1) + EPS
    }

    fn segments_intersect(a: (f64, f64), b: (f64, f64), c: (f64, f64), d: (f64, f64)) -> bool {
        const EPS: f64 = 1e-12;
        let o1 = orientation(a, b, c);
        let o2 = orientation(a, b, d);
        let o3 = orientation(c, d, a);
        let o4 = orientation(c, d, b);

        if ((o1 > EPS && o2 < -EPS) || (o1 < -EPS && o2 > EPS))
            && ((o3 > EPS && o4 < -EPS) || (o3 < -EPS && o4 > EPS))
        {
            return true;
        }

        (o1.abs() <= EPS && on_segment(a, b, c))
            || (o2.abs() <= EPS && on_segment(a, b, d))
            || (o3.abs() <= EPS && on_segment(c, d, a))
            || (o4.abs() <= EPS && on_segment(c, d, b))
    }

    let edge_count = vertices.len();
    for i in 0..edge_count {
        let a = vertices[i];
        let b = vertices[(i + 1) % edge_count];
        for j in (i + 1)..edge_count {
            if j == i + 1 || (i == 0 && j == edge_count - 1) {
                continue;
            }
            let c = vertices[j];
            let d = vertices[(j + 1) % edge_count];
            if segments_intersect(a, b, c, d) {
                return false;
            }
        }
    }

    true
}

impl Form {
    pub fn is_valid(&self) -> bool {
        match self {
            Form::Circle { radius } => radius.is_finite() && *radius > 0.0,
            Form::Line { length } => length.is_finite() && *length > 0.0,
            Form::Rectangle { width, height } => {
                width.is_finite() && height.is_finite() && *width > 0.0 && *height > 0.0
            }
            Form::RegularPolygon { sides, radius } => {
                *sides >= 3 && radius.is_finite() && *radius > 0.0
            }
            Form::Polygon { vertices } => polygon_geometry_is_valid(vertices),
            Form::Fluid {
                nominal_area,
                boundary,
                arc_boundary,
                multi_contact_boundary,
            } => {
                if !nominal_area.is_finite() || *nominal_area <= 0.0 {
                    return false;
                }
                if usize::from(boundary.is_some())
                    + usize::from(arc_boundary.is_some())
                    + usize::from(multi_contact_boundary.is_some())
                    > 1
                {
                    return false;
                }
                if let Some(multi) = multi_contact_boundary {
                    return multi.is_valid()
                        && (multi.nominal_area - nominal_area).abs()
                            <= 1e-10 * nominal_area.max(1.0);
                }
                if let Some(arc) = arc_boundary {
                    return arc.is_valid()
                        && (arc.segment_area().unwrap_or(f64::NAN) - nominal_area).abs()
                            <= 1e-10 * nominal_area.max(1.0);
                }
                boundary
                    .as_ref()
                    .map_or(true, |vertices| polygon_geometry_is_valid(vertices))
            }
        }
    }

    pub fn polygon_vertices(&self) -> Option<Vec<(f64, f64)>> {
        match self {
            Form::Circle { .. } | Form::Line { .. } => None,
            Form::Fluid {
                boundary,
                arc_boundary,
                multi_contact_boundary,
                ..
            } => {
                if arc_boundary.is_some() || multi_contact_boundary.is_some() {
                    None
                } else {
                    boundary.clone()
                }
            }
            Form::Rectangle { width, height } => {
                let hw = width / 2.0;
                let hh = height / 2.0;
                Some(vec![(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)])
            }
            Form::RegularPolygon { sides, radius } => {
                let n = sides.to_owned() as usize;
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
            Form::Line { length } => *length / 2.0,
            Form::Rectangle { width, height } => {
                ((width / 2.0).powi(2) + (height / 2.0).powi(2)).sqrt()
            }
            Form::RegularPolygon { radius, .. } => *radius,
            Form::Polygon { vertices } => vertices
                .iter()
                .map(|(x, y)| (x * x + y * y).sqrt())
                .fold(0.0_f64, f64::max),
            Form::Fluid {
                nominal_area,
                boundary,
                arc_boundary,
                multi_contact_boundary,
            } => {
                if let Some(multi) = multi_contact_boundary {
                    multi.bounding_radius()
                } else if let Some(arc) = arc_boundary {
                    arc.center_x.hypot(arc.center_y) + arc.radius
                } else {
                    boundary
                        .as_ref()
                        .map(|vertices| {
                            vertices
                                .iter()
                                .map(|(x, y)| x.hypot(*y))
                                .fold(0.0_f64, f64::max)
                        })
                        .filter(|radius| *radius > 0.0)
                        .unwrap_or_else(|| (nominal_area / std::f64::consts::PI).sqrt())
                }
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Shape {
    pub form: Form,
}

impl Shape {
    pub fn is_valid(&self) -> bool {
        self.form.is_valid()
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
        self.part_a < part_count && self.part_b < part_count && self.part_a != self.part_b
    }
}

const MATERIAL_COUNT_EPSILON: f64 = 1e-9;

fn is_whole_count(amount: f64) -> bool {
    amount.is_finite() && amount >= 0.0 && amount.fract() == 0.0
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
    pub fn free_base(name: impl Into<String>, amount: f64) -> Self {
        Self {
            parts: vec![(name.into(), amount)],
            internal_bonds: Vec::new(),
        }
    }

    pub fn is_valid(&self) -> bool {
        if self.parts.is_empty() {
            return self.internal_bonds.is_empty();
        }
        if !self.parts.iter().all(|(name, amount)| {
            !name.is_empty()
                && *amount >= 1.0
                && *amount < u64::MAX as f64
                && is_whole_count(*amount)
        }) {
            return false;
        }

        // Each part retains its own physical identity, so duplicate resource
        // names are valid in a composite. Their combined inventory still must
        // fit the same integer range used by the independent inventory oracle.
        // Compositions are small; use an allocation-free scan to keep validation
        // cheap on frequently checked physical materials.
        for (index, (name, _)) in self.parts.iter().enumerate() {
            if self.parts[..index]
                .iter()
                .any(|(previous_name, _)| previous_name == name)
            {
                continue;
            }
            let mut total = 0_u64;
            for (part_name, amount) in &self.parts {
                if part_name == name {
                    let Some(merged) = total.checked_add(*amount as u64) else {
                        return false;
                    };
                    total = merged;
                }
            }
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

    /// Validate constituent counts and resolve every resource name to exactly
    /// one entry in the authoritative catalog. Duplicate catalog names are an
    /// ambiguous identity, not a valid fallback to the first matching entry.
    pub fn is_valid_for_catalog(&self, catalog: &[BaseResource]) -> bool {
        self.is_valid()
            && catalog.iter().enumerate().all(|(index, resource)| {
                !resource.name.is_empty()
                    && !catalog[..index]
                        .iter()
                        .any(|previous| previous.name == resource.name)
            })
            && self.parts.iter().all(|(name, _)| {
                catalog
                    .iter()
                    .filter(|resource| resource.name == *name)
                    .count()
                    == 1
            })
    }

    pub fn has_internal_structure(&self) -> bool {
        !self.internal_bonds.is_empty()
    }

    pub fn potential_energy(&self, catalog: &[BaseResource]) -> f64 {
        if !self.is_valid_for_catalog(catalog) {
            return f64::NAN;
        }
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
        self.has_internal_structure() && self.total_amount() >= 2.0 - 1e-9
    }

    pub fn mass(&self, catalog: &[BaseResource]) -> f64 {
        if !self.is_valid_for_catalog(catalog) {
            return f64::NAN;
        }
        self.parts
            .iter()
            .map(|(name, amount)| {
                catalog
                    .iter()
                    .find(|resource| &resource.name == name)
                    .map(|resource| resource.properties.mass * amount)
                    .unwrap_or(f64::NAN)
            })
            .sum()
    }

    pub fn weighted_properties(&self, catalog: &[BaseResource]) -> ResourceProperties {
        if !self.is_valid_for_catalog(catalog) {
            return ResourceProperties {
                mass: f64::NAN,
                potential_energy: f64::NAN,
                reactivity: f64::NAN,
                chemical_position: None,
                cohesion: f64::NAN,
            };
        }
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
                chemical_position: None,
                cohesion: 0.0,
            };
        }
        ResourceProperties {
            mass: mass / w,
            potential_energy: pe / w,
            reactivity: reac / w,
            // Chemical position is intentionally not averaged across a
            // formation. Mixed materials must retain constituent identities.
            chemical_position: None,
            cohesion: coh / w,
        }
    }

    pub fn take(&mut self, amount: f64) -> Option<Material> {
        if self.has_internal_structure()
            || !self.is_valid()
            || !is_whole_count(amount)
            || amount < 1.0
        {
            return None;
        }
        let total = self.total_amount();
        if !is_whole_count(total) || total <= 0.0 {
            return None;
        }
        let taken = amount.min(total);
        let fraction = taken / total;
        let allocations: Vec<f64> = self
            .parts
            .iter()
            .map(|(_, quantity)| *quantity * fraction)
            .collect();
        if allocations.iter().any(|piece| !is_whole_count(*piece)) {
            return None;
        }
        let mut parts = Vec::new();
        for ((name, quantity), piece) in self.parts.iter_mut().zip(allocations) {
            if piece > MATERIAL_COUNT_EPSILON {
                parts.push((name.clone(), piece));
            }
            *quantity -= piece;
            if quantity.abs() <= MATERIAL_COUNT_EPSILON {
                *quantity = 0.0;
            }
        }
        self.parts
            .retain(|(_, quantity)| *quantity > MATERIAL_COUNT_EPSILON);
        Some(Material {
            parts,
            internal_bonds: Vec::new(),
        })
    }
}

#[cfg(test)]
mod material_contract_tests {
    use super::*;

    #[test]
    fn material_counts_are_whole_and_catalog_bound() {
        let catalog = default_catalog();
        assert!(Material::free_base("Carbon", 2.0).is_valid());
        assert!(!Material::free_base("Carbon", 1.5).is_valid());
        assert!(!Material::free_base("Carbon", 1.0000000001).is_valid());
        assert!(
            !Material::free_base("Carbon", 1e-10).is_valid(),
            "positive sub-unit quantities are fractional atoms, not whole-count values"
        );
        assert!(
            !Material::free_base("Carbon", u64::MAX as f64).is_valid(),
            "constituent counts must remain representable by the independent inventory oracle"
        );
        let largest_representable_count = u64::MAX as f64 - 2048.0;
        let overflowing_composite = Material {
            parts: vec![
                ("Carbon".into(), largest_representable_count),
                ("Carbon".into(), largest_representable_count),
            ],
            internal_bonds: vec![InternalBond {
                part_a: 0,
                part_b: 1,
            }],
        };
        assert!(
            !overflowing_composite.is_valid_for_catalog(&catalog),
            "repeated constituents must not overflow the per-resource inventory range"
        );
        let repeated_but_representable = Material {
            parts: vec![("Carbon".into(), 2.0), ("Carbon".into(), 3.0)],
            internal_bonds: vec![InternalBond {
                part_a: 0,
                part_b: 1,
            }],
        };
        assert!(
            repeated_but_representable.is_valid_for_catalog(&catalog),
            "separate physical constituents of the same resource remain valid when their sum fits"
        );
        assert!(Material::free_base("Carbon", 1.0).is_valid_for_catalog(&catalog));

        let mut ambiguous_catalog = catalog.clone();
        ambiguous_catalog.push(catalog[0].clone());
        assert!(
            !Material::free_base("Carbon", 1.0).is_valid_for_catalog(&ambiguous_catalog),
            "duplicate catalog names must not resolve to an arbitrary resource definition"
        );

        let unknown = Material::free_base("UnknownResource", 1.0);
        assert!(!unknown.is_valid_for_catalog(&catalog));
        assert!(unknown.mass(&catalog).is_nan());
        assert!(unknown.potential_energy(&catalog).is_nan());
        assert!(unknown.weighted_properties(&catalog).mass.is_nan());
        assert!(fresh_energy(&catalog, "UnknownResource", 1.0).is_nan());
    }

    #[test]
    fn material_take_never_splits_atoms_or_mutates_on_unrepresentable_mix() {
        let mut mixture = Material {
            parts: vec![("Carbon".into(), 1.0), ("Oxygen".into(), 1.0)],
            internal_bonds: Vec::new(),
        };
        let original = mixture.clone();
        assert!(mixture.take(1.0).is_none());
        assert_eq!(mixture, original);

        let mut carbon = Material::free_base("Carbon", 3.0);
        let taken = carbon
            .take(1.0)
            .expect("one whole carbon should be extractable");
        assert_eq!(taken.parts, vec![("Carbon".into(), 1.0)]);
        assert_eq!(carbon.parts, vec![("Carbon".into(), 2.0)]);
        let remaining = carbon.clone();
        assert!(carbon.take(0.5).is_none());
        assert!(carbon.take(1e-10).is_none());
        assert_eq!(
            carbon, remaining,
            "sub-unit requests must not mutate material"
        );
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

/// Convert cohesion into boundary permeability using Water and Carbon as the
/// physical endpoints. Water is fully permeable; Carbon is a hard wall.
pub fn permeability_from_cohesion(cohesion: f64, catalog: &[BaseResource]) -> f64 {
    let water = catalog
        .iter()
        .find(|resource| resource.name == "Water")
        .map(|resource| resource.properties.cohesion);
    let carbon = catalog
        .iter()
        .find(|resource| resource.name == "Carbon")
        .map(|resource| resource.properties.cohesion);
    let (Some(water), Some(carbon)) = (water, carbon) else {
        return 0.0;
    };
    if !cohesion.is_finite() || !water.is_finite() || !carbon.is_finite() || carbon <= water {
        return 0.0;
    }
    ((carbon - cohesion) / (carbon - water)).clamp(0.0, 1.0)
}

pub fn property_ranges(catalog: &[BaseResource]) -> ResourceProperties {
    if catalog.is_empty() {
        return ResourceProperties {
            mass: 1.0,
            potential_energy: 1.0,
            reactivity: 1.0,
            chemical_position: None,
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
        // Keep the measured material property in its native scale. No
        // approved exponential influence function exists in this subsystem.
        let er = r.properties.reactivity;
        min_reac = min_reac.min(er);
        max_reac = max_reac.max(er);
        min_coh = min_coh.min(r.properties.cohesion);
        max_coh = max_coh.max(r.properties.cohesion);
    }
    ResourceProperties {
        mass: (max_mass - min_mass).max(f64::EPSILON),
        potential_energy: (max_energy - min_energy).max(f64::EPSILON),
        reactivity: (max_reac - min_reac).max(f64::EPSILON),
        chemical_position: None,
        cohesion: (max_coh - min_coh).max(f64::EPSILON),
    }
}

pub const NOMINAL_UNIT_AREA: f64 = 0.5;

pub fn default_catalog() -> Vec<BaseResource> {
    vec![
        BaseResource {
            name: "Carbon".into(),
            properties: ResourceProperties {
                mass: 1.00,
                potential_energy: 1.0,
                reactivity: 0.10,
                chemical_position: Some(10.5),
                cohesion: 0.95,
            },
            physical_state: PhysicalState::Rigid,
            shape: Shape {
                form: Form::RegularPolygon {
                    sides: 6,
                    radius: 1.0,
                },
            },
        },
        BaseResource {
            name: "Oxygen".into(),
            properties: ResourceProperties {
                mass: 0.75,
                potential_energy: 20.0,
                reactivity: 4.0,
                chemical_position: Some(1.5),
                cohesion: 0.10,
            },
            physical_state: PhysicalState::Rigid,
            shape: Shape {
                form: Form::RegularPolygon {
                    sides: 3,
                    radius: 0.577_350_269_189_625_8,
                },
            },
        },
        BaseResource {
            name: "Hydrogen".into(),
            properties: ResourceProperties {
                mass: 0.25,
                potential_energy: 12.0,
                reactivity: 3.50,
                chemical_position: Some(5.5),
                cohesion: 0.05,
            },
            physical_state: PhysicalState::Rigid,
            // Hydrogen is a finite-area rigid strip: 1.0 long and 0.1 thick.
            // It is intentionally not represented as a zero-thickness line.
            shape: Shape {
                form: Form::Rectangle {
                    width: 1.0,
                    height: 0.1,
                },
            },
        },
        BaseResource {
            name: "Sulfur".into(),
            properties: ResourceProperties {
                mass: 1.50,
                potential_energy: 8.0,
                reactivity: 2.0,
                chemical_position: Some(3.5),
                cohesion: 0.45,
            },
            physical_state: PhysicalState::Rigid,
            shape: Shape {
                form: Form::Polygon {
                    // Sulfur parallelogram: 1.0/0.5 alternating edges,
                    // 60°/120° interior angles, centered at the origin.
                    vertices: vec![
                        (-0.625, -0.216_506_350_946_109_65),
                        (0.375, -0.216_506_350_946_109_65),
                        (0.625, 0.216_506_350_946_109_65),
                        (-0.375, 0.216_506_350_946_109_65),
                    ],
                },
            },
        },
        BaseResource {
            name: "Nitrogen".into(),
            properties: ResourceProperties {
                mass: 1.25,
                potential_energy: 0.75,
                reactivity: 0.35,
                chemical_position: Some(8.5),
                cohesion: 0.70,
            },
            physical_state: PhysicalState::Rigid,
            shape: Shape {
                form: Form::Rectangle {
                    width: 1.0,
                    height: 1.0,
                },
            },
        },
        BaseResource {
            name: "Phosphorus".into(),
            properties: ResourceProperties {
                mass: 1.75,
                potential_energy: 1.50,
                reactivity: 0.75,
                chemical_position: Some(12.5),
                cohesion: 0.60,
            },
            physical_state: PhysicalState::Rigid,
            shape: Shape {
                form: Form::Polygon {
                    // Isosceles trapezoid: 1.5-unit bottom, 1.0-unit top,
                    // and two 0.5-unit sides. Centered at the origin.
                    // Height = sqrt(3)/4, from the 0.25-unit half-base offset.
                    vertices: vec![
                        (-0.75, -0.21650635094610965),
                        (0.75, -0.21650635094610965),
                        (0.5, 0.21650635094610965),
                        (-0.5, 0.21650635094610965),
                    ],
                },
            },
        },
        BaseResource {
            name: "Water".into(),
            properties: ResourceProperties {
                mass: 1.00,
                potential_energy: 0.0,
                reactivity: 0.0,
                chemical_position: Some(7.0),
                cohesion: 0.00,
            },
            physical_state: PhysicalState::Fluid,
            shape: Shape {
                form: Form::Circle {
                    radius: (NOMINAL_UNIT_AREA / std::f64::consts::PI).sqrt(),
                },
            },
        },
    ]
}

pub fn fresh_energy(catalog: &[BaseResource], name: &str, amount: f64) -> f64 {
    if !is_whole_count(amount) {
        return f64::NAN;
    }
    catalog
        .iter()
        .find(|resource| resource.name == name)
        .map(|resource| resource.properties.potential_energy * amount)
        .unwrap_or(f64::NAN)
}

#[cfg(test)]
mod shape_tests {
    use super::*;

    #[test]
    fn polygon_validation_rejects_degenerate_geometry() {
        assert!(!Form::Polygon {
            vertices: vec![(0.0, 0.0), (1.0, 0.0), (2.0, 0.0)],
        }
        .is_valid());
        assert!(!Form::Polygon {
            vertices: vec![(0.0, 0.0), (1.0, 0.0), (0.0, 0.0)],
        }
        .is_valid());
    }

    #[test]
    fn polygon_validation_rejects_self_intersection() {
        assert!(!Form::Polygon {
            vertices: vec![(0.0, 0.0), (1.0, 1.0), (0.0, 1.0), (1.0, 0.0)],
        }
        .is_valid());
    }

    #[test]
    fn polygon_validation_accepts_concave_simple_geometry() {
        assert!(Form::Polygon {
            vertices: vec![
                (-1.0, -1.0),
                (1.0, -1.0),
                (1.0, 0.0),
                (0.0, 0.0),
                (0.0, 1.0),
                (-1.0, 1.0),
            ],
        }
        .is_valid());
    }

    #[test]
    fn catalog_still_constructs_with_seven_resources() {
        let catalog = default_catalog();
        assert_eq!(catalog.len(), 7);
        let expected_names = [
            "Carbon",
            "Oxygen",
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
                Form::Circle { .. } | Form::Line { .. } | Form::Fluid { .. } => {
                    assert!(resource.shape.form.polygon_vertices().is_none())
                }
                Form::Rectangle { .. } => {
                    assert_eq!(resource.shape.form.polygon_vertices().unwrap().len(), 4)
                }
                Form::RegularPolygon { sides, .. } => assert_eq!(
                    resource.shape.form.polygon_vertices().unwrap().len(),
                    sides.to_owned() as usize
                ),
                Form::Polygon { vertices } => assert_eq!(
                    resource.shape.form.polygon_vertices().unwrap().len(),
                    vertices.len()
                ),
            }
        }
    }

    #[test]
    fn locked_resource_geometry_assignments_are_correct() {
        let catalog = default_catalog();
        let find = |name: &str| catalog.iter().find(|r| r.name == name).unwrap();
        assert!(matches!(
            find("Hydrogen").shape.form,
            Form::Rectangle { width, height }
                if (width - 1.0).abs() < 1e-12 && (height - 0.1).abs() < 1e-12
        ));
        assert!(matches!(
            find("Carbon").shape.form,
            Form::RegularPolygon { sides: 6, .. }
        ));
        assert!(matches!(
            find("Oxygen").shape.form,
            Form::RegularPolygon { sides: 3, .. }
        ));
        assert!(matches!(
            &find("Sulfur").shape.form,
            Form::Polygon { vertices } if vertices.len() == 4
        ));
        assert!(matches!(
            find("Nitrogen").shape.form,
            Form::Rectangle { .. }
        ));
        assert!(
            matches!(&find("Phosphorus").shape.form, Form::Polygon { vertices } if vertices.len() == 4)
        );
        assert!(matches!(find("Water").shape.form, Form::Circle { .. }));
        assert_eq!(find("Water").physical_state, PhysicalState::Fluid);
        assert_eq!(find("Hydrogen").physical_state, PhysicalState::Rigid);
    }

    #[test]
    fn cohesion_permeability_uses_water_and_carbon_as_endpoints() {
        let catalog = default_catalog();
        assert!((permeability_from_cohesion(0.0, &catalog) - 1.0).abs() < 1e-12);
        assert!((permeability_from_cohesion(0.95, &catalog) - 0.0).abs() < 1e-12);
        assert!((permeability_from_cohesion(0.475, &catalog) - 0.5).abs() < 1e-12);
        assert_eq!(permeability_from_cohesion(f64::NAN, &catalog), 0.0);
    }

    #[test]
    fn resource_edge_lengths_follow_the_shared_connection_scale() {
        fn distance(a: (f64, f64), b: (f64, f64)) -> f64 {
            (a.0 - b.0).hypot(a.1 - b.1)
        }

        let catalog = default_catalog();
        let find = |name: &str| catalog.iter().find(|r| r.name == name).unwrap();

        for name in ["Carbon", "Oxygen"] {
            let vertices = find(name).shape.form.polygon_vertices().unwrap();
            for i in 0..vertices.len() {
                assert!(
                    (distance(vertices[i], vertices[(i + 1) % vertices.len()]) - 1.0).abs() < 1e-9
                );
            }
        }

        let sulfur = find("Sulfur").shape.form.polygon_vertices().unwrap();
        assert_eq!(sulfur.len(), 4);
        let sulfur_lengths = (0..sulfur.len())
            .map(|i| distance(sulfur[i], sulfur[(i + 1) % sulfur.len()]))
            .collect::<Vec<_>>();
        assert!((sulfur_lengths[0] - 1.0).abs() < 1e-9);
        assert!((sulfur_lengths[1] - 0.5).abs() < 1e-9);
        assert!((sulfur_lengths[2] - 1.0).abs() < 1e-9);
        assert!((sulfur_lengths[3] - 0.5).abs() < 1e-9);

        // The two long edges are parallel to each other, as are the two short
        // edges. Their shared acute/obtuse angles are 60°/120°.
        let cross = |a: (f64, f64), b: (f64, f64)| a.0 * b.1 - a.1 * b.0;
        let dot = |a: (f64, f64), b: (f64, f64)| a.0 * b.0 + a.1 * b.1;
        let edge = |i: usize| {
            (
                sulfur[(i + 1) % 4].0 - sulfur[i].0,
                sulfur[(i + 1) % 4].1 - sulfur[i].1,
            )
        };
        assert!(cross(edge(0), edge(2)).abs() < 1e-9);
        assert!(cross(edge(1), edge(3)).abs() < 1e-9);
        for (a, b, expected) in [(0, 1, 120.0_f64), (1, 2, 60.0_f64)] {
            let incoming = (-edge(a).0, -edge(a).1);
            let outgoing = edge(b);
            let interior = cross(incoming, outgoing)
                .atan2(dot(incoming, outgoing))
                .to_degrees()
                .abs();
            assert!((interior - expected).abs() < 1e-7);
        }

        // Centering is a geometry contract: the centroid and bounding-box
        // center are both at the origin.
        let centroid = (
            sulfur.iter().map(|p| p.0).sum::<f64>() / 4.0,
            sulfur.iter().map(|p| p.1).sum::<f64>() / 4.0,
        );
        assert!(centroid.0.abs() < 1e-12 && centroid.1.abs() < 1e-12);
        let min_x = sulfur.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
        let max_x = sulfur.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
        let min_y = sulfur.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
        let max_y = sulfur.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
        assert!((min_x + max_x).abs() < 1e-12);
        assert!((min_y + max_y).abs() < 1e-12);

        let doubled_area = (0..4)
            .map(|i| sulfur[i].0 * sulfur[(i + 1) % 4].1 - sulfur[(i + 1) % 4].0 * sulfur[i].1)
            .sum::<f64>()
            .abs();
        assert!((doubled_area / 2.0 - 3.0_f64.sqrt() / 4.0).abs() < 1e-9);
        let phosphorus = find("Phosphorus").shape.form.polygon_vertices().unwrap();
        assert_ne!(
            sulfur, phosphorus,
            "sulfur's parallelogram must remain distinct from phosphorus's trapezoid"
        );

        // Shape-only replacement: all non-geometric sulfur properties stay fixed.
        let sulfur_resource = find("Sulfur");
        assert!((sulfur_resource.properties.mass - 1.50).abs() < 1e-12);
        assert!((sulfur_resource.properties.potential_energy - 8.0).abs() < 1e-12);
        assert!((sulfur_resource.properties.reactivity - 2.0).abs() < 1e-12);
        assert_eq!(sulfur_resource.properties.chemical_position, Some(3.5));
        assert!((sulfur_resource.properties.cohesion - 0.45).abs() < 1e-12);

        let hydrogen = find("Hydrogen").shape.form.clone();
        assert!(matches!(
            hydrogen,
            Form::Rectangle { width, height }
                if (width - 1.0).abs() < 1e-12 && (height - 0.1).abs() < 1e-12
        ));

        let nitrogen = find("Nitrogen").shape.form.polygon_vertices().unwrap();
        for i in 0..nitrogen.len() {
            assert!((distance(nitrogen[i], nitrogen[(i + 1) % nitrogen.len()]) - 1.0).abs() < 1e-9);
        }

        let phosphorus = find("Phosphorus").shape.form.polygon_vertices().unwrap();
        assert_eq!(phosphorus.len(), 4);
        let lengths = (0..phosphorus.len())
            .map(|i| distance(phosphorus[i], phosphorus[(i + 1) % phosphorus.len()]))
            .collect::<Vec<_>>();
        assert!(
            (lengths[0] - 1.5).abs() < 1e-9,
            "bottom edge must be 1.5 units"
        );
        assert!(
            (lengths[1] - 0.5).abs() < 1e-9,
            "right sloping edge must be 0.5 units"
        );
        assert!((lengths[2] - 1.0).abs() < 1e-9, "top edge must be 1.0 unit");
        assert!(
            (lengths[3] - 0.5).abs() < 1e-9,
            "left sloping edge must be 0.5 units"
        );
        assert!((phosphorus[0].0 + phosphorus[1].0).abs() < 1e-9);
        assert!((phosphorus[2].0 + phosphorus[3].0).abs() < 1e-9);
        assert!((phosphorus[0].1 + phosphorus[2].1).abs() < 1e-9);
        assert!((phosphorus[1].1 + phosphorus[3].1).abs() < 1e-9);
    }

    #[test]
    fn water_preserves_the_nominal_unit_area() {
        let water = default_catalog()
            .into_iter()
            .find(|resource| resource.name == "Water")
            .unwrap();
        let Form::Circle { radius } = water.shape.form else {
            panic!("water must retain its fluid circle geometry");
        };
        assert!((std::f64::consts::PI * radius * radius - NOMINAL_UNIT_AREA).abs() < 1e-12);
    }

    #[test]
    fn every_resource_has_a_unique_shape() {
        let catalog = default_catalog();
        for i in 0..catalog.len() {
            for j in (i + 1)..catalog.len() {
                assert_ne!(catalog[i].shape.form, catalog[j].shape.form);
            }
        }
    }

    #[test]
    fn shape_vocabulary_is_actually_exercised() {
        let catalog = default_catalog();
        assert!(catalog
            .iter()
            .any(|r| matches!(r.shape.form, Form::Polygon { .. })));
        assert!(catalog
            .iter()
            .any(|r| matches!(r.shape.form, Form::RegularPolygon { sides, .. } if sides != 6)));
        // Hydrogen is intentionally a finite-area rectangular strip, not
        // a zero-width line; keep the test aligned with the resource contract.
        assert!(catalog
            .iter()
            .any(|r| matches!(r.shape.form, Form::Rectangle { .. })));
    }

    #[test]
    fn legacy_fluid_serialization_defaults_the_exact_arc_field() {
        let form: Form =
            serde_json::from_str(r#"{"Fluid":{"nominal_area":0.5,"boundary":null}}"#).unwrap();
        assert!(matches!(
            &form,
            Form::Fluid {
                nominal_area,
                boundary: None,
                arc_boundary: None, ..} if (*nominal_area - 0.5).abs() < f64::EPSILON
        ));
        assert!(form.is_valid());
    }

    #[test]
    fn exact_fluid_arc_round_trips_without_polygon_approximation() {
        let family = crate::capillary_geometry::CapillaryContactFamily::solve(
            0.5,
            std::f64::consts::FRAC_PI_2,
        )
        .unwrap();
        let arc =
            crate::capillary_geometry::CircularArcBoundary::from_flat_contact(family).unwrap();
        let form = Form::Fluid {
            nominal_area: 0.5,
            boundary: None,
            arc_boundary: Some(arc),
            multi_contact_boundary: None,
        };
        let json = serde_json::to_string(&form).unwrap();
        let restored: Form = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, form);
        assert!(restored.polygon_vertices().is_none());
        assert!(restored.is_valid());
    }

    #[test]
    fn multi_contact_fluid_form_round_trips_as_exact_geometry() {
        let angle = std::f64::consts::FRAC_PI_2;
        let boundary = crate::capillary_geometry::MultiContactFluidBoundary {
            nominal_area: 4.0,
            pieces: vec![
                crate::capillary_geometry::FluidBoundaryPiece::SolidContact {
                    start_x: -1.0,
                    start_y: -1.0,
                    end_x: 1.0,
                    end_y: -1.0,
                    contact_angle_radians: angle,
                },
                crate::capillary_geometry::FluidBoundaryPiece::SolidContact {
                    start_x: 1.0,
                    start_y: -1.0,
                    end_x: 1.0,
                    end_y: 1.0,
                    contact_angle_radians: angle,
                },
                crate::capillary_geometry::FluidBoundaryPiece::SolidContact {
                    start_x: 1.0,
                    start_y: 1.0,
                    end_x: -1.0,
                    end_y: 1.0,
                    contact_angle_radians: angle,
                },
                crate::capillary_geometry::FluidBoundaryPiece::SolidContact {
                    start_x: -1.0,
                    start_y: 1.0,
                    end_x: -1.0,
                    end_y: -1.0,
                    contact_angle_radians: angle,
                },
            ],
        };
        let form = Form::Fluid {
            nominal_area: 4.0,
            boundary: None,
            arc_boundary: None,
            multi_contact_boundary: Some(boundary),
        };
        assert!(form.is_valid());
        assert!(form.polygon_vertices().is_none());
        assert!(form.bounding_radius() >= 2.0_f64.sqrt());
        let restored: Form = serde_json::from_str(&serde_json::to_string(&form).unwrap()).unwrap();
        assert_eq!(restored, form);
        assert!(restored.is_valid());
    }

    #[test]
    fn serialization_round_trip_preserves_shape_and_state() {
        for resource in default_catalog() {
            let json = serde_json::to_string(&resource).unwrap();
            let restored: BaseResource = serde_json::from_str(&json).unwrap();
            assert_eq!(restored.name, resource.name);
            assert_eq!(restored.shape.form, resource.shape.form);
            assert_eq!(restored.physical_state, resource.physical_state);
        }
    }
}
