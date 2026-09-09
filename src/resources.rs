use serde::{Deserialize, Serialize};

use crate::math::{complexity, exponential_influence};
use crate::material_structure::MaterialStructure;

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
    pub shape: Shape,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Form {
    Circle { radius: f64 },
    Line { length: f64, radius: f64 },
    Rectangle { width: f64, height: f64 },
    RegularPolygon { sides: u8, radius: f64 },
    Polygon { vertices: Vec<(f64, f64)> },
    Fluid { nominal_area: f64 },
}

impl Form {
    pub fn is_valid(&self) -> bool {
        match self {
            Form::Circle { radius } => radius.is_finite() && *radius > 0.0,
            Form::Line { length, radius } => length.is_finite() && radius.is_finite() && *length > 0.0 && *radius > 0.0,
            Form::Rectangle { width, height } => width.is_finite() && height.is_finite() && *width > 0.0 && *height > 0.0,
            Form::RegularPolygon { sides, radius } => *sides >= 3 && radius.is_finite() && *radius > 0.0,
            Form::Polygon { vertices } => vertices.len() >= 3 && vertices.iter().all(|(x, y)| x.is_finite() && y.is_finite()),
            Form::Fluid { nominal_area } => nominal_area.is_finite() && *nominal_area > 0.0,
        }
    }

    pub fn polygon_vertices(&self) -> Option<Vec<(f64, f64)>> {
        match self {
            Form::Circle { .. } | Form::Line { .. } | Form::Fluid { .. } => None,
            Form::Rectangle { width, height } => {
                let hw = width / 2.0;
                let hh = height / 2.0;
                Some(vec![(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)])
            }
            Form::RegularPolygon { sides, radius } => {
                let n = *sides as usize;
                Some((0..n).map(|k| {
                    let angle = (k as f64) * std::f64::consts::TAU / (*sides as f64);
                    (radius * angle.cos(), radius * angle.sin())
                }).collect())
            }
            Form::Polygon { vertices } => Some(vertices.clone()),
        }
    }

    pub fn rigid_bounding_radius(&self) -> Option<f64> {
        match self {
            Form::Circle { radius } => Some(*radius),
            Form::Line { length, radius } => Some(((length / 2.0).powi(2) + radius.powi(2)).sqrt()),
            Form::Rectangle { width, height } => Some(((width / 2.0).powi(2) + (height / 2.0).powi(2)).sqrt()),
            Form::RegularPolygon { radius, .. } => Some(*radius),
            Form::Polygon { vertices } => Some(vertices.iter().map(|(x, y)| (x * x + y * y).sqrt()).fold(0.0_f64, f64::max)),
            Form::Fluid { .. } => None,
        }
    }

    pub fn bounding_radius(&self) -> f64 {
        self.rigid_bounding_radius().unwrap_or(0.0)
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
    pub fn is_valid(&self) -> bool { self.form.is_valid() }

    pub fn connection_sites(&self) -> ConnectionSites {
        match &self.form {
            Form::Circle { radius } => ConnectionSites::Circumference { radius: *radius },
            Form::Line { length, .. } => ConnectionSites::Corners(vec![
                ConnectionPoint { x: -length / 2.0, y: 0.0, direction_radians: std::f64::consts::PI },
                ConnectionPoint { x: length / 2.0, y: 0.0, direction_radians: 0.0 },
            ]),
            Form::Fluid { .. } => ConnectionSites::Undetermined,
            other => {
                let vertices = other.polygon_vertices().expect("rigid polygonal form has vertices");
                ConnectionSites::Corners(vertices.into_iter().map(|(x, y)| ConnectionPoint {
                    x, y, direction_radians: y.atan2(x),
                }).collect())
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
        if catalog.is_empty() { return Self { mass: 0.0, potential_energy: 0.0, reactivity: 0.0, cohesion: 0.0 }; }
        let count = catalog.len() as f64;
        Self {
            mass: catalog.iter().map(|r| r.properties.mass).sum::<f64>() / count,
            potential_energy: catalog.iter().map(|r| r.properties.potential_energy).sum::<f64>() / count,
            reactivity: catalog.iter().map(|r| r.properties.reactivity).sum::<f64>() / count,
            cohesion: catalog.iter().map(|r| r.properties.cohesion).sum::<f64>() / count,
        }
    }
}

/// One aggregate composition component. Quantity belongs here, not to a
/// physical constituent. Structured material uses one physical constituent
/// per `MaterialStructure` entry instead of fractional constituent amounts.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MaterialComponent {
    pub resource: String,
    pub amount: f64,
}

/// Material is explicitly split into composition and physical structure.
/// Unstructured ecological stock has `structure == None`; structured physical
/// material carries its constituent identities and attachment graph in
/// `MaterialStructure`. Constituent order is never physical identity.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Material {
    pub composition: Vec<MaterialComponent>,
    pub structure: Option<MaterialStructure>,
}

impl Material {
    pub fn free_base(name: impl Into<String>, amount: f64) -> Self {
        Self { composition: vec![MaterialComponent { resource: name.into(), amount }], structure: None }
    }

