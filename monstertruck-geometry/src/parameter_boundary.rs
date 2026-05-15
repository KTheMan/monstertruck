use crate::prelude::*;

/// Face-local 2D trim curve used to preserve better boundary structure than a
/// raw UV polyline.
#[derive(
    Clone,
    Debug,
    ParametricCurve,
    BoundedCurve,
    ParameterDivision1D,
    Cut,
    Invertible,
    SearchNearestParameterD1,
    SearchParameterD1,
)]
pub enum BoundaryCurve2D {
    /// Linear trim segment in UV.
    Line(Line<Point2>),
    /// Degree-1 B-spline trim through sampled UV points.
    BsplineCurve(BsplineCurve<Point2>),
    /// Rational trim curve in UV.
    NurbsCurve(NurbsCurve<Vector3>),
}

impl<S> ParameterBoundary2D<S> for Line<Point3> {}

impl<S> ParameterBoundary2D<S> for BsplineCurve<Point3> {}

impl<S> ParameterBoundary2D<S> for NurbsCurve<Vector4> {}

impl<C, S> ExactParameterBoundary2D<S> for ParameterCurve<C, S>
where
    C: ParametricCurve2D<Point = Point2>
        + BoundedCurve<Point = Point2>
        + SearchParameter<CurveParameter, Point = Point2>
        + SearchNearestParameter<CurveParameter, Point = Point2>
        + Cut
        + Invertible
        + Clone,
    S: ParametricSurface3D
        + SearchParameter<SurfaceParameter, Point = Point3>
        + SearchNearestParameter<SurfaceParameter, Point = Point3>
        + Clone
        + PartialEq,
{
    type BoundaryCurve = Self;

    fn exact_parameter_boundary_2d(&self, surface: &S) -> Option<Self::BoundaryCurve> {
        (self.surface() == surface).then(|| self.clone())
    }
}

impl<S: Clone> BoundaryCurveFromSamples<S> for ParameterCurve<Line<Point2>, S> {
    fn boundary_curve_from_samples(surface: &S, points: Vec<Point2>) -> Option<Self> {
        (points.len() >= 2).then(|| {
            let front = points.first().copied().unwrap();
            let back = points.last().copied().unwrap();
            ParameterCurve::new(Line(front, back), surface.clone())
        })
    }
}

impl<S: Clone> BoundaryCurveFromSamples<S> for ParameterCurve<BoundaryCurve2D, S> {
    fn boundary_curve_from_samples(surface: &S, points: Vec<Point2>) -> Option<Self> {
        if points.len() < 2 {
            None
        } else {
            let front = points.first().copied().unwrap();
            let back = points.last().copied().unwrap();
            let line = Line(front, back);
            let is_linear = points.iter().copied().all(|point| {
                line.search_nearest_parameter(point, None, 1)
                    .is_some_and(|t| line.evaluate(t).near(&point))
            });
            Some(ParameterCurve::new(
                if is_linear {
                    BoundaryCurve2D::Line(line)
                } else {
                    let denom = (points.len() - 1) as f64;
                    let knot_vec = KnotVector::from(
                        std::iter::once(0.0)
                            .chain((0..points.len()).map(|index| index as f64 / denom))
                            .chain(std::iter::once(1.0))
                            .collect::<Vec<_>>(),
                    );
                    BoundaryCurve2D::BsplineCurve(BsplineCurve::new(knot_vec, points))
                },
                surface.clone(),
            ))
        }
    }
}

impl<C, S> ParameterBoundary2D<S> for ParameterCurve<C, S>
where
    C: ParametricCurve2D + BoundedCurve + ParameterDivision1D<Point = Point2>,
    S: PartialEq,
{
    fn parameter_boundary_2d(&self, surface: &S, tolerance: f64) -> Option<Vec<Point2>> {
        (self.surface() == surface).then(|| {
            self.curve()
                .parameter_division(self.curve().range_tuple(), tolerance)
                .1
        })
    }
}

impl<C, S> ParameterBoundary2D<S> for TrimmedCurve<C>
where C: ParameterBoundary2D<S>
{
    fn parameter_boundary_2d(&self, surface: &S, tolerance: f64) -> Option<Vec<Point2>> {
        self.curve().parameter_boundary_2d(surface, tolerance)
    }
}

impl<C, T, S> ParameterBoundary2D<S> for Processor<C, T> where Processor<C, T>: ParametricCurve3D {}

