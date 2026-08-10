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

## Obsolete-event no-op/corruption rule (specified 2026-08-09)

Specified in protocol terms — the review's BLOCKER (this semantic choice must be fixed *before* evidence is generated: a broad no-op can mask a real mutation). Verified against `core/src/resource/registry.rs` `apply_validated`. `docs/PROTOCOL.md:43` delegates resource obsolete/replacement/tombstone semantics to the resource-state contract, i.e. this feature.

**The problem (confirmed in code).** `apply_validated` runs the generation guard *first*: it computes `projected_generation` = max applied `source_adapter_generation` across the adapter's views (`registry.rs:101-107`) and rejects any event with `generation.value < projected_generation` as `CorruptLog` ("lowers adapter generation", `:108-112`) — *before* the per-record obsolete filter (`revision_lsn >= event_lsn → continue`, `:119-125` views / `:144-150` resources). So an event that is per-record-obsolete can still be rejected as corruption once a newer generation has been projected. This contradicts the feature's own observer contract ("a redelivered event at or below the record revision is inert") and bites under catch-up/reconnect re-feed ordering (ordered replay from LSN 0 does not produce it — the original finding correctly notes it is latent).

**Why per-record obsolete is NOT a sound inertness test (do not just reorder the checks).** A single `ResourceStateEvent` can touch several views and identities. An event may be obsolete for one projected record yet still carry a previously-unseen identity, a current mutation for another view, a terminal replacement, or a view revision not yet represented. Per-record `revision_lsn >= event_lsn` across the event's touched records is therefore necessary but not sufficient for inertness.

**Specified rule — inertness at the applied-prefix boundary, tracked outside the fold.**
1. Maintain a **validated applied-LSN cursor per `(authority_domain, adapter)`** — the highest *contiguous* LSN the projection has applied (the global applied prefix). This is global prefix state, NOT derivable from per-record `revision_lsn` (records are sparse; a record's revision can lag the prefix).
2. **Evaluate prefix-coverage BEFORE the generation guard.** An incoming event whose `event_lsn ≤ cursor` is **prefix-covered → inert audit no-op**, regardless of its source generation. This restores the "redelivered event at or below revision is inert" contract for the obsolete case without weakening generation monotonicity.
3. An event whose `event_lsn > cursor` is **new** → it must satisfy generation monotonicity (a lower source-generation event beyond the prefix is corruption: ordered application is monotonic per adapter) + per-record `from_revision` validation, then advance the cursor to the new contiguous frontier.
4. Generation monotonicity is thereby preserved as a **new-event** invariant; obsolete events are routed through the prefix cursor rather than the generation guard — so the two no longer collide.

**Cross-cutting seam.** This is the resource-plane instance of the cross-projection replay-integrity invariant (couples with the `authority-provenance-hardening` replay-gap-detection split and the sessions replay-equality work): a shared contiguous-prefix + gap-free + reject-`Unspecified` replay discipline across authority/session/resource projections. Scope the resource cursor here; promote to a shared replay-integrity seam if a second projection needs the same cursor.

**Evidence the feature must then generate** (narrows the arbitrary-sequence finding): obsolete catch-up-prefix events across adapter-generation transitions; a lower-generation event that is prefix-covered (inert, not corruption) vs one beyond the prefix (corruption); same-event replacements; post-terminal mutation attempts; hot/replay/replay-twice equality after every accepted prefix; negative traces whose rejected candidates leave the durable prefix + projection unchanged.
