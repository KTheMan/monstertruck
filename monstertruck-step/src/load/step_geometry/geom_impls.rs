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

fn exact_parameter_curve_on(curve: &Curve3D, surface: &Surface) -> Option<Pcurve> {
    match curve {
        Curve3D::Pcurve(curve)
            if SurfaceCurve3D::same_surface(curve.surface().as_ref(), surface) =>
        {
            Some(curve.clone())
        }
        Curve3D::SurfaceCurve(curve) => curve.parameter_curve_on(surface).cloned(),
        Curve3D::IntersectionCurve(curve)
            if SurfaceCurve3D::same_surface(curve.surface0().as_ref(), surface)
                || SurfaceCurve3D::same_surface(curve.surface1().as_ref(), surface) =>
        {
            exact_parameter_curve_on(curve.leader().as_ref(), surface)
        }
        _ => None,
    }
}

fn to_modeling_trim(
    curve: &Pcurve,
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
            SurfaceCurveAssociatedGeometry::Surface(surface) => surface.as_ref(),
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
            | Curve3D::Pcurve(_)
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
        let debug_profile = std::env::var("MT_PROFILE_CURVE_DIVISION").is_ok();
        let started = std::time::Instant::now();
        let result = match self {
            Curve3D::Line(curve) => curve.parameter_division(range, tol),
            Curve3D::Polyline(curve) => curve.parameter_division(range, tol),
            Curve3D::Conic(curve) => curve.parameter_division(range, tol),
            Curve3D::BsplineCurve(curve) => curve.parameter_division(range, tol),
            Curve3D::Pcurve(curve) => curve.parameter_division(range, tol),
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
                Curve3D::Pcurve(_) => "Pcurve",
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
            Curve3D::Pcurve(_) => None,
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
        // TODO: Re-enable sampled boundary projection after `parameter_division` can return `Option` or `Result`.
        // The old fallback can assert below `TOLERANCE` after transformed curve tolerance rescaling.
        match self {
            Curve3D::Pcurve(curve) if curve.surface().as_ref() == surface => Some(
                curve
                    .curve()
                    .parameter_division(curve.curve().range_tuple(), tolerance)
                    .1,
            ),
            Curve3D::SurfaceCurve(curve) => {
                curve.parameter_curve_on(surface).map(|parameter_curve| {
                    parameter_curve
                        .curve()
                        .parameter_division(parameter_curve.curve().range_tuple(), tolerance)
                        .1
                })
            }
            Curve3D::IntersectionCurve(curve) => exact_parameter_curve_on(curve.leader(), surface)
                .map(|parameter_curve| {
                    parameter_curve
                        .curve()
                        .parameter_division(parameter_curve.curve().range_tuple(), tolerance)
                        .1
                }),
            _ => None,
        }
    }
}

impl ExactParameterBoundary2D<Surface> for Curve3D {
    type BoundaryCurve = Pcurve;

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

impl BoundaryCurveFromSamples<Surface> for Pcurve {
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
            Curve3D::Pcurve(curve) => Ok(ModelingCurve::ParameterCurve(ParameterCurve::new(
                curve.curve().as_ref().try_into()?,
                Box::new(curve.surface().as_ref().try_into()?),
            ))),
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

impl<'a> TryFrom<SurfaceCurveTrimRef<'a>> for Pcurve {
    type Error = StepConvertingError;

    fn try_from(value: SurfaceCurveTrimRef<'a>) -> std::result::Result<Self, Self::Error> {
        let curve = value.curve();
        let surface = value.surface();
        exact_parameter_curve_on(&Curve3D::SurfaceCurve(curve.clone()), surface)
            .ok_or_else(|| "STEP surface curve has no exact trim on the requested surface.".into())
    }
}

impl<'a> TryFrom<CurveTrimRef<'a>> for Pcurve {
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
            Curve3D::Pcurve(curve) => matches!(
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
