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
//! G0 is the established solver target. G1 through G3 have procedural
//! polynomial and rational evidence and imported workflow execution, but still
//! require independent higher-order certification before a production-readiness
//! claim. G4 uses the same order-generic jet representation and residual
//! machinery but requires explicit experimental opt-in. Solver tolerances
//! belong only to the unclamped `f64` styling layer; topology sewing and
//! solidification tolerances are intentionally absent.
//!
//! [`crate::nurbs::continuity_solver::BoundaryContinuitySolver::solve`] and
//! [`crate::nurbs::continuity_solver::execute_boundary_continuity_contracts`] are transactional:
//! they borrow
//! inputs, borrow the unchanged master, and own only the solved dependent
//! surface. Replay resolves persistent semantic
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
    pub const fn config(&self) -> &ContinuitySolverConfig { &self.config }

    /// Returns the validated dense-work resource budget.
    pub const fn resource_budget(&self) -> &ContinuityResourceBudget { &self.resource_budget }

    /// Solves one boundary-continuity request without mutating either input.
    ///
    /// The first surface is the fixed master. The returned solution borrows
    /// that surface and owns a solved clone of the second surface, so callers
    /// can apply the modified output transactionally without copying the
    /// unchanged master.
    ///
    /// # Examples
    ///
    /// ```
    /// use monstertruck_geometry::base::Vector4;
    /// use monstertruck_geometry::nurbs::continuity::{
    ///     BoundaryAlignment, ContinuityOrder, SurfaceBoundary,
    /// };
    /// use monstertruck_geometry::nurbs::continuity_solver::{
    ///     BoundaryContinuityRequest, BoundaryContinuitySolver, ContinuitySolverConfig,
    /// };
    /// use monstertruck_geometry::nurbs::{BsplineSurface, KnotVector, NurbsSurface};
    ///
    /// let plane = |x_start| {
    ///     let control_points = vec![
    ///         vec![
    ///             Vector4::new(x_start, 0.0, 0.0, 1.0),
    ///             Vector4::new(x_start, 1.0, 0.0, 1.0),
    ///         ],
    ///         vec![
    ///             Vector4::new(x_start + 1.0, 0.0, 0.0, 1.0),
    ///             Vector4::new(x_start + 1.0, 1.0, 0.0, 1.0),
    ///         ],
    ///     ];
    ///     NurbsSurface::new(BsplineSurface::new(
    ///         (
    ///             KnotVector::bezier_knot(1),
    ///             KnotVector::bezier_knot(1),
    ///         ),
    ///         control_points,
    ///     ))
    /// };
    /// let first = plane(-1.0);
    /// let second = plane(0.0);
    /// let request = BoundaryContinuityRequest::new(
    ///     SurfaceBoundary::UEnd,
    ///     SurfaceBoundary::UStart,
    ///     BoundaryAlignment::Aligned,
    ///     ContinuityOrder::G0,
    /// );
    /// let solver = BoundaryContinuitySolver::new(ContinuitySolverConfig::default())?;
    /// let solution = solver.solve(&first, &second, request)?;
    ///
    /// assert_eq!(solution.first(), &first);
    /// assert_eq!(solution.second(), &second);
    /// # Ok::<(), monstertruck_geometry::nurbs::continuity_solver::ContinuitySolveError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns a typed [`ContinuitySolveError`] when either surface cannot
    /// represent the requested order, a checked work dimension exceeds the
    /// resource budget, the sampled boundary is invalid, or the nonlinear
    /// solve does not meet every requested tolerance.
    pub fn solve<'first>(
        &self,
        first: &'first crate::nurbs::NurbsSurface<crate::base::Vector4>,
        second: &crate::nurbs::NurbsSurface<crate::base::Vector4>,
        request: BoundaryContinuityRequest,
    ) -> Result<BoundaryContinuitySolution<'first>, ContinuitySolveError> {
        lm::solve(first, second, request, &self.config, self.resource_budget)
    }
}
