//! Truncated bivariate Taylor jets for continuity-constraint composition.
//!
//! Coefficients are factorial-normalized: coefficient `(i, j)` stores
//! `partial_s^i partial_r^j f / (i! j!)`. Products therefore use ordinary
//! bivariate polynomial convolution. Every operation deterministically drops
//! terms whose total order exceeds [`MAX_CONTINUITY_ORDER`].

use crate::nurbs::continuity::MAX_CONTINUITY_ORDER;
use std::array;
use std::ops::{Add, Div, Mul, Sub};

const COEFFICIENT_COUNT: usize = (MAX_CONTINUITY_ORDER + 1) * (MAX_CONTINUITY_ORDER + 2) / 2;

/// Scalar operations required by [`TaylorJet`].
///
/// The solver's dynamic dual scalar implements this trait so the same jet
/// algebra can evaluate values and transition-variable derivatives.
pub(crate) trait JetScalar:
    Clone
    + Add<Self, Output = Self>
    + Sub<Self, Output = Self>
    + Mul<Self, Output = Self>
    + Div<Self, Output = Self> {
    /// Returns additive identity.
    fn zero() -> Self;

    /// Returns multiplicative identity.
    fn one() -> Self;

    /// Converts an `f64` constant.
    fn from_f64(value: f64) -> Self;

    /// Returns the scalar exponential.
    fn exp(self) -> Self;
}

impl JetScalar for f64 {
    #[inline(always)]
    fn zero() -> Self { 0.0 }

    #[inline(always)]
    fn one() -> Self { 1.0 }

    #[inline(always)]
    fn from_f64(value: f64) -> Self { value }

    #[inline(always)]
    fn exp(self) -> Self { f64::exp(self) }
}

/// Factorial-normalized bivariate Taylor polynomial truncated by total order.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TaylorJet<S> {
    order: usize,
    coefficients: [S; COEFFICIENT_COUNT],
}

impl<S: JetScalar> TaylorJet<S> {
    /// Creates a constant jet through `order`.
    ///
    /// # Panics
    ///
    /// Panics when `order` exceeds [`MAX_CONTINUITY_ORDER`].
    pub(crate) fn constant(order: usize, value: S) -> Self {
        assert!(
            order <= MAX_CONTINUITY_ORDER,
            "Taylor-jet order {order} exceeds {MAX_CONTINUITY_ORDER}",
        );
        let mut jet = Self::zero(order);
        jet.coefficients[coefficient_index(0, 0)] = value;
        jet
    }

    /// Creates the zero jet through `order`.
    ///
    /// # Panics
    ///
    /// Panics when `order` exceeds [`MAX_CONTINUITY_ORDER`].
    pub(crate) fn zero(order: usize) -> Self {
        assert!(
            order <= MAX_CONTINUITY_ORDER,
            "Taylor-jet order {order} exceeds {MAX_CONTINUITY_ORDER}",
        );
        Self {
            order,
            coefficients: array::from_fn(|_| S::zero()),
        }
    }

    /// Creates the first coordinate variable with expansion value `value`.
    ///
    /// The returned jet represents `value + delta_s`.
    pub(crate) fn coordinate_s(order: usize, value: S) -> Self {
        let mut jet = Self::constant(order, value);
        if order > 0 {
            jet.coefficients[coefficient_index(1, 0)] = S::one();
        }
        jet
    }

    /// Creates the second coordinate variable with expansion value `value`.
    ///
    /// The returned jet represents `value + delta_r`.
    pub(crate) fn coordinate_r(order: usize, value: S) -> Self {
        let mut jet = Self::constant(order, value);
        if order > 0 {
            jet.coefficients[coefficient_index(0, 1)] = S::one();
        }
        jet
    }

    /// Builds a jet from factorial-normalized coefficients.
    ///
    /// `coefficient(i, j)` is called once for every active term in ascending
    /// total order, then ascending first-coordinate order.
    ///
    /// # Panics
    ///
    /// Panics when `order` exceeds [`MAX_CONTINUITY_ORDER`].
    pub(crate) fn from_coefficients(
        order: usize,
        mut coefficient: impl FnMut(usize, usize) -> S,
    ) -> Self {
        let mut jet = Self::zero(order);
        (0..=order).for_each(|total| {
            (0..=total).for_each(|i| {
                let j = total - i;
                jet.coefficients[coefficient_index(i, j)] = coefficient(i, j);
            });
        });
        jet
    }

    /// Returns the active total order.
    #[inline(always)]
    pub(crate) const fn order(&self) -> usize { self.order }

    /// Returns factorial-normalized coefficient `(i, j)`.
    ///
    /// Returns `None` when `i + j` exceeds this jet's active order.
    #[inline(always)]
    pub(crate) fn coefficient(&self, i: usize, j: usize) -> Option<&S> {
        (i + j <= self.order).then(|| &self.coefficients[coefficient_index(i, j)])
    }

