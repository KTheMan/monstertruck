//! Axis-aligned bounds that are actually BOUNDS.
//!
//! # Why this module exists
//!
//! The obvious way to box a B-rep solid -- fold its vertices into a
//! [`BoundingBox`] -- is not a bounding box at all once a face is curved. A
//! vertex is a CORNER of the topology; the surface between corners bulges past
//! it, and nothing in the B-rep says by how much. The measured witness (spec
//! 013, ledger class C15) is ROTOR `#25387`: a sphere of radius `12.5` sliced to
//! the slab `|x| <= 9` and bored by a cylinder of radius `6`. Its vertices all
//! sit on the four trim circles, so the vertex box is `18 x 17.349 x 12`
//! (`3747.4`), while the solid's own closed-form volume is `5273.16` -- the
//! CORRECT volume exceeds its "bounding" box by 40.7%. Any test that certifies
//! a volume with `v <= vertex_box` therefore rejects correct answers and
//! accepts a whole family of wrong ones.
//!
//! What this module computes instead is a bound with a proof attached, per
//! face, and it returns [`None`] rather than guess when it has no proof. A
//! caller that gets `None` has learned something true (this solid carries a
//! face class we cannot certify); a caller that gets `Some` has a box the solid
//! provably fits inside.
//!
//! # The per-face arguments
//!
//! A solid is contained in the convex hull of its boundary, and its boundary is
//! the union of its faces, so a box containing every face contains the solid.
//! Each surface class gets its own argument:
//!
//! * **B-spline / NURBS surface** -- the convex-hull property: a B-spline patch
//!   lies in the convex hull of its control net, and a rational patch with
//!   strictly positive weights lies in the convex hull of the PROJECTED control
//!   points. A non-positive weight voids the property, so it refuses.
//! * **Plane** -- a planar face is the region its boundary wires enclose, so it
//!   lies in the convex hull of those wires. Bound the wires (see below) and
//!   the face is bounded.
//! * **Sphere / torus** -- bound the WHOLE analytic surface, untrimmed. Sound
//!   by construction, and loose by exactly the amount the trim would have
//!   removed (see [`analytic_sphere_looseness`] for the closed form).
//! * **Surface of revolution over a straight profile** (this is what STEP
//!   cylinders and cones become: `RevolutionSurface<Line>`) -- the untrimmed
//!   surface is UNBOUNDED, since the profile line's parameter is not clamped to
//!   the segment the STEP builder handed it. But on such a surface both the
//!   axial coordinate and the distance from the axis are functions of the
//!   profile parameter `u` ALONE. The face is compact and connected, so its
//!   `u`-projection is an interval whose ends are attained on its boundary; the
//!   axial coordinate is affine in `u` and the radial distance is convex in
//!   `u`, so both attain their extremes over the face at those same ends.
//!   Bounding the face's boundary wires therefore bounds the face.
//!
//! Anything else -- a T-spline patch, a surface behind a non-similarity
//! transform, a general surface of revolution -- refuses.
//!
//! # Bounding a wire
//!
//! Edge curves get the same treatment: a segment is its two endpoints, a
//! B-spline curve its control polygon, a rational curve its projected control
//! polygon (positive weights only). A parameter curve or an intersection curve
//! carries no cheap hull -- its `leader` is an APPROXIMATION, not an enclosure
//! -- so those refuse too.
//!
//! # This is a bound, not a tight bound
//!
//! Untrimmed analytic surfaces are the loose part. On `#25387` the sphere faces
//! alone force the full `25 x 25 x 25` box (`15625`) where the solid actually
//! occupies `18 x 25 x 25`. That is sound and it is ~3x the solid's volume; a
//! caller wanting tightness should say so and get a different instrument, not a
//! quietly-unsound one. MEASURED over the four in-repo fixtures, as a ratio of
//! the certified box to the vertex hull: boxy 1.00x, ap224 1.15x, io1 2.95x,
//! coffy 71.5x. The tail is the untrimmed-surface classes, and coffy shows what
//! they cost when a face is a small trim of a large surface.
//!
//! # What it assumes, stated so nobody has to guess
//!
//! * A B-spline / NURBS patch or curve is evaluated within its own knot domain.
//!   The convex-hull property is a statement about that domain; extrapolation
//!   beyond it is not bounded by the control net and is not claimed here.
//!   (Trims are erased before geometry mapping, so a face's domain is its
//!   surface's domain.)
//! * A face is compact and connected -- used only in the revolution argument,
//!   to say that a `u`-extreme of the face is attained on its boundary.
//! * A `Processor`'s `Matrix4` is a similarity. It is checked, not assumed:
//!   anything else refuses.

