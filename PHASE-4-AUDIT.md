# Phase 4 -- Audit Ledger and Evidence Matrix

This ledger records the Phase 1--3 review evidence required by
[`PHASE-4-VALIDATION.md`](PHASE-4-VALIDATION.md). It separates observed
behavior from release claims and keeps incomplete checks visible.

## Review boundary

The authored Phase 1--3 review range is
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

The pre-final-review Phase 4 evidence cutoff is `68c1fa7e`. It contains
the charter, the focused repair stack from `bd29b3eb` through `404b4e33`, the
public solved-transition output in `fe3c58a8`, generation-edit replay coverage
in `5e22e3a3`, and the versioned corpus, certifier, baseline, and reviewed
receipt introduced in `4c259080`. The final PR review found and repaired
additional certificate-domain, replay-output, and evidence-schema issues; the
replacement receipt and final gate reruns are recorded below.

The published review stack, observed on 2026-07-30, is:

| Pull request | Base | Head | Contents | Current metadata |
| --- | --- | --- | --- | --- |
| [`KTheMan/monstertruck#1`](https://github.com/KTheMan/monstertruck/pull/1) | `agent/phase-3-variational-continuity` | `agent/phase-4-validation-audit` | Charter and acceptance program at `285e1550`. | Merged as `02b69d7a` on 2026-07-30. |
| [`KTheMan/monstertruck#2`](https://github.com/KTheMan/monstertruck/pull/2) | `agent/phase-3-variational-continuity` | `agent/phase-4-audit-repairs` | Audit ledger and focused repairs through `33eff4da`. | Merged as `25e13e6e` on 2026-07-30. |
| [`KTheMan/monstertruck#3`](https://github.com/KTheMan/monstertruck/pull/3) | `agent/phase-3-variational-continuity` | `agent/phase-4-real-world-validation` | Solved-transition API, edit replay, corpus, certifier, receipt, and claim promotion. | Open. The fork workflow is active; the final review push runs the hosted gates before the pull request leaves draft status. |

These pull requests were intentionally stacked. Merging pull requests 1 and 2
retargeted pull request 3 to the Phase 3 branch while preserving its five
authored commits as the remaining review diff. The resulting base tree is
byte-identical to the focused-repair tip `33eff4da`.

## Review scope and manual deferral

This ledger is an engineering architecture, numerical, and robustness review.
It is not a formal security audit, threat model, exploitability assessment, or
third-party dependency review.

An optional automated security-classification workflow was stopped after the
execution platform rejected a delegated dynamic crash-reproduction request.
The request was not retried or reframed. No unfinished workflow result is
treated as a vulnerability finding, severity rating, or security clearance.

The completed code and history review remains engineering input. The following
checks are deferred to a human maintainer:

- confirm that every supported empty B-spline and NURBS control-net entry point
  now returns a typed unsupported result rather than indexing an absent row;
- enumerate remaining panic and unchecked-index sinks across legacy, tracked,
  and trimmed topology serialization routes;
- review allocation limits that must be enforced before deserialization
  allocates caller-declared vectors;
- perform any threat, exploitability, or severity assessment outside this
  engineering ledger.

This deferral does not suppress independently observed engineering defects.
Empty-net panics, unchecked serialized indices, unbounded solver dimensions,
partial mutation, and tracking-identity correspondence are tracked below as
robustness issues. No security claim is inferred from this ledger.

## Evidence matrix

| Area | State | Evidence | Required promotion |
| --- | --- | --- | --- |
| `G0`--`G3` exact continuity path | Realistically validated | The 18-case CAD-like procedural corpus exercises explicit `G0`, `G1`, `G2`, and `G3` layouts. The public certifier checks every mixed derivative through the requested order at endpoints, mapped knot boundaries from both surfaces, and span-interior samples. | Imported production geometry remains a separate evidence class. |
| Nonzero-offset repair | Realistically validated through `G1` | The curved-seam quintic offset case converges in two accepted steps and passes external `G0`/`G1` certification. | Add independently certified nonzero-offset `G2` and `G3` repairs. |
| Rational derivative and reparameterization formulas | Analytically verified | Independent formula review traced homogeneous jets, quotient conversion, and transition-field composition through order four. The external corpus supplies supporting residual evidence for positive nonuniform rational weights. | Imported rational production surfaces remain pending. |
| Multi-span quintic polynomial and rational path | Realistically validated | Exact polynomial and nonuniform rational cases contain multiple cross and seam spans. G3 residuals pass the independent public-evaluation certifier. | Add repeated-knot and extreme-positive-weight cases. |
| Unequal and reversed parameterization | Realistically validated for the recorded cases | The corpus covers nonlinear unequal `G1`, rational unequal/reversed `G1`, reversed `G3`, and unequal cross-domain `G3`; mapped boundaries from both surfaces are explicit certification samples. | Curved unequal-domain nonzero-offset `G2`/`G3` repair remains pending. |
| Tracking initialization and compressed restoration | Implemented | Focused regressions cover failed initializer rollback, fallible edge-map identity, kind mismatch rollback, serialized index correspondence, checked indices, and empty-face rejection. | Oversized payload limits and same-kind semantic-reference correspondence. |
| Tracked cuts and modeling-session atomicity | Implemented | The implementations stage session state through lineage commit; existing success-path cut and modeling tests pass. | Failure injected after partial staged work for every cut and modeling wrapper, with complete pre/post comparison. |
| Continuity replay after edits and late failure | Realistically validated for the synthetic chain | `5e22e3a3` advances and rebinds a tracking generation, replays a meaningful upstream repair, rejects stale IDs, orders lexically conflicting contracts by dependency, preserves each solved local transition, and proves unchanged caller input after a downstream failure. | Imported modeling-session edit histories and coupled systems remain pending. |
| Solver allocation bounds | Implemented | `bd29b3eb` adds checked budgets and seven focused resource/capability tests cover representative dimensions and typed limit failures. | Corpus measurements at accepted boundaries and host-level deserialization budgets. |
| Serialized topology robustness | Implemented | `06fedf9c` checks referenced topology indices, face cardinality, tracking dimensions, and index-to-ID correspondence. | Malformed trimmed-route coverage and allocation limits before `Vec` deserialization. |
| Deterministic output | Realistically validated on Windows/MSVC | Each case matches an immediate rerun and a separate-process v4 baseline verification. The raw reviewed receipt preserves geometry/report/dense/error evidence and work counters. | Cross-platform digest equivalence is not claimed. |
| Wasm compilation | Implemented | The committed receipt for `404b4e33` records successful geometry-library test-module compilation for `wasm32-unknown-unknown`. | Keep the compile gate in CI. |
| Wasm runtime usability | Not yet substantiated | No browser or Wasm runtime execution receipt is committed. | Execute representative solve, replay, and bounded-failure cases in a supported Wasm host. |
| Experimental `G4` reachability | Realistically validated as experimental | A curved-seam, multi-span, nonuniform rational quintic case certifies through order four with maximum normalized order-four residual `3.874577898e-3` using domain-valid one-sided cross stencils. | This does not promote production `G4`; nonzero-offset and imported evidence remain pending. |

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
- **Disposition:** Implemented and focused-regression-tested in `bd29b3eb`.
  The default solver now has finite
  limits for control points, spans, samples, variables, residuals, Jacobian
  elements, augmented QR elements, and iterations. Checked arithmetic runs
  before dense allocation; an explicit unbounded budget remains available to
  trusted hosts that enforce an equivalent external policy. Realistic
  accepted-boundary measurements remain pending.

### `P4-A002` -- Solver iteration count is caller-unbounded

- **Impact:** A caller can request excessive repeated dense evaluations even
  when each individual problem fits the allocation budget.
- **Evidence:** `max_iterations` is required to be positive but has no upper
  bound.
- **Disposition:** Implemented and focused-regression-tested in `bd29b3eb`;
  solver construction rejects an iteration ceiling above its resource budget.

### `P4-A003` -- Failed tracking operations can partially mutate state

- **Impact:** A late binding, lineage, stale-ID, or topology error can leave
  consumed serials, semantic bindings, or topology IDs from an operation that
  returned `Err`. Retry output can then differ.
- **Evidence:** tracking assignment allocates and binds incrementally.
  Modeling wrappers pass the caller's mutable session directly through a
  fallible topology operation.
- **Disposition:** Implemented in `99fb3835`. Initializers clone and stage
  topology plus session state; tracked cuts and modeling wrappers stage the
  complete session through lineage recording and commit only on success.
  Failed initializer rollback has a focused regression. Equivalent injected
  late-failure coverage for every cut and modeling wrapper remains pending, so
  the broader end-to-end atomicity claim is not yet substantiated.

### `P4-A004` -- Tracked compressed topology can lose index correspondence

- **Impact:** Valid reordered serialized references can attach edge or vertex
  tracking IDs to a different runtime entity.
- **Evidence:** extraction rebuilds edge and vertex indices by first traversal
  encounter, then indexes the parallel tracking vectors with the rebuilt
  order instead of the serialized array indices.
- **Disposition:** Implemented and focused-regression-tested in `06fedf9c`.
  Tracking IDs are applied to vertices and edges while those arrays are
  reconstructed, before face traversal can reorder their uses.

### `P4-A005` -- Tracked deserialization delegates unchecked indices

- **Impact:** Out-of-range edge endpoint or face edge-use indices can panic;
  deserializing an empty tracked face can reach `pop().unwrap()`.
- **Evidence:** tracking dimensions are checked, but referenced topology
  indices are accessed directly. The empty-face unwrap also exists on the
  legacy route, so the root sink predates Phase 2 even though the tracked
  wrapper reaches it.
- **Disposition:** Implemented and focused-regression-tested in `06fedf9c` as
  robustness work. Compressed
  references use checked lookup and return typed extraction errors; serialized
  faces require exactly one extracted face.

### `P4-A006` -- Existing tracking IDs are not checked against semantic kind

- **Impact:** Corrupted persisted state can present a current-generation ID
  whose binding kind or semantic reference does not match the topology entity
  carrying it.
- **Evidence:** initialization calls `validate_current`, which checks session,
  generation, and serial. It does not compare the existing binding with the
  visited vertex, edge, or face kind.
- **Disposition:** Kind validation is implemented and
  focused-regression-tested in `48da2bd1`. Existing bound IDs must match the
  visited topology kind, and failure remains transactional. Same-kind semantic
  reference correspondence cannot be inferred from the current topology wire
  format, so that narrower integrity claim remains not yet substantiated.

### `P4-A007` -- Empty continuity capability inputs can panic

- **Impact:** capability queries index the first control-net row before
  proving that it exists.
- **Evidence:** both B-spline and NURBS capability constructors use
  `control_points()[0]`.
- **Disposition:** Implemented and focused-regression-tested in `552177b8`;
  empty control nets report `Unsupported` without indexing a missing row.

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
- **Disposition:** The public-evaluation certifier was introduced in
  `4c259080` and corrected during final pull request review. It uses independent
  finite differences at endpoints, mapped knot boundaries from both surfaces,
  and span-distributed interior samples. Separate one-sided cross stencils stay
  inside each surface domain. It records absolute and scale-normalized finite
  residuals and does not describe the finite set as a global proof.

### `P4-A010` -- Diagnostic rank naming is ambiguous

- **Impact:** consumers may interpret the reported rank as the raw continuity
  Jacobian rank, while it is derived from the damped augmented system.
- **Disposition:** Fixed in the Phase 4 validation branch. The public getter
  identifies the quantity as the rank of the most recently solved damped
  augmented system and states that zero means certification completed before
  any linear solve. Raw dense certification remains separate.

### `P4-A011` -- Dense cross stencils extrapolated outside both surface domains

- **Impact:** the original certificate evaluated half of each centered
  cross-seam stencil outside the corresponding patch. Its residuals therefore
  depended on undocumented B-spline extrapolation rather than only valid
  boundary-domain evaluations.
- **Disposition:** Fixed in the final pull request 3 review. The certifier now
  generates separate one-sided common-coordinate stencils: nonpositive
  coordinates for the first chart and nonnegative coordinates for the second.
  Each side has independently generated finite-difference weights. All case
  evidence and the digest schema were regenerated as v4.

### `P4-A012` -- Contract replay discarded accepted coordinate transitions

- **Impact:** direct solves exposed their local transition, but replay retained
  only the contract ID and report. A replay consumer could not reproduce or
  independently certify the actual mapping accepted for each contract.
- **Disposition:** Fixed in the final pull request 3 review.
  `ContinuityContractSolve` now owns and exposes the accepted
  `BoundaryTransition`, and replay consumes every direct solution through the
  transition-preserving decomposition.

### `P4-A013` -- Negative evidence outcomes were double JSON-encoded

- **Impact:** outcome fields contained quoted JSON strings such as
  `"\"degenerate_boundary\""`, weakening the stability and readability of the
  evidence schema.
- **Disposition:** Fixed in the final pull request 3 review. Typed error kinds
  now provide stable unquoted names, and the raw receipt was regenerated.

## Remaining evidence gaps

### Replay and edit history

- Cover the existing same-surface, coupled-writer, and dependency-cycle error
  variants as complete batches rather than only reviewing their branches.
- Replay imported modeling-session edits with multiple successful dependent
  solves. The committed synthetic chain covers generation rebinding, changed
  geometry, dependency ordering, stale-ID rejection, and late failure.

### Unequal parameterization and continuity certification

- Add nonzero-offset unequal `G2` and `G3` repairs, repeated knots, and extreme
  positive rational weights.
- Add imported models whose NURBS surfaces, boundary selection, and
  orientation can be reproduced without proprietary or provenance ambiguity.

### Adversarial and malformed inputs

- Exercise contradictory or unsatisfiable multi-contract batches, late solver
  failure, near-degenerate boundaries, extreme but positive weights, repeated
  knots, scale extremes, and non-finite transition/configuration values.
- Add malformed trimmed-topology cases and oversized serialized vector cases.
  The current extraction repairs validate indices after deserialization; they
  do not bound allocation performed by the deserializer itself.
- Inject semantic-label collisions into tracked cuts and each modeling wrapper
  after staged work has begun, then compare the full caller-visible session.

### Determinism and realistic corpus

- The committed receipt establishes same-host Windows/MSVC determinism for
  procedural CAD-like fixtures. Cross-platform digest equality remains
  unsubstantiated because raw floating-point and native-width hashing is
  intentionally host-sensitive.
- Fixture provenance is documented at corpus level. A future imported corpus
  should add per-case source/license fields and explicit serialized knot,
  multiplicity, and weight-range metadata.
- Imported production CAD evidence remains pending manual provenance and model
  selection.

## Known repository gate blockers

The checkout and toolchain observations in this table were refreshed on
2026-07-30. Committed validation receipts retain their original commit scope.

| Gate | Current observation | Ownership |
| --- | --- | --- |
| Evidence cutoff | The public API, replay, and original corpus evidence is committed through `68c1fa7e`; the final review repair regenerates the v4 receipt and records the closing gate reruns. | Phase 4. |
| Geometry library tests | The final CPU run passes all 176 geometry library tests. Transition, replay, resource-budget, and capability suites add 12 passing focused tests. | Phase 4. |
| Focused tracking tests | The committed receipt records core tracking: 9 passed; topology tracking: 7 passed; modeling tracked wrappers: 5 passed. Transactional, compressed-index, and malformed-input integration suites add seven passing tests. | Phase 4. |
| Targeted lint | `cargo clippy -p monstertruck-geometry --all-targets --no-deps -- -W warnings` passes, including the validation example. | Phase 4. |
| `resources` | Initialized at `9bf9de14`; all previously missing named assets are present. | Resolved. |
| Workspace Clippy | `cargo clippy --all-targets -- -W warnings` passes. Cargo separately reports future-incompatibility notices in dependency versions `nom 3.2.1` and `quick-xml 0.22.0`. | Dependency maintenance, not a Phase 4 warning failure. |
| Workspace CPU tests | The `just` executable is absent. Running the exact `test-cpu` recipe's `cargo test` command directly passes, including doctests and all Phase 4 suites. | `just` installation remains an environment convenience blocker only. |
| Corpus | The corrected v4 full emit passes all 18 cases in 316.8 seconds of recorded case time. A separate process reproduces every v4 baseline digest. | Phase 4. |
| Formatting | `cargo +nightly fmt --all` established the repository's declared formatting baseline, including mechanical rewrites in Phase 1--3 files, and `cargo +nightly fmt --all -- --check` passes. | Resolved. |
| GitHub-hosted CI | The fork reports the `CI` workflow as active. The workflow checks nightly formatting, workspace Clippy, the CPU suite, meshing feature combinations, geometry-library Wasm test compilation, and a full 18-case continuity-corpus emit. The final pull request push must pass these hosted checks before leaving draft status. | Phase 4. |
| `.blueprints` | Initialization was attempted twice and the configured `https://github.com/virtualritz/blueprints.git` returned `Repository not found`. The shared baseline files remain unavailable; the supplied root repository rules governed this work. | Upstream access/setup. |
| Wasm compile | `cargo test -p monstertruck-geometry --lib --target wasm32-unknown-unknown --no-run` succeeds after the final transition/corpus work and emits the geometry test Wasm module. | Phase 4. |
| Workspace Wasm test compile | `cargo test --workspace --target wasm32-unknown-unknown --no-run` first exposed and prompted repair of a target-specific `TrimmedFace` import, then reached unrelated legacy dependencies that are not Wasm-compatible: `memchr 1.0.2` expects unavailable `libc` C symbols and `lzma-sys 0.1.20` requires native C headers. The older `just wasm-build` route uses the repository-prohibited `cargo build`, so CI retains the supported geometry test compile instead of claiming workspace-wide Wasm coverage. | Dependency and workspace policy maintenance. |
| Wasm runtime | No supported-host runtime execution receipt exists. Compile success does not establish browser/runtime usability, deterministic replay, or failure behavior. | Phase 4. |

## Delivery status

| Deliverable | Status |
| --- | --- |
| Phase 4 charter and claim register | Committed as `285e1550`; pull request [`KTheMan/monstertruck#1`](https://github.com/KTheMan/monstertruck/pull/1) merged as `02b69d7a`. |
| Audit ledger | Committed through `33eff4da`; pull request [`KTheMan/monstertruck#2`](https://github.com/KTheMan/monstertruck/pull/2) merged as `25e13e6e`. |
| Focused repair commits | `bd29b3eb`, `552177b8`, `99fb3835`, `06fedf9c`, `48da2bd1`, `b003bb47`, and `404b4e33`; all are in pull request 2. |
| Solved transition and replay evidence | `fe3c58a8` and `5e22e3a3` on the third stack layer. |
| Corpus and independent certifier | Introduced in `4c259080`; final review corrects domain sampling and regenerates the 18-case v4 baseline and raw Windows/MSVC receipt. |
| Evidence records and final claim promotion | Complete in pull request [`KTheMan/monstertruck#3`](https://github.com/KTheMan/monstertruck/pull/3), targeting the merged Phase 3 branch. |
