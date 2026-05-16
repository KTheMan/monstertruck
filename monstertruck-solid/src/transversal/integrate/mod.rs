use crate::alternative::Alternative;

use super::*;
use monstertruck_geometry::prelude::*;
use monstertruck_meshing::prelude::*;
use monstertruck_topology::{errors::Error as TopologyError, *};
use thiserror::Error;

/// Only solids consisting of faces whose surface is implemented this trait can be used for set operations.
pub trait ShapeOpsSurface:
    ParametricSurface3D
    + ParameterDivision2D
    + SearchParameter<SurfaceParameter, Point = Point3>
    + SearchNearestParameter<SurfaceParameter, Point = Point3>
    + Invertible
    + Send
    + Sync {
}
impl<S> ShapeOpsSurface for S where S: ParametricSurface3D
        + ParameterDivision2D
        + SearchParameter<SurfaceParameter, Point = Point3>
        + SearchNearestParameter<SurfaceParameter, Point = Point3>
        + Invertible
        + Send
        + Sync
{
}

/// Only solids consisting of edges whose curve is implemented this trait can be used for set operations.
pub trait ShapeOpsCurve<S: ShapeOpsSurface>:
    ParametricCurve3D
    + ParameterDivision1D<Point = Point3>
    + Cut
    + Invertible
    + From<IntersectionCurve<BsplineCurve<Point3>, S, S>>
    + SearchParameter<CurveParameter, Point = Point3>
    + SearchNearestParameter<CurveParameter, Point = Point3>
    + Send
    + Sync {
}
impl<C, S: ShapeOpsSurface> ShapeOpsCurve<S> for C where C: ParametricCurve3D
        + ParameterDivision1D<Point = Point3>
        + Cut
        + Invertible
        + From<IntersectionCurve<BsplineCurve<Point3>, S, S>>
        + SearchParameter<CurveParameter, Point = Point3>
        + SearchNearestParameter<CurveParameter, Point = Point3>
        + Send
        + Sync
{
}

/// Errors for boolean shape operations.
///
/// The internal algorithm is the upstream-derived passthrough that
/// returns `None` for many distinct failure modes (intersection-curve
/// build failure, face division failure, alt-shell back-conversion
/// failure, output shell that fails manifold checks). The current
/// `From<Option>` collapses all of those into [`ShapeOpsError::Internal`];
/// a future rework should split them back out.
#[derive(Debug, Error)]
pub enum ShapeOpsError {
    /// `tol` was not positive enough for robust meshing and projection.
    #[error("`tol` must be at least `TOLERANCE`.")]
    InvalidTolerance,
    /// The internal pipeline failed without further differentiation.
    #[error("internal boolean-op failure during `{operation}`.")]
    Internal {
        /// `and`, `or`, `difference`, or `symmetric_difference`.
        operation: &'static str,
    },
    /// The generated shell is topologically invalid.
    #[error("invalid output shell for `{operation}`: {source}.")]
    InvalidOutputShell {
        /// Boolean operation name.
        operation: &'static str,
        /// Topology validation error.
        #[source]
        source: TopologyError,
    },
}

/// Result alias used by the public boolean shape operations.
pub type ShapeOpsResult<T> = std::result::Result<T, ShapeOpsError>;

type AltCurveShell<C, S> =
    Shell<Point3, Alternative<C, IntersectionCurve<PolylineCurve<Point3>, S, S>>, S>;

fn altshell_to_shell<C: ShapeOpsCurve<S>, S: ShapeOpsSurface>(
    altshell: &AltCurveShell<C, S>,
    tol: f64,
) -> Option<Shell<Point3, C, S>> {
    altshell.try_mapped(
        |p| Some(*p),
        |c| match c {
            Alternative::FirstType(c) => Some(c.clone()),
            Alternative::SecondType(ic) => {
                let bsp = BsplineCurve::quadratic_approximation(ic, ic.range_tuple(), tol, 100)?;
                Some(
                    IntersectionCurve::new(ic.surface0().clone(), ic.surface1().clone(), bsp)
                        .into(),
                )
            }
        },
        |s| Some(s.clone()),
    )
}

