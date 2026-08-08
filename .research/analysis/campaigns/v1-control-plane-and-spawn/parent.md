---
provenance: agent-synthesis
updated: 2026-08-08
scope: four-peer control-plane comparison and Pi spawn/restart capability
revision_mode: correction
---

# Control-plane composition and spawn-lifecycle direction

## Reader context

This analysis uses five consumer-side concepts:

- An **Operation** is an authorized control-plane request whose accepted state is
  durably tracked through a terminal disposition.
- An **authority domain** is the bounded control context in which grants,
  revocation, routing authority, and reconciliation are evaluated against one
  authoritative state.
- A **logical target** is the stable identity an operator intends to control
  across runtime replacement.
- A **generation** is a monotonic incarnation number for one logical target; it
  lets a control plane reject late reports from a replaced runtime.
- An **adapter** is the boundary component that translates the control-plane
  contract to an external runtime such as Pi.

These are the commissioning consumer's analytical and design terms, not
terminology attributed to the compared systems. `{extends}` The scoped peer
corpus is Mission Control, amux, Happy, and the publicly accessible CodeAgent
Mobile client. CodeAgent Mobile's backend was not accessible, so conclusions
about its backend durability, terminal-state handling, ownership conflicts, and
fencing remain acquisition-gated. [codeagent-mobile]{1}

## Verdict

**Corpus-bounded composition verdict:** no fetched/accessible evidence — across
the four scoped peers, with CodeAgent Mobile's backend acquisition-gated —
demonstrates the five-axis composition of durable acceptance through terminal
disposition, caller-stable idempotency, target-incarnation fencing,
authoritative post-restart reconciliation, and resource-scoped authority; the
verdict is provisional pending that backend. `{inferred: aggregates}` Mission
Control supplies durable task/run governance,
approvals, and workspace controls but its inspected spawn route mints a new id
per request; amux supplies persistent outbox, deduplication, steering, and local
recovery with explicit expiry/drop bounds; Happy supplies durable message sync,
sequence cursors, and optimistic version checks while live RPC and pre-accept
intent are weaker; CodeAgent Mobile supplies command ids, relay acknowledgements,
and a baton state machine while the unavailable backend controls several
load-bearing guarantees. [mission-control-src]{1}{2}{3}{4}{6}{7}
[amux-outbox]{1}{2}{3}{4}{6}{7} [happy-relay]{2}{3}{4}{6}{7}
[codeagent-mobile]{1}{3}{4}{5}{7}

The defensible consumer claim is therefore the composition, not broad
“durability” and not any individual primitive: accepted control intent remains
accountable until terminal disposition; retries reconcile by stable Operation
identity; replacement runtimes are generation-fenced; reconnect uses
source-appropriate authoritative state; and authority is scoped to the addressed
resource. `{inferred: composes}` [mission-control-src]{2}{3}{7}
[amux-outbox]{4}{7}{10} [happy-relay]{2}{3}{7}{8}
[codeagent-mobile]{1}{4}{7}{8} The sentence is a proposed consumer contract,
not a claim that any cited source uses this ontology.

The spawn lifecycle is a **proposed lifecycle direction**, not a decided
contract. `{extends}` Its working shape is a stable logical target plus a
monotonic runtime generation, with the adapter responsible for native
continuation mechanics and the consumer core responsible for Operation state,
authority, durable target identity, generation monotonicity, and stale-event
fencing. Three forks remain open: whether the first generation is `0` or `1`;
whether restart is a new `spawn` Operation or a typed continuation payload on a
spawn Operation; and whether crash evidence maps the current generation to
`unavailable`, `failed`, or `stale`.

Pi exposes an **analogous** persisted-session/runtime-replacement separation:
sessions are JSONL trees, while `AgentSessionRuntime` replaces the active
session/runtime and requires consumers to re-subscribe. [pi-sessions]{3}
[pi-sdk]{1} Mapping that separation to a consumer-owned logical-target ontology
and monotonic generation fence is an extension, not a Pi-native contract.
`{extends}`

## Five-axis evaluation

The axes used below are analytical criteria. `{extends}`

