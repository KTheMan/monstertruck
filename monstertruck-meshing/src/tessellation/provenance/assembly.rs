use std::iter::repeat_n;

use monstertruck_core::{Point3, StableId};
use monstertruck_mesh::{PolygonMesh, PolylineCurve};
use monstertruck_topology::{EdgeId, Shell, Solid, compress::*};
use rustc_hash::FxHashSet as HashSet;

use crate::*;

use super::{EdgeLocator, EdgeProvenance, FaceProvenance, TessellationWithProvenance};

#[derive(Default)]
pub(super) struct ProvenanceAccumulator {
    polygon: PolygonMesh,
    edge_polylines: Vec<PolylineCurve<Point3>>,
    faces: Vec<FaceProvenance>,
    edges: Vec<EdgeProvenance>,
    triangle_faces: Vec<usize>,
    quadrangle_faces: Vec<usize>,
    other_faces: Vec<usize>,
    line_edges: Vec<usize>,
}

impl ProvenanceAccumulator {
    #[cfg(test)]
    pub(super) fn add_face(&mut self, provenance: FaceProvenance) { self.faces.push(provenance); }

    fn add_shell<C, S>(
        &mut self,
        source: &Shell<Point3, C, S>,
        meshed: &Shell<Point3, PolylineCurve<Point3>, Option<PolygonMesh>>,
        shell_index: usize,
    ) {
        debug_assert_eq!(source.len(), meshed.len());
        let face_offset = self.faces.len();
        self.faces
            .extend(
                source
                    .face_iter()
                    .enumerate()
                    .map(|(face_index, face)| FaceProvenance {
                        shell_index,
                        face_index,
                        stable_id: assigned_id(face.stable_id()),
                    }),
            );
        meshed
            .face_iter()
            .enumerate()
            .filter_map(|(face_index, face)| {
                face.surface().map(|mut polygon| {
                    if !face.orientation() {
                        polygon.invert();
                    }
                    (face_offset + face_index, polygon)
                })
            })
            .for_each(|(face_index, polygon)| self.add_polygon(face_index, polygon));

        let mut visited = HashSet::<EdgeId<C>>::default();
        let entries = source
            .face_iter()
            .zip(meshed.face_iter())
            .enumerate()
            .flat_map(|(face_index, (source_face, meshed_face))| {
                source_face
                    .boundaries()
                    .into_iter()
                    .zip(meshed_face.boundaries())
                    .enumerate()
                    .flat_map(move |(boundary_index, (source_wire, meshed_wire))| {
                        source_wire.into_iter().zip(meshed_wire).enumerate().map(
                            move |(edge_index, (source_edge, meshed_edge))| {
                                (
                                    face_index,
                                    boundary_index,
                                    edge_index,
                                    source_edge,
                                    meshed_edge,
                                )
                            },
                        )
                    })
            })
            .filter_map(
                |(face_index, boundary_index, edge_index, source_edge, meshed_edge)| {
                    visited.insert(source_edge.id()).then(|| {
                        (
                            meshed_edge.curve(),
                            EdgeProvenance {
                                locator: EdgeLocator::TopologyUse {
                                    shell_index,
                                    face_index,
                                    boundary_index,
                                    edge_index,
                                },
                                stable_id: assigned_id(source_edge.stable_id()),
                            },
                        )
                    })
                },
            );
        self.add_edges(entries);
    }

    fn add_compressed_shell(
        &mut self,
        meshed: &CompressedShell<Point3, PolylineCurve<Point3>, Option<PolygonMesh>>,
        shell_index: usize,
        face_stable_ids: Option<&[StableId]>,
        edge_stable_ids: Option<&[StableId]>,
    ) {
        let face_offset = self.faces.len();
        self.faces.extend(
            meshed
                .faces
                .iter()
                .enumerate()
                .map(|(face_index, _)| FaceProvenance {
                    shell_index,
                    face_index,
                    stable_id: stable_id_at(face_stable_ids, face_index),
                }),
        );
        meshed
            .faces
            .iter()
            .enumerate()
            .filter_map(|(face_index, face)| {
                face.surface.clone().map(|mut polygon| {
                    if !face.orientation {
                        polygon.invert();
                    }
                    (face_offset + face_index, polygon)
                })
            })
            .for_each(|(face_index, polygon)| self.add_polygon(face_index, polygon));

        self.add_edges(meshed.edges.iter().enumerate().map(|(edge_index, edge)| {
            (
                edge.curve.clone(),
                EdgeProvenance {
                    locator: EdgeLocator::Compressed {
                        shell_index,
                        edge_index,
                    },
                    stable_id: stable_id_at(edge_stable_ids, edge_index),
                },
            )
        }));
    }

    fn add_edges(
        &mut self,
        entries: impl IntoIterator<Item = (PolylineCurve<Point3>, EdgeProvenance)>,
    ) {
        entries.into_iter().for_each(|(polyline, provenance)| {
            let edge_index = self.edges.len();
            self.line_edges
                .extend(repeat_n(edge_index, polyline.len().saturating_sub(1)));
            self.edge_polylines.push(polyline);
            self.edges.push(provenance);
        });
        debug_assert_eq!(self.edge_polylines.len(), self.edges.len());
    }

    pub(super) fn add_polygon(&mut self, face_index: usize, polygon: PolygonMesh) {
        self.triangle_faces
            .extend(repeat_n(face_index, polygon.tri_faces().len()));
        self.quadrangle_faces.extend(repeat_n(
            face_index,
            polygon.quad_faces().len().saturating_mul(2),
        ));
        self.other_faces.extend(
            polygon
                .other_faces()
                .iter()
                .flat_map(|face| repeat_n(face_index, face.len().saturating_sub(2))),
        );
        self.polygon.merge(polygon);
    }

