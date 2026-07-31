//! Serialized topological data exchange format
//!
//! Topological data structures in monstertruck are subject to editing and has complex reference relationships.
//! They are not suitable for direct serialization and must be converted to lighter and simpler data structures.
//! These structures, prefixed with `Compressed`, are a group of structures that are easy to serialize,
//! but not suitable for real-time shape editing.
//!
//! They directly reflect the results of parsing data from json or STEP, and all member variables are public.
//! Boundary connectivity and closure are checked when converting to proprietary data structures, `Vertex`, `Edge`, and so on.

use monstertruck_core::{ContentHasher, DeterministicContentHash};
use rustc_hash::FxHashMap as HashMap;
use serde::{Deserialize, Serialize};

use crate::{errors::Error, *};

/// Serialized compressed edge
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompressedEdge<C> {
    /// vertices of the edge
    pub vertices: (usize, usize),
    /// curve geometry of the edge
    pub curve: C,
}

impl<C> CompressedEdge<C> {
    #[inline(always)]
    fn create_edge<P>(self, v: &[Vertex<P>]) -> Result<Edge<P, C>> {
        let front = v
            .get(self.vertices.0)
            .ok_or(Error::InvalidCompressedTopologyIndex {
                entity: "vertex",
                index: self.vertices.0,
                len: v.len(),
            })?;
        let back = v
            .get(self.vertices.1)
            .ok_or(Error::InvalidCompressedTopologyIndex {
                entity: "vertex",
                index: self.vertices.1,
                len: v.len(),
            })?;
        Edge::try_new(front, back, self.curve)
    }
}

/// The index of an edge in `CompressedShell`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CompressedEdgeIndex {
    /// the index of the edge
    pub index: usize,
    /// the orientation of the edge
    pub orientation: bool,
}

impl From<(usize, bool)> for CompressedEdgeIndex {
    fn from((index, orientation): (usize, bool)) -> Self { Self { index, orientation } }
}

/// Face-local use of an edge in a compressed boundary.
///
/// This is the serialized equivalent of a coedge or edge-use: it refers to a shared
/// edge by `index`, stores the local `orientation`, and may also carry an exact trim
/// curve for this face-use.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompressedEdgeUse<T> {
    /// The index of the shared edge in the parent `CompressedShell`.
    pub index: usize,
    /// The local orientation of the edge-use.
    pub orientation: bool,
    /// The exact face-local trim carried by this edge-use, if available.
    pub trim_curve: Option<T>,
}

impl<T> From<CompressedEdgeIndex> for CompressedEdgeUse<T> {
    fn from(CompressedEdgeIndex { index, orientation }: CompressedEdgeIndex) -> Self {
        Self {
            index,
            orientation,
            trim_curve: None,
        }
    }
}

impl<T> From<(usize, bool, Option<T>)> for CompressedEdgeUse<T> {
    fn from((index, orientation, trim_curve): (usize, bool, Option<T>)) -> Self {
        Self {
            index,
            orientation,
            trim_curve,
        }
    }
}

/// Serialized compressed face
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompressedFace<S> {
    /// Boundaries of the face
    pub boundaries: Vec<Vec<CompressedEdgeIndex>>,
    /// orientation of the face
    pub orientation: bool,
    /// surface geometry of the face
    pub surface: S,
}

/// Serialized compressed face with exact face-local trim storage.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompressedTrimmedFace<S, T> {
    /// Boundaries of the face as oriented edge-uses.
    pub boundaries: Vec<Vec<CompressedEdgeUse<T>>>,
    /// orientation of the face
    pub orientation: bool,
    /// surface geometry of the face
    pub surface: S,
}

impl<S> CompressedFace<S> {
    fn create_face<P, C>(self, edges: &[Edge<P, C>]) -> Result<Face<P, C, S>> {
        let wires = self
            .boundaries
            .into_iter()
            .map(|wire| {
                wire.into_iter()
                    .map(|CompressedEdgeIndex { index, orientation }| {
                        let edge =
                            edges
                                .get(index)
                                .ok_or(Error::InvalidCompressedTopologyIndex {
                                    entity: "edge",
                                    index,
                                    len: edges.len(),
                                })?;
                        Ok(match orientation {
                            true => edge.clone(),
                            false => edge.inverse(),
                        })
                    })
                    .collect::<Result<Wire<P, C>>>()
            })
            .collect::<Result<Vec<_>>>()?;
        let mut face = Face::try_new(wires, self.surface)?;
        if !self.orientation {
            face.invert();
        }
        Ok(face)
    }
}

/// Serialized compressed shell
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompressedShell<P, C, S> {
    /// all geometries of vertices
    pub vertices: Vec<P>,
    /// all geometries and end vertices of edges
    pub edges: Vec<CompressedEdge<C>>,
    /// all geometries and boundaries of faces
    pub faces: Vec<CompressedFace<S>>,
    /// Stable IDs for vertices (same order as `vertices`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertex_stable_ids: Option<Vec<StableId>>,
    /// Stable IDs for edges (same order as `edges`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edge_stable_ids: Option<Vec<StableId>>,
    /// Stable IDs for faces (same order as `faces`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub face_stable_ids: Option<Vec<StableId>>,
}

/// Tracking metadata parallel to a [`CompressedShell`]'s topology arrays.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompressedTopologyTracking {
    /// Tracking IDs for vertices (same order as `vertices`).
    pub vertices: Vec<Option<TrackingId>>,
    /// Tracking IDs for edges (same order as `edges`).
    pub edges: Vec<Option<TrackingId>>,
    /// Tracking IDs for faces (same order as `faces`).
    pub faces: Vec<Option<TrackingId>>,
}

