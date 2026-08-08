# Upstream Continuity Compliance

This document records the correction branch's compliance with
[`virtualritz/monstertruck` issue #4](https://github.com/virtualritz/monstertruck/issues/4)
and
[`virtualritz/monstertruck` PR #13](https://github.com/virtualritz/monstertruck/pull/13).
It is the branch-local source of truth for completion claims and for downstream
work removed to preserve the upstream contribution shape.

## Completion rule

Work proceeds strictly in priority order: all P1 findings, then all P2
findings, then all P3 findings. A phase is complete only after all of the
following are true:

1. The implementation and documentation match the live upstream direction.
2. Prescribed local and hosted checks pass.
3. An independent agent is instructed to analyze the phase's completion claims
   against upstream issue #4 and upstream PR #13.
4. That agent returns no unresolved finding for the phase.

A passing test suite is evidence for the behavior it asserts. It is not by
itself evidence that an API name, contribution boundary, or unasserted workflow
stage follows upstream direction.

## Independent audit baseline

The first independent audit rejected the combined P1/P2 completion claim.

| Priority | Accepted evidence | Unresolved at baseline |
| --- | --- | --- |
| P1 | The three trait-owned files match upstream PR #13, and the removed public layer-1 vocabulary is absent. | The solver still exposes an unreviewed continuity-specific bounded-work family instead of demonstrably reusing the vocabulary and refusal/meter shape named in issue #4. |
| P2 | The STEP adapter runs after import and refuses one partial linear trim with typed `TrimmedBoundary`. | The imported workflow does not independently certify the repaired result or run the promised pinned external-kernel check. Replacement, tessellation, and round-trip assertions are too weak; refusal classification and coverage have gaps. |
| P3 | The downstream branch has hosted CI. | The original P3 structure/base findings will be re-audited only after P1 and P2 pass their independent gates. |

No P1 or P2 completion claim is valid while the corresponding unresolved cell
is nonempty.

## Authoritative acceptance criteria

### P1 -- layer 1 and bounded work

- `monstertruck-traits` carries only the checked order, exact `BoundarySide`
  vocabulary, typed support report, and typed unsupported reasons accepted in
  upstream PR #13.
- Representation-specific inspection stays in `monstertruck-geometry`.
- Removed maturity, capability-level, formula, public axis, and compatibility
  vocabulary does not survive through aliases.
- Bounded solving follows the carrier, refusal, and meter shape established by
  `BudgetedDivision`, `DivisionTruncated`, and the division work meters named
  by the issue #4 maintainer response. Division-specific payload types are not
  reused for solving, but a limits-only input type is not misrepresented as the
  required budgeted outcome carrier.

## Phase records

### P1 candidate resolution

The first P1 gate found two blockers. This branch then made the following
breaking corrections:

- Removed public `InspectedSurfaceContinuityCapability`. B-spline and NURBS
  inspection now returns the trait-owned `SurfaceContinuityCapability`
  directly, and solver errors store that exact report.
- Removed public cross-degree and cross-control-row facts from capability
  diagnostics and from validation output.
- Replaced the stateful `BoundaryContinuitySolver::new_with_budget` API with
  `BoundaryContinuitySolver::solve_with_budget`.
- Renamed the input limits bag from `ContinuityWorkBudget` to
  `ContinuityLimits` so it cannot be confused with an outcome carrier.
- Added `BudgetedContinuitySolve`, which contains the solve outcome, the exact
  thread-local work delta, and an optional typed `ContinuityTruncated` refusal.
  A refusal never contains partial solved geometry. The ordinary `solve` path
  converts the default-limits outcome directly to its typed `Result`.
- After the second P1 gate rejected per-allocation limits, changed iteration,
  Jacobian, and QR limits to cumulative actual-work budgets. Checks occur
  immediately before work is charged. Zero-iteration certification therefore
  succeeds with a zero iteration limit, successful work cannot exceed its
  limit, and refusal reports the already-spent and next cumulative counts in
  the same units as the meters.
- Added process-isolated `cargo nextest` coverage proving carrier work equals
  meter deltas for successful and truncated solves, including process totals,
  high-water marks, and cumulative Jacobian/QR refusal.

Candidate verification passed `cargo +nightly fmt --all`,
`cargo clippy --all-targets -- -W warnings`, and the prescribed CPU nextest
command (`720` passed, `21` skipped). The final independent P1 gate reported:
"P1 complete. No unresolved P1 findings remain against upstream issue #4 or
PR #13." P1 is therefore closed; P2 may proceed.

### P2 -- trimmed refusal and imported evidence

- Every arbitrary trimmed seam refuses with typed `TrimmedBoundary` before the
  solver runs. Other unsupported conditions retain their own typed reason.
- The headless public-API path imports STEP, identifies an adjacent full-side
  seam, solves, independently certifies the result, replaces the dependent
  face, produces a nonempty tessellation, exports STEP, and re-imports and
  revalidates the repaired result.
- At least one exported result passes a pinned OCCT or FreeCAD command-line
  validation.
- STEP remains outside the numerical geometry crate's dependency graph.

Candidate P2 resolution makes seam construction reject a repeated face and
checks that the shared edge exists in the imported shell before deriving exact
full-side capability from both face-local edge uses. Exact means exact: all
four sides and both endpoint directions are accepted, while even a one-ULP
short line is a typed `TrimmedBoundary` refusal. Missing, curved, partial,
first-face, and second-face trims refuse transactionally before solver work;
unsupported surface representations retain `UnsupportedRepresentation`.

The headless imported workflow first certifies a conforming imported baseline,
then applies a deterministic interior control-point edit after import. The edit
preserves the shared `G0` seam but demonstrably breaks `G1` before the solver
runs. Repair proves the fixed control net remains unchanged, the edited
dependent control net changes, and all four dependent edge uses retain bound
trims. Non-shared boundary edge geometry and vertices are synchronized with the
replacement face. Trim counts and bindings must remain nonzero and exactly
equal after STEP re-import; reversed edge-use trims are serialized in the edge
leader's direction so the shared trim survives the round trip.

Deterministic public-evaluation finite differences certify `G0` and `G1` at
both endpoints and nine interior samples after repair and again after STEP
re-import. Every repaired face must tessellate to nonzero vertices and
triangles. Focused tests cover both out-of-range face positions and prove a
typed solver failure leaves the imported shell and work meters unchanged. The
exported artifact is checked in hosted CI by
`cadquery-ocp==7.9.3.1.1`, which carries OCCT 7.9.3.1, using successful STEP
transfer, non-null shape, `BRepCheck_Analyzer`, and exact face count as the
external command-line oracle.

The preceding candidate at `39ac2ad7` passed all seven hosted checks, including
the pinned OCCT continuity job. The current correction passes
`cargo +nightly fmt --all -- --check`,
`cargo clippy --all-targets -- -W warnings`, the prescribed CPU nextest command
(`729` passed, `21` skipped), all CPU-crate doctests, `cargo-rdme` 2.1.0, the
imported repair example, and the pinned local OCCT 7.9.3.1 oracle (`2` valid
faces). All seven hosted checks for implementation revision `c2c2bd7d` and
stable documentation revision `3ccefe49` also pass, including the pinned OCCT
continuity job. The final independent P2 gate reported: "P2 complete. No
unresolved P2 findings remain against upstream issue #4 or PR #13 at stable
revision `3ccefe49`." P2 is therefore closed; P3 may proceed.

The independent reviewer also confirmed that P1 remains unregressed. This P2
verdict does not pre-approve P3 structure, base, or evidence findings.

### P3 -- structure and evidence state

P3 opened only after the independent P2 gate accepted the stable P2 evidence.
The independent P3 baseline then rejected completion until the downstream
stack followed the current upstream repository shape through revision
`a60482c3` (upstream PRs #12--#18), the draft PR descriptions reported exact
hosted results, and every STEP implementation moved into `monstertruck-io`.

The P3 candidate makes these structural corrections:

- The fork's `master` and `dev` bases merge current upstream `a60482c3`
  without rewriting their downstream history. The audited P1 and P2 revisions
  remain ancestors of their current correction heads.
- Downstream PR #12 remains based on `master` and has only the layer-1 trait
  and guidance delta. Downstream PR #11 is stacked directly on PR #12 and has
  only the two-file layer-2 geometry capability delta.
- Arbitrary trimmed-seam validation is not claimed as layer-2 solver work in
  PR #11. It stays in the later STEP integration layer and refuses typed
  `TrimmedBoundary` before numerical work.
- The imported repair adapter now lives at
  `monstertruck_io::step::continuity`. Its sibling tests, external test,
  executable example, and tracked fixture all live under `monstertruck-io`.
  The deprecated `monstertruck-step` crate is restored to the exact upstream
  implementation-free re-export shim.
- The continuity fixture is a `.step` file in an enumerated, nonempty corpus,
  so the upstream moved-fixture guard fails if its directory moves or empties.
- The pinned OCCT job runs the `monstertruck-io` example. The CPU job is named
  for Nextest, and active commands, feature tables, and diagnostic guidance use
  the current I/O package vocabulary.
- The accepted P1 trait files remain byte-for-byte identical to upstream PR
  #13. The accepted P2 import, deterministic edit, typed refusal, transactional
  replacement, independent certification, tessellation, STEP round trip, and
  pinned external-kernel evidence remain required without adding an I/O
  dependency to `monstertruck-geometry`.

Implementation revision `15b28c57` passes the prescribed CPU Nextest command
(`730` passed, `21` skipped), all CPU-crate doctests,
`cargo clippy --all-targets -- -D warnings`,
`cargo +nightly fmt --all -- --check`, every workspace `cargo-rdme` check, and
`git diff --check`. The imported repair example succeeds, and its exported
two-face artifact passes the pinned local OCCT 7.9.3.1 oracle with two valid
faces. The tracked continuity fixture sweep passes with one of one fixtures.
An active-source vocabulary scan found no stale `monstertruck-step`
implementation path, `monstertruck_step` package alias, `cargo test (cpu)` job
name, or premature P3 completion claim. Remaining references to
`monstertruck-step` describe the exact deprecated shim or historical migration;
`MONSTERTRUCK_STEP_CORPUS` remains the name of the STEP-format corpus variable,
not a crate alias.

All seven hosted checks pass on exact candidate revision `1d6f0f82`:
`cargo build (wasm32)`, `cargo clippy`, `cargo fmt`,
`cargo nextest (cpu)`, `continuity validation corpus`,
`monstertruck-meshing feature subsets`, and `topology state validation`. The
continuity job runs the I/O-owned repair example and the pinned OCCT oracle.

P3 closes only when exact-head local and hosted checks pass and an independent
agent, instructed to analyze the completion claims against upstream issue #4
and upstream PR #13, reports no unresolved P3 finding. Until then this section
records a candidate, not a completion claim.

## Downstream restoration ledger

These removals are intentional. They may return only through separately
reviewed later-layer work and must not be restored as compatibility aliases on
the correction branch.

| Removed downstream item | Reason | Future home or prerequisite |
| --- | --- | --- |
| `ContinuityMaturity` and maturity evidence fields | Not part of upstream PR #13's layer-1 contract. | A separately reviewed evidence/reporting layer. |
| `ContinuityCapabilityLevel` and coarse feasibility API | Duplicated the accepted typed `SurfaceContinuitySupport` contract. | Do not restore; use the accepted typed report. |
| Public `SurfaceAxis` and degree/control-row formula constructors | Upstream keeps the shared trait layer representation-neutral. | Concrete geometry inspection only. |
| `ContinuityBudget`, `ContinuityTotals`, `ContinuityWorkTruncated`, and related aliases | Parallel bounded-work vocabulary contradicted issue #4. | Any future solver instrumentation must pass the P1 vocabulary review. |
| `InspectedSurfaceContinuityCapability` and public cross-degree/control-row diagnostics | Duplicated the accepted trait-owned capability carrier. | Keep representation facts private; do not restore a public wrapper. |
| `BoundaryContinuitySolver::new_with_budget`, its stored budget, and `budget()` | Made the input limits bag look like the required budgeted outcome API. | Downstream callers must use `solve_with_budget` and inspect `BudgetedContinuitySolve`. |
| `ContinuityWorkBudget` | Its name incorrectly implied that an input limits bag was the upstream-shaped carrier. | Use `ContinuityLimits` for input and `BudgetedContinuitySolve` for output. |
| Validation corpus maturity/capability-level schema | Depended on removed downstream-only layer-1 concepts. | A later evidence schema built on accepted public types. |
| Infallible `StepContinuitySeam::new` | It admitted a pseudo-seam that selected one face twice. | Callers must handle the typed `SameFace` result. |
| Trimmed STEP export for trim types without `Clone + Invertible` | Correct STEP `PCURVE` emission must reverse a face-local trim when its edge use opposes the shared edge leader. | Downstream trim types must implement `Clone + Invertible`; do not restore directionally invalid serialization. |
| Continuity implementation under `monstertruck-step` | Upstream PR #12 folded STEP implementation into `monstertruck-io`; the old crate is only a deprecated re-export shim. | All future STEP continuity work belongs in `monstertruck_io::step::continuity`. The shim must remain implementation-free. |
| Layer-2 claim for arbitrary trimmed-seam validation | Issue #4 places the STEP adapter after the numerical solver and requires a typed refusal there, not in the geometry contribution. | Keep full-side solving in layer 2; keep trimmed-seam validation in the later `monstertruck-io::step` integration layer. |
| Draft notes blaming STEP packaging and randomized-test failures on the fork base | Upstream PRs #12, #14, and #17 corrected the package layout, fixture coverage, and deterministic tests. Retaining those notes would describe resolved failures. | Reintroduce a failure note only with evidence from the exact current revision. |

The ledger will be extended whenever strict compliance removes more downstream
work.
