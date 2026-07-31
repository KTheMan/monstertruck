//! Geometric-continuity orders and surface capability diagnostics.
//!
//! G0 through G3 are production targets. G4 is a validated representation
//! target with experimental solver maturity. The checked order type and
//! capability model deliberately avoid a G3-specific ceiling.

use super::{BsplineSurface, NurbsSurface};
use crate::errors::{Error, Result};
use serde::{Deserialize, Deserializer, Serialize};

/// Highest continuity order currently validated by the kernel.
///
/// Raising this limit does not change the serialized representation of
/// [`ContinuityOrder`].
pub const MAX_CONTINUITY_ORDER: usize = 4;

/// Checked geometric-continuity order.
///
/// The serialized value is numeric so future kernel versions can extend the
/// accepted range without changing continuity contracts.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
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
    /// Fourth-order geometric continuity.
    pub const G4: Self = Self(4);

    /// Creates a checked continuity order.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnsupportedContinuityOrder`] when `order` exceeds
    /// [`MAX_CONTINUITY_ORDER`].
    pub fn new(order: usize) -> Result<Self> {
        if order <= MAX_CONTINUITY_ORDER {
            Ok(Self(order as u8))
        } else {
            Err(Error::UnsupportedContinuityOrder(
                order,
                MAX_CONTINUITY_ORDER,
            ))
        }
    }

    /// Returns the numeric derivative order.
    #[inline(always)]
    pub const fn as_usize(self) -> usize { self.0 as usize }

    /// Returns the solver maturity associated with this order.
    #[inline(always)]
    pub const fn maturity(self) -> ContinuityMaturity {
        if self.0 <= Self::G3.0 {
            ContinuityMaturity::Production
        } else {
            ContinuityMaturity::Experimental
        }
    }

    /// Returns the mathematical minimum cross-boundary degree.
    #[inline(always)]
    pub const fn minimum_degree(self) -> usize { self.as_usize() }

    /// Returns the preferred cross-boundary degree for constrained styling.
    ///
    /// The `2k - 1` policy leaves degrees of freedom for fairness after
    /// satisfying a G`k` boundary jet. It recommends degree five for G3 and
    /// degree seven for G4.
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
    type Error = Error;

    fn try_from(value: usize) -> Result<Self> { Self::new(value) }
}

impl From<ContinuityOrder> for usize {
    fn from(value: ContinuityOrder) -> Self { value.as_usize() }
}

impl<'de> Deserialize<'de> for ContinuityOrder {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where D: Deserializer<'de> {
        let order = u8::deserialize(deserializer)?;
        Self::new(order as usize).map_err(serde::de::Error::custom)
    }
}

/// Maturity of a requested continuity order.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContinuityMaturity {
    /// Covered by the production G0 through G3 target.
    Production,
    /// Available for experimentation but not a production guarantee.
    Experimental,
}

/// Parameter axis of a tensor-product surface.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum SurfaceAxis {
    /// The surface's `u` parameter.
    U,
    /// The surface's `v` parameter.
    V,
}

/// Oriented boundary of a tensor-product surface.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum SurfaceBoundary {
    /// Boundary at the start of the `u` parameter range.
    UStart,
    /// Boundary at the end of the `u` parameter range.
    UEnd,
    /// Boundary at the start of the `v` parameter range.
    VStart,
    /// Boundary at the end of the `v` parameter range.
    VEnd,
}

impl SurfaceBoundary {
    /// Returns the axis normal to this boundary in parameter space.
    #[inline(always)]
    pub const fn cross_axis(self) -> SurfaceAxis {
        match self {
            Self::UStart | Self::UEnd => SurfaceAxis::U,
            Self::VStart | Self::VEnd => SurfaceAxis::V,
        }
    }

    /// Returns the axis running along this boundary.
    #[inline(always)]
    pub const fn boundary_axis(self) -> SurfaceAxis {
        match self.cross_axis() {
            SurfaceAxis::U => SurfaceAxis::V,
            SurfaceAxis::V => SurfaceAxis::U,
        }
    }
}

/// Degree-based support level for a continuity request.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContinuityCapabilityLevel {
    /// The cross-boundary degree or control-row count is insufficient.
    Unsupported,
    /// The jet is representable but has limited fairness freedom.
    Feasible,
    /// The preferred styling degree is available.
    Recommended,
}

/// Degree and control-row diagnostics for one surface boundary.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct SurfaceContinuityCapability {
    boundary: SurfaceBoundary,
    requested: ContinuityOrder,
    cross_degree: usize,
    cross_control_rows: usize,
    level: ContinuityCapabilityLevel,
}

impl SurfaceContinuityCapability {
    /// Inspects a polynomial B-spline surface boundary.
    pub fn for_bspline<P>(
        surface: &BsplineSurface<P>,
        boundary: SurfaceBoundary,
        requested: ContinuityOrder,
    ) -> Self {
        let dimensions = (
            surface.control_points().len(),
            surface.control_points().first().map_or(0, Vec::len),
        );
        let degrees = if dimensions.0 == 0 || dimensions.1 == 0 {
            (0, 0)
        } else {
            surface.degrees()
        };
        Self::from_degrees_and_dimensions(degrees, dimensions, boundary, requested)
    }

