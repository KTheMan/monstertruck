# Upstream sync survey -- `ricosjp/truck`

Snapshot date: 2026-05-14.

## Setup

- Remote `upstream` -> `https://github.com/ricosjp/truck.git`.
- Merge-base: `9845ce71` (2026-02-12, "Merge branch 'cargo-upgrade-20260212' into 'master'").
- Upstream is **131 commits** ahead of that base.

## Commit class breakdown

| Class                          | Count | Action |
|--------------------------------|------:|--------|
| Merge commits                  |    47 | n/a    |
| `cargo upgrade` rolls          |    30 | skip   |
| `Update CHANGELOG`             |    10 | skip   |
| `fmt`/`clippy`/`dos2unix`      |     5 | skip   |
| **Substantive work**           |  **53** | review |

## Substantive feature branches

Upstream crate -> our crate mapping (used in the table):
`truck-base`->`monstertruck-core`, `truck-geotrait`->`monstertruck-traits`,
`truck-geometry`->`monstertruck-geometry`, `truck-topology`->`monstertruck-topology`,
`truck-polymesh`->`monstertruck-mesh`, `truck-meshalgo`->`monstertruck-meshing`,
`truck-modeling`->`monstertruck-modeling`, `truck-shapeops`->`monstertruck-solid`,
`truck-stepio`->`monstertruck-step`, `truck-platform`->`monstertruck-gpu`,
`truck-rendimpl`->`monstertruck-render`, `truck-assembly`->`monstertruck-assembly`,
`truck-js`->`monstertruck-wasm`, `truck-drafting`->`monstertruck-sketch` (proposed, new).

| Branch                          | Upstream crate(s)                                           | Conflict surface in our tree | Clean? |
|---------------------------------|-------------------------------------------------------------|-------------------------------|--------|
| **create-truck-drafting**       | `truck-drafting` (new)                                      | none -- new crate              | yes ** |
| drafting-multi-connector        | `truck-drafting`                                            | depends on bucket-1 landing   | yes ** |
| degenerate-corner-fillet        | `truck-drafting`                                            | depends on bucket-1 landing   | yes ** |
| drafting-proptest               | `truck-drafting`, `truck-modeling` (1 file)                 | yes (modeling)                | no     |
| fix-fillet-estimation           | `truck-drafting`, `truck-js`, `truck-platform`, `truck-rendimpl` | yes (wasm, gpu)            | no     |
| **simple-fillet-with-side**     | `truck-geometry`, `truck-geotrait`, `truck-shapeops`        | massive -- rewrites `fillet/mod.rs` against our `rolling_ball_fillet` redesign | no |
| 213-assy-step-output            | `truck-assembly`, `truck-js`, `truck-stepio`                | yes (step + wasm); paths renamed `in/`->`load/`, `out/`->`save/` | no |
| curve2d-to_same_geometry        | `truck-stepio`                                              | yes (step rename)             | no     |
| fix-rotated-curve-step          | `truck-stepio`                                              | yes (step rename)             | no     |
| stl-binary-read_exact           | `truck-polymesh/src/stl/*`                                  | minor                         | no -- still requires path translation |
| space-after-solid               | `truck-polymesh/src/stl/*`                                  | minor                         | no -- still requires path translation |
| offset-geoemtries               | `truck-base`, `truck-geometry`, `truck-geotrait`, `truck-shapeops` | yes everywhere         | no     |
| better-hash                     | `truck-base`, `truck-geometry`                              | yes (+ benchmarks)            | no     |
| bspline-basis-window            | `truck-geometry/nurbs/basis.rs` + 11 cross-crate updates    | basis.rs heavily renamed      | no     |
| fix-max-ders                    | `truck-base`, `truck-geometry`, `truck-geotrait`            | yes                           | no     |
| fix-empty-bounding-box          | `truck-base/bounding_box.rs`                                | yes (`cgmath_extend_traits.rs` etc.) | no |
| altnative-circle_arc            | `truck-modeling` (tangent constraint for arcs)              | yes                           | no     |
| fix-geotrait-tests              | `truck-geotrait` tests                                      | yes (trait method removals)   | no     |
| remove-render-object-by-id      | `truck-platform/scene.rs`                                   | yes (we already touched `gpu/src/scene.rs`) | no |
| fix-example-pages-on-mac        | `example-pages-generator`, `truck-platform`, `truck-rendimpl` | yes                         | no     |
| proptest-to-attribute           | 26 files across `base/geometry/meshalgo/modeling`           | yes everywhere                | no     |

