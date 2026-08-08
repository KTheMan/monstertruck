//! Bounded-work controls and meters for dense continuity solving.

use std::cell::Cell;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

use thiserror::Error;

use super::types::{ContinuityResource, ContinuitySolveError};

/// Finite work limits applied by [`BoundaryContinuitySolver`](super::BoundaryContinuitySolver).
///
/// The defaults bound every caller-controlled dimension that contributes to
/// sampling or dense linear algebra. Use [`Self::unbounded`] only when a
/// trusted caller applies an equivalent external budget.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct ContinuityWorkBudget {
    max_iterations: usize,
    max_control_points: usize,
    max_spans: usize,
    max_samples: usize,
    max_variables: usize,
    max_residuals: usize,
    max_jacobian_elements: usize,
    max_qr_elements: usize,
}

impl Default for ContinuityWorkBudget {
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

impl ContinuityWorkBudget {
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
    pub const fn max_iterations(self) -> usize { self.max_iterations }

    /// Returns the combined surface control-point limit.
    pub const fn max_control_points(self) -> usize { self.max_control_points }

    /// Returns the combined nonzero seam-span limit.
    pub const fn max_spans(self) -> usize { self.max_spans }

    /// Returns the combined optimizer and certification sample limit.
    pub const fn max_samples(self) -> usize { self.max_samples }

    /// Returns the optimization-variable limit.
    pub const fn max_variables(self) -> usize { self.max_variables }

    /// Returns the combined optimizer and certification residual limit.
    pub const fn max_residuals(self) -> usize { self.max_residuals }

    /// Returns the optimizer Jacobian element limit.
    pub const fn max_jacobian_elements(self) -> usize { self.max_jacobian_elements }

    /// Returns the augmented QR matrix element limit.
    pub const fn max_qr_elements(self) -> usize { self.max_qr_elements }

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
        let budget = match resource {
            ContinuityResource::Iterations => self.max_iterations,
            ContinuityResource::ControlPoints => self.max_control_points,
            ContinuityResource::Spans => self.max_spans,
            ContinuityResource::Samples => self.max_samples,
            ContinuityResource::Variables => self.max_variables,
            ContinuityResource::Residuals => self.max_residuals,
            ContinuityResource::JacobianElements => self.max_jacobian_elements,
            ContinuityResource::QrElements => self.max_qr_elements,
        };
        if requested <= budget {
            Ok(())
        } else {
            mark_continuity_truncated();
            Err(ContinuityTruncated {
                resource,
                requested,
                budget,
            }
            .into())
        }
    }
}

/// What continuity solves on one thread cost since the meter was last cleared.
///
/// Counts are deterministic dense-work units rather than elapsed time.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub struct ContinuityWork {
    /// Nonlinear iterations attempted.
    pub iterations: u64,
    /// Dense Jacobian elements consumed by attempted iterations.
    pub jacobian_elements: u64,
    /// Dense augmented QR elements consumed by attempted iterations.
    pub qr_elements: u64,
    /// Whether a checked dimension exhausted its budget.
    pub truncated: bool,
}

thread_local! {
    static CONTINUITY_WORK: Cell<ContinuityWork> = const { Cell::new(ContinuityWork {
        iterations: 0,
        jacobian_elements: 0,
        qr_elements: 0,
        truncated: false,
    }) };
}

static CONTINUITY_ITERATIONS_TOTAL: AtomicU64 = AtomicU64::new(0);
static CONTINUITY_JACOBIAN_ELEMENTS_TOTAL: AtomicU64 = AtomicU64::new(0);
static CONTINUITY_QR_ELEMENTS_TOTAL: AtomicU64 = AtomicU64::new(0);
static CONTINUITY_TRUNCATIONS_TOTAL: AtomicU64 = AtomicU64::new(0);
static CONTINUITY_ITERATIONS_MAX: AtomicU64 = AtomicU64::new(0);
static CONTINUITY_JACOBIAN_ELEMENTS_MAX: AtomicU64 = AtomicU64::new(0);
static CONTINUITY_QR_ELEMENTS_MAX: AtomicU64 = AtomicU64::new(0);

/// Reads work charged on this thread without clearing it.
#[must_use]
pub fn continuity_work() -> ContinuityWork { CONTINUITY_WORK.with(Cell::get) }

/// Reads and clears work charged on this thread.
pub fn take_continuity_work() -> ContinuityWork {
    CONTINUITY_WORK.replace(ContinuityWork::default())
}

/// Reads process-wide work and refusal totals across every thread.
#[must_use]
pub fn continuity_totals() -> (ContinuityWork, u64) {
    let truncations = CONTINUITY_TRUNCATIONS_TOTAL.load(Relaxed);
    (
        ContinuityWork {
            iterations: CONTINUITY_ITERATIONS_TOTAL.load(Relaxed),
            jacobian_elements: CONTINUITY_JACOBIAN_ELEMENTS_TOTAL.load(Relaxed),
            qr_elements: CONTINUITY_QR_ELEMENTS_TOTAL.load(Relaxed),
            truncated: truncations != 0,
        },
        truncations,
    )
}

