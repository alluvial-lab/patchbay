---
id: pi-lifecycle-conformance-review-2026-08-16
kind: story
stage: done
tags: [review, spawn, adapter]
parent: research-handoff-pi-adapter-capability-lifecycle-conformance
created: 2026-08-16
updated: 2026-08-16
---

# Review: Pi integrated lifecycle conformance and manifest activation

## Verdict

**CLEAN**

Commit `bb02191` honestly activates the managed Pi declaration. Each newly positive generic assurance/profile claim has a production mechanism, a real-boundary integration path, and a mutation-sensitive oracle. The callback-fencing and volatile-ingress fixes preserve the Unit 3/4 authority boundaries: prior callbacks are inert during replacement, only the exactly promoted candidate can publish behind its claim fence, reload-owned callbacks cannot self-queue behind exclusive ownership, and the volatile envelope remains explicitly non-authoritative.

## Findings

No material findings or nits.

## Claim → evidence → oracle verification

| Activated claim | Production and real-boundary evidence | Oracle / adversary | Result |
|---|---|---|---|
| Managed `spawn` and target shape `pi-rpc` | The emitted manifest is gated by the exact evidence set (`pi-adapter/src/core_client.ts:633-672`). The real test observes the durably registered activated manifest, submits fresh and continuation spawns, and waits for both authority-bearing promotions (`pi-adapter/tests/e2e.test.ts:641,744,767,815`). | The draft activation runner checks `SPAWN`, the exact shape, replacement, all V1 positives, every absent mechanism, and the wrong version (`pi-adapter/tests/conformance-vectors.test.ts:159-225`). Setting one production evidence member false made the focused manifest test fail before emission. | Proven; active. |
| Session replacement and continuation proof | The real child materializes a physical JSONL, the successor resumes the same Pi session id as generation N+1, and promoted evidence carries `RESUMED` (`pi-adapter/tests/e2e.test.ts:791-835`). Both fresh and continuation reports are staged at a lower LSN than promotion (`pi-adapter/tests/e2e.test.ts:839-860`). | `fresh spawn journals…publishes only after exact promotion` and `require_resume refuses a memory-only prior` cover the failure boundaries; the mutation cycle kills early publication and memory-only resume mutants. | Proven; active. |
| Generation fencing | Core promotion remains the only current-generation transition; the real test requires one promotion and exact staged-before-promoted ordering. Registry callbacks additionally check attachment identity, current entry, active replacement claim, and the promoted candidate's exact claim (`pi-adapter/src/session_registry.ts:287-296`). | Removing the prior-callback fence produced 4 escaped callbacks instead of 0. Forcing promoted-candidate fence ownership false suppressed the required post-promotion publication (0 instead of 1). The regular suite covers stale generations and the mutation suite kills publication-before-promotion. | Proven; active. |
| External cursor support and authoritative reconciliation | Materialization enters the durable cursor path; the real lifecycle waits for publication/journal completion and finds durable cursor state only after core acknowledgement (`pi-adapter/tests/e2e.test.ts:929-959`). N+1 cursor reuse and exact omission deletion execute through the production reconciler (`pi-adapter/tests/entry_reconciler.test.ts:203,245`), with the shared consuming fold exercised by the draft cursor vector. | Mutants collapsing continuity scope, skipping replacement publication, and acknowledging without the atomic cursor write all died. Contracts also executed 38 registered vector mutation witnesses. | Proven; `AUTHORITATIVE` is honest. |
| Challenged control proof and cwd correlation | Every real child must load the production control extension and complete the challenged handshake before staging/resume/reload. Generic RPC identity alone remains insufficient. | Wrong initialized cwd with correct RPC path/id fails (`pi-adapter/tests/control_handshake.test.ts:62`); the generic-RPC-trust mutant died. Nonce, epoch, source, and marker-only cases are covered by the focused handshake suite. | Proven; profile claim active. |
| Conditional materialization and restart-stability boundary | Fresh spawn publishes a volatile snapshot with `restartStable=false`; after a real offline assistant append, the selected JSONL exists and contains the fixture; continuation is materialized and restart-stable (`pi-adapter/tests/e2e.test.ts:791-796,942-954`). Volatile ingress admits only the three generated Pi projection schema families (`pi-adapter/src/core_client.ts:350-356,700-703`). | Memory-only `require_resume` dies before successor launch; removing volatile-schema admission kills the focused ingress oracle. Volatile projections carry no authoritative epoch and materialization starts at epoch one in the focused adapter/operator tests. | Proven; unmaterialized cursor/reload durability remains unavailable. |
| Strict persisted tree integrity | The real `RESUMED` path necessarily passes raw file validation, RPC equality, seal-prefix verification, and post-launch extension validation before staging. | Focused tests reject malformed/duplicate/orphan/multiple-root and unsupported/truncated trees (`pi-adapter/tests/session_file.test.ts:95,123`), plus symlink/root escape, inode swap, and raw/RPC mismatch. The malformed-line skipping mutant died. | Proven; active. |
| Idle materialized reload and bounded scope | The real generation-2 child completes reload only after persisted request/completion markers (`pi-adapter/tests/e2e.test.ts:862-883`), with re-handshake, rebind, cursor reconcile, and unchanged process identity in the production controller. The profile keeps arbitrary dependency graphs, Pi runtime `/dist`, native dependencies, executable, and unknown scope replacement-only (`pi-adapter/src/core_client.ts:769-776`). | Real Pi coverage proves the entrypoint refreshes while transitive and installed-package `/dist` artifacts stay old (`pi-adapter/tests/reload_controller.test.ts:86`). Streaming/compacting/queued/unsettled/delivery-busy/unmaterialized admission fails pre-effect (`pi-adapter/tests/reload_controller.test.ts:204`). Streaming-admission and dependency-overclaim mutants died. | Proven; active without scope overclaim. |
| Patchbay-boundary deduplication and manual handling of ambiguity | The declaration remains `AT_PATCHBAY_BOUNDARY`, not end-to-end (`pi-adapter/src/core_client.ts:668`). Reconnect/replay produces one delivered/running/completed chain; launch-effect ambiguity poisons and does not relaunch. `MANUAL_REQUIRED` accurately qualifies the remaining external-effect uncertainty. | Mutating the declaration to end-to-end made the exact manifest oracle fail. Response-loss classification, launch journal order, poison, duplicate-delivery, and replayed-promotion mutants all died. | Honest conservative claim. |
| Pi profile vocabulary stays opaque to core | Pi cwd/session-file/cursor/reload semantics remain generated profile bytes; generic core fields contain only adapter-neutral capability/assurance dimensions. Diagnostics expose only the safe descriptor. | Exact profile decoding rejects malformed/unsupported values; the manifest test asserts empty resource capabilities and a session-only target category. Contracts drift, Rust boundary, CLI, and web tests all passed. | Proven; no Pi vocabulary leaked into core semantics. |
| Offline fixture boundary | The integrated materialization command appends deterministic assistant history without an LLM turn or credential lookup; real Pi runs with `PI_OFFLINE=1`. | Missing catalog/auth markers and even a forged marker around an ambient `ModelRuntime` fail (`pi-adapter/tests/pi_session.test.ts:183,233`). Both fixture-injection mutants died. | Proven; no ambient credential dependency. |
| Unproven declarations remain conservative | Deprecated `idempotencyStrength` stays `UNSPECIFIED`, snapshot support remains `PARTIAL`, dedup is not end-to-end, unknown effects require manual action, resource capabilities remain empty, and approval/elicitation response support is not advertised by Pi. | The exact manifest/profile oracle fails on changed assurance, shape, snapshot, exclusions, or forbidden operations; the full first real-core flow checks the response-operation omissions. | No unproven positive claim found. |

