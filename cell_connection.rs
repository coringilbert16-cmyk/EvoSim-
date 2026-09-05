//! Physical connections between cells, including cells belonging to different organisms.
//!
//! This module defines connectivity only. Material transfer, cooperation, and
//! other consequences are separate behaviors that may use an established
//! physical connection.

use serde::{Deserialize, Serialize};

/// Identifies one physical connection site on one organism's structural cell.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
    pub endpoint_a: CellSiteRef,
    pub endpoint_b: CellSiteRef,
}

impl CellConnection {
    pub fn new(endpoint_a: CellSiteRef, endpoint_b: CellSiteRef) -> Option<Self> {
        if endpoint_a == endpoint_b {
            return None;
        }
        Some(Self {
            endpoint_a,
            endpoint_b,
        })
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

    fn site(organism_id: &str, unit_index: usize, point_index: usize) -> CellSiteRef {
        CellSiteRef {
            organism_id: organism_id.into(),
            unit_index,
            point_index,
        }
    }

    #[test]
    fn connection_identifies_cross_organism_endpoints() {
        let connection = CellConnection::new(site("parent", 0, 0), site("child", 0, 0))
            .expect("distinct endpoints should connect");
        assert!(connection.crosses_organism_boundary());
    }

    #[test]
    fn connection_is_undirected() {
        let a = site("parent", 0, 0);
        let b = site("child", 0, 1);
        let connection = CellConnection::new(a, b).unwrap();
        assert!(connection.connects(a, b));
        assert!(connection.connects(b, a));
    }

    #[test]
    fn self_connection_is_rejected() {
        let a = site("organism", 0, 0);
        assert!(CellConnection::new(a, a).is_none());
    }
}
