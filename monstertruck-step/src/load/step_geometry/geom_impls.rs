use super::*;
use monstertruck_geometry::prelude::{
    SupportsExactPatchDomains, TryIntoBsplineSurface, TryIntoHomogeneousBsplineCurve,
    TryIntoHomogeneousBsplineSurface,
};
use monstertruck_modeling::{
    Conic2D as ModelingConic2D, Curve as ModelingCurve, Curve2D as ModelingCurve2D,
    Surface as ModelingSurface,
};
use monstertruck_traits::SnapCurveEndpoints;
use std::env;

#[cfg(test)]
use std::f64::consts::TAU;

fn sampled_parameter_boundary<C>(
    curve: &C,
    surface: &Surface,
    tolerance: f64,
) -> Option<Vec<Point2>>
where
    C: ParametricCurve3D + BoundedCurve + ParameterDivision1D<Point = Point3>,
{
    fn abs_diff(previous: f64) -> impl Fn(&f64, &f64) -> std::cmp::Ordering {
        let distance = move |value: &f64| f64::abs(*value - previous);
        // SAFETY: All compared values are finite after the finiteness check in
        // `normalize_axis`.
        move |lhs: &f64, rhs: &f64| distance(lhs).partial_cmp(&distance(rhs)).unwrap()
    }

    fn normalize_axis(
        value: f64,
        previous: Option<f64>,
        period: Option<f64>,
        range: Option<(f64, f64)>,
    ) -> Option<f64> {
        if !value.is_finite() {
            None
        } else if let Some(previous) = previous {
            if let Some(period) = period {
                (-2..=2)
                    .map(|index| value + index as f64 * period)
                    .min_by(abs_diff(previous))
            } else if let Some(range) = range {
                Some(clamp_near_range(value, range))
            } else {
                Some(value)
            }
        } else if let Some((min, max)) = range {
            if let Some(period) = period {
                let span = max - min;
                if span.so_small() {
                    Some(min)
                } else {
                    let mut normalized = value - f64::floor((value - min) / period) * period;
                    if normalized < min {
                        normalized += period;
                    }
                    if normalized > max {
                        normalized -= period;
                    }
                    Some(normalized.clamp(min, max))
                }
            } else {
                Some(clamp_near_range(value, (min, max)))
            }
        } else {
            Some(value)
        }
    }

    fn clamp_near_range(value: f64, (min, max): (f64, f64)) -> f64 {
        if value < min && min - value < TOLERANCE {
            min
        } else if value > max && value - max < TOLERANCE {
            max
        } else {
            value
        }
    }

    let normalize_uv = |uv: Point2, previous: Option<(f64, f64)>| {
        let (urange, vrange) = surface.try_range_tuple();
        Some(Point2::new(
            normalize_axis(uv.x, previous.map(|(u, _)| u), surface.u_period(), urange)?,
            normalize_axis(uv.y, previous.map(|(_, v)| v), surface.v_period(), vrange)?,
        ))
    };
    let points = curve.parameter_division(curve.range_tuple(), tolerance).1;
    let project = |point: Point3, hint: Option<(f64, f64)>| {
        surface
            .search_parameter(point, hint, 100)
            .or_else(|| surface.search_parameter(point, None, 100))
            .or_else(|| surface.search_nearest_parameter(point, hint, 100))
            .or_else(|| surface.search_nearest_parameter(point, None, 100))
            .map(|(u, v)| Point2::new(u, v))
    };
    points
        .iter()
        .copied()
        .scan(None, |hint, point| {
            let uv = project(point, *hint).and_then(|uv| normalize_uv(uv, *hint));
            *hint = uv.map(|uv| (uv.x, uv.y));
            Some(uv)
        })
        .collect::<Option<Vec<_>>>()
        .or_else(|| {
            points
                .into_iter()
                .scan(None, |hint, point| {
                    let uv = project(point, *hint).and_then(|uv| normalize_uv(uv, *hint));
                    *hint = uv.map(|uv| (uv.x, uv.y));
                    Some(uv)
                })
                .collect()
        })
}

fn exact_parameter_curve_on(curve: &Curve3D, surface: &Surface) -> Option<StepParameterCurve> {
    match (curve, surface) {
        (Curve3D::ParameterCurve(curve), surface)
            if SurfaceCurve3D::same_surface(curve.surface().as_ref(), surface) =>
        {
            Some(curve.clone())
        }
        (Curve3D::SurfaceCurve(curve), surface) => curve
            .parameter_curve_on(surface)
            .cloned()
            .or_else(|| match surface {
                Surface::ElementarySurface(ElementarySurface::Plane(_)) => {
                    exact_parameter_curve_on(curve.leader(), surface)
                }
                _ => None,
            }),
        (Curve3D::Line(curve), Surface::ElementarySurface(ElementarySurface::Plane(plane))) => {
            exact_line_parameter_curve_on_plane(curve, plane, surface)
        }
        (Curve3D::Conic(curve), surface) => exact_conic_parameter_curve_on(curve, surface),
        (Curve3D::IntersectionCurve(curve), surface)
            if SurfaceCurve3D::same_surface(curve.surface0().as_ref(), surface)
                || SurfaceCurve3D::same_surface(curve.surface1().as_ref(), surface) =>
        {
            exact_parameter_curve_on(curve.leader().as_ref(), surface)
        }
        _ => None,
    }
}

fn projected_conic_transform_on_plane(transform: &Matrix4, plane: &Plane) -> Matrix3 {
    let project = |point| {
        let parameter = plane.parameter(transform.transform_point(point));
        Point2::new(parameter.x, parameter.y)
    };
    let origin = project(Point3::origin());
    let u_axis = project(Point3::new(1.0, 0.0, 0.0)) - origin;
    let v_axis = project(Point3::new(0.0, 1.0, 0.0)) - origin;
    Matrix3::from_cols(
        Vector3::new(u_axis.x, u_axis.y, 0.0),
        Vector3::new(v_axis.x, v_axis.y, 0.0),
        Vector3::new(origin.x, origin.y, 1.0),
    )
}

