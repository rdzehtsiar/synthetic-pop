# Safety Policy

Synthetic Pop is intended for legitimate demo, testing, staging, simulation, and seed-data workflows. It must not be used to impersonate real people, evade trust systems, or create deceptive identities.

## Current Milestone 1 Scaffold

- No policy filtering engine is implemented.
- No generator, exporter, app, binding, or data pack is implemented.
- The current CLI placeholder does not create identities or communities.
- This document defines the safety bar for future work; it is not evidence that filtering exists today.

## Allowed Use Cases

- Local demo datasets for applications.
- Integration and staging fixtures.
- Synthetic community simulations.
- Repeatable test data for development and QA.
- Educational examples that are clearly synthetic.

## Disallowed Use Cases

- Impersonating real people or organizations.
- Creating accounts or profiles for deception, fraud, spam, harassment, or evasion.
- Producing credentials, government identifiers, payment data, or secrets that appear valid.
- Generating content intended to bypass platform moderation or identity checks.
- Presenting synthetic outputs as observed real-world data.

## Required Before Alpha Generation

- Add policy checks for scenarios and exports that can block unsafe fields, formats, or requested use cases.
- Mark generated outputs as synthetic through metadata where the format supports it.
- Avoid generating valid-looking sensitive identifiers unless a future policy explicitly allows safe dummy formats.
- Add tests that cover allowed and disallowed policy cases.

## Required Before v0

- Document policy behavior for each supported scenario and export format.
- Keep policy failures clear and actionable for legitimate users.
- Review new data packs and scenarios against this policy before release.
- Treat safety regressions as release blockers.
