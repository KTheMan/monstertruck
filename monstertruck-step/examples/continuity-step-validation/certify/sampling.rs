//! Point sampling and mapped seam-grid construction.

use super::super::classify::SeamSelection;
use super::super::errors::ValidationError;
use super::{CertificationStencils, SampleGrid};
use monstertruck_geometry::nurbs::NurbsSurface;
use monstertruck_geometry::nurbs::continuity_solver::BoundaryTransition;
use monstertruck_geometry::prelude::{
    BoundedSurface, InnerSpace, ParametricSurface, Point3, Vector3, Vector4,
};

pub(super) fn certification_seams(
    first: &NurbsSurface<Vector4>,
    second: &NurbsSurface<Vector4>,
    transition: &BoundaryTransition,
    seam: SeamSelection,
    intervals: usize,
) -> Result<Vec<f64>, ValidationError> {
    let first_knots = seam_knots(first, seam.first_boundary);
    let second_knots = seam_knots(second, seam.second_boundary);
    let (first_start, first_extent) = knot_domain(first_knots)?;
    let (second_start, second_extent) = knot_domain(second_knots)?;
    if !first_extent.is_finite()
        || first_extent <= 0.0
        || !second_extent.is_finite()
        || second_extent <= 0.0
    {
        Err(ValidationError::InvalidCertificationGeometry {
            reason: "a seam knot domain is non-finite or empty",
        })
    } else {
        let uniform = (0..=intervals).map(|sample| sample as f64 / intervals as f64);
        let first_boundaries = first_knots
            .iter()
            .map(|knot| (*knot - first_start) / first_extent)
            .map(Ok);
        let second_boundaries = second_knots
            .iter()
            .map(|knot| (*knot - second_start) / second_extent)
            .map(|target| transition_preimage(transition, target));
        let mut seams = uniform
            .map(Ok)
            .chain(first_boundaries)
            .chain(second_boundaries)
            .collect::<Result<Vec<_>, _>>()?;
        if seams
            .iter()
            .any(|value| !value.is_finite() || !(-1.0e-12..=1.0 + 1.0e-12).contains(value))
        {
            Err(ValidationError::InvalidCertificationGeometry {
                reason: "a mapped knot boundary lies outside the master seam domain",
            })
        } else {
            seams
                .iter_mut()
                .for_each(|value| *value = value.clamp(0.0, 1.0));
            seams.sort_by(f64::total_cmp);
            seams.dedup_by(|first, second| (*first - *second).abs() <= 1.0e-12);
            Ok(seams)
        }
    }
}

fn knot_domain(knots: &[f64]) -> Result<(f64, f64), ValidationError> {
    let start = knots
        .first()
        .copied()
        .ok_or(ValidationError::InvalidCertificationGeometry {
            reason: "a seam knot vector is empty",
        })?;
    let end = knots
        .last()
        .copied()
        .ok_or(ValidationError::InvalidCertificationGeometry {
            reason: "a seam knot vector is empty",
        })?;
    Ok((start, end - start))
}

fn seam_knots(
    surface: &NurbsSurface<Vector4>,
    boundary: monstertruck_geometry::nurbs::continuity::SurfaceBoundary,
) -> &[f64] {
    use monstertruck_geometry::nurbs::continuity::SurfaceBoundary;

    match boundary {
        SurfaceBoundary::UStart | SurfaceBoundary::UEnd => surface.knot_vector_v(),
        SurfaceBoundary::VStart | SurfaceBoundary::VEnd => surface.knot_vector_u(),
    }
}

fn transition_preimage(
    transition: &BoundaryTransition,
    target: f64,
) -> Result<f64, ValidationError> {
    let mapped_start = transition
        .mapped_coordinates(0.0, 0.0)
        .map(|coordinates| coordinates.0)
        .ok_or(ValidationError::TransitionSamplingFailed {
            seam: 0.0,
            cross: 0.0,
        })?;
    let mapped_end = transition
        .mapped_coordinates(1.0, 0.0)
        .map(|coordinates| coordinates.0)
        .ok_or(ValidationError::TransitionSamplingFailed {
            seam: 1.0,
            cross: 0.0,
        })?;
    if (target - mapped_start).abs() <= 1.0e-14 {
        Ok(0.0)
    } else if (target - mapped_end).abs() <= 1.0e-14 {
        Ok(1.0)
    } else if target <= mapped_start.min(mapped_end) || target >= mapped_start.max(mapped_end) {
        Err(ValidationError::InvalidCertificationGeometry {
            reason: "a second-surface knot lies outside the solved seam transition",
        })
    } else {
        let increasing = mapped_start < mapped_end;
        let (low, high) = (0..64).try_fold((0.0, 1.0), |(low, high), _| {
            let middle = 0.5 * (low + high);
            let mapped = transition
                .mapped_coordinates(middle, 0.0)
                .map(|coordinates| coordinates.0)
                .ok_or(ValidationError::TransitionSamplingFailed {
                    seam: middle,
                    cross: 0.0,
                })?;
            Ok::<_, ValidationError>(if (mapped < target) == increasing {
                (middle, high)
            } else {
                (low, middle)
            })
        })?;
        Ok(0.5 * (low + high))
    }
}