fn pcurve_matches_surface_curve<C>(curve: &C, trim: &Curve2D, surface: &Surface) -> bool
where C: ParametricCurve3D<Point = Point3> + BoundedCurve {
    let (t0, t1) = curve.range_tuple();
    [
        t0,
        (3.0 * t0 + t1) * 0.25,
        (t0 + t1) * 0.5,
        (t0 + 3.0 * t1) * 0.25,
        t1,
    ]
    .into_iter()
    .all(|parameter| {
        let uv = trim.evaluate(parameter);
        surface
            .evaluate(uv.x, uv.y)
            .near(&curve.evaluate(parameter))
    })
}

fn exact_ellipse_parameter_curve_on_plane(
    curve: &Ellipse<Point3, Matrix4>,
    plane: &Plane,
    surface: &Surface,
) -> Option<StepParameterCurve> {
    let trimmed = TrimmedCurve::new(UnitCircle::new(), curve.entity().range());
    let mut projected = Processor::with_transform(
        trimmed,
        projected_conic_transform_on_plane(curve.transform(), plane),
    );
    if !curve.orientation() {
        projected.invert();
    }
    let trim = Curve2D::Conic(Conic2D::Ellipse(projected));
    pcurve_matches_surface_curve(curve, &trim, surface)
        .then(|| StepParameterCurve::new(Box::new(trim), Box::new(surface.clone())))
}

fn exact_hyperbola_parameter_curve_on_plane(
    curve: &Hyperbola<Point3, Matrix4>,
    plane: &Plane,
    surface: &Surface,
) -> Option<StepParameterCurve> {
    let trimmed = TrimmedCurve::new(UnitHyperbola::new(), curve.entity().range());
    let mut projected = Processor::with_transform(
        trimmed,
        projected_conic_transform_on_plane(curve.transform(), plane),
    );
    if !curve.orientation() {
        projected.invert();
    }
    let trim = Curve2D::Conic(Conic2D::Hyperbola(projected));
    pcurve_matches_surface_curve(curve, &trim, surface)
        .then(|| StepParameterCurve::new(Box::new(trim), Box::new(surface.clone())))
}

fn exact_parabola_parameter_curve_on_plane(
    curve: &Parabola<Point3, Matrix4>,
    plane: &Plane,
    surface: &Surface,
) -> Option<StepParameterCurve> {
    let trimmed = TrimmedCurve::new(UnitParabola::new(), curve.entity().range());
    let mut projected = Processor::with_transform(
        trimmed,
        projected_conic_transform_on_plane(curve.transform(), plane),
    );
    if !curve.orientation() {
        projected.invert();
    }
    let trim = Curve2D::Conic(Conic2D::Parabola(projected));
    pcurve_matches_surface_curve(curve, &trim, surface)
        .then(|| StepParameterCurve::new(Box::new(trim), Box::new(surface.clone())))
}

fn line_parameter_curve_to_pcurve(line: Line<Point2>, surface: &Surface) -> StepParameterCurve {
    StepParameterCurve::new(Box::new(Curve2D::Line(line)), Box::new(surface.clone()))
}

fn exact_line_parameter_curve_on_plane(
    curve: &Line<Point3>,
    plane: &Plane,
    surface: &Surface,
) -> Option<StepParameterCurve> {
    let front = plane.parameter(curve.front());
    let back = plane.parameter(curve.back());
    let trim = Curve2D::Line(Line(
        Point2::new(front.x, front.y),
        Point2::new(back.x, back.y),
    ));
    pcurve_matches_surface_curve(curve, &trim, surface)
        .then(|| StepParameterCurve::new(Box::new(trim), Box::new(surface.clone())))
}

fn nearest_periodic_component(value: f64, reference: f64, period: Option<f64>) -> f64 {
    period.map_or(value, |period| {
        value + ((reference - value) / period).round() * period
    })
}

fn nearest_periodic_surface_parameter(
    point: Point2,
    reference: Point2,
    periods: (Option<f64>, Option<f64>),
) -> Point2 {
    Point2::new(
        nearest_periodic_component(point.x, reference.x, periods.0),
        nearest_periodic_component(point.y, reference.y, periods.1),
    )
}

fn exact_line_parameter_curve_by_surface_search<C>(
    curve: &C,
    surface: &Surface,
) -> Option<StepParameterCurve>
where
    C: ParametricCurve3D<Point = Point3> + BoundedCurve,
{
    let (t0, t1) = curve.range_tuple();
    let periods = (surface.u_period(), surface.v_period());
    let samples = [
        t0,
        (3.0 * t0 + t1) * 0.25,
        (t0 + t1) * 0.5,
        (t0 + 3.0 * t1) * 0.25,
        t1,
    ]
    .into_iter()
    .try_fold(Vec::with_capacity(5), |mut samples, parameter| {
        let point = curve.evaluate(parameter);
        let hint = samples
            .last()
            .map(|(_, uv): &(Point3, Point2)| (*uv).into());
        let uv = surface.search_parameter(point, hint, 30)?;
        let uv = Point2::from(uv);
        let uv = samples
            .last()
            .map(|(_, reference): &(Point3, Point2)| {
                nearest_periodic_surface_parameter(uv, *reference, periods)
            })
            .unwrap_or(uv);
        samples.push((point, uv));
        Some(samples)
    })?;
    let line = Line(samples.first()?.1, samples.last()?.1);
    (!line.0.near(&line.1)).then_some(())?;
    samples
        .iter()
        .all(|(point, uv)| {
            line.search_nearest_parameter(*uv, None, 1)
                .filter(|parameter| *parameter >= -TOLERANCE && *parameter <= 1.0 + TOLERANCE)
                .map(|parameter| line.evaluate(parameter.clamp(0.0, 1.0)))
                .is_some_and(|projected| {
                    projected.distance2(*uv) <= TOLERANCE * TOLERANCE
                        && surface.evaluate(projected.x, projected.y).near(point)
                })
        })
        .then(|| line_parameter_curve_to_pcurve(line, surface))
}

