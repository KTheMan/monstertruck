//! Variational geometric-continuity solving for NURBS surface boundaries.
//!
//! The solver matches every mixed coefficient of two factorial-normalized
//! boundary Taylor jets through the requested total order. Its local transition
//! has a monotone endpoint-preserving seam map `phi(t)`, tangential cross terms
//! `alpha_j(t)`, and inward terms `beta_j(t)`. `beta_1` is represented as an
//! exponential, so the transition cannot flip the surface side. This is
//! geometric `Gk` matching rather than raw parametric `Ck` equality.
//!
//! The first surface is a fixed master. The solver optimizes Euclidean
//! coordinates in a boundary strip of the second rational surface while
//! retaining its positive homogeneous weights. Exact forward dual derivatives
//! assemble the Jacobian, and damped least squares is solved by deterministic
//! column-pivoted Householder QR without normal equations. Continuity terms are
//! normalized by the master-boundary length and weighted by their per-order
//! tolerances; anchor, thin-strip fairness, and transition regularization remain
//! separate styling controls. A denser degree-aware midpoint grid is evaluated
//! independently after collocation and can veto convergence between optimizer
//! samples.
//!
//! G0 through G3 are production targets. G4 uses the same order-generic jet
//! representation and residual machinery but requires explicit experimental
//! opt-in. Solver tolerances belong only to the unclamped `f64` styling layer;
//! topology sewing and solidification tolerances are intentionally absent.
//!
//! [`crate::nurbs::continuity_solver::BoundaryContinuitySolver::solve`] and
//! [`crate::nurbs::continuity_solver::execute_boundary_continuity_contracts`] are transactional:
//! they borrow
//! inputs and return owned solved clones. Replay resolves persistent semantic
//! contracts against current-generation tracking IDs, orders acyclic
//! master-to-dependent chains, and rejects coupled multi-boundary systems until
//! a future joint solver can preserve all obligations simultaneously.

mod boundary;
mod dual;
mod lm;
mod problem;
mod qr;
mod replay;
mod resource;
mod sampling;
mod taylor;
mod types;

pub use replay::{
    ContinuityContractSolve, ContinuityReplayError, ContinuityReplayExecutionError,
    ContinuityReplaySolution, ResolvedBoundaryContinuityRequest, TrackedSurfaceIdRegistry,
    execute_boundary_continuity_contracts, prepare_boundary_continuity_requests,
};
pub use resource::ContinuityResourceBudget;
pub use types::{
    BoundaryContinuityRequest, BoundaryContinuitySolution, BoundaryEndpoint, BoundaryTransition,
    ContinuityResource, ContinuitySolveError, ContinuitySolveReport, ContinuitySolverConfig,
    ContinuityTermination, OrderResidual,
};

#[cfg(test)]
mod tests;

/// Deterministic variational boundary-continuity solver.
#[derive(Clone, Debug, PartialEq)]
pub struct BoundaryContinuitySolver {
    config: ContinuitySolverConfig,
    resource_budget: ContinuityResourceBudget,
}

impl BoundaryContinuitySolver {
    /// Creates a solver after validating all convergence controls.
    ///
    /// # Errors
    ///
    /// Returns [`ContinuitySolveError::InvalidConfig`] when a convergence,
    /// damping, sampling, rank, or regularization control is invalid, and
    /// [`ContinuitySolveError::ResourceLimitExceeded`] when the requested
    /// iteration count exceeds the default resource budget.
    pub fn new(config: ContinuitySolverConfig) -> Result<Self, ContinuitySolveError> {
        Self::new_with_resource_budget(config, ContinuityResourceBudget::default())
    }

    /// Creates a solver with an explicit dense-work resource budget.
    ///
    /// # Errors
    ///
    /// Returns [`ContinuitySolveError::InvalidConfig`] when the solver controls
    /// are invalid, and
    /// [`ContinuitySolveError::ResourceLimitExceeded`] when the requested
    /// iteration count exceeds the supplied budget.
    pub fn new_with_resource_budget(
        config: ContinuitySolverConfig,
        resource_budget: ContinuityResourceBudget,
    ) -> Result<Self, ContinuitySolveError> {
        config.validate()?;
        resource_budget.ensure(ContinuityResource::Iterations, config.max_iterations())?;
        Ok(Self {
            config,
            resource_budget,
        })
    }

    /// Returns the validated solver configuration.
    pub const fn config(&self) -> &ContinuitySolverConfig {
        &self.config
    }

    /// Returns the validated dense-work resource budget.
    pub const fn resource_budget(&self) -> &ContinuityResourceBudget {
        &self.resource_budget
    }

    /// Solves one boundary-continuity request without mutating either input.
    ///
    /// The first surface is the fixed master. The returned solution owns an
    /// unchanged clone of that surface and a solved clone of the second
    /// surface, so callers can apply both outputs transactionally.
    ///
    /// # Errors
    ///
    /// Returns a typed [`ContinuitySolveError`] when either surface cannot
    /// represent the requested order, a checked work dimension exceeds the
    /// resource budget, the sampled boundary is invalid, or the nonlinear
    /// solve does not meet every requested tolerance.
    pub fn solve(
        &self,
        first: &crate::nurbs::NurbsSurface<crate::base::Vector4>,
        second: &crate::nurbs::NurbsSurface<crate::base::Vector4>,
        request: BoundaryContinuityRequest,
    ) -> Result<BoundaryContinuitySolution, ContinuitySolveError> {
        lm::solve(first, second, request, &self.config, self.resource_budget)
    }
}