use crate::*;

/// How much an analytic sphere's axis-aligned bounding box exceeds the sphere:
/// `(2r)^3 / (4/3 pi r^3) = 6 / pi`, independent of the radius.
///
/// Stated as a constant because it is the honest answer to "how loose is this
/// bound?" for the commonest curved class: a certified box around a ball is
/// 91.0% larger than the ball. Anything much looser than this on a
/// sphere-dominated solid is a bug in the caller's expectations, not in the
/// bound.
pub const ANALYTIC_SPHERE_LOOSENESS: f64 = 6.0 / std::f64::consts::PI;

/// The ratio of the certified box's volume to the enclosed ball's volume, for a
/// ball of any radius: exactly [`ANALYTIC_SPHERE_LOOSENESS`].
#[inline]
pub const fn analytic_sphere_looseness() -> f64 { ANALYTIC_SPHERE_LOOSENESS }

/// Relative tolerance for judging a `Matrix4` a similarity.
const SIMILARITY_TOL: f64 = 1.0e-9;

/// A certified axis-aligned box containing every point of `solid`, or [`None`]
/// when any one of its faces carries a class this module cannot prove a bound
/// for.
///
/// The box contains the solid's SURFACE and therefore -- the surface being a
/// closed boundary -- the solid. See the module note for the per-class
/// arguments and for how loose the result is.
pub fn certified_solid_bounding_box(solid: &Solid) -> Option<BoundingBox<Point3>> {
    let mut result = BoundingBox::<Point3>::new();
    let mut any = false;
    for shell in solid.boundaries() {
        for face in shell.face_iter() {
            result += certified_face_bounding_box(face)?;
            any = true;
        }
    }
    any.then_some(result)
}

/// A certified axis-aligned box containing every point of `face`, or [`None`].
pub fn certified_face_bounding_box(face: &Face) -> Option<BoundingBox<Point3>> {
    let surface = face.surface();
    let hull = face_boundary_hull(face);
    certified_surface_bounding_box(&surface, hull.as_deref())
}

/// A certified axis-aligned box containing a face carried by `surface` whose
/// boundary wires are enclosed by the convex hull of `boundary_hull`, or
/// [`None`] when no argument in this module applies.
///
/// `boundary_hull` may be [`None`] (the caller could not bound the wires); the
/// classes that need it then refuse, while the self-bounded analytic and
/// control-net classes still answer.
pub fn certified_surface_bounding_box(
    surface: &Surface,
    boundary_hull: Option<&[Point3]>,
) -> Option<BoundingBox<Point3>> {
    match surface {
        // A planar face is exactly what its wires enclose.
        Surface::Plane(_) => hull_box(boundary_hull?),
        // Convex-hull property, straight.
        Surface::BsplineSurface(surface) => {
            hull_box_iter(surface.control_points().iter().flatten().copied())
        }
        // Convex-hull property, rational: only with strictly positive weights.
        Surface::NurbsSurface(surface) => {
            let mut points = Vec::new();
            for row in surface.control_points() {
                for control in row {
                    points.push(projected_control_point(*control)?);
                }
            }
            hull_box(&points)
        }
        // The whole ball, untrimmed.
        Surface::SphericalSurface(processor) => {
            let scale = similarity_scale(processor.transform())?;
            let sphere = processor.entity();
            let center = transform_point(processor.transform(), sphere.center());
            let radius = scale * sphere.radius();
            (radius.is_finite() && radius >= 0.0).then(|| {
                let offset = Vector3::new(radius, radius, radius);
                BoundingBox::from_iter([center - offset, center + offset])
            })
        }
        // The whole ring torus, untrimmed: `|radial| <= R + r`, `|axial| <= r`
        // about its own axis, which the transform carries to world space.
        Surface::ToroidalSurface(processor) => {
            let scale = similarity_scale(processor.transform())?;
            let torus = processor.entity();
            let center = transform_point(processor.transform(), torus.center());
            let axis = transform_direction(processor.transform(), Vector3::unit_z())?;
            let large = scale * torus.large_radius().abs();
            let small = scale * torus.small_radius().abs();
            (large.is_finite() && small.is_finite())
                .then(|| revolution_region_box(center, axis, (-small, small), large + small))
        }
        // Cylinders and cones. Bounded only via the face's own wires -- see the
        // module note on why the untrimmed surface is not bounded at all.
        Surface::RevolutionSurface(processor) => {
            let revolution = processor.entity();
            // The convexity/affineness argument is stated for a STRAIGHT
            // profile and is claimed for nothing else.
            if !matches!(revolution.entity_curve(), Curve::Line(_)) {
                return None;
            }
            similarity_scale(processor.transform())?;
            let origin = transform_point(processor.transform(), revolution.origin());
            let axis = transform_direction(processor.transform(), revolution.axis())?;
            revolution_face_box(origin, axis, boundary_hull?)
        }
        // No hull argument claimed for a T-mesh here.
        Surface::TsplineSurface(_) => None,
    }
}

