//! Geometric primitives for CAD modeling: B-spline and NURBS curves/surfaces,
//! knot vectors, and decorator types (revolved, extruded, intersection curves).

#![cfg_attr(not(debug_assertions), deny(warnings))]
#![deny(clippy::all, rust_2018_idioms)]
#![warn(
    missing_docs,
    missing_debug_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces,
    unused_qualifications
)]

use monstertruck_core::bounding_box::Bounded;
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, ops::Bound};

const INCLUDE_CURVE_TRIALS: usize = 100;
const PRESEARCH_DIVISION: usize = 50;

/// re-export `monstertruck_core`
pub mod base {
    pub use monstertruck_core::{
        assert_near, assert_near2, bounding_box::BoundingBox, cgmath64::*, hash, hash::HashGen,
        prop_assert_near, prop_assert_near2, tolerance::*,
    };
    pub use monstertruck_traits::*;
}
/// NURBS and B-spline curves, surfaces, and knot vectors.
pub mod nurbs;

/// Error types for geometry operations.
pub mod errors;

/// Concrete geometric primitives: [`Plane`](crate::specifieds::Plane), [`Sphere`](crate::specifieds::Sphere), [`Line`](crate::specifieds::Line), etc.
pub mod specifieds;

/// Composite geometry: revolved curves, intersection curves, processor wrappers.
pub mod decorators;

/// T-Spline and T-NURCC surface types.
pub mod t_spline;

mod analytic_surface;
/// [`DeterministicContentHash`](monstertruck_core::DeterministicContentHash) impls for geometry types.
mod content_hash_impls;

/// Trait for extracting an exact polynomial B-spline surface representation.
mod bspline_conversion;
mod parameter_boundary;

/// re-export all modules.
pub mod prelude {
    use crate::*;
    pub use analytic_surface::{
        AnalyticSurfaceKind, HomogeneousExtrusionSurface, SphericalRevolutionSurface,
        SurfaceParameterAxis, TryIntoAnalyticSurfaceKind,
    };
    pub use base::*;
    pub use bspline_conversion::{
        SupportsExactPatchDomains, TryIntoBsplineSurface, TryIntoHomogeneousBsplineCurve,
        TryIntoHomogeneousBsplineSurface,
    };
    pub use decorators::*;
    pub use errors::*;
    pub use nurbs::*;
    pub use parameter_boundary::BoundaryCurve2D;
    pub use specifieds::*;
    pub use t_spline::*;
}
