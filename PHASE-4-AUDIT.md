# Phase 4 -- Audit Ledger and Evidence Matrix

This ledger records the Phase 1--3 review evidence required by
[`PHASE-4-VALIDATION.md`](PHASE-4-VALIDATION.md). It separates observed
behavior from release claims and keeps incomplete checks visible.

## Review boundary

The reviewed change range is
`45e5f8d6db5f0c9ebcade615938f19c1d33ea1d8..08be4549506a0420085d7ce745ac7b24ac2113b1`.
The range contains the three published continuity commits:

| Phase | Commit | Review shape |
| --- | --- | --- |
| 1 | `2ddb4627` | High-order continuity foundations. |
| 2 | `9f75f131` | Tracking and continuity contracts. |
| 3 | `08be4549` | Variational continuity solver. |

The history is linear and each phase has its own branch. The Phase 2 and
Phase 3 commits are large, but rewriting published history would make existing
review links unstable. Phase 4 therefore preserves those commits and supplies
file-level review maps and focused repair commits.

## Automated-review deferral

An optional automated vulnerability-classification workflow was stopped during
candidate validation because a delegated request to construct a crash
reproducer was rejected by the execution platform's cyber-safety classifier.
Phase 4 does not attempt to work around that control.

The completed architecture and failure-safety review remains valid engineering
input. The unfinished automated stages are deferred to a human maintainer:

- dynamic crash reproduction for empty B-spline and NURBS control nets;
- formal validation and attack-path classification of the remaining automated
  candidate set;
- any exploitability or severity claim that would depend on those stages.

This deferral does not suppress the underlying engineering defects. Empty-net
panics, unchecked serialized indices, unbounded allocation dimensions, partial
mutation, and tracking-identity correspondence are tracked below and repaired
or tested as robustness issues. No security claim is inferred from this
ledger.

## Evidence matrix

| Area | State | Evidence | Required promotion |
| --- | --- | --- | --- |
| `G0`--`G3` continuity API and solver path | Implemented | Geometry library tests pass: 175 tests. Focused solver review covered all new solver modules. | Independent dense certification over the committed corpus. |
| Rational derivative and reparameterization formulas | Analytically verified | Independent review traced homogeneous jets, quotient conversion, and transition-field composition through order four. | Record formula references and corpus residuals for nonuniform weights. |
| Multi-span and unequal-parameter support | Implemented | Sampling and transition code exist; current tests exercise the mechanisms. | Multi-span quintic and nonlinear-map corpus cases. |
| Tracking and contract replay | Implemented | Core, topology, and modeling tracking suites pass. Stable IDs and contract records round-trip. | Failure-atomic replay and upstream-edit scenarios with canonical digests. |
| Failure atomicity | Not yet substantiated | Review found session/topology mutation before later fallible operations. | Stage state and commit it only after the complete operation succeeds. |
| Allocation bounds | Not yet substantiated | Per-span density and transition degree are capped, but total spans, variables, residuals, dense Jacobian work, and iteration count are not. | Checked preflight budget and rejected-boundary coverage. |
| Serialized topology robustness | Not yet substantiated | Tracking-array lengths are checked; referenced vertex/edge indices and original index-to-ID correspondence are not fully checked. | Checked extraction and adversarial round trips. |
| Deterministic output | Not yet substantiated | Traversals are ordered, but no fresh-process result digest is committed. | Repeat each corpus case and compare canonical result records. |
| Wasm compatibility | Not yet substantiated | No successful Phase 4 Wasm gate is recorded. | Run the target gate or record the exact dependency/toolchain failure. |
| Production `G4` | Not yet substantiated | Exact degree-seven planar coverage exists. Realistic curved evidence does not. | Remains experimental and outside the Phase 4 release gate. |

## Findings

### `P4-A001` -- Dense solver work has no total budget

- **Impact:** A long but valid seam can request quadratic dense automatic
  differentiation storage and larger residual/Jacobian work before an
  iteration limit can help.
- **Evidence:** `ContinuitySolverConfig::validate` caps per-span sampling and
  transition degree only. `PreparedProblem` derives variables from every
  boundary control and creates a full gradient for each variable. The
  Levenberg--Marquardt path clones dense matrices for augmentation and QR.
- **Bounded estimate:** At `G3`, transition degree three, and 1,000 along-seam
  controls, the initial variable count is 18,027. The initial gradient payload
  alone is about 2.42 GiB before residual gradients and matrix copies.
- **Disposition:** Open; Phase 4 repair required.

### `P4-A002` -- Solver iteration count is caller-unbounded

- **Impact:** A caller can request excessive repeated dense evaluations even
  when each individual problem fits the allocation budget.
- **Evidence:** `max_iterations` is required to be positive but has no upper
  bound.
- **Disposition:** Open; include iterations in the solver work budget.