/// A compressed shell accompanied by immutable topology tracking metadata.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrackedCompressedShell<P, C, S> {
    /// Compressed topology and geometry.
    pub topology: CompressedShell<P, C, S>,
    /// Tracking metadata parallel to the topology arrays.
    pub tracking: CompressedTopologyTracking,
}

/// Serialized compressed shell with exact face-local trim storage.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompressedTrimmedShell<P, C, S, T> {
    /// all geometries of vertices
    pub vertices: Vec<P>,
    /// all geometries and end vertices of edges
    pub edges: Vec<CompressedEdge<C>>,
    /// all geometries and boundaries of faces, including optional face-local trims
    pub faces: Vec<CompressedTrimmedFace<S, T>>,
}

/// Serialized compressed solid
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompressedSolid<P, C, S> {
    /// all boundaries of solid
    pub boundaries: Vec<CompressedShell<P, C, S>>,
    /// The stable ID allocator state, preserved across serialization.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id_allocator: Option<StableIdAllocator>,
    /// Per-element attributes, preserved across serialization.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attributes: Option<SolidAttributes>,
}

/// A compressed solid accompanied by per-boundary tracking metadata.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrackedCompressedSolid<P, C, S> {
    /// Compressed topology and geometry.
    pub topology: CompressedSolid<P, C, S>,
    /// Tracking metadata parallel to `topology.boundaries`.
    pub tracking: Vec<CompressedTopologyTracking>,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum ShellSerialization<P, C, S> {
    Tracked(TrackedCompressedShell<P, C, S>),
    Legacy(CompressedShell<P, C, S>),
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum SolidSerialization<P, C, S> {
    Tracked(TrackedCompressedSolid<P, C, S>),
    Legacy(CompressedSolid<P, C, S>),
}

/// Serialized compressed solid with exact face-local trim storage.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompressedTrimmedSolid<P, C, S, T> {
    /// all boundaries of solid
    pub boundaries: Vec<CompressedTrimmedShell<P, C, S, T>>,
}

struct CompressDirector<P, C> {
    vmap: HashMap<VertexId<P>, (usize, P)>,
    emap: HashMap<EdgeId<C>, (usize, CompressedEdge<C>)>,
    vertex_stable_ids: HashMap<VertexId<P>, (usize, StableId)>,
    edge_stable_ids: HashMap<EdgeId<C>, (usize, StableId)>,
    vertex_tracking_ids: HashMap<VertexId<P>, (usize, Option<TrackingId>)>,
    edge_tracking_ids: HashMap<EdgeId<C>, (usize, Option<TrackingId>)>,
}

type PersistentTopologyParts<P, C> = (
    Vec<P>,
    Vec<CompressedEdge<C>>,
    Vec<StableId>,
    Vec<StableId>,
    Vec<Option<TrackingId>>,
    Vec<Option<TrackingId>>,
);

impl<P: Clone, C: Clone> CompressDirector<P, C> {
    #[inline(always)]
    fn new() -> Self {
        Self {
            vmap: HashMap::default(),
            emap: HashMap::default(),
            vertex_stable_ids: HashMap::default(),
            edge_stable_ids: HashMap::default(),
            vertex_tracking_ids: HashMap::default(),
            edge_tracking_ids: HashMap::default(),
        }
    }
    #[inline(always)]
    fn get_vid(&mut self, vertex: &Vertex<P>) -> usize {
        let id = self.vmap.len();
        let vid = vertex.id();
        self.vertex_stable_ids
            .entry(vid)
            .or_insert_with(|| (id, vertex.stable_id()));
        self.vertex_tracking_ids
            .entry(vid)
            .or_insert_with(|| (id, vertex.tracking_id().cloned()));
        self.vmap
            .entry(vid)
            .or_insert_with(|| (id, vertex.point()))
            .0
    }

    #[inline(always)]
    fn get_eid(&mut self, edge: &Edge<P, C>) -> CompressedEdgeIndex {
        match self.emap.get(&edge.id()) {
            Some(got) => (got.0, edge.orientation()).into(),
            None => {
                let id = self.emap.len();
                let front_id = self.get_vid(edge.absolute_front());
                let back_id = self.get_vid(edge.absolute_back());
                let curve = edge.curve();
                let cedge = CompressedEdge {
                    vertices: (front_id, back_id),
                    curve,
                };
                self.edge_stable_ids
                    .insert(edge.id(), (id, edge.stable_id()));
                self.edge_tracking_ids
                    .insert(edge.id(), (id, edge.tracking_id().cloned()));
                self.emap.insert(edge.id(), (id, cedge));
                (id, edge.orientation()).into()
            }
        }
    }

    #[inline(always)]
    fn create_boundary(&mut self, boundary: &Wire<P, C>) -> Vec<CompressedEdgeIndex> {
        boundary.iter().map(|edge| self.get_eid(edge)).collect()
    }

    #[inline(always)]
    fn create_trimmed_boundary<S, T, F>(
        &mut self,
        boundary: &Wire<P, C>,
        surface: &S,
        trim_curve: &mut F,
    ) -> Vec<CompressedEdgeUse<T>>
    where
        F: FnMut(&Edge<P, C>, &S) -> Option<T>,
    {
        boundary
            .iter()
            .map(|edge| {
                let CompressedEdgeIndex { index, orientation } = self.get_eid(edge);
                CompressedEdgeUse {
                    index,
                    orientation,
                    trim_curve: trim_curve(edge, surface),
                }
            })
            .collect()
    }