1. **Durable accepted intent** — after acceptance, intent survives relevant
   process loss and remains represented until a visible disposition.
2. **Caller-stable idempotency** — retry identity resolves to the same logical
   operation beyond a short transport-failure window.
3. **Incarnation fencing** — an old runtime cannot act as the current target
   merely because it retains an old session/resource id.
4. **Authoritative recovery** — reconnect/restart reconstructs the relevant
   durable state rather than trusting remembered stream or presence state.
5. **Scoped authority** — credentials or grants bind action rights to the
   addressed resource and control domain.

### Per-peer matrix

| Peer | Durable accepted intent | Caller-stable idempotency | Incarnation fencing | Authoritative recovery | Scoped authority |
|---|---|---|---|---|---|
| **Mission Control** | **Partial.** Tasks, runs, spawn history, and jobs persist, but accepted dispatch without a run id can require manual reconciliation and the inspected spawn route returns directly. [mission-control-src]{2}{3}{4}{5} | **Narrow.** Registration upserts and some jobs/calls use keys, but the spawn route generates a fresh id and the platform PRD names idempotent commands incomplete. [mission-control-src]{4}{6} | **Not located for runtime targets** in the fetched tree. [mission-control-src]{10} | **Partial.** Durable task/run reconciliation exists; the inspected spawn route and run-id-less dispatch are weaker. [mission-control-src]{2}{3}{4}{5} | **Substantial but coarse.** Agent/workspace-bound keys and fail-closed workspace checks do not establish Operation-descendant grants. [mission-control-src]{7}{8} |
| **amux** | **Partial and bounded.** Browser and steering intent can survive reconnect/restart, but limits, expiry, supersession, and explicit drops are part of the design. [amux-outbox]{1}{3}{6}{7} | **Strong within a bounded response-loss window.** Persistent `(session,msg_id)` dedup records before delivery, expires after 600 seconds, and fails open on storage error. [amux-outbox]{4}{5} | **Not located for worker commands.** Stable worker identity survives stop/start; located generation guards protect other subsystems. [amux-outbox]{9} | **Substantial for local state.** SQLite queues/events and pane/log/JSONL evidence support replay and reconciliation, but not one general Operation state machine. [amux-outbox]{2}{5}{6}{8} | **Absent as a built-in network boundary.** The project states it has no built-in authentication. [amux-outbox]{10} |
| **Happy** | **Partial by layer.** Server-accepted messages are durable; the normal pre-accept outbox is memory-only, the offline stub discards calls, and RPC is live routing. [happy-relay]{3}{4}{5}{7} | **Strong for messages.** Required local ids, transactional lookup, and database uniqueness make message retries idempotent. [happy-relay]{3} | **Resource-version checks only.** `expectedVersion` rejects stale blob writes, but no replacement-runtime generation is present. [happy-relay]{6}{10} | **Strong for messages/session data.** Database sequences, cursor fetch, and persisted resume metadata repair gaps. [happy-relay]{1}{2}{9} | **Partial.** Account ownership, rooms, and per-session/per-machine access keys constrain access; primary bearer identity remains account-wide. [happy-relay]{7}{8} |
| **CodeAgent Mobile** | **Acquisition-gated partial.** Client comments describe a backend queue, but the client acknowledges before handler execution and the backend is unavailable. [codeagent-mobile]{1}{3}{4} | **Partial.** Command ids and bounded in-memory processed-id dedup suppress ordinary redelivery; process restart forgets that set. [codeagent-mobile]{3}{4} | **Not located in the fetched client protocol.** Baton and command envelopes have no monotonic runtime generation. [codeagent-mobile]{10} | **Relay/reconnect evidence only.** Watchdog/fallback and baton publication exist, but publication is best-effort and failed switch can restore state variables without restarting the stopped driver. [codeagent-mobile]{6}{7} | **Meaningful pairing scope, backend-gated.** Pairing tokens and poll secrets are scoped, while the implementation of server verification is outside the fetched repository. [codeagent-mobile]{1}{8}{9} |

