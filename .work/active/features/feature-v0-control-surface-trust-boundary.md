---
id: feature-v0-control-surface-trust-boundary
kind: feature
stage: done
tags: [security, protocol]
parent: epic-v0-1-0-implementation
depends_on: [feature-v0-protocol-seam, feature-v0-core-authority, feature-v0-web-server]
release_binding: null
gate_origin: null
created: 2026-07-21
updated: 2026-07-22
---

# Feature: v0.1.0 control-surface trust boundary (real transport principals + bootstrap)

## Brief

Build the real control-surface security boundary that `docs/SECURITY.md` commits but that the shipped v0.1.0 components deferred. The protocol-seam feature settled the **compound-issuer** *requirement* (the core independently verifies both the transport principal and the operator identity — SECURITY:143) and shipped the web-server *half* (the web-server verifies the operator session at its boundary and forwards `x-patchbay-operator-id` + `x-patchbay-operator-session-id`), but it explicitly deferred "the real operator-session and transport-principal verifier" (`core/src/authority/issuer.rs`: "v0.1.0 tests supply a test context; the real operator-session and transport-principal verifier lands with the protocol seam and web server"). That deferred work — plus the bootstrap channel that creates the first operator/grant, and the shared operator record — never landed. This feature builds it, so the v0.1.0 control-surface security posture the docs promise is actually real rather than asserted.

This is the forcing-function discovery from `feature-v0-cli`: the CLI cannot be a real transport principal (its resolved auth posture, option 1) against the shipped core, because (a) there is no bootstrap/grant-admin RPC on `ControlService`, (b) the web-server reads the operator record only from env at startup, and (c) the core's `MetadataIssuerContext` hard-codes the endpoint to `patchbay-web-server` and accepts any non-empty operator-id without verification. See `## Implementation discovery (origin)` below for the verified findings.

Scope spans the four packages the boundary crosses: `contracts/` (the bootstrap + principal-identity schema), `core/` (the real transport-principal verifier + grant-admin ingestion), `server/` (the verified-issuer context that distinguishes principals), and the operator-record contract `web-server/` consumes. The CLI (`feature-v0-cli`) depends on this feature; its resolved option-1 auth posture becomes realizable once this lands.

## Epic context

- Parent epic: `epic-v0-1-0-implementation`
- Position in epic: a cross-cutting security-boundary feature that the protocol-seam, core-authority, and web-server features each partially delivered but none completed. It unblocks the CLI (the last v0.1.0 surface) and makes the compound-issuer verification the docs promise genuinely enforced rather than trusted-on-input.

## Foundation references

- `docs/SECURITY.md` — Compound issuer (§143), Enrollment and authentication (first operator via CLI/local-console bootstrap, §77-81; one-time setup secret that expires, §78; lockdown-exit channel distinct from routine web login, §208)
- `docs/PROTOCOL.md` — OperationKind registry, authority grants, audit
- `docs/ARCHITECTURE.md` — v0.1.0 process topology (web server, CLI, core, adapter may run on different machines; the core is the network-reachable fixed point + single durable writer)
- `feature-v0-protocol-seam` (done) — settled the compound-issuer wire-evidence shape (forwarded verified session record evidence); deferred the real verifier
- `feature-v0-cli` (drafting, blocked on this feature) — the forcing function

## Grounding (verified against shipped code, 2026-07-21)

- `ControlService` exposes only `Submit`/`Subscribe`/`LoadSnapshot` (`contracts/proto/patchbay/control.proto`). No bootstrap, operator-session enrollment, setup-secret, or grant-administration RPC.
- The core has an internal `ingest_grant` function (`core/src/authority/ingest.rs:20`) but no control-service method exposes it. Every `Submit` passes through the live-grant check before acceptance — so the first grant cannot be created by submitting an Operation (chicken-and-egg).
- The web-server reads `PATCHBAY_OPERATOR_ID` + `PATCHBAY_OPERATOR_PASSWORD_HASH` at startup into an in-memory `SessionStore` (`web-server/src/main.ts:43-46`, `web-server/src/sessions.ts`). A CLI-created password record would not be consumed.
- No shipped component stores, expires, or consumes the one-time setup secret (SECURITY:78).
- `server/src/issuer.rs:9` hard-codes `WEB_SERVER_ENDPOINT_ID = "patchbay-web-server"`; `MetadataIssuerContext::from_request` accepts any non-empty operator-id/session-id from metadata without verifying them, returns `None` for device + endpoint generation, and stamps the endpoint as the web-server regardless of caller. A direct CLI request cannot be represented as its own full transport principal.

