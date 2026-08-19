//! Transactional solver output.

use super::super::super::NurbsSurface;
use super::{BoundaryTransition, ContinuitySolveReport};
use crate::base::Vector4;
/// Transactional output from a successful continuity solve.
///
/// The unchanged first surface is borrowed from the solve input. The solved
/// second surface, immutable boundary transition, and solve report are owned by
/// the result.
#[derive(Clone, Debug, PartialEq)]
pub struct BoundaryContinuitySolution<'first> {
    first: &'first NurbsSurface<Vector4>,
    second: NurbsSurface<Vector4>,
    transition: BoundaryTransition,
    report: ContinuitySolveReport,
}

impl<'first> BoundaryContinuitySolution<'first> {
    pub(in crate::nurbs::continuity_solver) const fn new(
        first: &'first NurbsSurface<Vector4>,
        second: NurbsSurface<Vector4>,
        transition: BoundaryTransition,
        report: ContinuitySolveReport,
    ) -> Self {
        Self {
            first,
            second,
            transition,
            report,
        }
    }

    /// Returns the unchanged reference surface.
    pub const fn first(&self) -> &'first NurbsSurface<Vector4> { self.first }

    /// Returns the solved second surface.
    pub const fn second(&self) -> &NurbsSurface<Vector4> { &self.second }

    /// Returns the solved master-to-second coordinate transition.
    pub const fn transition(&self) -> &BoundaryTransition { &self.transition }

    /// Returns the convergence report.
    pub const fn report(&self) -> &ContinuitySolveReport { &self.report }

    #[doc(hidden)]
    /// Compatibility decomposition that discards the solved transition.
    pub fn into_parts(
        self,
    ) -> (
        &'first NurbsSurface<Vector4>,
        NurbsSurface<Vector4>,
        ContinuitySolveReport,
    ) {
        (self.first, self.second, self.report)
    }

    /// Consumes the result and returns both surfaces, the solved transition,
    /// and the report.
    pub fn into_parts_with_transition(
        self,
    ) -> (
        &'first NurbsSurface<Vector4>,
        NurbsSurface<Vector4>,
        BoundaryTransition,
        ContinuitySolveReport,
    ) {
        (self.first, self.second, self.transition, self.report)
    }
}
