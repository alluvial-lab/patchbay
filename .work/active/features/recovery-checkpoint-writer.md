---
id: recovery-checkpoint-writer
kind: feature
stage: drafting
tags: [perf, protocol, foundation]
parent: null
depends_on: [snapshot-core-generation-semantics]
release_binding: null
gate_origin: null
created: 2026-08-09
updated: 2026-08-09
---

# Recovery checkpoint writer + scheduling policy

## Brief
Add a production checkpoint writer so recovery replay cost stays bounded as the durable log grows. Split out of `storage-recovery-correctness` (a `[perf]`-bearing item the consolidation had silently dropped). Absorbs:

- `backlog-snapshot-checkpoint-writer` — **OPEN**: production state says durable checkpointing is deferred (`server/src/state.rs:242-247`) and recovery replays the whole log (`PROTOCOL.md:578-584`); a snapshot table + write port exist (`core/src/storage/rusqlite.rs:65-69,806-830`) but no production scheduling/materialization writer. *Src:* docs-audit 2026-07-27.

## Direction
Define an explicit policy over committed events/bytes (or measured replay cost), with crash-safe retry and failure observability; a snapshot write failure must leave the log authoritative and recovery-correct. **Scope the bound honestly**: the checkpoint namespace is session-only today, so a session-only checkpoint does NOT bound whole-core recovery (authority/command/Elicitation/resource rebuild from log) — either choose session-only with a narrowly stated bound, a typed composite checkpoint, or per-projection checkpoints each anchored to one durable prefix; don't claim globally bounded recovery until every load-bearing projection has a compatible checkpoint.

## The "no second state store" invariant (corrected)
The consolidation's "no second state store" was ambiguous and **foreclosing** (it could prohibit the existing derived snapshot table, replicas, or a second backend — all post-v1 seams). Restate the actual invariant: **no derived checkpoint, replica, or projection may become an independent ordering or authority source; recovery validates it against the durable log's domain/epoch/LSN anchor.** Physical-topology commitments (single backend, etc.) are scoped to v1.

## Foundation references
- `docs/PROTOCOL.md` (`:578-584`), `docs/ARCHITECTURE.md` (`:197-201` — snapshots are derived; log is ordering authority), `docs/SPEC.md` (post-v1 storage seams)
- Code: `core/src/storage/rusqlite.rs`, `server/src/state.rs`
