use anyhow::{Result, anyhow, ensure};
use monstertruck_geometry::base::{EuclideanSpace, InnerSpace, Point3, Vector3, Zero};
use monstertruck_geometry::nurbs::NurbsSurface;
use monstertruck_geometry::nurbs::continuity::SurfaceBoundary;
use monstertruck_geometry::nurbs::continuity_solver::{
    BoundaryContinuityRequest, BoundaryContinuitySolution,
};
use monstertruck_traits::ParametricSurface;
use serde::Serialize;

use crate::corpus::DenseSpec;

type SampleGrid = Vec<Vec<Point3>>;

/// Independent dense common-coordinate residual summary.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DenseMetrics {
    pub maximum_absolute_residual_by_order: Vec<f64>,
    pub maximum_normalized_residual_by_order: Vec<f64>,
    pub maximum_normal_angle: f64,
    pub worst_seam_by_order: Vec<f64>,
    pub worst_mixed_derivative_by_order: Vec<[usize; 2]>,
}

/// Certifies a solved transition without using solver collocation residuals.
pub fn certify(
    solution: &BoundaryContinuitySolution,
    request: BoundaryContinuityRequest,
    spec: DenseSpec,
    scale: f64,
) -> Result<DenseMetrics> {
    ensure!(spec.seam_samples > 0);
    ensure!(2 * spec.stencil_radius + 1 > request.order().as_usize());
    ensure!(spec.normalized_step.is_finite() && spec.normalized_step > 0.0);
    let order = request.order().as_usize();
    let radius = spec.stencil_radius;
    let centered_seam_nodes = (-(radius as isize)..=radius as isize)
        .map(|index| index as f64 * spec.normalized_step)
        .collect::<Vec<_>>();
    let first_cross_nodes = (0..=2 * radius)
        .map(|index| -((2 * radius - index) as f64) * spec.normalized_step)
        .collect::<Vec<_>>();
    let second_cross_nodes = (0..=2 * radius)
        .map(|index| index as f64 * spec.normalized_step)
        .collect::<Vec<_>>();
    let first_cross_weights = (0..=order)
        .map(|derivative| finite_difference_weights(&first_cross_nodes, derivative))
        .collect::<Result<Vec<_>>>()?;
    let second_cross_weights = (0..=order)
        .map(|derivative| finite_difference_weights(&second_cross_nodes, derivative))
        .collect::<Result<Vec<_>>>()?;
    let margin = (radius as f64 + 1.0) * spec.normalized_step;
    ensure!(margin < 0.25);
    ensure!(2.0 * radius as f64 * spec.normalized_step <= 1.0);

    let mut maximum_absolute_residual_by_order = vec![0.0_f64; order + 1];
    let mut maximum_normalized_residual_by_order = vec![0.0_f64; order + 1];
    let mut worst_seam_by_order = vec![0.0_f64; order + 1];
    let mut worst_mixed_derivative_by_order = vec![[0, 0]; order + 1];
    let mut maximum_normal_angle = 0.0_f64;
    certification_seams(solution, request, spec.seam_samples, margin)?
        .into_iter()
        .try_for_each(|seam| -> Result<()> {
            let seam_nodes = if seam == 0.0 {
                (0..=2 * radius)
                    .map(|index| index as f64 * spec.normalized_step)
                    .collect::<Vec<_>>()
            } else if seam == 1.0 {
                (0..=2 * radius)
                    .map(|index| -((2 * radius - index) as f64) * spec.normalized_step)
                    .collect::<Vec<_>>()
            } else {
                centered_seam_nodes.clone()
            };
            let seam_weights = (0..=order)
                .map(|derivative| finite_difference_weights(&seam_nodes, derivative))
                .collect::<Result<Vec<_>>>()?;
            let (first_grid, second_grid) = sample_grids(
                solution,
                request,
                seam,
                &seam_nodes,
                &first_cross_nodes,
                &second_cross_nodes,
            )?;
            (0..=order).try_for_each(|total| -> Result<()> {
                (0..=total).try_for_each(|cross_order| -> Result<()> {
                    let seam_order = total - cross_order;
                    let first = mixed_derivative(
                        &first_grid,
                        &seam_weights[seam_order],
                        &first_cross_weights[cross_order],
                    );
                    let second = mixed_derivative(
                        &second_grid,
                        &seam_weights[seam_order],
                        &second_cross_weights[cross_order],
                    );
                    let absolute = (first - second).magnitude();
                    let normalized = absolute / scale;
                    ensure_finite_vector(first)?;
                    ensure_finite_vector(second)?;
                    ensure!(absolute.is_finite() && normalized.is_finite());
                    maximum_absolute_residual_by_order[total] =
                        maximum_absolute_residual_by_order[total].max(absolute);
                    if normalized > maximum_normalized_residual_by_order[total] {
                        maximum_normalized_residual_by_order[total] = normalized;
                        worst_seam_by_order[total] = seam;
                        worst_mixed_derivative_by_order[total] = [seam_order, cross_order];
                    }
                    Ok(())
                })?;
                Ok(())
            })?;
            if order >= 1 {
                let first_seam =
                    mixed_derivative(&first_grid, &seam_weights[1], &first_cross_weights[0]);
                let first_cross =
                    mixed_derivative(&first_grid, &seam_weights[0], &first_cross_weights[1]);
                let second_seam =
                    mixed_derivative(&second_grid, &seam_weights[1], &second_cross_weights[0]);
                let second_cross =
                    mixed_derivative(&second_grid, &seam_weights[0], &second_cross_weights[1]);
                [first_seam, first_cross, second_seam, second_cross]
                    .into_iter()
                    .try_for_each(ensure_nonzero_finite_vector)?;
                let first_normal = first_seam.cross(first_cross);
                let second_normal = second_seam.cross(second_cross);
                [first_normal, second_normal]
                    .into_iter()
                    .try_for_each(ensure_nonzero_finite_vector)?;
                let cosine = first_normal
                    .normalize()
                    .dot(second_normal.normalize())
                    .abs()
                    .clamp(-1.0, 1.0);
                let angle = cosine.acos();
                ensure!(angle.is_finite());
                maximum_normal_angle = maximum_normal_angle.max(angle);
            }
            Ok(())
        })?;

    Ok(DenseMetrics {
        maximum_absolute_residual_by_order,
        maximum_normalized_residual_by_order,
        maximum_normal_angle,
        worst_seam_by_order,
        worst_mixed_derivative_by_order,
    })
}

