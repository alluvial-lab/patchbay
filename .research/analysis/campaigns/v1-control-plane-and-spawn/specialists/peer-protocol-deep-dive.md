---
provenance: agent-synthesis
updated: 2026-08-08
---

# Peer protocol deep dive

## Bottom line

The fetched peers substantially narrow several individual claims that Patchbay might otherwise present as unique: durable task/run state, offline mutation queues, restart-surviving message deduplication, cursor recovery, optimistic version checks, pairing-scoped credentials, and turn-safe driver handoff all exist in the corpus. [mission-control-src]{2} [amux-outbox]{1} [amux-outbox]{4} [happy-relay]{2} [happy-relay]{3} [codeagent-mobile]{5} [codeagent-mobile]{8}

The remaining moat is the **composition**, not any one primitive: no fetched source demonstrates a general operation contract that combines durable acceptance through terminal disposition, caller-stable idempotency, target-incarnation fencing, authoritative post-restart reconciliation, and resource-scoped authority. {inferred: diverges} Mission Control is primarily durable task/run governance with approvals and workspace controls; amux has unusually concrete outbox/steering/dedup mechanics but deliberately expires or collapses intent and has no built-in auth; Happy has strong durable message/session synchronization but live RPC ownership and memory-only pre-send intent; CodeAgent Mobile has command IDs, an at-least-once relay claim, and a real baton state machine, but the public client acknowledges commands before execution and omits the backend that decides persistence and ownership. [mission-control-src]{2} [mission-control-src]{3} [mission-control-src]{4} [amux-outbox]{3} [amux-outbox]{7} [amux-outbox]{10} [happy-relay]{4} [happy-relay]{5} [happy-relay]{7} [codeagent-mobile]{1} [codeagent-mobile]{3} [codeagent-mobile]{4}

Patchbay should therefore avoid a broad “durability” claim. A defensible v1 claim is narrower: **accepted control intent remains accountable until a terminal disposition, retries are operation-idempotent, replacement runtimes are generation-fenced, reconnect reads authoritative state, and authority is bound to the addressed resource.** The peers close parts of that sentence; the fetched evidence does not close the whole sentence. {inferred: converges}

## Evaluation frame

For this facet, the five axes mean:

1. **Durable accepted intent** — after the control plane says accepted, intent survives client/server restart and remains represented until completed, rejected, cancelled, superseded, or explicitly expired/dropped.
2. **Idempotency** — a caller-stable operation identity makes retries return/reconcile the same logical operation rather than merely reducing duplicates in one short failure window.
3. **Fencing/generations** — a stale runtime incarnation cannot mutate current target state or execute current commands solely because it retains an old session/resource identifier.
4. **Authoritative recovery** — reconnect/restart reconstructs state from durable authority rather than remembered UI stream state or presence alone.
5. **Scoped authority** — credentials/claims bind action rights to the addressed resource and operation domain, not only to an account-wide or coarse role.

These are analytical criteria applied to the sources (`{extends}`), not terminology asserted by the peers.

## Protocol capability matrix

