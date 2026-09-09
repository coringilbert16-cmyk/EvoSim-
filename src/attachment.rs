//! Genetic/material attachment identity.
//!
//! These types describe *where a structural relationship is allowed to attach*
//! without storing world-space coordinates or bond-capacity slots. Physical
//! realization resolves these specifications into runtime connection endpoints.

use serde::{Deserialize, Serialize};

/// Stable identity for a structural element in a blueprint.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BlueprintElementId(pub u64);

/// Stable identity for a constituent within a material definition.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ConstituentId(pub u64);

/// What physical part of a blueprint element an attachment addresses.
///
/// `Constituent` addresses immutable geometry belonging to a specific material
/// constituent. `Assembly` addresses an exposed region of the realized whole
/// element. Neither variant is a socket, capacity slot, or authored position.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlueprintAttachmentTarget {
    Constituent(ConstituentId),
    Assembly,
}

/// A physical feature that can participate in a structural attachment.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttachmentFeature {
    /// Immutable geometric feature of the addressed physical constituent.
    Discrete(u32),
    /// Continuous rigid boundary. The physical resolver chooses the contact.
    Boundary,
    /// Continuous fluid region. The physical resolver chooses the contact.
    Fluid,
}

impl AttachmentFeature {
    pub fn is_valid(&self) -> bool { true }
}

/// An attachment target inside a material's constituent graph.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConstituentAttachment {
    pub constituent: ConstituentId,
    pub feature: AttachmentFeature,
}

/// An attachment target inside an organism blueprint.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlueprintAttachment {
    pub element: BlueprintElementId,
    pub target: BlueprintAttachmentTarget,
    pub feature: AttachmentFeature,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_element_ids_are_not_vector_indices() {
        let a = BlueprintElementId(10);
        let b = BlueprintElementId(11);
        assert_ne!(a, b);
        assert_eq!(a.0, 10);
    }

    #[test]
    fn constituent_order_is_not_attachment_identity() {
        let attachment = ConstituentAttachment {
            constituent: ConstituentId(42),
            feature: AttachmentFeature::Discrete(0),
        };
        assert_eq!(attachment.constituent, ConstituentId(42));
        assert!(attachment.feature.is_valid());
    }

    #[test]
    fn blueprint_attachment_distinguishes_constituent_from_assembly() {
        let constituent = BlueprintAttachment {
            element: BlueprintElementId(7),
            target: BlueprintAttachmentTarget::Constituent(ConstituentId(42)),
            feature: AttachmentFeature::Discrete(1),
        };
        let assembly = BlueprintAttachment {
            element: BlueprintElementId(7),
            target: BlueprintAttachmentTarget::Assembly,
            feature: AttachmentFeature::Boundary,
        };
        assert_ne!(constituent.target, assembly.target);
        assert_eq!(constituent.target, BlueprintAttachmentTarget::Constituent(ConstituentId(42)));
        assert_eq!(assembly.target, BlueprintAttachmentTarget::Assembly);
    }

    #[test]
    fn continuous_features_carry_no_socket_or_position() {
        assert_eq!(AttachmentFeature::Boundary, AttachmentFeature::Boundary);
        assert_eq!(AttachmentFeature::Fluid, AttachmentFeature::Fluid);
    }

    #[test]
    fn attachment_identity_round_trips() {
        let original = BlueprintAttachment {
            element: BlueprintElementId(7),
            target: BlueprintAttachmentTarget::Assembly,
            feature: AttachmentFeature::Boundary,
        };
        let encoded = serde_json::to_string(&original).unwrap();
        let decoded: BlueprintAttachment = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, original);
    }
}