** "clean" for the `truck-drafting` branches means *no conflict with existing files*. They still need directory rename `truck-drafting/` -> `monstertruck-sketch/`, `truck_drafting` symbol rename, edition/dep-version adjustments, and re-licensing review.

## Why nothing else applies cleanly

Our v0.3.0 prep changed something in every renamed crate since the merge base:

- `derivatives.rs` deleted in `monstertruck-core`.
- `rbf_surface` -> `rolling_ball_fillet`, `af_surface` -> `approximate_fillet_surface` in `monstertruck-geometry`.
- Public methods removed from `monstertruck-traits::{curve, surface, search_parameter}`.
- `step/{in,out}` -> `step/{load,save}` directory rename plus new STEP conic/trim code paths in `monstertruck-step`.
- `monstertruck-meshing` triangulation totally rewritten (`boundary.rs` +1302, `mod.rs` +1399).

Plus directory renames `truck-*` -> `monstertruck-*` and stepio's `in/`/`out/` -> `load/`/`save/` defeat git's rename detection at the directory level for many files. A direct `git cherry-pick` therefore won't apply.

## Action plan

### Bucket 1 -- port `truck-drafting` as `monstertruck-sketch` (next)

- Add crate `monstertruck-sketch` to workspace.
- Copy `truck-drafting/{src/, Cargo.toml}` and rename:
  - `truck_drafting` -> `monstertruck_sketch`
  - `truck_base` -> `monstertruck_core`, `truck_geotrait` -> `monstertruck_traits`,
    `truck_geometry` -> `monstertruck_geometry`, `truck_topology` -> `monstertruck_topology`
- Cargo.toml: switch to `{ workspace = true }` deps, bump edition to 2024 (already), version `0.1.0`.
- Re-license (Apache-2.0) and add `description`/`homepage`/`repository` to match siblings.
- Run `just lint-check` and `just test-cpu`, fix any naming impedance vs our renamed APIs.
- Add a `sketch` feature on the meta-crate `monstertruck`.

### Bucket 2 -- fillet improvements (DEFERRED)

`simple-fillet-with-side`, `degenerate-corner-fillet`, `fix-fillet-estimation` are
substantial rewrites of the original `rbf_surface` path that we've already
redesigned as `rolling_ball_fillet`. Re-conciling two parallel fillet redesigns
is its own project; revisit once our `rolling_ball_fillet` stabilizes.

### Bucket 3 -- targeted bug-fix cherry-picks (DEFERRED)

Hand-port these six small commits (each <50 lines):

| SHA       | Description                                          | Target crate            |
|-----------|------------------------------------------------------|-------------------------|
| `f43020bf`| Fix syntax error when reading binary STL > 8192b     | `monstertruck-mesh`     |
| `6c135abc`| Add a space after "solid" in STL ASCII format        | `monstertruck-mesh`     |
| `1fc145c9`| Fix empty bounding box                               | `monstertruck-core`     |
| `7b1f4171`| Fix offset-surface tessellation failure              | `monstertruck-meshing`  |
| `b5cbaed9`| Faster hash                                          | `monstertruck-core`     |
| `524f5f53`| Fix revolved curve to surface (STEP)                 | `monstertruck-step`     |

### Bucket 4 -- nice-to-have (DEFERRED)

- `bspline-basis-window` (77e25635) -- `BasisWindow` for basis-function range handling; sizeable but local to `nurbs/basis.rs`.
- `proptest-to-attribute` -- modernize property tests; needs a sweep but no semantic change.
- `9031e6dd` offset-geometries -- new feature.
- Truck releases hash 0.x bumps (truck-geometry 0.5.0, etc.) -- update version pins
  in `truck-sync.md` itself when porting.

## Notes for future syncs

- The merge-base advances every time we land a port. Update `9845ce71` to the
  new effective base when bucket 1 ships.
- Prefer `git cherry-pick -x` to preserve upstream SHAs in commit messages.
- Verify each port against our renames (`load`/`save`, `rolling_ball_fillet`,
  `approximate_fillet_surface`) -- don't restore upstream's original names.
