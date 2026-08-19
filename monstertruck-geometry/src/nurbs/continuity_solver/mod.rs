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
//! G0 is the established solver target. G1 through G3 have independent
//! finite-difference certification over procedural polynomial and rational
//! fixtures, but still require external-kernel and production-model evidence
//! before a production-readiness claim. G4 uses the same
//! order-generic implementation machinery but requires explicit experimental
//! opt-in. Solver tolerances belong only to the unclamped `f64` styling layer;
//! topology sewing and solidification tolerances are intentionally absent.
//!
//! This first solver deliberately targets the established `f64` trait family.
//! The scalar-generic `v2` traits remain scaffolding and do not yet expose the
//! arbitrary-order derivative contract required here, so a later `v2` port is
//! an explicit migration rather than an implicit claim of genericity.
//!
//! [`crate::nurbs::continuity_solver::BoundaryContinuitySolver::solve`] is
//! transactional: it borrows both inputs, borrows the unchanged master in the
//! result, and owns the solved dependent surface, immutable transition, and
//! solve report.

mod boundary;
mod dual;
mod lm;
mod problem;
mod qr;
mod resource;
mod sampling;
mod taylor;
mod types;

pub use resource::{
    BudgetedContinuitySolve, ContinuityLimits, ContinuityTruncated, ContinuityWork,
    continuity_max_work, continuity_totals, continuity_work, take_continuity_max_work,
    take_continuity_totals, take_continuity_work,
};
pub use types::{
    BoundaryContinuityRequest, BoundaryContinuitySolution, BoundaryEndpoint, BoundaryTransition,
    BoundaryTransitionEvaluationError, ContinuityResource, ContinuitySolveError,
    ContinuitySolveReport, ContinuitySolverConfig, ContinuityTermination, OrderResidual,
};

#[cfg(test)]
mod tests;

/// Deterministic variational boundary-continuity solver.
#[derive(Clone, Debug, PartialEq)]
pub struct BoundaryContinuitySolver {
    config: ContinuitySolverConfig,
}

impl BoundaryContinuitySolver {
    /// Creates a solver after validating all convergence controls.
    ///
    /// # Errors
    ///
    /// Returns [`ContinuitySolveError::InvalidConfig`] when a convergence,
    /// damping, sampling, rank, or regularization control is invalid.
    pub fn new(config: ContinuitySolverConfig) -> Result<Self, ContinuitySolveError> {
        config.validate()?;
        Ok(Self { config })
    }

    /// Returns the validated solver configuration.
    pub const fn config(&self) -> &ContinuitySolverConfig { &self.config }

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
    ///     BoundaryAlignment, ContinuityOrder, BoundarySide,
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
    ///     BoundarySide::MaxU,
    ///     BoundarySide::MinU,
    ///     BoundaryAlignment::Aligned,
    ///     ContinuityOrder::G0,
    /// );
    /// let solver = BoundaryContinuitySolver::new(ContinuitySolverConfig::default())?;
    /// let solution = solver.solve(&first, &second, request)?;
    ///
    /// assert_eq!(solution.first(), &first);
    /// assert_eq!(solution.second(), &second);
    /// assert_eq!(solution.transition().mapped_seam_coordinate(0.5)?, 0.5);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
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
        self.solve_with_budget(first, second, request, ContinuityLimits::default())
            .outcome
    }

    /// Solves with explicit deterministic work limits and reports actual work.
    ///
    /// Unlike [`Self::solve`], budget exhaustion is retained in the
    /// [`BudgetedContinuitySolve`] carrier for headroom studies. The carrier
    /// never returns partially solved geometry: [`BudgetedContinuitySolve::outcome`]
    /// is an error whenever [`BudgetedContinuitySolve::truncated`] is present.
    pub fn solve_with_budget<'first>(
        &self,
        first: &'first crate::nurbs::NurbsSurface<crate::base::Vector4>,
        second: &crate::nurbs::NurbsSurface<crate::base::Vector4>,
        request: BoundaryContinuityRequest,
        limits: ContinuityLimits,
    ) -> BudgetedContinuitySolve<'first> {
        let start = continuity_work();
        let budget = resource::ContinuityBudget::new(limits, start);
        let outcome = lm::solve(first, second, request, &self.config, budget);
        let truncated = match &outcome {
            Err(ContinuitySolveError::Truncated(truncated)) => Some(*truncated),
            _ => None,
        };
        let work = resource::continuity_work_since(start, truncated.is_some());
        BudgetedContinuitySolve {
            outcome,
            work,
            truncated,
        }
    }
}