fn certification_seams(
    solution: &BoundaryContinuitySolution,
    request: BoundaryContinuityRequest,
    chebyshev_count: usize,
    margin: f64,
) -> Result<Vec<f64>> {
    let first_knots = match request.first_boundary() {
        SurfaceBoundary::UStart | SurfaceBoundary::UEnd => solution.first().knot_vector_v(),
        SurfaceBoundary::VStart | SurfaceBoundary::VEnd => solution.first().knot_vector_u(),
    };
    let second_knots = match request.second_boundary() {
        SurfaceBoundary::UStart | SurfaceBoundary::UEnd => solution.second().knot_vector_v(),
        SurfaceBoundary::VStart | SurfaceBoundary::VEnd => solution.second().knot_vector_u(),
    };
    let first_start = first_knots[0];
    let first_extent = first_knots[first_knots.len() - 1] - first_start;
    let second_start = second_knots[0];
    let second_extent = second_knots[second_knots.len() - 1] - second_start;
    ensure!(first_extent.is_finite() && first_extent > 0.0);
    ensure!(second_extent.is_finite() && second_extent > 0.0);
    let first_boundaries = first_knots
        .iter()
        .map(|knot| (*knot - first_start) / first_extent);
    let second_boundaries = second_knots
        .iter()
        .map(|knot| (*knot - second_start) / second_extent)
        .map(|target| transition_preimage(solution, target));
    let mut boundaries = first_boundaries
        .map(Ok)
        .chain(second_boundaries)
        .collect::<Result<Vec<_>>>()?;
    boundaries.iter().try_for_each(|boundary| {
        ensure!(
            boundary.is_finite() && (-1.0e-12..=1.0 + 1.0e-12).contains(boundary),
            "mapped seam boundary lies outside the master domain",
        );
        Ok(())
    })?;
    boundaries
        .iter_mut()
        .for_each(|boundary| *boundary = boundary.clamp(0.0, 1.0));
    boundaries.sort_by(f64::total_cmp);
    boundaries.dedup_by(|first, second| (*first - *second).abs() <= 1.0e-12);
    let span_count = boundaries.len().saturating_sub(1);
    ensure!(span_count > 0);
    let interiors = boundaries
        .windows(2)
        .enumerate()
        .flat_map(|(span, bounds)| {
            let width = bounds[1] - bounds[0];
            let count =
                chebyshev_count / span_count + usize::from(span < chebyshev_count % span_count);
            let local_margin = margin / width;
            (local_margin < 0.5)
                .then(|| {
                    chebyshev_midpoints(count, local_margin)
                        .map(move |local| bounds[0] + local * width)
                })
                .into_iter()
                .flatten()
        })
        .collect::<Vec<_>>();
    let mut seams = boundaries.into_iter().chain(interiors).collect::<Vec<_>>();
    seams.sort_by(f64::total_cmp);
    seams.dedup_by(|first, second| (*first - *second).abs() <= 1.0e-12);
    Ok(seams)
}

