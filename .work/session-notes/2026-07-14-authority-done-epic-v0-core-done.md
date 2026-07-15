# Session note — 2026-07-14 (authority arc done, epic-v0-core done)

A durable handoff note for the next session. Read this before continuing.

## Where we are

`epic-v0-core` (the Rust coordination core) — **DONE.** All 4 child features complete. The authority feature shipped end-to-end this session (implement → deep review → fix arc → convergence → done). The parent `epic-v0-1-0-implementation` is now at `implementing` (root child done; 5 surface/adapter layers remain at drafting).

## What happened this session

### 1. `feature-v0-core-authority` — full arc to DONE (commit `bf6f710`)
Implement-orchestrator (5 waves, 8 stories) → deep review (two-phase, cross-model gpt-5.6-sol xhigh) → 2 blockers + 1 verification gap + 4 backlog → fix arc (3 stories) → adversarial re-review caught an incomplete fix → fix pass 2 → convergence stable → DONE.

**Implementation (5 waves, mostly serial — inherent: the authority module is one coherent safety-critical unit with an atomic cross-module signature change and a shared `mod.rs`):**
- Wave 1: `story-sessions-spawn-origin-field` (sessions prereq — additive proto field, alone because contract regen serializes on shared gen)
- Wave 2: `story-v0-core-authority-registry` (foundation — GrantRecord, grant_authorizes, target_scope_matches, AuthorityRegistry)
- Wave 3: bundle B1{grant-check + acceptance-issuer-context + ingest} — the atomic `GrantCheck::check` signature change (`&ActorEndpointRef` → `&dyn IssuerContext`) is build-breaking across core+tests; one agent kept it green. xhigh tier.
- Wave 4: bundle B2{spawn-tail + replay} — two log-consumer folds sharing `mod.rs`
- Wave 5: `story-v0-core-authority-proptests` (7 oracles + 2 mutation tests + 1 documented gap)

**One subagent misread the intra-run dep-readiness rule** (refused to start because S1 was at `review` not `done`). Corrected by re-dispatch with explicit guidance: intra-run `review`-stage deps are buildable by the wave-plan design; the "must be `done`" rule applies to deps OUTSIDE the work set. **This was a real process lesson — add this guidance to the implement-orchestrator skill prompt proactively.**

**The deep review found 2 real blockers (both contradicted PINNED design decisions, not deferrals):**
- Blocker 1: `same_session` (RuntimeSession scope match) omitted `deployment_scope` — the exact-tuple was pinned as adapter+deployment+runtime+generation. The matrix test blessed the bug (empty deployment_scope on both sides). A grant for `(pi, machine-a, session-1, gen-7)` would have authorized `(pi, machine-b, session-1, gen-7)`.
- Blocker 2: `observe_revocation` treated a same-generation revocation as exact redelivery WITHOUT comparing content — contradicted the rev3 CorruptLog guarantee.

**Plus 1 verification gap:** the `compound_issuer` proptest called `GrantCheck::check` directly, not `acceptance::submit` — so it proved the impl rejects mismatched verified actors, but didn't prove the `submit` call site passes a verified issuer. rev3-review finding 4 intended this as an integration property.

**The convergence loop caught an incomplete fix (the key moment):**
- The first blocker-2 fix compared only `revoked_at` + `revocation_policy`, missing `revoked_by`/`reason`/`audit_id`. A fresh-context adversarial re-review caught this — exactly the sessions B3→B5-style incomplete-fix pattern. The convergence loop exists for this; it found it.
- Fix pass 2: `GrantRecord` now retains the full revocation fingerprint; `observe_revocation` compares generation+timestamp+policy+actor+reason+audit_id. Genuinely closed.

### 2. Prerequisite stories + re-opened parents → DONE (commits `95cdde8`, `942edb6`)
- `story-sessions-spawn-origin-field` + `story-acceptance-issuer-context` → fast-lane review → done.
- Re-reviewed their parents (`feature-v0-core-sessions`, `feature-v0-core-acceptance`) — delta-scoped, both remain done. The issuer-context child notably RESOLVES the acceptance feature's own deep-review blocker #3 (verified-identity seam) — brings acceptance INTO alignment with `docs/SECURITY.md:143`.

### 3. Epic-level review → epic-v0-core DONE (commit `5abfb8f`)
All 4 child features done. Aggregate alignment confirmed: Ports & Adapters seams realized (authority impls GrantCheck, sessions impls TargetResolver, no adapter leak into domain). Cross-cutting promoted properties (BoundaryDedup, NoAcceptedToCompleted, GenerationMonotonic) verified at feature boundaries with mutation evidence.

