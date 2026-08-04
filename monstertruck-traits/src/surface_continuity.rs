//! Checked geometric-continuity requests and surface capability diagnostics.
//!
//! These types are scalar-neutral trait foundations. Numerical solvers can use
//! them without making continuity order or boundary-side vocabulary specific
//! to one geometry representation.

use thiserror::Error;

/// Highest continuity order currently represented by the public API.
pub const MAX_CONTINUITY_ORDER: usize = 4;

/// A requested continuity order is outside the represented range.
#[derive(Clone, Copy, Debug, Error, Hash, PartialEq, Eq)]
#[error("continuity order {requested} exceeds the supported maximum {maximum}")]
pub struct UnsupportedContinuityOrder {
    requested: usize,
    maximum: usize,
}

impl UnsupportedContinuityOrder {
    /// Returns the rejected order.
    pub const fn requested(self) -> usize { self.requested }

    /// Returns the highest represented order.
    pub const fn maximum(self) -> usize { self.maximum }
}

/// Checked geometric-continuity order.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ContinuityOrder(u8);

impl ContinuityOrder {
    /// Positional continuity.
    pub const G0: Self = Self(0);
    /// Tangent-plane continuity.
    pub const G1: Self = Self(1);
    /// Curvature continuity.
    pub const G2: Self = Self(2);
    /// Third-order geometric continuity.
    pub const G3: Self = Self(3);
    /// Experimental fourth-order geometric continuity.
    ///
    /// Representability does not imply production solver support. Numerical
    /// solvers must require an explicit experimental opt-in before solving a
    /// `G4` request.
    pub const G4: Self = Self(4);

    /// Creates a checked continuity order.
    ///
    /// # Errors
    ///
    /// Returns [`UnsupportedContinuityOrder`] when `order` exceeds
    /// [`MAX_CONTINUITY_ORDER`].
    pub const fn new(order: usize) -> Result<Self, UnsupportedContinuityOrder> {
        if order <= MAX_CONTINUITY_ORDER {
            Ok(Self(order as u8))
        } else {
            Err(UnsupportedContinuityOrder {
                requested: order,
                maximum: MAX_CONTINUITY_ORDER,
            })
        }
    }

    /// Returns the numeric derivative order.
    #[inline(always)]
    pub const fn as_usize(self) -> usize { self.0 as usize }

    /// Returns the public maturity classification.
    #[inline(always)]
    pub const fn maturity(self) -> ContinuityMaturity {
        if self.0 == Self::G0.0 {
            ContinuityMaturity::Established
        } else if self.0 <= Self::G3.0 {
            ContinuityMaturity::Provisional
        } else {
            ContinuityMaturity::Experimental
        }
    }

    /// Returns the mathematical minimum cross-boundary degree.
    #[inline(always)]
    pub const fn minimum_degree(self) -> usize { self.as_usize() }

    /// Returns the preferred cross-boundary degree for constrained styling.
    #[inline(always)]
    pub const fn recommended_degree(self) -> usize {
        if self.0 == 0 {
            0
        } else {
            2 * self.as_usize() - 1
        }
    }

    /// Returns the number of boundary-adjacent control rows in a full jet.
    #[inline(always)]
    pub const fn constrained_rows(self) -> usize { self.as_usize() + 1 }
}

impl TryFrom<usize> for ContinuityOrder {
    type Error = UnsupportedContinuityOrder;

    fn try_from(value: usize) -> Result<Self, Self::Error> { Self::new(value) }
}

impl From<ContinuityOrder> for usize {
    fn from(value: ContinuityOrder) -> Self { value.as_usize() }
}

/// Public maturity classification for a continuity request.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum ContinuityMaturity {
    /// Established positional-continuity target.
    Established,
    /// Implemented higher-order target awaiting independent certification.
    Provisional,
    /// Experimental reachability outside the production target.
    Experimental,
}

/// Parameter axis of a tensor-product surface.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum SurfaceAxis {
    /// The surface's `u` parameter.
    U,
    /// The surface's `v` parameter.
    V,
}

/// Side of a full tensor-product surface parameter domain.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum BoundarySide {
    /// Side on which `u` is minimal.
    MinU,
    /// Side on which `u` is maximal.
    MaxU,
    /// Side on which `v` is minimal.
    MinV,
    /// Side on which `v` is maximal.
    MaxV,
}

impl BoundarySide {
    /// Returns the axis normal to this side in parameter space.
    #[inline(always)]
    pub const fn cross_axis(self) -> SurfaceAxis {
        match self {
            Self::MinU | Self::MaxU => SurfaceAxis::U,
            Self::MinV | Self::MaxV => SurfaceAxis::V,
        }
    }

