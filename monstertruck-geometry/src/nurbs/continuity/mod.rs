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
) -> SurfaceContinuityCapability {
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
) -> SurfaceContinuityCapability
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
        (None, Some(reason)) => SurfaceContinuityCapability::unsupported(
            side,
            requested,
            reason,
            polynomial.maximum_supported_order(),
        ),
    }
}

fn capability<P>(
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
            let degree_u = valid_axis_degree(knots_u, count_u)?;
            let degree_v = valid_axis_degree(knots_v, count_v)?;
            Ok(((degree_u, degree_v), (count_u, count_v)))
        });

    match facts {
        Ok((degrees, dimensions)) => capability_from_facts(degrees, dimensions, side, requested),
        Err(reason) => SurfaceContinuityCapability::unsupported(side, requested, reason, None),
    }
}

fn capability_from_facts(
    degrees: (usize, usize),
    dimensions: (usize, usize),
    side: BoundarySide,
    requested: ContinuityOrder,
) -> SurfaceContinuityCapability {
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
        Some(reason) => {
            SurfaceContinuityCapability::unsupported(side, requested, reason, maximum_order)
        }
        None => match maximum_order.and_then(|maximum_order| {
            SurfaceContinuityCapability::try_supported_through(side, requested, maximum_order).ok()
        }) {
            Some(report) => report,
            None => SurfaceContinuityCapability::unsupported(
                side,
                requested,
                UnsupportedContinuityCapability::UnsupportedRepresentation,
                maximum_order,
            ),
        },
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
mod tests;
