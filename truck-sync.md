# Upstream sync survey -- `ricosjp/truck`

Snapshot date: 2026-05-14.

## Setup

- Remote `upstream` -> `https://github.com/ricosjp/truck.git`.
- Merge-base: `9845ce71` (2026-02-12, "Merge branch 'cargo-upgrade-20260212' into 'master'").
- Upstream is **131 commits** ahead of that base.

## Executive verdict

Treat upstream `ricosjp/truck` as a patch queue, not as a branch to merge. A direct merge or broad cherry-pick would reintroduce old upstream architecture into crates that have already been renamed, cleaned up, and substantially rewritten in `monstertruck`.

The incoming work falls into four categories:

1. **Already landed here.** Some items listed as pending in the first survey are already present in current `monstertruck`.
2. **Small correctness fixes worth porting.** STEP and STL fixes that are local and easy to verify.
3. **Useful ideas that need redesign before landing.** `BasisWindow`, offset geometry, assembly STEP output, and tangent arc construction.
4. **Do not merge wholesale.** Fillet and drafting work are not ready to become public `monstertruck` APIs without a naming, docs, and design pass.

Recommended immediate direction:

- Do **not** integrate `truck-drafting` as `monstertruck-sketch` yet unless we explicitly want an experimental crate.
- Port only small, audited fixes first.
- Promote `BasisWindow` to an explicit performance task.
- Keep fillet, offset, assembly STEP, and sketching as separate design projects.

## Commit class breakdown

| Class                          | Count | Action |
|--------------------------------|------:|--------|
| Merge commits                  |    47 | Ignore. |
| `cargo upgrade` rolls          |    30 | Skip. |
| `Update CHANGELOG`             |    10 | Skip. |
| `fmt`/`clippy`/`dos2unix`      |     5 | Skip unless touching same code. |
| **Substantive work**           |  **53** | Review and hand-port selectively. |

## Crate mapping

Use this mapping when reading upstream commits:

| Upstream crate | Monstertruck crate |
|----------------|--------------------|
| `truck-base` | `monstertruck-core` |
| `truck-geotrait` | `monstertruck-traits` |
| `truck-geometry` | `monstertruck-geometry` |
| `truck-topology` | `monstertruck-topology` |
| `truck-polymesh` | `monstertruck-mesh` |
| `truck-meshalgo` | `monstertruck-meshing` |
| `truck-modeling` | `monstertruck-modeling` |
| `truck-shapeops` | `monstertruck-solid` |
| `truck-stepio` | `monstertruck-step` |
| `truck-platform` | `monstertruck-gpu` |
| `truck-rendimpl` | `monstertruck-render` |
| `truck-assembly` | `monstertruck-assembly` |
| `truck-js` | `monstertruck-wasm` |
| `truck-drafting` | `monstertruck-sketch`, if it ever lands. |

STEP path mapping:

- `truck-stepio/src/in` -> `monstertruck-step/src/load`.
- `truck-stepio/src/out` -> `monstertruck-step/src/save`.

Geometry naming mapping:

- `KnotVec` -> `KnotVector`.
- Upstream `BSpline*` spelling -> current `Bspline*` spelling.
- `rbf_surface` -> do not resurrect. Map useful ideas into `rolling_ball_fillet` only.
- `af_surface` -> `approximate_fillet_surface`.
- Avoid abbreviations such as `assy`, `geom_impls`, `transit`, `bdd_box`, and `gen` in new public APIs.

## Why broad merge/cherry-pick is wrong

Our v0.3.0 prep changed something in every renamed crate since the merge base:

- `derivatives.rs` was renamed/reworked in `monstertruck-core`.
- `rbf_surface` -> `rolling_ball_fillet` and `af_surface` -> `approximate_fillet_surface` in `monstertruck-geometry`.
- Public trait methods were removed or renamed in `monstertruck-traits`.
- `step/{in,out}` -> `step/{load,save}` plus substantial STEP conic/trim fixes in `monstertruck-step`.
- `monstertruck-meshing` triangulation was heavily rewritten.
- Directory renames `truck-*` -> `monstertruck-*` defeat git's rename detection across many files.

A direct cherry-pick will either fail mechanically or, worse, apply old concepts into renamed code where the semantic assumptions no longer hold.

