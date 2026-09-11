//! Minimal developmental seed selector.
//!
//! The seed is selected from a small family of physically realizable
//! constructions. The selector preserves the intended architecture (core,
//! soft interior, enclosing membrane) without preserving the historical
//! 183-atom reconstruction.

use crate::atomic_blueprint::{AtomicBlueprint, BlueprintAtom, BlueprintBond, BlueprintTransform};
use crate::resources::{ConnectionEndpoint, Material};
use crate::structural_blueprint::{BlueprintConnection, BlueprintElement, BlueprintPlacement, StructuralBlueprint};
use crate::resources::default_catalog;

#[derive(Clone, Debug)]
pub(crate) struct SeedSelection {
    pub structural: StructuralBlueprint,
    pub atomic: AtomicBlueprint,
}

fn point(radius: f64, angle: f64) -> (f64, f64) {
    (radius * angle.cos(), radius * angle.sin())
}

fn boundary_angle(from: (f64, f64), to: (f64, f64)) -> f64 {
    (to.1 - from.1).atan2(to.0 - from.0)
}

fn bond(a: usize, pa: (f64, f64), b: usize, pb: (f64, f64)) -> BlueprintBond {
    BlueprintBond {
        atom_a: a,
        endpoint_a: ConnectionEndpoint::Boundary { angle_radians: boundary_angle(pa, pb) },
        atom_b: b,
        endpoint_b: ConnectionEndpoint::Boundary { angle_radians: boundary_angle(pb, pa) },
        required_bonds: 1,
    }
}

fn candidate(n: usize, radius: f64) -> (StructuralBlueprint, AtomicBlueprint, f64) {
    let soft_radius = 2.0 * radius;
    let membrane_radius = 4.0 * radius;

    let mut positions = Vec::with_capacity(1 + 2 * n);
    positions.push((0.0, 0.0));
    for i in 0..n {
        positions.push(point(soft_radius, std::f64::consts::TAU * i as f64 / n as f64));
    }
    for i in 0..n {
        positions.push(point(membrane_radius, std::f64::consts::TAU * i as f64 / n as f64));
    }

    let mut atoms = Vec::with_capacity(positions.len());
    atoms.push(BlueprintAtom {
        resource: "Carbon".into(),
        transform: BlueprintTransform { x: 0.0, y: 0.0, rotation_radians: 0.0 },
    });
    for i in 0..n {
        let (x, y) = positions[1 + i];
        atoms.push(BlueprintAtom {
            resource: "Water".into(),
            transform: BlueprintTransform { x, y, rotation_radians: 0.0 },
        });
    }
    for i in 0..n {
        let (x, y) = positions[1 + n + i];
        atoms.push(BlueprintAtom {
            resource: "Sulfur".into(),
            transform: BlueprintTransform { x, y, rotation_radians: 0.0 },
        });
    }

    let mut bonds = Vec::with_capacity(3 * n);
    for i in 0..n {
        let soft = 1 + i;
        let membrane = 1 + n + i;
        let next_membrane = 1 + n + ((i + 1) % n);
        bonds.push(bond(0, positions[0], soft, positions[soft]));
        bonds.push(bond(soft, positions[soft], membrane, positions[membrane]));
        bonds.push(bond(membrane, positions[membrane], next_membrane, positions[next_membrane]));
    }

    let atomic = AtomicBlueprint {
        anchor: BlueprintTransform { x: 0.0, y: 0.0, rotation_radians: 0.0 },
        atoms,
        core_atoms: vec![0],
        bonds,
    };

    let mut elements = Vec::with_capacity(1 + 2 * n);
    elements.push(BlueprintElement {
        material: Material::free_base("Carbon", 1.0),
        placement: BlueprintPlacement { x: 0.0, y: 0.0, rotation_radians: 0.0 },
    });
    for i in 0..n {
        let (x, y) = positions[1 + i];
        elements.push(BlueprintElement {
            material: Material::free_base("Water", 1.0),
            placement: BlueprintPlacement { x, y, rotation_radians: 0.0 },
        });
    }
    for i in 0..n {
        let (x, y) = positions[1 + n + i];
        elements.push(BlueprintElement {
            material: Material::free_base("Sulfur", 1.0),
            placement: BlueprintPlacement { x, y, rotation_radians: 0.0 },
        });
    }

    let mut connections = Vec::with_capacity(4 * n);
    for i in 0..n {
        let soft = 1 + i;
        let next_soft = 1 + ((i + 1) % n);
        let membrane = 1 + n + i;
        let next_membrane = 1 + n + ((i + 1) % n);
        connections.push(BlueprintConnection { element_a: 0, element_b: soft });
        connections.push(BlueprintConnection { element_a: soft, element_b: membrane });
        connections.push(BlueprintConnection { element_a: membrane, element_b: next_membrane });
        connections.push(BlueprintConnection { element_a: soft, element_b: next_soft });
    }

    let structural = StructuralBlueprint::with_core_elements(elements, connections, vec![0]);
    let score = (n as f64) * 10.0 - membrane_radius;
    (structural, atomic, score)
}

pub(crate) fn select_seed() -> SeedSelection {
    let catalog = default_catalog();
    let radius = catalog
        .iter()
        .find(|r| r.name == "Carbon")
        .map(|r| r.shape.form.bounding_radius())
        .expect("default catalog must contain Carbon");

    let mut best: Option<(StructuralBlueprint, AtomicBlueprint, f64)> = None;
    for n in 5..=8 {
        let (structural, atomic, score) = candidate(n, radius);
        if structural.validate().is_err() || atomic.validate().is_err() {
            continue;
        }
        if atomic.realize(&catalog).is_err() || structural.realize(&catalog).is_err() {
            continue;
        }
        if best.as_ref().map_or(true, |(_, _, best_score)| score > *best_score) {
            best = Some((structural, atomic, score));
        }
    }

    let (structural, atomic, _) = best.expect(
        "seed selector could not find a physically realizable core/interior/membrane candidate",
    );
    SeedSelection { structural, atomic }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selector_produces_a_small_three_layer_seed() {
        let seed = select_seed();
        assert_eq!(seed.structural.elements.len(), 13);
        assert_eq!(seed.structural.core_elements, vec![0]);
        assert_eq!(seed.atomic.atoms.len(), 13);
        assert_eq!(seed.atomic.core_atoms, vec![0]);
        assert_eq!(seed.atomic.bonds.len(), 18);
        assert!(seed.structural.validate().is_ok());
        assert!(seed.atomic.validate().is_ok());
    }
}
