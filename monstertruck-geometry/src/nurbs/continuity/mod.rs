use monstertruck_core::cgmath64::{Homogeneous, control_point::ControlPoint};
pub use monstertruck_traits::surface_continuity::{
    BoundarySide, ContinuityOrder, MAX_CONTINUITY_ORDER, SurfaceContinuityCapability,
    UnsupportedContinuityCapability,
};

use super::{BsplineSurface, KnotVector, NurbsSurface};

/// Orientation of the second boundary relative to the first.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum BoundaryAlignment {
    /// Both boundaries use the same traversal direction.
    Aligned,
    /// The second boundary uses the opposite traversal direction.
    Reversed,
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

impl<P: ControlPoint<f64>> BsplineSurface<P> {
    /// Reports whether this B-spline representation can expose the requested
    /// derivatives along a full parameter-domain side.
    ///
    /// This checks the clamped cross-boundary knot vector, degree, and control
    /// rows. It does not establish compatibility with another surface or
    /// feasibility for a numerical solver.
    ///
    /// # Examples
    ///
    /// ```
    /// use monstertruck_geometry::prelude::*;
    ///
    /// let surface = BsplineSurface::new(
    ///     (KnotVector::bezier_knot(1), KnotVector::bezier_knot(2)),
    ///     vec![vec![Point3::new(0.0, 0.0, 0.0); 3]; 2],
    /// );
    ///
    /// let capability =
    ///     surface.continuity_capability(BoundarySide::MinV, ContinuityOrder::G2);
    ///
    /// assert_eq!(capability.unsupported_reason(), None);
    /// assert_eq!(capability.maximum_supported_order(), Some(ContinuityOrder::G2));
    /// ```
    pub fn continuity_capability(
        &self,
        side: BoundarySide,
        requested: ContinuityOrder,
    ) -> SurfaceContinuityCapability {
        let control_points = self.control_points();
        let dimensions = control_points
            .first()
            .filter(|row| !row.is_empty())
            .map(Vec::len)
            .filter(|&count_v| control_points.iter().all(|row| row.len() == count_v))
            .map(|count_v| (control_points.len(), count_v));

        match dimensions {
            Some((count_u, count_v)) => {
                let (knots, control_count) = match side {
                    BoundarySide::MinU | BoundarySide::MaxU => (self.knot_vector_u(), count_u),
                    BoundarySide::MinV | BoundarySide::MaxV => (self.knot_vector_v(), count_v),
                };
                capability_for_axis(knots, control_count, side, requested)
            }
            None => unsupported(
                side,
                requested,
                UnsupportedContinuityCapability::InvalidControlNet,
                None,
            ),
        }
    }
}

impl<V> NurbsSurface<V>
where V: Homogeneous<Scalar = f64> + ControlPoint<f64, Diff = V>
{
    /// Reports whether this positive-weight NURBS representation can expose
    /// the requested derivatives along a full parameter-domain side.
    ///
    /// In addition to the underlying B-spline requirements, every homogeneous
    /// control point must carry a finite positive weight. This does not
    /// establish compatibility with another surface or solver feasibility.
    ///
    /// # Examples
    ///
    /// ```
    /// use monstertruck_geometry::prelude::*;
    ///
    /// let surface = NurbsSurface::new(BsplineSurface::new(
    ///     (KnotVector::bezier_knot(1), KnotVector::bezier_knot(1)),
    ///     vec![vec![Vector4::new(0.0, 0.0, 0.0, 1.0); 2]; 2],
    /// ));
    ///
    /// let capability =
    ///     surface.continuity_capability(BoundarySide::MaxU, ContinuityOrder::G1);
    ///
    /// assert_eq!(capability.unsupported_reason(), None);
    /// ```
    pub fn continuity_capability(
        &self,
        side: BoundarySide,
        requested: ContinuityOrder,
    ) -> SurfaceContinuityCapability {
        let polynomial = self
            .non_rationalized()
            .continuity_capability(side, requested);
        let mut weights = self
            .control_points()
            .iter()
            .flatten()
            .map(|point| point.weight());

        if polynomial.unsupported_reason().is_some() {
            polynomial
        } else if weights.clone().any(|weight| !weight.is_finite()) {
            unsupported(
                side,
                requested,
                UnsupportedContinuityCapability::NonFiniteWeight,
                None,
            )
        } else if weights.any(|weight| weight <= 0.0) {
            unsupported(
                side,
                requested,
                UnsupportedContinuityCapability::NonPositiveWeight,
                None,
            )
        } else {
            polynomial
        }
    }
}

/// Build an unsupported report, asserting the constructor's precondition once.
///
/// [`SurfaceContinuityCapability::try_unsupported`] rejects a *known* maximum
/// order at or above the requested order, because such a report would contradict
/// itself. Every call below either passes `None` -- structural failures learn
/// nothing about the achievable order -- or a maximum that the surrounding
/// insufficiency check has already proven strictly lower. Doing the unwrap here
/// keeps that argument in one place instead of repeating it at eight call sites.
fn unsupported(
    side: BoundarySide,
    requested: ContinuityOrder,
    reason: UnsupportedContinuityCapability,
    maximum_order: Option<ContinuityOrder>,
) -> SurfaceContinuityCapability {
    // SAFETY: `maximum_order` is `None`, or strictly below `requested` by the
    // insufficiency test that selected this reason.
    SurfaceContinuityCapability::try_unsupported(side, requested, reason, maximum_order).unwrap()
}

