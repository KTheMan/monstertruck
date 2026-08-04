//! Traits for extracting exact B-spline surface representations.

use crate::prelude::*;

/// A closed `(u, v)` rectangle in a surface's own parameter frame, as
/// `((u_min, u_max), (v_min, v_max))`.
pub type SurfaceParameterRectangle = ((f64, f64), (f64, f64));

/// Which parameter axes of a converted control net carry the surface's OWN
/// parameter as their knot span, rather than the renormalized `[0, 1]` knot
/// frame the untrimmed conversions emit. `(u, v)`.
pub type SurfaceFrameAxes = (bool, bool);

/// A trim-aware homogeneous conversion: the control net, plus which of its
/// parameter axes are already expressed in the surface's own parameter frame.
///
/// A consumer that re-labels patch domains from the knot frame onto the
/// surface's parameter frame must SKIP the axes flagged here -- they are already
/// there, and re-labelling them would squash a trim-spanning net back onto the
/// profile curve's incidental range, reintroducing exactly the inconsistency
/// this conversion exists to remove.
#[derive(Clone, Debug)]
pub struct HomogeneousSurfaceConversion {
    /// The homogeneous control net.
    pub surface: BsplineSurface<Vector4>,
    /// Axes whose knot span IS the surface's own parameter.
    pub surface_frame_axes: SurfaceFrameAxes,
}

impl From<BsplineSurface<Vector4>> for HomogeneousSurfaceConversion {
    #[inline]
    fn from(surface: BsplineSurface<Vector4>) -> Self {
        Self {
            surface,
            surface_frame_axes: (false, false),
        }
    }
}

/// Whether `requested` reaches outside `own` by more than the padding a trim
/// range carries by construction.
///
/// The consumer pads every reported trim axis by
/// `max(TOLERANCE, span * 1e-9)` (`expand_param_axis_range`) before anyone can
/// read it, so a request that differs from the profile's own range by no more
/// than that pad IS the profile's own range, and re-spanning on it would perturb
/// exact geometry for nothing. The scale-relative `span * 1e-9` term is what
/// makes this survive a large model; the `TOLERANCE` floor is not a threshold of
/// this function's own choosing, it is the upstream pad reproduced.
fn parameter_range_reaches_outside(requested: (f64, f64), own: (f64, f64)) -> bool {
    let (requested_min, requested_max) =
        (requested.0.min(requested.1), requested.0.max(requested.1));
    let (own_min, own_max) = (own.0.min(own.1), own.0.max(own.1));
    if !(requested_min.is_finite()
        && requested_max.is_finite()
        && own_min.is_finite()
        && own_max.is_finite())
    {
        return false;
    }
    let pad = TOLERANCE.max((requested_max - requested_min).abs() * 1.0e-9);
    requested_min < own_min - pad || requested_max > own_max + pad
}

/// Attempts to produce an exact homogeneous B-spline curve representation.
///
/// The returned control net is in homogeneous coordinates, so true rational
/// NURBS curves remain exact.
pub trait TryIntoHomogeneousBsplineCurve {
    /// Converts this curve into a homogeneous B-spline curve, if possible.
    fn try_into_homogeneous_bspline_curve(&self) -> Option<BsplineCurve<Vector4>>;

    /// Converts this curve into a homogeneous B-spline curve spanning the given
    /// parameter interval, which may lie wholly OUTSIDE the curve's own bounded
    /// range.
    ///
    /// Only curves that carry an exact analytic continuation beyond their own
    /// range -- in practice, straight [`Line`]s -- can answer this; every other
    /// curve returns [`None`] and the caller falls back to
    /// [`Self::try_into_homogeneous_bspline_curve`]. Extrapolating a B-spline
    /// or a trimmed conic past its knot span would NOT be the same curve, so the
    /// default is a refusal rather than a guess.
    ///
    /// The returned curve's knot vector spans `range` itself, i.e. the emitted
    /// parameter IS the curve's own parameter, not a renormalized copy.
    #[inline]
    fn try_into_homogeneous_bspline_curve_over(
        &self,
        _range: (f64, f64),
    ) -> Option<BsplineCurve<Vector4>> {
        None
    }
}

/// Attempts to produce an exact homogeneous B-spline surface representation.
///
/// The returned control net is in homogeneous coordinates, so true rational
/// NURBS surfaces remain exact.
pub trait TryIntoHomogeneousBsplineSurface {
    /// Converts this surface into a homogeneous B-spline surface, if possible.
    fn try_into_homogeneous_bspline_surface(&self) -> Option<BsplineSurface<Vector4>>;

