use super::*;

#[test]
fn quotient_and_exponential_propagate_exact_derivatives() {
    let x = Dual::variable(2.0, 0, 2);
    let y = Dual::variable(3.0, 1, 2);
    let result = (x.clone() / y.clone()) + (x * y).exp();
    let expected_value = 2.0 / 3.0 + 6.0_f64.exp();

    assert!((result.value() - expected_value).abs() < 1.0e-12);
    assert!((result.gradient()[0] - (1.0 / 3.0 + 3.0 * 6.0_f64.exp())).abs() < 1.0e-10);
    assert!((result.gradient()[1] - (-2.0 / 9.0 + 2.0 * 6.0_f64.exp())).abs() < 1.0e-10);
}

#[test]
fn scalar_constants_broadcast_to_variable_dimensions() {
    let variable = Dual::variable(4.0, 1, 3);
    let result = variable + Dual::constant(2.0);

    assert_eq!(result.value(), 6.0);
    assert_eq!(result.gradient(), &[0.0, 1.0, 0.0]);
}