pub(super) fn seam_stencil(seam: f64, radius: usize, step: f64, centered: &[f64]) -> Vec<f64> {
    let reach = radius as f64 * step;
    if seam < reach {
        (0..=2 * radius).map(|index| index as f64 * step).collect()
    } else if seam > 1.0 - reach {
        (0..=2 * radius)
            .map(|index| -((2 * radius - index) as f64) * step)
            .collect()
    } else {
        centered.to_vec()
    }
}

pub(super) fn sample_grids(
    first: &NurbsSurface<Vector4>,
    second: &NurbsSurface<Vector4>,
    transition: &BoundaryTransition,
    selection: SeamSelection,
    seam: f64,
    seam_nodes: &[f64],
    stencils: &CertificationStencils,
) -> Result<(SampleGrid, SampleGrid), ValidationError> {
    let first_grid = seam_nodes
        .iter()
        .map(|&seam_delta| {
            stencils
                .first_cross_nodes
                .iter()
                .map(|&cross| {
                    evaluate_boundary(first, selection.first_boundary, seam + seam_delta, -cross)
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .collect::<Result<Vec<_>, _>>()?;
    let second_grid = seam_nodes
        .iter()
        .map(|&seam_delta| {
            stencils
                .second_cross_nodes
                .iter()
                .map(|&cross| {
                    let first_seam = seam + seam_delta;
                    let (mapped_seam, mapped_cross) = transition
                        .mapped_coordinates(first_seam, cross)
                        .ok_or(ValidationError::TransitionSamplingFailed {
                            seam: first_seam,
                            cross,
                        })?;
                    evaluate_boundary(second, selection.second_boundary, mapped_seam, mapped_cross)
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok((first_grid, second_grid))
}

fn evaluate_boundary(
    surface: &NurbsSurface<Vector4>,
    boundary: monstertruck_geometry::nurbs::continuity::SurfaceBoundary,
    seam: f64,
    inward: f64,
) -> Result<Point3, ValidationError> {
    use monstertruck_geometry::nurbs::continuity::SurfaceBoundary;

    const DOMAIN_EPSILON: f64 = 1.0e-9;
    if !(-DOMAIN_EPSILON..=1.0 + DOMAIN_EPSILON).contains(&seam)
        || !(-DOMAIN_EPSILON..=1.0 + DOMAIN_EPSILON).contains(&inward)
    {
        Err(ValidationError::BoundarySamplingOutsideDomain { seam, inward })
    } else {
        let seam = seam.clamp(0.0, 1.0);
        let inward = inward.clamp(0.0, 1.0);
        let ((u_start, u_end), (v_start, v_end)) = surface.range_tuple();
        let along_u = u_start + seam * (u_end - u_start);
        let along_v = v_start + seam * (v_end - v_start);
        let point = match boundary {
            SurfaceBoundary::UStart => {
                surface.evaluate(u_start + inward * (u_end - u_start), along_v)
            }
            SurfaceBoundary::UEnd => surface.evaluate(u_end - inward * (u_end - u_start), along_v),
            SurfaceBoundary::VStart => {
                surface.evaluate(along_u, v_start + inward * (v_end - v_start))
            }
            SurfaceBoundary::VEnd => surface.evaluate(along_u, v_end - inward * (v_end - v_start)),
        };
        if point.x.is_finite() && point.y.is_finite() && point.z.is_finite() {
            Ok(point)
        } else {
            Err(ValidationError::NonFiniteCertificate { seam })
        }
    }
}

pub(super) fn surface_pair_scale(
    first: &NurbsSurface<Vector4>,
    second: &NurbsSurface<Vector4>,
) -> Result<f64, ValidationError> {
    let bounds = [first, second]
        .into_iter()
        .flat_map(|surface| {
            (0..=8).flat_map(move |u_sample| {
                (0..=8).map(move |v_sample| {
                    let ((u_start, u_end), (v_start, v_end)) = surface.range_tuple();
                    let u = u_start + (u_end - u_start) * u_sample as f64 / 8.0;
                    let v = v_start + (v_end - v_start) * v_sample as f64 / 8.0;
                    surface.evaluate(u, v)
                })
            })
        })
        .fold(
            [
                f64::INFINITY,
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            ],
            |mut bounds, point| {
                bounds[0] = bounds[0].min(point.x);
                bounds[1] = bounds[1].min(point.y);
                bounds[2] = bounds[2].min(point.z);
                bounds[3] = bounds[3].max(point.x);
                bounds[4] = bounds[4].max(point.y);
                bounds[5] = bounds[5].max(point.z);
                bounds
            },
        );
    let scale = Vector3::new(
        bounds[3] - bounds[0],
        bounds[4] - bounds[1],
        bounds[5] - bounds[2],
    )
    .magnitude();
    if scale.is_finite() && scale > f64::EPSILON {
        Ok(scale)
    } else {
        Err(ValidationError::InvalidCertificationGeometry {
            reason: "the evaluated surface pair has no finite positive scale",
        })
    }
}