| Peer | Durable accepted intent | Idempotency | Fencing / generations | Authoritative recovery | Scoped authority |
|---|---|---|---|---|---|
| Mission Control | **Partial.** Tasks, runs, spawn history, and provisioning jobs persist, but `/api/spawn` creates a fresh internal id per call and its own GET path still reads logs; accepted async dispatch without a run id is marked manual-reconciliation-required. [mission-control-src]{3} [mission-control-src]{4} [mission-control-src]{5} | **Narrow.** Registration upserts and some downstream calls/jobs carry keys, but provisioning creates fresh UUIDs and the platform PRD itself names idempotent commands as incomplete. [mission-control-src]{6} | **Not found for runtime targets.** No operation/runtime incarnation contract was located. [mission-control-src]{10} | **Partial.** Durable task recovery and run-id reconciliation exist; spawn recovery and run-id-less acceptance are weaker. [mission-control-src]{2} [mission-control-src]{3} [mission-control-src]{4} | **Substantial coarse scope.** RBAC, agent/workspace-bound expiring/revocable keys, strict-workspace fail-closed checks; not an operation-descendant/resource-grant model. [mission-control-src]{7} [mission-control-src]{8} |
| amux | **Partial and bounded.** Browser commands and steering rows survive reconnect/restart, but queue limits, seven-day reconciliation, 14-day steering expiry, supersession, and explicit drops are part of the design. [amux-outbox]{1} [amux-outbox]{3} [amux-outbox]{6} [amux-outbox]{7} | **Strong for specific message/event windows.** Persistent `(session,msg_id)` dedup covers response-loss restart, but expires after ten minutes and fails open on storage error; event idempotency is separately durable within retention. [amux-outbox]{4} [amux-outbox]{8} | **Not found for workers.** Stable worker identity survives stop/start, while generation guards found in source protect cache/tunnel subsystems rather than worker commands. [amux-outbox]{9} | **Substantial for local state.** SQLite queues/events, pane/log/JSONL sources of truth, and replay reconciliation recover local truth; this is not one general operation-state machine. [amux-outbox]{2} [amux-outbox]{5} [amux-outbox]{6} [amux-outbox]{8} | **Absent as a network boundary.** The project explicitly has no built-in auth. [amux-outbox]{10} |
| Happy | **Partial by layer.** Server-accepted messages are durable and deduplicated; pre-accept CLI outbox state is memory-only, and the offline stub discards calls. RPC is live routing, not durable command acceptance. [happy-relay]{3} [happy-relay]{4} [happy-relay]{5} [happy-relay]{7} | **Strong for messages.** Required local IDs plus a database uniqueness constraint and transactional lookup make v3 message retry idempotent. [happy-relay]{3} | **Resource-version checks only.** `expectedVersion` prevents stale overwrites of mutable blobs, but no replacement-runtime generation fences later events/commands. [happy-relay]{6} [happy-relay]{10} | **Strong for messages/session state.** Per-user/per-session sequence allocation and cursor fetch repair gaps; resume rebuilds from durable encrypted metadata and provider-native identifiers. [happy-relay]{1} [happy-relay]{2} [happy-relay]{9} | **Partial.** Account ownership checks, scoped rooms, and per-session/per-machine encrypted access keys constrain data/routing, but the primary bearer identity is account-wide. [happy-relay]{7} [happy-relay]{8} |
| CodeAgent Mobile | **Acquisition-gated partial.** Client source describes a backend-held, ack-drained at-least-once command queue, but the client acks before handler execution; backend durability and terminal-state semantics are unavailable. [codeagent-mobile]{1} [codeagent-mobile]{3} [codeagent-mobile]{4} | **Partial.** Command IDs and bounded in-memory processed-id dedup suppress ordinary redelivery; process restart can forget dedup. A different file-change outbox has disk persistence and backend SETNX dedup, but is not the command path. [codeagent-mobile]{3} [codeagent-mobile]{4} [codeagent-mobile]{11} | **Not found.** Baton uses conversation/session/plugin identities without a monotonic incarnation. [codeagent-mobile]{10} | **Relay/reconnect oriented.** SSE watchdog, polling fallback, snapshots, and baton publication exist; baton publication is best-effort, and failed driver start restores state variables without restarting the stopped driver. [codeagent-mobile]{6} [codeagent-mobile]{7} | **Meaningful pairing scope, backend-gated.** Plugin token and poll secret are tied to pairing/session/plugin; ACP guardrails explicitly are not a hard boundary. [codeagent-mobile]{8} [codeagent-mobile]{9} |

## What peers have closed

### Message duplication and reconnect are not a moat by themselves

amux directly handles the “effect landed, response vanished, server restarted, client retried” case with a persisted message id table and pane-level delivery verification. [amux-outbox]{4} [amux-outbox]{5} Happy goes further for encrypted session messages: caller IDs are database-unique, batch writes return existing records on retry, and sequence cursors recover gaps. [happy-relay]{2} [happy-relay]{3} CodeAgent Mobile's public relay also exposes command ids, at-least-once redelivery, and acknowledgements, though its exact backend guarantee remains unavailable. [codeagent-mobile]{2} [codeagent-mobile]{3}

Patchbay's differentiator cannot be “we reconnect” or “we deduplicate.” It must specify the durability domain, idempotency lifetime, terminal-state obligation, and stale-target behavior. {inferred: qualifies}

### Durable task state and approval gates are established peer territory

Mission Control persists task/run lifecycle, retries, quality review, spawn diagnostic records, audit, and workspace security. [mission-control-src]{1} [mission-control-src]{2} [mission-control-src]{5} amux adds SQLite task claims, status gates, steering queues, and observable action events. [amux-outbox]{6} [amux-outbox]{8} These are substantial control-plane capabilities; Patchbay should not equate a richer dashboard state machine with the missing protocol guarantees. {inferred: qualifies}

