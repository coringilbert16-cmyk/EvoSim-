use serde::{Deserialize, Serialize};

use crate::structure::Bond;

/// Stable structural identity of a bond. Endpoint identity is the only data
/// used to locate a bond in an organism's graph.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct BondIdentity {
    pub(crate) unit_a: usize,
    pub(crate) point_a: usize,
    pub(crate) unit_b: usize,
    pub(crate) point_b: usize,
}

impl BondIdentity {
    pub(crate) fn from_bond(bond: &Bond) -> Self {
        Self {
            unit_a: bond.unit_a,
            point_a: bond.point_a,
            unit_b: bond.unit_b,
            point_b: bond.point_b,
        }
    }

    pub(crate) fn matches(&self, other: &BondIdentity) -> bool {
        (self.unit_a == other.unit_a
            && self.point_a == other.point_a
            && self.unit_b == other.unit_b
            && self.point_b == other.point_b)
            || (self.unit_a == other.unit_b
                && self.point_a == other.point_b
                && self.unit_b == other.unit_a
                && self.point_b == other.point_a)
    }
}

/// Formation-time interaction state captured by a bond.
///
/// The surplus is the output of COMBINE's formation mathematics. Strength and
/// stored bond energy remain derived from that captured interaction outcome;
/// they are not separate chemical state in this abstraction.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub(crate) struct BondInteraction {
    pub(crate) formation_surplus: f64,
}

impl BondInteraction {
    pub(crate) fn from_bond(bond: &Bond) -> Option<Self> {
        if !bond.bond_energy.is_finite() || bond.bond_energy < 0.0 {
            return None;
        }
        Some(Self {
            formation_surplus: bond.bond_energy,
        })
    }

    pub(crate) fn bond_energy(self) -> f64 {
        self.formation_surplus.max(0.0)
    }

    pub(crate) fn strength(self) -> f64 {
        crate::combine::experimental_bond_strength(self.formation_surplus)
            .clamp(0.0, 1.0)
    }

    pub(crate) fn break_work(self, complexity: f64) -> f64 {
        self.strength() * complexity.max(0.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BondInteractionSnapshot {
    pub(crate) identity: BondIdentity,
    pub(crate) interaction: BondInteraction,
}

impl BondInteractionSnapshot {
    pub(crate) fn from_bond(bond: &Bond) -> Option<Self> {
        Some(Self {
            identity: BondIdentity::from_bond(bond),
            interaction: BondInteraction::from_bond(bond)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bond() -> Bond {
        Bond {
            unit_a: 2,
            point_a: 1,
            unit_b: 5,
            point_b: 3,
            strength: 0.2,
            bond_energy: 4.0,
        }
    }

    #[test]
    fn identity_ignores_interaction_values() {
        let a = BondIdentity::from_bond(&bond());
        let mut changed = bond();
        changed.strength = 0.9;
        changed.bond_energy = 99.0;
        assert_eq!(a, BondIdentity::from_bond(&changed));
    }

    #[test]
    fn interaction_derives_strength_from_formation_surplus() {
        let b = bond();
        let interaction = BondInteraction::from_bond(&b).unwrap();
        assert_eq!(interaction.bond_energy(), 4.0);
        assert_eq!(interaction.strength(), crate::combine::experimental_bond_strength(4.0));
    }

    #[test]
    fn snapshot_contains_independent_identity_and_interaction() {
        let snapshot = BondInteractionSnapshot::from_bond(&bond()).unwrap();
        assert_eq!(snapshot.identity.unit_a, 2);
        assert_eq!(snapshot.identity.point_b, 3);
        assert_eq!(snapshot.interaction.formation_surplus, 4.0);
    }
}
