---
id: pi-feature-review-2026-08-16
kind: story
stage: done
tags: [review, spawn, adapter]
parent: research-handoff-pi-adapter-capability
created: 2026-08-16
updated: 2026-08-16
---

# Integrated feature review — Pi managed lifecycle capability

## Verdict

**BLOCKER**

The real core + real Pi lifecycle is integrated and the three original Pi-side design BLOCKERs are closed, but the activated public capability is not yet honest against the generic assurance registry. The feature also carries a cumulative journal retention/size cliff and leaves the canonical protocol plus Pi adapter contract describing the pre-activation manifest. Return the feature with the bounded scope below; do not advance it to `done` yet.

Review mode: independent fresh-context feature review, effective weight `thorough`, pass 1 over the six completed Pi checkpoints and their converged child reviews. This pass reviewed the integrated capability and cross-story contracts rather than reopening converged child detail.

## Findings

### BLOCKER 1 — `AUTHORITATIVE` conflates transcript replacement with adapter-wide Operation-outcome reconciliation

**Locations:** `pi-adapter/src/core_client.ts:647-673`; `.work/active/features/capability-manifest-durability-and-reconciliation-depth.md:59,141`; `.work/active/stories/capability-manifest-durability-and-reconciliation-depth-consumer-wiring.md:59-61`; `pi-adapter/src/spawn_supervisor.ts:421-429,760-766`; `pi-adapter/src/reload_controller.ts:70-76`

The activated V1 manifest declares `reconciliation_strength = AUTHORITATIVE` across `spawn`, `instruct`, `cancel`, `interrupt`, `reconfigure`, and `session-management`. The registry this feature consumes defines that value as the conservative adapter-wide minimum and reserves it for an adapter that can query/rebuild the authoritative outcome for **every declared externally-effecting path** inside its retention scope. The sibling consumer design expected Pi's post-conformance value to be bounded, not authoritative.

The landed evidence proves authoritative **persisted-transcript projection replacement**: exact-set rebuild, omitted-member deletion, publication acknowledgement, and local cursor CAS. That evidence earns `cursor_support = true`; it does not establish adapter-wide Operation-outcome reconciliation. The supervisor deliberately retains an unrecoverable `launch_attempted / may_exist` cell that poisons and reports `execution_outcome_unknown`, and reload likewise has a persisted post-effect ambiguity outcome requiring manual reconciliation. `unproven_outcome_action = MANUAL_REQUIRED` honestly exposes those holes, but it does not make the reconciliation-strength claim authoritative.

The Unit-6 activation oracle encodes the same category error, so the green suite cannot adjudicate it: it expects `AUTHORITATIVE` after exercising cursor replacement. Downgrade the generic minimum to the strongest value actually proved (the dependency design points to `BOUNDED`, otherwise `NONE`), and update the activation vector/oracles. Keep `AUTHORITATIVE` only if Pi gains and tests authoritative outcome reconstruction for every declared externally-effecting path.

### MATERIAL 2 — the recovery journal is both unbounded over history and smaller than the supported projection/session boundary

**Locations:** `pi-adapter/src/spawn_supervisor.ts:541,651-659`; `pi-adapter/src/spawn_journal.ts:14,310-326,338-342,414-424`; `pi-adapter/src/cursor_store.ts:23-24,309-323`; `pi-adapter/src/session_file.ts:24-29`

Every successful managed spawn permanently retains the complete serialized staged cursor publication in a per-claim journal. There is no terminal compaction/pruning path, while startup `reconcileAll()` scans and validates every historical journal. Disk use and adapter startup therefore grow with the number of historical spawns multiplied by their projected transcript sizes, even after core promotion and local publication are durable.

The bounds also disagree at a dangerous lifecycle point: strict session validation accepts up to 64 MiB and the cursor store accepts up to 16 MiB, but the spawn journal caps the full recovery capsule at 2 MiB. `recordStagedPublication` runs after the successor launch and, for continuation, after prior N was terminated. A valid projection that exceeds 2 MiB can therefore fail at the journal write only after external effect, poison the claim, and leave the prior unavailable. The current tests cover small fixtures but no boundary-sized continuation or long-history restart.