    /// Returns the axis running along this side.
    #[inline(always)]
    pub const fn boundary_axis(self) -> SurfaceAxis {
        match self.cross_axis() {
            SurfaceAxis::U => SurfaceAxis::V,
            SurfaceAxis::V => SurfaceAxis::U,
        }
    }
}

/// Degree-based support level for a continuity request.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum ContinuityCapabilityLevel {
    /// The cross-boundary degree or control-row count is insufficient.
    Unsupported,
    /// The jet is representable but has limited fairness freedom.
    Feasible,
    /// The preferred styling degree is available.
    Recommended,
}

/// Degree and control-row diagnostics for one full surface side.
///
/// Capability reports mathematical representability only. In particular, a
/// feasible or recommended `G4` report remains experimental.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct SurfaceContinuityCapability {
    side: BoundarySide,
    requested: ContinuityOrder,
    cross_degree: usize,
    cross_control_rows: usize,
    level: ContinuityCapabilityLevel,
}

impl SurfaceContinuityCapability {
    /// Builds a capability report from generic tensor-product surface facts.
    pub const fn from_degrees_and_dimensions(
        degrees: (usize, usize),
        dimensions: (usize, usize),
        side: BoundarySide,
        requested: ContinuityOrder,
    ) -> Self {
        let (cross_degree, cross_control_rows) = match side.cross_axis() {
            SurfaceAxis::U => (degrees.0, dimensions.0),
            SurfaceAxis::V => (degrees.1, dimensions.1),
        };
        let level = if cross_degree < requested.minimum_degree()
            || cross_control_rows < requested.constrained_rows()
        {
            ContinuityCapabilityLevel::Unsupported
        } else if cross_degree < requested.recommended_degree() {
            ContinuityCapabilityLevel::Feasible
        } else {
            ContinuityCapabilityLevel::Recommended
        };
        Self {
            side,
            requested,
            cross_degree,
            cross_control_rows,
            level,
        }
    }

    /// Returns the inspected surface side.
    pub const fn side(self) -> BoundarySide { self.side }

    /// Returns the requested continuity order.
    pub const fn requested(self) -> ContinuityOrder { self.requested }

    /// Returns the request's maturity classification.
    pub const fn maturity(self) -> ContinuityMaturity { self.requested.maturity() }

    /// Returns the degree normal to the side in parameter space.
    pub const fn cross_degree(self) -> usize { self.cross_degree }

    /// Returns the number of control rows normal to the side.
    pub const fn cross_control_rows(self) -> usize { self.cross_control_rows }

    /// Returns the degree-based capability level.
    pub const fn level(self) -> ContinuityCapabilityLevel { self.level }

    /// Returns the control rows remaining beyond the constrained boundary jet.
    pub const fn fairness_rows(self) -> usize {
        self.cross_control_rows
            .saturating_sub(self.requested.constrained_rows())
    }

    /// Returns whether the requested jet is mathematically representable.
    pub const fn is_feasible(self) -> bool {
        !matches!(self.level, ContinuityCapabilityLevel::Unsupported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_order_rejects_values_outside_the_public_range() {
        let error = ContinuityOrder::new(MAX_CONTINUITY_ORDER + 1)
            .expect_err("orders above G4 must be rejected");

        assert_eq!(error.requested(), 5);
        assert_eq!(error.maximum(), MAX_CONTINUITY_ORDER);
        assert_eq!(
            ContinuityOrder::G0.maturity(),
            ContinuityMaturity::Established
        );
        assert_eq!(
            ContinuityOrder::G3.maturity(),
            ContinuityMaturity::Provisional
        );
        assert_eq!(
            ContinuityOrder::G4.maturity(),
            ContinuityMaturity::Experimental
        );
    }

    #[test]
    fn capability_uses_the_axis_normal_to_the_selected_side() {
        let u = SurfaceContinuityCapability::from_degrees_and_dimensions(
            (5, 2),
            (6, 3),
            BoundarySide::MinU,
            ContinuityOrder::G3,
        );
        let v = SurfaceContinuityCapability::from_degrees_and_dimensions(
            (5, 2),
            (6, 3),
            BoundarySide::MaxV,
            ContinuityOrder::G3,
        );

        assert_eq!(u.level(), ContinuityCapabilityLevel::Recommended);
        assert_eq!(v.level(), ContinuityCapabilityLevel::Unsupported);
        assert_eq!(u.cross_control_rows(), 6);
        assert_eq!(v.cross_control_rows(), 3);
    }
}
