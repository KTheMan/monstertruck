//! Continuity repair adapters for imported STEP topology.
//!
//! This module keeps topology and STEP concerns outside the numerical geometry
//! solver. A seam is selected by its two face indices and shared edge index.
//! Only a trim that is exactly one complete tensor-product patch side reaches
//! the solver; every arbitrary trimmed seam is refused with
//! [`UnsupportedContinuityCapability::TrimmedBoundary`].

use monstertruck_geometry::nurbs::continuity::{
    BoundaryAlignment, BoundarySide, ContinuityOrder, UnsupportedContinuityCapability,
};
use monstertruck_geometry::nurbs::continuity_solver::{
    BoundaryContinuityRequest, BoundaryContinuitySolver, ContinuitySolveError,
    ContinuitySolveReport,
};
use monstertruck_geometry::prelude::{BoundedCurve, Invertible, ParametricCurve};
use monstertruck_geometry::prelude::{BoundedSurface, NurbsSurface, ParameterCurve, Vector4};
use thiserror::Error;

use crate::load::convert::StepCompressedTrimmedShell;
use crate::load::step_geometry::{Curve2D, Curve3D, Surface};

/// A shared edge and the two imported faces that meet along it.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct StepContinuitySeam {
    first_face: usize,
    second_face: usize,
    shared_edge: usize,
}

impl StepContinuitySeam {
    /// Creates an imported seam selection.
    ///
    /// # Errors
    ///
    /// Returns [`StepContinuityError::SameFace`] when both indices select the
    /// same face rather than an adjacent pair.
    pub const fn new(
        first_face: usize,
        second_face: usize,
        shared_edge: usize,
    ) -> Result<Self, StepContinuityError> {
        if first_face == second_face {
            Err(StepContinuityError::SameFace { face: first_face })
        } else {
            Ok(Self {
                first_face,
                second_face,
                shared_edge,
            })
        }
    }

    /// Returns the fixed reference face index.
    pub const fn first_face(self) -> usize { self.first_face }

    /// Returns the optimized face index.
    pub const fn second_face(self) -> usize { self.second_face }

    /// Returns the shared edge index.
    pub const fn shared_edge(self) -> usize { self.shared_edge }
}