fn process_one_pair_of_shells<C: ShapeOpsCurve<S>, S: ShapeOpsSurface>(
    shell0: &Shell<Point3, C, S>,
    shell1: &Shell<Point3, C, S>,
    tol: f64,
) -> Option<[Shell<Point3, C, S>; 2]> {
    nonpositive_tolerance!(tol);
    let poly_shell0 = shell0.triangulation(tol);
    let poly_shell1 = shell1.triangulation(tol);
    let altshell0: AltCurveShell<C, S> =
        shell0.mapped(|x| *x, |c| Alternative::FirstType(c.clone()), Clone::clone);
    let altshell1: AltCurveShell<C, S> =
        shell1.mapped(|x| *x, |c| Alternative::FirstType(c.clone()), Clone::clone);
    let loops_store::LoopsStoreQuadruple {
        geom_loops_store0: loops_store0,
        geom_loops_store1: loops_store1,
        ..
    } = loops_store::create_loops_stores(&altshell0, &poly_shell0, &altshell1, &poly_shell1)?;
    let mut cls0 = divide_face::divide_faces(&altshell0, &loops_store0, tol)?;
    cls0.integrate_by_component();
    let mut cls1 = divide_face::divide_faces(&altshell1, &loops_store1, tol)?;
    cls1.integrate_by_component();
    let [mut and0, mut or0, unknown0] = cls0.and_or_unknown();
    unknown0.into_iter().try_for_each(|face| {
        let pt = face.boundaries()[0].vertex_iter().next().unwrap().point();
        let dir = hash::take_one_unit(pt);
        let count = poly_shell1.iter().try_fold(0, |count, face| {
            let poly = face.surface()?;
            Some(count + poly.signed_crossing_faces(pt, dir))
        })?;
        if count >= 1 {
            and0.push(face);
        } else {
            or0.push(face);
        }
        Some(())
    })?;
    let [mut and1, mut or1, unknown1] = cls1.and_or_unknown();
    unknown1.into_iter().try_for_each(|face| {
        let pt = face.boundaries()[0].vertex_iter().next().unwrap().point();
        let dir = hash::take_one_unit(pt);
        let count = poly_shell0.iter().try_fold(0, |count, face| {
            let poly = face.surface()?;
            Some(count + poly.signed_crossing_faces(pt, dir))
        })?;
        if count >= 1 {
            and1.push(face);
        } else {
            or1.push(face);
        }
        Some(())
    })?;
    and0.append(&mut and1);
    or0.append(&mut or1);
    Some([
        altshell_to_shell(&and0, tol)?,
        altshell_to_shell(&or0, tol)?,
    ])
}

fn finalize<C: ShapeOpsCurve<S>, S: ShapeOpsSurface>(
    operation: &'static str,
    shell: Shell<Point3, C, S>,
) -> ShapeOpsResult<Solid<Point3, C, S>> {
    let boundaries = shell.connected_components();
    Solid::try_new(boundaries)
        .map_err(|source| ShapeOpsError::InvalidOutputShell { operation, source })
}

/// AND operation between two solids.
pub fn and<C: ShapeOpsCurve<S>, S: ShapeOpsSurface>(
    solid0: &Solid<Point3, C, S>,
    solid1: &Solid<Point3, C, S>,
    tol: f64,
) -> ShapeOpsResult<Solid<Point3, C, S>> {
    let operation = "and";
    let pair = |a: &Shell<Point3, C, S>, b: &Shell<Point3, C, S>| {
        process_one_pair_of_shells(a, b, tol).ok_or(ShapeOpsError::Internal { operation })
    };
    let mut iter0 = solid0.boundaries().iter();
    let mut iter1 = solid1.boundaries().iter();
    let shell0 = iter0.next().unwrap();
    let shell1 = iter1.next().unwrap();
    let [mut and_shell, _] = pair(shell0, shell1)?;
    for shell in iter0 {
        let [res, _] = pair(&and_shell, shell)?;
        and_shell = res;
    }
    for shell in iter1 {
        let [res, _] = pair(&and_shell, shell)?;
        and_shell = res;
    }
    finalize(operation, and_shell)
}

/// OR operation between two solids.
pub fn or<C: ShapeOpsCurve<S>, S: ShapeOpsSurface>(
    solid0: &Solid<Point3, C, S>,
    solid1: &Solid<Point3, C, S>,
    tol: f64,
) -> ShapeOpsResult<Solid<Point3, C, S>> {
    let operation = "or";
    let pair = |a: &Shell<Point3, C, S>, b: &Shell<Point3, C, S>| {
        process_one_pair_of_shells(a, b, tol).ok_or(ShapeOpsError::Internal { operation })
    };
    let mut iter0 = solid0.boundaries().iter();
    let mut iter1 = solid1.boundaries().iter();
    let shell0 = iter0.next().unwrap();
    let shell1 = iter1.next().unwrap();
    let [_, mut or_shell] = pair(shell0, shell1)?;
    for shell in iter0 {
        let [_, res] = pair(&or_shell, shell)?;
        or_shell = res;
    }
    for shell in iter1 {
        let [_, res] = pair(&or_shell, shell)?;
        or_shell = res;
    }
    finalize(operation, or_shell)
}

/// Difference: the region inside solid0 but outside solid1.
pub fn difference<C: ShapeOpsCurve<S>, S: ShapeOpsSurface>(
    solid0: &Solid<Point3, C, S>,
    solid1: &Solid<Point3, C, S>,
    tol: f64,
) -> ShapeOpsResult<Solid<Point3, C, S>> {
    let mut neg = solid1.clone();
    neg.not();
    and(solid0, &neg, tol)
}

/// Symmetric difference (XOR): the region inside exactly one of the two solids.
pub fn symmetric_difference<C: ShapeOpsCurve<S>, S: ShapeOpsSurface>(
    solid0: &Solid<Point3, C, S>,
    solid1: &Solid<Point3, C, S>,
    tol: f64,
) -> ShapeOpsResult<Solid<Point3, C, S>> {
    let d0 = difference(solid0, solid1, tol)?;
    let d1 = difference(solid1, solid0, tol)?;
    or(&d0, &d1, tol)
}

#[cfg(test)]
mod tests;
