use super::{
    BoundaryContinuityRequest, BoundaryContinuitySolver, ContinuitySolveError,
    ContinuitySolverConfig, ContinuityTermination,
};
use crate::base::Vector4;
use crate::nurbs::continuity::BoundaryAlignment;
use crate::nurbs::continuity::{BoundarySide, ContinuityMaturity, ContinuityOrder};
use crate::nurbs::{BsplineSurface, KnotVector, NurbsSurface};

const SEAM_DEGREE: usize = 3;

#[test]
fn solves_exact_boundaries_through_g3() {
    let (first, second) = adjacent_planes(5, BoundaryAlignment::Aligned, false, 0.0);

    [
        ContinuityOrder::G0,
        ContinuityOrder::G1,
        ContinuityOrder::G2,
        ContinuityOrder::G3,
    ]
    .into_iter()
    .for_each(|order| {
        let config = ContinuitySolverConfig::default();
        let solution = BoundaryContinuitySolver::new(config.clone())
            .expect("the default configuration is valid")
            .solve(&first, &second, request(order, BoundaryAlignment::Aligned))
            .expect("the exact adjacent planes satisfy the requested continuity");

        assert_eq!(solution.first(), &first);
        assert_eq!(solution.second(), &second);
        assert_converged(solution.report(), &config, order);
        assert_eq!(solution.report().maturity(), order.maturity());
        assert_eq!(solution.report().iterations(), 0);
        assert_eq!(solution.report().accepted_steps(), 0);
    });
}

#[test]
fn solves_g3_with_reversed_boundary_alignment() {
    let (first, second) = adjacent_planes(5, BoundaryAlignment::Reversed, false, 0.0);
    let config = ContinuitySolverConfig::default();
    let solution = BoundaryContinuitySolver::new(config.clone())
        .expect("the default configuration is valid")
        .solve(
            &first,
            &second,
            request(ContinuityOrder::G3, BoundaryAlignment::Reversed),
        )
        .expect("the reversed parameterization represents the same boundary jet");

    assert_eq!(solution.second(), &second);
    assert_converged(solution.report(), &config, ContinuityOrder::G3);
}

#[test]
fn solves_g3_for_rational_boundary_parameterization() {
    let (first, second) = adjacent_planes(5, BoundaryAlignment::Aligned, true, 0.0);
    let config = ContinuitySolverConfig::default();
    let solution = BoundaryContinuitySolver::new(config.clone())
        .expect("the default configuration is valid")
        .solve(
            &first,
            &second,
            request(ContinuityOrder::G3, BoundaryAlignment::Aligned),
        )
        .expect("matching positive weights preserve the shared rational boundary jet");

    assert_eq!(solution.second(), &second);
    assert_converged(solution.report(), &config, ContinuityOrder::G3);
}

#[test]
fn repairs_a_small_g3_boundary_strip_offset() {
    let (first, second) = adjacent_planes(5, BoundaryAlignment::Aligned, false, 1.0e-3);
    let config = ContinuitySolverConfig::default()
        .with_anchor_weight(0.0)
        .with_fairness_weight(0.0)
        .with_max_iterations(80);
    let solution = BoundaryContinuitySolver::new(config.clone())
        .expect("the test configuration is valid")
        .solve(
            &first,
            &second,
            request(ContinuityOrder::G3, BoundaryAlignment::Aligned),
        )
        .expect("the solver repairs a small boundary-strip displacement");

    assert_ne!(solution.second(), &second);
    assert_converged(solution.report(), &config, ContinuityOrder::G3);
    assert!(solution.report().accepted_steps() > 0);
    assert!(solution.report().final_objective() < solution.report().initial_objective());
}

