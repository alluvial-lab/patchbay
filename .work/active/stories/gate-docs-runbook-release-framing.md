---
id: gate-docs-runbook-release-framing
kind: story
stage: drafting
tags: [documentation]
parent: null
depends_on: []
release_binding: null
gate_origin: docs
created: 2026-08-11
updated: 2026-08-11
---

# Runbook v0.1.0 framing now encloses post-v0.1 operations

## Drift category
readme-staleness

## Location
- Doc: `docs/RUNBOOK.md:1`
- Contradicting source: `docs/RUNBOOK.md:80`

## Current doc text
> # Patchbay v0.1.0 Runbook

The same document now instructs operators on post-v0.1 revocation, lockdown, diagnostics, source-ordering, and recovery behavior while its title and limitations heading still frame the whole artifact as the v0.1.0 runbook.

## Contradiction

The operational instructions describe the current v0.2.0-capable system, not only the v0.1.0 walking skeleton. The version framing makes current guidance look historical and labels current limitations as v0.1.0-only.

## Required edit

Rename the document and current operational/limitations framing to version-neutral current truth. Keep any genuinely v0.1.0-specific claims narrowly labeled rather than wrapping the whole runbook in the old version.

## Release disposition

Parked unbound under the operator's low-risk gate policy; it does not block v0.2.0 shipment.
