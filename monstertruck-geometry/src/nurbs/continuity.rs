//! Geometry adapters for trait-owned continuity foundations.
//!
//! Checked order, full-boundary side vocabulary, and capability diagnostics
//! live in [`monstertruck_traits::surface_continuity`]. Local transition
//! semantics and representation inspection remain in `monstertruck-geometry`.

use monstertruck_core::cgmath64::{Homogeneous, control_point::ControlPoint};

use super::{BsplineSurface, KnotVector, NurbsSurface};

pub use monstertruck_traits::surface_continuity::{
    BoundarySide, ContinuityOrder, InvalidContinuityCapability, MAX_CONTINUITY_ORDER,
    SurfaceContinuityCapability, SurfaceContinuitySupport, UnsupportedContinuityCapability,
    UnsupportedContinuityOrder,
};

/// Orientation of the second boundary relative to the first.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum BoundaryAlignment {
    /// Both boundaries use the same traversal direction.
    Aligned,
    /// The second boundary uses the opposite traversal direction.
    Reversed,
}

/// A concrete surface inspection paired with the trait-owned capability report.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct InspectedSurfaceContinuityCapability {
    report: SurfaceContinuityCapability,
    cross_degree: usize,
    cross_control_rows: usize,
}

impl InspectedSurfaceContinuityCapability {
    const fn new(
        report: SurfaceContinuityCapability,
        cross_degree: usize,
        cross_control_rows: usize,
    ) -> Self {
        Self {
            report,
            cross_degree,
            cross_control_rows,
        }
    }

    /// Returns the trait-owned capability report.
    pub const fn report(self) -> SurfaceContinuityCapability { self.report }

    /// Returns the inspected surface side.
    pub const fn side(self) -> BoundarySide { self.report.side() }

    /// Returns the requested continuity order.
    pub const fn requested(self) -> ContinuityOrder { self.report.requested() }

    /// Returns the typed support determination.
    pub const fn support(self) -> SurfaceContinuitySupport { self.report.support() }

    /// Returns the degree normal to the inspected side.
    ///
    /// Returns zero when representation validation failed before deriving a
    /// degree.
    pub const fn cross_degree(self) -> usize { self.cross_degree }

    /// Returns the number of control rows normal to the inspected side.
    ///
    /// Returns zero when representation validation failed before deriving the
    /// control-net dimensions.
    pub const fn cross_control_rows(self) -> usize { self.cross_control_rows }

    /// Returns the highest supported order when inspection established it.
    pub const fn maximum_supported_order(self) -> Option<ContinuityOrder> {
        self.report.maximum_supported_order()
    }

    /// Returns the typed reason that the request is unsupported.
    pub const fn unsupported_reason(self) -> Option<UnsupportedContinuityCapability> {
        self.report.unsupported_reason()
    }

