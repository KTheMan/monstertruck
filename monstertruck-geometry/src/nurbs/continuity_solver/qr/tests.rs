use super::*;

const TOLERANCE: f64 = 1.0e-12;

fn assert_near(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= TOLERANCE,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn solves_full_rank_system() {
    let solution = solve_column_pivoted(&[vec![2.0, 1.0], vec![1.0, 3.0]], &[5.0, 7.0], TOLERANCE)
        .expect("the finite full-rank system should solve");

    assert_eq!(solution.rank, 2);
    assert_near(solution.step[0], 1.6);
    assert_near(solution.step[1], 1.8);
    assert_near(solution.residual_norm, 0.0);
}

#[test]
fn pivots_before_solving() {
    let solution = solve_column_pivoted(
        &[vec![0.0, 1.0], vec![1.0e-10, 0.0]],
        &[2.0, 1.0e-10],
        1.0e-12,
    )
    .expect("the scaled full-rank system should solve");

    assert_eq!(solution.rank, 2);
    assert_near(solution.step[0], 1.0);
    assert_near(solution.step[1], 2.0);
}

#[test]
fn minimizes_overdetermined_residual() {
    let solution = solve_column_pivoted(
        &[vec![1.0, 0.0], vec![0.0, 1.0], vec![1.0, 1.0]],
        &[1.0, 2.0, 4.0],
        TOLERANCE,
    )
    .expect("the overdetermined system should solve");

    assert_eq!(solution.rank, 2);
    assert_near(solution.step[0], 4.0 / 3.0);
    assert_near(solution.step[1], 7.0 / 3.0);
    assert_near(solution.residual_norm, (1.0_f64 / 3.0).sqrt());
}

#[test]
fn returns_deterministic_basic_rank_deficient_solution() {
    let solution = solve_column_pivoted(
        &[vec![1.0, 1.0], vec![2.0, 2.0], vec![3.0, 3.0]],
        &[1.0, 2.0, 3.0],
        TOLERANCE,
    )
    .expect("the consistent rank-deficient system should solve");

    assert_eq!(solution.rank, 1);
    assert_near(solution.step[0], 1.0);
    assert_near(solution.step[1], 0.0);
    assert_near(solution.residual_norm, 0.0);
}

#[test]
fn repeated_runs_are_bit_deterministic() {
    let rows = [
        vec![1.0, 2.0, -1.0],
        vec![2.0, -1.0, 3.0],
        vec![0.5, 4.0, 2.0],
        vec![3.0, 0.25, 1.0],
    ];
    let rhs = [2.0, -1.0, 5.0, 0.5];
    let expected =
        solve_column_pivoted(&rows, &rhs, TOLERANCE).expect("the reference solve should work");

    (0..32).for_each(|_| {
        let actual =
            solve_column_pivoted(&rows, &rhs, TOLERANCE).expect("the repeated solve should work");
        assert_eq!(actual.rank, expected.rank);
        assert_eq!(
            actual
                .step
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>(),
            expected
                .step
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            actual.residual_norm.to_bits(),
            expected.residual_norm.to_bits()
        );
    });
}

#[test]
fn rejects_invalid_inputs() {
    assert!(solve_column_pivoted(&[], &[], TOLERANCE).is_none());
    assert!(solve_column_pivoted(&[vec![1.0]], &[], TOLERANCE).is_none());
    assert!(solve_column_pivoted(&[vec![1.0], vec![1.0, 2.0]], &[1.0, 2.0], TOLERANCE).is_none());
    assert!(solve_column_pivoted(&[vec![f64::NAN]], &[1.0], TOLERANCE).is_none());
    assert!(solve_column_pivoted(&[vec![1.0]], &[1.0], -1.0).is_none());
}
