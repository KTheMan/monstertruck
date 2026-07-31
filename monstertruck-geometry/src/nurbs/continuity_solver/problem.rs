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
use crate::nurbs::continuity::{ContinuityOrder, SurfaceAxis, SurfaceContinuityCapability};
use crate::nurbs::{BasisWindow, KnotVector, NurbsSurface};
use monstertruck_traits::ParametricSurface;

type ControlVariables<'a> = (&'a [Vec<Option<usize>>], &'a [Dual]);

pub(super) struct PreparedProblem {
    first: NurbsSurface<Vector4>,
    second: NurbsSurface<Vector4>,
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

impl PreparedProblem {
    pub(super) fn new(
        first: &NurbsSurface<Vector4>,
        second: &NurbsSurface<Vector4>,
        request: BoundaryContinuityRequest,
        config: &ContinuitySolverConfig,
        resource_budget: ContinuityResourceBudget,
    ) -> Result<Self, ContinuitySolveError> {
        config.validate()?;
        if request.order() == ContinuityOrder::G4 && !config.allows_experimental_g4() {
            return Err(ContinuitySolveError::ExperimentalG4Disabled);
        }
        let first_frame = BoundaryFrame::try_new(first, request.first_boundary())
            .map_err(|_| ContinuitySolveError::InvalidBoundary(BoundaryEndpoint::First))?;
        let second_frame = BoundaryFrame::try_new(second, request.second_boundary())
            .map_err(|_| ContinuitySolveError::InvalidBoundary(BoundaryEndpoint::Second))?;
        validate_capability(
            first,
            request.first_boundary(),
            request.order(),
            BoundaryEndpoint::First,
        )?;
        validate_capability(
            second,
            request.second_boundary(),
            request.order(),
            BoundaryEndpoint::Second,
        )?;
        validate_weights(first, BoundaryEndpoint::First, config.minimum_weight())?;
        validate_weights(second, BoundaryEndpoint::Second, config.minimum_weight())?;

        let control_point_count = checked_add(
            checked_mul(
                first_frame.u_control_count(),
                first_frame.v_control_count(),
                "surface control-point dimension overflowed",
            )?,
            checked_mul(
                second_frame.u_control_count(),
                second_frame.v_control_count(),
                "surface control-point dimension overflowed",
            )?,
            "surface control-point dimension overflowed",
        )?;
        resource_budget.ensure(ContinuityResource::ControlPoints, control_point_count)?;
        let first_spans = frame_span_count(first, first_frame, BoundaryEndpoint::First)?;
        let second_spans = frame_span_count(second, second_frame, BoundaryEndpoint::Second)?;
        let span_count = checked_add(first_spans, second_spans, "seam span count overflowed")?;
        resource_budget.ensure(ContinuityResource::Spans, span_count)?;
        let validation_density = validation_density(first_frame, second_frame, request, config)?;
        let optimizer_sample_upper = checked_mul(
            span_count,
            checked_add(
                config.samples_per_span(),
                2,
                "optimizer sample density overflowed",
            )?,
            "optimizer sample count overflowed",
        )?;
        let validation_sample_upper = checked_mul(
            span_count,
            validation_density,
            "validation sample count overflowed",
        )?;
        resource_budget.ensure(
            ContinuityResource::Samples,
            checked_add(
                optimizer_sample_upper,
                validation_sample_upper,
                "total sample count overflowed",
            )?,
        )?;

        let mut samples = frame_samples(first, first_frame, config.samples_per_span())
            .into_iter()
            .chain(
                frame_samples(second, second_frame, config.samples_per_span())
                    .into_iter()
                    .map(|seam| match request.alignment() {
                        crate::nurbs::contract::BoundaryAlignment::Aligned => seam,
                        crate::nurbs::contract::BoundaryAlignment::Reversed => 1.0 - seam,
                    }),
            )
            .collect::<Vec<_>>();
        samples.sort_by(f64::total_cmp);
        samples.dedup_by(|first, second| first.to_bits() == second.to_bits());
        if samples.is_empty() {
            return Err(ContinuitySolveError::InvalidBoundary(
                BoundaryEndpoint::First,
            ));
        }
        let mut validation_samples =
            frame_validation_samples(first, first_frame, validation_density)
                .into_iter()
                .chain(
                    frame_validation_samples(second, second_frame, validation_density)
                        .into_iter()
                        .map(|seam| match request.alignment() {
                            crate::nurbs::contract::BoundaryAlignment::Aligned => seam,
                            crate::nurbs::contract::BoundaryAlignment::Reversed => 1.0 - seam,
                        }),
                )
                .collect::<Vec<_>>();
        validation_samples.sort_by(f64::total_cmp);
        validation_samples.dedup_by(|first, second| first.to_bits() == second.to_bits());
        validation_samples.retain(|candidate| {
            samples
                .binary_search_by(|sample| sample.total_cmp(candidate))
                .is_err()
        });
        if validation_samples.is_empty() {
            return Err(ContinuitySolveError::InvalidBoundary(
                BoundaryEndpoint::First,
            ));
        }
        resource_budget.ensure(
            ContinuityResource::Samples,
            checked_add(
                samples.len(),
                validation_samples.len(),
                "total sample count overflowed",
            )?,
        )?;

        let strip_rows =
            (request.order().constrained_rows() + 2).min(second_frame.cross_control_count());
        let strip_control_count = checked_mul(
            strip_rows,
            second_frame.along_control_count(),
            "boundary strip dimension overflowed",
        )?;
        let control_variable_count = checked_mul(
            3,
            strip_control_count,
            "control variable dimension overflowed",
        )?;
        let transition = TransitionLayout::try_new(
            request.order().as_usize(),
            config.transition_degree().checked_add(1).ok_or(
                ContinuitySolveError::InvalidConfig("transition field dimension overflowed"),
            )?,
            control_variable_count,
        )?;
        let variable_count = checked_add(
            control_variable_count,
            transition.variable_count(),
            "optimization variable dimension overflowed",
        )?;
        resource_budget.ensure(ContinuityResource::Variables, variable_count)?;
        let taylor_terms = checked_mul(
            request.order().as_usize() + 1,
            request.order().as_usize() + 2,
            "Taylor residual dimension overflowed",
        )? / 2;
        let continuity_residuals = checked_mul(
            checked_mul(samples.len(), 3, "continuity residual dimension overflowed")?,
            taylor_terms,
            "continuity residual dimension overflowed",
        )?;
        let fairness_stencils = if strip_rows < 3 || config.fairness_weight() == 0.0 {
            0
        } else {
            strip_rows
                .saturating_sub(1)
                .min(second_frame.cross_control_count().saturating_sub(2))
        };
        let fairness_residuals = checked_mul(
            checked_mul(
                fairness_stencils,
                second_frame.along_control_count(),
                "fairness residual dimension overflowed",
            )?,
            3,
            "fairness residual dimension overflowed",
        )?;
        let optimizer_residuals = [
            continuity_residuals,
            control_variable_count,
            fairness_residuals,
            transition.variable_count(),
        ]
        .into_iter()
        .try_fold(0usize, |total, count| {
            checked_add(total, count, "optimizer residual dimension overflowed")
        })?;
        let validation_residuals = checked_mul(
            checked_mul(
                validation_samples.len(),
                3,
                "validation residual dimension overflowed",
            )?,
            taylor_terms,
            "validation residual dimension overflowed",
        )?;
        resource_budget.ensure(
            ContinuityResource::Residuals,
            checked_add(
                optimizer_residuals,
                validation_residuals,
                "total residual dimension overflowed",
            )?,
        )?;
        resource_budget.ensure(
            ContinuityResource::JacobianElements,
            checked_mul(
                optimizer_residuals,
                variable_count,
                "Jacobian dimension overflowed",
            )?,
        )?;
        let qr_elements = checked_mul(
            checked_add(
                optimizer_residuals,
                variable_count,
                "augmented QR row dimension overflowed",
            )?,
            variable_count,
            "augmented QR dimension overflowed",
        )?;

        let characteristic_length = characteristic_length(first, first_frame, &samples)?;
        validate_regular_boundary(
            first,
            first_frame,
            BoundaryEndpoint::First,
            &samples,
            characteristic_length,
        )?;
        validate_regular_boundary(
            second,
            second_frame,
            BoundaryEndpoint::Second,
            &samples,
            characteristic_length,
        )?;

        let control_indices = second_frame
            .control_strip_indices(strip_rows)
            .map_err(|_| ContinuitySolveError::InvalidBoundary(BoundaryEndpoint::Second))?;
        let mut control_offsets =
            vec![vec![None; second_frame.v_control_count()]; second_frame.u_control_count()];
        control_indices
            .iter()
            .enumerate()
            .for_each(|(index, &(row, column))| {
                control_offsets[row][column] = Some(3 * index);
            });
        let initial_variables = control_indices
            .iter()
            .flat_map(|&(row, column)| {
                let point = second.control_point(row, column);
                [point.x / point.w, point.y / point.w, point.z / point.w]
            })
            .chain((0..transition.variable_count()).map(|_| 0.0))
            .collect();

        Ok(Self {
            first: first.clone(),
            second: second.clone(),
            first_frame,
            second_frame,
            request,
            samples,
            validation_samples,
            characteristic_length,
            control_indices,
            control_offsets,
            transition,
            initial_variables,
            strip_rows,
            qr_elements,
        })
    }

