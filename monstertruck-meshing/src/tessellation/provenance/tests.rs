use std::iter::repeat_n;

use monstertruck_mesh::{Faces, StandardAttributes};
use monstertruck_modeling::{BoundingBox, Point3, Solid, primitive};

use super::super::{TessellationPrimitiveMode, TessellationPrimitiveOptions};
use super::assembly::{ProvenanceAccumulator, assigned_id, stable_id_at};
use super::*;

fn cuboid() -> Solid {
    primitive::cuboid(BoundingBox::from_iter([
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 1.0, 1.0),
    ]))
}

fn quad_options() -> TessellationOptions {
    TessellationOptions {
        primitive: TessellationPrimitiveOptions {
            mode: TessellationPrimitiveMode::PreferQuads,
            ..Default::default()
        },
        ..Default::default()
    }
}

fn assert_mapping_lengths(tessellation: &TessellationWithProvenance) {
    assert_eq!(
        tessellation.triangle_face_indices.len(),
        tessellation.polygon.faces().triangle_iter().len(),
    );
    assert_eq!(
        tessellation.line_edge_indices.len(),
        tessellation
            .edge_polylines
            .iter()
            .map(|polyline| polyline.len().saturating_sub(1))
            .sum::<usize>(),
    );
    assert!(
        tessellation
            .triangle_face_indices
            .iter()
            .all(|index| *index < tessellation.faces.len()),
    );
    assert!(
        tessellation
            .line_edge_indices
            .iter()
            .all(|index| *index < tessellation.edges.len()),
    );
}

#[test]
fn live_solid_provenance_resolves_shared_topology() {
    let mut solid = cuboid();
    solid.ensure_topology_stable_ids();
    let options = quad_options();
    let tessellation = solid.triangulation_with_provenance(options);
    let meshed = solid
        .boundaries()
        .iter()
        .map(|shell| triangulation_with(shell, options))
        .collect::<Vec<_>>();

    assert_mapping_lengths(&tessellation);
    assert_eq!(tessellation.faces.len(), 6);
    assert_eq!(tessellation.edges.len(), 12);
    tessellation.faces.iter().for_each(|provenance| {
        let face = &solid.boundaries()[provenance.shell_index][provenance.face_index];
        assert_eq!(provenance.stable_id, assigned_id(face.stable_id()));
    });
    tessellation
        .edges
        .iter()
        .zip(&tessellation.edge_polylines)
        .for_each(|(provenance, polyline)| {
            let EdgeLocator::TopologyUse {
                shell_index,
                face_index,
                boundary_index,
                edge_index,
            } = provenance.locator
            else {
                panic!("live topology must use a topology-use locator");
            };
            let source_boundaries = solid.boundaries()[shell_index][face_index].boundaries();
            let meshed_boundaries = meshed[shell_index][face_index].boundaries();
            assert_eq!(
                provenance.stable_id,
                assigned_id(source_boundaries[boundary_index][edge_index].stable_id()),
            );
            assert_eq!(
                polyline,
                &meshed_boundaries[boundary_index][edge_index].curve(),
            );
        });
    assert_eq!(tessellation, solid.triangulation_with_provenance(options),);
}

#[test]
fn unassigned_live_topology_remains_snapshot_resolvable() {
    let solid = cuboid();
    let tessellation = solid.triangulation_with_provenance(TessellationOptions::default());

    assert_mapping_lengths(&tessellation);
    assert!(
        tessellation
            .faces
            .iter()
            .all(|face| face.stable_id.is_none())
    );
    assert!(
        tessellation
            .edges
            .iter()
            .all(|edge| edge.stable_id.is_none())
    );
    assert!(
        tessellation
            .edges
            .iter()
            .all(|edge| matches!(edge.locator, EdgeLocator::TopologyUse { .. })),
    );
}

