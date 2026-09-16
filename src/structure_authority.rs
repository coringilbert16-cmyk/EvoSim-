//! Audit helpers for the physical-structure authority boundary.
//!
//! This module intentionally reports architectural inconsistencies instead of
//! silently repairing them. The physical graph is the authoritative owner of
//! realized constituent identity and external bonds; `Material` remains the
//! authoritative description of an individual material object until that
//! object is realized into the graph.
use crate::resources::BaseResource;
use crate::structure::OrganismStructure;
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum AuthorityFinding {
    ZeroConstituentId { unit_index: usize },
    DuplicateConstituentId { unit_index: usize },
    InvalidMaterial { unit_index: usize },
    UnknownResource { unit_index: usize, resource: String },
    InvalidBond { bond_index: usize },
    DuplicateBond { bond_index: usize },
    StructuredMaterialInUnit { unit_index: usize },
}

/// Audits the graph without mutating it.
///
/// `StructuredMaterialInUnit` is deliberately reported rather than rejected:
/// the repository currently has callers that put an intact composite
/// `Material` into a `StructuralUnit`, but the material model does not yet
/// carry enough placement/connection information to make that representation
/// physically authoritative. That gap must be resolved explicitly before the
/// representation can be collapsed into the graph.
pub(crate) fn audit_structure(
    structure: &OrganismStructure,
    catalog: &[BaseResource],
) -> Vec<AuthorityFinding> {
    let mut findings = Vec::new();
    let mut ids = HashSet::new();

    for (unit_index, unit) in structure.units.iter().enumerate() {
        if unit.physical_id.0 == 0 {
            findings.push(AuthorityFinding::ZeroConstituentId { unit_index });
        } else if !ids.insert(unit.physical_id) {
            findings.push(AuthorityFinding::DuplicateConstituentId { unit_index });
        }
        if !unit.material.is_valid() {
            findings.push(AuthorityFinding::InvalidMaterial { unit_index });
        }
        for (resource, _) in &unit.material.parts {
            if !catalog.iter().any(|base| base.name == *resource) {
                findings.push(AuthorityFinding::UnknownResource {
                    unit_index,
                    resource: resource.clone(),
                });
            }
        }
        if unit.material.has_internal_structure() {
            findings.push(AuthorityFinding::StructuredMaterialInUnit { unit_index });
        }
    }

    for (bond_index, bond) in structure.bonds.iter().enumerate() {
        if !structure.is_valid_bond(bond, catalog) {
            findings.push(AuthorityFinding::InvalidBond { bond_index });
        }
        if structure.bonds[..bond_index]
            .iter()
            .any(|previous| previous.has_same_identity(bond))
        {
            findings.push(AuthorityFinding::DuplicateBond { bond_index });
        }
    }

    findings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::{default_catalog, InternalBond, Material};
    use crate::structure::{Bond, BondEndpoint, ConnectionEndpoint, Placement, StructuralUnit};

    fn placement(x: f64, y: f64) -> Placement {
        Placement {
            x,
            y,
            rotation_radians: 0.0,
        }
    }

    #[test]
    fn clean_atomic_graph_has_no_authority_findings() {
        let catalog = default_catalog();
        let mut structure = OrganismStructure::new();
        structure.add_unit(StructuralUnit::new("Carbon", placement(0.0, 0.0)));
        assert!(audit_structure(&structure, &catalog).is_empty());
    }

    #[test]
    fn structured_unit_is_reported_without_being_silently_rewritten() {
        let catalog = default_catalog();
        let mut structure = OrganismStructure::new();
        let material = Material {
            parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
            internal_bonds: vec![InternalBond { part_a: 0, part_b: 1 }],
        };
        structure.add_unit(StructuralUnit::from_material(material, placement(0.0, 0.0)).unwrap());
        assert!(audit_structure(&structure, &catalog)
            .contains(&AuthorityFinding::StructuredMaterialInUnit { unit_index: 0 }));
    }

    #[test]
    fn duplicate_bond_is_reported() {
        let catalog = default_catalog();
        let mut structure = OrganismStructure::new();
        let a = structure.add_unit(StructuralUnit::new("Carbon", placement(0.0, 0.0)));
        let b = structure.add_unit(StructuralUnit::new("Carbon", placement(0.876, 0.0)));
        let id_a = structure.physical_id(a).unwrap();
        let id_b = structure.physical_id(b).unwrap();
        let bond = Bond {
            endpoint_a: BondEndpoint::new(id_a, ConnectionEndpoint::Corner { point_index: 0 }),
            endpoint_b: BondEndpoint::new(id_b, ConnectionEndpoint::Corner { point_index: 3 }),
            strength: 0.95,
            bond_energy: 1.0,
        };
        structure.bonds.push(bond);
        structure.bonds.push(bond);
        assert!(audit_structure(&structure, &catalog)
            .contains(&AuthorityFinding::DuplicateBond { bond_index: 1 }));
    }
}
