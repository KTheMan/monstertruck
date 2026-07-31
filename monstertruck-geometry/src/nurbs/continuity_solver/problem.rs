//! Boundary-problem preparation and analytic residual/Jacobian assembly.

use super::boundary::BoundaryFrame;
use super::dual::Dual;
use super::resource::ContinuityResourceBudget;
use super::sampling::{nonzero_span_count, seam_samples, seam_validation_samples};
use super::taylor::{JetScalar, TaylorJet};
use super::types::{
    BoundaryContinuityRequest, BoundaryEndpoint, BoundaryTransition, ContinuityResource,
    ContinuitySolveError, ContinuitySolverConfig, OrderResidual,
};
use crate::base::{InnerSpace, Vector3, Vector4};
use crate::nurbs::continuity::{
    BoundaryAlignment, ContinuityOrder, SurfaceAxis, SurfaceContinuityCapability,
};
use crate::nurbs::{BasisWindow, KnotVector, NurbsSurface};
use monstertruck_traits::ParametricSurface;

type ControlVariables<'a> = (&'a [Vec<Option<usize>>], &'a [Dual]);

pub(super) struct PreparedProblem<'surface> {
    first: &'surface NurbsSurface<Vector4>,
    second: &'surface NurbsSurface<Vector4>,
    first_frame: BoundaryFrame,
    second_frame: BoundaryFrame,
    request: BoundaryContinuityRequest,
    samples: Vec<f64>,
    validation_samples: Vec<f64>,
    characteristic_length: f64,
    control_indices: Vec<(usize, usize)>,
    control_offsets: Vec<Vec<Option<usize>>>,
    transition: TransitionLayout,
    initial_variables: Vec<f64>,
    strip_rows: usize,
    qr_elements: usize,
}

pub(super) struct ResidualEvaluation {
    pub(super) values: Vec<f64>,
    pub(super) jacobian: Vec<Vec<f64>>,
    pub(super) objective: f64,
    pub(super) residuals: Vec<OrderResidual>,
}

#[derive(Clone, Copy)]
struct TransitionLayout {
    field_count: usize,
    seam_map_offset: usize,
    seam_map_variable_count: usize,
    alpha_offset: usize,
    log_beta_offset: usize,
    higher_beta_offset: usize,
    order: usize,
    variable_count: usize,
}

mod evaluation;
mod preparation;
mod transition;
impl TransitionLayout {
    fn try_new(
        order: usize,
        field_count: usize,
        start: usize,
    ) -> Result<Self, ContinuitySolveError> {
        let seam_map_offset = start;
        let seam_map_variable_count = field_count.saturating_sub(1);
        let alpha_offset = seam_map_offset.checked_add(seam_map_variable_count).ok_or(
            ContinuitySolveError::InvalidConfig("transition field dimension overflowed"),
        )?;
        let order_fields =
            order
                .checked_mul(field_count)
                .ok_or(ContinuitySolveError::InvalidConfig(
                    "transition field dimension overflowed",
                ))?;
        let log_beta_offset =
            alpha_offset
                .checked_add(order_fields)
                .ok_or(ContinuitySolveError::InvalidConfig(
                    "transition field dimension overflowed",
                ))?;
        let higher_beta_offset =
            log_beta_offset
                .checked_add(field_count)
                .ok_or(ContinuitySolveError::InvalidConfig(
                    "transition field dimension overflowed",
                ))?;
        let variable_count = if order == 0 {
            seam_map_variable_count
        } else {
            checked_add(
                seam_map_variable_count,
                checked_mul(
                    checked_mul(2, order, "transition field dimension overflowed")?,
                    field_count,
                    "transition field dimension overflowed",
                )?,
                "transition field dimension overflowed",
            )?
        };
        Ok(Self {
            field_count,
            seam_map_offset,
            seam_map_variable_count,
            alpha_offset,
            log_beta_offset,
            higher_beta_offset,
            order,
            variable_count,
        })
    }

    fn variable_count(self) -> usize { self.variable_count }

    fn seam_map<T>(self, variables: &[T]) -> &[T] {
        &variables[self.seam_map_offset..self.seam_map_offset + self.seam_map_variable_count]
    }

    fn alpha<T>(self, variables: &[T], order: usize) -> &[T] {
        let start = self.alpha_offset + (order - 1) * self.field_count;
        &variables[start..start + self.field_count]
    }

    fn log_beta<T>(self, variables: &[T]) -> &[T] {
        &variables[self.log_beta_offset..self.log_beta_offset + self.field_count]
    }

