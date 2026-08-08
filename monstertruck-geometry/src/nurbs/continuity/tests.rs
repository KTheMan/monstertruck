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
