//! Validates the accepted topology snapshot foundation without publishing a new API.

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, bail, ensure};
use monstertruck_topology::compress::CompressedSolid;
use monstertruck_topology::{AttributeValue, Edge, Face, Shell, Solid, Vertex, wire};
use serde::Serialize;
use serde_json::Value;

const BASE_REVISION: &str = "4b8774247dae1909c85db99e1b66b10c93878cb7";
const ATTRIBUTE_NAME: &str = "tracking/source";
const REFERENCE_NAME: &str = "tracking/references";

type FixtureSolid = Solid<[i64; 3], u32, u32>;
type FixtureCompressed = CompressedSolid<[i64; 3], u32, u32>;

#[derive(Debug)]
enum Mode {
    Print,
    Emit(PathBuf),
    Check(PathBuf),
}

#[derive(Debug, PartialEq, Serialize)]
struct Receipt {
    schema_version: u32,
    story: &'static str,
    evidence_class: &'static str,
    base_revision: &'static str,
    identity_boundary: &'static str,
    immediate_rerun_equal: bool,
    evidence: Evidence,
    deferred: [&'static str; 4],
}

#[derive(Debug, PartialEq, Serialize)]
struct Evidence {
    source: StateSnapshot,
    topology: TopologyEvidence,
    roundtrip: RoundtripEvidence,
    mutations: MutationEvidence,
}

#[derive(Debug, PartialEq, Serialize)]
struct StateSnapshot {
    compressed: FixtureCompressed,
    topology_hash: u64,
    topology_attribute_hash: u64,
    content_hash: u64,
}

#[derive(Debug, PartialEq, Serialize)]
struct TopologyEvidence {
    vertex_uses: usize,
    edge_uses: usize,
    faces: usize,
    unique_vertices: usize,
    unique_edges: usize,
    unique_faces: usize,
    all_ids_assigned: bool,
    ids_unique_within_kinds: bool,
    ids_unique_across_kinds: bool,
    allocator_next: u64,
    allocator_above_assigned_ids: bool,
    attributes_keyed_by_existing_ids: bool,
}

#[derive(Debug, PartialEq, Serialize)]
struct RoundtripEvidence {
    compressed_equal: bool,
    topology_hash_equal: bool,
    topology_attribute_hash_equal: bool,
    content_hash_equal: bool,
    id_assignment_idempotent: bool,
}

#[derive(Debug, PartialEq, Serialize)]
struct MutationEvidence {
    attribute_preserves_topology_hash: bool,
    attribute_changes_attribute_hash: bool,
    attribute_changes_content_hash: bool,
    geometry_preserves_topology_hash: bool,
    geometry_preserves_attribute_hash: bool,
    geometry_changes_content_hash: bool,
}

fn main() -> Result<()> {
    let mode = mode()?;
    let first = evidence()?;
    let second = evidence()?;
    ensure!(first == second, "immediate topology evidence rerun changed");
    let receipt = Receipt {
        schema_version: 1,
        story: "MT-402 topology snapshot foundation",
        evidence_class: "Implemented",
        base_revision: BASE_REVISION,
        identity_boundary: "StableId/StableIdAllocator in monstertruck-core; attributes keyed by StableId in monstertruck-topology",
        immediate_rerun_equal: true,
        evidence: first,
        deferred: [
            "tracking sessions, bindings, and lineage until a topology tracking API is accepted",
            "typed modeling-wrapper result and publication snapshots until the modeling layer",
            "caller-owned mutable modeling arguments until wrapper failure injection",
            "cross-architecture hash equivalence until hash encoding is architecture-independent",
        ],
    };
    let serialized = format!("{}\n", serde_json::to_string_pretty(&receipt)?);
    match mode {
        Mode::Print => print!("{serialized}"),
        Mode::Emit(path) => fs::write(&path, serialized)
            .with_context(|| format!("failed to write {}", path.display()))?,
        Mode::Check(path) => {
            let expected_text = fs::read_to_string(&path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            let expected = serde_json::from_str::<Value>(&expected_text)
                .with_context(|| format!("failed to parse {}", path.display()))?;
            let actual = serde_json::from_str::<Value>(&serialized)?;
            ensure!(
                expected == actual,
                "topology validation receipt differs from {}",
                path.display()
            );
        }
    }
    Ok(())
}

fn mode() -> Result<Mode> {
    match env::args().skip(1).collect::<Vec<_>>().as_slice() {
        [] => Ok(Mode::Print),
        [flag, path] if flag == "--emit" => Ok(Mode::Emit(PathBuf::from(path))),
        [flag, path] if flag == "--check" => Ok(Mode::Check(PathBuf::from(path))),
        _ => bail!("usage: topology-state-validation [--emit PATH | --check PATH]"),
    }
}

fn evidence() -> Result<Evidence> {
    let source = fixture()?;
    let source_snapshot = snapshot(&source);
    let roundtrip = Solid::extract(source_snapshot.compressed.clone())?;
    let roundtrip_snapshot = snapshot(&roundtrip);

    let mut reassigned = Solid::extract(source_snapshot.compressed.clone())?;
    reassigned.ensure_topology_stable_ids();
    let reassigned_snapshot = snapshot(&reassigned);

    let mut attribute_mutated = Solid::extract(source_snapshot.compressed.clone())?;
    let face_id = attribute_mutated
        .face_iter()
        .next()
        .map(Face::stable_id)
        .context("fixture has no face")?;
    attribute_mutated.face_attributes_mut().set(
        "tracking/reviewed",
        face_id,
        AttributeValue::Bool(true),
    );
    let attribute_snapshot = snapshot(&attribute_mutated);

    let geometry_mutated = Solid::extract(source_snapshot.compressed.clone())?;
    geometry_mutated
        .vertex_iter()
        .next()
        .context("fixture has no vertex")?
        .set_point([2, 3, 5]);
    let geometry_snapshot = snapshot(&geometry_mutated);

    let evidence = Evidence {
        topology: topology_evidence(&source)?,
        roundtrip: RoundtripEvidence {
            compressed_equal: source_snapshot.compressed == roundtrip_snapshot.compressed,
            topology_hash_equal: source_snapshot.topology_hash == roundtrip_snapshot.topology_hash,
            topology_attribute_hash_equal: source_snapshot.topology_attribute_hash
                == roundtrip_snapshot.topology_attribute_hash,
            content_hash_equal: source_snapshot.content_hash == roundtrip_snapshot.content_hash,
            id_assignment_idempotent: source_snapshot == reassigned_snapshot,
        },
        mutations: MutationEvidence {
            attribute_preserves_topology_hash: source_snapshot.topology_hash
                == attribute_snapshot.topology_hash,
            attribute_changes_attribute_hash: source_snapshot.topology_attribute_hash
                != attribute_snapshot.topology_attribute_hash,
            attribute_changes_content_hash: source_snapshot.content_hash
                != attribute_snapshot.content_hash,
            geometry_preserves_topology_hash: source_snapshot.topology_hash
                == geometry_snapshot.topology_hash,
            geometry_preserves_attribute_hash: source_snapshot.topology_attribute_hash
                == geometry_snapshot.topology_attribute_hash,
            geometry_changes_content_hash: source_snapshot.content_hash
                != geometry_snapshot.content_hash,
        },
        source: source_snapshot,
    };
    validate(&evidence)?;
    Ok(evidence)
}

fn validate(evidence: &Evidence) -> Result<()> {
    let topology = &evidence.topology;
    ensure!(
        topology.all_ids_assigned,
        "fixture contains an unassigned stable id"
    );
    ensure!(
        topology.ids_unique_within_kinds,
        "stable ids are duplicated within a topology kind"
    );
    ensure!(
        topology.ids_unique_across_kinds,
        "stable ids overlap across topology kinds"
    );
    ensure!(
        topology.allocator_above_assigned_ids,
        "allocator does not remain above assigned ids"
    );
    ensure!(
        topology.attributes_keyed_by_existing_ids,
        "attribute fixture contains an unresolved semantic reference"
    );
    let roundtrip = &evidence.roundtrip;
    ensure!(roundtrip.compressed_equal, "compressed round trip changed");
    ensure!(
        roundtrip.topology_hash_equal,
        "topology hash changed on round trip"
    );
    ensure!(
        roundtrip.topology_attribute_hash_equal,
        "topology attribute hash changed on round trip"
    );
    ensure!(
        roundtrip.content_hash_equal,
        "content hash changed on round trip"
    );
    ensure!(
        roundtrip.id_assignment_idempotent,
        "stable id reassignment changed an assigned topology"
    );
    let mutations = &evidence.mutations;
    ensure!(
        mutations.attribute_preserves_topology_hash
            && mutations.attribute_changes_attribute_hash
            && mutations.attribute_changes_content_hash,
        "attribute mutation did not remain isolated from topology"
    );
    ensure!(
        mutations.geometry_preserves_topology_hash
            && mutations.geometry_preserves_attribute_hash
            && mutations.geometry_changes_content_hash,
        "geometry mutation did not remain isolated from topology and attributes"
    );
    Ok(())
}

fn topology_evidence(solid: &FixtureSolid) -> Result<TopologyEvidence> {
    let compressed = solid.compress();
    let compressed_vertex_count = compressed
        .boundaries
        .iter()
        .map(|boundary| boundary.vertices.len())
        .sum::<usize>();
    let compressed_edge_count = compressed
        .boundaries
        .iter()
        .map(|boundary| boundary.edges.len())
        .sum::<usize>();
    let compressed_face_count = compressed
        .boundaries
        .iter()
        .map(|boundary| boundary.faces.len())
        .sum::<usize>();
    let vertex_ids = solid
        .vertex_iter()
        .map(|vertex| vertex.stable_id())
        .collect::<BTreeSet<_>>();
    let edge_ids = solid
        .edge_iter()
        .map(|edge| edge.stable_id())
        .collect::<BTreeSet<_>>();
    let face_ids = solid
        .face_iter()
        .map(Face::stable_id)
        .collect::<BTreeSet<_>>();
    let all_ids = vertex_ids
        .iter()
        .chain(&edge_ids)
        .chain(&face_ids)
        .copied()
        .collect::<BTreeSet<_>>();
    let maximum_id = all_ids
        .iter()
        .map(|id| id.raw())
        .max()
        .context("fixture has no stable ids")?;
    let source_vertex = vertex_ids
        .first()
        .copied()
        .context("fixture has no vertex stable id")?;
    let reference_face = face_ids
        .first()
        .copied()
        .context("fixture has no face stable id")?;
    Ok(TopologyEvidence {
        vertex_uses: solid.vertex_iter().count(),
        edge_uses: solid.edge_iter().count(),
        faces: solid.face_iter().count(),
        unique_vertices: vertex_ids.len(),
        unique_edges: edge_ids.len(),
        unique_faces: face_ids.len(),
        all_ids_assigned: all_ids.iter().all(|id| id.is_assigned()),
        ids_unique_within_kinds: vertex_ids.len() == compressed_vertex_count
            && edge_ids.len() == compressed_edge_count
            && face_ids.len() == compressed_face_count,
        ids_unique_across_kinds: all_ids.len()
            == vertex_ids.len() + edge_ids.len() + face_ids.len(),
        allocator_next: solid.id_allocator().peek(),
        allocator_above_assigned_ids: solid.id_allocator().peek() > maximum_id,
        attributes_keyed_by_existing_ids: solid
            .vertex_attributes()
            .get(ATTRIBUTE_NAME, source_vertex)
            == Some(&AttributeValue::String("fixture".to_owned()))
            && matches!(
                solid.face_attributes().get(REFERENCE_NAME, reference_face),
                Some(AttributeValue::IdSet(ids)) if ids.iter().all(|id| all_ids.contains(id))
            ),
    })
}

fn snapshot(solid: &FixtureSolid) -> StateSnapshot {
    StateSnapshot {
        compressed: solid.compress(),
        topology_hash: solid.topology_hash(),
        topology_attribute_hash: solid.topology_attribute_hash(),
        content_hash: solid.content_hash(),
    }
}

fn fixture() -> Result<FixtureSolid> {
    let vertices = Vertex::from_points([[0, 0, 0], [1, 0, 0], [0, 1, 0], [0, 0, 1]]);
    let edges = [
        Edge::new(&vertices[0], &vertices[1], 0),
        Edge::new(&vertices[0], &vertices[2], 1),
        Edge::new(&vertices[0], &vertices[3], 2),
        Edge::new(&vertices[1], &vertices[2], 3),
        Edge::new(&vertices[1], &vertices[3], 4),
        Edge::new(&vertices[2], &vertices[3], 5),
    ];
    let wires = [
        wire![&edges[0], &edges[3], &edges[1].inverse()],
        wire![&edges[1], &edges[5], &edges[2].inverse()],
        wire![&edges[2], &edges[4].inverse(), &edges[0].inverse()],
        wire![&edges[3], &edges[5], &edges[4].inverse()],
    ];
    let mut faces = wires
        .into_iter()
        .enumerate()
        .map(|(index, wire)| Face::new(vec![wire], index as u32))
        .collect::<Vec<_>>();
    faces
        .last_mut()
        .context("fixture has no terminal face")?
        .invert();
    let shell: Shell<[i64; 3], u32, u32> = faces.into();
    let mut solid = Solid::try_new(vec![shell])?;
    solid.ensure_topology_stable_ids();

    let vertex_id = solid
        .vertex_iter()
        .next()
        .map(|vertex| vertex.stable_id())
        .context("fixture has no vertex")?;
    let edge_ids = solid
        .edge_iter()
        .map(|edge| edge.stable_id())
        .collect::<BTreeSet<_>>();
    let face_id = solid
        .face_iter()
        .next()
        .map(Face::stable_id)
        .context("fixture has no face")?;
    solid.vertex_attributes_mut().set(
        ATTRIBUTE_NAME,
        vertex_id,
        AttributeValue::String("fixture".to_owned()),
    );
    solid.face_attributes_mut().set(
        REFERENCE_NAME,
        face_id,
        AttributeValue::IdSet(edge_ids.into_iter().take(2).collect()),
    );
    Ok(solid)
}