/// The face classes of `solid` that [`certified_solid_bounding_box`] declines,
/// each as `surface-class[/curve-class ...]`, deduplicated and sorted.
///
/// Diagnosis, not certification: an empty result means every face was bounded.
/// A caller staring at a `None` needs to know WHICH class it tripped over, and
/// guessing is how instruments start lying.
pub fn uncertifiable_face_classes(solid: &Solid) -> Vec<String> {
    let mut classes: Vec<String> = Vec::new();
    for shell in solid.boundaries() {
        for face in shell.face_iter() {
            if certified_face_bounding_box(face).is_some() {
                continue;
            }
            let surface = face.surface();
            let mut blockers: Vec<&'static str> = face
                .edge_iter()
                .filter_map(|edge| {
                    let curve = edge.curve();
                    let mut sink = Vec::new();
                    push_curve_hull(&curve, &mut sink)
                        .is_none()
                        .then(|| curve_class_name(&curve))
                })
                .collect();
            blockers.sort_unstable();
            blockers.dedup();
            let mut label = surface_class_name(&surface).to_string();
            for blocker in blockers {
                label.push('/');
                label.push_str(blocker);
            }
            classes.push(label);
        }
    }
    classes.sort();
    classes.dedup();
    classes
}

/// The [`Surface`] variant's name, for diagnosis.
fn surface_class_name(surface: &Surface) -> &'static str {
    match surface {
        Surface::Plane(_) => "Plane",
        Surface::BsplineSurface(_) => "BsplineSurface",
        Surface::NurbsSurface(_) => "NurbsSurface",
        Surface::RevolutionSurface(_) => "RevolutionSurface",
        Surface::TsplineSurface(_) => "TsplineSurface",
        Surface::SphericalSurface(_) => "SphericalSurface",
        Surface::ToroidalSurface(_) => "ToroidalSurface",
    }
}

/// The [`Curve`] variant's name, for diagnosis.
fn curve_class_name(curve: &Curve) -> &'static str {
    match curve {
        Curve::Line(_) => "Line",
        Curve::BsplineCurve(_) => "BsplineCurve",
        Curve::NurbsCurve(_) => "NurbsCurve",
        Curve::ParameterCurve(_) => "ParameterCurve",
        Curve::IntersectionCurve(_) => "IntersectionCurve",
    }
}

/// Points whose convex hull encloses every boundary wire of `face`, or [`None`]
/// when some edge carries a curve with no cheap enclosure.
pub fn face_boundary_hull(face: &Face) -> Option<Vec<Point3>> {
    let mut points = Vec::new();
    for edge in face.edge_iter() {
        push_curve_hull(&edge.curve(), &mut points)?;
    }
    (!points.is_empty()).then_some(points)
}

/// Appends points whose convex hull encloses `curve`, or returns [`None`] when
/// the curve class carries no such finite point set.
///
/// The two composite classes never touch their stored `leader`: a leader is a
/// FITTED approximation of the curve and an approximation is not an enclosure.
/// They are bounded structurally instead --
///
/// * a parameter curve `surface(c(t))` by pushing `c`'s 2D control hull through
///   [`surface_patch_box`], and
/// * an intersection curve by the fact that it lies on BOTH its surfaces, so
///   any sound box for EITHER of them (or for either boundary parameter curve,
///   which is tighter) contains it.
///
/// -- and the box's eight corners are pushed, a convex hull containing it.
pub fn push_curve_hull(curve: &Curve, points: &mut Vec<Point3>) -> Option<()> {
    match curve {
        Curve::Line(Line(front, back)) => {
            points.push(*front);
            points.push(*back);
            Some(())
        }
        Curve::BsplineCurve(curve) => {
            points.extend(curve.control_points().iter().copied());
            Some(())
        }
        Curve::NurbsCurve(curve) => {
            for control in curve.control_points() {
                points.push(projected_control_point(*control)?);
            }
            Some(())
        }
        Curve::ParameterCurve(pcurve) => {
            push_box_corners(&parameter_curve_box(pcurve)?, points);
            Some(())
        }
        Curve::IntersectionCurve(curve) => {
            // FOUR independent enclosures, each sound on its own, so their
            // INTERSECTION is sound and no looser than the best of them.
            let candidates = [
                curve.boundary0().and_then(parameter_curve_box),
                curve.boundary1().and_then(parameter_curve_box),
                certified_surface_bounding_box(curve.surface0(), None),
                certified_surface_bounding_box(curve.surface1(), None),
            ];
            let mut tightest: Option<BoundingBox<Point3>> = None;
            for candidate in candidates.into_iter().flatten() {
                tightest = Some(match tightest {
                    None => candidate,
                    Some(current) => intersect_boxes(current, candidate)?,
                });
            }
            push_box_corners(&tightest?, points);
            Some(())
        }
    }
}

