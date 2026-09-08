//! Browser-facing observation contracts derived from simulation truth.
//!
//! Observation types deliberately contain no simulation-owned state. They
//! describe what may be observed at a requested resolution and provide the
//! foundation for the World -> Organism -> Structure observation hierarchy.

use serde::{Deserialize, Serialize};

/// Semantic resolution of an observation.
///
/// These are observation levels, not simulation or camera zoom levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum ObservationLevel {
    World,
    Organism,
    Structure,
}

/// Context describing what the observer is asking to see.
///
/// Selection and focus remain browser-owned state. This type is deliberately
/// request-oriented so the server can later use it to produce only the detail
/// required by the current observation without making selection simulation
/// state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ObservationContext {
    pub(crate) level: ObservationLevel,
    #[serde(default)]
    pub(crate) organism_ids: Vec<String>,
    #[serde(default)]
    pub(crate) focused_organism_id: Option<String>,
}

impl ObservationContext {
    pub(crate) fn world() -> Self {
        Self {
            level: ObservationLevel::World,
            organism_ids: Vec::new(),
            focused_organism_id: None,
        }
    }

    pub(crate) fn organism(organism_ids: Vec<String>) -> Self {
        Self {
            level: ObservationLevel::Organism,
            organism_ids,
            focused_organism_id: None,
        }
    }

    pub(crate) fn structure(
        organism_ids: Vec<String>,
        focused_organism_id: Option<String>,
    ) -> Self {
        Self {
            level: ObservationLevel::Structure,
            organism_ids,
            focused_organism_id,
        }
    }
}

/// Common envelope for an observation produced from simulation truth.
///
/// The payload is intentionally separate from the context. Context describes
/// the requested scope; the payload is the derived result. Neither is an
/// alternate representation of an organism or environment owned by the
/// simulation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ObservationProjection {
    pub(crate) context: ObservationContext,
    pub(crate) payload: ObservationPayload,
}

impl ObservationProjection {
    pub(crate) fn world(payload: WorldObservation) -> Self {
        Self {
            context: ObservationContext::world(),
            payload: ObservationPayload::World(payload),
        }
    }

    pub(crate) fn organism(
        context: ObservationContext,
        payload: OrganismObservation,
    ) -> Option<Self> {
        if context.level != ObservationLevel::Organism {
            return None;
        }
        Some(Self {
            context,
            payload: ObservationPayload::Organism(payload),
        })
    }

    pub(crate) fn structure(
        context: ObservationContext,
        payload: StructureObservation,
    ) -> Option<Self> {
        if context.level != ObservationLevel::Structure {
            return None;
        }
        Some(Self {
            context,
            payload: ObservationPayload::Structure(payload),
        })
    }
}

/// Concrete observation payloads are introduced here as separate types so
/// each semantic level can evolve without coupling the browser to simulation
/// internals. Their authoritative data will be populated in later phases.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub(crate) struct WorldObservation {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub(crate) struct OrganismObservation {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub(crate) struct StructureObservation {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum ObservationPayload {
    World(WorldObservation),
    Organism(OrganismObservation),
    Structure(StructureObservation),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observation_levels_are_semantic_not_zoom_values() {
        assert_ne!(ObservationLevel::World, ObservationLevel::Organism);
        assert_ne!(ObservationLevel::Organism, ObservationLevel::Structure);
    }

    #[test]
    fn world_context_contains_no_selection() {
        let context = ObservationContext::world();
        assert_eq!(context.level, ObservationLevel::World);
        assert!(context.organism_ids.is_empty());
        assert!(context.focused_organism_id.is_none());
    }

    #[test]
    fn structure_context_preserves_stable_ids_and_focus() {
        let context = ObservationContext::structure(
            vec!["17".into(), "23".into()],
            Some("23".into()),
        );
        assert_eq!(context.level, ObservationLevel::Structure);
        assert_eq!(context.organism_ids, vec!["17", "23"]);
        assert_eq!(context.focused_organism_id.as_deref(), Some("23"));
    }

    #[test]
    fn projection_rejects_payload_at_the_wrong_level() {
        let context = ObservationContext::world();
        assert!(ObservationProjection::organism(
            context,
            OrganismObservation::default()
        )
        .is_none());
    }

    #[test]
    fn observation_projection_round_trips_through_json() {
        let projection = ObservationProjection::structure(
            ObservationContext::structure(vec!["17".into()], Some("17".into())),
            StructureObservation::default(),
        )
        .unwrap();

        let encoded = serde_json::to_string(&projection).unwrap();
        let decoded: ObservationProjection = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, projection);
    }
}