Each peer shows partial evidence on several axes — Mission Control, Happy, and
CodeAgent on roughly four at partial strength, amux on three; none satisfies all
five. “Evidence on” does not mean the peer satisfies the consumer's full axis
definition. `{inferred: compares}` [mission-control-src]{2}{7}
[amux-outbox]{4}{8} [happy-relay]{2}{3}{8}
[codeagent-mobile]{3}{7}{8} The aggregate conclusion remains: no
fetched/accessible evidence — across the four scoped peers, with CodeAgent
Mobile's backend acquisition-gated — demonstrates the five-axis composition;
the conclusion is provisional pending that backend. `{inferred: aggregates}`

## Architectural direction from Mission Control

Mission Control is the selected architectural namesake for adapter-neutral
control-plane structure (an editorial selection for comparison, not a
source-claimed status). Its evidence supports direction rather than code reuse
or semantic equivalence:

1. Separate host/runtime detection from a complete declared capability manifest,
   with uncertain capabilities false. [mission-control-src]{9} Add
   durability-specific fields such as dedup strength, continuation proof,
   cursor support, and generation-fence support. `{extends}`
2. Keep durable run/task provenance beside adapter-native ids, but make the
   accepted Operation the consumer's source of truth rather than an optional
   diagnostic record. [mission-control-src]{5} `{extends}`
3. Preserve atomic task claim before delivery, then add caller idempotency and
   target generation checks. [mission-control-src]{2} `{extends}`
4. Represent reconciliation limits honestly: when the substrate cannot prove an
   outcome, expose `unknown` or manual reconciliation rather than deriving
   completion from acceptance. [mission-control-src]{3} `{extends}`
5. Preserve fail-closed workspace boundaries without equating coarse role
   derivation with fine-grained Operation authority. [mission-control-src]{7}{8}
   `{inferred: qualifies}`

## Proposed spawn lifecycle direction

### Candidate fields

The following field set is authored consumer design. `{extends}`

- `logical_target_id`: stable across runtime replacement;
- `runtime_session_id`: adapter-reported current external session identity;
- `generation`: monotonic incarnation for that target;
- `continuation_of`: optional prior runtime-session/generation or typed native
  continuation token;
- `spawn_operation_id`: durable provenance for the creation attempt;
- `target_spec`: opaque adapter payload, including optional project/cwd shape;
- `idempotency_strength`: adapter-declared external retry guarantee.

Pi's persisted session/runtime replacement and Herdr's separation of live
processes from restored session shape motivate separate logical and live
identities, but neither source specifies the field set above. [pi-sdk]{1}
[herdr-state]{1}{2} `{inferred: composes}`

### Candidate transitions and obligations

Every transition below is authored design rather than a source-attested
contract. `{extends}`

1. **Spawn.** Validate target and authority; durably record acceptance before
   delivery; on success register the logical target and first generation, then
   tie descendant authority to the spawn Operation. The first-generation value
   remains an open `0`-versus-`1` fork. `{extends}`
2. **Detach.** Treat control-surface detachment as endpoint/subscription loss,
   not target death; do not increment generation merely because the endpoint
   detached. Herdr preserves processes across client detach. [herdr-concepts]{2}{5}
   `{extends}`
3. **Crash.** Record evidence about the current generation without silently
   allocating a replacement. The state vocabulary remains an open
   `unavailable`-versus-`failed`-versus-`stale` fork. Pi separates prompt
   acceptance from later streamed failure and distinguishes low-level
   `agent_end` from settled completion. [pi-rpc]{2}{5} `{extends}`
4. **Restart as continuation.** Create a strictly greater generation referencing
   the prior one; tombstone the old generation before the replacement becomes
   live; report `resumed`, `new_context`, or `unknown` rather than promising
   arbitrary process-state restoration. Whether this is a new `spawn` Operation
   or a typed continuation payload remains open. Pi exposes continuation flags,
   while Herdr says server restart restores shape and conditionally native
   conversation state, not arbitrary processes. [pi-sessions]{2}
   [herdr-state]{2}{3} `{extends}`
5. **Reconnect.** Attach the endpoint, then reconcile from the authoritative
   state available for each layer. Do not infer runtime liveness from a
   remembered stream. [pi-rpc]{4}{7}{8} [herdr-state]{4} `{extends}`
