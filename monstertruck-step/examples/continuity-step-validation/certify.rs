//! Independent dense common-coordinate certification through `G3`.

mod finite_difference;
mod sampling;

use super::classify::SeamSelection;
use super::errors::ValidationError;
use finite_difference::{
    ensure_finite_vector, ensure_nonzero_finite_vector, finite_difference_weights, mixed_derivative,
};
use monstertruck_geometry::nurbs::NurbsSurface;
use monstertruck_geometry::nurbs::continuity::ContinuityOrder;
use monstertruck_geometry::nurbs::continuity_solver::BoundaryTransition;
use monstertruck_geometry::prelude::{InnerSpace, Point3, Vector4};
use sampling::{certification_seams, sample_grids, seam_stencil, surface_pair_scale};
use serde::Serialize;

type SampleGrid = Vec<Vec<Point3>>;

const MAX_CERTIFICATION_INTERVALS: usize = 65_536;
const MAX_STENCIL_RADIUS: usize = 16;

struct CertificationStencils {
    centered_seam_nodes: Vec<f64>,
    first_cross_nodes: Vec<f64>,
    second_cross_nodes: Vec<f64>,
    first_cross_weights: Vec<Vec<f64>>,
    second_cross_weights: Vec<Vec<f64>>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(super) struct Certificate {
    pub(super) samples: usize,
    pub(super) scale: f64,
    pub(super) normalized_step: f64,
    pub(super) stencil_radius: usize,
    pub(super) maximum_absolute_residual_by_order: Vec<f64>,
    pub(super) maximum_normalized_residual_by_order: Vec<f64>,
    pub(super) maximum_normalized_residual_tolerance_by_order: Vec<f64>,
    pub(super) maximum_normal_angle: f64,
    pub(super) maximum_normal_angle_tolerance: f64,
    pub(super) worst_seam_by_order: Vec<f64>,
    pub(super) worst_mixed_derivative_by_order: Vec<[usize; 2]>,
}

pub(super) struct CertificationConfig<'a> {
    pub(super) intervals: usize,
    pub(super) normalized_step: f64,
    pub(super) stencil_radius: usize,
    pub(super) maximum_residual_by_order: &'a [f64],
    pub(super) maximum_normal_angle: f64,
}

pub(super) fn certify(
    first: &NurbsSurface<Vector4>,
    second: &NurbsSurface<Vector4>,
    transition: &BoundaryTransition,
    seam: SeamSelection,
    order: ContinuityOrder,
    config: CertificationConfig<'_>,
) -> Result<Certificate, ValidationError> {
    validate_config(order, &config)?;
    let derivative_order = order.as_usize();
    let radius = config.stencil_radius;
    let centered_seam_nodes = (-(radius as isize)..=radius as isize)
        .map(|index| index as f64 * config.normalized_step)
        .collect::<Vec<_>>();
    let first_cross_nodes = (0..=2 * radius)
        .map(|index| -((2 * radius - index) as f64) * config.normalized_step)
        .collect::<Vec<_>>();
    let second_cross_nodes = (0..=2 * radius)
        .map(|index| index as f64 * config.normalized_step)
        .collect::<Vec<_>>();
    let first_cross_weights = (0..=derivative_order)
        .map(|derivative| finite_difference_weights(&first_cross_nodes, derivative))
        .collect::<Result<Vec<_>, _>>()?;
    let second_cross_weights = (0..=derivative_order)
        .map(|derivative| finite_difference_weights(&second_cross_nodes, derivative))
        .collect::<Result<Vec<_>, _>>()?;
    let stencils = CertificationStencils {
        centered_seam_nodes,
        first_cross_nodes,
        second_cross_nodes,
        first_cross_weights,
        second_cross_weights,
    };
    let seams = certification_seams(first, second, transition, seam, config.intervals)?;
    let scale = surface_pair_scale(first, second)?;
    let mut certificate = Certificate {
        samples: seams.len(),
        scale,
        normalized_step: config.normalized_step,
        stencil_radius: radius,
        maximum_absolute_residual_by_order: vec![0.0; derivative_order + 1],
        maximum_normalized_residual_by_order: vec![0.0; derivative_order + 1],
        maximum_normalized_residual_tolerance_by_order: config.maximum_residual_by_order
            [..=derivative_order]
            .to_vec(),
        maximum_normal_angle: 0.0,
        maximum_normal_angle_tolerance: config.maximum_normal_angle,
        worst_seam_by_order: vec![0.0; derivative_order + 1],
        worst_mixed_derivative_by_order: vec![[0, 0]; derivative_order + 1],
    };

    seams.into_iter().try_for_each(|seam_parameter| {
        let seam_nodes = seam_stencil(
            seam_parameter,
            radius,
            config.normalized_step,
            &stencils.centered_seam_nodes,
        );
        let seam_weights = (0..=derivative_order)
            .map(|derivative| finite_difference_weights(&seam_nodes, derivative))
            .collect::<Result<Vec<_>, _>>()?;
        let (first_grid, second_grid) = sample_grids(
            first,
            second,
            transition,
            seam,
            seam_parameter,
            &seam_nodes,
            &stencils,
        )?;
        record_residuals(
            &mut certificate,
            derivative_order,
            seam_parameter,
            &first_grid,
            &second_grid,
            &seam_weights,
            &stencils,
        )
    })?;

    verify_certificate(
        &certificate,
        config.maximum_residual_by_order,
        config.maximum_normal_angle,
    )?;
    Ok(certificate)
}

