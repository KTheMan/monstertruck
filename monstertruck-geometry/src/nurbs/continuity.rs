//! Geometry adapters for trait-owned continuity foundations.
//!
//! Checked order, full-boundary side vocabulary, and capability diagnostics
//! live in [`monstertruck_traits::surface_continuity`]. Local transition semantics and
//! the numerical solver remain in `monstertruck-geometry`.

use super::{BsplineSurface, NurbsSurface};

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
        surface.knot_vector_u().len(),
        surface.knot_vector_v().len(),
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
        surface.knot_vector_u().len(),
        surface.knot_vector_v().len(),
        side,
        requested,
    )
}

fn capability<P>(
    control_points: &[Vec<P>],
    knot_count_u: usize,
    knot_count_v: usize,
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
            let degree_u = knot_count_u.checked_sub(count_u)?.checked_sub(1)?;
            let degree_v = knot_count_v.checked_sub(count_v)?.checked_sub(1)?;
            Some(((degree_u, degree_v), (count_u, count_v)))
        });
    let (degrees, dimensions) = dimensions.unwrap_or(((0, 0), (0, 0)));
    SurfaceContinuityCapability::from_degrees_and_dimensions(degrees, dimensions, side, requested)
}
