# Quality Checks

Milestone 5 validation proves the Rust workspace compiles, deterministic RNG
primitives behave as documented, the canonical data model is serde-ready, forum
scenario generation works, supported exports are available, and deferred
surfaces do not claim unbuilt functionality.

## Required Local Commands

Run these commands from the repository root before committing changes:

```sh
cargo fmt --check
cargo build
cargo test
cargo clippy --all-targets --all-features -- -D warnings
rg -n "rand|thread_rng|SystemTime|UNIX_EPOCH|now\(|env::|reqwest|ureq|tokio"
```

Expected outcome:

- `cargo fmt --check` reports no formatting changes.
- `cargo build` compiles the full Rust workspace.
- `cargo test` runs the crate tests for the core API, canonical model
  primitives/entities/validation, deterministic RNG golden vectors, forum
  scenario config/generation, persona and interest generation, bio generation,
  export formats, CLI generation, and deferred policy surfaces.
- `cargo clippy --all-targets --all-features -- -D warnings` reports no
  warnings.
- The `rg` nondeterminism/network scan has no matches in the current milestone
  implementation.

## Current Test Scope

The current tests validate deterministic RNG primitives, canonical model
records, milestone boundaries, generated forum data, and forum exports. They
check that:

- product identity and `milestone 5 scenario generation and export` status are
  exposed by the core crate;
- `CoreEngine` can be constructed with default or explicit config;
- core capabilities truthfully report offline/local core, deterministic RNG,
  canonical data model, generation, and export availability, while keeping
  policy, bindings, desktop, and WASM unavailable;
- `Seed`, `GenerationSize`, and `CoreRunRequest` accept valid inputs and reject
  empty seeds, zero sizes, and sizes above `100_000`;
- `synthetic_pop_core::model` exposes string-backed ID/value primitives, shared
  relationship/reaction/activity enums, and serde-ready canonical entities for
  identity, persona, interest, forum, community, organization, relationship,
  and activity records;
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
- the first implemented scenario is `forum`;
- forum scenario YAML is validated, generation is deterministic for the same
  config, and changing the seed changes generated output;
- every generated user receives one valid persona ID and 2-5 valid interest
  IDs from the static catalog;
- persona scoring tests cover bounded deterministic fields and visible
  technical, social, and culture interest correlations;
- generated bios are non-empty, deterministic for the same seed, change with
  seed variation, avoid the old generic bio pattern, reference assigned
  interests, and pass a 10k-user duplicate-rate smoke test;
- export tests cover JSON, JSONL, CSV, SQLite SQL, PostgreSQL SQL, and Prisma
  seed output, including first-class persona and interest records;
- the CLI status and `generate forum` command reflect implemented generation
  and export behavior;
- the policy crate remains disabled.

Policy simulation tests, data-pack fixture tests, Tauri/WASM checks, and
binding tests should wait until those surfaces exist.

## Repository Hygiene

`Cargo.lock` is tracked so workspace checks use the same dependency resolution.
Rust build output is ignored via `/target/` in the repository `.gitignore`.
