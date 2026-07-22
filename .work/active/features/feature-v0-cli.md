---
id: feature-v0-cli
kind: feature
stage: review
tags: [ux, protocol]
parent: epic-v0-1-0-implementation
depends_on: [feature-v0-protocol-seam, feature-v0-control-surface-trust-boundary]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-22
---

# Feature: CLI

## Brief

Build the diagnostic CLI for setup, administration, debugging, and scripted access. The CLI is not a second independent product surface with divergent semantics — it speaks the same protocol semantics as the web cockpit, just through a different surface. It reuses the shared TypeScript operator domain and protocol client.

v0.1.0 CLI scope (per `docs/UX.md`): setup/configuration, adapter enrollment, session inspection, command submission for scripting, audit queries, and diagnostic commands (`audit-query`, `inspect-command`, `session-health`, `adapter-status`). The CLI is the operator's tool for the things that are awkward in a web UI — initial setup, scripted automation, and deep debugging.

## Epic context

- Parent epic: `epic-v0-1-0-implementation`
- Position in epic: side branch off the protocol seam. Independent of the web server and cockpit; can proceed in parallel with the web chain once the seam exists.

## Foundation references

- `docs/UX.md` — CLI section, diagnostic commands, surface-neutral conformance floor
- `docs/ARCHITECTURE.md` — shared TypeScript operator domain, CLI as a control surface
- `docs/PROTOCOL.md` — OperationKind registry, authority, audit
- `docs/SPEC.md` — v0.1.0 observability scope (CLI diagnostic commands as projections of the durable event log + audit records)

## Grounding (2026-07-20)

Verified against shipped code + foundation docs before designing:

- **Transport:** the CLI is a first-class control surface that speaks gRPC directly to the core (not via the web-server). `docs/SECURITY.md:143` commits the CLI as a transport principal verified at the core. The web-server's `core-client.ts` (`createGrpcTransport` + `x-patchbay-core-secret` interceptor against `ControlService`) is the exact pattern the CLI reuses — same `@connectrpc/connect-node` + `@patchbay/contracts`.
- **Protocol surface:** `ControlService` exposes `Submit` / `Subscribe` / `LoadSnapshot` (`contracts/proto/patchbay/control.proto`). The four diagnostic commands issue `query` Operations via `Submit` (`OperationKind.QUERY`, committed v0.1.0, full lifecycle, no direct-to-completed shortcut — PROTOCOL:155). The core accepts `QUERY` (`core/src/acceptance/pipeline.rs:29`, `core/src/authority/state.rs:83`).
- **Compound-issuer wire shape (settled by `feature-v0-protocol-seam` Q1):** the core trusts `x-patchbay-core-secret` (transport principal) + `x-patchbay-operator-id` + `x-patchbay-operator-session-id` (operator identity vouched by the transport principal). `web-server/src/routes/rpc.ts` forwards exactly these. The core's `IssuerContext` trait (`core/src/authority/issuer.rs`) reads verified actor/endpoint/device/generation from this metadata.
- **Operator record + password:** the web-server verifies operator passwords against a `scrypt$<salt>$<hash>` record (`web-server/src/sessions.ts`) configured via `PATCHBAY_OPERATOR_ID` + `PATCHBAY_OPERATOR_PASSWORD_HASH`. The first operator is created via CLI/local-console bootstrap (SECURITY:77-78), NOT an unauthenticated web page.
- **Shared TS operator domain to reuse:** `@patchbay/contracts` (generated TS), the Connect gRPC client pattern from `web-server/src/core-client.ts`. The CLI does NOT need the browser presentation model / reconnect state machine — it is a synchronous command-response tool (submit → await result → print). The cockpit's presentation fold is irrelevant to the CLI; the CLI prints `SubmissionResult` / `SessionSnapshot` / audit records directly.
- **Diagnostic command data sources (UX.md):** `audit-query` (audit log, via `query` Op), `inspect-command <id>` (event log + audit, via `query` Op filtered by command/correlation id), `session-health` (session state axes, via `query` Op / `LoadSnapshot`), `adapter-status` (adapter registry, via `query` Op). All read-only; no new storage/write path. Redaction is enforced at the core boundary (SECURITY:236 — `attachment_method.descriptor` excluded from `adapter-status`).

