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
/// The resource catalog is the authority for construction-time default shape;
/// this value is the realized physical instance of it.
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

    /// Rigid geometry cannot be replaced. Fluid geometry may be re-realized
    /// when the surrounding structure requires a context-fitting shape.
    ///
    /// The caller is responsible for establishing that this constituent is a
    /// fluid resource. The method itself only accepts a valid replacement.
    pub fn replace_fluid_realization(&mut self, shape: Shape) -> bool {
        if !shape.is_valid() {
            return false;
        }
        self.shape = shape;
        true
    }

    /// Rigid geometry cannot be replaced. This compatibility method accepts
    /// only an identical shape and never mutates the realized geometry.
    #[allow(dead_code)]
    pub fn replace(&mut self, shape: Shape) -> bool {
        self.shape == shape
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
    fn fluid_geometry_can_be_re_realized() {
        let default_shape = Shape {
            form: crate::resources::Form::Circle { radius: 1.0 },
        };
        let mut geometry = PhysicalGeometry::from_default(&default_shape);
        let fitted = Shape {
            form: crate::resources::Form::Polygon {
                vertices: vec![(-1.0, 0.0), (0.0, 1.0), (1.0, 0.0), (0.0, -1.0)],
            },
        };
        assert!(geometry.replace_fluid_realization(fitted.clone()));
        assert_eq!(geometry.shape(), &fitted);
    }

    #[test]
    fn invalid_fluid_realization_is_rejected() {
        let default_shape = Shape {
            form: crate::resources::Form::Circle { radius: 1.0 },
        };
        let mut geometry = PhysicalGeometry::from_default(&default_shape);
        assert!(!geometry.replace_fluid_realization(Shape {
            form: crate::resources::Form::Polygon { vertices: vec![(0.0, 0.0), (1.0, 0.0)] },
        }));
        assert_eq!(geometry.shape(), &default_shape);
    }

    #[test]
    fn different_shape_cannot_replace_rigid_geometry() {
        let default_shape = Shape {
            form: crate::resources::Form::Circle { radius: 1.0 },
        };
        let mut geometry = PhysicalGeometry::from_default(&default_shape);
        assert!(!geometry.replace(Shape {
            form: crate::resources::Form::Circle { radius: 2.0 },
        }));
        assert_eq!(geometry.shape(), &default_shape);
    }

    #[test]
    fn identical_shape_is_a_no_op() {
        let default_shape = Shape {
            form: crate::resources::Form::Circle { radius: 1.0 },
        };
        let mut geometry = PhysicalGeometry::from_default(&default_shape);
        assert!(geometry.replace(default_shape.clone()));
        assert_eq!(geometry.shape(), &default_shape);
    }
}
