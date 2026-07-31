//! Deterministic damped least-squares iteration.

use super::problem::{PreparedProblem, ResidualEvaluation};
use super::qr::solve_column_pivoted;
use super::resource::ContinuityResourceBudget;
use super::types::{
    BoundaryContinuityRequest, BoundaryContinuitySolution, ContinuityResource,
    ContinuitySolveError, ContinuitySolveReport, ContinuitySolveReportData, ContinuitySolverConfig,
    ContinuityTermination, OrderResidual,
};
use crate::base::Vector4;
use crate::nurbs::NurbsSurface;

pub(super) fn solve(
    first: &NurbsSurface<Vector4>,
    second: &NurbsSurface<Vector4>,
    request: BoundaryContinuityRequest,
    config: &ContinuitySolverConfig,
    resource_budget: ContinuityResourceBudget,
) -> Result<BoundaryContinuitySolution, ContinuitySolveError> {
    let problem = PreparedProblem::new(first, second, request, config, resource_budget)?;
    let mut variables = problem.initial_variables().to_vec();
    let mut evaluation = problem.evaluate(&variables, config, true)?;
    let initial_objective = evaluation.objective;
    let mut damping = config.initial_damping();
    let mut accepted_steps = 0;
    let mut rejected_steps = 0;
    let mut last_step_norm = 0.0;
    let mut numerical_rank = 0;

    if let Some(residuals) =
        certified_residuals(&problem, &variables, &evaluation, request, config)?
    {
        let report = report(
            ContinuityTermination::Converged,
            request,
            0,
            accepted_steps,
            rejected_steps,
            initial_objective,
            &evaluation,
            &residuals,
            last_step_norm,
            damping,
            numerical_rank,
            problem.variable_count(),
        );
        return Ok(BoundaryContinuitySolution::new(
            problem.first().clone(),
            problem.solved_second(&variables),
            report,
        ));
    }

    resource_budget.ensure(ContinuityResource::QrElements, problem.qr_elements())?;
    for iteration in 1..=config.max_iterations() {
        let (rows, rhs) = augmented_system(&evaluation, damping, problem.variable_count());
        let least_squares = solve_column_pivoted(&rows, &rhs, config.rank_tolerance())
            .ok_or(ContinuitySolveError::NoDescentDirection)?;
        numerical_rank = least_squares.rank;
        last_step_norm = stable_norm(&least_squares.step);
        if !last_step_norm.is_finite() || !least_squares.residual_norm.is_finite() {
            return Err(ContinuitySolveError::NoDescentDirection);
        }

        let trial_variables = variables
            .iter()
            .zip(&least_squares.step)
            .map(|(&value, &step)| value + step)
            .collect::<Vec<_>>();
        let trial = match problem.evaluate(&trial_variables, config, false) {
            Ok(trial) => Some(trial),
            Err(ContinuitySolveError::NonFiniteResidual) => None,
            Err(error) => return Err(error),
        };
        if trial
            .as_ref()
            .is_some_and(|trial| trial.objective < evaluation.objective)
        {
            variables = trial_variables;
            evaluation = problem.evaluate(&variables, config, true)?;
            accepted_steps += 1;
            damping = (damping / 3.0).max(config.minimum_damping());
            if let Some(residuals) =
                certified_residuals(&problem, &variables, &evaluation, request, config)?
            {
                let report = report(
                    ContinuityTermination::Converged,
                    request,
                    iteration,
                    accepted_steps,
                    rejected_steps,
                    initial_objective,
                    &evaluation,
                    &residuals,
                    last_step_norm,
                    damping,
                    numerical_rank,
                    problem.variable_count(),
                );
                return Ok(BoundaryContinuitySolution::new(
                    problem.first().clone(),
                    problem.solved_second(&variables),
                    report,
                ));
            }
        } else {
            rejected_steps += 1;
            damping = (damping * 10.0).min(config.maximum_damping());
            if damping == config.maximum_damping() {
                let residuals = combined_residuals(&problem, &variables, &evaluation, config)?;
                let report = report(
                    ContinuityTermination::NoDescent,
                    request,
                    iteration,
                    accepted_steps,
                    rejected_steps,
                    initial_objective,
                    &evaluation,
                    &residuals,
                    last_step_norm,
                    damping,
                    numerical_rank,
                    problem.variable_count(),
                );
                return Err(ContinuitySolveError::DidNotConverge(Box::new(report)));
            }
        }
    }

    let residuals = combined_residuals(&problem, &variables, &evaluation, config)?;
    let report = report(
        ContinuityTermination::MaximumIterations,
        request,
        config.max_iterations(),
        accepted_steps,
        rejected_steps,
        initial_objective,
        &evaluation,
        &residuals,
        last_step_norm,
        damping,
        numerical_rank,
        problem.variable_count(),
    );
    Err(ContinuitySolveError::DidNotConverge(Box::new(report)))
}

