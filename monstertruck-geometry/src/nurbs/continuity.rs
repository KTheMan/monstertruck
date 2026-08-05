use super::{BsplineSurface, KnotVector, NurbsSurface};
use monstertruck_core::cgmath64::Homogeneous;
use monstertruck_traits::surface_continuity::{
    BoundarySide, ContinuityOrder, SurfaceContinuityCapability,
};

impl<P> BsplineSurface<P> {
    /// Reports whether this B-spline representation can expose the requested
    /// derivatives along a full parameter-domain side.
    ///
    /// This checks the clamped cross-boundary knot vector, degree, and control
    /// rows. It does not establish compatibility with another surface or
    /// feasibility for a numerical solver.
    ///
    /// # Examples
    ///
    /// ```
    /// use monstertruck_geometry::prelude::*;
    ///
    /// let surface = BsplineSurface::new(
    ///     (KnotVector::bezier_knot(1), KnotVector::bezier_knot(2)),
    ///     vec![vec![Point3::new(0.0, 0.0, 0.0); 3]; 2],
    /// );
    ///
    /// assert!(
    ///     surface
    ///         .continuity_capability(BoundarySide::MinV, ContinuityOrder::G2)
    ///         .is_supported()
    /// );
    /// ```
    pub fn continuity_capability(
        &self,
        side: BoundarySide,
        requested: ContinuityOrder,
    ) -> SurfaceContinuityCapability {
        let (knots, control_count) = match side {
            BoundarySide::MinU | BoundarySide::MaxU => {
                (self.knot_vector_u(), self.control_points().len())
            }
            BoundarySide::MinV | BoundarySide::MaxV => (
                self.knot_vector_v(),
                self.control_points().first().map_or(0, Vec::len),
            ),
        };

        capability_for_axis(knots, control_count, side, requested)
    }
}

impl<V> NurbsSurface<V>
where V: Copy + Homogeneous<Scalar = f64>
{
    /// Reports whether this positive-weight NURBS representation can expose
    /// the requested derivatives along a full parameter-domain side.
    ///
    /// In addition to the underlying B-spline requirements, every homogeneous
    /// control point must carry a finite positive weight. This does not
    /// establish compatibility with another surface or solver feasibility.
    ///
    /// # Examples
    ///
    /// ```
    /// use monstertruck_geometry::prelude::*;
    ///
    /// let surface = NurbsSurface::new(BsplineSurface::new(
    ///     (KnotVector::bezier_knot(1), KnotVector::bezier_knot(1)),
    ///     vec![vec![Vector4::new(0.0, 0.0, 0.0, 1.0); 2]; 2],
    /// ));
    ///
    /// assert!(
    ///     surface
    ///         .continuity_capability(BoundarySide::MaxU, ContinuityOrder::G1)
    ///         .is_supported()
    /// );
    /// ```
    pub fn continuity_capability(
        &self,
        side: BoundarySide,
        requested: ContinuityOrder,
    ) -> SurfaceContinuityCapability {
        let polynomial = self
            .non_rationalized()
            .continuity_capability(side, requested);
        let positive_weights = self
            .control_points()
            .iter()
            .flatten()
            .all(|point| point.weight().is_finite() && point.weight() > 0.0);

        if polynomial.is_supported() && positive_weights {
            SurfaceContinuityCapability::supported(side, requested)
        } else {
            SurfaceContinuityCapability::unsupported(side, requested)
        }
    }
}

fn capability_for_axis(
    knots: &KnotVector,
    control_count: usize,
    side: BoundarySide,
    requested: ContinuityOrder,
) -> SurfaceContinuityCapability {
    let supported = knots
        .len()
        .checked_sub(control_count)
        .and_then(|difference| difference.checked_sub(1))
        .is_some_and(|degree| {
            control_count > requested.as_usize()
                && degree >= requested.as_usize()
                && knots.is_clamped(degree)
        });

    if supported {
        SurfaceContinuityCapability::supported(side, requested)
    } else {
        SurfaceContinuityCapability::unsupported(side, requested)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use monstertruck_core::cgmath64::{Point3, Vector4};

    fn polynomial_surface(degree_u: usize, degree_v: usize) -> BsplineSurface<Point3> {
        let control_points = (0..=degree_u)
            .map(|u| {
                (0..=degree_v)
                    .map(|v| Point3::new(u as f64, v as f64, 0.0))
                    .collect()
            })
            .collect();

        BsplineSurface::new(
            (
                KnotVector::bezier_knot(degree_u),
                KnotVector::bezier_knot(degree_v),
            ),
            control_points,
        )
    }

    #[test]
    fn boundary_side_selects_the_cross_boundary_axis() {
        let surface = polynomial_surface(1, 3);

        assert!(
            surface
                .continuity_capability(BoundarySide::MinV, ContinuityOrder::G3)
                .is_supported()
        );
        assert!(
            !surface
                .continuity_capability(BoundarySide::MinU, ContinuityOrder::G2)
                .is_supported()
        );
    }

    #[test]
    fn unclamped_cross_boundary_axis_is_unsupported() {
        let surface = BsplineSurface::new(
            (
                KnotVector::from(vec![0.0, 1.0, 2.0, 3.0]),
                KnotVector::bezier_knot(1),
            ),
            vec![
                vec![Point3::new(0.0, 0.0, 0.0); 2],
                vec![Point3::new(1.0, 0.0, 0.0); 2],
            ],
        );

        assert!(
            !surface
                .continuity_capability(BoundarySide::MaxU, ContinuityOrder::G1)
                .is_supported()
        );
        assert!(
            surface
                .continuity_capability(BoundarySide::MaxV, ContinuityOrder::G1)
                .is_supported()
        );
    }

    #[test]
    fn nurbs_capability_requires_positive_finite_weights() {
        let positive = NurbsSurface::new(BsplineSurface::new(
            (KnotVector::bezier_knot(1), KnotVector::bezier_knot(1)),
            vec![vec![Vector4::new(0.0, 0.0, 0.0, 1.0); 2]; 2],
        ));
        let zero_weight = NurbsSurface::new(BsplineSurface::new(
            (KnotVector::bezier_knot(1), KnotVector::bezier_knot(1)),
            vec![vec![Vector4::new(0.0, 0.0, 0.0, 0.0); 2]; 2],
        ));

        assert!(
            positive
                .continuity_capability(BoundarySide::MinU, ContinuityOrder::G1)
                .is_supported()
        );
        assert!(
            !zero_weight
                .continuity_capability(BoundarySide::MinU, ContinuityOrder::G0)
                .is_supported()
        );
    }
}
