//! Runs the imported STEP continuity-repair evidence path headlessly.
//!
//! The fixture contains two adjacent NURBS faces. This example imports them,
//! selects their shared full-side seam, solves and applies `G1`, tessellates the
//! replacement shell, exports it to STEP, and re-imports the result. It also
//! turns the imported seam into a partial trim and verifies the adapter returns
//! the required typed refusal without invoking a best-effort solve.

use anyhow::{Context, Result, ensure};
use monstertruck_geometry::nurbs::continuity::{
    BoundaryAlignment, ContinuityOrder, UnsupportedContinuityCapability,
};
use monstertruck_geometry::nurbs::continuity_solver::{
    BoundaryContinuitySolver, ContinuitySolverConfig,
};
use monstertruck_meshing::prelude::MeshableShape;
use monstertruck_step::continuity::{
    StepContinuityError, StepContinuitySeam, repair_step_continuity,
};
use monstertruck_step::load::Table;
use monstertruck_step::load::step_geometry::{Curve2D, Line, ParameterCurve, Point2, Surface};
use monstertruck_step::save::{CompleteStepDisplay, StepModel};

const FIXTURE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/resources/continuity-full-side.p21",
));
const FIRST_FACE: usize = 0;
const SECOND_FACE: usize = 1;
const SHARED_EDGE: usize = 1;

fn imported_shell() -> Result<monstertruck_step::load::convert::StepCompressedTrimmedShell> {
    let table = Table::from_step(FIXTURE).context("failed to parse continuity STEP fixture")?;
    let holder = table
        .shell
        .values()
        .next()
        .context("continuity STEP fixture has no shell")?;
    table
        .to_compressed_trimmed_shell(holder)
        .context("failed to import continuity STEP shell")
}

fn make_shared_seam_partial(
    shell: &mut monstertruck_step::load::convert::StepCompressedTrimmedShell,
) -> Result<()> {
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

fn main() -> Result<()> {
    let seam = StepContinuitySeam::new(FIRST_FACE, SECOND_FACE, SHARED_EDGE);
    let solver = BoundaryContinuitySolver::new(ContinuitySolverConfig::default())?;

    let mut trimmed = imported_shell()?;
    make_shared_seam_partial(&mut trimmed)?;
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

    let mut shell = imported_shell()?;
    let report = repair_step_continuity(
        &mut shell,
        seam,
        BoundaryAlignment::Aligned,
        ContinuityOrder::G1,
        &solver,
    )?;
    ensure!(
        matches!(shell.faces[SECOND_FACE].surface, Surface::NurbsSurface(_)),
        "the dependent STEP face was not replaced with the solved surface",
    );

    let tessellated = shell.triangulation(0.01);
    let tessellated_faces = tessellated
        .faces
        .iter()
        .filter(|face| face.surface.is_some())
        .count();
    ensure!(
        tessellated_faces == shell.faces.len(),
        "only {tessellated_faces}/{} repaired faces tessellated",
        shell.faces.len(),
    );

    let exported =
        CompleteStepDisplay::new(StepModel::from(&shell), Default::default()).to_string();
    let reimported_table =
        Table::from_step(&exported).context("failed to parse repaired STEP export")?;
    let reimported_holder = reimported_table
        .shell
        .values()
        .next()
        .context("repaired STEP export has no shell")?;
    let reimported = reimported_table
        .to_compressed_trimmed_shell(reimported_holder)
        .context("failed to re-import repaired STEP shell")?;
    ensure!(
        reimported.faces.len() == shell.faces.len(),
        "STEP round-trip changed the repaired face count",
    );

    println!("typed_trimmed_refusal=TrimmedBoundary");
    println!("termination={:?}", report.termination());
    println!("tessellated_faces={tessellated_faces}");
    println!("reimported_faces={}", reimported.faces.len());
    Ok(())
}
