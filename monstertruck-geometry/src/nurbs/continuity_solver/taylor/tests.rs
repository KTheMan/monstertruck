use super::*;

const ORDER: usize = MAX_CONTINUITY_ORDER;
const EPSILON: f64 = 1.0e-12;

fn coefficient(jet: &TaylorJet<f64>, i: usize, j: usize) -> f64 {
    *jet.coefficient(i, j)
        .expect("the requested coefficient is active")
}

fn assert_near(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= EPSILON * (1.0 + expected.abs()),
        "expected {expected}, got {actual}",
    );
}

#[test]
fn factorial_normalized_product_obeys_the_product_rule() {
    let s = TaylorJet::coordinate_s(ORDER, 2.0);
    let r = TaylorJet::coordinate_r(ORDER, 3.0);
    let product = s.clone().powi(2) * r;

    assert_near(coefficient(&product, 0, 0), 12.0);
    assert_near(coefficient(&product, 1, 0), 12.0);
    assert_near(coefficient(&product, 0, 1), 4.0);
    assert_near(coefficient(&product, 2, 0), 3.0);
    assert_near(coefficient(&product, 1, 1), 4.0);
    assert_near(coefficient(&product, 2, 1), 1.0);
}

#[test]
fn reciprocal_and_division_are_formal_inverses() {
    let s = TaylorJet::coordinate_s(ORDER, 0.0);
    let r = TaylorJet::coordinate_r(ORDER, 0.0);
    let denominator = TaylorJet::constant(ORDER, 1.0) - s.clone() - r.clone();
    let reciprocal = denominator.reciprocal();
    let quotient = TaylorJet::constant(ORDER, 1.0) / denominator.clone();
    let identity = denominator * reciprocal.clone();

    (0..=ORDER).for_each(|total| {
        (0..=total).for_each(|i| {
            let j = total - i;
            let expected_reciprocal = binomial(total, i) as f64;
            assert_near(coefficient(&reciprocal, i, j), expected_reciprocal);
            assert_near(coefficient(&quotient, i, j), expected_reciprocal);
            assert_near(
                coefficient(&identity, i, j),
                if total == 0 { 1.0 } else { 0.0 },
            );
        });
    });
}

#[test]
fn exponential_composes_bivariate_coordinate_products() {
    let s = TaylorJet::coordinate_s(ORDER, 1.0);
    let r = TaylorJet::coordinate_r(ORDER, 2.0);
    let exponential = (s * r).exp();
    let exp_two = 2.0_f64.exp();

    assert_near(coefficient(&exponential, 0, 0), exp_two);
    assert_near(coefficient(&exponential, 1, 0), 2.0 * exp_two);
    assert_near(coefficient(&exponential, 0, 1), exp_two);
    assert_near(coefficient(&exponential, 1, 1), 3.0 * exp_two);
    assert_near(coefficient(&exponential, 2, 0), 2.0 * exp_two);
    assert_near(coefficient(&exponential, 0, 2), 0.5 * exp_two);
}

#[test]
fn every_operation_truncates_deterministically_at_total_order_four() {
    let s = TaylorJet::coordinate_s(ORDER, 0.0);
    let r = TaylorJet::coordinate_r(ORDER, 0.0);
    let fifth_order = (s.clone() + r.clone()).powi(5);
    let fourth_order = (s + r).powi(4);

    (0..=ORDER).for_each(|total| {
        (0..=total).for_each(|i| {
            let j = total - i;
            assert_near(coefficient(&fifth_order, i, j), 0.0);
            assert_near(
                coefficient(&fourth_order, i, j),
                if total == ORDER {
                    binomial(ORDER, i) as f64
                } else {
                    0.0
                },
            );
        });
    });
    assert_eq!(fourth_order.order(), ORDER);
    assert!(fourth_order.coefficient(5, 0).is_none());
    assert!(fourth_order.coefficient(2, 3).is_none());
}

#[test]
fn runtime_order_is_stricter_than_storage_capacity() {
    let mut jet = TaylorJet::from_coefficients(2, |i, j| (10 * i + j) as f64);
    assert_eq!(jet.order(), 2);
    assert_eq!(jet.coefficient(1, 1), Some(&11.0));
    assert!(jet.coefficient(2, 1).is_none());
    assert!(jet.coefficient_mut(3, 0).is_none());
    *jet.coefficient_mut(0, 2)
        .expect("the requested coefficient is active") = 7.0;
    assert_eq!(jet.coefficient(0, 2), Some(&7.0));
    assert_eq!(TaylorJet::coordinate_s(2, 0.0).powi(3), TaylorJet::zero(2),);
}

const fn binomial(n: usize, k: usize) -> usize {
    if k == 0 || k == n {
        1
    } else {
        binomial(n - 1, k - 1) + binomial(n - 1, k)
    }
}
