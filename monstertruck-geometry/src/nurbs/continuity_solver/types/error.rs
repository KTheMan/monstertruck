//! Typed continuity-solver failures.

use super::super::super::continuity::SurfaceContinuityCapability;
use super::{BoundaryEndpoint, ContinuityResource, ContinuitySolveReport};
use thiserror::Error;
/// Failure to prepare or solve a geometric-continuity problem.
#[derive(Clone, Debug, Error, PartialEq)]
pub enum ContinuitySolveError {
    /// A solver configuration invariant was violated.
    #[error("invalid continuity solver configuration: {0}")]
    InvalidConfig(&'static str),
    /// A checked caller-controlled dimension exceeded its solver budget.
    #[error("continuity solver {resource:?} budget exceeded: requested {requested}, limit {limit}")]
    ResourceLimitExceeded {
        /// Dimension that exceeded the budget.
        resource: ContinuityResource,
        /// Checked required count.
        requested: usize,
        /// Configured maximum count.
        limit: usize,
    },
    /// G4 was requested without explicit experimental opt-in.
    #[error("G4 continuity solving requires explicit experimental opt-in")]
    ExperimentalG4Disabled,
    /// A boundary lacks sufficient degree or control rows.
    #[error("{endpoint:?} boundary cannot represent the requested continuity: {capability:?}")]
    UnsupportedCapability {
        /// Problem endpoint that failed capability validation.
        endpoint: BoundaryEndpoint,
        /// Degree and control-row diagnostics.
        capability: SurfaceContinuityCapability,
    },
    /// A surface boundary is empty, non-finite, unclamped, or degenerate.
    #[error("{0:?} boundary has an invalid tensor-product parameter domain")]
    InvalidBoundary(BoundaryEndpoint),
    /// A finite rational weight is below the configured positive minimum.
    #[error("{endpoint:?} surface has invalid weight {weight} at ({row}, {column})")]
    NonPositiveWeight {
        /// Problem endpoint containing the weight.
        endpoint: BoundaryEndpoint,
        /// Control-net row.
        row: usize,
        /// Control-net column.
        column: usize,
        /// Invalid rational weight.
        weight: f64,
    },
    /// A homogeneous control-point coordinate is non-finite.
    #[error("{endpoint:?} surface has a non-finite control point at ({row}, {column})")]
    NonFiniteControlPoint {
        /// Problem endpoint containing the control point.
        endpoint: BoundaryEndpoint,
        /// Control-net row.
        row: usize,
        /// Control-net column.
        column: usize,
    },
    /// A sampled surface tangent frame is singular.
    #[error("{endpoint:?} boundary is degenerate at sample {sample}")]
    DegenerateBoundary {
        /// Problem endpoint containing the degenerate sample.
        endpoint: BoundaryEndpoint,
        /// Canonical sample index.
        sample: usize,
    },
    /// Residual evaluation produced a non-finite value.
    #[error("continuity residual evaluation produced a non-finite value")]
    NonFiniteResidual,
    /// Jacobian evaluation produced a non-finite value.
    #[error("continuity Jacobian evaluation produced a non-finite value")]
    NonFiniteJacobian,
    /// No finite descent step could be produced.
    #[error("continuity solver could not produce a descent direction")]
    NoDescentDirection,
    /// Iteration stopped before every requested order met tolerance.
    #[error("continuity solver did not converge")]
    DidNotConverge(Box<ContinuitySolveReport>),
}