### Authority transfer has a concrete peer state machine

CodeAgent Mobile's `LOCAL_DRIVE → SWITCHING → MOBILE_DRIVE` controller, turn-boundary yield, one-active-driver routing, retained conversation id, and serialized state publication are reusable architectural evidence. [codeagent-mobile]{5} [codeagent-mobile]{7} This closes the claim that peers have only passive mirrors. It does not close durable ownership transfer because publication is non-fatal, runtime generations are absent, and switch failure can restore nominal ownership after stopping the old driver. [codeagent-mobile]{6} [codeagent-mobile]{7} [codeagent-mobile]{10}

### Recovery cursors and optimistic conflicts are established patterns

Happy's sequence allocation, forward cursor, gap-triggered fetch, and version-mismatch responses provide a concrete authoritative synchronization baseline. [happy-relay]{2} [happy-relay]{4} [happy-relay]{6} Patchbay must connect that baseline to operation status and target generation rather than presenting sequence ordering alone as novel. {inferred: extends}

## What remains open

1. **No general non-orphaning acceptance contract.** Mission Control can require manual recovery after accepted dispatch without a run id; amux expires/drops old queued intent; Happy can discard offline-stub calls and loses a process-resident outbox on crash; CodeAgent acknowledges before execution. [mission-control-src]{3} [amux-outbox]{7} [happy-relay]{4} [happy-relay]{5} [codeagent-mobile]{4}
2. **No agent-target generation fence.** Happy has field versions and amux has subsystem-local generations, but none of the fetched target protocols attach and enforce a runtime incarnation on command/event mutation. [amux-outbox]{9} [happy-relay]{10} [codeagent-mobile]{10} [mission-control-src]{10}
3. **No combined authority + operation provenance.** Mission Control and CodeAgent supply meaningful credential scoping, while Happy scopes account/session/machine data. None of the fetched contracts demonstrates an operation-derived grant tied to target generation and later terminal evidence. {inferred: diverges} [mission-control-src]{7} [mission-control-src]{8} [happy-relay]{8} [codeagent-mobile]{8}
4. **No uniform authoritative recovery across commands, state, and ownership.** Happy is authoritative for messages/session state but live for RPC; CodeAgent's ownership snapshot is backend-dependent and best-effort from the client; Mission Control's spawn route and durable spawn-history module are not one integrated path. [happy-relay]{2} [happy-relay]{7} [codeagent-mobile]{1} [codeagent-mobile]{7} [mission-control-src]{4} [mission-control-src]{5}

## Mission Control architectural direction to harvest

Mission Control offers directly reusable architectural direction because its shipped structure is adapter-neutral and MIT-presented, even though its operation semantics do not close the moat. [mission-control-src]{1}

1. **Declared capability depth, separate from runtime detection.** Keep “is installed/reachable” separate from “what this adapter can honestly guarantee,” require a complete manifest, and default uncertain capability fields to false. [mission-control-src]{9} Patchbay should add durability-specific dimensions such as external dedup strength, continuation proof, authoritative cursor support, and generation-fence support (`{extends}`).
2. **Durable run/task provenance beside adapter-native identifiers.** The `runs` and spawn records demonstrate useful separation of task relation, runtime/session identity, lineage, status, outcome, cost, and evidence. [mission-control-src]{5} Patchbay should retain that separation but make accepted Operation the command source of truth rather than an optional diagnostic record (`{extends}`).
3. **Atomic claim before delivery.** Mission Control's compare-and-swap task claim prevents two scheduler workers from concurrently dispatching one assigned task. [mission-control-src]{2} Reuse the case, then strengthen it with caller idempotency and target generation (`{extends}`).
4. **Represent reconciliation capability honestly.** Mission Control explicitly marks accepted-without-run-id work as manual reconciliation rather than pretending it can wait safely. [mission-control-src]{3} Patchbay adapters should likewise declare reconciliation strength and return `unknown`/manual-required when the substrate cannot prove an outcome (`{extends}`).
5. **Fail-closed workspace boundaries.** Agent/workspace-bound expiring and revocable keys plus strict-workspace denial are a useful deployment authority layer. [mission-control-src]{7} [mission-control-src]{8} Preserve the fail-closed posture, but do not mistake viewer/operator/admin role derivation for fine-grained operation authority (`{inferred: qualifies}`).

