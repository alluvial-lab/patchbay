---
id: research-handoff-pi-adapter-capability-lifecycle-conformance
kind: story
stage: done
tags: [adapter, verification]
parent: research-handoff-pi-adapter-capability
depends_on: [research-handoff-pi-adapter-capability-manifest-profile, research-handoff-pi-adapter-capability-control-session-integrity, research-handoff-pi-adapter-capability-rpc-process-supervisor, research-handoff-pi-adapter-capability-cursor-replay-resync, research-handoff-pi-adapter-capability-resource-reload-rehydration, research-handoff-spawn-runtime-evidence-promotion-contract, research-handoff-spawn-stale-event-fencing, research-handoff-spawn-completion-promotion-driver, research-handoff-spawn-reconnect-cursor-reconcile]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-16
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

- [x] Generic RPC path/id cannot clear cwd proof; exact challenged current-extension marker can, and wrong cwd/nonce/epoch/source fails.
- [x] Fresh path-without-file remains memory-only: it may become a promoted current new context, but resume/reload/restart-stable cursor stay unavailable.
- [x] Malformed interior line, duplicate id, orphan/broken parent, multiple root, unsupported entry, truncated frame, symlink/root escape, inode swap, and raw-vs-RPC mismatch all block `resumed`.
- [x] Exact continuation stages, remains non-current, then promotes only through `SpawnPromotionCommitted`; old-generation callbacks/output become quarantine and never transcript/current state.
- [x] Launch-effect ambiguity poisons the exact claim and does not launch another child; known identity reconciles only to the original claim/logical target.
- [x] Unknown cursor full replacement deletes an omitted stale projected entry in the real consuming fold; same-epoch retry is inert; cursor never leads projection.
- [x] N+1 resumed against the same verified Pi session loads N's cursor; a second logical target/native identity collision rejects.
- [x] Busy/unmaterialized reload rejects before effect; valid reload needs materialized request+completion, new handshake, rebind, and reconcile. Arbitrary dependency/runtime changes require process replacement.
- [x] The core manifest contains no Pi resource vocabulary; the opaque Pi profile and generic assurance fields are emitted/validated/replayed/rendered without becoming authority.
- [x] `AgentSessionRuntimeFixture` is the only SDK lifecycle test seam and all model/catalog/auth services are injected offline.
- [x] Every test awaits process-group exit, Observation/journal/cursor durability, expected promotion/poison, and late async completion; no orphan/error can pass after assertions.
- [x] Assurance prose says implementation-checked only unless independent formal/vector promotion exists.

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

## Implementation notes

- Execution capability: delegated cross-file implementation on the configured Codex worker; this story remains at `stage: review` for independent review.
- Manifest activation: `piCapabilityManifest` now requires one exact build-bound evidence set for the supervisor, challenged control handshake, strict tree validator, authoritative cursor replacement, idle materialized reload, and `pi-managed-lifecycle.v1`. It advertises managed `spawn`, the `pi-rpc` target shape, replacement/continuation/cursor/generation-fence/authoritative-reconciliation support, and manual handling for unproven outcomes. Every missing mechanism and wrong conformance version fails closed.
- Integrated process evidence: `pi-adapter/tests/e2e.test.ts` starts a real core server and real supervised offline Pi RPC children, proves fresh memory-only promotion, appends deterministic materialized history through a real extension command, resumes the exact physical session as generation N+1, verifies staged-before-promotion ordering and one promotion, performs live materialized reload, forces adapter delivery-stream reconnect/replay, checks no duplicate execution, and awaits journal/cursor/core acknowledgement plus process-group exit.
- Real-boundary fixes: the integrated oracle admitted the generated volatile projection envelope at core ingress, suppressed prior callbacks during replacement and current callbacks during reload-exclusive ownership, and allowed only the exact promoted candidate to publish behind its own promotion fence. The event waiters now cancel as soon as the exact durable transition/promotion appears instead of relying on an idle stream boundary.
- Shared vectors: new draft `pi-managed-lifecycle-manifest-activation` executes the Pi evidence-gated manifest boundary; draft `spawn-continuation-context-status-carriage` now executes Rust staging plus operator-domain fail-closed status presentation; and draft `spawn-reconnect-cursor-convergence` continues to execute both the shared exact-replacement owner and Pi cursor adapter. `docs/VERIFICATION.md` records these as implementation/draft-vector evidence only, not model-checked, checked-normative, or release-verified.
- Focused regression coverage: activation evidence/profile projection families, replacement/reload callback fencing, promoted-candidate publication, exact continuation status, and shared presentation/vector dispatch were added to the existing handshake/session-file/supervisor/cursor/reload suites.
- Mutation evidence: `cd pi-adapter && npm run test:mutations` passed with **25/25 killed** and restored the production tree after every run. Added witnesses kill generic-RPC cwd trust, malformed-line skipping, streaming reload admission, arbitrary-dependency reload overclaim, evidence-free manifest activation, missing volatile-envelope admission, and reload callback self-deadlock, alongside the existing journal/poison/resume/promotion/cursor/process/fixture witnesses.
- Full verification (2026-08-16):
  - Rust group: `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — passed.
  - Contracts group: `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build` — passed; 60 vectors, 19 promoted vectors, 33 implementation checks, 38 registered vector mutation witnesses, and clean generated/model traceability.
  - Operator-domain group: `cd operator-domain && npm run build && npm test` — passed, 32/32.
  - Pi-adapter group: `cd pi-adapter && npm test` — passed, 124/124, including both real-core flows and the real Pi managed lifecycle.
- Protobuf/generated bindings: unchanged. `contracts/ts` drift verification remained clean.
- Adjacent issues parked: none.
