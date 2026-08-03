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
    #[error("invalid certification configuration: {reason}")]
    InvalidCertificationConfig { reason: &'static str },
    #[error("invalid certification geometry: {reason}")]
    InvalidCertificationGeometry { reason: &'static str },
    #[error("the finite-difference certification stencil is singular")]
    SingularCertificationStencil,
    #[error("the transition is non-finite at seam {seam:e}, cross coordinate {cross:e}")]
    TransitionSamplingFailed { seam: f64, cross: f64 },
    #[error(
        "certification sampled outside the normalized domain at seam {seam:e}, inward {inward:e}"
    )]
    BoundarySamplingOutsideDomain { seam: f64, inward: f64 },
    #[error("certification produced a non-finite result at seam {seam:e}")]
    NonFiniteCertificate { seam: f64 },
    #[error("a degenerate tangent frame occurred at seam {seam:e}")]
    DegenerateTangentFrame { seam: f64 },
    #[error(
        "independent order-{order} residual certification failed: normalized maximum \
         {maximum:e} exceeds {tolerance:e}"
    )]
    CertificationResidualFailed {
        order: usize,
        maximum: f64,
        tolerance: f64,
    },
    #[error(
        "independent tangent-plane certification failed: maximum angle {maximum:e} radians \
         exceeds {tolerance:e}"
    )]
    CertificationNormalFailed { maximum: f64, tolerance: f64 },
    #[error("tessellation produced no triangles")]
    EmptyMesh,
    #[error("tessellation produced no mesh for face {face}")]
    MissingFaceMesh { face: usize },
    #[error("tessellation produced a non-triangular polygon for face {face}")]
    NonTriangularFaceMesh { face: usize },
    #[error("tessellation produced a non-finite position {position} for face {face}")]
    NonFiniteMeshPosition { face: usize, position: usize },
    #[error("tessellation produced no finite positive mesh scale")]
    InvalidMeshScale,
    #[error("face {face} triangle {triangle} contains an invalid position index")]
    InvalidTriangleVertex { face: usize, triangle: usize },
    #[error("face {face} triangle {triangle} has no complete surface-normal attribution")]
    MissingTriangleNormal { face: usize, triangle: usize },
    #[error("face {face} triangle {triangle} contains a non-finite or zero surface normal")]
    NonFiniteTriangleNormal { face: usize, triangle: usize },
    #[error(
        "face {face} triangle {triangle} has scale-normalized doubled area {normalized_double_area:e}, \
         not greater than {tolerance:e}"
    )]
    DegenerateTriangle {
        face: usize,
        triangle: usize,
        normalized_double_area: f64,
        tolerance: f64,
    },
    #[error(
        "face {face} triangle {triangle} has normal alignment {alignment:e}, below {tolerance:e}"
    )]
    InconsistentTriangleOrientation {
        face: usize,
        triangle: usize,
        alignment: f64,
        tolerance: f64,
    },
    #[error("re-imported STEP contains no shell")]
    EmptyReimport,
    #[error("re-imported STEP lost the solved spline face pair")]
    ReimportLostNurbsFaces,
    #[error("the evaluated shell has no finite positive bounding box")]
    InvalidBoundingBox,
    #[error(
        "STEP re-import changed the compressed topology signature: before={before}, after={after}"
    )]
    TopologyPersistenceMismatch { before: String, after: String },
    #[error(
        "STEP re-import changed the scale-normalized bounding box by {maximum:e}, \
         exceeding {tolerance:e}"
    )]
    BoundingBoxPersistenceFailed { maximum: f64, tolerance: f64 },
}
