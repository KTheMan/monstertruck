//! Controlled assignment of immutable, session-scoped topology identities.
//!
//! [`crate::StableId`] remains the serialized, solid-local compatibility key.
//! [`crate::TrackingId`] adds the session and replay generation needed by a
//! parametric feature graph. Tracking IDs have no public setter: callers hand
//! a newly built operation result to [`TopologyTracking::initialize_tracking`]
//! before publishing it.
//!
//! Operation policy is contextual:
//!
//! | Operation | Identity policy | Lineage |
//! | --- | --- | --- |
//! | map / rotate | preserve the one-to-one ID | `Preserved` |
//! | sweep / extrude / revolve | preserve embedded source topology and assign fresh IDs to generated topology | `Generated` |
//! | cut / split | assign distinct fresh IDs to ordered children | `Split` |
//!
//! A sweep internally maps its source to construct another layer. The
//! initializer detects copied IDs on distinct pointer identities and reseeds
//! those generated copies. This avoids giving the base and ceiling the same
//! identity while still allowing a standalone map to preserve identity.
//!
//! ## Initialization and serialization
//!
//! [`TopologyTracking::initialize_tracking`] assigns every distinct topology
//! element once. [`Shell::compress_tracked`](crate::Shell::compress_tracked)
//! and [`Shell::extract_tracked`](crate::Shell::extract_tracked) make tracking
//! metadata an explicit part of a compressed roundtrip.
//!
//! ```
//! use monstertruck_topology::{
//!     Edge, Face, FeatureId, Shell, TopologyTracking, TrackingSession,
//!     TrackingSessionId, Vertex, Wire,
//! };
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let vertices = Vertex::from_points([(), (), ()]);
//!     let face = Face::new(
//!         vec![Wire::from([
//!             Edge::new(&vertices[0], &vertices[1], ()),
//!             Edge::new(&vertices[1], &vertices[2], ()),
//!             Edge::new(&vertices[2], &vertices[0], ()),
//!         ])],
//!         (),
//!     );
//!     let mut shell = Shell::from([face]);
//!     let mut session =
//!         TrackingSession::new(TrackingSessionId::new("topology-session")?);
//!     let report =
//!         shell.initialize_tracking(&mut session, FeatureId::new("triangle")?)?;
//!     let expected = shell.tracking_ids();
//!
//!     let restored = Shell::extract_tracked(shell.compress_tracked())?;
//!
//!     assert_eq!(report.generated_ids().count(), 7);
//!     assert_eq!(restored.tracking_ids(), expected);
//!     Ok(())
//! }
//! ```

#[cfg(test)]
use crate::*;
#[cfg(test)]
use rustc_hash::FxHashSet as HashSet;

mod cuts;
mod implementations;
mod initialization;
mod interface;