    pub(super) fn initial_variables(&self) -> &[f64] { &self.initial_variables }

    pub(super) fn variable_count(&self) -> usize { self.initial_variables.len() }

    pub(super) const fn qr_elements(&self) -> usize { self.qr_elements }

    pub(super) fn first(&self) -> &NurbsSurface<Vector4> { &self.first }

    pub(super) fn solved_second(&self, variables: &[f64]) -> NurbsSurface<Vector4> {
        let mut surface = self.second.clone();
        self.control_indices
            .iter()
            .enumerate()
            .for_each(|(index, &(row, column))| {
                let point = surface.control_point_mut(row, column);
                let weight = point.w;
                point.x = variables[3 * index] * weight;
                point.y = variables[3 * index + 1] * weight;
                point.z = variables[3 * index + 2] * weight;
            });
        surface
    }

    pub(super) fn solved_transition(&self, variables: &[f64]) -> BoundaryTransition {
        let order = self.request.order();
        BoundaryTransition::new(
            order,
            self.request.alignment(),
            self.transition.seam_map(variables).to_vec(),
            (1..=order.as_usize())
                .map(|derivative| self.transition.alpha(variables, derivative).to_vec())
                .collect(),
            if order == ContinuityOrder::G0 {
                Vec::new()
            } else {
                self.transition.log_beta(variables).to_vec()
            },
            (2..=order.as_usize())
                .map(|derivative| self.transition.beta(variables, derivative).to_vec())
                .collect(),
        )
    }

