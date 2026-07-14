# Session note — 2026-07-13 (sessions arc done, authority designed + reviewed)

A durable handoff note for the next session. Read this before continuing.

## Where we are

`epic-v0-core` (the Rust coordination core) — **3 of 4 child features done, 1 implementing.** This session shipped the sessions feature (design → implement → review → fix arc → done) and ran the authority feature through design + 3 review passes. Authority is now implementer-ready at `stage: implementing` with 8 stories across 3 features.

## What happened this session

### 1. `feature-v0-core-sessions` — full arc to DONE (commit `f843dc0`)
Design session (5 questions resolved interactively) → implement-orchestrator (5 stories, linear chain) → deep review (two-phase, cross-model gpt-5.6-sol) → found 4 blockers (B1-B4) → fix dispatch → re-review found B5 (regression from B3 fix) → B5 fix → re-review approved → DONE.

**Key design decisions (sessions):**
- Q1 delta events (mirrors acceptance's `CommandTransition`)
- Q2 defer snapshot checkpointing (replay from LSN 0, matches acceptance/elicitation)
- Q3 `TargetResolver` = existence + tombstone-only (tombstone is identity, connectivity is delivery)
- Q4 direct ingestion writer (sessions owns its transitions → it writes them; the decisive precedent: `ingest_observation` writer vs `ElicitationSlotLayer` pure-tail)
- Q5 full feature with 5 child stories

**The blockers found (all real, verified against code):**
- B1 empty identity fields → unreplayable log (write/replay validation parity)
- B2 generation bump discarded new generation's state (proto carried only from/to; cloned stale state)
- B3 multi-field report truncated (early return after first delta)
- B4 tombstone key omitted adapter_id+deployment_scope (cross-adapter collision)
- B5 (regression from B3 fix) partial multi-delta failure → retry duplicates → unreplayable. Fixed via warm-after-each-append (makes retry idempotent)

**Final state:** 152→161 tests, clippy clean, `TargetResolver` port connected. 3 backlog items parked (authority-domain isolation, test-coverage gaps, idempotency/concurrency — all latent single-domain/single-writer v0.1.0).

### 2. `feature-v0-core-authority` — designed + 3 review passes → implementer-ready (commit `4396792`)
Decided NOT to do a formal-backing pass first (authority.qnt has 0 promoted properties, 4 demoted; v1 formal gate owns the uplift). Design went through 3 revisions driven by 3 cross-model design reviews.

**The scope evolution (the important story):**
- **Rev 1**: implicit operator authority + log-tail reactor. Review #1 found 10 blockers (implicit authority nullified the machinery; `is_operator` trusted self-asserted payload; etc.).
- **Rev 2**: went vertical (live slice) — durable bootstrap grants + IssuerContext + reactor + composition layer. Review #2 found 4 partially-resolved + 8 new defects (bootstrap 50/50; IssuerContext not real compound-issuer; reactor order-dependent; composition not wired to log; etc.).
- **Rev 3 (current)**: **dropped the "live" framing** — component-complete, not live. This was the key insight (operator's): "it doesn't need to be a *live* slice." Dropped bootstrap grant + live composition (the ingress doesn't exist anyway). Pinned the surviving correctness items. Review #3 approved with 5 in-stride fixes (all mechanical/protocol-pinned, not a bounce).

**Final design decisions (authority rev 3):**
- R1 no bootstrap grant (operator-auth is the ingress's job; tests inject grants)
- R2 `IssuerContext` verified-identity port + test double; domain-equality pinned
- R3 order-independent reactor (3 maps + try_issue after any); exercised via replay, no live consumer
- R4 minimal audit (grant-lifecycle provenance; distinct failed-auth audit deferred)
- R5 fleet target resolution out-of-scope (backlog)
- R6 `ElicitationResponderAuthority` = documented gap, not vacuous test

**The 5 rev3-review in-stride fixes:** spawn-tail domain isolation `(domain, command_id)`; audit_id=None (component-tested not protocol-complete); FleetAuthorityForSpawn oracle corrected (PROTOCOL permits adapter-level spawn grants); compound_issuer test depends on acceptance prerequisite; filed 3 backlog items.

## Current queue state

### `epic-v0-core` — implementing
| Feature | Stage | Notes |
|---|---|---|
| `feature-v0-core-persistence` | done | ✓ |
| `feature-v0-core-acceptance` | done | ✓ (will re-open when `story-acceptance-issuer-context` lands) |
| `feature-v0-core-sessions` | done | ✓ (will re-open when `story-sessions-spawn-origin-field` lands) |
| `feature-v0-core-authority` | **implementing** | design done (3 reviews); 8 stories ready |

### Authority stories (8, across 3 features) — ALL at `stage: implementing`
**Prerequisites (re-open their parents' review surfaces):**
- `story-sessions-spawn-origin-field` (parent: sessions) — add `SessionRegistered.spawn_origin: TypedCorrelation` (field 9). Unblocks the authority spawn-tail.
- `story-acceptance-issuer-context` (parent: acceptance) — `submit` takes `&dyn IssuerContext`; `GrantCheck::check` signature change; retain `Authorized.grant_id`. Unblocks authority GrantCheck end-to-end.

**Authority (parent: feature-v0-core-authority):**
1. `story-v0-core-authority-registry` — grant/revocation event model + `AuthorityRegistry` + `grant_authorizes` + `target_scope_matches` (no deps; takes `IssuerRef` not the trait — decouples from story 2)
2. `story-v0-core-authority-grant-check` — `IssuerContext` trait + `impl GrantCheck` (depends on 1; defines the trait the acceptance prerequisite uses)
3. `story-v0-core-authority-ingest` — grant/revocation writer (depends on 1)
4. `story-v0-core-authority-spawn-tail` — order-independent reactor (depends on 1, 3, AND sessions prerequisite)
5. `story-v0-core-authority-replay` — `rebuild_from_log` + wiring (depends on 1, 2, 3)
6. `story-v0-core-authority-proptests` — 7 oracles + mutation tests + 1 documented gap (depends on 1-5 + acceptance prerequisite)

**Dependency graph / wave plan:**
- Wave 1 (parallel, 3 items): sessions prerequisite, acceptance prerequisite, authority registry (story 1)
- Then: grant-check (2) + ingest (3) parallel after 1
- Then: spawn-tail (4) needs 1, 3, + sessions prerequisite
- Then: replay (5) needs 1, 2, 3
- Then: proptests (6) needs all + acceptance prerequisite

## Critical build/environment notes (READ BEFORE ANY CARGO)

- **`CARGO_HOME=/tmp/cargo-home`** is REQUIRED for all cargo commands — the default `~/.cargo` registry cache is read-only in this sandbox. `/tmp/cargo-home` has the vendored deps.
- **`buf` is at `/home/agent/.npm-global/bin/buf`** — include on PATH for proto regen.
- **`/tmp` can fill up** (tmpfs, 5.9G). `RusqliteStorage::open_in_memory()` uses a `/tmp` NamedTempFile (WAL needs file-backed SQLite). If tests fail with "database or disk is full", /tmp is full — clear it.
- **Proto regen wrinkle**: `cargo build -p patchbay-contracts` (build.rs/prost-build) produces the committed Rust gen format. `buf generate` (protoc-gen-prost) produces DIFFERENT formatting (pre-existing drift). To regen after a proto edit: edit proto → `cargo build -p patchbay-contracts` (Rust) → `buf generate` from `contracts/` (TS) → `git checkout contracts/rust/src/gen` → `cargo build -p patchbay-contracts` (restore Rust format). Gen diff must be additions-only.
- Verification commands: `CARGO_HOME=/tmp/cargo-home cargo build -p patchbay-core && cargo test -p patchbay-core && cargo clippy -p patchbay-core --all-targets`

## Backlog filed this session (6 items in `.work/backlog/`)
From sessions review (3): `backlog-sessions-authority-domain-isolation`, `backlog-sessions-test-coverage-gaps`, `backlog-sessions-idempotency-and-concurrency`.
From authority design (6, 3 overlap with the deferred-R items): `backlog-authority-failed-authorization-audit` (R4), `backlog-fleet-target-resolution` (R5), `backlog-grant-expiration-enforcement`, `backlog-authority-live-composition`, `backlog-authority-durable-acceptance-metadata`, `backlog-elicitation-responder-authority` (R6).

## Next logical step

**Dispatch `/agile-workflow:implement-orchestrator feature-v0-core-authority`.** The design is implementer-ready (3 review passes, no unresolved semantic 50/50s). Wave 1 = the 3 parallel-ready items (sessions prerequisite, acceptance prerequisite, authority registry).

The two prerequisites re-open their parents' review surfaces (per substrate rule: a child landing under a done feature re-opens it). That's expected — re-review sessions + acceptance when their prerequisite stories land.

## Things to watch for during authority implementation
- The `GrantCheck::check` signature change (`&ActorEndpointRef` → `&dyn IssuerContext`) is a port-shape change touching acceptance's `submit`. The acceptance prerequisite story owns the call-site update; coordinate so the trait lands first (or same wave).
- `story-v0-core-authority-grant-check` defines the `IssuerContext` trait that `story-acceptance-issuer-context` imports. Explicit edge, not "co-developed" — the trait lands first.
- The spawn-tail's 6-permutation order test is the key correctness assertion (order-independence).
- `spawn_revocation_does_not_cascade` (proptest #7) is the executable stand-in for a demoted formal property — must be mutation-survivable.

## Git log (this session, most recent first)
```
4396792 review: authority design rev3 -> implementing (5 findings resolved in-stride)
b86ed43 feature-design: feature-v0-core-authority revision 3 (component-complete, not live)
b642208 review: authority design rev2 -> drafting (4 blockers partially-resolved + 8 new)
520e4ff feature-design: feature-v0-core-authority revision 2 (vertical slice, 6 stories + 2 prerequisites)
48f447e review: authority design -> drafting (10 blockers, request changes)
fa558e9 feature-design: feature-v0-core-authority (6 child stories)  [rev 1]
f843dc0 review: feature-v0-core-sessions -> done (all 5 blockers resolved)
... (sessions implementation + fix arc)
8696777 feature-design: feature-v0-core-sessions (5 child stories)
dbb1f08 session note: acceptance arc — command lifecycle + acceptance pipeline done
```
