//! Session-scoped topology tracking and deterministic operation lineage.
//!
//! A [`TrackingSession`] allocates runtime topology identities and binds them
//! to stable semantic references. Advancing the generation starts a replay
//! with the same deterministic serial sequence while making prior
//! [`TrackingId`] values stale.
//!
//! ```
//! use monstertruck_core::{
//!     FeatureId, SemanticLabel, SemanticTopologyRef, TopologyKind, TrackingError,
//!     TrackingSession, TrackingSessionId,
//! };
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut session = TrackingSession::new(TrackingSessionId::new("edit-session")?);
//!     let reference = SemanticTopologyRef::new(
//!         FeatureId::new("pad")?,
//!         TopologyKind::Face,
//!         SemanticLabel::new("top")?,
//!     );
//!     let tracking_id = session.allocate()?;
//!     session.bind(reference.clone(), tracking_id.clone())?;
//!
//!     assert_eq!(session.resolve(&reference)?, &tracking_id);
//!     session.advance_generation()?;
//!     assert!(matches!(
//!         session.validate_current(&tracking_id),
//!         Err(TrackingError::StaleGeneration { .. }),
//!     ));
//!     Ok(())
//! }
//! ```

mod model;
mod session;

use std::result::Result;

#[cfg(test)]
use crate::DeterministicContentHash;
#[cfg(test)]
use std::collections::BTreeSet;

pub use model::{
    FeatureId, LineageEvent, LineageRelation, OperationKind, SemanticBinding, SemanticLabel,
    SemanticTopologyRef, TopologyKind, TrackingGeneration, TrackingId, TrackingSessionId,
};
pub use session::{TrackingError, TrackingSession};

/// Result type for topology tracking operations.
pub type TrackingResult<T> = Result<T, TrackingError>;

#[cfg(test)]
mod tests {
    use super::*;

    fn session_id(value: &str) -> TrackingSessionId {
        TrackingSessionId::new(value).expect("the test session identifier is valid")
    }

    fn semantic(label: &str) -> SemanticTopologyRef {
        SemanticTopologyRef::new(
            FeatureId::new("feature-1").expect("the test feature identifier is valid"),
            TopologyKind::Face,
            SemanticLabel::new(label).expect("the test semantic label is valid"),
        )
    }

    #[test]
    fn allocation_uses_one_serial_namespace_for_all_topology() {
        let mut session = TrackingSession::new(session_id("browser-session"));
        let vertex = session.allocate().expect("the serial range is available");
        let edge = session.allocate().expect("the serial range is available");
        let face = session.allocate().expect("the serial range is available");

        assert_eq!([vertex.serial(), edge.serial(), face.serial()], [1, 2, 3]);
        assert_eq!(BTreeSet::from([vertex, edge, face]).len(), 3);
    }

    #[test]
    fn session_qualification_makes_equal_serials_globally_unique() {
        let mut first = TrackingSession::new(session_id("first"));
        let mut second = TrackingSession::new(session_id("second"));
        let first_id = first.allocate().expect("the serial range is available");
        let second_id = second.allocate().expect("the serial range is available");

        assert_eq!(first_id.serial(), second_id.serial());
        assert_ne!(first_id, second_id);
    }

    #[test]
    fn replay_resets_serials_but_invalidates_old_ids() {
        let mut session = TrackingSession::new(session_id("replay"));
        let reference = semantic("top");
        let old_id = session.allocate().expect("the serial range is available");
        session
            .bind(reference.clone(), old_id.clone())
            .expect("the binding is valid");
        session
            .record_lineage(
                OperationKind::Map,
                LineageRelation::Preserved,
                old_id.clone(),
                [old_id.clone()],
            )
            .expect("the lineage is valid");

        let generation = session
            .advance_generation()
            .expect("the generation range is available");
        let replayed_id = session.allocate().expect("the serial range is available");

        assert_eq!(generation, TrackingGeneration::new(1));
        assert_eq!(old_id.serial(), replayed_id.serial());
        assert_ne!(old_id, replayed_id);
        assert!(session.bindings().is_empty());
        assert!(session.lineage().is_empty());
        assert!(matches!(
            session.validate_current(&old_id),
            Err(TrackingError::StaleGeneration { .. }),
        ));
    }