## Architectural choice

A real transport-principal model where each control surface (web-server, CLI) is a distinct, core-verifiable principal with its own endpoint/device/generation, plus a bootstrap RPC that creates the first operator + authority grant via the local-console channel, and a shared operator record (password hash + actor id) that both the web-server and the CLI verify against. The core's `IssuerContext` becomes a real verifier that distinguishes principals and rejects unverified identity, not a metadata-passthrough.

The good news from grounding: the core already has a validated, durable `ingest_grant` function (`core/src/authority/ingest.rs:20`) that validates + appends + projects a grant. It is simply not exposed via a control-service RPC. So the bootstrap unit is largely "expose `ingest_grant` via a bootstrap RPC, gated by the local-console/setup-secret channel" — not a from-scratch grant engine. Similarly, `MetadataIssuerContext::from_request` (`server/src/issuer.rs:21`) already extracts operator-id + operator-session-id from metadata; it just doesn't *verify* them or distinguish principals. The verifier unit is "make that extraction a real verification + add principal identity," not a from-scratch auth system.

## Design decisions (resolved 2026-07-21)

### D1 — Operator-record sharing mechanism: option 1 — core as source-of-truth, read via RPC

The operator record (actor id + `scrypt$<salt>$<hash>` password hash) lives in the core's durable store as a new operator-record artifact. The web-server and CLI call a read RPC at login to verify the password (the core does the scrypt check, or returns the hash for the surface to check — decided in Unit 3). Single source of truth; no shared file; the core owns all operator state. Honors the Single-Source-of-Truth principle and keeps all operator state in the single durable writer. The web-server's env-only posture (`PATCHBAY_OPERATOR_ID`/`PATCHBAY_OPERATOR_PASSWORD_HASH`) becomes a first-run fallback/override only, not the primary record.

### D2 — Bootstrap channel shape: option 1 — dedicated local-console RPC, local-listener only

The core binds a separate local-only listener (loopback or unix socket) for bootstrap; the setup secret is presented there. The bootstrap RPC is routinely unreachable from the network listener. This is the strongest channel separation and honors SECURITY:208's load-bearing warning ("if a future deployment ever makes bootstrap trust equivalent to routine web login (same factor, same remote channel), lockdown would provide no protection"). A setup-secret-gated RPC on the network listener (option 2) was rejected as weaker — same remote channel, just secret-gated. The setup secret still expires after use/timeout (SECURITY:78) as defense-in-depth.

## Implementation Units

### Unit 1: Operator-record storage + bootstrap RPC (local-console listener)

**Files**: `contracts/proto/patchbay/admin.proto` (new), `contracts/proto/patchbay/common.proto` (new `OperatorRecord` message), `core/src/authority/operator.rs` (new), `core/src/storage/` (operator-record persistence), `server/src/admin_service.rs` (new), `server/src/main.rs` (bind the local-console listener)

The operator record: a new durable artifact `OperatorRecord { actor_id, password_hash (scrypt$<salt>$<hash>), created_at, ... }` stored via the core's `Storage` port (a new append/read path, mirroring how grants are stored — schema-owned events, not a hand-rolled table). The bootstrap RPC `BootstrapOperator(BootstrapRequest) returns (BootstrapResult)` creates the first operator record + the operator's authority grant (exposing the existing `ingest_grant`). It is served on a **dedicated local-only listener** (loopback/unix socket), gated by a one-time setup secret that expires after use/timeout (SECURITY:78). The setup secret is generated at first-run and printed to the local console / written to a root-owned file.

