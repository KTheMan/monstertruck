# Repository Guidelines

## Blueprint Baseline

This repo adopts the shared [`blueprints`](https://github.com/virtualritz/blueprints)
baseline via the `.blueprints/` git submodule. Those files are the cross-project
default for agent behavior, code quality, and microtypography; the
project-specific rules in this file take precedence wherever they conflict.

Core rules (read first):

- [Agent behavior](.blueprints/base/AGENTS.md)
- [Rust agent rules](.blueprints/lang/rust/AGENTS.md) and [Rust testing](.blueprints/lang/rust/testing.md)
- [Git safety](.blueprints/base/git-safety.md) and [test ownership](.blueprints/base/test-ownership.md)

Reference:

- [Microtypography](.blueprints/base/microtypography.md) -- slashes, dashes, quotes, ellipses, spacing.
- [Documentation standards](.blueprints/base/documentation.md)
- [Commit messages](.blueprints/base/commit-messages.md)
- [API change protocol](.blueprints/base/api-changes.md)

## Build, Test, and Development Commands

**Use `cargo nextest run` for ordinary tests, `cargo test --doc` for doctests,
and `cargo run` for executable validation. NEVER use `cargo test` for ordinary
tests. NEVER use `cargo check` or `cargo build` for verification.**

**NEVER run `cargo clean` without asking the user first.**

```bash
# Run tests (this also builds everything)
cargo nextest run -p monstertruck-geometry --lib

# Run all core tests
just test-cpu

# Run a specific test
cargo nextest run -p monstertruck-geometry test_name

# Run doctests separately (`nextest` does not run them)
cargo test -p monstertruck-geometry --doc

# Format code
cargo fmt --all

# Run clippy linter
cargo clippy --all-targets -- -W warnings

# IMPORTANT: NEVER use --release flag unless the user EXPLICITLY requests it.
```

## Coding Style & Naming Conventions

- Follow standard Rust style: four-space indentation, `snake_case` for modules/functions, `CamelCase` for types.
- Write idiomatic and canonical Rust code. Avoid patterns common in imperative languages like C/C++/JS/TS that can be expressed more elegantly in Rust.
- Keep public APIs documented with `///` comments.
- Run `cargo fmt --all` before committing.

### Functional Style (CRITICAL)

- PREFER functional style over imperative style. Use `for_each` or `map` instead of for loops, use `collect` instead of pre-allocating a `Vec` and using `push`.
- PREFER direct initialization of collections. Use `BTreeMap::from([...])` or `vec![...]` instead of `new()` followed by `insert()`/`push()`.
- PREFER iterator chains over indexing into pre-allocated vectors. If you find yourself writing `vec![0.0; n]` followed by `for i in 0..n { v[i] = expr }`, rewrite as `(0..n).map(|i| expr).collect()`.

### Allocation Discipline

- AVOID unnecessary allocations, conversions, copies.
- When you need a small, fixed-upper-bound collection, prefer `SmallVec` (or similar stack-backed types) and build it with iterator/`collect` patterns rather than `new()+push` loops. This keeps things alloc-free.
- Prefer using the stack. Use `SmallVec` whenever it makes sense.
- Use `Arc`/`Rc` for shared immutable data.
- Prefer borrowing over ownership transfer when possible.

### Parallelism

- USE rayon to parallelize whenever larger amounts of data are being processed.
- Rayon is disabled on WASM targets.

### Safety

- AVOID using `unsafe` code unless absolutely necessary.
- AVOID return statements; structure functions with `if ... else if ... else` blocks instead. Early-return guard clauses (`if bad { return Err(...) }`) and `return` inside `match` arms in loops are acceptable when the alternative would deeply nest the entire function body.
- Prefer `?` operator and `Result` return types over `.unwrap()`.
- `.unwrap()` is permitted ONLY when an invariant guarantees the value will never be `None`/`Err`, with a `// SAFETY:` comment explaining why.

### Naming Conventions (Rust API Guidelines)

- **Casing**: `UpperCamelCase` for types/traits/variants; `snake_case` for functions/methods/modules/variables; `SCREAMING_SNAKE_CASE` for constants/statics.
- **Conversions**: `as_` for cheap borrowed-to-borrowed; `to_` for expensive conversions; `into_` for ownership-consuming conversions.
- **Getters**: No `get_` prefix (use `width()` not `get_width()`), except for unsafe variants like `get_unchecked()`.
- **Iterators**: `iter()` for `&T`, `iter_mut()` for `&mut T`, `into_iter()` for `T` by value.
- **Iterator type names**: Iterator type name matches the producing method.
- **Feature names**: Feature names are free of placeholder words.

### Imports

- Always import types at the top with `use` statements; never use inline paths in function bodies.
- External crate imports come before internal crate imports.
- No redundant type wrappers: never create wrapper enums/structs that duplicate types from dependencies. If an imported type does everything needed, re-export it with `pub use` instead.

## Testing Guidelines

- **CRITICAL: ALWAYS run `just test-cpu`, `just test-doc`, and `cargo clippy --all-targets -- -W warnings` before committing!**
- **CRITICAL: NEVER use `cargo test` for ordinary unit or integration tests. Use `cargo nextest run`. The work meters are process-global, so `cargo test` can cross-charge concurrently running tests in the same process and produce invalid failures.**
- **CRITICAL: Run doctests separately with `cargo test --doc`; `cargo nextest run` does not run doctests.**
- **CRITICAL: Address ALL warnings before EVERY commit!** This includes unused imports, dead code, deprecated API usage, and clippy warnings.
- Never use `#[allow(warnings)]` or similar suppressions without explicit user approval.
- **CRITICAL: Never modify test files** -- tests encode human intent.
- **CRITICAL: Never run tests with `RUST_TEST_UPDATE=1`** -- this modifies test expectations which is equivalent to changing tests.
- **CRITICAL: Never change expected test outputs** -- these are the ground truth.

## Visual Debugging for Meshing/Trim Bugs

When a STEP face is suspected of tessellating wrong but the failure is
not obvious from the test output, render the face to a diagnostic
image with `monstertruck-io/examples/preview-step-face.rs`:

```bash
cargo run -p monstertruck-io --example preview-step-face -- resources/step/abc-0008.step \
  --face 10 --face 17 --shell 1 --out target/face-previews
```

The example dumps PNGs of the chosen faces in parameter space with the
boundary wires, polylines, and any non-simple-wire artifacts drawn on
top. `--dump-trims` prints the raw trim parameter ranges before
tessellation -- useful when the boundary points themselves are
suspect.

This tool is the canonical replacement for ad-hoc "stick eprintln!s in
loops_store" debugging sessions and is the intended way for an agent
working on a meshing or trim-path bug to inspect the geometry without
asking a human to open the model in a viewer.

## Documentation

- All code comments MUST end with a period.
- All comments must be on their own line. Never put comments at the end of a line of code.
- All references to types, keywords, symbols etc. MUST be enclosed in backticks: `struct`, `Foo`.
- For each part of the docs, every first reference to a type, keyword, symbol etc. that is NOT the item itself being described MUST be linked: [`Foo`].
- En-dashes are expressed as two dashes: `--`. En-dashes are not used for connecting words, e.g. "compile-time".

## Commit & Pull Request Guidelines

- Use conventional commit prefixes: `feat:`, `fix:`, `chore:`.
- Keep messages concise and describe the user-facing effect.
- Every pull request should summarize intent, list key changes, and document how you validated them.
- **CRITICAL: NEVER claim that hosted CI passed unless `gh pr checks <PR>` reports the checks and their successful results.**
- Fork pull requests may have no hosted checks. When no checks are reported, state that fact and list only the local validation actually run.
- Never infer a hosted CI result from local commands or from the presence of workflow files.

## Branch Governance and Upstream Readiness

### Authority and contribution shape

- Treat current `upstream/master`, merged upstream semantics, and explicit
  maintainer direction as the source of truth. When they disagree with a
  downstream prototype, update the prototype rather than preserving a
  conflicting compatibility path.
- Follow `propose -> approve -> implement` for upstream API changes. Do not
  present an unapproved downstream API shape as accepted upstream policy.
- Preserve genuinely useful downstream ideas when they fit the established
  semantics. Separate or revise them when their contribution shape has not yet
  been approved; do not discard them merely because they extend the current
  minimum.
- Keep upstream pull requests narrowly reviewable and based on current
  `upstream/master`. Extract the already-proven minimal patch rather than
  merging downstream branch history into an upstream contribution branch.
- Keep tooling-only, repository-guidance, validation-history, and unrelated
  downstream changes out of continuity contribution branches.

### Two proving grounds

- Topic branches merge into `dev` through pull requests. Before merge, each
  topic must conform to current upstream vocabulary, crate ownership, error
  semantics, module layout, and approved API direction.
- The `dev` branch is the first proving ground. It may integrate broader
  downstream layers and evidence, but must not retain APIs or compatibility
  aliases that contradict established upstream semantics.
- Keep `dev` synchronized by merging the reconciled `master` branch into it
  whenever upstream semantics land. Repair active topic branches before they
  merge into the refreshed `dev` branch.
- Promotion from `dev` to `master` requires a pull request and is a second,
  independent proving ground. It is not automatic consolidation.
- Before a `dev -> master` promotion, reconcile `master` with current
  `upstream/master`. Re-audit semantic conformance, cross-crate behavior,
  feature combinations, documentation, and validation claims against the
  exact promotion revision.
- The `master` branch is the validated fork baseline. Do not promote an
  experiment that obscures established upstream semantics; isolate it behind
  an explicit experimental boundary or leave it on `dev`.
- Create an upstream contribution branch only after the implementation has
  passed the relevant `dev` and `master` gates. Rebase the contribution as a
  clean, focused patch on current `upstream/master`, then rerun validation on
  that exact tree.

### Propagating upstream decisions

When upstream establishes or merges semantics, apply this sequence:

1. Reconcile the change into `master` while preserving only intentional
   fork-local guidance and tooling differences.
2. Merge the reconciled `master` into `dev`.
3. Repair active topic branches before accepting them into `dev`.
4. Revalidate the integrated result during the next `dev -> master` pull
   request.
5. Extract future upstream pull requests from the resulting conforming
   implementation onto current `upstream/master`.

### Continuity program boundaries

- The production target is complex parametric operations with independently
  validated continuity through at least `G3`. Any `G4` path remains explicitly
  experimental in its public API, configuration, documentation, and evidence.
- Keep checked continuity order and representation-neutral capability
  vocabulary in `monstertruck-traits`. Keep representation inspection, local
  transition semantics, and numerical solving in `monstertruck-geometry`.
- Reuse existing arbitrary-order derivative APIs rather than adding parallel
  public jet-access vocabulary.
- Land the bounded, transactional solver before tracking, persistence,
  contracts, or replay. Later layers must use the existing stable identity and
  attribute systems rather than introducing a parallel identity model.
- Keep STEP integration and imported-model evidence outside the numerical
  geometry crate. Arbitrary trimmed seams must refuse with a typed reason
  rather than attempting a best-effort solve.
- Bounded operations must expose typed truncation, measured work, and no
  partial result. Tests involving process-global work meters must run under
  Nextest process isolation.

## Correct-or-Typed API Rules (HARDLINE)

- **NEVER represent an unsupported result as a `bool` when more than one cause is possible. Carry a typed, actionable reason for the unsupported path.**
- **NEVER collapse degree, knot, control-net, clamping, periodicity, boundary-side, representation, trimmed-seam, or similar capability failures into a single unsupported bit.**
- **NEVER return a silent partial result. Truncation and unsupported capability states must identify their condition through a typed error or reason.**

## Error Handling

### Fallible Operations

- Prefer `?` operator and `Result` return types over `.unwrap()`.
- Use `.ok_or()` or `.ok_or_else()` to convert `Option` to `Result` with meaningful errors.

### Safe Unwrap Pattern

`.unwrap()` is permitted ONLY when an invariant guarantees the value will never be `None`/`Err`. This requires a **safety comment**:

```rust
// SAFETY: We just inserted `key` into `map` on the previous line,
// so it must exist.
let value = map.get(&key).unwrap();
```

## Code Organization Best Practices

1. **Module Structure**: Keep modules focused on a single responsibility.
2. **Module Size**: Keep individual files reasonably sized (~300-500 lines). Split larger modules into submodules rather than having monolithic files.
3. **Public API**: Minimize public surface area; use `pub(crate)` liberally.
4. **Error Types**: Define per-crate error types with `thiserror`. Each crate has its own `errors.rs`.
5. **Tests**: Co-locate unit tests; integration tests in `tests/`.

## Performance Best Practices

### String Handling

- Use `&str` instead of `String` where ownership isn't needed.
- Avoid `.to_string()` for temporary values.
- Use string slices for function parameters when possible.

### Cache Optimization

- Consider caching expensive transformations.
- Use content-based hashing for deterministic cache keys.
- Sort collections before hashing to ensure determinism.

## Rust API Guidelines Checklist

Compressed from the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines). Only open that link and read selectively when auditing code against this checklist and needing clarification on a specific C-XXX rule.