    fn beta<T>(self, variables: &[T], order: usize) -> &[T] {
        let start = self.higher_beta_offset + (order - 2) * self.field_count;
        &variables[start..start + self.field_count]
    }
}

fn checked_add(
    first: usize,
    second: usize,
    message: &'static str,
) -> Result<usize, ContinuitySolveError> {
    first
        .checked_add(second)
        .ok_or(ContinuitySolveError::InvalidConfig(message))
}

fn checked_mul(
    first: usize,
    second: usize,
    message: &'static str,
) -> Result<usize, ContinuitySolveError> {
    first
        .checked_mul(second)
        .ok_or(ContinuitySolveError::InvalidConfig(message))
}

fn frame_span_count(
    surface: &NurbsSurface<Vector4>,
    frame: BoundaryFrame,
    endpoint: BoundaryEndpoint,
) -> Result<usize, ContinuitySolveError> {
    let knots = match frame.along_axis() {
        SurfaceAxis::U => surface.knot_vector_u(),
        SurfaceAxis::V => surface.knot_vector_v(),
    };
    nonzero_span_count(knots, frame.along_degree(), frame.along_control_count())
        .ok_or(ContinuitySolveError::InvalidBoundary(endpoint))
}

fn validation_density(
    first: BoundaryFrame,
    second: BoundaryFrame,
    request: BoundaryContinuityRequest,
    config: &ContinuitySolverConfig,
) -> Result<usize, ContinuitySolveError> {
    checked_mul(
        [
            first.along_degree().max(second.along_degree()),
            request.order().as_usize(),
            config.transition_degree(),
            1,
        ]
        .into_iter()
        .try_fold(0usize, |total, count| {
            checked_add(total, count, "validation sample density overflowed")
        })?,
        2,
        "validation sample density overflowed",
    )
    .map(|density| density.clamp(8, 64))
}

fn validate_capability(
    surface: &NurbsSurface<Vector4>,
    boundary: crate::nurbs::continuity::SurfaceBoundary,
    order: ContinuityOrder,
    endpoint: BoundaryEndpoint,
) -> Result<(), ContinuitySolveError> {
    let capability = SurfaceContinuityCapability::for_nurbs(surface, boundary, order);
    if capability.is_feasible() {
        Ok(())
    } else {
        Err(ContinuitySolveError::UnsupportedCapability {
            endpoint,
            capability,
        })
    }
}

fn validate_weights(
    surface: &NurbsSurface<Vector4>,
    endpoint: BoundaryEndpoint,
    minimum: f64,
) -> Result<(), ContinuitySolveError> {
    surface
        .control_points()
        .iter()
        .enumerate()
        .try_for_each(|(row, points)| {
            points.iter().enumerate().try_for_each(|(column, point)| {
                if !point.x.is_finite()
                    || !point.y.is_finite()
                    || !point.z.is_finite()
                    || !point.w.is_finite()
                {
                    Err(ContinuitySolveError::NonFiniteControlPoint {
                        endpoint,
                        row,
                        column,
                    })
                } else if point.w >= minimum {
                    Ok(())
                } else {
                    Err(ContinuitySolveError::NonPositiveWeight {
                        endpoint,
                        row,
                        column,
                        weight: point.w,
                    })
                }
            })
        })
}

fn frame_samples(
    surface: &NurbsSurface<Vector4>,
    frame: BoundaryFrame,
    samples_per_span: usize,
) -> Vec<f64> {
    let knots = match frame.along_axis() {
        SurfaceAxis::U => surface.knot_vector_u(),
        SurfaceAxis::V => surface.knot_vector_v(),
    };
    seam_samples(
        knots,
        frame.along_degree(),
        frame.along_control_count(),
        samples_per_span,
    )
    .unwrap_or_default()
}

fn frame_validation_samples(
    surface: &NurbsSurface<Vector4>,
    frame: BoundaryFrame,
    samples_per_span: usize,
) -> Vec<f64> {
    let knots = match frame.along_axis() {
        SurfaceAxis::U => surface.knot_vector_u(),
        SurfaceAxis::V => surface.knot_vector_v(),
    };
    seam_validation_samples(
        knots,
        frame.along_degree(),
        frame.along_control_count(),
        samples_per_span,
    )
    .unwrap_or_default()
}

