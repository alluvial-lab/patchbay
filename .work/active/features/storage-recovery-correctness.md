---
id: storage-recovery-correctness
kind: feature
stage: drafting
tags: [foundation, protocol]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-08
updated: 2026-08-09
---

# Storage recovery correctness

## Brief
Consolidate open durability and recovery-cost follow-ups into one storage correctness feature. Absorbed findings:

- **`backlog-core-generation-persistence`** — implement persistence + cross-incarnation validation for the wire-present core generation once restart ambiguity makes it load-bearing. *Src:* docs-audit 2026-07-27. *Currency (2026-08-09 review):* **OPEN** — session + resource snapshot materialization still set `core_generation: None` (`server/src/state.rs:304-310,383-389`); foundation still calls the field reserved (`PROTOCOL.md:455-458`, `GLOSSARY.md:27-39`). *Direction:* define whether core generation is a durable storage-continuity epoch or a per-process restart counter — this is **load-bearing**: foundation says it's assigned on restart AND that prior-generation snapshots are rejected, but a recovery checkpoint was necessarily written by the *previous* incarnation, so rejecting it on restart discards the checkpoint and forces full-log replay (which makes the checkpoint-writer finding ineffective). Resolve the contradiction before either finding ships. *Disposition:* **split** into snapshot/core-generation semantics + validation.
- **`backlog-snapshot-checkpoint-writer`** — add a production checkpoint writer + scheduling policy so recovery replay cost stays bounded as the durable log grows. *Src:* docs-audit 2026-07-27. *Currency:* **OPEN** — production state explicitly says durable checkpointing is deferred (`server/src/state.rs:242-247`) and recovery replays the whole log (`PROTOCOL.md:578-584`); a snapshot table + write port exist (`core/src/storage/rusqlite.rs:65-69,806-830`) but no production scheduling/materialization writer. *Direction:* define an explicit policy (committed events/bytes or measured replay cost), crash-safe retry + failure observability; snapshot write failure must leave the log authoritative + recovery-correct. Note: the checkpoint namespace is session-only today; a session-only checkpoint does NOT bound whole-core recovery (authority/command/Elicitation/resource rebuild from log). *Disposition:* **split** into recovery-cost/checkpoint implementation; depend on the generation-semantics decision.

*Currency verified 2026-08-09. Per the review this feature should **split into 2** (snapshot/core-generation semantics; production checkpoint writer) AND resolve one contradiction (core-generation rejection vs restart checkpoint) before shipping either. Also: the "Simplification opportunity" line "no second state store" is **ambiguous and foreclosing** — current architecture already permits a derived snapshot table + says replicas/second backends are post-v1 seams; restate the invariant as "no derived checkpoint/replica/projection may become an independent ordering or authority source; recovery validates it against the durable log's domain/epoch/LSN anchor."*

## Simplification opportunity
Keep snapshots as derived recovery checkpoints and preserve the durable event log as the sole ordering authority; do not introduce a second state store or a parallel recovery path.
