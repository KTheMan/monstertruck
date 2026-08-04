//! Public configuration, diagnostics, and failures for continuity solving.

use super::super::continuity::{
    BoundaryAlignment, BoundarySide, ContinuityMaturity, ContinuityOrder,
};

/// Identifies one endpoint of a boundary-continuity problem.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum BoundaryEndpoint {
    /// The fixed reference surface.
    First,
    /// The surface whose boundary strip is optimized.
    Second,
}

/// Caller-controlled dimension constrained by a [`ContinuityBudget`](super::ContinuityBudget).
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum ContinuityResource {
    /// Nonlinear solver iterations.
    Iterations,
    /// Combined input-surface control points.
    ControlPoints,
    /// Combined nonzero seam knot spans.
    Spans,
    /// Optimizer and independent validation samples.
    Samples,
    /// Dense automatic-differentiation variables.
    Variables,
    /// Optimizer and independent validation residuals.
    Residuals,
    /// Dense optimizer Jacobian elements.
    JacobianElements,
    /// Dense augmented QR matrix elements.
    QrElements,
}

/// Geometric-continuity request between two tensor-product surface boundaries.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct BoundaryContinuityRequest {
    first_side: BoundarySide,
    second_side: BoundarySide,
    alignment: BoundaryAlignment,
    order: ContinuityOrder,
}

impl BoundaryContinuityRequest {
    /// Creates a geometric-continuity request.
    pub const fn new(
        first_side: BoundarySide,
        second_side: BoundarySide,
        alignment: BoundaryAlignment,
        order: ContinuityOrder,
    ) -> Self {
        Self {
            first_side,
            second_side,
            alignment,
            order,
        }
    }

    /// Returns the boundary on the fixed reference surface.
    pub const fn first_side(self) -> BoundarySide { self.first_side }

    /// Returns the boundary on the optimized surface.
    pub const fn second_side(self) -> BoundarySide { self.second_side }

    /// Returns the seam-parameter alignment.
    pub const fn alignment(self) -> BoundaryAlignment { self.alignment }

    /// Returns the requested geometric-continuity order.
    pub const fn order(self) -> ContinuityOrder { self.order }
}

/// Deterministic convergence controls for [`BoundaryContinuitySolver`](super::BoundaryContinuitySolver).
///
/// These are styling-space tolerances. They are intentionally independent of
/// topology sewing and solidification tolerances.
#[derive(Clone, Debug, PartialEq)]
pub struct ContinuitySolverConfig {
    max_iterations: usize,
    samples_per_span: usize,
    transition_degree: usize,
    order_tolerances: [f64; 5],
    initial_damping: f64,
    minimum_damping: f64,
    maximum_damping: f64,
    rank_tolerance: f64,
    fairness_weight: f64,
    anchor_weight: f64,
    transition_weight: f64,
    minimum_weight: f64,
    allow_experimental_g4: bool,
}

impl Default for ContinuitySolverConfig {
    fn default() -> Self {
        Self {
            max_iterations: 40,
            samples_per_span: 3,
            transition_degree: 3,
            order_tolerances: [1.0e-9, 1.0e-8, 1.0e-7, 1.0e-6, 1.0e-5],
            initial_damping: 1.0e-3,
            minimum_damping: 1.0e-12,
            maximum_damping: 1.0e12,
            rank_tolerance: 1.0e-12,
            fairness_weight: 1.0e-4,
            anchor_weight: 1.0e-8,
            transition_weight: 1.0e-6,
            minimum_weight: 1.0e-10,
            allow_experimental_g4: false,
        }
    }
}

impl ContinuitySolverConfig {
    /// Returns the maximum number of nonlinear iterations.
    pub const fn max_iterations(&self) -> usize { self.max_iterations }

    /// Returns the number of optimizer Gauss--Legendre samples per seam span.
    ///
    /// Independent post-validation uses a denser degree-aware grid.
    pub const fn samples_per_span(&self) -> usize { self.samples_per_span }

