//! Validated dimensions, sampling, and initial-state preparation.

use super::*;

impl<'surface> PreparedProblem<'surface> {
    pub(in crate::nurbs::continuity_solver) fn new(
        first: &'surface NurbsSurface<Vector4>,
        second: &'surface NurbsSurface<Vector4>,
        request: BoundaryContinuityRequest,
        config: &ContinuitySolverConfig,
        budget: ContinuityBudget,
    ) -> Result<Self, ContinuitySolveError> {
        config.validate()?;
        if request.order() == ContinuityOrder::G4 && !config.allows_experimental_g4() {
            return Err(ContinuitySolveError::ExperimentalG4Disabled);
        }
        let first_frame = BoundaryFrame::try_new(first, request.first_side())
            .map_err(|_| ContinuitySolveError::InvalidBoundary(BoundaryEndpoint::First))?;
        let second_frame = BoundaryFrame::try_new(second, request.second_side())
            .map_err(|_| ContinuitySolveError::InvalidBoundary(BoundaryEndpoint::Second))?;
        validate_capability(
            first,
            request.first_side(),
            request.order(),
            BoundaryEndpoint::First,
        )?;
        validate_capability(
            second,
            request.second_side(),
            request.order(),
            BoundaryEndpoint::Second,
        )?;
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
        budget.ensure(ContinuityResource::ControlPoints, control_point_count)?;
        validate_weights(first, BoundaryEndpoint::First, config.minimum_weight())?;
        validate_weights(second, BoundaryEndpoint::Second, config.minimum_weight())?;
        let first_spans = frame_span_count(first, first_frame, BoundaryEndpoint::First)?;
        let second_spans = frame_span_count(second, second_frame, BoundaryEndpoint::Second)?;
        let span_count = checked_add(first_spans, second_spans, "seam span count overflowed")?;
        budget.ensure(ContinuityResource::Spans, span_count)?;
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
        budget.ensure(
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
                        BoundaryAlignment::Aligned => seam,
                        BoundaryAlignment::Reversed => 1.0 - seam,
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
                            BoundaryAlignment::Aligned => seam,
                            BoundaryAlignment::Reversed => 1.0 - seam,
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
        budget.ensure(
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
        budget.ensure(ContinuityResource::Variables, variable_count)?;
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
        budget.ensure(
            ContinuityResource::Residuals,
            checked_add(
                optimizer_residuals,
                validation_residuals,
                "total residual dimension overflowed",
            )?,
        )?;
        budget.ensure(
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
            first,
            second,
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
}
