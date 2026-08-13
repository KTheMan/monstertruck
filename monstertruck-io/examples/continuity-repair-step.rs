//! Runs the imported STEP continuity-repair evidence path headlessly.
//!
//! The fixture contains two adjacent NURBS faces. This example imports them,
//! identifies their shared full-side seam, perturbs dependent control rows one
//! through three while preserving the boundary row, solves and applies `G3`,
//! certifies the common boundary jet independently at 33 samples, replaces the
//! dependent face, produces a nonempty tessellation, exports it to STEP,
//! re-imports it, and repeats the certification. It also verifies that an
//! arbitrary trim receives the required typed refusal before solver work.

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
use monstertruck_io::step::continuity::{
    StepContinuityError, StepContinuitySeam, repair_step_continuity,
};
use monstertruck_io::step::load::Table;
use monstertruck_io::step::load::convert::StepCompressedTrimmedShell;
use monstertruck_io::step::load::step_geometry::{Curve2D, Line, Point2, Surface};
use monstertruck_io::step::save::{CompleteStepDisplay, StepModel};
use monstertruck_meshing::prelude::MeshableShape;

const FIXTURE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/resources/continuity/continuity-g3.step",
));
const FIRST_FACE: usize = 0;
const SECOND_FACE: usize = 1;
const SHARED_EDGE: usize = 1;
const SAMPLE_COUNT: usize = 33;
const FIXTURE_SCALE: f64 = 1.0;
const NORMALIZED_LIMITS: [f64; 4] = [1.0e-9, 1.0e-7, 1.0e-5, 1.0e-3];

#[derive(Debug, Parser)]
struct Args {
    /// STEP path written for the independent OCCT validation job.
    #[arg(long, default_value = "target/continuity-repaired.step")]
    out: PathBuf,
}

