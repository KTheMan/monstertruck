//! Types shared between the [`load`](crate::load) and [`save`](crate::save) sides of STEP I/O.

/// Identifying attributes attached to a STEP part (product or component).
///
/// Surfaced by the load side as the `attrs` payload of a STEP assembly node
/// or edge, and consumed by the save side when emitting `PRODUCT*` /
/// `SHAPE_DEFINITION_REPRESENTATION` entities.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PartAttributes {
    /// `id` of the part. Mirrors the STEP `PRODUCT.id` field.
    pub id: String,
    /// Human-readable name of the part.
    pub name: String,
    /// Free-form description of the part.
    pub description: String,
}

// `PartAttrs` is the upstream `truck-stepio` spelling. Abbreviating
// `Attributes` to `Attrs` saves three characters of typing at the cost of
// reading clarity at every call site; we standardise on the non-abbreviated
// name in public APIs and keep the alias only so existing callers (and code
// ported from `truck-stepio`) continue to compile. Slated for removal once
// downstream callers have moved off the old name.
#[deprecated(since = "0.3.1", note = "renamed to `PartAttributes`.")]
pub use PartAttributes as PartAttrs;
