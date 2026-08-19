//! Immutable local seam-coordinate transitions.

use thiserror::Error;

use super::super::super::continuity::BoundaryAlignment;
use super::super::super::continuity::ContinuityOrder;

const MAX_TRANSITION_CONTROL_COUNT: usize = 66;

/// Typed failure from evaluating a solved boundary transition.
#[derive(Clone, Copy, Debug, Error, Hash, PartialEq, Eq)]
#[non_exhaustive]
pub enum BoundaryTransitionEvaluationError {
    /// The normalized master seam coordinate is non-finite.
    #[error("the normalized master seam coordinate is non-finite")]
    NonFiniteSeam,
    /// The signed common cross-seam coordinate is non-finite.
    #[error("the signed common cross-seam coordinate is non-finite")]
    NonFiniteCross,
    /// A `G0` solve establishes only the on-seam correspondence.
    #[error("a G0 transition has no solved off-seam cross coordinate")]
    CrossCoordinateUnavailable,
    /// The accepted transition could not produce finite mapped coordinates.
    #[error("the solved boundary transition produced a non-finite coordinate")]
    NonFiniteOutput,
}

/// Immutable local coordinate transition from the master seam to the second surface.
///
/// The transition maps a normalized master seam coordinate and the solver's
/// signed common cross-seam coordinate to normalized coordinates on the second
/// boundary frame. The cross-seam coordinate is zero on the seam and positive
/// into the second surface, so it is the negative of the first surface's
/// normalized inward coordinate.
///
/// Cross-seam terms are the solver's Taylor expansion about `cross = 0`,
/// truncated at [`Self::order`]. The expansion is intended for local seam
/// certification. Finite output away from the seam does not guarantee that the
/// mapped coordinates remain in either surface domain or that the truncated
/// map is globally invertible. For `G0`, only the seam correspondence is
/// solved, so the canonical typed evaluator refuses off-seam evaluation.
///
/// The transition exposes the accepted reparameterization without exposing
/// optimizer variables or mutable solver state.
#[derive(Clone, Debug, PartialEq)]
pub struct BoundaryTransition {
    order: ContinuityOrder,
    alignment: BoundaryAlignment,
    seam_map_log_increments: Vec<f64>,
    alpha_fields: Vec<Vec<f64>>,
    log_beta_field: Vec<f64>,
    higher_beta_fields: Vec<Vec<f64>>,
}

impl BoundaryTransition {
    pub(in crate::nurbs::continuity_solver) const fn new(
        order: ContinuityOrder,
        alignment: BoundaryAlignment,
        seam_map_log_increments: Vec<f64>,
        alpha_fields: Vec<Vec<f64>>,
        log_beta_field: Vec<f64>,
        higher_beta_fields: Vec<Vec<f64>>,
    ) -> Self {
        Self {
            order,
            alignment,
            seam_map_log_increments,
            alpha_fields,
            log_beta_field,
            higher_beta_fields,
        }
    }

    /// Returns the second boundary's orientation relative to the master.
    pub const fn alignment(&self) -> BoundaryAlignment { self.alignment }

    /// Returns the solved transition order.
    pub const fn order(&self) -> ContinuityOrder { self.order }

    #[doc(hidden)]
    /// Returns the configured Bernstein degree of the cross-seam fields.
    pub fn cross_field_degree(&self) -> usize { self.seam_map_log_increments.len() }

    #[doc(hidden)]
    /// Returns the Bernstein degree of the endpoint-preserving seam map.
    pub fn seam_map_degree(&self) -> usize { self.seam_map_log_increments.len() + 1 }

    /// Maps a normalized master seam coordinate onto the second seam.
    ///
    /// The normalized seam coordinate is intended to lie in `0.0..=1.0`.
    /// Evaluation outside that interval is permitted for local numerical work
    /// but does not establish that the result lies inside either surface domain.
    ///
    /// # Errors
    ///
    /// Returns [`BoundaryTransitionEvaluationError::NonFiniteSeam`] for a
    /// non-finite input and
    /// [`BoundaryTransitionEvaluationError::NonFiniteOutput`] when the solved
    /// transition cannot produce a finite coordinate.
    pub fn mapped_seam_coordinate(
        &self,
        seam: f64,
    ) -> Result<f64, BoundaryTransitionEvaluationError> {
        if seam.is_finite() {
            self.mapped_seam(seam)
                .ok_or(BoundaryTransitionEvaluationError::NonFiniteOutput)
        } else {
            Err(BoundaryTransitionEvaluationError::NonFiniteSeam)
        }
    }

