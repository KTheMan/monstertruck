//! Deterministic content hashing for BRep cache invalidation.
//!
//! This module provides [`DeterministicContentHash`], a trait for producing
//! stable, deterministic `u64` hashes of semantic content. Unlike
//! [`std::hash::Hash`], implementations here must be independent of pointer
//! identity, allocation order, and [`HashMap`](std::collections::HashMap)
//! iteration order.
//!
//! The default hasher is xxHash3-64 via [`ContentHasher`].

use std::collections::BTreeMap;
use std::hash::Hasher;

use cgmath::{Matrix3, Matrix4, Point2, Point3, Vector2, Vector3, Vector4};

use crate::StableId;

/// Deterministic content hasher backed by xxHash3-64.
///
/// Wraps [`xxhash_rust::xxh3::Xxh3`] and implements [`std::hash::Hasher`]
/// delegation so it can be used as a standard [`Hasher`].
#[derive(Default)]
pub struct ContentHasher(xxhash_rust::xxh3::Xxh3);

impl std::fmt::Debug for ContentHasher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContentHasher").finish_non_exhaustive()
    }
}

impl ContentHasher {
    /// Create a new hasher with the default seed.
    pub fn new() -> Self { Self::default() }
}

impl Hasher for ContentHasher {
    fn finish(&self) -> u64 { self.0.finish() }
    fn write(&mut self, bytes: &[u8]) { self.0.write(bytes) }
}

/// Deterministic content hash for BRep cache invalidation.
///
/// Implementations must be stable across runs and independent of pointer
/// identity. Floats are hashed by their raw bit patterns; collections are
/// hashed in deterministic (sorted or index) order.
pub trait DeterministicContentHash {
    /// Feed this value into the given [`Hasher`].
    fn content_hash<H: Hasher>(&self, state: &mut H);

    /// Convenience: compute a standalone 64-bit hash.
    fn content_hash64(&self) -> u64 {
        let mut hasher = ContentHasher::new();
        self.content_hash(&mut hasher);
        hasher.finish()
    }
}

// ---------------------------------------------------------------------------
// Primitive types
// ---------------------------------------------------------------------------

impl DeterministicContentHash for () {
    fn content_hash<H: Hasher>(&self, _state: &mut H) {}
}

impl DeterministicContentHash for bool {
    fn content_hash<H: Hasher>(&self, state: &mut H) { state.write_u8(*self as u8); }
}

impl DeterministicContentHash for u8 {
    fn content_hash<H: Hasher>(&self, state: &mut H) { state.write_u8(*self); }
}

impl DeterministicContentHash for u16 {
    fn content_hash<H: Hasher>(&self, state: &mut H) { state.write_u16(*self); }
}

impl DeterministicContentHash for u32 {
    fn content_hash<H: Hasher>(&self, state: &mut H) { state.write_u32(*self); }
}

impl DeterministicContentHash for u64 {
    fn content_hash<H: Hasher>(&self, state: &mut H) { state.write_u64(*self); }
}

impl DeterministicContentHash for usize {
    fn content_hash<H: Hasher>(&self, state: &mut H) { state.write_usize(*self); }
}

impl DeterministicContentHash for i8 {
    fn content_hash<H: Hasher>(&self, state: &mut H) { state.write_i8(*self); }
}

impl DeterministicContentHash for i16 {
    fn content_hash<H: Hasher>(&self, state: &mut H) { state.write_i16(*self); }
}

impl DeterministicContentHash for i32 {
    fn content_hash<H: Hasher>(&self, state: &mut H) { state.write_i32(*self); }
}

impl DeterministicContentHash for i64 {
    fn content_hash<H: Hasher>(&self, state: &mut H) { state.write_i64(*self); }
}

impl DeterministicContentHash for isize {
    fn content_hash<H: Hasher>(&self, state: &mut H) { state.write_isize(*self); }
}

// ---------------------------------------------------------------------------
// Floating-point -- raw bit patterns.
// ---------------------------------------------------------------------------

impl DeterministicContentHash for f32 {
    fn content_hash<H: Hasher>(&self, state: &mut H) { state.write_u32(self.to_bits()); }
}

