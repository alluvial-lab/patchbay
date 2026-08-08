---
provenance: agent-synthesis
updated: 2026-08-08
---

# Spawn lifecycle

## Bottom line

Patchbay should model `spawn` as creating a **stable logical target identity plus a first runtime generation**, not as “the process.” A restart-as-continuation is a new generation attached to the same logical target, with an explicit continuation reference to the prior generation and a replacement event that tombstones the old generation. The adapter owns how continuation is realized; the core owns identity, authority, durable lifecycle, generation monotonicity, and stale-event fencing. {inferred: converges}

Pi supports this split directly: its persisted logical session is a JSONL tree with a stable session file/id and branches, while the active `AgentSession`/runtime/process can be replaced through `AgentSessionRuntime`; consumers must re-subscribe after replacement. [pi-sessions]{1} [pi-sdk]{1} Herdr's model independently separates persistent session/workspace shape from live processes and native agent-session restoration: detach keeps processes, whereas a server restart restores shape and can invoke native session restore, but does not restore arbitrary processes. [herdr-state]{1} [herdr-state]{2}

## Proposed lifecycle contract

The core contract should carry these distinct fields:

- `logical_target_id`: stable identity for the operator's named spawn target across replacement;
- `runtime_session_id`: adapter-reported identity for the current external session;
- `generation`: monotonically increasing incarnation number for that runtime session;
- `continuation_of`: optional prior `(runtime_session_id, generation)` (or a typed continuation token) supplied in the spawn request/result;
- `spawn_operation_id`: durable provenance for the creation attempt;
- `project_ref` / `cwd_spec`: adapter payload/config, not core identity;
- adapter-declared `idempotency_strength`, because Patchbay boundary dedup does not guarantee external process dedup for a retry.

This is an extension of the sources rather than a source-attested field list (`{extends}`). The need for separate logical and live identities is supported by Pi's session-file/session-runtime split and Herdr's process/session split. [pi-sdk]{1} [herdr-state]{1}

### State transitions and obligations

1. **Spawn**: validate target and authority; durably record `accepted` before delivery. Delivery may provision for an extended period (`running`). On successful adapter report, register logical target + generation 0/1 and issue the descendant grant tied to the spawn operation. Patchbay's existing lifecycle contract requires acceptance before delivery and separates accepted/delivered/running/completed; this facet does not add another command state. {inferred: applies}
2. **Detach**: treat control-surface detachment as loss of an endpoint/subscription, not target death. If the adapter/runtime remains reachable, no generation changes. Herdr explicitly preserves processes across client detach and lets a client reattach later. [herdr-concepts]{1} [herdr-concepts]{5}
3. **Crash**: distinguish adapter/process crash from logical target retirement. Record the current generation as unavailable/failed or stale according to adapter evidence; do not silently allocate a new generation. Pi RPC's accepted prompt can later fail through streamed events, and `agent_settled` is separate from low-level `agent_end`; this supports treating transport/process observations as evidence rather than identity mutation. [pi-rpc]{2} [pi-rpc]{5}
4. **Restart as continuation**: issue a new `spawn` Operation (or a typed spawn continuation payload) referencing the prior logical target and generation. The adapter invokes Pi's native continuation mechanism (`--continue`/`--session` as appropriate) and reports a strictly greater generation. The core tombstones the old generation before making the new one live. Pi's docs establish explicit session selection/continuation and runtime replacement, while the exact `--continue` mapping is adapter implementation detail. [pi-sessions]{1} [pi-sdk]{2}
5. **Reconnect**: attach a new operator endpoint to the logical target, then reconcile by snapshot/cursor. A reconnect must not infer “live” from a remembered UI stream. Pi RPC provides `get_entries` cursors that remain usable across client restarts; Herdr says handoff/replacement can interrupt streams and clients should reconnect and retry. [pi-rpc]{4} [herdr-state]{4}
6. **Duplicate/stale**: same idempotency key + equivalent spawn returns the existing durable command; a new intentional continuation has a new operation/idempotency key. Equal-generation registration is a no-op; lower-generation reports and events for tombstoned generations become `stale_event` audit records and cannot mutate the live generation. The latter is an application of Patchbay's existing generation fence (`{inferred: applies}`), not a claim about Pi or Herdr.

### What “restart” must not promise

A continuation restores adapter-native logical context, not arbitrary process state. Herdr states that restart restores saved shape but not shells, servers, tests, or arbitrary processes, and that native agent restore is conditional on an official session reference. [herdr-state]{2} [herdr-state]{3} Therefore the Patchbay result should expose whether continuation was `resumed`, `new_context`, or `unknown`, rather than presenting every new process as a seamless continuation (`{extends}`).

## Project/cwd targeting seam

**Decision: core-neutral, adapter-owned project/cwd targeting in v1.** Core `spawn` should carry an opaque, typed `target_spec` and adapter capability binding; it should not create a universal `Project` entity or assign shared-cwd semantics. A core may retain display metadata or an opaque operator label, but resolution/authority must remain against the adapter target and its deployment scope.

The trade-off is visible in the peers:

- Herdr makes workspace a first-class, named project/task/investigation container that owns tabs and panes; its workspace is coupled to terminal layout and process topology. [herdr-concepts]{1} [herdr-concepts]{2}
- Coder makes workspace a first-class compute environment created from a template; templates define resources/environment, and workspace lifecycle includes persistent/ephemeral resource behavior. [coder-workspaces]{1} [coder-workspaces]{3}
- Pi itself is cwd-bound in resource discovery and session storage, but its logical session API remains a session/file/runtime abstraction rather than a core “Project” entity. [pi-sdk]{2} [pi-sdk]{3}