/// A sound box for the 3D curve `surface(c(t))`.
fn parameter_curve_box(
    pcurve: &ParameterCurve<Curve2D, Box<Surface>>,
) -> Option<BoundingBox<Point3>> {
    surface_patch_box(pcurve.surface(), parameter_curve_uv_box(pcurve.curve())?)
}

/// A `(u, v)` rectangle containing every point of a 2D parameter curve.
///
/// Same convex-hull arguments as the 3D case, plus: a trimmed unit circle is an
/// arc of the unit circle, so the unit square bounds it whatever the trim, and
/// the `Matrix3` placement carries that square across affinely. A trimmed
/// hyperbola or parabola gets no bound here -- both are convex arcs whose
/// interior escapes the hull of their ends, and no cheap enclosure is claimed.
fn parameter_curve_uv_box(curve: &Curve2D) -> Option<BoundingBox<Point2>> {
    let mut points: Vec<Point2> = Vec::new();
    match curve {
        Curve2D::Line(Line(front, back)) => points.extend([*front, *back]),
        Curve2D::Polyline(polyline) => points.extend(polyline.0.iter().copied()),
        Curve2D::BsplineCurve(curve) => points.extend(curve.control_points().iter().copied()),
        Curve2D::NurbsCurve(curve) => {
            for control in curve.control_points() {
                let weight = control.z;
                if !(weight > 0.0 && weight.is_finite()) {
                    return None;
                }
                points.push(Point2::new(control.x / weight, control.y / weight));
            }
        }
        Curve2D::Conic(Conic2D::Ellipse(processor)) => {
            let matrix = processor.transform();
            for (x, y) in [(-1.0, -1.0), (-1.0, 1.0), (1.0, -1.0), (1.0, 1.0)] {
                let mapped = matrix * Vector3::new(x, y, 1.0);
                if (mapped.z - 1.0).abs() > 1.0e-9 {
                    return None;
                }
                points.push(Point2::new(mapped.x, mapped.y));
            }
        }
        Curve2D::Conic(Conic2D::Hyperbola(_) | Conic2D::Parabola(_)) => return None,
    }
    let mut result = BoundingBox::<Point2>::new();
    let mut any = false;
    for point in points {
        if !point.x.is_finite() || !point.y.is_finite() {
            return None;
        }
        result.push(point);
        any = true;
    }
    any.then_some(result)
}

/// A sound box for `surface` restricted to the `(u, v)` rectangle `domain`.
///
/// Only two classes exploit `domain`: a plane, where the map is affine so the
/// rectangle's four corners bound the image exactly, and a straight-profile
/// revolution, where the same axial-affine / radial-convex argument as
/// [`revolution_face_box`] applies to the `u`-interval, with a FULL turn
/// assumed in `v`. Every other class ignores `domain` and falls back to its
/// whole-surface bound, which is sound and merely loose.
fn surface_patch_box(
    surface: &Surface,
    domain: BoundingBox<Point2>,
) -> Option<BoundingBox<Point3>> {
    let (low, high) = (domain.min(), domain.max());
    match surface {
        Surface::Plane(plane) => {
            let mut points = Vec::with_capacity(4);
            for (u, v) in [
                (low.x, low.y),
                (low.x, high.y),
                (high.x, low.y),
                (high.x, high.y),
            ] {
                points.push(plane.origin() + plane.axis_u() * u + plane.axis_v() * v);
            }
            hull_box(&points)
        }
        Surface::RevolutionSurface(processor) => {
            let revolution = processor.entity();
            let Curve::Line(Line(front, back)) = revolution.entity_curve() else {
                return None;
            };
            similarity_scale(processor.transform())?;
            let matrix = processor.transform();
            let origin = transform_point(matrix, revolution.origin());
            let axis = transform_direction(matrix, revolution.axis())?;
            let (front, back) = (
                transform_point(matrix, *front),
                transform_point(matrix, *back),
            );
            // The profile is affine in `u`, so its ends over `[u_low, u_high]`
            // pin the axial interval, and the radius -- convex in `u` -- takes
            // its maximum at one of those same ends.
            let ends = [low.x, high.x].map(|u| front + (back - front) * u);
            let mut axial = (f64::INFINITY, f64::NEG_INFINITY);
            let mut radius = 0.0f64;
            for end in ends {
                let offset = end - origin;
                if !offset.x.is_finite() || !offset.y.is_finite() || !offset.z.is_finite() {
                    return None;
                }
                let along = offset.dot(axis);
                axial = (axial.0.min(along), axial.1.max(along));
                radius = radius.max((offset - axis * along).magnitude());
            }
            Some(revolution_region_box(origin, axis, axial, radius))
        }
        // Sound, and simply blind to the trim.
        other => certified_surface_bounding_box(other, None),
    }
}

