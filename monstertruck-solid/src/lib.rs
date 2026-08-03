//! Crate for operation shapes. Provides boolean operations to Solid.
//!
//! Shape healing and fillets are POST-CSG and kernel-independent, so they live
//! in their own crates: `monstertruck-healing` and `monstertruck-fillet`. This
//! crate is the classic polyline-marching boolean kernel and nothing else, which
//! is what lets an external SSI boolean backend stand in for it wholesale.

#![cfg_attr(not(debug_assertions), deny(warnings))]
#![deny(clippy::all, rust_2018_idioms)]
// Under `--no-default-features` no boolean backend is compiled: the classic
// marcher (`marching-ssi`, the published default) is off and the boolean entry
// points return `ShapeOpsError::NoBackend`. Their generic helpers then go
// unused, so allow dead code in exactly that no-backend configuration.
#![cfg_attr(not(feature = "marching-ssi"), allow(dead_code))]
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

mod transversal;
pub use transversal::{
    PlaneCut, ShapeOpsCurve, ShapeOpsError, ShapeOpsSurface, ShellOrientationHints,
    SnapCurveEndpoints, and, and_with_orientation_hints, clip_half_space_z, difference, or,
    plane_cut, symmetric_difference,
};
mod alternative;
