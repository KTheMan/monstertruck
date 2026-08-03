//! Headless imported STEP continuity validation.

mod certify;
mod classify;
mod errors;
mod mesh_validation;
mod persistence;
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
    /// Scale-normalized positional residual tolerance.
    #[arg(long, default_value_t = 1.0e-9)]
    position_tolerance: f64,
    /// Scale-normalized first-derivative residual tolerance.
    #[arg(long, default_value_t = 1.0e-7)]
    first_derivative_tolerance: f64,
    /// Scale-normalized second-derivative residual tolerance.
    #[arg(long, default_value_t = 1.0e-5)]
    second_derivative_tolerance: f64,
    /// Scale-normalized third-derivative residual tolerance.
    #[arg(long, default_value_t = 1.0e-3)]
    third_derivative_tolerance: f64,
    /// Maximum angle in radians between certified tangent planes.
    #[arg(long, default_value_t = 1.0e-7)]
    tangent_tolerance: f64,
    /// Number of intervals in the independent dense certification grid.
    #[arg(long, default_value_t = 512)]
    certification_intervals: usize,
    /// Normalized spacing for the independent finite-difference stencil.
    #[arg(long, default_value_t = 5.0e-3)]
    certification_step: f64,
    /// One-sided finite-difference stencil radius.
    #[arg(long, default_value_t = 4)]
    certification_stencil_radius: usize,
    /// Tessellation tolerance.
    #[arg(long, default_value_t = 1.0e-3)]
    mesh_tolerance: f64,
    /// Minimum scale-normalized doubled triangle area.
    #[arg(long, default_value_t = 1.0e-14)]
    triangle_area_tolerance: f64,
    /// Minimum cosine between triangle and surface normals.
    #[arg(long, default_value_t = 1.0e-6)]
    minimum_triangle_normal_alignment: f64,
    /// Scale-normalized bounding-box persistence tolerance.
    #[arg(long, default_value_t = 1.0e-9)]
    bounding_box_tolerance: f64,
    /// Optional JSON evidence receipt.
    #[arg(long)]
    receipt: Option<PathBuf>,
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
