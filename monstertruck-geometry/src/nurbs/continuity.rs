//! Geometry adapters for trait-owned continuity foundations.
//!
//! Checked order, full-boundary side vocabulary, and capability diagnostics
//! live in [`monstertruck_traits::surface_continuity`]. Local transition semantics and
//! the numerical solver remain in `monstertruck-geometry`.

use super::{BsplineSurface, KnotVector, NurbsSurface};

pub use monstertruck_traits::surface_continuity::{
    BoundarySide, ContinuityCapabilityLevel, ContinuityMaturity, ContinuityOrder,
    ControlNetContinuityIssue, KnotVectorContinuityIssue, MAX_CONTINUITY_ORDER, SurfaceAxis,
    SurfaceContinuityCapability, UnsupportedContinuityCapability, UnsupportedContinuityOrder,
};

/// Orientation of the second boundary relative to the first.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum BoundaryAlignment {
    /// Both boundaries use the same traversal direction.
    Aligned,
    /// The second boundary uses the opposite traversal direction.
    Reversed,
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
) -> SurfaceContinuityCapability {
    capability(
        surface.control_points(),
        surface.knot_vector_u(),
        surface.knot_vector_v(),
        side,
        requested,
    )
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
        .ok_or(UnsupportedContinuityCapability::InvalidControlNet(
            ControlNetContinuityIssue::Empty,
        ))
        .and_then(|count_v| {
            control_points
                .iter()
                .all(|row| row.len() == count_v)
                .then_some((control_points.len(), count_v))
                .ok_or(UnsupportedContinuityCapability::InvalidControlNet(
                    ControlNetContinuityIssue::NonRectangular,
                ))
        })
        .and_then(|(count_u, count_v)| {
            let degree_u = valid_axis_degree(knots_u, count_u, SurfaceAxis::U)?;
            let degree_v = valid_axis_degree(knots_v, count_v, SurfaceAxis::V)?;
            Ok(((degree_u, degree_v), (count_u, count_v)))
        });
    match facts {
        Ok((degrees, dimensions)) => SurfaceContinuityCapability::from_degrees_and_dimensions(
            degrees, dimensions, side, requested,
        ),
        Err(reason) => SurfaceContinuityCapability::unsupported(side, requested, reason),
    }
}

fn valid_axis_degree(
    knots: &KnotVector,
    control_count: usize,
    axis: SurfaceAxis,
) -> Result<usize, UnsupportedContinuityCapability> {
    let invalid = |issue| UnsupportedContinuityCapability::InvalidKnotVector { axis, issue };
    let degree = knots
        .len()
        .checked_sub(control_count)
        .and_then(|difference| difference.checked_sub(1))
        .ok_or_else(|| invalid(KnotVectorContinuityIssue::InvalidLength))?;
    let values = knots.as_slice();
    if !values.iter().all(|value| value.is_finite()) {
        Err(invalid(KnotVectorContinuityIssue::NonFinite))
    } else if !values.windows(2).all(|pair| pair[0] <= pair[1]) {
        Err(invalid(KnotVectorContinuityIssue::NonMonotonic))
    } else if !knots.is_clamped(degree) {
        Err(invalid(KnotVectorContinuityIssue::Unclamped))
    } else if values
        .get(degree)
        .zip(values.get(control_count))
        .is_none_or(|(start, end)| end <= start)
    {
        Err(invalid(KnotVectorContinuityIssue::DegenerateDomain))
    } else {
        Ok(degree)
    }
}
