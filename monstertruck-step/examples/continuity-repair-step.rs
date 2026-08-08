//! Runs the imported STEP continuity-repair evidence path headlessly.
//!
//! The fixture contains two adjacent NURBS faces. This example imports them,
//! identifies their shared full-side seam, applies a deterministic edit that
//! preserves `G0` while breaking `G1`, solves and applies `G1`, certifies the
//! repaired seam from public surface evaluations, replaces the dependent face,
//! produces a nonempty tessellation, exports it to STEP, re-imports it, and
//! repeats the independent certification. It also verifies that an arbitrary
//! trim receives the required typed refusal before solver work.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use clap::Parser;
use monstertruck_geometry::nurbs::continuity::{
    BoundaryAlignment, ContinuityOrder, UnsupportedContinuityCapability,
};
use monstertruck_geometry::nurbs::continuity_solver::{
    BoundaryContinuitySolver, ContinuitySolverConfig, continuity_work,
};
use monstertruck_geometry::prelude::{
    BoundedSurface, InnerSpace, NurbsSurface, ParameterCurve, ParametricSurface,
    TryIntoHomogeneousBsplineSurface, Vector4,
};
use monstertruck_meshing::prelude::MeshableShape;
use monstertruck_step::continuity::{
    StepContinuityError, StepContinuitySeam, repair_step_continuity,
};
use monstertruck_step::load::Table;
use monstertruck_step::load::convert::StepCompressedTrimmedShell;
use monstertruck_step::load::step_geometry::{Curve2D, Line, Point2, Surface};
use monstertruck_step::save::{CompleteStepDisplay, StepModel};

const FIXTURE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/resources/continuity-full-side.p21",
));
const FIRST_FACE: usize = 0;
const SECOND_FACE: usize = 1;
const SHARED_EDGE: usize = 1;
const POSITION_TOLERANCE: f64 = 1.0e-8;
const NORMAL_ALIGNMENT_TOLERANCE: f64 = 1.0e-8;

#[derive(Debug, Parser)]
struct Args {
    /// STEP path written for the independent OCCT validation job.
    #[arg(long, default_value = "target/continuity-repaired.step")]
    out: PathBuf,
}

#[derive(Clone, Copy, Debug)]
struct Certification {
    maximum_position_error: f64,
    minimum_normal_alignment: f64,
}

fn imported_shell(step: &str) -> Result<StepCompressedTrimmedShell> {
    let table = Table::from_step(step).context("failed to parse continuity STEP data")?;
    let holder = table
        .shell
        .values()
        .next()
        .context("continuity STEP data has no shell")?;
    table
        .to_compressed_trimmed_shell(holder)
        .context("failed to import continuity STEP shell")
}

fn make_shared_seam_partial(shell: &mut StepCompressedTrimmedShell) -> Result<()> {
    let surface = shell
        .faces
        .get(SECOND_FACE)
        .map(|face| face.surface.clone())
        .context("fixture second face is missing")?;
    let edge_use = shell
        .faces
        .get_mut(SECOND_FACE)
        .and_then(|face| {
            face.boundaries
                .iter_mut()
                .flatten()
                .find(|edge| edge.index == SHARED_EDGE)
        })
        .context("fixture shared edge-use is missing")?;
    edge_use.trim_curve = Some(ParameterCurve::new(
        Box::new(Curve2D::Line(Line(
            Point2::new(0.0, 0.25),
            Point2::new(0.0, 0.75),
        ))),
        Box::new(surface),
    ));
    Ok(())
}

fn exact_nurbs(surface: &Surface) -> Result<NurbsSurface<Vector4>> {
    surface
        .try_into_homogeneous_bspline_surface()
        .map(NurbsSurface::new)
        .context("continuity evidence requires an exact NURBS representation")
}

