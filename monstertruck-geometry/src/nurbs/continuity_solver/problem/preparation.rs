//! Validated dimensions, sampling, and initial-state preparation.

use super::*;

impl<'surface> PreparedProblem<'surface> {
    pub(in crate::nurbs::continuity_solver) fn new(
        first: &'surface NurbsSurface<Vector4>,
        second: &'surface NurbsSurface<Vector4>,
        request: BoundaryContinuityRequest,
        config: &ContinuitySolverConfig,
        budget: ContinuityLimits,
    ) -> Result<Self, ContinuitySolveError> {
        config.validate()?;
        if request.order() == ContinuityOrder::G4 && !config.allows_experimental_g4() {
            return Err(ContinuitySolveError::ExperimentalG4Disabled);
        }
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
        validate_coordinates(first, BoundaryEndpoint::First)?;
        validate_coordinates(second, BoundaryEndpoint::Second)?;
        let first_frame = BoundaryFrame::try_new(first, request.first_side())
            .map_err(|_| ContinuitySolveError::InvalidBoundary(BoundaryEndpoint::First))?;
        let second_frame = BoundaryFrame::try_new(second, request.second_side())
            .map_err(|_| ContinuitySolveError::InvalidBoundary(BoundaryEndpoint::Second))?;
        validate_along_knot_continuity(
            first,
            first_frame,
            request.order(),
            BoundaryEndpoint::First,
        )?;
        validate_along_knot_continuity(
            second,
            second_frame,
            request.order(),
            BoundaryEndpoint::Second,
        )?;
        let control_point_count = checked_add(
            checked_mul(
                first_frame.control_count_u(),
                first_frame.control_count_v(),
                "surface control-point dimension overflowed",
            )?,
            checked_mul(
                second_frame.control_count_u(),
                second_frame.control_count_v(),
                "surface control-point dimension overflowed",
            )?,
            "surface control-point dimension overflowed",
        )?;
        budget.ensure_dimension(ContinuityResource::ControlPoints, control_point_count)?;
        let first_spans = frame_span_count(first, first_frame, BoundaryEndpoint::First)?;
        let second_spans = frame_span_count(second, second_frame, BoundaryEndpoint::Second)?;
        let span_count = checked_add(first_spans, second_spans, "seam span count overflowed")?;
        budget.ensure_dimension(ContinuityResource::Spans, span_count)?;
        let validation_density = validation_density(first_frame, second_frame, request, config)?;
        [first_spans, second_spans]
            .into_iter()
            .try_for_each(|spans| {
                budget.ensure_dimension(
                    ContinuityResource::Samples,
                    optimizer_frame_sample_count(spans, config.samples_per_span())?,
                )?;
                budget.ensure_dimension(
                    ContinuityResource::Samples,
                    checked_mul(
                        spans,
                        validation_density,
                        "validation sample count overflowed",
                    )?,
                )
            })?;
        let first_samples = frame_samples(first, first_frame, config.samples_per_span());
        let second_samples = aligned_samples(
            frame_samples(second, second_frame, config.samples_per_span()),
            request.alignment(),
        );
        let sample_count = merged_unique_count(&first_samples, &second_samples);
        if sample_count == 0 {
            return Err(ContinuitySolveError::InvalidBoundary(
                BoundaryEndpoint::First,
            ));
        }
        let first_validation = frame_validation_samples(first, first_frame, validation_density);
        let second_validation = aligned_samples(
            frame_validation_samples(second, second_frame, validation_density),
            request.alignment(),
        );
        let validation_sample_count = merged_unique_count_excluding(
            &first_validation,
            &second_validation,
            &first_samples,
            &second_samples,
        );
        if validation_sample_count == 0 {
            return Err(ContinuitySolveError::InvalidBoundary(
                BoundaryEndpoint::First,
            ));
        }
        budget.ensure_dimension(
            ContinuityResource::Samples,
            checked_add(
                sample_count,
                validation_sample_count,
                "total sample count overflowed",
            )?,
        )?;
        let samples = merged_unique_collect(&first_samples, &second_samples, sample_count);
        let validation_samples = merged_unique_collect(
            &first_validation,
            &second_validation,
            merged_unique_count(&first_validation, &second_validation),
        )
        .into_iter()
        .filter(|candidate| {
            samples
                .binary_search_by(|sample| sample.total_cmp(candidate))
                .is_err()
        })
        .collect::<Vec<_>>();

        let strip_rows = (request.order().as_usize() + 3).min(second_frame.cross_control_count());
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
        budget.ensure_dimension(ContinuityResource::Variables, variable_count)?;
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
        budget.ensure_dimension(
            ContinuityResource::Residuals,
            checked_add(
                optimizer_residuals,
                validation_residuals,
                "total residual dimension overflowed",
            )?,
        )?;
        let jacobian_elements = checked_mul(
            optimizer_residuals,
            variable_count,
            "Jacobian dimension overflowed",
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
            BoundaryAlignment::Aligned,
            BoundaryEndpoint::First,
            &samples,
            characteristic_length,
        )?;
        validate_regular_boundary(
            second,
            second_frame,
            request.alignment(),
            BoundaryEndpoint::Second,
            &samples,
            characteristic_length,
        )?;

        let control_indices = second_frame
            .control_strip_indices(strip_rows)
            .map_err(|_| ContinuitySolveError::InvalidBoundary(BoundaryEndpoint::Second))?;
        let mut control_offsets =
            vec![vec![None; second_frame.control_count_v()]; second_frame.control_count_u()];
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
            jacobian_elements,
            qr_elements,
        })
    }
}

