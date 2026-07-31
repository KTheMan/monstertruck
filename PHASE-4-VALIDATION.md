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
| Implemented | The code path exists and has focused automated coverage. |
| Analytically verified | Independent mathematical checks support the result; the solver's own objective or collocation set is not the sole verifier. |
| Realistically validated | Reproducible tests exercise imported or CAD-like models, representative edits, scale variation, and failure cases. |
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

## Claim register

This table is deliberately conservative. Evidence is promoted only after the
named artifact and command are committed and reproducible.

| Claim | Current state | Required Phase 4 evidence |
| --- | --- | --- |
| The kernel exposes `G0`--`G3` continuity primitives and a variational repair path. | Implemented | Independent formula audit, dense residual certification, and corpus results. |
| Rational and multi-span boundaries are supported. | Implemented | Quintic and rational cases with non-unit weights, multiple spans, and verifier samples disjoint from solver collocation. |
| Unequal parameterizations and reversed alignment are supported. | Implemented | Nonlinear and reversed mapping cases with endpoint and interior derivative checks. |
| Failed solves and replay do not partially mutate tracked geometry. | Implemented | Adversarial and unsatisfiable replay cases that compare complete pre/post state. |
| Replay after upstream edits is deterministic and dependency ordered. | Implemented | A multi-contract edit/replay scenario repeated across fresh processes. |
| Public inputs have practical allocation and iteration bounds. | Not yet substantiated | Audit of every caller-controlled size/count and tests at each rejected boundary. |
| Serialized tracking and continuity data is robust against malformed input. | Not yet substantiated | Schema/round-trip audit plus malformed and oversized input coverage. |
| Results are deterministic. | Not yet substantiated | Repeated-run canonical result digests and stable diagnostic ordering. |
| The continuity path is usable on Wasm. | Not yet substantiated | A documented Wasm compile/test gate or a precise external blocker. |
| The implementation provides production `G4`. | Not yet substantiated | Not required. Keep `G4` experimental unless a separate evidence set passes. |
| The implementation provides "Class-A" surfacing. | Not yet substantiated | No Phase 4 claim is planned; this requires broader fairness and visual-quality criteria. |

## Validation corpus

The corpus and its runner must be deterministic, versioned, and runnable with
`cargo test` or `cargo run`. Each case records:

- a stable case identifier and short provenance;
- geometry source and whether it is generated, CAD-like, or imported;
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
| Failure | Degenerate boundaries, insufficient degree, contradictory constraints, malformed values, and bounded-count violations. |
| Determinism | Identical case results across repeated fresh-process runs. |

Imported assets may demonstrate loader and model integration, but an imported
file is not credited as continuity evidence unless the relevant surface,
boundary, orientation, and residuals are identified explicitly.

## Independent certification

The certification path must not reuse the solver's convergence decision as its
proof. It evaluates the repaired surfaces on a denser, degree-aware sample set
that is distinct from the optimization collocation set. For every requested
order it records absolute and scale-normalized residuals, including endpoints,
span boundaries, and interior points.

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

An optional automated vulnerability-classification workflow was deferred after
the execution platform rejected a delegated crash-reproducer request. Phase 4
does not work around that control. [`PHASE-4-AUDIT.md`](PHASE-4-AUDIT.md)
records the exact unfinished checks and carries the underlying engineering
issues as robustness findings so the remainder of Phase 4 can proceed.

## Quality gates

Phase 4 is complete only when all required gates pass or an external blocker is
precisely documented with ownership and a reproduction command:

- the Phase 1--3 architecture, mathematics, public API, failure behavior,
  serialization, allocations, replay, and history have independent review
  receipts;
- all high- and medium-impact in-scope findings are fixed or explicitly
  accepted;
- the committed corpus passes `G0`--`G3` independent certification;
- unsatisfiable and invalid cases preserve input state and return typed errors;
- repeated runs produce the same canonical result digest;
- relevant package tests pass;
- `cargo clippy --all-targets -- -W warnings` passes, or pre-existing unrelated
  workspace failures are recorded with exact locations;
- formatting is checked with the repository's declared toolchain, or its
  unavailable toolchain is recorded as a reproducible infrastructure blocker;
- Wasm compatibility is tested, or the dependency/toolchain blocker is recorded
  precisely;
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