## Callback fencing and volatile ingress

- **Replacement fence:** prior-incarnation callbacks fail `(!activeReplacementClaim || promotionOwnsFence)` and cannot emit transcript, model, lifecycle, or persisted-entry work while a replacement owns the target. The promoted entry records the exact promotion claim, so only that candidate can publish while the same fence remains held (`pi-adapter/src/session_registry.ts:159-165,224-256,287-296`).
- **Reload fence:** `withExclusiveCurrent` raises `observationsFenced` before `get_state` and keeps it set through marker polling, handshake, subscription rebind, reconciliation, and final report preparation; `finally` always clears it (`pi-adapter/src/runtime_action_gate.ts:119-143`). This prevents callback self-deadlock without relaxing the action reservation or replacement fence.
- **No Unit 3/4 weakening found:** successor publication still follows exact core promotion; cursor commit still follows durable projection acknowledgement; prior callbacks cannot kill or stale the successor; reload failure exits through the existing ambiguous/stale delivery classification after exclusive ownership unwinds.
- **Volatile ingress:** admission changed from a name-shaped regex to the exact generated suffix/replacement/volatile registry. The volatile compositor remains last-observation-wins, has no replacement epoch or restart-stable cursor claim, and is replaced by authoritative epoch one on materialization.

## Mutation matrix

