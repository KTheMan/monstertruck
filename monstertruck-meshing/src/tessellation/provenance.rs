//! Render-primitive provenance for tessellated topology.

use monstertruck_core::{Point3, StableId};
use monstertruck_mesh::{PolygonMesh, PolylineCurve};
use monstertruck_topology::{Shell, Solid, compress::*};
#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;

use crate::*;

use super::{
    ExactTrimBoundary2D, MeshableSurface, Parallelizable, PolylineableCurve, RobustMeshableSurface,
    TessellationOptions, cshell_triangulation_with, robust_cshell_triangulation_with,
    robust_triangulation_with, robust_trimmed_cshell_triangulation_with, triangulation_with,
    trimmed_cshell_triangulation_with,
};

mod assembly;
use assembly::{
    compressed_shell_provenance, compressed_solid_provenance, shell_provenance, solid_provenance,
    trimmed_shell_provenance, trimmed_solid_provenance,
};

/// Identity of a source face represented in a tessellation.
///
/// The indices locate the face in the exact source snapshot used for
/// tessellation. The [`StableId`] supports persistent selection when the source
/// topology assigned one.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FaceProvenance {
    shell_index: usize,
    face_index: usize,
    stable_id: Option<StableId>,
}

impl FaceProvenance {
    /// Returns the source boundary-shell index.
    pub const fn shell_index(&self) -> usize { self.shell_index }

    /// Returns the source face index within its shell.
    pub const fn face_index(&self) -> usize { self.face_index }

    /// Returns the source face's persistent identifier when assigned.
    pub const fn stable_id(&self) -> Option<StableId> { self.stable_id }
}

/// Location of a unique source edge in the snapshot used for tessellation.
///
/// Live topology stores shared edges as face-boundary uses, while compressed
/// topology stores them in a shell-level edge array. The variants reflect
/// those two source layouts without imposing a new identity system.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum EdgeLocator {
    /// A face-boundary use of an edge in a live [`Shell`].
    TopologyUse {
        /// The source boundary-shell index.
        shell_index: usize,
        /// The source face index within its shell.
        face_index: usize,
        /// The source boundary index within its face.
        boundary_index: usize,
        /// The source edge-use index within its boundary.
        edge_index: usize,
    },
    /// An edge in a [`CompressedShell`] edge array.
    Compressed {
        /// The source boundary-shell index.
        shell_index: usize,
        /// The source edge-array index within its shell.
        edge_index: usize,
    },
}

impl EdgeLocator {
    /// Returns the source boundary-shell index.
    pub const fn shell_index(self) -> usize {
        match self {
            Self::TopologyUse { shell_index, .. } | Self::Compressed { shell_index, .. } => {
                shell_index
            }
        }
    }
}

/// Identity of a unique source edge represented in a tessellation.
///
/// Shared edge uses are collapsed to one entry in deterministic first-use
/// order within each shell.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct EdgeProvenance {
    locator: EdgeLocator,
    stable_id: Option<StableId>,
}

impl EdgeProvenance {
    /// Returns a locator into the exact source snapshot used for tessellation.
    pub const fn locator(&self) -> EdgeLocator { self.locator }

    /// Returns the source boundary-shell index.
    pub const fn shell_index(&self) -> usize { self.locator.shell_index() }

    /// Returns the source edge's persistent identifier when assigned.
    pub const fn stable_id(&self) -> Option<StableId> { self.stable_id }
}

/// Tessellated render geometry with source-topology provenance.
///
/// `triangle_face_indices` follows the exact order of
/// [`Faces::triangle_iter`](monstertruck_mesh::Faces::triangle_iter), which is
/// also the triangle order used by `monstertruck-render`. `line_edge_indices`
/// follows the line-list order produced by rendering `edge_polylines`.
#[derive(Clone, Debug, PartialEq)]
pub struct TessellationWithProvenance {
    polygon: PolygonMesh,
    edge_polylines: Vec<PolylineCurve<Point3>>,
    faces: Vec<FaceProvenance>,
    edges: Vec<EdgeProvenance>,
    triangle_face_indices: Vec<usize>,
    line_edge_indices: Vec<usize>,
}

