# Topology snapshot foundation scope

This follow-up starts from the Phase 3 reconciliation head and applies the
maintainer direction in
[`virtualritz/monstertruck#4`](https://github.com/virtualritz/monstertruck/issues/4)
to the previously uncommitted MT-402 snapshot work.

## Accepted foundation

Generic identity remains limited to [`StableId`] and [`StableIdAllocator`] in
`monstertruck-core`. Topology-specific state uses the existing
[`ElementAttributes`] and [`AttributeValue`] stores in `monstertruck-topology`,
keyed by [`StableId`]. This branch does not restore `TrackingId`,
`TrackingSession`, or another identity system.

The validation-only `topology-state-validation` example composes existing
`monstertruck-topology` facilities:

- [`Solid::ensure_topology_stable_ids`] assigns the accepted identifiers;
- [`Solid::compress`] supplies the exact serializable topology, allocator, and
  attribute snapshot;
- [`Solid::topology_hash`], [`Solid::topology_attribute_hash`], and
  [`Solid::content_hash`] separate topology, attribute, and geometry changes;
- [`Solid::extract`] certifies the existing snapshot round trip.

The example-local receipt is evidence for those existing primitives. It does
not publish a new snapshot, tracking, persistence, or serialization API.

## Dirty-worktree disposition

| MT-402 work | Disposition |
| --- | --- |
| Custom topology and geometry projections | Replaced by the existing [`CompressedSolid`] representation and deterministic [`Solid`] hashes. A parallel snapshot model is redundant. |
| Custom content-signature byte recorder | Removed. The accepted deterministic hashes already distinguish topology, attributes, and geometry. |
| Stable topology identity and allocator capture | Retained through [`StableId`], [`StableIdAllocator`], and [`CompressedSolid`]. |
| Semantic references | Retained only as [`AttributeValue::IdSet`] values in [`ElementAttributes`] keyed by existing [`StableId`] values. |
| Tracking session, bindings, and lineage | Deferred. Implement later **atop** a separately accepted topology tracking API. |
| Typed modeling results, publication state, and caller-owned mutable arguments | Deferred to the modeling-wrapper failure-injection layer **atop** accepted topology tracking. |
| Earlier Phase 4/5 planning documents | Rewritten around the reconciled continuity revisions. This file remains authoritative for the MT-402 snapshot boundary; the current plans govern the broader contribution sequence. |
| Host/toolchain snapshot metadata | Removed from the committed receipt. Cross-architecture hash equivalence remains out of scope and should be validated externally with a portability matrix after the hash encoding is architecture-independent. |

## Review order

The remaining work stays split into independently reviewable layers:

1. topology tracking/persistence expressed through existing stable IDs and
   attributes;
2. modeling wrappers and failure injection atop accepted topology tracking;
3. contracts and replay atop accepted tracking/persistence;
4. STEP and tessellation adapters in their owning integration layers.

This branch provides evidence for the existing topology snapshot foundation
only. It does not claim that tracking, modeling rollback, replay, or adapter
behavior exists.

[`AttributeValue`]: monstertruck-topology/src/attributes.rs
[`AttributeValue::IdSet`]: monstertruck-topology/src/attributes.rs
[`CompressedSolid`]: monstertruck-topology/src/compress.rs
[`ElementAttributes`]: monstertruck-topology/src/attributes.rs
[`Solid`]: monstertruck-topology/src/solid.rs
[`Solid::compress`]: monstertruck-topology/src/compress.rs
[`Solid::content_hash`]: monstertruck-topology/src/compress.rs
[`Solid::ensure_topology_stable_ids`]: monstertruck-topology/src/solid.rs
[`Solid::extract`]: monstertruck-topology/src/compress.rs
[`Solid::topology_attribute_hash`]: monstertruck-topology/src/compress.rs
[`Solid::topology_hash`]: monstertruck-topology/src/compress.rs
[`StableId`]: monstertruck-core/src/id.rs
[`StableIdAllocator`]: monstertruck-core/src/id.rs