Add a terminal compact journal form sufficient to validate replayed promotion deliveries without retaining the full projection forever; bound/prune completed history under an explicit retention contract. Align the staged recovery payload with the supported projection bound, or preflight an explicit supported-size limit before prior termination/launch. Add large-session and many-completed-claims restart tests plus a bounded operator-visible failure if the limit is intentionally retained.

### BLOCKER 3 — canonical current-state documentation still describes the pre-activation Pi manifest

**Locations:** `docs/PROTOCOL.md:750-752`; `docs/ADAPTER-PI.md:91-104`; `pi-adapter/src/core_client.ts:642-697`

`docs/PROTOCOL.md` says the **current** Pi declaration has all continuation/cursor/fence fields false, reconciliation `none`, session replacement false, and no `spawn` or managed shape. `docs/ADAPTER-PI.md` repeats those values and explicitly says Pi rejects `spawn` as unsupported. The running constructor now declares `spawn`, shape `pi-rpc`, session replacement, continuation/cursor/generation-fence support, and stronger reconciliation.

This is false/stale standing contract text, not a future-state omission. It also makes `docs/UX.md` consumers defer to a canonical protocol assertion that contradicts the durable registered manifest. Roll both documents forward with the final evidence-backed declaration and its conditional materialization/manual-ambiguity limits. Preserve the adapter-honesty trust boundary and do not describe implementation/draft-vector evidence as formal or release verification.

## BLOCKER 6 / 9 / 10 closure matrix

| Original design gate finding | Landed closure | Integrated evidence | Result |
|---|---|---|---|
| **BLOCKER 6 — unknown-cursor replay was upsert-only** | Pi continuity is generation-independent; unknown cursor stages one exact-set replacement; the consumer deletes omitted memberships; publication precedes local cursor CAS and `live`. | `entry_reconciler.ts` consumes the shared `AuthoritativeCursorReplacement`; operator-domain/cockpit folds execute exact replacement; Unit-6 E2E waits for durable cursor/journal acknowledgement after promotion; mutation suites kill skipped replacement and cursor-without-write paths. | **CLOSED.** The generic `AUTHORITATIVE` manifest label remains a separate overclaim identified above. |
| **BLOCKER 9 — generic RPC could not prove cwd** | The adapter-owned control extension emits a challenged custom entry carrying initialized `ctx.cwd`, launch nonce, session id/path, and extension epoch; generic RPC cross-checks only fields it actually exposes. | Real Pi fresh/continuation/reload paths all handshake; wrong cwd with matching RPC path/id, stale challenge/nonce/epoch, wrong source, and marker-only mutants fail. | **CLOSED.** |
| **BLOCKER 10 — deferred JSONL was treated as durable** | `memory_only`, `materialized`, and `invalid` are explicit; no flush is invented; `require_resume` and reload require strict materialization, while fresh memory-only promotion is only `new_context`/volatile. | Unit-6 E2E starts fresh memory-only, materializes through an offline extension command, then performs exact resumed continuation and materialized reload. Missing-file/materialization and memory-only-resume mutants fail. | **CLOSED.** |

## Original Pi MATERIAL traceability

| Review theme | Integrated disposition |
|---|---|
| Pi vocabulary in core | Closed: Pi vocabulary is generated in `pi_adapter.proto`; core sees one bounded opaque profile descriptor/bytes and does not decode it. |
| Reload active-work race | Closed: the one runtime action gate owns stdin/action admission and rejects streaming, compacting, queued, unsettled, delivery-busy, direct-RPC-busy, stale, and unmaterialized cases before effect. |
| Session seal omitted full tree validity | Closed: every-line framing/schema/id/reference/tree validation plus raw/RPC equality and pre/post seal checks gate `resumed`. |
| Manifest completeness / activation | Mechanism gating closed, but declaration honesty is **not closed** because `AUTHORITATIVE` exceeds the consumed generic registry semantics (Finding 1). |
| External-runtime uniqueness / pending replacement fence | Closed through core reservation plus exact claim/fence consumption and the per-target action gate. |
| Reload scope overclaim | Closed: only entrypoint and Pi-enumerated resources reload; arbitrary imports, package `/dist`, native modules, executable, and unknown scope require replacement. |
| Generation-keyed cursor | Closed: verified Pi continuity key excludes Patchbay generation and enforces reverse logical-target binding. |
| Authenticated-adapter honesty limit | Closed in `docs/SECURITY.md`: authentication/correlation is not proof that a buggy or malicious current adapter reported external reality honestly. |
| Phase/connectivity mapping | Closed by generated phase/effect evidence and explicit failed/stale/offline + unknown-activity mappings. |
| SDK fixture ambient credentials | Closed: the named `AgentSessionRuntimeFixture` requires injected offline model/catalog/auth/resource/session services. |

