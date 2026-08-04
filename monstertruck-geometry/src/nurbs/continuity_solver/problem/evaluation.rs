//! Residual, Jacobian, solution, and transition evaluation.

use super::transition::{
    aligned_seam_jet, bernstein_field, compose_surface, monotone_seam_map, parameter_jets,
    physical_control_scalar,
};
use super::*;
use crate::nurbs::continuity_solver::resource::{ContinuityWork, charge_continuity_work};

impl PreparedProblem<'_> {
    pub(in crate::nurbs::continuity_solver) fn initial_variables(&self) -> &[f64] {
        &self.initial_variables
    }

    pub(in crate::nurbs::continuity_solver) fn variable_count(&self) -> usize {
        self.initial_variables.len()
    }

    pub(in crate::nurbs::continuity_solver) const fn qr_elements(&self) -> usize {
        self.qr_elements
    }

    pub(in crate::nurbs::continuity_solver) fn solved_second(
        &self,
        variables: &[f64],
    ) -> NurbsSurface<Vector4> {
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

    pub(in crate::nurbs::continuity_solver) fn solved_transition(
        &self,
        variables: &[f64],
    ) -> BoundaryTransition {
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

    pub(in crate::nurbs::continuity_solver) fn evaluate(
        &self,
        variables: &[f64],
        config: &ContinuitySolverConfig,
        with_jacobian: bool,
    ) -> Result<ResidualEvaluation, ContinuitySolveError> {
        self.evaluate_samples(variables, config, with_jacobian, &self.samples, true)
    }

    pub(in crate::nurbs::continuity_solver) fn validation_residuals(
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
                let first_value = compose_surface(self.first, None, &first_u, &first_v);
                let second_value = compose_surface(
                    self.second,
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
                            // SAFETY: Both jets were constructed at the requested total order.
                            let first = first_value[coordinate]
                                .coefficient(cross_order, seam_order)
                                .expect("the requested coefficient is active")
                                .clone();
                            // SAFETY: Both jets were constructed at the requested total order.
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
                        // SAFETY: `total` is bounded by the validated kernel order.
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
            charge_continuity_work(ContinuityWork {
                jacobian_elements: residuals.len().saturating_mul(variable_count) as u64,
                ..ContinuityWork::default()
            });
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
                            // SAFETY: The strip and seam ranges were validated during preparation.
                            self.second_frame
                                .control_point_index(offset, seam)
                                .expect("the prepared strip index is valid")
                        });
                        let current = physical_control_scalar(
                            self.second,
                            &self.control_offsets,
                            variables,
                            indices[0],
                            coordinate,
                        ) - physical_control_scalar(
                            self.second,
                            &self.control_offsets,
                            variables,
                            indices[1],
                            coordinate,
                        ) * Dual::constant(2.0)
                            + physical_control_scalar(
                                self.second,
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