fn exact_conic_parameter_curve_on(
    curve: &Conic3D,
    surface: &Surface,
) -> Option<StepParameterCurve> {
    match (curve, surface) {
        (Conic3D::Ellipse(curve), Surface::ElementarySurface(ElementarySurface::Plane(plane))) => {
            exact_ellipse_parameter_curve_on_plane(curve, plane, surface)
        }
        (
            Conic3D::Hyperbola(curve),
            Surface::ElementarySurface(ElementarySurface::Plane(plane)),
        ) => exact_hyperbola_parameter_curve_on_plane(curve, plane, surface),
        (Conic3D::Parabola(curve), Surface::ElementarySurface(ElementarySurface::Plane(plane))) => {
            exact_parabola_parameter_curve_on_plane(curve, plane, surface)
        }
        (Conic3D::Ellipse(curve), _) => {
            exact_line_parameter_curve_by_surface_search(curve, surface)
        }
        _ => None,
    }
}

fn to_modeling_trim(
    curve: &StepParameterCurve,
) -> std::result::Result<ParameterCurve<ModelingCurve2D, Box<ModelingSurface>>, StepConvertingError>
{
    Ok(ParameterCurve::new(
        curve.curve().as_ref().try_into()?,
        Box::new(curve.surface().as_ref().try_into()?),
    ))
}

impl SurfaceCurveAssociatedGeometry {
    fn surface(&self) -> &Surface {
        match self {
            SurfaceCurveAssociatedGeometry::ParameterCurve(curve) => curve.surface().as_ref(),
            SurfaceCurveAssociatedGeometry::Surface(surface) => surface,
        }
    }
}

impl ParametricCurve for SurfaceCurve3D {
    type Point = Point3;
    type Vector = Vector3;

    fn evaluate(&self, t: f64) -> Self::Point { self.leader().evaluate(t) }

    fn derivative(&self, t: f64) -> Self::Vector { self.leader().derivative(t) }

    fn derivative_2(&self, t: f64) -> Self::Vector { self.leader().derivative_2(t) }

    fn derivative_n(&self, n: usize, t: f64) -> Self::Vector { self.leader().derivative_n(n, t) }

    fn parameter_range(&self) -> ParameterRange { self.leader().parameter_range() }

    fn period(&self) -> Option<f64> { self.leader().period() }
}

impl BoundedCurve for SurfaceCurve3D {}

impl ParameterDivision1D for SurfaceCurve3D {
    type Point = Point3;

    fn parameter_division(&self, range: (f64, f64), tol: f64) -> (Vec<f64>, Vec<Self::Point>) {
        self.leader().parameter_division(range, tol)
    }
}

impl SurfaceCurveAssociatedGeometry {
    fn split_at(&mut self, t: f64) -> Self {
        match self {
            SurfaceCurveAssociatedGeometry::ParameterCurve(curve) => {
                SurfaceCurveAssociatedGeometry::ParameterCurve(curve.cut(t))
            }
            SurfaceCurveAssociatedGeometry::Surface(surface) => {
                SurfaceCurveAssociatedGeometry::Surface(surface.clone())
            }
        }
    }
}

impl Cut for SurfaceCurve3D {
    fn cut(&mut self, t: f64) -> Self {
        let leader = Box::new(self.leader_mut().cut(t));
        let associated_geometry = self
            .associated_geometry
            .iter_mut()
            .map(|entry| entry.split_at(t))
            .collect();
        Self::new(
            self.kind(),
            leader,
            associated_geometry,
            self.master_representation(),
        )
    }
}

impl SnapCurveEndpoints for SurfaceCurve3D {
    fn snap_endpoints(&mut self, front: Point3, back: Point3) {
        self.leader_mut().snap_endpoints(front, back);
    }
}

impl SnapCurveEndpoints for Curve3D {
    fn snap_endpoints(&mut self, front: Point3, back: Point3) {
        match self {
            Curve3D::Polyline(curve) => curve.snap_endpoints(front, back),
            Curve3D::SurfaceCurve(curve) => curve.snap_endpoints(front, back),
            Curve3D::IntersectionCurve(curve) => curve.snap_endpoints(front, back),
            Curve3D::Line(_)
            | Curve3D::Conic(_)
            | Curve3D::BsplineCurve(_)
            | Curve3D::ParameterCurve(_)
            | Curve3D::NurbsCurve(_) => {}
        }
    }
}

impl Invertible for SurfaceCurveAssociatedGeometry {
    fn invert(&mut self) {
        if let SurfaceCurveAssociatedGeometry::ParameterCurve(curve) = self {
            curve.invert();
        }
    }
}

impl Invertible for SurfaceCurve3D {
    fn invert(&mut self) {
        self.leader_mut().invert();
        self.associated_geometry
            .iter_mut()
            .for_each(Invertible::invert);
    }
}

impl SearchParameter<D1> for SurfaceCurve3D {
    type Point = Point3;

    fn search_parameter<H: Into<SearchParameterHint1D>>(
        &self,
        point: Self::Point,
        hint: H,
        trials: usize,
    ) -> Option<f64> {
        self.leader().search_parameter(point, hint, trials)
    }
}

impl SearchNearestParameter<D1> for SurfaceCurve3D {
    type Point = Point3;

    fn search_nearest_parameter<H: Into<SearchParameterHint1D>>(
        &self,
        point: Self::Point,
        hint: H,
        trials: usize,
    ) -> Option<f64> {
        self.leader().search_nearest_parameter(point, hint, trials)
    }
}

