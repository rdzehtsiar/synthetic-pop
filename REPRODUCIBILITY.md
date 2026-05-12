# Reproducibility

Synthetic Pop is intended to produce repeatable synthetic communities from explicit inputs. This document records what is true in the current Milestone 1 scaffold and what must be true before alpha or v0 generation is released.

## Current Milestone 1 Scaffold

- The repository contains a minimal Rust workspace and crate layout.
- There is no implemented generator, scenario runner, exporter, data pack loader, app, or binding yet.
- The CLI is a placeholder only and does not produce synthetic data.
- The current reproducibility guarantee is limited to source control reviewability and repeatable local validation of the scaffold.

## Required Before Alpha Generation

- Every generated dataset must be derived from explicit inputs: generator version, scenario version, seed, configuration, data pack versions, and export format.
- The same supported input set must produce the same output on every supported platform.
- Randomness must be deterministic and namespace-based so work can be parallelized without changing results.
- Outputs must include enough metadata to reproduce the run later.
- Snapshot tests must cover representative scenarios and detect unintended output drift.

## Required Before v0

- Reproducibility rules must be treated as compatibility commitments.
- Any intentional output change must be documented with a version bump or migration note.
- Release artifacts should be built through documented, repeatable commands.
- Benchmarks and fixture datasets must identify the exact version and configuration used to create them.
