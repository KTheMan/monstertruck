//! Canonical boundary coordinates for tensor-product NURBS surfaces.

use thiserror::Error;

use crate::base::Vector4;
use crate::nurbs::continuity::{SurfaceAxis, SurfaceBoundary};
#[cfg(test)]
use crate::nurbs::contract::BoundaryAlignment;
use crate::nurbs::{KnotVector, NurbsSurface};

/// One finite, nonempty surface parameter domain.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct ParameterDomain {
    start: f64,
    end: f64,
}

impl ParameterDomain {
    /// Returns the lower parameter bound.
    #[inline(always)]
    pub(super) const fn start(self) -> f64 { self.start }

    /// Returns the upper parameter bound.
    #[inline(always)]
    pub(super) const fn end(self) -> f64 { self.end }

    /// Returns the positive parameter span.
    #[inline(always)]
    pub(super) fn span(self) -> f64 { self.end - self.start }

    /// Maps a normalized coordinate to this domain.
    #[inline(always)]
    pub(super) fn parameter(self, normalized: f64) -> f64 { self.start + normalized * self.span() }
}

/// Canonical coordinates and control-net layout for one [`SurfaceBoundary`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct BoundaryFrame {
    boundary: SurfaceBoundary,
    u_domain: ParameterDomain,
    v_domain: ParameterDomain,
    u_degree: usize,
    v_degree: usize,
    u_control_count: usize,
    v_control_count: usize,
}

impl BoundaryFrame {
    /// Builds a validated frame for a tensor-product [`NurbsSurface`].
    pub(super) fn try_new(
        surface: &NurbsSurface<Vector4>,
        boundary: SurfaceBoundary,
    ) -> Result<Self, BoundaryFrameError> {
        let control_points = surface.control_points();
        let v_control_count = control_points
            .first()
            .map(Vec::len)
            .filter(|count| *count > 0)
            .ok_or(BoundaryFrameError::EmptyControlNet)?;
        if control_points
            .iter()
            .any(|row| row.len() != v_control_count)
        {
            Err(BoundaryFrameError::NonRectangularControlNet)
        } else {
            let u_control_count = control_points.len();
            let (u_degree, u_domain) =
                validate_axis(surface.knot_vector_u(), u_control_count, SurfaceAxis::U)?;
            let (v_degree, v_domain) =
                validate_axis(surface.knot_vector_v(), v_control_count, SurfaceAxis::V)?;
            Ok(Self {
                boundary,
                u_domain,
                v_domain,
                u_degree,
                v_degree,
                u_control_count,
                v_control_count,
            })
        }
    }

    /// Returns the selected surface boundary.
    #[inline(always)]
    #[cfg(test)]
    pub(super) const fn boundary(self) -> SurfaceBoundary { self.boundary }

    /// Returns the surface's `u` domain.
    #[inline(always)]
    pub(super) const fn u_domain(self) -> ParameterDomain { self.u_domain }

    /// Returns the surface's `v` domain.
    #[inline(always)]
    pub(super) const fn v_domain(self) -> ParameterDomain { self.v_domain }

    /// Returns the parameter axis running along the seam.
    #[inline(always)]
    pub(super) const fn along_axis(self) -> SurfaceAxis { self.boundary.boundary_axis() }

    /// Returns the parameter axis pointing across the seam.
    #[inline(always)]
    pub(super) const fn cross_axis(self) -> SurfaceAxis { self.boundary.cross_axis() }

    /// Returns the domain running along the seam.
    #[inline(always)]
    pub(super) const fn along_domain(self) -> ParameterDomain {
        match self.along_axis() {
            SurfaceAxis::U => self.u_domain,
            SurfaceAxis::V => self.v_domain,
        }
    }

    /// Returns the domain running across the seam.
    #[inline(always)]
    pub(super) const fn cross_domain(self) -> ParameterDomain {
        match self.cross_axis() {
            SurfaceAxis::U => self.u_domain,
            SurfaceAxis::V => self.v_domain,
        }
    }