### `P4-A003` -- Failed tracking operations can partially mutate state

- **Impact:** A late binding, lineage, stale-ID, or topology error can leave
  consumed serials, semantic bindings, or topology IDs from an operation that
  returned `Err`. Retry output can then differ.
- **Evidence:** tracking assignment allocates and binds incrementally.
  Modeling wrappers pass the caller's mutable session directly through a
  fallible topology operation.
- **Disposition:** Open; stage mutable session/output state and commit on
  success.

### `P4-A004` -- Tracked compressed topology can lose index correspondence

- **Impact:** Valid reordered serialized references can attach edge or vertex
  tracking IDs to a different runtime entity.
- **Evidence:** extraction rebuilds edge and vertex indices by first traversal
  encounter, then indexes the parallel tracking vectors with the rebuilt
  order instead of the serialized array indices.
- **Disposition:** Open; preserve original compressed-array correspondence.

### `P4-A005` -- Tracked deserialization delegates unchecked indices

- **Impact:** Out-of-range edge endpoint or face edge-use indices can panic;
  deserializing an empty tracked face can reach `pop().unwrap()`.
- **Evidence:** tracking dimensions are checked, but referenced topology
  indices are accessed directly. The empty-face unwrap also exists on the
  legacy route, so the root sink predates Phase 2 even though the tracked
  wrapper reaches it.
- **Disposition:** Open as robustness work; replace panics with typed
  extraction errors without claiming a newly introduced vulnerability.

### `P4-A006` -- Existing tracking IDs are not checked against semantic kind

- **Impact:** Corrupted persisted state can present a current-generation ID
  whose binding kind or semantic reference does not match the topology entity
  carrying it.
- **Evidence:** initialization calls `validate_current`, which checks session,
  generation, and serial. It does not compare the existing binding with the
  visited vertex, edge, or face kind.
- **Disposition:** Open; validate ID-to-binding kind and reference policy
  before preserving an existing ID.

### `P4-A007` -- Empty continuity capability inputs can panic

- **Impact:** capability queries index the first control-net row before
  proving that it exists.
- **Evidence:** both B-spline and NURBS capability constructors use
  `control_points()[0]`.
- **Disposition:** Open; provide a fallible query or a non-panicking
  unavailable capability result.

### `P4-A008` -- Public degree-elevation targets have no practical bound

- **Impact:** a small curve or surface plus a very large target can request
  repeated control-net growth, with multiplicative growth when both surface
  directions are elevated.
- **Evidence:** public `usize` targets drive
  `(current_degree..target_degree)` directly.
- **Disposition:** Accepted as a general geometry API limitation for this
  Phase 4 stack unless a continuity/replay route exposes it. Documented for a
  later fallible degree-elevation API; not a solver release claim.

### `P4-A009` -- Convergence diagnostics are not global certification

- **Impact:** `Converged` can be misread as proof over the full seam.
- **Evidence:** the solver merges collocation residuals with a finite,
  independent midpoint-style validation set. This can veto some false
  convergence but cannot prove a continuous global maximum.
- **Disposition:** Open documentation/diagnostic repair. Phase 4 adds a dense,
  degree-aware external certifier and avoids calling finite validation a proof.

### `P4-A010` -- Diagnostic rank naming is ambiguous

- **Impact:** consumers may interpret the reported rank as the raw continuity
  Jacobian rank, while it is derived from the damped augmented system.
- **Disposition:** Open; rename or document the quantity and keep raw
  certification separate.

## Known repository gate blockers

| Gate | Current observation | Ownership |
| --- | --- | --- |
| Geometry library tests | `cargo test -p monstertruck-geometry --lib`: 175 passed. | Phase 4. |
| Focused tracking tests | Core, topology, and modeling tracking suites passed during independent review. | Phase 4. |
| Workspace Clippy | `monstertruck-traits/src/algo/curve.rs:110` fails `clippy::nonminimal_bool`; the line predates the Phase 1 base. | Workspace maintenance; Phase 4 may repair in a separate commit. |
| Formatting | The repository uses nightly-only `rustfmt.toml` options. Nightly is not installed; stable proposes unrelated workspace-wide rewrites. | Toolchain/infrastructure. |
| Workspace tests | Required `resources` submodule assets are absent, including `resources/obj/cube.obj` and `resources/texture/WoodFloor024_1K_Color.png`. | Checkout/setup. |
| Wasm | A fresh Phase 4 target gate is required. Any failure will be recorded with dependency and target details. | Phase 4. |

## Delivery status

| Deliverable | Status |
| --- | --- |
| Phase 4 charter and claim register | Committed as `285e1550`; pull request `KTheMan/monstertruck#1`. |
| Audit ledger | In progress on the stacked audit/repair branch. |
| Focused repair commits | Pending. |
| Corpus and independent certifier | Pending. |
| Evidence records and final claim promotion | Pending. |
