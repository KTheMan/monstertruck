use monstertruck_geometry::base::Vector4;
use monstertruck_geometry::nurbs::continuity::BoundaryAlignment;
use monstertruck_geometry::nurbs::continuity::{BoundarySide, ContinuityOrder};
use monstertruck_geometry::nurbs::continuity_solver::{
    BoundaryContinuityRequest, BoundaryContinuitySolver, ContinuityLimits, ContinuityResource,
    ContinuitySolveError, ContinuitySolverConfig, ContinuityTruncated, take_continuity_work,
};
use monstertruck_geometry::nurbs::{BsplineSurface, KnotVector, NurbsSurface};

#[test]
fn resource_budget_builders_expose_their_limits() {
    let budget = ContinuityLimits::default()
        .with_max_iterations(64)
        .with_max_samples(4_096)
        .with_max_jacobian_elements(1_048_576);

    assert_eq!(budget.max_iterations(), 64);
    assert_eq!(budget.max_samples(), 4_096);
    assert_eq!(budget.max_jacobian_elements(), 1_048_576);
    assert_ne!(budget, ContinuityLimits::default());
}

#[test]
fn default_budget_accepts_representative_g3_problem() {
    let (first, second) = adjacent_planes(0.0);
    let solution = BoundaryContinuitySolver::new(ContinuitySolverConfig::default())
        .expect("the default configuration and budget are valid")
        .solve(&first, &second, request())
        .expect("the exact representative problem fits the default budget");

    assert_eq!(solution.first(), &first);
    assert_eq!(solution.second(), &second);
}

#[test]
fn every_preflight_dimension_has_a_typed_limit() {
    [
        (
            ContinuityResource::ControlPoints,
            ContinuityLimits::unbounded().with_max_control_points(1),
        ),
        (
            ContinuityResource::Spans,
            ContinuityLimits::unbounded().with_max_spans(1),
        ),
        (
            ContinuityResource::Samples,
            ContinuityLimits::unbounded().with_max_samples(1),
        ),
        (
            ContinuityResource::Variables,
            ContinuityLimits::unbounded().with_max_variables(1),
        ),
        (
            ContinuityResource::Residuals,
            ContinuityLimits::unbounded().with_max_residuals(1),
        ),
        (
            ContinuityResource::JacobianElements,
            ContinuityLimits::unbounded().with_max_jacobian_elements(1),
        ),
    ]
    .into_iter()
    .for_each(|(resource, budget)| {
        let (first, second) = adjacent_planes(0.0);
        let original_first = first.clone();
        let original_second = second.clone();
        let budgeted = BoundaryContinuitySolver::new(ContinuitySolverConfig::default())
            .expect("the solver configuration is valid")
            .solve_with_budget(&first, &second, request(), budget);
        let error = budgeted
            .outcome
            .expect_err("the selected dimension exceeds its one-element limit");

        assert!(matches!(
            error,
            ContinuitySolveError::Truncated(ContinuityTruncated {
                resource: actual,
                ..
            }) if actual == resource
        ));
        assert_eq!(first, original_first);
        assert_eq!(second, original_second);
    });
}

#[test]
fn iteration_limit_is_charged_only_when_an_iteration_is_attempted() {
    let solver =
        BoundaryContinuitySolver::new(ContinuitySolverConfig::default().with_max_iterations(9))
            .expect("the solver configuration is valid");
    let (first, exact) = adjacent_planes(0.0);
    let exact = solver.solve_with_budget(
        &first,
        &exact,
        request(),
        ContinuityLimits::unbounded().with_max_iterations(0),
    );

    assert!(exact.outcome.is_ok());
    assert_eq!(exact.work.iterations, 0);
    assert_eq!(exact.truncated, None);

    let (_, perturbed) = adjacent_planes(1.0e-3);
    let refused = solver.solve_with_budget(
        &first,
        &perturbed,
        request(),
        ContinuityLimits::unbounded().with_max_iterations(0),
    );
    let error = refused
        .outcome
        .expect_err("the first attempted iteration exceeds the explicit budget");

    assert!(matches!(
        error,
        ContinuitySolveError::Truncated(ContinuityTruncated {
            resource: ContinuityResource::Iterations,
            spent: 0,
            requested: 1,
            budget: 0,
        })
    ));
}

#[test]
fn qr_budget_is_only_required_after_zero_iteration_certification() {
    let budget = ContinuityLimits::unbounded().with_max_qr_elements(0);
    let solver = BoundaryContinuitySolver::new(ContinuitySolverConfig::default())
        .expect("the solver configuration is valid");
    let (first, exact) = adjacent_planes(0.0);

    take_continuity_work();
    let exact = solver.solve_with_budget(&first, &exact, request(), budget);
    exact
        .outcome
        .expect("an exact problem does not allocate an augmented QR matrix");
    let exact_work = take_continuity_work();
    assert_eq!(exact.work, exact_work);
    assert_eq!(exact.truncated, None);
    assert_eq!(exact_work.iterations, 0);
    assert!(exact_work.jacobian_elements > 0);
    assert_eq!(exact_work.qr_elements, 0);

    let (_, perturbed) = adjacent_planes(1.0e-3);
    take_continuity_work();
    let refused = solver.solve_with_budget(&first, &perturbed, request(), budget);
    let error = refused
        .outcome
        .expect_err("a non-exact problem requires the bounded QR path");
    let refused_work = take_continuity_work();
    assert_eq!(refused.work, refused_work);
    assert_eq!(
        refused.truncated,
        Some(match error {
            ContinuitySolveError::Truncated(truncated) => truncated,
            _ => panic!("the explicit QR limit must produce a typed refusal"),
        })
    );
    assert!(refused_work.jacobian_elements > 0);
    assert_eq!(refused_work.qr_elements, 0);
    assert!(refused_work.truncated);
    assert!(matches!(
        refused.truncated,
        Some(ContinuityTruncated {
            resource: ContinuityResource::QrElements,
            spent: 0,
            requested: 39_204,
            budget: 0,
        })
    ));
}

