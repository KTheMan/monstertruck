use monstertruck_core::{
    FeatureId, SemanticLabel, SemanticTopologyRef, TopologyKind, TrackingError, TrackingSession,
    TrackingSessionId,
};
use monstertruck_topology::compress::{
    CompressedEdge, CompressedEdgeIndex, CompressedFace, CompressedShell,
    CompressedTopologyTracking, TrackedCompressedShell,
};
use monstertruck_topology::{Edge, Shell, TopologyTracking, Vertex};

#[test]
fn failed_initialization_preserves_topology_and_session() {
    let feature = FeatureId::new("collision").expect("the test feature identifier is valid");
    let reference = SemanticTopologyRef::new(
        feature.clone(),
        TopologyKind::Edge,
        SemanticLabel::new("edge.0000").expect("the test semantic label is valid"),
    );
    let mut session = TrackingSession::new(
        TrackingSessionId::new("transaction-test").expect("the test session identifier is valid"),
    );
    let occupied = session
        .allocate()
        .expect("the test tracking serial is available");
    session
        .bind(reference, occupied)
        .expect("the conflicting reference is pre-bound");
    let vertices = Vertex::from_points([(), ()]);
    let mut edge = Edge::new(&vertices[0], &vertices[1], ());
    let original_ids = edge.tracking_ids();
    let original_session = session.clone();

    edge.initialize_tracking(&mut session, feature)
        .expect_err("the generated edge label conflicts with the pre-bound reference");

    assert_eq!(edge.tracking_ids(), original_ids);
    assert_eq!(session, original_session);
}

#[test]
fn fallible_edge_mapping_preserves_identity() {
    let vertices = Vertex::from_points([(), ()]);
    let mut edge = Edge::new(&vertices[0], &vertices[1], ());
    let mut session = TrackingSession::new(
        TrackingSessionId::new("mapping-test").expect("the test session identifier is valid"),
    );
    edge.initialize_tracking(
        &mut session,
        FeatureId::new("source").expect("the test feature identifier is valid"),
    )
    .expect("source tracking succeeds");
    let expected_ids = edge.tracking_ids();
    let expected_stable_id = edge.stable_id();
    let mapped = edge
        .try_mapped(|_| Some(()), |_| Some(()))
        .expect("both geometry mappings succeed");

    assert_eq!(mapped.tracking_ids(), expected_ids);
    assert_eq!(mapped.stable_id(), expected_stable_id);
}

#[test]
fn existing_binding_kind_must_match_the_topology_entity() {
    let mut session = TrackingSession::new(
        TrackingSessionId::new("kind-test").expect("the test session identifier is valid"),
    );
    let face_id = session
        .allocate()
        .expect("the test tracking serial is available");
    session
        .bind(
            SemanticTopologyRef::new(
                FeatureId::new("persisted").expect("the test feature identifier is valid"),
                TopologyKind::Face,
                SemanticLabel::new("face.0000").expect("the test semantic label is valid"),
            ),
            face_id.clone(),
        )
        .expect("the persisted face binding is valid");
    let topology = CompressedShell {
        vertices: vec![(), (), ()],
        edges: vec![
            CompressedEdge {
                vertices: (0, 1),
                curve: (),
            },
            CompressedEdge {
                vertices: (1, 2),
                curve: (),
            },
            CompressedEdge {
                vertices: (2, 0),
                curve: (),
            },
        ],
        faces: vec![CompressedFace {
            boundaries: vec![vec![
                CompressedEdgeIndex {
                    index: 0,
                    orientation: true,
                },
                CompressedEdgeIndex {
                    index: 1,
                    orientation: true,
                },
                CompressedEdgeIndex {
                    index: 2,
                    orientation: true,
                },
            ]],
            orientation: true,
            surface: (),
        }],
        vertex_stable_ids: None,
        edge_stable_ids: None,
        face_stable_ids: None,
    };
    let tracking = CompressedTopologyTracking {
        vertices: vec![Some(face_id), None, None],
        edges: vec![None, None, None],
        faces: vec![None],
    };
    let mut shell = Shell::extract_tracked(TrackedCompressedShell { topology, tracking })
        .expect("the compressed topology is structurally valid");
    let original_ids = shell.tracking_ids();
    let original_session = session.clone();
    let error = shell
        .initialize_tracking(
            &mut session,
            FeatureId::new("replay").expect("the test feature identifier is valid"),
        )
        .expect_err("a face-bound identifier cannot be preserved on a vertex");

    assert_eq!(
        error,
        TrackingError::TopologyKindMismatch {
            expected: TopologyKind::Vertex,
            actual: TopologyKind::Face,
        }
    );
    assert_eq!(shell.tracking_ids(), original_ids);
    assert_eq!(session, original_session);
}
