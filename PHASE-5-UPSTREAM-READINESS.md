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
| Nonzero repair | Independently certified through G3 on the committed imported fixtures. | Add repeated-knot and extreme-positive-weight cases before any production-maturity claim. |
| G4 | Procedurally validated as experimental reachability. | Keep experimental and outside the upstream acceptance requirement. |
| Failure safety and resource bounds | Implemented with focused transactional and checked-dimension coverage. | Preserve typed failures; measure accepted-boundary work separately from certification. |
| Public API compatibility | Audited against the upstream surface; removed topology constructors/setters are restored and ordinary shell serialization remains compatible. | Resolve the recorded maintainer-choice items before proposing each upstream slice. |
| Public API usability | Runnable rustdoc examples cover tracking, modeling, contracts, direct solving, and replay; the STEP example covers supported and unsupported imported boundaries. | Keep the examples warning-free as slices are separated. |
| Module responsibility | Large tracking and solver modules are split into focused submodules near the repository guideline. | Preserve the split while extracting upstream slices. |
| Imported workflow | Implemented and independently certified through the requested order for the committed polynomial G1, rational reversed G2, and quintic G3 fixtures; the full imported-validation state is not yet substantiated. | Add triangle nondegeneracy, topology/bounding-box comparisons, and post-reimport seam certification. |
| Persistence | The repaired G1 trimmed shell exports, re-imports, and tessellates to 8,339 finite-position triangles. | Add exact topology invariants and scale-relative seam/bounding-box certification after re-import before treating persistence as a release gate. |
| External interoperability | The four inputs and one repaired Monstertruck output are accepted as valid B-reps by pinned OCCT. | Retain the output receipt and repeat it when STEP serialization changes. |
| Wasm compile | Geometry test compilation passes. | Preserve upstream's workspace Wasm job and keep the targeted geometry test as additive coverage. |
| Determinism | Same-host/cross-process Windows evidence and Linux tolerance/outcome CI. | Keep cross-platform bitwise equality unclaimed. |
| Performance | Not yet substantiated. | Separate solver time, certification time, iterations, accepted steps, and bounded work dimensions. |

## Minimum imported corpus

The pre-upstream corpus contains three provenance-clean adjacent-seam fixtures:

1. a polynomial multi-span `G1` repair;
2. a positive nonuniform rational, unequal or reversed `G2` repair;
3. a degree-three-or-higher, preferably quintic, `G3` repair.

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
surface metadata for all four inputs.

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
It does not establish production G2/G3 maturity because repeated knots,
extreme weights, persistence invariants, and broader real-model evidence
remain open.

The example currently verifies a nonempty finite-position mesh and the
re-imported presence of the solved spline pair. It does not yet reject
zero-area triangles, compare complete topology and bounding-box signatures, or
repeat seam certification on the serialized/re-imported surfaces. Those are
explicit remaining gates, so this pass does not label the complete public
workflow `Imported workflow validated`.

## Persistence and external receipt

At least one successful trimmed-shell case is exported through the public STEP
writer and re-imported. The receipt records:

- parse success;
- face, edge, and shell counts;
- shell condition after tessellation/welding;
- finite/nondegenerate triangle counts;
- bounding-box and, where meaningful, mass-property drift;
- independently certified seam residuals after re-import.

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
| Persistent identity and topology | Core tracking identifiers, bindings, lineage, session, and errors; `TopologyTracking`, `TrackingReport`, explicit `TrackedCompressedShell`/`TrackedCompressedSolid` envelopes; and tracked modeling transforms, sweeps, extrusions, and revolves. |
| Contract replay | `ContractId`, semantic boundary references, `ContinuityContract`, resolved contract types, contract/replay errors, the tracked-surface registry, prepared requests, replay solutions, and batch execution functions. |

`BoundaryAlignment` has one canonical definition in the continuity layer and a
compatibility re-export from the contract layer. `TrackingResult` likewise has
one core definition and is re-exported by topology. The unchanged master
surface in `BoundaryContinuitySolution` is borrowed, which removes redundant
deep copies without changing its value semantics. Ordinary `Shell` and `Solid`
serialization retain the upstream format; tracking persistence is explicit.

The following choices require maintainer review before their owning upstream
slice is finalized:

- whether `TopologyTracking` should be sealed;
- whether extensible public enums should be `#[non_exhaustive]`;
- whether existing broad error variants are acceptable for the first slice;
- whether solver construction should implement `Default`;
- whether `ContractId` should implement `FromStr` or `AsRef<str>`;
- whether public `SmallVec` and QR implementation types expose too much
  implementation detail;
- whether experimental G4 enablement should remain a boolean policy;
- how tracking-session and tracked-envelope schema versions should evolve
  without breaking the restored upstream serialization format.

These are manual API/semver decisions, not hidden validation failures. They are
kept out of the implementation until the relevant upstream slice is reviewed.

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
- the three imported positive STEP workflows and the arbitrary-trim negative;
- independent dense common-coordinate certification through requested G1,
  G2, and G3 orders for the three imported positive STEP workflows;
- deterministic exported G1 bytes and pinned OCCT output validation.

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
- Regenerating all four STEP fixtures preserves every recorded SHA-256.
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