A universal Project would therefore import assumptions that are adapter-specific: repo/worktree identity, shared filesystem, persistent workspace resources, or terminal layout. It would also make a raw cwd look like an authority/routing identity, contrary to Patchbay's adapter-neutral deployment posture. An adapter-owned project reference can still be stable and named where the adapter supports it; a Pi adapter may define `project_ref` as a named cwd binding and resolve it to an absolute path locally. This preserves ergonomics without making Project part of core ontology (`{inferred: tension}` across peers).

**Reserved seam:** if later adapters need cross-adapter project correlation, promote a core `ProjectRef` only after defining its authority domain, portability, lifecycle, and non-shared-cwd semantics. Until then, `project_ref`, `cwd`, template, repo, worktree, and task identifiers are adapter-declared target-spec shapes.

## Conformance vector candidates

These should be executable vectors, each asserting both durable events and adapter-visible outcomes:

| Vector | Stimulus | Required assertions |
|---|---|---|
| `spawn-continuation` | Submit spawn; adapter reports generation 1; crash; submit continuation referencing `(logical, gen 1)`; adapter reports gen 2; reconnect | One logical target; two runtime generations; old generation tombstoned; gen 2 live; continuation provenance links operation to gen 1; reconnect returns authoritative gen 2 and cursor/snapshot state. |
| `detach-does-not-retire` | Spawn gen 1; attach endpoint A; detach A; runtime remains reachable; attach B | No generation increment or retirement; B reconciles the same target/gen; endpoint loss is not target loss. |
| `crash-before-ack` | Spawn accepted/delivered; adapter crashes before external create acknowledgement; retry same idempotency key | Core returns existing command or `unknown`/reconciliation state; it does not claim successful creation; adapter capability strength determines whether external duplicate risk is surfaced. |
| `restart-native-resume` | Spawn Pi session; terminate process; continuation with native session reference | Adapter reports whether Pi resumed the same logical transcript versus started a new context; core creates a new generation either way; UI does not equate process replacement with transcript continuity. |
| `restart-shape-only` | Runtime has cwd/project metadata and active process; runtime dies; adapter restores only target shape | New generation can be registered with `continuation_status = new_context`; old process-bound claims and in-flight commands do not reappear as live. |
| `reconnect-after-stream-loss` | Endpoint loses stream while generation changes; reconnect with cursor/snapshot | Reconciliation repairs missed terminal/generation events; stale stream cannot overwrite snapshot; duplicate replay is idempotent. |
| `duplicate-continuation` | Submit same continuation operation twice with same idempotency key/payload | One durable command and one external continuation attempt at the Patchbay boundary; second returns existing state. |
| `stale-generation-event` | Gen 1 emits after gen 2 is live | Event recorded as `stale_event`; gen 2 state and command records unchanged. |
| `equal/lower-generation-report` | Adapter reports gen 2 twice, then gen 1 | Equal report is no-op/redeclaration; lower report rejected/audited; live gen remains 2. |
| `duplicate-native-reference` | Two restored targets claim one native Pi session reference | Adapter rejects or marks one restore ambiguous; core never silently merges logical targets. Herdr documents duplicate native references as falling back to normal shells, providing a concrete failure expectation. [herdr-state]{3} |
| `project-cwd-boundary` | Same logical label with different adapter/project refs; cwd changes; reconnect | Core does not route by label/cwd; adapter resolves opaque target spec; metadata update does not create a generation unless the adapter reports replacement. |

## Contradictions

| Sources | Relationship | Positions |
|---|---|---|
| Herdr session-state vs Pi session docs | tension | Herdr's full server restart reconstructs pane shape and may create new shells, while Pi can continue a persisted logical conversation by session selection. [herdr-state]{2} [pi-sessions]{1} These are different persistence layers, not a single “restart” semantic. |
| Herdr workspace vs Patchbay core posture | incommensurable | Herdr treats a workspace as a terminal/process container; Patchbay's core is adapter-neutral and does not assume shared cwd/process topology. Herdr's entity cannot be imported as a universal core entity without changing the frame. [herdr-concepts]{1} [herdr-concepts]{2} |
| Pi runtime replacement vs Pi logical session | qualifies | Pi's runtime replacement changes active services/session object and requires subscription rebind, while its persisted session file/tree can remain the logical continuation source. [pi-sdk]{1} [pi-sessions]{1} |

## Disconfirming analysis

- I looked for evidence that detach should increment a generation or retire a target. Herdr's direct account says the opposite: client detach leaves panes/processes running, so endpoint detach must be modeled separately. [herdr-concepts]{5} [herdr-state]{1}
- I looked for evidence that a server/process restart always preserves the same process identity. Herdr explicitly says arbitrary processes are not preserved after server restart; native restore is conditional. [herdr-state]{2} [herdr-state]{3}
- I looked for evidence that Pi's session file alone is the live incarnation. The SDK separates `AgentSessionRuntime` replacement from `AgentSession`, and requires re-subscription after replacement. [pi-sdk]{1}
- I looked for evidence that a named workspace/project is universally portable. Herdr couples workspace to panes/tabs/processes, while Coder couples workspace to template-defined compute resources; these divergent meanings disconfirm a universal core Project. [herdr-concepts]{1} [coder-workspaces]{1} [coder-workspaces]{3}
- Web acquisition for Devin, Daytona, and amux did not yield a fetched, source-direct document in the available transport. No claims about those systems are included here; they remain an acquisition gap rather than training-recall evidence.

## Revisit if

Re-open this facet if the Pi adapter exposes a documented stable logical session id distinct from its session file path; if an adapter requires cross-adapter project routing; if v1 adds shared filesystem/worktree semantics; if spawn idempotency becomes end-to-end for a reference adapter; or if native continuation can report a stronger proof of transcript/state continuity than the current adapter result categories.
