use monstertruck_topology::compress::{
    CompressedEdge, CompressedEdgeIndex, CompressedFace, CompressedShell,
};
use monstertruck_topology::errors::Error;
use monstertruck_topology::{Face, Shell};

#[test]
fn invalid_compressed_vertex_index_returns_a_typed_error() {
    let compressed = CompressedShell {
        vertices: vec![()],
        edges: vec![CompressedEdge {
            vertices: (0, 1),
            curve: (),
        }],
        faces: Vec::<CompressedFace<()>>::new(),
        vertex_stable_ids: None,
        edge_stable_ids: None,
        face_stable_ids: None,
    };
    let error = Shell::extract(compressed)
        .expect_err("the second edge endpoint is outside the vertex array");

    assert_eq!(
        error,
        Error::InvalidCompressedTopologyIndex {
            entity: "vertex",
            index: 1,
            len: 1,
        }
    );
}

#[test]
fn invalid_compressed_edge_index_returns_a_typed_error() {
    let compressed = CompressedShell {
        vertices: vec![(), ()],
        edges: vec![CompressedEdge {
            vertices: (0, 1),
            curve: (),
        }],
        faces: vec![CompressedFace {
            boundaries: vec![vec![CompressedEdgeIndex {
                index: 1,
                orientation: true,
            }]],
            orientation: true,
            surface: (),
        }],
        vertex_stable_ids: None,
        edge_stable_ids: None,
        face_stable_ids: None,
    };
    let error = Shell::extract(compressed).expect_err("the face use is outside the edge array");

    assert_eq!(
        error,
        Error::InvalidCompressedTopologyIndex {
            entity: "edge",
            index: 1,
            len: 1,
        }
    );
}

#[test]
fn empty_serialized_face_returns_an_error() {
    let compressed = CompressedShell::<(), (), ()> {
        vertices: Vec::new(),
        edges: Vec::new(),
        faces: Vec::new(),
        vertex_stable_ids: None,
        edge_stable_ids: None,
        face_stable_ids: None,
    };
    let json = serde_json::to_string(&compressed).expect("the empty wrapper serializes");
    let error = serde_json::from_str::<Face<(), (), ()>>(&json)
        .expect_err("a serialized face must contain exactly one face");

    assert!(error.to_string().contains("contains 0 faces"));
}
