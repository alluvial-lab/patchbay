---
id: epic-v0-1-0-implementation
kind: epic
stage: done
tags: [foundation, protocol, verification]
depends_on: [epic-foundation-hardening]
parent: null
created: 2026-07-11
updated: 2026-07-16
gate_origin: null
release_binding: v0.1.0
---

# Epic: v0.1.0 implementation

## Brief

The v0.1.0 walking skeleton is fully designed but entirely unbuilt. The `epic-foundation-hardening` design arc produced the foundation docs (VISION, SPEC, ARCHITECTURE, PROTOCOL, SECURITY, VERIFICATION, UX, GLOSSARY, ADAPTER-PI), generated Protobuf contracts (Rust + TS bindings), formal models (8 promoted / 39 stated-normative properties), and conformance vectors. No application code exists yet — the only Rust and TypeScript in the repo are generated protobuf bindings.

This epic implements the first executable Patchbay milestone: one operator controls Pi-backed sessions through a responsive web cockpit and diagnostic CLI, proving the durable control loop and getting the initial operator operational. The foundation docs, generated contracts, and formal models are the inputs; running code is the output.

The v0.1.0 scope is defined in `docs/SPEC.md` § "v0.1.0 walking skeleton": one operator, one authoritative coordination core, local durable persistence behind ports, Pi adapter first, responsive web cockpit + CLI, no native mobile / HA / multi-operator / leases. The architecture is defined in `docs/ARCHITECTURE.md` § "v0.1.0 component slice" and "v0.1.0 process topology": a Rust coordination core (single authoritative writer) plus a TypeScript web server (HTTP-terminating control surface), with the Pi adapter as the only required runtime adapter.

## Why this is epic-sized

This epic turns a complete design into a running system across six layers (coordination core, internal protocol seam, Pi adapter, web server, web cockpit, CLI). It spans two languages (Rust, TypeScript), introduces the first application code in the repo, and must satisfy the formal-model-backed safety properties while remaining usable enough to replace the operator's current remote-pi workflow. The coordination core alone is the largest piece and may warrant its own decomposition during feature-design.

## Critical path

```
epic-v0-core  (root — nothing starts until the core exists)
    │
    ├── feature-v0-protocol-seam  [depends: core]
    │       │
    │       ├── feature-v0-web-server  [depends: seam]
    │       │       │
    │       │       └── feature-v0-web-cockpit  [depends: web-server]
    │       │
    │       └── feature-v0-cli  [depends: seam]
    │
    └── feature-v0-pi-adapter  [depends: core]
```

- **Phone-usable path:** core → protocol-seam → web-server → web-cockpit
- **Agent-control path:** core → pi-adapter (parallel with the web chain after core lands)
- **CLI:** side branch off the protocol seam

Parallel work within a layer and across independent branches is handled by `implement-orchestrator` wave dispatch; the depends_on graph above is the structural ordering, not a serialization constraint.

## Relationship to v1.0.0 work

`epic-public-product-contract` (v1.0.0 public-product design) is sidelined pending this epic. Its remaining child features carry `epic-v0-1-0-implementation` in their `depends_on` so the substrate honestly reflects that the v1.0.0 public product cannot ship without v0.1.0 built. The v1.0.0 design work that already landed (verification-claim-correction) is preserved; only the unbuilt v1.0.0 features are blocked.

## Foundation references

- `docs/SPEC.md` — v0.1.0 walking skeleton scope and exclusions
- `docs/ARCHITECTURE.md` — v0.1.0 component slice, process topology, persistence topology
- `docs/PROTOCOL.md` — canonical state registries, acceptance semantics, idempotency, snapshots, authority
- `docs/SECURITY.md` — threat model, grants, audit
- `docs/VERIFICATION.md` — property-graded assurance, 8 promoted / 39 stated-normative
- `docs/UX.md` — surface-neutral conformance floor, v0 web cockpit instance
- `docs/ADAPTER-PI.md` — Pi parity checklist, session_new = generation bump, snapshot tier = partial
- `contracts/proto/patchbay/*.proto` — generated contract source (7 proto packages)
- `contracts/rust/`, `contracts/ts/` — generated Rust + TS bindings
- Formal models in `contracts/` — `command_lifecycle.qnt`, `session_generation.qnt`, `csrf_browser.qnt`, `elicitation_lifecycle.qnt`, `authority.qnt`, `patchbay-relational.als`