fn characteristic_length(
    first: &NurbsSurface<Vector4>,
    first_frame: BoundaryFrame,
    samples: &[f64],
) -> Result<f64, ContinuitySolveError> {
    let points = samples
        .iter()
        .map(|&seam| {
            let first_parameters = first_frame.parameters(seam, 0.0);
            first.evaluate(first_parameters.0, first_parameters.1)
        })
        .collect::<Vec<_>>();
    let minimum = points.iter().fold(
        Vector3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY),
        |minimum, point| {
            Vector3::new(
                minimum.x.min(point.x),
                minimum.y.min(point.y),
                minimum.z.min(point.z),
            )
        },
    );
    let maximum = points.iter().fold(
        Vector3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY),
        |maximum, point| {
            Vector3::new(
                maximum.x.max(point.x),
                maximum.y.max(point.y),
                maximum.z.max(point.z),
            )
        },
    );
    let length = (maximum - minimum).magnitude();
    if length.is_finite() && length > f64::EPSILON {
        Ok(length)
    } else {
        Err(ContinuitySolveError::InvalidBoundary(
            BoundaryEndpoint::First,
        ))
    }
}

fn validate_regular_boundary(
    surface: &NurbsSurface<Vector4>,
    frame: BoundaryFrame,
    endpoint: BoundaryEndpoint,
    samples: &[f64],
    characteristic_length: f64,
) -> Result<(), ContinuitySolveError> {
    samples
        .iter()
        .copied()
        .enumerate()
        .try_for_each(|(sample, seam)| {
            let (u, v) = frame.parameters(seam, 0.0);
            let u_tangent = surface.derivative_u(u, v) * frame.u_domain().span();
            let v_tangent = surface.derivative_v(u, v) * frame.v_domain().span();
            let u_length = u_tangent.magnitude();
            let v_length = v_tangent.magnitude();
            let area = u_tangent.cross(v_tangent).magnitude();
            if area.is_finite()
                && u_length.is_finite()
                && v_length.is_finite()
                && u_length > 1.0e-12 * characteristic_length
                && v_length > 1.0e-12 * characteristic_length
                && area > 1.0e-12 * characteristic_length * characteristic_length
            {
                Ok(())
            } else {
                Err(ContinuitySolveError::DegenerateBoundary { endpoint, sample })
            }
        })
}

fn order_residuals(
    residuals: &[(usize, usize, usize, usize, [Dual; 3])],
    requested: ContinuityOrder,
) -> Result<Vec<OrderResidual>, ContinuitySolveError> {
    (0..=requested.as_usize())
        .map(|order| {
            let values = residuals
                .iter()
                .filter(|(actual, _, _, _, _)| *actual == order)
                .map(|(_, cross, seam, sample, vector)| {
                    let magnitude = vector
                        .iter()
                        .map(|value| value.value() * value.value())
                        .sum::<f64>()
                        .sqrt();
                    (*sample, *cross, *seam, magnitude)
                })
                .collect::<Vec<_>>();
            if values.is_empty() || values.iter().any(|(_, _, _, value)| !value.is_finite()) {
                Err(ContinuitySolveError::NonFiniteResidual)
            } else {
                let rms = (values
                    .iter()
                    .map(|(_, _, _, value)| value * value)
                    .sum::<f64>()
                    / values.len() as f64)
                    .sqrt();
                // SAFETY: The empty `values` case returns before this maximum is computed.
                let (worst_sample, cross_derivative, seam_derivative, maximum) = values
                    .into_iter()
                    .max_by(|first, second| first.3.total_cmp(&second.3))
                    .expect("the residual list is nonempty");
                Ok(OrderResidual::new(
                    // SAFETY: `order` is bounded by the validated kernel order.
                    ContinuityOrder::new(order)
                        .expect("the prepared order is within the kernel limit"),
                    rms,
                    maximum,
                    worst_sample,
                    false,
                    cross_derivative,
                    seam_derivative,
                ))
            }
        })
        .collect()
}

fn stable_norm(values: &[f64]) -> f64 {
    let scale = values.iter().map(|value| value.abs()).fold(0.0, f64::max);
    if scale == 0.0 {
        0.0
    } else {
        scale
            * values
                .iter()
                .map(|value| {
                    let scaled = value / scale;
                    scaled * scaled
                })
                .sum::<f64>()
                .sqrt()
    }
}

const fn factorial(value: usize) -> f64 {
    match value {
        0 | 1 => 1.0,
        2 => 2.0,
        3 => 6.0,
        4 => 24.0,
        _ => f64::INFINITY,
    }
}

#[cfg(test)]
mod tests;
