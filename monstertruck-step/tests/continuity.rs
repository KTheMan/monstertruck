use monstertruck_geometry::nurbs::continuity::{
    BoundaryAlignment, ContinuityOrder, UnsupportedContinuityCapability,
};
use monstertruck_geometry::nurbs::continuity_solver::{
    BoundaryContinuitySolver, ContinuitySolverConfig, continuity_work,
};
use monstertruck_geometry::prelude::{BsplineCurve, KnotVector, ParameterCurve, Plane, Point2};
use monstertruck_step::continuity::{
    StepContinuityError, StepContinuitySeam, repair_step_continuity,
};
use monstertruck_step::load::Table;
use monstertruck_step::load::convert::StepCompressedTrimmedShell;
use monstertruck_step::load::step_geometry::{Curve2D, ElementarySurface, Line, Surface};

const FIXTURE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/resources/continuity-full-side.p21",
));
const FIRST_FACE: usize = 0;
const SECOND_FACE: usize = 1;
const SHARED_EDGE: usize = 1;

#[test]
fn seam_requires_two_distinct_faces() {
    assert_eq!(
        StepContinuitySeam::new(FIRST_FACE, FIRST_FACE, SHARED_EDGE),
        Err(StepContinuityError::SameFace { face: FIRST_FACE }),
    );
}

#[test]
fn imported_arbitrary_trims_are_typed_transactional_refusals() {
    let imported = imported_shell();

    assert_trimmed_refusal(imported.clone(), FIRST_FACE, None);
    assert_trimmed_refusal(
        imported.clone(),
        SECOND_FACE,
        Some(Curve2D::Line(Line(
            Point2::new(0.0, 0.25),
            Point2::new(0.0, 0.75),
        ))),
    );
    assert_trimmed_refusal(
        imported.clone(),
        FIRST_FACE,
        Some(Curve2D::Line(Line(
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0 - f64::EPSILON),
        ))),
    );
    assert_trimmed_refusal(
        imported,
        SECOND_FACE,
        Some(Curve2D::BsplineCurve(BsplineCurve::new(
            KnotVector::bezier_knot(2),
            vec![
                Point2::new(0.0, 0.0),
                Point2::new(0.2, 0.5),
                Point2::new(0.0, 1.0),
            ],
        ))),
    );
}

#[test]
fn imported_unsupported_surface_keeps_representation_reason() {
    let mut shell = imported_shell();
    shell.faces[SECOND_FACE].surface =
        Surface::ElementarySurface(ElementarySurface::Plane(Plane::xy()));
    let before = shell.clone();
    let work_before = continuity_work();

    let error = repair_step_continuity(
        &mut shell,
        seam(),
        BoundaryAlignment::Aligned,
        ContinuityOrder::G1,
        &solver(),
    )
    .expect_err("an elementary surface must not be classified as a trim failure");

    assert_eq!(
        error,
        StepContinuityError::UnsupportedCapability {
            face: SECOND_FACE,
            reason: UnsupportedContinuityCapability::UnsupportedRepresentation,
        },
    );
    assert_eq!(shell, before, "a refused repair must be transactional");
    assert_eq!(
        continuity_work(),
        work_before,
        "adapter refusal must happen before solver work",
    );
}

#[test]
fn imported_seam_requires_a_real_edge_shared_by_both_faces() {
    let shell = imported_shell();
    let missing_edge = shell.edges.len();
    let not_shared_edge = shell.faces[FIRST_FACE]
        .boundaries
        .iter()
        .flatten()
        .map(|edge| edge.index)
        .find(|edge| {
            !shell.faces[SECOND_FACE]
                .boundaries
                .iter()
                .flatten()
                .any(|second| second.index == *edge)
        })
        .expect("the first fixture face has an unshared edge");

    assert_topology_refusal(
        shell.clone(),
        StepContinuitySeam::new(FIRST_FACE, SECOND_FACE, missing_edge)
            .expect("the selected faces are distinct"),
        StepContinuityError::EdgeOutOfRange { edge: missing_edge },
    );
    assert_topology_refusal(
        shell,
        StepContinuitySeam::new(FIRST_FACE, SECOND_FACE, not_shared_edge)
            .expect("the selected faces are distinct"),
        StepContinuityError::EdgeNotUsedByFace {
            face: SECOND_FACE,
            edge: not_shared_edge,
        },
    );
}

fn assert_trimmed_refusal(
    mut shell: StepCompressedTrimmedShell,
    face_index: usize,
    replacement: Option<Curve2D>,
) {
    let surface = shell.faces[face_index].surface.clone();
    let edge_use = shell.faces[face_index]
        .boundaries
        .iter_mut()
        .flatten()
        .find(|edge| edge.index == SHARED_EDGE)
        .expect("the imported fixture contains the shared edge-use");
    edge_use.trim_curve =
        replacement.map(|curve| ParameterCurve::new(Box::new(curve), Box::new(surface)));
    let before = shell.clone();
    let work_before = continuity_work();

    let error = repair_step_continuity(
        &mut shell,
        seam(),
        BoundaryAlignment::Aligned,
        ContinuityOrder::G1,
        &solver(),
    )
    .expect_err("an arbitrary trim must be refused");

    assert_eq!(
        error,
        StepContinuityError::UnsupportedCapability {
            face: face_index,
            reason: UnsupportedContinuityCapability::TrimmedBoundary,
        },
    );
    assert_eq!(shell, before, "a refused repair must be transactional");
    assert_eq!(
        continuity_work(),
        work_before,
        "adapter refusal must happen before solver work",
    );
}

fn assert_topology_refusal(
    mut shell: StepCompressedTrimmedShell,
    seam: StepContinuitySeam,
    expected: StepContinuityError,
) {
    let before = shell.clone();
    let work_before = continuity_work();
    let error = repair_step_continuity(
        &mut shell,
        seam,
        BoundaryAlignment::Aligned,
        ContinuityOrder::G1,
        &solver(),
    )
    .expect_err("a non-adjacent selection must be refused");

    assert_eq!(error, expected);
    assert_eq!(shell, before, "a refused repair must be transactional");
    assert_eq!(
        continuity_work(),
        work_before,
        "topology refusal must happen before solver work",
    );
}

fn imported_shell() -> StepCompressedTrimmedShell {
    let table = Table::from_step(FIXTURE).expect("the continuity fixture parses");
    let holder = table
        .shell
        .values()
        .next()
        .expect("the continuity fixture contains a shell");
    table
        .to_compressed_trimmed_shell(holder)
        .expect("the continuity fixture imports")
}

fn seam() -> StepContinuitySeam {
    StepContinuitySeam::new(FIRST_FACE, SECOND_FACE, SHARED_EDGE)
        .expect("the fixture selects two distinct faces")
}

fn solver() -> BoundaryContinuitySolver {
    BoundaryContinuitySolver::new(ContinuitySolverConfig::default())
        .expect("the default solver configuration is valid")
}
