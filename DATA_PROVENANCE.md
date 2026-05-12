# Data Provenance

Synthetic Pop is designed to generate synthetic data without copying private or proprietary source datasets. This document separates the current scaffold state from the provenance requirements needed before generation ships.

## Current Milestone 1 Scaffold

- No data packs are implemented.
- No generator is implemented.
- No real, scraped, purchased, or third-party dataset is bundled.
- The repository currently contains only source code scaffolding, placeholders, and trust documentation.

## Provenance Principles

- Inputs must be documented, reviewable, and license-compatible.
- Synthetic outputs must not be represented as real people, real communities, or observed user behavior.
- Public examples should be small, artificial fixtures unless a documented public source is explicitly approved.
- Any future data pack must include origin, license, transformation notes, and update process.

## Required Before Alpha Generation

- Add a provenance manifest format for every data pack.
- Document whether each data pack is hand-authored, procedurally generated, derived from public domain material, or derived from another approved source.
- Reject data packs without clear licensing and transformation history.
- Provide contributor guidance for adding or changing provenance metadata.

## Required Before v0

- Version every shipped data pack.
- Publish a provenance summary for all bundled packs.
- Add automated checks that fail when required provenance fields are missing.
- Maintain a changelog for material data pack changes.
