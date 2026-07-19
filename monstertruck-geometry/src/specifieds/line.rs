use super::*;
use monstertruck_core::cgmath64::control_point::ControlPoint;

impl<P: Copy> Line<P> {
    /// initialize line from vector
    #[inline]
    pub fn from_origin_direction<V>(origin: P, direction: V) -> Self
    where P: std::ops::Add<V, Output = P> {
        Self(origin, origin + direction)
    }
}

impl<P> Line<P>
where
    P: EuclideanSpace<Scalar = f64>,
    P::Diff: InnerSpace<Scalar = f64>,
{
    /// Returns the projected point to the line.
    /// # Examples
    /// ```
    /// use monstertruck_geometry::prelude::*;
    /// let line = Line(Point2::new(0.0, 0.0), Point2::new(1.0, 2.0));
    /// let pt = Point2::new(0.0, 1.0);
    /// assert_near!(line.projection(pt), Point2::new(0.4, 0.8));
    /// ```
    pub fn projection(self, point: P) -> P {
        let (u, v) = (point - self.0, self.1 - self.0);
        self.0 + v * u.dot(v) / v.dot(v)
    }
    /// Returns the distance between the line and the point `point`.
    /// # Examples
    /// ```
    /// use monstertruck_geometry::prelude::*;
    /// let line = Line(Point2::new(0.0, 0.0), Point2::new(3.0, 4.0));
    ///
    /// // The foot of the perpendicular line is on a line segment
    /// let pt = Point2::new(3.0, 0.0);
    /// assert_near!(line.distance_to_point(pt), 2.4);
    ///
    /// // The foot of the perpendicular line is not on a line segment
    /// let pt = Point2::new(0.0, -4.0);
    /// assert_near!(line.distance_to_point(pt), 2.4);
    /// ```
    pub fn distance_to_point(self, point: P) -> f64 {
        let (u, v) = (point - self.0, self.1 - self.0);
        (u - v * u.dot(v) / v.dot(v)).magnitude()
    }
    /// Returns the distance between the sengment and the point `point`.
    /// # Examples
    /// ```
    /// use monstertruck_geometry::prelude::*;
    /// let line = Line(Point2::new(0.0, 0.0), Point2::new(3.0, 4.0));
    ///
    /// // The foot of the perpendicular line is on a line segment
    /// let pt = Point2::new(3.0, 0.0);
    /// assert_near!(line.distance_to_point_as_segment(pt), 2.4);
    ///
    /// // The foot of the perpendicular line is not on a line segment
    /// let pt = Point2::new(0.0, -4.0);
    /// assert_near!(line.distance_to_point_as_segment(pt), 4.0);
    /// ```
    pub fn distance_to_point_as_segment(self, point: P) -> f64 {
        let (u, v) = (point - self.0, self.1 - self.0);
        let t = f64::clamp(u.dot(v) / v.dot(v), 0.0, 1.0);
        (u - v * t).magnitude()
    }
}

impl Line<Point2> {
    /// Returns the intersection of two lines and its parameters.
    /// # Examples
    /// ```
    /// use monstertruck_geometry::prelude::*;
    /// let line0 = Line(Point2::new(0.0, 0.0), Point2::new(9.0, 3.0));
    /// let line1 = Line(Point2::new(0.0, 6.0), Point2::new(9.0, 0.0));
    /// let (s, t, p) = line0.intersection(line1).unwrap();
    /// assert_near!(line0.subs(s), Point2::new(6.0, 2.0));
    /// assert_near!(line1.subs(t), Point2::new(6.0, 2.0));
    /// assert_near!(p, Point2::new(6.0, 2.0));
    /// ```
    /// # Failures
    /// Returns `None` if two lines are parallel.
    /// ```
    /// use monstertruck_geometry::prelude::*;
    /// let line0 = Line(Point2::new(0.0, 0.0), Point2::new(9.0, 3.0));
    /// let line1 = Line(Point2::new(-1.0, 0.0), Point2::new(8.0, 3.0));
    /// assert!(line0.intersection(line1).is_none());
    /// ```
    pub fn intersection(self, other: Line<Point2>) -> Option<(f64, f64, Point2)> {
        let mat = Matrix2::from_cols(self.1 - self.0, other.0 - other.1);
        let v = other.0 - self.0;
        let params = mat.invert().map(|inv| inv * v)?;
        Some((params.x, params.y, self.subs(params.x)))
    }
}

