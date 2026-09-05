//! Physical connections between cells, including cells belonging to different organisms.
//!
//! This module defines connectivity only. Material transfer, cooperation, and
//! other consequences are separate behaviors that may use an established
//! physical connection.

use crate::contact::connection_points_contact;
use crate::resources::BaseResource;
use crate::structure::{ConnectionSiteRef, OrganismStructure};
use serde::{Deserialize, Serialize};

/// Identifies one physical connection site on one organism's structural cell.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct CellSiteRef {
    pub organism_id: String,
    pub unit_index: usize,
    pub point_index: usize,
}

/// A physical connection between two cells.
///
/// The endpoints identify the actual physical cells and connection sites. No
/// biological relationship is stored here: parenthood, cooperation, and other
/// meanings are consequences interpreted by higher-level systems.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CellConnection {
    endpoint_a: CellSiteRef,
    endpoint_b: CellSiteRef,
}

impl CellConnection {
    /// Establishes a cross-organism connection only when both sites exist and
    /// their actual world-space connection points are in physical contact.
    ///
    /// This is the sole runtime constructor for a physical cell connection.
    pub fn try_establish(
        endpoint_a: CellSiteRef,
        endpoint_b: CellSiteRef,
        structure_a: &OrganismStructure,
        structure_b: &OrganismStructure,
        catalog: &[BaseResource],
        tolerance: f64,
        min_facing: f64,
    ) -> Result<Self, &'static str> {
        if endpoint_a == endpoint_b {
            return Err("connection endpoints must differ");
        }
        if endpoint_a.organism_id == endpoint_b.organism_id {
            return Err("cell connection must cross an organism boundary");
        }
        let point_a = structure_a
            .connection_site(
                ConnectionSiteRef {
                    unit_index: endpoint_a.unit_index,
                    point_index: endpoint_a.point_index,
                },
                catalog,
            )
            .ok_or("invalid first connection site")?;
        let point_b = structure_b
            .connection_site(
                ConnectionSiteRef {
                    unit_index: endpoint_b.unit_index,
                    point_index: endpoint_b.point_index,
                },
                catalog,
            )
            .ok_or("invalid second connection site")?;
        let unit_a = structure_a
            .units
            .get(endpoint_a.unit_index)
            .ok_or("invalid first unit")?;
        let unit_b = structure_b
            .units
            .get(endpoint_b.unit_index)
            .ok_or("invalid second unit")?;
        if !connection_points_contact(point_a, unit_a, point_b, unit_b, tolerance, min_facing) {
            return Err("connection sites are not in physical contact");
        }
        Ok(Self {
            endpoint_a,
            endpoint_b,
        })
    }

    pub fn endpoint_a(&self) -> CellSiteRef {
        self.endpoint_a.clone()
    }

    pub fn endpoint_b(&self) -> CellSiteRef {
        self.endpoint_b.clone()
    }

    pub fn connects(&self, a: CellSiteRef, b: CellSiteRef) -> bool {
        (self.endpoint_a == a && self.endpoint_b == b)
            || (self.endpoint_a == b && self.endpoint_b == a)
    }

    pub fn touches(&self, site: CellSiteRef) -> bool {
        self.endpoint_a == site || self.endpoint_b == site
    }

    pub fn crosses_organism_boundary(&self) -> bool {
        self.endpoint_a.organism_id != self.endpoint_b.organism_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::structure::{Placement, StructuralUnit};

    fn site(organism_id: &str, unit_index: usize, point_index: usize) -> CellSiteRef {
        CellSiteRef {
            organism_id: organism_id.into(),
            unit_index,
            point_index,
        }
    }

    fn carbon_structure(x: f64, y: f64) -> OrganismStructure {
        let mut structure = OrganismStructure::new();
        structure.add_unit(StructuralUnit::new(
            "Carbon",
            Placement {
                x,
                y,
                rotation_radians: 0.0,
            },
        ));
        structure
    }

    #[test]
    fn physical_connection_rejects_same_organism() {
        let catalog = crate::resources::default_catalog();
        let a = carbon_structure(0.0, 0.0);
        let b = carbon_structure(0.0, 0.0);
        let result = CellConnection::try_establish(
            site("same", 0, 0),
            site("same", 0, 3),
            &a,
            &b,
            &catalog,
            0.25,
            0.0,
        );
        assert_eq!(result, Err("cell connection must cross an organism boundary"));
    }

    #[test]
    fn physical_connection_requires_actual_contact() {
        let catalog = crate::resources::default_catalog();
        let a = carbon_structure(0.0, 0.0);
        let b = carbon_structure(10.0, 0.0);
        let result = CellConnection::try_establish(
            site("a", 0, 0),
            site("b", 0, 0),
            &a,
            &b,
            &catalog,
            0.25,
            0.0,
        );
        assert_eq!(result, Err("connection sites are not in physical contact"));
    }

    #[test]
    fn physical_connection_accepts_contacting_cross_organism_sites() {
        let catalog = crate::resources::default_catalog();
        let a = carbon_structure(0.0, 0.0);
        let b = carbon_structure(1.0, 0.0);
        let connection = CellConnection::try_establish(
            site("a", 0, 0),
            site("b", 0, 3),
            &a,
            &b,
            &catalog,
            0.25,
            0.0,
        )
        .expect("contacting cross-organism sites should connect");
        assert!(connection.crosses_organism_boundary());
        assert!(connection.connects(site("a", 0, 0), site("b", 0, 3)));
        assert!(connection.connects(site("b", 0, 3), site("a", 0, 0)));
    }
}
