//! Boolean operations over STEP-imported geometry (upstream `ricosjp/truck`
//! issue #91 / PR #111).
//!
//! `monstertruck_solid::{and, or, difference, symmetric_difference}` require
//! the solid's curve/surface types to satisfy [`ShapeOpsCurve`]/
//! [`ShapeOpsSurface`]. The load-side STEP enums [`Curve3D`]/[`Surface`] must
//! therefore implement the whole bound set -- the load-bearing part being
//! `From<IntersectionCurve<BsplineCurve<Point3>, Surface, Surface>>`, which is
//! what issue #91 was missing upstream.
//!
//! This is a compile-time check: if the bounds were unsatisfied the body would
//! fail to type-check. Monstertruck's `From` impl already preserves the
//! intersection surfaces (rather than collapsing to the bspline leader), so a
//! boolean result over STEP geometry round-trips back to STEP with its
//! `IntersectionCurve` topology intact.

use monstertruck_solid::{ShapeOpsCurve, ShapeOpsSurface};
use monstertruck_step::load::step_geometry::{Curve3D, Surface};

fn assert_shapeops_bounds<C: ShapeOpsCurve<S>, S: ShapeOpsSurface>() {}

#[test]
fn step_curve3d_and_surface_satisfy_shapeops_bounds() {
    assert_shapeops_bounds::<Curve3D, Surface>();
}