## Decomposition

Six child features, one per architectural layer. The coordination core is the largest and may decompose further during `feature-design`; the others are feature-sized.

### Child features

- `epic-v0-core` — Rust coordination core: durable event log, storage port, operation acceptance + idempotency, authority checks, snapshots, crash recovery — depends on: `[]` (epic-sized; decomposes into child features via `epic-design`)
- `feature-v0-protocol-seam` — web↔core internal protocol seam: internal RPC, streaming channel, auth boundary — depends on: `[epic-v0-core]`
- `feature-v0-pi-adapter` — Pi adapter: session discovery, prompt delivery, cancel/interrupt, replies/events/snapshots — depends on: `[epic-v0-core]`
- `feature-v0-web-server` — TS web server: HTTP termination, operator sessions, CSRF, speaks Connect to core — depends on: `[feature-v0-protocol-seam]`
- `feature-v0-web-cockpit` — responsive web cockpit: session list, composer, delivery states, reconnect — depends on: `[feature-v0-web-server]`
- `feature-v0-cli` — CLI: setup, admin, debug, scripted access — depends on: `[feature-v0-protocol-seam]`

### Child status (current, 2026-07-23 — roll-forward)

The decomposition above lists 6 children and predates the realized decomposition. The actual child set is 9 features + the core epic, **all done**:

- `epic-v0-core` — **done** (durable event log, storage port, acceptance + idempotency, authority, snapshots, recovery)
- `feature-v0-protocol-seam` — **done**
- `feature-v0-web-server` — **done**
- `feature-v0-web-cockpit` — **done** (standard review, 6 blockers fixed + mutation-verified)
- `feature-v0-presentation-component-layer` — **done** (thorough, 7-pass convergence)
- `feature-v0-elicitation-response-contract` — **done**
- `feature-v0-approval-response-contract` — **done** (DENIED→Declined, mutation-verified)
- `feature-v0-control-surface-trust-boundary` — **done** (thorough, 3 security blockers fixed + mutation-verified)
- `feature-v0-pi-adapter` — **done**
- `feature-v0-cli` — **done** (standard review, 2 blockers fixed + mutation-verified)

## Review outcome (2026-07-23) — Pass 1 (maximum/complementary): Request changes → implementing

Maximum-weight epic aggregate review, pass 1 (complementary/completeness, fresh-context `kimi-coding/k3` — cross-model vs the gpt-5.6-sol implementation workers). Verdict: **Request changes** — 4 receiver-confirmed aggregate blockers + 5 important findings, every one invisible to per-feature review because all suites test pairwise seams, not the composed system. The per-feature "earned not asserted" properties are genuinely real (mutation-verified); the composed walking skeleton was broken in the operator's daily-use paths. Epic bounced `review → implementing` for the corrective wave.

### Blockers (all verified by the orchestrator against code)

