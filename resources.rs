use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BaseResource {
    pub name: String,
    pub properties: ResourceProperties,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct ResourceProperties {
    pub mass: f64,
    pub potential_energy: f64,
    pub reactivity: f64,
    pub cohesion: f64,
    pub form: Shape,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum Shape {
    Hexagon,
    Triangle,
    Circle,
    Pentagon,
    Rectangle,
    LShape,
    Fluid,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
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

    pub fn potential_energy(&self, catalog: &[BaseResource]) -> f64 {
        self.parts
            .iter()
            .map(|(name, amount)| fresh_energy(catalog, name, *amount))
            .sum()
    }

    pub fn total_amount(&self) -> f64 {
        self.parts.iter().map(|(_, amount)| *amount).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.parts.is_empty() || self.total_amount() <= f64::EPSILON
    }

    pub fn mass(&self, catalog: &[BaseResource]) -> f64 {
        self.parts
            .iter()
            .filter_map(|(name, amount)| find_resource(catalog, name).map(|r| r.properties.mass * *amount))
            .sum()
    }

    pub fn weighted_properties(&self, catalog: &[BaseResource]) -> ResourceProperties {
        let total = self.total_amount();
        if total <= f64::EPSILON {
            return ResourceProperties {
                mass: 0.0,
                potential_energy: 0.0,
                reactivity: 0.0,
                cohesion: 0.0,
                form: Shape::Fluid,
            };
        }
        let mut mass = 0.0;
        let mut potential_energy = 0.0;
        let mut reactivity = 0.0;
        let mut cohesion = 0.0;
        let mut form = Shape::Fluid;
        for (name, amount) in &self.parts {
            if let Some(resource) = find_resource(catalog, name) {
                let weight = *amount / total;
                mass += resource.properties.mass * *amount;
                potential_energy += resource.properties.potential_energy * weight;
                reactivity += resource.properties.reactivity * weight;
                cohesion += resource.properties.cohesion * weight;
                form = resource.properties.form;
            }
        }
        ResourceProperties {
            mass,
            potential_energy,
            reactivity,
            cohesion,
            form,
        }
    }
}

fn find_resource<'a>(catalog: &'a [BaseResource], name: &str) -> Option<&'a BaseResource> {
    catalog.iter().find(|r| r.name == name)
}

fn fresh_energy(catalog: &[BaseResource], name: &str, amount: f64) -> f64 {
    find_resource(catalog, name)
        .map(|r| r.properties.potential_energy * amount)
        .unwrap_or(0.0)
}

pub fn default_catalog() -> Vec<BaseResource> {
    vec![
        BaseResource { name: "Carbon".into(), properties: ResourceProperties { mass: 1.0, potential_energy: 1.0, reactivity: 0.1, cohesion: 0.95, form: Shape::Hexagon } },
        BaseResource { name: "Methane".into(), properties: ResourceProperties { mass: 1.0, potential_energy: 20.0, reactivity: 4.0, cohesion: 0.10, form: Shape::Triangle } },
        BaseResource { name: "Hydrogen".into(), properties: ResourceProperties { mass: 1.0, potential_energy: 12.0, reactivity: 3.0, cohesion: 0.05, form: Shape::Circle } },
        BaseResource { name: "Sulfur".into(), properties: ResourceProperties { mass: 1.0, potential_energy: 8.0, reactivity: 2.0, cohesion: 0.40, form: Shape::Pentagon } },
        BaseResource { name: "Nitrogen".into(), properties: ResourceProperties { mass: 1.0, potential_energy: 0.5, reactivity: 0.2, cohesion: 0.70, form: Shape::Rectangle } },
        BaseResource { name: "Phosphorus".into(), properties: ResourceProperties { mass: 1.0, potential_energy: 0.8, reactivity: 0.3, cohesion: 0.60, form: Shape::LShape } },
        BaseResource { name: "Water".into(), properties: ResourceProperties { mass: 1.0, potential_energy: 0.0, reactivity: 0.0, cohesion: 0.50, form: Shape::Fluid } },
    ]
}
