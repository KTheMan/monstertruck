//! Deterministic seam collocation.

use super::super::KnotVector;
use std::f64::consts::PI;

pub(super) fn nonzero_span_count(
    knots: &KnotVector,
    degree: usize,
    control_count: usize,
) -> Option<usize> {
    if control_count == 0 || degree >= knots.len() || control_count >= knots.len() {
        None
    } else {
        let start = knots[degree];
        let end = knots[control_count];
        if !start.is_finite() || !end.is_finite() || start >= end {
            None
        } else {
            let count = knots[degree..=control_count]
                .windows(2)
                .filter(|span| span[0].is_finite() && span[1].is_finite() && span[0] < span[1])
                .count();
            (count > 0).then_some(count)
        }
    }
}

pub(super) fn seam_samples(
    knots: &KnotVector,
    degree: usize,
    control_count: usize,
    samples_per_span: usize,
) -> Option<Vec<f64>> {
    if samples_per_span == 0 || control_count == 0 || degree >= knots.len() {
        None
    } else {
        let start = knots[degree];
        let end = knots[control_count];
        if !start.is_finite() || !end.is_finite() || start >= end {
            None
        } else {
            let nodes = gauss_legendre_nodes(samples_per_span)?;
            let spans: Vec<_> = knots[degree..=control_count]
                .windows(2)
                .filter_map(|span| {
                    let first = span[0];
                    let second = span[1];
                    (first.is_finite() && second.is_finite() && first < second)
                        .then_some((first, second))
                })
                .collect();
            (!spans.is_empty()).then(|| {
                let mut samples = spans
                    .iter()
                    .flat_map(|&(first, second)| {
                        nodes.iter().map(move |&node| {
                            let parameter = 0.5 * ((second - first).mul_add(node, second + first));
                            (parameter - start) / (end - start)
                        })
                    })
                    .chain(spans.iter().flat_map(|&(first, second)| {
                        [
                            (first - start) / (end - start),
                            (second - start) / (end - start),
                        ]
                    }))
                    .collect::<Vec<_>>();
                samples.sort_by(f64::total_cmp);
                samples.dedup_by(|first, second| first == second);
                samples
            })
        }
    }
}

pub(super) fn seam_validation_samples(
    knots: &KnotVector,
    degree: usize,
    control_count: usize,
    samples_per_span: usize,
) -> Option<Vec<f64>> {
    if samples_per_span == 0 || control_count == 0 || degree >= knots.len() {
        None
    } else {
        let start = knots[degree];
        let end = knots[control_count];
        if !start.is_finite() || !end.is_finite() || start >= end {
            None
        } else {
            let samples = knots[degree..=control_count]
                .windows(2)
                .filter_map(|span| {
                    let first = span[0];
                    let second = span[1];
                    (first.is_finite() && second.is_finite() && first < second)
                        .then_some((first, second))
                })
                .flat_map(|(first, second)| {
                    (0..samples_per_span).map(move |index| {
                        let fraction = (index as f64 + 0.5) / samples_per_span as f64;
                        (first + fraction * (second - first) - start) / (end - start)
                    })
                })
                .collect::<Vec<_>>();
            (!samples.is_empty()).then_some(samples)
        }
    }
}

fn gauss_legendre_nodes(count: usize) -> Option<Vec<f64>> {
    if count == 0 {
        None
    } else {
        let mut nodes = vec![0.0; count];
        let half = count.div_ceil(2);
        (0..half).for_each(|index| {
            let mut root = (PI * (index as f64 + 0.75) / (count as f64 + 0.5)).cos();
            (0..24).for_each(|_| {
                let (value, derivative) = legendre_value_derivative(count, root);
                root -= value / derivative;
            });
            nodes[index] = -root;
            nodes[count - index - 1] = root;
        });
        nodes.iter().all(|node| node.is_finite()).then_some(nodes)
    }
}

fn legendre_value_derivative(order: usize, x: f64) -> (f64, f64) {
    let (previous, value) = (1..=order).fold((1.0, x), |(before, current), degree| {
        if degree == 1 {
            (before, current)
        } else {
            (
                current,
                ((2 * degree - 1) as f64 * x * current - (degree - 1) as f64 * before)
                    / degree as f64,
            )
        }
    });
    let derivative = order as f64 * (previous - x * value) / (1.0 - x * x);
    (value, derivative)
}

#[cfg(test)]
mod tests;
