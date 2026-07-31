//! Independent dense G1 certification.

use super::classify::{SeamSelection, boundary_parameters};
use super::errors::ValidationError;
use monstertruck_geometry::nurbs::NurbsSurface;
use monstertruck_geometry::nurbs::continuity_solver::BoundaryTransition;
use monstertruck_geometry::prelude::{
    InnerSpace, MetricSpace, ParametricSurface, Vector3, Vector4,
};

#[derive(Clone, Copy, Debug)]
pub(super) struct Certificate {
    pub(super) samples: usize,
    pub(super) position_maximum: f64,
    pub(super) tangent_maximum: f64,
}

pub(super) fn certify_g1(
    first: &NurbsSurface<Vector4>,
    second: &NurbsSurface<Vector4>,
    transition: &BoundaryTransition,
    seam: SeamSelection,
    intervals: usize,
    position_tolerance: f64,
    tangent_tolerance: f64,
) -> Result<Certificate, ValidationError> {
    if intervals < 32 {
        Err(ValidationError::SparseCertification)
    } else {
        let samples = (0..=intervals)
            .map(|sample| certify_sample(first, second, transition, seam, sample, intervals))
            .collect::<Result<Vec<_>, _>>()?;
        let position_maximum = samples
            .iter()
            .map(|sample| sample.position)
            .fold(0.0, f64::max);
        let tangent_maximum = samples
            .iter()
            .map(|sample| sample.tangent)
            .fold(0.0, f64::max);
        if position_maximum > position_tolerance || tangent_maximum > tangent_tolerance {
            Err(ValidationError::CertificationFailed {
                position_maximum,
                position_tolerance,
                tangent_maximum,
                tangent_tolerance,
            })
        } else {
            Ok(Certificate {
                samples: intervals + 1,
                position_maximum,
                tangent_maximum,
            })
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct SampleResidual {
    position: f64,
    tangent: f64,
}

fn certify_sample(
    first: &NurbsSurface<Vector4>,
    second: &NurbsSurface<Vector4>,
    transition: &BoundaryTransition,
    seam: SeamSelection,
    sample: usize,
    intervals: usize,
) -> Result<SampleResidual, ValidationError> {
    let normalized = sample as f64 / intervals as f64;
    let second_normalized = transition
        .mapped_coordinates(normalized, 0.0)
        .map(|(mapped_seam, _)| mapped_seam)
        .filter(|value| value.is_finite())
        .ok_or(ValidationError::DegenerateTangentFrame { sample })?;
    let (first_u, first_v) = boundary_parameters(first, seam.first_boundary, normalized);
    let (second_u, second_v) = boundary_parameters(second, seam.second_boundary, second_normalized);
    let first_point = first.evaluate(first_u, first_v);
    let second_point = second.evaluate(second_u, second_v);
    let first_normal = tangent_normal(first, first_u, first_v)
        .ok_or(ValidationError::DegenerateTangentFrame { sample })?;
    let second_normal = tangent_normal(second, second_u, second_v)
        .ok_or(ValidationError::DegenerateTangentFrame { sample })?;
    Ok(SampleResidual {
        position: first_point.distance(second_point),
        tangent: first_normal.cross(second_normal).magnitude(),
    })
}

fn tangent_normal(surface: &NurbsSurface<Vector4>, u: f64, v: f64) -> Option<Vector3> {
    let normal = surface.derivative_u(u, v).cross(surface.derivative_v(u, v));
    let magnitude = normal.magnitude();
    (magnitude.is_finite() && magnitude > f64::EPSILON).then(|| normal / magnitude)
}
