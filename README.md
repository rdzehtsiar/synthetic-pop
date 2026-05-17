# Synthetic Pop

[![Tests](https://github.com/rdzehtsiar/synthetic-pop/actions/workflows/tests.yml/badge.svg)](https://github.com/rdzehtsiar/synthetic-pop/actions/workflows/tests.yml)
[![codecov](https://codecov.io/gh/rdzehtsiar/synthetic-pop/graph/badge.svg)](https://codecov.io/gh/rdzehtsiar/synthetic-pop)
[![Quality Gate Status](https://sonarcloud.io/api/project_badges/measure?project=rdzehtsiar_synthetic-pop&metric=alert_status)](https://sonarcloud.io/summary/new_code?id=rdzehtsiar_synthetic-pop)

Synthetic Pop is an offline, deterministic synthetic community generator for
demos, staging data, simulations, and repeatable test fixtures. The first
implemented product wedge is a forum community generator that produces
synthetic users, deterministic personas, interests, communities, posts,
comments, membership relationships, and activity events from explicit inputs.

Current status: Milestone 5 scenario generation and export. The Rust workspace
contains a core engine API, request validation primitives, deterministic
field-level RNG primitives, reproducibility metadata, a serde-ready canonical
model, forum scenario generation, persona-aware bios, CLI generation, and forum
dataset exports. Policy filtering, language bindings, desktop, and WASM
surfaces are still unavailable.

## Forum Scenario Config

The forum scenario accepts YAML with this shape:

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

`output_format` is optional and defaults to `jsonl`. Supported values are
`json`, `jsonl`, `csv`, `sqlite-sql`, `postgres-sql`, and `prisma-seed`.
`population.user_count`, `content.post_count`, and `content.comment_count` are
accepted as aliases for `users`, `posts`, and `comments`.

## CLI

Generate from direct flags:

```sh
synthetic-pop generate forum \
  --seed demo \
  --users 100 \
  --communities general,support \
  --posts 500 \
  --comments 1000 \
  --format jsonl
```

Generate from a config file and write an export:

```sh
synthetic-pop generate forum \
  --config scenario.yml \
  --format postgres-sql \
  --output forum.sql
```

The CLI supports `json`, `jsonl`, `csv`, `sqlite-sql`, `postgres-sql`, and
`prisma-seed` exports. When `--output` is omitted, output is written to stdout.

## Determinism

For the same supported inputs, the forum generator and exporters produce the
same dataset and export text. Deterministic derivation is based on the explicit
seed, scenario config, deterministic RNG algorithm, and selected export format.
Changing the seed, counts, community list, or format can change the output.

Each generated user has exactly one persona record. Persona numeric scores are
bounded `0.0..=1.0` values for Big Five traits plus posting frequency,
controversy affinity, humor affinity, technical depth, and meme affinity.
Categorical persona fields are `verbosity`, `sleep_phase`, and
`activity_pattern`, serialized as `snake_case`.

The forum generator includes a static in-code interest catalog. Each user gets
2-5 deterministic interest IDs scored from persona traits: technical depth
boosts programming/Linux/systems/tooling interests, extroversion plus posting
frequency boosts community/events/collaboration interests, and humor plus meme
affinity boosts gaming/memes/culture interests. Community membership and
post/comment author selection use those persona signals where matching records
exist while preserving requested counts and referential consistency.

User bios are generated after personas and interests exist. They are offline
template output based on role phrases, assigned interests, verbosity,
technical/humor/meme affinities, and agreeableness or controversy affinity.
They do not use LLM-backed generation or runtime learning.

Synthetic Pop does not currently apply policy filtering or external data-pack
loading to generated records. Do not treat generated output as policy-screened
content.

## Local Validation

Run the Rust workspace checks from the repository root:

```sh
cargo build
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

See `docs/QUALITY.md` for the expected outcome and current test scope.

## License

Licensed under the Apache License, Version 2.0.
See [LICENSE](./LICENSE.txt).
