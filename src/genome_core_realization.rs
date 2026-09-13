//! Genome-core realization boundary.
//!
//! This module is deliberately thin: physical construction remains owned by
//! `construction_realization`, while this layer records the correspondence
//! between inherited blueprint elements and the actual structural units that
//! were produced. Blueprint indices are never used as physical indices.

use crate::construction_realization::realize_material_with_constraints;
use crate::resources::BaseResource;
use crate::structural_blueprint::StructuralBlueprint;
use crate::structure::OrganismStructure;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RealizedBlueprint {
    pub elements: Vec<RealizedBlueprintElement>,
}

impl RealizedBlueprint {
    pub fn units_for(&self, blueprint_element_index: usize) -> Option<&[usize]> {
        self.elements
            .iter()
            .find(|element| element.blueprint_element_index == blueprint_element_index)
            .map(|element| element.structure_unit_indices.as_slice())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RealizedBlueprintElement {
    pub blueprint_element_index: usize,
    pub structure_unit_indices: Vec<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GenomeCoreRealization {
    pub structure: OrganismStructure,
    pub mapping: RealizedBlueprint,
}

impl GenomeCoreRealization {
    /// Realize blueprint elements in inherited order while preserving physical
    /// constituent indices. The blueprint's `core_elements` field is not used
    /// to declare a physical core; the core is determined later by the cavity
    /// criterion in `genome_core_geometry`.
    pub fn realize(
        blueprint: &StructuralBlueprint,
        catalog: &[BaseResource],
    ) -> Result<Self, String> {
        let mut structure = OrganismStructure::new();
        let mut mapping = RealizedBlueprint::default();

        for (element_index, element) in blueprint.elements.iter().enumerate() {
            let ids = realize_material_with_constraints(&mut structure, element, catalog, &[])?;
            mapping.elements.push(RealizedBlueprintElement {
                blueprint_element_index: element_index,
                structure_unit_indices: ids,
            });
        }

        Ok(Self { structure, mapping })
    }

    pub fn units_for(&self, blueprint_element_index: usize) -> Option<&[usize]> {
        self.mapping.units_for(blueprint_element_index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::{default_catalog, Material};
    use crate::structural_blueprint::{BlueprintElement, BlueprintPlacement};

    #[test]
    fn realized_element_indices_are_physical_not_blueprint_indices() {
        let blueprint = StructuralBlueprint::new(
            vec![
                BlueprintElement {
                    material: Material::free_base("Carbon", 1.0),
                    placement: BlueprintPlacement { x: 0.0, y: 0.0, rotation_radians: 0.0 },
                },
                BlueprintElement {
                    material: Material::free_base("Carbon", 1.0),
                    placement: BlueprintPlacement { x: 2.0, y: 0.0, rotation_radians: 0.0 },
                },
            ],
            vec![],
        );

        let realized = GenomeCoreRealization::realize(&blueprint, &default_catalog()).unwrap();
        assert_eq!(realized.units_for(0), Some([0usize].as_slice()));
        assert_eq!(realized.units_for(1), Some([1usize].as_slice()));
    }
}
