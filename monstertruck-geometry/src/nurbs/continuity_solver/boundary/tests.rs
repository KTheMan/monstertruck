use super::*;
use crate::nurbs::BsplineSurface;

fn surface() -> NurbsSurface<Vector4> {
    let u_knots = KnotVector::from(vec![2.0, 2.0, 2.0, 3.0, 5.0, 5.0, 5.0]);
    let v_knots = KnotVector::from(vec![-4.0, -4.0, 0.0, 7.0, 7.0]);
    let control_points = (0..4)
        .map(|u| {
            (0..3)
                .map(|v| Vector4::new(u as f64, v as f64, (u + v) as f64, 1.0))
                .collect()
        })
        .collect();
    NurbsSurface::new(BsplineSurface::new((u_knots, v_knots), control_points))
}

fn frame(boundary: SurfaceBoundary) -> BoundaryFrame {
    BoundaryFrame::try_new(&surface(), boundary)
        .expect("the test surface has finite clamped domains")
}

fn assert_close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() <= f64::EPSILON);
}

#[test]
fn all_boundaries_map_normalized_coordinates_inward() {
    let cases = [
        (SurfaceBoundary::UStart, (3.5, -1.25), 11.0, 3.0),
        (SurfaceBoundary::UEnd, (3.5, -1.25), 11.0, -3.0),
        (SurfaceBoundary::VStart, (2.75, 1.5), 3.0, 11.0),
        (SurfaceBoundary::VEnd, (2.75, 1.5), 3.0, -11.0),
    ];
    cases
        .into_iter()
        .for_each(|(boundary, expected, along_span, inward_scale)| {
            let frame = frame(boundary);
            let actual = frame.parameters(0.25, 0.5);
            assert_close(actual.0, expected.0);
            assert_close(actual.1, expected.1);
            assert_close(frame.along_parameter_span(), along_span);
            assert_close(frame.inward_parameter_scale(), inward_scale);
        });
}

#[test]
fn frames_expose_axis_degree_and_control_layout() {
    let u_boundary = frame(SurfaceBoundary::UStart);
    assert_eq!(u_boundary.boundary(), SurfaceBoundary::UStart);
    assert_eq!(u_boundary.along_axis(), SurfaceAxis::V);
    assert_eq!(u_boundary.cross_axis(), SurfaceAxis::U);
    assert_eq!(u_boundary.along_degree(), 1);
    assert_eq!(u_boundary.cross_degree(), 2);
    assert_eq!(u_boundary.along_control_count(), 3);
    assert_eq!(u_boundary.cross_control_count(), 4);
    assert_eq!(
        u_boundary.u_domain(),
        ParameterDomain {
            start: 2.0,
            end: 5.0
        }
    );
    assert_eq!(
        u_boundary.v_domain(),
        ParameterDomain {
            start: -4.0,
            end: 7.0
        }
    );
    assert_close(u_boundary.u_domain().start(), 2.0);
    assert_close(u_boundary.u_domain().end(), 5.0);

    let v_boundary = frame(SurfaceBoundary::VStart);
    assert_eq!(v_boundary.along_axis(), SurfaceAxis::U);
    assert_eq!(v_boundary.cross_axis(), SurfaceAxis::V);
    assert_eq!(v_boundary.along_degree(), 2);
    assert_eq!(v_boundary.cross_degree(), 1);
    assert_eq!(v_boundary.along_control_count(), 4);
    assert_eq!(v_boundary.cross_control_count(), 3);
}

#[test]
fn reversed_alignment_reflects_normalized_seam_and_derivative_sign() {
    assert_close(map_normalized_seam(0.2, BoundaryAlignment::Aligned), 0.2);
    assert_close(map_normalized_seam(0.2, BoundaryAlignment::Reversed), 0.8);
    assert_close(seam_alignment_sign(BoundaryAlignment::Aligned), 1.0);
    assert_close(seam_alignment_sign(BoundaryAlignment::Reversed), -1.0);
}

#[test]
fn strip_indices_use_boundary_distance_then_seam_order() {
    assert_eq!(
        frame(SurfaceBoundary::UStart)
            .control_strip_indices(2)
            .expect("two rows fit"),
        vec![(0, 0), (0, 1), (0, 2), (1, 0), (1, 1), (1, 2)],
    );
    assert_eq!(
        frame(SurfaceBoundary::UEnd)
            .control_strip_indices(2)
            .expect("two rows fit"),
        vec![(3, 0), (3, 1), (3, 2), (2, 0), (2, 1), (2, 2)],
    );
    assert_eq!(
        frame(SurfaceBoundary::VStart)
            .control_strip_indices(2)
            .expect("two rows fit"),
        vec![
            (0, 0),
            (1, 0),
            (2, 0),
            (3, 0),
            (0, 1),
            (1, 1),
            (2, 1),
            (3, 1),
        ],
    );
    assert_eq!(
        frame(SurfaceBoundary::VEnd)
            .control_strip_indices(2)
            .expect("two rows fit"),
        vec![
            (0, 2),
            (1, 2),
            (2, 2),
            (3, 2),
            (0, 1),
            (1, 1),
            (2, 1),
            (3, 1),
        ],
    );
}

#[test]
fn invalid_surface_layouts_return_errors_without_panicking() {
    let valid_knots = KnotVector::bezier_knot(1);
    let empty = NurbsSurface::new(BsplineSurface::new_unchecked(
        (valid_knots.clone(), valid_knots.clone()),
        Vec::<Vec<Vector4>>::new(),
    ));
    assert_eq!(
        BoundaryFrame::try_new(&empty, SurfaceBoundary::UStart),
        Err(BoundaryFrameError::EmptyControlNet),
    );

    let points = vec![
        vec![Vector4::new(0.0, 0.0, 0.0, 1.0); 3],
        vec![Vector4::new(0.0, 0.0, 0.0, 1.0); 3],
        vec![Vector4::new(0.0, 0.0, 0.0, 1.0); 3],
    ];
    let unclamped = NurbsSurface::new(BsplineSurface::new_unchecked(
        (
            KnotVector::from(vec![0.0, 1.0, 2.0, 3.0, 4.0]),
            KnotVector::from(vec![0.0, 0.0, 1.0, 2.0, 2.0]),
        ),
        points.clone(),
    ));
    assert_eq!(
        BoundaryFrame::try_new(&unclamped, SurfaceBoundary::UStart),
        Err(BoundaryFrameError::UnclampedKnotVector {
            axis: SurfaceAxis::U,
        }),
    );

    let nonrectangular = NurbsSurface::new(BsplineSurface::new_unchecked(
        (valid_knots.clone(), valid_knots),
        vec![
            vec![Vector4::new(0.0, 0.0, 0.0, 1.0); 2],
            vec![Vector4::new(0.0, 0.0, 0.0, 1.0)],
        ],
    ));
    assert_eq!(
        BoundaryFrame::try_new(&nonrectangular, SurfaceBoundary::UStart),
        Err(BoundaryFrameError::NonRectangularControlNet),
    );
}

#[test]
fn oversized_strips_return_a_typed_error() {
    assert_eq!(
        frame(SurfaceBoundary::VStart).control_strip_indices(4),
        Err(BoundaryFrameError::StripExceedsControlNet {
            requested: 4,
            available: 3,
        }),
    );
}