fn capability_for_axis(
    knots: &KnotVector,
    control_count: usize,
    side: BoundarySide,
    requested: ContinuityOrder,
) -> SurfaceContinuityCapability {
    let degree = knots
        .len()
        .checked_sub(control_count)
        .and_then(|difference| difference.checked_sub(1));
    let values = knots.as_slice();
    let valid_knots = values.iter().all(|value| value.is_finite())
        && values.windows(2).all(|pair| pair[0] <= pair[1]);
    let positive_domain = degree.is_some_and(|degree| {
        values
            .get(degree)
            .zip(values.get(control_count))
            .is_some_and(|(start, end)| end > start)
    });

    match degree.filter(|_| valid_knots && positive_domain) {
        Some(degree) if knots.is_clamped(degree) => {
            capability_for_degree_and_rows(degree, control_count, side, requested)
        }
        Some(_) => unsupported(
            side,
            requested,
            UnsupportedContinuityCapability::UnclampedBoundary,
            None,
        ),
        None => unsupported(
            side,
            requested,
            UnsupportedContinuityCapability::InvalidKnotVector,
            None,
        ),
    }
}

fn capability_for_degree_and_rows(
    degree: usize,
    control_count: usize,
    side: BoundarySide,
    requested: ContinuityOrder,
) -> SurfaceContinuityCapability {
    let required_degree = requested.as_usize();
    let required_rows = required_degree + 1;
    let insufficient_degree = degree < required_degree;
    let insufficient_rows = control_count < required_rows;
    let maximum_value = degree
        .min(control_count.saturating_sub(1))
        .min(MAX_CONTINUITY_ORDER);
    let maximum_order = match maximum_value {
        0 => ContinuityOrder::G0,
        1 => ContinuityOrder::G1,
        2 => ContinuityOrder::G2,
        3 => ContinuityOrder::G3,
        _ => ContinuityOrder::G4,
    };

    if insufficient_degree && insufficient_rows {
        unsupported(
            side,
            requested,
            UnsupportedContinuityCapability::InsufficientDegreeAndControlRows {
                available_degree: degree,
                required_degree,
                available_rows: control_count,
                required_rows,
            },
            Some(maximum_order),
        )
    } else if insufficient_degree {
        unsupported(
            side,
            requested,
            UnsupportedContinuityCapability::InsufficientDegree {
                available: degree,
                required: required_degree,
            },
            Some(maximum_order),
        )
    } else if insufficient_rows {
        unsupported(
            side,
            requested,
            UnsupportedContinuityCapability::InsufficientControlRows {
                available: control_count,
                required: required_rows,
            },
            Some(maximum_order),
        )
    } else {
        // SAFETY: Both degree and row checks establish `maximum_order >= requested`.
        SurfaceContinuityCapability::try_supported_through(side, requested, maximum_order).unwrap()
    }
}

/// Inspects a polynomial B-spline surface for the downstream validation corpus.
///
/// Production code uses [`BsplineSurface::continuity_capability`]. This adapter
/// additionally validates both parameter axes so the preserved robustness
/// corpus can continue detecting malformed surfaces outside the inspected side.
#[doc(hidden)]
pub fn capability_for_bspline<P>(
    surface: &BsplineSurface<P>,
    side: BoundarySide,
    requested: ContinuityOrder,
) -> SurfaceContinuityCapability {
    validation_capability(
        surface.control_points(),
        surface.knot_vector_u(),
        surface.knot_vector_v(),
        side,
        requested,
    )
}

/// Inspects a rational B-spline surface for the downstream validation corpus.
///
/// Production code uses [`NurbsSurface::continuity_capability`]. This adapter
/// retains the corpus's stricter rational-weight diagnostic precedence and is
/// excluded from an upstream contribution.
#[doc(hidden)]
pub fn capability_for_nurbs<V>(
    surface: &NurbsSurface<V>,
    side: BoundarySide,
    requested: ContinuityOrder,
) -> SurfaceContinuityCapability
where
    V: Homogeneous<Scalar = f64> + ControlPoint<f64, Diff = V>,
{
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

    match invalid_weight {
        Some(reason) => unsupported(side, requested, reason, None),
        None => validation_capability(
            surface.control_points(),
            surface.knot_vector_u(),
            surface.knot_vector_v(),
            side,
            requested,
        ),
    }
}

fn validation_capability<P>(
    control_points: &[Vec<P>],
    knots_u: &KnotVector,
    knots_v: &KnotVector,
    side: BoundarySide,
    requested: ContinuityOrder,
) -> SurfaceContinuityCapability {
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
            let degree_u = validation_axis_degree(knots_u, count_u)?;
            let degree_v = validation_axis_degree(knots_v, count_v)?;
            Ok(((degree_u, degree_v), (count_u, count_v)))
        });

    match facts {
        Ok((degrees, dimensions)) => {
            let (degree, rows) = match cross_axis(side) {
                SurfaceAxis::U => (degrees.0, dimensions.0),
                SurfaceAxis::V => (degrees.1, dimensions.1),
            };
            capability_for_degree_and_rows(degree, rows, side, requested)
        }
        Err(reason) => unsupported(side, requested, reason, None),
    }
}

fn validation_axis_degree(
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
mod tests;
