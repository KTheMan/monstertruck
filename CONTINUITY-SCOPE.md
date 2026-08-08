# Continuity contribution scope

This branch follows the maintainer direction in
[`virtualritz/monstertruck#4`](https://github.com/virtualritz/monstertruck/issues/4)
after upstream pull requests 6 and 7.

## WIP integration pocket

This fork branch integrates only the first two requested layers into the Phase
3 WIP. It is not an upstream contribution PR and must not be submitted as one
combined change. The upstream changes are extracted in order as separate PRs,
with the geometry PR based on the accepted traits PR:

1. `monstertruck-traits` owns checked continuity order, the exact
   `BoundarySide::{MinU, MaxU, MinV, MaxV}` vocabulary, and scalar-neutral
   surface capability diagnostics.
2. `monstertruck-geometry` owns local transition semantics and a direct,
   transactional solver for two full tensor-product NURBS boundaries.

The public surface API continues to use `ParametricSurface::derivative_mn`; the
branch does not add a redundant public jet API. Private Taylor arithmetic is
limited to composing the local transition and differentiating optimized
controls. Evidence uses example-local DTOs rather than publishing a solver
serialization or persistence contract. The solver uses the established `f64`
trait family deliberately. A later port to the scalar-generic v2 traits is a
separate migration after v2 exposes the required arbitrary-order derivative
contract. `G0` through `G3` are the production target, while `G4` is explicitly
experimental and requires public opt-in.

Dense work uses explicit `ContinuityLimits`, a `BudgetedContinuitySolve`
outcome, typed `ContinuityTruncated` refusal, and deterministic work meters.
This follows the bounded-work carrier/refusal/meter shape already used by
surface parameter division.

## Explicitly out of scope

| Work | Placement |
| --- | --- |
| Topology tracking and persistence | Implement later **atop** `StableId`/`StableIdAllocator` in `monstertruck-core` and the `ElementAttributes`/`AttributeValue` storage in `monstertruck-topology`. No parallel identity system belongs in `monstertruck-core`. |
| Modeling tracking wrappers | Implement later **atop** an accepted topology tracking API, not inside the direct geometry solver contribution. |
| Contract graphs and replay | Implement later **atop** the separately reviewed tracking/persistence layer. Solver acceptance does not imply their API shape. |
| STEP seam selection/identification, repair adapters, export, and re-import evidence | Implemented in `monstertruck-step::continuity`, atop the direct solver API. Automatic seam discovery remains an explicit non-goal. `continuity-repair-step` exercises import, adjacent shared-edge selection, a deterministic post-import edit that preserves `G0` while breaking `G1`, solve, independent certification, replacement, nonempty tessellation, export, re-import, repeated certification, and a pinned OCCT validity oracle without adding STEP dependencies to the numerical crate. |
| Arbitrary trimmed seams | `repair_step_continuity` accepts only an exact complete patch side. Missing, curved, partial, and otherwise arbitrary trims return typed `TrimmedBoundary`; the adapter never attempts best-effort repair. |
| Tessellation provenance | Implement later **atop** existing topology stable IDs and attributes at the topology/meshing integration boundary. It is not solver state. |

## Planned review sequence

Upstream work stays split into independently reviewable changes:

1. traits checked order and capability;
2. geometry local transition and direct full-boundary solver;
3. topology tracking/persistence using existing IDs;
4. contracts/replay atop accepted tracking.

This file supersedes the broader phase-planning documents previously carried
on the WIP branch for purposes of upstream contribution shape.
