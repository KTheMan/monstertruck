use monstertruck_geometry::base::Vector4;
use monstertruck_geometry::nurbs::continuity::BoundaryAlignment;
use monstertruck_geometry::nurbs::continuity::{BoundarySide, ContinuityOrder};
use monstertruck_geometry::nurbs::continuity_solver::{
    BoundaryContinuityRequest, BoundaryContinuitySolver, ContinuityResource, ContinuitySolveError,
    ContinuitySolverConfig, ContinuityTruncated, ContinuityWorkBudget, take_continuity_work,
};
use monstertruck_geometry::nurbs::{BsplineSurface, KnotVector, NurbsSurface};

#[test]
fn resource_budget_builders_expose_their_limits() {
    let budget = ContinuityWorkBudget::default()
        .with_max_iterations(64)
        .with_max_samples(4_096)
        .with_max_jacobian_elements(1_048_576);

    assert_eq!(budget.max_iterations(), 64);
    assert_eq!(budget.max_samples(), 4_096);
    assert_eq!(budget.max_jacobian_elements(), 1_048_576);
    assert_ne!(budget, ContinuityWorkBudget::default());
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
            ContinuityWorkBudget::unbounded().with_max_control_points(1),
        ),
        (
            ContinuityResource::Spans,
            ContinuityWorkBudget::unbounded().with_max_spans(1),
        ),
        (
            ContinuityResource::Samples,
            ContinuityWorkBudget::unbounded().with_max_samples(1),
        ),
        (
            ContinuityResource::Variables,
            ContinuityWorkBudget::unbounded().with_max_variables(1),
        ),
        (
            ContinuityResource::Residuals,
            ContinuityWorkBudget::unbounded().with_max_residuals(1),
        ),
        (
            ContinuityResource::JacobianElements,
            ContinuityWorkBudget::unbounded().with_max_jacobian_elements(1),
        ),
    ]
    .into_iter()
    .for_each(|(resource, budget)| {
        let (first, second) = adjacent_planes(0.0);
        let original_first = first.clone();
        let original_second = second.clone();
        let error =
            BoundaryContinuitySolver::new_with_budget(ContinuitySolverConfig::default(), budget)
                .expect("the positive test budget is valid")
                .solve(&first, &second, request())
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
fn iteration_limit_is_checked_when_the_solver_is_created() {
    let error = BoundaryContinuitySolver::new_with_budget(
        ContinuitySolverConfig::default().with_max_iterations(9),
        ContinuityWorkBudget::unbounded().with_max_iterations(8),
    )
    .expect_err("the requested iteration count exceeds the explicit budget");

    assert!(matches!(
        error,
        ContinuitySolveError::Truncated(ContinuityTruncated {
            resource: ContinuityResource::Iterations,
            requested: 9,
            budget: 8,
        })
    ));
}

#[test]
fn qr_budget_is_only_required_after_zero_iteration_certification() {
    let budget = ContinuityWorkBudget::unbounded().with_max_qr_elements(0);
    let solver =
        BoundaryContinuitySolver::new_with_budget(ContinuitySolverConfig::default(), budget)
            .expect("the positive QR budget is valid");
    let (first, exact) = adjacent_planes(0.0);

    take_continuity_work();
    solver
        .solve(&first, &exact, request())
        .expect("an exact problem does not allocate an augmented QR matrix");
    let exact_work = take_continuity_work();
    assert_eq!(exact_work.iterations, 0);
    assert!(exact_work.jacobian_elements > 0);
    assert_eq!(exact_work.qr_elements, 0);

    let (_, perturbed) = adjacent_planes(1.0e-3);
    take_continuity_work();
    let error = solver
        .solve(&first, &perturbed, request())
        .expect_err("a non-exact problem requires the bounded QR path");
    let refused_work = take_continuity_work();
    assert!(refused_work.jacobian_elements > 0);
    assert_eq!(refused_work.qr_elements, 0);
    assert!(refused_work.truncated);
    assert!(matches!(
        error,
        ContinuitySolveError::Truncated(ContinuityTruncated {
            resource: ContinuityResource::QrElements,
            requested: 39_204,
            budget: 0,
        })
    ));
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
                ContinuityWorkBudget::unbounded().with_max_control_points(requested - 1)
            }
            ContinuityResource::Spans => {
                ContinuityWorkBudget::unbounded().with_max_spans(requested - 1)
            }
            ContinuityResource::Samples => {
                ContinuityWorkBudget::unbounded().with_max_samples(requested - 1)
            }
            ContinuityResource::Variables => {
                ContinuityWorkBudget::unbounded().with_max_variables(requested - 1)
            }
            ContinuityResource::Residuals => {
                ContinuityWorkBudget::unbounded().with_max_residuals(requested - 1)
            }
            ContinuityResource::JacobianElements => {
                ContinuityWorkBudget::unbounded().with_max_jacobian_elements(requested - 1)
            }
            ContinuityResource::Iterations | ContinuityResource::QrElements => {
                unreachable!("the table covers preparation dimensions only")
            }
        };
        let (first, second) = adjacent_planes(0.0);
        let error =
            BoundaryContinuitySolver::new_with_budget(ContinuitySolverConfig::default(), budget)
                .expect("the threshold-minus-one budget is valid")
                .solve(&first, &second, request())
                .expect_err("the checked representative dimension exceeds its limit");

        assert!(matches!(
            error,
            ContinuitySolveError::Truncated(ContinuityTruncated {
                resource: actual,
                requested: actual_requested,
                budget,
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
