---
id: research-handoff-pi-adapter-capability-lifecycle-conformance
kind: story
stage: implementing
tags: [adapter, verification]
parent: research-handoff-pi-adapter-capability
depends_on: [research-handoff-pi-adapter-capability-manifest-profile, research-handoff-pi-adapter-capability-control-session-integrity, research-handoff-pi-adapter-capability-rpc-process-supervisor, research-handoff-pi-adapter-capability-cursor-replay-resync, research-handoff-pi-adapter-capability-resource-reload-rehydration, research-handoff-spawn-runtime-evidence-promotion-contract, research-handoff-spawn-stale-event-fencing, research-handoff-spawn-completion-promotion-driver, research-handoff-spawn-reconnect-cursor-reconcile]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-12
---

# Integrated Pi lifecycle, profile, and replacement conformance

## Redesign disposition

Rewritten to gate Pi declaration activation on the resolved cwd/materialization/tree/cursor/reload contracts and the core's staged-successor/quarantine/promotion path.

## Checkpoint

Bind the opaque generated Pi profile, challenged control handshake, conditional materialization, strict session tree, claim-aware supervisor/journal, Pi-session-scoped authoritative cursor replacement, idle-only reload, core stale fence, and atomic promotion into implementation-backed v1 evidence. Use real Pi RPC children for process/file/extension boundaries and deterministic injected fixtures only for isolated failure control.

Do not call this evidence model-checked, checked-normative, or release-verified unless separate promotion gates actually clear.

## Design

**Files**
- `pi-adapter/tests/{rpc_client,control_handshake,session_file,spawn_supervisor,cursor_reconcile,reload}.test.ts` — focused contract/regression/mutation tests.
- `pi-adapter/tests/e2e.test.ts` — real core + real Pi RPC child lifecycle using offline materialized fixtures/control commands.
- `operator-domain` / cockpit tests — exact replacement fold removes omitted Pi persisted entries and remains idempotent on same epoch.
- `contracts/vectors/` and registered runners — Pi refinements for `spawn-continuation`, `restart-native-resume`, `restart-shape-only`, `cursor-gap-repair`, `duplicate-native-reference`, `manifest-overclaim`, `reload-active-rejected`, `session-tree-corrupt`, and stale-generation/quarantine behavior.
- `docs/VERIFICATION.md` traceability updates generated only by the established scripts.

The real-process harness creates two kinds of sessions without ambient credentials:

1. fresh unmaterialized Pi sessions used for handshake/materialization-unavailable behavior;
2. prebuilt current-version materialized JSONL fixtures containing valid assistant history, used for resume/reload/tree/cursor tests.

Control extension commands execute without an LLM turn. SDK-isolated tests use `AgentSessionRuntimeFixture` with a fully injected offline `ModelRuntime`, resource loader, session manager, and catalog/auth stubs. A test that succeeds only because a developer has credentials/models is invalid.

The exact Pi manifest is activated only at this checkpoint. Activation proves the configured build contains the supervisor, control extension, strict validator, effect journal, authoritative replacement consumer, reload gate, and matching conformance version. Generic assurance dimensions come from `capability-manifest-durability-and-reconciliation-depth`; missing/uncertain evidence stays false/unknown.

## Acceptance evidence

- [ ] Generic RPC path/id cannot clear cwd proof; exact challenged current-extension marker can, and wrong cwd/nonce/epoch/source fails.
- [ ] Fresh path-without-file remains memory-only: it may become a promoted current new context, but resume/reload/restart-stable cursor stay unavailable.
- [ ] Malformed interior line, duplicate id, orphan/broken parent, multiple root, unsupported entry, truncated frame, symlink/root escape, inode swap, and raw-vs-RPC mismatch all block `resumed`.
- [ ] Exact continuation stages, remains non-current, then promotes only through `SpawnPromotionCommitted`; old-generation callbacks/output become quarantine and never transcript/current state.
- [ ] Launch-effect ambiguity poisons the exact claim and does not launch another child; known identity reconciles only to the original claim/logical target.
- [ ] Unknown cursor full replacement deletes an omitted stale projected entry in the real consuming fold; same-epoch retry is inert; cursor never leads projection.
- [ ] N+1 resumed against the same verified Pi session loads N's cursor; a second logical target/native identity collision rejects.
- [ ] Busy/unmaterialized reload rejects before effect; valid reload needs materialized request+completion, new handshake, rebind, and reconcile. Arbitrary dependency/runtime changes require process replacement.
- [ ] The core manifest contains no Pi resource vocabulary; the opaque Pi profile and generic assurance fields are emitted/validated/replayed/rendered without becoming authority.
- [ ] `AgentSessionRuntimeFixture` is the only SDK lifecycle test seam and all model/catalog/auth services are injected offline.
- [ ] Every test awaits process-group exit, Observation/journal/cursor durability, expected promotion/poison, and late async completion; no orphan/error can pass after assertions.
- [ ] Assurance prose says implementation-checked only unless independent formal/vector promotion exists.

## Required mutation witnesses

- trust `get_state` as cwd proof;
- treat non-empty `sessionFile` or in-memory custom marker as durable;
- skip malformed raw line / overwrite duplicate id / allow orphan root;
- publish claimed-successor transcript before promotion;
- key cursor by Patchbay generation;
- unknown-cursor upsert instead of exact replacement;
- commit cursor before core replacement acknowledgement;
- invoke reload while streaming/compacting/queued or complete on marker alone;
- treat arbitrary dependency `/dist` as reloadable;
- emit full Pi manifest without sibling assurance or mechanism evidence;
- instantiate ambient `ModelRuntime`/credential/catalog behavior in lifecycle tests.

## Ordering constraint

Final Pi checkpoint after every Pi mechanism plus the spawn runtime-evidence contract, shared stale fence, atomic completion/promotion driver, and reconnect convergence. Green evidence enables manifest activation and feature review; it does not bypass the spawn feature's own review/assurance gates.
