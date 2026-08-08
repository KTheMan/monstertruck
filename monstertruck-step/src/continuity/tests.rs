use monstertruck_geometry::prelude::{BsplineCurve, KnotVector, ParameterCurve, Point2};

use super::*;
use crate::load::Table;
use crate::load::step_geometry::Line;

const FIXTURE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/resources/continuity-full-side.p21",
));

#[test]
fn exact_full_sides_accept_both_endpoint_directions() {
    let surface = fixture_surface();
    [
        (
            BoundarySide::MinU,
            Point2::new(0.0, 0.0),
            Point2::new(0.0, 1.0),
        ),
        (
            BoundarySide::MaxU,
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
        ),
        (
            BoundarySide::MinV,
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
        ),
        (
            BoundarySide::MaxV,
            Point2::new(0.0, 1.0),
            Point2::new(1.0, 1.0),
        ),
    ]
    .into_iter()
    .for_each(|(expected, first, second)| {
        assert_eq!(
            full_boundary_side(&line_trim(&surface, first, second), &surface),
            Some(expected),
        );
        assert_eq!(
            full_boundary_side(&line_trim(&surface, second, first), &surface),
            Some(expected),
        );
    });
}

#[test]
fn near_endpoint_and_curved_trims_are_not_full_sides() {
    let surface = fixture_surface();
    let near_endpoint = line_trim(
        &surface,
        Point2::new(0.0, 0.0),
        Point2::new(0.0, 1.0 - f64::EPSILON),
    );
    let curved = ParameterCurve::new(
        Box::new(Curve2D::BsplineCurve(BsplineCurve::new(
            KnotVector::bezier_knot(2),
            vec![
                Point2::new(0.0, 0.0),
                Point2::new(0.2, 0.5),
                Point2::new(0.0, 1.0),
            ],
        ))),
        Box::new(Surface::NurbsSurface(surface.clone())),
    );

    assert_eq!(full_boundary_side(&near_endpoint, &surface), None);
    assert_eq!(full_boundary_side(&curved, &surface), None);
}

fn fixture_surface() -> NurbsSurface<Vector4> {
    let table = Table::from_step(FIXTURE).expect("the continuity fixture parses");
    let holder = table
        .shell
        .values()
        .next()
        .expect("the continuity fixture contains a shell");
    let shell = table
        .to_compressed_trimmed_shell(holder)
        .expect("the continuity fixture imports");
    exact_nurbs(&shell.faces[0].surface, 0).expect("the fixture surface converts exactly")
}

fn line_trim(
    surface: &NurbsSurface<Vector4>,
    first: Point2,
    second: Point2,
) -> crate::load::step_geometry::StepParameterCurve {
    ParameterCurve::new(
        Box::new(Curve2D::Line(Line(first, second))),
        Box::new(Surface::NurbsSurface(surface.clone())),
    )
}