impl DeterministicContentHash for f64 {
    fn content_hash<H: Hasher>(&self, state: &mut H) { state.write_u64(self.to_bits()); }
}

// ---------------------------------------------------------------------------
// Strings.
// ---------------------------------------------------------------------------

impl DeterministicContentHash for str {
    fn content_hash<H: Hasher>(&self, state: &mut H) {
        state.write_usize(self.len());
        state.write(self.as_bytes());
    }
}

impl DeterministicContentHash for String {
    fn content_hash<H: Hasher>(&self, state: &mut H) { self.as_str().content_hash(state); }
}

// ---------------------------------------------------------------------------
// Slices and arrays.
// ---------------------------------------------------------------------------

impl<T: DeterministicContentHash> DeterministicContentHash for [T] {
    fn content_hash<H: Hasher>(&self, state: &mut H) {
        state.write_usize(self.len());
        self.iter().for_each(|item| item.content_hash(state));
    }
}

impl<T: DeterministicContentHash, const N: usize> DeterministicContentHash for [T; N] {
    fn content_hash<H: Hasher>(&self, state: &mut H) {
        self.iter().for_each(|item| item.content_hash(state));
    }
}

// ---------------------------------------------------------------------------
// Standard containers.
// ---------------------------------------------------------------------------

impl<T: DeterministicContentHash> DeterministicContentHash for Vec<T> {
    fn content_hash<H: Hasher>(&self, state: &mut H) { self.as_slice().content_hash(state); }
}

impl<T: DeterministicContentHash> DeterministicContentHash for Option<T> {
    fn content_hash<H: Hasher>(&self, state: &mut H) {
        match self {
            None => state.write_u8(0),
            Some(v) => {
                state.write_u8(1);
                v.content_hash(state);
            }
        }
    }
}

impl<K: DeterministicContentHash, V: DeterministicContentHash> DeterministicContentHash
    for BTreeMap<K, V>
{
    fn content_hash<H: Hasher>(&self, state: &mut H) {
        state.write_usize(self.len());
        // BTreeMap iterates in key order -- deterministic.
        self.iter().for_each(|(k, v)| {
            k.content_hash(state);
            v.content_hash(state);
        });
    }
}

// ---------------------------------------------------------------------------
// Tuples.
// ---------------------------------------------------------------------------

impl<A: DeterministicContentHash, B: DeterministicContentHash> DeterministicContentHash for (A, B) {
    fn content_hash<H: Hasher>(&self, state: &mut H) {
        self.0.content_hash(state);
        self.1.content_hash(state);
    }
}

impl<A: DeterministicContentHash, B: DeterministicContentHash, C: DeterministicContentHash>
    DeterministicContentHash for (A, B, C)
{
    fn content_hash<H: Hasher>(&self, state: &mut H) {
        self.0.content_hash(state);
        self.1.content_hash(state);
        self.2.content_hash(state);
    }
}

// ---------------------------------------------------------------------------
// References.
// ---------------------------------------------------------------------------

impl<T: DeterministicContentHash + ?Sized> DeterministicContentHash for &T {
    fn content_hash<H: Hasher>(&self, state: &mut H) { (**self).content_hash(state); }
}

impl<T: DeterministicContentHash + ?Sized> DeterministicContentHash for Box<T> {
    fn content_hash<H: Hasher>(&self, state: &mut H) { (**self).content_hash(state); }
}

// ---------------------------------------------------------------------------
// cgmath types -- hash by component bit patterns.
// ---------------------------------------------------------------------------

impl DeterministicContentHash for Point2<f64> {
    fn content_hash<H: Hasher>(&self, state: &mut H) {
        self.x.content_hash(state);
        self.y.content_hash(state);
    }
}

impl DeterministicContentHash for Point3<f64> {
    fn content_hash<H: Hasher>(&self, state: &mut H) {
        self.x.content_hash(state);
        self.y.content_hash(state);
        self.z.content_hash(state);
    }
}

impl DeterministicContentHash for Vector2<f64> {
    fn content_hash<H: Hasher>(&self, state: &mut H) {
        self.x.content_hash(state);
        self.y.content_hash(state);
    }
}

