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

    pub(super) const fn value(&self) -> f64 { self.value }

    pub(super) fn gradient(&self) -> &[f64] { &self.gradient }

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

    fn derivative(&self, index: usize) -> f64 { self.gradient.get(index).copied().unwrap_or(0.0) }

    fn product_gradient(&self, other: &Self) -> Vec<f64> {
        (0..self.gradient_len(other))
            .map(|index| {
                self.derivative(index) * other.value + self.value * other.derivative(index)
            })
            .collect()
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

    fn mul(self, other: Self) -> Self::Output {
        Self {
            value: self.value * other.value,
            gradient: self.product_gradient(&other),
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
    fn zero() -> Self { Self::constant(0.0) }

    fn one() -> Self { Self::constant(1.0) }

    fn from_f64(value: f64) -> Self { Self::constant(value) }

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
mod tests;