## Architectural choice

A Node TypeScript CLI package (`patchbay-cli`) that reuses `@patchbay/contracts` + the `createGrpcTransport` Connect client pattern from `web-server/src/core-client.ts`, speaking gRPC directly to the core. The CLI is a synchronous command-response tool: each invocation builds an `Operation` (or calls `LoadSnapshot`/`Subscribe` for state reads), submits it, awaits the `SubmissionResult` (or stream), and prints structured output, then exits. It does NOT run the browser presentation model or reconnect state machine — those are cockpit concerns.

Command framework: a minimal hand-rolled arg parser (the command set is small and fixed: `setup`, `login`, `audit-query`, `inspect-command`, `session-health`, `adapter-status`, plus `instruct`/`cancel`/`interrupt` for scripting). No heavy framework; `process.argv` dispatch + a small options parser. (A framework like `commander`/`yargs` is a mechanical choice — the design does not depend on it.)

Output: human-readable by default; `--json` for machine-readable (scripting). Exit codes: 0 success, non-zero for errors / no-results / auth failure (scriptable per SPEC:47).

## Implementation Units

### Unit 1: CLI scaffold + core client + auth

**File**: `cli/package.json`, `cli/tsconfig.json`, `cli/src/main.ts`, `cli/src/core-client.ts`, `cli/src/auth.ts`, `cli/src/credentials.ts`

The `patchbay-cli` Node package + Connect gRPC clients to the shipped trust-boundary surface. Two clients: a `ControlService` client (network listener, for Submit/Subscribe/LoadSnapshot/VerifyOperatorPassword/EnrollControlSurfacePrincipal/RevokeOperatorSession) and an `AdminService` client (local-console listener, for BootstrapOperator). Both mirror `web-server/src/core-client.ts` (`createGrpcTransport` + `x-patchbay-core-secret` interceptor).

```typescript
// cli/src/core-client.ts — mirrors web-server/src/core-client.ts
import { createClient, type Interceptor } from "@connectrpc/connect";
import { createGrpcTransport } from "@connectrpc/connect-node";
import { ControlService, AdminService } from "@patchbay/contracts";

function coreSecretInterceptor(coreSecret: string): Interceptor {
  return (next) => async (request) => {
    request.header.set("x-patchbay-core-secret", coreSecret);
    return next(request);
  };
}

export function makeControlClient(coreAddr: string, coreSecret: string) {
  return createClient(ControlService,
    createGrpcTransport({ baseUrl: coreAddr, interceptors: [coreSecretInterceptor(coreSecret)] }));
}
export function makeAdminClient(adminAddr: string, coreSecret: string) {
  return createClient(AdminService,
    createGrpcTransport({ baseUrl: adminAddr, interceptors: [coreSecretInterceptor(coreSecret)] }));
}
```

```typescript
// cli/src/auth.ts — the auth interceptor reading the credential store
// The credential store (cli/src/credentials.ts) holds the PrincipalCredential
// (principal_id + secret) + OperatorSessionId returned by setup/login.
// Every state-changing Submit adds: x-patchbay-principal-id, x-patchbay-principal-secret,
// x-patchbay-operator-id, x-patchbay-operator-session-id — exactly the headers
// the shipped MetadataIssuerContext verifies (server/src/issuer.rs).
export function authInterceptor(creds: CliCredentials): Interceptor {
  return (next) => async (request) => {
    request.header.set("x-patchbay-principal-id", creds.principal.principalId);
    request.header.set("x-patchbay-principal-secret", creds.principal.secret);
    request.header.set("x-patchbay-operator-id", creds.operatorActorId);
    request.header.set("x-patchbay-operator-session-id", creds.sessionId);
    return next(request);
  };
}
```

**Implementation Notes**:
- The credential store (`cli/src/credentials.ts`) holds a `PrincipalCredential { principal_id, secret, operator_actor_id, endpoint_id, device_id, endpoint_generation }` + `OperatorSessionId`, written 0600 to a CLI-local file (e.g. `~/.patchbay/cli-credentials.json`). Populated by `login`/`setup`; read by the auth interceptor. SECURITY:93 (high-entropy, meaningless client-side) applies; the secret is a bearer credential — file perms 0600, never logged.
- Fail fast: refuse state-changing commands if no credential store exists (direct the operator to `patchbay-cli login`); refuse to run without `PATCHBAY_CORE_SECRET`.
- The CLI is a full transport principal: each `login` enrolls a distinct CLI endpoint (its own `EndpointId`/`DeviceId`/`Generation`), verified by the core against the durable `ControlSurfacePrincipalRecord`.

