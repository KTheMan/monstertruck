use anyhow::Result;
use monstertruck_core::{ContentHasher, DeterministicContentHash};
use monstertruck_geometry::base::Vector4;
use monstertruck_geometry::nurbs::NurbsSurface;
use monstertruck_geometry::nurbs::continuity_solver::BoundaryContinuitySolution;
use std::hash::Hasher;

use crate::corpus::{CaseSpec, DenseSpec};
use crate::dense::DenseMetrics;

pub const DIGEST_VERSION: &str = "xxh3-64-public-transition-v6";

/// Hashes a successful solved case in canonical index order.
pub fn solved(
    fixture_version: &str,
    case: &CaseSpec,
    dense_spec: DenseSpec,
    solution: &BoundaryContinuitySolution<'_>,
    dense: &DenseMetrics,
) -> Result<String> {
    let mut hasher = ContentHasher::new();
    hash_context(fixture_version, case, dense_spec, &mut hasher)?;
    "converged".content_hash(&mut hasher);
    hash_surface(solution.first(), &mut hasher);
    hash_surface(solution.second(), &mut hasher);
    hash_transition(solution, &mut hasher);
    hash_report(solution, &mut hasher);
    dense
        .maximum_absolute_residual_by_order
        .content_hash(&mut hasher);
    dense
        .maximum_normalized_residual_by_order
        .content_hash(&mut hasher);
    dense.maximum_normal_angle.content_hash(&mut hasher);
    dense.worst_seam_by_order.content_hash(&mut hasher);
    dense
        .worst_mixed_derivative_by_order
        .content_hash(&mut hasher);
    Ok(format!("{:016x}", hasher.finish()))
}

/// Hashes a structured negative outcome.
pub fn error(
    fixture_version: &str,
    case: &CaseSpec,
    dense_spec: DenseSpec,
    error: &serde_json::Value,
) -> Result<String> {
    let mut hasher = ContentHasher::new();
    hash_context(fixture_version, case, dense_spec, &mut hasher)?;
    hasher.write(&serde_json::to_vec(error)?);
    Ok(format!("{:016x}", hasher.finish()))
}

fn hash_context(
    fixture_version: &str,
    case: &CaseSpec,
    dense_spec: DenseSpec,
    hasher: &mut ContentHasher,
) -> Result<()> {
    hasher.write(&serde_json::to_vec(&(fixture_version, case, dense_spec))?);
    Ok(())
}

fn hash_transition(solution: &BoundaryContinuitySolution<'_>, hasher: &mut ContentHasher) {
    let transition = solution.transition();
    format!("{:?}", transition.alignment()).content_hash(hasher);
    transition.order().as_usize().content_hash(hasher);
    transition.cross_field_degree().content_hash(hasher);
    transition.seam_map_degree().content_hash(hasher);
    let samples = (0..=65)
        .flat_map(|index| {
            [-0.04, -0.03, -0.02, -0.01, 0.0, 0.01, 0.02, 0.03, 0.04]
                .map(move |cross| transition.mapped_coordinates(index as f64 / 65.0, cross))
        })
        .collect::<Vec<_>>();
    samples.content_hash(hasher);
}

fn hash_report(solution: &BoundaryContinuitySolution<'_>, hasher: &mut ContentHasher) {
    let report = solution.report();
    format!("{:?}", report.termination()).content_hash(hasher);
    report.iterations().content_hash(hasher);
    report.accepted_steps().content_hash(hasher);
    report.rejected_steps().content_hash(hasher);
    report.initial_objective().content_hash(hasher);
    report.final_objective().content_hash(hasher);
    report.residuals().iter().for_each(|residual| {
        residual.order().as_usize().content_hash(hasher);
        residual.rms().content_hash(hasher);
        residual.maximum().content_hash(hasher);
        residual.worst_sample().content_hash(hasher);
        residual.is_validation_sample().content_hash(hasher);
        residual.cross_derivative().content_hash(hasher);
        residual.seam_derivative().content_hash(hasher);
    });
    report.gradient_infinity_norm().content_hash(hasher);
    report.step_norm().content_hash(hasher);
    report.damping().content_hash(hasher);
    report.numerical_rank().content_hash(hasher);
    report.variable_count().content_hash(hasher);
    report.residual_count().content_hash(hasher);
}

fn hash_surface(surface: &NurbsSurface<Vector4>, hasher: &mut ContentHasher) {
    surface.knot_vector_u().as_ref().content_hash(hasher);
    surface.knot_vector_v().as_ref().content_hash(hasher);
    surface.control_points().len().content_hash(hasher);
    surface.control_points().iter().for_each(|row| {
        row.as_slice().content_hash(hasher);
    });
}