fn exact_line_boundary_on_plane(
    line: &Line<Point3>,
    plane: &Plane,
) -> Option<ParameterCurve<Line<Point2>, Plane>> {
    let (u0, v0) = plane.search_parameter(line.front(), None, 1)?;
    let (u1, v1) = plane.search_parameter(line.back(), None, 1)?;
    let boundary = ParameterCurve::new(Line(Point2::new(u0, v0), Point2::new(u1, v1)), *plane);
    boundary
        .evaluate(0.5)
        .near(&line.evaluate(0.5))
        .then_some(boundary)
}

fn exact_boundary_segment<C, B, P>(curve: &C, boundary: &B) -> Option<(f64, f64)>
where
    C: ParametricCurve<Point = P> + BoundedCurve<Point = P>,
    B: ParametricCurve<Point = P>
        + BoundedCurve<Point = P>
        + SearchParameter<CurveParameter, Point = P>,
    P: Copy, {
    let (t0, t1) = curve.range_tuple();
    let samples = [
        t0,
        (3.0 * t0 + t1) * 0.25,
        (t0 + t1) * 0.5,
        (t0 + 3.0 * t1) * 0.25,
        t1,
    ];
    samples
        .into_iter()
        .all(|t| {
            boundary
                .search_parameter(curve.evaluate(t), None, 100)
                .is_some()
        })
        .then(|| {
            let front = boundary.search_parameter(curve.front(), None, 100)?;
            let back = boundary
                .search_parameter(curve.back(), Some(front), 100)
                .or_else(|| boundary.search_parameter(curve.back(), None, 100))?;
            Some((front, back))
        })
        .flatten()
}

fn exact_boundary_line_on_surface<C, B, P>(
    curve: &C,
    last_u: usize,
    last_v: usize,
    column_curve: impl Fn(usize) -> B,
    row_curve: impl Fn(usize) -> B,
    ((u0, u1), (v0, v1)): ((f64, f64), (f64, f64)),
) -> Option<Line<Point2>>
where
    C: ParametricCurve<Point = P>
        + BoundedCurve<Point = P>
        + SearchParameter<CurveParameter, Point = P>,
    P: Copy,
    B: ParametricCurve<Point = P>
        + BoundedCurve<Point = P>
        + SearchParameter<CurveParameter, Point = P>,
{
    exact_boundary_segment(curve, &column_curve(0))
        .map(|(s0, s1)| Line(Point2::new(u0, s0), Point2::new(u0, s1)))
        .or_else(|| {
            exact_boundary_segment(curve, &column_curve(last_u))
                .map(|(s0, s1)| Line(Point2::new(u1, s0), Point2::new(u1, s1)))
        })
        .or_else(|| {
            exact_boundary_segment(curve, &row_curve(0))
                .map(|(s0, s1)| Line(Point2::new(s0, v0), Point2::new(s1, v0)))
        })
        .or_else(|| {
            exact_boundary_segment(curve, &row_curve(last_v))
                .map(|(s0, s1)| Line(Point2::new(s0, v1), Point2::new(s1, v1)))
        })
}

fn exact_bspline_boundary_on_surface(
    curve: &BsplineCurve<Point3>,
    surface: &BsplineSurface<Point3>,
) -> Option<ParameterCurve<Line<Point2>, BsplineSurface<Point3>>> {
    let (u_range, v_range) = surface.try_range_tuple();
    let ((u0, u1), (v0, v1)) = (u_range?, v_range?);
    let last_u = surface.control_points().len().checked_sub(1)?;
    let last_v = surface.control_points().first()?.len().checked_sub(1)?;
    let boundary = exact_boundary_line_on_surface(
        curve,
        last_u,
        last_v,
        |index| surface.column_curve(index),
        |index| surface.row_curve(index),
        ((u0, u1), (v0, v1)),
    )?;
    Some(ParameterCurve::new(boundary, surface.clone()))
}

fn exact_nurbs_boundary_on_surface(
    curve: &NurbsCurve<Vector4>,
    surface: &NurbsSurface<Vector4>,
) -> Option<ParameterCurve<Line<Point2>, NurbsSurface<Vector4>>> {
    let (u_range, v_range) = surface.try_range_tuple();
    let ((u0, u1), (v0, v1)) = (u_range?, v_range?);
    let last_u = surface.control_points().len().checked_sub(1)?;
    let last_v = surface.control_points().first()?.len().checked_sub(1)?;
    let boundary = exact_boundary_line_on_surface(
        curve,
        last_u,
        last_v,
        |index| surface.column_curve(index),
        |index| surface.row_curve(index),
        ((u0, u1), (v0, v1)),
    )?;
    Some(ParameterCurve::new(boundary, surface.clone()))
}

