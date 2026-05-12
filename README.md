# Synthetic Pop

Synthetic Pop is planned as an offline, deterministic synthetic community
generator for demos, staging data, simulations, and repeatable test fixtures.
The first product wedge remains a forum community generator that can eventually
produce synthetic users, profiles, relationships, and activity from explicit
inputs.

Current status: Milestone 2 Rust core foundation. The repository now contains a
small Rust workspace with a core engine API, request validation primitives, and
truthful capability flags. It does not generate data yet.

## Rust Core Foundation

`crates/core` exposes the implemented foundation:

- `current_status()` reports `milestone 2 rust core foundation`.
- `CoreEngine` owns a `CoreEngineConfig` and exposes current capabilities.
- `CoreCapabilities` marks the offline/local core as available while
  generation, policy filtering, exports, bindings, desktop, and WASM remain
  unavailable.
- `Seed`, `GenerationSize`, and `CoreRunRequest` validate request inputs only.
  Empty seeds, zero sizes, and sizes above `100_000` are rejected.

The core API is intentionally validation-only in this milestone. A valid
`CoreRunRequest` is not a generation result and no synthetic records are
produced.

## CLI Status

The `synthetic-pop` binary is still a placeholder. It prints the current core
status, states that generation commands are not implemented, and points to the
planned first command shape:

```sh
synthetic-pop generate forum --seed demo --users 10000
```

That command shape is documentation for the intended interface; it is not
implemented yet.

## Repository Layout

- `crates/core`: Milestone 2 Rust core foundation and validation-only engine
  API.
- `crates/cli`: placeholder CLI binary and CLI-facing status message.
- `crates/policy`: deferred policy filtering surface.
- `crates/export`: deferred export surface.
- `crates/scenarios`: planned scenario metadata, starting with `forum`.
- `apps/desktop`: placeholder for a future desktop application.
- `apps/web-demo`: placeholder for a future web demo or WASM-backed surface.
- `bindings/python`: placeholder for future Python bindings.
- `bindings/node`: placeholder for future Node bindings.
- `data-packs`: placeholder for future local, versioned data packs.
- `examples`: placeholder for future example projects and fixtures.
- `docs`: design and quality notes.

## Trust Documents

The trust documents define project boundaries while generation remains deferred:

- `REPRODUCIBILITY.md`: current guarantees and future deterministic output
  requirements.
- `DATA_PROVENANCE.md`: provenance rules for future data packs and examples.
- `SAFETY_POLICY.md`: allowed and disallowed uses, plus future policy checks.
- `OFFLINE_GUARANTEE.md`: offline-first expectations for the core generation
  path.
- `BENCHMARKS.md`: benchmark policy and future performance reporting rules.
- `CONTRIBUTING.md`: contribution expectations.

## Local Validation

Run the Rust workspace checks from the repository root:

```sh
cargo build
cargo test
cargo clippy --all-targets --all-features
```

See `docs/QUALITY.md` for the expected outcome and current test scope.

## Deferred Work

Generation, policy filtering, export formats, data pack loading, packaged
examples, desktop app, web app, Tauri integration, WASM, and language bindings
remain deferred. They are represented only by placeholders or capability flags
until later milestones define and implement their behavior.