    #[inline(always)]
    fn create_cface<S: Clone>(&mut self, face: &Face<P, C, S>) -> CompressedFace<S> {
        CompressedFace {
            boundaries: face
                .boundaries
                .iter()
                .map(|wire| self.create_boundary(wire))
                .collect(),
            orientation: face.orientation(),
            surface: face.surface(),
        }
    }

    #[inline(always)]
    fn create_trimmed_cface<S: Clone, T, F>(
        &mut self,
        face: &Face<P, C, S>,
        trim_curve: &mut F,
    ) -> CompressedTrimmedFace<S, T>
    where
        F: FnMut(&Edge<P, C>, &S) -> Option<T>,
    {
        let surface = face.surface();
        CompressedTrimmedFace {
            boundaries: face
                .boundaries
                .iter()
                .map(|wire| self.create_trimmed_boundary(wire, &surface, trim_curve))
                .collect(),
            orientation: face.orientation(),
            surface,
        }
    }

    #[inline(always)]
    fn map2vec<K, T>(map: HashMap<K, (usize, T)>) -> Vec<T> {
        let mut vec: Vec<_> = map.into_iter().map(|entry| entry.1).collect();
        vec.sort_by_key(|x| x.0);
        vec.into_iter().map(|x| x.1).collect()
    }

    #[inline(always)]
    fn vertices_edges(self) -> (Vec<P>, Vec<CompressedEdge<C>>) {
        (Self::map2vec(self.vmap), Self::map2vec(self.emap))
    }

    #[inline(always)]
    fn persistent_ids(self) -> PersistentTopologyParts<P, C> {
        let vertices = Self::map2vec(self.vmap);
        let edges = Self::map2vec(self.emap);
        let vsids = Self::map2vec_simple(self.vertex_stable_ids);
        let esids = Self::map2vec_simple(self.edge_stable_ids);
        let vtids = Self::map2vec_simple(self.vertex_tracking_ids);
        let etids = Self::map2vec_simple(self.edge_tracking_ids);
        (vertices, edges, vsids, esids, vtids, etids)
    }

    #[inline(always)]
    fn map2vec_simple<K, T>(map: HashMap<K, (usize, T)>) -> Vec<T> {
        let mut vec: Vec<_> = map.into_iter().map(|entry| entry.1).collect();
        vec.sort_by_key(|x| x.0);
        vec.into_iter().map(|x| x.1).collect()
    }
}

impl<P: Clone, C: Clone, S: Clone> Shell<P, C, S> {
    fn compressed_with_tracking_parts(
        &self,
    ) -> (CompressedShell<P, C, S>, CompressedTopologyTracking) {
        let mut director = CompressDirector::new();
        let face_stable_ids: Vec<StableId> = self.iter().map(|face| face.stable_id()).collect();
        let face_tracking_ids = self
            .iter()
            .map(|face| face.tracking_id().cloned())
            .collect();
        let mut face_closure = |face: &Face<P, C, S>| director.create_cface(face);
        let faces: Vec<_> = self.iter().map(&mut face_closure).collect();
        let (vertices, edges, vsids, esids, vtids, etids) = director.persistent_ids();
        let has_any_stable_id = vsids.iter().any(|id| id.is_assigned())
            || esids.iter().any(|id| id.is_assigned())
            || face_stable_ids.iter().any(|id| id.is_assigned());
        (
            CompressedShell {
                vertices,
                edges,
                faces,
                vertex_stable_ids: if has_any_stable_id { Some(vsids) } else { None },
                edge_stable_ids: if has_any_stable_id { Some(esids) } else { None },
                face_stable_ids: if has_any_stable_id {
                    Some(face_stable_ids)
                } else {
                    None
                },
            },
            CompressedTopologyTracking {
                vertices: vtids,
                edges: etids,
                faces: face_tracking_ids,
            },
        )
    }

    /// Compresses the shell into the serialized compressed shell.
    pub fn compress(&self) -> CompressedShell<P, C, S> { self.compressed_with_tracking_parts().0 }

    /// Compresses this shell with immutable topology tracking metadata.
    pub fn compress_tracked(&self) -> TrackedCompressedShell<P, C, S> {
        let (topology, tracking) = self.compressed_with_tracking_parts();
        TrackedCompressedShell { topology, tracking }
    }

    /// Compresses the shell into serialized compressed topology with face-local trims.
    pub fn compress_with_face_trims<T, F>(
        &self,
        mut trim_curve: F,
    ) -> CompressedTrimmedShell<P, C, S, T>
    where
        F: FnMut(&Edge<P, C>, &S) -> Option<T>,
    {
        let mut director = CompressDirector::new();
        let mut face_closure =
            |face: &Face<P, C, S>| director.create_trimmed_cface(face, &mut trim_curve);
        let faces = self.iter().map(&mut face_closure).collect();
        let (vertices, edges) = director.vertices_edges();
        CompressedTrimmedShell {
            vertices,
            edges,
            faces,
        }
    }

    /// Compresses the shell while preserving exact face-local trims carried by the edge curves.
    pub fn compress_with_exact_face_trims(
        &self,
    ) -> CompressedTrimmedShell<P, C, S, <C as ExactParameterBoundary2D<S>>::BoundaryCurve>
    where C: ExactParameterBoundary2D<S> {
        self.compress_with_face_trims(|edge, surface| {
            edge.with_curve(|curve| curve.exact_parameter_boundary_2d(surface))
        })
    }
}

impl<P, C, S> Shell<P, C, S> {
    /// Extracts the serialized compressed shell into the shell.
    ///
    /// # Errors
    /// Returns [`Error::NotSimpleWire`] if any boundary wire has repeated vertices.
    /// STEP files from other CAD systems often produce such wires. To handle this,
    /// apply `SplitClosedEdgesAndFaces` or `RobustSplitClosedEdgesAndFaces`
    /// (from `monstertruck-solid`) to the `CompressedShell` before calling `extract`.
    /// The convenience function `monstertruck_solid::extract_healed` does both steps.
    pub fn extract(cshell: CompressedShell<P, C, S>) -> Result<Self> {
        Self::extract_with_tracking(cshell, None)
    }