impl Transformed<Matrix4> for SurfaceCurveAssociatedGeometry {
    fn transform_by(&mut self, trans: Matrix4) {
        match self {
            SurfaceCurveAssociatedGeometry::ParameterCurve(curve) => curve.transform_by(trans),
            SurfaceCurveAssociatedGeometry::Surface(surface) => surface.transform_by(trans),
        }
    }
}

impl Transformed<Matrix4> for SurfaceCurve3D {
    fn transform_by(&mut self, trans: Matrix4) {
        self.leader_mut().transform_by(trans);
        self.associated_geometry
            .iter_mut()
            .for_each(|entry| entry.transform_by(trans));
    }
}

impl ParameterDivision1D for Curve3D {
    type Point = Point3;

    fn parameter_division(&self, range: (f64, f64), tol: f64) -> (Vec<f64>, Vec<Self::Point>) {
        let debug_profile = env::var("MT_PROFILE_CURVE_DIVISION").is_ok();
        let started = std::time::Instant::now();
        let result = match self {
            Curve3D::Line(curve) => curve.parameter_division(range, tol),
            Curve3D::Polyline(curve) => curve.parameter_division(range, tol),
            Curve3D::Conic(curve) => curve.parameter_division(range, tol),
            Curve3D::BsplineCurve(curve) => curve.parameter_division(range, tol),
            Curve3D::ParameterCurve(curve) => curve.parameter_division(range, tol),
            Curve3D::SurfaceCurve(curve) => curve.parameter_division(range, tol),
            Curve3D::IntersectionCurve(curve) => curve.leader().parameter_division(range, tol),
            Curve3D::NurbsCurve(curve) => curve.parameter_division(range, tol),
        };
        if debug_profile {
            let kind = match self {
                Curve3D::Line(_) => "Line",
                Curve3D::Polyline(_) => "Polyline",
                Curve3D::Conic(_) => "Conic",
                Curve3D::BsplineCurve(_) => "BsplineCurve",
                Curve3D::ParameterCurve(_) => "StepParameterCurve",
                Curve3D::SurfaceCurve(_) => "SurfaceCurve",
                Curve3D::IntersectionCurve(_) => "IntersectionCurve",
                Curve3D::NurbsCurve(_) => "NurbsCurve",
            };
            eprintln!(
                "trace bool curve_division kind={} points={} tol={} elapsed_ms={:.3}",
                kind,
                result.1.len(),
                tol,
                started.elapsed().as_secs_f64() * 1000.0,
            );
        }
        result
    }
}

impl ToSameGeometry<Curve3D> for SurfaceCurve3D {
    fn to_same_geometry(&self) -> Curve3D { Curve3D::SurfaceCurve(self.clone()) }
}

impl From<IntersectionCurve<BsplineCurve<Point3>, Surface, Surface>> for Curve3D {
    fn from(ic: IntersectionCurve<BsplineCurve<Point3>, Surface, Surface>) -> Self {
        let (surface0, surface1, leader) = ic.destruct();
        Curve3D::IntersectionCurve(IntersectionCurve::new(
            Box::new(surface0),
            Box::new(surface1),
            Box::new(Curve3D::BsplineCurve(leader)),
        ))
    }
}

impl TryIntoHomogeneousBsplineSurface for Sphere {
    fn try_into_homogeneous_bspline_surface(&self) -> Option<BsplineSurface<Vector4>> {
        self.0.try_into_homogeneous_bspline_surface()
    }
}

impl TryIntoBsplineSurface for Sphere {
    fn try_into_bspline_surface(&self) -> Option<BsplineSurface<Point3>> {
        self.0.try_into_bspline_surface()
    }
}

impl TryIntoHomogeneousBsplineCurve for Curve3D {
    fn try_into_homogeneous_bspline_curve(&self) -> Option<BsplineCurve<Vector4>> {
        match self {
            Curve3D::Line(line) => line.try_into_homogeneous_bspline_curve(),
            Curve3D::Conic(Conic3D::Ellipse(curve)) => curve.try_into_homogeneous_bspline_curve(),
            Curve3D::Conic(_) => None,
            Curve3D::BsplineCurve(curve) => curve.try_into_homogeneous_bspline_curve(),
            Curve3D::ParameterCurve(_) => None,
            Curve3D::Polyline(_) => None,
            Curve3D::SurfaceCurve(curve) => curve.leader().try_into_homogeneous_bspline_curve(),
            Curve3D::IntersectionCurve(curve) => {
                curve.leader().try_into_homogeneous_bspline_curve()
            }
            Curve3D::NurbsCurve(curve) => curve.try_into_homogeneous_bspline_curve(),
        }
    }
}

impl ParameterBoundary2D<Surface> for Curve3D {
    fn parameter_boundary_2d(&self, surface: &Surface, tolerance: f64) -> Option<Vec<Point2>> {
        match self {
            Curve3D::ParameterCurve(curve) => {
                if curve.surface().as_ref() == surface {
                    Some(
                        curve
                            .curve()
                            .parameter_division(curve.curve().range_tuple(), tolerance)
                            .1,
                    )
                } else {
                    sampled_parameter_boundary(curve, surface, tolerance)
                }
            }
            Curve3D::SurfaceCurve(curve) => curve
                .parameter_curve_on(surface)
                .map(|parameter_curve| {
                    parameter_curve
                        .curve()
                        .parameter_division(parameter_curve.curve().range_tuple(), tolerance)
                        .1
                })
                .or_else(|| sampled_parameter_boundary(curve.leader(), surface, tolerance)),
            Curve3D::IntersectionCurve(curve) => {
                exact_parameter_curve_on(curve.leader().as_ref(), surface)
                    .map(|parameter_curve| {
                        parameter_curve
                            .curve()
                            .parameter_division(parameter_curve.curve().range_tuple(), tolerance)
                            .1
                    })
                    .or_else(|| {
                        sampled_parameter_boundary(curve.leader().as_ref(), surface, tolerance)
                    })
                    .or_else(|| {
                        curve
                            .leader()
                            .parameter_division(curve.range_tuple(), tolerance)
                            .0
                            .into_iter()
                            .map(|t| {
                                let (_, uv0, uv1) = curve.search_triple(t, 100)?;
                                if curve.surface0().as_ref() == surface {
                                    Some(uv0)
                                } else if curve.surface1().as_ref() == surface {
                                    Some(uv1)
                                } else {
                                    None
                                }
                            })
                            .collect::<Option<Vec<_>>>()
                    })
            }
            Curve3D::Line(_)
            | Curve3D::Polyline(_)
            | Curve3D::Conic(_)
            | Curve3D::BsplineCurve(_)
            | Curve3D::NurbsCurve(_) => sampled_parameter_boundary(self, surface, tolerance),
        }
    }
}