/// The overlap of two boxes, or [`None`] when they do not overlap.
///
/// An empty overlap means the two enclosures disagree, which is a contradiction
/// about geometry we are supposed to be certifying -- so it refuses rather than
/// pick a side.
fn intersect_boxes(
    first: BoundingBox<Point3>,
    second: BoundingBox<Point3>,
) -> Option<BoundingBox<Point3>> {
    let (first_low, first_high) = (first.min(), first.max());
    let (second_low, second_high) = (second.min(), second.max());
    let low = Point3::new(
        first_low.x.max(second_low.x),
        first_low.y.max(second_low.y),
        first_low.z.max(second_low.z),
    );
    let high = Point3::new(
        first_high.x.min(second_high.x),
        first_high.y.min(second_high.y),
        first_high.z.min(second_high.z),
    );
    (low.x <= high.x && low.y <= high.y && low.z <= high.z)
        .then(|| BoundingBox::from_iter([low, high]))
}

/// Pushes the eight corners of `bbox`; their convex hull is `bbox`.
fn push_box_corners(bbox: &BoundingBox<Point3>, points: &mut Vec<Point3>) {
    let (low, high) = (bbox.min(), bbox.max());
    for x in [low.x, high.x] {
        for y in [low.y, high.y] {
            for z in [low.z, high.z] {
                points.push(Point3::new(x, y, z));
            }
        }
    }
}

/// `(x/w, y/w, z/w)`, or [`None`] when the weight is not strictly positive --
/// the precondition of the rational convex-hull property.
fn projected_control_point(control: Vector4) -> Option<Point3> {
    let weight = control.w;
    (weight > 0.0 && weight.is_finite())
        .then(|| Point3::new(control.x / weight, control.y / weight, control.z / weight))
}

/// The box of a finite point set, or [`None`] when it is empty.
fn hull_box(points: &[Point3]) -> Option<BoundingBox<Point3>> {
    hull_box_iter(points.iter().copied())
}

/// The box of a finite point stream, or [`None`] when it is empty.
fn hull_box_iter(points: impl Iterator<Item = Point3>) -> Option<BoundingBox<Point3>> {
    let mut result = BoundingBox::<Point3>::new();
    let mut any = false;
    for point in points {
        if !point.x.is_finite() || !point.y.is_finite() || !point.z.is_finite() {
            return None;
        }
        result.push(point);
        any = true;
    }
    any.then_some(result)
}

/// The uniform scale factor of `matrix`, or [`None`] when it is not a
/// similarity (the only class of transform under which "revolution about an
/// axis" and "ball of radius r" survive as such).
fn similarity_scale(matrix: &Matrix4) -> Option<f64> {
    if matrix.x.w.abs() > SIMILARITY_TOL
        || matrix.y.w.abs() > SIMILARITY_TOL
        || matrix.z.w.abs() > SIMILARITY_TOL
        || (matrix.w.w - 1.0).abs() > SIMILARITY_TOL
    {
        return None;
    }
    let columns = [
        matrix.x.truncate(),
        matrix.y.truncate(),
        matrix.z.truncate(),
    ];
    let scale = columns[0].magnitude();
    if !scale.is_finite() || scale <= 0.0 {
        return None;
    }
    let length_tol = SIMILARITY_TOL * scale;
    let dot_tol = SIMILARITY_TOL * scale * scale;
    for column in &columns[1..] {
        if (column.magnitude() - scale).abs() > length_tol {
            return None;
        }
    }
    for (first, second) in [(0, 1), (0, 2), (1, 2)] {
        if columns[first].dot(columns[second]).abs() > dot_tol {
            return None;
        }
    }
    Some(scale)
}

/// `matrix` applied to a point, affine part included.
fn transform_point(matrix: &Matrix4, point: Point3) -> Point3 {
    let homogeneous = matrix * point.to_homogeneous();
    Point3::new(homogeneous.x, homogeneous.y, homogeneous.z)
}