fn exact_boundary_on_homogeneous_surface<C, S>(
    curve: &C,
    surface: &S,
) -> Option<ParameterCurve<Line<Point2>, S>>
where
    C: ParametricCurve3D
        + BoundedCurve<Point = Point3>
        + SearchParameter<CurveParameter, Point = Point3>
        + TryIntoHomogeneousBsplineCurve,
    S: Clone + ParametricSurface3D + TryIntoHomogeneousBsplineSurface,
{
    let hom_surface = surface.try_into_homogeneous_bspline_surface()?;
    let (u_range, v_range) = surface.try_range_tuple();
    let ranges = (u_range?, v_range?);
    let last_u = hom_surface.control_points().len().checked_sub(1)?;
    let last_v = hom_surface.control_points().first()?.len().checked_sub(1)?;
    let boundary = exact_boundary_line_on_surface(
        curve,
        last_u,
        last_v,
        |index| NurbsCurve::new(hom_surface.column_curve(index)),
        |index| NurbsCurve::new(hom_surface.row_curve(index)),
        ranges,
    )?;
    Some(ParameterCurve::new(boundary, surface.clone()))
}

impl ExactParameterBoundary2D<Plane> for Line<Point3> {
    type BoundaryCurve = ParameterCurve<Line<Point2>, Plane>;

    fn exact_parameter_boundary_2d(&self, surface: &Plane) -> Option<Self::BoundaryCurve> {
        exact_line_boundary_on_plane(self, surface)
    }
}

impl ExactParameterBoundary2D<BsplineSurface<Point3>> for BsplineCurve<Point3> {
    type BoundaryCurve = ParameterCurve<Line<Point2>, BsplineSurface<Point3>>;

    fn exact_parameter_boundary_2d(
        &self,
        surface: &BsplineSurface<Point3>,
    ) -> Option<Self::BoundaryCurve> {
        exact_bspline_boundary_on_surface(self, surface)
    }
}

impl ExactParameterBoundary2D<NurbsSurface<Vector4>> for NurbsCurve<Vector4> {
    type BoundaryCurve = ParameterCurve<Line<Point2>, NurbsSurface<Vector4>>;

    fn exact_parameter_boundary_2d(
        &self,
        surface: &NurbsSurface<Vector4>,
    ) -> Option<Self::BoundaryCurve> {
        exact_nurbs_boundary_on_surface(self, surface)
    }
}

impl<C> ExactParameterBoundary2D<RevolutionSurface<C>> for Line<Point3>
where C: Clone + ParametricCurve3D + BoundedCurve + TryIntoHomogeneousBsplineCurve
{
    type BoundaryCurve = ParameterCurve<Line<Point2>, RevolutionSurface<C>>;

    fn exact_parameter_boundary_2d(
        &self,
        surface: &RevolutionSurface<C>,
    ) -> Option<Self::BoundaryCurve> {
        exact_boundary_on_homogeneous_surface(self, surface)
    }
}

impl<C> ExactParameterBoundary2D<RevolutionSurface<C>> for BsplineCurve<Point3>
where C: Clone + ParametricCurve3D + BoundedCurve + TryIntoHomogeneousBsplineCurve
{
    type BoundaryCurve = ParameterCurve<Line<Point2>, RevolutionSurface<C>>;

    fn exact_parameter_boundary_2d(
        &self,
        surface: &RevolutionSurface<C>,
    ) -> Option<Self::BoundaryCurve> {
        exact_boundary_on_homogeneous_surface(self, surface)
    }
}

impl<C> ExactParameterBoundary2D<RevolutionSurface<C>> for NurbsCurve<Vector4>
where C: Clone + ParametricCurve3D + BoundedCurve + TryIntoHomogeneousBsplineCurve
{
    type BoundaryCurve = ParameterCurve<Line<Point2>, RevolutionSurface<C>>;

    fn exact_parameter_boundary_2d(
        &self,
        surface: &RevolutionSurface<C>,
    ) -> Option<Self::BoundaryCurve> {
        exact_boundary_on_homogeneous_surface(self, surface)
    }
}