### Naming (see also Naming Conventions above)

- **C-CASE**: RFC 430 casing (`UpperCamelCase`, `snake_case`, `SCREAMING_SNAKE`).
- **C-CONV**: `as_` cheap borrow-to-borrow; `to_` expensive; `into_` consuming.
- **C-GETTER**: No `get_` prefix; just `width()` not `get_width()`.
- **C-ITER**: `iter()` -> `&T`, `iter_mut()` -> `&mut T`, `into_iter()` -> `T`.
- **C-ITER-TY**: Iterator type name matches producing method.
- **C-FEATURE**: Feature names are free of placeholder words.
- **C-WORD-ORDER**: Consistent word order in names.

### Interoperability

- **C-COMMON-TRAITS**: Public types eagerly implement `Debug`, `Clone`, `Hash`, `PartialEq`, `Eq`, and `Copy` if directly derivable. Implement `Ord`, `PartialOrd`, `Display`, `Default` where meaningful.
- **C-CONV-TRAITS**: Use `From`, `AsRef`, `AsMut` for conversions.
- **C-COLLECT**: Collections implement `FromIterator` and `Extend`.
- **C-SEND-SYNC**: Types are `Send` + `Sync` where possible.
- **C-GOOD-ERR**: Error types are meaningful and well-behaved.
- **C-RW-VALUE**: Reader/writer functions take `R: Read`/`W: Write` by value.

