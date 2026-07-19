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
    /// Returns `true` when exact patch domains are the preferred
    /// broad-phase partitioning for this surface.
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
// Sphere and Torus: exact rational-NURBS (homogeneous B-spline) conversions.
//
// Both are true NURBS (non-unit control-point weights), so they convert to a
// homogeneous `BsplineSurface<Vector4>` but NOT to a polynomial
// `BsplineSurface<Point3>` (`try_into_bspline_surface` stays `None`).
//
// The analytic `Sphere`/`Torus` carry only a center and radii, i.e. they are
// always canonically placed with the z-axis as axis of symmetry and the u=0
// meridian toward +x; general placement is carried by the enclosing
// `Processor<_, Matrix4>`, whose arm transforms the control net.
//
// Parameter convention: the circle/revolution directions use the SAME
// normalized [0, 1] knot span as `full_unit_circle_curve` and the existing
// `RevolutionSurface` conversion (cylinders already ride that path), i.e. a
// full turn spans knot [0, 1] with quarter turns at 0.25/0.5/0.75 -- NOT
// radians. Inside each quadratic arc the param->angle map is the exact
// rational-circle map (non-linear except at the arc knots). Consumers erase
// STEP trims and re-derive boundary parameters by search, so this matches --
// and is no worse than -- revolve-built equivalents. The tensor product
// separates exactly, so evaluating the emitted surface reproduces the analytic
// `subs` at the corresponding angles to machine precision (see this module's
// tests).
// ---------------------------------------------------------------------------

/// Homogeneous (weighted) control point `(weight * point, weight)`.
#[inline]
fn weighted_control_point(point: Point3, weight: f64) -> Vector4 {
    Vector4::new(point.x * weight, point.y * weight, point.z * weight, weight)
}

