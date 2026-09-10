use crate::resources::Shape;
use serde::{Deserialize, Serialize};

impl PartialEq for Shape {
    fn eq(&self, other: &Self) -> bool {
        self.form == other.form
    }
}

/// The geometry an individual physical constituent currently occupies.
///
/// Resource catalog geometry remains immutable. This value is per physical
/// constituent and is therefore the only place where a realized object's
/// geometry may diverge from its resource's default geometry.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PhysicalGeometry {
    pub shape: Shape,
}

impl PhysicalGeometry {
    /// Start a physical constituent at the resource's immutable default shape.
    pub fn from_default(shape: &Shape) -> Self {
        Self { shape: shape.clone() }
    }

    /// Return the currently realized shape.
    pub fn shape(&self) -> &Shape {
        &self.shape
    }

    /// Return the currently realized form.
    pub fn form(&self) -> &crate::resources::Form {
        &self.shape.form
    }

    /// Replace the realized geometry after a physical interaction has
    /// produced a new valid configuration.
    pub fn replace(&mut self, shape: Shape) -> bool {
        if !shape.is_valid() {
            return false;
        }
        self.shape = shape;
        true
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
    fn invalid_replacement_is_rejected_without_mutation() {
        let default_shape = Shape {
            form: crate::resources::Form::Circle { radius: 1.0 },
        };
        let mut geometry = PhysicalGeometry::from_default(&default_shape);
        assert!(!geometry.replace(Shape {
            form: crate::resources::Form::Circle { radius: 0.0 },
        }));
        assert_eq!(geometry.shape(), &default_shape);
    }
}
