# Tracked Modeling Wrapper Failure-Point Matrix, Version 1

## Audit metadata

| Field | Value |
| --- | --- |
| Story | `MT-401` |
| Baseline | `4054cc81` (`4054cc8122b4a69776664caf7eb948aedfaaa906`) |
| Audit date | 2026-08-03 |
| Platform | `x86_64-pc-windows-msvc` |
| Toolchain | `rustc 1.94.0 (4a4ef493e 2026-03-02)`, LLVM `21.1.8` |
| Evidence class | `Implemented` -- source-audit evidence only |
| Coverage result | All 18 unique public wrapper functions matched the expected set. |
| Audited package features | `monstertruck-modeling` default, `solid`, and `fillet`; `fillet` enables `solid`. |
| Audited package manifest | `monstertruck-modeling/Cargo.toml` -- SHA-256 `e8144574a200aad7a16a3ab74b1711fa35ff850384ef0c9b673ec5ef4ed71b19` |
| Audited module exposure | `monstertruck-modeling/src/lib.rs` -- SHA-256 `d9fcaa07d3ecf2f7ab18a08402557c1294ef1717f5bfa24b53f35fefcf1928e9` |
| Audited wrapper source | `monstertruck-modeling/src/tracked.rs` -- SHA-256 `0fcf0e3756fc424cebc75fda2df0da2d145944fc89fd775290d62cbc99258ce1` |
| Audited session source | `monstertruck-core/src/tracking/session.rs` -- SHA-256 `723a1b053d359cadc66e60ea730bc979591464ce2e95ee997bb5ba5c31741558` |
| Audited tracking interface | `monstertruck-topology/src/tracking/interface.rs` -- SHA-256 `7df2df51a4baef2f829378d14a717136a22cac449da73bbbc036be5514da7022` |
| Audited tracking initialization | `monstertruck-topology/src/tracking/initialization.rs` -- SHA-256 `9ec976a7a575975a44cce4c2e3861e7f64d1df1fa2674b68ed93987378704c19` |
| Audited topology tracking implementations | `monstertruck-topology/src/tracking/implementations.rs` -- SHA-256 `33b0d97da1c5d066927efd118c23de10e2340afc4a83c5225d8256e4289a3879` |
| Audited shape-operation errors | `monstertruck-solid/src/transversal/integrate/mod.rs` -- SHA-256 `e3bad7a4b7ff236d1bbc1c1b6f2a2144c8210e4b04a59d4f941bca2009634ad5` |
| Audited fillet errors | `monstertruck-solid/src/fillet/error.rs` -- SHA-256 `2297592fbb8cba3db7e0dafd9d9d167248d3725e7e55712d869e8b442dfb9a0b` |
| Audited fillet adapter | `monstertruck-solid/src/fillet/edge_select.rs` -- SHA-256 `89e108e780c6efe0e9b41aec73a63f2e79ff1f941b15e79488e293f7c19e0a0b` |

This inventory records the public tracked-modeling surface and its fallible
source paths. It does not inject any failure and does not procedurally validate
rollback, transactionality, or atomicity. The matrix is input to the complete
snapshot work in `MT-402` and the executable failure cases in `MT-403` through
`MT-405`.

## Failure-point codes

