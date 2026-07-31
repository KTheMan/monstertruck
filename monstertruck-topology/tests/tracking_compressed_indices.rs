use std::collections::BTreeMap;

use monstertruck_core::{TrackingSession, TrackingSessionId};
use monstertruck_topology::Shell;
use monstertruck_topology::compress::{
    CompressedEdge, CompressedEdgeIndex, CompressedFace, CompressedShell,
    CompressedTopologyTracking, TrackedCompressedShell,
};

#[test]
fn reordered_face_references_preserve_compressed_tracking_indices() {
    let mut session = TrackingSession::new(
        TrackingSessionId::new("compressed-index-test")
            .expect("the test session identifier is valid"),
    );
    let tracking_ids = (0..7)
        .map(|_| {
            session
                .allocate()
                .expect("the test tracking serial range is available")
        })
        .collect::<Vec<_>>();
    let topology = CompressedShell {
        vertices: vec![10, 20, 30],
        edges: vec![
            CompressedEdge {
                vertices: (0, 1),
                curve: 100,
            },
            CompressedEdge {
                vertices: (1, 2),
                curve: 200,
            },
            CompressedEdge {
                vertices: (2, 0),
                curve: 300,
            },
        ],
        faces: vec![CompressedFace {
            boundaries: vec![vec![
                CompressedEdgeIndex {
                    index: 2,
                    orientation: true,
                },
                CompressedEdgeIndex {
                    index: 0,
                    orientation: true,
                },
                CompressedEdgeIndex {
                    index: 1,
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
        vertices: tracking_ids[0..3].iter().cloned().map(Some).collect(),
        edges: tracking_ids[3..6].iter().cloned().map(Some).collect(),
        faces: vec![Some(tracking_ids[6].clone())],
    };
    let shell = Shell::extract_tracked(TrackedCompressedShell { topology, tracking })
        .expect("the reordered compressed shell is structurally valid");
    let face = &shell[0];
    let edge_ids = face.boundaries()[0]
        .edge_iter()
        .map(|edge| {
            (
                edge.curve(),
                edge.tracking_id()
                    .expect("the compressed edge tracking ID is restored")
                    .clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let vertex_ids = face.boundaries()[0]
        .edge_iter()
        .flat_map(|edge| [edge.absolute_front(), edge.absolute_back()])
        .map(|vertex| {
            (
                vertex.point(),
                vertex
                    .tracking_id()
                    .expect("the compressed vertex tracking ID is restored")
                    .clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();

    assert_eq!(edge_ids[&100], tracking_ids[3]);
    assert_eq!(edge_ids[&200], tracking_ids[4]);
    assert_eq!(edge_ids[&300], tracking_ids[5]);
    assert_eq!(vertex_ids[&10], tracking_ids[0]);
    assert_eq!(vertex_ids[&20], tracking_ids[1]);
    assert_eq!(vertex_ids[&30], tracking_ids[2]);
    assert_eq!(face.tracking_id(), Some(&tracking_ids[6]));
}
