use anyhow::Result;
use monstertruck_core::{ContentHasher, DeterministicContentHash};
use monstertruck_geometry::base::Vector4;
use monstertruck_geometry::nurbs::NurbsSurface;
use monstertruck_geometry::nurbs::continuity_solver::BoundaryContinuitySolution;
use std::hash::Hasher;

use crate::corpus::{CaseSpec, DenseSpec};
use crate::dense::DenseMetrics;

pub const DIGEST_VERSION: &str = "xxh3-64-public-transition-v3";

/// Hashes a successful solved case in canonical index order.
pub fn solved(
    fixture_version: &str,
    case: &CaseSpec,
    dense_spec: DenseSpec,
    solution: &BoundaryContinuitySolution,
    dense: &DenseMetrics,
) -> Result<String> {
    let mut hasher = ContentHasher::new();
    hash_context(fixture_version, case, dense_spec, &mut hasher)?;
    "converged".content_hash(&mut hasher);
    hash_surface(solution.first(), &mut hasher);
    hash_surface(solution.second(), &mut hasher);
    hash_transition(solution, &mut hasher)?;
    hasher.write(&serde_json::to_vec(solution.report())?);
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

fn hash_transition(
    solution: &BoundaryContinuitySolution,
    hasher: &mut ContentHasher,
) -> Result<()> {
    let transition = solution.transition();
    hasher.write(&serde_json::to_vec(&(
        transition.alignment(),
        transition.order(),
        transition.field_degree(),
    ))?);
    let samples = (0..=65)
        .flat_map(|index| {
            [-0.4, -0.3, -0.2, -0.1, 0.0, 0.1, 0.2, 0.3, 0.4]
                .map(move |cross| transition.mapped_coordinates(index as f64 / 65.0, cross))
        })
        .collect::<Vec<_>>();
    hasher.write(&serde_json::to_vec(&samples)?);
    Ok(())
}

fn hash_surface(surface: &NurbsSurface<Vector4>, hasher: &mut ContentHasher) {
    surface.knot_vector_u().as_ref().content_hash(hasher);
    surface.knot_vector_v().as_ref().content_hash(hasher);
    surface.control_points().len().content_hash(hasher);
    surface.control_points().iter().for_each(|row| {
        row.as_slice().content_hash(hasher);
    });
}
