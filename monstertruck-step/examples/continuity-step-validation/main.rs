//! Headless imported STEP continuity validation.

mod certify;
mod classify;
mod errors;
mod workflow;

use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;

#[derive(Clone, Debug, Parser)]
#[command(about = "Validate an imported full-boundary NURBS continuity repair.")]
struct Args {
    /// Input Phase 5 STEP fixture.
    #[arg(default_value = "validation/continuity/polynomial-g1.step")]
    input: PathBuf,
    /// One-based shell index.
    #[arg(long, default_value_t = 1)]
    shell: usize,
    /// Output STEP path.
    #[arg(long, default_value = "target/continuity-step-validation/solved.step")]
    output: PathBuf,
    /// Requested geometric-continuity order in `1..=3`.
    #[arg(long, default_value_t = 1)]
    order: u8,
    /// Deterministic dependent boundary-strip displacement.
    #[arg(long, default_value_t = 1.0e-3)]
    perturbation: f64,
    /// Absolute tolerance for imported seam classification.
    #[arg(long, default_value_t = 1.0e-7)]
    classification_tolerance: f64,
    /// Absolute tolerance for dense positional certification.
    #[arg(long, default_value_t = 1.0e-7)]
    position_tolerance: f64,
    /// Maximum sine of the angle between certified tangent planes.
    #[arg(long, default_value_t = 1.0e-6)]
    tangent_tolerance: f64,
    /// Number of intervals in the independent dense certification grid.
    #[arg(long, default_value_t = 512)]
    certification_intervals: usize,
    /// Tessellation tolerance.
    #[arg(long, default_value_t = 1.0e-3)]
    mesh_tolerance: f64,
}

fn main() -> Result<()> {
    let args = Args::parse();
    workflow::execute(&args).with_context(|| {
        format!(
            "Phase 5 STEP continuity validation failed for {}",
            args.input.display()
        )
    })
}