#[test]
fn g4_requires_opt_in_and_remains_reachable() {
    let (first, second) = adjacent_planes(7, BoundaryAlignment::Aligned, false, 0.0);
    let request = request(ContinuityOrder::G4, BoundaryAlignment::Aligned);
    let disabled = BoundaryContinuitySolver::new(ContinuitySolverConfig::default())
        .expect("the default configuration is valid")
        .solve(&first, &second, request);

    assert!(matches!(
        disabled,
        Err(ContinuitySolveError::ExperimentalG4Disabled)
    ));

    let config = ContinuitySolverConfig::default().with_experimental_g4(true);
    let solution = BoundaryContinuitySolver::new(config.clone())
        .expect("the experimental configuration is valid")
        .solve(&first, &second, request)
        .expect("an exact degree-seven boundary jet keeps G4 reachable");

    assert_eq!(solution.second(), &second);
    assert_converged(solution.report(), &config, ContinuityOrder::G4);
    assert_eq!(
        solution.report().maturity(),
        ContinuityMaturity::Experimental
    );
}

#[test]
fn rejects_invalid_inputs_without_mutating_borrowed_surfaces() {
    let (first, mut second) = adjacent_planes(5, BoundaryAlignment::Aligned, false, 0.0);
    second.control_point_mut(0, 2).w = 0.0;
    let original_first = first.clone();
    let original_second = second.clone();
    let error = BoundaryContinuitySolver::new(ContinuitySolverConfig::default())
        .expect("the default configuration is valid")
        .solve(
            &first,
            &second,
            request(ContinuityOrder::G3, BoundaryAlignment::Aligned),
        )
        .expect_err("a zero rational weight is rejected before solving");

    assert!(matches!(
        error,
        ContinuitySolveError::NonPositiveWeight {
            endpoint: super::BoundaryEndpoint::Second,
            row: 0,
            column: 2,
            weight: 0.0,
        }
    ));
    assert_eq!(first, original_first);
    assert_eq!(second, original_second);
}

#[test]
fn rejects_insufficient_degree_and_degenerate_boundaries() {
    let (low_first, low_second) = adjacent_planes(2, BoundaryAlignment::Aligned, false, 0.0);
    let solver = BoundaryContinuitySolver::new(ContinuitySolverConfig::default())
        .expect("the default configuration is valid");
    assert!(matches!(
        solver.solve(
            &low_first,
            &low_second,
            request(ContinuityOrder::G3, BoundaryAlignment::Aligned),
        ),
        Err(ContinuitySolveError::UnsupportedCapability {
            endpoint: super::BoundaryEndpoint::First,
            ..
        })
    ));

    let (mut degenerate, second) = adjacent_planes(5, BoundaryAlignment::Aligned, false, 0.0);
    degenerate
        .control_points_mut()
        .for_each(|point| point.x = 0.0);
    assert!(matches!(
        solver.solve(
            &degenerate,
            &second,
            request(ContinuityOrder::G3, BoundaryAlignment::Aligned),
        ),
        Err(ContinuitySolveError::DegenerateBoundary {
            endpoint: super::BoundaryEndpoint::First,
            ..
        })
    ));
}

#[test]
fn configuration_rejects_excessive_sampling() {
    assert!(matches!(
        BoundaryContinuitySolver::new(ContinuitySolverConfig::default().with_samples_per_span(65),),
        Err(ContinuitySolveError::InvalidConfig(_))
    ));
}

#[test]
fn solves_geometric_continuity_across_nonlinear_seam_parameters() {
    let (first, second) = nonlinearly_parameterized_planes();
    let config = ContinuitySolverConfig::default()
        .with_anchor_weight(1.0)
        .with_fairness_weight(0.0)
        .with_transition_weight(0.0)
        .with_max_iterations(80);
    let solution = BoundaryContinuitySolver::new(config.clone())
        .expect("the nonlinear-seam configuration is valid")
        .solve(
            &first,
            &second,
            request(ContinuityOrder::G1, BoundaryAlignment::Aligned),
        )
        .expect("the monotone seam map absorbs the parameterization mismatch");

    assert_converged(solution.report(), &config, ContinuityOrder::G1);
    let maximum_control_displacement = solution
        .second()
        .control_points()
        .iter()
        .flatten()
        .zip(second.control_points().iter().flatten())
        .map(|(solved, original)| {
            ((solved.x - original.x).powi(2)
                + (solved.y - original.y).powi(2)
                + (solved.z - original.z).powi(2))
            .sqrt()
        })
        .fold(0.0, f64::max);
    assert!(maximum_control_displacement < 1.0e-5);
}

