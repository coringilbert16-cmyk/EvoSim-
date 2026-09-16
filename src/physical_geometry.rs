use crate::resources::Shape;
use crate::structure::StructuralUnit;
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

    /// Rigid geometry cannot be replaced. This compatibility method accepts
    /// only an identical shape and never mutates the realized geometry.
    pub fn replace(&mut self, shape: Shape) -> bool {
        self.shape == shape
    }
}

impl StructuralUnit {
    /// Return only geometry that has already been realized on this physical
    /// constituent. Construction-time catalog geometry must never be used as
    /// a fallback for physical queries.
    pub fn realized_shape(&self) -> Option<&Shape> {
        self.geometry.as_ref().map(PhysicalGeometry::shape)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::structure::Placement;

    #[test]
    fn physical_geometry_starts_from_default_shape() {
        let default_shape = Shape {
            form: crate::resources::Form::Circle { radius: 1.0 },
        };
        let geometry = PhysicalGeometry::from_default(&default_shape);
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

    #[test]
    fn unrealized_constituent_has_no_physical_shape() {
        let unit = StructuralUnit::new(
            "Carbon",
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        );
        assert!(unit.realized_shape().is_none());
    }

    #[test]
    fn realized_constituent_exposes_only_its_physical_shape() {
        let default_shape = Shape {
            form: crate::resources::Form::Circle { radius: 1.0 },
        };
        let mut unit = StructuralUnit::new(
            "Carbon",
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        );
        unit.geometry = Some(PhysicalGeometry::from_default(&default_shape));
        assert_eq!(unit.realized_shape(), Some(&default_shape));
    }
}
