# Phase 5 -- Upstream Readiness

Phase 5 turns the locally proven continuity work into a focused proposal and,
after maintainer approval, an upstream contribution based on current
`upstream/master`.

## Exact baselines

| Tree | Revision | Status |
| --- | --- | --- |
| `upstream/master` | `609c1b5a` | Merged PR 13 continuity-traits foundation and current upstream authority. |
| upstream PR 19 | `bf4cd9c5` | Maintainer-authored capability-inspection slice, open against `609c1b5a`; this is the current proposed capability shape. |
| `agent/upstream-layer-2-replacement` | `39f6a86a` | Preserved local combined implementation. Its capability portion is superseded by PR 19, while its transition and solver work remain follow-on material. |
| `dev` | current branch head | First proving ground; plan integration descends from `3a81e439`, while code evidence remains bound to `47e837ca`. |
| `master` | current branch head | Second proving ground; runtime promotion lineage descends from `0dd9fdc9`. Not pushed. |
| `agent/mt402-state-snapshots` | `e1708d66` | Preserved review branch for the plan rewrite and superseded custom MT-402 assessment. |
| `archive/mt402-superseded-modeling-harness` | `568c3c4f` | Unmerged preservation of the superseded harness and its useful probes. |

Validation claims in this document apply only to the revision named in their
row. No hosted CI result or upstream acceptance is implied for a local branch.

## Completed locally

- The checked continuity vocabulary and crate ownership established by merged
  PR 13 are present in `upstream/master`.
- PR 19 at `bf4cd9c5` carries the maintainer's revised capability-inspection
  slice, including inherent methods and the checked unsupported-report helper.
- Revision `39f6a86a` preserves the broader local transition and solver work,
  but is no longer a current upstream-shaped capability contribution.
- Revision `47e837ca` adds the production `G3` proof matrix and passed its
  local phase verification.
- Revision `0dd9fdc9` promotes the verified result through the second local
  proving ground.
- `G4` remains an explicitly experimental path and is not part of the
  production acceptance claim.

These results establish local implementation and evidence. PR 19 remains open,
and transition plus solver APIs have not been approved upstream.

## Upstream proposal gate

The immediate upstream-facing action is to track PR 19 to its exact merged
revision. That PR contains only:

1. concrete B-spline and NURBS capability inspection;
2. typed unsupported outcomes and highest-supported-order reporting;
3. the checked `try_unsupported` invariant in one local helper.

Two-surface compatibility, transition semantics, and solver feasibility are
explicitly outside PR 19. They must be proposed as a follow-on after the merged
capability source is reconciled through the local proving grounds.

## Contribution sequence

1. Track PR 19 from `bf4cd9c5` to its exact merged revision.
2. Reconcile that merged revision into `master`, then `dev`, without preserving
   the old free-function capability API as a compatibility layer.
3. Adapt the local transition and solver implementation to the inherent
   capability methods and exact diagnostic precedence that upstream lands.
4. Prepare a separate transition and solver proposal with typed truncation,
   measured work, transactional behavior, and experimental `G4` called out.
5. Recut the approved follow-on as a clean branch from the then-current
   `upstream/master`, without duplicating PR 19.
6. Reapply the production `G3` evidence from `47e837ca` where it fits the
   approved contribution boundary.
7. Validate the exact contribution revision and report only those results.

## Production `G3` evidence boundary

The local production gate covers the supported full-boundary path. Its
evidence includes polynomial and positive-rational surfaces, multi-span and
repeated-knot cases, scale and weight conditioning, unequal and reversed
parameterization, independent dense certification, deterministic bounded
failure, and imported full-side workflow evidence.

This does not extend support to arbitrary trimmed subcurves, automatic seam
discovery, topology sewing, global constraint networks, or production `G4`.
Arbitrary trimmed seams remain a typed refusal case.

## Deferred downstream layers

Tracking, persistence, contracts, and replay remain downstream until the
upstream solver proposal and API review are accepted. Later work must build on
the repository's established identity and attribute systems.

The custom MT-402 modeling snapshot harness is superseded by the existing
topology-owned foundation recorded in `TRACKING-SCOPE.md`. Its useful probes
remain preserved at archive commit `568c3c4f` for later extraction, but its
parallel projections, result state, and mutable-state model must not be
integrated. MT-403 and later tracked-modeling atomicity work remain deferred
until upstream solver acceptance and an approved tracking/modeling shape.

## Worktree roles

| Worktree | Role |
| --- | --- |
| Repository root | Active workspace on the reconciled local `master` lineage. |
| `monstertruck-dev-integration` | Current `dev` proving ground; plan integration descends from `3a81e439` and runtime evidence is bound to `47e837ca`. |
| `monstertruck-master-strict-compliance` | Current local `master` proving ground; runtime promotion lineage descends from `0dd9fdc9`. |
| `monstertruck-upstream-layer2` | Exact PR 19 review checkout at `bf4cd9c5`; the prior combined candidate remains preserved at `39f6a86a`. |

The production-`G3`, plan-review, and superseded MT-402 work remain preserved
as branch refs at `8c4ec61c`, `e1708d66`, and `568c3c4f` after their dedicated
worktrees are retired. No worktree is evidence for a different revision.

## Phase completion

Phase 5 is complete only when maintainer direction has established the public
shape, the approved contribution has been reconstructed on current
`upstream/master`, and that exact revision has passed its required local gates.