6. **Duplicate/stale.** An equivalent retry at the consumer boundary resolves to
   the existing Operation. Equal-generation reports are no-ops; lower-generation
   reports and tombstoned-generation events become audit evidence and cannot
   mutate the live target. `{extends}`

The evidence lines up around one bounded direction: persisted logical context
and live process/runtime incarnation must not be treated as the same thing.
`{inferred: converges}` Pi supplies persisted-session/runtime separation, Herdr
supplies process-versus-restored-shape separation, and the generation fence is
the commissioning consumer's extension. [pi-sessions]{3} [pi-sdk]{1}
[herdr-state]{1}{2}{4}

### Project/cwd targeting seam

For the commissioning consumer, keep project/cwd targeting core-neutral and
adapter-owned in the proposed v1 direction: `spawn` carries opaque typed
`target_spec`; project, cwd, template, repo, worktree, and layout remain
adapter-declared shapes rather than universal core identity. `{extends}` Herdr's
workspace owns tabs and panes; Coder's workspace is template-defined compute;
Pi rebuilds runtime services for the effective cwd and opens or continues
sessions through cwd-aware session management. [herdr-concepts]{1}{2}
[coder-workspaces]{1}{3} [pi-sdk]{2}{3} These frames do not establish one portable
Project entity. `{inferred: diverges}` A future core `ProjectRef` remains a
reserved seam pending explicit authority-domain, portability, lifecycle, and
non-shared-cwd semantics. `{extends}`

## Pi adapter boundary

### Persisted session reconciliation, not universal authority

Pi RPC uses strict LF-delimited JSONL and separates correlated command responses
from asynchronous events. [pi-rpc]{1} Parallel tools qualify event ordering:
starts follow assistant source order, updates can interleave, ends follow
completion order, and final tool-result messages return to assistant source
order; `message_end.message` is authoritative for the finalized message.
[pi-rpc]{6}

`get_entries(since)` is authoritative only for **persisted Pi session-entry
reconciliation and the current leaf id**. It returns the append-order suffix
strictly after a stable entry id, includes abandoned branches and pre-compaction
history, and returns `leafId`; an unknown cursor fails explicitly. [pi-rpc]{4}{7}{8}
It is not authority for process liveness, external side effects, or the
consumer's Operation state. The cursor orders appended session entries; the live
RPC event stream is not a universal total order. `{inferred: bounds}`

### Reload, continuation, and supervised replacement

`/reload` replaces the extension runtime and rediscovered resources in-process;
future calls use the new extension version, while the invoking handler remains
an old call frame. [pi-extensions]{1}{5} The loader aliases Pi runtime packages
to the installed running package's `dist`, and the inspected loader contains no
running-executable/package-graph replacement mechanism. [pi-loader]{2}{3}
Accordingly, process termination and respawn is the reliable Pi/runtime package
upgrade boundary; this conclusion does not deny extension-entrypoint hot reload.

Pi sessions auto-save as JSONL trees and can be continued with `-c` or selected
with `--session <path|id>`. [pi-sessions]{2}{3} Session replacement tears down
old session-bound objects and requires extensions to rebuild in-memory state;
custom entries can persist the state needed for rehydration. [pi-extensions]{7}{8}
Continuation therefore preserves attested session data, not arbitrary in-memory
or external process state.

The documented RPC command inventory contains session replacement and inspection
but no process restart primitive. [pi-rpc]{9} An adapter may therefore implement
quiesce/abort policy, terminate the Pi process, respawn against an explicit
session, and reconcile persisted entries/cursor before reporting its own target
state. `{extends}` That terminate/respawn/reconcile workflow is adapter-owned
consumer design, not a Pi RPC guarantee.

## Conformance-vector selection