| Code | Failure point | Typed result at the tracked wrapper boundary |
| --- | --- | --- |
| `FP-SOURCE-EMPTY` | `current_ids` observes no tracking IDs on a required source. | `Error::UntrackedSource`. |
| `FP-SOURCE-CURRENT` | `current_ids` rejects an ID that is from the wrong session, stale generation, or outside the allocated serial range. | `Error::Tracking(TrackingError::WrongSession { .. })`, `Error::Tracking(TrackingError::StaleGeneration { .. })`, or `Error::Tracking(TrackingError::UnknownTrackingId(_))`. |
| `FP-RAW-SHAPE` | A solid Boolean or cut implementation rejects the raw modeling operation. | `Error::ShapeOperation(String)`. |
| `FP-RAW-FILLET` | The generic fillet implementation rejects the cloned shell. | `Error::Fillet(String)`. |
| `FP-OUTPUT-TRACKING` | `TopologyTracking::initialize_tracking` rejects existing IDs, allocation, a semantic binding, or another tracking invariant while finalizing a local output. | `Error::Tracking(TrackingError)`. |
| `FP-OUTPUT-IDS` | `record_preserved` rejects missing or non-current tracking IDs on a locally mapped output. | `Error::UntrackedSource`, `Error::Tracking(TrackingError::WrongSession { .. })`, `Error::Tracking(TrackingError::StaleGeneration { .. })`, or `Error::Tracking(TrackingError::UnknownTrackingId(_))`. |
| `FP-IDENTITY` | A topology-preserving map changes the ordered tracking-ID set. | `Error::IdentityMismatch`. |
| `FP-LINEAGE` | `TrackingSession::record_lineage` rejects a parent, child, relation, or uniqueness invariant. | `Error::Tracking(TrackingError)`. |
| `FP-SECTION-MATCH` | A plane-cut section face cannot be matched to the finalized result solid. | `Error::SectionTrackingMismatch`. |

`FP-OUTPUT-TRACKING` includes allocation and binding failures such as
`TrackingError::SerialOverflow`, `TrackingError::SemanticReferenceAlreadyBound`,
and `TrackingError::TrackingIdAlreadyBound`. The exact reachable subtype
depends on the topology implementation. `FP-LINEAGE` includes current-ID
validation and lineage-shape errors such as `TrackingError::EmptyLineage`,
`TrackingError::DuplicateLineageChild`, and
`TrackingError::DeletedLineageHasChildren`.

## Caller-visible snapshot components

The following component codes identify state that `MT-402` must capture
canonically. A component's presence means that a failed wrapper must be checked
against it; it does not claim that this audit exercised or proved preservation.

| Code | Caller-visible component |
| --- | --- |
| `CV-TOPOLOGY` | Topology identities, deterministic topology order and connectivity, and assigned `TrackingId` values for every source topology. |
| `CV-GEOMETRY` | Canonical geometry signatures for every source topology. |
| `CV-SESSION` | `TrackingSession` identifier, generation, and next serial. |
| `CV-BINDINGS` | Ordered semantic references and their tracking-ID bindings. |
| `CV-LINEAGE` | Ordered lineage events, including operation, relation, parent, and ordered children. |
| `CV-RESULT` | The typed result and whether any local output becomes caller-visible. |
| `CV-MUTABLE` | The complete value of a caller-owned mutable topology argument. This currently applies to the shell accepted by `fillet_edges`. |

## Wrapper matrix

The feature value `default` means that the wrapper has no feature gate in the
audited source. `fillet` enables `solid` in `monstertruck-modeling`.