impl ExactParameterBoundary2D<Surface> for Curve3D {
    type BoundaryCurve = StepParameterCurve;

    fn exact_parameter_boundary_2d(&self, surface: &Surface) -> Option<Self::BoundaryCurve> {
        CurveTrimRef::new(self, surface).try_into().ok()
    }
}

fn curve2d_from_sampled_boundary(points: Vec<Point2>) -> Option<Curve2D> {
    if points.len() < 2 {
        None
    } else {
        let front = points.first().copied()?;
        let back = points.last().copied()?;
        let line = Line(front, back);
        let is_linear = points.iter().copied().all(|point| {
            line.search_nearest_parameter(point, None, 1)
                .is_some_and(|t| line.evaluate(t).near(&point))
        });
        if is_linear {
            Some(Curve2D::Line(line))
        } else {
            let denom = (points.len() - 1) as f64;
            let knot_vec = KnotVector::from(
                std::iter::once(0.0)
                    .chain((0..points.len()).map(|index| index as f64 / denom))
                    .chain(std::iter::once(1.0))
                    .collect::<Vec<_>>(),
            );
            Some(Curve2D::BsplineCurve(BsplineCurve::new(knot_vec, points)))
        }
    }
}

impl BoundaryCurveFromSamples<Surface> for StepParameterCurve {
    fn boundary_curve_from_samples(surface: &Surface, points: Vec<Point2>) -> Option<Self> {
        curve2d_from_sampled_boundary(points)
            .map(|curve| ParameterCurve::new(Box::new(curve), Box::new(surface.clone())))
    }
}

impl TryIntoBsplineSurface for Surface {
    fn try_into_bspline_surface(&self) -> Option<BsplineSurface<Point3>> {
        match self {
            Surface::ElementarySurface(ElementarySurface::Plane(surface)) => {
                surface.try_into_bspline_surface()
            }
            Surface::ElementarySurface(ElementarySurface::Sphere(surface)) => {
                surface.try_into_bspline_surface()
            }
            Surface::ElementarySurface(ElementarySurface::CylindricalSurface(surface)) => {
                surface.try_into_bspline_surface()
            }
            Surface::ElementarySurface(ElementarySurface::ToroidalSurface(surface)) => {
                surface.try_into_bspline_surface()
            }
            Surface::ElementarySurface(ElementarySurface::ConicalSurface(surface)) => {
                surface.try_into_bspline_surface()
            }
            Surface::SweepSurface(SweepSurface::ExtrusionSurface(surface)) => {
                surface.try_into_bspline_surface()
            }
            Surface::SweepSurface(SweepSurface::RevolutionSurface(surface)) => {
                surface.try_into_bspline_surface()
            }
            Surface::BsplineSurface(surface) => surface.try_into_bspline_surface(),
            Surface::NurbsSurface(surface) => surface.try_into_bspline_surface(),
        }
    }
}

impl TryIntoHomogeneousBsplineSurface for Surface {
    fn try_into_homogeneous_bspline_surface(&self) -> Option<BsplineSurface<Vector4>> {
        match self {
            Surface::ElementarySurface(ElementarySurface::Plane(surface)) => {
                surface.try_into_homogeneous_bspline_surface()
            }
            Surface::ElementarySurface(ElementarySurface::Sphere(surface)) => {
                surface.try_into_homogeneous_bspline_surface()
            }
            Surface::ElementarySurface(ElementarySurface::CylindricalSurface(surface)) => {
                surface.try_into_homogeneous_bspline_surface()
            }
            Surface::ElementarySurface(ElementarySurface::ToroidalSurface(surface)) => {
                surface.try_into_homogeneous_bspline_surface()
            }
            Surface::ElementarySurface(ElementarySurface::ConicalSurface(surface)) => {
                surface.try_into_homogeneous_bspline_surface()
            }
            Surface::SweepSurface(SweepSurface::ExtrusionSurface(surface)) => {
                surface.try_into_homogeneous_bspline_surface()
            }
            Surface::SweepSurface(SweepSurface::RevolutionSurface(surface)) => {
                surface.try_into_homogeneous_bspline_surface()
            }
            Surface::BsplineSurface(surface) => surface.try_into_homogeneous_bspline_surface(),
            Surface::NurbsSurface(surface) => surface.try_into_homogeneous_bspline_surface(),
        }
    }
}

impl SupportsExactPatchDomains for Surface {
    fn supports_exact_patch_domains(&self) -> bool {
        matches!(self, Surface::BsplineSurface(_) | Surface::NurbsSurface(_))
    }
}