impl TessellationWithProvenance {
    /// Returns the flattened polygon mesh.
    pub const fn polygon(&self) -> &PolygonMesh { &self.polygon }

    /// Consumes the tessellation and returns the flattened polygon mesh.
    pub fn into_polygon(self) -> PolygonMesh { self.polygon }

    /// Returns unique tessellated source-edge polylines.
    ///
    /// This slice is parallel to [`edges`](Self::edges).
    pub fn edge_polylines(&self) -> &[PolylineCurve<Point3>] { &self.edge_polylines }

    /// Returns source-face records in shell and face order.
    pub fn faces(&self) -> &[FaceProvenance] { &self.faces }

    /// Returns unique source-edge records in shell and first-use order.
    pub fn edges(&self) -> &[EdgeProvenance] { &self.edges }

    /// Maps each rendered triangle to an index in [`faces`](Self::faces).
    pub fn triangle_face_indices(&self) -> &[usize] { &self.triangle_face_indices }

    /// Maps each rendered edge line to an index in [`edges`](Self::edges).
    pub fn line_edge_indices(&self) -> &[usize] { &self.line_edge_indices }

    /// Returns the source face represented by a rendered triangle.
    pub fn face_for_triangle(&self, triangle_index: usize) -> Option<&FaceProvenance> {
        self.triangle_face_indices
            .get(triangle_index)
            .and_then(|face_index| self.faces.get(face_index))
    }

    /// Returns the source edge represented by a rendered line.
    pub fn edge_for_line(&self, line_index: usize) -> Option<&EdgeProvenance> {
        self.line_edge_indices
            .get(line_index)
            .and_then(|edge_index| self.edges.get(edge_index))
    }
}

/// Tessellates topology while retaining render-primitive provenance.
///
/// Existing [`MeshableShape`](super::MeshableShape) implementations remain
/// unchanged. This opt-in path is intended for consumers such as GUI picking
/// systems that must resolve a rendered triangle or line back to topology.
///
/// ```
/// use monstertruck_meshing::prelude::*;
/// use monstertruck_modeling::{BoundingBox, Point3, Solid, primitive};
///
/// let mut solid: Solid = primitive::cuboid(BoundingBox::from_iter([
///     Point3::new(0.0, 0.0, 0.0),
///     Point3::new(1.0, 1.0, 1.0),
/// ]));
/// solid.ensure_topology_stable_ids();
/// let mut options = TessellationOptions::default();
/// options.primitive.mode = TessellationPrimitiveMode::PreferQuads;
/// let tessellation = solid.triangulation_with_provenance(options);
///
/// assert_eq!(
///     tessellation.triangle_face_indices().len(),
///     tessellation.polygon().faces().triangle_iter().len(),
/// );
/// assert_eq!(
///     tessellation.line_edge_indices().len(),
///     tessellation
///         .edge_polylines()
///         .iter()
///         .map(|polyline| polyline.len().saturating_sub(1))
///         .sum::<usize>(),
/// );
/// assert!(tessellation
///     .face_for_triangle(0)
///     .and_then(FaceProvenance::stable_id)
///     .is_some());
/// assert!(tessellation
///     .edge_for_line(0)
///     .and_then(EdgeProvenance::stable_id)
///     .is_some());
/// ```
pub trait MeshableShapeWithProvenance {
    /// Tessellates the shape with face and edge provenance.
    ///
    /// # Panics
    ///
    /// Panics when [`TessellationOptions::tolerance`] is less than `TOLERANCE`.
    fn triangulation_with_provenance(
        &self,
        options: TessellationOptions,
    ) -> TessellationWithProvenance;
}

/// Robustly tessellates topology while retaining render-primitive provenance.
pub trait RobustMeshableShapeWithProvenance {
    /// Tessellates the shape using robust surface-parameter searches.
    ///
    /// # Panics
    ///
    /// Panics when [`TessellationOptions::tolerance`] is less than `TOLERANCE`.
    fn robust_triangulation_with_provenance(
        &self,
        options: TessellationOptions,
    ) -> TessellationWithProvenance;
}
impl<C, S> MeshableShapeWithProvenance for Shell<Point3, C, S>
where
    C: PolylineableCurve,
    S: MeshableSurface,
{
    fn triangulation_with_provenance(
        &self,
        options: TessellationOptions,
    ) -> TessellationWithProvenance {
        let meshed = triangulation_with(self, options);
        shell_provenance(self, &meshed)
    }
}