## Reusable conformance cases

| Vector | Peer-derived failure shape | Patchbay assertion |
|---|---|---|
| `effect-before-response-loss` | amux records a message id before terminal injection because keys may land before the HTTP response dies. [amux-outbox]{4} | Retry with the same operation key returns/reconciles the same Operation and cannot repeat the external effect, including across adapter restart. |
| `ack-before-dispatch-crash` | CodeAgent acks queue receipt before invoking the handler. [codeagent-mobile]{4} | Crash after delivery acknowledgement but before adapter start leaves the operation deliverable/reconcilable, never silently drained. |
| `accepted-without-external-run-id` | Mission Control marks this manual-reconciliation-required. [mission-control-src]{3} | Operation remains `unknown`/reconciling with explicit evidence limits; it cannot become completed from acceptance alone. |
| `offline-intent-process-crash` | Happy's normal outbox is memory-only and its offline stub methods are no-ops. [happy-relay]{4} [happy-relay]{5} | Once Patchbay returns accepted, client and server process loss cannot erase intent. Pre-accept local-only calls must not report acceptance. |
| `cursor-gap-repair` | Happy fetches after sequence on reconnect or any sequence gap. [happy-relay]{2} [happy-relay]{4} | Snapshot/cursor recovery is authoritative, monotonic, paginated, and idempotent; stale stream data cannot overwrite recovered state. |
| `version-is-not-generation` | Happy rejects stale blob versions but has no runtime incarnation. [happy-relay]{6} [happy-relay]{10} | A current payload version emitted by an old target generation is still fenced as stale. |
| `stale-worker-same-logical-id` | amux worker identity survives stop/start without a worker-command generation. [amux-outbox]{9} | Old-incarnation command ack/result/event cannot mutate the replacement incarnation. |
| `failed-handoff-after-old-stop` | CodeAgent restores prior state variables after next-driver start fails, without restarting the stopped driver. [codeagent-mobile]{6} | Ownership recovery proves a live controller before publishing steady ownership; otherwise state is failed/unowned, not nominally restored. |
| `baton-publication-loss` | CodeAgent baton state publication is serialized but non-fatal. [codeagent-mobile]{7} | A lost transient handoff event is repaired from authoritative ownership snapshot carrying generation/revision. |
| `bounded-dedup-expiry` | amux send dedup expires after ten minutes; browser and steering intent also age out. [amux-outbox]{3} [amux-outbox]{4} [amux-outbox]{7} | Retry outside the declared outage envelope has a defined same-operation result; any expiry is an explicit terminal disposition, never silent forgetfulness. |
| `superseded-offline-operations` | amux collapses contradictory and last-write-wins offline mutations. [amux-outbox]{3} | Supersession is typed, resource-specific, auditable, and cannot collapse non-commutative operations by URL coincidence. |
| `manifest-overclaim` | Mission Control requires every true capability to cite a shipping path. [mission-control-src]{9} | Adapter cannot advertise end-to-end idempotency, continuation, recovery, or fencing without a conformance vector proving the declared strength. |
| `authority-cross-resource` | Mission Control workspace checks and Happy session ownership constrain access. [mission-control-src]{8} [happy-relay]{8} | A valid credential for target/session A cannot submit, observe, acknowledge, cancel, or complete an operation for B. |
| `dedup-store-unavailable` | amux dedup fails open when SQLite errors. [amux-outbox]{4} | Boundary policy is explicit: fail closed, degrade with duplicate-risk status, or reject; never silently claim idempotency. |

## Contradictions

| Sources | Relationship | Positions |
|---|---|---|
| amux P5 vs amux implementation bounds | tension | P5 names logical sends “exactly-once” across production failure modes, while the implementation's dedup record expires after 600 seconds, fails open on storage error, and offline/steering queues can drop aged intent. [amux-outbox]{5} [amux-outbox]{3} [amux-outbox]{4} [amux-outbox]{7} The contract is credible for its named short response-loss window, not an unbounded operation guarantee. |
| Happy durable-sync protocol vs Happy offline client paths | qualifies | Happy says recoverable state is a sequenced persistent update, and stored messages satisfy that model; the process-local outbox and offline no-op stub sit before server acceptance and do not inherit server durability. [happy-relay]{1} [happy-relay]{3} [happy-relay]{4} [happy-relay]{5} |
| CodeAgent single-driver state vs failed-switch recovery | tension | The controller promises exactly one active driver and restores the prior steady state on error, but it may already have stopped that prior driver before the error and does not restart it. [codeagent-mobile]{5} [codeagent-mobile]{6} |
| Mission Control durable spawn module vs spawn API route | tension | The repository has a durable spawn-history/run module, while the inspected `/api/spawn` route generates a per-request id, returns directly, and still implements history GET by log parsing. [mission-control-src]{4} [mission-control-src]{5} This shows architectural direction without proving end-to-end route integration. |