**Acceptance Criteria**:
- [ ] `patchbay-cli` runs, parses args, dispatches to a command handler, prints output, exits with the right code
- [ ] The control client reaches a running `patchbay-core-server` (health probe)
- [ ] Refuses to run without `PATCHBAY_CORE_SECRET`; refuses state-changing commands without a credential store
- [ ] The auth interceptor adds all four verified headers; a request without them is rejected by the core (verified)

### Unit 2: `setup` + `login` + `logout` (operator bootstrap + enrollment)

**File**: `cli/src/commands/setup.ts`, `cli/src/commands/login.ts`, `cli/src/commands/logout.ts`

`setup` runs against the **local-console `AdminService`** (loopback listener, `PATCHBAY_CORE_ADMIN_ADDR`): `BootstrapOperator(BootstrapRequest{ setup_secret, operator_actor_id, password_hash, principal })` → `BootstrapResult{ grant_id, session_id, principal: PrincipalCredential }`. It creates the first operator record + authority grant + the CLI's first principal, writing the `PrincipalCredential` + `OperatorSessionId` to the 0600 credential store. The setup secret is one-time (expires after use/timeout — the core enforces this; SECURITY:78). `login` runs against the **network `ControlService`**: `VerifyOperatorPassword(VerifyOperatorPasswordRequest{ operator_actor_id, password, principal })` → `VerifyOperatorPasswordResult{ operator_session_id, principal: PrincipalCredential }` (the core does the scrypt check, throttled per SECURITY:85, and enrolls a new CLI endpoint). `logout` calls `RevokeOperatorSession`.

**Implementation Notes**:
- `setup` is the bootstrap channel (SECURITY:208: local CLI/console — distinct from routine web login; this channel distinction is load-bearing for lockdown exit). It speaks to the local listener, NOT the network listener.
- The `password_hash` in `BootstrapRequest` is the `scrypt$<salt>$<hash>` format (mirror `web-server/src/sessions.ts:hashPassword`). The CLI hashes the operator-chosen password locally before sending it to the local AdminService; OR the AdminService accepts a plaintext password and hashes it — verify which the shipped `AdminService.BootstrapOperator` expects (check `server/src/admin_service.rs`) and match it. Prefer the core doing the hash (single hashing site), but match what shipped.
- `VerifyOperatorPassword` returns a fresh `PrincipalCredential` for the CLI endpoint on success — the CLI stores it. The operator actor id comes from the existing operator record (created by `setup`).
- The credential store is written 0600; the bearer `PrincipalCredential.secret` is never logged.

**Acceptance Criteria**:
- [ ] `setup` calls the local `AdminService.BootstrapOperator`, creates the operator record + grant + first principal, writes the credential store
- [ ] `login` calls the network `ControlService.VerifyOperatorPassword` with a new CLI endpoint enrollment, stores the returned `PrincipalCredential` + `OperatorSessionId`
- [ ] `logout` calls `RevokeOperatorSession`; subsequent state-changing commands are rejected (no valid session)
- [ ] The credential store is 0600; the secret is never logged
- [ ] `setup` speaks to the local listener only (not the network listener)

### Unit 3a: `session-health` (buildable now via LoadSnapshot)

**File**: `cli/src/commands/session-health.ts`

`session-health` prints the session connectivity × activity axes for one or all sessions. Unlike the other three diagnostic commands, it does NOT need a `query` Operation projection — it calls `LoadSnapshot` (which exists and returns `SessionSnapshot` carrying `sessions[]` with connectivity/activity states) and prints the axes directly.

**Implementation Notes**:
- Prints the full canonical registries: `SessionConnectivityState` (`live`/`stale`/`offline`/`unknown`/`failed`) × `SessionActivityState` (`idle`/`working`/`unknown`) — the same axes the cockpit binds.
- Output: human-readable table by default; `--json` for scripting. Exit 0 with results; non-zero for no-sessions / error.

