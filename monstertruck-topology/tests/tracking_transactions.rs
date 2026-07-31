use monstertruck_core::{
    FeatureId, SemanticLabel, SemanticTopologyRef, TopologyKind, TrackingSession, TrackingSessionId,
};
use monstertruck_topology::{Edge, TopologyTracking, Vertex};

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