    fn extract_with_tracking(
        cshell: CompressedShell<P, C, S>,
        tracking: Option<CompressedTopologyTracking>,
    ) -> Result<Self> {
        let CompressedShell {
            vertices,
            edges,
            faces,
            vertex_stable_ids,
            edge_stable_ids,
            face_stable_ids,
        } = cshell;
        let (vertex_tracking_ids, edge_tracking_ids, face_tracking_ids) = match tracking {
            Some(tracking) => (
                Some(tracking.vertices),
                Some(tracking.edges),
                Some(tracking.faces),
            ),
            None => (None, None, None),
        };
        let mut vertices: Vec<_> = vertices.into_iter().map(Vertex::new).collect();
        if let Some(vsids) = vertex_stable_ids {
            vertices
                .iter_mut()
                .zip(vsids)
                .for_each(|(vertex, stable_id)| vertex.set_stable_id(stable_id));
        }
        if let Some(tracking_ids) = vertex_tracking_ids {
            vertices
                .iter_mut()
                .zip(tracking_ids)
                .for_each(|(vertex, tracking_id)| vertex.set_tracking_id(tracking_id));
        }
        let mut edges = edges
            .into_iter()
            .map(move |edge| edge.create_edge(&vertices))
            .collect::<Result<Vec<_>>>()?;
        if let Some(esids) = edge_stable_ids {
            edges
                .iter_mut()
                .zip(esids)
                .for_each(|(edge, stable_id)| edge.set_stable_id(stable_id));
        }
        if let Some(tracking_ids) = edge_tracking_ids {
            edges
                .iter_mut()
                .zip(tracking_ids)
                .for_each(|(edge, tracking_id)| edge.set_tracking_id(tracking_id));
        }
        let mut faces = faces
            .into_iter()
            .map(move |face| face.create_face(&edges))
            .collect::<Result<Vec<_>>>()?;
        if let Some(fsids) = face_stable_ids {
            faces
                .iter_mut()
                .zip(fsids)
                .for_each(|(face, stable_id)| face.set_stable_id(stable_id));
        }
        if let Some(tracking_ids) = face_tracking_ids {
            faces
                .iter_mut()
                .zip(tracking_ids)
                .for_each(|(face, tracking_id)| face.set_tracking_id(tracking_id));
        }
        Ok(Shell::from(faces))
    }

    /// Extracts compressed topology and restores immutable tracking metadata.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidTrackingMetadata`] when a tracking array does
    /// not match its corresponding compressed topology array.
    pub fn extract_tracked(tracked: TrackedCompressedShell<P, C, S>) -> Result<Self> {
        let TrackedCompressedShell { topology, tracking } = tracked;
        if tracking.vertices.len() != topology.vertices.len()
            || tracking.edges.len() != topology.edges.len()
            || tracking.faces.len() != topology.faces.len()
        {
            return Err(Error::InvalidTrackingMetadata);
        }
        Self::extract_with_tracking(topology, Some(tracking))
    }
}

impl<S, T> CompressedTrimmedFace<S, T> {
    /// Drops face-local trim data and returns a plain `CompressedFace`.
    pub fn erase_trims(self) -> CompressedFace<S> {
        CompressedFace {
            boundaries: self
                .boundaries
                .into_iter()
                .map(|wire| {
                    wire.into_iter()
                        .map(
                            |CompressedEdgeUse {
                                 index, orientation, ..
                             }| {
                                CompressedEdgeIndex { index, orientation }
                            },
                        )
                        .collect()
                })
                .collect(),
            orientation: self.orientation,
            surface: self.surface,
        }
    }

    /// Clones into a plain `CompressedFace` without cloning trim data.
    pub fn cloned_without_trims(&self) -> CompressedFace<S>
    where S: Clone {
        CompressedFace {
            boundaries: self
                .boundaries
                .iter()
                .map(|wire| {
                    wire.iter()
                        .map(
                            |CompressedEdgeUse {
                                 index, orientation, ..
                             }| CompressedEdgeIndex {
                                index: *index,
                                orientation: *orientation,
                            },
                        )
                        .collect()
                })
                .collect(),
            orientation: self.orientation,
            surface: self.surface.clone(),
        }
    }
}

impl<P, C, S, T> CompressedTrimmedShell<P, C, S, T> {
    /// Drops all face-local trim data and returns a plain `CompressedShell`.
    pub fn erase_trims(self) -> CompressedShell<P, C, S> {
        CompressedShell {
            vertices: self.vertices,
            edges: self.edges,
            faces: self
                .faces
                .into_iter()
                .map(CompressedTrimmedFace::erase_trims)
                .collect(),
            vertex_stable_ids: None,
            edge_stable_ids: None,
            face_stable_ids: None,
        }
    }

    /// Clones into a plain `CompressedShell` without cloning trim data.
    pub fn cloned_without_trims(&self) -> CompressedShell<P, C, S>
    where
        P: Clone,
        C: Clone,
        S: Clone, {
        CompressedShell {
            vertices: self.vertices.clone(),
            edges: self.edges.clone(),
            faces: self
                .faces
                .iter()
                .map(CompressedTrimmedFace::cloned_without_trims)
                .collect(),
            vertex_stable_ids: None,
            edge_stable_ids: None,
            face_stable_ids: None,
        }
    }
}