/// Zeroes the process-wide work and refusal totals and returns what they held.
pub fn take_continuity_totals() -> (ContinuityWork, u64) {
    let truncations = CONTINUITY_TRUNCATIONS_TOTAL.swap(0, Relaxed);
    (
        ContinuityWork {
            iterations: CONTINUITY_ITERATIONS_TOTAL.swap(0, Relaxed),
            jacobian_elements: CONTINUITY_JACOBIAN_ELEMENTS_TOTAL.swap(0, Relaxed),
            qr_elements: CONTINUITY_QR_ELEMENTS_TOTAL.swap(0, Relaxed),
            truncated: truncations != 0,
        },
        truncations,
    )
}

/// Reads the high-water mark for one complete continuity solve.
#[must_use]
pub fn continuity_max_work() -> ContinuityWork {
    ContinuityWork {
        iterations: CONTINUITY_ITERATIONS_MAX.load(Relaxed),
        jacobian_elements: CONTINUITY_JACOBIAN_ELEMENTS_MAX.load(Relaxed),
        qr_elements: CONTINUITY_QR_ELEMENTS_MAX.load(Relaxed),
        truncated: false,
    }
}

/// Zeroes the complete-solve high-water marks and returns what they held.
pub fn take_continuity_max_work() -> ContinuityWork {
    ContinuityWork {
        iterations: CONTINUITY_ITERATIONS_MAX.swap(0, Relaxed),
        jacobian_elements: CONTINUITY_JACOBIAN_ELEMENTS_MAX.swap(0, Relaxed),
        qr_elements: CONTINUITY_QR_ELEMENTS_MAX.swap(0, Relaxed),
        truncated: false,
    }
}

pub(super) fn charge_continuity_work(work: ContinuityWork) {
    CONTINUITY_WORK.with(|cell| {
        let current = cell.get();
        cell.set(ContinuityWork {
            iterations: current.iterations.saturating_add(work.iterations),
            jacobian_elements: current
                .jacobian_elements
                .saturating_add(work.jacobian_elements),
            qr_elements: current.qr_elements.saturating_add(work.qr_elements),
            truncated: current.truncated || work.truncated,
        });
    });
    CONTINUITY_ITERATIONS_TOTAL.fetch_add(work.iterations, Relaxed);
    CONTINUITY_JACOBIAN_ELEMENTS_TOTAL.fetch_add(work.jacobian_elements, Relaxed);
    CONTINUITY_QR_ELEMENTS_TOTAL.fetch_add(work.qr_elements, Relaxed);
    if work.truncated {
        CONTINUITY_TRUNCATIONS_TOTAL.fetch_add(1, Relaxed);
    }
}

pub(super) struct ContinuityWorkSession {
    start: ContinuityWork,
}

impl ContinuityWorkSession {
    pub(super) fn begin() -> Self {
        Self {
            start: continuity_work(),
        }
    }
}

impl Drop for ContinuityWorkSession {
    fn drop(&mut self) {
        let end = continuity_work();
        CONTINUITY_ITERATIONS_MAX.fetch_max(
            end.iterations.saturating_sub(self.start.iterations),
            Relaxed,
        );
        CONTINUITY_JACOBIAN_ELEMENTS_MAX.fetch_max(
            end.jacobian_elements
                .saturating_sub(self.start.jacobian_elements),
            Relaxed,
        );
        CONTINUITY_QR_ELEMENTS_MAX.fetch_max(
            end.qr_elements.saturating_sub(self.start.qr_elements),
            Relaxed,
        );
    }
}

fn mark_continuity_truncated() {
    charge_continuity_work(ContinuityWork {
        truncated: true,
        ..ContinuityWork::default()
    });
}

/// A checked continuity-work dimension exceeded its explicit budget.
#[derive(Clone, Copy, Debug, Error, Hash, PartialEq, Eq)]
#[error("continuity solver {resource:?} budget exhausted: requested {requested}, budget {budget}")]
pub struct ContinuityTruncated {
    /// Dimension that exceeded the budget.
    pub resource: ContinuityResource,
    /// Checked required count.
    pub requested: usize,
    /// Configured maximum count.
    pub budget: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn work_meter_accumulates_and_clears_on_the_current_thread() {
        take_continuity_work();
        charge_continuity_work(ContinuityWork {
            iterations: 2,
            jacobian_elements: 12,
            qr_elements: 20,
            truncated: false,
        });

        assert_eq!(
            take_continuity_work(),
            ContinuityWork {
                iterations: 2,
                jacobian_elements: 12,
                qr_elements: 20,
                truncated: false,
            }
        );
        assert_eq!(continuity_work(), ContinuityWork::default());
    }

    #[test]
    fn budget_refusal_is_typed_and_marks_the_work_meter() {
        take_continuity_work();
        let error = ContinuityWorkBudget::unbounded()
            .with_max_variables(3)
            .ensure(ContinuityResource::Variables, 4)
            .expect_err("the checked dimension exceeds its budget");

        assert_eq!(
            error,
            ContinuitySolveError::Truncated(ContinuityTruncated {
                resource: ContinuityResource::Variables,
                requested: 4,
                budget: 3,
            })
        );
        assert!(take_continuity_work().truncated);
    }
}