```protobuf
// contracts/proto/patchbay/admin.proto (new) — local-console admin service
service AdminService {
  rpc BootstrapOperator(BootstrapRequest) returns (BootstrapResult);
  // future: RevokeOperator, RotatePassword, etc. — additive, non-breaking
}
message BootstrapRequest {
  string setup_secret = 1;       // one-time, expires after use/timeout
  ActorId operator_actor_id = 2;
  string password_hash = 3;     // scrypt$<salt>$<hash> (surface supplies; core stores)
  // OR the core generates the hash from a supplied password — decided in Unit 1
}
message BootstrapResult {
  GrantId grant_id = 1;
  OperatorSessionId session_id = 2;  // optional: bootstrap may establish the first session
}
```

**Implementation Notes**:
- The local-console listener binds loopback (127.0.0.1) or a unix socket with 0600 perms; it is NOT the network listener. `server/src/main.rs` binds both: the network `ControlService`/`AdapterControlService` on `PATCHBAY_BIND_ADDR`, and the local `AdminService` on a loopback/unix address.
- The setup secret: generated at first run (CSPRNG), printed to stdout / written to a root-owned file, expires after one use or a timeout. The `AdminService` rejects a second bootstrap once an operator record exists (idempotent: bootstrap is first-run-only).
- Expose `ingest_grant` via the bootstrap — the grant's `subject_actor_id` is the new operator, `target_scope` is the authority domain, `allowed_operation_kinds` is the full operator set.
- Whether the core stores the password hash and the surface verifies, or the core does the scrypt check itself, is a Unit-1 detail — prefer the core doing the check (single verification site, no hash leakage to surfaces).

**Acceptance Criteria**:
- [x] `AdminService.BootstrapOperator` on the local listener creates the operator record + authority grant; a second call is rejected (first-run-only)
- [x] The setup secret expires after use or timeout (SECURITY:78)
- [x] The bootstrap RPC is unreachable from the network listener (local-only)
- [x] The operator record is durable (survives a core restart via the storage port)

### Unit 2: Real transport-principal verifier

**Files**: `server/src/issuer.rs`, `contracts/proto/patchbay/control.proto` (principal-identity metadata), `server/src/service.rs`

`MetadataIssuerContext::from_request` becomes a real verifier. Each control surface presents a distinct, verifiable principal identity (endpoint/device/generation); the core distinguishes the web-server principal from the CLI principal; self-asserted operator identity is rejected (bound to verified principal evidence, not accepted on non-empty). The deferred "real operator-session and transport-principal verifier" from the protocol-seam decision lands here.

**Implementation Notes**:
- Today `MetadataIssuerContext` hard-codes `WEB_SERVER_ENDPOINT_ID = "patchbay-web-server"` and accepts any non-empty operator-id/session-id. The fix: the web-server and CLI each present a verifiable principal identity (a principal credential established at enrollment — e.g. a per-principal secret or a signed token, NOT a self-asserted string); the core verifies it and stamps the real endpoint/device/generation. The operator-id is bound to the verified principal (the principal vouches for the operator, as the web-server does today — but now the principal itself is verified, not trusted-on-input).
- The web-server's principal credential is established at bootstrap or via a separate enrollment; the CLI's principal credential is established at `login` (Unit 4 / the CLI feature). Both are stored core-side.
- Must be genuinely enforced (testable, not asserted): a request with a self-asserted operator-id and no verifiable principal is REJECTED. The current "accept any non-empty" behavior is the failure mode to eliminate. Property-test it (mutate the verifier to accept unverified; the test fails).
- Do not regress the web-server's 4 `csrf_browser.qnt` properties — those are load-bearing, done, and tested. The verifier change strengthens the operator-identity half; the CSRF/session half is untouched.

**Acceptance Criteria**:
- [x] A request with a self-asserted operator-id and no verifiable principal is rejected
- [x] The core distinguishes the web-server principal from the CLI principal (different endpoint/device/generation)
- [x] The verifier is property-tested: mutating it to accept unverified identity fails the test
- [x] The web-server's 4 csrf_browser.qnt properties still hold

### Unit 3: Operator-record read RPC + web-server/CLI consumption

**Files**: `contracts/proto/patchbay/admin.proto` (or `control.proto`), `server/src/service.rs` (read RPC), `web-server/src/sessions.ts` + `web-server/src/main.ts` (consume the shared record)