impl<C> ExactParameterBoundary2D<Processor<RevolutionSurface<C>, Matrix4>> for Line<Point3>
where C: Clone + ParametricCurve3D + BoundedCurve + TryIntoHomogeneousBsplineCurve
{
    type BoundaryCurve = ParameterCurve<Line<Point2>, Processor<RevolutionSurface<C>, Matrix4>>;

    fn exact_parameter_boundary_2d(
        &self,
        surface: &Processor<RevolutionSurface<C>, Matrix4>,
    ) -> Option<Self::BoundaryCurve> {
        exact_boundary_on_homogeneous_surface(self, surface)
    }
}

impl<C> ExactParameterBoundary2D<Processor<RevolutionSurface<C>, Matrix4>> for BsplineCurve<Point3>
where C: Clone + ParametricCurve3D + BoundedCurve + TryIntoHomogeneousBsplineCurve
{
    type BoundaryCurve = ParameterCurve<Line<Point2>, Processor<RevolutionSurface<C>, Matrix4>>;

    fn exact_parameter_boundary_2d(
        &self,
        surface: &Processor<RevolutionSurface<C>, Matrix4>,
    ) -> Option<Self::BoundaryCurve> {
        exact_boundary_on_homogeneous_surface(self, surface)
    }
}

impl<C> ExactParameterBoundary2D<Processor<RevolutionSurface<C>, Matrix4>> for NurbsCurve<Vector4>
where C: Clone + ParametricCurve3D + BoundedCurve + TryIntoHomogeneousBsplineCurve
{
    type BoundaryCurve = ParameterCurve<Line<Point2>, Processor<RevolutionSurface<C>, Matrix4>>;

    fn exact_parameter_boundary_2d(
        &self,
        surface: &Processor<RevolutionSurface<C>, Matrix4>,
    ) -> Option<Self::BoundaryCurve> {
        exact_boundary_on_homogeneous_surface(self, surface)
    }
}

impl<C, S0, S1, T, S> ExactParameterBoundary2D<S> for SurfaceCurve<C, S0, S1, T, T>
where
    T: ParametricCurve3D<Point = Point3>
        + BoundedCurve<Point = Point3>
        + SearchNearestParameter<CurveParameter, Point = Point3>
        + Cut
        + Invertible
        + Clone,
    S0: PartialEq<S>,
    S1: PartialEq<S>,
{
    type BoundaryCurve = T;

    fn exact_parameter_boundary_2d(&self, surface: &S) -> Option<Self::BoundaryCurve> {
        if self.surface0() == surface {
            self.boundary0().cloned()
        } else if self.surface1() == surface {
            self.boundary1().cloned()
        } else {
            None
        }
    }
}

impl<C, S0, S1, T0, T1, S> ParameterBoundary2D<S> for SurfaceCurve<C, S0, S1, T0, T1>
where
    C: ParametricCurve3D + BoundedCurve,
    S0: ParametricSurface3D
        + SearchNearestParameter<SurfaceParameter, Point = Point3>
        + PartialEq<S>,
    S1: ParametricSurface3D
        + SearchNearestParameter<SurfaceParameter, Point = Point3>
        + PartialEq<S>,
    T0: ParameterBoundary2D<S>,
    T1: ParameterBoundary2D<S>,
    T0: Clone,
    T1: Clone,
{
    fn parameter_boundary_2d(&self, surface: &S, tolerance: f64) -> Option<Vec<Point2>> {
        if self.surface0() == surface {
            self.boundary0()
                .and_then(|boundary| boundary.parameter_boundary_2d(surface, tolerance))
                .or_else(|| {
                    self.parameter_division(self.range_tuple(), tolerance)
                        .0
                        .into_iter()
                        .map(|t| self.search_triple(t, 100).map(|(_, uv0, _)| uv0))
                        .collect()
                })
        } else if self.surface1() == surface {
            self.boundary1()
                .and_then(|boundary| boundary.parameter_boundary_2d(surface, tolerance))
                .or_else(|| {
                    self.parameter_division(self.range_tuple(), tolerance)
                        .0
                        .into_iter()
                        .map(|t| self.search_triple(t, 100).map(|(_, _, uv1)| uv1))
                        .collect()
                })
        } else {
            None
        }
    }
}
