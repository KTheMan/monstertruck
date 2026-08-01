//! Finite-difference weights and derivative accumulation.

use super::super::errors::ValidationError;
use super::SampleGrid;
use monstertruck_geometry::prelude::{EuclideanSpace, InnerSpace, Vector3, Zero};

pub(super) fn mixed_derivative(
    grid: &SampleGrid,
    seam_weights: &[f64],
    cross_weights: &[f64],
) -> Vector3 {
    grid.iter()
        .zip(seam_weights)
        .flat_map(|(row, &seam_weight)| {
            row.iter()
                .zip(cross_weights)
                .map(move |(point, &cross_weight)| point.to_vec() * (seam_weight * cross_weight))
        })
        .fold(Vector3::zero(), |sum, value| sum + value)
}

pub(super) fn finite_difference_weights(
    nodes: &[f64],
    derivative: usize,
) -> Result<Vec<f64>, ValidationError> {
    if derivative >= nodes.len() {
        Err(ValidationError::InvalidCertificationConfig {
            reason: "the derivative order exceeds the stencil width",
        })
    } else {
        let mut matrix = (0..nodes.len())
            .map(|power| {
                nodes
                    .iter()
                    .map(|node| node.powi(power as i32))
                    .chain(std::iter::once(if power == derivative {
                        factorial(derivative)
                    } else {
                        0.0
                    }))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        (0..nodes.len()).try_for_each(|column| {
            let pivot = (column..nodes.len())
                .max_by(|&first, &second| {
                    matrix[first][column]
                        .abs()
                        .total_cmp(&matrix[second][column].abs())
                })
                .ok_or(ValidationError::SingularCertificationStencil)?;
            if matrix[pivot][column].abs() <= f64::EPSILON {
                Err(ValidationError::SingularCertificationStencil)
            } else {
                matrix.swap(column, pivot);
                let divisor = matrix[column][column];
                (column..=nodes.len()).for_each(|index| {
                    matrix[column][index] /= divisor;
                });
                (0..nodes.len())
                    .filter(|&row| row != column)
                    .for_each(|row| {
                        let factor = matrix[row][column];
                        (column..=nodes.len()).for_each(|index| {
                            matrix[row][index] -= factor * matrix[column][index];
                        });
                    });
                Ok(())
            }
        })?;
        Ok(matrix.into_iter().map(|row| row[nodes.len()]).collect())
    }
}

pub(super) fn ensure_finite_vector(vector: Vector3, seam: f64) -> Result<(), ValidationError> {
    if vector.x.is_finite() && vector.y.is_finite() && vector.z.is_finite() {
        Ok(())
    } else {
        Err(ValidationError::NonFiniteCertificate { seam })
    }
}

pub(super) fn ensure_nonzero_finite_vector(
    vector: Vector3,
    seam: f64,
) -> Result<(), ValidationError> {
    ensure_finite_vector(vector, seam)?;
    if vector.magnitude2() > f64::EPSILON {
        Ok(())
    } else {
        Err(ValidationError::DegenerateTangentFrame { seam })
    }
}

fn factorial(value: usize) -> f64 { (1..=value).product::<usize>() as f64 }