/// `matrix` applied to a direction, renormalised, or [`None`] when it collapses.
fn transform_direction(matrix: &Matrix4, direction: Vector3) -> Option<Vector3> {
    let mapped = (matrix * direction.extend(0.0)).truncate();
    let length = mapped.magnitude();
    (length.is_finite() && length > 0.0).then(|| mapped / length)
}

/// The box of `{ origin + t * axis + w : t in axial, w perp axis, |w| <= radius }`.
///
/// Along a world axis `e` the extreme of `dot(w, e)` over that disc is
/// `radius * sqrt(1 - dot(axis, e)^2)`, which is what the `sqrt` below is.
fn revolution_region_box(
    origin: Point3,
    axis: Vector3,
    axial: (f64, f64),
    radius: f64,
) -> BoundingBox<Point3> {
    let (mut low, mut high) = ([0.0f64; 3], [0.0f64; 3]);
    for index in 0..3 {
        let along = axis[index];
        let (first, second) = (axial.0 * along, axial.1 * along);
        let perpendicular = radius * (1.0 - along * along).max(0.0).sqrt();
        low[index] = origin[index] + first.min(second) - perpendicular;
        high[index] = origin[index] + first.max(second) + perpendicular;
    }
    BoundingBox::from_iter([
        Point3::new(low[0], low[1], low[2]),
        Point3::new(high[0], high[1], high[2]),
    ])
}

/// The box of a cylinder/cone face, from points enclosing its boundary wires.
///
/// The boundary hull's axial span contains the face's, and its maximal radius
/// is at least the face's (the face's own extremes are attained on its
/// boundary, and the boundary lies in the hull) -- see the module note.
fn revolution_face_box(
    origin: Point3,
    axis: Vector3,
    boundary_hull: &[Point3],
) -> Option<BoundingBox<Point3>> {
    let (mut low, mut high, mut radius) = (f64::INFINITY, f64::NEG_INFINITY, 0.0f64);
    for point in boundary_hull {
        let offset = point - origin;
        if !offset.x.is_finite() || !offset.y.is_finite() || !offset.z.is_finite() {
            return None;
        }
        let along = offset.dot(axis);
        low = low.min(along);
        high = high.max(along);
        radius = radius.max((offset - axis * along).magnitude());
    }
    (low <= high && radius.is_finite())
        .then(|| revolution_region_box(origin, axis, (low, high), radius))
}

#[cfg(test)]
mod tests {
    use super::*;
    use monstertruck_geometry::prelude::TryIntoHomogeneousBsplineSurface;

    /// The bound around a ball is the ball's own box, and it is looser than the
    /// ball by `6 / pi` -- 91.0%. Stated against the closed form so the number
    /// is checked, not asserted.
    #[test]
    fn an_analytic_sphere_is_bounded_by_its_own_box_and_by_6_over_pi() {
        let center = Point3::new(3.0, -4.0, 0.5);
        let radius = 12.5;
        let surface = Surface::SphericalSurface(Processor::new(Sphere::new(center, radius)));
        let bound = certified_surface_bounding_box(&surface, None)
            .expect("an analytic sphere bounds itself without a boundary hull");
        let diagonal = bound.diagonal();
        for (axis, extent) in [("x", diagonal.x), ("y", diagonal.y), ("z", diagonal.z)] {
            assert!(
                (extent - 2.0 * radius).abs() <= 1.0e-12 * radius,
                "a ball of radius {radius} must box to {} along {axis}; got {extent}",
                2.0 * radius,
            );
        }
        let box_volume = diagonal.x * diagonal.y * diagonal.z;
        let ball_volume = 4.0 / 3.0 * std::f64::consts::PI * radius.powi(3);
        let looseness = box_volume / ball_volume;
        assert!(
            (looseness - ANALYTIC_SPHERE_LOOSENESS).abs() <= 1.0e-12,
            "the certified box around a ball must exceed it by exactly 6/pi = \
             {ANALYTIC_SPHERE_LOOSENESS}; measured {looseness}",
        );
        // Every point of the sphere is inside, including the six poles that
        // carry no vertex -- the failure mode the vertex box has.
        for offset in [
            Vector3::unit_x(),
            -Vector3::unit_x(),
            Vector3::unit_y(),
            -Vector3::unit_y(),
            Vector3::unit_z(),
            -Vector3::unit_z(),
        ] {
            assert!(
                bound.contains(center + offset * radius),
                "the pole {:?} of the ball must be inside its certified box",
                center + offset * radius,
            );
        }
    }

