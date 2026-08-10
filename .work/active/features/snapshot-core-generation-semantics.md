---
id: snapshot-core-generation-semantics
kind: feature
stage: drafting
tags: [foundation, protocol]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-09
updated: 2026-08-09
---

# Snapshot / core-generation semantics

## Brief
Define core generation's role in snapshot/recovery, split out of `storage-recovery-correctness`. Absorbs:

- `backlog-core-generation-persistence` — **OPEN**: session + resource snapshot materialization still sets `core_generation: None` (`server/src/state.rs:304-310,383-389`); foundation calls the field reserved (`PROTOCOL.md:455-458`, `GLOSSARY.md:27-39`). *Src:* docs-audit 2026-07-27.

## The contradiction this must resolve first (load-bearing)
Foundation defines core generation as core-assigned **on restart** and says snapshots from another generation are rejected (`GLOSSARY.md:27-39`, `VERIFICATION.md:218-225`). But a recovery checkpoint is necessarily written by the **previous** incarnation. If a new core increments generation and rejects the previous generation's snapshot, **every restart discards the checkpoint and replays the full log** — which makes `recovery-checkpoint-writer` ineffective. This contradiction blocks both storage children until resolved.

## Direction
Decide explicitly: either (a) core generation is a **durable storage-continuity epoch** (not a per-process restart counter), so a restart continues the same epoch and accepts its own prior checkpoint; or (b) recovery checkpoints require a separately specified compatibility rule that survives an incarnation change. Then implement persistence + cross-incarnation validation for the wire-present field once restart ambiguity makes it load-bearing. Do not ship `recovery-checkpoint-writer` against an unresolved answer.

## Foundation references
- `docs/PROTOCOL.md` (`:455-458`), `docs/GLOSSARY.md` (`:27-39`), `docs/VERIFICATION.md` (`:218-225`)
- Code: `server/src/state.rs`