impl<P: ControlPoint<f64>> ParametricCurve for Line<P> {
    type Point = P;
    type Vector = P::Diff;
    #[inline]
    fn derivative_n(&self, n: usize, t: f64) -> Self::Vector {
        match n {
            0 => self.evaluate(t).to_vec(),
            1 => self.1 - self.0,
            _ => Self::Vector::zero(),
        }
    }
    #[inline]
    fn evaluate(&self, t: f64) -> Self::Point { self.0 + (self.1 - self.0) * t }
    #[inline]
    fn derivative(&self, _: f64) -> Self::Vector { self.1 - self.0 }
    #[inline]
    fn derivative_2(&self, _: f64) -> Self::Vector { Self::Vector::zero() }
    /// Return `0.0..=1.0` i.e. we regard it as a segment
    #[inline]
    fn parameter_range(&self) -> ParameterRange { (Bound::Included(0.0), Bound::Included(1.0)) }
}

impl<P: ControlPoint<f64>> BoundedCurve for Line<P> {}

impl<P: ControlPoint<f64>> Cut for Line<P> {
    #[inline]
    fn cut(&mut self, t: f64) -> Self {
        let r = self.subs(t);
        let res = Self(r, self.1);
        self.1 = r;
        res
    }
}

impl<P: ControlPoint<f64>> ParameterDivision1D for Line<P> {
    type Point = P;
    #[inline]
    fn parameter_division(&self, range: (f64, f64), _: f64) -> (Vec<f64>, Vec<P>) {
        (
            vec![range.0, range.1],
            vec![self.subs(range.0), self.subs(range.1)],
        )
    }
}

impl<P: Copy> Invertible for Line<P> {
    #[inline]
    fn invert(&mut self) { *self = Self(self.1, self.0); }
    #[inline]
    fn inverse(&self) -> Self { Self(self.1, self.0) }
}

impl<P> SearchNearestParameter<CurveParameter> for Line<P>
where
    P: ControlPoint<f64>,
    P::Diff: InnerSpace<Scalar = f64>,
{
    type Point = P;
    #[inline]
    fn search_nearest_parameter<H: Into<SearchParameterHint1D>>(
        &self,
        pt: P,
        _: H,
        _: usize,
    ) -> Option<f64> {
        let b = self.1 - self.0;
        Some((pt - self.0).dot(b) / b.dot(b))
    }
}

impl<P> SearchParameter<CurveParameter> for Line<P>
where
    P: ControlPoint<f64> + Tolerance,
    P::Diff: InnerSpace<Scalar = f64>,
{
    type Point = P;
    #[inline]
    fn search_parameter<H: Into<SearchParameterHint1D>>(
        &self,
        pt: P,
        _: H,
        _: usize,
    ) -> Option<f64> {
        let b = self.1 - self.0;
        let t = (pt - self.0).dot(b) / b.dot(b);
        match self.subs(t).near(&pt) {
            true => Some(t),
            false => None,
        }
    }
}

impl<P: EuclideanSpace, M: Transform<P>> Transformed<M> for Line<P> {
    #[inline]
    fn transform_by(&mut self, trans: M) {
        self.0 = trans.transform_point(self.0);
        self.1 = trans.transform_point(self.1);
    }
    #[inline]
    fn transformed(&self, trans: M) -> Self {
        Line(trans.transform_point(self.0), trans.transform_point(self.1))
    }
}

impl<P> From<Line<P>> for BsplineCurve<P> {
    fn from(Line(p, q): Line<P>) -> Self {
        BsplineCurve::new_unchecked(KnotVector::bezier_knot(1), vec![p, q])
    }
}

impl<P: Copy> ToSameGeometry<BsplineCurve<P>> for Line<P> {
    fn to_same_geometry(&self) -> BsplineCurve<P> { BsplineCurve::from(*self) }
}

// -- v2 scalar-generic impls ------------------------------------------------

use monstertruck_core::scalar::{HasScalar, ToleranceScalar, ToleranceV2};
use monstertruck_traits::v2;

impl<P> v2::ParametricCurve for Line<P>
where P: HasScalar + ControlPoint<<P as HasScalar>::Scalar>
{
    type Scalar = <P as HasScalar>::Scalar;
    type Point = P;
    type Vector = P::Diff;

    #[inline]
    fn evaluate(&self, t: Self::Scalar) -> P { self.0 + (self.1 - self.0) * t }
    #[inline]
    fn derivative(&self, _: Self::Scalar) -> P::Diff { self.1 - self.0 }
    #[inline]
    fn derivative_2(&self, _: Self::Scalar) -> P::Diff { P::Diff::zero() }
    #[inline]
    fn derivative_n(&self, n: usize, t: Self::Scalar) -> P::Diff {
        match n {
            0 => v2::ParametricCurve::evaluate(self, t).to_vec(),
            1 => self.1 - self.0,
            _ => P::Diff::zero(),
        }
    }
    #[inline]
    fn period(&self) -> Option<Self::Scalar> { None }
    #[inline]
    fn try_range_tuple(&self) -> Option<(Self::Scalar, Self::Scalar)> {
        Some((Self::Scalar::zero(), Self::Scalar::one()))
    }
}

