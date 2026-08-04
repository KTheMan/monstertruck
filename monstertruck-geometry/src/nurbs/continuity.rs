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
    let control_points = surface.control_points();
    let degrees = control_points
        .first()
        .is_some_and(|row| !row.is_empty())
        .then(|| surface.degrees());
    capability(control_points, degrees, side, requested)
}

/// Inspects a rational B-spline surface side.
pub fn capability_for_nurbs<V>(
    surface: &NurbsSurface<V>,
    side: BoundarySide,
    requested: ContinuityOrder,
) -> SurfaceContinuityCapability {
    let control_points = surface.control_points();
    let degrees = control_points
        .first()
        .is_some_and(|row| !row.is_empty())
        .then(|| surface.degrees());
    capability(control_points, degrees, side, requested)
}

fn capability<P>(
    control_points: &[Vec<P>],
    degrees: Option<(usize, usize)>,
    side: BoundarySide,
    requested: ContinuityOrder,
) -> SurfaceContinuityCapability {
    let dimensions = (
        control_points.len(),
        control_points.first().map_or(0, Vec::len),
    );
    let degrees = degrees.unwrap_or((0, 0));
    SurfaceContinuityCapability::from_degrees_and_dimensions(degrees, dimensions, side, requested)
}
