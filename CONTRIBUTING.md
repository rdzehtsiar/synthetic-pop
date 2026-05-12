# Contributing

This project is currently in Milestone 1: trust and credibility foundations. Contributions should keep promises narrow, explicit, and aligned with what the code actually implements.

## Current Milestone 1 Scaffold

- The repository contains the planned structure and minimal Rust crates.
- The CLI is a placeholder and does not generate data.
- Apps, bindings, data packs, exports, policy filtering, benchmarks, and generation are not implemented.
- Trust documents define requirements for future work, not completed product behavior.

## Contribution Guidelines

- Keep changes scoped to one milestone or feature area.
- Do not claim implemented behavior unless there is code and validation for it.
- Prefer deterministic behavior and explicit configuration over ambient environment state.
- Keep generated examples clearly synthetic.
- Update trust documentation when a change affects offline behavior, provenance, reproducibility, safety, or benchmark claims.

## Local Validation

For Rust changes, run:

```bash
cargo build
cargo test
cargo clippy --all-targets --all-features
```

Documentation-only changes should at minimum be reviewed for accuracy and checked for trailing whitespace.

## Required Before Alpha Generation

- Add tests with every implemented generation, policy, export, or data pack behavior.
- Document new public commands, file formats, and compatibility expectations.
- Include provenance metadata for any new data pack.
- Keep the default generation path offline.

## Required Before v0

- Maintain release notes for compatibility-affecting changes.
- Treat reproducibility, safety, and offline regressions as release blockers.
- Keep contributor setup steps current for all supported crates, apps, and bindings.