impl<P, C, S, T> CompressedTrimmedSolid<P, C, S, T> {
    /// Drops all face-local trim data and returns a plain `CompressedSolid`.
    pub fn erase_trims(self) -> CompressedSolid<P, C, S> {
        CompressedSolid {
            boundaries: self
                .boundaries
                .into_iter()
                .map(CompressedTrimmedShell::erase_trims)
                .collect(),
            id_allocator: None,
            attributes: None,
        }
    }

    /// Clones into a plain `CompressedSolid` without cloning trim data.
    pub fn cloned_without_trims(&self) -> CompressedSolid<P, C, S>
    where
        P: Clone,
        C: Clone,
        S: Clone, {
        CompressedSolid {
            boundaries: self
                .boundaries
                .iter()
                .map(CompressedTrimmedShell::cloned_without_trims)
                .collect(),
            id_allocator: None,
            attributes: None,
        }
    }
}

impl<P: Clone, C: Clone, S: Clone> CompressedShell<P, C, S> {
    /// Maps the shared edge curves into a new [`CompressedShell`].
    pub fn map_curves<D, F>(&self, mut map_curve: F) -> CompressedShell<P, D, S>
    where F: FnMut(&C) -> D {
        CompressedShell {
            vertices: self.vertices.clone(),
            edges: self
                .edges
                .iter()
                .map(|edge| CompressedEdge {
                    vertices: edge.vertices,
                    curve: map_curve(&edge.curve),
                })
                .collect(),
            faces: self.faces.clone(),
            vertex_stable_ids: self.vertex_stable_ids.clone(),
            edge_stable_ids: self.edge_stable_ids.clone(),
            face_stable_ids: self.face_stable_ids.clone(),
        }
    }

    /// Attaches face-local trims to each edge-use in this compressed shell.
    pub fn with_face_trims<T, F>(&self, mut trim_curve: F) -> CompressedTrimmedShell<P, C, S, T>
    where F: FnMut(&C, &S) -> Option<T> {
        CompressedTrimmedShell {
            vertices: self.vertices.clone(),
            edges: self.edges.clone(),
            faces: self
                .faces
                .iter()
                .map(|face| CompressedTrimmedFace {
                    boundaries: face
                        .boundaries
                        .iter()
                        .map(|wire| {
                            wire.iter()
                                .map(|CompressedEdgeIndex { index, orientation }| {
                                    CompressedEdgeUse {
                                        index: *index,
                                        orientation: *orientation,
                                        trim_curve: trim_curve(
                                            &self.edges[*index].curve,
                                            &face.surface,
                                        ),
                                    }
                                })
                                .collect()
                        })
                        .collect(),
                    orientation: face.orientation,
                    surface: face.surface.clone(),
                })
                .collect(),
        }
    }

    /// Attaches exact face-local trims to each edge-use in this compressed shell.
    pub fn with_exact_face_trims(
        &self,
    ) -> CompressedTrimmedShell<P, C, S, <C as ExactParameterBoundary2D<S>>::BoundaryCurve>
    where C: ExactParameterBoundary2D<S> {
        self.with_face_trims(|curve, surface| curve.exact_parameter_boundary_2d(surface))
    }
}

impl<P: Clone, C: Clone, S: Clone> CompressedSolid<P, C, S> {
    /// Maps the shared edge curves into a new [`CompressedSolid`].
    pub fn map_curves<D, F>(&self, mut map_curve: F) -> CompressedSolid<P, D, S>
    where F: FnMut(&C) -> D {
        CompressedSolid {
            boundaries: self
                .boundaries
                .iter()
                .map(|shell| shell.map_curves(&mut map_curve))
                .collect(),
            id_allocator: self.id_allocator.clone(),
            attributes: self.attributes.clone(),
        }
    }

    /// Attaches face-local trims to each edge-use in this compressed solid.
    pub fn with_face_trims<T, F>(&self, mut trim_curve: F) -> CompressedTrimmedSolid<P, C, S, T>
    where F: FnMut(&C, &S) -> Option<T> {
        CompressedTrimmedSolid {
            boundaries: self
                .boundaries
                .iter()
                .map(|shell| shell.with_face_trims(&mut trim_curve))
                .collect(),
        }
    }

    /// Attaches exact face-local trims to each edge-use in this compressed solid.
    pub fn with_exact_face_trims(
        &self,
    ) -> CompressedTrimmedSolid<P, C, S, <C as ExactParameterBoundary2D<S>>::BoundaryCurve>
    where C: ExactParameterBoundary2D<S> {
        self.with_face_trims(|curve, surface| curve.exact_parameter_boundary_2d(surface))
    }
}

impl<P: Clone, C: Clone, S: Clone> Solid<P, C, S> {
    /// Compresses the solid into the serialized compressed solid.
    pub fn compress(&self) -> CompressedSolid<P, C, S> {
        CompressedSolid {
            boundaries: self
                .boundaries()
                .iter()
                .map(|shell| shell.compress())
                .collect(),
            id_allocator: Some(self.id_allocator.clone()),
            attributes: if self.attributes.is_empty() {
                None
            } else {
                Some(self.attributes.clone())
            },
        }
    }

    /// Compresses this solid with immutable topology tracking metadata.
    pub fn compress_tracked(&self) -> TrackedCompressedSolid<P, C, S> {
        let tracked_boundaries: Vec<_> = self
            .boundaries()
            .iter()
            .map(Shell::compress_tracked)
            .collect();
        TrackedCompressedSolid {
            topology: CompressedSolid {
                boundaries: tracked_boundaries
                    .iter()
                    .map(|boundary| boundary.topology.clone())
                    .collect(),
                id_allocator: Some(self.id_allocator.clone()),
                attributes: if self.attributes.is_empty() {
                    None
                } else {
                    Some(self.attributes.clone())
                },
            },
            tracking: tracked_boundaries
                .into_iter()
                .map(|boundary| boundary.tracking)
                .collect(),
        }
    }