The following selection composes peer failure shapes with the proposed lifecycle;
it is authored verification direction, not source terminology. `{inferred: selects}`
The load-bearing selection uses evidence from all four peers. [mission-control-src]{3}{4}
[amux-outbox]{3}{4}{7} [happy-relay]{2}{4}{5}{6}
[codeagent-mobile]{4}{6}{7} Its premise is corpus-bounded: no fetched/accessible
evidence — across the four scoped peers, with CodeAgent Mobile's backend
acquisition-gated — demonstrates the five-axis composition; the selection is
provisional pending that backend. `{inferred: bounds}`

- **Acceptance accountability:** `effect-before-response-loss`,
  `ack-before-dispatch-crash`, `accepted-without-external-run-id`, and
  `offline-intent-process-crash`. These target the corpus-bounded composition
  gap, with CodeAgent backend behavior still acquisition-gated. `{extends}`
- **Recovery and incarnation:** `cursor-gap-repair`,
  `version-is-not-generation`, and `stale-worker-same-logical-id`. These test a
  proposed generation fence not demonstrated by fetched/accessible evidence
  across the four scoped peers; CodeAgent backend evidence remains pending.
  `{extends}`
- **Spawn lifecycle:** `spawn-continuation`, `detach-does-not-retire`,
  `crash-before-ack`, `restart-native-resume`, `restart-shape-only`,
  `reconnect-after-stream-loss`, `duplicate-continuation`,
  `stale-generation-event`, `equal/lower-generation-report`,
  `duplicate-native-reference`, and `project-cwd-boundary`. `{extends}`
- **Boundary honesty:** `bounded-dedup-expiry`,
  `superseded-offline-operations`, `manifest-overclaim`,
  `authority-cross-resource`, and `dedup-store-unavailable`. `{extends}`

The vector set is provisional until the open lifecycle forks are resolved; its
labels do not claim that an inaccessible backend lacks a capability.

## Contradictions

These propositions and resolutions stand independently of any other analysis.

| Sources and named propositions | Relationship | Resolution status |
|---|---|---|
| **amux P5** calls logical sends “exactly-once” and explicitly includes server restart in the response-loss window. [amux-outbox]{5} **amux implementation** expires send-dedup rows after 600 seconds, fails open on storage error, and separately expires/drops older queued intent. [amux-outbox]{3}{4}{7} | **tension** | **Bounded.** Preserve the source's qualifier: the exactly-once claim is credible for the named short response-loss window, not an unbounded Operation guarantee. |
| **Mission Control durable spawn module** records spawn history and associated runs in SQLite. [mission-control-src]{5} **Mission Control `/api/spawn` route** generates a fresh per-request id, returns directly, and reads history from logs. [mission-control-src]{4} | **tension** | **Bounded.** The durable module is architectural evidence; the inspected route does not establish end-to-end integration or caller-stable spawn idempotency. |
| **CodeAgent baton model** specifies one active driver through steady and switching states. [codeagent-mobile]{5} **CodeAgent failed-switch path** may stop the prior driver, fail to start the next, then restore prior state variables without restarting the stopped driver. [codeagent-mobile]{6} | **tension** | **Open.** Client behavior leaves nominal ownership and actual liveness misaligned; backend conflict and snapshot rules are acquisition-gated. |
| **Pi extension documentation** supports in-process extension/resource reload and says future calls use the new extension version. [pi-extensions]{1}{5} **Pi loader source** aliases runtime packages to the running installed package and does not replace the process package graph. [pi-loader]{2}{3} | **qualifies** | **Bounded by layer.** Extension-entrypoint reload and runtime-package/process replacement are different layers. |
| **Happy server-accepted message path** stores messages transactionally with stable local-id dedup. [happy-relay]{3} **Happy pre-accept client path** keeps the normal outbox in memory and has an offline stub whose methods are no-ops. [happy-relay]{4}{5} | **qualifies** | **Bounded by acceptance boundary.** Server durability starts after server acceptance; pre-accept memory state does not inherit it. |
| **Herdr workspace** is a terminal/process container owning tabs and panes. [herdr-concepts]{1}{2} **The commissioning consumer's core posture** treats project/cwd as adapter metadata rather than universal identity. `{extends}` | **incommensurable** | **Incommensurable — cross-frame, not within-source.** Importing Herdr's entity as a universal core Project would change the comparison frame; neither proposition refutes the other. |