impl<C, S> RobustMeshableShapeWithProvenance for Shell<Point3, C, S>
where
    C: PolylineableCurve,
    S: RobustMeshableSurface,
{
    fn robust_triangulation_with_provenance(
        &self,
        options: TessellationOptions,
    ) -> TessellationWithProvenance {
        let meshed = robust_triangulation_with(self, options);
        shell_provenance(self, &meshed)
    }
}

impl<C, S> MeshableShapeWithProvenance for Solid<Point3, C, S>
where
    C: PolylineableCurve,
    S: MeshableSurface,
{
    fn triangulation_with_provenance(
        &self,
        options: TessellationOptions,
    ) -> TessellationWithProvenance {
        #[cfg(not(target_arch = "wasm32"))]
        let meshed = self
            .boundaries()
            .par_iter()
            .map(|shell| triangulation_with(shell, options))
            .collect::<Vec<_>>();
        #[cfg(target_arch = "wasm32")]
        let meshed = self
            .boundaries()
            .iter()
            .map(|shell| triangulation_with(shell, options))
            .collect::<Vec<_>>();
        solid_provenance(self, &meshed)
    }
}

impl<C, S> RobustMeshableShapeWithProvenance for Solid<Point3, C, S>
where
    C: PolylineableCurve,
    S: RobustMeshableSurface,
{
    fn robust_triangulation_with_provenance(
        &self,
        options: TessellationOptions,
    ) -> TessellationWithProvenance {
        #[cfg(not(target_arch = "wasm32"))]
        let meshed = self
            .boundaries()
            .par_iter()
            .map(|shell| robust_triangulation_with(shell, options))
            .collect::<Vec<_>>();
        #[cfg(target_arch = "wasm32")]
        let meshed = self
            .boundaries()
            .iter()
            .map(|shell| robust_triangulation_with(shell, options))
            .collect::<Vec<_>>();
        solid_provenance(self, &meshed)
    }
}

impl<C, S> MeshableShapeWithProvenance for CompressedShell<Point3, C, S>
where
    C: PolylineableCurve + ParameterBoundary2D<S>,
    S: MeshableSurface,
{
    fn triangulation_with_provenance(
        &self,
        options: TessellationOptions,
    ) -> TessellationWithProvenance {
        let meshed = cshell_triangulation_with(self, options);
        compressed_shell_provenance(self, &meshed)
    }
}

impl<C, S> RobustMeshableShapeWithProvenance for CompressedShell<Point3, C, S>
where
    C: PolylineableCurve + ParameterBoundary2D<S>,
    S: RobustMeshableSurface,
{
    fn robust_triangulation_with_provenance(
        &self,
        options: TessellationOptions,
    ) -> TessellationWithProvenance {
        let meshed = robust_cshell_triangulation_with(self, options);
        compressed_shell_provenance(self, &meshed)
    }
}

impl<C, S> MeshableShapeWithProvenance for CompressedSolid<Point3, C, S>
where
    C: PolylineableCurve + ParameterBoundary2D<S>,
    S: MeshableSurface,
{
    fn triangulation_with_provenance(
        &self,
        options: TessellationOptions,
    ) -> TessellationWithProvenance {
        #[cfg(not(target_arch = "wasm32"))]
        let meshed = self
            .boundaries
            .par_iter()
            .map(|shell| cshell_triangulation_with(shell, options))
            .collect::<Vec<_>>();
        #[cfg(target_arch = "wasm32")]
        let meshed = self
            .boundaries
            .iter()
            .map(|shell| cshell_triangulation_with(shell, options))
            .collect::<Vec<_>>();
        compressed_solid_provenance(self, &meshed)
    }
}

