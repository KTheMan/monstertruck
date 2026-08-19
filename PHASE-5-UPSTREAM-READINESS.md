# Phase 5 -- Upstream Readiness

Phase 5 turns the locally proven continuity work into a focused proposal and,
after maintainer approval, an upstream contribution based on current
`upstream/master`.

## Exact baselines

| Tree | Revision | Status |
| --- | --- | --- |
| `upstream/master` | `4973612f` | Current upstream authority after merged PR 22; it contains PR 13 and PR 19 unchanged. |
| upstream PR 19 | `06201787` | Historical merge provenance for the maintainer-authored capability-inspection slice, from reviewed head `bf4cd9c5`; this remains the accepted capability shape. |
| `agent/layer-2-include` | `389e1da7` | Dev proving-ground code slice synchronized through `4973612f`; it assumes an included read-only transition evaluator without treating that shape as approved upstream. |
| `dev` | `55bc8b32` | First proving ground synchronized through `4973612f`; code evidence also retains the fixed production-`G3` lineage at `47e837ca`. |
| `master` | `7a5fa5e2` | Second proving ground synchronized through `4973612f`. Not pushed. |
| `agent/mt402-state-snapshots` | `e1708d66` | Preserved review branch for the plan rewrite and superseded custom MT-402 assessment. |
| `archive/mt402-superseded-modeling-harness` | `568c3c4f` | Unmerged preservation of the superseded harness and its useful probes. |

Validation claims in this document apply only to the revision named in their
row. No hosted CI result or upstream acceptance is implied for a local branch.

## Completed locally

- Current `upstream/master` at `4973612f` contains the checked continuity
  vocabulary and crate ownership established by merged PR 13.
- Merged PR 19 at `06201787` remains the provenance for the maintainer's revised
  capability-inspection slice, including inherent methods and the checked
  unsupported-report helper; no later upstream commit changes those files.
- Current topic `agent/layer-2-include` preserves the broader local transition
  and solver work on upstream 0.4.0. Its dev review assumes an included
  read-only evaluator, but its public solver shape is not approved upstream.
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

Current upstream `4973612f`, including PR 19 provenance at `06201787`, is
reconciled through both local proving grounds at master `7a5fa5e2` and dev
`55bc8b32`. PR 22 makes no continuity-file change.
The immediate upstream-facing action is to propose the transition and solver
follow-on without extending the accepted capability slice, which contains only:

1. concrete B-spline and NURBS capability inspection;
2. typed unsupported outcomes and highest-supported-order reporting;
3. the checked `try_unsupported` invariant in one local helper.

Two-surface compatibility, transition semantics, and solver feasibility are
explicitly outside PR 19. They must be proposed as a separate follow-on.

The workbench already exposes a read-only `BoundaryTransition` evaluator used
for independent certification, metrics, and deterministic evidence. Downstream
control can be composed over the same evaluator. The dev proving-ground topic
assumes that an upstream successful result should also include it; this remains
a review hypothesis until the issue #4 follow-up receives maintainer direction.

The workbench temporarily retains compatibility-only degree getters, an
`Option` evaluator, and overlapping solution decompositions because existing
integration tests exercise them and test ownership forbids rewriting those
tests. The include topic adds a typed canonical evaluator and keeps numerical
mechanics private. A clean upstream recut must omit the compatibility facade and
preserve typed transactional failure.

## Contribution sequence

1. Preserve reviewed PR 19 head `bf4cd9c5` and merged revision `06201787` as
   provenance while treating `4973612f` as the current contribution base.
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

Imported evidence uses the existing `monstertruck-io` STEP read/write path.
IGES evidence remains outside geometry and can be shaped after upstream
end-to-end support and a file fixture establish the appropriate adapter; neither
format introduces a geometry dependency or a parallel exchange abstraction.

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
| `monstertruck-workbench` | Active construction workspace on `agent/layer-2-include`; the include hypothesis is shaped here before dev review. |
| `monstertruck` | Primary Git checkout retaining shared worktree metadata, detached at current `upstream/master`; not an integration target. |
| `monstertruck-dev` | Current `dev` proving ground; plan integration descends from `3a81e439` and runtime evidence is bound to `47e837ca`. |
| `monstertruck-master` | Current local `master` proving ground; runtime promotion lineage descends from `0dd9fdc9`. |

The production-`G3`, plan-review, and superseded MT-402 work remain preserved
as branch refs at `8c4ec61c`, `e1708d66`, and `568c3c4f` after their dedicated
worktrees are retired. No worktree is evidence for a different revision.
The historical PR 19 review worktree was retired after the merged capability
semantics were reconciled. The obsolete `agent/pr19-plan-reconciliation` branch
was deleted after its tree matched both proving grounds. Current upstream
`4973612f` is now present in both proving grounds.

## Phase completion

Phase 5 is complete only when maintainer direction has established the public
shape, the approved contribution has been reconstructed on current
`upstream/master`, and that exact revision has passed its required local gates.
