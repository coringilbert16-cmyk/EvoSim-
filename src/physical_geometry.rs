use crate::resources::Shape;
use serde::{Deserialize, Serialize};

impl PartialEq for Shape {
    fn eq(&self, other: &Self) -> bool {
        self.form == other.form
    }
}

/// The immutable physical geometry of one realized constituent.
///
/// A rigid constituent may translate and rotate through `Placement`, but its
/// local shape cannot be replaced, deformed, or resized after realization.
/// Construction creates this value; all physical consumers read it through
/// `shape()`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PhysicalGeometry {
    shape: Shape,
}

impl PhysicalGeometry {
    /// Start a physical constituent from its resource's immutable default shape.
    pub fn from_default(shape: &Shape) -> Self {
        Self {
            shape: shape.clone(),
        }
    }

    /// Return the realized immutable shape.
    pub fn shape(&self) -> &Shape {
        &self.shape
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physical_geometry_starts_from_default_shape() {
        let default_shape = Shape {
            form: crate::resources::Form::Circle { radius: 1.0 },
        };
        let geometry = PhysicalGeometry::from_default(&default_shape);
        assert_eq!(geometry.shape(), &default_shape);
    }

    #[test]
    fn realized_shape_is_read_only_through_the_geometry_api() {
        let default_shape = Shape {
            form: crate::resources::Form::Circle { radius: 1.0 },
        };
        let geometry = PhysicalGeometry::from_default(&default_shape);
        assert_eq!(geometry.shape(), &default_shape);
    }
}
