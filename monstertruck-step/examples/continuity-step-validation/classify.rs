//! Full-patch NURBS seam classification.

use super::errors::ValidationError;
use monstertruck_geometry::nurbs::continuity::SurfaceBoundary;
use monstertruck_geometry::nurbs::contract::BoundaryAlignment;
use monstertruck_geometry::prelude::{
    BoundedSurface, MetricSpace, NurbsSurface, ParametricSurface, Point2, Point3, TOLERANCE,
    Vector4,
};
use monstertruck_meshing::prelude::{
    ExactParameterBoundary2D, ExactTrimBoundary2D, ParameterBoundary2D,
};
use monstertruck_step::load::step_geometry::{Curve3D, StepParameterCurve, Surface};
use monstertruck_topology::compress::{CompressedEdgeUse, CompressedTrimmedShell};

const CLASSIFICATION_INTERVALS: usize = 32;
const BOUNDARIES: [SurfaceBoundary; 4] = [
    SurfaceBoundary::UStart,
    SurfaceBoundary::UEnd,
    SurfaceBoundary::VStart,
    SurfaceBoundary::VEnd,
];
const ALIGNMENTS: [BoundaryAlignment; 2] =
    [BoundaryAlignment::Aligned, BoundaryAlignment::Reversed];

pub(super) type ImportedShell =
    CompressedTrimmedShell<Point3, Curve3D, Surface, StepParameterCurve>;

#[derive(Clone, Copy, Debug)]
pub(super) struct SeamSelection {
    pub(super) first_face: usize,
    pub(super) second_face: usize,
    pub(super) first_boundary: SurfaceBoundary,
    pub(super) second_boundary: SurfaceBoundary,
    pub(super) alignment: BoundaryAlignment,
    pub(super) classification_maximum: f64,
}

pub(super) fn select_full_nurbs_seam(
    shell: &ImportedShell,
    tolerance: f64,
) -> Result<SeamSelection, ValidationError> {
    let nurbs_faces = shell
        .faces
        .iter()
        .enumerate()
        .filter_map(|(index, face)| to_nurbs(&face.surface).map(|surface| (index, surface)))
        .collect::<Vec<_>>();
    if nurbs_faces.len() < 2 {
        Err(ValidationError::InsufficientNurbsFaces(nurbs_faces.len()))
    } else {
        let best = nurbs_faces
            .iter()
            .enumerate()
            .flat_map(|(left, (first_face, first))| {
                nurbs_faces[left + 1..]
                    .iter()
                    .map(move |(second_face, second)| (*first_face, first, *second_face, second))
            })
            .flat_map(|(first_face, first, second_face, second)| {
                BOUNDARIES.into_iter().flat_map(move |first_boundary| {
                    BOUNDARIES.into_iter().flat_map(move |second_boundary| {
                        ALIGNMENTS.into_iter().map(move |alignment| {
                            let maximum = positional_maximum(
                                first,
                                first_boundary,
                                second,
                                second_boundary,
                                alignment,
                            );
                            SeamSelection {
                                first_face,
                                second_face,
                                first_boundary,
                                second_boundary,
                                alignment,
                                classification_maximum: maximum,
                            }
                        })
                    })
                })
            })
            .filter(|selection| {
                selection.classification_maximum.is_finite()
                    && selection.classification_maximum <= tolerance
            })
            .min_by(|left, right| {
                left.classification_maximum
                    .total_cmp(&right.classification_maximum)
            })
            .ok_or(ValidationError::NoCoincidentFullBoundary { tolerance })?;
        validate_shared_full_trim(shell, best, tolerance)?;
        Ok(best)
    }
}

pub(super) fn to_nurbs(surface: &Surface) -> Option<NurbsSurface<Vector4>> {
    match surface {
        Surface::BsplineSurface(surface) => Some(NurbsSurface::from(surface.clone())),
        Surface::NurbsSurface(surface) => Some(surface.clone()),
        _ => None,
    }
}

pub(super) fn boundary_parameters(
    surface: &NurbsSurface<Vector4>,
    boundary: SurfaceBoundary,
    seam: f64,
) -> (f64, f64) {
    let ((u_start, u_end), (v_start, v_end)) = surface.range_tuple();
    match boundary {
        SurfaceBoundary::UStart => (u_start, lerp(v_start, v_end, seam)),
        SurfaceBoundary::UEnd => (u_end, lerp(v_start, v_end, seam)),
        SurfaceBoundary::VStart => (lerp(u_start, u_end, seam), v_start),
        SurfaceBoundary::VEnd => (lerp(u_start, u_end, seam), v_end),
    }
}

