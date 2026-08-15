---
id: research-handoff-spawn-reconnect-cursor-reconcile
kind: story
stage: review
tags: [adapter, protocol, verification]
parent: research-handoff-spawn
depends_on: [research-handoff-spawn-restart-continuation-orchestration, research-handoff-spawn-cursor-authoritative-replacement-contract]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-15
---

# Reconnect and authoritative cursor convergence

## Redesign disposition

Rewritten. Unknown-cursor recovery cannot upsert a full fetch into an old projection or key external continuity solely by Patchbay generation.

## Checkpoint

Prove convergence across three distinct authorities: core `(authority_domain_id, LSN)` lifecycle replay; adapter external persisted-state cursor scoped by verified external continuity identity; and surface snapshot/cursor reconciliation. A remembered stream/process handle/wall clock proves none of them.

A known external cursor applies a suffix. Unknown cursor stages a complete exact-set/tree projection, validates it, and atomically replaces projection + leaf + cursor + epoch. Stale omitted entries disappear. The replacement remains stale/unknown until commit and current process evidence; cursor installation cannot precede projection replacement.

## Design

**Files**
- `core/src/session/{registry,replay}.rs`, `server/src/{checkpoint,snapshot}.rs` — replay promotion, claims, quarantine, tombstones, and descendant authority.
- Adapter-neutral cursor replacement consumers; Pi storage/reconciler remains downstream.
- `web-cockpit/src/domain/{reconcile,model}.ts` — replace cached logical/current state only from newer core authority.
- Cross-layer vectors/runners.

Core replay and external cursor replay remain distinct; neither cursor translates into the other. Endpoint detach/reconnect does not change runtime generation. Adapter reconnect reauthenticates attachment generation before reporting current evidence.

## Acceptance evidence

- [x] Missing N→N+1 stream events reconcile to one logical target with N tombstoned and N+1 current/authorized only after promotion.
- [x] A poisoned/staged candidate remains non-live after restart/reconnect.
- [x] Unknown external cursor exact replacement removes stale omitted entries and atomically installs cursor/leaf/epoch.
- [x] External cursor scope survives Patchbay generation when verified native continuity is the same; cross-native-session reuse rejects.
- [x] Core replay reconstructs claims/fences/quarantine/promotion/authority deterministically.
- [x] Cached N, remembered live streams, or stale upsert entries cannot overwrite repaired authority.
- [x] Detach-does-not-retire, reconnect-after-stream-loss, cursor-gap-repair, and upsert-only mutations pass/fail as expected.

## Ordering constraint

Final spawn-side checkpoint after restart orchestration and the early cursor contract. The Pi redesign implements the reference external-cursor port.

## Implementation notes

- Execution capability: `openai-codex/gpt-5.6-sol`; caller-selected convergence proof unit for the security-critical spawn spine. Dispatch was direct-read only because Units 1–9 and Leaf 4 already exposed the exact replay, attachment, cursor, snapshot, and conformance seams; no recursive subagent was attempted.
- Review weight: `thorough` from the autopilot caller. Implementation is intentionally left at `stage: review` for the independent convergence review.
- Core authority convergence:
  - Strengthened `core/tests/runtime_evidence_promotion.rs` so the production continuation prefix is replayed repeatedly both before and after atomic promotion. Pre-promotion replay reconstructs the poisoned exact claim, pending-N fence, reserved N+1, and current N while proving N+1 non-live. Post-promotion replay independently converges sessions/logical-target tombstones, claim consumption, command completion, and descendant authority.
  - The existing outer-quarantine exhaustive test remains the replay oracle for all generated runtime evidence families; quarantine stays inert in every normal projection. The existing promotion/checkpoint suites continue to prove exact current/reserved pre-state, managed tombstone symmetry, and disposable inconsistent checkpoints.