impl TryFrom<&Curve3D> for ModelingCurve {
    type Error = StepConvertingError;
    fn try_from(value: &Curve3D) -> std::result::Result<Self, Self::Error> {
        match value {
            Curve3D::Line(line) => Ok((*line).into()),
            Curve3D::BsplineCurve(curve) => Ok(curve.clone().into()),
            Curve3D::NurbsCurve(curve) => Ok(curve.clone().into()),
            Curve3D::ParameterCurve(curve) => {
                Ok(ModelingCurve::ParameterCurve(ParameterCurve::new(
                    curve.curve().as_ref().try_into()?,
                    Box::new(curve.surface().as_ref().try_into()?),
                )))
            }
            Curve3D::SurfaceCurve(curve) => {
                let surfaces = curve
                    .associated_geometry()
                    .iter()
                    .map(SurfaceCurveAssociatedGeometry::surface)
                    .collect::<Vec<_>>();
                if surfaces.len() >= 2 {
                    let surface0 = surfaces[0].try_into()?;
                    let surface1 = surfaces[1].try_into()?;
                    let boundary0 = curve
                        .parameter_curve_on(surfaces[0])
                        .cloned()
                        .map(|trim| to_modeling_trim(&trim))
                        .transpose()?;
                    let boundary1 = curve
                        .parameter_curve_on(surfaces[1])
                        .cloned()
                        .map(|trim| to_modeling_trim(&trim))
                        .transpose()?;
                    Ok(ModelingCurve::IntersectionCurve(
                        SurfaceCurve::with_boundaries(
                            Box::new(surface0),
                            Box::new(surface1),
                            Box::new(curve.leader().try_into()?),
                            boundary0,
                            boundary1,
                        ),
                    ))
                } else {
                    curve.leader().try_into()
                }
            }
            Curve3D::IntersectionCurve(curve) => Ok(ModelingCurve::IntersectionCurve(
                SurfaceCurve::with_boundaries(
                    Box::new(curve.surface0().as_ref().try_into()?),
                    Box::new(curve.surface1().as_ref().try_into()?),
                    Box::new(curve.leader().as_ref().try_into()?),
                    None,
                    None,
                ),
            )),
            _ => value
                .try_into_homogeneous_bspline_curve()
                .map(|curve| ModelingCurve::NurbsCurve(NurbsCurve::new(curve)))
                .ok_or_else(|| "STEP curve cannot be represented in modeling geometry.".into()),
        }
    }
}

impl TryFrom<&Conic2D> for ModelingConic2D {
    type Error = StepConvertingError;
    fn try_from(value: &Conic2D) -> std::result::Result<Self, Self::Error> {
        match value {
            Conic2D::Ellipse(curve) => Ok((*curve).into()),
            Conic2D::Hyperbola(curve) => Ok((*curve).into()),
            Conic2D::Parabola(curve) => Ok((*curve).into()),
        }
    }
}

impl TryFrom<&Curve2D> for ModelingCurve2D {
    type Error = StepConvertingError;
    fn try_from(value: &Curve2D) -> std::result::Result<Self, Self::Error> {
        match value {
            Curve2D::Line(curve) => Ok((*curve).into()),
            Curve2D::Polyline(curve) => Ok(curve.clone().into()),
            Curve2D::Conic(curve) => Ok(ModelingCurve2D::Conic(curve.try_into()?)),
            Curve2D::BsplineCurve(curve) => Ok(curve.clone().into()),
            Curve2D::NurbsCurve(curve) => Ok(curve.clone().into()),
        }
    }
}

impl<'a> TryFrom<SurfaceCurveTrimRef<'a>> for StepParameterCurve {
    type Error = StepConvertingError;

    fn try_from(value: SurfaceCurveTrimRef<'a>) -> std::result::Result<Self, Self::Error> {
        let curve = value.curve();
        let surface = value.surface();
        exact_parameter_curve_on(&Curve3D::SurfaceCurve(curve.clone()), surface)
            .ok_or_else(|| "STEP surface curve has no exact trim on the requested surface.".into())
    }
}

impl<'a> TryFrom<CurveTrimRef<'a>> for StepParameterCurve {
    type Error = StepConvertingError;

    fn try_from(value: CurveTrimRef<'a>) -> std::result::Result<Self, Self::Error> {
        exact_parameter_curve_on(value.curve(), value.surface())
            .ok_or_else(|| "STEP curve has no exact trim on the requested surface.".into())
    }
}

impl TryFrom<&Surface> for ModelingSurface {
    type Error = StepConvertingError;
    fn try_from(value: &Surface) -> std::result::Result<Self, Self::Error> {
        match value {
            Surface::ElementarySurface(ElementarySurface::Plane(surface)) => Ok((*surface).into()),
            _ => value
                .try_into_homogeneous_bspline_surface()
                .map(|surface| ModelingSurface::NurbsSurface(NurbsSurface::new(surface)))
                .or_else(|| {
                    value
                        .try_into_bspline_surface()
                        .map(ModelingSurface::BsplineSurface)
                })
                .ok_or_else(|| "STEP surface cannot be represented in modeling geometry.".into()),
        }
    }
}

impl ToSameGeometry<Curve3D> for Line<Point3> {
    #[inline]
    fn to_same_geometry(&self) -> Curve3D { Curve3D::Line(*self) }
}

impl ToSameGeometry<Curve3D> for Processor<TrimmedCurve<UnitCircle<Point3>>, Matrix4> {
    #[inline]
    fn to_same_geometry(&self) -> Curve3D { Curve3D::Conic(Conic3D::Ellipse(*self)) }
}

impl ToSameGeometry<Curve3D> for BsplineCurve<Point3> {
    #[inline]
    fn to_same_geometry(&self) -> Curve3D { Curve3D::BsplineCurve(self.clone()) }
}

impl Conic3D {
    pub fn posture(&self) -> Matrix4 {
        match self {
            Conic3D::Ellipse(processor) => *processor.transform(),
            Conic3D::Hyperbola(processor) => *processor.transform(),
            Conic3D::Parabola(processor) => *processor.transform(),
        }
    }
}

