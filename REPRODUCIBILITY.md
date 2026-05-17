# Reproducibility

Synthetic Pop produces repeatable synthetic communities from explicit inputs.
This document records what is true in the current Milestone 5 scenario
generation and export surface and what must be true before v0.

## Current Milestone 5 Scenario Generation And Export

- The repository contains a Rust workspace with a core engine API, validation
  primitives, deterministic RNG primitives, reproducibility metadata, a
  serde-ready canonical data model, forum scenario generation, and forum
  dataset exports.
- `synthetic_pop_core::model` defines canonical primitives and records for
  users, profiles, usernames, personas, interests, statuses, posts, comments,
  reactions, relationships, communities, organizations, and activity events.
- `synthetic_pop_scenarios` parses validated forum configs and generates
  users, one persona per user, static catalog interests, persona-aware bios,
  communities, posts, comments, membership relationships, and activity events.
- `synthetic_pop_export` exports forum datasets as `json`, `jsonl`, `csv`,
  `sqlite-sql`, `postgres-sql`, and `prisma-seed`, including first-class
  persona and interest records.
- `DeterministicRandom` and helper functions derive field-level values from
  `(seed, namespace, entity_id, field)` without shared mutable RNG state.
- Persona traits are derived from the explicit seed, the `"personas"`
  namespace, persona ID, and field name. Numeric persona fields are stable
  bounded `f32` values in `0.0..=1.0`; categorical persona fields serialize as
  `snake_case`.
- Interest assignment uses a static in-code catalog and deterministic scoring
  from persona traits plus deterministic tie-breakers. No external data pack is
  loaded for the catalog in this milestone.
- Bios are deterministic template strings generated after persona and interest
  assignment. They can reference assigned interests and local phrase pools only.
- The deterministic RNG algorithm is identified by
  `synthetic-pop-deterministic-rng-v1:length-delimited-utf8+fnv1a64+splitmix64`
  and exposed through `reproducibility_metadata()`.
- Golden tests pin representative RNG outputs and verify order-independent and
  parallel-safe derivation.
- For the same supported seed, forum config, generator/export code, and export
  format, the generated dataset and export text are expected to be identical.
- There is no implemented policy filter, external data pack loader, desktop
  app, WASM surface, language binding, runtime learning, or LLM-backed language
  generation yet.

## Current Input Surface

Forum config is explicit YAML:

```yaml
scenario: forum
seed: demo
population:
  users: 100
communities:
  - general
  - support
content:
  posts: 500
  comments: 1000
output_format: jsonl
```

`output_format` defaults to `jsonl`. The CLI may override the config format
with `--format`.

## Required Before v0

- Reproducibility rules must be treated as compatibility commitments.
- Any intentional output change must be documented with a version bump or
  migration note.
- Outputs should include enough metadata to reproduce the run later.
- Release artifacts should be built through documented, repeatable commands.
- Benchmarks and fixture datasets must identify the exact version and
  configuration used to create them.