    /// The SAME ball routed the other way -- as the rational NURBS net the
    /// repo's own `TryIntoHomogeneousBsplineSurface` builds for a `Sphere` --
    /// and its control hull measured against the closed form.
    ///
    /// This is the "how loose is a control hull, really?" question, answered on
    /// the production net rather than on a hand-built one. The measurement is
    /// asserted, not assumed: the hull must CONTAIN a dense sampling of the
    /// ball (soundness) and its looseness is pinned to what it measures.
    #[test]
    fn the_production_rational_sphere_net_bounds_the_same_ball() {
        let radius = 2.0;
        let center = Point3::new(-1.0, 0.5, 4.0);
        let sphere = Sphere::new(center, radius);
        let net = sphere
            .try_into_homogeneous_bspline_surface()
            .expect("the repo builds a rational net for a sphere");
        let surface = Surface::NurbsSurface(NurbsSurface::new(net));
        let bound = certified_surface_bounding_box(&surface, None)
            .expect("a positively-weighted rational patch bounds itself");
        // SOUNDNESS first: every sampled point of the analytic ball is inside.
        for latitude_step in 0..=32 {
            let latitude = std::f64::consts::PI * f64::from(latitude_step) / 32.0
                - std::f64::consts::FRAC_PI_2;
            for longitude_step in 0..64 {
                let longitude = std::f64::consts::TAU * f64::from(longitude_step) / 64.0;
                let point = center
                    + Vector3::new(
                        radius * latitude.cos() * longitude.cos(),
                        radius * latitude.cos() * longitude.sin(),
                        radius * latitude.sin(),
                    );
                assert!(
                    bound.contains(point),
                    "the ball point {point:?} escapes the rational net's control hull \
                     -- the convex-hull property is being misapplied",
                );
            }
        }
        let diagonal = bound.diagonal();
        let hull_volume = diagonal.x * diagonal.y * diagonal.z;
        let ball_volume = 4.0 / 3.0 * std::f64::consts::PI * radius.powi(3);
        let looseness = hull_volume / ball_volume;
        // MEASURED (2026-08-02): the production rational net's coordinate
        // extremes are exactly +-r, so its axis-aligned hull is the ball's own
        // box and the control-hull route costs NOTHING over the analytic one on
        // a sphere. Do not "improve" this by widening the band -- a drift here
        // means the net changed shape.
        assert!(
            (looseness - ANALYTIC_SPHERE_LOOSENESS).abs() <= 1.0e-9,
            "the rational net's hull box is expected to coincide with the analytic \
             box (looseness 6/pi = {ANALYTIC_SPHERE_LOOSENESS}); measured \
             {looseness} from a {} x {} x {} box",
            diagonal.x,
            diagonal.y,
            diagonal.z,
        );
    }

    /// A non-positive weight voids the rational convex-hull property, so the
    /// bound refuses rather than inventing one.
    #[test]
    fn a_non_positive_weight_refuses() {
        let net = vec![
            vec![
                Vector4::new(0.0, 0.0, 0.0, 1.0),
                Vector4::new(0.0, 1.0, 0.0, 1.0),
            ],
            vec![
                Vector4::new(1.0, 0.0, 0.0, 1.0),
                // The weight that voids the property.
                Vector4::new(1.0, 1.0, 0.0, 0.0),
            ],
        ];
        let knots = KnotVector::from(vec![0.0, 0.0, 1.0, 1.0]);
        let surface = Surface::NurbsSurface(NurbsSurface::new(BsplineSurface::new(
            (knots.clone(), knots),
            net,
        )));
        assert!(
            certified_surface_bounding_box(&surface, None).is_none(),
            "a zero weight must refuse: the convex-hull property does not hold",
        );
    }