A read RPC `GetOperatorRecord(ActorId) returns (OperatorRecord)` (or `VerifyOperatorPassword`) the web-server and CLI call at login. The web-server's env-only posture becomes a first-run fallback: if no operator record exists in the core, the env vars bootstrap a minimal record (or refuse to start, directing the operator to run `patchbay-cli setup`). The CLI verifies against the same record.

**Implementation Notes**:
- Per D1-option-1, the core is source-of-truth. Prefer the core doing the scrypt check (`VerifyOperatorPassword` returns a verified `OperatorSessionId` or rejects) — single verification site, no hash leakage. The web-server and CLI both call it.
- The web-server's `SessionStore` stays in-memory (sessions are still server-side per SECURITY:89); only the *password verification* moves to the core. The session record the web-server creates is still its own.
- Backward-compat: the env-only posture (`PATCHBAY_OPERATOR_*`) is preserved as a first-run fallback so the web-server can still start before bootstrap exists, but the primary path is the shared record. Decide in Unit 3 whether to deprecate the env path or keep it as an override.

**Acceptance Criteria**:
- [x] `VerifyOperatorPassword` verifies against the core's operator record; the web server uses it and the CLI-facing contract/client capability is generated for `feature-v0-cli`
- [x] The web-server no longer relies solely on env vars for the operator record (the password hash is an optional fallback/override; core verification is primary)
- [x] A login with a wrong password is rejected; a login with the right password establishes a session
- [x] The web-server's 4 csrf_browser.qnt properties still hold

### Unit 4: CLI-principal enrollment (boundary with feature-v0-cli)

**Files**: `cli/src/commands/login.ts` (in `feature-v0-cli`, once Units 1-3 land)

The CLI enrolls as a transport principal (its own endpoint/device/generation) via the bootstrap/session channel, establishing the credential store `feature-v0-cli`'s option-1 auth posture requires. This unit likely lives in the CLI feature itself once Units 1-3 land; the boundary is: this feature delivers the *capability* (the enrollment RPC + the verifier that accepts a CLI principal), and the CLI feature delivers the *client* (the `login` command + credential store). Decided when Units 1-3 are done.

**Acceptance Criteria**:
- [x] A CLI endpoint can enroll as a transport principal distinct from the web-server
- [x] The CLI's enrolled principal is verified by the core (not self-asserted)

## Implementation Order

1. Unit 1 (operator-record storage + bootstrap RPC + local listener) — the foundation; nothing else runs without it
2. Unit 2 (real transport-principal verifier) — depends on Unit 1's principal-identity model
3. Unit 3 (operator-record read RPC + web-server consumption) — depends on Unit 1's record + Unit 2's verifier
4. Unit 4 (CLI-principal enrollment) — depends on Units 1-3; likely folds into `feature-v0-cli`

## Testing

- **Interface tests:** the bootstrap RPC creates the operator record + grant (first-run-only); the read/verify RPC works; the local listener is unreachable from the network.
- **Regression tests:** the web-server's 4 `csrf_browser.qnt` properties still hold after the verifier change (load-bearing — do not regress).
- **Property tests:** the verifier rejects unverified identity (mutate-to-accept fails); the setup secret expires after use/timeout; bootstrap is idempotent (first-run-only).
- **Unit tests:** operator-record durability (survives restart); principal distinction (web-server vs CLI).

## Risks

- This is a security-bearing cross-cutting change. The compound-issuer verification must be genuinely enforced (testable, not asserted — the standard the cockpit/component-layer arcs set). A verifier that accepts any input is not a verifier; the current `MetadataIssuerContext` accepts any non-empty operator-id, which is exactly the failure mode to eliminate.
- The bootstrap channel must remain distinct from routine web login (SECURITY:208 — load-bearing for lockdown exit). D2-option-1 (dedicated local listener) is the strongest separation; do not collapse it.
- Crosses four packages; will warrant child stories per package boundary (contracts/core/server/web-server).
- The web-server's current env-only operator record is a working (if unergonomic) posture; replacing it must not regress the web-server's 4 csrf_browser.qnt properties (those are load-bearing, done, and tested).
- The local-console listener is a new network surface (loopback/unix); ensure it is genuinely local-only (not accidentally exposed).

## Implementation discovery (origin)