#[test]
fn jacobian_and_qr_limits_apply_to_cumulative_actual_work() {
    let solver = BoundaryContinuitySolver::new(ContinuitySolverConfig::default())
        .expect("the solver configuration is valid");
    let (first, perturbed) = adjacent_planes(1.0e-3);
    let jacobian = solver.solve_with_budget(
        &first,
        &perturbed,
        request(),
        ContinuityLimits::unbounded().with_max_jacobian_elements(29_403),
    );

    assert!(matches!(
        jacobian.truncated,
        Some(ContinuityTruncated {
            resource: ContinuityResource::JacobianElements,
            spent: 29_403,
            requested: 58_806,
            budget: 29_403,
        })
    ));
    assert_eq!(jacobian.work.jacobian_elements, 29_403);

    let (_, qr_perturbed) = adjacent_planes(1.0e-2);
    let qr = solver.solve_with_budget(
        &first,
        &qr_perturbed,
        request(),
        ContinuityLimits::unbounded().with_max_qr_elements(39_204),
    );

    assert_eq!(
        qr.truncated,
        Some(ContinuityTruncated {
            resource: ContinuityResource::QrElements,
            spent: 39_204,
            requested: 78_408,
            budget: 39_204,
        })
    );
    assert_eq!(qr.work.qr_elements, 39_204);
}

#[test]
fn representative_g3_dimensions_match_the_checked_preflight() {
    let expected = [
        (ContinuityResource::ControlPoints, 48),
        (ContinuityResource::Spans, 2),
        (ContinuityResource::Samples, 25),
        (ContinuityResource::Variables, 99),
        (ContinuityResource::Residuals, 897),
        (ContinuityResource::JacobianElements, 29_403),
    ];

    expected.into_iter().for_each(|(resource, requested)| {
        let budget = match resource {
            ContinuityResource::ControlPoints => {
                ContinuityLimits::unbounded().with_max_control_points(requested - 1)
            }
            ContinuityResource::Spans => {
                ContinuityLimits::unbounded().with_max_spans(requested - 1)
            }
            ContinuityResource::Samples => {
                ContinuityLimits::unbounded().with_max_samples(requested - 1)
            }
            ContinuityResource::Variables => {
                ContinuityLimits::unbounded().with_max_variables(requested - 1)
            }
            ContinuityResource::Residuals => {
                ContinuityLimits::unbounded().with_max_residuals(requested - 1)
            }
            ContinuityResource::JacobianElements => {
                ContinuityLimits::unbounded().with_max_jacobian_elements(requested - 1)
            }
            ContinuityResource::Iterations | ContinuityResource::QrElements => {
                unreachable!("the table covers preparation dimensions only")
            }
        };
        let (first, second) = adjacent_planes(0.0);
        let budgeted = BoundaryContinuitySolver::new(ContinuitySolverConfig::default())
            .expect("the solver configuration is valid")
            .solve_with_budget(&first, &second, request(), budget);
        let error = budgeted
            .outcome
            .expect_err("the checked representative dimension exceeds its limit");

        assert!(matches!(
            error,
            ContinuitySolveError::Truncated(ContinuityTruncated {
                resource: actual,
                requested: actual_requested,
                budget,
                ..
            }) if actual == resource
                && actual_requested == requested
                && budget == requested - 1
        ));
    });
}

fn request() -> BoundaryContinuityRequest {
    BoundaryContinuityRequest::new(
        BoundarySide::MaxU,
        BoundarySide::MinU,
        BoundaryAlignment::Aligned,
        ContinuityOrder::G3,
    )
}

fn adjacent_planes(boundary_offset: f64) -> (NurbsSurface<Vector4>, NurbsSurface<Vector4>) {
    (plane(false, 0.0), plane(true, boundary_offset))
}

fn plane(second: bool, boundary_offset: f64) -> NurbsSurface<Vector4> {
    const CROSS_DEGREE: usize = 5;
    const SEAM_DEGREE: usize = 3;

    let control_points = (0..=CROSS_DEGREE)
        .map(|cross| {
            (0..=SEAM_DEGREE)
                .map(|seam| {
                    let x = cross as f64 / CROSS_DEGREE as f64 - if second { 0.0 } else { 1.0 };
                    let y = seam as f64 / SEAM_DEGREE as f64;
                    let z = if second && cross < 4 {
                        boundary_offset * [1.0, -0.5, 0.25, -0.125][cross]
                    } else {
                        0.0
                    };
                    Vector4::new(x, y, z, 1.0)
                })
                .collect()
        })
        .collect();
    NurbsSurface::new(BsplineSurface::new(
        (
            KnotVector::bezier_knot(CROSS_DEGREE),
            KnotVector::bezier_knot(SEAM_DEGREE),
        ),
        control_points,
    ))
}
