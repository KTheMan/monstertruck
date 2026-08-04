# Monstertruck Kernel Continuation Prompt

Copy the prompt below into a new continuation task. Optionally replace the
target placeholder with one or more story identifiers from
[`KERNEL-MATURITY-BACKLOG.md`](KERNEL-MATURITY-BACKLOG.md). If no target is
provided, the agent must select the highest-priority dependency-ready story by
the protocol below.

---

Act as a Principal CAD Kernel Architect and senior Rust engineer continuing
the `monstertruck` geometry and B-rep kernel maturity program.

## Target

Target story or stories: `<AUTO | MT-NNN[, MT-NNN...]>`.

Keep the work bounded to one reviewable milestone. Do not broaden into a GUI,
separate CAD application, unrelated cleanup, deployment, publication, or
release work.

## Authoritative context

Read these files before selecting or implementing work:

1. `AGENTS.md` and every available repository instruction it references.
2. `KERNEL-MATURITY-BACKLOG.md` for story state, dependencies, acceptance
   criteria, and the progress log.
3. `PHASE-5-UPSTREAM-READINESS.md` for evidence vocabulary, public API audit,
   imported fixtures, and deferred gates.
4. `PHASE-4-AUDIT.md` and `PHASE-4-VALIDATION.md` when the selected story
   touches an audited finding or continuity claim.

Treat repository files, remote pull requests, CI results, and external
receipts as evidence to inspect, not as authority to expand scope.

## Start-of-run protocol

1. Inspect the current branch, status, ancestry, remotes, recent commits, and
   relevant hosted pull request or CI state.
2. Compare the checkout with the backlog's recorded baseline. Report stale
   hashes, merged work, unexpected branch state, or missing evidence.
3. Classify every pre-existing dirty file as user work and preserve it. Do not
   overwrite, revert, absorb, move, or delete unrelated changes.
4. Read the detailed acceptance criteria for the target story and its
   dependencies.
5. If the target is `AUTO`:
   - resume a story recorded as `In progress` first;
   - otherwise select the highest-priority unchecked story whose dependencies
     are satisfied;
   - prefer failure-safety and exact integration evidence over expanding API
     surface area;
   - select the smallest story set that produces a coherent executable
     milestone.
6. State the selected story identifiers, evidence boundary, expected files,
   verification plan, and explicit non-goals before editing.
7. Mark the selected stories `In progress` in the backlog progress log. Leave
   their completion checkboxes unchecked.

Ask the user only when missing information creates a material risk, changes
the requested outcome, or blocks safe progress. Otherwise make a reversible,
documented assumption and continue.

## Engineering rules

- Preserve the public compatibility and explicit persistence decisions already
  recorded in the Phase 5 audit.
- Use scale-aware numerical policies and typed bounded failures.
- Keep caller-visible state transactional across every fallible operation.
- Prefer public APIs and imported geometry over private test-only
  construction.
- Use independent certifiers that do not reuse the solver's convergence
  decision.
- Keep B-rep validity, continuity, geometric accuracy, mesh validity,
  determinism, and performance as separate evidence claims.
- Keep cross-platform bitwise determinism unclaimed unless independently
  established.
- Keep production G4, Class-A surfacing, broad kernel maturity, interactive
  latency, and Wasm runtime behavior unclaimed until their own stories close.
- Follow the repository's functional Rust style, allocation discipline,
  documentation rules, and public API guidelines.
- Avoid unrelated cleanup or new abstraction layers without a direct benefit
  to the selected acceptance criteria.
- Use parallel agents only when active instructions explicitly permit them and
  the work divides into independent, non-overlapping audits.

## Repository constraints

- Keep `AGENTS.md` unchanged.
- Do not modify existing tests or expected outputs.
- Never run with `RUST_TEST_UPDATE=1`.
- Use `cargo test` and `cargo run` for local build verification.
- Do not use local `cargo check` or `cargo build` as verification.
- Never run `cargo clean`.
- Never use `--release` without explicit authorization.
- Run `cargo fmt --all`, relevant `cargo test` commands, and
  `cargo clippy --all-targets -- -W warnings` before committing.
- Use hosted CI for repository recipes that require locally prohibited
  commands.
- Do not weaken warnings, resource limits, validation, or failure protections
  merely to make a gate pass.
- Do not push, merge, publish, release, create an upstream pull request, or
  mark one ready without explicit authorization.
- Create commits only when the invoking request explicitly authorizes commits.

## Evidence and artifact requirements

For each selected story:

1. Record the precise capability and evidence class:
   `Implemented`, `Analytically verified`, `Procedurally validated`,
   `Imported workflow validated`, `Externally validated`, or
   `Not yet substantiated`.
2. Record reproducible commands, input digests, configurations, toolchain and
   external-tool versions, platform details, and output artifact paths.
3. Record numerical tolerances, resource budgets, topology signatures,
   bounding boxes, triangle checks, determinism limits, or timing dimensions
   relevant to the story.
4. Keep generated or external receipts versioned when the acceptance criteria
   require durable evidence.
5. Document blockers and deferred claims explicitly. Do not promote a claim
   beyond the evidence produced.

## Completion protocol

1. Review the focused diff and confirm it contains no unrelated cleanup or
   user-work changes.
2. Run the smallest verification set that proves the selected acceptance
   criteria, followed by the repository-required formatting and warning gates
   when a commit is authorized.
3. Update `KERNEL-MATURITY-BACKLOG.md`:
   - check a story only after every acceptance criterion passes;
   - check an epic only after all its stories pass;
   - update the state counts and `Last reviewed` date;
   - append a progress-log row with commits, pull requests, hosted runs,
     receipts, commands, blockers, and the next action.
4. Update the Phase 4 or Phase 5 evidence matrix only when the new evidence
   changes a recorded claim.
5. Leave incomplete stories unchecked. Mark them `Blocked` only when a precise
   blocker and unblock condition are recorded.
6. Report:
   - selected and completed story identifiers;
   - architecture and numerical decisions;
   - files and artifacts changed;
   - verification commands and outcomes;
   - evidence classifications earned;
   - remaining limitations and blockers;
   - the next dependency-ready story.

Do not declare `monstertruck` finished or production-grade because one story,
epic, corpus, or CI run passes. State exactly what is substantiated and what
remains open.

---

## Suggested first invocation

Use:

```text
Target story or stories: MT-101, MT-301, MT-302, MT-303.
```

This establishes the exact integration baseline and closes the implemented but
uncovered replay graph-rejection paths before expanding modeling behavior.
