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

    /// Returns whether this order is experimental.
    #[inline(always)]
    pub const fn is_experimental(self) -> bool { self.0 == Self::G4.0 }
}

impl TryFrom<usize> for ContinuityOrder {
    type Error = UnsupportedContinuityOrder;

    fn try_from(value: usize) -> Result<Self, Self::Error> { Self::new(value) }
}

impl From<ContinuityOrder> for usize {
    fn from(value: ContinuityOrder) -> Self { value.as_usize() }
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

/// A representation-specific capability report for one full surface side.
///
/// Concrete surface implementations determine support using their own degree,
/// knot, control-net, and representation requirements. This report carries
/// that determination without embedding those rules in the trait crate. It
/// does not establish compatibility with another surface or feasibility for a
/// numerical solver. A report for `G4` remains experimental.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct SurfaceContinuityCapability {
    side: BoundarySide,
    requested: ContinuityOrder,
    supported: bool,
}

impl SurfaceContinuityCapability {
    /// Reports that a representation supports the requested side and order.
    pub const fn supported(side: BoundarySide, requested: ContinuityOrder) -> Self {
        Self {
            side,
            requested,
            supported: true,
        }
    }

    /// Reports that a representation does not support the requested side and
    /// order.
    pub const fn unsupported(side: BoundarySide, requested: ContinuityOrder) -> Self {
        Self {
            side,
            requested,
            supported: false,
        }
    }

    /// Returns the inspected surface side.
    pub const fn side(self) -> BoundarySide { self.side }

    /// Returns the requested continuity order.
    pub const fn requested(self) -> ContinuityOrder { self.requested }

    /// Returns whether the inspected representation supports the request.
    ///
    /// This is a representation capability, not a solver-feasibility or
    /// two-surface compatibility result.
    pub const fn is_supported(self) -> bool { self.supported }
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
        assert!(!ContinuityOrder::G0.is_experimental());
        assert!(!ContinuityOrder::G3.is_experimental());
        assert!(ContinuityOrder::G4.is_experimental());
    }

    #[test]
    fn checked_order_conversions_preserve_the_order() {
        [
            ContinuityOrder::G0,
            ContinuityOrder::G1,
            ContinuityOrder::G2,
            ContinuityOrder::G3,
            ContinuityOrder::G4,
        ]
        .into_iter()
        .enumerate()
        .for_each(|(order, checked)| {
            assert_eq!(ContinuityOrder::try_from(order), Ok(checked));
            assert_eq!(usize::from(checked), order);
            assert_eq!(checked.as_usize(), order);
        });
    }

    #[test]
    fn capability_reports_preserve_every_side_and_status() {
        [
            BoundarySide::MinU,
            BoundarySide::MaxU,
            BoundarySide::MinV,
            BoundarySide::MaxV,
        ]
        .into_iter()
        .for_each(|side| {
            let supported = SurfaceContinuityCapability::supported(side, ContinuityOrder::G3);
            let unsupported = SurfaceContinuityCapability::unsupported(side, ContinuityOrder::G4);

            assert_eq!(supported.side(), side);
            assert_eq!(supported.requested(), ContinuityOrder::G3);
            assert!(supported.is_supported());
            assert_eq!(unsupported.side(), side);
            assert_eq!(unsupported.requested(), ContinuityOrder::G4);
            assert!(!unsupported.is_supported());
        });
    }
}
