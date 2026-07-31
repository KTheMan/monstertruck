//! Typed validation failures.

use monstertruck_geometry::nurbs::continuity::SurfaceBoundary;
use thiserror::Error;

#[derive(Clone, Debug, Error, PartialEq)]
pub(super) enum ValidationError {
    #[error("shell indices are one-based")]
    ZeroShellIndex,
    #[error("STEP shell {requested} was not found; the file contains {available} shells")]
    ShellNotFound { requested: usize, available: usize },
    #[error("continuity order {0} is unsupported by this validation example")]
    UnsupportedOrder(u8),
    #[error("at least two B-spline or rational NURBS faces are required; found {0}")]
    InsufficientNurbsFaces(usize),
    #[error(
        "no coincident complete rectangular NURBS boundaries were found within tolerance {tolerance}"
    )]
    NoCoincidentFullBoundary { tolerance: f64 },
    #[error(
        "faces {first_face} and {second_face} coincide on {first_boundary:?}/{second_boundary:?}, \
         but their shared trim is an arbitrary subcurve rather than both complete patch boundaries"
    )]
    UnsupportedTrimmedSubcurve {
        first_face: usize,
        second_face: usize,
        first_boundary: SurfaceBoundary,
        second_boundary: SurfaceBoundary,
    },
    #[error(
        "dependent boundary {boundary:?} has only {available} control rows; {required} required"
    )]
    InsufficientBoundaryStrip {
        boundary: SurfaceBoundary,
        available: usize,
        required: usize,
    },
    #[error("certification interval count must be at least 32")]
    SparseCertification,
    #[error("validation tolerance `{name}` must be positive and finite")]
    InvalidTolerance { name: &'static str },
    #[error("a non-finite or degenerate tangent frame occurred at dense sample {sample}")]
    DegenerateTangentFrame { sample: usize },
    #[error(
        "independent G1 certification failed: position {position_maximum:e} \
         (limit {position_tolerance:e}), tangent {tangent_maximum:e} \
         (limit {tangent_tolerance:e})"
    )]
    CertificationFailed {
        position_maximum: f64,
        position_tolerance: f64,
        tangent_maximum: f64,
        tangent_tolerance: f64,
    },
    #[error("tessellation produced no finite triangles")]
    EmptyOrNonFiniteMesh,
    #[error("re-imported STEP contains no shell")]
    EmptyReimport,
    #[error("re-imported STEP lost the solved spline face pair")]
    ReimportLostNurbsFaces,
}
