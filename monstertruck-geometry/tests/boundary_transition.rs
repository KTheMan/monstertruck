use monstertruck_geometry::base::{ParametricSurface, Vector4};
use monstertruck_geometry::nurbs::continuity::BoundaryAlignment;
use monstertruck_geometry::nurbs::continuity::{BoundarySide, ContinuityOrder};
use monstertruck_geometry::nurbs::continuity_solver::{
    BoundaryContinuityRequest, BoundaryContinuitySolver, ContinuitySolverConfig,
};
use monstertruck_geometry::nurbs::{BsplineSurface, KnotVector, NurbsSurface};

const SEAM_DEGREE: usize = 3;

#[test]
fn exact_solutions_expose_the_solver_coordinate_transition_through_g4() {
    [
        ContinuityOrder::G0,
        ContinuityOrder::G1,
        ContinuityOrder::G2,
        ContinuityOrder::G3,
        ContinuityOrder::G4,
    ]
    .into_iter()
    .for_each(|order| {
        [BoundaryAlignment::Aligned, BoundaryAlignment::Reversed]
            .into_iter()
            .for_each(|alignment| {
                let (first, second) = adjacent_planes(7, alignment);
                let config = ContinuitySolverConfig::default()
                    .with_experimental_g4(order == ContinuityOrder::G4);
                let solution = BoundaryContinuitySolver::new(config)
                    .expect("the transition test configuration is valid")
                    .solve(&first, &second, request(order, alignment))
                    .expect("the exact surfaces satisfy the requested continuity");
                let transition = solution.transition();

                assert_eq!(transition.order(), order);
                assert_eq!(transition.alignment(), alignment);
                assert_eq!(transition.cross_field_degree(), SEAM_DEGREE);
                assert_eq!(transition.seam_map_degree(), SEAM_DEGREE + 1);
                [0.0, 0.23, 0.71, 1.0].into_iter().for_each(|seam| {
                    [-0.1, 0.0, 0.1].into_iter().for_each(|cross| {
                        let mapped = transition
                            .mapped_coordinates(seam, cross)
                            .expect("a solved finite transition evaluates");
                        let expected_seam = match alignment {
                            BoundaryAlignment::Aligned => seam,
                            BoundaryAlignment::Reversed => 1.0 - seam,
                        };
                        assert!((mapped.0 - expected_seam).abs() < 1.0e-12);
                        assert!((mapped.1 - cross).abs() < 1.0e-12);
                    });
                });
                assert!(transition.mapped_coordinates(f64::NAN, 0.0).is_none());
                assert!(transition.mapped_coordinates(0.5, f64::INFINITY).is_none());
            });
    });
}

#[test]
fn nonlinear_seam_map_replays_the_solved_boundary_correspondence() {
    let (first, second) = nonlinearly_parameterized_planes();
    let config = ContinuitySolverConfig::default()
        .with_anchor_weight(1.0)
        .with_fairness_weight(0.0)
        .with_transition_weight(0.0)
        .with_max_iterations(80);
    let solution = BoundaryContinuitySolver::new(config)
        .expect("the nonlinear transition configuration is valid")
        .solve(
            &first,
            &second,
            request(ContinuityOrder::G1, BoundaryAlignment::Aligned),
        )
        .expect("the seam map absorbs the nonlinear parameterization");
    let transition = solution.transition();

    assert_eq!(transition.order(), ContinuityOrder::G1);
    assert!([0.2, 0.4, 0.6, 0.8].into_iter().any(|seam| {
        (transition
            .mapped_coordinates(seam, 0.0)
            .expect("the solved transition evaluates")
            .0
            - seam)
            .abs()
            > 1.0e-3
    }));
    [0.0, 0.13, 0.37, 0.68, 1.0].into_iter().for_each(|seam| {
        let (second_seam, second_cross) = transition
            .mapped_coordinates(seam, 0.0)
            .expect("the solved transition evaluates on the seam");
        let first_point = first.evaluate(1.0, seam);
        let second_point = solution.second().evaluate(second_cross, second_seam);

        assert!((first_point.x - second_point.x).abs() < 1.0e-8);
        assert!((first_point.y - second_point.y).abs() < 1.0e-8);
        assert!((first_point.z - second_point.z).abs() < 1.0e-8);
    });
}

#[test]
fn consuming_solution_parts_can_retain_the_transition() {
    let (first, second) = adjacent_planes(5, BoundaryAlignment::Aligned);
    let solution = BoundaryContinuitySolver::new(ContinuitySolverConfig::default())
        .expect("the transition test configuration is valid")
        .solve(
            &first,
            &second,
            request(ContinuityOrder::G3, BoundaryAlignment::Aligned),
        )
        .expect("the exact surfaces satisfy G3 continuity");

    let (solved_first, solved_second, transition, report) = solution.into_parts_with_transition();

    assert_eq!(solved_first, &first);
    assert_eq!(solved_second, second);
    assert_eq!(transition.order(), ContinuityOrder::G3);
    assert_eq!(report.iterations(), 0);
}

fn request(order: ContinuityOrder, alignment: BoundaryAlignment) -> BoundaryContinuityRequest {
    BoundaryContinuityRequest::new(BoundarySide::MaxU, BoundarySide::MinU, alignment, order)
}

fn adjacent_planes(
    cross_degree: usize,
    alignment: BoundaryAlignment,
) -> (NurbsSurface<Vector4>, NurbsSurface<Vector4>) {
    (
        plane(cross_degree, false, BoundaryAlignment::Aligned),
        plane(cross_degree, true, alignment),
    )
}

fn plane(cross_degree: usize, second: bool, alignment: BoundaryAlignment) -> NurbsSurface<Vector4> {
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
                    Vector4::new(x, y, 0.0, 1.0)
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
    let master_seam = [0.0, 0.08, 0.36, 0.76, 1.0];
    let linear_seam = [0.0, 0.25, 0.5, 0.75, 1.0];
    let surface = |second: bool| {
        let seam_values = if second { linear_seam } else { master_seam };
        let control_points = (0..=3)
            .map(|cross| {
                seam_values
                    .into_iter()
                    .map(|seam| {
                        let x = cross as f64 / 3.0 - if second { 0.0 } else { 1.0 };
                        Vector4::new(x, seam, 0.0, 1.0)
                    })
                    .collect()
            })
            .collect();
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