## Disconfirming analysis

- I searched for evidence that Mission Control already had a caller-stable, durable spawn operation. Contrary evidence included separate spawn-history/run persistence and a gateway idempotency field; inspection still found a fresh server-generated spawn id per HTTP call, no caller retry key, and a log-backed GET route. [mission-control-src]{4} [mission-control-src]{5} [mission-control-src]{6}
- I searched beyond amux's browser outbox for server-side non-expiring accepted intent. The durable steering queue and session-event ledger are stronger than a UI queue, but steering deliberately expires, guarded intent can be dropped as stale, and send dedup is time-bounded/best-effort. [amux-outbox]{4} [amux-outbox]{6} [amux-outbox]{7} [amux-outbox]{8}
- I searched Happy for a durable outgoing command queue rather than assuming WebSocket reconnect was enough. The v3 message path is durable after server receipt, but the CLI pending outbox is memory-only, the offline stub drops calls, and RPC registration is live room membership. [happy-relay]{3} [happy-relay]{4} [happy-relay]{5} [happy-relay]{7}
- I searched CodeAgent Mobile for stronger backend semantics than relay/reconnect. The client source describes non-destructive pending-command delivery, backend acknowledgements, Redis baton snapshots, HMAC binding, and backend SETNX for one file outbox—material disconfirming evidence against calling it “just a relay.” [codeagent-mobile]{3} [codeagent-mobile]{7} [codeagent-mobile]{8} [codeagent-mobile]{11} The backend source is absent, and the public client still reveals ack-before-dispatch and best-effort baton publication, so the combined operation/ownership guarantee remains acquisition-gated. [codeagent-mobile]{1} [codeagent-mobile]{4} [codeagent-mobile]{7}
- I searched all four pinned trees for operation-target incarnation fields rather than treating every `seq`, `version`, UUID, or local generation guard as equivalent. The located versions order state, deduplicate messages, or fence cache/tunnel work; none binds agent commands/results to a monotonic runtime incarnation. [mission-control-src]{10} [amux-outbox]{9} [happy-relay]{10} [codeagent-mobile]{10}
- I searched for scoped authority beyond coarse auth. Mission Control's workspace-bound keys/checks, Happy's ownership filters/access keys, and CodeAgent's pairing secrets are real scope mechanisms. [mission-control-src]{7} [mission-control-src]{8} [happy-relay]{8} [codeagent-mobile]{8} They narrow the gap but do not show an operation-derived grant bound to a target incarnation. {inferred: diverges}

## Acquisition candidates

### Blocking

- **Source:** CodeAgent Mobile backend implementation for `/api/commands/pending[/stream]`, `/api/commands/ack`, `/api/commands/result`, `/api/baton/events`, plugin-auth verification, and Redis snapshots.
- **Class:** `primary-doc`
- **Web availability:** not present in the fetched public client repository; that README explicitly excludes backend/mobile/web source. Public GitHub account enumeration and direct probes of plausible backend repository names did not locate a public backend. [codeagent-mobile]{1}
- **Completes:** whether command acceptance is durably stored; queue retention and terminal disposition; ack-vs-execution semantics; result idempotency; authoritative baton snapshot and ownership conflict rules; stale-client fencing; exact session/plugin authority checks. Until acquired, CodeAgent's backend-level closure remains acquisition-gated rather than inferred from client comments.

No enriching acquisition candidate was surfaced that was both relevant to this facet and named canonically by a fetched source.

## Revisit if

Re-open this facet if any peer adds or documents a caller-visible durable operation resource; if Mission Control integrates `/api/spawn` with durable spawn/run state and caller idempotency; if amux adds authenticated resource grants or worker incarnation tokens; if Happy persists outgoing intent across CLI restart or makes RPC durable; if CodeAgent publishes/acquires its backend implementation; or if any peer introduces a monotonic runtime generation enforced on command delivery, acknowledgement, and result mutation.