fn transition_preimage(solution: &BoundaryContinuitySolution, target: f64) -> Result<f64> {
    let transition = solution.transition();
    let mapped_start = transition
        .mapped_coordinates(0.0, 0.0)
        .ok_or_else(|| anyhow!("the transition is non-finite at the seam start"))?
        .0;
    let mapped_end = transition
        .mapped_coordinates(1.0, 0.0)
        .ok_or_else(|| anyhow!("the transition is non-finite at the seam end"))?
        .0;
    if (target - mapped_start).abs() <= 1.0e-14 {
        Ok(0.0)
    } else if (target - mapped_end).abs() <= 1.0e-14 {
        Ok(1.0)
    } else {
        let increasing = mapped_start < mapped_end;
        ensure!(
            target > mapped_start.min(mapped_end) && target < mapped_start.max(mapped_end),
            "second seam boundary lies outside the solved transition range",
        );
        let (low, high) = (0..64).try_fold((0.0, 1.0), |(low, high), _| {
            let middle = 0.5 * (low + high);
            let mapped = transition
                .mapped_coordinates(middle, 0.0)
                .ok_or_else(|| anyhow!("the transition became non-finite during inversion"))?
                .0;
            Ok::<_, anyhow::Error>(if (mapped < target) == increasing {
                (middle, high)
            } else {
                (low, middle)
            })
        })?;
        Ok(0.5 * (low + high))
    }
}

fn ensure_finite_vector(vector: Vector3) -> Result<()> {
    ensure!(
        vector.x.is_finite() && vector.y.is_finite() && vector.z.is_finite(),
        "dense certification produced a non-finite derivative",
    );
    Ok(())
}

fn ensure_nonzero_finite_vector(vector: Vector3) -> Result<()> {
    ensure_finite_vector(vector)?;
    ensure!(
        vector.magnitude2() > f64::EPSILON,
        "dense certification produced a singular tangent frame",
    );
    Ok(())
}