    /// Returns mutable factorial-normalized coefficient `(i, j)`.
    ///
    /// Returns `None` when `i + j` exceeds this jet's active order.
    #[inline(always)]
    #[cfg(test)]
    pub(crate) fn coefficient_mut(&mut self, i: usize, j: usize) -> Option<&mut S> {
        (i + j <= self.order).then(|| {
            let index = coefficient_index(i, j);
            &mut self.coefficients[index]
        })
    }

    /// Multiplies every active coefficient by `scalar`.
    pub(crate) fn scaled(self, scalar: S) -> Self {
        Self::from_coefficients(self.order, |i, j| {
            self.coefficient_value(i, j) * scalar.clone()
        })
    }

    /// Multiplies every active coefficient by an `f64` constant.
    pub(crate) fn scaled_f64(self, scalar: f64) -> Self { self.scaled(S::from_f64(scalar)) }

    /// Returns the multiplicative reciprocal, truncated to this jet's order.
    pub(crate) fn reciprocal(&self) -> Self {
        let constant_inverse = S::one() / self.coefficient_value(0, 0);
        let mut result = Self::constant(self.order, constant_inverse.clone());
        (1..=self.order).for_each(|total| {
            (0..=total).for_each(|i| {
                let j = total - i;
                let sum = (0..=i)
                    .flat_map(|source_i| (0..=j).map(move |source_j| (source_i, source_j)))
                    .filter(|&(source_i, source_j)| source_i + source_j > 0)
                    .fold(S::zero(), |sum, (source_i, source_j)| {
                        sum + self.coefficient_value(source_i, source_j)
                            * result.coefficient_value(i - source_i, j - source_j)
                    });
                result.coefficients[coefficient_index(i, j)] =
                    (S::zero() - sum) * constant_inverse.clone();
            });
        });
        result
    }

    /// Raises this jet to a non-negative integer power.
    pub(crate) fn powi(&self, mut exponent: usize) -> Self {
        let mut base = self.clone();
        let mut result = Self::constant(self.order, S::one());
        while exponent > 0 {
            if exponent % 2 == 1 {
                result = result * base.clone();
            }
            exponent /= 2;
            if exponent > 0 {
                base = base.clone() * base;
            }
        }
        result
    }

    /// Returns the exponential of this jet.
    pub(crate) fn exp(&self) -> Self {
        let constant = self.coefficient_value(0, 0);
        let remainder = self.clone() - Self::constant(self.order, constant.clone());
        let mut term = Self::constant(self.order, S::one());
        let mut result = term.clone();
        let mut inverse_factorial = 1.0;
        (1..=self.order).for_each(|power| {
            term = term.clone() * remainder.clone();
            inverse_factorial /= power as f64;
            result = result.clone() + term.clone().scaled_f64(inverse_factorial);
        });
        result.scaled(constant.exp())
    }

    #[inline(always)]
    fn coefficient_value(&self, i: usize, j: usize) -> S {
        self.coefficients[coefficient_index(i, j)].clone()
    }

    fn assert_same_order(&self, other: &Self) {
        assert_eq!(
            self.order, other.order,
            "Taylor-jet operations require equal orders",
        );
    }
}

impl<S: JetScalar> Add for TaylorJet<S> {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        self.assert_same_order(&other);
        Self::from_coefficients(self.order, |i, j| {
            self.coefficient_value(i, j) + other.coefficient_value(i, j)
        })
    }
}

impl<S: JetScalar> Sub for TaylorJet<S> {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        self.assert_same_order(&other);
        Self::from_coefficients(self.order, |i, j| {
            self.coefficient_value(i, j) - other.coefficient_value(i, j)
        })
    }
}

impl<S: JetScalar> Mul for TaylorJet<S> {
    type Output = Self;

    fn mul(self, other: Self) -> Self::Output {
        self.assert_same_order(&other);
        Self::from_coefficients(self.order, |i, j| {
            (0..=i)
                .flat_map(|left_i| (0..=j).map(move |left_j| (left_i, left_j)))
                .fold(S::zero(), |sum, (left_i, left_j)| {
                    sum + self.coefficient_value(left_i, left_j)
                        * other.coefficient_value(i - left_i, j - left_j)
                })
        })
    }
}

impl<S: JetScalar> Div for TaylorJet<S> {
    type Output = Self;

    fn div(self, other: Self) -> Self::Output {
        self.assert_same_order(&other);
        Mul::mul(self, other.reciprocal())
    }
}

#[inline(always)]
const fn coefficient_index(i: usize, j: usize) -> usize {
    let row_start = i * (2 * MAX_CONTINUITY_ORDER + 3 - i) / 2;
    row_start + j
}

#[cfg(test)]
mod tests {
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
}
