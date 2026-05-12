# Quality Checks

Milestone 4 validation proves the Rust core foundation compiles, deterministic
RNG primitives behave as documented, the canonical data model is serde-ready,
reproducibility metadata is exposed, and deferred surfaces do not claim unbuilt
functionality.

## Required Local Commands

Run these commands from the repository root before committing changes:

```sh
cargo build
cargo test
cargo clippy --all-targets --all-features
```

Expected outcome:

- `cargo build` compiles the full Rust workspace.
- `cargo test` runs the crate tests for the core API, canonical model
  primitives/entities/validation, deterministic RNG golden vectors, CLI status
  placeholder, and deferred policy/export/scenario surfaces.
- `cargo clippy --all-targets --all-features` reports no warnings.

## Current Test Scope

The current tests validate deterministic RNG primitives, canonical model
records, and milestone boundaries, not generated forum data. They check that:

- product identity and `milestone 4 canonical data model` status are exposed by
  the core crate;
- `CoreEngine` can be constructed with default or explicit config;
- core capabilities truthfully report offline/local core and deterministic RNG
  availability, report the canonical data model as available, and keep
  generation, policy, exports, bindings, desktop, and WASM unavailable;
- `Seed`, `GenerationSize`, and `CoreRunRequest` accept valid inputs and reject
  empty seeds, zero sizes, and sizes above `100_000`;
- `synthetic_pop_core::model` exposes string-backed ID/value primitives, shared
  relationship/reaction/activity enums, and serde-ready canonical entities for
  identity, forum, community, organization, relationship, and activity records;
- primitive model values reject empty or whitespace-only strings, serde
  deserialization preserves those boundaries, and lightweight entity
  constructors reject empty required fields such as usernames, display names,
  post/comment bodies, community names, and organization names;
- representative model entities round-trip through JSON, including the example
  user shape and forum/community relationship records;
- deterministic RNG golden vectors, bounded values, booleans, slice selection,
  call-order independence, and parallel-safe derivation remain stable;
- reproducibility metadata exposes the current metadata and RNG algorithm
  versions;
- the CLI message names the planned `generate forum` command while stating that
  deterministic RNG and the canonical data model are available and generation
  commands are not implemented;
- policy and export crates remain disabled;
- the first planned scenario remains `forum`.

Do not add generated user/post/forum fixtures, policy simulation tests, export
golden files, Tauri/WASM checks, or binding tests until those surfaces exist.

## Repository Hygiene

`Cargo.lock` is tracked so workspace checks use the same dependency resolution.
Rust build output is ignored via `/target/` in the repository `.gitignore`.
