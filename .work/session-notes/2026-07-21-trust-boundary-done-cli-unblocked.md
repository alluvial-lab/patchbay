# Session note — 2026-07-21 (trust-boundary built; CLI unblocked)

A durable handoff note for the next session. Read this before continuing.

## What happened this session

The session started by kicking off the CLI design (the last v0.1.0 layer).
The CLI design surfaced a genuine semantic 50/50 (CLI auth posture) → resolved
option 1 (CLI as a full transport principal). Implementation then surfaced a
deeper prerequisite: the resolved auth posture is not realizable against the
shipped core boundary (no bootstrap RPC, web-server env-only operator record,
core hard-codes the endpoint to the web-server and accepts any operator-id).
Operator chose option 1 at the epic level: scope a new prerequisite feature
(`feature-v0-control-surface-trust-boundary`) that builds the real trust
boundary the docs promise. That feature was designed, implemented, thoroughly
reviewed (3 security blockers found), fixed, and closed → done. The CLI is now
unblocked.

### 1. CLI design (commits `8ca0359`, `c40d1f4`)

`feature-v0-cli` designed: Node TS CLI speaking gRPC directly to the core,
reusing the web-server's `core-client.ts` pattern. 4 units (scaffold+auth,
setup+login, diagnostic commands, scripting commands). Two design decisions:
- **Auth posture (option 1):** CLI runs its own operator-session bootstrap;
  full transport principal; forwards the same headers as the web-server.
- **Scope (option 1):** ship what's buildable now. `session-health` (via
  LoadSnapshot) + setup/login + scripting commands. The three deep-debug
  commands (audit-query/inspect-command/adapter-status) are BLOCKED on a
  core-diagnostics prerequisite (no QuerySpec/QueryResult/AuditRecord schema,
  no query handler, no audit-log storage) — stubbed honestly, not dropped.

### 2. CLI implementation → deeper blocker (commit `4da38dd`)

The implementation worker correctly stopped (returned the feature to
`drafting`, no `cli/` files created) when it found the resolved option-1 auth
posture could not be realized against the shipped core boundary. Three verified
findings:
- No bootstrap/grant-admin RPC on `ControlService` (only Submit/Subscribe/
  LoadSnapshot; the first grant can't be created by an Operation —
  chicken-and-egg; the core has `ingest_grant` internally but doesn't expose
  it).
- The web-server reads the operator record only from env at startup
  (`PATCHBAY_OPERATOR_ID`/`PATCHBAY_OPERATOR_PASSWORD_HASH`); a CLI-created
  record would not be consumed.
- `server/src/issuer.rs` hard-codes `WEB_SERVER_ENDPOINT_ID =
  "patchbay-web-server"`; `MetadataIssuerContext::from_request` accepts any
  non-empty operator-id/session-id without verification, returns `None` for
  device/endpoint-generation. A direct CLI request cannot be represented as
  its own full transport principal.

### 3. Trust-boundary feature scoped + designed (commits `9d844db`, `8abfcfc`, `8d8d636`)

`feature-v0-control-surface-trust-boundary` (deps: protocol-seam +
core-authority + web-server, all done). Grounding found good news: the core
already has `ingest_grant` (just not exposed) and `MetadataIssuerContext`
already extracts metadata (just doesn't verify). Two design decisions:
- **D1 (option 1):** core as source-of-truth for the operator record, read via
  RPC. Single source of truth; env-only becomes fallback.
- **D2 (option 1):** dedicated local-console RPC on a local-only listener
  (loopback). Strongest channel separation per SECURITY:208 (lockdown-exit
  channel distinct from routine web login).

### 4. Trust-boundary implementation (commit `7a81e99`)

One feature-owning worker (`gpt-5.6-sol`, xhigh). Delivered: durable operator +
credential-hashed principal records; loopback-only `AdminService.BootstrapOperator`
with expiring one-use setup secret; exposed `ingest_grant`; real compound-issuer
verification rejecting self-asserted identities; core-owned scrypt password
verification + principal enrollment RPCs; web-server login consumes the core
record. 4 trust_boundary tests + the web-server's 20/20 (4 csrf properties
hold). The orchestrator independently mutation-verified the self-asserted-
identity rejection (bypassing `verify_principal` fails the test).

### 5. Thorough review — Request changes, 3 blockers (commit `8aa16c5`)

Fresh-context `gpt-5.6-sol` thorough review. Confirmed the credential
verification is load-bearing (mutation fails) but found 3 material security
gaps:
1. **Operator-session evidence remains self-asserted** — `issuer.rs:40` just
   requires a non-empty session header; no session record/expiry/revocation/
   binding. Contradicts SECURITY:143.
2. **The generic subscription leaks the stored password verifier** —
   `OperatorRecord` (with `password_hash`) is a `StoredEventKind` in the event
   log; `Subscribe` returns every raw payload to any verified principal.
   Offline-guessing exposure. Contradicts the feature's own "never returns the
   password hash" claim.
