# Quality Checks

Milestone 1 keeps validation deliberately small. The repository should prove
that the scaffold compiles, tests run across every crate, and the Rust lints do
not report issues before deeper generation behavior exists.

## Required Local Commands

Run these commands from the repository root before committing scaffold changes:

```sh
cargo build
cargo test
cargo clippy --all-targets --all-features
```

Expected outcome:

- `cargo build` compiles the full workspace.
- `cargo test` runs the lightweight tests in each crate.
- `cargo clippy --all-targets --all-features` reports no warnings.

## Current Test Scope

The current tests intentionally validate milestone boundaries, not generator
behavior. They check that:

- product identity and scaffold status are exposed by the core crate;
- the CLI message names the planned command without implementing it;
- policy and export crates do not claim unbuilt functionality;
- the first planned scenario remains `forum`.

Do not add fixtures, data generation, benchmark harnesses, or policy simulation
tests in Milestone 1. Those belong to later milestones once the public behavior
is defined.

## Repository Hygiene

`Cargo.lock` is tracked so every workspace check uses the same dependency
resolution once dependencies are added. Rust build output is ignored via
`/target/` in the repository `.gitignore`.