    /// Returns the Bernstein degree used for seam-transition fields.
    pub const fn transition_degree(&self) -> usize { self.transition_degree }

    /// Returns the normalized residual tolerance for `order`.
    pub fn tolerance(&self, order: ContinuityOrder) -> f64 {
        self.order_tolerances[order.as_usize()]
    }

    /// Returns the initial Levenberg--Marquardt damping.
    pub const fn initial_damping(&self) -> f64 { self.initial_damping }

    /// Returns the minimum Levenberg--Marquardt damping.
    pub const fn minimum_damping(&self) -> f64 { self.minimum_damping }

    /// Returns the maximum Levenberg--Marquardt damping.
    pub const fn maximum_damping(&self) -> f64 { self.maximum_damping }

    /// Returns the relative rank threshold for column-pivoted QR.
    pub const fn rank_tolerance(&self) -> f64 { self.rank_tolerance }

    /// Returns the thin-strip fairness weight.
    pub const fn fairness_weight(&self) -> f64 { self.fairness_weight }

    /// Returns the control-point displacement anchor weight.
    pub const fn anchor_weight(&self) -> f64 { self.anchor_weight }

    /// Returns the seam-reparameterization regularization weight.
    pub const fn transition_weight(&self) -> f64 { self.transition_weight }

    /// Returns the minimum accepted rational weight.
    pub const fn minimum_weight(&self) -> f64 { self.minimum_weight }

    /// Returns whether experimental G4 solving is enabled.
    pub const fn allows_experimental_g4(&self) -> bool { self.allow_experimental_g4 }

    /// Sets the maximum nonlinear iteration count.
    pub const fn with_max_iterations(mut self, max_iterations: usize) -> Self {
        self.max_iterations = max_iterations;
        self
    }

    /// Sets the number of Gauss--Legendre samples per nonzero seam span.
    pub const fn with_samples_per_span(mut self, samples_per_span: usize) -> Self {
        self.samples_per_span = samples_per_span;
        self
    }

    /// Sets the Bernstein degree of seam-transition fields.
    pub const fn with_transition_degree(mut self, transition_degree: usize) -> Self {
        self.transition_degree = transition_degree;
        self
    }

    /// Sets one normalized derivative-order tolerance.
    pub fn with_tolerance(mut self, order: ContinuityOrder, tolerance: f64) -> Self {
        self.order_tolerances[order.as_usize()] = tolerance;
        self
    }

    /// Sets the control-point displacement anchor weight.
    pub const fn with_anchor_weight(mut self, anchor_weight: f64) -> Self {
        self.anchor_weight = anchor_weight;
        self
    }

    /// Sets the thin-strip fairness weight.
    pub const fn with_fairness_weight(mut self, fairness_weight: f64) -> Self {
        self.fairness_weight = fairness_weight;
        self
    }

    /// Sets the seam-transition regularization weight.
    pub const fn with_transition_weight(mut self, transition_weight: f64) -> Self {
        self.transition_weight = transition_weight;
        self
    }

    /// Sets the initial Levenberg--Marquardt damping.
    pub const fn with_initial_damping(mut self, initial_damping: f64) -> Self {
        self.initial_damping = initial_damping;
        self
    }

    /// Sets the minimum Levenberg--Marquardt damping.
    pub const fn with_minimum_damping(mut self, minimum_damping: f64) -> Self {
        self.minimum_damping = minimum_damping;
        self
    }

    /// Sets the maximum Levenberg--Marquardt damping.
    pub const fn with_maximum_damping(mut self, maximum_damping: f64) -> Self {
        self.maximum_damping = maximum_damping;
        self
    }

    /// Sets the relative numerical-rank threshold.
    pub const fn with_rank_tolerance(mut self, rank_tolerance: f64) -> Self {
        self.rank_tolerance = rank_tolerance;
        self
    }