pub use interface::{TopologyTracking, TrackingReplacement, TrackingReport, TrackingResult};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compress::TrackedCompressedShell;

    fn feature(value: &str) -> FeatureId {
        FeatureId::new(value).expect("the test feature identifier is valid")
    }

    fn session() -> TrackingSession {
        TrackingSession::new(
            TrackingSessionId::new("topology-tests").expect("the test session identifier is valid"),
        )
    }

    fn triangle() -> Face<(), (), ()> {
        let vertices = Vertex::from_points([(), (), ()]);
        Face::new(
            vec![Wire::from([
                Edge::new(&vertices[0], &vertices[1], ()),
                Edge::new(&vertices[1], &vertices[2], ()),
                Edge::new(&vertices[2], &vertices[0], ()),
            ])],
            (),
        )
    }

    fn square() -> (Face<(), (), ()>, Vec<Vertex<()>>) {
        let vertices = Vertex::from_points([(), (), (), ()]);
        let face = Face::new(
            vec![Wire::from([
                Edge::new(&vertices[0], &vertices[1], ()),
                Edge::new(&vertices[1], &vertices[2], ()),
                Edge::new(&vertices[2], &vertices[3], ()),
                Edge::new(&vertices[3], &vertices[0], ()),
            ])],
            (),
        );
        (face, vertices)
    }

    #[test]
    fn initialization_is_unique_and_shared_uses_agree() {
        let mut face = triangle();
        let mut session = session();
        let report = face
            .initialize_tracking(&mut session, feature("triangle"))
            .expect("tracking initialization succeeds");

        assert_eq!(report.all_ids().len(), 7);
        assert_eq!(report.generated().len(), 7);
        assert_eq!(
            report.all_ids().iter().collect::<HashSet<_>>().len(),
            report.all_ids().len(),
        );
        assert_eq!(
            face.boundaries()[0][0].back().tracking_id(),
            face.boundaries()[0][1].front().tracking_id(),
        );
    }

    #[test]
    fn mapped_face_preserves_one_to_one_tracking() {
        let mut face = triangle();
        let mut session = session();
        face.initialize_tracking(&mut session, feature("source"))
            .expect("source tracking succeeds");
        let source_ids = face.tracking_ids();
        let mapped = face.mapped(|_| (), |_| (), |_| ());

        assert_eq!(mapped.tracking_ids(), source_ids);
    }

    #[test]
    fn one_session_allocates_unique_ids_across_multiple_results() {
        let mut first = Shell::from([triangle()]);
        let mut second = Shell::from([triangle()]);
        let mut session = session();
        first
            .initialize_tracking(&mut session, feature("first"))
            .expect("first tracking initialization succeeds");
        second
            .initialize_tracking(&mut session, feature("second"))
            .expect("second tracking initialization succeeds");
        let first_ids: HashSet<_> = first.tracking_ids().into_iter().collect();
        let second_ids: HashSet<_> = second.tracking_ids().into_iter().collect();

        assert!(first_ids.is_disjoint(&second_ids));
    }

    #[test]
    fn edge_split_assigns_distinct_children_and_ordered_lineage() {
        #[derive(Clone)]
        struct Segment;

        impl ParametricCurve for Segment {
            type Point = f64;
            type Vector = f64;
            fn evaluate(&self, parameter: f64) -> f64 { parameter }
            fn derivative(&self, _: f64) -> f64 { 1.0 }
            fn derivative_2(&self, _: f64) -> f64 { 0.0 }
            fn derivative_n(&self, order: usize, parameter: f64) -> f64 {
                match order {
                    0 => self.evaluate(parameter),
                    1 => 1.0,
                    _ => 0.0,
                }
            }
            fn parameter_range(&self) -> ParameterRange {
                (
                    std::ops::Bound::Included(0.0),
                    std::ops::Bound::Included(1.0),
                )
            }
        }

        impl BoundedCurve for Segment {}

        impl Cut for Segment {
            fn cut(&mut self, _: f64) -> Self { Self }
        }

        let vertices = Vertex::from_points([0.0, 1.0]);
        let mut edge = Edge::new(&vertices[0], &vertices[1], Segment);
        let mut session = session();
        edge.initialize_tracking(&mut session, feature("source"))
            .expect("source tracking succeeds");
        let parent = edge.tracking_id().cloned().expect("the parent is tracked");
        let split_vertex = Vertex::new(0.5);
        let (first, second) = edge
            .cut_with_parameter_tracked(&split_vertex, 0.5, &mut session, feature("split"))
            .expect("tracking succeeds")
            .expect("the parameter is interior");
        let first_id = first.tracking_id().expect("the first child is tracked");
        let second_id = second.tracking_id().expect("the second child is tracked");

        assert_ne!(first_id, second_id);
        assert_ne!(first_id, &parent);
        assert_ne!(second_id, &parent);
        let event = session.lineage().last().expect("lineage was recorded");
        assert_eq!(event.parent(), &parent);
        assert_eq!(event.children(), &[first_id.clone(), second_id.clone()]);
        assert_eq!(event.relation(), LineageRelation::Split);
    }

    #[test]
    fn face_cut_assigns_two_fresh_faces_and_split_lineage() {
        let (mut face, vertices) = square();
        let mut session = session();
        face.initialize_tracking(&mut session, feature("source-face"))
            .expect("source tracking succeeds");
        let parent = face.tracking_id().cloned().expect("the parent is tracked");
        let diagonal = Edge::new(&vertices[0], &vertices[2], ());
        let (first, second) = face
            .cut_by_edge_tracked(diagonal, &mut session, feature("face-cut"))
            .expect("tracking succeeds")
            .expect("the diagonal splits the square");
        let first_id = first.tracking_id().expect("the first face is tracked");
        let second_id = second.tracking_id().expect("the second face is tracked");

        assert_ne!(first_id, second_id);
        assert_ne!(first_id, &parent);
        assert_ne!(second_id, &parent);
        let event = session.lineage().last().expect("lineage was recorded");
        assert_eq!(event.parent(), &parent);
        assert_eq!(event.children(), &[first_id.clone(), second_id.clone()]);
        assert_eq!(event.relation(), LineageRelation::Split);
    }

    #[test]
    fn tracking_ids_survive_compressed_topology_roundtrip() {
        let mut shell = Shell::from([triangle()]);
        let mut session = session();
        shell
            .initialize_tracking(&mut session, feature("serialized"))
            .expect("tracking initialization succeeds");
        let expected = shell.tracking_ids();
        let extracted =
            Shell::extract_tracked(shell.compress_tracked()).expect("compressed shell is valid");

        assert_eq!(extracted.tracking_ids(), expected);
    }

    #[test]
    fn explicit_tracked_shell_serde_roundtrip_uses_tracking_wrapper() {
        let mut shell = Shell::from([triangle()]);
        let mut session = session();
        shell
            .initialize_tracking(&mut session, feature("serde"))
            .expect("tracking initialization succeeds");
        let expected = shell.tracking_ids();
        let json =
            serde_json::to_string(&shell.compress_tracked()).expect("tracked shell serializes");
        let compressed: TrackedCompressedShell<(), (), ()> =
            serde_json::from_str(&json).expect("tracked shell deserializes");
        let extracted = Shell::extract_tracked(compressed).expect("compressed shell is valid");

        assert!(json.contains("\"tracking\""));
        assert_eq!(extracted.tracking_ids(), expected);
    }
}
