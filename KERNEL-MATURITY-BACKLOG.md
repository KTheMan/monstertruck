# Kernel Maturity Backlog

This backlog starts from the reconciled local baselines recorded in
[`PHASE-5-UPSTREAM-READINESS.md`](PHASE-5-UPSTREAM-READINESS.md). Repository
governance and validation commands remain in `AGENTS.md` and are not repeated
here.

## Current state

| Work | Revision | State |
| --- | --- | --- |
| Upstream continuity foundation | `upstream/master` at `06201787` | Merged upstream through PR 19, including PR 13's checked traits and PR 19's inherent geometry inspection. |
| Upstream capability-inspection slice | PR 19 merged as `06201787` | Maintainer-authored and authoritative for the capability API and diagnostic precedence. |
| Local transition and solver follow-on | `39f6a86a` | Implemented and verified locally, but based on the superseded combined shape; it must be recut on merged authority `06201787`. |
| Production full-boundary `G3` gate | code tree `47e837ca`; plan integration `3a81e439` | Complete and independently verified locally; later dev commits are documentation-only. |
| Second proving-ground promotion | current local `master` head; runtime promotion lineage at `0dd9fdc9` | Complete locally; not pushed. |
| Experimental `G4` | local continuity branches | Reachable only through an explicit experimental boundary; not production. |
| MT-402 custom modeling snapshots | archive commit `568c3c4f` | Superseded by the topology-owned snapshot foundation; preserved for selective later extraction only. |

## P0 -- Reconcile PR 19 and propose the solver follow-on

### MT-901 -- Record the exact merged PR 19 revision

- Treat merged `upstream/master` at `06201787` as the capability-inspection
  authority.
- Preserve its inherent `continuity_capability` methods, checked
  `try_unsupported` helper invariant, typed reasons, and single-surface scope.
- Keep transition compatibility and solver feasibility outside PR 19.

**Done:** PR 19 merged as `06201787`; the reviewed head was `bf4cd9c5`, and no
local claim extends beyond the merged revision.

### MT-902 -- Reconcile the merged capability slice -- complete

- Reconciled `06201787` into `master` at `0c24aeb0`, then into `dev` at
  `2f081f7d`, in the order required by `AGENTS.md`.
- Production solver call sites use the merged inherent methods and preserve
  upstream diagnostic precedence exactly. Stricter local validation adapters
  remain excluded from the upstream contribution.

**Done:** Both proving grounds contain the exact merged capability semantics,
and the local solver consumes the inherent methods. The follow-on recut must
contain no downstream validation adapters.

### MT-903 -- Propose and recut the solver follow-on

- Start from the then-current `upstream/master`.
- Apply only transition semantics, bounded solving, and their scoped evidence;
  do not duplicate PR 19 capability code.
- Obtain maintainer direction for the public transition and solver surface.
- Port the relevant production `G3` evidence from `47e837ca`.
- Retain typed refusal for arbitrary trimmed seams.

**Done when:** The branch is a focused patch on the current upstream base and
contains no downstream branch history.

### MT-904 -- Validate the exact contribution revision

- Run the repository-required gates on the final contribution tree.
- Run the independent dense `G3` certification and imported full-side
  validation on that same revision.
- Record local results separately from any hosted checks.

**Done when:** Every reported result names the exact revision checked, and any
hosted-check statement is backed by the check provider's result.

## P1 -- Continuity evidence extensions

These items extend evidence after the approved solver shape is stable. They do
not block the already verified local production `G3` gate.

### MT-920 -- Broaden imported-model coverage

- Add provenance-clean full-boundary models with more surface classes, scales,
  knot structures, and positive weight ranges.
- Keep imported B-rep validity separate from independent continuity
  certification.

**Done when:** Each fixture has versioned provenance and a deterministic typed
outcome on the exact tested revision.

### MT-921 -- Measure solver and certification costs

- Record solver work independently from dense certification, import,
  tessellation, and export.
- Include successful, truncated, and unsupported cases.

**Done when:** Receipts expose bounded work and phase-level measurements
without unsupported performance claims.

### MT-922 -- Extend experimental `G4` evidence

- Keep all public configuration and documentation explicitly experimental.
- Add conditioning and failure evidence without promoting a production claim.

**Done when:** The evidence demonstrates experimental reachability and typed
bounded failure only.

## Deferred -- Tracking, persistence, contracts, and replay

The following sequence begins only after upstream solver acceptance:

1. review tracking and persistence against the established stable identity and
   attribute systems;
2. propose continuity contracts and replay on top of the accepted solver;
3. re-evaluate the preserved MT-402 probes against the approved downstream model;
4. begin MT-403 construction and sweep rollback evidence;
5. continue MT-404 Boolean and cut rollback evidence;
6. continue MT-405 fillet rollback evidence.

The custom MT-402 modeling harness is preserved on the unmerged
`archive/mt402-superseded-modeling-harness` branch at `568c3c4f`. The accepted
foundation is the topology-owned validation recorded in `TRACKING-SCOPE.md`.
Repeated-edge discrimination, NaN-payload repeatability, and possible
six-type coverage may be extracted later as focused topology-owned probes;
parallel projections, result state, and mutable-state models must not be
restored without the approved downstream layers.

## Later kernel maturity work

After the upstream solver and downstream replay boundaries are stable:

- enforce bounded persisted-data decoding and schema evolution;
- establish supported Wasm runtime evidence rather than compile-only claims;
- broaden intersection, topology-healing, Boolean, and meshing corpora;
- record reproducible native and Wasm performance evidence.

## Dependency order

1. Keep MT-901's merged authority fixed at `06201787` until upstream advances.
2. Keep completed MT-902 synchronized if upstream advances.
3. Complete MT-903 and MT-904 on the resulting current `upstream/master`.
4. Reconcile the accepted result through the proving grounds governed by
   `AGENTS.md`.
5. Begin tracking, persistence, contracts, and replay only after upstream
   solver acceptance.
6. Re-evaluate the preserved MT-402 probes before starting MT-403 through MT-405.
7. Expand broader kernel maturity work from stable, revision-bounded evidence.

## Worktree disposition

- Use `monstertruck-workbench` as the active construction workspace. Keep
  branch-specific proving-ground work in the dedicated `dev` and `master`
  worktrees; the primary `monstertruck` checkout retains shared Git metadata
  but is not an integration target.
- Keep dedicated `dev` and `master` worktrees for their distinct proving-ground
  responsibilities. Create an upstream-contribution worktree only when a clean
  recut is ready to start from current `upstream/master`.
- Preserve the production-`G3`, plan-review, and superseded MT-402 work as
  branch refs at `8c4ec61c`, `e1708d66`, and `568c3c4f`; they do not require
  permanent worktrees.
- Retire a worktree only after its unique work is preserved and its replacement
  revision is recorded here or in the phase plan.