fn measure_g1(first: &Surface, second: &Surface) -> Result<Certification> {
    let first = exact_nurbs(first)?;
    let second = exact_nurbs(second)?;
    let ((first_min_u, first_max_u), (first_min_v, first_max_v)) = first.range_tuple();
    let ((second_min_u, second_max_u), (second_min_v, second_max_v)) = second.range_tuple();
    let step = 1.0e-5;

    let certification = (0..=10).try_fold(
        Certification {
            maximum_position_error: 0.0,
            minimum_normal_alignment: 1.0,
        },
        |certification, sample| -> Result<Certification> {
            let seam = f64::from(sample) / 10.0;
            let first_v = first_min_v + (first_max_v - first_min_v) * seam;
            let second_v = second_min_v + (second_max_v - second_min_v) * seam;
            let first_point = first.subs(first_max_u, first_v);
            let second_point = second.subs(second_min_u, second_v);
            let position_error = (first_point - second_point).magnitude();

            let first_cross =
                first_point - first.subs(first_max_u - (first_max_u - first_min_u) * step, first_v);
            let second_cross = second.subs(
                second_min_u + (second_max_u - second_min_u) * step,
                second_v,
            ) - second_point;
            let seam_before = (seam - step).max(0.0);
            let seam_after = (seam + step).min(1.0);
            let first_tangent = first.subs(
                first_max_u,
                first_min_v + (first_max_v - first_min_v) * seam_after,
            ) - first.subs(
                first_max_u,
                first_min_v + (first_max_v - first_min_v) * seam_before,
            );
            let second_tangent = second.subs(
                second_min_u,
                second_min_v + (second_max_v - second_min_v) * seam_after,
            ) - second.subs(
                second_min_u,
                second_min_v + (second_max_v - second_min_v) * seam_before,
            );
            let first_normal = first_cross.cross(first_tangent);
            let second_normal = second_cross.cross(second_tangent);
            ensure!(
                first_normal.magnitude2() > 0.0 && second_normal.magnitude2() > 0.0,
                "finite-difference certification encountered a singular tangent plane",
            );
            let normal_alignment = first_normal
                .normalize()
                .dot(second_normal.normalize())
                .abs();

            Ok(Certification {
                maximum_position_error: certification.maximum_position_error.max(position_error),
                minimum_normal_alignment: certification
                    .minimum_normal_alignment
                    .min(normal_alignment),
            })
        },
    )?;
    Ok(certification)
}

fn certify_g1(first: &Surface, second: &Surface) -> Result<Certification> {
    let certification = measure_g1(first, second)?;
    ensure!(
        certification.maximum_position_error <= POSITION_TOLERANCE,
        "independent G0 check measured error {} above {POSITION_TOLERANCE}",
        certification.maximum_position_error,
    );
    ensure!(
        1.0 - certification.minimum_normal_alignment <= NORMAL_ALIGNMENT_TOLERANCE,
        "independent G1 check measured normal misalignment {} above {NORMAL_ALIGNMENT_TOLERANCE}",
        1.0 - certification.minimum_normal_alignment,
    );
    Ok(certification)
}

fn apply_deterministic_edit(shell: &mut StepCompressedTrimmedShell) -> Result<()> {
    let mut surface = exact_nurbs(&shell.faces[SECOND_FACE].surface)?;
    surface.elevate_udegree();
    surface.elevate_vdegree();
    let seam_control_count = surface
        .control_points()
        .first()
        .map(Vec::len)
        .context("dependent surface has no control rows")?;
    surface
        .control_points_mut()
        .enumerate()
        .filter(|(index, _)| index / seam_control_count == 1 && index % seam_control_count == 1)
        .for_each(|(_, point)| point.z += 0.2 * point.w);
    let replacement = Surface::NurbsSurface(surface);
    let face = shell
        .faces
        .get_mut(SECOND_FACE)
        .context("fixture second face is missing")?;
    face.surface = replacement.clone();
    face.boundaries
        .iter_mut()
        .flatten()
        .filter_map(|edge| edge.trim_curve.as_mut())
        .for_each(|trim| {
            *trim = ParameterCurve::new(trim.curve().clone(), Box::new(replacement.clone()));
        });
    Ok(())
}