impl IncludeCurve<Curve3D> for Plane {
    fn include(&self, curve: &Curve3D) -> bool {
        match curve {
            Curve3D::Line(line) => self.include(line),
            Curve3D::BsplineCurve(bsp) => self.include(bsp),
            Curve3D::NurbsCurve(bsp) => self.include(bsp),
            Curve3D::Conic(conic) => {
                let mat = conic.posture();
                let axis = mat.z.truncate();
                axis.cross(self.normal()).so_small()
            }
            Curve3D::Polyline(poly) => poly
                .iter()
                .all(|p| self.search_parameter(*p, None, 1).is_some()),
            Curve3D::ParameterCurve(curve) => matches!(
                curve.surface().as_ref(),
                Surface::ElementarySurface(ElementarySurface::Plane(surface)) if self == surface
            ),
            Curve3D::SurfaceCurve(curve) => self.include(curve.leader()),
            Curve3D::IntersectionCurve(curve) => self.include(curve.leader().as_ref()),
        }
    }
}

impl ToSameGeometry<Surface> for Plane {
    #[inline]
    fn to_same_geometry(&self) -> Surface {
        Surface::ElementarySurface(ElementarySurface::Plane(*self))
    }
}

impl ToSameGeometry<Surface> for ExtrusionSurface<Curve3D, Vector3> {
    #[inline]
    fn to_same_geometry(&self) -> Surface {
        Surface::SweepSurface(SweepSurface::ExtrusionSurface(self.clone()))
    }
}

impl ToSameGeometry<Surface> for RevolutionSurface<Curve3D> {
    #[inline]
    fn to_same_geometry(&self) -> Surface {
        let default = || {
            let (curve, origin, axis) = (self.entity_curve().inverse(), self.origin(), self.axis());
            let mut processor =
                Processor::new(RevolutionSurface::by_revolution(curve, origin, axis));
            processor.invert();
            Surface::SweepSurface(SweepSurface::RevolutionSurface(processor))
        };
        match self.entity_curve() {
            Curve3D::Line(line) => {
                let &Line(p, q) = line;
                let v = q - p;
                let axis = self.axis();
                if v.cross(axis).so_small() {
                    let o = self.origin();
                    let origin = o + (q - o).dot(axis) * axis;
                    let line = Line(q, q - v.normalize());
                    let revo = RevolutionSurface::by_revolution(line, origin, axis);
                    let mut processor = Processor::new(revo);
                    processor.invert();
                    Surface::ElementarySurface(ElementarySurface::CylindricalSurface(processor))
                } else {
                    default()
                }
            }
            Curve3D::SurfaceCurve(_) => default(),
            Curve3D::IntersectionCurve(_) => default(),
            _ => default(),
        }
    }
}

#[test]
fn sampled_parameter_boundary_preserves_unbounded_cylinder_axis_parameter() {
    let center = Point3::new(0.0, 0.0, 68.0);
    let axis = Vector3::unit_z();
    let radius = 0.3;
    let p = center + radius * Vector3::unit_x();
    let mut cylinder = Processor::new(RevolutionSurface::by_revolution(
        Line(p, p + axis),
        center,
        axis,
    ));
    cylinder.invert();
    let surface = Surface::ElementarySurface(ElementarySurface::CylindricalSurface(cylinder));
    let curve = Line(
        Point3::new(radius, 0.0, -6.25),
        Point3::new(radius, 0.0, 6.25),
    );

    let boundary = sampled_parameter_boundary(&curve, &surface, 0.001).unwrap();
    let max_abs = boundary
        .iter()
        .flat_map(|uv| [uv.x.abs(), uv.y.abs()])
        .fold(0.0, f64::max);

    assert!(max_abs > 10.0);
}

#[test]
fn raw_conic_boundary_without_pcurve_uses_sampled_projection_at_safe_tolerance() {
    let curve = Curve3D::Conic(Conic3D::Ellipse(
        Processor::new(TrimmedCurve::new(UnitCircle::<Point3>::new(), (0.0, TAU)))
            .transformed(Matrix4::from_nonuniform_scale(100.0, 100.0, 100.0)),
    ));
    let surface = Surface::ElementarySurface(ElementarySurface::Plane(Plane::xy()));

    let boundary = curve
        .parameter_boundary_2d(&surface, 1.0e-3)
        .expect("safe raw conic projection should produce a parameter boundary.");

    assert!(boundary.len() > 4);
    assert!(
        boundary
            .iter()
            .any(|point| point.distance2(Point2::new(100.0, 0.0)) < 1.0e-6)
    );
}

#[test]
fn raw_line_boundary_without_pcurve_uses_sampled_projection() {
    let curve = Curve3D::Line(Line(
        Point3::new(0.25, 0.5, 0.0),
        Point3::new(0.75, 0.5, 0.0),
    ));
    let surface = Surface::ElementarySurface(ElementarySurface::Plane(Plane::xy()));
    let boundary = curve
        .parameter_boundary_2d(&surface, 1.0e-5)
        .expect("safe raw line projection should produce a parameter boundary.");

    assert!(boundary.len() >= 2);
    assert!(
        boundary
            .first()
            .is_some_and(|point| point.near(&Point2::new(0.25, 0.5)))
    );
    assert!(
        boundary
            .last()
            .is_some_and(|point| point.near(&Point2::new(0.75, 0.5)))
    );
}

#[test]
fn surface_curve_line_without_pcurve_converts_to_exact_pcurve() {
    let leader = Curve3D::Line(Line(
        Point3::new(0.25, 0.5, 0.0),
        Point3::new(0.75, 0.5, 0.0),
    ));
    let curve = Curve3D::SurfaceCurve(SurfaceCurve3D::new(
        SurfaceCurveKind::Surface,
        Box::new(leader),
        Vec::new(),
        SurfaceCurveRepresentation::Curve3D,
    ));
    let surface = Surface::ElementarySurface(ElementarySurface::Plane(Plane::xy()));

    let boundary = StepParameterCurve::try_from(CurveTrimRef::new(&curve, &surface))
        .expect("surface curve leader should produce an exact parameter boundary.");

    match boundary.curve().as_ref() {
        Curve2D::Line(line) => {
            assert!(line.front().near(&Point2::new(0.25, 0.5)));
            assert!(line.back().near(&Point2::new(0.75, 0.5)));
        }
        curve => panic!("expected line boundary, got {curve:?}"),
    }
}