    /// Returns the polynomial degree along the seam.
    #[inline(always)]
    pub(super) const fn along_degree(self) -> usize {
        match self.along_axis() {
            SurfaceAxis::U => self.u_degree,
            SurfaceAxis::V => self.v_degree,
        }
    }

    /// Returns the polynomial degree across the seam.
    #[inline(always)]
    #[cfg(test)]
    pub(super) const fn cross_degree(self) -> usize {
        match self.cross_axis() {
            SurfaceAxis::U => self.u_degree,
            SurfaceAxis::V => self.v_degree,
        }
    }

    /// Returns the control-point count along the seam.
    #[inline(always)]
    pub(super) const fn along_control_count(self) -> usize {
        match self.along_axis() {
            SurfaceAxis::U => self.u_control_count,
            SurfaceAxis::V => self.v_control_count,
        }
    }

    /// Returns the control-point count across the seam.
    #[inline(always)]
    pub(super) const fn cross_control_count(self) -> usize {
        match self.cross_axis() {
            SurfaceAxis::U => self.u_control_count,
            SurfaceAxis::V => self.v_control_count,
        }
    }

    /// Returns the surface's `u` control-point count.
    #[inline(always)]
    pub(super) const fn u_control_count(self) -> usize { self.u_control_count }

    /// Returns the surface's `v` control-point count.
    #[inline(always)]
    pub(super) const fn v_control_count(self) -> usize { self.v_control_count }

    /// Returns the positive scale from normalized seam coordinates.
    #[inline(always)]
    pub(super) fn along_parameter_span(self) -> f64 { self.along_domain().span() }

    /// Returns the positive scale from normalized cross-boundary coordinates.
    #[inline(always)]
    pub(super) fn cross_parameter_span(self) -> f64 { self.cross_domain().span() }

    /// Returns the sign of increasing normalized inward distance.
    #[inline(always)]
    pub(super) const fn inward_parameter_sign(self) -> f64 {
        match self.boundary {
            SurfaceBoundary::UStart | SurfaceBoundary::VStart => 1.0,
            SurfaceBoundary::UEnd | SurfaceBoundary::VEnd => -1.0,
        }
    }

    /// Returns the signed scale from normalized inward distance.
    #[inline(always)]
    pub(super) fn inward_parameter_scale(self) -> f64 {
        self.inward_parameter_sign() * self.cross_parameter_span()
    }

    /// Maps normalized seam and inward coordinates to surface parameters.
    pub(super) fn parameters(self, seam: f64, inward: f64) -> (f64, f64) {
        let along = self.along_domain().parameter(seam);
        let cross = match self.inward_parameter_sign().is_sign_positive() {
            true => self.cross_domain().parameter(inward),
            false => self.cross_domain().parameter(1.0 - inward),
        };
        match self.boundary {
            SurfaceBoundary::UStart | SurfaceBoundary::UEnd => (cross, along),
            SurfaceBoundary::VStart | SurfaceBoundary::VEnd => (along, cross),
        }
    }

    /// Selects one control point by boundary distance and seam index.
    pub(super) fn control_point_index(
        self,
        boundary_distance: usize,
        seam_index: usize,
    ) -> Option<(usize, usize)> {
        if boundary_distance >= self.cross_control_count()
            || seam_index >= self.along_control_count()
        {
            None
        } else {
            Some(match self.boundary {
                SurfaceBoundary::UStart => (boundary_distance, seam_index),
                SurfaceBoundary::UEnd => (self.u_control_count - 1 - boundary_distance, seam_index),
                SurfaceBoundary::VStart => (seam_index, boundary_distance),
                SurfaceBoundary::VEnd => (seam_index, self.v_control_count - 1 - boundary_distance),
            })
        }
    }