#[derive(Clone, Copy, Debug)]
struct Certification {
    maximum_normalized_residual_by_order: [f64; 4],
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

fn measure_g3(first: &Surface, second: &Surface) -> Result<Certification> {
    let first = exact_nurbs(first)?;
    let second = exact_nurbs(second)?;
    let ((first_min_u, first_max_u), (first_min_v, first_max_v)) = first.range_tuple();
    let ((second_min_u, second_max_u), (second_min_v, second_max_v)) = second.range_tuple();
    let first_u_extent = first_max_u - first_min_u;
    let first_v_extent = first_max_v - first_min_v;
    let second_u_extent = second_max_u - second_min_u;
    let second_v_extent = second_max_v - second_min_v;
    ensure!(
        [
            first_u_extent,
            first_v_extent,
            second_u_extent,
            second_v_extent,
        ]
        .into_iter()
        .all(|extent| extent.is_finite() && extent > 0.0),
        "the imported continuity fixture has a degenerate parameter domain",
    );

    (0..SAMPLE_COUNT).try_fold(
        Certification {
            maximum_normalized_residual_by_order: [0.0; 4],
        },
        |mut certification, sample| -> Result<Certification> {
            let seam = sample as f64 / (SAMPLE_COUNT - 1) as f64;
            let first_v = first_min_v + first_v_extent * seam;
            let second_v = second_min_v + second_v_extent * seam;
            (0..=3).try_for_each(|total| -> Result<()> {
                (0..=total).try_for_each(|cross_order| -> Result<()> {
                    let seam_order = total - cross_order;
                    let first_derivative =
                        first.derivative_mn(cross_order, seam_order, first_max_u, first_v)
                            * first_u_extent.powi(cross_order as i32)
                            * first_v_extent.powi(seam_order as i32);
                    let second_derivative =
                        second.derivative_mn(cross_order, seam_order, second_min_u, second_v)
                            * second_u_extent.powi(cross_order as i32)
                            * second_v_extent.powi(seam_order as i32);
                    let residual =
                        (first_derivative - second_derivative).magnitude() / FIXTURE_SCALE;
                    ensure!(
                        residual.is_finite(),
                        "dense G3 certification produced a non-finite residual",
                    );
                    certification.maximum_normalized_residual_by_order[total] =
                        certification.maximum_normalized_residual_by_order[total].max(residual);
                    Ok(())
                })
            })?;
            Ok(certification)
        },
    )
}

fn certify_g3(first: &Surface, second: &Surface) -> Result<Certification> {
    let certification = measure_g3(first, second)?;
    certification
        .maximum_normalized_residual_by_order
        .iter()
        .zip(NORMALIZED_LIMITS)
        .enumerate()
        .try_for_each(|(order, (&actual, limit))| -> Result<()> {
            ensure!(
                actual <= limit,
                "independent G{order} residual {actual} exceeds {limit}",
            );
            Ok(())
        })?;
    Ok(certification)
}

fn apply_deterministic_edit(shell: &mut StepCompressedTrimmedShell) -> Result<()> {
    let mut surface = exact_nurbs(&shell.faces[SECOND_FACE].surface)?;
    ensure!(
        surface.degrees() == (5, 5),
        "the imported continuity fixture is not degree five",
    );
    let seam_control_count = surface
        .control_points()
        .first()
        .map(Vec::len)
        .context("dependent surface has no control rows")?;
    surface
        .control_points_mut()
        .enumerate()
        .filter_map(|(index, point)| {
            let row = index / seam_control_count;
            (1..=3).contains(&row).then_some((row, point))
        })
        .for_each(|(row, point)| {
            const ROW_PATTERN: [f64; 4] = [0.0, 1.0, -0.5, 0.25];
            point.z += 1.0e-3 * ROW_PATTERN[row] * point.w;
        });
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
    let solver = BoundaryContinuitySolver::new(
        ContinuitySolverConfig::default()
            .with_max_iterations(80)
            .with_anchor_weight(0.0)
            .with_fairness_weight(0.0)
            .with_transition_weight(1.0),
    )?;

    let mut trimmed = imported_shell(FIXTURE)?;
    make_shared_seam_partial(&mut trimmed)?;
    let trimmed_before = trimmed.clone();
    let work_before = continuity_work();
    let refusal = repair_step_continuity(
        &mut trimmed,
        seam,
        BoundaryAlignment::Aligned,
        ContinuityOrder::G3,
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
    ensure!(
        exact_nurbs(&original_first)?.degrees() == (5, 5)
            && exact_nurbs(&baseline_second)?.degrees() == (5, 5),
        "the directly imported continuity fixture is not degree five",
    );
    let baseline_certification = certify_g3(&original_first, &baseline_second)
        .context("the directly imported fixture failed baseline G3 certification")?;
    let imported_trim_state = dependent_trim_state(&shell);
    ensure!(
        imported_trim_state.0 > 0
            && imported_trim_state.0 == imported_trim_state.1
            && imported_trim_state.2,
        "the imported dependent face does not have a complete bound trim set",
    );
    apply_deterministic_edit(&mut shell)?;
    let edited_second = shell.faces[SECOND_FACE].surface.clone();
    let edited_measurement = measure_g3(&original_first, &edited_second)?;
    ensure!(
        edited_measurement.maximum_normalized_residual_by_order[0] <= NORMALIZED_LIMITS[0],
        "the deterministic edit moved the shared seam",
    );
    (1..=3).try_for_each(|order| -> Result<()> {
        ensure!(
            edited_measurement.maximum_normalized_residual_by_order[order]
                > NORMALIZED_LIMITS[order],
            "the deterministic edit did not create a measurable G{order} defect",
        );
        Ok(())
    })?;
    let report = repair_step_continuity(
        &mut shell,
        seam,
        BoundaryAlignment::Aligned,
        ContinuityOrder::G3,
        &solver,
    )?;
    ensure!(
        report.accepted_steps() > 0,
        "the G3 repair reported no accepted edit",
    );
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
    let repaired_certification = certify_g3(
        &shell.faces[FIRST_FACE].surface,
        &shell.faces[SECOND_FACE].surface,
    )
    .context("the repaired imported seam failed G3 certification")?;

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
    let reimported_certification = certify_g3(
        &reimported.faces[FIRST_FACE].surface,
        &reimported.faces[SECOND_FACE].surface,
    )
    .context("the re-imported repaired seam failed G3 certification")?;

    println!("typed_trimmed_refusal=TrimmedBoundary");
    println!("termination={:?}", report.termination());
    println!(
        "baseline_maximum_normalized_residual_by_order={:?}",
        baseline_certification.maximum_normalized_residual_by_order,
    );
    println!(
        "edited_maximum_normalized_residual_by_order={:?}",
        edited_measurement.maximum_normalized_residual_by_order,
    );
    println!(
        "repaired_maximum_normalized_residual_by_order={:?}",
        repaired_certification.maximum_normalized_residual_by_order,
    );
    println!("tessellated_faces={tessellated_faces}");
    println!("tessellated_vertices={tessellated_vertices}");
    println!("tessellated_triangles={tessellated_triangles}");
    println!("reimported_faces={}", reimported.faces.len());
    println!("round_trip_control_error={round_trip_control_error}");
    println!(
        "reimported_maximum_normalized_residual_by_order={:?}",
        reimported_certification.maximum_normalized_residual_by_order,
    );
    println!("exported_step={}", args.out.display());
    Ok(())
}
