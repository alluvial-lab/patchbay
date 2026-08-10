---
id: resource-reconciliation-followups
kind: feature
stage: drafting
tags: [adapter, protocol]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-08
updated: 2026-08-09
---

# Resource reconciliation follow-ups

## Brief
Consolidate the two items parked from the resource-state review into the resource reconciliation follow-up. Absorbed findings:

- **`backlog-resource-generation-obsolete-event-no-op`** — preserve obsolete-event no-op behavior when generation monotonicity and replay/catch-up ordering interact. *Src:* parked from the `…resource-state` review. *Currency (2026-08-09 review):* **OPEN** — the adapter-generation rejection runs first (`core/src/resource/registry.rs:100-112`), before per-view/per-record obsolete-LSN filtering (`registry.rs:119-149`), so an event otherwise obsolete for its affected records can still become corruption after a newer generation is projected — exactly as the finding states. *Direction:* the no-op rule must be defined at the replay/catch-up prefix boundary, not by weakening generation monotonicity inside the fold: an event is wholly inert only when the projection is known to represent a contiguous prefix through its LSN; a lower-generation event is otherwise corruption. Track a validated global applied cursor outside the resource fold. *Disposition:* **keep** — the obsolete-event semantic rule must be specified in protocol terms *before* the evidence is generated (the review's BLOCKER: a broad no-op can mask a real mutation).
- **`backlog-resource-reconciliation-arbitrary-sequences`** — expand reconciliation evidence to arbitrary sequences (the brief's "two-report sampler" framing is **stale**). *Src:* parked from the `…resource-state` review. *Currency:* **PARTIAL** — an arbitrary 1–20-step, 100-case report trace already exists (`core/tests/resource_reconciliation.rs:159-179`), but it fixes adapter generation to 1 (`:319-323`) and doesn't combine generation transitions, explicit replacements, replay, and terminal mutation attempts in one generated trace; focused replacement/generation/terminal tests exist separately (`resource_ingest.rs:172-273`, `resource_state.rs:15-77`). *Direction:* add the missing cross-dimensional traces (generation transitions within a generated trace; same-event replacements; post-terminal mutation attempts; obsolete catch-up prefixes; hot/replay/replay-twice equality after every accepted prefix; negative traces leave prefix+projection unchanged). *Disposition:* **keep**, narrowed to the missing dimensions — do NOT reimplement a generic arbitrary-sequence test.

*Currency verified 2026-08-09. Per the review this feature is **coherent as one narrowed feature** — both surviving concerns belong to resource reconciliation — but: (1) specify the obsolete-event no-op/corruption rule in protocol terms before generating evidence (it's a semantic choice, not just a test); (2) acknowledge the existing arbitrary-sequence coverage and focus only on the missing cross-dimensional traces.*

## Simplification opportunity
Express obsolete-event handling and arbitrary sequence evidence through the existing resource conformance/reconciliation fold rather than adding a parallel resource-state mechanism.