fn dependent_trim_state(shell: &StepCompressedTrimmedShell) -> (usize, usize, bool) {
    let face = &shell.faces[SECOND_FACE];
    let edge_uses = face.boundaries.iter().flatten().count();
    let trims = face
        .boundaries
        .iter()
        .flatten()
        .filter(|edge| edge.trim_curve.is_some())
        .count();
    let rebound = face.boundaries.iter().flatten().all(|edge| {
        edge.trim_curve
            .as_ref()
            .is_some_and(|trim| trim.surface().as_ref() == &face.surface)
    });
    (edge_uses, trims, rebound)
}

fn write_export(path: &Path, step: &str) -> Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(path, step).with_context(|| format!("failed to write {}", path.display()))
}

fn control_net_error(
    expected: &NurbsSurface<Vector4>,
    actual: &NurbsSurface<Vector4>,
) -> Result<f64> {
    ensure!(
        expected.control_points().len() == actual.control_points().len()
            && expected
                .control_points()
                .iter()
                .zip(actual.control_points())
                .all(|(expected, actual)| expected.len() == actual.len()),
        "STEP round-trip changed the repaired control-net dimensions",
    );
    Ok(expected
        .control_points()
        .iter()
        .flatten()
        .zip(actual.control_points().iter().flatten())
        .map(|(expected, actual)| (*expected - *actual).magnitude())
        .fold(0.0, f64::max))
}