| Public wrapper | Feature / deprecation gate | Sources and mutability | Raw modeling stage | Failure codes and boundary errors | Staging and commit boundary | Caller-visible state at risk | Follow-up owner |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `transformed` | `default`. | One immutable generic topology; mutable session. | `builder::transformed` creates a local output and has no typed raw-stage error. | `FP-SOURCE-EMPTY`, `FP-SOURCE-CURRENT`, `FP-OUTPUT-IDS`, `FP-IDENTITY`, `FP-LINEAGE`. | Source validation and raw mapping precede staging. Inside the cloned session, `record_preserved` revalidates the local output's IDs, compares its ordered IDs with the source, and records preserved lineage. The session commits only after all three steps succeed; the output is returned after commit. | `CV-TOPOLOGY`, `CV-GEOMETRY`, `CV-SESSION`, `CV-BINDINGS`, `CV-LINEAGE`, `CV-RESULT`. | `MT-402` / `MT-403`. |
| `cloned` | `default`. | One immutable generic topology; mutable session. | `builder::clone` creates a local output and has no typed raw-stage error. | `FP-SOURCE-EMPTY`, `FP-SOURCE-CURRENT`, `FP-OUTPUT-IDS`, `FP-IDENTITY`, `FP-LINEAGE`. | Source validation and raw mapping precede staging. Inside the cloned session, `record_preserved` revalidates the local output's IDs, compares its ordered IDs with the source, and records preserved lineage. The session commits only after all three steps succeed; the output is returned after commit. | `CV-TOPOLOGY`, `CV-GEOMETRY`, `CV-SESSION`, `CV-BINDINGS`, `CV-LINEAGE`, `CV-RESULT`. | `MT-402` / `MT-403`. |
| `translated` | `default`. | One immutable generic topology; mutable session. | `builder::translated` creates a local output and has no typed raw-stage error. | `FP-SOURCE-EMPTY`, `FP-SOURCE-CURRENT`, `FP-OUTPUT-IDS`, `FP-IDENTITY`, `FP-LINEAGE`. | Source validation and raw mapping precede staging. Inside the cloned session, `record_preserved` revalidates the local output's IDs, compares its ordered IDs with the source, and records preserved lineage. The session commits only after all three steps succeed; the output is returned after commit. | `CV-TOPOLOGY`, `CV-GEOMETRY`, `CV-SESSION`, `CV-BINDINGS`, `CV-LINEAGE`, `CV-RESULT`. | `MT-402` / `MT-403`. |
| `rotated` | `default`. | One immutable generic topology; mutable session. | `builder::rotated` creates a local output and has no typed raw-stage error. | `FP-SOURCE-EMPTY`, `FP-SOURCE-CURRENT`, `FP-OUTPUT-IDS`, `FP-IDENTITY`, `FP-LINEAGE`. | Source validation and raw mapping precede staging. Inside the cloned session, `record_preserved` revalidates the local output's IDs, compares its ordered IDs with the source, and records preserved lineage. The session commits only after all three steps succeed; the output is returned after commit. | `CV-TOPOLOGY`, `CV-GEOMETRY`, `CV-SESSION`, `CV-BINDINGS`, `CV-LINEAGE`, `CV-RESULT`. | `MT-402` / `MT-403`. |
| `scaled` | `default`. | One immutable generic topology; mutable session. | `builder::scaled` creates a local output and has no typed raw-stage error. | `FP-SOURCE-EMPTY`, `FP-SOURCE-CURRENT`, `FP-OUTPUT-IDS`, `FP-IDENTITY`, `FP-LINEAGE`. | Source validation and raw mapping precede staging. Inside the cloned session, `record_preserved` revalidates the local output's IDs, compares its ordered IDs with the source, and records preserved lineage. The session commits only after all three steps succeed; the output is returned after commit. | `CV-TOPOLOGY`, `CV-GEOMETRY`, `CV-SESSION`, `CV-BINDINGS`, `CV-LINEAGE`, `CV-RESULT`. | `MT-402` / `MT-403`. |
| `sweep` | `default`. | One immutable generic topology; mutable session. | `topology.sweep` creates a local output and has no typed raw-stage error. | `FP-SOURCE-EMPTY`, `FP-SOURCE-CURRENT`, `FP-OUTPUT-TRACKING`, `FP-LINEAGE`. | Source validation and raw construction precede staging. Output tracking and generated-lineage recording use a cloned session. A failed closure leaves the session unchanged and the mutated local output unpublished; success commits the session before returning the output. | `CV-TOPOLOGY`, `CV-GEOMETRY`, `CV-SESSION`, `CV-BINDINGS`, `CV-LINEAGE`, `CV-RESULT`. | `MT-402` / `MT-403`. |
| `extrude` | `default`. | One immutable generic topology; mutable session. | `builder::extrude` creates a local output and has no typed raw-stage error. | `FP-SOURCE-EMPTY`, `FP-SOURCE-CURRENT`, `FP-OUTPUT-TRACKING`, `FP-LINEAGE`. | Source validation and raw construction precede staging. Output tracking and generated-lineage recording use a cloned session. A failed closure leaves the session unchanged and the mutated local output unpublished; success commits the session before returning the output. | `CV-TOPOLOGY`, `CV-GEOMETRY`, `CV-SESSION`, `CV-BINDINGS`, `CV-LINEAGE`, `CV-RESULT`. | `MT-402` / `MT-403`. |
| `revolve` | `default`. | One immutable generic topology; mutable session. | `builder::revolve` creates a local output and has no typed raw-stage error. | `FP-SOURCE-EMPTY`, `FP-SOURCE-CURRENT`, `FP-OUTPUT-TRACKING`, `FP-LINEAGE`. | Source validation and raw construction precede staging. Output tracking and generated-lineage recording use a cloned session. A failed closure leaves the session unchanged and the mutated local output unpublished; success commits the session before returning the output. | `CV-TOPOLOGY`, `CV-GEOMETRY`, `CV-SESSION`, `CV-BINDINGS`, `CV-LINEAGE`, `CV-RESULT`. | `MT-402` / `MT-403`. |
| `revolve_wire` | `default`. | One immutable `Wire`; mutable session. | `builder::revolve_wire` creates a local `Shell` and has no typed raw-stage error. | `FP-SOURCE-EMPTY`, `FP-SOURCE-CURRENT`, `FP-OUTPUT-TRACKING`, `FP-LINEAGE`. | Source validation and raw construction precede staging. Output tracking and generated-lineage recording use a cloned session. A failed closure leaves the session unchanged and the mutated local shell unpublished; success commits the session before returning the shell. | `CV-TOPOLOGY`, `CV-GEOMETRY`, `CV-SESSION`, `CV-BINDINGS`, `CV-LINEAGE`, `CV-RESULT`. | `MT-402` / `MT-403`. |
| `cone` | `default`; deprecated in favor of `revolve_wire`. | One immutable `Wire`; mutable session. | Reads the front-vertex origin, or uses the zero point for an empty wire, then delegates to `revolve_wire`. | `FP-SOURCE-EMPTY`, `FP-SOURCE-CURRENT`, `FP-OUTPUT-TRACKING`, `FP-LINEAGE`, inherited from `revolve_wire`. | Delegation inherits `revolve_wire` staging: source validation and construction precede a cloned-session transaction; the session and local shell publish only after success. | `CV-TOPOLOGY`, `CV-GEOMETRY`, `CV-SESSION`, `CV-BINDINGS`, `CV-LINEAGE`, `CV-RESULT`. | `MT-402` / `MT-403`. |
| `and` | `solid`. | Two immutable `Solid` values; mutable session. | `monstertruck_solid::and` creates a local result solid. | `FP-SOURCE-EMPTY`, `FP-SOURCE-CURRENT`, `FP-RAW-SHAPE`, `FP-OUTPUT-TRACKING`, `FP-LINEAGE`. | Both sources are validated and the raw Boolean completes before staging. `finalize_output` mutates the local solid and records lineage on a cloned session. Success commits the session before returning the solid; failure leaves the output unpublished. | Both sources' `CV-TOPOLOGY` and `CV-GEOMETRY`; `CV-SESSION`, `CV-BINDINGS`, `CV-LINEAGE`, `CV-RESULT`. | `MT-402` / `MT-404`. |
| `and_with_orientation_hints` | `solid`. | Two immutable `Solid` values and immutable orientation hints; mutable session. | `monstertruck_solid::and_with_orientation_hints` creates a local result solid. | `FP-SOURCE-EMPTY`, `FP-SOURCE-CURRENT`, `FP-RAW-SHAPE`, `FP-OUTPUT-TRACKING`, `FP-LINEAGE`. | Both sources are validated and the raw Boolean completes before staging. `finalize_output` mutates the local solid and records lineage on a cloned session. Success commits the session before returning the solid; failure leaves the output unpublished. | Both sources' `CV-TOPOLOGY` and `CV-GEOMETRY`; `CV-SESSION`, `CV-BINDINGS`, `CV-LINEAGE`, `CV-RESULT`. | `MT-402` / `MT-404`. |
| `or` | `solid`. | Two immutable `Solid` values; mutable session. | `monstertruck_solid::or` creates a local result solid. | `FP-SOURCE-EMPTY`, `FP-SOURCE-CURRENT`, `FP-RAW-SHAPE`, `FP-OUTPUT-TRACKING`, `FP-LINEAGE`. | Both sources are validated and the raw Boolean completes before staging. `finalize_output` mutates the local solid and records lineage on a cloned session. Success commits the session before returning the solid; failure leaves the output unpublished. | Both sources' `CV-TOPOLOGY` and `CV-GEOMETRY`; `CV-SESSION`, `CV-BINDINGS`, `CV-LINEAGE`, `CV-RESULT`. | `MT-402` / `MT-404`. |
| `difference` | `solid`. | Two immutable `Solid` values; mutable session. | `monstertruck_solid::difference` creates a local result solid. | `FP-SOURCE-EMPTY`, `FP-SOURCE-CURRENT`, `FP-RAW-SHAPE`, `FP-OUTPUT-TRACKING`, `FP-LINEAGE`. | Both sources are validated and the raw Boolean completes before staging. `finalize_output` mutates the local solid and records lineage on a cloned session. Success commits the session before returning the solid; failure leaves the output unpublished. | Both sources' `CV-TOPOLOGY` and `CV-GEOMETRY`; `CV-SESSION`, `CV-BINDINGS`, `CV-LINEAGE`, `CV-RESULT`. | `MT-402` / `MT-404`. |
| `symmetric_difference` | `solid`. | Two immutable `Solid` values; mutable session. | `monstertruck_solid::symmetric_difference` creates a local result solid. | `FP-SOURCE-EMPTY`, `FP-SOURCE-CURRENT`, `FP-RAW-SHAPE`, `FP-OUTPUT-TRACKING`, `FP-LINEAGE`. | Both sources are validated and the raw Boolean completes before staging. `finalize_output` mutates the local solid and records lineage on a cloned session. Success commits the session before returning the solid; failure leaves the output unpublished. | Both sources' `CV-TOPOLOGY` and `CV-GEOMETRY`; `CV-SESSION`, `CV-BINDINGS`, `CV-LINEAGE`, `CV-RESULT`. | `MT-402` / `MT-404`. |
| `clip_half_space_z` | `solid`. | One immutable `Solid`; mutable session. | `monstertruck_solid::clip_half_space_z` creates a local result solid. | `FP-SOURCE-EMPTY`, `FP-SOURCE-CURRENT`, `FP-RAW-SHAPE`, `FP-OUTPUT-TRACKING`, `FP-LINEAGE`. | The source is validated and raw clipping completes before staging. `finalize_output` mutates the local solid and records lineage on a cloned session. Success commits the session before returning the solid; failure leaves the output unpublished. | `CV-TOPOLOGY`, `CV-GEOMETRY`, `CV-SESSION`, `CV-BINDINGS`, `CV-LINEAGE`, `CV-RESULT`. | `MT-402` / `MT-404`. |
| `plane_cut` | `solid`. | One immutable `Solid`; mutable session. | `monstertruck_solid::plane_cut` creates a local result solid and section. | `FP-SOURCE-EMPTY`, `FP-SOURCE-CURRENT`, `FP-RAW-SHAPE`, `FP-OUTPUT-TRACKING`, `FP-LINEAGE`, `FP-SECTION-MATCH`. | The source is validated and raw cutting completes before staging. The cloned-session closure finalizes `output.solid` and matches every section face. Any finalization or matching failure leaves the session unchanged and local output unpublished. Success commits the session, then replaces the section with matched tracked faces before returning. | `CV-TOPOLOGY`, `CV-GEOMETRY`, `CV-SESSION`, `CV-BINDINGS`, `CV-LINEAGE`, `CV-RESULT`. | `MT-402` / `MT-404`. |
| `fillet_edges` | `fillet`, which enables `solid`. | One mutable `Shell`, an immutable selected-edge slice, and a mutable session. | Clones the shell, then `monstertruck_solid::fillet_edges_generic` mutates only the local clone. | `FP-SOURCE-EMPTY`, `FP-SOURCE-CURRENT`, `FP-RAW-FILLET`, `FP-OUTPUT-TRACKING`, `FP-LINEAGE`. | The shell and every selected edge are validated before cloning. Raw filleting changes only the clone. Finalization uses a cloned session. The session commits after successful finalization, and the infallible shell assignment publishes the local output last. | The shell and selected edges' `CV-TOPOLOGY` and `CV-GEOMETRY`; `CV-SESSION`, `CV-BINDINGS`, `CV-LINEAGE`, `CV-RESULT`, `CV-MUTABLE`. | `MT-402` / `MT-405`. |