impl DeterministicContentHash for Vector3<f64> {
    fn content_hash<H: Hasher>(&self, state: &mut H) {
        self.x.content_hash(state);
        self.y.content_hash(state);
        self.z.content_hash(state);
    }
}

impl DeterministicContentHash for Vector4<f64> {
    fn content_hash<H: Hasher>(&self, state: &mut H) {
        self.x.content_hash(state);
        self.y.content_hash(state);
        self.z.content_hash(state);
        self.w.content_hash(state);
    }
}

impl DeterministicContentHash for Matrix3<f64> {
    fn content_hash<H: Hasher>(&self, state: &mut H) {
        self.x.content_hash(state);
        self.y.content_hash(state);
        self.z.content_hash(state);
    }
}

impl DeterministicContentHash for Matrix4<f64> {
    fn content_hash<H: Hasher>(&self, state: &mut H) {
        self.x.content_hash(state);
        self.y.content_hash(state);
        self.z.content_hash(state);
        self.w.content_hash(state);
    }
}

// ---------------------------------------------------------------------------
// Domain types from monstertruck-core.
// ---------------------------------------------------------------------------

impl DeterministicContentHash for StableId {
    fn content_hash<H: Hasher>(&self, state: &mut H) { self.raw().content_hash(state); }
}

// ---------------------------------------------------------------------------
// Tests.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_input_same_hash() {
        assert_eq!(1_u64.content_hash64(), 1_u64.content_hash64());
    }

    #[test]
    fn different_input_different_hash() {
        assert_ne!(1_u64.content_hash64(), 2_u64.content_hash64());
    }

    #[test]
    fn bool_hashing() {
        assert_ne!(true.content_hash64(), false.content_hash64());
    }

    #[test]
    fn string_hashing() {
        assert_eq!("hello".content_hash64(), "hello".content_hash64());
        assert_ne!("hello".content_hash64(), "world".content_hash64());
    }

    #[test]
    fn vec_hashing() {
        let a = vec![1_u64, 2, 3];
        let b = vec![1_u64, 2, 3];
        let c = vec![1_u64, 2, 4];
        assert_eq!(a.content_hash64(), b.content_hash64());
        assert_ne!(a.content_hash64(), c.content_hash64());
    }

    #[test]
    fn option_hashing() {
        let a: Option<u64> = Some(42);
        let b: Option<u64> = Some(42);
        let c: Option<u64> = None;
        assert_eq!(a.content_hash64(), b.content_hash64());
        assert_ne!(a.content_hash64(), c.content_hash64());
    }

    #[test]
    fn f64_bit_pattern_hashing() {
        assert_eq!(1.0_f64.content_hash64(), 1.0_f64.content_hash64());
        assert_ne!(1.0_f64.content_hash64(), 1.0000000001_f64.content_hash64());
    }

    #[test]
    fn point3_hashing() {
        let a = Point3::new(1.0, 2.0, 3.0);
        let b = Point3::new(1.0, 2.0, 3.0);
        let c = Point3::new(1.0, 2.0, 4.0);
        assert_eq!(a.content_hash64(), b.content_hash64());
        assert_ne!(a.content_hash64(), c.content_hash64());
    }

    #[test]
    fn stable_id_hashing() {
        let a = StableId::new(1);
        let b = StableId::new(1);
        let c = StableId::new(2);
        assert_eq!(a.content_hash64(), b.content_hash64());
        assert_ne!(a.content_hash64(), c.content_hash64());
    }

    #[test]
    fn btreemap_hashing() {
        let a = BTreeMap::from([(1_u64, 10_u64), (2, 20)]);
        let b = BTreeMap::from([(1_u64, 10_u64), (2, 20)]);
        let c = BTreeMap::from([(1_u64, 10_u64), (2, 21)]);
        assert_eq!(a.content_hash64(), b.content_hash64());
        assert_ne!(a.content_hash64(), c.content_hash64());
    }

    #[test]
    fn tuple_hashing() {
        assert_eq!(
            (1_u64, 2_u64).content_hash64(),
            (1_u64, 2_u64).content_hash64()
        );
        assert_ne!(
            (1_u64, 2_u64).content_hash64(),
            (1_u64, 3_u64).content_hash64()
        );
    }
}
