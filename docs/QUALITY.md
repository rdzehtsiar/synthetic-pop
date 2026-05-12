# Quality Checks

Milestone 2 validation proves the Rust core foundation compiles, its
validation-only API behaves as documented, and deferred surfaces do not claim
unbuilt functionality.

## Required Local Commands

Run these commands from the repository root before committing changes:

```sh
cargo build
cargo test
cargo clippy --all-targets --all-features
```

Expected outcome:

- `cargo build` compiles the full Rust workspace.
- `cargo test` runs the crate tests for the core API, CLI status placeholder,
  and deferred policy/export/scenario surfaces.
- `cargo clippy --all-targets --all-features` reports no warnings.

## Current Test Scope

The current tests validate milestone boundaries, not generation behavior. They
check that:

- product identity and `milestone 2 rust core foundation` status are exposed by
  the core crate;
- `CoreEngine` can be constructed with default or explicit config;
- core capabilities truthfully report offline/local core availability and keep
  generation, policy, exports, bindings, desktop, and WASM unavailable;
- `Seed`, `GenerationSize`, and `CoreRunRequest` accept valid inputs and reject
  empty seeds, zero sizes, and sizes above `100_000`;
- the CLI message names the planned `generate forum` command while stating that
  generation commands are not implemented;
- policy and export crates remain disabled;
- the first planned scenario remains `forum`.

Do not add generation fixtures, policy simulation tests, export golden files,
Tauri/WASM checks, or binding tests until those surfaces exist.

## Repository Hygiene

`Cargo.lock` is tracked so workspace checks use the same dependency resolution.
Rust build output is ignored via `/target/` in the repository `.gitignore`.
