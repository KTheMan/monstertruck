# Phase 5 -- Upstream Readiness

Phase 5 turns the locally proven continuity work into a focused proposal and,
after maintainer approval, an upstream contribution based on current
`upstream/master`.

## Exact baselines

| Tree | Revision | Status |
| --- | --- | --- |
| `upstream/master` | `609c1b5a` | Merged PR 13 continuity-traits foundation and current upstream authority. |
| `agent/upstream-layer-2-replacement` | `39f6a86a` | Local upstream-shaped geometry candidate containing representation inspection, transitions, and a bounded transactional full-side solver. Not pushed or accepted upstream. |
| `dev` | current branch head | First proving ground; plan integration descends from `3a81e439`, while code evidence remains bound to `47e837ca`. |
| `master` | `0dd9fdc9` | Second proving ground and current local fork baseline. Not pushed. |
| `agent/mt402-state-snapshots` | based on `dev` | Review branch for the superseded custom MT-402 harness and current plans. |
| `archive/mt402-superseded-modeling-harness` | `568c3c4f` | Unmerged preservation of the superseded harness and its useful probes. |

Validation claims in this document apply only to the revision named in their
row. No hosted CI result or upstream acceptance is implied for a local branch.

## Completed locally

- The checked continuity vocabulary and crate ownership established by merged
  PR 13 are present in `upstream/master`.
- Revision `39f6a86a` provides the upstream-shaped layer-2 implementation on
  top of that foundation without carrying downstream branch history.
- Revision `47e837ca` adds the production `G3` proof matrix and passed its
  local phase verification.
- Revision `0dd9fdc9` promotes the verified result through the second local
  proving ground.
- `G4` remains an explicitly experimental path and is not part of the
  production acceptance claim.

These results establish local implementation and evidence. They do not mean
that the layer-2 public API or solver contribution has been approved upstream.

## Upstream proposal gate

The next upstream-facing action is an API and contribution-shape proposal
against `609c1b5a`. It must present one coherent geometry layer:

1. concrete B-spline and NURBS capability inspection;
2. aligned and reversed full-boundary transitions;
3. a bounded, transactional full-side solver;
4. typed unsupported and truncated outcomes with measured work;
5. polynomial and positive-rational evidence through production `G3`;
6. an explicit experimental opt-in for `G4`.

The proposal must identify the exact public items requiring maintainer review.
Implementation evidence may illustrate the proposal, but local names and
signatures remain provisional until approved.

## Contribution sequence

1. Review the `39f6a86a` delta against `609c1b5a` and remove anything not
   required by the proposed geometry layer.
2. Prepare the API proposal with typed outcome semantics, ownership, module
   boundaries, and the experimental `G4` boundary called out explicitly.
3. Obtain maintainer direction before changing or publishing the proposed
   public surface.
4. Recut the approved implementation as a clean branch from the then-current
   `upstream/master`.
5. Reapply the production `G3` evidence from `47e837ca` where it fits the
   approved contribution boundary.
6. Validate the exact contribution revision and report only those results.

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
| Repository root | Preservation checkout for earlier uncommitted work and history inspection. |
| `monstertruck-dev-integration` | Current `dev` proving ground; plan integration descends from `3a81e439` and runtime evidence is bound to `47e837ca`. |
| `monstertruck-master-strict-compliance` | Local `master` proving ground at `0dd9fdc9`. |
| `monstertruck-upstream-layer2` | Focused upstream layer-2 candidate at `39f6a86a`. |
| `monstertruck-production-g3` | Isolated production `G3` evidence worktree. |
| `monstertruck-mt402-conformance` | Review checkout for current plans and archived MT-402 design work. |

Keep each worktree until its unique work is committed, deliberately moved, or
recorded as superseded. No worktree is evidence for a different revision.

## Phase completion

Phase 5 is complete only when maintainer direction has established the public
shape, the approved contribution has been reconstructed on current
`upstream/master`, and that exact revision has passed its required local gates.
