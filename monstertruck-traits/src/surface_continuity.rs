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

/// Reason that a control net cannot be inspected for continuity capability.
#[derive(Clone, Copy, Debug, Error, Hash, PartialEq, Eq)]
#[non_exhaustive]
pub enum ControlNetContinuityIssue {
    /// The control net has no usable rows or columns.
    #[error("the control net is empty")]
    Empty,
    /// The control-net rows do not all have the same length.
    #[error("the control net is not rectangular")]
    NonRectangular,
}

/// Reason that a knot vector cannot support continuity inspection.
#[derive(Clone, Copy, Debug, Error, Hash, PartialEq, Eq)]
#[non_exhaustive]
pub enum KnotVectorContinuityIssue {
    /// The knot and control-point counts cannot define a degree.
    #[error("the knot count is incompatible with the control-point count")]
    InvalidLength,
    /// At least one knot is not finite.
    #[error("the knot vector contains a non-finite value")]
    NonFinite,
    /// The knots are not ordered nondecreasingly.
    #[error("the knot vector is not nondecreasing")]
    NonMonotonic,
    /// The knot vector is not clamped for its degree.
    #[error("the knot vector is not clamped")]
    Unclamped,
    /// The active parameter domain has zero or negative length.
    #[error("the knot vector has a degenerate parameter domain")]
    DegenerateDomain,
}

/// Typed reason that a surface cannot represent a continuity request.
#[derive(Clone, Copy, Debug, Error, Hash, PartialEq, Eq)]
#[non_exhaustive]
pub enum UnsupportedContinuityCapability {
    /// The surface control net is malformed.
    #[error("invalid surface control net: {0}")]
    InvalidControlNet(ControlNetContinuityIssue),
    /// A parameter axis has an invalid knot vector.
    #[error("invalid {axis:?}-axis knot vector: {issue}")]
    InvalidKnotVector {
        /// Axis described by the invalid knot vector.
        axis: SurfaceAxis,
        /// Specific knot-vector failure.
        issue: KnotVectorContinuityIssue,
    },
    /// A rational representation contains a non-finite weight.
    #[error("the rational control point at ({row}, {column}) has a non-finite weight")]
    NonFiniteWeight {
        /// Control-net row containing the invalid weight.
        row: usize,
        /// Control-net column containing the invalid weight.
        column: usize,
    },
    /// A rational representation contains a non-positive weight.
    #[error("the rational control point at ({row}, {column}) has a non-positive weight")]
    NonPositiveWeight {
        /// Control-net row containing the invalid weight.
        row: usize,
        /// Control-net column containing the invalid weight.
        column: usize,
    },
    /// The inspected side belongs to a periodic parameter direction.
    #[error("the inspected boundary is periodic")]
    PeriodicBoundary,
    /// The representation cannot expose the requested full-boundary jet.
    #[error("the surface representation cannot expose the requested full-boundary jet")]
    UnsupportedRepresentation,
    /// The request addresses a trimmed seam rather than a full patch side.
    #[error("trimmed seams are unsupported by full-boundary continuity capability")]
    TrimmedBoundary,
    /// The cross-boundary degree is below the mathematical minimum.
    #[error("cross-boundary degree {available} is below the required degree {required}")]
    InsufficientDegree {
        /// Available cross-boundary degree.
        available: usize,
        /// Required cross-boundary degree.
        required: usize,
    },
    /// The control net has too few cross-boundary rows for the requested jet.
    #[error("{available} cross-boundary control rows are below the required {required}")]
    InsufficientControlRows {
        /// Available cross-boundary control rows.
        available: usize,
        /// Required cross-boundary control rows.
        required: usize,
    },
    /// Both degree and control-row requirements are unsatisfied.
    #[error(
        "cross-boundary degree {available_degree} and control rows {available_rows} are below the required {required_degree} and {required_rows}"
    )]
    InsufficientDegreeAndControlRows {
        /// Available cross-boundary degree.
        available_degree: usize,
        /// Required cross-boundary degree.
        required_degree: usize,
        /// Available cross-boundary control rows.
        available_rows: usize,
        /// Required cross-boundary control rows.
        required_rows: usize,
    },
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
enum ContinuityCapabilityAssessment {
    Unsupported(UnsupportedContinuityCapability),
    Feasible,
    Recommended,
}