    pub(super) fn finish(self) -> TessellationWithProvenance {
        let tessellation = TessellationWithProvenance {
            polygon: self.polygon,
            edge_polylines: self.edge_polylines,
            faces: self.faces,
            edges: self.edges,
            triangle_face_indices: self
                .triangle_faces
                .into_iter()
                .chain(self.quadrangle_faces)
                .chain(self.other_faces)
                .collect(),
            line_edge_indices: self.line_edges,
        };
        debug_assert_eq!(
            tessellation.triangle_face_indices.len(),
            tessellation.polygon.faces().triangle_iter().len(),
        );
        debug_assert_eq!(
            tessellation.line_edge_indices.len(),
            tessellation
                .edge_polylines
                .iter()
                .map(|polyline| polyline.len().saturating_sub(1))
                .sum::<usize>(),
        );
        debug_assert!(
            tessellation
                .triangle_face_indices
                .iter()
                .all(|index| *index < tessellation.faces.len()),
        );
        debug_assert!(
            tessellation
                .line_edge_indices
                .iter()
                .all(|index| *index < tessellation.edges.len()),
        );
        tessellation
    }
}

pub(super) fn assigned_id(id: StableId) -> Option<StableId> { id.is_assigned().then_some(id) }

pub(super) fn stable_id_at(ids: Option<&[StableId]>, index: usize) -> Option<StableId> {
    ids.and_then(|ids| ids.get(index))
        .copied()
        .and_then(assigned_id)
}

pub(super) fn shell_provenance<C, S>(
    source: &Shell<Point3, C, S>,
    meshed: &Shell<Point3, PolylineCurve<Point3>, Option<PolygonMesh>>,
) -> TessellationWithProvenance {
    let mut accumulator = ProvenanceAccumulator::default();
    accumulator.add_shell(source, meshed, 0);
    accumulator.finish()
}

pub(super) fn solid_provenance<C, S>(
    source: &Solid<Point3, C, S>,
    meshed: &[Shell<Point3, PolylineCurve<Point3>, Option<PolygonMesh>>],
) -> TessellationWithProvenance {
    source
        .boundaries()
        .iter()
        .zip(meshed)
        .enumerate()
        .fold(
            ProvenanceAccumulator::default(),
            |mut accumulator, (shell_index, (source, meshed))| {
                accumulator.add_shell(source, meshed, shell_index);
                accumulator
            },
        )
        .finish()
}

pub(super) fn compressed_shell_provenance<C, S>(
    source: &CompressedShell<Point3, C, S>,
    meshed: &CompressedShell<Point3, PolylineCurve<Point3>, Option<PolygonMesh>>,
) -> TessellationWithProvenance {
    debug_assert_eq!(source.faces.len(), meshed.faces.len());
    debug_assert_eq!(source.edges.len(), meshed.edges.len());
    let mut accumulator = ProvenanceAccumulator::default();
    accumulator.add_compressed_shell(
        meshed,
        0,
        source.face_stable_ids.as_deref(),
        source.edge_stable_ids.as_deref(),
    );
    accumulator.finish()
}

pub(super) fn compressed_solid_provenance<C, S>(
    source: &CompressedSolid<Point3, C, S>,
    meshed: &[CompressedShell<Point3, PolylineCurve<Point3>, Option<PolygonMesh>>],
) -> TessellationWithProvenance {
    source
        .boundaries
        .iter()
        .zip(meshed)
        .enumerate()
        .fold(
            ProvenanceAccumulator::default(),
            |mut accumulator, (shell_index, (source, meshed))| {
                debug_assert_eq!(source.faces.len(), meshed.faces.len());
                debug_assert_eq!(source.edges.len(), meshed.edges.len());
                accumulator.add_compressed_shell(
                    meshed,
                    shell_index,
                    source.face_stable_ids.as_deref(),
                    source.edge_stable_ids.as_deref(),
                );
                accumulator
            },
        )
        .finish()
}

pub(super) fn trimmed_shell_provenance<C, S, T>(
    source: &CompressedTrimmedShell<Point3, C, S, T>,
    meshed: &CompressedShell<Point3, PolylineCurve<Point3>, Option<PolygonMesh>>,
) -> TessellationWithProvenance {
    let mut accumulator = ProvenanceAccumulator::default();
    accumulator.add_compressed_shell(meshed, 0, None, None);
    debug_assert_eq!(source.faces.len(), meshed.faces.len());
    debug_assert_eq!(source.edges.len(), meshed.edges.len());
    accumulator.finish()
}

pub(super) fn trimmed_solid_provenance<C, S, T>(
    source: &CompressedTrimmedSolid<Point3, C, S, T>,
    meshed: &[CompressedShell<Point3, PolylineCurve<Point3>, Option<PolygonMesh>>],
) -> TessellationWithProvenance {
    source
        .boundaries
        .iter()
        .zip(meshed)
        .enumerate()
        .fold(
            ProvenanceAccumulator::default(),
            |mut accumulator, (shell_index, (source, meshed))| {
                debug_assert_eq!(source.faces.len(), meshed.faces.len());
                debug_assert_eq!(source.edges.len(), meshed.edges.len());
                accumulator.add_compressed_shell(meshed, shell_index, None, None);
                accumulator
            },
        )
        .finish()
}