fn augmented_system(
    evaluation: &ResidualEvaluation,
    damping: f64,
    variable_count: usize,
) -> (Vec<Vec<f64>>, Vec<f64>) {
    let mut rows = evaluation.jacobian.clone();
    let mut rhs = evaluation
        .values
        .iter()
        .map(|&value| -value)
        .collect::<Vec<_>>();
    let damping_scale = damping.sqrt();
    rows.extend((0..variable_count).map(|column| {
        let mut row = vec![0.0; variable_count];
        row[column] = damping_scale;
        row
    }));
    rhs.resize(rhs.len() + variable_count, 0.0);
    (rows, rhs)
}

fn tolerances_met(
    residuals: &[OrderResidual],
    request: BoundaryContinuityRequest,
    config: &ContinuitySolverConfig,
) -> bool {
    residuals.iter().all(|residual| {
        residual.order() <= request.order()
            && residual.maximum() <= config.tolerance(residual.order())
    })
}

fn certified_residuals(
    problem: &PreparedProblem,
    variables: &[f64],
    evaluation: &ResidualEvaluation,
    request: BoundaryContinuityRequest,
    config: &ContinuitySolverConfig,
) -> Result<Option<Vec<OrderResidual>>, ContinuitySolveError> {
    if !tolerances_met(&evaluation.residuals, request, config) {
        Ok(None)
    } else {
        let merged = combined_residuals(problem, variables, evaluation, config)?;
        Ok(tolerances_met(&merged, request, config).then_some(merged))
    }
}

fn combined_residuals(
    problem: &PreparedProblem,
    variables: &[f64],
    evaluation: &ResidualEvaluation,
    config: &ContinuitySolverConfig,
) -> Result<Vec<OrderResidual>, ContinuitySolveError> {
    problem
        .validation_residuals(variables, config)
        .map(|validation| merge_residuals(&evaluation.residuals, &validation))
}

fn merge_residuals(
    collocation: &[OrderResidual],
    validation: &[OrderResidual],
) -> Vec<OrderResidual> {
    collocation
        .iter()
        .copied()
        .zip(validation.iter().copied())
        .map(|(collocation, validation)| {
            let validation_is_worst = validation.maximum() > collocation.maximum();
            let worst = if validation_is_worst {
                validation
            } else {
                collocation
            };
            OrderResidual::new(
                collocation.order(),
                collocation.rms().max(validation.rms()),
                worst.maximum(),
                worst.worst_sample(),
                validation_is_worst,
                worst.cross_derivative(),
                worst.seam_derivative(),
            )
        })
        .collect()
}

fn gradient_infinity_norm(evaluation: &ResidualEvaluation) -> f64 {
    let variable_count = evaluation.jacobian.first().map_or(0, |row| row.len());
    (0..variable_count)
        .map(|column| {
            evaluation
                .jacobian
                .iter()
                .zip(&evaluation.values)
                .map(|(row, &residual)| row[column] * residual)
                .sum::<f64>()
                .abs()
        })
        .fold(0.0, f64::max)
}

#[allow(clippy::too_many_arguments)]
fn report(
    termination: ContinuityTermination,
    request: BoundaryContinuityRequest,
    iterations: usize,
    accepted_steps: usize,
    rejected_steps: usize,
    initial_objective: f64,
    evaluation: &ResidualEvaluation,
    residuals: &[OrderResidual],
    step_norm: f64,
    damping: f64,
    numerical_rank: usize,
    variable_count: usize,
) -> ContinuitySolveReport {
    ContinuitySolveReport::from_data(ContinuitySolveReportData {
        termination,
        order: request.order(),
        iterations,
        accepted_steps,
        rejected_steps,
        initial_objective,
        final_objective: evaluation.objective,
        residuals: residuals.to_vec(),
        gradient_infinity_norm: gradient_infinity_norm(evaluation),
        step_norm,
        damping,
        numerical_rank,
        variable_count,
        residual_count: evaluation.values.len(),
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nurbs::continuity::{ContinuityOrder, SurfaceBoundary};
    use crate::nurbs::contract::BoundaryAlignment;

    #[test]
    fn independent_validation_can_veto_collocation_convergence() {
        let config = ContinuitySolverConfig::default();
        let collocation = [OrderResidual::new(
            ContinuityOrder::G0,
            1.0e-12,
            1.0e-12,
            0,
            false,
            0,
            0,
        )];
        let validation = [OrderResidual::new(
            ContinuityOrder::G0,
            2.0e-8,
            2.0e-8,
            4,
            false,
            0,
            0,
        )];
        let merged = merge_residuals(&collocation, &validation);

        assert!(tolerances_met(
            &collocation,
            BoundaryContinuityRequest::new(
                SurfaceBoundary::UEnd,
                SurfaceBoundary::UStart,
                BoundaryAlignment::Aligned,
                ContinuityOrder::G0,
            ),
            &config,
        ));
        assert!(!tolerances_met(
            &merged,
            BoundaryContinuityRequest::new(
                SurfaceBoundary::UEnd,
                SurfaceBoundary::UStart,
                BoundaryAlignment::Aligned,
                ContinuityOrder::G0,
            ),
            &config,
        ));
        assert!(merged[0].is_validation_sample());
        assert_eq!(merged[0].worst_sample(), 4);
    }
}
