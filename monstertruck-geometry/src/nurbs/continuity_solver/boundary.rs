//! Canonical boundary coordinates for tensor-product NURBS surfaces.

use thiserror::Error;

use crate::base::Vector4;
use crate::nurbs::continuity::{BoundaryAlignment, BoundarySide, SurfaceAxis};
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

/// Canonical coordinates and control-net layout for one [`BoundarySide`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct BoundaryFrame {
    boundary: BoundarySide,
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
        boundary: BoundarySide,
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
    pub(super) const fn boundary(self) -> BoundarySide { self.boundary }

    /// Returns the surface's `u` domain.
    #[inline(always)]
    pub(super) const fn domain_u(self) -> ParameterDomain { self.u_domain }

    /// Returns the surface's `v` domain.
    #[inline(always)]
    pub(super) const fn domain_v(self) -> ParameterDomain { self.v_domain }

    #[cfg(test)]
    pub(super) const fn u_domain(self) -> ParameterDomain { self.domain_u() }

    #[cfg(test)]
    pub(super) const fn v_domain(self) -> ParameterDomain { self.domain_v() }

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
    pub(super) const fn control_count_u(self) -> usize { self.u_control_count }

    /// Returns the surface's `v` control-point count.
    #[inline(always)]
    pub(super) const fn control_count_v(self) -> usize { self.v_control_count }

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
            BoundarySide::MinU | BoundarySide::MinV => 1.0,
            BoundarySide::MaxU | BoundarySide::MaxV => -1.0,
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
            BoundarySide::MinU | BoundarySide::MaxU => (cross, along),
            BoundarySide::MinV | BoundarySide::MaxV => (along, cross),
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
                BoundarySide::MinU => (boundary_distance, seam_index),
                BoundarySide::MaxU => (self.u_control_count - 1 - boundary_distance, seam_index),
                BoundarySide::MinV => (seam_index, boundary_distance),
                BoundarySide::MaxV => (seam_index, self.v_control_count - 1 - boundary_distance),
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
mod tests;