- Adapter external-cursor convergence:
  - Added `operator-domain/tests/conformance-vectors.test.ts`, the `operator-domain` conformance runner, and `contracts/vectors/spawn-reconnect-cursor-convergence.json`. The runner consumes the concrete Leaf-4 `AuthoritativeCursorReplacement` through only its four narrow ports, proves N/N+1 share a scope only under the same verified external continuity, rejects cross-native reuse, retains the old projection stale during fetch/stage, and installs exact membership + leaf + cursor + epoch together. Omitted stale entries disappear; cursor/leaf publication carries the complete replacement rather than a cursor-only prefix.
  - Known-cursor suffix/idempotency and CAS/crash-prefix behavior remain covered by the Leaf-4 suite and are executed in the required operator-domain group. The Pi persisted-state store/profile remains deliberately downstream; no Pi path/session identity or protocol field was pulled into this adapter-neutral unit.
- Surface authority convergence:
  - `web-cockpit/src/domain/{model,reconcile}.ts` now validates matching positive core/storage continuity generations across session/resource baselines, validates every session record against its domain and snapshot horizon, records the accepted continuity anchor, and permits cached model replacement only from a strictly newer `(authority_domain_id, core_generation, LSN)` core prefix.
  - Snapshot replacement remains exact-set, never upsert: cached N and omitted members disappear before N+1 can render current/live. Equal-LSN remembered stream events are inert, and equal/older cached-N snapshots install nothing. New model and cross-layer runner tests prove atomic replacement and stale/live dominance.
- Detach/reconnect convergence: strengthened `server/src/adapter_service/tests.rs` to prove abnormal delivery-stream detach changes only connectivity/activity: runtime generation remains exactly N and no tombstone is created before or after current authenticated evidence restores live. Existing current-token/new-attachment tests prove attachment generation/token reauthentication precedes accepted current evidence.
- Files changed: `web-cockpit/src/domain/{model,reconcile}.ts`; `web-cockpit/tests/{model,reconcile,conformance-vectors}.test.ts`; `operator-domain/tests/conformance-vectors.test.ts`; `contracts/vectors/spawn-reconnect-cursor-convergence.json`; `contracts/scripts/check-vectors.mjs`; generated traceability in `docs/VERIFICATION.md`; strengthened core/server tests; this story file. No `.proto` or generated binding was edited.
- Tests and acceptance evidence: core continuation replay/promotion convergence; poisoned staged non-publication; exhaustive quarantine inertness; exact cursor replacement/known suffix/CAS crash and race suite; cross-native scope rejection; newer snapshot replacement; remembered-stream inertness; detach-does-not-retire; current attachment token fencing; web/CLI presentation suites. The draft cross-layer vector executes both adapter-neutral and surface production seams without claiming formal promotion.
- Mutation evidence: every requested source mutant was injected over the staged baseline, failed its focused production test, and was restored with `git restore`: cached-N equality overwrite; remembered-stream equal-LSN acceptance; surface stale-upsert merge; cursor unknown-recovery upsert-only merge; cursor-before-projection install; detach generation retirement/advance; and cross-native continuity-key reuse. All focused clean tests passed after restoration.
- Full verification:
  1. **PASS** — `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` (including 35 runtime-evidence/promotion tests and 82 server unit tests).
  2. **PASS** — `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build` (57 vectors, 17 promoted, 26 implementation checks, 38 registered mutation witnesses; generated bindings clean).
  3. **PASS** — `cd operator-domain && npm run build && npm test` (27/27).
  4. **PASS** — `cd pi-adapter && npm test` (38/38, including real-core e2e).
  - Surface suites also **PASS**: `cd web-cockpit && npm test` (133/133) and `cd cli && npm test` (48/48 plus real-core resource projection).
- Simplification: no parallel replay state machine, cursor translation, Pi-specific core ontology, replacement upsert path, generation allocator, or second snapshot authority was added. The vector runner reuses the sealed Leaf-4 transition owner and the cockpit reuses the existing exact snapshot builder.
- Discrepancies from design: the named `core/src/session/{registry,replay}.rs` and `server/src/{checkpoint,snapshot}.rs` mechanisms required no production edits: Units 1–9 already landed ordered aggregate replay, exact promotion/tombstone folding, typed checkpoints, and current snapshots. Unit 10 consumes those mechanisms and adds mutation-sensitive convergence evidence at their stable public seams rather than duplicating them. The Pi concrete cursor store remains downstream exactly as designed.
- Adjacent issues parked: none.
