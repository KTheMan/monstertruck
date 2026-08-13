# Imported continuity fixture

`continuity-g3.step` is the deterministic degree-five, two-face, full-side
fixture used by the imported `G3` continuity evidence path. It is generated
from the established minimal STEP topology fixture. The generator raises both
tensor-product degrees without changing the represented planes and normalizes
the STEP header timestamp.

Regenerate it from the repository root with:

```console
cargo run -p monstertruck-io --example generate-continuity-g3-fixture
```

Verify that the checked-in fixture matches its provenance path with:

```console
cargo run -p monstertruck-io --example generate-continuity-g3-fixture -- --check
```
