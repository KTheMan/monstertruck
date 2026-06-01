//! Traits for extracting exact B-spline surface representations.

use crate::prelude::*;

/// Attempts to produce an exact homogeneous B-spline curve representation.
///
/// The returned control net is in homogeneous coordinates, so true rational
/// NURBS curves remain exact.
pub trait TryIntoHomogeneousBsplineCurve {
    /// Converts this curve into a homogeneous B-spline curve, if possible.
    fn try_into_homogeneous_bspline_curve(&self) -> Option<BsplineCurve<Vector4>>;
}

/// Attempts to produce an exact homogeneous B-spline surface representation.
///
/// The returned control net is in homogeneous coordinates, so true rational
/// NURBS surfaces remain exact.
pub trait TryIntoHomogeneousBsplineSurface {
    /// Converts this surface into a homogeneous B-spline surface, if possible.
    fn try_into_homogeneous_bspline_surface(&self) -> Option<BsplineSurface<Vector4>>;
}

/// Reports whether exact knot-span patch domains should be preferred over
/// polygon-derived domains for this surface.
pub trait SupportsExactPatchDomains {
    /// Returns `true` when exact knot-span patch domains are preferred for this
    /// surface.
    fn supports_exact_patch_domains(&self) -> bool;
}

impl TryIntoHomogeneousBsplineCurve for Line<Point3> {
    #[inline]
    fn try_into_homogeneous_bspline_curve(&self) -> Option<BsplineCurve<Vector4>> {
        Some(BsplineCurve::lift_up(BsplineCurve::new(
            KnotVector::bezier_knot(1),
            vec![self.0, self.1],
        )))
    }
}

impl TryIntoHomogeneousBsplineCurve for BsplineCurve<Point3> {
    #[inline]
    fn try_into_homogeneous_bspline_curve(&self) -> Option<BsplineCurve<Vector4>> {
        Some(BsplineCurve::lift_up(self.clone()))
    }
}

impl TryIntoHomogeneousBsplineCurve for BsplineCurve<Vector4> {
    #[inline]
    fn try_into_homogeneous_bspline_curve(&self) -> Option<BsplineCurve<Vector4>> {
        Some(self.clone())
    }
}

impl TryIntoHomogeneousBsplineCurve for NurbsCurve<Vector4> {
    #[inline]
    fn try_into_homogeneous_bspline_curve(&self) -> Option<BsplineCurve<Vector4>> {
        Some(self.non_rationalized().clone())
    }
}

impl TryIntoHomogeneousBsplineCurve for TrimmedCurve<UnitCircle<Point3>> {
    #[inline]
    fn try_into_homogeneous_bspline_curve(&self) -> Option<BsplineCurve<Vector4>> {
        let curve: NurbsCurve<Vector4> = self.to_same_geometry();
        Some(curve.into())
    }
}

/// Attempts to produce an exact polynomial (non-rational) B-spline surface.
///
/// Returns [`Some`] for surface types that can be represented exactly as a
/// [`BsplineSurface<Point3>`] without loss of geometric information.  Returns
/// [`None`] for surfaces that are inherently rational (true NURBS with
/// non-unit weights), surfaces of revolution, T-splines, or any other type
/// that cannot be expressed as a polynomial B-spline.
pub trait TryIntoBsplineSurface {
    /// Converts this surface into a polynomial B-spline surface, if possible.
    fn try_into_bspline_surface(&self) -> Option<BsplineSurface<Point3>>;
}

// ---------------------------------------------------------------------------
// BsplineSurface<Point3>: identity conversion.
// ---------------------------------------------------------------------------

impl TryIntoBsplineSurface for BsplineSurface<Point3> {
    #[inline]
    fn try_into_bspline_surface(&self) -> Option<BsplineSurface<Point3>> { Some(self.clone()) }
}

impl TryIntoHomogeneousBsplineSurface for BsplineSurface<Point3> {
    #[inline]
    fn try_into_homogeneous_bspline_surface(&self) -> Option<BsplineSurface<Vector4>> {
        Some(BsplineSurface::lift_up(self.clone()))
    }
}

