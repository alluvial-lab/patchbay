---
id: research-handoff-spawn-reconnect-cursor-reconcile
kind: story
stage: implementing
tags: [adapter, protocol, verification]
parent: research-handoff-spawn
depends_on: [research-handoff-spawn-restart-continuation-orchestration]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-12
---

# Reconnect and cursor reconciliation across generation replacement

## Checkpoint

Prove that endpoint, adapter, and core reconnect converge on the durable logical-target/current-generation state. A remembered stream, WebSocket, Pi stdout event, process handle, or wall-clock age never establishes liveness. The core replays its authority-domain log/cursor; the Pi adapter reconciles persisted session entries with `get_entries(since)` semantics; control surfaces consume newer events and/or an authoritative snapshot.

An unknown Pi entry cursor is an explicit resync path, not an empty suffix. Reconnect does not increment a runtime generation. Only a committed spawn continuation claim + authenticated replacement report advances it.

## Design

**Files**
- `contracts/proto/patchbay/sessions.proto` — carry logical-target/current-runtime/continuation metadata in `SessionSnapshot` and checkpoint records.
- `core/src/session/registry.rs`, `core/src/session/replay.rs`, and `server/src/snapshot.rs` — replay/checkpoint exact logical-target and tombstone state under domain/core-generation/LSN anchors.
- `server/src/adapter_service.rs` — adapter delivery reconnect begins from durable cursor and current attachment epoch; no remembered stream authority.
- `pi-adapter/src/cursor_store.ts` (new) — persist accepted Pi entry cursor per logical target/runtime generation only after processing.
- `pi-adapter/src/spawn_supervisor.ts` and `pi-adapter/src/pi_session.ts` — fetch suffix/current leaf after continuation; full explicit resync on unknown cursor.
- `web-cockpit/src/domain/reconcile.ts` and `web-cockpit/src/domain/model.ts` — replace cached logical/current generation from newer snapshot/event and keep stale presentation until confirmed.
- `contracts/vectors/spawn-reconnect-generation.json` (new) plus core/server/Pi/cockpit checks — cross-layer executable trace.

```ts
export interface PiEntryCursorStore {
  load(logicalTargetId: string, generation: bigint): Promise<string | undefined>;
  commit(logicalTargetId: string, generation: bigint, entryId: string): Promise<void>;
  clear(logicalTargetId: string, generation: bigint): Promise<void>;
}

export type PiCursorReconcile =
  | { kind: "suffix"; entries: readonly unknown[]; leafId: string | null }
  | { kind: "full-resync"; entries: readonly unknown[]; leafId: string | null };
```

Adapter and control-surface cursors remain different authorities: Pi entry ids order persisted Pi session entries; core `(authority_domain_id, LSN)` orders Patchbay lifecycle state. Neither cursor is converted into the other.

## Acceptance evidence

- [ ] Reconnect after missing the N→N+1 transition returns one logical target with N tombstoned and N+1 current; cached N cannot overwrite it.
- [ ] Endpoint detach/reconnect leaves the generation unchanged when the runtime remains reachable.
- [ ] Adapter reconnect/replacement reauthenticates attachment generation, reconciles Pi persisted entries, and reports current generation with a fresh source revision.
- [ ] Unknown Pi cursor triggers full resync and explicit evidence; it never yields false empty/current state.
- [ ] Core replay/checkpoint recovery reconstructs claims, logical targets, tombstones, continuation status, commands, and descendant authority from the durable prefix.
- [ ] Cursor replay is idempotent; duplicated entries/events do not duplicate transcript or lifecycle mutations.
- [ ] Stale/unknown remains visually stale/unknown until authoritative evidence confirms live.
- [ ] `reconnect-after-stream-loss`, `detach-does-not-retire`, and `cursor-gap-repair` vectors pass end to end and kill remembered-stream-as-live mutations.

## Ordering constraint

Depends on completed restart orchestration; it is the final lifecycle convergence checkpoint.
