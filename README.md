# Synthetic Pop

Synthetic Pop is planned as an offline, deterministic synthetic community
generator for demos, staging data, simulations, and repeatable test fixtures.
The first product wedge is a forum community generator that can eventually
produce synthetic users, profiles, relationships, and activity from explicit
inputs.

Current status: Milestone 1 scaffold only. The repository contains the trust
documents, Rust workspace, crate layout, and placeholders needed to start
implementation. The CLI does not generate data yet.

## Planned First Command

The intended first generation command shape is:

```sh
synthetic-pop generate forum --seed demo --users 10000
```

This command is not implemented yet. Today, the `synthetic-pop` binary only
prints a placeholder message that points to the planned command.

## Repository Layout

- `crates/core`: shared product identity and future core generator surface.
- `crates/cli`: placeholder CLI binary and CLI-facing helpers.
- `crates/policy`: future safety and policy enforcement crate.
- `crates/export`: future export format crate.
- `crates/scenarios`: future scenario definitions, starting with `forum`.
- `apps/desktop`: placeholder for a future desktop application.
- `apps/web-demo`: placeholder for a future web demo.
- `bindings/python`: placeholder for future Python bindings.
- `bindings/node`: placeholder for future Node bindings.
- `data-packs`: placeholder for future local, versioned data packs.
- `examples`: placeholder for future example projects and fixtures.
- `docs`: placeholder for future design and user documentation.

## Trust Documents

Milestone 1 establishes the public trust boundaries before generation exists:

- `REPRODUCIBILITY.md`: current scaffold guarantees and future deterministic
  output requirements.
- `DATA_PROVENANCE.md`: provenance rules for future data packs and examples.
- `SAFETY_POLICY.md`: allowed and disallowed uses, plus future policy checks.
- `OFFLINE_GUARANTEE.md`: offline-first expectations for the core generation
  path.
- `BENCHMARKS.md`: benchmark policy and future performance reporting rules.
- `CONTRIBUTING.md`: contribution expectations for the scaffold and future
  generator work.

Each document separates what is implemented now from what must exist before
alpha or v0 generation.

## Local Validation

The Rust workspace is intentionally small, but it should compile and test:

```sh
cargo build
cargo test
cargo clippy --all-targets --all-features
```

See `docs/QUALITY.md` for the expected outcome of each command and the current
test scope.

## Intentionally Deferred

Milestone 1 does not implement generation, policy filtering, exports, data pack
loading, desktop or web apps, language bindings, benchmark suites, or packaged
examples. Those surfaces are represented only as crate or directory placeholders
until later milestones define their behavior.
