# Phase 4 -- Current Continuity Validation

This document defines the reproducible validation surface for the reconciled
continuity work. The primary evidence tree is `dev` at `47e837ca`, built from
the upstream foundation at `609c1b5a` and the upstream-shaped geometry layer at
`39f6a86a`. Upstream PR 19, merged as `06201787` from reviewed head `bf4cd9c5`,
supersedes the capability portion of that local layer. The current
reconciliation reruns the capability,
production `G3`, and experimental `G4` evidence with production solver calls
using PR 19's inherent capability method on the merged source shape.

## Evidence classes

### Capability and solver behavior

The capability example and focused tests exercise concrete B-spline and NURBS
inspection, all settled boundary sides, aligned and reversed transitions,
typed unsupported outcomes, bounded work, typed truncation, and the explicit
experimental `G4` gate.

This is implementation evidence for the local layer-2 tree. It is not approval
of the provisional public API.

### Production `G3` corpus

The default version-two corpus is the production full-boundary gate. Its 14
cases cover changed-solution repair for polynomial and positive-rational
surfaces, parameter-direction and alignment variants, repeated knots, scale
and weight conditioning, and deterministic bounded failure.

The independent dense certifier evaluates 33 span-distributed seam
coordinates, both endpoints, and mapped span boundaries from both surfaces. It
uses separate finite-difference tensor stencils and public surface evaluation.
It does not reuse the solver's residuals, derivatives, Jacobian, or convergence
decision.

Each corpus case runs twice. The emitted observation requires matching outcome,
digest, dense evidence, and deterministic work counters. Elapsed time is
reported but excluded from equality.

### Experimental `G4`

The preserved version-one corpus includes the explicit experimental `G4`
reachability and disabled-gate cases. These results do not promote `G4` into the
production acceptance set.

### Imported STEP workflow

The checked-in STEP fixture is generated deterministically from the repository's
minimal two-face topology fixture. The headless workflow:

1. imports the degree-five NURBS faces;
2. makes a deterministic nonzero edit to dependent control rows;
3. repairs the shared full-side seam to `G3`;
4. independently certifies the repaired boundary jet at 33 samples;
5. tessellates and exports the repaired shell;
6. re-imports the export and repeats certification;
7. verifies that a partial trim returns `TrimmedBoundary` before solver work and
   without shell mutation.

This is imported full-side integration evidence. It does not establish support
for arbitrary trimmed subcurves or automatic seam discovery.

## Reproduction commands

Run the geometry capability path:

```powershell
cargo run -p monstertruck-geometry --example continuity-capability-validation
```

Emit the default production `G3` observations:

```powershell
cargo run -p monstertruck-geometry --example continuity-validation -- `
  --emit target/continuity-validation-v2.json
```

Emit the preserved version-one observations, including experimental `G4`:

```powershell
cargo run -p monstertruck-geometry --example continuity-validation -- `
  --corpus monstertruck-geometry/resources/continuity-validation/v1/corpus.json `
  --emit target/continuity-validation-v1.json
```

Verify fixture provenance and run the STEP workflow:

```powershell
cargo run -p monstertruck-io --example generate-continuity-g3-fixture -- --check
cargo run -p monstertruck-io --example continuity-repair-step -- `
  --out target/continuity-repaired.step
```

Repository-wide test, documentation, lint, and formatting gates are defined in
`AGENTS.md` and are not duplicated here.

## Result interpretation

| Observation | Supported statement |
| --- | --- |
| Version-two corpus succeeds | The recorded full-boundary cases meet their independent `G3` limits on the exact tested revision. |
| A version-two bounded case truncates | The exact resource, spent work, request, and budget match its typed expectation and no solution is returned. |
| Version-one experimental cases succeed | Experimental `G4` is reachable for the recorded cases only. |
| STEP workflow succeeds | The recorded full-side fixture survives repair, tessellation, export, re-import, and independent `G3` certification. |
| Partial-trim check succeeds | The workflow refuses the recorded arbitrary trim with the typed reason before numerical work and without mutation. |

Passing these cases does not establish global continuity over unsupported seam
classes, production `G4`, general topology sewing, or upstream API acceptance.

## Revision ledger

| Revision | Validation statement |
| --- | --- |
| `609c1b5a` | Upstream PR 13 traits foundation. |
| `bf4cd9c5` | Reviewed PR 19 capability-inspection head; `gh pr checks 19` reported all five hosted checks successful for this revision before merge. |
| `06201787` | Merged upstream PR 19 capability-inspection authority. |
| `39f6a86a` | Locally verified combined layer-2 implementation; its capability portion predates PR 19 and is superseded for future upstream work. |
| `47e837ca` | Locally verified production `G3` corpus and STEP evidence in the first proving ground. |
| `0dd9fdc9` | Local second proving-ground promotion; not pushed. |

The PR 19 hosted-check statement above applies only to `bf4cd9c5` and was read
with `gh pr checks 19`. All other results in this document are local. Future
results must name the exact revision and command that produced them.

## Downstream validation boundary

The custom MT-402 modeling snapshot harness is superseded. Current snapshot
evidence is the topology-owned `StableId`, attribute, compression, hash, and
round-trip foundation recorded in `TRACKING-SCOPE.md`. Selected probes from
archive commit `568c3c4f` may be recast later as focused topology tests, but
the parallel projection and modeling-result layers are not current evidence.

Those layers remain deferred until the upstream solver proposal and public API
shape are accepted. Later validation must be recut against the accepted model
rather than preserving superseded downstream APIs.
