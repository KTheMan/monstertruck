# Phase 4 -- Engineering Validation and Release Evidence

Phase 4 is the stabilization and evidence phase for the high-order continuity
work introduced in Phases 1--3. It does not add a separate application. It
audits the kernel work already present, repairs defects found by that audit, and
produces reproducible evidence suitable for code review and release decisions.

The release target is reliable `G0`--`G3` surface continuity. `G4` remains an
opt-in experimental reach goal. Phase 4 must leave the architecture open to
further `G4` work, but production `G4` support is not an acceptance requirement.

## Evidence vocabulary

Every material claim uses one of these states:

| State | Meaning |
| --- | --- |
| Implemented | The code path exists and has focused automated coverage for the stated behavior. This does not imply realistic validation or release readiness. |
| Analytically verified | Independent mathematical checks support the result; the solver's own objective or collocation set is not the sole verifier. |
| Procedurally validated | Reproducible generated fixtures exercise the public implementation with known construction, representative edits, scale variation, and failure cases. |
| Imported workflow validated | Provenance-clean CAD assets reach the feature through public import, topology, solve, certification, tessellation, and persistence APIs. |
| Externally validated | A pinned independent CAD implementation accepts and checks the exported result. |
| Not yet substantiated | The implementation or evidence is insufficient for the claim. |

Passing a unit test is evidence of implementation, not by itself evidence of
production readiness. Generated cases can establish exact mathematical
properties, but they do not replace imported or CAD-like cases. Performance
claims require measurements from the committed harness. Terms such as
"Class-A", "production-ready", and "Wasm-ready" remain out of release notes
until their corresponding evidence is recorded.

## Scope

Phase 4 has five workstreams:

1. **Architecture and API audit.** Review the Phase 1--3 public API, ownership
   model, serialization, compatibility, tracking identities, contract replay,
   deterministic behavior, and extension points through experimental `G4`.
2. **Numerical audit.** Review derivative and reparameterization mathematics,
   rational evaluation, rank handling, convergence criteria, independent
   residual certification, conditioning, tolerances, and scale behavior.
3. **Failure-safety audit.** Review untrusted numeric input, allocation bounds,
   deserialization, panic paths, unsatisfiable constraints, transactional
   replay, and Wasm-specific constraints.
4. **Real-world validation.** Commit a deterministic corpus and runnable harness
   for generated reference cases, CAD-like surfaces, imported geometry where
   the file format preserves the needed surface data, edit/replay scenarios,
   and adversarial cases.
5. **Delivery audit.** Keep findings, fixes, evidence, commits, and pull requests
   small enough to review. Record workspace blockers instead of silently
   weakening a gate.

Phase 4 does not include a GUI, interactive modeler, general constraint system,
or an assertion that geometric continuity alone establishes visual surface
quality.

The Phase 4 audit is an engineering architecture, numerical, and robustness
review. It is not a formal security audit, threat model, exploitability
assessment, or third-party dependency review. An unfinished optional automated
security-classification workflow cannot promote or clear any claim in this
document; the deferred checks require manual maintainer review.

## Claim register

This table is deliberately conservative. Evidence is promoted only after the
named artifact and command are committed and reproducible.

| Claim | Current state | Required Phase 4 evidence |
| --- | --- | --- |
| The kernel exposes `G0`--`G3` continuity primitives and a variational repair path. | Procedurally validated for the recorded CAD-like cases | Exact `G0`--`G3`, one nonzero-offset `G1` repair, dense public-evaluation certification, and typed failures are committed. Nonzero-offset `G2`/`G3` remains pending. |
| Nonuniform rational multi-span boundaries are supported. | Procedurally validated for exact G3 | Curved-seam quintic rational cases with positive nonuniform weights pass independent certification. |
| Multi-span boundaries are supported. | Procedurally validated for the recorded knot layouts | Polynomial and rational cases contain multiple cross and seam spans; repeated-knot variants remain pending. |
| Unequal parameterizations and reversed alignment are supported. | Procedurally validated for the recorded cases | Nonlinear unequal/reversed `G1`, reversed `G3`, and unequal cross-domain `G3` pass mapped-boundary and interior certification. |
| Failed solves and replay do not partially mutate tracked geometry or tracking sessions. | Implemented with focused replay validation | Initializer and topology regressions plus a multi-contract late replay failure preserve caller-visible inputs. Equivalent injected failure for every modeling wrapper remains pending. |
| Replay after upstream edits is dependency ordered. | Procedurally validated for the synthetic chain | Changed-generation geometry, stale-ID rejection, lexically conflicting contract IDs, dependency ordering, and downstream failure are covered. |
| Solver inputs have practical allocation and iteration bounds. | Implemented | Checked budgets cover control points, spans, samples, variables, residuals, Jacobian/QR elements, and iterations. Host deserialization limits remain separate. |
| Serialized tracking topology rejects the audited malformed indices and kind mismatch. | Implemented | Focused regressions cover index correspondence, checked references, face cardinality, tracking dimensions, and kind mismatch. Oversized deserializer allocation remains pending. |
| Results are deterministic on the recorded Windows/MSVC host. | Procedurally validated | Immediate reruns and a separate-process full verify reproduce all 18 v4 digests. Cross-platform equality is not claimed. |
| The geometry continuity test target compiles to Wasm. | Implemented | Preserve the committed compile receipt and run it in CI. |
| The continuity path is usable on Wasm. | Not yet substantiated | Runtime execution in a supported Wasm host, including deterministic replay and bounded failure cases. |
| Experimental `G4` is reachable. | Procedurally validated as experimental | The rational quintic G4 case certifies with maximum normalized order-four residual `3.874577898e-3` using domain-valid one-sided cross stencils; production G4 remains not substantiated. |
| The implementation provides production `G4`. | Not yet substantiated | Not required. Keep `G4` experimental pending nonzero-offset and imported evidence. |
| The implementation provides "Class-A" surfacing. | Not yet substantiated | No Phase 4 claim is planned; this requires broader fairness and visual-quality criteria. |

