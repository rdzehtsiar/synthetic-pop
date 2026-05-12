# Benchmarks

Synthetic Pop should publish honest performance measurements once generation exists. Milestone 1 does not include benchmark numbers because there is no generator to measure.

## Current Milestone 1 Scaffold

- No benchmarks are implemented.
- No generator, policy filter, exporter, data pack loader, app, or binding is implemented.
- No throughput, memory, latency, or output-size claims are currently made.
- Any performance discussion before generation exists is a target, not a measurement.

## Benchmarking Principles

- Publish only reproducible measurements.
- Include hardware, operating system, Rust version, build profile, generator version, scenario version, seed, and configuration.
- Measure complete user-relevant workflows, not only isolated internals.
- Keep benchmark fixtures synthetic and clearly versioned.

## Required Before Alpha Generation

- Add at least one local benchmark path for the first supported scenario.
- Track generation time, peak memory where practical, and output size.
- Include a small deterministic fixture suitable for CI or local smoke checks.
- Document how to run benchmarks and how to interpret results.

## Required Before v0

- Publish benchmark results for supported release platforms.
- Compare release builds, not debug builds, for published numbers.
- Add regression thresholds for critical workflows once realistic baselines exist.
- Keep old benchmark results identifiable by version instead of overwriting context.
