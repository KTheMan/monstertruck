use monstertruck_geometry::base::Vector4;
use monstertruck_geometry::nurbs::continuity::{
    BoundarySide, ContinuityCapabilityLevel, ContinuityOrder, capability_for_bspline,
    capability_for_nurbs,
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
        assert_eq!(capability.level(), ContinuityCapabilityLevel::Unsupported);
        assert_eq!(capability.cross_control_rows(), 0);
    });
}
