//! Allocation-light derivative jets for parametric curves and surfaces.
//!
//! The storage is triangular for surfaces and stack-backed through G4. The
//! representation is order-generic within the validated kernel limit, so
//! continuity and solver code does not need G3-specific derivative structs.

use super::continuity::ContinuityOrder;
use monstertruck_traits::{ParametricCurve, ParametricSurface};
use smallvec::SmallVec;

const INLINE_JET_TERMS: usize = 5;

/// Curve derivatives from order zero through a requested order.
#[derive(Clone, Debug, PartialEq)]
pub struct CurveJet<V> {
    order: ContinuityOrder,
    derivatives: SmallVec<[V; INLINE_JET_TERMS]>,
}

impl<V> CurveJet<V> {
    /// Evaluates a curve jet at `parameter`.
    pub fn from_curve<C>(curve: &C, order: ContinuityOrder, parameter: f64) -> Self
    where
        C: ParametricCurve<Vector = V>,
    {
        Self {
            order,
            derivatives: (0..=order.as_usize())
                .map(|derivative_order| curve.derivative_n(derivative_order, parameter))
                .collect(),
        }
    }

    /// Returns the highest derivative order stored in the jet.
    #[inline(always)]
    pub const fn order(&self) -> ContinuityOrder {
        self.order
    }

    /// Returns a derivative by order.
    #[inline(always)]
    pub fn derivative(&self, order: usize) -> Option<&V> {
        self.derivatives.get(order)
    }

    /// Returns all derivatives in ascending order.
    #[inline(always)]
    pub fn derivatives(&self) -> &[V] {
        &self.derivatives
    }

    /// Consumes the jet and returns its derivatives in ascending order.
    #[inline(always)]
    pub fn into_derivatives(self) -> SmallVec<[V; INLINE_JET_TERMS]> {
        self.derivatives
    }
}

type SurfaceJetRow<V> = SmallVec<[V; INLINE_JET_TERMS]>;

/// Triangular mixed-derivative jet of a parametric surface.
///
/// A jet of order `k` stores every derivative `(m, n)` for which
/// `m + n <= k`, including `(0, 0)`.
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceJet<V> {
    order: ContinuityOrder,
    derivatives: SmallVec<[SurfaceJetRow<V>; INLINE_JET_TERMS]>,
}

impl<V> SurfaceJet<V> {
    /// Evaluates a surface jet at `(u, v)`.
    pub fn from_surface<S>(surface: &S, order: ContinuityOrder, u: f64, v: f64) -> Self
    where
        S: ParametricSurface<Vector = V>,
    {
        Self {
            order,
            derivatives: (0..=order.as_usize())
                .map(|m| {
                    (0..=order.as_usize() - m)
                        .map(|n| surface.derivative_mn(m, n, u, v))
                        .collect()
                })
                .collect(),
        }
    }

    /// Returns the highest total derivative order stored in the jet.
    #[inline(always)]
    pub const fn order(&self) -> ContinuityOrder {
        self.order
    }

    /// Returns the mixed derivative `(m, n)`.
    ///
    /// Returns `None` when `m + n` exceeds the stored order.
    #[inline(always)]
    pub fn derivative(&self, m: usize, n: usize) -> Option<&V> {
        self.derivatives.get(m).and_then(|row| row.get(n))
    }

    /// Returns the triangular derivative rows.
    #[inline(always)]
    pub fn rows(&self) -> impl ExactSizeIterator<Item = &[V]> {
        self.derivatives.iter().map(SmallVec::as_slice)
    }