## End-to-end and cross-story assessment

- **Real composed lifecycle:** the Unit-6 real-process test runs fresh managed spawn, memory-only publication, offline materialization, exact continuation to N+1, challenged handshake/tree validation, staged successor quarantine, atomic core promotion, post-promotion cursor publication/CAS, materialized same-process reload, delivery-stream reconnect/replay, and process-group shutdown. The code seam walk matches that order.
- **Shared contracts:** the Pi supervisor consumes the generated accepted claim, compound continuation provenance, pending-replacement fence, phase/effect vocabulary, staged successor, and promotion delivery. Cursor reconciliation consumes the shared operator-domain transition owner. No Pi vocabulary was found in core behavior branches.
- **Availability/failure honesty:** pre-effect refusal, post-launch poison, stale/failed/offline session evidence, and manual-required ambiguity are visible through canonical failure/state presentation. No child note silently parked an implementation hole; all six say none after their convergence rounds. Findings 1-2 are aggregate contract/operational issues not visible at one child boundary.
- **Formal labels:** `docs/VERIFICATION.md` correctly calls the Pi lifecycle implementation-checked and the three Pi/spawn vectors draft. It does not claim model-checked, checked-normative, release-verified, or formal coverage.
- **Security/UX:** `docs/SECURITY.md` states the authenticated-adapter honesty assumption and canonical redaction boundary. `docs/UX.md` reuses canonical state/failure/retry presentation and adds no Pi protocol state.

## Activated manifest: declaration versus evidence

| Current declaration | Evidence assessment |
|---|---|
| Operations: `spawn`, `attach`, `instruct`, `cancel`, `interrupt`, `query`, `reconfigure`, `session-management` | Mechanisms/delivery paths exist; approval and elicitation response remain honestly absent. |
| Shape: `pi-rpc`; session replacement `true` | Exact managed fresh/continuation real-process path and atomic promotion are exercised. Resume availability remains conditional on materialization as described by the opaque profile. |
| Deduplication: `at-Patchbay-boundary` | Honest; no end-to-end external exactly-once claim. |
| Continuation proof / cursor / generation fence: `true` | Supported by exact seal/handshake, external continuity replacement, callback/action fences, core quarantine, and promotion ordering. |
| Reconciliation: `authoritative` | **Overclaimed.** Evidence is authoritative for persisted transcript membership, not every declared externally-effecting Operation outcome. |
| Unproven outcome action: `manual-required` | Honest and necessary for launch/reload ambiguity; it is also evidence that adapter-wide outcome reconciliation is not authoritative. |
| Snapshot: `partial`; target category: runtime session; no resource capabilities | Honest. |
| Opaque `patchbay.PiRuntimeProfile.v1` | Complete adapter-owned generated semantics; core validates only generic framing/size and diagnostics omit raw bytes/paths. |

## Substrate

- Direct children: **6/6 `done`**.
- Child review convergence passes: control/session integrity **2**; manifest/profile **2**; RPC supervisor **4**; cursor replay/resync **2**; resource reload **2**; lifecycle activation **1**.
- All child review files are retained under `.work/active/reviews/`.
- The feature body's child list exactly matches the six child files.
- The extension-pressure section uses all three required classifications: committed v1.0.0, reserved seams, and explicitly rejected directions.

## Full clean-tree suite

