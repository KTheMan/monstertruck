# Phase 4 -- Current Continuity Audit

This document replaces the earlier downstream Phase 4 history ledger. It audits
the continuity implementation and evidence that exist on the reconciled local
trees. It does not carry forward claims about superseded tracking, replay, or
solver branches.

## Revision boundary

| Tree | Revision | Audit role |
| --- | --- | --- |
| `upstream/master` | `06201787` | Merged PR 13 checked traits plus PR 19 inherent geometry capability inspection; current upstream semantic authority. |
| upstream PR 19 | `06201787` | Maintainer-authored capability-inspection slice, merged from reviewed head `bf4cd9c5`. |
| `agent/upstream-layer-2-replacement` | `39f6a86a` | Preserved local combined capability, transition, and solver implementation; no longer the current upstream contribution shape. |
| `dev` | `47e837ca` | First local proving ground, including the production `G3` evidence extension. |
| `master` | `0dd9fdc9` | Second local proving-ground promotion. Not pushed. |

A result recorded for one revision does not automatically apply to another.
No hosted CI or upstream acceptance is claimed for the local branches.

## Architecture finding

Merged PR 13 establishes the checked continuity vocabulary in
`monstertruck-traits`. Merged PR 19 establishes the geometry capability
shape: inherent inspection methods on B-spline and NURBS surfaces, one checked
unsupported-report helper, and no two-surface compatibility or solver
feasibility in that slice.

Revision `39f6a86a` contains one coherent geometry layer:

- B-spline and NURBS capability inspection with typed unsupported reasons;
- aligned and reversed full-boundary transitions;
- a bounded full-side solver with typed truncation and measured work;
- transactional solve output, with no partial solved surface on failure;
- explicit opt-in for experimental `G4`.

This combined local shape remains useful implementation evidence, but it is not
the current upstream contribution boundary. Its capability code must be
replaced by the exact merged PR 19 implementation, and transition plus solving
must be proposed as a follow-on.

The current reconciliation incorporates the exact merged PR 19 capability
implementation and changes production solver validation to call the inherent
`NurbsSurface::continuity_capability` method. Stricter free-function adapters
remain hidden for the established downstream robustness corpus only; they are
not part of the future upstream solver recut.

## Evidence finding

Revision `47e837ca` adds the current production `G3` evidence to the layer-2
implementation. The default version-two corpus contains 14 cases covering:

- changed-solution polynomial and positive-rational `G3` repair;
- aligned, reversed, unequal, and unequal-reversed parameterizations;
- multi-span degree-five seams and a repeated interior knot;
- positive rational weights from `2^-10` through `2^10` at scales `10^-3`,
  `1`, and `10^3`;
- exact typed iteration and QR truncation.

Successful cases use a dense certifier that is independent of the solver's
residual, automatic derivatives, Jacobian, and convergence decision. Each case
is executed twice and compares deterministic output and work evidence. The
preserved version-one corpus supplies broader boundary-side coverage and the
experimental `G4` reachability cases.

The STEP evidence remains outside `monstertruck-geometry`. It imports the
checked-in two-face degree-five fixture, applies a nonzero `G3` repair on a
full-side seam, certifies the result, tessellates it, exports it, re-imports it,
and certifies it again. A partial trim returns the typed
`TrimmedBoundary` reason before numerical work and leaves the shell unchanged.

## Claim boundary

| Claim | Current disposition |
| --- | --- |
| Checked continuity foundation through `G4` vocabulary | Merged upstream in `609c1b5a`. |
| Concrete B-spline and NURBS capability inspection | Merged upstream as `06201787` from reviewed head `bf4cd9c5`; all five hosted checks reported successful for the reviewed head before merge. |
| Full-boundary polynomial and positive-rational repair through production `G3` | Implemented and locally evidenced on `47e837ca`. |
| Bounded failure with typed truncation and no partial solved surface | Implemented in the local layer-2 candidate and exercised by the corpus. |
| Imported STEP full-side `G3` repair and round trip | Exercised by the current headless example. |
| Arbitrary trimmed-seam solving | Unsupported; the current path returns a typed refusal before solver work. |
| Production `G4` | Not claimed. The public path and its evidence remain experimental. |
| Upstream acceptance of capability inspection | Established by merged PR 19 at `06201787`. |
| Upstream acceptance of transition and solver APIs | Not claimed. A follow-on proposal is still required. |

## MT-402 boundary

The custom MT-402 modeling snapshot harness is superseded by the existing
topology-owned foundation in `TRACKING-SCOPE.md`. Its parallel projections,
result state, and mutable-state model are not part of the current plan. Archive
commit `568c3c4f` preserves useful edge-identity and NaN-payload probes for
possible focused topology tests after the downstream shape is approved.

Tracking, persistence, contracts, replay, and the later MT-403--MT-405 failure
program remain deferred until the upstream solver shape is accepted.

## Open audit work

1. Reconcile merged PR 19 revision `06201787` through the local proving grounds.
2. Reconcile the merged capability slice through `master`, then `dev`.
3. Propose transition and solver APIs without duplicating the capability slice.
4. Recut the approved follow-on on the then-current `upstream/master`.
5. Re-run the production `G3`, experimental `G4`, and imported STEP evidence on
   that exact contribution revision.
6. Broaden imported-model provenance and conditioning evidence without
   extending the supported seam class.