**Acceptance Criteria**:
- [ ] `session-health [session-id]` prints the connectivity × activity axes for one or all sessions
- [ ] `--json` output is machine-readable
- [ ] Read-only (uses `LoadSnapshot`; no `query` Operation, no storage write)

### Unit 3b: `audit-query`, `inspect-command`, `adapter-status` — BLOCKED on a core-diagnostics prerequisite

**Files**: `cli/src/commands/audit-query.ts`, `cli/src/commands/inspect-command.ts`, `cli/src/commands/adapter-status.ts`

**Status (2026-07-20): blocked, not implemented in this feature.** These three commands require a core-side diagnostic projection surface that does not yet exist and that the CLI cannot build (it lives in `core/` + `contracts/`, outside the CLI's write scope). Specifically:

- The core **accepts** `OperationKind.QUERY` into its lifecycle (it's in the allowed-kinds set, `core/src/acceptance/pipeline.rs:29`), but there is **no query-payload schema** (`QuerySpec`/`QueryResult`/`AuditRecord` — none exist in `contracts/proto/patchbay/`), **no query handler** in the core that decodes a query and projects results, and **no audit-log storage** as a queryable projection (the authority projection is grant-only).
- `ControlService.Submit` returns only `SubmissionResult` — nothing carries diagnostic results back to the caller. `audit-query`/`inspect-command`/`adapter-status` all need a query-result RPC + a core-side projection of audit records / command lifecycle traces / adapter manifests.
- `PROTOCOL.md:623` commits these three commands to v0.1.0 observability ("v0.1.0 observability = audit log + CLI `audit-query`/`inspect-command`/`session-health`/`adapter-status`"). The audit log + the query projections are committed but not yet built — they are a prerequisite feature, not the CLI's job to backfill.

**Scope decision (operator, 2026-07-20): option 1** — ship the CLI with what is genuinely buildable now (Units 1, 2, 3a, 4) and surface the diagnostic-projection prerequisite as a separate feature (`feature-v0-core-diagnostics` or similar) that builds the `QuerySpec`/`QueryResult`/`AuditRecord` schema + the core's query-result projection + the audit-log storage. The three blocked commands are stubbed with a clear "requires core-diagnostics (not yet implemented)" message + non-zero exit, so the CLI's command surface is honest about what is and isn't available; they wire up when the prerequisite lands. This keeps the CLI's write scope clean (`cli/` only) and avoids either ballooning the CLI into a cross-cutting feature or silently shrinking v0.1.0 by dropping the commands entirely.

**What this means for v0.1.0**: the cockpit (done) provides live session control; the CLI ships the **load-bearing bootstrap channel** (`setup`/`login` — the first-operator creation the cockpit needs to exist at all, plus the lockdown-exit channel), the **scripting commands** (`instruct`/`cancel`/`interrupt`), and **`session-health`**. The three deep-debug commands land when core-diagnostics does. This is an honest partial v0.1.0 CLI; the committed-but-unbuilt diagnostic projections are tracked as a prerequisite, not silently dropped.

**When unblocked**, these commands issue a `query` Operation via `Submit` (or the new query-result RPC), filter by actor/command/target/time/outcome (`audit-query`), command id/correlation id (`inspect-command`), or adapter-id/all (`adapter-status`), and print structured output. Redaction is enforced at the core boundary (the CLI inherits it; `adapter-status` never shows raw `attachment_method.descriptor`; `inspect-command` never surfaces prompt bodies — SECURITY:236,240). `inspect-command`'s delivery trace is a projection, not authoritative `CommandState` (UX.md).

### Unit 4: Scripting commands (`instruct`, `cancel`, `interrupt`) + output contract

**File**: `cli/src/commands/instruct.ts`, `cli/src/commands/cancel.ts`, `cli/src/commands/interrupt.ts`, `cli/src/output.ts`

The command-submission commands for scripting: `instruct <target> <prompt>` (Submit an `instruct` Operation), `cancel <command-id>`, `interrupt <command-id>`. Plus the shared output formatter (human-readable + `--json`).

**Implementation Notes**:
- Target resolution: `<target>` is a stable session identity (adapter/scope/runtime/gen) or a resolvable alias. The CLI must show stable target identity before submission (the conformance floor's identity-before-intent — same as the cockpit).
- `instruct` reads the prompt from an arg or stdin (`-` for stdin) for scripting.
- Idempotency: the CLI generates an idempotency key (or accepts `--idempotency-key`) so scripted retries are safe per the retry-safety matrix.
- Exit codes: 0 accepted/completed; non-zero for rejected/failed/unknown (scriptable per SPEC:47). `SubmissionOutcome.UNKNOWN` prints a clear "reconcile via inspect-command" message.

**Acceptance Criteria**:
- [ ] `instruct` submits an `instruct` Operation and prints the `SubmissionResult`
- [ ] `cancel` / `interrupt` submit the corresponding OperationKind
- [ ] `--json` output is machine-readable; exit codes are scriptable
- [ ] `instruct` shows stable target identity before submission (identity-before-intent)
- [ ] Idempotency key is generated/accepted for scripted retry-safety

## Implementation Order

1. Unit 1 (scaffold + core client + admin client + auth) — the foundation
2. Unit 2 (`setup` + `login` + `logout`) — depends on Unit 1; setup uses AdminService, login uses VerifyOperatorPassword
3. Unit 3a (`session-health`) — depends on Unit 1; uses LoadSnapshot
4. Unit 4 (scripting commands + output) — depends on Unit 1; can parallelize with 3a
5. Unit 3b (`audit-query`/`inspect-command`/`adapter-status`) — BLOCKED; stubbed now, implemented when core-diagnostics lands

## Testing

- **Interface tests:** the core client reaches the core (smoke); each command builds a valid `Operation` and decodes the result.
- **Regression tests:** identity-before-intent on `instruct`; the credential store is 0600 + never logs the secret.
- **Unit tests:** output formatting (human + `--json`); exit-code mapping for `SubmissionOutcome`; the three blocked commands stub cleanly with a non-zero exit.
- **Test data:** a fixture operator record + grant for the core-client smoke test.

## Risks

- **Auth posture:** RESOLVED (option 1, 2026-07-20 — see the Blocker section below).
- **Core-diagnostics prerequisite (Unit 3b):** the three deep-debug commands need a core-side query-result projection + audit-log storage + `QuerySpec`/`QueryResult`/`AuditRecord` schema that does not yet exist (`core/` + `contracts/`, outside the CLI's write scope). Surfaces as a separate prerequisite feature; the three commands are stubbed honestly in this feature.
- **CLI credential store security:** the credential store must be file-permission-locked (0600) and never log the secret. SECURITY:93 (session ids high-entropy, meaningless client-side).

## Simplification

- No browser presentation model / reconnect state machine — the CLI is synchronous command-response.
- No heavy command framework — small fixed command set, hand-rolled dispatch.
- No new storage/write path — `session-health` is read-only via `LoadSnapshot`; the scripting commands submit Operations the core already handles.
- The three deep-debug commands are stubbed, not implemented — they depend on a prerequisite that doesn't exist; shipping them half-built would be dishonest.

## Blocker (2026-07-20) — CLI auth posture — RESOLVED (option 1)

The CLI is a first-class control surface that speaks gRPC directly to the core (SECURITY:143). The protocol seam settled the **compound-issuer** wire shape around the *web-server* as the verified transport principal that vouches for the operator (it forwards `x-patchbay-operator-id` + `x-patchbay-operator-session-id` after verifying the operator's login cookie/CSRF). The CLI bypasses the web-server, so the question was: how does the CLI establish the operator identity the core requires?

### Resolution (operator, 2026-07-20): option 1 — CLI runs its own operator-session bootstrap (full transport principal)

`setup`/`login` create an operator session locally (the CLI verifies the operator password against the same `scrypt$<salt>$<hash>` record the web-server uses, OR consumes the one-time setup secret), then the CLI forwards `x-patchbay-operator-id` + `x-patchbay-operator-session-id` exactly like the web-server. The CLI is a full transport principal with its own endpoint/device record + generation, audited per-session. This matches the compound-issuer model end-to-end (the core independently verifies the CLI principal + operator identity), satisfies SECURITY:81 ("A CLI endpoint enrolls only through local setup credentials or an existing authenticated operator session"), and is consistent with the v0.1.0 topology — the CLI installs on the operator's machine and speaks gRPC over the network to the core, exactly as the browser cockpit is a remote control surface.

**Why option 1 over option 2:** option 2 (core-secret + a configured operator id, no per-session login) would be a quiet weakening of the documented posture — it bypasses operator-session verification, weakens the audit trail (every CLI action is "the configured operator, no session"), and does not satisfy SECURITY:81's session path. Option 1 keeps the security model uniform across surfaces: the core verifies every control surface the same way. The CLI is a remote client like the browser, so it authenticates like one.

**Consequences for the design (folded into the units below):**
- The CLI carries a local credential store (0600, file-permission-locked) holding the operator-session id + operator actor id + endpoint/device + generation, populated by `login` and read by the auth interceptor. SECURITY:93 (session ids high-entropy, meaningless client-side) applies.
- The `login` flow verifies the operator password against the same `scrypt$` record format the web-server uses (`web-server/src/sessions.ts: hashPassword`/`verifyPassword`). This is a mechanical reuse of the password-verification code, not a re-implementation of the session *store* — the CLI's session lives in its local credential store, not the web-server's in-memory `SessionStore`. (Whether to factor the password code into a shared util or duplicate it is a mechanical choice resolved in Unit 2.)
- The CLI's operator-session bootstrap establishes an `OperatorSessionId` (which already exists in the generated contracts per the protocol-seam decision). The CLI forwards it; the core's `IssuerContext` reads verified actor/endpoint/device/generation from the metadata, exactly as it does for the web-server.
- `setup` remains the bootstrap channel (SECURITY:77-78, 208: first operator via CLI/local-console; the channel distinction from routine web login is load-bearing for lockdown exit).

Option 2 is explicitly rejected for v0.1.0 (it would be a new, weaker posture not in the foundation docs; promotion would be a reversal with a protocol-change ceremony).

## Implementation discovery (2026-07-21) — bootstrap/session transport prerequisite

Implementation stopped before scaffolding Unit 1 because the shipped core boundary cannot realize the resolved auth posture or Unit 2 within the CLI-only write scope:

- `ControlService` exposes only `Submit`, `Subscribe`, and `LoadSnapshot`. It has no operator bootstrap, operator-session enrollment, setup-secret consumption, or grant-administration RPC.
- Every `Submit` passes through the existing live-grant check before acceptance. Although `Grant` is a generated contract and the core has an internal `ingest_grant` function, no control-service method exposes that function. The first grant therefore cannot be created by submitting an Operation; doing so would require the grant being bootstrapped.
- The web server does not read an operator record created by another component. It requires `PATCHBAY_OPERATOR_ID` and `PATCHBAY_OPERATOR_PASSWORD_HASH` at startup and keeps operator sessions in its own in-memory `SessionStore`. A CLI-local password record would not become the record the web server verifies.
- No shipped component stores, expires, or consumes the one-time setup secret required by SECURITY § Enrollment and authentication.
- The core does not independently verify the forwarded operator session. After the shared core-secret interceptor succeeds, `MetadataIssuerContext` accepts any non-empty operator-id and operator-session-id metadata. It also hard-codes the verified endpoint to `patchbay-web-server` and supplies no device or endpoint generation, so a direct CLI request cannot be represented as its own full transport principal.

This is not a mechanical CLI implementation choice. Proceeding would require one of two materially different security designs: (1) add the missing bootstrap/operator-session/control-surface-principal contract and core/server implementation, then build the CLI against it; or (2) weaken the resolved posture to trust the shared core secret plus self-supplied identity/configuration, which is the already explicitly rejected option 2. A CLI-only implementation cannot create the first operator/grant, enforce setup-secret expiry, or establish a core-verifiable CLI session without pretending those security claims are satisfied.

### Blocker

A prerequisite must first define and implement the bootstrap and CLI-principal boundary across `contracts/`, `core/`, and `server/` (and define how the resulting password record is consumed by `web-server/`). That work is outside this feature's allowed write scope. Per the design-flaw escape hatch, this feature returns to `drafting`; no `cli/` files were created.

### Resolution (operator, 2026-07-21): scope the prerequisite as `feature-v0-control-surface-trust-boundary` (option 1 at the epic level)

Rather than weaken the CLI to the rejected option 2, the operator chose to scope a new prerequisite feature that builds the real control-surface trust boundary the docs promise: the bootstrap + grant-admin RPC, the real transport-principal verifier (distinguishing the web-server principal from the CLI principal, rejecting self-asserted identity), the shared operator record, and the setup-secret lifecycle. That feature lives at `.work/active/features/feature-v0-control-surface-trust-boundary.md` (stage: drafting) and spans `contracts/` + `core/` + `server/` + the operator-record contract `web-server/` consumes.

`feature-v0-cli` now `depends_on: [feature-v0-protocol-seam, feature-v0-control-surface-trust-boundary]`. The CLI stays at `stage: drafting` (returned here by the implementation-discovery escape hatch) until the trust-boundary feature lands; its resolved option-1 auth posture becomes realizable then. The discovery findings that motivated the prerequisite are preserved above.

## Implementation notes (2026-07-22)

- Execution capability: one feature-owning implementation worker, direct-read only. The CLI is one cohesive security-bearing package and keeping Units 1–4 in one context avoided credential/auth/output handoff gaps; no child stories or delegation were needed.
- Review weight: `standard` (caller-specified stop at feature review; the dispatching agent owns the independent review).
- Files changed: new `cli/` package with the prescribed `src/{main.ts,core-client.ts,auth.ts,credentials.ts,output.ts,commands/}` layout, package manifest/lockfile/tsconfig, unit/interface tests, and an opt-in real-core smoke test.
- Tests added: 15 Node tests covering the four auth headers, fail-closed configuration, loopback-only admin addressing, setup/login/logout, 0600 credential permissions, bearer-secret output exclusion, snapshot decoding and session-health axes, identity-before-intent ordering, scripting Operation construction/correlation/idempotency, SubmissionOutcome exit codes, and honest Unit 3b stubs. `tests/core-smoke.mjs` additionally boots the shipped core and proves setup → login → authenticated `LoadSnapshot` → logout → missing-credential rejection over the real listeners.
- Simplification: hand-rolled fixed command dispatch; no browser presentation/reconnect model, no heavy CLI framework, no CLI storage beyond the credential file, and no false deep-diagnostic implementation.
- Adjacent issues parked: none. Unit 3b remains the already-recorded core-diagnostics prerequisite and is represented only by the required non-zero stubs.

### Checkpoint evidence

1. **Unit 1 — scaffold/client/auth/credentials:** `PATCHBAY_CORE_SECRET` is mandatory; network and loopback clients use Connect gRPC; authenticated calls read the credential store at request time and add the principal id, principal secret, operator id, and core-issued session id. Credential writes are atomic, directory mode 0700, file mode 0600.
2. **Unit 2 — setup/login/logout:** `setup` hashes locally into the `scrypt$<salt>$<hash>` shape required by `AdminService` and can address only a loopback admin URL. `login` consumes the throttled `VerifyOperatorPassword` RPC and enrolls a fresh endpoint. `logout` revokes at the core before deleting the local credential.
3. **Unit 3a — session-health:** decodes the generated `SessionSnapshot` bytes returned by `LoadSnapshot`, checks domain/LSN consistency, and emits canonical connectivity × activity values as a table or JSON.
4. **Unit 4 — scripting/output:** `instruct` resolves aliases through the authoritative snapshot, emits stable adapter/scope/runtime/generation identity to stderr before calling `Submit`, and supports prompt stdin. `cancel`/`interrupt` recover the original runtime target from the finite authorized command-record subscription because the shipped Operation contract requires a runtime target while the designed CLI syntax supplies only command id. A supplied idempotency key deterministically derives the command id from key + target (or accepts `--command-id`) so retries reuse both protocol identities. Submission outcomes map to stable non-zero codes and `unknown` directs reconciliation through core command records.
5. **Unit 3b — stubs:** `audit-query`, `inspect-command`, and `adapter-status` print the exact core-diagnostics prerequisite message and exit non-zero.

### Integrated verification

- `cd cli && npm test` — PASS (15/15).
- `cd cli && npm run test:core-smoke` — PASS against `patchbay-core-server`; real setup/login/authenticated snapshot/logout path.
- `cd contracts/ts && npm run check:presentation` — PASS.
- `cd contracts/ts && npm run build && npm run check:vectors` — PASS (24 vectors; no contract changes).
- Acceptance walkthrough: every implemented Unit 1/2/3a/4 criterion is covered by executable evidence above; Unit 3b is intentionally and visibly stubbed per the resolved scope decision. No forbidden package, contract, core, server, adapter, web, mockup, or foundation-doc source was changed.

## Review-fix pass (standard, 2026-07-21)

- Blocker 1: verified that `canonicalSessionIdentity` already emits the complete stable tuple (`adapter`, `scope`, `runtime`, `generation`) and tightened `cli/tests/scripting-commands.test.ts` to require the exact emitted JSON identity before the `submit` event. No production identity change was needed.
- Blocker 2: credential writes now harden a parent directory to 0700 only when the recursive `mkdir` call created it. Existing configured parent directories retain their permissions, while the atomic credential file remains 0600. Added a regression test using an existing 0755 custom parent.
- Mutation evidence for Blocker 1: temporarily reduced the production canonical identity to adapter + generation; the focused scripting test failed with the reduced actual identity against the complete expected tuple. Reverted the mutation and confirmed the focused tests green.
- Regression evidence for Blocker 2: before the production fix, the new custom-parent test failed because the existing directory changed from expected 0755 to actual 0700. After the fix, the test preserves 0755 and confirms the credential file is 0600.
- Final verification: `cd cli && npm test` — PASS (16/16); `cd cli && npm run test:core-smoke` — PASS (setup → login → authenticated `LoadSnapshot` → logout/rejection).
- Lifecycle: completed the corrective implementation pass and returned `feature-v0-cli` to `stage: review`; the standard-weight dispatching orchestrator owns adjudication and closure.

## Review outcome (2026-07-22) — Request changes → implementing (standard pass 1)

Standard-weight feature review (fresh-context `gpt-5.6-sol`, same model class as the implementation worker — same-harness, NOT cross-model vs the implementer; cross-model vs the umans orchestrator per the global AGENTS.md advisory-review slot). The reviewer confirmed the functional verification passes and the auth-header completeness is genuinely earned (the real-core smoke proves the four headers — a mutation omitting any fails the smoke). Two would-be findings correctly rejected (the auth-header unit test is earned by the smoke; the Unit 3b stubs are honest, not contradictory). But the pass returned **Request changes** with 2 receiver-confirmed material blockers. Feature bounced `review → implementing`.

### Blockers (both receiver-confirmed after adjudication)

1. **Identity-before-intent test is not mutation-sensitive (self-defining)** (`cli/tests/scripting-commands.test.ts:79`): the test only checks `startsWith("adapter=pi-adapter")`. A mutation reducing the displayed identity to adapter+generation (omitting scope + runtime session id) would still pass. The ordering (identity before submit) is earned; the full-identity composition is not. Fix: tighten the test to assert the EXACT emitted identity tuple (`adapter=<id>;scope=<id>;runtime=<id>;generation=<n>` or whatever production emits); verify production is complete first; mutate-to-reduce must FAIL.
2. **Credential writes chmod an arbitrary configured parent directory to 0700** (`cli/src/credentials.ts:62-64`): every write unconditionally `chmod(dirname(path), 0o700)`. For `/tmp/patchbay.json` that would chmod `/tmp`; reproduced (an existing 0755 temp dir became 0700). Fix: preserve the 0600 file mode without mutating an existing parent directory's mode; only harden a directory the CLI creates. Add a regression proving a custom existing parent retains its mode.

### Rejected proposals (reviewer — sound)
- Auth-header unit test is self-defining because it imports `AUTH_HEADERS` — rejected: `tests/core-smoke.mjs` does authenticated `LoadSnapshot` + logout against the real verifier; changing/omitting any of the four headers fails the smoke. Header completeness is genuinely earned by the aggregate evidence.
- Unit 3b stubs contradict committed observability — rejected: their explicit non-zero prerequisite message matches the operator-approved partial scope; they neither silently drop nor falsely implement the commands.

### Notes
- Effective weight: standard (one pass; receiver adjudicates + fixes receiver-confirmed blockers + verifies + closes without re-review).
- The self-defining-test finding (Blocker 1) is the same failure mode the cockpit's review caught (mutation-survivable property tests) — the standard the prior arcs set applies forward.
