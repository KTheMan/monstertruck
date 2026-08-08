use monstertruck_geometry::base::Vector4;
use monstertruck_geometry::nurbs::continuity::{BoundaryAlignment, BoundarySide, ContinuityOrder};
use monstertruck_geometry::nurbs::continuity_solver::{
    BoundaryContinuityRequest, BoundaryContinuitySolver, ContinuityLimits, ContinuitySolverConfig,
    take_continuity_max_work, take_continuity_totals, take_continuity_work,
};
use monstertruck_geometry::nurbs::{BsplineSurface, KnotVector, NurbsSurface};

#[test]
fn budgeted_carrier_matches_thread_process_and_high_water_meters() {
    take_continuity_work();
    take_continuity_totals();
    take_continuity_max_work();
    let first = plane(-1.0);
    let second = plane(0.0);
    let request = BoundaryContinuityRequest::new(
        BoundarySide::MaxU,
        BoundarySide::MinU,
        BoundaryAlignment::Aligned,
        ContinuityOrder::G0,
    );
    let budgeted = BoundaryContinuitySolver::new(ContinuitySolverConfig::default())
        .expect("the solver configuration is valid")
        .solve_with_budget(&first, &second, request, ContinuityLimits::default());

    assert!(budgeted.outcome.is_ok());
    assert_eq!(budgeted.truncated, None);
    assert_eq!(take_continuity_work(), budgeted.work);
    assert_eq!(take_continuity_totals(), (budgeted.work, 0));
    assert_eq!(take_continuity_max_work(), budgeted.work);
}

fn plane(x_start: f64) -> NurbsSurface<Vector4> {
    NurbsSurface::new(BsplineSurface::new(
        (KnotVector::bezier_knot(1), KnotVector::bezier_knot(1)),
        vec![
            vec![
                Vector4::new(x_start, 0.0, 0.0, 1.0),
                Vector4::new(x_start, 1.0, 0.0, 1.0),
            ],
            vec![
                Vector4::new(x_start + 1.0, 0.0, 0.0, 1.0),
                Vector4::new(x_start + 1.0, 1.0, 0.0, 1.0),
            ],
        ],
    ))
}