| Group | Result |
|---|---|
| Rust workspace | **PASS** — all-target build, full workspace tests/doctests, and warnings-denied all-target clippy. |
| Contracts | **PASS** — generated drift, vectors, models, TypeScript build; 60 vectors, 19 promoted, 33 implementation checks, 38 mutation witnesses. |
| Operator domain | **PASS** — 32/32. |
| Pi adapter | **PASS** — 124/124, including both real-core flows and real Pi managed fresh/continuation/reload/reconnect lifecycle. |
| Web cockpit | **PASS** — 148/148. |
| CLI | **PASS** — 53/53 plus real-core resource projection. |
| token-commune adapter | **PASS** — 63/63, including both real-core flows. |
| Hygiene | **PASS** — clean tracked tree and `git diff --check`; no suite-created Patchbay core or Pi RPC process remained. A pre-existing long-running mockup server was not part of this review and was left untouched. |

## Recommendation

**Return the feature with scope; do not advance to `done`.**

Required current-cycle scope:

1. Make the generic reconciliation declaration match the registry's adapter-wide Operation-outcome semantics and strengthen the activation oracle so cursor replacement cannot satisfy that field by itself.
2. Bound completed-journal retention and remove the post-launch 2 MiB recovery-capsule cliff, with large-session and many-history restart evidence.
3. Roll `docs/PROTOCOL.md` and `docs/ADAPTER-PI.md` from the pre-activation declaration to the actual evidence-backed manifest and conditional availability limits.

Because the effective weight is `thorough`, rerun an integrated fresh-context feature review after those fixes. Advance only when that pass has no receiver-confirmed material current-cycle blocker.

## Fixed note — Pi feature r1 (2026-08-16)

- **Finding 1 fixed:** the production Pi V1 declaration and all activation/E2E oracles now use adapter-wide `bounded` reconciliation plus `manual-required`. Exact-set transcript replacement remains authoritative only inside its verified continuity scope; launch/reload Operation-outcome ambiguity remains `execution_outcome_unknown`. The complete evidence gate now names `boundedJournalRetention` and version `pi-managed-lifecycle.v2`. Rationale: `BOUNDED` is the strongest conservative minimum supported by both the authoritative cursor subset and the manual external-effect cells; `AUTHORITATIVE` would still conflate two different proof domains.
- **Finding 2 fixed:** active per-claim journals admit 64 MiB (the strict session bound and above the 16 MiB cursor-store bound), then compact the full staged projection to a small terminal receipt after promotion + publication/cursor commit. Completed receipts are bounded to 24 hours and 128 records, are pruned by count/time on commit/startup, and are excluded from active recovery scans; legacy full terminal records compact on the next prune. Safely terminal pre-launch claims are deleted after core failure publication, while launch-attempt/poison ambiguity cannot be abandoned or evicted by completed-history retention. Tests cross the old 2 MiB boundary, restart with many completed claims, expire receipts while retaining active ambiguity, and cover safe/unsafe abandonment.
- **Finding 3 fixed:** `docs/PROTOCOL.md`, `docs/ADAPTER-PI.md`, and the corresponding `docs/VERIFICATION.md` evidence text now describe activated `spawn`/`pi-rpc`, replacement/continuation/cursor/fence support, bounded reconciliation, conditional materialization, manual ambiguity, and the journal retention contract. No Protobuf change was needed.
- **Mutation evidence:** all **28/28** registered Pi mutants were killed. The new adapter-wide `AUTHORITATIVE` overclaim, completed-retention prune skip, and safe-abandonment retention mutants each fail focused oracles; the mutation runner restored sources after every probe. No repository doc/manifest mismatch harness exists, so documentation drift was checked directly against the manifest/vector and by the full contracts/docs checks rather than inventing a second registry.
- **Full verification:** Rust `cargo fmt --all -- --check && cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`; contracts `npm run check:drift && npm run check:vectors && npm run check:models && npm run build && npm run check:presentation && npm run test:presentation` (60 vectors, 19 promoted, 33 implementation checks, 38 mutation witnesses); operator-domain **32/32**; Pi adapter **127/127 + 28/28 mutants**; web cockpit **148/148**; CLI **53/53 + real-core resource projection**; token-commune **63/63**. All passed. `git diff --check` passed and no Patchbay core/Pi RPC test process remained.