    pub fn is_structured(&self) -> bool { self.structure.is_some() }
    pub fn has_internal_structure(&self) -> bool { self.structure.as_ref().map_or(false, |s| !s.internal_bonds.is_empty()) }
    pub fn is_connected(&self) -> bool { self.structure.as_ref().map_or(self.composition.len() <= 1, MaterialStructure::is_connected) }

    pub fn is_valid(&self) -> bool {
        if self.composition.iter().any(|c| c.resource.is_empty() || !c.amount.is_finite() || c.amount <= 0.0) { return false; }
        match &self.structure {
            None => true,
            Some(structure) => structure.is_valid() && !structure.constituents.is_empty(),
        }
    }

    pub fn total_amount(&self) -> f64 {
        if let Some(structure) = &self.structure { structure.constituents.len() as f64 } else { self.composition.iter().map(|c| c.amount).sum() }
    }

    pub fn is_empty(&self) { }

    pub fn empty(&self) -> bool { self.total_amount() <= 1e-12 }

    pub fn can_break(&self) -> bool { self.structure.as_ref().map_or(false, |s| s.constituents.len() >= 2 && !s.internal_bonds.is_empty()) }

    pub fn potential_energy(&self, catalog: &[BaseResource]) -> f64 {
        if let Some(structure) = &self.structure {
            structure.constituents.iter().map(|c| fresh_energy(catalog, &c.resource, 1.0)).sum()
        } else {
            self.composition.iter().map(|c| fresh_energy(catalog, &c.resource, c.amount)).sum()
        }
    }

    pub fn mass(&self, catalog: &[BaseResource]) -> f64 {
        if let Some(structure) = &self.structure {
            structure.constituents.iter().map(|c| catalog.iter().find(|b| b.name == c.resource).map(|b| b.properties.mass).unwrap_or(0.0)).sum()
        } else {
            self.composition.iter().map(|c| catalog.iter().find(|b| b.name == c.resource).map(|b| b.properties.mass * c.amount).unwrap_or(0.0)).sum()
        }
    }

    pub fn weighted_properties(&self, catalog: &[BaseResource]) -> ResourceProperties {
        let mut mass = 0.0; let mut pe = 0.0; let mut reac = 0.0; let mut coh = 0.0; let mut w = 0.0;
        if let Some(structure) = &self.structure {
            for c in &structure.constituents {
                if let Some(base) = catalog.iter().find(|b| b.name == c.resource) {
                    w += 1.0; mass += base.properties.mass; pe += base.properties.potential_energy; reac += base.properties.reactivity; coh += base.properties.cohesion;
                }
            }
        } else {
            for c in &self.composition {
                if let Some(base) = catalog.iter().find(|b| b.name == c.resource) {
                    w += c.amount; mass += base.properties.mass * c.amount; pe += base.properties.potential_energy * c.amount; reac += base.properties.reactivity * c.amount; coh += base.properties.cohesion * c.amount;
                }
            }
        }
        if w <= 0.0 { return ResourceProperties { mass: 0.0, potential_energy: 0.0, reactivity: 0.0, cohesion: 0.0 }; }
        ResourceProperties { mass: mass / w, potential_energy: pe / w, reactivity: reac / w, cohesion: coh / w }
    }

    /// Structured material cannot be fractionally extracted. Separation must
    /// go through explicit physical decomposition so the attachment graph is
    /// preserved and resolved correctly.
    pub fn take(&mut self, amount: f64) -> Option<Material> {
        if self.is_structured() { return None; }
        let total = self.total_amount();
        if amount <= 0.0 || total <= 0.0 { return None; }
        let taken = amount.min(total);
        let frac = taken / total;
        let mut components = Vec::new();
        for component in &mut self.composition {
            let piece = component.amount * frac;
            component.amount -= piece;
            components.push(MaterialComponent { resource: component.resource.clone(), amount: piece });
        }
        self.composition.retain(|c| c.amount > 1e-12);
        Some(Material { composition: components, structure: None })
    }