## Branch verdicts

| Branch | Verdict | Reason |
|--------|---------|--------|
| `create-truck-drafting` | Defer. | Useful 2D construction reference, but not public API quality yet. |
| `drafting-multi-connector` | Defer with drafting. | Depends on the sketch crate decision. |
| `degenerate-corner-fillet` | Defer with drafting. | Sketch-specific fixes; should be reviewed with sketch design. |
| `drafting-proptest` | Defer with drafting. | Tests are useful reference material only if sketch lands. |
| `fix-fillet-estimation` | Do not merge wholesale. | Mixes drafting, wasm, gpu, render, and fillet assumptions. |
| `simple-fillet-with-side` | Do not merge wholesale. | Rewrites old fillet architecture that conflicts with `rolling_ball_fillet`. |
| `213-assy-step-output` | Defer as its own project. | Complementary, but large enough to deserve a save/assembly design. |
| `curve2d-to_same_geometry` | Port early. | Local STEP/modeling correctness improvement. |
| `fix-rotated-curve-step` | Port early. | Local STEP correctness improvement. |
| `stl-binary-read_exact` | Already landed. | Current `monstertruck-mesh` already uses `read_exact`. |
| `space-after-solid` | Optional small port. | Low risk, low value; improves ASCII STL compatibility. |
| `offset-geoemtries` | Defer as its own project. | Potentially useful but large, misspelled, and entangled with derivatives/fillets. |
| `better-hash` | Do not port as-is. | Converts generic scalars through `f64` via `ToPrimitive`; conflicts with precision/generic-number direction. |
| `bspline-basis-window` | Promote. | Real performance improvement; port conceptually with `SmallVec` and current naming. |
| `fix-max-ders` | Partial port only. | Recursion guard/hash-channel fix is useful; reducing `MAX_DER_ORDER` from `31` to `10` is not justified yet. |
| `fix-empty-bounding-box` | Already landed. | Current `BoundingBox::is_empty` checks every dimension. |
| `altnative-circle_arc` | Port later with renamed API. | Useful modeling/sketch feature; upstream spelling and names need cleanup. |
| `fix-geotrait-tests` | Skip. | Test modernization only, against upstream trait surface. |
| `remove-render-object-by-id` | Skip for now. | GPU/render API churn, not related to kernel correctness. |
| `fix-example-pages-on-mac` | Skip for now. | Example/gpu/render maintenance, not core. |
| `proptest-to-attribute` | Skip for now. | Test-style churn; no semantic improvement. |

## Phase 0 -- Update tracking before porting

Before any code ports:

- Mark already-landed fixes as already landed.
- Do not advance the merge base yet. The effective sync base advances only after we land audited ports with their own commits.
- For every hand-port, include the upstream SHA in the commit body, for example `Upstream: ricosjp/truck@524f5f53`.
- Keep each logical upstream change in its own commit.
- Run the normal `monstertruck` verification for the touched crates.

## Phase 1 -- Small correctness ports

### `524f5f53` -- revolved line to cylinder STEP conversion

Target:

- `monstertruck-step/src/load/step_geometry/geom_impls.rs`.

Why:

- Current code still has the old inverted/default conversion logic for `RevolutionSurface<Curve3D>` when the entity curve is a line parallel to the axis.
- The upstream fix corrects the origin/line choice and removes the extra processor inversion.

Porting notes:

- Adapt `RevolutedCurve` naming to current `RevolutionSurface` naming.
- Preserve all current STEP conic/trim fixes.
- Add or keep a focused regression that loads/exports a cylindrical swept surface and verifies axis/radius/orientation.

### `08d2cbf1` -- `ToSameGeometry` for STEP 2D geometries

Target:

- `monstertruck-step/src/load/step_geometry/geom_impls.rs`.

Why:

- Local, useful, and conceptually clean.
- Helps STEP pcurve/trim handling by making 2D geometry conversions explicit.

Porting notes:

- Map upstream `Curve2D`/`Conic2D` to current modeling/step geometry types.
- Use current `ParametricCurve` naming, not upstream's older spellings.
- Add tests around line, conic, and B-spline 2D conversion.

### `6c135abc` -- ASCII STL header compatibility