impl SupportsExactPatchDomains for BsplineSurface<Point3> {
    #[inline]
    fn supports_exact_patch_domains(&self) -> bool { true }
}

impl TryIntoHomogeneousBsplineSurface for BsplineSurface<Vector4> {
    #[inline]
    fn try_into_homogeneous_bspline_surface(&self) -> Option<BsplineSurface<Vector4>> {
        Some(self.clone())
    }
}

impl SupportsExactPatchDomains for BsplineSurface<Vector4> {
    #[inline]
    fn supports_exact_patch_domains(&self) -> bool { true }
}

// ---------------------------------------------------------------------------
// NurbsSurface<Vector4>: extract polynomial form when all weights ≈ 1.
// ---------------------------------------------------------------------------

impl TryIntoHomogeneousBsplineSurface for NurbsSurface<Vector4> {
    #[inline]
    fn try_into_homogeneous_bspline_surface(&self) -> Option<BsplineSurface<Vector4>> {
        Some(self.non_rationalized().clone())
    }
}

impl SupportsExactPatchDomains for NurbsSurface<Vector4> {
    #[inline]
    fn supports_exact_patch_domains(&self) -> bool { true }
}

impl TryIntoBsplineSurface for NurbsSurface<Vector4> {
    fn try_into_bspline_surface(&self) -> Option<BsplineSurface<Point3>> {
        let ctrl = self.control_points();
        // Check that every control-point weight is close to 1.0.
        let all_unit = ctrl
            .iter()
            .flat_map(|row| row.iter())
            .all(|v| (v.w - 1.0).abs() < TOLERANCE);
        if !all_unit {
            return None;
        }
        // Project Vector4(x, y, z, w) -> Point3(x/w, y/w, z/w).
        let pts: Vec<Vec<Point3>> = ctrl
            .iter()
            .map(|row| row.iter().map(|v| v.to_point()).collect())
            .collect();
        Some(BsplineSurface::new(self.knot_vectors().clone(), pts))
    }
}

// ---------------------------------------------------------------------------
// Plane: exact bilinear Bezier patch.
// ---------------------------------------------------------------------------

impl TryIntoBsplineSurface for Plane {
    #[inline]
    fn try_into_bspline_surface(&self) -> Option<BsplineSurface<Point3>> {
        Some(BsplineSurface::from(*self))
    }
}

impl TryIntoHomogeneousBsplineSurface for Plane {
    #[inline]
    fn try_into_homogeneous_bspline_surface(&self) -> Option<BsplineSurface<Vector4>> {
        Some(BsplineSurface::lift_up(BsplineSurface::from(*self)))
    }
}

impl SupportsExactPatchDomains for Plane {
    #[inline]
    fn supports_exact_patch_domains(&self) -> bool { false }
}

fn full_unit_circle_curve() -> BsplineCurve<Vector4> {
    let w = std::f64::consts::FRAC_1_SQRT_2;
    BsplineCurve::new_unchecked(
        KnotVector::from(vec![
            0.0, 0.0, 0.0, 0.25, 0.25, 0.5, 0.5, 0.75, 0.75, 1.0, 1.0, 1.0,
        ]),
        vec![
            Vector4::new(1.0, 0.0, 0.0, 1.0),
            Vector4::new(w, w, 0.0, w),
            Vector4::new(0.0, 1.0, 0.0, 1.0),
            Vector4::new(-w, w, 0.0, w),
            Vector4::new(-1.0, 0.0, 0.0, 1.0),
            Vector4::new(-w, -w, 0.0, w),
            Vector4::new(0.0, -1.0, 0.0, 1.0),
            Vector4::new(w, -w, 0.0, w),
            Vector4::new(1.0, 0.0, 0.0, 1.0),
        ],
    )
}

fn circle_orbit_transform(center: Point3, radial: Vector3, axis: Vector3) -> Matrix4 {
    Matrix4::from_cols(
        radial.extend(0.0),
        axis.cross(radial).extend(0.0),
        axis.extend(0.0),
        center.to_homogeneous(),
    )
}