fn positional_maximum(
    first: &NurbsSurface<Vector4>,
    first_boundary: SurfaceBoundary,
    second: &NurbsSurface<Vector4>,
    second_boundary: SurfaceBoundary,
    alignment: BoundaryAlignment,
) -> f64 {
    (0..=CLASSIFICATION_INTERVALS)
        .map(|sample| sample as f64 / CLASSIFICATION_INTERVALS as f64)
        .map(|seam| {
            let second_seam = match alignment {
                BoundaryAlignment::Aligned => seam,
                BoundaryAlignment::Reversed => 1.0 - seam,
            };
            let (first_u, first_v) = boundary_parameters(first, first_boundary, seam);
            let (second_u, second_v) = boundary_parameters(second, second_boundary, second_seam);
            first
                .evaluate(first_u, first_v)
                .distance(second.evaluate(second_u, second_v))
        })
        .fold(0.0, f64::max)
}

fn validate_shared_full_trim(
    shell: &ImportedShell,
    selection: SeamSelection,
    tolerance: f64,
) -> Result<(), ValidationError> {
    let first = &shell.faces[selection.first_face];
    let second = &shell.faces[selection.second_face];
    let first_surface =
        to_nurbs(&first.surface).ok_or(ValidationError::InsufficientNurbsFaces(0))?;
    let second_surface =
        to_nurbs(&second.surface).ok_or(ValidationError::InsufficientNurbsFaces(0))?;
    let shared_edge = first
        .boundaries
        .iter()
        .flatten()
        .filter(|first_use| {
            second
                .boundaries
                .iter()
                .flatten()
                .any(|second_use| second_use.index == first_use.index)
        })
        .find(|first_use| {
            trim_covers_boundary(
                first_use,
                shell,
                &first.surface,
                &first_surface,
                selection.first_boundary,
                tolerance,
            ) && second
                .boundaries
                .iter()
                .flatten()
                .filter(|second_use| second_use.index == first_use.index)
                .any(|second_use| {
                    trim_covers_boundary(
                        second_use,
                        shell,
                        &second.surface,
                        &second_surface,
                        selection.second_boundary,
                        tolerance,
                    )
                })
        });
    shared_edge
        .map(|_| ())
        .ok_or(ValidationError::UnsupportedTrimmedSubcurve {
            first_face: selection.first_face + 1,
            second_face: selection.second_face + 1,
            first_boundary: selection.first_boundary,
            second_boundary: selection.second_boundary,
        })
}

fn trim_covers_boundary(
    edge_use: &CompressedEdgeUse<StepParameterCurve>,
    shell: &ImportedShell,
    imported_surface: &Surface,
    surface: &NurbsSurface<Vector4>,
    boundary: SurfaceBoundary,
    tolerance: f64,
) -> bool {
    let sampling_tolerance = tolerance.max(TOLERANCE);
    let points = edge_use
        .trim_curve
        .as_ref()
        .map(|trim| trim.exact_trim_boundary_2d(sampling_tolerance))
        .filter(|points| !points.is_empty())
        .or_else(|| {
            let curve = &shell.edges.get(edge_use.index)?.curve;
            curve
                .exact_parameter_boundary_2d(imported_surface)
                .map(|trim| trim.exact_trim_boundary_2d(sampling_tolerance))
                .filter(|points| !points.is_empty())
                .or_else(|| curve.parameter_boundary_2d(imported_surface, sampling_tolerance))
        });
    points.is_some_and(|points| {
        points.len() >= 2 && points_cover_boundary(&points, surface, boundary, tolerance)
    })
}

fn points_cover_boundary(
    points: &[Point2],
    surface: &NurbsSurface<Vector4>,
    boundary: SurfaceBoundary,
    tolerance: f64,
) -> bool {
    let ((u_start, u_end), (v_start, v_end)) = surface.range_tuple();
    let (constant, varying_start, varying_end, axis_u) = match boundary {
        SurfaceBoundary::UStart => (u_start, v_start, v_end, true),
        SurfaceBoundary::UEnd => (u_end, v_start, v_end, true),
        SurfaceBoundary::VStart => (v_start, u_start, u_end, false),
        SurfaceBoundary::VEnd => (v_end, u_start, u_end, false),
    };
    let constant_matches = points.iter().all(|point| {
        let value = if axis_u { point.x } else { point.y };
        (value - constant).abs() <= tolerance
    });
    let varying = points
        .iter()
        .map(|point| if axis_u { point.y } else { point.x })
        .collect::<Vec<_>>();
    let minimum = varying.iter().copied().fold(f64::INFINITY, f64::min);
    let maximum = varying.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    constant_matches
        && (minimum - varying_start.min(varying_end)).abs() <= tolerance
        && (maximum - varying_start.max(varying_end)).abs() <= tolerance
}

fn lerp(start: f64, end: f64, parameter: f64) -> f64 { start + (end - start) * parameter }