    /// Inspects a rational B-spline surface boundary.
    pub fn for_nurbs<V>(
        surface: &NurbsSurface<V>,
        boundary: SurfaceBoundary,
        requested: ContinuityOrder,
    ) -> Self {
        let dimensions = (
            surface.control_points().len(),
            surface.control_points().first().map_or(0, Vec::len),
        );
        let degrees = if dimensions.0 == 0 || dimensions.1 == 0 {
            (0, 0)
        } else {
            surface.degrees()
        };
        Self::from_degrees_and_dimensions(degrees, dimensions, boundary, requested)
    }

    fn from_degrees_and_dimensions(
        degrees: (usize, usize),
        dimensions: (usize, usize),
        boundary: SurfaceBoundary,
        requested: ContinuityOrder,
    ) -> Self {
        let (cross_degree, cross_control_rows) = match boundary.cross_axis() {
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
            boundary,
            requested,
            cross_degree,
            cross_control_rows,
            level,
        }
    }

    /// Returns the inspected surface boundary.
    #[inline(always)]
    pub const fn boundary(self) -> SurfaceBoundary { self.boundary }

    /// Returns the requested continuity order.
    #[inline(always)]
    pub const fn requested(self) -> ContinuityOrder { self.requested }

    /// Returns the degree normal to the boundary in parameter space.
    #[inline(always)]
    pub const fn cross_degree(self) -> usize { self.cross_degree }

    /// Returns the number of control rows normal to the boundary.
    #[inline(always)]
    pub const fn cross_control_rows(self) -> usize { self.cross_control_rows }

    /// Returns the degree-based capability level.
    #[inline(always)]
    pub const fn level(self) -> ContinuityCapabilityLevel { self.level }

    /// Returns the maturity of the requested solver target.
    #[inline(always)]
    pub const fn maturity(self) -> ContinuityMaturity { self.requested.maturity() }

    /// Returns the control rows remaining beyond the constrained boundary jet.
    #[inline(always)]
    pub const fn fairness_rows(self) -> usize {
        self.cross_control_rows
            .saturating_sub(self.requested.constrained_rows())
    }

    /// Returns whether the requested jet is mathematically representable.
    #[inline(always)]
    pub const fn is_feasible(self) -> bool {
        !matches!(self.level, ContinuityCapabilityLevel::Unsupported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::Vector4;
    use crate::nurbs::KnotVector;
    use serde::de::value::{Error as ValueError, U8Deserializer};

    fn surface(udegree: usize, vdegree: usize) -> NurbsSurface<Vector4> {
        let points = (0..=udegree)
            .map(|u| {
                (0..=vdegree)
                    .map(|v| Vector4::new(u as f64, v as f64, (u * v) as f64, 1.0))
                    .collect()
            })
            .collect();
        NurbsSurface::new(BsplineSurface::new(
            (
                KnotVector::bezier_knot(udegree),
                KnotVector::bezier_knot(vdegree),
            ),
            points,
        ))
    }

    #[test]
    fn checked_order_leaves_a_numeric_extension_path() {
        assert_eq!(ContinuityOrder::new(3), Ok(ContinuityOrder::G3));
        assert_eq!(ContinuityOrder::new(4), Ok(ContinuityOrder::G4));
        assert_eq!(
            ContinuityOrder::new(5),
            Err(Error::UnsupportedContinuityOrder(5, 4)),
        );
        assert_eq!(
            ContinuityOrder::G3.maturity(),
            ContinuityMaturity::Production
        );
        assert_eq!(
            ContinuityOrder::G4.maturity(),
            ContinuityMaturity::Experimental,
        );
    }

    #[test]
    fn deserialization_preserves_the_checked_order_invariant() {
        let valid = ContinuityOrder::deserialize(U8Deserializer::<ValueError>::new(4));
        let invalid = ContinuityOrder::deserialize(U8Deserializer::<ValueError>::new(5));
        assert!(matches!(valid, Ok(ContinuityOrder::G4)));
        assert!(invalid.is_err());
    }

    #[test]
    fn capability_distinguishes_feasible_and_recommended_degrees() {
        let degree_six = surface(6, 5);
        let g3 = SurfaceContinuityCapability::for_nurbs(
            &degree_six,
            SurfaceBoundary::UStart,
            ContinuityOrder::G3,
        );
        assert_eq!(g3.level(), ContinuityCapabilityLevel::Recommended);
        assert_eq!(g3.fairness_rows(), 3);

        let g4 = SurfaceContinuityCapability::for_nurbs(
            &degree_six,
            SurfaceBoundary::UStart,
            ContinuityOrder::G4,
        );
        assert_eq!(g4.level(), ContinuityCapabilityLevel::Feasible);
        assert_eq!(g4.maturity(), ContinuityMaturity::Experimental);

        let degree_seven = surface(7, 5);
        let g4 = SurfaceContinuityCapability::for_nurbs(
            &degree_seven,
            SurfaceBoundary::UStart,
            ContinuityOrder::G4,
        );
        assert_eq!(g4.level(), ContinuityCapabilityLevel::Recommended);
    }

    #[test]
    fn capability_uses_the_cross_boundary_axis() {
        let surface = surface(3, 5);
        let across_u = SurfaceContinuityCapability::for_nurbs(
            &surface,
            SurfaceBoundary::UEnd,
            ContinuityOrder::G4,
        );
        let across_v = SurfaceContinuityCapability::for_nurbs(
            &surface,
            SurfaceBoundary::VEnd,
            ContinuityOrder::G4,
        );
        assert_eq!(across_u.level(), ContinuityCapabilityLevel::Unsupported);
        assert_eq!(across_v.level(), ContinuityCapabilityLevel::Feasible);
    }
}