    /// Maps a normalized master seam coordinate and signed cross-seam
    /// coordinate to the second frame.
    ///
    /// `cross` is zero on the seam and positive into the second surface. It is
    /// the negative of the first surface's normalized inward coordinate.
    ///
    /// This evaluates the local Taylor expansion through [`Self::order`].
    /// The returned tuple is `(second_seam, second_inward)`. The normalized
    /// seam coordinate is intended to lie in `0.0..=1.0`; callers performing
    /// certification should use a one-sided cross-seam stencil near zero.
    ///
    /// # Errors
    ///
    /// Returns a typed [`BoundaryTransitionEvaluationError`] for non-finite
    /// input or output. A `G0` transition accepts only `cross == 0.0`, because
    /// the solver did not establish an off-seam cross-coordinate mapping.
    pub fn try_mapped_coordinates(
        &self,
        seam: f64,
        cross: f64,
    ) -> Result<(f64, f64), BoundaryTransitionEvaluationError> {
        if !seam.is_finite() {
            Err(BoundaryTransitionEvaluationError::NonFiniteSeam)
        } else if !cross.is_finite() {
            Err(BoundaryTransitionEvaluationError::NonFiniteCross)
        } else if self.order == ContinuityOrder::G0 && cross != 0.0 {
            Err(BoundaryTransitionEvaluationError::CrossCoordinateUnavailable)
        } else {
            let mapped_seam = self
                .mapped_seam(seam)
                .ok_or(BoundaryTransitionEvaluationError::NonFiniteOutput)?;
            if self.order == ContinuityOrder::G0 {
                Ok((mapped_seam, 0.0))
            } else {
                let second_seam = self.alpha_fields.iter().enumerate().try_fold(
                    mapped_seam,
                    |value, (index, field)| {
                        let order = index + 1;
                        bernstein_value(field, seam).map(|coefficient| {
                            value + coefficient * cross.powi(order as i32) / factorial(order)
                        })
                    },
                );
                let first_beta = bernstein_value(&self.log_beta_field, seam).map(f64::exp);
                let second_cross = first_beta.and_then(|first_beta| {
                    self.higher_beta_fields.iter().enumerate().try_fold(
                        first_beta * cross,
                        |value, (index, field)| {
                            let order = index + 2;
                            bernstein_value(field, seam).map(|coefficient| {
                                value + coefficient * cross.powi(order as i32) / factorial(order)
                            })
                        },
                    )
                });
                second_seam
                    .zip(second_cross)
                    .filter(|(seam, cross)| seam.is_finite() && cross.is_finite())
                    .ok_or(BoundaryTransitionEvaluationError::NonFiniteOutput)
            }
        }
    }

    #[doc(hidden)]
    /// Compatibility evaluator retained for the downstream evidence corpus.
    ///
    /// For `G0`, this preserves the legacy identity cross-coordinate convention.
    pub fn mapped_coordinates(&self, seam: f64, cross: f64) -> Option<(f64, f64)> {
        if self.order == ContinuityOrder::G0 {
            if cross.is_finite() {
                self.mapped_seam_coordinate(seam)
                    .ok()
                    .map(|mapped_seam| (mapped_seam, cross))
            } else {
                None
            }
        } else {
            self.try_mapped_coordinates(seam, cross).ok()
        }
    }

    fn mapped_seam(&self, seam: f64) -> Option<f64> {
        if !seam.is_finite() {
            None
        } else {
            let total = self
                .seam_map_log_increments
                .iter()
                .try_fold(1.0, |total, value| {
                    let increment = value.exp();
                    let next = total + increment;
                    (increment.is_finite() && next.is_finite()).then_some(next)
                })?;
            let control_count = self
                .seam_map_log_increments
                .len()
                .checked_add(2)
                .filter(|&count| count <= MAX_TRANSITION_CONTROL_COUNT)?;
            let mut controls = [0.0; MAX_TRANSITION_CONTROL_COUNT];
            let mut cumulative = 0.0;
            self.seam_map_log_increments
                .iter()
                .map(|value| value.exp())
                .chain(std::iter::once(1.0))
                .enumerate()
                .for_each(|(index, increment)| {
                    cumulative += increment / total;
                    controls[index + 1] = cumulative;
                });
            let mapped = bernstein_value(&controls[..control_count], seam)?;
            let aligned = match self.alignment {
                BoundaryAlignment::Aligned => mapped,
                BoundaryAlignment::Reversed => 1.0 - mapped,
            };
            aligned.is_finite().then_some(aligned)
        }
    }
}

fn bernstein_value(coefficients: &[f64], parameter: f64) -> Option<f64> {
    if coefficients.is_empty() || coefficients.len() > MAX_TRANSITION_CONTROL_COUNT {
        None
    } else {
        let mut level = [0.0; MAX_TRANSITION_CONTROL_COUNT];
        level[..coefficients.len()].copy_from_slice(coefficients);
        (1..coefficients.len()).for_each(|remaining| {
            (0..coefficients.len() - remaining).for_each(|index| {
                level[index] = (1.0 - parameter) * level[index] + parameter * level[index + 1];
            });
        });
        level[0].is_finite().then_some(level[0])
    }
}

const fn factorial(value: usize) -> f64 {
    match value {
        0 | 1 => 1.0,
        2 => 2.0,
        3 => 6.0,
        4 => 24.0,
        _ => f64::INFINITY,
    }
}

#[cfg(test)]
mod tests;
