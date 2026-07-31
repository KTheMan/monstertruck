use monstertruck_geometry::base::Vector4;
use monstertruck_geometry::nurbs::continuity::{
    ContinuityCapabilityLevel, ContinuityOrder, SurfaceBoundary, SurfaceContinuityCapability,
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
        SurfaceContinuityCapability::for_bspline(
            &polynomial,
            SurfaceBoundary::UEnd,
            ContinuityOrder::G3,
        ),
        SurfaceContinuityCapability::for_nurbs(
            &rational,
            SurfaceBoundary::VStart,
            ContinuityOrder::G3,
        ),
    ]
    .into_iter()
    .for_each(|capability| {
        assert_eq!(capability.level(), ContinuityCapabilityLevel::Unsupported);
        assert_eq!(capability.cross_control_rows(), 0);
    });
}