    /// Sets the minimum accepted homogeneous weight.
    pub const fn with_minimum_weight(mut self, minimum_weight: f64) -> Self {
        self.minimum_weight = minimum_weight;
        self
    }

    /// Enables or disables experimental G4 solving.
    pub const fn with_experimental_g4(mut self, enabled: bool) -> Self {
        self.allow_experimental_g4 = enabled;
        self
    }

    pub(super) fn validate(&self) -> Result<(), ContinuitySolveError> {
        let positive_finite = |value: f64| value.is_finite() && value > 0.0;
        if self.max_iterations == 0 {
            Err(ContinuitySolveError::InvalidConfig(
                "max_iterations must be positive",
            ))
        } else if self.samples_per_span == 0 || self.samples_per_span > 64 {
            Err(ContinuitySolveError::InvalidConfig(
                "samples_per_span must be in 1..=64",
            ))
        } else if self.transition_degree == 0 || self.transition_degree > 64 {
            Err(ContinuitySolveError::InvalidConfig(
                "transition_degree must be in 1..=64",
            ))
        } else if self
            .order_tolerances
            .iter()
            .any(|&value| !positive_finite(value))
        {
            Err(ContinuitySolveError::InvalidConfig(
                "order tolerances must be positive and finite",
            ))
        } else if !positive_finite(self.initial_damping)
            || !positive_finite(self.minimum_damping)
            || !positive_finite(self.maximum_damping)
            || self.minimum_damping > self.initial_damping
            || self.initial_damping > self.maximum_damping
        {
            Err(ContinuitySolveError::InvalidConfig(
                "damping bounds must be positive, finite, and ordered",
            ))
        } else if !positive_finite(self.rank_tolerance)
            || !positive_finite(self.minimum_weight)
            || [
                self.fairness_weight,
                self.anchor_weight,
                self.transition_weight,
            ]
            .into_iter()
            .any(|value| !value.is_finite() || value < 0.0)
        {
            Err(ContinuitySolveError::InvalidConfig(
                "rank, weight, and regularization controls are invalid",
            ))
        } else {
            Ok(())
        }
    }
}

/// Solver termination state.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum ContinuityTermination {
    /// Every requested derivative-order residual met its tolerance.
    Converged,
    /// The maximum iteration count was reached.
    MaximumIterations,
    /// No finite descent step could be accepted.
    NoDescent,
}

/// Residual diagnostics for one geometric-continuity order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OrderResidual {
    order: ContinuityOrder,
    rms: f64,
    maximum: f64,
    worst_sample: usize,
    validation_sample: bool,
    cross_derivative: usize,
    seam_derivative: usize,
}

impl OrderResidual {
    pub(super) const fn new(
        order: ContinuityOrder,
        rms: f64,
        maximum: f64,
        worst_sample: usize,
        validation_sample: bool,
        cross_derivative: usize,
        seam_derivative: usize,
    ) -> Self {
        Self {
            order,
            rms,
            maximum,
            worst_sample,
            validation_sample,
            cross_derivative,
            seam_derivative,
        }
    }

    /// Returns the derivative order.
    pub const fn order(self) -> ContinuityOrder { self.order }

    /// Returns the root-mean-square normalized residual.
    pub const fn rms(self) -> f64 { self.rms }

    /// Returns the maximum normalized residual.
    pub const fn maximum(self) -> f64 { self.maximum }

    /// Returns the index of the sample with the maximum residual.
    pub const fn worst_sample(self) -> usize { self.worst_sample }

    /// Returns whether `worst_sample` belongs to the independent validation grid.
    pub const fn is_validation_sample(self) -> bool { self.validation_sample }

    /// Returns the cross-boundary derivative order of the worst residual.
    pub const fn cross_derivative(self) -> usize { self.cross_derivative }

    /// Returns the seam-direction derivative order of the worst residual.
    pub const fn seam_derivative(self) -> usize { self.seam_derivative }
}

