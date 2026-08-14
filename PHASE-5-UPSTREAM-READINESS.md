# Phase 5 -- Upstream Readiness

Phase 5 turns the locally proven continuity work into a focused proposal and,
after maintainer approval, an upstream contribution based on current
`upstream/master`.

## Exact baselines

| Tree | Revision | Status |
| --- | --- | --- |
| `upstream/master` | `06201787` | Merged PR 13 checked traits plus PR 19 inherent geometry capability inspection; current upstream authority. |
| upstream PR 19 | `06201787` | Maintainer-authored capability-inspection slice, merged from reviewed head `bf4cd9c5`; this is the accepted capability shape. |
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
- Merged PR 19 at `06201787` carries the maintainer's revised capability-inspection
  slice, including inherent methods and the checked unsupported-report helper.
- Revision `39f6a86a` preserves the broader local transition and solver work,
  but is no longer a current upstream-shaped capability contribution.
- Revision `47e837ca` adds the production `G3` proof matrix and passed its
  local phase verification.
- Revision `0dd9fdc9` promotes the verified result through the second local
  proving ground.
- `G4` remains an explicitly experimental path and is not part of the
  production acceptance claim.

These results establish local implementation and evidence. Merged PR 19
establishes the capability API only; transition plus solver APIs have not been
approved upstream.

## Upstream proposal gate

Merged revision `06201787` is reconciled through both local proving grounds.
The immediate upstream-facing action is to propose the transition and solver
follow-on without extending the accepted capability slice, which contains only:

1. concrete B-spline and NURBS capability inspection;
2. typed unsupported outcomes and highest-supported-order reporting;
3. the checked `try_unsupported` invariant in one local helper.

Two-surface compatibility, transition semantics, and solver feasibility are
explicitly outside PR 19. They must be proposed as a separate follow-on.

## Contribution sequence

1. Preserve reviewed PR 19 head `bf4cd9c5` and merged revision `06201787` as
   provenance.
2. Keep the completed `master` then `dev` reconciliation synchronized if
   upstream advances.
3. Keep the local transition and solver implementation on the inherent
   capability methods and exact merged diagnostic precedence.
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
| Repository root | Active topic workspace; currently used for PR 19 reconciliation work. |
| `monstertruck-dev-integration` | Current `dev` proving ground; plan integration descends from `3a81e439` and runtime evidence is bound to `47e837ca`. |
| `monstertruck-master-strict-compliance` | Current local `master` proving ground; runtime promotion lineage descends from `0dd9fdc9`. |
| `monstertruck-upstream-layer2` | Historical PR 19 review checkout at `bf4cd9c5`; merged authority is `06201787`, and the prior combined candidate remains preserved at `39f6a86a`. |

The production-`G3`, plan-review, and superseded MT-402 work remain preserved
as branch refs at `8c4ec61c`, `e1708d66`, and `568c3c4f` after their dedicated
worktrees are retired. No worktree is evidence for a different revision.

## Phase completion

Phase 5 is complete only when maintainer direction has established the public
shape, the approved contribution has been reconstructed on current
`upstream/master`, and that exact revision has passed its required local gates.
