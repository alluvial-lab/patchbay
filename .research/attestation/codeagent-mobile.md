---
source_handle: codeagent-mobile
fetched: 2026-08-08
source_url: https://github.com/edgar-durand/codeagent-mobile-clients/tree/736509c05993b3c11f268eb44e0d49b4d704f02a
provenance: source-direct
---

## Summary

The fetched CodeAgent Mobile repository contains public CLI and IDE client bridges, not the backend, mobile app, or web dashboard. Its command relay advertises backend-queued at-least-once delivery with explicit acknowledgements and client-side deduplication. Its baton controller serializes turn-safe ownership transfers between local and mobile drivers and retains a provider conversation id across driver replacement. Authentication is pairing-scoped. Baton publication and several lifecycle signals are deliberately best-effort, and the client protocols contain no runtime-generation fence.

## Structural metadata

- Repository: `edgar-durand/codeagent-mobile-clients`
- Commit: `736509c05993b3c11f268eb44e0d49b4d704f02a`
- Commit subject: `chore(changelog): notes for v2.61.95 [skip ci]`
- License presented by repository: MIT
- Fetched as a local Git clone and inspected at the pinned commit.

## Key passages

1. The README identifies this repository as the public source for client-side bridges. Its architecture diagram places an external CodeAgent backend between mobile/web and these clients, and it explicitly directs backend/mobile/web issues elsewhere because those sources are not in this repository. (`README.md`, introduction, architecture, and contributing notes)

2. A remote command envelope contains `id`, `sessionId`, `pluginId`, type, payload, status, and creation time. Shared validation drops malformed individual commands without discarding the whole batch. (`packages/shared/src/protocol/remote-command.ts`)

3. The CLI relay advertises acknowledgement support on SSE and polling delivery requests. Source comments state that the backend then peeks commands non-destructively and drains them only after command-id acknowledgement, giving at-least-once delivery. The client keeps a bounded in-memory set of processed ids to suppress reconnect/redelivery duplicates. (`apps/cli/src/services/command-relay.service.ts`, class fields, `pollSecretHeader`, `dispatchCommands`, and `rememberProcessed`)

4. `dispatchCommands` posts receipt acknowledgements for every received id before invoking command handlers. A failed acknowledgement leads to redelivery; a process failure after a successful acknowledgement but before/during dispatch is not repaired by the fetched client. The dedup set is memory-only and capped at 1,000 ids. (`apps/cli/src/services/command-relay.service.ts`, `dispatchCommands`, `ackCommands`, and `processedIds`)

5. The baton protocol defines steady `LOCAL_DRIVE` and `MOBILE_DRIVE` states plus transient `SWITCHING`. `BatonController` permits a switch only from the expected steady state, waits for a safe turn boundary, stops the current driver, starts the next using the retained conversation id, then changes the active driver. Non-baton commands route to the live active driver. (`apps/cli/src/baton/types.ts`; `apps/cli/src/baton/baton-controller.ts`; `apps/cli/src/baton/wire-baton.ts`, `makeOnCommand`)

6. On a switch exception, the controller restores its prior state variables and republishes the prior steady state. It does not restart the driver it already stopped before the failed next-driver start. (`apps/cli/src/baton/baton-controller.ts`, `switchDriver` catch path)

7. Baton state POSTs are serialized client-side so `SWITCHING` cannot be overtaken by the following steady state. The source says the backend writes a Redis snapshot and publishes SSE before returning success, but the client call is fire-and-forget/non-fatal, `postBatonEvent` converts transport failure to `{ ok: false }`, and the publisher chain discards that result. (`apps/cli/src/baton/wire-baton.ts`, `makeSerializedBatonPoster` and controller composition; `apps/cli/src/services/pairing.service.ts`, `postBatonEvent`)

8. Pairing establishes a per-pairing plugin authentication token for command/output POSTs and a proof-of-possession poll secret for command delivery and reconnect. Source comments say the backend HMAC binds plugin authentication to the body’s `(sessionId, pluginId)`. Legacy pairings may omit the poll secret. (`apps/cli/src/services/pairing.service.ts`, `PairedUserInfo`, reconnect, and authenticated POST notes; `apps/cli/src/services/command-relay.service.ts`, `pollSecretHeader`)

9. Native ACP guardrails classify risky actions as deny/confirm/off, but their own source states that they are soft guardrails rather than a security boundary: in-process, bypass-permission, and PTY tool calls can bypass them. The file says real containment belongs server-side in scoped/revocable tokens and per-user containers, whose implementation is outside the fetched repository. (`packages/shared/src/guardrails/index.ts`)

10. The baton carries a conversation id and protects late binding from clobbering a live conversation, but the command and baton envelopes have no runtime generation/incarnation. A stale client process with the same session/plugin tuple is not visibly fenced by a monotonic generation in this fetched client protocol. (`apps/cli/src/baton/baton-controller.ts`; `packages/shared/src/protocol/remote-command.ts`; repository-wide search at the pinned commit)

11. A separate end-of-turn file-change outbox is append-only JSONL on disk, uses a stable `turnId`, atomically compacts with temp-file rename, retries transient failures, and relies on a backend SETNX gate for deduplication. It expires entries after 24 hours. This is durable payload telemetry for turn files, not the general remote-command execution path. (`apps/cli/src/services/turn-files/files-outbox.ts`)

## Acquisition record

- Public GitHub enumeration for `edgar-durand` on 2026-08-08 exposed `codeagent-mobile-clients` and `codeagent-mobile-ide`; the latter describes a headless IDE component library, not the relay backend.
- Direct public Git probes for plausible backend repository names under `edgar-durand` and `CodeAgentMobile` returned repository-not-found.
- The fetched README itself states that backend/mobile/web sources are not in this repository. The backend implementation therefore remains unavailable from the inspected public source surface.