This feature was scoped in direct response to `feature-v0-cli`'s implementation-discovery blocker (commit `4da38dd`, 2026-07-21). The CLI worker correctly stopped when it found the resolved option-1 auth posture (CLI as a full transport principal with its own operator-session bootstrap) could not be realized against the shipped core boundary. The verified findings are reproduced in `## Grounding` above. Rather than weaken the CLI to the rejected option 2, the operator chose to scope this prerequisite feature (option 1 at the epic level): build the real trust boundary the docs promise, then build the CLI against it.

## Implementation notes

- Execution capability: one feature-owning high-capability implementation worker; direct-read/no-delegation was used because the caller assigned one cohesive security-boundary owner and prohibited nested delegation.
- Review weight: `standard` (project default); implementation intentionally stops at `stage: review` per the caller's lifecycle boundary.
- Delivery shape: one ownership bundle with Units 1–4 as checkpoints; no child stories were spawned.
- Files changed:
  - contracts: `contracts/proto/patchbay/admin.proto`, `common.proto`, `control.proto`, Rust/TS generation inputs and generated artifacts;
  - core: `core/src/authority/operator.rs`, authority/acceptance projection integration, and operator durability tests;
  - server: `server/src/admin_service.rs`, `identity.rs`, `issuer.rs`, `service.rs`, `state.rs`, `main.rs`, and gRPC/trust-boundary tests;
  - web server: core-owned login verification, in-memory principal credential storage, principal forwarding, and integration/smoke tests.
- Tests added: durable operator/principal replay; first-run-only bootstrap; setup-secret use/timeout expiry; malformed-bootstrap no-write; network-listener Admin rejection; wrong-password rejection; missing/wrong principal rejection; actor-binding rejection; distinct web/CLI endpoint-device-generation identity; authenticated endpoint enrollment; web login through the core record.
- Simplification: reused the existing event log and `ingest_grant`; no operator table, shared file, duplicate grant engine, or password-hash read RPC was introduced. `COMMITTED_OPERATION_KINDS` is now the shared implementation registry used by both acceptance and the bootstrap grant.
- Mechanical choices resolved in stride:
  - selected a dedicated loopback TCP listener (default `127.0.0.1:50052`) rather than a Unix socket; startup rejects any non-loopback admin bind and rejects reuse of the network address;
  - the core performs the Node-compatible scrypt check and never returns the stored password hash;
  - principal bearer secrets are CSPRNG-generated and only SHA-256 credential hashes are durably stored;
  - bootstrap prevalidates every record, appends the deterministic authority-domain grant before the operator record, then enrolls the initial principal. This ordering makes grant-before-operator retry recoverable while ensuring a durable operator always has its grant.
- Unit 4 boundary: this feature ships `EnrollControlSurfacePrincipal`, password-authenticated enrollment, generated clients, and verifier support for CLI identities. `feature-v0-cli` remains responsible for the CLI command and 0600 credential store.
- Discrepancies from design: `OperatorRecord` and principal messages live in `admin.proto` rather than `common.proto`; env password hash is now optional and available only to an explicitly selected fallback verifier, while the production default calls the core; no foundation docs were changed because their intended trust-boundary claims remain the target implemented here.
- Adjacent issue retained: the separately discovered core-diagnostics/audit-log projection prerequisite still has no dedicated `AuditRecord` schema. This feature durably records the operator, principal, and grant artifacts but does not silently invent the broader audit subsystem.

## Integrated verification