    /// Compresses the solid into serialized compressed topology with face-local trims.
    pub fn compress_with_face_trims<T, F>(
        &self,
        mut trim_curve: F,
    ) -> CompressedTrimmedSolid<P, C, S, T>
    where
        F: FnMut(&Edge<P, C>, &S) -> Option<T>,
    {
        CompressedTrimmedSolid {
            boundaries: self
                .boundaries()
                .iter()
                .map(|shell| shell.compress_with_face_trims(&mut trim_curve))
                .collect(),
        }
    }

    /// Compresses the solid while preserving exact face-local trims carried by the edge curves.
    pub fn compress_with_exact_face_trims(
        &self,
    ) -> CompressedTrimmedSolid<P, C, S, <C as ExactParameterBoundary2D<S>>::BoundaryCurve>
    where C: ExactParameterBoundary2D<S> {
        self.compress_with_face_trims(|edge, surface| {
            edge.curve().exact_parameter_boundary_2d(surface)
        })
    }

    /// Extracts the serialized compressed solid into the solid.
    pub fn extract(csolid: CompressedSolid<P, C, S>) -> Result<Self> {
        let id_allocator = csolid.id_allocator.unwrap_or_default();
        let attributes = csolid.attributes.unwrap_or_default();
        let shells: Result<Vec<Shell<P, C, S>>> =
            csolid.boundaries.into_iter().map(Shell::extract).collect();
        let mut solid = Solid::try_new(shells?)?;
        solid.id_allocator = id_allocator;
        solid.attributes = attributes;
        Ok(solid)
    }

    /// Extracts a compressed solid and restores immutable tracking metadata.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidTrackingMetadata`] when the tracking shell
    /// count or any nested tracking array does not match the topology.
    pub fn extract_tracked(tracked: TrackedCompressedSolid<P, C, S>) -> Result<Self> {
        let TrackedCompressedSolid { topology, tracking } = tracked;
        if tracking.len() != topology.boundaries.len() {
            return Err(Error::InvalidTrackingMetadata);
        }
        let id_allocator = topology.id_allocator.unwrap_or_default();
        let attributes = topology.attributes.unwrap_or_default();
        let shells = topology
            .boundaries
            .into_iter()
            .zip(tracking)
            .map(|(topology, tracking)| {
                Shell::extract_tracked(TrackedCompressedShell { topology, tracking })
            })
            .collect::<Result<Vec<_>>>()?;
        let mut solid = Solid::try_new(shells)?;
        solid.id_allocator = id_allocator;
        solid.attributes = attributes;
        Ok(solid)
    }
}

// ---------------------------------------------------------------------------
// Deterministic topology hashing.
// ---------------------------------------------------------------------------

/// Options controlling which data is included in a [`Solid`] hash.
#[derive(Clone, Copy, Debug)]
pub struct SolidHashOptions {
    /// Include analytic geometry content (point, curve, surface data).
    pub include_geometry: bool,
    /// Include sparse per-element attributes.
    pub include_attributes: bool,
    /// Include stable IDs in the hash.
    pub include_stable_ids: bool,
}

impl Default for SolidHashOptions {
    fn default() -> Self {
        Self {
            include_geometry: false,
            include_attributes: false,
            include_stable_ids: true,
        }
    }
}

/// Hash the structural connectivity of a compressed shell, ignoring geometry.
fn hash_shell_topology<P, C, S>(shell: &CompressedShell<P, C, S>, hasher: &mut ContentHasher) {
    // Vertex count.
    hasher.write_usize(shell.vertices.len());
    // Edge connectivity (vertex indices only).
    hasher.write_usize(shell.edges.len());
    shell.edges.iter().for_each(|e| {
        e.vertices.0.content_hash(hasher);
        e.vertices.1.content_hash(hasher);
    });
    // Face boundary structure.
    hasher.write_usize(shell.faces.len());
    shell.faces.iter().for_each(|f| {
        f.orientation.content_hash(hasher);
        hasher.write_usize(f.boundaries.len());
        f.boundaries.iter().for_each(|wire| {
            hasher.write_usize(wire.len());
            wire.iter().for_each(|ei| {
                ei.index.content_hash(hasher);
                ei.orientation.content_hash(hasher);
            });
        });
    });
}

/// Hash the stable IDs carried by a compressed shell.
fn hash_shell_stable_ids<P, C, S>(shell: &CompressedShell<P, C, S>, hasher: &mut ContentHasher) {
    shell.vertex_stable_ids.content_hash(hasher);
    shell.edge_stable_ids.content_hash(hasher);
    shell.face_stable_ids.content_hash(hasher);
}

/// Hash the geometry content of a compressed shell.
fn hash_shell_geometry<P, C, S>(shell: &CompressedShell<P, C, S>, hasher: &mut ContentHasher)
where
    P: DeterministicContentHash,
    C: DeterministicContentHash,
    S: DeterministicContentHash, {
    shell.vertices.content_hash(hasher);
    shell
        .edges
        .iter()
        .for_each(|e| e.curve.content_hash(hasher));
    shell
        .faces
        .iter()
        .for_each(|f| f.surface.content_hash(hasher));
}