impl<P> v2::BoundedCurve for Line<P>
where P: HasScalar + ControlPoint<<P as HasScalar>::Scalar>
{
    #[inline]
    fn range_tuple(&self) -> (Self::Scalar, Self::Scalar) {
        (Self::Scalar::zero(), Self::Scalar::one())
    }
}

impl<P> v2::Cut for Line<P>
where P: HasScalar + ControlPoint<<P as HasScalar>::Scalar>
{
    #[inline]
    fn cut(&mut self, t: Self::Scalar) -> Self {
        let r = v2::ParametricCurve::evaluate(self, t);
        let res = Self(r, self.1);
        self.1 = r;
        res
    }
}

impl<P> v2::SearchNearestParameter<v2::D1<<P as HasScalar>::Scalar>> for Line<P>
where
    P: HasScalar + ControlPoint<<P as HasScalar>::Scalar>,
    P::Diff: InnerSpace<Scalar = <P as HasScalar>::Scalar>,
{
    type Point = P;

    #[inline]
    fn search_nearest_parameter<H: Into<v2::SearchParameterHint1D<<P as HasScalar>::Scalar>>>(
        &self,
        pt: P,
        _: H,
        _: usize,
    ) -> Option<<P as HasScalar>::Scalar> {
        let b = self.1 - self.0;
        Some((pt - self.0).dot(b) / b.dot(b))
    }
}

impl<P> v2::SearchParameter<v2::D1<<P as HasScalar>::Scalar>> for Line<P>
where
    P: HasScalar
        + ControlPoint<<P as HasScalar>::Scalar>
        + ToleranceV2<Epsilon = <P as HasScalar>::Scalar>,
    P::Diff: InnerSpace<Scalar = <P as HasScalar>::Scalar>,
    <P as HasScalar>::Scalar: ToleranceScalar,
{
    type Point = P;

    #[inline]
    fn search_parameter<H: Into<v2::SearchParameterHint1D<<P as HasScalar>::Scalar>>>(
        &self,
        pt: P,
        _: H,
        _: usize,
    ) -> Option<<P as HasScalar>::Scalar> {
        let b = self.1 - self.0;
        let t = (pt - self.0).dot(b) / b.dot(b);
        v2::ParametricCurve::evaluate(self, t)
            .near_v2(&pt)
            .then_some(t)
    }
}

#[test]
fn line() {
    let line = Line(Point2::new(1.0, 0.0), Point2::new(0.0, 1.0));

    // subs
    assert_near!(line.subs(0.4), Point2::new(0.6, 0.4));

    // inverse
    let line_inverse = line.inverse();
    assert_eq!(line.0, line_inverse.1);
    assert_eq!(line.1, line_inverse.0);

    // cut
    let mut line0 = line;
    let line1 = line0.cut(0.4);
    assert_eq!(line.0, line0.0);
    assert_near!(line0.1, line.subs(0.4));
    assert_eq!(line0.1, line1.0);
    assert_eq!(line1.1, line.1);

    // SNP
    assert_near!(
        line.search_nearest_parameter(Point2::new(1.0, 1.0), None, 0)
            .unwrap(),
        0.5
    );
    assert!(
        line.search_parameter(Point2::new(1.0, 1.0), None, 0)
            .is_none()
    );
}

// -- Phase 4.0: scalar-generic validation (f32) ----------------------------

#[cfg(test)]
mod scalar_generic_tests {
    use super::*;
    use monstertruck_traits::v2;

    type Point2F32 = cgmath::Point2<f32>;
    type Point3F32 = cgmath::Point3<f32>;
    type Vector3F32 = cgmath::Vector3<f32>;

    // -- Compile-time trait satisfaction ------------------------------------

    const fn _assert_v2_curve<C: v2::ParametricCurve>() {}
    const fn _assert_v2_bounded<C: v2::BoundedCurve>() {}
    const fn _assert_v2_cut<C: v2::Cut>() {}
    const fn _assert_v2_search<C: v2::SearchParameter<v2::D1<f32>>>() {}
    const fn _assert_v2_nearest<C: v2::SearchNearestParameter<v2::D1<f32>>>() {}

    #[allow(dead_code)]
    const _: () = {
        _assert_v2_curve::<Line<Point2F32>>();
        _assert_v2_curve::<Line<Point3F32>>();
        _assert_v2_bounded::<Line<Point2F32>>();
        _assert_v2_bounded::<Line<Point3F32>>();
        _assert_v2_cut::<Line<Point2F32>>();
        _assert_v2_cut::<Line<Point3F32>>();
        _assert_v2_search::<Line<Point2F32>>();
        _assert_v2_search::<Line<Point3F32>>();
        _assert_v2_nearest::<Line<Point2F32>>();
        _assert_v2_nearest::<Line<Point3F32>>();
    };