impl<C> TryIntoHomogeneousBsplineSurface for RevolutionSurface<C>
where C: TryIntoHomogeneousBsplineCurve
{
    fn try_into_homogeneous_bspline_surface(&self) -> Option<BsplineSurface<Vector4>> {
        let profile = self.entity_curve().try_into_homogeneous_bspline_curve()?;
        let axis = self.axis().normalize();
        let origin = self.origin();
        let circle = full_unit_circle_curve();
        let knot_vecs = (profile.knot_vector().clone(), circle.knot_vector().clone());
        let circle_control_points = circle.control_points().clone();
        let control_points = profile
            .control_points()
            .iter()
            .map(|profile_point| {
                let weight = profile_point.weight();
                if weight.abs() <= TOLERANCE {
                    return None;
                }
                let point = profile_point.to_point();
                let center = origin + axis * (point - origin).dot(axis);
                let radial = point - center;
                if radial.magnitude2() <= TOLERANCE * TOLERANCE {
                    // All orbit points collapse to the pole, but preserve the
                    // circle weight pattern so that iso-u rational curves keep
                    // the correct weight ratios for exact circles.
                    return Some(
                        circle_control_points
                            .iter()
                            .map(|cp| *profile_point * cp.weight())
                            .collect(),
                    );
                }
                let transform = circle_orbit_transform(center, radial, axis);
                Some(
                    circle_control_points
                        .iter()
                        .map(|circle_point| transform * *circle_point * weight)
                        .collect(),
                )
            })
            .collect::<Option<Vec<Vec<_>>>>()?;
        Some(BsplineSurface::new_unchecked(knot_vecs, control_points))
    }
}

impl<C> SupportsExactPatchDomains for RevolutionSurface<C> {
    #[inline]
    fn supports_exact_patch_domains(&self) -> bool { false }
}

// ---------------------------------------------------------------------------
// Types that cannot be represented as polynomial B-splines.
// ---------------------------------------------------------------------------

impl TryIntoHomogeneousBsplineSurface for Sphere {
    #[inline]
    fn try_into_homogeneous_bspline_surface(&self) -> Option<BsplineSurface<Vector4>> { None }
}

impl SupportsExactPatchDomains for Sphere {
    #[inline]
    fn supports_exact_patch_domains(&self) -> bool { false }
}

impl TryIntoBsplineSurface for Sphere {
    #[inline]
    fn try_into_bspline_surface(&self) -> Option<BsplineSurface<Point3>> { None }
}

impl TryIntoHomogeneousBsplineSurface for Torus {
    #[inline]
    fn try_into_homogeneous_bspline_surface(&self) -> Option<BsplineSurface<Vector4>> { None }
}

impl SupportsExactPatchDomains for Torus {
    #[inline]
    fn supports_exact_patch_domains(&self) -> bool { false }
}

impl TryIntoBsplineSurface for Torus {
    #[inline]
    fn try_into_bspline_surface(&self) -> Option<BsplineSurface<Point3>> { None }
}

impl<C> TryIntoBsplineSurface for RevolutionSurface<C> {
    #[inline]
    fn try_into_bspline_surface(&self) -> Option<BsplineSurface<Point3>> { None }
}

impl<C> TryIntoHomogeneousBsplineSurface for ExtrusionSurface<C, Vector3>
where C: TryIntoHomogeneousBsplineCurve
{
    fn try_into_homogeneous_bspline_surface(&self) -> Option<BsplineSurface<Vector4>> {
        let curve = self.entity_curve().try_into_homogeneous_bspline_curve()?;
        let dir = self.extruding_vector();
        let knot_vecs = (curve.knot_vector().clone(), KnotVector::bezier_knot(1));
        // Row at v=0: original curve control points.
        // Row at v=1: each (x,y,z,w) -> (x+dx*w, y+dy*w, z+dz*w, w).
        let row0 = curve.control_points().clone();
        let row1: Vec<Vector4> = row0
            .iter()
            .map(|p| Vector4::new(p.x + dir.x * p.w, p.y + dir.y * p.w, p.z + dir.z * p.w, p.w))
            .collect();
        Some(BsplineSurface::new(knot_vecs, vec![row0, row1]))
    }
}

