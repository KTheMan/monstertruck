//! Dynamic forward derivatives for nonlinear seam-transition variables.

use super::taylor::JetScalar;
use std::ops::{Add, Div, Mul, Sub};

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Dual {
    value: f64,
    gradient: Vec<f64>,
}

impl Dual {
    pub(super) fn constant(value: f64) -> Self {
        Self {
            value,
            gradient: Vec::new(),
        }
    }

    pub(super) fn variable(value: f64, index: usize, variable_count: usize) -> Self {
        Self {
            value,
            gradient: (0..variable_count)
                .map(|column| usize::from(column == index) as f64)
                .collect(),
        }
    }

    pub(super) const fn value(&self) -> f64 {
        self.value
    }

    pub(super) fn gradient(&self) -> &[f64] {
        &self.gradient
    }

    fn gradient_len(&self, other: &Self) -> usize {
        match (self.gradient.len(), other.gradient.len()) {
            (0, count) | (count, 0) => count,
            (first, second) => {
                assert_eq!(
                    first, second,
                    "dual operations require equal nonconstant gradient dimensions",
                );
                first
            }
        }
    }

    fn derivative(&self, index: usize) -> f64 {
        self.gradient.get(index).copied().unwrap_or(0.0)
    }
}

impl Add for Dual {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        let count = self.gradient_len(&other);
        Self {
            value: self.value + other.value,
            gradient: (0..count)
                .map(|index| self.derivative(index) + other.derivative(index))
                .collect(),
        }
    }
}

impl Sub for Dual {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        let count = self.gradient_len(&other);
        Self {
            value: self.value - other.value,
            gradient: (0..count)
                .map(|index| self.derivative(index) - other.derivative(index))
                .collect(),
        }
    }
}

impl Mul for Dual {
    type Output = Self;

    #[allow(clippy::suspicious_arithmetic_impl)]
    fn mul(self, other: Self) -> Self::Output {
        let count = self.gradient_len(&other);
        Self {
            value: self.value * other.value,
            gradient: (0..count)
                .map(|index| {
                    self.derivative(index) * other.value + self.value * other.derivative(index)
                })
                .collect(),
        }
    }
}

impl Div for Dual {
    type Output = Self;

    fn div(self, other: Self) -> Self::Output {
        let count = self.gradient_len(&other);
        let denominator = other.value * other.value;
        Self {
            value: self.value / other.value,
            gradient: (0..count)
                .map(|index| {
                    (self.derivative(index) * other.value - self.value * other.derivative(index))
                        / denominator
                })
                .collect(),
        }
    }
}

impl JetScalar for Dual {
    fn zero() -> Self {
        Self::constant(0.0)
    }

    fn one() -> Self {
        Self::constant(1.0)
    }

    fn from_f64(value: f64) -> Self {
        Self::constant(value)
    }

    fn exp(self) -> Self {
        let value = self.value.exp();
        Self {
            value,
            gradient: self
                .gradient
                .into_iter()
                .map(|derivative| value * derivative)
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
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
}