fn main() -> Result<()> {
    let args = Args::parse();
    let seam = StepContinuitySeam::new(FIRST_FACE, SECOND_FACE, SHARED_EDGE)?;
    let solver = BoundaryContinuitySolver::new(ContinuitySolverConfig::default())?;

    let mut trimmed = imported_shell(FIXTURE)?;
    make_shared_seam_partial(&mut trimmed)?;
    let trimmed_before = trimmed.clone();
    let work_before = continuity_work();
    let refusal = repair_step_continuity(
        &mut trimmed,
        seam,
        BoundaryAlignment::Aligned,
        ContinuityOrder::G1,
        &solver,
    )
    .expect_err("an arbitrary trimmed seam must be refused");
    ensure!(
        matches!(
            refusal,
            StepContinuityError::UnsupportedCapability {
                face: SECOND_FACE,
                reason: UnsupportedContinuityCapability::TrimmedBoundary,
            }
        ),
        "partial seam returned the wrong typed refusal: {refusal}",
    );
    ensure!(
        trimmed == trimmed_before,
        "a refused repair mutated the shell"
    );
    ensure!(
        continuity_work() == work_before,
        "a trim refusal reached the numerical solver",
    );

    let mut shell = imported_shell(FIXTURE)?;
    let original_first = shell.faces[FIRST_FACE].surface.clone();
    let baseline_second = shell.faces[SECOND_FACE].surface.clone();
    let baseline_certification = certify_g1(&original_first, &baseline_second)?;
    let imported_trim_state = dependent_trim_state(&shell);
    ensure!(
        imported_trim_state.0 > 0
            && imported_trim_state.0 == imported_trim_state.1
            && imported_trim_state.2,
        "the imported dependent face does not have a complete bound trim set",
    );
    apply_deterministic_edit(&mut shell)?;
    let edited_second = shell.faces[SECOND_FACE].surface.clone();
    let edited_measurement = measure_g1(&original_first, &edited_second)?;
    ensure!(
        edited_measurement.maximum_position_error <= POSITION_TOLERANCE,
        "the deterministic edit moved the shared seam",
    );
    ensure!(
        1.0 - edited_measurement.minimum_normal_alignment > NORMAL_ALIGNMENT_TOLERANCE,
        "the deterministic edit did not create a measurable G1 defect",
    );
    let report = repair_step_continuity(
        &mut shell,
        seam,
        BoundaryAlignment::Aligned,
        ContinuityOrder::G1,
        &solver,
    )?;
    ensure!(
        shell.faces[FIRST_FACE].surface == original_first,
        "the fixed STEP face changed",
    );
    ensure!(
        exact_nurbs(&shell.faces[SECOND_FACE].surface)?.control_points()
            != exact_nurbs(&edited_second)?.control_points(),
        "the dependent STEP face control net did not change",
    );
    let repaired_trim_state = dependent_trim_state(&shell);
    ensure!(
        repaired_trim_state == imported_trim_state,
        "dependent trims were not rebound to the repaired face",
    );
    let repaired_certification = certify_g1(
        &shell.faces[FIRST_FACE].surface,
        &shell.faces[SECOND_FACE].surface,
    )?;

    let tessellated = shell.triangulation(0.01);
    let (tessellated_faces, tessellated_vertices, tessellated_triangles) = tessellated
        .faces
        .iter()
        .try_fold((0, 0, 0), |(faces, vertices, triangles), face| {
            let mesh = face
                .surface
                .as_ref()
                .context("a repaired face produced no tessellation")?;
            let face_vertices = mesh.positions().len();
            let face_triangles = mesh.faces().triangle_iter().count();
            ensure!(
                face_vertices > 0,
                "a repaired face tessellated with no vertices"
            );
            ensure!(
                face_triangles > 0,
                "a repaired face tessellated with no triangles"
            );
            Ok::<_, anyhow::Error>((
                faces + 1,
                vertices + face_vertices,
                triangles + face_triangles,
            ))
        })?;
    ensure!(
        tessellated_faces == shell.faces.len(),
        "only {tessellated_faces}/{} repaired faces tessellated",
        shell.faces.len(),
    );

    let exported =
        CompleteStepDisplay::new(StepModel::from(&shell), Default::default()).to_string();
    write_export(&args.out, &exported)?;
    let reimported =
        imported_shell(&exported).context("failed to re-import repaired STEP export")?;
    ensure!(
        reimported.faces.len() == shell.faces.len(),
        "STEP round-trip changed the repaired face count",
    );
    ensure!(
        reimported.faces[SECOND_FACE].surface != edited_second,
        "STEP round-trip restored the unrepaired dependent face",
    );
    let round_trip_control_error = control_net_error(
        &exact_nurbs(&shell.faces[SECOND_FACE].surface)?,
        &exact_nurbs(&reimported.faces[SECOND_FACE].surface)?,
    )?;
    ensure!(
        round_trip_control_error <= 1.0e-12,
        "STEP round-trip changed the repaired dependent control net by {round_trip_control_error}",
    );
    let reimported_trim_state = dependent_trim_state(&reimported);
    let reimported_missing_trims = reimported.faces[SECOND_FACE]
        .boundaries
        .iter()
        .flatten()
        .filter(|edge| edge.trim_curve.is_none())
        .map(|edge| edge.index)
        .collect::<Vec<_>>();
    ensure!(
        reimported_trim_state == imported_trim_state,
        "STEP round-trip did not preserve the repaired trim state: imported {imported_trim_state:?}, re-imported {reimported_trim_state:?}, missing {reimported_missing_trims:?}",
    );
    let reimported_certification = certify_g1(
        &reimported.faces[FIRST_FACE].surface,
        &reimported.faces[SECOND_FACE].surface,
    )?;

    println!("typed_trimmed_refusal=TrimmedBoundary");
    println!("termination={:?}", report.termination());
    println!(
        "baseline_minimum_normal_alignment={}",
        baseline_certification.minimum_normal_alignment,
    );
    println!(
        "edited_minimum_normal_alignment={}",
        edited_measurement.minimum_normal_alignment,
    );
    println!(
        "repaired_maximum_position_error={}",
        repaired_certification.maximum_position_error,
    );
    println!(
        "repaired_minimum_normal_alignment={}",
        repaired_certification.minimum_normal_alignment,
    );
    println!("tessellated_faces={tessellated_faces}");
    println!("tessellated_vertices={tessellated_vertices}");
    println!("tessellated_triangles={tessellated_triangles}");
    println!("reimported_faces={}", reimported.faces.len());
    println!("round_trip_control_error={round_trip_control_error}");
    println!(
        "reimported_maximum_position_error={}",
        reimported_certification.maximum_position_error,
    );
    println!(
        "reimported_minimum_normal_alignment={}",
        reimported_certification.minimum_normal_alignment,
    );
    println!("exported_step={}", args.out.display());
    Ok(())
}