## Validation corpus

The corpus and its runner must be deterministic, versioned, and runnable with
`cargo test` or `cargo run`. Each corpus version records:

- stable case identifiers and corpus-level generated-fixture provenance;
- per-case geometry source and licensing when a fixture is imported;
- requested continuity order and whether experimental behavior is enabled;
- boundary orientation and parameter mapping;
- scale, degree, spans, knot multiplicities, and rational weight range;
- expected disposition: converge, reject input, or fail without mutation;
- solver diagnostics and canonical output digest;
- independent maximum position/derivative residuals by order;
- elapsed time and bounded work counters when used as benchmark evidence.

The minimum corpus covers:

| Family | Required cases |
| --- | --- |
| Exact references | Planar and curved `G0`--`G3` pairs with known jets. |
| Degree and spans | Multi-span quintic polynomial and rational pairs. |
| Parameterization | Unequal domains, nonlinear monotone maps, and reversed boundaries. |
| Rational behavior | Non-unit, nonuniform weights and safe rejection of invalid weights. |
| Scale | Geometrically equivalent cases at small, unit, and large scales. |
| Edit/replay | Upstream surface edits followed by dependency-ordered contract replay. |
| Failure | Degenerate boundaries, insufficient degree, malformed values, bounded-count violations, and deterministic iteration-bounded nonconvergence. Contradictory multi-contract evidence remains deferred. |
| Determinism | Identical case results across repeated fresh-process runs. |

Imported assets may demonstrate loader and model integration, but an imported
file is not credited as continuity evidence unless the relevant surface,
boundary, orientation, and residuals are identified explicitly.

## Independent certification

The certification path must not reuse the solver's convergence decision as its
proof. It evaluates the repaired surfaces with independent public surface
evaluation and finite-difference machinery on a denser, degree-aware sample
set. Mandatory endpoints, knot boundaries, and occasional interior
coordinates may overlap the optimization collocation set; additional
span-distributed samples and all residual calculations are independent. For
every requested order it records absolute and scale-normalized residuals,
including endpoints, span boundaries, and interior points.

Certification must reject non-finite values and report the worst parameter and
derivative order. A case passes only when all required residuals are within its
recorded tolerance. Experimental `G4` results are reported separately and never
promote the `G0`--`G3` release gate.

## Audit ledger

Each finding is recorded with:

- stable identifier and affected phase;
- exact code or history evidence;
- severity and user-visible consequence;
- disposition: fixed, accepted, deferred, or not applicable;
- commit containing the repair, when fixed;
- reproducer and regression command;
- effect on the claim register.

The robustness review of deserialization, resource limits, failure safety,
replay integrity, numeric validation, and Wasm behavior is kept as a distinct
audit artifact. Numerical correctness and product-readiness claims require
their own evidence and are not inferred from a classification workflow.

An optional automated security-classification workflow was deferred after the
execution platform rejected a delegated dynamic crash-reproduction request.
Phase 4 did not retry or reframe that request. The partial workflow supplies no
security finding, severity, or clearance. [`PHASE-4-AUDIT.md`](PHASE-4-AUDIT.md)
records the manual-review boundary and carries independently observed
engineering issues as robustness findings.

## Quality gates

Phase 4 is complete only when all required gates pass or an external blocker is
precisely documented with ownership and a reproduction command:

- the Phase 1--3 architecture, mathematics, public API, failure behavior,
  serialization, allocations, replay, and history have independent review
  receipts;
- all high- and medium-impact in-scope findings are fixed or explicitly
  accepted;
- the committed corpus passes `G0`--`G3` independent certification;
- invalid and deliberately iteration-bounded nonconvergence cases preserve
  input state and return typed errors; contradictory multi-contract evidence
  is explicitly deferred;
- repeated runs produce the same canonical result digest;
- relevant package tests pass;
- `cargo clippy --all-targets -- -W warnings` passes, or pre-existing unrelated
  workspace failures are recorded with exact locations;
- formatting is checked with the repository's declared nightly toolchain;
  existing workspace-wide drift is resolved or explicitly accepted rather than
  described as a missing-toolchain blocker;
- the Wasm compile gate passes, and any runtime-usability claim has a
  supported-host execution receipt or remains explicitly not substantiated;
- commits are coherent and the stacked pull requests state intent, evidence,
  validation commands, known limits, and dependency order.

## Delivery plan

Phase 4 work is split into reviewable commits and, where useful, stacked pull
requests:

1. Phase 4 charter, claim register, and acceptance gates.
2. Audit ledger and focused correctness/failure-safety repairs.
3. Corpus, independent certification runner, and deterministic result schema.
4. Real-world evidence, benchmark records, and final claim updates.

The first pull request targets the Phase 3 branch so reviewers can inspect the
validation program without mixing it with later repairs. Subsequent pull
requests target the preceding Phase 4 branch until the evidence chain is
complete. No Phase 4 document should present an unmeasured aspiration as an
observed result.