#[test]
fn surface_curve_line_without_pcurve_on_cylinder_stays_fallback_only() {
    let axis = Vector3::unit_z();
    let center = Point3::origin();
    let point = Point3::new(1.0, 0.0, 0.0);
    let profile = Line(point, point + axis);
    let surface = Surface::ElementarySurface(ElementarySurface::CylindricalSurface(
        Processor::new(RevolutionSurface::by_revolution(profile, center, axis)),
    ));
    let leader = Curve3D::Line(Line(surface.evaluate(0.0, 0.0), surface.evaluate(0.0, 1.0)));
    let curve = Curve3D::SurfaceCurve(SurfaceCurve3D::new(
        SurfaceCurveKind::Surface,
        Box::new(leader),
        Vec::new(),
        SurfaceCurveRepresentation::Curve3D,
    ));

    assert!(StepParameterCurve::try_from(CurveTrimRef::new(&curve, &surface)).is_err());
    assert!(curve.exact_parameter_boundary_2d(&surface).is_none());
}

#[test]
fn raw_line_without_pcurve_on_cylinder_stays_fallback_only() {
    let axis = Vector3::unit_z();
    let center = Point3::origin();
    let point = Point3::new(1.0, 0.0, 0.0);
    let profile = Line(point, point + axis);
    let surface = Surface::ElementarySurface(ElementarySurface::CylindricalSurface(
        Processor::new(RevolutionSurface::by_revolution(profile, center, axis)),
    ));
    let curve = Curve3D::Line(Line(surface.evaluate(0.0, 0.0), surface.evaluate(0.0, 1.0)));

    assert!(StepParameterCurve::try_from(CurveTrimRef::new(&curve, &surface)).is_err());
    assert!(curve.exact_parameter_boundary_2d(&surface).is_none());
}

#[test]
fn builder() {
    use monstertruck_meshing::prelude::*;
    use monstertruck_modeling::builder;
    monstertruck_topology::prelude!(Point3, Curve3D, Surface);

    // cube
    let v = builder::vertices([(0.0, 0.0, 0.0), (1.0, 0.0, 0.0)]);
    let e = builder::line(&v[0], &v[1]);
    let f = builder::extrude(&e, Vector3::unit_y());
    let cube: Solid = builder::extrude(&f, Vector3::unit_z());
    let mut poly = cube.triangulation(0.1).to_polygon();
    poly.put_together_same_attrs(1.0e-3).remove_unused_attrs();
    assert_eq!(poly.shell_condition(), ShellCondition::Closed);

    // cylinder
    let v = builder::vertices([(1.0, 0.0, 1.0), (1.0, 0.0, 0.0)]);
    let e = builder::line(&v[0], &v[1]);
    let mut shell = builder::revolve(
        &e,
        Point3::origin(),
        Vector3::unit_z(),
        builder::SweepAngle::Closed,
        2,
    );
    let boundaries = shell.extract_boundaries();
    assert_eq!(boundaries.len(), 2);
    shell.push(builder::try_attach_plane([boundaries[0].inverse()]).unwrap());
    shell.push(builder::try_attach_plane([boundaries[1].inverse()]).unwrap());
    let cylinder = Solid::new(vec![shell]);
    let mut poly = cylinder.triangulation(0.1).to_polygon();
    poly.put_together_same_attrs(1.0e-3).remove_unused_attrs();
    assert_eq!(poly.shell_condition(), ShellCondition::Closed);

    // torus
    let v = builder::vertex((1.5, 0.0, 0.0));
    let w = builder::revolve(
        &v,
        Point3::new(1.0, 0.0, 0.0),
        Vector3::unit_y(),
        builder::SweepAngle::Closed,
        2,
    );
    let f = builder::try_attach_plane([w]).unwrap();
    let torus: Solid = builder::revolve(
        &f,
        Point3::origin(),
        Vector3::unit_z(),
        builder::SweepAngle::Closed,
        2,
    );
    let mut poly = torus.triangulation(0.1).to_polygon();
    poly.put_together_same_attrs(1.0e-3).remove_unused_attrs();
    assert_eq!(poly.shell_condition(), ShellCondition::Closed);

    // cylinder hole
    let v = builder::vertex((-1.0, -1.0, -1.0));
    let e = builder::extrude(&v, 2.0 * Vector3::unit_x());
    let f = builder::extrude(&e, 2.0 * Vector3::unit_y());
    let s: Solid = builder::extrude(&f, 2.0 * Vector3::unit_z());
    let mut shell = s.into_boundaries().pop().unwrap();
    let line = builder::line(
        &builder::vertex((0.5, 0.0, 1.0)),
        &builder::vertex((0.5, 0.0, -1.0)),
    );
    let hole = builder::revolve(
        &line,
        Point3::origin(),
        -Vector3::unit_z(),
        builder::SweepAngle::Closed,
        2,
    );
    let boundary = hole.extract_boundaries();
    assert_eq!(boundary.len(), 2);
    if boundary[0][0].front().point().z < 0.0 {
        let _ = shell[0].add_boundary(boundary[0].inverse());
        let _ = shell[5].add_boundary(boundary[1].inverse());
    } else {
        let _ = shell[0].add_boundary(boundary[1].inverse());
        let _ = shell[5].add_boundary(boundary[0].inverse());
    }
    shell.extend(hole);
    let solid = Solid::new(vec![shell]);
    let mut poly = solid.triangulation(0.1).to_polygon();
    poly.put_together_same_attrs(1.0e-3).remove_unused_attrs();
    assert_eq!(poly.shell_condition(), ShellCondition::Closed);
}