impl<C, S> RobustMeshableShapeWithProvenance for CompressedSolid<Point3, C, S>
where
    C: PolylineableCurve + ParameterBoundary2D<S>,
    S: RobustMeshableSurface,
{
    fn robust_triangulation_with_provenance(
        &self,
        options: TessellationOptions,
    ) -> TessellationWithProvenance {
        #[cfg(not(target_arch = "wasm32"))]
        let meshed = self
            .boundaries
            .par_iter()
            .map(|shell| robust_cshell_triangulation_with(shell, options))
            .collect::<Vec<_>>();
        #[cfg(target_arch = "wasm32")]
        let meshed = self
            .boundaries
            .iter()
            .map(|shell| robust_cshell_triangulation_with(shell, options))
            .collect::<Vec<_>>();
        compressed_solid_provenance(self, &meshed)
    }
}

impl<C, S, T> MeshableShapeWithProvenance for CompressedTrimmedShell<Point3, C, S, T>
where
    C: PolylineableCurve + ParameterBoundary2D<S> + ExactParameterBoundary2D<S>,
    S: MeshableSurface,
    T: ExactTrimBoundary2D + Parallelizable,
    <C as ExactParameterBoundary2D<S>>::BoundaryCurve: ExactTrimBoundary2D,
{
    fn triangulation_with_provenance(
        &self,
        options: TessellationOptions,
    ) -> TessellationWithProvenance {
        let meshed = trimmed_cshell_triangulation_with(self, options);
        trimmed_shell_provenance(self, &meshed)
    }
}

impl<C, S, T> RobustMeshableShapeWithProvenance for CompressedTrimmedShell<Point3, C, S, T>
where
    C: PolylineableCurve + ParameterBoundary2D<S> + ExactParameterBoundary2D<S>,
    S: RobustMeshableSurface,
    T: ExactTrimBoundary2D + Parallelizable,
    <C as ExactParameterBoundary2D<S>>::BoundaryCurve: ExactTrimBoundary2D,
{
    fn robust_triangulation_with_provenance(
        &self,
        options: TessellationOptions,
    ) -> TessellationWithProvenance {
        let meshed = robust_trimmed_cshell_triangulation_with(self, options);
        trimmed_shell_provenance(self, &meshed)
    }
}

impl<C, S, T> MeshableShapeWithProvenance for CompressedTrimmedSolid<Point3, C, S, T>
where
    C: PolylineableCurve + ParameterBoundary2D<S> + ExactParameterBoundary2D<S>,
    S: MeshableSurface,
    T: ExactTrimBoundary2D + Parallelizable,
    <C as ExactParameterBoundary2D<S>>::BoundaryCurve: ExactTrimBoundary2D,
{
    fn triangulation_with_provenance(
        &self,
        options: TessellationOptions,
    ) -> TessellationWithProvenance {
        #[cfg(not(target_arch = "wasm32"))]
        let meshed = self
            .boundaries
            .par_iter()
            .map(|shell| trimmed_cshell_triangulation_with(shell, options))
            .collect::<Vec<_>>();
        #[cfg(target_arch = "wasm32")]
        let meshed = self
            .boundaries
            .iter()
            .map(|shell| trimmed_cshell_triangulation_with(shell, options))
            .collect::<Vec<_>>();
        trimmed_solid_provenance(self, &meshed)
    }
}

impl<C, S, T> RobustMeshableShapeWithProvenance for CompressedTrimmedSolid<Point3, C, S, T>
where
    C: PolylineableCurve + ParameterBoundary2D<S> + ExactParameterBoundary2D<S>,
    S: RobustMeshableSurface,
    T: ExactTrimBoundary2D + Parallelizable,
    <C as ExactParameterBoundary2D<S>>::BoundaryCurve: ExactTrimBoundary2D,
{
    fn robust_triangulation_with_provenance(
        &self,
        options: TessellationOptions,
    ) -> TessellationWithProvenance {
        #[cfg(not(target_arch = "wasm32"))]
        let meshed = self
            .boundaries
            .par_iter()
            .map(|shell| robust_trimmed_cshell_triangulation_with(shell, options))
            .collect::<Vec<_>>();
        #[cfg(target_arch = "wasm32")]
        let meshed = self
            .boundaries
            .iter()
            .map(|shell| robust_trimmed_cshell_triangulation_with(shell, options))
            .collect::<Vec<_>>();
        trimmed_solid_provenance(self, &meshed)
    }
}

#[cfg(test)]
mod tests;
