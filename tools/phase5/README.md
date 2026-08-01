# Phase 5 OCCT evidence tools

These tools generate provenance-clean continuity fixtures and inspect STEP
results with an implementation independent from `monstertruck`.

Create the isolated environment:

```powershell
python -m venv target/phase5-python
target/phase5-python/Scripts/python.exe -E -m pip install `
  -r tools/phase5/requirements-occt.txt
```

Generate the fixtures and their content-addressed manifest:

```powershell
target/phase5-python/Scripts/python.exe -E `
  tools/phase5/generate_occt_fixtures.py
```

Validate the checked-in inputs:

```powershell
target/phase5-python/Scripts/python.exe -E `
  tools/phase5/validate_step_occt.py `
  validation/continuity/polynomial-g1.step `
  validation/continuity/rational-reversed-g2.step `
  validation/continuity/quintic-g3.step `
  validation/continuity/arbitrary-trim-negative.step `
  --output validation/continuity/occt-validation.json
```

Validate the repaired Monstertruck output separately:

```powershell
target/phase5-python/Scripts/python.exe -E `
  tools/phase5/validate_step_occt.py `
  validation/continuity/monstertruck-polynomial-g1-solved.step `
  --output validation/continuity/monstertruck-output-occt.json
```

Use `-E` on Windows so an ambient `PYTHONPATH` cannot shadow the pinned OCP
wheel. Both scripts fail if the runtime wrapper is not exactly `7.8.1.1`.

The generated STEP header uses a fixed timestamp. Repeated generation with the
same pinned runtime therefore produces the hashes recorded in `manifest.json`.
The fixture geometry and scripts are distributed under the repository license.

## Independent Monstertruck seam certificates

Run the public imported workflow and emit versioned JSON receipts:

```powershell
cargo run -p monstertruck-step --example continuity-step-validation -- `
  validation/continuity/polynomial-g1.step --order 1 `
  --output target/continuity-certification/polynomial-g1-solved.step `
  --receipt validation/continuity/monstertruck-polynomial-g1-certificate.json

cargo run -p monstertruck-step --example continuity-step-validation -- `
  validation/continuity/rational-reversed-g2.step --order 2 `
  --output target/continuity-certification/rational-reversed-g2-solved.step `
  --receipt validation/continuity/monstertruck-rational-reversed-g2-certificate.json

cargo run -p monstertruck-step --example continuity-step-validation -- `
  validation/continuity/quintic-g3.step --order 3 `
  --output target/continuity-certification/quintic-g3-solved.step `
  --receipt validation/continuity/monstertruck-quintic-g3-certificate.json
```

The certificate samples points through public surface evaluation and estimates
every mixed derivative through the requested order with finite differences. It
uses the public solved transition but does not reuse solver residuals or the
solver convergence decision. The default normalized per-order limits are
`1e-9`, `1e-7`, `1e-5`, and `1e-3` through order three.
