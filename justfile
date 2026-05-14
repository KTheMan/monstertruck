set shell := ["bash", "-euo", "pipefail", "-c"]

# Crates exercised by `test-cpu` (anything that doesn't need a GPU).
cpu_crates := "-p monstertruck-core -p monstertruck-traits -p monstertruck-geometry -p monstertruck-topology -p monstertruck-mesh -p monstertruck-meshing -p monstertruck-modeling -p monstertruck-solid -p monstertruck-step"

# Crates exercised by `test-gpu`.
gpu_crates := "-p monstertruck-gpu -p monstertruck-render"

# RUSTFLAGS required for the wasm32 target.
wasm_rustflags := '--cfg=web_sys_unstable_apis --cfg=getrandom_backend="wasm_js"'

# Default: show available recipes.
default:
    @just --list

# Aggregate: what CI runs.
ci: fmt-check lint-check test-cpu meshing-features

# Format code.
fmt:
    cargo fmt --all

# Verify formatting without writing.
fmt-check:
    cargo fmt --all -- --check

# Run clippy with autofix (modifies working tree).
lint:
    cargo clippy --fix --allow-dirty --allow-staged --all-targets -- -D warnings

# Run clippy without fixing (CI-safe).
lint-check:
    cargo clippy --all-targets -- -D warnings

# Run CPU-only tests on the stable toolchain.
test-cpu:
    cargo test {{ cpu_crates }} --features derive --features polynomial -- --nocapture

# Run CPU-only tests on the nightly toolchain.
test-cpu-nightly:
    rustup run nightly cargo test {{ cpu_crates }} --features derive --features polynomial -- --nocapture

# Run GPU tests (requires a working GPU).
test-gpu:
    cargo test {{ gpu_crates }} -- --nocapture --test-threads=1

# Feature subset build checks for `monstertruck-meshing`.
meshing-features:
    cargo check -p monstertruck-meshing --no-default-features --features analyzers
    cargo check -p monstertruck-meshing --no-default-features --features filters
    cargo check -p monstertruck-meshing --no-default-features --features tessellation

# Build the workspace for the `wasm32-unknown-unknown` target.
wasm-build:
    RUSTFLAGS='{{ wasm_rustflags }}' cargo build --target=wasm32-unknown-unknown

# Build the workspace for wasm32 with the `webgl` feature.
webgl-build:
    RUSTFLAGS='{{ wasm_rustflags }}' cargo build --target=wasm32-unknown-unknown --features webgl

# Build and run the JS/Deno tests for `monstertruck-wasm`.
wasm-js-test:
    RUSTFLAGS='--cfg=getrandom_backend="wasm_js"' bash -c '\
        cd monstertruck-wasm && \
        wasm-pack build --target web && \
        deno test -A tests/'

# Full wasm test suite: wasm32 build + webgl build + JS tests.
wasm-test: wasm-build webgl-build wasm-js-test

# Build the ad-hoc viewer (wasm-pack + bootstrap files).
adhoc-viewer:
    RUSTFLAGS='--cfg=getrandom_backend="wasm_js"' bash -c '\
        cd monstertruck-wasm && \
        wasm-pack build --target web && \
        cp examples/index.html pkg/ && \
        cp examples/bootstrap.js pkg/ && \
        cp examples/script.js pkg/'

# Build the WebGPU example pages.
wgpu-examples:
    RUSTFLAGS='{{ wasm_rustflags }}' cargo run --bin example-pages-generator

# Build everything that ships in the GitHub Pages site.
page-build: adhoc-viewer wgpu-examples

# Generate shape JSON fixtures used by examples and tests.
create-shape-json:
    cd resources/shape && \
    cargo run -p monstertruck-modeling --example bottle && \
    cargo run -p monstertruck-modeling --example cube && \
    cargo run -p monstertruck-modeling --example cylinder && \
    cargo run -p monstertruck-modeling --example punched-cube && \
    cargo run -p monstertruck-modeling --example torus-punched-cube && \
    cargo run -p monstertruck-modeling --example cube-in-cube && \
    cargo run -p monstertruck-modeling --example torus && \
    cargo run -p monstertruck-modeling --example sphere && \
    cargo run -p monstertruck-modeling --example torus -- 500 100 large-torus.json && \
    cargo run -p monstertruck-solid --example punched-cube-shapeops

# Build rustdoc for every crate (no deps).
doc:
    cargo doc --no-deps --workspace

# Regenerate per-crate READMEs and fail if anything changed.
readme-check:
    cargo run --bin readme-generator
    git diff --exit-code

# Wipe target directory.
clean:
    cargo clean