- Contracts: `cd contracts/ts && npm run build && npm run check:vectors && npm run check:presentation` — passed (24 vectors; 4 presentation registries; axe/contrast passed).
- Rust build: `cargo build --workspace --all-targets` with the project `CARGO_HOME`/`PATH` — passed.
- Rust tests: `cargo test --workspace` — passed, including 4 trust-boundary integration tests, 2 issuer tests, operator durability/replay tests, and all existing workspace tests.
- Rust lint: `cargo clippy --workspace --all-targets -- -D warnings` — passed.
- Web server: `cd web-server && npm run build && npm test` — passed (20/20), including all four load-bearing `csrf_browser.qnt`-traced properties: `CsrfRejectsUnauthenticated`, `CsrfRejectsMissingProof`, `RevokedSessionCannotCommand`, and `browser_local_state_not_authority`.
- Real-process seam: `cd web-server && npm run test:core-smoke` — passed after local-console bootstrap; the web client reached the network core with the issued principal credential.
- Local-only evidence: a client calling `AdminService.BootstrapOperator` on the network listener receives `UNIMPLEMENTED`; `main.rs` additionally rejects `0.0.0.0` and non-loopback admin addresses.
- Setup-secret evidence: valid first use succeeds; second use returns `FAILED_PRECONDITION`; a zero-duration injected setup secret returns `FAILED_PRECONDITION` without writing operator/grant/principal events.
- Mutation evidence (load-bearing): temporarily mutated `MetadataIssuerContext::from_request` to accept actor/session metadata when principal metadata was absent. `cargo test -p patchbay-core-server --lib issuer::tests::unverified_identity_is_rejected_for_all_non_empty_claims` failed with minimal input `actor = "a", session = "a"`; the mutation was reverted and the same property test passed.
- Formatting/integrity: `cargo fmt --all -- --check` and `git diff --check` — passed. `check:drift` was not run per the documented repository gap and caller instruction.

## Review outcome (2026-07-21) — Request changes → implementing (thorough pass 1)

Thorough-weight feature review (fresh-context `gpt-5.6-sol`, same model class as the implementation worker — same-harness, NOT cross-model vs the implementer; cross-model vs the umans orchestrator per the global AGENTS.md advisory-review slot). The reviewer independently confirmed the credential verification is load-bearing (the specified synthesized-principal bypass mutation failed `bootstrap_is_local_first_run_only_and_establishes_distinct_principals` + `authenticated_principal_can_enroll_another_endpoint`; reverted to 4/4 green). But the pass returned **Request changes** with 3 receiver-confirmed material current-cycle security blockers. Feature bounced `review → implementing`; thorough weight means re-review after fixes until a pass has no receiver-confirmed material blockers.

### Blockers (all receiver-confirmed after adjudication against repo context)

