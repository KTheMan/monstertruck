use anyhow::{Result, ensure};
use monstertruck_geometry::base::Vector4;
use monstertruck_geometry::nurbs::continuity::BoundarySide;
use monstertruck_geometry::nurbs::{BsplineSurface, KnotVector, NurbsSurface};

use crate::corpus::{CaseSpec, FixtureMutation, GeometrySpec, WeightModel};

/// Fully constructed surfaces for one corpus case.
pub struct Fixture {
    pub first: NurbsSurface<Vector4>,
    pub second: NurbsSurface<Vector4>,
}

/// Builds a separable, multi-span tensor-product fixture.
pub fn build(case: &CaseSpec) -> Result<Fixture> {
    ensure!(case.geometry.scale.is_finite() && case.geometry.scale > 0.0);
    ensure!(case.geometry.boundary_offset.is_finite());
    ensure!(
        case.geometry.second_cross_domain_scale.is_finite()
            && case.geometry.second_cross_domain_scale > 0.0
    );
    ensure!(
        case.request.alignment.build() == case.geometry.second_seam_parameterization.alignment()
    );

    let cross_degree = case.geometry.cross_degree;
    let cross_knots = cross_knots(cross_degree)?;
    let seam_degree = 5;
    let seam_knots = KnotVector::try_from(vec![
        0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.25, 0.5, 0.75, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
    ])?;
    let cross_count = cross_knots.len() - cross_degree - 1;
    let seam_count = seam_knots.len() - seam_degree - 1;
    let seam_greville = greville_values(&seam_knots, seam_degree, seam_count);
    let whole_points = tensor_points(
        cross_count,
        &seam_greville,
        case.geometry.scale,
        case.geometry.weight_model,
        case.geometry.planar,
    );
    let mut first = NurbsSurface::new(BsplineSurface::new(
        (cross_knots, seam_knots.clone()),
        whole_points,
    ));
    let second = first.cut_u(0.0);
    let mut second = reparameterize_second(&second, &seam_knots, &seam_greville, case.geometry)?;

    perturb_boundary(
        &mut second,
        case.geometry.boundary_offset * case.geometry.scale,
    );
    match case.geometry.mutation {
        FixtureMutation::None => {}
        FixtureMutation::ZeroSecondWeight => {
            second.control_point_mut(0, 2).w = 0.0;
        }
        FixtureMutation::DegenerateFirstBoundary => {
            first.control_points_mut().for_each(|point| {
                point.x = 0.0;
                point.z = 0.0;
            });
        }
    }
    Ok(Fixture {
        first: orient_boundary(first, BoundarySide::MaxU, case.request.first_side.build()),
        second: orient_boundary(second, BoundarySide::MinU, case.request.second_side.build()),
    })
}

fn orient_boundary(
    surface: NurbsSurface<Vector4>,
    source: BoundarySide,
    target: BoundarySide,
) -> NurbsSurface<Vector4> {
    let source_is_min = matches!(source, BoundarySide::MinU | BoundarySide::MinV);
    let target_is_min = matches!(target, BoundarySide::MinU | BoundarySide::MinV);
    let mut oriented = if source_is_min == target_is_min {
        surface
    } else {
        reverse_u(surface)
    };
    if matches!(target, BoundarySide::MinV | BoundarySide::MaxV) {
        oriented.swap_axes();
    }
    oriented
}

fn reverse_u(surface: NurbsSurface<Vector4>) -> NurbsSurface<Vector4> {
    let mut knots_u = surface.knot_vector_u().clone();
    knots_u.invert();
    let control_points = surface.control_points().iter().rev().cloned().collect();
    NurbsSurface::new(BsplineSurface::new(
        (knots_u, surface.knot_vector_v().clone()),
        control_points,
    ))
}

fn cross_knots(degree: usize) -> Result<KnotVector> {
    if degree == 5 {
        Ok(KnotVector::try_from(vec![
            -1.0, -1.0, -1.0, -1.0, -1.0, -1.0, -0.5, 0.5, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
        ])?)
    } else {
        ensure!(degree == 2, "the corpus supports cross degree two or five");
        Ok(KnotVector::try_from(vec![-1.0, -1.0, -1.0, 1.0, 1.0, 1.0])?)
    }
}

