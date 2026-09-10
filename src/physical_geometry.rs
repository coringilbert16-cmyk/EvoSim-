use crate::resources::Form;
use serde::{Deserialize, Serialize};

/// The geometry an individual physical constituent currently occupies.
///
/// Resource catalog geometry remains immutable. This value is per physical
/// constituent and is therefore the only place where a realized object's
/// geometry may diverge from its resource's default geometry.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PhysicalGeometry {
    pub form: Form,
}

impl PhysicalGeometry {
    /// Start a physical constituent at the resource's immutable default form.
    pub fn from_default(form: &Form) -> Self {
        Self { form: form.clone() }
    }

    /// Return the currently realized geometry.
    pub fn form(&self) -> &Form {
        &self.form
    }

    /// Replace the realized geometry after a physical interaction has
    /// produced a new valid configuration.
    pub fn replace(&mut self, form: Form) -> bool {
        if !form.is_valid() {
            return false;
        }
        self.form = form;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physical_geometry_starts_from_default_form() {
        let default_form = Form::Circle { radius: 1.0 };
        let geometry = PhysicalGeometry::from_default(&default_form);
        assert_eq!(geometry.form(), &default_form);
    }

    #[test]
    fn invalid_replacement_is_rejected_without_mutation() {
        let default_form = Form::Circle { radius: 1.0 };
        let mut geometry = PhysicalGeometry::from_default(&default_form);
        assert!(!geometry.replace(Form::Circle { radius: 0.0 }));
        assert_eq!(geometry.form(), &default_form);
    }
}