    #[test]
    fn validation_rejects_cross_session_and_unknown_ids() {
        let mut first = TrackingSession::new(session_id("first"));
        let second = TrackingSession::new(session_id("second"));
        let allocated = first.allocate().expect("the serial range is available");
        let unknown: TrackingId = "first@0:2"
            .parse()
            .expect("the compact tracking ID is valid");

        assert!(matches!(
            second.validate_current(&allocated),
            Err(TrackingError::WrongSession { .. }),
        ));
        assert_eq!(
            first.validate_current(&unknown),
            Err(TrackingError::UnknownTrackingId(unknown)),
        );
    }

    #[test]
    fn binding_is_idempotent_and_resolves_current_id() {
        let mut session = TrackingSession::new(session_id("binding"));
        let reference = semantic("top");
        let tracking_id = session.allocate().expect("the serial range is available");

        session
            .bind(reference.clone(), tracking_id.clone())
            .expect("the binding is valid");
        session
            .bind(reference.clone(), tracking_id.clone())
            .expect("repeating the binding is idempotent");

        assert_eq!(session.bindings().len(), 1);
        assert_eq!(session.resolve(&reference), Ok(&tracking_id));
    }

    #[test]
    fn tracking_id_serde_uses_a_compact_string() {
        let mut session = TrackingSession::new(session_id("cad-session"));
        session
            .advance_generation()
            .expect("the generation range is available");
        let tracking_id = session.allocate().expect("the serial range is available");

        let json = serde_json::to_string(&tracking_id).expect("serialization succeeds");
        let restored: TrackingId = serde_json::from_str(&json).expect("deserialization succeeds");

        assert_eq!(json, "\"cad-session@1:1\"");
        assert_eq!(restored, tracking_id);
    }

    #[test]
    fn tracking_id_content_hash_covers_every_identity_component() {
        let first: TrackingId = "first@0:1"
            .parse()
            .expect("the compact tracking ID is valid");
        let other_session: TrackingId = "second@0:1"
            .parse()
            .expect("the compact tracking ID is valid");
        let other_generation: TrackingId = "first@1:1"
            .parse()
            .expect("the compact tracking ID is valid");
        let other_serial: TrackingId = "first@0:2"
            .parse()
            .expect("the compact tracking ID is valid");

        assert_ne!(first.content_hash64(), other_session.content_hash64());
        assert_ne!(first.content_hash64(), other_generation.content_hash64());
        assert_ne!(first.content_hash64(), other_serial.content_hash64());
    }

    #[test]
    fn tracking_session_round_trip_preserves_bindings_and_lineage() {
        let mut session = TrackingSession::new(session_id("serialized"));
        let parent = session.allocate().expect("the serial range is available");
        let child = session.allocate().expect("the serial range is available");
        session
            .bind(semantic("source"), parent.clone())
            .expect("the binding is valid");
        session
            .bind(semantic("result"), child.clone())
            .expect("the binding is valid");
        session
            .record_lineage(
                OperationKind::Extrude,
                LineageRelation::Generated,
                parent,
                [child],
            )
            .expect("the lineage is valid");

        let json = serde_json::to_string(&session).expect("serialization succeeds");
        let restored: TrackingSession =
            serde_json::from_str(&json).expect("deserialization succeeds");

        assert_eq!(restored, session);
    }

    #[test]
    fn validated_strings_reject_ambiguous_or_unstable_values() {
        assert!(TrackingSessionId::new("contains:separator").is_err());
        assert!(TrackingSessionId::new("contains@separator").is_err());
        assert!(FeatureId::new(" feature").is_err());
        assert!(SemanticLabel::new("").is_err());
    }
}