## Disconfirming analysis

- Mission Control was searched for a caller-stable durable spawn Operation. The
  durable spawn-history/run module and gateway idempotency field are positive
  evidence, but the inspected route still generates a fresh id and the PRD names
  idempotent commands incomplete. [mission-control-src]{4}{5}{6}
- amux was searched beyond the browser outbox for longer-lived accepted intent.
  Its steering queue and event ledger are stronger than a UI queue, but steering
  expires, guarded intent can drop, and send dedup remains time-bounded and
  best-effort on storage failure. [amux-outbox]{4}{6}{7}{8}
- Happy was searched for durable outgoing commands rather than assuming socket
  reconnect was sufficient. Server-accepted messages are durable, but the
  pre-accept outbox is memory-only, the offline stub discards calls, and RPC
  ownership is live room membership. [happy-relay]{3}{4}{5}{7}
- CodeAgent Mobile was searched for stronger semantics than a relay. Pairing
  secrets, non-destructive delivery comments, baton serialization, and a
  separate persistent telemetry outbox are material positive evidence.
  [codeagent-mobile]{3}{7}{8}{11} Ack-before-dispatch and best-effort baton
  publication remain visible in the client, while backend semantics remain
  inaccessible. [codeagent-mobile]{1}{4}{7}
- All four fetched peer trees were checked for target-incarnation fields rather
  than treating every sequence, version, UUID, or subsystem generation as a
  runtime fence. The located evidence did not demonstrate the five-axis
  composition, but the CodeAgent backend limitation prevents an unconditional
  absence claim. [mission-control-src]{10} [amux-outbox]{9}
  [happy-relay]{10} [codeagent-mobile]{1}{10} `{inferred: bounds}`
- Pi's event-order evidence was checked against its persisted-entry cursor. The
  parallel-tool exception prevents treating live events as a universal total
  order; `get_entries` supplies append-order persisted session reconciliation
  only. [pi-rpc]{4}{6}{7}

## Acquisition gap

The blocking acquisition is the CodeAgent Mobile backend implementation named
by the public client architecture but absent from the fetched repository.
[codeagent-mobile]{1} Acquisition must establish, without relying on client
comments alone, backend command persistence and retention, acknowledgement and
result semantics, idempotency, authoritative baton ownership/snapshot rules,
stale-client fencing, and server-side pairing scope. Until then, no
fetched/accessible evidence — across the four scoped peers, with CodeAgent
Mobile's backend acquisition-gated — demonstrates the five-axis composition;
that verdict remains provisional pending the backend. `{inferred: bounds}`

## Commissioning-consumer implications

- Use the corpus-bounded five-axis composition as a provisional product
  differentiator, not a universal market claim and not a claim that individual
  durability/reconnect/dedup primitives are unique. `{inferred: applies}`
- Carry the proposed spawn lifecycle forward only with the three open forks
  explicit: first generation `0`/`1`, restart Operation shape, and crash-state
  vocabulary. `{extends}`
- Have a Pi adapter declare transport framing, event-order caveat, persisted
  entry cursor strength, session persistence/replacement, reload boundary,
  resource scope, and state-rehydration behavior. `{extends}`
- Keep Pi process supervision and terminate/respawn/reconcile orchestration in
  the adapter boundary. `{extends}`

## Revisit triggers

Revisit the conclusions if the CodeAgent backend becomes accessible; any scoped
peer publishes a caller-visible durable Operation resource or enforced runtime
generation; Mission Control integrates its spawn route with durable run state
and caller idempotency; amux adds authenticated resource grants or worker
incarnation tokens; Happy persists outgoing intent across client restart or
makes RPC durable; Pi adds a process restart RPC, a stronger total event-order
guarantee, a runtime-package hot-swap contract, or durable runtime-generation
state; or a consumer requires cross-adapter project routing.

## Revisions

- **2026-08-08 — correction:** bounded the composition verdict to the fetched
  corpus and CodeAgent acquisition gap; made consumer extensions explicit;
  repaired Pi source locators and authority scope; embedded the peer matrix,
  contradiction ledger, and open lifecycle forks.