    /// Requires the representation to support the request.
    ///
    /// # Errors
    ///
    /// Returns [`UnsupportedContinuityCapability`] with the failed
    /// representation requirement.
    pub const fn require_supported(self) -> Result<Self, UnsupportedContinuityCapability> {
        match self.report.require_supported() {
            Ok(_) => Ok(self),
            Err(reason) => Err(reason),
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub(crate) enum SurfaceAxis {
    U,
    V,
}

pub(crate) const fn cross_axis(side: BoundarySide) -> SurfaceAxis {
    match side {
        BoundarySide::MinU | BoundarySide::MaxU => SurfaceAxis::U,
        BoundarySide::MinV | BoundarySide::MaxV => SurfaceAxis::V,
    }
}

pub(crate) const fn boundary_axis(side: BoundarySide) -> SurfaceAxis {
    match cross_axis(side) {
        SurfaceAxis::U => SurfaceAxis::V,
        SurfaceAxis::V => SurfaceAxis::U,
    }
}

/// Inspects a polynomial B-spline surface side.
pub fn capability_for_bspline<P>(
    surface: &BsplineSurface<P>,
    side: BoundarySide,
    requested: ContinuityOrder,
) -> InspectedSurfaceContinuityCapability {
    capability(
        surface.control_points(),
        surface.knot_vector_u(),
        surface.knot_vector_v(),
        side,
        requested,
    )
}

/// Inspects a rational B-spline surface side.
pub fn capability_for_nurbs<V>(
    surface: &NurbsSurface<V>,
    side: BoundarySide,
    requested: ContinuityOrder,
) -> InspectedSurfaceContinuityCapability
where
    V: Homogeneous<Scalar = f64> + ControlPoint<f64, Diff = V>,
{
    let polynomial = capability(
        surface.control_points(),
        surface.knot_vector_u(),
        surface.knot_vector_v(),
        side,
        requested,
    );
    let invalid_weight = surface
        .control_points()
        .iter()
        .flatten()
        .map(|point| point.weight())
        .find_map(|weight| {
            if !weight.is_finite() {
                Some(UnsupportedContinuityCapability::NonFiniteWeight)
            } else if weight <= 0.0 {
                Some(UnsupportedContinuityCapability::NonPositiveWeight)
            } else {
                None
            }
        });

    match (polynomial.unsupported_reason(), invalid_weight) {
        (Some(_), _) | (None, None) => polynomial,
        (None, Some(reason)) => InspectedSurfaceContinuityCapability::new(
            SurfaceContinuityCapability::unsupported(
                side,
                requested,
                reason,
                polynomial.maximum_supported_order(),
            ),
            polynomial.cross_degree,
            polynomial.cross_control_rows,
        ),
    }
}

fn capability<P>(
    control_points: &[Vec<P>],
    knots_u: &KnotVector,
    knots_v: &KnotVector,
    side: BoundarySide,
    requested: ContinuityOrder,
) -> InspectedSurfaceContinuityCapability {
    let facts = control_points
        .first()
        .filter(|row| !row.is_empty())
        .map(Vec::len)
        .ok_or(UnsupportedContinuityCapability::InvalidControlNet)
        .and_then(|count_v| {
            control_points
                .iter()
                .all(|row| row.len() == count_v)
                .then_some((control_points.len(), count_v))
                .ok_or(UnsupportedContinuityCapability::InvalidControlNet)
        })
        .and_then(|(count_u, count_v)| {
            let degree_u = valid_axis_degree(knots_u, count_u)?;
            let degree_v = valid_axis_degree(knots_v, count_v)?;
            Ok(((degree_u, degree_v), (count_u, count_v)))
        });

    match facts {
        Ok((degrees, dimensions)) => capability_from_facts(degrees, dimensions, side, requested),
        Err(reason) => InspectedSurfaceContinuityCapability::new(
            SurfaceContinuityCapability::unsupported(side, requested, reason, None),
            0,
            0,
        ),
    }
}

fn capability_from_facts(
    degrees: (usize, usize),
    dimensions: (usize, usize),
    side: BoundarySide,
    requested: ContinuityOrder,
) -> InspectedSurfaceContinuityCapability {
    let (cross_degree, cross_control_rows) = match cross_axis(side) {
        SurfaceAxis::U => (degrees.0, dimensions.0),
        SurfaceAxis::V => (degrees.1, dimensions.1),
    };
    let required_degree = requested.as_usize();
    let required_rows = required_degree + 1;
    let maximum = cross_degree
        .min(cross_control_rows.saturating_sub(1))
        .min(MAX_CONTINUITY_ORDER);
    let maximum_order = ContinuityOrder::new(maximum).ok();
    let insufficient_degree = cross_degree < required_degree;
    let insufficient_rows = cross_control_rows < required_rows;
    let reason = if insufficient_degree && insufficient_rows {
        Some(
            UnsupportedContinuityCapability::InsufficientDegreeAndControlRows {
                available_degree: cross_degree,
                required_degree,
                available_rows: cross_control_rows,
                required_rows,
            },
        )
    } else if insufficient_degree {
        Some(UnsupportedContinuityCapability::InsufficientDegree {
            available: cross_degree,
            required: required_degree,
        })
    } else if insufficient_rows {
        Some(UnsupportedContinuityCapability::InsufficientControlRows {
            available: cross_control_rows,
            required: required_rows,
        })
    } else {
        None
    };
    match reason {
        Some(reason) => InspectedSurfaceContinuityCapability::new(
            SurfaceContinuityCapability::unsupported(side, requested, reason, maximum_order),
            cross_degree,
            cross_control_rows,
        ),
        None => {
            let report = match maximum_order.and_then(|maximum_order| {
                SurfaceContinuityCapability::try_supported_through(side, requested, maximum_order)
                    .ok()
            }) {
                Some(report) => report,
                None => SurfaceContinuityCapability::unsupported(
                    side,
                    requested,
                    UnsupportedContinuityCapability::UnsupportedRepresentation,
                    maximum_order,
                ),
            };
            InspectedSurfaceContinuityCapability::new(report, cross_degree, cross_control_rows)
        }
    }
}

fn valid_axis_degree(
    knots: &KnotVector,
    control_count: usize,
) -> Result<usize, UnsupportedContinuityCapability> {
    let degree = knots
        .len()
        .checked_sub(control_count)
        .and_then(|difference| difference.checked_sub(1))
        .ok_or(UnsupportedContinuityCapability::InvalidKnotVector)?;
    let values = knots.as_slice();
    if !values.iter().all(|value| value.is_finite())
        || !values.windows(2).all(|pair| pair[0] <= pair[1])
    {
        Err(UnsupportedContinuityCapability::InvalidKnotVector)
    } else if !knots.is_clamped(degree) {
        Err(UnsupportedContinuityCapability::UnclampedBoundary)
    } else if values
        .get(degree)
        .zip(values.get(control_count))
        .is_none_or(|(start, end)| end <= start)
    {
        Err(UnsupportedContinuityCapability::InvalidKnotVector)
    } else {
        Ok(degree)
    }
}

#[cfg(test)]
mod tests {
    use monstertruck_core::cgmath64::Vector4;

    use super::*;

    fn rational_surface(weight: f64) -> NurbsSurface<Vector4> {
        NurbsSurface::new(BsplineSurface::new(
            (KnotVector::bezier_knot(1), KnotVector::bezier_knot(1)),
            vec![
                vec![
                    Vector4::new(0.0, 0.0, 0.0, 1.0),
                    Vector4::new(0.0, 1.0, 0.0, weight),
                ],
                vec![
                    Vector4::new(1.0, 0.0, 0.0, 1.0),
                    Vector4::new(1.0, 1.0, 0.0, 1.0),
                ],
            ],
        ))
    }

    #[test]
    fn rational_capability_preserves_typed_weight_failures() {
        let non_finite = capability_for_nurbs(
            &rational_surface(f64::NAN),
            BoundarySide::MinU,
            ContinuityOrder::G1,
        );
        let non_positive = capability_for_nurbs(
            &rational_surface(0.0),
            BoundarySide::MinU,
            ContinuityOrder::G1,
        );

        assert_eq!(
            non_finite.unsupported_reason(),
            Some(UnsupportedContinuityCapability::NonFiniteWeight)
        );
        assert_eq!(
            non_positive.unsupported_reason(),
            Some(UnsupportedContinuityCapability::NonPositiveWeight)
        );
    }
}