### 4. Stale-stage correction → epic-v0-1-0-implementation drafting → implementing (commit `0f658d3`)
**Operator caught a substrate-discipline bug I'd been rationalizing.** `epic-v0-1-0-implementation` was at `drafting` despite a fully settled decomposition (6 children + critical path + depends_on). Per `epic-design` Phase 8, an epic advances `drafting → implementing` when its DECOMPOSITION is done, not when all children are done. I'd been misreading "drafting" as "early/unbuilt" — wrong. The skill is explicit; the stage was just stale. Corrected.

## Current queue state

### `epic-v0-1-0-implementation` — implementing (root child done; 5 layers drafting)
| Child | Layer | Stage |
|---|---|---|
| `epic-v0-core` | Rust coordination core | **done** |
| `feature-v0-protocol-seam` | web↔core RPC seam | drafting (next on critical path) |
| `feature-v0-pi-adapter` | Pi adapter | drafting (parallel off core) |
| `feature-v0-web-server` | TS web server | drafting |
| `feature-v0-web-cockpit` | web cockpit | drafting |
| `feature-v0-cli` | CLI | drafting |

### `epic-v0-core` — done (all 4 child features done)
| Feature | Stage |
|---|---|
| `feature-v0-core-persistence` | done |
| `feature-v0-core-acceptance` | done |
| `feature-v0-core-authority` | done |
| `feature-v0-core-sessions` | done |

## What v0.1.0 authority delivers (the feature just shipped)