    pub(super) fn evaluate(
        &self,
        variables: &[f64],
        config: &ContinuitySolverConfig,
        with_jacobian: bool,
    ) -> Result<ResidualEvaluation, ContinuitySolveError> {
        self.evaluate_samples(variables, config, with_jacobian, &self.samples, true)
    }

    pub(super) fn validation_residuals(
        &self,
        variables: &[f64],
        config: &ContinuitySolverConfig,
    ) -> Result<Vec<OrderResidual>, ContinuitySolveError> {
        self.evaluate_samples(variables, config, false, &self.validation_samples, false)
            .map(|evaluation| evaluation.residuals)
    }

    fn evaluate_samples(
        &self,
        variables: &[f64],
        config: &ContinuitySolverConfig,
        with_jacobian: bool,
        samples: &[f64],
        include_regularization: bool,
    ) -> Result<ResidualEvaluation, ContinuitySolveError> {
        if variables.len() != self.variable_count() {
            return Err(ContinuitySolveError::InvalidConfig(
                "optimization variable dimension changed",
            ));
        }
        let variable_count = usize::from(with_jacobian) * variables.len();
        let scalars = variables
            .iter()
            .enumerate()
            .map(|(index, &value)| {
                if with_jacobian {
                    Dual::variable(value, index, variable_count)
                } else {
                    Dual::constant(value)
                }
            })
            .collect::<Vec<_>>();
        let mut residuals = Vec::new();
        let mut continuity = Vec::new();
        let order = self.request.order().as_usize();
        samples
            .iter()
            .copied()
            .enumerate()
            .for_each(|(sample_index, seam)| {
                let cross = TaylorJet::coordinate_s(order, Dual::constant(0.0));
                let seam_jet = TaylorJet::coordinate_r(order, Dual::constant(seam));
                let first_inward = TaylorJet::zero(order) - cross.clone();
                let (first_u, first_v) =
                    parameter_jets(self.first_frame, seam_jet.clone(), first_inward);
                let (second_seam, second_inward) = self.transition_jets(&scalars, seam_jet, cross);
                let (second_u, second_v) =
                    parameter_jets(self.second_frame, second_seam, second_inward);
                let first_value = compose_surface(&self.first, None, &first_u, &first_v);
                let second_value = compose_surface(
                    &self.second,
                    Some((&self.control_offsets, &scalars)),
                    &second_u,
                    &second_v,
                );
                (0..=order).for_each(|total| {
                    (0..=total).for_each(|cross_order| {
                        let seam_order = total - cross_order;
                        let scale = factorial(cross_order) * factorial(seam_order)
                            / self.characteristic_length;
                        let vector: [Dual; 3] = std::array::from_fn(|coordinate| {
                            let first = first_value[coordinate]
                                .coefficient(cross_order, seam_order)
                                .expect("the requested coefficient is active")
                                .clone();
                            let second = second_value[coordinate]
                                .coefficient(cross_order, seam_order)
                                .expect("the requested coefficient is active")
                                .clone();
                            (first - second) * Dual::constant(scale)
                        });
                        continuity.push((
                            total,
                            cross_order,
                            seam_order,
                            sample_index,
                            vector.clone(),
                        ));
                        let tolerance = config.tolerance(
                            ContinuityOrder::new(total)
                                .expect("the prepared order is within the kernel limit"),
                        );
                        residuals.extend(
                            vector
                                .into_iter()
                                .map(|value| value * Dual::constant(1.0 / tolerance)),
                        );
                    });
                });
            });

        if include_regularization {
            let anchor_scale = config.anchor_weight().sqrt() / self.characteristic_length;
            scalars[..3 * self.control_indices.len()]
                .iter()
                .zip(&self.initial_variables)
                .for_each(|(value, &initial)| {
                    residuals.push(
                        (value.clone() - Dual::constant(initial)) * Dual::constant(anchor_scale),
                    );
                });
            self.append_fairness_residuals(&scalars, config, &mut residuals);
            let transition_scale = config.transition_weight().sqrt();
            scalars[3 * self.control_indices.len()..]
                .iter()
                .for_each(|value| {
                    residuals.push(value.clone() * Dual::constant(transition_scale));
                });
        }

        if residuals.iter().any(|value| !value.value().is_finite()) {
            return Err(ContinuitySolveError::NonFiniteResidual);
        }
        let values = residuals.iter().map(Dual::value).collect::<Vec<_>>();
        let jacobian = if with_jacobian {
            let rows = residuals
                .iter()
                .map(|value| value.gradient().to_vec())
                .collect::<Vec<_>>();
            if rows.iter().flatten().any(|value| !value.is_finite()) {
                return Err(ContinuitySolveError::NonFiniteJacobian);
            }
            rows
        } else {
            Vec::new()
        };
        let residual_norm = stable_norm(&values);
        let objective = 0.5 * residual_norm * residual_norm;
        if !objective.is_finite() {
            return Err(ContinuitySolveError::NonFiniteResidual);
        }
        let order_residuals = order_residuals(&continuity, self.request.order())?;
        Ok(ResidualEvaluation {
            values,
            jacobian,
            objective,
            residuals: order_residuals,
        })
    }

