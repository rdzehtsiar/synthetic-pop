# Reproducibility

Synthetic Pop is intended to produce repeatable synthetic communities from
explicit inputs. This document records what is true in the current Milestone 3
deterministic generation system and what must be true before alpha or v0
generation is released.

## Current Milestone 3 Deterministic Generation System

- The repository contains a Rust workspace with a core engine API, validation
  primitives, deterministic RNG primitives, and reproducibility metadata.
- `DeterministicRandom` and helper functions derive field-level values from
  `(seed, namespace, entity_id, field)` without shared mutable RNG state.
- The deterministic RNG algorithm is identified by
  `synthetic-pop-deterministic-rng-v1:length-delimited-utf8+fnv1a64+splitmix64`
  and exposed through `reproducibility_metadata()`.
- Golden tests pin representative RNG outputs and verify order-independent and
  parallel-safe derivation.
- There is no implemented data model generator, scenario runner, exporter, data
  pack loader, desktop app, WASM surface, or language binding yet.
- The CLI is a placeholder only and does not produce synthetic data.
- The current reproducibility guarantee is limited to the documented RNG
  primitives, metadata, source control reviewability, and repeatable local
  validation.

## Required Before Alpha Generation

- Every generated dataset must be derived from explicit inputs: generator version, scenario version, seed, configuration, data pack versions, and export format.
- The same supported input set must produce the same output on every supported platform.
- Scenario randomness must use the deterministic namespace-based RNG so work can
  be parallelized without changing results.
- Outputs must include enough metadata to reproduce the run later.
- Snapshot tests must cover representative scenarios and detect unintended output drift.

## Required Before v0

- Reproducibility rules must be treated as compatibility commitments.
- Any intentional output change must be documented with a version bump or migration note.
- Release artifacts should be built through documented, repeatable commands.
- Benchmarks and fixture datasets must identify the exact version and configuration used to create them.
