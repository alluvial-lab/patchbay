---
source_handle: happy-relay
fetched: 2026-08-08
source_url: https://github.com/slopus/happy/tree/2c8ecacc19f14abd81111a4605ac8c7f6bedb7e1
provenance: source-direct
---

## Summary

Happy is an end-to-end-encrypted session relay and remote-control system. It separates persistent updates from ephemeral presence, stores encrypted session messages in Postgres, uses monotonic sequence numbers and cursor fetch for reconnect recovery, deduplicates messages by client `localId`, and applies optimistic version checks to mutable state. RPC ownership is live Socket.IO room membership and is re-registered after reconnect. The CLI's normal message outbox is memory-resident, while the special offline-session stub discards method calls until the real session is swapped in.

## Structural metadata

- Repository: `slopus/happy`
- Commit: `2c8ecacc19f14abd81111a4605ac8c7f6bedb7e1`
- Commit subject: `chore(app): August 7 changelog entry with Community Credits convention`
- Fetched as a local Git clone and inspected at the pinned commit.

## Key passages

1. The protocol separates database-backed `update` events, each with a per-user monotonic sequence, from ephemeral presence and usage. Connection scopes are user, session, and machine; recipient rooms route durable updates only to the user's relevant connections. (`docs/protocol.md`, “Protocol design motivations,” WebSocket events, and sequencing; `docs/backend-architecture.md`, “Realtime sync architecture”; `packages/happy-server/sources/app/events/eventRouter.ts`)

2. `allocateUserSeq` and `allocateSessionSeq` atomically increment database counters. The v3 read endpoint verifies session ownership, returns messages by `seq` after a caller cursor in ascending order, supports bounded pagination, and also supports backward history paging. (`packages/happy-server/sources/storage/seq.ts`; `packages/happy-server/sources/app/api/routes/v3SessionRoutes.ts`, GET route)

3. The v3 write endpoint requires a nonempty `localId` for every message, verifies the target session belongs to the authenticated account, deduplicates within the request and against stored messages, allocates session sequence numbers transactionally for only new messages, and returns both existing and newly created records. The database has `@@unique([sessionId, localId])` and an index on `(sessionId, seq)`. (`packages/happy-server/sources/app/api/routes/v3SessionRoutes.ts`, POST route; `packages/happy-server/prisma/schema.prisma`, `SessionMessage`)

4. `ApiSessionClient` keeps `pendingOutbox` as an in-memory array. Enqueue encrypts content and assigns a random UUID; flush sends batches over the v3 endpoint and removes entries after success. The implementation sends the newest pending batch first and backfills older entries later. On socket reconnect, cursor fetch is invalidated; an exact next sequence can be applied directly, while a gap triggers authoritative HTTP fetch after `lastSeq`. (`packages/happy-cli/src/api/apiSession.ts`, `pendingOutbox`, socket `connect`/`update`, `fetchMessages`, `flushOutbox`, and `enqueueMessage`)

5. If initial server session creation fails, the CLI creates an `offline-<tag>` stub and retries session creation in the background. Every stub send, lifecycle, metadata, state, control-transfer, and handler-registration method is a no-op until reconnection swaps in a real session. (`packages/happy-cli/src/utils/offlineSessionStub.ts`; `packages/happy-cli/src/utils/setupOfflineReconnection.ts`)

6. Session metadata and agent state updates require `expectedVersion`. The server first compares the current version, then uses an update condition containing that version; mismatches return the current version/value instead of silently overwriting. Similar versioning is documented for machine state, artifacts, access keys, and KV. (`packages/happy-server/sources/app/api/socket/sessionUpdateHandler.ts`; `docs/protocol.md`, “Optimistic concurrency”)

7. RPC method registration is represented by Socket.IO room membership. Disconnect automatically removes membership; reconnect causes `RpcHandlerManager` to re-emit registrations. Calls fail when no owner is present or when the owner disappears mid-call rather than becoming a durable queued command. (`docs/realtime-sync-and-rpc.md`, “Room Model” and “RPC Flow”; `docs/multi-process.md`, daemon lifecycle; `packages/happy-cli/src/api/rpc/RpcHandlerManager.ts`)

8. Authentication is account-wide Bearer-token based, while routes verify that sessions and machines belong to that account. Socket fan-out further scopes recipients by user/session/machine rooms. `AccessKey` records are encrypted, unique per `(accountId, machineId, sessionId)`, and versioned. (`docs/backend-architecture.md`, authentication and storage; `packages/happy-server/prisma/schema.prisma`, `AccessKey`; `packages/happy-server/sources/app/api/routes/v3SessionRoutes.ts`)

9. Resume resolves a stable Happy session record, decrypts stored metadata, and requires a path plus optional underlying Claude session id or Codex thread id. Local resume state retains encryption/version/sequence data; server metadata can refresh missing provider-native resume identifiers. (`packages/happy-cli/src/resume/resolveHappySession.ts`; `packages/happy-cli/src/resume/localResumeStore.ts`)

10. Session and message sequence/version fields order state and reject conflicting writes, but the fetched runtime protocol has no target-incarnation/generation field that fences commands or events from an old process after a replacement. Turn and subagent ids in the session rendering protocol identify conversational events, not process generations. (`docs/protocol.md`; `docs/session-protocol.md`; repository-wide search at the pinned commit)
