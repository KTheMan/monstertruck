# Continuity validation corpus

The versioned corpus exercises the public boundary-continuity solver against
procedurally generated multi-span quintic polynomial and rational NURBS
surfaces. It covers aligned, unequal, and reversed seam parameterizations,
scale variants, experimental G4 reachability, and structured failure cases.

Successful cases are independently certified at 33 disjoint Chebyshev seam
coordinates, both endpoints, and every mapped span boundary from both the first
and second seams. Each coordinate uses a `9 x 9` finite-difference tensor
stencil, including for G4. Seam stencils are centered in span interiors and
one-sided at domain endpoints; cross-seam stencils are centered. The certifier
records absolute and fixture-scale-normalized residuals from the solved public
boundary transition and public surface evaluation. It checks every mixed
derivative through the requested order without reusing solver collocation
points, automatic derivatives, Jacobians, or internal validation residuals.

Each case runs twice in the same process. Raw solved surfaces, public
transition samples, report, and independent metrics are hashed with the
deterministic `ContentHasher`. Immediate reruns must produce the same digest.
Both run times and solver work counters are emitted; elapsed time is diagnostic
only and is excluded from equality and digest comparisons.

Emit reviewed observations:

```powershell
cargo run -p monstertruck-geometry --example continuity-validation -- `
  --emit target/continuity-validation-observed.json
```

Verify the committed baseline:

```powershell
cargo run -p monstertruck-geometry --example continuity-validation -- --verify
```

The runner never rewrites the committed baseline. Review emitted metrics and
copy approved digests into `v1/baseline.json`.

Run emit and verify as separate commands. The fresh-process verify, together
with each process's immediate rerun check, is the cross-process determinism
receipt.

`v1/evidence-windows-msvc.json` is the unchanged raw reviewed full-run receipt.
It records absolute and normalized residuals, worst locations and mixed
derivatives, solver work counters, and both per-case run times. The recorded
full emit took 323.4 seconds on Windows `x86_64-pc-windows-msvc` with Rust
1.94.0 and LLVM 21.1.8.
