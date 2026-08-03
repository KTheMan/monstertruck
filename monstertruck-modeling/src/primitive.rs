use crate::builder;
use monstertruck_geometry::prelude::*;
use monstertruck_topology::*;
use std::f64::consts::PI;

/// rectangle
/// # Example
/// ```
/// use monstertruck_modeling::*;
///
/// let r#box = BoundingBox::from_iter([
///     Point2::new(-1.0, -2.0),
///     Point2::new(2.0, 1.0),
/// ]);
/// let plane = Plane::zx();
/// let rect: Wire = primitive::rect(r#box, plane);
///
/// assert_eq!(rect[0].front().point(), Point3::new(-2.0, 0.0, -1.0));
/// assert_eq!(rect[1].front().point(), Point3::new(-2.0, 0.0, 2.0));
/// assert_eq!(rect[2].front().point(), Point3::new(1.0, 0.0, 2.0));
/// assert_eq!(rect[3].front().point(), Point3::new(1.0, 0.0, -1.0));
/// ```
/// # Remarks
/// Since it is a rectangle in a coordinate system on plane,
/// if the coordinate system is tilted, a parallelogram is drawn.
/// ```
/// use monstertruck_modeling::*;
///
/// let r#box = BoundingBox::from_iter([
///     Point2::new(-1.0, -2.0),
///     Point2::new(2.0, 1.0),
/// ]);
/// let plane = Plane::new(
///     Point3::origin(),
///     Point3::new(1.0, 0.0, 0.0),
///     Point3::new(1.0, 1.0, 0.0),
/// );
/// let rect: Wire = primitive::rect(r#box, plane);
///
/// assert_eq!(rect[0].front().point(), Point3::new(-3.0, -2.0, 0.0));
/// assert_eq!(rect[1].front().point(), Point3::new(0.0, -2.0, 0.0));
/// assert_eq!(rect[2].front().point(), Point3::new(3.0, 1.0, 0.0));
/// assert_eq!(rect[3].front().point(), Point3::new(0.0, 1.0, 0.0));
/// ```
pub fn rect<C>(r#box: BoundingBox<Point2>, plane: Plane) -> Wire<Point3, C>
where Line<Point3>: ToSameGeometry<C> {
    let (min, max) = (r#box.min(), r#box.max());
    let v = builder::vertices([
        plane.subs(min.x, min.y),
        plane.subs(max.x, min.y),
        plane.subs(max.x, max.y),
        plane.subs(min.x, max.y),
    ]);
    wire![
        builder::line(&v[0], &v[1]),
        builder::line(&v[1], &v[2]),
        builder::line(&v[2], &v[3]),
        builder::line(&v[3], &v[0]),
    ]
}

/// circle, specified by the start point and the rotation axis.
/// # Example
/// ```
/// use monstertruck_modeling::*;
///
/// let origin = Point3::new(1.0, -2.0, 3.0);
/// let axis = Vector3::new(0.0, 1.0, 0.0);
/// let start = origin + Vector3::new(3.0, 0.0, 4.0);
///
/// let wire: Wire = primitive::circle(start, origin, axis, 2);
///
/// assert_eq!(wire.len(), 2);
/// for edge in wire {
///     let arc = edge.oriented_curve();
///     let (t0, t1) = arc.range_tuple();
///     for i in 0..=10 {
///         let u = i as f64 / 10.0;
///         let t = (1.0 - u) * t0 + u * t1;
///         let p = arc.subs(t);
///         let der = arc.der(t);
///         assert_near!(p.distance(origin), 5.0);
///         assert!(der.dot(axis).so_small());
///         assert!((p - origin).cross(der).dot(axis) > 0.0);
///     }
/// }
/// ```
pub fn circle<C>(start: Point3, origin: Point3, axis: Vector3, division: usize) -> Wire<Point3, C>
where Processor<TrimmedCurve<UnitCircle<Point3>>, Matrix4>: ToSameGeometry<C> {
    let origin = origin + (start - origin).dot(axis) * axis;
    let radius = start - origin;
    let y = axis.cross(radius);
    let mat = Matrix4::from_cols(
        radius.extend(0.0),
        y.extend(0.0),
        axis.extend(0.0),
        origin.to_homogeneous(),
    );

    let make_vertices = move |i: usize| {
        let t = 2.0 * PI * i as f64 / division as f64;
        let p = Point3::new(f64::cos(t), f64::sin(t), 0.0);
        Vertex::new(mat.transform_point(p))
    };
    let v = (0..division).map(make_vertices).collect::<Vec<_>>();

    let make_edges = move |i: usize| {
        let t0 = 2.0 * PI * i as f64 / division as f64;
        let t1 = 2.0 * PI * (i + 1) as f64 / division as f64;
        let unit_circle = UnitCircle::new();
        let trimmed = TrimmedCurve::new(unit_circle, (t0, t1));
        let mut arc = Processor::new(trimmed);
        arc.transform_by(mat);
        Edge::new(&v[i], &v[(i + 1) % division], arc.to_same_geometry())
    };
    (0..division).map(make_edges).collect()
}

/// cuboid, defined by bounding box
/// # Example
/// ```
/// use monstertruck_modeling::*;
/// let p = Point3::new(-1.0, 2.0, -3.0);
/// let q = Point3::new(10.0, -5.0, 4.0);
///
/// let bbd = BoundingBox::from_iter([p, q]);
/// let solid: Solid = primitive::cuboid(bbd);
///
/// for v in solid.vertex_iter() {
///     let x = v.point();
///     assert!(x.x.near(&p.x) || x.x.near(&q.x));
///     assert!(x.y.near(&p.y) || x.y.near(&q.y));
///     assert!(x.z.near(&p.z) || x.z.near(&q.z));
/// }
/// ```
pub fn cuboid<C, S>(r#box: BoundingBox<Point3>) -> Solid<Point3, C, S>
where
    Line<Point3>: ToSameGeometry<C>,
    Plane: ToSameGeometry<S>, {
    let (p, q) = (r#box.min(), r#box.max());
    let v = builder::vertices([
        (p.x, p.y, p.z),
        (q.x, p.y, p.z),
        (q.x, q.y, p.z),
        (p.x, q.y, p.z),
        (p.x, p.y, q.z),
        (q.x, p.y, q.z),
        (q.x, q.y, q.z),
        (p.x, q.y, q.z),
    ]);
    let e = [
        builder::line(&v[0], &v[1]),
        builder::line(&v[1], &v[2]),
        builder::line(&v[2], &v[3]),
        builder::line(&v[3], &v[0]),
        builder::line(&v[0], &v[4]),
        builder::line(&v[1], &v[5]),
        builder::line(&v[2], &v[6]),
        builder::line(&v[3], &v[7]),
        builder::line(&v[4], &v[5]),
        builder::line(&v[5], &v[6]),
        builder::line(&v[6], &v[7]),
        builder::line(&v[7], &v[4]),
    ];

    let wire0 = wire![
        e[3].inverse(),
        e[2].inverse(),
        e[1].inverse(),
        e[0].inverse(),
    ];
    let plane0 = Plane::new(v[0].point(), v[3].point(), v[1].point());
    let mut shell = shell![Face::new_unchecked(vec![wire0], plane0.to_same_geometry())];

    (0..4).for_each(|i| {
        let wirei = wire![
            e[i].clone(),
            e[(i + 1) % 4 + 4].clone(),
            e[i + 8].inverse(),
            e[i + 4].inverse(),
        ];
        let planei = Plane::new(v[i].point(), v[i + 1].point(), v[i + 4].point());
        shell.push(Face::new_unchecked(vec![wirei], planei.to_same_geometry()));
    });

    let wire5 = wire![e[8].clone(), e[9].clone(), e[10].clone(), e[11].clone(),];
    let plane5 = Plane::new(v[4].point(), v[5].point(), v[7].point());
    shell.push(Face::new_unchecked(vec![wire5], plane5.to_same_geometry()));

    Solid::new(vec![shell])
}

#[cfg(test)]
mod tests {
    use crate::*;

    /// RED witness (010 stage P-D'): every face of a cuboid SHOULD carry the
    /// canonical rectangular frame -- in the face's own surface parameters its
    /// trim loop is the unit square, so the loop's parameter bounds are
    /// `((0, 1), (0, 1))`.
    ///
    /// Today the side-face plane frame takes `v[i + 1]` unreduced, so at `i == 3`
    /// the u-axis is a face DIAGONAL rather than an edge. That face's trim loop
    /// is the sheared parallelogram `(0,0) (1,-1) (1,0) (0,1)`, whose bounds are
    /// `((0, 1), (-1, 1))` -- a rectangle of twice the face's area. The
    /// exact-clip domain the boolean kernel derives for a face is exactly these
    /// bounds (`trimmed_face_param_range_from_loops_exact`), so the shear makes
    /// the domain rectangle disagree with the true trim window.
    ///
    /// The one-line repair (`v[(i + 1) % 4]`) turns this green, but it is NOT
    /// yet landable: the loops_store seam cut on that face currently depends on
    /// the enlarged rectangle. With the canonical frame, the finding-006 guard
    /// row `SW-B3-PLANE-SPHERE-DIFFERENCE-Iab-T00-S1-D07-Ga` oversplits face 4
    /// (3 loops `[4,9,9]` -> 4 loops `[4,4,10,9]`, an extra kept `And`
    /// fragment) and regresses from `pi/12` to `3/2 * pi/12` -- a SILENT-WRONG
    /// solid, plus a 1 s -> 14 s cost. Un-ignore together with the loops_store
    /// fix that makes the seam cut independent of the domain rectangle.
    #[test]
    #[ignore = "RED witness: the canonical-frame repair regresses the 006 guard \
                row SW-B3-PLANE-SPHERE-DIFFERENCE-Iab-T00-S1-D07-Ga to a \
                silent-wrong 3/2 * pi/12; needs the loops_store seam-cut fix first"]
    fn cuboid_faces_carry_the_canonical_unit_square_frame() {
        // Deliberately non-cubic and off-origin, with the corners given in an
        // order the bounding box must normalise.
        let p = Point3::new(-1.0, 2.0, -3.0);
        let q = Point3::new(10.0, -5.0, 4.0);
        let solid: Solid = primitive::cuboid(BoundingBox::from_iter([p, q]));
        let shell = &solid.boundaries()[0];
        assert_eq!(shell.len(), 6, "a cuboid has six faces.");

        shell.iter().enumerate().for_each(|(index, face)| {
            let surface = face.surface();
            let ((u_min, u_max), (v_min, v_max)) = face
                .boundaries()
                .iter()
                .flatten()
                .map(|edge| {
                    surface
                        .search_parameter(edge.front().point(), None, 100)
                        .unwrap_or_else(|| {
                            panic!("face {index}: a boundary vertex is not on its own surface")
                        })
                })
                .fold(
                    (
                        (f64::INFINITY, f64::NEG_INFINITY),
                        (f64::INFINITY, f64::NEG_INFINITY),
                    ),
                    |((u_min, u_max), (v_min, v_max)), (u, v)| {
                        ((u_min.min(u), u_max.max(u)), (v_min.min(v), v_max.max(v)))
                    },
                );
            assert_near!(u_min, 0.0, "face {index} u_min");
            assert_near!(u_max, 1.0, "face {index} u_max");
            assert_near!(v_min, 0.0, "face {index} v_min");
            assert_near!(v_max, 1.0, "face {index} v_max");
        });
    }
}