impl<P: Clone, C: Clone, S: Clone> Solid<P, C, S> {
    /// Compute a deterministic hash of the topology structure and stable IDs.
    ///
    /// This hash is independent of pointer identity, attribute edits, and
    /// geometry content. Two solids with the same connectivity and stable IDs
    /// produce the same hash.
    pub fn topology_hash(&self) -> u64 {
        let compressed = self.compress();
        let mut hasher = ContentHasher::new();
        hasher.write_usize(compressed.boundaries.len());
        compressed.boundaries.iter().for_each(|shell| {
            hash_shell_topology(shell, &mut hasher);
            hash_shell_stable_ids(shell, &mut hasher);
        });
        // Include allocator state (semantically relevant).
        if let Some(ref alloc) = compressed.id_allocator {
            hasher.write_u8(1);
            alloc.peek().content_hash(&mut hasher);
        } else {
            hasher.write_u8(0);
        }
        hasher.finish()
    }

    /// Compute a deterministic hash of topology, stable IDs, and sparse attributes.
    ///
    /// Unlike [`topology_hash`](Self::topology_hash), this changes when per-element
    /// attributes (e.g. selection stamps) are modified.
    pub fn topology_attribute_hash(&self) -> u64 {
        let compressed = self.compress();
        let mut hasher = ContentHasher::new();
        hasher.write_usize(compressed.boundaries.len());
        compressed.boundaries.iter().for_each(|shell| {
            hash_shell_topology(shell, &mut hasher);
            hash_shell_stable_ids(shell, &mut hasher);
        });
        if let Some(ref alloc) = compressed.id_allocator {
            hasher.write_u8(1);
            alloc.peek().content_hash(&mut hasher);
        } else {
            hasher.write_u8(0);
        }
        // Include attributes.
        self.attributes.content_hash(&mut hasher);
        hasher.finish()
    }
}

impl<P, C, S> Solid<P, C, S>
where
    P: Clone + DeterministicContentHash,
    C: Clone + DeterministicContentHash,
    S: Clone + DeterministicContentHash,
{
    /// Compute a full deterministic content hash.
    ///
    /// Includes canonical topology, stable IDs, sparse attributes, and
    /// analytic geometry. Remains deterministic across runs and independent
    /// of pointer identity.
    pub fn content_hash(&self) -> u64 {
        let compressed = self.compress();
        let mut hasher = ContentHasher::new();
        hasher.write_usize(compressed.boundaries.len());
        compressed.boundaries.iter().for_each(|shell| {
            hash_shell_topology(shell, &mut hasher);
            hash_shell_stable_ids(shell, &mut hasher);
            hash_shell_geometry(shell, &mut hasher);
        });
        if let Some(ref alloc) = compressed.id_allocator {
            hasher.write_u8(1);
            alloc.peek().content_hash(&mut hasher);
        } else {
            hasher.write_u8(0);
        }
        self.attributes.content_hash(&mut hasher);
        hasher.finish()
    }
}

// -------------------------- test --------------------------

#[test]
fn compress_extract() {
    let cube = solid::cube();
    let shell0 = &cube.boundaries()[0];
    let shell1 = Shell::extract(shell0.compress()).unwrap();
    assert!(same_topology(shell0, &shell1));
}

#[allow(dead_code)]
fn vmap_subroutin<P, Q>(
    v0: &Vertex<P>,
    v1: &Vertex<Q>,
    vmap: &mut HashMap<VertexId<P>, VertexId<Q>>,
) -> bool {
    match vmap.get(&v0.id()) {
        Some(got) => *got == v1.id(),
        None => {
            vmap.insert(v0.id(), v1.id());
            true
        }
    }
}

#[allow(dead_code)]
fn emap_subroutin<P, Q, C, D>(
    edge0: &Edge<P, C>,
    edge1: &Edge<Q, D>,
    vmap: &mut HashMap<VertexId<P>, VertexId<Q>>,
    emap: &mut HashMap<EdgeId<C>, EdgeId<D>>,
) -> bool {
    match emap.get(&edge0.id()) {
        Some(got) => *got == edge1.id(),
        None => {
            emap.insert(edge0.id(), edge1.id());
            vmap_subroutin(edge0.front(), edge1.front(), vmap)
                && vmap_subroutin(edge0.back(), edge1.back(), vmap)
        }
    }
}

#[allow(dead_code)]
fn same_topology<P, C, S, Q, D, T>(one: &Shell<P, C, S>, other: &Shell<Q, D, T>) -> bool {
    let mut vmap = HashMap::<VertexId<P>, VertexId<Q>>::default();
    let mut emap = HashMap::<EdgeId<C>, EdgeId<D>>::default();
    if one.len() != other.len() {
        return false;
    }
    for (face0, face1) in one.iter().zip(other.iter()) {
        let biters0 = face0.boundary_iters();
        let biters1 = face1.boundary_iters();
        if biters0.len() != biters1.len() {
            return false;
        }
        for (biter0, biter1) in biters0.into_iter().zip(biters1) {
            if biter0.len() != biter1.len() {
                return false;
            }
            for (edge0, edge1) in biter0.zip(biter1) {
                if !emap_subroutin(&edge0, &edge1, &mut vmap, &mut emap) {
                    return false;
                }
            }
        }
    }
    true
}

#[test]
fn stable_ids_survive_compress_extract() {
    // Build a cube, take it apart, assign stable IDs to faces, rebuild.
    let cube = solid::cube();
    let mut shells = cube.into_boundaries();
    let shell = &mut shells[0];
    let mut alloc = StableIdAllocator::new();
    for face in shell.face_iter_mut() {
        face.set_stable_id(alloc.allocate());
    }

    let orig_sids: Vec<_> = shell.face_iter().map(|f| f.stable_id()).collect();
    let compressed = shell.compress();

    // All face stable IDs should appear in the compressed form.
    let csids = compressed.face_stable_ids.as_ref().unwrap();
    assert_eq!(&orig_sids, csids);

    // Extract and verify.
    let extracted_shell = Shell::extract(compressed).unwrap();
    let ext_sids: Vec<_> = extracted_shell.face_iter().map(|f| f.stable_id()).collect();
    assert_eq!(orig_sids, ext_sids);
}