impl TryIntoHomogeneousBsplineSurface for Sphere {
    fn try_into_homogeneous_bspline_surface(&self) -> Option<BsplineSurface<Vector4>> {
        let radius = self.radius();
        if !radius.is_finite() || radius <= TOLERANCE {
            return None;
        }
        let center = self.center();
        let circle = full_unit_circle_curve();
        // Longitude (v) direction: full circle. Projected control polygon +
        // weights.
        let longitude: Vec<(Point3, f64)> = circle
            .control_points()
            .iter()
            .map(|point| (point.to_point(), point.weight()))
            .collect();
        // Meridian (u) direction: unit semicircle from the +z pole (u = 0)
        // through +x (u = pi/2) to the -z pole (u = pi), in (radial, axis)
        // coordinates. Standard two-arc rational quadratic; corner weights
        // cos(pi/4). Knots [0,0,0, 0.5,0.5, 1,1,1].
        let w = std::f64::consts::FRAC_1_SQRT_2;
        // (radial, axis, weight)
        let meridian = [
            (0.0, 1.0, 1.0),
            (1.0, 1.0, w),
            (1.0, 0.0, 1.0),
            (1.0, -1.0, w),
            (0.0, -1.0, 1.0),
        ];
        // u = colatitude (meridian), v = longitude (matches `Sphere::subs`).
        let control_points: Vec<Vec<Vector4>> = meridian
            .iter()
            .map(|&(radial_unit, axis_unit, meridian_weight)| {
                let radial = radius * radial_unit;
                let height = radius * axis_unit;
                longitude
                    .iter()
                    .map(|&(direction, longitude_weight)| {
                        let position = Point3::new(
                            center.x + radial * direction.x,
                            center.y + radial * direction.y,
                            center.z + height,
                        );
                        weighted_control_point(position, meridian_weight * longitude_weight)
                    })
                    .collect()
            })
            .collect();
        let meridian_knots = KnotVector::from(vec![0.0, 0.0, 0.0, 0.5, 0.5, 1.0, 1.0, 1.0]);
        Some(BsplineSurface::new_unchecked(
            (meridian_knots, circle.knot_vector().clone()),
            control_points,
        ))
    }
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
    fn try_into_homogeneous_bspline_surface(&self) -> Option<BsplineSurface<Vector4>> {
        let (large_radius, small_radius) = (self.large_radius(), self.small_radius());
        if !large_radius.is_finite()
            || !small_radius.is_finite()
            || small_radius <= TOLERANCE
            || large_radius <= TOLERANCE
        {
            return None;
        }
        // Spindle tori (small radius exceeds large radius) self-intersect near
        // the axis and are not valid B-rep faces, so reject them. Horn tori
        // (R == r, including the floating-point near-horn fillets found in real
        // STEP data where R may sit a few ulps below r) ARE representable: the
        // inner equator pinches to a single point on the axis and every
        // control-point weight stays positive.
        if small_radius - large_radius > TOLERANCE * (large_radius + small_radius) {
            return None;
        }
        let center = self.center();
        let circle = full_unit_circle_curve();
        // Projected unit-circle control polygon + weights, shared by the ring
        // (u, major) and tube (v, minor) directions.
        let polygon: Vec<(Point3, f64)> = circle
            .control_points()
            .iter()
            .map(|point| (point.to_point(), point.weight()))
            .collect();
        // u = ring/major angle, v = tube/minor angle (matches `Torus::subs`).
        let control_points: Vec<Vec<Vector4>> = polygon
            .iter()
            .map(|&(ring, ring_weight)| {
                polygon
                    .iter()
                    .map(|&(tube, tube_weight)| {
                        // Tube cross-section circle in (rho = radial, zeta =
                        // axis), centered at (R, 0) with radius r.
                        let rho = large_radius + small_radius * tube.x;
                        let zeta = small_radius * tube.y;
                        let position = Point3::new(
                            center.x + rho * ring.x,
                            center.y + rho * ring.y,
                            center.z + zeta,
                        );
                        weighted_control_point(position, ring_weight * tube_weight)
                    })
                    .collect()
            })
            .collect();
        let knots = circle.knot_vector().clone();
        Some(BsplineSurface::new_unchecked(
            (knots.clone(), knots),
            control_points,
        ))
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::{FRAC_1_SQRT_2, PI};

    const GRID: usize = 17;

    fn as_nurbs(surface: BsplineSurface<Vector4>) -> NurbsSurface<Vector4> {
        NurbsSurface::new(surface)
    }

    fn grid_params() -> impl Iterator<Item = f64> {
        (0..GRID).map(|i| i as f64 / (GRID - 1) as f64)
    }

    /// Exact `param -> angle` map of the full rational-quadratic unit circle the
    /// conversion uses. The map is non-linear inside each 90-degree arc, so it
    /// must be *evaluated*, not assumed to be `2*pi*t`.
    fn full_circle_angle(t: f64) -> f64 {
        let p = full_unit_circle_curve().subs(t);
        let angle = p.y.atan2(p.x);
        if angle < 0.0 { angle + 2.0 * PI } else { angle }
    }

    /// Exact `param -> colatitude` map of the sphere meridian semicircle.
    fn meridian_colatitude(s: f64) -> f64 {
        let w = FRAC_1_SQRT_2;
        let curve = BsplineCurve::new_unchecked(
            KnotVector::from(vec![0.0, 0.0, 0.0, 0.5, 0.5, 1.0, 1.0, 1.0]),
            vec![
                Vector4::new(0.0, 1.0, 0.0, 1.0),
                Vector4::new(w, w, 0.0, w),
                Vector4::new(1.0, 0.0, 0.0, 1.0),
                Vector4::new(w, -w, 0.0, w),
                Vector4::new(0.0, -1.0, 0.0, 1.0),
            ],
        );
        let p = curve.subs(s);
        // sin(colatitude) = radial = x/w, cos(colatitude) = axis = y/w.
        p.x.atan2(p.y)
    }

    fn torus_off_surface(torus: &Torus, point: Point3) -> f64 {
        let c = torus.center();
        let radial = ((point.x - c.x).powi(2) + (point.y - c.y).powi(2)).sqrt();
        ((radial - torus.large_radius()).powi(2) + (point.z - c.z).powi(2)).sqrt()
            - torus.small_radius()
    }

    fn assert_torus_grid(torus: Torus) {
        let hom = torus
            .try_into_homogeneous_bspline_surface()
            .expect("torus converts to homogeneous NURBS");
        let conv = as_nurbs(hom);
        let tol = 1.0e-9 * (torus.large_radius() + torus.small_radius());
        for s in grid_params() {
            let u_ang = full_circle_angle(s);
            for t in grid_params() {
                let v_ang = full_circle_angle(t);
                let got = conv.subs(s, t);
                let want = torus.subs(u_ang, v_ang);
                assert!(
                    got.distance(want) <= tol,
                    "torus subs mismatch at ({s},{t}) -> ({u_ang},{v_ang}): \
                     got {got:?} want {want:?} d={}",
                    got.distance(want)
                );
                let off = torus_off_surface(&torus, got).abs();
                assert!(off <= tol, "torus off-surface at ({s},{t}): {off}");
            }
        }
    }

    fn assert_sphere_grid(sphere: Sphere) {
        let hom = sphere
            .try_into_homogeneous_bspline_surface()
            .expect("sphere converts to homogeneous NURBS");
        let conv = as_nurbs(hom);
        let tol = 1.0e-9 * sphere.radius();
        for s in grid_params() {
            let u_ang = meridian_colatitude(s);
            for t in grid_params() {
                let v_ang = full_circle_angle(t);
                let got = conv.subs(s, t);
                let want = sphere.subs(u_ang, v_ang);
                assert!(
                    got.distance(want) <= tol,
                    "sphere subs mismatch at ({s},{t}) -> ({u_ang},{v_ang}): \
                     got {got:?} want {want:?} d={}",
                    got.distance(want)
                );
                let off = (got.distance(sphere.center()) - sphere.radius()).abs();
                assert!(off <= tol, "sphere off-surface at ({s},{t}): {off}");
            }
        }
    }

    #[test]
    fn torus_ring_matches_analytic_on_grid() {
        assert_torus_grid(Torus::new(Point3::new(1.0, -2.0, 0.5), 3.0, 1.0));
    }

    #[test]
    fn torus_horn_matches_analytic_on_grid() {
        assert_torus_grid(Torus::new(Point3::origin(), 2.0, 2.0));
    }

    #[test]
    fn torus_tiny_pi_scale_matches_analytic() {
        // Pi horn-fillet scale (0.1 mm).
        assert_torus_grid(Torus::new(Point3::new(0.03, -0.01, 0.02), 0.1, 0.1));
    }

    #[test]
    fn torus_fp_near_horn_is_representable() {
        // Pi torus #1: large radius a few ulps below small radius. Must NOT be
        // rejected as a spindle and must convert geometrically-exactly.
        let torus = Torus::new(
            Point3::origin(),
            0.099_999_999_992_725,
            0.099_999_999_999_987_88,
        );
        assert!(torus.try_into_homogeneous_bspline_surface().is_some());
        assert_torus_grid(torus);
    }

    #[test]
    fn torus_horn_inner_equator_collapses_to_center() {
        let torus = Torus::new(Point3::new(0.5, 0.5, 0.5), 1.5, 1.5);
        let conv = as_nurbs(torus.try_into_homogeneous_bspline_surface().unwrap());
        // Tube angle pi (v-knot 0.5) is the inner equator -> a single point at
        // the center for a horn torus.
        for s in grid_params() {
            let p = conv.subs(s, 0.5);
            assert!(
                p.distance(torus.center()) <= 1.0e-9 * torus.large_radius(),
                "horn inner equator not at center at s={s}: {p:?}"
            );
        }
    }

    #[test]
    fn torus_seams_are_periodic() {
        let torus = Torus::new(Point3::origin(), 4.0, 1.0);
        let conv = as_nurbs(torus.try_into_homogeneous_bspline_surface().unwrap());
        let tol = 1.0e-9 * (torus.large_radius() + torus.small_radius());
        for g in grid_params() {
            assert!(
                conv.subs(0.0, g).distance(conv.subs(1.0, g)) <= tol,
                "ring seam not periodic at v={g}"
            );
            assert!(
                conv.subs(g, 0.0).distance(conv.subs(g, 1.0)) <= tol,
                "tube seam not periodic at u={g}"
            );
        }
    }

    #[test]
    fn torus_spindle_is_rejected() {
        // Small radius exceeds large radius: a self-intersecting spindle torus,
        // not a valid B-rep face.
        let torus = Torus::new(Point3::origin(), 1.0, 2.0);
        assert!(torus.try_into_homogeneous_bspline_surface().is_none());
        assert!(torus.try_into_bspline_surface().is_none());
    }

    #[test]
    fn torus_is_rational_not_polynomial() {
        let torus = Torus::new(Point3::origin(), 3.0, 1.0);
        assert!(torus.try_into_bspline_surface().is_none());
    }

    #[test]
    fn sphere_unit_matches_analytic_on_grid() {
        assert_sphere_grid(Sphere::new(Point3::origin(), 1.0));
    }

    #[test]
    fn sphere_offset_matches_analytic_on_grid() {
        assert_sphere_grid(Sphere::new(Point3::new(1.0, 2.0, 3.0), 4.56));
    }

    #[test]
    fn sphere_tiny_matches_analytic_on_grid() {
        assert_sphere_grid(Sphere::new(Point3::new(-0.02, 0.05, 0.01), 0.1));
    }

    #[test]
    fn sphere_poles_collapse() {
        let sphere = Sphere::new(Point3::new(1.0, -1.0, 2.0), 2.5);
        let conv = as_nurbs(sphere.try_into_homogeneous_bspline_surface().unwrap());
        let north = sphere.center() + Vector3::new(0.0, 0.0, sphere.radius());
        let south = sphere.center() + Vector3::new(0.0, 0.0, -sphere.radius());
        let tol = 1.0e-9 * sphere.radius();
        for t in grid_params() {
            assert!(
                conv.subs(0.0, t).distance(north) <= tol,
                "north pole at t={t}"
            );
            assert!(
                conv.subs(1.0, t).distance(south) <= tol,
                "south pole at t={t}"
            );
        }
    }

    #[test]
    fn sphere_longitude_seam_is_periodic() {
        let sphere = Sphere::new(Point3::origin(), 3.0);
        let conv = as_nurbs(sphere.try_into_homogeneous_bspline_surface().unwrap());
        let tol = 1.0e-9 * sphere.radius();
        for s in grid_params() {
            assert!(
                conv.subs(s, 0.0).distance(conv.subs(s, 1.0)) <= tol,
                "longitude seam not periodic at u={s}"
            );
        }
    }

    #[test]
    fn sphere_is_rational_not_polynomial() {
        assert!(
            Sphere::new(Point3::origin(), 1.0)
                .try_into_bspline_surface()
                .is_none()
        );
    }

    #[test]
    fn transformed_torus_via_processor_lies_on_surface() {
        // General placement rides the pre-existing `Processor<_, Matrix4>` arm;
        // confirm the composed surface is the correctly reoriented torus.
        let torus = Torus::new(Point3::origin(), 3.0, 1.0);
        // Rotate about x by 0.6 rad (tilts the torus axis off +z), then translate.
        let (sn, cs) = 0.6f64.sin_cos();
        let rot_x = Matrix4::from_cols(
            Vector4::new(1.0, 0.0, 0.0, 0.0),
            Vector4::new(0.0, cs, sn, 0.0),
            Vector4::new(0.0, -sn, cs, 0.0),
            Vector4::new(0.0, 0.0, 0.0, 1.0),
        );
        let transform = Matrix4::from_translation(Vector3::new(5.0, -3.0, 2.0)) * rot_x;
        let placed = Processor::with_transform(torus, transform);
        let conv = as_nurbs(
            placed
                .try_into_homogeneous_bspline_surface()
                .expect("placed torus converts"),
        );
        let inv = transform.invert().expect("rigid transform is invertible");
        let tol = 1.0e-9 * (torus.large_radius() + torus.small_radius());
        for s in grid_params() {
            for t in grid_params() {
                let got = conv.subs(s, t);
                let local_h = inv * got.to_homogeneous();
                let local = Point3::new(
                    local_h.x / local_h.w,
                    local_h.y / local_h.w,
                    local_h.z / local_h.w,
                );
                let off = torus_off_surface(&torus, local).abs();
                assert!(off <= tol, "placed torus off-surface at ({s},{t}): {off}");
            }
        }
    }
}