3. **Network `VerifyOperatorPassword` lacks throttle + audit** — unrestricted
   scrypt after only the core-secret interceptor; direct callers bypass the
   web-server's throttle (SECURITY:85) and no audit (SECURITY:220-221).

### 6. Fix stride + receiver closure → done (commits `f37ad96`, `bb7d073`)

Focused fix worker addressed all 3: core-issued expiring/revocable actor-bound
operator sessions; authentication/authority records filtered from `Subscribe`;
core RPC account/network throttling + concurrency cap + decay + redacted audit
lines. The receiver independently mutation-verified all three:
- Blocker 1: bypass `verify_operator_session` → 4 tests FAILED. Earned.
- Blocker 2: re-include `OperatorRecord` in Subscribe →
  `subscribe_excludes_authentication_and_authority_records` FAILED. Earned.
- Blocker 3: disable the limiter block predicates →
  `password_rpc_throttles_before_a_correct_password_and_recovers_after_decay`
  FAILED. Earned.

Thorough converged: no receiver-confirmed material blockers. Feature → done.

## Where we are now

`epic-v0-1-0-implementation` — **8/9 features done** + `feature-v0-cli` at
`drafting` (now unblocked). The trust boundary the docs promise is genuinely
enforced (mutation-tested), not asserted. The CLI is the last v0.1.0 layer.

## What's next: implement the CLI, then the epic review

`feature-v0-cli` (`stage: drafting`, depends on `feature-v0-protocol-seam`
[done] + `feature-v0-control-surface-trust-boundary` [done]) is now
unblocked. Its resolved option-1 auth posture is realizable: the CLI's `login`
obtains a core-issued session token; its `setup` uses the local-console
bootstrap; its credential store holds the principal credential + session token.
Re-run `feature-design` to refresh the design against the now-shipped trust
boundary (the auth unit's details changed — core-issued sessions, principal
enrollment RPC), then implement. The three deep-debug commands (Unit 3b)
remain stubbed pending a separate core-diagnostics feature.

After the CLI lands, all epic children are done and the epic is ready for its
deeper aggregate review.

## Commits this session

```
bb7d073 review: feature-v0-control-surface-trust-boundary (Approve — thorough converged; -> done)
f37ad96 review-fix: feature-v0-control-surface-trust-boundary (3 thorough-pass blockers; -> review)
8aa16c5 review: feature-v0-control-surface-trust-boundary (Request changes -> implementing; 3 blockers)
7a81e99 implement: feature-v0-control-surface-trust-boundary
8d8d636 feature-design: feature-v0-control-surface-trust-boundary (D1+D2 resolved; -> implementing)
8abfcfc feature-design: feature-v0-control-surface-trust-boundary (grounding + 2 decisions surfaced)
9d844db scope: feature-v0-control-surface-trust-boundary (prerequisite for the CLI; option 1)
4da38dd implementation-discovery: feature-v0-cli bootstrap prerequisite
c40d1f4 feature-design: feature-v0-cli (scope decision — option 1; split diagnostic commands)
8ca0359 feature-design: feature-v0-cli (blocker resolved — option 1; -> implementing)
f55d1ad feature-design: feature-v0-cli (design + grounded shapes; blocker on CLI auth posture)
```

## Notes for the next session

- The trust boundary is the security-bearing foundation. Three operator
  decisions are recorded and should NOT be re-litigated:
  - **CLI auth (option 1):** CLI is a full transport principal with its own
    operator-session bootstrap.
  - **D1 (option 1):** core as source-of-truth for the operator record.
  - **D2 (option 1):** dedicated local-console listener for bootstrap.
- The trust boundary's claimed security properties (reject self-asserted
  identity; no password-hash leak via Subscribe; throttle before scrypt) are
  genuinely earned — each has a mutation test that fails on the mutated
  implementation. The component-layer arc's "earned not asserted" standard
  now extends to security.
- **Foundation-doc drift (not a blocker):** `docs/SECURITY.md` §143 still says
  the operator-session wire shape is "deferred to feature-web-core-protocol-
  seam" — but the trust-boundary feature now implements it. Rolling-foundation
  note; roll it forward in a foundation-doc pass. The implementation is the
  source of truth.
- The CLI's three deep-debug commands (audit-query/inspect-command/
  adapter-status) remain stubbed pending a separate `feature-v0-core-diagnostics`
  (or similar) that builds the `QuerySpec`/`QueryResult`/`AuditRecord` schema
  + the core's query-result projection + audit-log storage. PROTOCOL.md:623
  commits these to v0.1.0 observability; they land when that feature does.
- `.work/bin/work-view` still hangs on `board`/`--blocking` (pre-existing
  uncommitted binary mod). Verify state via `grep ^stage:`.
- `check:drift` is the known broken repo gap (needs `protoc-gen-prost`);
  not in CI, not this feature's concern.
- Regen procedure (pre-existing): `buf generate` for TS (canonical),
  `git checkout -- contracts/rust/src/gen` to discard buf's wrong Rust output,
  then `cargo build` to regen Rust via the crate's `build.rs`/prost-build.