## Reproducible coverage check

Run the following from the repository root in PowerShell. Empty
`Compare-Object` output and the final count of `18` establish that this
inventory's expected public-wrapper set matches the audited source surface.

```powershell
$path = 'monstertruck-modeling/src/tracked.rs'
$expected = @(
    'and',
    'and_with_orientation_hints',
    'clip_half_space_z',
    'cloned',
    'cone',
    'difference',
    'extrude',
    'fillet_edges',
    'or',
    'plane_cut',
    'revolve',
    'revolve_wire',
    'rotated',
    'scaled',
    'sweep',
    'symmetric_difference',
    'transformed',
    'translated'
) | Sort-Object
$actual = Select-String -Path $path -Pattern '^pub fn ([a-z_][a-z0-9_]*)' |
    ForEach-Object { $_.Matches[0].Groups[1].Value } |
    Sort-Object
$difference = Compare-Object -ReferenceObject $expected -DifferenceObject $actual
$difference
if ($difference -or $actual.Count -ne 18) {
    throw "tracked wrapper inventory mismatch: expected 18, found $($actual.Count)"
}
$actual.Count
```

The audited source digests can be reproduced with:

```powershell
Get-FileHash -Algorithm SHA256 @(
    'monstertruck-modeling/Cargo.toml',
    'monstertruck-modeling/src/lib.rs',
    'monstertruck-modeling/src/tracked.rs',
    'monstertruck-core/src/tracking/session.rs',
    'monstertruck-topology/src/tracking/interface.rs',
    'monstertruck-topology/src/tracking/initialization.rs',
    'monstertruck-topology/src/tracking/implementations.rs',
    'monstertruck-solid/src/transversal/integrate/mod.rs',
    'monstertruck-solid/src/fillet/error.rs',
    'monstertruck-solid/src/fillet/edge_select.rs'
)
```

## Relevant Cargo verification

No Rust source, test, or expected output is changed by this inventory. The
feature-complete focused package gates passed on the audited tree:

```text
cargo test -p monstertruck-modeling --lib --features fillet tracked::tests
# 5 passed; 0 failed; 23 filtered out.

cargo clippy -p monstertruck-modeling --all-targets --features fillet -- -W warnings
# Passed without warnings.
```

`cargo fmt --all` remains a required pre-commit gate when a commit is
authorized. It was not run for this documentation-only inventory.

## Evidence limit and next action

This matrix establishes only that the audited implementation exposes 18
tracked wrappers and identifies their visible state, error propagation, and
commit structure. It does not establish that all listed errors are reachable
through supported public inputs. It does not demonstrate late failure after
partial staged work. It does not compare pre-operation and post-failure state.
Accordingly, no rollback or atomicity behavior is procedurally validated by
this artifact.

`MT-402` must define and implement deterministic complete snapshots for the
listed `CV-*` components. `MT-403`, `MT-404`, and `MT-405` must then use those
snapshots to produce executable late-failure and rollback evidence for their
assigned wrapper families.