1. **Operator-session evidence remains self-asserted** (`server/src/issuer.rs:40`, `trust_boundary.rs:174`): `MetadataIssuerContext` merely requires a non-empty `x-patchbay-operator-session-id` header and stores it — no session record, no expiry, no revocation, no actor/session binding. An invented `"web-session"` string submits successfully. Contradicts SECURITY:143 ("the core independently verifies... the operator identity") and VERIFICATION:242. Fix: introduce core-verifiable session evidence (core-issued token the core verifies against a session record — exists, not expired, not revoked, bound to the verified principal's actor); reject invented/expired/revoked/mismatched sessions. Mutation-test it.
2. **The generic subscription leaks the stored password verifier** (`core/src/authority/operator.rs:203`, `server/src/service.rs:167`, `admin.proto:16`): `OperatorRecord` (with `password_hash`) is a `StoredEventKind::OperatorRecord` in the durable event log, and `Subscribe` returns every raw `StoredEventPayload` to any verified principal. Any enrolled principal can subscribe and decode the password hash — offline-guessing exposure. Directly contradicts the feature body's "never returns the stored password hash" claim. Fix: filter the Subscribe stream to exclude the security/authentication event kinds (`OperatorRecord`/`ControlSurfacePrincipal`/`Grant`/`DescendantGrant`/`Revocation`) — they're authority/security records, not operator-facing state; preserve cursor semantics. Test that decodes Subscribe output and proves no `OperatorRecord`/password_hash is present.
3. **Network password enrollment lacks the mandatory throttle and audit** (`server/src/service.rs:210`, `main.rs:58`): `VerifyOperatorPassword` is on the network `ControlService` and performs unrestricted scrypt after only the core-secret interceptor. Direct CLI/core callers bypass the web-server's account/network throttle (SECURITY:85) and success/failure is not durably audited (SECURITY:220-221). Fix: enforce throttling at the authoritative RPC boundary (mirror `web-server/src/login-limiter.ts`: account + network dimension, decay, pre-scrypt check, concurrent cap); emit redacted success/failure audit lines (no password/hash — actor id + caller address + outcome per SECURITY:236).

### Rejected proposals (reviewer — sound)
- Bootstrap is network-reachable because it uses TCP — rejected (`main.rs` enforces loopback; `trust_boundary.rs:65` confirms network → `Unimplemented`).
- Sequential bootstrap makes it unrecoverable — rejected (deterministic grant handles grant-first retry; durable operator record allows password-authenticated re-enrollment).
- This feature introduced raw attachment descriptors / prompt-body diagnostics — rejected (no new diagnostic projection; the leak is specifically the operator auth record via the existing raw subscription).

### Notes
- Effective weight: thorough (multi-pass convergence; re-review after fixes until no receiver-confirmed material blockers).
- The credential verification (Blocker 1's prerequisite) is genuinely earned — the reviewer's independent mutation test confirmed it. The 3 blockers are about the *rest* of the compound-issuer promise (session evidence), a real leak path (password hash via Subscribe), and a missing control (throttle/audit).

## Review-fix pass (thorough, 2026-07-21)

All three receiver-confirmed material blockers from thorough pass 1 were fixed; the feature is re-advanced to `review` for the required convergence pass.

### Fixes

1. **Core-verifiable operator sessions:** selected a core-issued opaque token backed by a process-local core session registry. The registry records the bound operator actor, expiry, and revocation state; a core restart invalidates all sessions (fail-closed). `MetadataIssuerContext` now verifies that the presented session exists, is active/unexpired, and is bound to the verified transport principal's operator actor. Bootstrap and `VerifyOperatorPassword` issue registered sessions; the web server stores the core token inside its server-side browser-session record and forwards that token rather than its independently generated browser cookie id. `RevokeOperatorSession` lets logout revoke the current core session. Tests reject invented, expired, revoked, and actor-mismatched sessions.
2. **No authentication material in Subscribe:** changed the subscription projection to a fail-closed operator-facing whitelist (`Operation`, `Observation`, `Elicitation`, `SessionState`, `CommandTransition`). Authority/authentication records (`OperatorRecord`, `ControlSurfacePrincipal`, `Grant`, `DescendantGrant`, `Revocation`) are never yielded. Visible events retain their durable event ids/LSNs, so filtered records appear only as cursor gaps; the test decodes every returned payload and asserts that neither an auth kind nor the stored `password_hash` bytes are present.
3. **Authoritative RPC throttle + redacted audit:** added a core-side in-memory limiter at `VerifyOperatorPassword`, before scrypt, with per-actor account and per-caller-network windows, bounded tracked-key maps, decay, and a concurrent-verification cap. Throttle rejection is `RESOURCE_EXHAUSTED` with retry metadata. Success, credential failure, throttling, verification failure, and enrollment failure emit structured redacted audit lines containing actor id, direct caller address, outcome, reason, and blocked dimensions only—never passwords or hashes. A recording sink proves emission/redaction in tests; production wires the stderr sink in `main.rs`. The previously recorded core-diagnostics/audit-ingest prerequisite remains the follow-on for durable audit storage because this repository still has no `AuditRecord` schema or audit-ingest port; this pass did not invent an incompatible persistence side channel.

### Verification

- `CARGO_HOME=/home/agent/projects/patchbay/.cargo-home PATH="/home/agent/.cargo/bin:$PATH" cargo test --workspace` — passed, including 7 trust-boundary integration tests and the new issuer/limiter unit tests.
- `CARGO_HOME=/home/agent/projects/patchbay/.cargo-home PATH="/home/agent/.cargo/bin:$PATH" cargo clippy --workspace --all-targets -- -D warnings` — passed.
- `cd web-server && npm test` — passed 20/20, including all four load-bearing `csrf_browser.qnt`-traced properties; the login integration test proves the core-issued session token is forwarded.
- `cd contracts/ts && npm run build && npm run check:vectors && npm run check:presentation` — passed (24 vectors; 4 presentation registries; axe/contrast passed).
- `cargo fmt --all -- --check` and `git diff --check` — passed. `check:drift` was not run per the known repository gap.
- **Mutation 1 (session verification):** temporarily bypassed `ProjectionState::verify_operator_session`; `bootstrap_is_local_first_run_only_and_establishes_distinct_principals` failed because the invented `"invented-session"` request was accepted. Reverted; all 7 trust-boundary tests passed.
- **Mutation 2 (subscription leak):** temporarily re-added `StoredEventKind::OperatorRecord` to the subscription whitelist; `subscribe_excludes_authentication_and_authority_records` failed on the decoded `OperatorRecord`. Reverted; the strengthened cursor-gap/leak test passed.
- **Mutation 3 (RPC throttle):** temporarily disabled both limiter block predicates; `password_rpc_throttles_before_a_correct_password_and_recovers_after_decay` failed because the correct-password request reached scrypt/enrollment and succeeded instead of returning `RESOURCE_EXHAUSTED`. Reverted; the burst, pre-scrypt rejection, decay recovery, and audit assertions passed.

### Scope notes

- Mechanical choice logged: process-local opaque session records were chosen over signed self-contained tokens because lookup makes expiry/revocation/actor binding explicit, minimizes cryptographic surface, and fails closed on restart; reauthentication restores a session.
- Foundation drift surfaced but not edited per the corrective write boundary: `docs/SECURITY.md` §143 still says the operator-session wire/evidence shape is deferred even though this feature now implements it.
- Effective review weight remains `thorough`; this pass stops at `stage: review` for orchestrator re-review and does not self-advance to `done`.

## Review closure (2026-07-21) — receiver adjudication → done (thorough converged)

Thorough-weight closure: the first independent review pass found 3 receiver-confirmed material security blockers; a focused fix stride addressed all 3; the receiver (orchestrator) independently re-verified each with an adversarial mutation test rather than trusting the worker's claim:

- **Blocker 1 (session verification):** mutated `verify_operator_session` to always pass (`|| true`) → `bootstrap_is_local_first_run_only_and_establishes_distinct_principals`, `revoked_core_session_cannot_be_reused`, `subscribe_excludes_authentication_and_authority_records`, and `authenticated_principal_can_enroll_another_endpoint` all FAILED. Earned.
- **Blocker 2 (Subscribe leak):** mutated `operator_facing_subscribe_event` to yield all events (re-include `OperatorRecord`/password_hash) → `subscribe_excludes_authentication_and_authority_records` FAILED ("filtered security records keep their durable LSNs as cursor gaps"). Earned.
- **Blocker 3 (throttle):** mutated `login_security.rs` to force `account_blocked = false` + `network_blocked = false` → `password_rpc_throttles_before_a_correct_password_and_recovers_after_decay` FAILED (correct-password request reached scrypt instead of returning `RESOURCE_EXHAUSTED`). Earned.

All three mutation tests genuinely fail on the mutated implementation. The "earned not asserted" standard is met for all three blockers — the same bar the component-layer arc (7-pass thorough) and the cockpit (mutation-tested conformance properties) set.

### Final integrated verification

- `cargo test --workspace` — passed (7 trust-boundary integration tests + issuer/limiter unit tests).
- `cargo clippy --workspace --all-targets -- -D warnings` — passed.
- `cd web-server && npm test` — 20/20, including the 4 load-bearing `csrf_browser.qnt` properties (no regression).
- `cd contracts/ts && npm run build && npm run check:vectors && npm run check:presentation` — passed (24 vectors; 4 presentation registries).

### Closure

All 3 receiver-confirmed blockers fixed and independently mutation-verified by the receiver. Thorough weight converged: this pass has no receiver-confirmed material current-cycle blockers. Feature advanced `review → done`.

### Foundation-doc drift surfaced (not a blocker; rolling-foundation note)

`docs/SECURITY.md` §143 still says the operator-session wire/evidence shape is "deferred to `feature-web-core-protocol-seam`" — but this feature now implements it (core-verifiable session evidence). This is a stale deferral note, not a false claim. It should be rolled forward in a foundation-doc pass (the deferral is now resolved), but it does not block this feature's completion. Tracked as a follow-on; the implementation is the source of truth.

### What this unblocks

`feature-v0-cli` (at `drafting`, depends on this feature) is now unblocked — its resolved option-1 auth posture (CLI as a full transport principal with its own operator-session bootstrap) is realizable against the shipped core boundary. The CLI's `login` obtains a core-issued session token; its `setup` uses the local-console bootstrap; its credential store holds the principal credential + session token. The CLI feature can now advance to implementation.