Target:

- `monstertruck-mesh/src/stl.rs`.

Why:

- Some readers expect `solid ` before the optional name.
- Very low risk.

Porting notes:

- If current tests assert exact `solid\n`, update behavior intentionally with a new compatibility test.
- Do not mix this with binary STL changes; `read_exact` is already present.

### Partial `7b1f4171` -- surface division recursion guard and independent hash channels

Targets:

- `monstertruck-traits/src/algo/surface.rs`.
- Possibly related derivative code only if required by tests.

Why:

- The recursion guard prevents unbounded subdivision.
- Using `hash2` for independent `p`/`q` is better than using `hash1` twice.

Do not port blindly:

- Do not reduce `MAX_DER_ORDER` from `31` to `10` without evidence.
- Do not pull in offset-surface code as part of this small fix.

Porting notes:

- Replace magic `100` with a named constant, e.g. `MAX_PARAMETER_DIVISION_RECURSION`.
- Preserve our docs/comment style.
- Add a test that previously recursed excessively or produced correlated samples.

## Phase 2 -- NURBS performance: active basis windows

Upstream:

- `77e25635` (`BasisWindow`).

Verdict:

- Port the idea, not the literal code.

Why:

- Current `KnotVector::bspline_basis_functions` returns a full `SmallVec` with zeros for all basis functions.
- B-spline evaluation only needs the active window, usually `degree + 1` values.
- This helps curve/surface evaluation and therefore STEP meshing, NSI export, and any algorithm repeatedly evaluating NURBS.

Design requirements:

- Use `SmallVec`, not upstream `tinyvec`.
- Name the type `BasisWindow` or `BsplineBasisWindow`; prefer `BasisWindow` if scoped inside `nurbs`.
- Expose:
  - `base_index()` or `start_index()`.
  - `values()`.
  - `to_full_values()` only as compatibility/debug helper.
- Update `BsplineCurve` and `BsplineSurface` to zip active control points only.
- Keep old full-vector behavior only where tests or public compatibility require it.
- Rewrite docs to avoid upstream wording and to link first references per repo docs rules.

Suggested task split:

1. Add `BasisWindow` plus tests for base index and full reconstruction.
2. Convert `KnotVector::try_bspline_basis_functions` to return the window.
3. Update `BsplineCurve` evaluation.
4. Update `BsplineSurface` evaluation.
5. Run NURBS, STEP, and meshing tests.

## Phase 3 -- Modeling tangent arcs

Upstream:

- `993e156c` (`altnative-circle_arc`).

Verdict:

- Useful, but rename before public exposure.

Rename plan:

- `ArcConstraint` -> `CircularArcConstraint`.
- `Transit` -> `ThroughPoint`.
- `Tangent` -> `StartTangent`.
- `circle_arc_by_tangent0` -> `circle_arc_by_start_tangent`.
- Avoid `0` suffixes in public names.

Functional notes:

- Validate degenerate tangent and tangent/chord parallelism explicitly.
- Return `Result` from fallible constructors.
- Keep panicking convenience wrappers only if this crate already has that pattern; otherwise prefer `try_*` only for new API.

Target:

- `monstertruck-modeling` first.
- `monstertruck-sketch` can reuse it later if sketch lands.

## Phase 4 -- Assembly STEP output

Upstream:

- `213-assy-step-output`.

Verdict:

- Complementary, but not urgent.

Why:

- Current `monstertruck-step/src/save` has geometry and topology save modules, but no assembly save module.
- Assembly STEP output is useful but touches `monstertruck-assembly`, `monstertruck-step`, wasm/examples/tests.

Design requirements before porting:

- Add `monstertruck-step/src/save/assembly.rs`.
- Use `assembly` in new names, not `assy`.
- Keep existing `monstertruck-assembly::assy` module only for compatibility; do not spread the abbreviation.
- Avoid upstream's broad generic display machinery if a smaller API can cover current `monstertruck` shapes.
- Add round-trip or structural STEP-output tests.

## Phase 5 -- Offset geometry

Upstream:

- `9031e6dd` (`offset-geoemtries`).

Verdict:

- Potentially useful, but defer as a separate design project.

Why:

- Large diff across traits, derivatives, fillets, and geometry decorators.
- Contains naming/documentation issues and old upstream assumptions.
- Offset geometry can be valuable, but it is easy to add unstable API surface here.

If ported later:

- Rename `Offset` into explicit `OffsetCurve` and `OffsetSurface` unless a single generic type is truly justified.
- Rename `NormalField` into `NormalOffsetField` or another precise term.
- Replace Japanese comments and vague docs.
- Validate derivative formulas independently.
- Do not pull in old fillet changes as collateral.

## Phase 6 -- Fillet work

Upstream branches:

- `simple-fillet-with-side`.
- `fix-fillet-estimation`.
- Related `rbf_surface`/`af_surface` commits.

Verdict:

- Do not merge wholesale.

Why:

- This is a parallel redesign of upstream's old fillet path.
- Our code has already moved to `rolling_ball_fillet` and `approximate_fillet_surface` names/structure.
- Merging would likely regress architecture and make the Yang/boolean work harder to reason about.

Use upstream only as reference:

- Mine tests and edge cases.
- Mine specific numerical fixes if they apply to our current fillet implementation.
- Do not resurrect `rbf_surface` module structure.
- Do not block Boolean/Yang work on this.

## `truck-drafting` / potential `monstertruck-sketch` assessment

Verdict:

- Useful as reference material.
- Not ready to integrate into public `monstertruck` as-is.
- If we want sketching soon, land it behind an explicit `experimental-sketch` feature or keep it out of the workspace until its API is cleaned up.

### What upstream drafting contains

Files:

- `truck-drafting/src/geometry.rs`: 2D curve enum and conversions.
- `truck-drafting/src/draw.rs`: construction helpers for vertices, lines, polylines, Beziers, arcs, and tangent connector sequences.
- `truck-drafting/src/corner.rs`: fillet/chamfer operations on 2D wires.
- `truck-drafting/src/geom_impls.rs`: geometric helper algorithms.
- `truck-drafting/src/errors.rs`: crate error type.
- `truck-drafting/src/lib.rs`: re-exports and topology alias.

The core idea is sound: a small 2D construction layer over topology and geometry. The implementation is not yet at the quality bar for our public API.

### Naming changes required

Crate/module naming:

- `truck-drafting` -> `monstertruck-sketch`.
- `truck_drafting` -> `monstertruck_sketch`.
- `draw` -> `sketch` or `builder`. Prefer `builder` if matching `monstertruck-modeling`; prefer `draw` only if this is intentionally user-facing drafting syntax.
- `geom_impls.rs` -> `construction.rs` or `algorithms.rs`.
- `corner.rs` can stay if it only contains corner treatments; otherwise split into `fillet.rs` and `chamfer.rs`.

Type naming:

- `Curve` -> `SketchCurve` or `Curve2D`. Prefer `Curve2D` only if it does not conflict with existing modeling type names.
- `CircleArc` alias -> `CircularArc2D`.
- `ArcConstraint` -> `CircularArcConstraint`.
- `Transit` -> `ThroughPoint`.
- `Tangent` -> `StartTangent`.
- `TrimmableCurve2D` is acceptable if it remains private or crate-local.
- `CornerResult` -> `CornerTreatment` or `CornerReplacement`. `CornerResult` says nothing about content.

Function naming:

- `circle_arc_by_tangent0` -> `circle_arc_by_start_tangent`.
- `parameter_at_curve_length` -> `parameter_at_signed_distance` or `parameter_at_arc_length_offset`.
- `rot_4` -> `rotate_quarter_turn`.
- `lines_crossing_point` -> `line_intersection`.
- `arc_arc_transit` -> `arc_arc_transition_point`.
- `line_arc_line_transit` -> `line_arc_line_transition_points`.
- `arc_line_arc_transit` -> `arc_line_arc_transition_points`.
- `fillet_cndidate` test typo -> `fillet_candidate`.

Docs wording:

- Replace "Tries to returns" with "Creates" / "Attempts to create".
- Replace "inter control points" with "interior control points".
- All comments must end in periods.
- First references to types in docs need links per repo policy.
- Examples must use `monstertruck_sketch`, not `truck_drafting`.

### Functional concerns before integration

