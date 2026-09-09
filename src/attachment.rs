//! Genetic/material attachment identity.
//!
//! These types describe *where a structural relationship is allowed to attach*
//! without storing world-space coordinates or bond-capacity slots. Physical
//! realization resolves these specifications into runtime connection endpoints.

use serde::{Deserialize, Serialize};

/// Stable identity for a structural element in a blueprint.
///
/// This is intentionally distinct from a vector index. Mutation may add or
/// remove elements without changing the identity of unrelated elements.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BlueprintElementId(pub u64);

/// Stable identity for a constituent within a material definition.
///
/// Constituent order is not physical identity. A material realization may use
/// this identifier to resolve internal structural relationships after other
/// constituents are inserted, removed, or reordered.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ConstituentId(pub u64);

/// A physical feature that can participate in a structural attachment.
///
/// Discrete features identify immutable geometric features such as a polygon
/// corner or a line terminal. Boundary and Fluid are continuous regions: they
/// contain no authored socket index, capacity count, or world-space position.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttachmentFeature {
    /// Immutable feature identity within the referenced physical constituent.
    Discrete(u32),
    /// A continuous rigid boundary. The physical resolver chooses the actual
    /// contact location from geometry and the opposing attachment.
    Boundary,
    /// A continuous fluid region. The physical resolver chooses the actual
    /// contact location from fluid occupancy and the opposing attachment.
    Fluid,
}

impl AttachmentFeature {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Discrete(_) | Self::Boundary | Self::Fluid => true,
        }
    }
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
    fn continuous_features_carry_no_socket_or_position() {
        assert_eq!(AttachmentFeature::Boundary, AttachmentFeature::Boundary);
        assert_eq!(AttachmentFeature::Fluid, AttachmentFeature::Fluid);
    }

    #[test]
    fn attachment_identity_round_trips() {
        let original = BlueprintAttachment {
            element: BlueprintElementId(7),
            feature: AttachmentFeature::Boundary,
        };
        let encoded = serde_json::to_string(&original).unwrap();
        let decoded: BlueprintAttachment = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, original);
    }
}