    /// Returns the number of derivatives stored in the triangular jet.
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.derivatives.iter().map(SmallVec::len).sum()
    }

    /// Returns whether the jet stores no derivatives.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.derivatives.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::{InnerSpace, Vector3, Vector4};
    use crate::nurbs::{BsplineCurve, BsplineSurface, KnotVector, NurbsCurve, NurbsSurface};

    fn rational_curve(degree: usize) -> NurbsCurve<Vector4> {
        let points = (0..=degree)
            .map(|i| {
                let weight = 1.0 + i as f64 * 0.1;
                Vector4::new(
                    i as f64 * weight,
                    (i * i) as f64 * weight,
                    (i % 2) as f64 * weight,
                    weight,
                )
            })
            .collect();
        NurbsCurve::new(BsplineCurve::new(KnotVector::bezier_knot(degree), points))
    }

    fn rational_surface(udegree: usize, vdegree: usize) -> NurbsSurface<Vector4> {
        let points = (0..=udegree)
            .map(|u| {
                (0..=vdegree)
                    .map(|v| {
                        let weight = 1.0 + (u + v) as f64 * 0.05;
                        Vector4::new(
                            u as f64 * weight,
                            v as f64 * weight,
                            (u * v) as f64 * weight,
                            weight,
                        )
                    })
                    .collect()
            })
            .collect();
        NurbsSurface::new(BsplineSurface::new(
            (
                KnotVector::bezier_knot(udegree),
                KnotVector::bezier_knot(vdegree),
            ),
            points,
        ))
    }

    #[test]
    fn curve_jet_captures_every_derivative_through_g4() {
        let curve = rational_curve(6);
        let jet: CurveJet<Vector3> = CurveJet::from_curve(&curve, ContinuityOrder::G4, 0.37);
        assert_eq!(jet.derivatives().len(), 5);
        (0..=4).for_each(|order| {
            let expected = curve.derivative_n(order, 0.37);
            let actual = jet
                .derivative(order)
                .expect("requested derivative must exist");
            assert!((*actual - expected).magnitude() < 1.0e-12);
        });
    }

    #[test]
    fn surface_jet_uses_triangular_storage_through_g4() {
        let surface = rational_surface(6, 5);
        let jet: SurfaceJet<Vector3> =
            SurfaceJet::from_surface(&surface, ContinuityOrder::G4, 0.31, 0.63);
        assert_eq!(jet.len(), 15);
        (0..=4).for_each(|m| {
            (0..=4 - m).for_each(|n| {
                let expected = surface.derivative_mn(m, n, 0.31, 0.63);
                let actual = jet
                    .derivative(m, n)
                    .expect("requested derivative must exist");
                assert!((*actual - expected).magnitude() < 1.0e-12);
            });
        });
        assert!(jet.derivative(3, 2).is_none());
    }

    #[test]
    fn rational_curve_elevation_to_degree_six_preserves_g4_jet() {
        let mut elevated = rational_curve(5);
        let original = elevated.clone();
        elevated.elevate_degree_to(6);
        assert_eq!(elevated.degree(), 6);
        [0.17, 0.43, 0.81].into_iter().for_each(|parameter| {
            let original_jet: CurveJet<Vector3> =
                CurveJet::from_curve(&original, ContinuityOrder::G4, parameter);
            let elevated_jet: CurveJet<Vector3> =
                CurveJet::from_curve(&elevated, ContinuityOrder::G4, parameter);
            original_jet
                .derivatives()
                .iter()
                .zip(elevated_jet.derivatives())
                .for_each(|(expected, actual)| {
                    let error = (*actual - *expected).magnitude();
                    assert!(error <= 1.0e-8 * (1.0 + expected.magnitude()));
                });
        });
    }

    #[test]
    fn rational_surface_elevation_to_degree_six_preserves_g4_jet() {
        let mut elevated = rational_surface(5, 5);
        let original = elevated.clone();
        elevated.elevate_degrees_to((6, 6));
        assert_eq!(elevated.degrees(), (6, 6));
        [(0.19, 0.27), (0.47, 0.61), (0.83, 0.72)]
            .into_iter()
            .for_each(|(u, v)| {
                let original_jet: SurfaceJet<Vector3> =
                    SurfaceJet::from_surface(&original, ContinuityOrder::G4, u, v);
                let elevated_jet: SurfaceJet<Vector3> =
                    SurfaceJet::from_surface(&elevated, ContinuityOrder::G4, u, v);
                original_jet
                    .rows()
                    .flatten()
                    .zip(elevated_jet.rows().flatten())
                    .for_each(|(expected, actual)| {
                        let error = (*actual - *expected).magnitude();
                        assert!(error <= 1.0e-7 * (1.0 + expected.magnitude()));
                    });
            });
    }
}