A **component-complete, tested** authority layer (NOT live-wired, per rev3 design): deny-by-default grant evaluation against durable grants; the `GrantCheck` port with verified `IssuerContext` (not self-asserted) + domain equality; durable Grant/DescendantGrant/Revocation events with provenance; revocation marks-not-deletes (audit retention) with COMPLETE conflicting-duplicate detection; the descendant-grant allowed-kind set (8 kinds, spawn+attach excluded); two-lever non-cascade revocation (structural); descendant-grant-on-spawn via order-independent log-tail correlating `spawn_origin`; the `AuthorityDomainId` key shape (federation seam); 7 property oracles + 2 mutation tests + 1 documented gap (#8 ElicitationResponderAuthority).

The live operator-issuing path (ingress + fleet-target-resolution + live composition) is follow-on — see backlog.

## Backlog filed this session (4 new authority items in `.work/backlog/`)
From the authority deep review:
- `backlog-authority-payload-actor-in-descendant-issuance` — descendant subject derived from self-asserted Operation.sender; resolve with durable acceptance metadata. Couples with `backlog-authority-durable-acceptance-metadata` + `backlog-authority-live-composition`.
- `backlog-authority-grant-selection-determinism` — overlapping matching grants return nondeterministic grant_id (HashMap iteration). Latent single-operator.
- `backlog-authority-ingest-pre-append-conflict-check` — ingest appends before conflict check (poisons log on conflict); no durable retry idempotency. Latent single-writer; blocking for live path.
- `backlog-authority-replay-gap-detection` — replay accepts gapped LSNs + ignores Unspecified-kind. Defense-in-depth; rusqlite is gap-free.

Plus 3 fix stories (now done): `story-fix-authority-runtime-session-deployment-scope`, `story-fix-authority-conflicting-revocation-detection`, `story-fix-authority-compound-issuer-integration-test`.

(Pre-existing authority backlog: `backlog-authority-durable-acceptance-metadata`, `backlog-authority-live-composition`, `backlog-authority-failed-authorization-audit`, `backlog-grant-expiration-enforcement`, `backlog-fleet-target-resolution`, `backlog-elicitation-responder-authority`.)

## Critical build/environment notes (READ BEFORE ANY CARGO)

- **`CARGO_HOME=/tmp/cargo-home`** is REQUIRED for all cargo commands — the default `~/.cargo` registry cache is read-only. `/tmp/cargo-home` has the vendored deps.
- **`buf` at `/home/agent/.npm-global/bin/buf`**; `protoc-gen-prost` at `/home/agent/.cargo/bin/protoc-gen-prost`. Include both on PATH for proto regen.
- **`/tmp` fills up (tmpfs, 5.9G).** `RusqliteStorage::open_in_memory()` uses `/tmp` NamedTempFiles. **This session hit a real `/tmp` disk-full mid-review** — 146K accumulated `.tmp*` SQLite tempfiles (from test-run panics across prior sessions) filled it, making elicitation tests fail with "database or disk is full." Looked like a code regression; was environment. Fix: `find /tmp -maxdepth 1 -name '.tmp*' -type f -delete`. If tests fail with disk-full, check `df -h /tmp` FIRST before debugging code.
- **Proto regen wrinkle** (unchanged): edit proto → `cargo build -p patchbay-contracts` (Rust, prost-build) → `buf generate` from `contracts/` (TS) → `git checkout contracts/rust/src/gen` → `cargo build -p patchbay-contracts` (restore Rust format). Gen diff must be additions-only. `buf generate` produces DRIFTED Rust formatting; prost-build produces the committed format.
- Verification: `CARGO_HOME=/tmp/cargo-home cargo build -p patchbay-core && cargo test -p patchbay-core && cargo clippy --all-targets -- -D warnings && cargo fmt --all --check`
- **189 tests** across `patchbay-core` (33 authority tests: 11 registry + 5 grant-check + 6 ingest + 6 spawn-tail + 2 replay + 11 proptest incl. 2 mutation + 1 integration). clippy clean, fmt clean.

## Process lessons (for the next session / skill improvements)

1. **Intra-run dep-readiness for implement-orchestrator subagents.** A subagent refused to start because a dependency story was at `review` not `done`. The orchestrator's wave structure orders intra-run deps; `review`-stage means "implemented, awaiting review, code stable." The "must be `done`" rule applies to deps OUTSIDE the work set, not intra-run deps the wave plan already ordered. **Proactively include this guidance in implement-orchestrator worker prompts** so they don't halt unnecessarily. (I corrected by re-dispatch with explicit guidance, but it cost a turn.)
2. **Epic stage semantics.** An epic advances `drafting → implementing` when its DECOMPOSITION is done (per `epic-design` Phase 8), not when children progress. Don't rationalize a `drafting` epic with a settled decomposition as "early" — it's a stale stage; advance it. (Operator caught this; I'd been misreading.)
3. **The deep-review convergence loop is load-bearing.** The blocker-2 incomplete fix (compared only policy+timestamp, not actor/reason/audit_id) passed green tests and clippy. Only the fresh-context adversarial re-review caught it. This is the same shape as the sessions B3→B5 regression. **Always run the re-review pass after a fix arc in safety-claiming features** — don't rubber-stamp fixes.
4. **`/tmp` disk-full is a recurring environment hazard.** The `tempfile` crate's SQLite tempfiles accumulate when tests panic. Consider a pre-test cleanup or a larger TMPDIR. Symptom: "database or disk is full" on rusqlite tests. Not a code bug.

## Next logical step

**The v0.1.0 critical path now moves to the surface/adapter layers.** `epic-v0-core` is done — the protocol seam, Pi adapter, web server, cockpit, and CLI can build against a tested core.

The next dependency-layer children (both unblocked, both at `drafting`):
- `feature-v0-protocol-seam` (depends on `epic-v0-core` ✓) — web↔core internal RPC, streaming channel, auth boundary. Next on the critical path.
- `feature-v0-pi-adapter` (depends on `epic-v0-core` ✓) — Pi adapter. Parallel branch.

Either needs a `feature-design` pass first (they're at `drafting`). The protocol-seam is the root of the phone-usable path (→ web-server → web-cockpit); the pi-adapter is the agent-control path. The protocol-seam also unblocks the CSRF/browser formal properties (the 4 `csrf_browser.qnt` promoted properties belong to `feature-v0-web-server`, which depends on the seam).

**Alternatively**, if the operator wants to harden the core before building on it: the 4 authority backlog items (payload-actor trust, grant-selection determinism, ingest pre-append conflict, replay gap detection) are latent but become blocking at the live path. None block v0.1.0 component-complete.

## Git log (this session, most recent first)
```
0f658d3 epic: epic-v0-1-0-implementation drafting -> implementing (decomposition was settled, stale stage)
5abfb8f review: epic-v0-core -> done (all 4 child features done, integration verified)
bf6f710 review: feature-v0-core-authority -> done (all findings closed, convergence stable)
6663af4 review: feature-v0-core-authority re-advanced to review (all 3 findings closed, convergence stable)
35d0774 review: complete blocker-2 fix (full revocation fingerprint) + advance feature to review
17fc421 implement: story-fix-authority-compound-issuer-integration-test
43ad835 implement: story-fix-authority-conflicting-revocation-detection
b871cf8 implement: story-fix-authority-runtime-session-deployment-scope
a7fa1e9 review: feature-v0-core-authority deep review -> implementing (2 blockers + 1 verification gap, 4 backlog)
942edb6 review: re-review re-opened parents feature-v0-core-sessions + feature-v0-core-acceptance (delta, remain done)
95cdde8 review: prereq stories spawn-origin-field + acceptance-issuer-context -> done
7c9daf3 implement: feature-v0-core-authority (6 stories + 2 prerequisites ready for review)
3106e9f implement: story-v0-core-authority-proptests
6c144b3 implement: story-v0-core-authority-replay
9a5f1a0 implement: story-v0-core-authority-spawn-tail
fe2f2da implement: story-v0-core-authority-ingest
9da850f implement: story-acceptance-issuer-context
e9339c4 implement: story-v0-core-authority-grant-check
009ca90 implement: story-v0-core-authority-registry
6f98304 implement: story-sessions-spawn-origin-field
```