#[test]
fn multi_shell_indices_are_preserved() {
    let first = cuboid().into_boundaries();
    let second = cuboid().into_boundaries();
    let mut solid = Solid::new_unchecked(first.into_iter().chain(second).collect());
    solid.ensure_topology_stable_ids();

    let tessellation = solid.triangulation_with_provenance(TessellationOptions::default());

    assert_mapping_lengths(&tessellation);
    assert_eq!(tessellation.faces.len(), 12);
    assert_eq!(tessellation.edges.len(), 24);
    assert_eq!(
        tessellation
            .faces
            .iter()
            .map(FaceProvenance::shell_index)
            .collect::<Vec<_>>(),
        [repeat_n(0, 6), repeat_n(1, 6)]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>(),
    );
    assert!(
        tessellation
            .edges
            .iter()
            .any(|edge| edge.shell_index() == 1),
    );
}

#[test]
fn compressed_shape_families_preserve_array_locators() {
    let mut solid = cuboid();
    solid.ensure_topology_stable_ids();
    let compressed = solid.compress();
    let options = quad_options();

    let shell = compressed.boundaries[0].triangulation_with_provenance(options);
    let robust_shell = compressed.boundaries[0].robust_triangulation_with_provenance(options);
    let tessellation = compressed.triangulation_with_provenance(options);
    let robust = compressed.robust_triangulation_with_provenance(options);
    let meshed = compressed
        .boundaries
        .iter()
        .map(|shell| cshell_triangulation_with(shell, options))
        .collect::<Vec<_>>();

    [&shell, &robust_shell, &tessellation, &robust]
        .into_iter()
        .for_each(assert_mapping_lengths);
    assert_eq!(tessellation.faces.len(), 6);
    assert_eq!(tessellation.edges.len(), 12);
    tessellation
        .edges
        .iter()
        .zip(&tessellation.edge_polylines)
        .for_each(|(provenance, polyline)| {
            let EdgeLocator::Compressed {
                shell_index,
                edge_index,
            } = provenance.locator
            else {
                panic!("compressed topology must use an array locator");
            };
            assert_eq!(
                provenance.stable_id,
                stable_id_at(
                    compressed.boundaries[shell_index]
                        .edge_stable_ids
                        .as_deref(),
                    edge_index,
                ),
            );
            assert_eq!(polyline, &meshed[shell_index].edges[edge_index].curve);
        });
}

#[test]
fn exact_trimmed_shape_families_expose_provenance() {
    let solid = cuboid();
    let trimmed = solid.compress_with_exact_face_trims();
    let options = TessellationOptions::default();

    let shell = trimmed.boundaries[0].triangulation_with_provenance(options);
    let robust_shell = trimmed.boundaries[0].robust_triangulation_with_provenance(options);
    let tessellation = trimmed.triangulation_with_provenance(options);
    let robust = trimmed.robust_triangulation_with_provenance(options);

    [&shell, &robust_shell, &tessellation, &robust]
        .into_iter()
        .for_each(|tessellation| {
            assert_mapping_lengths(tessellation);
            assert_eq!(tessellation.faces.len(), 6);
            assert_eq!(tessellation.edges.len(), 12);
            assert!(
                tessellation
                    .faces
                    .iter()
                    .all(|face| face.stable_id.is_none())
            );
            assert!(
                tessellation
                    .edges
                    .iter()
                    .all(|edge| edge.stable_id.is_none())
            );
            assert!(
                tessellation
                    .edges
                    .iter()
                    .all(|edge| matches!(edge.locator, EdgeLocator::Compressed { .. })),
            );
        });
}

#[test]
fn triangle_mapping_matches_face_storage_order() {
    let faces: Faces = [vec![0, 1, 2], vec![0, 1, 2, 3], vec![0, 1, 2, 3, 4]]
        .into_iter()
        .collect();
    let polygon = PolygonMesh::new(
        StandardAttributes {
            positions: (0..5)
                .map(|index| Point3::new(index as f64, 0.0, 0.0))
                .collect(),
            ..Default::default()
        },
        faces,
    );
    let mut accumulator = ProvenanceAccumulator::default();

    accumulator.add_face(FaceProvenance {
        shell_index: 0,
        face_index: 0,
        stable_id: None,
    });
    accumulator.add_polygon(0, polygon);
    let tessellation = accumulator.finish();

    assert_eq!(tessellation.triangle_face_indices, vec![0; 6]);
    assert_eq!(
        tessellation.triangle_face_indices.len(),
        tessellation.polygon.faces().triangle_iter().len(),
    );
}