1. **B1 — pi-adapter e2e RED at HEAD** (`pi-adapter/tests/e2e.test.ts`): the trust-boundary change made verified principals mandatory; the e2e fixture still self-asserts headers + seeds grants via raw SQLite. `npm test` → 5/6, `[unauthenticated] missing transport principal`. Nothing caught it (no CI). Fix: rewrite the fixture through the real flow (BootstrapOperator → VerifyOperatorPassword → enrolled principal + core session); grants via the bootstrap RPC, not SQLite.
2. **B2 — `LoadSnapshot` has no producer** (`core/src/acceptance/replay.rs:80-89`): the materializer is deferred, so `LoadSnapshot` returns `present: false` against any real deployment — silently killing CLI `session-health`/`instruct` target resolution and the cockpit reconcile. Fix: materialize `SessionSnapshot` on read from the rebuilt projection at current LSN (no `write_snapshot`; durable checkpoint stays deferred).
3. **B3 — cockpit treats the seam's normal stream completion as an error** (`web-cockpit/src/domain/reconcile.ts:90`): throws `"subscription stream ended"`, permanently degrades the model, locks the composer (stableTarget requires `reconciled`). Fix: re-subscribe on clean completion without degrading; degrade only on transport errors/gaps.
4. **B4 — authority-domain defaults disagree** (`web-cockpit/index.html:6` hard-codes `operator-domain`; core + CLI default from env): out-of-the-box the cockpit is 100% rejected, no config path, no docs. Fix: web-server takes domain from env + serves it to the cockpit; align all three defaults; document.

### Important (confirmed; fix wave inclusion per operator decision)

- **I1 — core-diagnostics scope:** OPERATOR DECISION (2026-07-23): **ship the honest partial; re-scope foundation docs before tagging.** Reclassify `audit-query`/`inspect-command`/`adapter-status` + the durable queryable audit log as reserved/post-v0.1.0 in SPEC/PROTOCOL/UX/SECURITY. Core-diagnostics parks as the first v0.x fast-follower.
- **I2 — no TS suite in CI:** add the 4 npm suites (why B1 shipped undetected).
- **I3 — no browser login UI:** OPERATOR DECISION: **include in the fix wave** — a minimal login view posting to the existing `/login` API. The flagship surface cannot be authenticated through the product without it.
- **I4 — foundation/README roll-forward:** README "Current status" claims no daemon/web app/adapter exists (false); SECURITY §143 stale "deferred" wording. Folded into the fix wave.
- **I5 — no composed end-to-end proof + no runbook:** OPERATOR DECISION: **include in the fix wave** — a composed walking-skeleton smoke (boots the topology, drives one instruction from a surface to a Pi session and back) + a runbook. Subsumes B1's fixture pattern + B4's docs; evidences ADAPTER-PI §8 migration criteria.

### Nits (noted, not blocking)
- N1 — epic body child list stale (rolled forward above).
- N2 — fleet-scoped `spawn` can rot in `accepted` forever (no adapter delivery for fleet targets; unreachable via shipped surfaces today — note for the spawn fast-follower).
- N3 — pi-adapter e2e grant-seeding bypasses the ingest path (subsumed by B1).

### Pass 2 (adversarial) pending

After the corrective wave lands, pass 2 (adversarial attack, `gpt-5.6-sol` — different model class) runs against the fixed state per maximum ordering, then convergence until a pass yields no receiver-confirmed material current-cycle blockers.

### Fix wave (2026-07-23) — all blockers + important findings addressed

Two parallel fix workers (disjoint write sets; orchestrator verified + committed) landed the full corrective wave in 9 commits:

- `428c219` **B2** — `LoadSnapshot` materializes on read from the rebuilt projection (durable checkpoint stays deferred). Unblocks CLI `session-health`/`instruct` + cockpit reconcile. Orchestrator reconciled the web-server core-smoke's stale `present:false` assertion (cross-boundary seam neither worker could touch).
- `80c76d0` **B1** — pi-adapter e2e authenticates through the real trust boundary (loopback bootstrap → VerifyPassword → enrolled principal + core session; no SQLite grant seeding). 6/6 green.
- `29fb9f9` **B3** — cockpit treats clean subscription completion as the normal polling boundary (re-subscribe without degrading; degrade only on transport errors/gaps). Mutation-verified property tests re-verified; new completion regression test.
- `5f4690e` **B4** — web-server reads `PATCHBAY_AUTHORITY_DOMAIN_ID` (default `default`, aligned with core/CLI) and templates it into the served cockpit HTML.
- `c08f963` **I3** — cockpit operator login view (401 → login form → existing `/login` API → startup continues).
- `cadddfe` **I2** — CI runs all 4 TS suites (web-server, web-cockpit, cli, pi-adapter) + core build.
- `6c1d748` **I5** — composed walking-skeleton e2e: core + Pi adapter + CLI, bootstrap → login → instruct → live/working → completed/idle. Passes.
- `6db620f` **I4** — README + foundation docs rolled forward to the honest v0.1.0 partial (operator-confirmed: audit-query/inspect-command/adapter-status + durable audit log reclassified reserved/post-v0.1.0; session-health committed; SECURITY §143 deferral resolved).
- `a8f6b2e` **I5-runbook** — `docs/RUNBOOK.md` (startup order, env matrix, bootstrap flow, commands, verification).

