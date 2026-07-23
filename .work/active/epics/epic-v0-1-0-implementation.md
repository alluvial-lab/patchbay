---
id: epic-v0-1-0-implementation
kind: epic
stage: implementing
tags: [foundation, protocol, verification]
depends_on: [epic-foundation-hardening]
parent: null
created: 2026-07-11
updated: 2026-07-16
gate_origin: null
release_binding: null
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
