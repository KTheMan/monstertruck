# Continuity validation corpus

The versioned corpora exercise the public boundary-continuity solver against
procedurally generated multi-span quintic polynomial and rational NURBS
surfaces. It covers aligned, unequal, and reversed seam parameterizations,
every ordered pairing of `MinU`, `MaxU`, `MinV`, and `MaxV` in both alignments,
scale variants, experimental G4 reachability, and structured failure cases.

The preserved `v1` corpus supplies broad orientation and experimental coverage.
The default `v2` production-G3 gate adds changed-solution repair evidence for
aligned, reversed, unequal, and unequal-reversed seams. It includes a valid
degree-five seam with a repeated interior knot and positive rational weights
spanning `2^-10..=2^10` at scales `10^-3`, `1`, and `10^3`. Explicit iteration
and QR limits verify exact typed truncation through the bounded-solve carrier.

Successful cases are independently certified at 33 span-distributed Chebyshev
seam coordinates, both endpoints, and every mapped span boundary from both the
first and second seams. Mandatory endpoints, knot boundaries, and some
interior coordinates may overlap solver collocation locations. The certifier
uses the public solved transition and public surface evaluation, but does not
reuse solver residuals, automatic derivatives, Jacobians, or the convergence
decision. Each coordinate uses separate `9 x 9` finite-difference tensor
stencils, including for G4. Seam stencils are centered in span interiors and
one-sided at domain endpoints.
Cross-seam stencils are one-sided within each surface domain and use the
solver's signed common cross coordinate. The certifier records absolute and
fixture-scale-normalized residuals from the solved public boundary transition
and public surface evaluation. It checks every mixed derivative through the
requested order.

Each case runs twice in the same process. Raw solved surfaces, public
transition samples, report, and independent metrics are hashed with the
deterministic `ContentHasher`. Immediate reruns must produce the same digest.
Transition fingerprint samples stay within `-0.04..=0.04` of the seam, where
the order-truncated local map is intended to be evaluated.
Both run times and solver work counters are emitted; elapsed time is diagnostic
only and is excluded from equality and digest comparisons.

The evidence schema is example-local. It converts public enums, reports, and
errors into observation values without making their Rust types a serialization
or persistence contract.

Emit reviewed observations:

```powershell
cargo run -p monstertruck-geometry --example continuity-validation -- `
  --emit target/continuity-validation-observed.json
```

The runner does not commit host-specific digest expectations. CI exercises the
same emit path and requires each case's two immediate runs, outcome, dense
certificate, digest, and deterministic work units to agree.