Final integration verification (orchestrator, post-wave): Rust workspace all suites green; pi-adapter 6/6; web-cockpit 38/38; web-server 21/21 + core-smoke; cli 16/16 + core-smoke; composed e2e passes; contracts vectors + presentation + models green.

Epic re-advanced `implementing → review` for pass 2 (adversarial).

## Review outcome — Pass 2 (2026-07-23, adversarial, gpt-5.6-sol): Request changes → implementing

Maximum-weight pass 2 (adversarial attack, fresh-context `gpt-5.6-sol` — cross-model vs the K3 complementary reviewer). Verdict: **Request changes** — 4 receiver-confirmed blockers (all verified by the orchestrator against code), 3 important findings, 1 nit, and a clean list of rejected attacks (the credential-binding, loopback-bypass, HTML-injection, throttle-enumeration, and future-kind-leak attacks all failed — the boundary held). Epic bounced `review → implementing` for the corrective wave.

### Blockers (verified by orchestrator)

1. **B1 — trust-boundary filter + B2/B3 fixes wedge the cockpit** (`server/src/service.rs:207` filter × `web-cockpit/src/domain/reconcile.ts:131`): the filter intentionally keeps dropped auth records' LSNs as cursor gaps; the cockpit treats every gap as loss, demands a bounded snapshot, B2's current materialized snapshot always exceeds the bound → rejected → replay hits the same holes → permanently unreconciled → composer locked. Fix (cockpit-local): on gap, adopt the current materialized snapshot (authoritative sessions) + replay the visible prefix (visible kinds are never filtered) + advance cursor.
2. **B2 — stale adapter generation stays authenticated** (`server/src/adapter_service.rs:61`): `verify_request` checks only the shared attachment secret + caller-supplied adapter id — no binding to the registered generation. The `stale adapter generation` error variant exists (`core/src/adapter/mod.rs:388`) but is never enforced in the auth path. Fix: bind requests to the registered generation; reject stale.
3. **B3a — delivery-ack ordering rots commands at `delivered`** (`core/src/adapter/mod.rs:278` × `server/src/adapter_service.rs:333`): the ack commits `accepted→delivered` before the RPC completes; `ReceiveDeliveries` offers only `accepted`. A crash between commit and response leaves the command `delivered`-never-executed with no redelivery. OPERATOR DECISION (Q1a): redelivery also offers `delivered`-but-not-`running` (bounded; adapter re-polls and re-executes idempotently).
4. **B3b — no adapter-liveness mechanism** (adapter death → core presents `live/working` forever; stale-never-live violated). OPERATOR DECISION (Q2a): fix now via the connection-liveness signal — mark the adapter's sessions stale when its gRPC stream drops; refresh on re-attach.
5. **B4 — non-loopback plaintext core transport** (`server/src/main.rs:37`): the general listener allows non-loopback binds serving h2c — every bearer trust root crosses plaintext. OPERATOR DECISION (Q3a): constrain the general listener to loopback in v0.1.0 (refuse non-loopback binds, like the admin listener); split-deploy + TLS is a documented fast-follower.

### Important (fix wave / parked)

