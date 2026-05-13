# Synthetic Pop

Synthetic Pop is an offline, deterministic synthetic community generator for
demos, staging data, simulations, and repeatable test fixtures. The first
implemented product wedge is a forum community generator that produces
synthetic users, communities, posts, comments, membership relationships, and
activity events from explicit inputs.

Current status: Milestone 5 scenario generation and export. The Rust workspace
contains a core engine API, request validation primitives, deterministic
field-level RNG primitives, reproducibility metadata, a serde-ready canonical
model, forum scenario generation, CLI generation, and forum dataset exports.
Policy filtering, language bindings, desktop, and WASM surfaces are still
unavailable.

## Rust Core Foundation

`crates/core` exposes the implemented foundation:

- `current_status()` reports `milestone 5 scenario generation and export`.
- `CoreEngine` owns a `CoreEngineConfig` and exposes current capabilities.
- `CoreCapabilities` marks the offline/local core, deterministic RNG,
  canonical data model, forum generation, and exports as available. Policy
  filtering, bindings, desktop, and WASM remain unavailable.
- `Seed`, `GenerationSize`, and `CoreRunRequest` validate request inputs.
  Empty seeds, zero sizes, and sizes above `100_000` are rejected.
- `DeterministicRandom` and the helper functions `random_u64`,
  `random_bounded_u64`, `random_bool`, `random_index`, and `random_choice`
  derive repeatable field-level values from `(seed, namespace, entity_id,
  field)`.
- `reproducibility_metadata()` reports the current metadata version and the
  deterministic RNG algorithm identifier/version.
- `synthetic_pop_core::model` defines the canonical record surface for users,
  profiles, usernames, personas, interests, statuses, posts, comments,
  reactions, relationships, communities, organizations, and activity events.

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

Synthetic Pop does not currently apply policy filtering or external data-pack
loading to generated records. Do not treat generated output as policy-screened
content.

## Repository Layout

- `crates/core`: Rust core foundation with deterministic RNG primitives,
  reproducibility metadata, canonical model types, and capability status.
- `crates/scenarios`: implemented `forum` scenario config parsing and dataset
  generation.
- `crates/export`: implemented forum dataset exports.
- `crates/cli`: CLI status and `generate forum` command.
- `crates/policy`: deferred policy filtering surface.
- `apps/desktop`: placeholder for a future desktop application.
- `apps/web-demo`: placeholder for a future web demo or WASM-backed surface.
- `bindings/python`: placeholder for future Python bindings.
- `bindings/node`: placeholder for future Node bindings.
- `data-packs`: placeholder for future local, versioned data packs.
- `examples`: placeholder for future example projects and fixtures.
- `docs`: design and quality notes.

## Trust Documents

- `REPRODUCIBILITY.md`: current deterministic generation/export guarantees and
  future compatibility requirements.
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

Policy filtering, data pack loading, packaged examples, desktop app, web app,
Tauri integration, WASM, and language bindings remain deferred. They are
represented only by placeholders or capability flags until later milestones
define and implement their behavior.
