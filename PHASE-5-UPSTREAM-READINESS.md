# Phase 5 -- Upstream Readiness

Phase 5 converts the reviewed continuity prototype into an upstream-shaped
contribution. It does not broaden the solver's geometric scope. The upstream
design discussion is
[`virtualritz/monstertruck#4`](https://github.com/virtualritz/monstertruck/issues/4).

## Evidence vocabulary

Claims use the narrowest state supported by their evidence:

| State | Meaning |
| --- | --- |
| Implemented | The code path exists and has focused automated coverage. |
| Analytically verified | Independent mathematical review supports the formulas or invariants. |
| Procedurally validated | Reproducible generated fixtures exercise the public implementation with known construction and expected behavior. |
| Imported workflow validated | A provenance-clean CAD file reaches the feature through public import, topology, solve, certification, tessellation, and persistence APIs. |
| Externally validated | A pinned independent CAD implementation accepts and checks the exported result. |
| Not yet substantiated | The implementation or evidence is insufficient for the stated claim. |

`Procedurally validated` replaces the broader use of `Realistically validated`
for generated fixtures. It does not weaken the Phase 4 evidence. It prevents a
generated surface corpus from being mistaken for an imported production-CAD
workflow.

## Upstream contribution boundary

The first upstream proposal is a low-level two-surface solver for full
tensor-product NURBS boundaries. It does not claim:

- automatic seam discovery;
- arbitrary trimmed-subcurve support;
- topology sewing or solidification;
- a coupled constraint graph;
- Class-A surface quality;
- production `G4`.

The direct solver must remain usable without topology tracking. Persistent
tracking and contract replay are later, independently reviewable layers.

## Acceptance matrix

| Area | Current state | Upstream gate |
| --- | --- | --- |
| G0--G3 formulas and jets | Analytically verified and procedurally validated. | Preserve exact/property coverage and add executable public examples. |
| Nonzero repair | Imported workflow validated through G3 on the committed fixtures, including a multiplicity-three internal knot and strictly positive rational weights spanning `1e-8` through `1`. | Retain the fixture-bounded full-boundary G1--G3 claim; keep unequal-parameter nonzero-offset G3, arbitrary subcurves, and G4 outside it. |
| G4 | Procedurally validated as experimental reachability. | Keep experimental and outside the upstream acceptance requirement. |
| Failure safety | Procedurally validated for the committed synthetic replay chains. Commit `95e6b9ee` records schema-two preparation contradiction, same-surface/coupled-writer/cycle graph rejection, one-iteration bounded sequential nonconvergence, and late dependency failure after a successful staged solve. Every case preserves complete geometry, tracking-session, and contract-input snapshots; staged transitions/reports compare across reruns and failed batches return no solution. The MT-401 matrix source-audits all 18 tracked-modeling wrappers but adds no rollback evidence. The bounded replay case does not establish global mathematical infeasibility. | Implement MT-402 snapshots before tracked-modeling failure injection. |
| Resource bounds | Implemented with focused checked-dimension coverage. | Measure accepted-boundary work separately from certification. |
| Public API compatibility | Implemented and audited against the upstream surface; removed topology constructors/setters are restored and ordinary shell serialization remains compatible. | Resolve the recorded maintainer-choice items before proposing each upstream slice. |
| Public API usability | Implemented with runnable rustdoc examples for tracking, modeling, contracts, direct solving, and replay; the STEP example covers supported and unsupported imported boundaries. | Keep the examples warning-free as slices are separated. |
| Module responsibility | Implemented by splitting large tracking and solver modules into focused submodules near the repository guideline. | Preserve the split while extracting upstream slices. |
| Imported workflow | Imported workflow validated before export and after re-import through the requested order for five committed positive fixtures: polynomial G1, rational reversed G2, repeated-knot G2, extreme-positive-weight G2, and quintic G3. Every face tessellates exclusively to finite, scale-qualified, consistently oriented triangles; canonical topology and sampled bounding boxes persist. | Retain all five versioned receipts as release regressions. |
| Persistence | Imported workflow validated for all five repaired shells: exact canonical combinatorial signatures persist after STEP re-import, scale-normalized bounding-box drifts are at most `6.127015e-17`, and post-import seam certificates reproduce the pre-export residuals. | Retain the versioned receipts as regression gates and repeat external OCCT validation when STEP serialization changes. |
| External interoperability | Externally validated for B-rep interoperability: all six inputs and the repaired Monstertruck G1 output are accepted as valid B-reps by pinned OCCT, which also confirms the repeated-knot multiplicity and `1e8` rational-weight ratio. External seam-order validation is not claimed. | Retain the receipts and repeat them when fixture or STEP serialization changes. |
| Wasm compile | Implemented for both the restored full-workspace Wasm build and additive geometry-test compilation. | Preserve both hosted compile gates without promoting runtime usability. |
| Determinism | Procedurally validated by same-host/cross-process Windows evidence and Linux tolerance/outcome CI. | Keep cross-platform bitwise equality unclaimed. |
| Performance | Not yet substantiated. | Separate solver time, certification time, iterations, accepted steps, and bounded work dimensions. |

## Integration narrative reconciliation

Fork PR 4 ended at `a98cc9e2` and merged as `13c568ac`. Its final tree includes
higher-order imported repair, replayable persistence evidence, scale-qualified
triangle validation, and the repeated-knot and extreme-weight fixtures. All
seven jobs passed in hosted run `30854797421`. Its merged description now
records those final capabilities and their remaining limits rather than the
superseded three-workflow, G1-only deferrals.

Fork PRs 5 and 6 added tracked modeling and provenance integration. The exact
combined tip later passed all seven jobs in hosted run `30861560069`, attempt 2.
The five schema-3 receipts close the older three-workflow, G1-only, triangle,
topology/bounding-box, post-reimport-certificate, and higher-order-certificate
deferrals. The remaining limits are arbitrary boundary subcurves,
unequal-parameter nonzero-offset G3, Wasm runtime behavior, broad real-world
production-model coverage, external seam-order certification, and the API
decisions recorded below.

## Minimum imported corpus

The pre-upstream baseline contains three provenance-clean adjacent-seam
fixtures:

1. a polynomial multi-span `G1` repair;
2. a positive nonuniform rational, unequal or reversed `G2` repair;
3. a degree-three-or-higher, preferably quintic, `G3` repair.

The fixture stress extension adds:

4. a polynomial G2 pair with a degree-five seam direction and one internal
   multiplicity-three knot, preserving exactly C2 continuity at that knot;
5. a rational G2 pair with strictly positive weights from `1e-8` through `1`,
   a `1e8` ratio.

At least one fixture is a trimmed multi-face shell rather than two loose
surfaces. Every fixture records source, license, generating application and
version, units, file digest, selected faces and edge, expected boundary
classification, and solver configuration.

To reach the `Imported workflow validated` state, each successful case must:

1. load through public STEP APIs;
2. locate two faces incident to one compressed edge;
3. classify both face-local parameter curves as complete rectangular patch
   boundaries;
4. apply a deterministic edit to the dependent boundary strip;
5. solve through the public API;
6. certify the solved seam through the independent dense evaluator;
7. replace the dependent face surface without changing trim ownership;
8. tessellate to finite, nondegenerate output;
9. preserve recorded topology, bounding-box, and scale-relative seam
   invariants.

One negative fixture must contain an arbitrary trimmed seam. It must return a
typed unsupported result and leave all caller-visible inputs unchanged.

The committed corpus lives in `validation/continuity`. It is generated from
repository-owned definitions by `tools/phase5/generate_occt_fixtures.py` using
OCP `7.8.1.1`/OCCT `7.8.1`. `manifest.json` records provenance, construction,
degrees, parameter mapping, and SHA-256 digests. `occt-validation.json` records
independent re-import, B-rep validity, topology counts, bounding boxes, and
surface metadata, knot values and multiplicities, and rational-weight ranges
for all six inputs.

The public workflow is
`monstertruck-step/examples/continuity-step-validation`. Its independent
certificate uses public point evaluation, one-sided cross-boundary stencils,
the solved public coordinate transition, and mixed finite differences. It does
not reuse the solver objective or convergence decision. The certificate grid
contains 513 uniform seam positions plus mapped knot boundaries, producing 514
distinct samples for the committed fixtures.

The polynomial G1 repair has maximum normalized residuals
`[2.139619e-16, 1.309688e-11]`; the rational reversed G2 repair has
`[1.310417e-16, 1.034364e-11, 6.780301e-9]`; and the quintic G3 repair has
`[1.458563e-16, 8.987455e-12, 5.836967e-9, 4.147814e-6]`. Their respective
per-order limits are `[1e-9, 1e-7]`, `[1e-9, 1e-7, 1e-5]`, and
`[1e-9, 1e-7, 1e-5, 1e-3]`. Every case has maximum tangent-plane angle
`2.580957e-8` radians against a `1e-7` limit. Versioned JSON receipts are
committed beside the fixtures as `monstertruck-*-certificate.json`.

This closes the independent imported G2/G3 residual gate for these fixtures.
The repeated-knot G2 repair has maximum normalized residuals
`[1.354938e-16, 1.076137e-11, 7.112911e-9]`; the extreme-positive-weight G2
repair has `[1.750627e-16, 1.095664e-11, 7.007594e-9]`. Their post-import
certificates reproduce those arrays exactly. Combined with the baseline G1,
reversed-rational G2, and G3 cases, this supplies fixture-bounded imported
workflow validation for the explicitly supported full-boundary G1--G3 path.

Before committing an output or receipt, the example now re-imports the
in-memory STEP text, matches vertices by scale-relative position, and requires
the exact canonical edge set and oriented face-boundary cycles to match. It
also compares a 33-by-33 sample bounding box for every NURBS face plus all
shell vertices. The normalized maximum drifts are `5.011168e-17` for G1,
`1.290067e-28` for G2, and `6.127015e-17` for G3 against the default `1e-9`
limit.

The re-imported seam is selected again and independently certified. Its
maximum normalized residuals and tangent-plane angle reproduce the pre-export
certificate in every committed case. Schema-three receipts record both
certificates, both topology and bounding-box signatures, the comparison
result, the tolerance, and triangle-validity evidence before export and after
re-import.

Every tessellated face must contain triangles only. Every position and
referenced surface normal must be finite, every scale-normalized doubled area
must exceed `1e-14`, and every triangle normal must have cosine at least
`1e-6` against each of its three surface normals. The minimum doubled areas
across both passes are `4.756197e-5` for G1, `2.431324e-5` for G2, and
`1.646745e-5` for G3. The corresponding minimum normal alignments are
`0.995411`, `0.992599`, and `0.991544`. The repeated-knot and extreme-weight
cases have minimum doubled areas `6.084457e-5` and `3.073434e-5`, with minimum
normal alignments `0.999371` and `0.999435`. This closes the complete imported
workflow gate for the committed production corpus.

## Persistence and external receipt

Each successful trimmed-shell case is exported through the public STEP writer
and re-imported before any caller-visible output is written. The current
receipt records:

- parse success;
- face, edge, and shell counts;
- an exact canonical combinatorial topology signature;
- finite, triangle-only, nondegenerate, and consistently oriented mesh
  evidence before export and after re-import;
- sampled bounding-box signatures and scale-normalized drift;
- independently certified seam residuals after re-import.

Mass-property drift is not claimed for these open shells. The rational G2 case
validly re-tessellates from 9,797 to 9,777 triangles after STEP re-import; the
receipt therefore certifies both meshes rather than requiring
implementation-specific tessellation identity.

A pinned OCCT or FreeCAD command-line check then reads the exported STEP file,
runs its shape checker, and records tool version, command, input hash, topology
counts, bounding box, and checker result. This receipt may be a release gate
rather than a mandatory dependency of the normal Rust test suite.

`monstertruck-polynomial-g1-solved.step` is byte-for-byte deterministic across
two same-host runs with SHA-256
`41bf105b12840ce25a1a28ae77183b574fa790a36b02fbe07402bc8a4ebb06a1`.
`monstertruck-output-occt.json` records OCP `7.8.1.1`/OCCT `7.8.1`
re-importing it as a valid shell with two B-spline faces, seven edges, and six
vertices. This is external validation of the exported B-rep, not independent
validation of the seam's geometric-continuity order.

## Upstream delivery slices

Each pull request includes its own tests and documentation:

1. checked continuity order, capability, jets, and local transition semantics;
2. bounded direct solver and procedural/imported evidence;
3. generic identity plus topology tracking after separate API review;
4. continuity contracts and replay integration.

Fork-specific Phase 4 delivery history is not included in upstream pull
requests. The upstream-facing documents describe architecture, supported
behavior, validation commands, measured evidence, and known limits only.

## Public API audit

The proposed surface is grouped into four independently reviewable layers:

| Layer | Public items |
| --- | --- |
| Continuity primitives | `ContinuityOrder`, `SurfaceAxis`, `SurfaceBoundary`, `BoundaryAlignment`, `ContinuityCapabilityLevel`, and `SurfaceContinuityCapability`. |
| Direct solver | `BoundaryContinuitySolver`, `BoundaryContinuityRequest`, `BoundaryEndpoint`, `ContinuitySolverConfig`, `ContinuityResourceBudget`, `ContinuityResource`, `ContinuityTermination`, `OrderResidual`, `ContinuitySolveReport`, `BoundaryContinuitySolution`, `BoundaryTransition`, and `ContinuitySolveError`. |
| Persistent identity and topology | Core tracking identifiers, bindings, lineage, session, and errors; `TopologyTracking`, `TrackingReport`, explicit `TrackedCompressedShell`/`TrackedCompressedSolid` envelopes; and all 18 tracked modeling wrappers enumerated by the MT-401 version-one failure-point matrix. |
| Contract replay | `ContractId`, semantic boundary references, `ContinuityContract`, resolved contract types, contract/replay errors, the tracked-surface registry, prepared requests, replay solutions, and batch execution functions. |

`BoundaryAlignment` has one canonical definition in the continuity layer and a
compatibility re-export from the contract layer. `TrackingResult` likewise has
one core definition and is re-exported by topology. The unchanged master
surface in `BoundaryContinuitySolution` is borrowed, which removes redundant
deep copies without changing its value semantics. Ordinary `Shell` and `Solid`
serialization retain the upstream format; tracking persistence is explicit.

The following register records every unresolved API choice. Role owners identify
who must decide during slice review; they do not record approval.

| Decision | Current evidence, not a decision | Owner | Affected contribution slice | Decision state |
| --- | --- | --- | --- | --- |
| Seal `TopologyTracking` | The public trait is unsealed and includes the `#[doc(hidden)]` `into_untracked` construction hook. | Upstream topology API maintainers at slice review. | MT-903 -- tracking. | Requires maintainer direction; the current unsealed shape is not accepted policy. |
| Add `#[non_exhaustive]` to extensible public enums | Audited continuity, solver, tracking, contract, and replay enums are exhaustive; `ContinuityOrder` is a checked newtype. | Maintainer of each owning slice. | MT-901 through MT-904; MT-905 for optional tracked-modeling enums it includes. | Requires maintainer direction per enum and slice; no blanket policy is implied. |
| Public error granularity | `ContinuityOrder::new` and `TryFrom<usize>` use the crate-wide geometry error; later layers define dedicated solver, tracking, contract, and replay errors. | Upstream continuity-primitives API maintainers at slice review. | MT-901 -- continuity primitives. | Requires maintainer direction on a broad crate error versus a dedicated error. |
| Solver construction and `Default` | `ContinuitySolverConfig` and `ContinuityResourceBudget` implement `Default`; `BoundaryContinuitySolver` uses validated, fallible `new` and `new_with_resource_budget` constructors. | Upstream direct-solver API maintainers at slice review. | MT-902 -- bounded solver. | Requires maintainer direction; fallible construction is not a decision against `Default`. |
| `ContractId` string conversions | `ContractId` has validated `new`, `as_str`, `Display`, and Serde support, but no `FromStr` or `AsRef<str>`. | Upstream contract/replay API maintainers at slice review. | MT-904 -- replay. | Requires maintainer direction on `FromStr`, `AsRef<str>`, both, or neither. |
| Exposed `SmallVec` and QR implementation detail | `CurveJet::into_derivatives` returns `SmallVec`; QR types are crate-private, but `rank_tolerance`, `ContinuityResource::QrElements`, and the QR-element budget expose the algorithm in public controls. | Upstream continuity-primitives maintainer for `SmallVec`; direct-solver maintainer for QR controls. | MT-901 and MT-902. | Requires maintainer direction; decide the two exposures independently. |
| Experimental G4 policy | G4 remains outside production acceptance; configuration defaults it off and exposes `with_experimental_g4(bool)`. | Upstream direct-solver API maintainers at slice review. | MT-902, with MT-901's checked G4 representation as context. | Requires maintainer direction on the policy type and API; experimental status remains unchanged. |
| Tracking-session and tracked-envelope schema evolution | Contracts have explicit schema version 1 and reject unknown versions; `TrackingSession`, `TrackedCompressedShell`, and `TrackedCompressedSolid` serialize without envelope schema fields. Ordinary `Shell` and `Solid` compatibility remains mandatory. | Upstream tracking/persistence API maintainers at slice review. | MT-903 -- tracking; MT-504 supplies later schema-evolution evidence. | Requires maintainer direction on versioning and migration while preserving ordinary serialization. |

Until direction is captured, the prototype shapes above are implementation
evidence only and are not accepted upstream API policy. The open design issue
[`virtualritz/monstertruck#4`](https://github.com/virtualritz/monstertruck/issues/4)
has no maintainer response as of 2026-08-03.

## Validation status and deferred gates

The final local gates pass:

- the exact `test-cpu` cargo command across nine CPU crates with `derive` and
  `polynomial` features;
- all 176 geometry library tests and geometry integration tests;
- 53 core, 111 topology, 156 geometry, and 27 modeling doctests with
  `RUSTDOCFLAGS=-D warnings`;
- workspace `cargo clippy --all-targets -- -W warnings`;
- the complete 18-case continuity corpus, with every case repeated and matched
  to its committed digest;
- the five imported positive STEP workflows and the arbitrary-trim negative;
- independent dense common-coordinate certification through requested G1,
  G2, and G3 orders for the five imported positive STEP workflows;
- finite, nondegenerate, consistently oriented triangle certification before
  export and after re-import for all five positive STEP workflows;
- deterministic exported G1 bytes and pinned OCCT output validation.
- the public replay validation example and schema-two receipt for deterministic,
  transactional preparation contradiction, graph rejection, bounded
  sequential nonconvergence, and late dependency failure, including staged-prefix
  transition/report evidence and complete caller-input snapshots.

Exact integration tip `4054cc81` passed all seven Ubuntu jobs in
[workflow run `30861560069`, attempt 2](https://github.com/KTheMan/monstertruck/actions/runs/30861560069/attempts/2).
The hosted stable toolchain was `rustc 1.97.1 (8bab26f4f 2026-07-14)`;
formatting used `rustc 1.99.0-nightly (11177f223 2026-08-02)`. Attempt 1
failed the existing randomized Newton property test; the unchanged rerun
passed. The schema-two replay validation was committed afterward at
`95e6b9ee` and is not part of that hosted run.

The following local commands are deliberately deferred:

- `just test-cpu` cannot start because `just` is not installed. Its exact
  underlying cargo command was run successfully instead.
- The `.blueprints` submodule cannot be initialized from its configured
  `https://github.com/virtualritz/blueprints.git` URL because GitHub reports
  that repository as unavailable. The checked-in root `AGENTS.md` governed
  this pass.
- Local Wasm and meshing feature recipes invoke `cargo build` or `cargo check`,
  which this repository's agent rules prohibit for local verification. The
  restored CI jobs remain the authoritative gates.
- Cargo reports future-incompatibility notices in third-party `nom 3.2.1` and
  `quick-xml 0.22.0`; no new warning originates in the changed crates.

## Continuation backlog

The remaining integration, continuity, failure-safety, persistence, Wasm,
performance, real-model, and upstream-preparation work is decomposed into
epics and stories in
[`KERNEL-MATURITY-BACKLOG.md`](KERNEL-MATURITY-BACKLOG.md). That document is
the authoritative completion checklist and progress log. Use the reusable
[`CONTINUATION-PROMPT.md`](CONTINUATION-PROMPT.md) to start or resume a bounded
story set while keeping evidence and tracker state synchronized.

## Work log

- The upstream design discussion was opened and verified while authenticated
  as `KTheMan`.
- Removed public topology entry points are restored without making
  session-scoped tracking mutation public.
- Generated `G1`--`G3` results are labeled procedural evidence. Imported and
  external evidence remain separate gates.
- The upstream full-workspace Wasm job is restored. The geometry test compile
  remains additive rather than replacing upstream coverage.
- A repository-local, pinned OCCT fixture and validation route is used for
  external evidence. It is not a dependency of the normal Rust test suite.
- The continuity CI job runs all five imported positive workflows and verifies
  that the arbitrary-trim negative remains typed and transactional.
- Exact integration tip `4054cc81` passed all seven hosted jobs in workflow run
  `30861560069`, attempt 2. The unchanged rerun cleared an existing randomized
  Newton property-test failure from attempt 1.
- The schema-two replay-batch receipt covers deterministic, transactional
  preparation contradiction, graph rejection, bounded sequential nonconvergence, and
  late dependency failure through the public replay API.
- Regenerating all six STEP fixtures preserves every recorded SHA-256.
- OCCT independently re-imports every fixture and the repaired G1 output as a
  valid two-face B-spline shell. The negative arbitrary-trim fixture exits with
  the typed unsupported-subcurve path and creates no output file.
- The user explicitly authorized correcting a fork test when its assertion
  conflicts with verified upstream compatibility. The affected tracked-shell
  test previously serialized `&shell` and deserialized directly to `Shell`; it
  now serializes `shell.compress_tracked()`, deserializes
  `TrackedCompressedShell`, and calls `Shell::extract_tracked`. This preserves
  the upstream ordinary `Shell` format while retaining the same tracking
  roundtrip assertion.
- The allocation audit changed the unchanged first surface in
  `BoundaryContinuitySolution` from owned to borrowed. The affected test
  assertion changed from `assert_eq!(solved_first, first)` to
  `assert_eq!(solved_first, &first)`; the value equality check remains intact.
  No unrelated expectation is weakened.

The prototype's `ContinuityMaturity` classification is internal. It remains in
the version-one evidence serialization only to keep the reviewed corpus digest
stable; it is not part of the public upstream API. Evidence level is release
metadata, not a geometric property, so no replacement evidence enum is
introduced into the kernel.