fn greville_values(knots: &KnotVector, degree: usize, count: usize) -> Vec<f64> {
    (0..count)
        .map(|index| knots[index + 1..=index + degree].iter().sum::<f64>() / degree as f64)
        .collect()
}

fn tensor_points(
    cross_count: usize,
    seam_values: &[f64],
    scale: f64,
    weight_model: WeightModel,
    planar: bool,
) -> Vec<Vec<Vector4>> {
    (0..cross_count)
        .map(|cross| {
            let normalized = cross as f64 / (cross_count - 1) as f64;
            let x = -1.0 + 2.0 * normalized;
            let cross_z = if planar {
                0.0
            } else {
                0.18 * x * x + 0.07 * x * x * x - 0.1
            };
            let cross_weight = match weight_model {
                WeightModel::Polynomial => 1.0,
                WeightModel::Rational => 1.0 + 0.08 * ((cross * 5 + 3) % 7) as f64,
            };
            seam_values
                .iter()
                .map(|&y| {
                    let seam_weight = weight(weight_model);
                    let homogeneous_weight = cross_weight * seam_weight;
                    let z = cross_z + if planar { 0.0 } else { seam_z(y) };
                    Vector4::new(
                        scale * x * homogeneous_weight,
                        scale * y * homogeneous_weight,
                        scale * z * homogeneous_weight,
                        homogeneous_weight,
                    )
                })
                .collect()
        })
        .collect()
}

fn reparameterize_second(
    second: &NurbsSurface<Vector4>,
    seam_knots: &KnotVector,
    greville: &[f64],
    geometry: GeometrySpec,
) -> Result<NurbsSurface<Vector4>> {
    let parameterization = geometry.second_seam_parameterization;
    let seam_values = greville
        .iter()
        .map(|&value| {
            let warped = if parameterization.is_unequal() {
                value * value * (3.0 - 2.0 * value)
            } else {
                value
            };
            if parameterization.is_reversed() {
                1.0 - warped
            } else {
                warped
            }
        })
        .collect::<Vec<_>>();
    let points = second
        .control_points()
        .iter()
        .map(|row| {
            let reference = row[0];
            let physical_x = reference.x / reference.w;
            let physical_z = reference.z / reference.w;
            let cross_z = physical_z
                - if geometry.planar {
                    0.0
                } else {
                    geometry.scale * seam_z(greville[0])
                };
            let cross_weight = reference.w / weight(geometry.weight_model);
            seam_values
                .iter()
                .map(|&y| {
                    let seam_weight = weight(geometry.weight_model);
                    let homogeneous_weight = cross_weight * seam_weight;
                    let z = cross_z
                        + if geometry.planar {
                            0.0
                        } else {
                            geometry.scale * seam_z(y)
                        };
                    Vector4::new(
                        physical_x * homogeneous_weight,
                        geometry.scale * y * homogeneous_weight,
                        z * homogeneous_weight,
                        homogeneous_weight,
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let cross_knots = KnotVector::try_from(
        second
            .knot_vector_u()
            .iter()
            .map(|knot| knot * geometry.second_cross_domain_scale)
            .collect::<Vec<_>>(),
    )?;
    Ok(NurbsSurface::new(BsplineSurface::new(
        (cross_knots, seam_knots.clone()),
        points,
    )))
}

fn seam_z(parameter: f64) -> f64 {
    0.08 * parameter * (1.0 - parameter) + 0.03 * parameter * parameter * parameter
}

fn weight(model: WeightModel) -> f64 {
    match model {
        WeightModel::Polynomial => 1.0,
        WeightModel::Rational => 1.2,
    }
}

fn perturb_boundary(surface: &mut NurbsSurface<Vector4>, physical_offset: f64) {
    let rows = surface.control_points().len().min(4);
    (0..rows).for_each(|row| {
        let pattern = [1.0, -0.5, 0.25, -0.125][row];
        let columns = surface.control_points()[row].len();
        (0..columns).for_each(|column| {
            let point = surface.control_point_mut(row, column);
            point.z += physical_offset * pattern * point.w;
        });
    });
}