    pub fn composition(&self) -> &[MaterialComponent] { &self.composition }
    pub fn structure(&self) -> Option<&MaterialStructure> { self.structure.as_ref() }
}

pub fn merge_parts(parts: &[MaterialComponent]) -> Vec<MaterialComponent> {
    let mut out = Vec::new();
    for component in parts {
        if let Some(existing) = out.iter_mut().find(|c: &&mut MaterialComponent| c.resource == component.resource) {
            existing.amount += component.amount;
        } else { out.push(component.clone()); }
    }
    out.retain(|c| c.amount > 1e-12);
    out
}

pub fn combine_materials(_inputs: &[Material]) -> Material {
    panic!("combine_materials is retired: COMBINE must resolve physical attachment geometry before constructing structured Material")
}

pub fn combine_work_cost(material: &Material, catalog: &[BaseResource], water_field: f64) -> f64 {
    let n = material.total_amount().max(2.0);
    let props = material.weighted_properties(catalog);
    let reac = exponential_influence(effective_reactivity(props.reactivity, water_field));
    let cohesion = props.cohesion.clamp(0.0, 1.0);
    let c = complexity(n);
    (c * (1.0 + cohesion) * (1.25 - reac)).max(0.2)
}

pub fn effective_reactivity(reactivity: f64, water_field: f64) -> f64 { reactivity / (1.0 + water_field.max(0.0)) }

pub fn property_ranges(catalog: &[BaseResource]) -> ResourceProperties {
    if catalog.is_empty() { return ResourceProperties { mass: 1.0, potential_energy: 1.0, reactivity: 1.0, cohesion: 1.0 }; }
    let mut min_mass=f64::INFINITY; let mut max_mass=f64::NEG_INFINITY; let mut min_energy=f64::INFINITY; let mut max_energy=f64::NEG_INFINITY; let mut min_reac=f64::INFINITY; let mut max_reac=f64::NEG_INFINITY; let mut min_coh=f64::INFINITY; let mut max_coh=f64::NEG_INFINITY;
    for r in catalog { min_mass=min_mass.min(r.properties.mass); max_mass=max_mass.max(r.properties.mass); min_energy=min_energy.min(r.properties.potential_energy); max_energy=max_energy.max(r.properties.potential_energy); let er=exponential_influence(r.properties.reactivity); min_reac=min_reac.min(er); max_reac=max_reac.max(er); min_coh=min_coh.min(r.properties.cohesion); max_coh=max_coh.max(r.properties.cohesion); }
    ResourceProperties { mass:(max_mass-min_mass).max(f64::EPSILON), potential_energy:(max_energy-min_energy).max(f64::EPSILON), reactivity:(max_reac-min_reac).max(f64::EPSILON), cohesion:(max_coh-min_coh).max(f64::EPSILON) }
}

pub const NOMINAL_UNIT_AREA: f64 = 0.5;

pub fn default_catalog() -> Vec<BaseResource> {
    vec![
        BaseResource { name:"Carbon".into(), properties:ResourceProperties{mass:1.00,potential_energy:1.0,reactivity:0.10,cohesion:0.95}, shape:Shape{form:Form::RegularPolygon{sides:6,radius:0.438_691}} },
        BaseResource { name:"Methane".into(), properties:ResourceProperties{mass:0.75,potential_energy:20.0,reactivity:4.0,cohesion:0.10}, shape:Shape{form:Form::RegularPolygon{sides:3,radius:0.620_403}} },
        BaseResource { name:"Hydrogen".into(), properties:ResourceProperties{mass:0.25,potential_energy:12.0,reactivity:3.50,cohesion:0.05}, shape:Shape{form:Form::Line{length:2.342_920,radius:0.10}} },
        BaseResource { name:"Sulfur".into(), properties:ResourceProperties{mass:1.50,potential_energy:8.0,reactivity:2.0,cohesion:0.45}, shape:Shape{form:Form::RegularPolygon{sides:5,radius:0.458_577}} },
        BaseResource { name:"Nitrogen".into(), properties:ResourceProperties{mass:1.25,potential_energy:0.75,reactivity:0.35,cohesion:0.70}, shape:Shape{form:Form::Rectangle{width:1.511_858,height:0.330_719}} },
        BaseResource { name:"Phosphorus".into(), properties:ResourceProperties{mass:1.75,potential_energy:1.50,reactivity:0.75,cohesion:0.60}, shape:Shape{form:Form::Polygon{vertices:vec![(-0.408_248,-0.408_248),(0.408_248,-0.408_248),(0.408_248,0.0),(0.0,0.0),(0.0,0.408_248),(-0.408_248,0.408_248)]}} },
        BaseResource { name:"Water".into(), properties:ResourceProperties{mass:1.00,potential_energy:0.0,reactivity:0.0,cohesion:0.50}, shape:Shape{form:Form::Fluid{nominal_area:NOMINAL_UNIT_AREA}} },
    ]
}

pub fn fresh_energy(catalog: &[BaseResource], name: &str, amount: f64) -> f64 { catalog.iter().find(|b| b.name == name).map(|b| b.properties.potential_energy * amount).unwrap_or(0.0) }

#[cfg(test)]
mod shape_tests {
    use super::*;
    #[test] fn catalog_still_constructs_with_seven_resources(){assert_eq!(default_catalog().len(),7);}
    #[test] fn every_catalog_resource_has_a_valid_shape(){for r in default_catalog(){assert!(r.shape.is_valid());}}
    #[test] fn form_parameters_are_valid(){for r in default_catalog(){assert!(r.shape.form.is_valid());}}
    #[test] fn fluid_has_no_rigid_bounding_radius(){let w=default_catalog().into_iter().find(|r|r.name=="Water").unwrap();assert_eq!(w.shape.form.rigid_bounding_radius(),None);}
    #[test] fn rigid_forms_have_rigid_bounding_radii(){for r in default_catalog(){if !matches!(r.shape.form,Form::Fluid{..}){assert!(r.shape.form.rigid_bounding_radius().unwrap().is_finite());}}}
}