    // -- Runtime correctness (f32) -----------------------------------------

    #[test]
    fn f32_evaluate_and_derivative() {
        let line: Line<Point3F32> =
            Line(Point3F32::new(1.0, 0.0, 0.0), Point3F32::new(0.0, 1.0, 0.0));

        let mid = v2::ParametricCurve::evaluate(&line, 0.5f32);
        assert!((mid.x - 0.5).abs() < 1e-6);
        assert!((mid.y - 0.5).abs() < 1e-6);
        assert!((mid.z - 0.0).abs() < 1e-6);

        let tangent: Vector3F32 = v2::ParametricCurve::derivative(&line, 0.5f32);
        assert!((tangent.x - (-1.0)).abs() < 1e-6);
        assert!((tangent.y - 1.0).abs() < 1e-6);
        assert!((tangent.z - 0.0).abs() < 1e-6);

        let accel: Vector3F32 = v2::ParametricCurve::derivative_2(&line, 0.5f32);
        assert!((accel.x).abs() < 1e-6);
        assert!((accel.y).abs() < 1e-6);
        assert!((accel.z).abs() < 1e-6);
    }

    #[test]
    fn f32_bounded_curve() {
        let line: Line<Point3F32> =
            Line(Point3F32::new(0.0, 0.0, 0.0), Point3F32::new(1.0, 1.0, 1.0));
        let (t0, t1) = v2::BoundedCurve::range_tuple(&line);
        assert_eq!(t0, 0.0f32);
        assert_eq!(t1, 1.0f32);
    }

    #[test]
    fn f32_cut() {
        let line: Line<Point3F32> =
            Line(Point3F32::new(1.0, 0.0, 0.0), Point3F32::new(0.0, 1.0, 0.0));
        let mut left = line;
        let right = v2::Cut::cut(&mut left, 0.4f32);

        // left covers [0, 0.4]
        assert_eq!(left.0, line.0);
        let expected_mid = v2::ParametricCurve::evaluate(&line, 0.4f32);
        assert!((left.1.x - expected_mid.x).abs() < 1e-6);
        assert!((left.1.y - expected_mid.y).abs() < 1e-6);

        // right covers [0.4, 1]
        assert!((right.0.x - expected_mid.x).abs() < 1e-6);
        assert_eq!(right.1, line.1);
    }

    #[test]
    fn f32_presearch() {
        let line: Line<Point3F32> =
            Line(Point3F32::new(0.0, 0.0, 0.0), Point3F32::new(1.0, 0.0, 0.0));
        let query = Point3F32::new(0.7, 0.0, 0.0);
        let t = v2::algo::curve::presearch(&line, query, (0.0f32, 1.0f32), 100);
        assert!((t - 0.7).abs() < 0.02); // within one division step
    }

    #[test]
    fn f32_search_parameter_and_nearest() {
        let line: Line<Point3F32> =
            Line(Point3F32::new(0.0, 0.0, 0.0), Point3F32::new(1.0, 0.0, 0.0));
        let on_curve = Point3F32::new(0.25, 0.0, 0.0);
        let off_curve = Point3F32::new(0.25, 1.0, 0.0);

        let found = v2::SearchParameter::<v2::D1<f32>>::search_parameter(
            &line,
            on_curve,
            v2::SearchParameterHint1D::None,
            0,
        )
        .unwrap();
        assert!((found - 0.25).abs() < 1e-6);

        assert!(
            v2::SearchParameter::<v2::D1<f32>>::search_parameter(
                &line,
                off_curve,
                v2::SearchParameterHint1D::None,
                0,
            )
            .is_none()
        );

        let nearest = v2::SearchNearestParameter::<v2::D1<f32>>::search_nearest_parameter(
            &line,
            off_curve,
            v2::SearchParameterHint1D::None,
            0,
        )
        .unwrap();
        assert!((nearest - 0.25).abs() < 1e-6);
    }

    // -- f32 vs f64 equivalence --------------------------------------------

    #[test]
    fn f32_f64_equivalence() {
        let line_f32: Line<Point3F32> =
            Line(Point3F32::new(1.0, 2.0, 3.0), Point3F32::new(4.0, 5.0, 6.0));
        let line_f64: Line<Point3> = Line(Point3::new(1.0, 2.0, 3.0), Point3::new(4.0, 5.0, 6.0));

        for &t in &[0.0, 0.25, 0.5, 0.75, 1.0] {
            let p32 = v2::ParametricCurve::evaluate(&line_f32, t as f32);
            let p64 = v2::ParametricCurve::evaluate(&line_f64, t);
            assert!((p32.x as f64 - p64.x).abs() < 1e-6);
            assert!((p32.y as f64 - p64.y).abs() < 1e-6);
            assert!((p32.z as f64 - p64.z).abs() < 1e-6);
        }
    }
}