| Mutation / injection | Expected oracle | Result |
|---|---|---|
| Full repository Pi mutation cycle | All control/tree/cursor/journal/promotion/reload/process/fixture mutants must fail | **25/25 killed**; final rebuild restored the source tree. |
| Set production `PI_CAPABILITY_EVIDENCE.supervisor=false` | Default manifest emission must fail closed | **Killed**: focused test exited 1 with `lacks complete conformance evidence`. |
| Overclaim `deduplicationStrength=END_TO_END` | Exact manifest oracle must reject unproven assurance | **Killed**: focused test reported actual 3 vs expected boundary strength 2. |
| Remove prior replacement-callback fence | Old callbacks must remain inert | **Killed**: focused test observed 4 callbacks, expected 0. |
| Deny the exact promoted candidate ownership of its fence | Post-promotion staged publication must occur once | **Killed**: focused spawn oracle observed 0, expected 1. |
| Remove reload callback suppression | Reload-owned callbacks must not queue behind their own gate | **Killed** in mutation cycle. |
| Remove volatile projection ingress | Fresh memory-only promotion must remain publishable but non-authoritative | **Killed** in mutation cycle. |
| Skip challenged cwd proof / malformed-line rejection / cursor publication / cursor atomic write / journal-before-launch / poison / materialization gate / promotion ordering / streaming reload rejection / process-replacement exclusion / offline injection | Each named mechanism's focused contract must detect its break | **Killed** in mutation cycle. |

Every manual mutation was applied to the main tree, run as one focused test, restored with `git restore`, and followed by a clean `git status`/diff check.

## Full verification

| Group | Result |
|---|---|
| Rust | `cargo build --workspace --all-targets`; `cargo test --workspace`; `cargo clippy --workspace --all-targets -- -D warnings` — **passed**. |
| Contracts | drift, vectors, models, build — **passed**; 60 vectors, 19 promoted, 33 implementation checks, 38 mutation witnesses. |
| Operator domain | build + **32/32 passed**. |
| Pi adapter | **124/124 passed**, including both real-core flows and the real Pi managed lifecycle. |
| Web cockpit | **148/148 passed**. |
| CLI | **53/53 passed** plus real-core resource projection. |
| token-commune adapter | **63/63 passed**, including real-core/gateway flows. |
| Pi mutation suite | **25/25 killed**. |

Post-run hygiene: git index and working tree were clean before review-file creation; no Patchbay core or Pi RPC child remained running. The lifecycle test removed its per-run core database, session, journal, cursor, extension, and process resources. Only pre-existing empty ignored scratch directories dated before this review remained, and they were not deleted or treated as evidence from this run.

## Assurance-label check

`docs/VERIFICATION.md:323-362` labels the new section **implementation-checked** and explicitly says the three linked vectors are draft, not checked-normative, not a formal promotion, and not release verification. The generated traceability row keeps `AdapterCapabilityAssuranceHonesty` stated-normative while listing the Pi activation vector as draft (`docs/VERIFICATION.md:652`). No assurance overclaim found.

## Recommendation

**Advance `research-handoff-pi-adapter-capability-lifecycle-conformance` to `done`.**