### Macros

- **C-EVOCATIVE**: Input syntax evocative of output.
- **C-MACRO-ATTR**: Compose well with attributes.
- **C-ANYWHERE**: Item macros work anywhere items are allowed.
- **C-MACRO-VIS**: Support visibility specifiers.
- **C-MACRO-TY**: Type fragments are flexible.

### Documentation (see also Documentation section above)

- **C-CRATE-DOC**: Crate-level docs thorough with examples.
- **C-EXAMPLE**: All public items have a rustdoc example.
- **C-QUESTION-MARK**: Examples use `?`, not `try!` or `unwrap`.
- **C-FAILURE**: Docs cover error, panic, and safety conditions.
- **C-LINK**: Prose hyperlinks to relevant items.
- **C-METADATA**: `Cargo.toml` has all common metadata.
- **C-HIDDEN**: Rustdoc hides unhelpful implementation details.

### Predictability

- **C-SMART-PTR**: Smart pointers don't add inherent methods.
- **C-CONV-SPECIFIC**: Conversions live on the most specific type.
- **C-METHOD**: Functions with a clear receiver are methods.
- **C-NO-OUT**: No out-parameters; return values instead.
- **C-OVERLOAD**: Operator overloads are unsurprising.
- **C-DEREF**: Only smart pointers implement `Deref`/`DerefMut`.
- **C-CTOR**: Constructors are static inherent methods.