fn request(order: ContinuityOrder, alignment: BoundaryAlignment) -> BoundaryContinuityRequest {
    BoundaryContinuityRequest::new(BoundarySide::MaxU, BoundarySide::MinU, alignment, order)
}

fn adjacent_planes(
    cross_degree: usize,
    alignment: BoundaryAlignment,
    rational: bool,
    boundary_offset: f64,
) -> (NurbsSurface<Vector4>, NurbsSurface<Vector4>) {
    let first = bezier_plane(cross_degree, false, BoundaryAlignment::Aligned, false, 0.0);
    let second = bezier_plane(cross_degree, true, alignment, rational, boundary_offset);
    let first = if rational {
        bezier_plane(cross_degree, false, BoundaryAlignment::Aligned, true, 0.0)
    } else {
        first
    };
    (first, second)
}

fn bezier_plane(
    cross_degree: usize,
    second: bool,
    alignment: BoundaryAlignment,
    rational: bool,
    boundary_offset: f64,
) -> NurbsSurface<Vector4> {
    let control_points = (0..=cross_degree)
        .map(|cross| {
            (0..=SEAM_DEGREE)
                .map(|seam| {
                    let normalized_cross = cross as f64 / cross_degree as f64;
                    let normalized_seam = seam as f64 / SEAM_DEGREE as f64;
                    let y = match (second, alignment) {
                        (true, BoundaryAlignment::Reversed) => 1.0 - normalized_seam,
                        _ => normalized_seam,
                    };
                    let x = normalized_cross - if second { 0.0 } else { 1.0 };
                    let z = if second && cross < 4 {
                        boundary_offset * [1.0, -0.5, 0.25, -0.125][cross]
                    } else {
                        0.0
                    };
                    let weight = if rational {
                        1.0 + 0.2 * seam as f64
                    } else {
                        1.0
                    };
                    Vector4::new(x * weight, y * weight, z * weight, weight)
                })
                .collect()
        })
        .collect();
    NurbsSurface::new(BsplineSurface::new(
        (
            KnotVector::bezier_knot(cross_degree),
            KnotVector::bezier_knot(SEAM_DEGREE),
        ),
        control_points,
    ))
}

fn nonlinearly_parameterized_planes() -> (NurbsSurface<Vector4>, NurbsSurface<Vector4>) {
    const SEAM_CONTROLS: usize = 5;
    let first_seam = [0.0, 0.08, 0.36, 0.76, 1.0];
    let linear_seam = [0.0, 0.25, 0.5, 0.75, 1.0];
    let surface = |second: bool| {
        let seam_values = if second { linear_seam } else { first_seam };
        let control_points = (0..=3)
            .map(|cross| {
                seam_values
                    .into_iter()
                    .map(|seam| {
                        let x = cross as f64 / 3.0 - if second { 0.0 } else { 1.0 };
                        Vector4::new(x, seam, 0.0, 1.0)
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        NurbsSurface::new(BsplineSurface::new(
            (
                KnotVector::bezier_knot(3),
                KnotVector::bezier_knot(SEAM_CONTROLS - 1),
            ),
            control_points,
        ))
    };
    (surface(false), surface(true))
}

fn assert_converged(
    report: &super::ContinuitySolveReport,
    config: &ContinuitySolverConfig,
    requested: ContinuityOrder,
) {
    assert_eq!(report.termination(), ContinuityTermination::Converged);
    assert_eq!(report.residuals().len(), requested.as_usize() + 1);
    report.residuals().iter().for_each(|residual| {
        assert!(
            residual.maximum() <= config.tolerance(residual.order()),
            "{:?} maximum residual {} exceeds tolerance {}",
            residual.order(),
            residual.maximum(),
            config.tolerance(residual.order()),
        );
    });
}