/// Failure to adapt an imported STEP seam to the continuity solver.
#[derive(Clone, Debug, Error, PartialEq)]
pub enum StepContinuityError {
    /// A seam must join two distinct adjacent faces.
    #[error("STEP continuity seam selects face {face} twice")]
    SameFace {
        /// Repeated face index.
        face: usize,
    },
    /// A selected face index is not present in the imported shell.
    #[error("STEP continuity face index {face} is out of range")]
    FaceOutOfRange {
        /// Missing face index.
        face: usize,
    },
    /// The selected edge is not used by one of the selected faces.
    #[error("STEP continuity edge {edge} is not used by face {face}")]
    EdgeNotUsedByFace {
        /// Face that does not use the edge.
        face: usize,
        /// Missing edge-use index.
        edge: usize,
    },
    /// The selected shared edge is not present in the imported shell.
    #[error("STEP continuity edge index {edge} is out of range")]
    EdgeOutOfRange {
        /// Missing edge index.
        edge: usize,
    },
    /// An imported face cannot expose the requested full-side capability.
    #[error("STEP continuity face {face} is unsupported: {reason}")]
    UnsupportedCapability {
        /// Unsupported face index.
        face: usize,
        /// Typed unsupported condition.
        reason: UnsupportedContinuityCapability,
    },
    /// The numerical continuity solve failed.
    #[error(transparent)]
    Solve(#[from] ContinuitySolveError),
}

/// Repairs one full-side seam in an imported STEP shell.
///
/// The first face remains fixed. The second face is replaced transactionally
/// only after the solve succeeds. Its face-local parameter curves are rebound
/// to the replacement surface, and its changed non-shared boundary edges and
/// vertices are synchronized before the shell is returned to STEP export.
///
/// # Errors
///
/// Returns [`StepContinuityError::UnsupportedCapability`] with
/// [`UnsupportedContinuityCapability::TrimmedBoundary`] when either selected
/// edge-use is not exactly one complete patch side. Missing topology,
/// unsupported surface representations, and solver failures are also returned
/// as typed errors.
pub fn repair_step_continuity(
    shell: &mut StepCompressedTrimmedShell,
    seam: StepContinuitySeam,
    alignment: BoundaryAlignment,
    order: ContinuityOrder,
    solver: &BoundaryContinuitySolver,
) -> Result<ContinuitySolveReport, StepContinuityError> {
    shell
        .edges
        .get(seam.shared_edge)
        .ok_or(StepContinuityError::EdgeOutOfRange {
            edge: seam.shared_edge,
        })?;
    let first = selected_surface(shell, seam.first_face, seam.shared_edge)?;
    let second = selected_surface(shell, seam.second_face, seam.shared_edge)?;
    let request = BoundaryContinuityRequest::new(first.side, second.side, alignment, order);
    let solution = solver.solve(&first.surface, &second.surface, request)?;
    let report = solution.report().clone();
    let replacement = Surface::NurbsSurface(solution.second().clone());

    let rebound_edges = {
        let face =
            shell
                .faces
                .get_mut(seam.second_face)
                .ok_or(StepContinuityError::FaceOutOfRange {
                    face: seam.second_face,
                })?;
        face.surface = replacement.clone();
        face.boundaries
            .iter_mut()
            .flatten()
            .filter_map(|edge_use| {
                edge_use.trim_curve.as_mut().map(|trim| {
                    *trim =
                        ParameterCurve::new(trim.curve().clone(), Box::new(replacement.clone()));
                    (edge_use.index, edge_use.orientation, trim.clone())
                })
            })
            .collect::<Vec<_>>()
    };
    if let Curve3D::SurfaceCurve(surface_curve) = shell.edges[seam.shared_edge].curve.clone() {
        shell.edges[seam.shared_edge].curve = surface_curve.leader().clone();
    }
    rebound_edges
        .into_iter()
        .filter(|(edge, _, _)| *edge != seam.shared_edge)
        .for_each(|(edge_index, orientation, mut trim)| {
            if !orientation {
                trim.invert();
            }
            let (minimum, maximum) = trim.range_tuple();
            let endpoints = (trim.subs(minimum), trim.subs(maximum));
            let edge = &mut shell.edges[edge_index];
            shell.vertices[edge.vertices.0] = endpoints.0;
            shell.vertices[edge.vertices.1] = endpoints.1;
            edge.curve = Curve3D::ParameterCurve(trim);
        });

    Ok(report)
}

#[derive(Clone)]
struct SelectedSurface {
    surface: NurbsSurface<Vector4>,
    side: BoundarySide,
}

fn selected_surface(
    shell: &StepCompressedTrimmedShell,
    face_index: usize,
    edge_index: usize,
) -> Result<SelectedSurface, StepContinuityError> {
    let face = shell
        .faces
        .get(face_index)
        .ok_or(StepContinuityError::FaceOutOfRange { face: face_index })?;
    let edge_use = face
        .boundaries
        .iter()
        .flatten()
        .find(|edge_use| edge_use.index == edge_index)
        .ok_or(StepContinuityError::EdgeNotUsedByFace {
            face: face_index,
            edge: edge_index,
        })?;
    let surface = exact_nurbs(&face.surface, face_index)?;
    let side = edge_use
        .trim_curve
        .as_ref()
        .and_then(|trim| full_boundary_side(trim, &surface))
        .ok_or(StepContinuityError::UnsupportedCapability {
            face: face_index,
            reason: UnsupportedContinuityCapability::TrimmedBoundary,
        })?;
    Ok(SelectedSurface { surface, side })
}

fn exact_nurbs(
    surface: &Surface,
    face_index: usize,
) -> Result<NurbsSurface<Vector4>, StepContinuityError> {
    match surface {
        Surface::BsplineSurface(surface) => Ok(NurbsSurface::from(surface.clone())),
        Surface::NurbsSurface(surface) => Ok(surface.clone()),
        _ => Err(StepContinuityError::UnsupportedCapability {
            face: face_index,
            reason: UnsupportedContinuityCapability::UnsupportedRepresentation,
        }),
    }
}

fn full_boundary_side(
    trim: &crate::load::step_geometry::StepParameterCurve,
    surface: &NurbsSurface<Vector4>,
) -> Option<BoundarySide> {
    let Curve2D::Line(line) = trim.curve().as_ref() else {
        return None;
    };
    let ((min_u, max_u), (min_v, max_v)) = BoundedSurface::range_tuple(surface);
    let first = line.0;
    let second = line.1;
    let spans = |left: f64, right: f64, minimum: f64, maximum: f64| {
        (left == minimum && right == maximum) || (left == maximum && right == minimum)
    };

    if first.x == min_u && second.x == min_u && spans(first.y, second.y, min_v, max_v) {
        Some(BoundarySide::MinU)
    } else if first.x == max_u && second.x == max_u && spans(first.y, second.y, min_v, max_v) {
        Some(BoundarySide::MaxU)
    } else if first.y == min_v && second.y == min_v && spans(first.x, second.x, min_u, max_u) {
        Some(BoundarySide::MinV)
    } else if first.y == max_v && second.y == max_v && spans(first.x, second.x, min_u, max_u) {
        Some(BoundarySide::MaxV)
    } else {
        None
    }
}

#[cfg(test)]
#[path = "continuity/tests.rs"]
mod tests;