fn sample_grids(
    solution: &BoundaryContinuitySolution,
    request: BoundaryContinuityRequest,
    seam: f64,
    seam_nodes: &[f64],
    first_cross_nodes: &[f64],
    second_cross_nodes: &[f64],
) -> Result<(SampleGrid, SampleGrid)> {
    let first = seam_nodes
        .iter()
        .map(|&seam_delta| {
            first_cross_nodes
                .iter()
                .map(|&cross| {
                    evaluate_boundary(
                        solution.first(),
                        request.first_boundary(),
                        seam + seam_delta,
                        -cross,
                    )
                })
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<_>>>()?;
    let second = seam_nodes
        .iter()
        .map(|&seam_delta| {
            second_cross_nodes
                .iter()
                .map(|&cross| {
                    let (mapped_seam, mapped_cross) = solution
                        .transition()
                        .mapped_coordinates(seam + seam_delta, cross)
                        .ok_or_else(|| anyhow!("the solved transition became non-finite"))?;
                    evaluate_boundary(
                        solution.second(),
                        request.second_boundary(),
                        mapped_seam,
                        mapped_cross,
                    )
                })
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<_>>>()?;
    Ok((first, second))
}

fn evaluate_boundary(
    surface: &NurbsSurface<monstertruck_geometry::base::Vector4>,
    boundary: SurfaceBoundary,
    seam: f64,
    inward: f64,
) -> Result<Point3> {
    const DOMAIN_EPSILON: f64 = 1.0e-9;
    ensure!(
        (-DOMAIN_EPSILON..=1.0 + DOMAIN_EPSILON).contains(&seam)
            && (-DOMAIN_EPSILON..=1.0 + DOMAIN_EPSILON).contains(&inward),
        "dense certification attempted to sample outside {boundary:?}: \
         seam={seam}, inward={inward}",
    );
    let seam = seam.clamp(0.0, 1.0);
    let inward = inward.clamp(0.0, 1.0);
    let u = surface.knot_vector_u();
    let v = surface.knot_vector_v();
    let (u_start, u_end) = (u[0], u[u.len() - 1]);
    let (v_start, v_end) = (v[0], v[v.len() - 1]);
    let along_u = u_start + seam * (u_end - u_start);
    let along_v = v_start + seam * (v_end - v_start);
    let point = match boundary {
        SurfaceBoundary::UStart => surface.evaluate(u_start + inward * (u_end - u_start), along_v),
        SurfaceBoundary::UEnd => surface.evaluate(u_end - inward * (u_end - u_start), along_v),
        SurfaceBoundary::VStart => surface.evaluate(along_u, v_start + inward * (v_end - v_start)),
        SurfaceBoundary::VEnd => surface.evaluate(along_u, v_end - inward * (v_end - v_start)),
    };
    ensure!(
        point.x.is_finite() && point.y.is_finite() && point.z.is_finite(),
        "dense certification sampled a non-finite point",
    );
    Ok(point)
}

fn mixed_derivative(grid: &[Vec<Point3>], seam_weights: &[f64], cross_weights: &[f64]) -> Vector3 {
    grid.iter()
        .zip(seam_weights)
        .flat_map(|(row, &seam_weight)| {
            row.iter()
                .zip(cross_weights)
                .map(move |(point, &cross_weight)| point.to_vec() * (seam_weight * cross_weight))
        })
        .fold(Vector3::zero(), |sum, value| sum + value)
}

fn finite_difference_weights(nodes: &[f64], derivative: usize) -> Result<Vec<f64>> {
    ensure!(derivative < nodes.len());
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
    (0..nodes.len()).try_for_each(|column| -> Result<()> {
        let pivot = (column..nodes.len())
            .max_by(|&first, &second| {
                matrix[first][column]
                    .abs()
                    .total_cmp(&matrix[second][column].abs())
            })
            .ok_or_else(|| anyhow!("finite-difference system has no pivot"))?;
        ensure!(matrix[pivot][column].abs() > f64::EPSILON);
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
    })?;
    Ok(matrix.into_iter().map(|row| row[nodes.len()]).collect())
}

fn chebyshev_midpoints(count: usize, margin: f64) -> impl Iterator<Item = f64> {
    (0..count).map(move |index| {
        let angle = std::f64::consts::PI * (2 * index + 1) as f64 / (2 * count) as f64;
        margin + (1.0 - 2.0 * margin) * 0.5 * (1.0 - angle.cos())
    })
}

fn factorial(value: usize) -> f64 { (1..=value).product::<usize>() as f64 }