    fn transition_jets(
        &self,
        variables: &[Dual],
        seam: TaylorJet<Dual>,
        cross: TaylorJet<Dual>,
    ) -> (TaylorJet<Dual>, TaylorJet<Dual>) {
        let order = self.transition.order;
        let mapped_seam = monotone_seam_map(self.transition.seam_map(variables), seam.clone());
        if order == 0 {
            (
                aligned_seam_jet(mapped_seam, self.request.alignment()),
                cross,
            )
        } else {
            let mut second_seam = aligned_seam_jet(mapped_seam, self.request.alignment());
            let log_beta = bernstein_field(self.transition.log_beta(variables), seam.clone());
            let mut second_inward = log_beta.exp() * cross.clone();
            (1..=order).for_each(|derivative_order| {
                let power = cross
                    .powi(derivative_order)
                    .scaled_f64(1.0 / factorial(derivative_order));
                second_seam = second_seam.clone()
                    + bernstein_field(
                        self.transition.alpha(variables, derivative_order),
                        seam.clone(),
                    ) * power.clone();
                if derivative_order >= 2 {
                    second_inward = second_inward.clone()
                        + bernstein_field(
                            self.transition.beta(variables, derivative_order),
                            seam.clone(),
                        ) * power;
                }
            });
            (second_seam, second_inward)
        }
    }

