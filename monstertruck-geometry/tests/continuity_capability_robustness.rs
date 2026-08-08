use monstertruck_geometry::base::Vector4;
use monstertruck_geometry::nurbs::continuity::{
    BoundarySide, ContinuityOrder, capability_for_bspline, capability_for_nurbs,
};
use monstertruck_geometry::nurbs::{BsplineSurface, KnotVector, NurbsSurface};

#[test]
fn empty_control_nets_report_unsupported_without_panicking() {
    let polynomial = BsplineSurface::<Vector4>::new_unchecked(
        (
            KnotVector::from(vec![0.0, 0.0]),
            KnotVector::from(vec![0.0, 0.0]),
        ),
        Vec::new(),
    );
    let rational = NurbsSurface::new(polynomial.clone());

    [
        capability_for_bspline(&polynomial, BoundarySide::MaxU, ContinuityOrder::G3),
        capability_for_nurbs(&rational, BoundarySide::MinV, ContinuityOrder::G3),
    ]
    .into_iter()
    .for_each(|capability| {
        assert!(capability.unsupported_reason().is_some());
        assert_eq!(capability.cross_control_rows(), 0);
    });
}

#[test]
fn malformed_nonempty_control_nets_report_unsupported_without_panicking() {
    let point = Vector4::new(0.0, 0.0, 0.0, 1.0);
    let net = || vec![vec![point; 2]; 2];
    let short_knots = BsplineSurface::new_unchecked(
        (
            KnotVector::from(vec![0.0, 1.0]),
            KnotVector::from(vec![0.0, 1.0]),
        ),
        net(),
    );
    let nonrectangular = BsplineSurface::new_unchecked(
        (
            KnotVector::from(vec![0.0, 0.0, 1.0, 1.0]),
            KnotVector::from(vec![0.0, 0.0, 1.0, 1.0]),
        ),
        vec![vec![point; 2], vec![point]],
    );
    let invalid_knots = [
        vec![0.0, 0.0, f64::INFINITY, 1.0],
        vec![0.0, 0.0, 1.0, 0.5],
        vec![0.0, 0.25, 0.75, 1.0],
    ]
    .map(|values| {
        BsplineSurface::new_unchecked(
            (
                KnotVector::from(values),
                KnotVector::from(vec![0.0, 0.0, 1.0, 1.0]),
            ),
            net(),
        )
    });

    let capabilities =
        [
            capability_for_bspline(&short_knots, BoundarySide::MinU, ContinuityOrder::G0),
            capability_for_nurbs(
                &NurbsSurface::new(nonrectangular),
                BoundarySide::MaxV,
                ContinuityOrder::G0,
            ),
        ]
        .into_iter()
        .chain(invalid_knots.iter().map(|surface| {
            capability_for_bspline(surface, BoundarySide::MaxU, ContinuityOrder::G1)
        }));

    capabilities.for_each(|capability| {
        assert!(capability.unsupported_reason().is_some());
        assert_eq!(capability.cross_control_rows(), 0);
    });
}