    /// Converts this surface into a homogeneous B-spline surface whose control
    /// net covers the given trim rectangle in this surface's OWN parameter
    /// frame.
    ///
    /// Swept analytic surfaces (`RevolutionSurface`, and the extrusions that
    /// share its shape) are built from a profile curve whose bounded parameter
    /// range is incidental: a STEP `CYLINDRICAL_SURFACE` arrives as a
    /// revolution of a UNIT-LENGTH line, so the naive conversion emits a
    /// one-unit slab of a surface the analytic form treats as unbounded, and any
    /// consumer that reads the emitted control hull is told the face does not
    /// reach where it plainly does. Passing the face's real trim rectangle here
    /// makes the emitted net span the extent the CONSUMER needs instead.
    ///
    /// `None` -- and every implementation that does not override this -- is
    /// exactly [`Self::try_into_homogeneous_bspline_surface`], reported as
    /// carrying no surface-frame axis.
    #[inline]
    fn try_into_homogeneous_bspline_surface_over(
        &self,
        _parameter_range: Option<SurfaceParameterRectangle>,
    ) -> Option<HomogeneousSurfaceConversion> {
        self.try_into_homogeneous_bspline_surface().map(Into::into)
    }
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

    /// A line is defined for every real parameter, so its restriction to any
    /// interval is the SAME line, exactly. The emitted degree-1 Bezier carries
    /// the requested interval as its knot vector, so `subs(t)` still equals
    /// `Line::subs(t)` for every `t` in it -- the parameterization is preserved,
    /// not renormalized.
    #[inline]
    fn try_into_homogeneous_bspline_curve_over(
        &self,
        range: (f64, f64),
    ) -> Option<BsplineCurve<Vector4>> {
        let (start, end) = (range.0.min(range.1), range.0.max(range.1));
        if !start.is_finite() || !end.is_finite() || end <= start {
            return None;
        }
        let direction = self.1 - self.0;
        Some(BsplineCurve::lift_up(BsplineCurve::new(
            KnotVector::from(vec![start, start, end, end]),
            vec![self.0 + direction * start, self.0 + direction * end],
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

impl<C> RevolutionSurface<C>
where C: TryIntoHomogeneousBsplineCurve
{
    /// The profile curve to sweep, given the face's trim rectangle in this
    /// surface's own parameter frame (`u` = profile parameter, `v` = revolution
    /// angle), and whether it was re-spanned onto the requested range.
    ///
    /// Re-spanning happens only when the request REACHES OUTSIDE the profile
    /// curve's own knot span -- a face that lives entirely inside it is already
    /// covered and is left byte-identical, which is what keeps the unit-height
    /// fillet faces and every frozen fixture unmoved. When it does happen the
    /// emitted knot span is the requested range itself, so the swept surface's
    /// profile parameter IS the surface's own profile parameter and the consumer
    /// is told so via [`SurfaceFrameAxes`].
    fn profile_curve_over(
        &self,
        parameter_range: Option<SurfaceParameterRectangle>,
    ) -> Option<(BsplineCurve<Vector4>, bool)> {
        let own = self.entity_curve().try_into_homogeneous_bspline_curve()?;
        let knots = own.knot_vector();
        let own_span = (*knots.first()?, *knots.last()?);
        let widened = parameter_range
            .map(|range| range.0)
            .filter(|profile_range| parameter_range_reaches_outside(*profile_range, own_span))
            .and_then(|profile_range| {
                self.entity_curve()
                    .try_into_homogeneous_bspline_curve_over(profile_range)
            });
        Some(match widened {
            Some(curve) => (curve, true),
            None => (own, false),
        })
    }

    fn revolve_profile(&self, profile: &BsplineCurve<Vector4>) -> Option<BsplineSurface<Vector4>> {
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

impl<C> TryIntoHomogeneousBsplineSurface for RevolutionSurface<C>
where C: TryIntoHomogeneousBsplineCurve
{
    fn try_into_homogeneous_bspline_surface(&self) -> Option<BsplineSurface<Vector4>> {
        let profile = self.entity_curve().try_into_homogeneous_bspline_curve()?;
        self.revolve_profile(&profile)
    }

    fn try_into_homogeneous_bspline_surface_over(
        &self,
        parameter_range: Option<SurfaceParameterRectangle>,
    ) -> Option<HomogeneousSurfaceConversion> {
        let (profile, respanned) = self.profile_curve_over(parameter_range)?;
        // `u` is the profile axis of a `RevolutionSurface`; `v` (the revolution
        // angle) stays in the normalized circle knot frame either way.
        Some(HomogeneousSurfaceConversion {
            surface: self.revolve_profile(&profile)?,
            surface_frame_axes: (respanned, false),
        })
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
        // `BsplineSurface` indexes `control_points[u][v]`, and `try_new` checks
        // `knot_vecs.0` against the OUTER length and `knot_vecs.1` against the
        // inner one. The knot vectors above are (profile, sweep), so the outer
        // index must be the profile and the inner one the two sweep rows -- one
        // 2-entry row per profile control point, NOT two rows of profile control
        // points.
        //
        // Emitting it the other way round is not a cosmetic transposition: it
        // pairs the profile's knot vector with a 2-long axis and the sweep's
        // 4-knot Bezier vector with the profile's control points, so any profile
        // with 4 or more control points fails `try_new`'s knot rule and
        // `BsplineSurface::new` PANICS, while shorter profiles silently produce a
        // surface with `u` and `v` exchanged. Measured 2026-07-30 on `Ai-14R.stp`:
        // all 3,341 `SURFACE_OF_LINEAR_EXTRUSION` faces panicked here.
        //
        // At v=0 the control point is the profile's own; at v=1 it is translated
        // by the extrusion vector, in homogeneous form
        // (x,y,z,w) -> (x+dx*w, y+dy*w, z+dz*w, w).
        let control_points: Vec<Vec<Vector4>> = curve
            .control_points()
            .iter()
            .map(|p| {
                vec![
                    *p,
                    Vector4::new(p.x + dir.x * p.w, p.y + dir.y * p.w, p.z + dir.z * p.w, p.w),
                ]
            })
            .collect();
        Some(BsplineSurface::new(knot_vecs, control_points))
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

    /// An inverted processor SWAPS the `(u, v)` axes (see the `ParametricSurface`
    /// impl), so the trim rectangle has to be swapped back into the entity's own
    /// frame before it is forwarded -- otherwise a STEP cylinder, which is
    /// exactly such an inverted processor, would be handed its angular range as
    /// the profile range.
    fn try_into_homogeneous_bspline_surface_over(
        &self,
        parameter_range: Option<SurfaceParameterRectangle>,
    ) -> Option<HomogeneousSurfaceConversion> {
        let entity_range = parameter_range.map(|(u_range, v_range)| match self.orientation() {
            true => (u_range, v_range),
            false => (v_range, u_range),
        });
        let HomogeneousSurfaceConversion {
            mut surface,
            surface_frame_axes,
        } = self
            .entity()
            .try_into_homogeneous_bspline_surface_over(entity_range)?;
        surface
            .control_points_mut()
            .for_each(|point| *point = *self.transform() * *point);
        let surface_frame_axes = match self.orientation() {
            true => surface_frame_axes,
            false => {
                surface.invert();
                (surface_frame_axes.1, surface_frame_axes.0)
            }
        };
        Some(HomogeneousSurfaceConversion {
            surface,
            surface_frame_axes,
        })
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

    // -----------------------------------------------------------------------
    // Trim-driven span of the swept analytic surfaces (stage P-CONV).
    //
    // A STEP `CYLINDRICAL_SURFACE` loads as
    // `Processor<RevolutionSurface<Line<Point3>>, Matrix4>` with a UNIT-LENGTH
    // profile line and `orientation = false`, so the untrimmed conversion emits
    // one axial unit of a surface the analytic form treats as unbounded. Nothing
    // in the workspace pinned that before, which is why it survived.
    // -----------------------------------------------------------------------

    /// The STEP loader's own construction (`step_types.rs`, `CylindricalSurface`
    /// -> `step_geometry::CylindricalSurface`): a unit-length profile line at
    /// `center + x * radius`, revolved about `axis`, wrapped in an INVERTED
    /// processor (which swaps the `(u, v)` axes, so `v` is the profile axis).
    fn step_cylinder(
        center: Point3,
        axis: Vector3,
        radius: f64,
    ) -> Processor<RevolutionSurface<Line<Point3>>, Matrix4> {
        let radial = Vector3::unit_x();
        let start = center + radial * radius;
        let mut cylinder = Processor::new(RevolutionSurface::by_revolution(
            Line(start, start + axis),
            center,
            axis,
        ));
        cylinder.invert();
        cylinder
    }

    fn control_net_bbox(surface: &BsplineSurface<Vector4>) -> (Point3, Point3) {
        surface
            .control_points()
            .iter()
            .flat_map(|row| row.iter())
            .map(|point| point.to_point())
            .fold(
                (
                    Point3::new(f64::MAX, f64::MAX, f64::MAX),
                    Point3::new(f64::MIN, f64::MIN, f64::MIN),
                ),
                |(min, max), point| {
                    (
                        Point3::new(min.x.min(point.x), min.y.min(point.y), min.z.min(point.z)),
                        Point3::new(max.x.max(point.x), max.y.max(point.y), max.z.max(point.z)),
                    )
                },
            )
    }

    /// THE REGRESSION PIN. A cylinder whose face is trimmed to `v` in `[0, 80]`
    /// must convert to a control net that spans all 80 units, not the profile
    /// line's incidental 1.
    #[test]
    fn trimmed_cylinder_control_net_spans_the_face_not_the_profile() {
        let cylinder = step_cylinder(Point3::origin(), Vector3::unit_z(), 8.0);

        // Untrimmed: the one-unit stub, unchanged.
        let stub = cylinder
            .try_into_homogeneous_bspline_surface()
            .expect("cylinder converts");
        let (stub_min, stub_max) = control_net_bbox(&stub);
        assert!(
            (stub_max.z - stub_min.z - 1.0).abs() < 1.0e-12,
            "{stub_min:?} {stub_max:?}"
        );

        // `v` is the profile axis (the processor is inverted), so the face's
        // trim rectangle is (angle, height).
        let converted = cylinder
            .try_into_homogeneous_bspline_surface_over(Some(((0.0, 1.0), (0.0, 80.0))))
            .expect("trimmed cylinder converts");
        assert_eq!(
            converted.surface_frame_axes,
            (false, true),
            "the re-spanned axis must be reported to the consumer"
        );
        let trimmed = converted.surface;
        let (min, max) = control_net_bbox(&trimmed);
        assert!(
            min.z <= 0.0 + 1.0e-12 && max.z >= 80.0 - 1.0e-12,
            "converted control net must cover the whole 80-unit face, got z in [{}, {}]",
            min.z,
            max.z
        );
        // Radially it is still exactly the r = 8 cylinder's control polygon: the
        // rational-quadratic circle's control points project onto the square
        // circumscribing radius 8, so the net's x/y extent is exactly +-8.
        assert!((max.x - 8.0).abs() < 1.0e-9 && (min.x + 8.0).abs() < 1.0e-9);
        assert!((max.y - 8.0).abs() < 1.0e-9 && (min.y + 8.0).abs() < 1.0e-9);

        // Knot span: the emitted `v` parameter IS the surface's own profile
        // parameter over the requested interval, not a renormalized copy.
        let (_, v_knots) = trimmed.knot_vectors();
        assert!((v_knots[0] - 0.0).abs() < 1.0e-12);
        assert!((v_knots[v_knots.len() - 1] - 80.0).abs() < 1.0e-12);

        // And it is still EXACTLY the same cylinder: every evaluated point sits
        // on radius 8, and `subs` agrees with the analytic surface.
        let conv = as_nurbs(trimmed);
        for s in grid_params() {
            for t in grid_params() {
                let v = 80.0 * t;
                let got = conv.subs(s, v);
                let want = ParametricSurface::evaluate(&cylinder, s * 2.0 * PI, v);
                let radius = (got.x * got.x + got.y * got.y).sqrt();
                assert!(
                    (radius - 8.0).abs() <= 1.0e-9 * 80.0,
                    "off-cylinder at ({s},{v}): r={radius}"
                );
                assert!(
                    (got.z - v).abs() <= 1.0e-9 * 80.0,
                    "height mismatch at ({s},{v}): {got:?}"
                );
                // The angular map is the exact rational-circle map, so compare
                // against the analytic surface at the SAME evaluated angle.
                let angle = full_circle_angle(s);
                let want_at_angle = ParametricSurface::evaluate(&cylinder, angle, v);
                assert!(
                    got.distance(want_at_angle) <= 1.0e-9 * 80.0,
                    "subs mismatch at ({s},{v}): got {got:?} want {want_at_angle:?} \
                     (untrimmed reference {want:?})"
                );
            }
        }
    }

    /// The widening is DISJOINTNESS-gated: a trim that merely overlaps the
    /// profile's own range leaves the conversion byte-identical, which is what
    /// keeps the unit-height fillet faces (and every frozen fixture) unmoved.
    #[test]
    fn overlapping_trim_leaves_the_cylinder_conversion_byte_identical() {
        let cylinder = step_cylinder(Point3::new(1.0, -2.0, 3.0), Vector3::unit_z(), 2.5);
        let plain = cylinder
            .try_into_homogeneous_bspline_surface()
            .expect("cylinder converts");
        for trim in [
            // Exactly the profile's own range.
            ((0.0, 1.0), (0.0, 1.0)),
            // The upstream trim pad (`expand_param_axis_range`, at least
            // TOLERANCE) -- the same range, reported padded.
            ((0.0, 1.0), (-1.0e-6, 1.0 + 1.0e-6)),
            // Strictly inside: already covered, nothing to re-span.
            ((0.0, 1.0), (0.25, 0.75)),
        ] {
            let over = cylinder
                .try_into_homogeneous_bspline_surface_over(Some(trim))
                .expect("cylinder converts");
            assert_eq!(over.surface_frame_axes, (false, false));
            let over = over.surface;
            assert_eq!(
                over.control_points(),
                plain.control_points(),
                "overlapping trim {trim:?} must not move the control net"
            );
            assert_eq!(over.knot_vectors(), plain.knot_vectors());
        }
        // ...but a trim that genuinely reaches BELOW the profile start does
        // re-span: the face really is somewhere the naive sweep does not go.
        let below = cylinder
            .try_into_homogeneous_bspline_surface_over(Some(((0.0, 1.0), (-0.6, 1.0))))
            .expect("cylinder converts");
        assert_eq!(below.surface_frame_axes, (false, true));
        let (_, v_knots) = below.surface.knot_vectors();
        assert!((v_knots[0] + 0.6).abs() < 1.0e-12);
    }

    /// `None` -- and every non-swept surface -- is exactly the plain conversion.
    #[test]
    fn untrimmed_and_non_swept_conversions_are_unchanged() {
        let cylinder = step_cylinder(Point3::origin(), Vector3::unit_z(), 3.0);
        assert_eq!(
            cylinder
                .try_into_homogeneous_bspline_surface_over(None)
                .unwrap()
                .surface
                .control_points(),
            cylinder
                .try_into_homogeneous_bspline_surface()
                .unwrap()
                .control_points()
        );
        let trim = Some(((0.0, 1.0), (-500.0, -400.0)));
        for (over, plain) in [
            (
                Sphere::new(Point3::new(1.0, 2.0, 3.0), 4.0)
                    .try_into_homogeneous_bspline_surface_over(trim),
                Sphere::new(Point3::new(1.0, 2.0, 3.0), 4.0).try_into_homogeneous_bspline_surface(),
            ),
            (
                Torus::new(Point3::origin(), 3.0, 1.0)
                    .try_into_homogeneous_bspline_surface_over(trim),
                Torus::new(Point3::origin(), 3.0, 1.0).try_into_homogeneous_bspline_surface(),
            ),
            (
                Plane::new(
                    Point3::origin(),
                    Point3::new(1.0, 0.0, 0.0),
                    Point3::new(0.0, 1.0, 0.0),
                )
                .try_into_homogeneous_bspline_surface_over(trim),
                Plane::new(
                    Point3::origin(),
                    Point3::new(1.0, 0.0, 0.0),
                    Point3::new(0.0, 1.0, 0.0),
                )
                .try_into_homogeneous_bspline_surface(),
            ),
        ] {
            assert_eq!(
                over.map(|converted| converted.surface.control_points().clone()),
                plain.map(|surface| surface.control_points().clone())
            );
        }
    }

    /// A cone is the same `RevolutionSurface<Line>` with a slanted profile, so
    /// the widened sweep must stay the exact cone -- the radius has to keep
    /// growing linearly along the axis, not freeze at the stub's.
    #[test]
    fn trimmed_cone_stays_an_exact_cone() {
        // Half-angle 45 degrees: profile from (1, 0, 0) towards (2, 0, 1).
        let start = Point3::new(1.0, 0.0, 0.0);
        let mut cone = Processor::new(RevolutionSurface::by_revolution(
            Line(start, start + Vector3::new(1.0, 0.0, 1.0)),
            Point3::origin(),
            Vector3::unit_z(),
        ));
        cone.invert();
        let conv = as_nurbs(
            cone.try_into_homogeneous_bspline_surface_over(Some(((0.0, 1.0), (10.0, 40.0))))
                .expect("cone converts")
                .surface,
        );
        for s in grid_params() {
            for t in grid_params() {
                let v = 10.0 + 30.0 * t;
                let point = conv.subs(s, v);
                let radius = (point.x * point.x + point.y * point.y).sqrt();
                // On this cone radius == 1 + height and height == v.
                assert!((point.z - v).abs() <= 1.0e-9 * 40.0, "height at ({s},{v})");
                assert!(
                    (radius - (1.0 + v)).abs() <= 1.0e-9 * 40.0,
                    "off-cone at ({s},{v}): r={radius} expected {}",
                    1.0 + v
                );
            }
        }
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
