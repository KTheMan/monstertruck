//! Deterministic dense least-squares solution by column-pivoted QR.

/// Result of a dense least-squares solve.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LeastSquaresSolution {
    /// Solution in the original column order.
    pub(crate) step: Vec<f64>,
    /// Numerical rank selected by the relative rank tolerance.
    pub(crate) rank: usize,
    /// Euclidean norm of the residual in the original system.
    pub(crate) residual_norm: f64,
}

/// Solves `rows * step ~= rhs` using column-pivoted Householder QR.
///
/// `rank_tolerance` is relative to the largest initial column norm. For a
/// rank-deficient system, non-pivot variables are set to zero, yielding a
/// deterministic basic solution. Equal-norm pivot candidates are ordered by
/// their original column index.
pub(crate) fn solve_column_pivoted(
    rows: &[Vec<f64>],
    rhs: &[f64],
    rank_tolerance: f64,
) -> Option<LeastSquaresSolution> {
    let row_count = rows.len();
    let column_count = rows.first()?.len();
    if rhs.len() != row_count
        || !rank_tolerance.is_finite()
        || rank_tolerance < 0.0
        || rows
            .iter()
            .any(|row| row.len() != column_count || row.iter().any(|value| !value.is_finite()))
        || rhs.iter().any(|value| !value.is_finite())
    {
        return None;
    }

    let mut matrix = rows.to_vec();
    let mut transformed_rhs = rhs.to_vec();
    let mut permutation: Vec<usize> = (0..column_count).collect();
    let mut rank = 0;
    let mut leading_norm = 0.0;

    for diagonal in 0..row_count.min(column_count) {
        let pivot = (diagonal..column_count).try_fold(diagonal, |best, candidate| {
            let best_norm = column_norm(&matrix, diagonal, best)?;
            let candidate_norm = column_norm(&matrix, diagonal, candidate)?;
            Some(
                if candidate_norm > best_norm
                    || (candidate_norm == best_norm && permutation[candidate] < permutation[best])
                {
                    candidate
                } else {
                    best
                },
            )
        })?;
        let pivot_norm = column_norm(&matrix, diagonal, pivot)?;

        if diagonal == 0 {
            leading_norm = pivot_norm;
        }
        let rank_threshold = rank_tolerance * leading_norm;
        if !rank_threshold.is_finite() {
            return None;
        }
        if pivot_norm <= rank_threshold {
            break;
        }

        matrix.iter_mut().for_each(|row| row.swap(diagonal, pivot));
        permutation.swap(diagonal, pivot);

        let first = matrix[diagonal][diagonal];
        let sign = if first.is_sign_negative() { -1.0 } else { 1.0 };
        let reflector: Vec<f64> = matrix[diagonal..]
            .iter()
            .map(|row| row[diagonal] / pivot_norm)
            .enumerate()
            .map(|(index, value)| if index == 0 { value + sign } else { value })
            .collect();
        let reflector_norm = vector_norm(reflector.iter().copied())?;
        let reflector: Vec<f64> = reflector
            .into_iter()
            .map(|value| value / reflector_norm)
            .collect();

        for column in diagonal + 1..column_count {
            let projection = 2.0
                * compensated_sum(
                    reflector
                        .iter()
                        .zip(matrix[diagonal..].iter())
                        .map(|(coefficient, row)| coefficient * row[column]),
                )?;
            matrix[diagonal..]
                .iter_mut()
                .zip(reflector.iter())
                .for_each(|(row, coefficient)| row[column] -= projection * coefficient);
        }

        let rhs_projection = 2.0
            * compensated_sum(
                reflector
                    .iter()
                    .zip(transformed_rhs[diagonal..].iter())
                    .map(|(coefficient, value)| coefficient * value),
            )?;
        transformed_rhs[diagonal..]
            .iter_mut()
            .zip(reflector.iter())
            .for_each(|(value, coefficient)| *value -= rhs_projection * coefficient);

        matrix[diagonal][diagonal] = -sign * pivot_norm;
        matrix[diagonal + 1..]
            .iter_mut()
            .for_each(|row| row[diagonal] = 0.0);
        rank += 1;
    }

    if matrix
        .iter()
        .flatten()
        .chain(transformed_rhs.iter())
        .any(|value| !value.is_finite())
    {
        return None;
    }

    let mut pivoted_step = vec![0.0; column_count];
    for row in (0..rank).rev() {
        let known = compensated_sum(
            matrix[row][row + 1..rank]
                .iter()
                .zip(pivoted_step[row + 1..rank].iter())
                .map(|(coefficient, value)| coefficient * value),
        )?;
        pivoted_step[row] = (transformed_rhs[row] - known) / matrix[row][row];
        if !pivoted_step[row].is_finite() {
            return None;
        }
    }

    let mut step = vec![0.0; column_count];
    permutation
        .iter()
        .zip(pivoted_step)
        .for_each(|(&original_column, value)| step[original_column] = value);
    let residual_norm = rows
        .iter()
        .zip(rhs)
        .try_fold((0.0, 1.0), |norm_state, (row, expected)| {
            let actual = compensated_sum(
                row.iter()
                    .zip(step.iter())
                    .map(|(coefficient, value)| coefficient * value),
            )?;
            accumulate_norm(norm_state, actual - expected)
        })
        .and_then(finish_norm)?;

    Some(LeastSquaresSolution {
        step,
        rank,
        residual_norm,
    })
}

fn column_norm(matrix: &[Vec<f64>], start_row: usize, column: usize) -> Option<f64> {
    vector_norm(matrix[start_row..].iter().map(|row| row[column]))
}

fn compensated_sum(mut values: impl Iterator<Item = f64>) -> Option<f64> {
    values
        .try_fold((0.0, 0.0), |(sum, correction), value| {
            let adjusted = value - correction;
            let next = sum + adjusted;
            let next_correction = (next - sum) - adjusted;
            if next.is_finite() && next_correction.is_finite() {
                Some((next, next_correction))
            } else {
                None
            }
        })
        .map(|(sum, _)| sum)
}

fn vector_norm(mut values: impl Iterator<Item = f64>) -> Option<f64> {
    values
        .try_fold((0.0, 1.0), accumulate_norm)
        .and_then(finish_norm)
}

fn accumulate_norm((scale, sum_squares): (f64, f64), value: f64) -> Option<(f64, f64)> {
    let absolute = value.abs();
    if !absolute.is_finite() {
        None
    } else if absolute == 0.0 {
        Some((scale, sum_squares))
    } else if scale < absolute {
        let ratio = scale / absolute;
        Some((absolute, 1.0 + sum_squares * ratio * ratio))
    } else {
        let ratio = absolute / scale;
        Some((scale, sum_squares + ratio * ratio))
    }
}

fn finish_norm((scale, sum_squares): (f64, f64)) -> Option<f64> {
    let norm = if scale == 0.0 {
        0.0
    } else {
        scale * sum_squares.sqrt()
    };
    norm.is_finite().then_some(norm)
}

#[cfg(test)]
mod tests;
