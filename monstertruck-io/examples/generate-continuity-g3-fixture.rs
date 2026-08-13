//! Generates the deterministic degree-five STEP continuity fixture.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use clap::Parser;
use monstertruck_geometry::prelude::{
    NurbsSurface, ParameterCurve, TryIntoHomogeneousBsplineSurface, Vector4,
};
use monstertruck_io::step::load::Table;
use monstertruck_io::step::load::convert::StepCompressedTrimmedShell;
use monstertruck_io::step::load::step_geometry::Surface;
use monstertruck_io::step::save::{CompleteStepDisplay, StepModel};

const SOURCE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/continuity/continuity-full-side.step",
));
const FIXED_FILE_NAME: &str =
    "FILE_NAME('continuity-g3.step', '2026-08-13 00:00:00', (''), (''), 'monstertruck', '', '');";

#[derive(Debug, Parser)]
struct Args {
    /// Fails when the checked-in fixture differs from generated output.
    #[arg(long)]
    check: bool,
    /// Output path. Defaults to the checked-in fixture path.
    #[arg(long)]
    out: Option<PathBuf>,
}

fn imported_shell() -> Result<StepCompressedTrimmedShell> {
    let table = Table::from_step(SOURCE).context("failed to parse the seed STEP fixture")?;
    let holder = table
        .shell
        .values()
        .next()
        .context("the seed STEP fixture has no shell")?;
    table
        .to_compressed_trimmed_shell(holder)
        .context("failed to import the seed STEP shell")
}

fn exact_nurbs(surface: &Surface) -> Result<NurbsSurface<Vector4>> {
    surface
        .try_into_homogeneous_bspline_surface()
        .map(NurbsSurface::new)
        .context("the seed fixture requires exact tensor-product surfaces")
}

fn elevate_faces(shell: &mut StepCompressedTrimmedShell) -> Result<()> {
    shell.faces.iter_mut().try_for_each(|face| -> Result<()> {
        let mut surface = exact_nurbs(&face.surface)?;
        (surface.udegree()..5).for_each(|_| {
            surface.elevate_udegree();
        });
        (surface.vdegree()..5).for_each(|_| {
            surface.elevate_vdegree();
        });
        ensure!(surface.degrees() == (5, 5));
        let replacement = Surface::NurbsSurface(surface);
        face.surface = replacement.clone();
        face.boundaries
            .iter_mut()
            .flatten()
            .filter_map(|edge| edge.trim_curve.as_mut())
            .for_each(|trim| {
                *trim = ParameterCurve::new(trim.curve().clone(), Box::new(replacement.clone()));
            });
        Ok(())
    })
}

fn normalized_step(shell: &StepCompressedTrimmedShell) -> String {
    CompleteStepDisplay::new(StepModel::from(shell), Default::default())
        .to_string()
        .lines()
        .map(|line| {
            if line.starts_with("FILE_NAME(") {
                FIXED_FILE_NAME
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn checked_in_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("resources")
        .join("continuity")
        .join("continuity-g3.step")
}

fn main() -> Result<()> {
    let args = Args::parse();
    let path = args.out.unwrap_or_else(checked_in_path);
    let mut shell = imported_shell()?;
    elevate_faces(&mut shell)?;
    let generated = normalized_step(&shell);

    if args.check {
        let checked_in = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        ensure!(
            checked_in == generated,
            "{} is not the deterministic generator output",
            path.display(),
        );
        println!("verified {}", path.display());
        Ok(())
    } else {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        fs::write(&path, generated)
            .with_context(|| format!("failed to write {}", path.display()))?;
        println!("wrote {}", path.display());
        Ok(())
    }
}