fn optimizer_frame_sample_count(
    spans: usize,
    samples_per_span: usize,
) -> Result<usize, ContinuitySolveError> {
    checked_add(
        checked_mul(
            spans,
            checked_add(samples_per_span, 1, "optimizer sample density overflowed")?,
            "optimizer sample count overflowed",
        )?,
        1,
        "optimizer sample count overflowed",
    )
}

fn aligned_samples(mut samples: Vec<f64>, alignment: BoundaryAlignment) -> Vec<f64> {
    if alignment == BoundaryAlignment::Reversed {
        samples
            .iter_mut()
            .for_each(|sample| *sample = 1.0 - *sample);
        samples.sort_by(f64::total_cmp);
        samples.dedup_by(|first, second| first == second);
    }
    samples
}

fn merged_unique_count(first: &[f64], second: &[f64]) -> usize {
    let mut count = 0;
    for_each_merged_unique(first, second, |_| count += 1);
    count
}

fn merged_unique_count_excluding(
    first: &[f64],
    second: &[f64],
    excluded_first: &[f64],
    excluded_second: &[f64],
) -> usize {
    let mut count = 0;
    for_each_merged_unique(first, second, |sample| {
        let excluded = excluded_first
            .binary_search_by(|candidate| candidate.total_cmp(&sample))
            .is_ok()
            || excluded_second
                .binary_search_by(|candidate| candidate.total_cmp(&sample))
                .is_ok();
        count += usize::from(!excluded);
    });
    count
}

fn merged_unique_collect(first: &[f64], second: &[f64], count: usize) -> Vec<f64> {
    let mut samples = Vec::with_capacity(count);
    for_each_merged_unique(first, second, |sample| samples.push(sample));
    samples
}

fn for_each_merged_unique(first: &[f64], second: &[f64], mut visit: impl FnMut(f64)) {
    let (mut first_index, mut second_index, mut previous) = (0, 0, None);
    while first_index < first.len() || second_index < second.len() {
        let take_first = second_index == second.len()
            || (first_index < first.len()
                && first[first_index].total_cmp(&second[second_index]).is_le());
        let sample = if take_first {
            let sample = first[first_index];
            first_index += 1;
            sample
        } else {
            let sample = second[second_index];
            second_index += 1;
            sample
        };
        if previous != Some(sample) {
            visit(sample);
            previous = Some(sample);
        }
    }
}