    /// A cylinder's certified box is the tube its own boundary circles cut out
    /// -- not the infinite surface, and not the vertex hull.
    #[test]
    fn a_cylinder_face_is_bounded_by_its_boundary_wires() {
        let radius = 3.0;
        let origin = Point3::new(0.0, 0.0, 0.0);
        let axis = Vector3::unit_z();
        let profile = Line(Point3::new(radius, 0.0, 0.0), Point3::new(radius, 0.0, 1.0));
        let surface = Surface::RevolutionSurface(Processor::new(RevolutionSurface::by_revolution(
            Curve::Line(profile),
            origin,
            axis,
        )));
        // Two boundary circles at z = 0 and z = 7, given as their (sound)
        // enclosing squares -- a rational circle's control hull is exactly that.
        let hull: Vec<Point3> = [0.0, 7.0]
            .into_iter()
            .flat_map(|z| {
                [(-1.0, -1.0), (-1.0, 1.0), (1.0, -1.0), (1.0, 1.0)]
                    .into_iter()
                    .map(move |(x, y)| Point3::new(x * radius, y * radius, z))
            })
            .collect();
        let bound = certified_surface_bounding_box(&surface, Some(&hull))
            .expect("a straight-profile revolution bounds from its wires");
        let diagonal = bound.diagonal();
        // The hull's corners sit at radius*sqrt(2) from the axis, so the tube is
        // bounded at that radius -- sound, and looser than the true 3.0.
        let expected = 2.0 * radius * std::f64::consts::SQRT_2;
        assert!(
            (diagonal.x - expected).abs() <= 1.0e-12 * expected
                && (diagonal.y - expected).abs() <= 1.0e-12 * expected,
            "the tube's radial extent must be the hull's {expected}; got {} x {}",
            diagonal.x,
            diagonal.y,
        );
        assert!(
            (diagonal.z - 7.0).abs() <= 1.0e-12,
            "the tube's axial extent is exactly its wires' 7.0; got {}",
            diagonal.z,
        );
        // Every point of the cylinder wall is inside.
        for step in 0..16 {
            let angle = std::f64::consts::TAU * f64::from(step) / 16.0;
            for z in [0.0, 3.5, 7.0] {
                let point = Point3::new(radius * angle.cos(), radius * angle.sin(), z);
                assert!(
                    bound.contains(point),
                    "the wall point {point:?} must be inside the certified box",
                );
            }
        }
    }

    /// Without a boundary hull a cylinder cannot be bounded at all, and says so.
    #[test]
    fn a_cylinder_without_wires_refuses() {
        let surface = Surface::RevolutionSurface(Processor::new(RevolutionSurface::by_revolution(
            Curve::Line(Line(Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 0.0, 1.0))),
            Point3::origin(),
            Vector3::unit_z(),
        )));
        assert!(
            certified_surface_bounding_box(&surface, None).is_none(),
            "an untrimmed cylinder is unbounded; a bound must not be invented",
        );
    }

    /// A ring torus is bounded by `R + r` across and `r` along its axis.
    #[test]
    fn a_ring_torus_is_bounded_by_its_own_radii() {
        let (large, small) = (5.0, 1.5);
        let center = Point3::new(1.0, 2.0, 3.0);
        let surface = Surface::ToroidalSurface(Processor::new(Torus::new(center, large, small)));
        let bound = certified_surface_bounding_box(&surface, None).expect("a torus bounds itself");
        let diagonal = bound.diagonal();
        assert!(
            (diagonal.x - 2.0 * (large + small)).abs() <= 1.0e-12
                && (diagonal.y - 2.0 * (large + small)).abs() <= 1.0e-12,
            "a ring torus spans 2(R+r) across its axis; got {} x {}",
            diagonal.x,
            diagonal.y,
        );
        assert!(
            (diagonal.z - 2.0 * small).abs() <= 1.0e-12,
            "a ring torus spans 2r along its axis; got {}",
            diagonal.z,
        );
    }

    /// The whole point, in one assertion: on the `#25387` geometry the vertex
    /// box is SMALLER than the solid's own volume and the certified box is not.
    ///
    /// The numbers are the measured ones (spec 013): `R = 12.5`, `h = 9`,
    /// `r = 6`, closed-form volume `5273.16`, vertex box `18 x 17.349 x 12`.
    /// Only the sphere face is needed to make the point, so this stays a pure
    /// geometry test with no corpus.
    #[test]
    fn the_certified_box_contains_a_volume_the_vertex_box_does_not() {
        let (big_radius, half_x, bore_radius) = (12.5f64, 9.0f64, 6.0f64);
        let trim = (big_radius * big_radius - half_x * half_x).sqrt();
        let closed_form = 2.0
            * std::f64::consts::PI
            * (big_radius * big_radius * half_x
                - half_x.powi(3) / 3.0
                - bore_radius * bore_radius * half_x);
        let vertex_box = (2.0 * half_x) * (2.0 * trim) * (2.0 * bore_radius);
        assert!(
            closed_form > vertex_box,
            "the witness requires the correct volume {closed_form} to EXCEED the \
             vertex box {vertex_box}",
        );
        let surface =
            Surface::SphericalSurface(Processor::new(Sphere::new(Point3::origin(), big_radius)));
        let bound = certified_surface_bounding_box(&surface, None).expect("sphere bounds itself");
        let diagonal = bound.diagonal();
        let certified = diagonal.x * diagonal.y * diagonal.z;
        assert!(
            certified > closed_form,
            "the certified box {certified} must contain the solid's volume \
             {closed_form}; that is the whole defect",
        );
    }
}