/// Deterministic diagnostics from one continuity solve.
#[derive(Clone, Debug, PartialEq)]
pub struct ContinuitySolveReport {
    termination: ContinuityTermination,
    maturity: ContinuityMaturity,
    iterations: usize,
    accepted_steps: usize,
    rejected_steps: usize,
    initial_objective: f64,
    final_objective: f64,
    residuals: Vec<OrderResidual>,
    gradient_infinity_norm: f64,
    step_norm: f64,
    damping: f64,
    numerical_rank: usize,
    variable_count: usize,
    residual_count: usize,
}

impl ContinuitySolveReport {
    pub(super) fn from_data(data: ContinuitySolveReportData) -> Self {
        Self {
            termination: data.termination,
            maturity: data.order.maturity(),
            iterations: data.iterations,
            accepted_steps: data.accepted_steps,
            rejected_steps: data.rejected_steps,
            initial_objective: data.initial_objective,
            final_objective: data.final_objective,
            residuals: data.residuals,
            gradient_infinity_norm: data.gradient_infinity_norm,
            step_norm: data.step_norm,
            damping: data.damping,
            numerical_rank: data.numerical_rank,
            variable_count: data.variable_count,
            residual_count: data.residual_count,
        }
    }

    /// Returns the termination state.
    pub const fn termination(&self) -> ContinuityTermination { self.termination }

    /// Returns the public maturity classification of the solved order.
    pub const fn maturity(&self) -> ContinuityMaturity { self.maturity }

    /// Returns the number of attempted nonlinear iterations.
    pub const fn iterations(&self) -> usize { self.iterations }

    /// Returns the number of accepted steps.
    pub const fn accepted_steps(&self) -> usize { self.accepted_steps }

    /// Returns the number of rejected steps.
    pub const fn rejected_steps(&self) -> usize { self.rejected_steps }

    /// Returns the initial scaled least-squares objective.
    pub const fn initial_objective(&self) -> f64 { self.initial_objective }

    /// Returns the final scaled least-squares objective.
    pub const fn final_objective(&self) -> f64 { self.final_objective }

    /// Returns per-order residual diagnostics.
    pub fn residuals(&self) -> &[OrderResidual] { &self.residuals }

    /// Returns the final gradient infinity norm.
    pub const fn gradient_infinity_norm(&self) -> f64 { self.gradient_infinity_norm }

    /// Returns the final step norm.
    pub const fn step_norm(&self) -> f64 { self.step_norm }

    /// Returns the final damping value.
    pub const fn damping(&self) -> f64 { self.damping }

    /// Returns the numerical rank of the most recently solved damped augmented
    /// system.
    ///
    /// Returns zero when certification succeeds before the first linear solve.
    pub const fn numerical_rank(&self) -> usize { self.numerical_rank }

    /// Returns the number of optimization variables.
    pub const fn variable_count(&self) -> usize { self.variable_count }

    /// Returns the number of scalar residuals.
    pub const fn residual_count(&self) -> usize { self.residual_count }
}

pub(super) struct ContinuitySolveReportData {
    pub(super) termination: ContinuityTermination,
    pub(super) order: ContinuityOrder,
    pub(super) iterations: usize,
    pub(super) accepted_steps: usize,
    pub(super) rejected_steps: usize,
    pub(super) initial_objective: f64,
    pub(super) final_objective: f64,
    pub(super) residuals: Vec<OrderResidual>,
    pub(super) gradient_infinity_norm: f64,
    pub(super) step_norm: f64,
    pub(super) damping: f64,
    pub(super) numerical_rank: usize,
    pub(super) variable_count: usize,
    pub(super) residual_count: usize,
}

mod error;
mod solution;
mod transition;

pub use error::ContinuitySolveError;
pub use solution::BoundaryContinuitySolution;
pub use transition::BoundaryTransition;