    /// Selects a boundary strip in boundary-distance then seam order.
    pub(super) fn control_strip_indices(
        self,
        boundary_rows: usize,
    ) -> Result<Vec<(usize, usize)>, BoundaryFrameError> {
        if boundary_rows > self.cross_control_count() {
            Err(BoundaryFrameError::StripExceedsControlNet {
                requested: boundary_rows,
                available: self.cross_control_count(),
            })
        } else {
            Ok((0..boundary_rows)
                .flat_map(|boundary_distance| {
                    (0..self.along_control_count()).filter_map(move |seam_index| {
                        self.control_point_index(boundary_distance, seam_index)
                    })
                })
                .collect())
        }
    }
}

/// Maps a normalized seam coordinate according to [`BoundaryAlignment`].
#[inline(always)]
#[cfg(test)]
pub(super) const fn map_normalized_seam(seam: f64, alignment: BoundaryAlignment) -> f64 {
    match alignment {
        BoundaryAlignment::Aligned => seam,
        BoundaryAlignment::Reversed => 1.0 - seam,
    }
}

/// Returns the normalized derivative sign for a [`BoundaryAlignment`].
#[inline(always)]
#[cfg(test)]
pub(super) const fn seam_alignment_sign(alignment: BoundaryAlignment) -> f64 {
    match alignment {
        BoundaryAlignment::Aligned => 1.0,
        BoundaryAlignment::Reversed => -1.0,
    }
}

fn validate_axis(
    knots: &KnotVector,
    control_count: usize,
    axis: SurfaceAxis,
) -> Result<(usize, ParameterDomain), BoundaryFrameError> {
    let knot_count = knots.len();
    let degree =
        knot_count
            .checked_sub(control_count.checked_add(1).ok_or(
                BoundaryFrameError::InvalidKnotCount {
                    axis,
                    knots: knot_count,
                    controls: control_count,
                },
            )?)
            .ok_or(BoundaryFrameError::InvalidKnotCount {
                axis,
                knots: knot_count,
                controls: control_count,
            })?;
    let values = knots.as_slice();
    if values.iter().any(|knot| !knot.is_finite()) {
        Err(BoundaryFrameError::NonFiniteKnotDomain { axis })
    } else if values.windows(2).any(|pair| pair[0] > pair[1]) {
        Err(BoundaryFrameError::NonMonotonicKnotVector { axis })
    } else if values.is_empty() || !knots.is_clamped(degree) {
        Err(BoundaryFrameError::UnclampedKnotVector { axis })
    } else {
        let start = values
            .get(degree)
            .copied()
            .ok_or(BoundaryFrameError::InvalidKnotCount {
                axis,
                knots: knot_count,
                controls: control_count,
            })?;
        let end =
            values
                .get(control_count)
                .copied()
                .ok_or(BoundaryFrameError::InvalidKnotCount {
                    axis,
                    knots: knot_count,
                    controls: control_count,
                })?;
        if end > start {
            Ok((degree, ParameterDomain { start, end }))
        } else {
            Err(BoundaryFrameError::NonPositiveParameterDomain { axis })
        }
    }
}