    fn append_fairness_residuals(
        &self,
        variables: &[Dual],
        config: &ContinuitySolverConfig,
        residuals: &mut Vec<Dual>,
    ) {
        if self.strip_rows < 3 || config.fairness_weight() == 0.0 {
            return;
        }
        let scale = config.fairness_weight().sqrt() / self.characteristic_length;
        (1..self.strip_rows)
            .filter(|&distance| distance + 1 < self.second_frame.cross_control_count())
            .for_each(|distance| {
                (0..self.second_frame.along_control_count()).for_each(|seam| {
                    (0..3).for_each(|coordinate| {
                        let indices = [distance - 1, distance, distance + 1].map(|offset| {
                            self.second_frame
                                .control_point_index(offset, seam)
                                .expect("the prepared strip index is valid")
                        });
                        let current = physical_control_scalar(
                            &self.second,
                            &self.control_offsets,
                            variables,
                            indices[0],
                            coordinate,
                        ) - physical_control_scalar(
                            &self.second,
                            &self.control_offsets,
                            variables,
                            indices[1],
                            coordinate,
                        ) * Dual::constant(2.0)
                            + physical_control_scalar(
                                &self.second,
                                &self.control_offsets,
                                variables,
                                indices[2],
                                coordinate,
                            );
                        residuals.push(current * Dual::constant(scale));
                    });
                });
            });
    }
}

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

fn parameter_jets(
    frame: BoundaryFrame,
    seam: TaylorJet<Dual>,
    inward: TaylorJet<Dual>,
) -> (TaylorJet<Dual>, TaylorJet<Dual>) {
    let along = TaylorJet::constant(seam.order(), Dual::constant(frame.along_domain().start()))
        + seam.scaled_f64(frame.along_parameter_span());
    let cross_start = if frame.inward_parameter_sign().is_sign_positive() {
        frame.cross_domain().start()
    } else {
        frame.cross_domain().end()
    };
    let cross = TaylorJet::constant(inward.order(), Dual::constant(cross_start))
        + inward.scaled_f64(frame.inward_parameter_scale());
    match frame.cross_axis() {
        SurfaceAxis::U => (cross, along),
        SurfaceAxis::V => (along, cross),
    }
}

fn aligned_seam_jet(
    seam: TaylorJet<Dual>,
    alignment: crate::nurbs::contract::BoundaryAlignment,
) -> TaylorJet<Dual> {
    match alignment {
        crate::nurbs::contract::BoundaryAlignment::Aligned => seam,
        crate::nurbs::contract::BoundaryAlignment::Reversed => {
            TaylorJet::constant(seam.order(), Dual::constant(1.0)) - seam
        }
    }
}

fn bernstein_field(coefficients: &[Dual], parameter: TaylorJet<Dual>) -> TaylorJet<Dual> {
    let order = parameter.order();
    let one_minus = TaylorJet::constant(order, Dual::constant(1.0)) - parameter.clone();
    let mut level = coefficients
        .iter()
        .cloned()
        .map(|coefficient| TaylorJet::constant(order, coefficient))
        .collect::<Vec<_>>();
    (1..coefficients.len()).for_each(|remaining| {
        level = (0..coefficients.len() - remaining)
            .map(|index| {
                level[index].clone() * one_minus.clone()
                    + level[index + 1].clone() * parameter.clone()
            })
            .collect();
    });
    level
        .into_iter()
        .next()
        .unwrap_or_else(|| TaylorJet::zero(order))
}

fn monotone_seam_map(free_log_increments: &[Dual], parameter: TaylorJet<Dual>) -> TaylorJet<Dual> {
    let mut increments = free_log_increments
        .iter()
        .cloned()
        .map(Dual::exp)
        .collect::<Vec<_>>();
    increments.push(Dual::constant(1.0));
    let total = increments
        .iter()
        .cloned()
        .fold(Dual::constant(0.0), |sum, increment| sum + increment);
    let controls = std::iter::once(Dual::constant(0.0))
        .chain(
            increments
                .into_iter()
                .scan(Dual::constant(0.0), |cumulative, increment| {
                    *cumulative = cumulative.clone() + increment / total.clone();
                    Some(cumulative.clone())
                }),
        )
        .collect::<Vec<_>>();
    bernstein_field(&controls, parameter)
}

fn compose_surface(
    surface: &NurbsSurface<Vector4>,
    variables: Option<ControlVariables<'_>>,
    u: &TaylorJet<Dual>,
    v: &TaylorJet<Dual>,
) -> [TaylorJet<Dual>; 3] {
    let order = u.order();
    let u_base = u
        .coefficient(0, 0)
        .expect("the constant coefficient is active")
        .value();
    let v_base = v
        .coefficient(0, 0)
        .expect("the constant coefficient is active")
        .value();
    let u_basis = basis_jets(
        surface.knot_vector_u(),
        surface.udegree(),
        surface.control_points().len(),
        u_base,
        u,
    );
    let v_basis = basis_jets(
        surface.knot_vector_v(),
        surface.vdegree(),
        surface.control_points()[0].len(),
        v_base,
        v,
    );
    let denominator = surface
        .control_points()
        .iter()
        .enumerate()
        .flat_map(|(row, points)| {
            let u_basis = &u_basis;
            let v_basis = &v_basis;
            points.iter().enumerate().map(move |(column, point)| {
                u_basis[row].clone()
                    * v_basis[column].clone()
                    * TaylorJet::constant(order, Dual::constant(point.w))
            })
        })
        .fold(TaylorJet::zero(order), |sum, value| sum + value);
    std::array::from_fn(|coordinate| {
        let numerator = surface
            .control_points()
            .iter()
            .enumerate()
            .flat_map(|(row, points)| {
                let u_basis = &u_basis;
                let v_basis = &v_basis;
                points.iter().enumerate().map(move |(column, point)| {
                    let physical = match variables {
                        Some((offsets, values)) => offsets[row][column]
                            .map(|offset| values[offset + coordinate].clone())
                            .unwrap_or_else(|| {
                                Dual::constant(physical_coordinate(point, coordinate))
                            }),
                        None => Dual::constant(physical_coordinate(point, coordinate)),
                    };
                    u_basis[row].clone()
                        * v_basis[column].clone()
                        * TaylorJet::constant(order, physical * Dual::constant(point.w))
                })
            })
            .fold(TaylorJet::zero(order), |sum, value| sum + value);
        numerator / denominator.clone()
    })
}

fn basis_jets(
    knots: &KnotVector,
    degree: usize,
    control_count: usize,
    base: f64,
    parameter: &TaylorJet<Dual>,
) -> Vec<TaylorJet<Dual>> {
    let order = parameter.order();
    let delta = parameter.clone() - TaylorJet::constant(order, Dual::constant(base));
    let windows = (0..=order)
        .map(|derivative| knots.bspline_basis_functions(degree, derivative, base))
        .collect::<Vec<_>>();
    (0..control_count)
        .map(|control| {
            (0..=order).fold(TaylorJet::zero(order), |sum, derivative| {
                sum + delta
                    .powi(derivative)
                    .scaled_f64(basis_value(&windows[derivative], control) / factorial(derivative))
            })
        })
        .collect()
}

fn basis_value(window: &BasisWindow, index: usize) -> f64 {
    index
        .checked_sub(window.start_index())
        .and_then(|offset| window.values().get(offset))
        .copied()
        .unwrap_or(0.0)
}

fn physical_control_scalar(
    surface: &NurbsSurface<Vector4>,
    offsets: &[Vec<Option<usize>>],
    variables: &[Dual],
    (row, column): (usize, usize),
    coordinate: usize,
) -> Dual {
    offsets[row][column]
        .map(|offset| variables[offset + coordinate].clone())
        .unwrap_or_else(|| {
            Dual::constant(physical_coordinate(
                surface.control_point(row, column),
                coordinate,
            ))
        })
}

fn physical_coordinate(point: &Vector4, coordinate: usize) -> f64 { point[coordinate] / point.w }

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
                let (worst_sample, cross_derivative, seam_derivative, maximum) = values
                    .into_iter()
                    .max_by(|first, second| first.3.total_cmp(&second.3))
                    .expect("the residual list is nonempty");
                Ok(OrderResidual::new(
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
mod tests {
    use super::*;

    #[test]
    fn transition_layout_partitions_every_order_without_overlap() {
        (0..=4).for_each(|order| {
            let start = 7;
            let field_count = 4;
            let layout = TransitionLayout::try_new(order, field_count, start)
                .expect("small transition layouts are representable");
            let variables = (0..start + layout.variable_count())
                .map(|index| Dual::constant(index as f64))
                .collect::<Vec<_>>();

            assert_eq!(layout.seam_map(&variables).len(), field_count - 1);
            assert_eq!(layout.seam_map(&variables)[0].value(), start as f64);
            if order > 0 {
                assert_eq!(
                    layout.alpha(&variables, 1)[0].value(),
                    (start + field_count - 1) as f64
                );
                assert_eq!(
                    layout.log_beta(&variables)[0].value(),
                    (start + field_count - 1 + order * field_count) as f64
                );
                if order > 1 {
                    assert_eq!(
                        layout.beta(&variables, order)[field_count - 1].value(),
                        (start + layout.variable_count() - 1) as f64
                    );
                }
            }
        });
    }

    #[test]
    fn seam_map_is_endpoint_preserving_and_strictly_monotone() {
        let logs = [
            Dual::constant(-1.0),
            Dual::constant(0.5),
            Dual::constant(-0.25),
        ];
        let values = [0.0, 0.2, 0.5, 0.8, 1.0].map(|parameter| {
            let jet =
                monotone_seam_map(&logs, TaylorJet::coordinate_r(1, Dual::constant(parameter)));
            (
                jet.coefficient(0, 0)
                    .expect("the value coefficient is active")
                    .value(),
                jet.coefficient(0, 1)
                    .expect("the seam derivative is active")
                    .value(),
            )
        });

        assert!(values[0].0.abs() < 1.0e-15);
        assert!((values[4].0 - 1.0).abs() < 1.0e-15);
        assert!(values.windows(2).all(|pair| pair[0].0 < pair[1].0));
        assert!(values.iter().all(|(_, derivative)| *derivative > 0.0));
    }
}
