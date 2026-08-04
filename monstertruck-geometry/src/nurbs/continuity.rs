//! Geometry adapters for trait-owned continuity foundations.
//!
//! Checked order, full-boundary side vocabulary, and capability diagnostics
//! live in [`monstertruck_traits::surface_continuity`]. Local transition semantics and
//! the numerical solver remain in `monstertruck-geometry`.

use super::{BsplineSurface, KnotVector, NurbsSurface};

pub use monstertruck_traits::surface_continuity::{
    BoundarySide, ContinuityCapabilityLevel, ContinuityMaturity, ContinuityOrder,
    MAX_CONTINUITY_ORDER, SurfaceAxis, SurfaceContinuityCapability, UnsupportedContinuityOrder,
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
    let dimensions = control_points
        .first()
        .map(Vec::len)
        .filter(|&count| count > 0)
        .filter(|&count| control_points.iter().all(|row| row.len() == count))
        .and_then(|count_v| {
            let count_u = control_points.len();
            let degree_u = valid_axis_degree(knots_u, count_u)?;
            let degree_v = valid_axis_degree(knots_v, count_v)?;
            Some(((degree_u, degree_v), (count_u, count_v)))
        });
    let (degrees, dimensions) = dimensions.unwrap_or(((0, 0), (0, 0)));
    SurfaceContinuityCapability::from_degrees_and_dimensions(degrees, dimensions, side, requested)
}

fn valid_axis_degree(knots: &KnotVector, control_count: usize) -> Option<usize> {
    let degree = knots.len().checked_sub(control_count)?.checked_sub(1)?;
    let values = knots.as_slice();
    let valid_values = values.iter().all(|value| value.is_finite())
        && values.windows(2).all(|pair| pair[0] <= pair[1]);
    let positive_domain = values
        .get(degree)
        .zip(values.get(control_count))
        .is_some_and(|(start, end)| end > start);
    (valid_values && knots.is_clamped(degree) && positive_domain).then_some(degree)
}
