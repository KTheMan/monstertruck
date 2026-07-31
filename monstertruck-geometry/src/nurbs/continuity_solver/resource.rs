//! Resource budgets for dense continuity solving.

use serde::{Deserialize, Serialize};

use super::types::{ContinuityResource, ContinuitySolveError};

/// Finite work limits applied by [`BoundaryContinuitySolver`](super::BoundaryContinuitySolver).
///
/// The defaults bound every caller-controlled dimension that contributes to
/// sampling or dense linear algebra. Use [`Self::unbounded`] only when a
/// trusted caller applies an equivalent external budget.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ContinuityResourceBudget {
    max_iterations: usize,
    max_control_points: usize,
    max_spans: usize,
    max_samples: usize,
    max_variables: usize,
    max_residuals: usize,
    max_jacobian_elements: usize,
    max_qr_elements: usize,
}

impl Default for ContinuityResourceBudget {
    fn default() -> Self {
        Self {
            max_iterations: 128,
            max_control_points: 1_048_576,
            max_spans: 4_096,
            max_samples: 32_768,
            max_variables: 2_048,
            max_residuals: 65_536,
            max_jacobian_elements: 4_194_304,
            max_qr_elements: 4_194_304,
        }
    }
}

impl ContinuityResourceBudget {
    /// Returns a budget with no intentional finite limits.
    ///
    /// Checked dimension arithmetic still rejects integer overflow.
    pub const fn unbounded() -> Self {
        Self {
            max_iterations: usize::MAX,
            max_control_points: usize::MAX,
            max_spans: usize::MAX,
            max_samples: usize::MAX,
            max_variables: usize::MAX,
            max_residuals: usize::MAX,
            max_jacobian_elements: usize::MAX,
            max_qr_elements: usize::MAX,
        }
    }

    /// Returns the nonlinear iteration limit.
    pub const fn max_iterations(self) -> usize {
        self.max_iterations
    }

    /// Returns the combined surface control-point limit.
    pub const fn max_control_points(self) -> usize {
        self.max_control_points
    }

    /// Returns the combined nonzero seam-span limit.
    pub const fn max_spans(self) -> usize {
        self.max_spans
    }

    /// Returns the combined optimizer and certification sample limit.
    pub const fn max_samples(self) -> usize {
        self.max_samples
    }

    /// Returns the optimization-variable limit.
    pub const fn max_variables(self) -> usize {
        self.max_variables
    }

    /// Returns the combined optimizer and certification residual limit.
    pub const fn max_residuals(self) -> usize {
        self.max_residuals
    }

    /// Returns the optimizer Jacobian element limit.
    pub const fn max_jacobian_elements(self) -> usize {
        self.max_jacobian_elements
    }

    /// Returns the augmented QR matrix element limit.
    pub const fn max_qr_elements(self) -> usize {
        self.max_qr_elements
    }

    /// Sets the nonlinear iteration limit.
    pub const fn with_max_iterations(mut self, limit: usize) -> Self {
        self.max_iterations = limit;
        self
    }

    /// Sets the combined surface control-point limit.
    pub const fn with_max_control_points(mut self, limit: usize) -> Self {
        self.max_control_points = limit;
        self
    }

    /// Sets the combined nonzero seam-span limit.
    pub const fn with_max_spans(mut self, limit: usize) -> Self {
        self.max_spans = limit;
        self
    }

    /// Sets the combined optimizer and certification sample limit.
    pub const fn with_max_samples(mut self, limit: usize) -> Self {
        self.max_samples = limit;
        self
    }

    /// Sets the optimization-variable limit.
    pub const fn with_max_variables(mut self, limit: usize) -> Self {
        self.max_variables = limit;
        self
    }

    /// Sets the combined optimizer and certification residual limit.
    pub const fn with_max_residuals(mut self, limit: usize) -> Self {
        self.max_residuals = limit;
        self
    }

    /// Sets the optimizer Jacobian element limit.
    pub const fn with_max_jacobian_elements(mut self, limit: usize) -> Self {
        self.max_jacobian_elements = limit;
        self
    }

    /// Sets the augmented QR matrix element limit.
    pub const fn with_max_qr_elements(mut self, limit: usize) -> Self {
        self.max_qr_elements = limit;
        self
    }

    pub(super) fn ensure(
        self,
        resource: ContinuityResource,
        requested: usize,
    ) -> Result<(), ContinuitySolveError> {
        let limit = match resource {
            ContinuityResource::Iterations => self.max_iterations,
            ContinuityResource::ControlPoints => self.max_control_points,
            ContinuityResource::Spans => self.max_spans,
            ContinuityResource::Samples => self.max_samples,
            ContinuityResource::Variables => self.max_variables,
            ContinuityResource::Residuals => self.max_residuals,
            ContinuityResource::JacobianElements => self.max_jacobian_elements,
            ContinuityResource::QrElements => self.max_qr_elements,
        };
        if requested <= limit {
            Ok(())
        } else {
            Err(ContinuitySolveError::ResourceLimitExceeded {
                resource,
                requested,
                limit,
            })
        }
    }
}