/// Degree and control-row diagnostics for one full surface side.
///
/// Capability reports mathematical representability only. In particular, a
/// feasible or recommended `G4` report remains experimental.
///
/// # Examples
///
/// ```
/// use monstertruck_traits::{
///     BoundarySide, ContinuityOrder, SurfaceContinuityCapability,
///     UnsupportedContinuityCapability,
/// };
///
/// let capability = SurfaceContinuityCapability::from_degrees_and_dimensions(
///     (2, 5),
///     (3, 6),
///     BoundarySide::MinU,
///     ContinuityOrder::G3,
/// );
///
/// assert_eq!(capability.maximum_supported_order(), Some(ContinuityOrder::G2));
/// assert_eq!(
///     capability.unsupported_reason(),
///     Some(UnsupportedContinuityCapability::InsufficientDegreeAndControlRows {
///         available_degree: 2,
///         required_degree: 3,
///         available_rows: 3,
///         required_rows: 4,
///     }),
/// );
/// ```
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct SurfaceContinuityCapability {
    side: BoundarySide,
    requested: ContinuityOrder,
    cross_degree: usize,
    cross_control_rows: usize,
    maximum_supported_order: Option<ContinuityOrder>,
    assessment: ContinuityCapabilityAssessment,
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
        let required_degree = requested.minimum_degree();
        let required_rows = requested.constrained_rows();
        let insufficient_degree = cross_degree < required_degree;
        let insufficient_rows = cross_control_rows < required_rows;
        let maximum_supported_order = if cross_control_rows == 0 {
            None
        } else {
            let row_limited = cross_control_rows - 1;
            let surface_maximum = if cross_degree < row_limited {
                cross_degree
            } else {
                row_limited
            };
            let maximum = if surface_maximum < MAX_CONTINUITY_ORDER {
                surface_maximum
            } else {
                MAX_CONTINUITY_ORDER
            };
            Some(ContinuityOrder(maximum as u8))
        };
        let assessment = if insufficient_degree && insufficient_rows {
            ContinuityCapabilityAssessment::Unsupported(
                UnsupportedContinuityCapability::InsufficientDegreeAndControlRows {
                    available_degree: cross_degree,
                    required_degree,
                    available_rows: cross_control_rows,
                    required_rows,
                },
            )
        } else if insufficient_degree {
            ContinuityCapabilityAssessment::Unsupported(
                UnsupportedContinuityCapability::InsufficientDegree {
                    available: cross_degree,
                    required: required_degree,
                },
            )
        } else if insufficient_rows {
            ContinuityCapabilityAssessment::Unsupported(
                UnsupportedContinuityCapability::InsufficientControlRows {
                    available: cross_control_rows,
                    required: required_rows,
                },
            )
        } else if cross_degree < requested.recommended_degree() {
            ContinuityCapabilityAssessment::Feasible
        } else {
            ContinuityCapabilityAssessment::Recommended
        };
        Self {
            side,
            requested,
            cross_degree,
            cross_control_rows,
            maximum_supported_order,
            assessment,
        }
    }

    /// Builds an unsupported report from a typed inspection failure.
    pub const fn unsupported(
        side: BoundarySide,
        requested: ContinuityOrder,
        reason: UnsupportedContinuityCapability,
    ) -> Self {
        Self {
            side,
            requested,
            cross_degree: 0,
            cross_control_rows: 0,
            maximum_supported_order: None,
            assessment: ContinuityCapabilityAssessment::Unsupported(reason),
        }
    }

    /// Returns the inspected surface side.
    pub const fn side(self) -> BoundarySide { self.side }

    /// Returns the requested continuity order.
    pub const fn requested(self) -> ContinuityOrder { self.requested }

    /// Returns the request's maturity classification.
    pub const fn maturity(self) -> ContinuityMaturity { self.requested.maturity() }

    /// Returns the degree normal to the side in parameter space.
    ///
    /// Returns zero when surface validation failed before deriving a degree.
    pub const fn cross_degree(self) -> usize { self.cross_degree }

    /// Returns the number of control rows normal to the side.
    ///
    /// Returns zero when surface validation failed before producing dimensions.
    pub const fn cross_control_rows(self) -> usize { self.cross_control_rows }

    /// Returns the coarse capability level.
    pub const fn level(self) -> ContinuityCapabilityLevel {
        match self.assessment {
            ContinuityCapabilityAssessment::Unsupported(_) => {
                ContinuityCapabilityLevel::Unsupported
            }
            ContinuityCapabilityAssessment::Feasible => ContinuityCapabilityLevel::Feasible,
            ContinuityCapabilityAssessment::Recommended => ContinuityCapabilityLevel::Recommended,
        }
    }

    /// Returns the highest continuity order represented by the inspected side.
    ///
    /// Returns `None` when invalid surface data prevents capability inspection.
    pub const fn maximum_supported_order(self) -> Option<ContinuityOrder> {
        self.maximum_supported_order
    }

    /// Returns the typed reason that the request is unsupported.
    pub const fn unsupported_reason(self) -> Option<UnsupportedContinuityCapability> {
        match self.assessment {
            ContinuityCapabilityAssessment::Unsupported(reason) => Some(reason),
            ContinuityCapabilityAssessment::Feasible
            | ContinuityCapabilityAssessment::Recommended => None,
        }
    }

    /// Requires the requested jet to be mathematically representable.
    ///
    /// # Errors
    ///
    /// Returns [`UnsupportedContinuityCapability`] with the specific failed
    /// requirement or invalid surface condition.
    pub const fn require_feasible(self) -> Result<Self, UnsupportedContinuityCapability> {
        match self.assessment {
            ContinuityCapabilityAssessment::Unsupported(reason) => Err(reason),
            ContinuityCapabilityAssessment::Feasible
            | ContinuityCapabilityAssessment::Recommended => Ok(self),
        }
    }

    /// Returns the control rows remaining beyond the constrained boundary jet.
    pub const fn fairness_rows(self) -> usize {
        self.cross_control_rows
            .saturating_sub(self.requested.constrained_rows())
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

    #[test]
    fn unsupported_capability_preserves_every_actionable_reason() {
        [
            UnsupportedContinuityCapability::InvalidControlNet(
                ControlNetContinuityIssue::NonRectangular,
            ),
            UnsupportedContinuityCapability::InvalidKnotVector {
                axis: SurfaceAxis::V,
                issue: KnotVectorContinuityIssue::NonMonotonic,
            },
            UnsupportedContinuityCapability::InsufficientDegree {
                available: 1,
                required: 3,
            },
            UnsupportedContinuityCapability::NonFiniteWeight { row: 2, column: 4 },
            UnsupportedContinuityCapability::NonPositiveWeight { row: 3, column: 1 },
            UnsupportedContinuityCapability::PeriodicBoundary,
            UnsupportedContinuityCapability::UnsupportedRepresentation,
            UnsupportedContinuityCapability::TrimmedBoundary,
        ]
        .into_iter()
        .for_each(|reason| {
            let capability = SurfaceContinuityCapability::unsupported(
                BoundarySide::MaxU,
                ContinuityOrder::G3,
                reason,
            );

            assert_eq!(capability.unsupported_reason(), Some(reason));
            assert_eq!(capability.require_feasible(), Err(reason));
        });
    }
}
