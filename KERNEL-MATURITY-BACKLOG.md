# Kernel Maturity Backlog

This backlog translates the continuation program into bounded epics and
reviewable stories. Its validated continuity baseline starts at integration tip
`4054cc81`; the current fork stack carries that work through merge `2cc28fed`
and the upstream-shape reconciliation merges ending at `86a71509`. It preserves
the evidence vocabulary defined in
[`PHASE-5-UPSTREAM-READINESS.md`](PHASE-5-UPSTREAM-READINESS.md).

The backlog is planning input, not a production-maturity claim. A story can
promote only the capability exercised by its recorded evidence. Broad kernel
claims remain out of scope until the real-model maturity epic supplies them.

## Current baseline

- The exact hosted continuity baseline remains `4054cc81`; its evidence is
  historical and is not silently promoted to later trees.
- Fork pull requests 4 through 7 are merged into
  `agent/phase-3-variational-continuity`, whose current tip is `2cc28fed`.
- Draft [fork pull request 8](https://github.com/KTheMan/monstertruck/pull/8)
  preserves exact ancestry for upstream pull request 6 at `c19918e9` and its
  stacked pull request 7 at `558b573e`. Reconciliation merges `f4cbe771` and
  `86a71509` retain the upstream shared-crate split, fallible edge shape, and
  postfix UV naming while preserving fork tracking semantics. This is a
  preparation branch, not a bounded upstream contribution slice.
- Fork pull request 4's merged description is reconciled with its final
  five-workflow G1--G3, persistence, tessellation, and CI evidence.
- The exact combined merge tip passed all seven hosted checks in workflow run
  `30861560069` on its second attempt. The first attempt exposed the existing
  randomized `monstertruck-core` Newton property-test flake; the unchanged
  rerun passed.
- Imported G1--G3 repair, STEP persistence, and triangle validity are
  validated on five provenance-clean generated fixtures.
- Unequal/reversed nonzero-offset G2 is covered. Unequal-parameter
  nonzero-offset G3 remains open.
- Committed schema-two evidence at `95e6b9ee` shows replay graph rejection,
  duplicate-ID preparation contradiction, bounded sequential nonconvergence,
  and late failure after staged success are deterministic and transactional.
  Complete geometry, tracking-session, contract-input, transition, and report
  comparisons pass. The version-one tracked-modeling matrix inventories all 18
  public wrappers and their fallible stages. Complete snapshots and failure
  injection remain open.
- Wasm runtime behavior, host-level deserialization allocation limits, and
  separated workflow performance remain unsubstantiated.
- The upstream design discussion remains open without maintainer direction.

## Priority model

| Priority | Meaning |
| --- | --- |
| P0 | Required to trust the current integration boundary. |
| P1 | Required before runtime, persistence, or performance maturity claims. |
| P2 | Broader kernel maturity and upstream contribution preparation. |

## Tracking conventions

**Last reviewed:** 2026-08-03.

The checklist below is the authoritative story tracker. Use it together with
the detailed acceptance criteria later in this document.

- An unchecked box means the story is not complete. Its working state can be
  `Not started`, `In progress`, or `Blocked` in the progress log.
- A checked box means every acceptance criterion is satisfied and the progress
  log links or identifies reproducible evidence.
- Do not check a story merely because its implementation exists or one test
  passes.
- Check an epic only after all its stories are checked.
- Record blockers with the exact failing command, error, owner, and unblock
  condition.
- Record pull requests, commits, hosted runs, receipts, and validation
  artifacts in the progress log rather than embedding transient status in the
  story text.
- Update the review date and summary counts whenever story state changes.

| State | Count |
| --- | ---: |
| Complete | 9 |
| In progress | 0 |
| Blocked | 0 |
| Not started | 29 |

## Progress checklist

- [x] Epic 1 -- Integration baseline closure.
  - [x] MT-101 -- Verify the combined integration tip.
  - [x] MT-102 -- Reconcile progress documentation.
  - [x] MT-103 -- Record unresolved API decisions.
- [ ] Epic 2 -- Continuity evidence closure.
  - [ ] MT-201 -- Add unequal-parameter nonzero-offset G3 evidence.
  - [ ] MT-202 -- Extend scale and conditioning coverage.
  - [ ] MT-203 -- Refresh external receipts.
- [x] Epic 3 -- Replay batch failure safety.
  - [x] MT-301 -- Cover same-surface conflicts.
  - [x] MT-302 -- Cover coupled optimized surfaces.
  - [x] MT-303 -- Cover dependency cycles.
  - [x] MT-304 -- Cover contradictory and unsatisfiable batches.
  - [x] MT-305 -- Cover late solve failure.
- [ ] Epic 4 -- Tracked modeling atomicity.
  - [x] MT-401 -- Inventory tracked operations.
  - [ ] MT-402 -- Establish complete state snapshots.
  - [ ] MT-403 -- Cover construction and sweep failures.
  - [ ] MT-404 -- Cover boolean and cut failures.
  - [ ] MT-405 -- Cover fillet failure.
- [ ] Epic 5 -- Bounded persistence and deserialization.
  - [ ] MT-501 -- Define persisted-data budgets.
  - [ ] MT-502 -- Implement bounded decoding.
  - [ ] MT-503 -- Harden malformed trimmed topology.
  - [ ] MT-504 -- Add schema evolution evidence.
- [ ] Epic 6 -- Wasm runtime substantiation.
  - [ ] MT-601 -- Select the supported Wasm host.
  - [ ] MT-602 -- Run a representative solve.
  - [ ] MT-603 -- Run replay and bounded-failure cases.
  - [ ] MT-604 -- Add a Wasm runtime gate.
- [ ] Epic 7 -- Performance and resource evidence.
  - [ ] MT-701 -- Add phase-level measurements.
  - [ ] MT-702 -- Define the benchmark corpus.
  - [ ] MT-703 -- Establish regression thresholds.
  - [ ] MT-704 -- Separate native and Wasm results.
- [ ] Epic 8 -- Kernel-wide real-model maturity.
  - [ ] MT-801 -- Establish a provenance-clean real-model corpus.
  - [ ] MT-802 -- Audit intersections.
  - [ ] MT-803 -- Audit topology and healing.
  - [ ] MT-804 -- Broaden boolean evidence.
  - [ ] MT-805 -- Broaden meshing evidence.
- [ ] Epic 9 -- Upstream contribution preparation.
  - [ ] MT-901 -- Prepare the continuity-primitives slice.
  - [ ] MT-902 -- Prepare the bounded-solver slice.
  - [ ] MT-903 -- Prepare the tracking slice.
  - [ ] MT-904 -- Prepare the replay slice.
  - [ ] MT-905 -- Prepare optional modeling and provenance slices.

## Epic 1 -- Integration baseline closure

**Priority:** P0.

**Outcome:** Establish one authoritative, reproducibly green integration
commit.

### MT-101 -- Verify the combined integration tip

- Run all seven hosted gates against the exact combined integration tree.
- Confirm that the tracked-modeling and tessellation-provenance changes work
  together.
- Record workflow URLs, commit SHA, toolchain, and outcomes.

**Acceptance criteria:** Every gate passes on the exact combined tree. The
receipt identifies the tested tree rather than only the individual pull
request heads.

### MT-102 -- Reconcile progress documentation

- Update the evidence matrix to include the gates closed in pull request 4.
- Remove stale deferred claims from milestone and pull request narratives.
- Preserve the six-level evidence vocabulary.

**Acceptance criteria:** Code, receipts, milestone narrative, and evidence
matrix agree about supported and deferred capabilities.

### MT-103 -- Record unresolved API decisions

Record the owner, affected contribution slice, and decision state for:

- sealing topology-tracking traits;
- `#[non_exhaustive]` on extensible public enums;
- public error granularity;
- solver construction and `Default`;
- contract-identifier string conversions;
- exposed `SmallVec` and QR implementation types;
- the experimental G4 policy;
- tracking-session and tracked-envelope schema evolution.

**Acceptance criteria:** Every item is either resolved from evidence or marked
as requiring maintainer direction. No decision is silently embedded in an
unrelated slice.

## Epic 2 -- Continuity evidence closure

**Priority:** P0.

**Outcome:** Close the remaining mathematical and imported-workflow gaps for
supported full-boundary G1--G3 repair.

### MT-201 -- Add unequal-parameter nonzero-offset G3 evidence

- Create a provenance-clean unequal-domain G3 STEP fixture.
- Apply a nonzero deterministic perturbation.
- Solve and independently certify it before export and after re-import.
- Record topology, bounding-box, triangle-validity, and digest evidence.

**Acceptance criteria:** Independent G0--G3 normalized residuals pass their
recorded tolerances before export and after re-import. The fixture and output
remain finite, topologically stable, and mesh-valid.

### MT-202 -- Extend scale and conditioning coverage

- Combine extreme model scales with repeated knots and extreme positive
  rational weights.
- Exercise near-degenerate but valid full boundaries.
- Record deterministic accepted results and typed bounded rejections.

**Acceptance criteria:** Each case either passes independent certification or
returns a deterministic typed failure within configured resource limits.

### MT-203 -- Refresh external receipts

- Validate new fixtures and repaired outputs with the pinned independent CAD
  implementation.
- Keep external B-rep validity separate from mathematical continuity.

**Acceptance criteria:** Versioned receipts record implementation versions,
input and output digests, topology counts, bounding boxes, and B-rep validity.
The documentation does not promote external continuity without an independent
continuity check.

## Epic 3 -- Replay batch failure safety

**Priority:** P0.

**Outcome:** Prove that every rejected or failed continuity batch is
deterministic and transactional.

### MT-301 -- Cover same-surface conflicts

- Exercise the [`SameSurfaceContract`](monstertruck-geometry/src/nurbs/continuity_solver/replay/errors.rs)
  path.

**Acceptance criteria:** The batch returns the expected typed error before
solving and leaves caller-visible geometry and tracking state unchanged.

### MT-302 -- Cover coupled optimized surfaces

- Exercise independent contracts that attempt to optimize the same surface.

**Acceptance criteria:** The conflict is identified canonically, the error is
deterministic, and the entire batch rolls back.

### MT-303 -- Cover dependency cycles

- Exercise two-contract and longer directed dependency cycles.

**Acceptance criteria:** The error reports a stable contract ordering, no
solver is executed, and caller-visible state remains unchanged.

### MT-304 -- Cover contradictory and unsatisfiable batches

- Construct independently valid contracts that cannot all be satisfied.
- Separate preparation-time contradiction from solve-time unsatisfiability.

**Acceptance criteria:** Each path returns a deterministic typed failure and
preserves complete pre-batch state.

### MT-305 -- Cover late solve failure

- Fail a batch after one or more staged solves have succeeded.
- Compare geometry maps, accepted transitions, reports, and caller inputs.

**Acceptance criteria:** No staged result escapes. Repeated executions produce
the same error and identical pre/post state.

## Epic 4 -- Tracked modeling atomicity

**Priority:** P0.

**Outcome:** Substantiate failure safety for every tracked modeling wrapper.

### MT-401 -- Inventory tracked operations

- Enumerate transforms, sweeps, extrusion, revolve, cone, booleans,
  half-space clipping, plane cuts, and fillets.
- Identify every fallible stage and caller-visible state component.

**Acceptance criteria:** A wrapper/failure-point matrix covers every public
tracked modeling entry point.

### MT-402 -- Establish complete state snapshots

- Canonically capture topology identities, tracking bindings, generation,
  serial state, semantic references, lineage, and geometry signatures.

**Acceptance criteria:** Snapshot comparison detects every externally visible
mutation and is deterministic on repeated runs.

### MT-403 -- Cover construction and sweep failures

- Inject failures into sweep, extrusion, revolve, and cone operations after
  staged work begins.

**Acceptance criteria:** Every wrapper returns a typed error and preserves the
complete input topology and tracking session.

### MT-404 -- Cover boolean and cut failures

- Cover intersection, union, difference, symmetric difference, half-space
  clipping, and plane cuts.

**Acceptance criteria:** Every wrapper has a late-failure case with complete
rollback evidence.

### MT-405 -- Cover fillet failure

- Trigger a failure after fillet topology staging begins.

**Acceptance criteria:** The shell, tracking session, identities, and lineage
match their pre-operation snapshots.

Existing test files and expectations remain unchanged. New evidence must use
permitted validation harnesses and versioned artifacts.

## Epic 5 -- Bounded persistence and deserialization

**Priority:** P1.

**Outcome:** Reject oversized or malformed persisted data before
caller-controlled collections are fully materialized.

### MT-501 -- Define persisted-data budgets

- Establish finite limits for vertices, edges, faces, wires, lineage events,
  bindings, and nested references.
- Define checked arithmetic for aggregate limits.

**Acceptance criteria:** Defaults are finite, documented, and configurable by
trusted hosts without weakening the default bounded path.

### MT-502 -- Implement bounded decoding

- Enforce limits while decoding rather than only after allocation.
- Preserve ordinary upstream topology serialization.
- Keep tracked persistence explicit and versioned.

**Acceptance criteria:** Oversized declared collections return typed errors
before full allocation. Valid existing ordinary topology payloads remain
compatible.

### MT-503 -- Harden malformed trimmed topology

- Cover invalid references, empty faces, excessive nesting, cardinality
  mismatches, and malformed tracking envelopes.

**Acceptance criteria:** Every case returns a bounded typed error without a
panic, partial state, or uncontrolled allocation.

### MT-504 -- Add schema evolution evidence

- Validate supported versions and reject unknown or incompatible versions.
- Record fixtures for each supported transition.

**Acceptance criteria:** A version compatibility matrix and reproducible
fixtures substantiate the supported persistence policy.

## Epic 6 -- Wasm runtime substantiation

**Priority:** P1.

**Outcome:** Promote Wasm from compile-only evidence to supported runtime
evidence.

### MT-601 -- Select the supported Wasm host

- Define the browser or command-line runtime, versions, and execution method.

**Acceptance criteria:** A documented non-interactive command reproduces the
runtime environment.

### MT-602 -- Run a representative solve

- Execute a bounded G1 or G2 solve in the supported runtime.

**Acceptance criteria:** Independent certification meets the recorded
tolerances and the receipt identifies the runtime version.

### MT-603 -- Run replay and bounded-failure cases

- Execute generation replay and a resource-limit failure in the supported
  runtime.

**Acceptance criteria:** Replay is deterministic and the oversized case
returns the expected typed bounded failure.

### MT-604 -- Add a Wasm runtime gate

- Preserve the existing compile gates.
- Add supported-runtime execution as an independent hosted gate.

**Acceptance criteria:** The hosted receipt covers solve, replay, and bounded
failure without treating compilation as runtime evidence.

## Epic 7 -- Performance and resource evidence

**Priority:** P1.

**Outcome:** Produce reproducible performance evidence without conflating
solver and workflow costs.

### MT-701 -- Add phase-level measurements

- Measure import, perturbation, solve, certification, tessellation, export,
  and re-import independently.

**Acceptance criteria:** Receipts report each phase separately together with
iterations, accepted steps, and bounded work dimensions.

### MT-702 -- Define the benchmark corpus

- Include small interactive, representative medium, stress, and bounded
  failure cases.

**Acceptance criteria:** Inputs, hardware metadata, toolchain, configuration,
and digests are recorded.

### MT-703 -- Establish regression thresholds

- Track wall time, iteration count, accepted and rejected steps, variables,
  residuals, triangle count, and peak memory where supported.

**Acceptance criteria:** Thresholds detect material regressions without
requiring bitwise timing equality.

### MT-704 -- Separate native and Wasm results

- Record native parallel and Wasm behavior independently.

**Acceptance criteria:** Reports state platform and execution model explicitly
and make no unsupported cross-platform equality claim.

## Epic 8 -- Kernel-wide real-model maturity

**Priority:** P2.

**Outcome:** Extend evidence beyond the continuity feature family into broader
CAD-kernel behavior.

### MT-801 -- Establish a provenance-clean real-model corpus

- Cover analytic, spline, trimmed, periodic, assembly, malformed, and large
  models.
- Record units, tolerances, topology, surface classes, bounding boxes, and
  digests.

**Acceptance criteria:** A versioned manifest records provenance and licenses
for every artifact.

### MT-802 -- Audit intersections

- Cover curve/curve, curve/surface, and surface/surface intersections across
  seams, singularities, degeneracies, and scale extremes.

**Acceptance criteria:** Every case has a typed bounded outcome and an
independent geometric check where practical.

### MT-803 -- Audit topology and healing

- Validate orientation, manifoldness, shell closure, edge sharing, parameter
  curves, sewing, and Euler consistency.
- Include the existing `abc-0006.step` healing failure.

**Acceptance criteria:** Every model has an explicit validity result, failure
classification, and diagnostic artifact where applicable.

### MT-804 -- Broaden boolean evidence

- Exercise curved, touching, coincident, narrow-feature, and multi-shell
  solids.

**Acceptance criteria:** Receipts compare topology, volume, bounding boxes,
meshes, and independent B-rep validity.

### MT-805 -- Broaden meshing evidence

- Validate trim containment, watertightness, seam behavior, triangle quality,
  topology provenance, tolerance scaling, and determinism.

**Acceptance criteria:** Every model has reproducible quality metrics and a
bounded typed failure where valid meshing is not possible.

## Epic 9 -- Upstream contribution preparation

**Priority:** P2.

**Outcome:** Convert the integration branch into reviewable,
upstream-compatible contribution slices.

### MT-901 -- Prepare the continuity-primitives slice

- Isolate continuity orders, surface capability, jets, documentation, and
  executable examples.

### MT-902 -- Prepare the bounded-solver slice

- Isolate the direct solver, transition semantics, resource budgets, and
  procedural/imported evidence.

### MT-903 -- Prepare the tracking slice

- Isolate generic identity, topology-specific semantics, explicit
  persistence, and lineage.

### MT-904 -- Prepare the replay slice

- Isolate contracts, batch execution, transactionality, and failure evidence.

### MT-905 -- Prepare optional modeling and provenance slices

- Keep tracked modeling and tessellation provenance separate from the
  continuity proposal.

**Acceptance criteria for every slice:**

- focused conventional commits;
- warning-free public documentation and examples;
- reproducible validation commands and evidence artifacts;
- explicit limitations and deferred work;
- no upstream pull request opened or marked ready without explicit
  authorization.

## Dependency sequence

1. Complete Epic 1 before treating the integration branch as the evidence
   baseline.
2. Run Epics 2 and 3 from that baseline.
3. Complete Epic 3 before Epic 4 expands failure-safety claims.
4. Complete Epic 5 before accepting persisted input from untrusted hosts.
5. Complete Epic 6 before making Wasm runtime claims.
6. Complete Epic 7 before making latency or throughput claims.
7. Use Epic 8 to determine broader kernel priorities from real-model evidence.
8. Prepare Epic 9 slices only after their owning evidence gates close or are
   explicitly documented as deferred.

Epics 1 and 3 are complete against the certified integration tip and local
schema-two replay evidence. MT-401 inventories the tracked-modeling failure
surface. The next dependency-ready milestone is MT-402, which defines complete
state snapshots before executable failure injection begins.

## Progress log

Add one row whenever a story enters `In progress`, becomes `Blocked`, or is
completed. Keep entries concise and put detailed receipts in versioned
artifacts.

| Date | Story | State | Tracking and evidence | Next action or unblock condition |
| --- | --- | --- | --- | --- |
| 2026-08-03 | Backlog setup | Complete | Nine epics and 38 stories defined; reusable continuation prompt added. | Begin MT-101 and MT-301 from the integration tip. |
| 2026-08-03 | MT-101, MT-301, MT-302, MT-303 | In progress | Exact-tip hosted verification and replay graph-rejection evidence selected at `4054cc81`; pull request 6 CI covered the different `35c4bbef` tree. | Run all seven hosted gates on `4054cc81`, then add deterministic transactional validation for same-surface, coupled-writer, and dependency-cycle rejection. |
| 2026-08-03 | MT-101 | Complete | Exact `4054cc81` tree passed all seven Ubuntu gates in [workflow run `30861560069`, attempt 2](https://github.com/KTheMan/monstertruck/actions/runs/30861560069/attempts/2). Stable was `rustc 1.97.1 (8bab26f4f 2026-07-14)`; formatting used `rustc 1.99.0-nightly (11177f223 2026-08-02)`. Attempt 1 failed the randomized `monstertruck-core/tests/newton.rs::test_newton1` after 117 successful cases; the unchanged CPU-job rerun passed. Keep the flaky gate visible while reconciling Epic 1. | Reconcile milestone documentation and the flaky-gate observation in MT-102. |
| 2026-08-03 | MT-301, MT-302, MT-303 | Complete | `cargo run -p monstertruck-geometry --example continuity-replay-validation -- --base-revision 4054cc8122b4a69776664caf7eb948aedfaaa906 --harness-sha256 347d13b7d60deea732ed7a5c9f21e2a8013d950592dc252a3b200646ee0fdf28 --toolchain <rustc-vv> --receipt validation/continuity/replay-batch-v1.json` passed twice per case on Windows/MSVC with `ContinuitySolverConfig::default()`. An acyclic zero-weight control reaches `Solve(NonPositiveWeight)`; the graph cases instead return canonical typed preflight errors and preserve complete geometry/session snapshots. Harness SHA-256: `347d13b7d60deea732ed7a5c9f21e2a8013d950592dc252a3b200646ee0fdf28`; receipt SHA-256: `4db93d69df4faf2c5650c6e49b4caee18f308eafc340e4ce358addb68a979522`. Focused replay tests: 6 passed; focused example Clippy and nightly rustfmt check passed. Evidence class: `Implemented`; commit `95e6b9ee` versions the reproducible receipt as `Procedurally validated` evidence. | Cover preparation-time contradictions and solve-time unsatisfiability in MT-304 after MT-102. |
| 2026-08-03 | MT-102, MT-103 | In progress | Documentation reconciliation and the eight-item unresolved API decision register selected against exact integration tip `4054cc81`. | Align milestone claims and record owner, contribution slice, and decision state for every MT-103 item without changing public APIs. |
| 2026-08-03 | MT-103 | Complete | The Phase 5 public API audit records all eight choices with current evidence, role-based upstream owner, MT-901--MT-904 contribution slice, and `Requires maintainer direction` state. Upstream issue [`virtualritz/monstertruck#4`](https://github.com/virtualritz/monstertruck/issues/4) remains open without maintainer response; no choice was silently implemented. Evidence class: `Implemented`. | Preserve the register until each owning slice receives maintainer direction. |
| 2026-08-03 | MT-102 | In progress | Local Phase 4/5 matrices now match the five schema-3 STEP receipts, exact integration CI, Wasm compile gates, and current limitations. A corrected PR 4 body is staged at `target/pr4-body-mt102.md`; its live merged description still contains the stale three-workflow, G1-only deferrals. | Obtain explicit authorization, update [fork PR 4](https://github.com/KTheMan/monstertruck/pull/4), verify the live body, then complete MT-102 and Epic 1. |
| 2026-08-03 | MT-102, Epic 1 | Complete | [Fork PR 4](https://github.com/KTheMan/monstertruck/pull/4) now matches the reviewed replacement exactly at merged head `a98cc9e2`: five positive STEP workflows, schema-3 G1--G3/persistence/tessellation evidence, final hosted run `30854797421`, and the actual remaining limitations. The stale three-workflow, G1-only deferrals are absent. At that review, the Phase 4/5 matrices used the six evidence classes and distinguished the then-uncommitted replay receipt from promoted evidence. Evidence classes: `Implemented`, `Procedurally validated`, `Imported workflow validated`, and `Externally validated` within their recorded boundaries. | Begin MT-304 and MT-305 from exact integration tip `4054cc81`. |
| 2026-08-03 | MT-304, MT-305 | In progress | Preparation-time contradiction, bounded solve-time nonconvergence, and late failure after staged success selected against exact integration tip `4054cc81`. Evidence is bounded to deterministic typed failures, complete pre/post caller-state snapshots, and non-escape of staged results. | Extend the replay validation harness and receipt, then run focused tests, formatting, and warning gates. |
| 2026-08-03 | MT-304, MT-305, Epic 3 | Complete | Schema-two public replay evidence at `validation/continuity/replay-batch-v2.json` preserves the schema-one receipt and covers two independently successful duplicate-ID contracts rejecting canonically during preparation, two independently successful one-iteration contracts returning downstream `DidNotConverge(MaximumIterations)` only when dependency-ordered together, and a late invalid-weight failure after a successful staged prefix. Every case repeats the same typed result and preserves geometry, tracking session, and contract inputs; staged surfaces, transitions, and reports compare equal across reruns and no failed batch returns a solution. This is bounded sequential nonconvergence, not global mathematical infeasibility. The receipt was reproduced byte-for-byte on Windows/MSVC. Harness SHA-256: `ff48a623bc8991c35cee0440cf3fcc329a4a5b2398b8b53eedea157246a0f896`; receipt SHA-256: `18090f1546f960a27fc741df632bfb10b6aadf2f056e04827faf40c07e22bb97`. `cargo test -p monstertruck-geometry replay` passed eight replay-filtered tests; workspace Clippy, final focused-example Clippy, and nightly formatting passed. Commit `95e6b9ee` versions the receipt as `Procedurally validated` evidence. | Begin MT-401, then use its wrapper/failure-point inventory to scope MT-402 snapshots. |
| 2026-08-03 | MT-401 | In progress | The public tracked-modeling wrapper and failure-point inventory was selected against exact integration tip `4054cc81`. Evidence is bounded to source-audited API coverage, feature gates, transaction boundaries, and caller-visible state components; no rollback behavior is promoted. | Create the versioned matrix, verify it against the feature-complete public surface, and record MT-402 snapshot requirements. |
| 2026-08-03 | MT-401 | Complete | Commit `95e6b9ee` versions `validation/tracking/tracked-modeling-wrapper-failure-matrix-v1.md`, which records all 18 unique public wrappers across the default, `solid`, and `fillet` feature surfaces, including the deprecated `cone` compatibility entry. Each row identifies source arity/mutability, raw operation, typed wrapper failures, staged and publication boundaries, caller-visible state, and its MT-402--MT-405 owner. A mechanical source-to-matrix comparison returned 18/18 with no difference. `cargo test -p monstertruck-modeling --lib --features fillet tracked::tests` passed 5 tests; feature-complete package Clippy passed with `-W warnings`. Matrix SHA-256: `290fe0569387cfcb1d3fa49189665dd819d3def6c6d6eb4ecc55fd13c28182aa`. Evidence class: `Implemented` source-audit evidence only; rollback and atomicity remain unsubstantiated. | Begin MT-402 by implementing canonical snapshots for every recorded `CV-*` component. |
| 2026-08-03 | Upstream shape reconciliation | Prepared | Draft [fork pull request 8](https://github.com/KTheMan/monstertruck/pull/8) starts from fork merge `2cc28fed`, preserves upstream pull request 6 head `c19918e9` through merge `f4cbe771`, and preserves stacked upstream pull request 7 head `558b573e` through merge `86a71509`. The reconciled tree retains the upstream crate split, Wasm dependency shape, fallible edge construction, and postfix UV names while porting tracked filleting and identity propagation. The MT-401 matrix was refreshed for the new crate paths and source hashes without changing its 18-wrapper scope or `Implemented` evidence boundary; refreshed SHA-256: `b5ac155b32b37f0750740c32ab9e32bb45b059a71ebf793a4a781b7545d1d850`. Local tracked-modeling, replay, formatting, and warning gates pass. | Keep the draft synchronized if either upstream head changes, require its hosted gates before merge, and extract future upstream slices from the eventual upstream merge base rather than this 42-commit fork stack. |

## Repository constraints

- Keep `AGENTS.md` unchanged.
- Preserve user work and unrelated dirty files.
- Do not modify existing tests or expected outputs.
- Never run with `RUST_TEST_UPDATE=1`.
- Use `cargo test` and `cargo run` for local build verification. Do not use
  local `cargo check` or `cargo build` as verification.
- Never run `cargo clean`.
- Never use `--release` without explicit authorization.
- Run formatting, relevant tests, and
  `cargo clippy --all-targets -- -W warnings` before committing.
- Do not merge, publish, release, or create an upstream pull request without
  explicit authorization.