impl<C> SupportsExactPatchDomains for ExtrusionSurface<C, Vector3> {
    #[inline]
    fn supports_exact_patch_domains(&self) -> bool { false }
}

impl<C> TryIntoBsplineSurface for ExtrusionSurface<C, Vector3>
where C: TryIntoHomogeneousBsplineCurve
{
    fn try_into_bspline_surface(&self) -> Option<BsplineSurface<Point3>> {
        let hom = self.try_into_homogeneous_bspline_surface()?;
        // Check that all weights are ≈ 1.
        let all_unit = hom
            .control_points()
            .iter()
            .flat_map(|row| row.iter())
            .all(|v| (v.w - 1.0).abs() < TOLERANCE);
        if !all_unit {
            return None;
        }
        let pts: Vec<Vec<Point3>> = hom
            .control_points()
            .iter()
            .map(|row| row.iter().map(|v| v.to_point()).collect())
            .collect();
        Some(BsplineSurface::new(hom.knot_vectors().clone(), pts))
    }
}

impl<T> TryIntoHomogeneousBsplineSurface for Processor<T, Matrix4>
where T: TryIntoHomogeneousBsplineSurface
{
    #[inline]
    fn try_into_homogeneous_bspline_surface(&self) -> Option<BsplineSurface<Vector4>> {
        let mut surface = self.entity().try_into_homogeneous_bspline_surface()?;
        surface
            .control_points_mut()
            .for_each(|point| *point = *self.transform() * *point);
        if !self.orientation() {
            surface.invert();
        }
        Some(surface)
    }
}

impl<T> SupportsExactPatchDomains for Processor<T, Matrix4>
where T: SupportsExactPatchDomains
{
    #[inline]
    fn supports_exact_patch_domains(&self) -> bool { self.entity().supports_exact_patch_domains() }
}

impl<T> TryIntoHomogeneousBsplineCurve for Processor<T, Matrix4>
where T: TryIntoHomogeneousBsplineCurve
{
    #[inline]
    fn try_into_homogeneous_bspline_curve(&self) -> Option<BsplineCurve<Vector4>> {
        let mut curve = self.entity().try_into_homogeneous_bspline_curve()?;
        curve
            .control_points_mut()
            .for_each(|point| *point = *self.transform() * *point);
        if !self.orientation() {
            curve.invert();
        }
        Some(curve)
    }
}

impl<T, M> TryIntoBsplineSurface for Processor<T, M>
where
    T: TryIntoBsplineSurface,
    M: Transform<Point3> + Copy,
{
    #[inline]
    fn try_into_bspline_surface(&self) -> Option<BsplineSurface<Point3>> {
        let mut surface = self.entity().try_into_bspline_surface()?;
        surface.transform_by(*self.transform());
        if !self.orientation() {
            surface.invert();
        }
        Some(surface)
    }
}

impl TryIntoHomogeneousBsplineSurface for Tmesh<Point3> {
    #[inline]
    fn try_into_homogeneous_bspline_surface(&self) -> Option<BsplineSurface<Vector4>> { None }
}

impl SupportsExactPatchDomains for Tmesh<Point3> {
    #[inline]
    fn supports_exact_patch_domains(&self) -> bool { false }
}

impl TryIntoBsplineSurface for Tmesh<Point3> {
    #[inline]
    fn try_into_bspline_surface(&self) -> Option<BsplineSurface<Point3>> { None }
}

// IntersectionCurve as a surface: not applicable.
impl<C, S0, S1> TryIntoHomogeneousBsplineSurface for IntersectionCurve<C, S0, S1> {
    #[inline]
    fn try_into_homogeneous_bspline_surface(&self) -> Option<BsplineSurface<Vector4>> { None }
}

impl<C, S0, S1> SupportsExactPatchDomains for IntersectionCurve<C, S0, S1> {
    #[inline]
    fn supports_exact_patch_domains(&self) -> bool { false }
}

impl<C, S0, S1> TryIntoBsplineSurface for IntersectionCurve<C, S0, S1> {
    #[inline]
    fn try_into_bspline_surface(&self) -> Option<BsplineSurface<Point3>> { None }
}
