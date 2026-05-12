# Offline Guarantee

Synthetic Pop's core promise is local, offline generation with no cloud service, API key, or telemetry requirement. This document states the current scaffold guarantee and the requirements for future generation releases.

## Current Milestone 1 Scaffold

- The Rust workspace has no third-party runtime service integration.
- The placeholder CLI does not call a network service.
- No desktop app, web demo, binding, exporter, data pack downloader, or generator is implemented.
- The repository may still rely on normal developer tooling such as Cargo during local development.

## Offline Product Requirements

- Core generation must run without internet access after the software and selected data packs are installed.
- No API keys or hosted providers may be required for generation.
- The default CLI path must not send telemetry or generated data off the machine.
- Any future feature that can access the network must be explicit, documented, optional, and outside the offline core path.

## Required Before Alpha Generation

- Add tests or checks that protect the core generator from accidental network dependencies.
- Document installation steps that separate dependency download from offline execution.
- Ensure supported generation commands work with network access disabled.
- Keep data pack resolution local and versioned.

## Required Before v0

- Publish an offline validation procedure for release candidates.
- Document all optional network-capable features, if any exist.
- Make the offline path the default user experience for CLI generation.
- Treat unintended network access in core generation as a release blocker.
