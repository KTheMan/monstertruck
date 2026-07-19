//! Crate for operation shapes. Provides boolean operations to Solid, and shape healing for importing shapes from other CAD systems.

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

pub(crate) use ahash::AHashSet as HashSet;

mod healing;
pub use healing::{
    OrientationNormalization, RobustSplitClosedEdgesAndFaces, SplitClosedEdgesAndFaces,
    extract_healed, extract_healed_trimmed, extract_healed_trimmed_solid,
    normalize_shell_orientation, normalize_trimmed_shell_orientation,
};
// Doc-hidden compressed-shell repair passes reused by an external SSI
// boolean-backend upgrade crate's boolean-output healing.
#[doc(hidden)]
pub use healing::{split_non_simple_compressed_wires, split_pinched_compressed_faces};
mod transversal;
pub use transversal::{
    PlaneCut, ShapeOpsCurve, ShapeOpsError, ShapeOpsSurface, ShellOrientationHints,
    SnapCurveEndpoints, and, and_with_orientation_hints, clip_half_space_z, difference, or,
    plane_cut, symmetric_difference,
};
mod alternative;
pub mod fillet;
pub use fillet::{
    FilletError, FilletIntersectionCurve, FilletOptions, FilletProfile, FilletableCurve,
    FilletableSurface, ParameterCurveLinear, RadiusSpec, fillet, fillet_along_wire, fillet_edges,
    fillet_edges_generic, fillet_with_side,
};
