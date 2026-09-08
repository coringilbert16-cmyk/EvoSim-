//! Authoritative visual identity for immutable resource types.
//!
//! These values are observational metadata only. They do not participate in
//! simulation mechanics, chemistry, energy accounting, or evolution.

use serde::{Deserialize, Serialize};

use crate::resources::BaseResource;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ResourceAppearance {
    /// CSS/canvas-compatible fill representation.
    pub(crate) fill: &'static str,
    /// CSS/canvas-compatible outline representation.
    pub(crate) outline: &'static str,
    /// Fill opacity, primarily used for transparent fluids such as water.
    pub(crate) fill_opacity: u8,
}

impl ResourceAppearance {
    pub(crate) const fn new(fill: &'static str, outline: &'static str, fill_opacity: u8) -> Self {
        Self {
            fill,
            outline,
            fill_opacity,
        }
    }
}

/// Resolve the immutable visual identity of a catalog resource.
///
/// Resource names are interpreted here, at the authoritative resource-visual
/// layer, rather than in individual renderers. Unknown resources receive a
/// neutral fallback so adding a resource cannot make the observation renderer
/// fail.
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
    use crate::resources::{BaseResource, Form, ResourceProperties, Shape};

    fn resource(name: &str) -> BaseResource {
        BaseResource {
            name: name.into(),
            properties: ResourceProperties {
                mass: 1.0,
                potential_energy: 1.0,
                reactivity: 1.0,
                cohesion: 1.0,
            },
            shape: Shape {
                form: Form::Circle { radius: 1.0 },
            },
        }
    }

    #[test]
    fn approved_resource_appearances_are_stable() {
        assert_eq!(appearance(&resource("Carbon")).fill, "#080808");
        assert_eq!(appearance(&resource("Sulfur")).fill, "#D6D44A");
        assert_eq!(appearance(&resource("Methane")), ResourceAppearance::new("#F5F5F5", "#C93636", 255));
        assert_eq!(appearance(&resource("Water")).fill_opacity, 72);
    }

    #[test]
    fn unknown_resources_have_a_neutral_fallback() {
        assert_eq!(appearance(&resource("Unknown")).fill, "#AAAAAA");
    }
}