#[test]
fn attributes_survive_compress_extract() {
    use crate::attributes::AttributeValue;
    let cube = solid::cube();
    let mut alloc = StableIdAllocator::new();
    let face_id = alloc.allocate();

    // Build compressed solid manually to set attributes.
    let mut compressed = cube.compress();
    let mut attrs = SolidAttributes::new();
    attrs.faces.set(
        "color",
        face_id,
        AttributeValue::Color([1.0, 0.0, 0.0, 1.0]),
    );
    compressed.attributes = Some(attrs);
    compressed.id_allocator = Some(alloc);

    let extracted = Solid::extract(compressed).unwrap();
    assert_eq!(
        extracted.face_attributes().get("color", face_id),
        Some(&AttributeValue::Color([1.0, 0.0, 0.0, 1.0]))
    );
}

#[test]
fn id_allocator_survives_compress_extract() {
    let mut cube = solid::cube();
    // Advance the allocator.
    let _a = cube.alloc_id();
    let _b = cube.alloc_id();
    let peek_before = cube.id_allocator().peek();

    let compressed = cube.compress();
    let extracted = Solid::extract(compressed).unwrap();
    assert_eq!(extracted.id_allocator().peek(), peek_before);
}

impl<P, C, S> Serialize for Shell<P, C, S>
where
    P: Clone + Serialize,
    C: Clone + Serialize,
    S: Clone + Serialize,
{
    fn serialize<Serializer>(
        &self,
        serializer: Serializer,
    ) -> std::result::Result<Serializer::Ok, Serializer::Error>
    where
        Serializer: serde::Serializer,
    {
        if self.tracking_ids().is_empty() {
            ShellSerialization::Legacy(self.compress()).serialize(serializer)
        } else {
            ShellSerialization::Tracked(self.compress_tracked()).serialize(serializer)
        }
    }
}

impl<'de, P, C, S> Deserialize<'de> for Shell<P, C, S>
where
    P: Clone + Deserialize<'de>,
    C: Clone + Deserialize<'de>,
    S: Clone + Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where D: serde::Deserializer<'de> {
        use serde::de::Error;
        match ShellSerialization::<P, C, S>::deserialize(deserializer)? {
            ShellSerialization::Tracked(tracked) => {
                Shell::extract_tracked(tracked).map_err(D::Error::custom)
            }
            ShellSerialization::Legacy(compressed) => {
                Shell::extract(compressed).map_err(D::Error::custom)
            }
        }
    }
}

impl<P, C, S> Serialize for Solid<P, C, S>
where
    P: Clone + Serialize,
    C: Clone + Serialize,
    S: Clone + Serialize,
{
    fn serialize<Serializer>(
        &self,
        serializer: Serializer,
    ) -> std::result::Result<Serializer::Ok, Serializer::Error>
    where
        Serializer: serde::Serializer,
    {
        if self.tracking_ids().is_empty() {
            SolidSerialization::Legacy(self.compress()).serialize(serializer)
        } else {
            SolidSerialization::Tracked(self.compress_tracked()).serialize(serializer)
        }
    }
}

impl<'de, P, C, S> Deserialize<'de> for Solid<P, C, S>
where
    P: Clone + Deserialize<'de>,
    C: Clone + Deserialize<'de>,
    S: Clone + Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where D: serde::Deserializer<'de> {
        use serde::de::Error;
        match SolidSerialization::<P, C, S>::deserialize(deserializer)? {
            SolidSerialization::Tracked(tracked) => {
                Solid::extract_tracked(tracked).map_err(D::Error::custom)
            }
            SolidSerialization::Legacy(compressed) => {
                Solid::extract(compressed).map_err(D::Error::custom)
            }
        }
    }
}

impl<P, C, S> Serialize for Face<P, C, S>
where
    P: Clone + Serialize,
    C: Clone + Serialize,
    S: Clone + Serialize,
{
    fn serialize<Serializer>(
        &self,
        serializer: Serializer,
    ) -> std::result::Result<Serializer::Ok, Serializer::Error>
    where
        Serializer: serde::Serializer,
    {
        Shell::from(vec![self.clone()]).serialize(serializer)
    }
}

impl<'de, P, C, S> Deserialize<'de> for Face<P, C, S>
where
    P: Clone + Deserialize<'de>,
    C: Clone + Deserialize<'de>,
    S: Clone + Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where D: serde::Deserializer<'de> {
        use serde::de::Error as _;

        let mut shell = Shell::deserialize(deserializer)?;
        if shell.len() == 1 {
            shell
                .pop()
                .ok_or_else(|| D::Error::custom(Error::InvalidCompressedFaceCount { count: 0 }))
        } else {
            Err(D::Error::custom(Error::InvalidCompressedFaceCount {
                count: shell.len(),
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloned_without_trims_matches_clone_then_erase_trims() {
        let shell = CompressedTrimmedShell {
            vertices: vec![0usize, 1usize],
            edges: vec![CompressedEdge {
                vertices: (0, 1),
                curve: 5usize,
            }],
            faces: vec![CompressedTrimmedFace {
                boundaries: vec![vec![CompressedEdgeUse {
                    index: 0,
                    orientation: true,
                    trim_curve: Some(7usize),
                }]],
                orientation: true,
                surface: (),
            }],
        };

        assert_eq!(shell.cloned_without_trims(), shell.clone().erase_trims());
    }
}