fn validate_config(
    order: ContinuityOrder,
    config: &CertificationConfig<'_>,
) -> Result<(), ValidationError> {
    let stencil_width = config
        .stencil_radius
        .checked_mul(2)
        .and_then(|value| value.checked_add(1))
        .ok_or(ValidationError::InvalidCertificationConfig {
            reason: "the stencil width overflows `usize`",
        })?;
    if config.intervals < 32 {
        Err(ValidationError::SparseCertification)
    } else if config.intervals > MAX_CERTIFICATION_INTERVALS {
        Err(ValidationError::InvalidCertificationConfig {
            reason: "the interval count must not exceed `65_536`",
        })
    } else if !(1..=MAX_STENCIL_RADIUS).contains(&config.stencil_radius) {
        Err(ValidationError::InvalidCertificationConfig {
            reason: "the stencil radius must be in `1..=16`",
        })
    } else if !config.normalized_step.is_finite() || config.normalized_step <= 0.0 {
        Err(ValidationError::InvalidCertificationConfig {
            reason: "the normalized step must be positive and finite",
        })
    } else if stencil_width <= order.as_usize()
        || 2.0 * config.stencil_radius as f64 * config.normalized_step > 1.0
    {
        Err(ValidationError::InvalidCertificationConfig {
            reason: "the stencil cannot support the requested order inside the normalized domain",
        })
    } else if config.maximum_residual_by_order.len() <= order.as_usize() {
        Err(ValidationError::InvalidCertificationConfig {
            reason: "one residual tolerance is required for every requested order",
        })
    } else if config
        .maximum_residual_by_order
        .iter()
        .take(order.as_usize() + 1)
        .chain(std::iter::once(&config.maximum_normal_angle))
        .any(|value| !value.is_finite() || *value <= 0.0)
    {
        Err(ValidationError::InvalidCertificationConfig {
            reason: "certification tolerances must be positive and finite",
        })
    } else {
        Ok(())
    }
}

fn record_residuals(
    certificate: &mut Certificate,
    order: usize,
    seam: f64,
    first_grid: &SampleGrid,
    second_grid: &SampleGrid,
    seam_weights: &[Vec<f64>],
    stencils: &CertificationStencils,
) -> Result<(), ValidationError> {
    (0..=order).try_for_each(|total| {
        (0..=total).try_for_each(|cross_order| {
            let seam_order = total - cross_order;
            let first = mixed_derivative(
                first_grid,
                &seam_weights[seam_order],
                &stencils.first_cross_weights[cross_order],
            );
            let second = mixed_derivative(
                second_grid,
                &seam_weights[seam_order],
                &stencils.second_cross_weights[cross_order],
            );
            ensure_finite_vector(first, seam)?;
            ensure_finite_vector(second, seam)?;
            let absolute = (first - second).magnitude();
            let normalized = absolute / certificate.scale;
            if !absolute.is_finite() || !normalized.is_finite() {
                Err(ValidationError::NonFiniteCertificate { seam })
            } else {
                certificate.maximum_absolute_residual_by_order[total] =
                    certificate.maximum_absolute_residual_by_order[total].max(absolute);
                if normalized > certificate.maximum_normalized_residual_by_order[total] {
                    certificate.maximum_normalized_residual_by_order[total] = normalized;
                    certificate.worst_seam_by_order[total] = seam;
                    certificate.worst_mixed_derivative_by_order[total] = [seam_order, cross_order];
                }
                Ok(())
            }
        })
    })?;
    if order >= 1 {
        let first_seam = mixed_derivative(
            first_grid,
            &seam_weights[1],
            &stencils.first_cross_weights[0],
        );
        let first_cross = mixed_derivative(
            first_grid,
            &seam_weights[0],
            &stencils.first_cross_weights[1],
        );
        let second_seam = mixed_derivative(
            second_grid,
            &seam_weights[1],
            &stencils.second_cross_weights[0],
        );
        let second_cross = mixed_derivative(
            second_grid,
            &seam_weights[0],
            &stencils.second_cross_weights[1],
        );
        [first_seam, first_cross, second_seam, second_cross]
            .into_iter()
            .try_for_each(|vector| ensure_nonzero_finite_vector(vector, seam))?;
        let first_normal = first_seam.cross(first_cross);
        let second_normal = second_seam.cross(second_cross);
        [first_normal, second_normal]
            .into_iter()
            .try_for_each(|vector| ensure_nonzero_finite_vector(vector, seam))?;
        let cosine = first_normal
            .normalize()
            .dot(second_normal.normalize())
            .abs()
            .clamp(-1.0, 1.0);
        let angle = cosine.acos();
        if angle.is_finite() {
            certificate.maximum_normal_angle = certificate.maximum_normal_angle.max(angle);
            Ok(())
        } else {
            Err(ValidationError::NonFiniteCertificate { seam })
        }
    } else {
        Ok(())
    }
}

fn verify_certificate(
    certificate: &Certificate,
    maximum_residual_by_order: &[f64],
    maximum_normal_angle: f64,
) -> Result<(), ValidationError> {
    certificate
        .maximum_normalized_residual_by_order
        .iter()
        .zip(maximum_residual_by_order)
        .enumerate()
        .find(|(_, (actual, limit))| actual > limit)
        .map_or(Ok(()), |(order, (actual, limit))| {
            Err(ValidationError::CertificationResidualFailed {
                order,
                maximum: *actual,
                tolerance: *limit,
            })
        })?;
    if certificate.maximum_normal_angle > maximum_normal_angle {
        Err(ValidationError::CertificationNormalFailed {
            maximum: certificate.maximum_normal_angle,
            tolerance: maximum_normal_angle,
        })
    } else {
        Ok(())
    }
}
