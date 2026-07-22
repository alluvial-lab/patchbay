# Session note — 2026-07-22 (CLI done; all v0.1.0 features done; epic review-ready)

A durable handoff note for the next session. Read this before continuing.

## What happened this session

The CLI (`feature-v0-cli`) — the last v0.1.0 implementation layer — went from
`drafting` (blocked on the trust boundary) through design refresh → implement →
review → fix → done. With the CLI done, **all 9 epic child features are done**;
`epic-v0-1-0-implementation` is ready for its deeper aggregate review.

### 1. CLI design refresh (commit `14d3f76`)

Refreshed Units 1 + 2 against the now-shipped trust-boundary surface (the trust
boundary landed the prior session). The CLI consumes the actual shipped RPCs:
`AdminService.BootstrapOperator` (local listener) for `setup`;
`ControlService.VerifyOperatorPassword` (throttled, core does scrypt) for
`login`; `EnrollControlSurfacePrincipal` + `RevokeOperatorSession` for
enrollment/logout. The credential store holds the `PrincipalCredential`
(principal_id + secret) + `OperatorSessionId`, written 0600. The auth
interceptor adds all four verified headers the shipped `MetadataIssuerContext`
verifies. Stage → implementing.

### 2. CLI implementation (commit `b67d9d2`)

One feature-owning worker (`gpt-5.6-sol`, high). Delivered the full `cli/`
package: scaffold + control/admin clients + auth interceptor + 0600 credential
store; setup/login/logout; session-health (via LoadSnapshot); scripting commands
(instruct/cancel/interrupt with identity-before-intent + idempotency + exit
codes); the three Unit 3b deep-debug commands stubbed honestly. 15 tests + a
real-core smoke (setup → login → authenticated LoadSnapshot → logout/rejection).

### 3. Standard review — Request changes, 2 blockers (commit `6bd7002`)

Fresh-context `gpt-5.6-sol` review. Confirmed the functional verification
passes + the auth-header completeness is genuinely earned (the real-core smoke
proves the four headers — a mutation omitting any fails the smoke). Correctly
rejected two would-be findings (the auth-header unit test is earned by the
smoke; the Unit 3b stubs are honest, not contradictory). But returned Request
changes with 2 receiver-confirmed material blockers:
1. **Identity-before-intent test is self-defining** — only checked
   `startsWith("adapter=pi-adapter")`; a mutation reducing the identity to
   adapter+generation would pass.
2. **Credential writes chmod an arbitrary parent directory to 0700** —
   `credentials.ts:62-64` unconditionally `chmod(dirname, 0o700)`; for
   `/tmp/x.json` that would chmod `/tmp`.

### 4. Fix stride + receiver closure → done (commits `3cec5db`, `53e701b`)

Focused fix worker addressed both. The receiver independently mutation-
verified:
- Blocker 1: mutate `canonicalSessionIdentity` to drop scope+runtime → the
  tightened identity test FAILED. Earned.
- Blocker 2: revert to unconditional parent chmod → the parent-mode-preservation
  regression FAILED. Earned.

Standard converged: no receiver-confirmed material blockers. Feature → done.

## Where we are now

`epic-v0-1-0-implementation` — **9/9 child features done**. The epic is at
`stage: implementing` but is ready to advance to `review` for its deeper
aggregate review (per the review skill: when all child features are done,
advance the epic and run its broader aggregate lane). This session did NOT
advance the epic — that's the next step.

### The 9 done features

- `feature-v0-protocol-seam` — gRPC ControlService + compound-issuer wire shape
- `feature-v0-web-server` — Fastify HTTP termination + sessions/CSRF + gRPC-Web bridge
- `feature-v0-web-cockpit` — responsive browser cockpit (the product center)
- `feature-v0-presentation-component-layer` — machine-checked conformance floor
- `feature-v0-elicitation-response-contract` — typed question contracts
- `feature-v0-approval-response-contract` — typed approval decisions (DENIED→Declined)
- `feature-v0-control-surface-trust-boundary` — real transport-principal verification + bootstrap
- `feature-v0-pi-adapter` — Pi adapter (the migration target)
- `feature-v0-cli` — the operator's CLI (bootstrap channel + scripting + session-health)

## What's next: the epic review

`epic-v0-1-0-implementation` → `review` for its deeper aggregate review. Per the
review skill, epic review does NOT repeat line-level child-feature review — it
inspects end-to-end capability completeness, cross-feature contracts, cumulative
foundation-doc alignment, operational/release interactions, and risks that only
appear at the larger boundary. Two rolling-foundation doc-drift items to fold in
during the epic review (both surfaced as non-blockers in their features):
- `docs/SECURITY.md` §143 still says the operator-session wire shape is
  "deferred" — but the trust-boundary feature now implements it.
- The cockpit's Q2 delivery-trace scope was reduced to a reserved seam; SPEC.md
  is the source of truth and is unchanged (no drift there, just a note).

The CLI's three deep-debug commands (audit-query/inspect-command/adapter-status)
remain stubbed pending a separate `feature-v0-core-diagnostics` (or similar)
that builds the `QuerySpec`/`QueryResult`/`AuditRecord` schema + the core's
query-result projection + audit-log storage. PROTOCOL.md:623 commits these to
v0.1.0 observability. The epic review should decide whether v0.1.0 ships
without them (honest partial) or whether core-diagnostics is a v0.1.0 blocker.

## Commits this session

```
53e701b review: feature-v0-cli (Approve — 2 blockers mutation-verified; -> done)
3cec5db review-fix: feature-v0-cli (2 standard-pass blockers; -> review)
6bd7002 review: feature-v0-cli (Request changes -> implementing; 2 blockers)
b67d9d2 implement: feature-v0-cli
14d3f76 feature-design: feature-v0-cli (refresh auth units against shipped trust boundary)
f18ff4a session-note: 2026-07-21 trust-boundary done; CLI unblocked
```

## Notes for the next session

- The CLI is the last v0.1.0 implementation layer; all 9 child features are
  done. The epic is ready for its deeper aggregate review.
- Three operator decisions recorded across the CLI + trust-boundary arcs —
  do NOT re-litigate:
  - **CLI auth (option 1):** CLI is a full transport principal with its own
    operator-session bootstrap.
  - **Trust boundary D1 (option 1):** core as source-of-truth for the operator
    record.
  - **Trust boundary D2 (option 1):** dedicated local-console listener for
    bootstrap.
- The trust-boundary security properties (reject self-asserted identity; no
  password-hash leak via Subscribe; throttle before scrypt) AND the CLI's
  claimed properties (auth header completeness via real-core smoke; 0600
  credential store + bearer-secret-never-logged; identity-before-intent exact
  full tuple; exit-code mapping) are genuinely earned — each has a mutation
  test that fails on the mutated implementation. The "earned not asserted"
  standard now spans the whole v0.1.0 control plane.
- The CLI's three deep-debug commands are stubbed pending
  `feature-v0-core-diagnostics`. The epic review should decide v0.1.0 scope
  for them.
- Two rolling-foundation doc-drift notes for the epic review (both non-blockers
  in their features): SECURITY §143 deferral wording; the cockpit Q2 trace
  reduction (SPEC unchanged).
- `.work/bin/work-view` still hangs on `board`/`--blocking`. Verify state via
  `grep ^stage:`.
- `check:drift` is the known broken repo gap (needs `protoc-gen-prost`); not
  in CI.
- Regen procedure (pre-existing): `buf generate` for TS (canonical),
  `git checkout -- contracts/rust/src/gen` to discard buf's wrong Rust output,
  then `cargo build` to regen Rust via the crate's `build.rs`/prost-build.