### Flexibility

- **C-INTERMEDIATE**: Expose intermediate results to avoid duplicate work.
- **C-CALLER-CONTROL**: Caller decides where to copy and place data.
- **C-GENERIC**: Minimize assumptions via generics.
- **C-OBJECT**: Traits are object-safe if useful as trait objects.

### Type Safety

- **C-NEWTYPE**: Newtypes for static distinctions.
- **C-CUSTOM-TYPE**: Arguments convey meaning through types, not `bool`/`Option`.
- **C-BITFLAG**: Flag sets use `bitflags`, not enums.
- **C-BUILDER**: Builders for complex value construction.

### Dependability

- **C-VALIDATE**: Functions validate arguments.
- **C-DTOR-FAIL**: Destructors never fail.
- **C-DTOR-BLOCK**: Blocking destructors have alternatives.

### Debuggability

- **C-DEBUG**: All public types implement `Debug`.
- **C-DEBUG-NONEMPTY**: `Debug` output is never empty.

### Future Proofing

- **C-SEALED**: Sealed traits protect against downstream impls.
- **C-STRUCT-PRIVATE**: Structs have private fields.
- **C-NEWTYPE-HIDE**: Newtypes encapsulate implementation details.
- **C-STRUCT-BOUNDS**: Data structures don't duplicate derived trait bounds.

### Necessities

- **C-STABLE**: Public deps of a stable crate are stable.
- **C-PERMISSIVE**: Crate and deps have permissive licenses.

## Writing Instructions

- Be concise.
- Use simple sentences. Technical jargon is fine.
- Do NOT overexplain basic concepts. Assume the user is technically proficient.
- AVOID flattering, corporate-ish or marketing language.
- AVOID vague and/or generic claims not substantiated by the context.
- AVOID weasel words.