- **I1 — revocation/authorization lifecycle partially exposed** (`control.proto:20`, `core/src/authority/state.rs:45`): only current-session revocation; no revoke-all/endpoint/device/grant revocation, no lockdown surface, grant expiration ignored, no Subscribe grant check. Exploitability limited in single-operator (one permanent bootstrap grant, no public grant-admin path). **Parked** as a fast-follower backlog item (security surface, not a single-operator-exploitable defect today).
- **I2 — forged sender identity persists** (`core/src/acceptance/pipeline.rs:39`): `Operation.sender` retained as audit data, not normalized to the verified issuer. Fix in wave: core normalizes sender from the verified issuer (mirrors the web-server's overwrite).
- **I3 — core restart strands browser sessions** (`web-server/src/routes/login.ts:97`): web-server session survives while the core session dies; cockpit doesn't show login and logout also fails. Fix in wave: web-server invalidates its session on dead-core-session RPC failure; logout clears the local session even if core revocation fails.

### Nit
- Login view submits an actor ID the route ignores (misleading field) — folded into the I3 login fix.

### Fix wave 2 (2026-07-23) — all pass-2 blockers + important findings addressed

Three fix workers (disjoint write sets; orchestrator verified + committed; the B2 worker required a coordinated server + pi-adapter scope after the first worker correctly surfaced the cross-boundary blocker). 6 commits:

- `ea47e9b` **B1** — cockpit reconciler tolerates the filter's intentional cursor gaps (adopts current snapshot, replays visible prefix, unlocks composer). Mutation tests re-verified.
- `a8d9c37` **I3** — web-server invalidates stranded browser sessions on dead-core-session RPC errors; fail-safe logout; login nit removed.
- `af00794` **B3a+B3b** — redelivery offers `delivered`-not-`running` (idempotent re-ack); abnormal stream disconnects mark adapter sessions stale (clean completion does not). Mutation checks verified.
- `880fd9d` **B4** — general listener loopback-constrained (refuses non-loopback; split-deploy + TLS documented as future).
- `0ae9661` **I2** — `Operation.sender` normalized from the verified issuer at Submit.
- `45c7375` **B2** — per-attachment fencing token (operator decision: option 2). Attach mints a CSPRNG bearer token, stores only its SHA-256 hash in memory, invalidates the prior attachment's token on re-attach, returns the bearer via response metadata (no proto change). Post-attach RPCs require it; stale/missing rejected. Fail-closed on core restart (adapter re-attaches). The pi-adapter captures/sends/re-attaches. The generation boundary is genuinely enforced, not asserted. Mutation check verified.
- `9cb3ca4` — parked I1 (revocation-lifecycle surface) as a fast-follower backlog item.

Integration verification (orchestrator, post-wave): Rust workspace all suites green + clippy clean; pi-adapter 6/6; web-cockpit 39/39; web-server 23/23 + core-smoke; cli 16/16 + core-smoke; composed e2e passes; presentation conformance green.

Epic re-advanced `implementing → review` for pass 3 (re-review, maximum convergence).

## Review closure — Pass 3 (2026-07-23, convergence, kimi-coding/k3): Approve with comments → done

Maximum-weight pass 3 (convergence re-review, fresh-context `kimi-coding/k3` — cross-model vs the gpt-5.6-sol implementers and the pass-2 adversarial reviewer). Verdict: **Approve with comments** — the convergence signal.

**All 7 pass-2 fixes verified genuine, not cosmetic.** Every attack rejected with evidence (three focused probe tests written against the live code): B1 does not mask real visible-event loss (the snapshot + full-prefix replay rebuilds everything; cursor advance honest); B2's double-attach race resolves to exactly one valid token inside one lock critical section; B2 token replay/missing/restart all rejected by existing tests; B3a duplicate-execution impossible by construction + test (exactly 1 CommandTransition); B3b clean-vs-abnormal correctly distinguished (epoch guard supersedes obsolete streams); B4 loopback matrix holds (0.0.0.0/::/public refused, hostname fails closed); I2 normalization is the sole ingress path and strengthens downstream; I3 neither over- nor under-invalidates. **0 new blockers.**

### Adjudication

- **P3-I1 (Important, parked):** the B3b staleness signal only fires during an active stream drain; the v0.1.0 polling fallback completes streams in milliseconds and execution happens after completion, so adapter death between polls or mid-execution leaves sessions presented `live/working` until restart. Verified by the receiver against `pi-adapter/src/main.ts:110-121` + `server/src/adapter_service.rs:500`. Receiver judgment: Important-not-Blocker (mechanism genuine; commands never lost via B3a; replacement-process confusion fenced via B2; presentational residual in single-operator deployment; natural recovery works). Disposition per the reviewer's recommendation: documented honestly in `docs/RUNBOOK.md` (correcting the `af00794` overclaim) + parked as `backlog-adapter-staleness-full-coverage` (heartbeat/last-report-age, or long-poll redesign). Commit `bf7c47f`.
- **P3-N1 (nit):** commands rot at `running` after mid-execution death — the documented Q1a bound; folded into the same backlog item.
- **P3-N2 (nit):** per-poll full-log command rebuild — perf note for when the log grows; folded into the backlog item.
- **P3-N3 (nit):** `validate_operation` still requires a caller-supplied `sender` that normalization overwrites — vestigial; noted.

### Convergence

Pass 1 found 4 blockers, pass 2 found 4 blockers + 3 important, pass 3 finds 0 blockers + 1 important (a coverage limitation of an operator-decided mechanism, the fix itself verified genuine). Per the maximum-weight convergence rule — a pass with no receiver-confirmed material current-cycle blockers — the epic is closed. **Epic advanced `review → done`.**

### What the maximum review earned

The two-lens, multi-model maximum review is what caught the class of defect per-feature review could not: pass 1 (complementary, K3) found the composed-system breaks (no snapshot producer, cockpit deadlock on normal completion, domain mismatch, red e2e at HEAD); pass 2 (adversarial, gpt-5.6-sol) found the security/recoverability gaps (filter×reconcile wedge, stale adapter generation, delivery-ack rot, no liveness mechanism, plaintext transport); pass 3 (K3) verified every fix genuine and found none remaining. Every fix wave was mutation-verified. The milestone's claims are now earned at epic scale, not just per-feature.

## Stage correction (2026-07-14)

Advanced `drafting → implementing`. The decomposition was settled (complete child list + critical-path diagram + depends_on graph) but the `epic-design` Phase 8 stage advance was never applied — a stale stage, not a deliberate hold. Per the `epic-design` skill, an epic advances to `implementing` once its decomposition is done, not once all children are done.

### Child status (2026-07-14, historical — superseded by the current roll-forward above)
- `epic-v0-core` — **done** (root of the critical path; completed this session)
- `feature-v0-protocol-seam` — **done** (the web↔core gRPC seam; transport caveat retired by interop spike, then designed + implemented + reviewed this arc)
- `feature-v0-pi-adapter` — **done** (Pi adapter: in-process `AgentSession` host + harvested session layer + adapter-facing core RPC surface; delivery lifecycle, reconnect w/ partial-snapshot reconcile, session_new replacement, spawn-foreclosure via session registry all verified; `spawn` is the fast-follower)
- `feature-v0-web-server` — **done** (TS web server: HTTP termination, operator sessions, CSRF synchronizer-token, Connect-Web bridge to core; the 4 csrf_browser.qnt properties verified at the HTTP boundary; login throttled per SECURITY.md:85)
- `feature-v0-web-cockpit` — drafting (now unblocked: depends on the done web-server; the last phone-usable layer)
- `feature-v0-cli` — drafting (now unblocked: depends on the done seam)

The epic is ~4/6 built by layer (core + seam + web-server + pi-adapter). The agent-control path (core → pi-adapter) is complete. The web cockpit (last phone-usable layer) and the CLI remain unblocked.
