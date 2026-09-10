//! Authoritative visual identity for immutable resource types.
//!
//! These values are observational metadata only. They do not participate in
//! simulation mechanics, chemistry, energy accounting, or evolution.

use serde::{Deserialize, Serialize};

use crate::resources::BaseResource;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ResourceAppearance {
    pub(crate) fill: String,
    pub(crate) outline: String,
    pub(crate) fill_opacity: u8,
}

impl ResourceAppearance {
    pub(crate) fn new(fill: &str, outline: &str, fill_opacity: u8) -> Self {
        Self { fill: fill.into(), outline: outline.into(), fill_opacity }
    }
}

pub(crate) fn appearance(resource: &BaseResource) -> ResourceAppearance {
    match resource.name.as_str() {
        "Carbon" => ResourceAppearance::new("#080808", "#777777", 255),
        "Sulfur" => ResourceAppearance::new("#D6D44A", "#A8A62F", 255),
        "Methane" => ResourceAppearance::new("#F5F5F5", "#C93636", 255),
        "Hydrogen" => ResourceAppearance::new("#E8E8E8", "#BDBDBD", 255),
        "Nitrogen" => ResourceAppearance::new("#315E9E", "#203F70", 255),
        "Phosphorus" => ResourceAppearance::new("#D65A32", "#9C3E23", 255),
        "Water" => ResourceAppearance::new("#CFEFFF", "#9CCFE8", 72),
        _ => ResourceAppearance::new("#AAAAAA", "#666666", 255),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::{BaseResource, Form, PhysicalState, ResourceProperties, Shape};

    fn resource(name: &str) -> BaseResource {
        BaseResource {
            name: name.into(),
            properties: ResourceProperties { mass: 1.0, potential_energy: 1.0, reactivity: 1.0, cohesion: 1.0 },
            physical_state: PhysicalState::Rigid,
            shape: Shape { form: Form::Circle { radius: 1.0 } },
        }
    }

    #[test]
    fn approved_resource_appearances_are_stable() {
        assert_eq!(appearance(&resource("Carbon")).fill, "#080808");
        assert_eq!(appearance(&resource("Sulfur")).fill, "#D6D44A");
        assert_eq!(appearance(&resource("Methane")), ResourceAppearance::new("#F5F5F5", "#C93636", 255));
        assert_eq!(appearance(&resource("Hydrogen")).fill, "#E8E8E8");
        assert_eq!(appearance(&resource("Nitrogen")).fill, "#315E9E");
        assert_eq!(appearance(&resource("Phosphorus")).fill, "#D65A32");
        assert_eq!(appearance(&resource("Water")).fill, "#CFEFFF");
        assert_eq!(appearance(&resource("Water")).fill_opacity, 72);
    }

    #[test]
    fn unknown_resources_have_a_neutral_fallback() {
        assert_eq!(appearance(&resource("Unknown")).fill, "#AAAAAA");
    }
}