/// Failure to construct or query a canonical [`BoundaryFrame`].
#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub(super) enum BoundaryFrameError {
    /// The surface has no rectangular control net.
    #[error("NURBS surface control net must be nonempty")]
    EmptyControlNet,
    /// Control-net rows do not share one length.
    #[error("NURBS surface control net must be rectangular")]
    NonRectangularControlNet,
    /// A knot count cannot represent the corresponding control axis.
    #[error("{axis:?} knot count {knots} cannot represent {controls} control points")]
    InvalidKnotCount {
        /// Invalid parameter axis.
        axis: SurfaceAxis,
        /// Actual knot count.
        knots: usize,
        /// Actual control-point count.
        controls: usize,
    },
    /// A knot domain contains a non-finite value.
    #[error("{axis:?} knot domain contains a non-finite value")]
    NonFiniteKnotDomain {
        /// Invalid parameter axis.
        axis: SurfaceAxis,
    },
    /// A knot vector decreases.
    #[error("{axis:?} knot vector must be nondecreasing")]
    NonMonotonicKnotVector {
        /// Invalid parameter axis.
        axis: SurfaceAxis,
    },
    /// A knot vector is not clamped at both ends.
    #[error("{axis:?} knot vector must be clamped")]
    UnclampedKnotVector {
        /// Invalid parameter axis.
        axis: SurfaceAxis,
    },
    /// A surface parameter domain has zero or negative span.
    #[error("{axis:?} parameter domain must have positive span")]
    NonPositiveParameterDomain {
        /// Invalid parameter axis.
        axis: SurfaceAxis,
    },
    /// A requested boundary strip exceeds the control net.
    #[error("requested {requested} boundary rows, but only {available} are available")]
    StripExceedsControlNet {
        /// Requested boundary-row count.
        requested: usize,
        /// Available cross-boundary control-point count.
        available: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nurbs::BsplineSurface;

    fn surface() -> NurbsSurface<Vector4> {
        let u_knots = KnotVector::from(vec![2.0, 2.0, 2.0, 3.0, 5.0, 5.0, 5.0]);
        let v_knots = KnotVector::from(vec![-4.0, -4.0, 0.0, 7.0, 7.0]);
        let control_points = (0..4)
            .map(|u| {
                (0..3)
                    .map(|v| Vector4::new(u as f64, v as f64, (u + v) as f64, 1.0))
                    .collect()
            })
            .collect();
        NurbsSurface::new(BsplineSurface::new((u_knots, v_knots), control_points))
    }

    fn frame(boundary: SurfaceBoundary) -> BoundaryFrame {
        BoundaryFrame::try_new(&surface(), boundary)
            .expect("the test surface has finite clamped domains")
    }

    fn assert_close(actual: f64, expected: f64) {
        assert!((actual - expected).abs() <= f64::EPSILON);
    }

    #[test]
    fn all_boundaries_map_normalized_coordinates_inward() {
        let cases = [
            (SurfaceBoundary::UStart, (3.5, -1.25), 11.0, 3.0),
            (SurfaceBoundary::UEnd, (3.5, -1.25), 11.0, -3.0),
            (SurfaceBoundary::VStart, (2.75, 1.5), 3.0, 11.0),
            (SurfaceBoundary::VEnd, (2.75, 1.5), 3.0, -11.0),
        ];
        cases
            .into_iter()
            .for_each(|(boundary, expected, along_span, inward_scale)| {
                let frame = frame(boundary);
                let actual = frame.parameters(0.25, 0.5);
                assert_close(actual.0, expected.0);
                assert_close(actual.1, expected.1);
                assert_close(frame.along_parameter_span(), along_span);
                assert_close(frame.inward_parameter_scale(), inward_scale);
            });
    }

    #[test]
    fn frames_expose_axis_degree_and_control_layout() {
        let u_boundary = frame(SurfaceBoundary::UStart);
        assert_eq!(u_boundary.boundary(), SurfaceBoundary::UStart);
        assert_eq!(u_boundary.along_axis(), SurfaceAxis::V);
        assert_eq!(u_boundary.cross_axis(), SurfaceAxis::U);
        assert_eq!(u_boundary.along_degree(), 1);
        assert_eq!(u_boundary.cross_degree(), 2);
        assert_eq!(u_boundary.along_control_count(), 3);
        assert_eq!(u_boundary.cross_control_count(), 4);
        assert_eq!(
            u_boundary.u_domain(),
            ParameterDomain {
                start: 2.0,
                end: 5.0
            }
        );
        assert_eq!(
            u_boundary.v_domain(),
            ParameterDomain {
                start: -4.0,
                end: 7.0
            }
        );
        assert_close(u_boundary.u_domain().start(), 2.0);
        assert_close(u_boundary.u_domain().end(), 5.0);

        let v_boundary = frame(SurfaceBoundary::VStart);
        assert_eq!(v_boundary.along_axis(), SurfaceAxis::U);
        assert_eq!(v_boundary.cross_axis(), SurfaceAxis::V);
        assert_eq!(v_boundary.along_degree(), 2);
        assert_eq!(v_boundary.cross_degree(), 1);
        assert_eq!(v_boundary.along_control_count(), 4);
        assert_eq!(v_boundary.cross_control_count(), 3);
    }

    #[test]
    fn reversed_alignment_reflects_normalized_seam_and_derivative_sign() {
        assert_close(map_normalized_seam(0.2, BoundaryAlignment::Aligned), 0.2);
        assert_close(map_normalized_seam(0.2, BoundaryAlignment::Reversed), 0.8);
        assert_close(seam_alignment_sign(BoundaryAlignment::Aligned), 1.0);
        assert_close(seam_alignment_sign(BoundaryAlignment::Reversed), -1.0);
    }

    #[test]
    fn strip_indices_use_boundary_distance_then_seam_order() {
        assert_eq!(
            frame(SurfaceBoundary::UStart)
                .control_strip_indices(2)
                .expect("two rows fit"),
            vec![(0, 0), (0, 1), (0, 2), (1, 0), (1, 1), (1, 2)],
        );
        assert_eq!(
            frame(SurfaceBoundary::UEnd)
                .control_strip_indices(2)
                .expect("two rows fit"),
            vec![(3, 0), (3, 1), (3, 2), (2, 0), (2, 1), (2, 2)],
        );
        assert_eq!(
            frame(SurfaceBoundary::VStart)
                .control_strip_indices(2)
                .expect("two rows fit"),
            vec![
                (0, 0),
                (1, 0),
                (2, 0),
                (3, 0),
                (0, 1),
                (1, 1),
                (2, 1),
                (3, 1),
            ],
        );
        assert_eq!(
            frame(SurfaceBoundary::VEnd)
                .control_strip_indices(2)
                .expect("two rows fit"),
            vec![
                (0, 2),
                (1, 2),
                (2, 2),
                (3, 2),
                (0, 1),
                (1, 1),
                (2, 1),
                (3, 1),
            ],
        );
    }

    #[test]
    fn invalid_surface_layouts_return_errors_without_panicking() {
        let valid_knots = KnotVector::bezier_knot(1);
        let empty = NurbsSurface::new(BsplineSurface::new_unchecked(
            (valid_knots.clone(), valid_knots.clone()),
            Vec::<Vec<Vector4>>::new(),
        ));
        assert_eq!(
            BoundaryFrame::try_new(&empty, SurfaceBoundary::UStart),
            Err(BoundaryFrameError::EmptyControlNet),
        );

        let points = vec![
            vec![Vector4::new(0.0, 0.0, 0.0, 1.0); 3],
            vec![Vector4::new(0.0, 0.0, 0.0, 1.0); 3],
            vec![Vector4::new(0.0, 0.0, 0.0, 1.0); 3],
        ];
        let unclamped = NurbsSurface::new(BsplineSurface::new_unchecked(
            (
                KnotVector::from(vec![0.0, 1.0, 2.0, 3.0, 4.0]),
                KnotVector::from(vec![0.0, 0.0, 1.0, 2.0, 2.0]),
            ),
            points.clone(),
        ));
        assert_eq!(
            BoundaryFrame::try_new(&unclamped, SurfaceBoundary::UStart),
            Err(BoundaryFrameError::UnclampedKnotVector {
                axis: SurfaceAxis::U,
            }),
        );

        let nonrectangular = NurbsSurface::new(BsplineSurface::new_unchecked(
            (valid_knots.clone(), valid_knots),
            vec![
                vec![Vector4::new(0.0, 0.0, 0.0, 1.0); 2],
                vec![Vector4::new(0.0, 0.0, 0.0, 1.0)],
            ],
        ));
        assert_eq!(
            BoundaryFrame::try_new(&nonrectangular, SurfaceBoundary::UStart),
            Err(BoundaryFrameError::NonRectangularControlNet),
        );
    }

    #[test]
    fn oversized_strips_return_a_typed_error() {
        assert_eq!(
            frame(SurfaceBoundary::VStart).control_strip_indices(4),
            Err(BoundaryFrameError::StripExceedsControlNet {
                requested: 4,
                available: 3,
            }),
        );
    }
}