1. Panicking wrappers.

   The crate exposes both `try_*` and panicking non-`try` constructors using `.unwrap()`. That is acceptable for examples in some APIs, but not for a foundational CAD kernel crate unless clearly documented.

   Recommendation:

   - Keep `try_*` as primary API.
   - Either remove panicking wrappers or document panic conditions explicitly.
   - If wrappers stay, use `expect` with invariant text, not bare `.unwrap()`.

2. Ambiguous arc construction semantics.

   `ArcConstraint::Transit` and `ArcConstraint::Tangent` are underspecified. `Tangent` means start tangent. `Transit` means through-point. These must be explicit in the type names.

3. Numerical robustness.

   `geom_impls.rs` uses Newton solves, hard-coded iteration count `100`, `TOLERANCE`, and fixed step arc-length integration. This needs named constants and tests for failure modes.

4. Fixed arc-length integration step.

   `parameter_at_curve_length` uses `max(8, length / 0.01)` RK4 steps. That is an implicit world-space tolerance and will fail scale invariance.

   Recommendation:

   - Accept a tolerance/config parameter or derive from curve scale.
   - Use existing curve parameter division or adaptive integration if available.

5. Generic curve enum overlap.

   `geometry.rs` defines a new 2D `Curve` enum that overlaps with existing modeling/geometry curve concepts. Before landing, decide whether sketch owns its own curve enum or reuses/re-exports existing `monstertruck-modeling` 2D curves.

6. Fillet/chamfer semantics on wires.

   `fillet_all` and `chamfer_all` clone and trim edge sequences. This is useful, but it needs stronger tests for:

   - Closed wires.
   - Near-degenerate adjacent edges.
   - Consecutive fillets overlapping.
   - Mixed line/arc/B-spline wires.
   - Orientation preservation.

7. Error names.

   Most error variants are useful, but wording should be tightened:

   - `NoConnection` -> `NoTangentConnection` or more specific variants.
   - `DegenerateConnectionCorner` -> `DegenerateConnectorCorner` or `DegenerateCorner` if duplicate distinction is unnecessary.
   - `FilletNewtonNotConverged` -> `FilletSolveDidNotConverge`.

8. Re-export policy.

   `lib.rs` re-exports broad `base::*` and `truck_geotrait::*`. That makes the crate convenient but pollutes public API.

   Recommendation:

   - Avoid broad prelude re-exports in `monstertruck-sketch`.
   - Provide a small `prelude` module instead.
   - Re-export only essential geometry/topology aliases.

### If we integrate later

Use this landing sequence:

1. Copy upstream `truck-drafting` into a temporary branch only.
2. Rename crate and imports to `monstertruck-sketch`.
3. Apply naming cleanup before adding it to the workspace.
4. Add a narrow `prelude`.
5. Make panicking wrappers an explicit API decision.
6. Add tests for line, polyline, arc, connector, fillet, chamfer, cyclic wire cases.
7. Only then add the crate to the workspace and meta-crate feature list.

### If we do not integrate now

Keep a note in this file and mine only these parts:

- Tangent circular arc construction into `monstertruck-modeling`.
- Specific fillet/chamfer tests as future `monstertruck-sketch` acceptance tests.
- Connector algorithms only after their tolerance/scaling behavior is redesigned.

## Recommended order

1. Port `524f5f53` STEP revolved-line/cylinder fix.
2. Port `08d2cbf1` STEP 2D geometry conversion fix.
3. Port the safe subset of `7b1f4171` surface-division recursion/hash fix.
4. Optionally port `6c135abc` ASCII STL header compatibility.
5. Design and port active B-spline basis windows with current names and `SmallVec`.
6. Port tangent circular arcs into `monstertruck-modeling` with cleaned names.
7. Reassess assembly STEP output.
8. Reassess offset geometry.
9. Reassess `monstertruck-sketch`; do not make it public until the API cleanup above is done.

## Notes for future syncs

- Prefer hand-porting over `git cherry-pick` for all non-trivial changes.
- Preserve upstream attribution in commit bodies.
- Do not restore upstream names when current `monstertruck` names are clearer.
- Do not advance the merge-base in this file until audited ports land.
- Treat drafting, fillet, offset, and assembly as independent projects, not as part of routine upstream sync.
