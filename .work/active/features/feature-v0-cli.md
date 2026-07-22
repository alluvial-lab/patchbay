---
id: feature-v0-cli
kind: feature
stage: drafting
tags: [ux, protocol]
parent: epic-v0-1-0-implementation
depends_on: [feature-v0-protocol-seam, feature-v0-control-surface-trust-boundary]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-21
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

**File**: `cli/package.json`, `cli/tsconfig.json`, `cli/src/main.ts`, `cli/src/core-client.ts`, `cli/src/auth.ts`

The `patchbay-cli` Node package + a Connect gRPC client to `ControlService` (mirroring `web-server/src/core-client.ts`: `createGrpcTransport` + `x-patchbay-core-secret` interceptor). Config from env (`PATCHBAY_CORE_ADDR`, `PATCHBAY_CORE_SECRET`) + a CLI-local credential store (the operator-session token / operator identity — see the Blocker below for the exact v0.1.0 auth posture). `main.ts` is the entry: arg dispatch → command handler → print → exit.

```typescript
// cli/src/core-client.ts — reuses the web-server's exact pattern
import { createClient, type Interceptor } from "@connectrpc/connect";
import { createGrpcTransport } from "@connectrpc/connect-node";
import { ControlService } from "@patchbay/contracts";

export function makeCoreClient(coreAddr: string, coreSecret: string) {
  const authenticateCorePrincipal: Interceptor = (next) => async (request) => {
    request.header.set("x-patchbay-core-secret", coreSecret);
    return next(request);
  };
  return createClient(
    ControlService,
    createGrpcTransport({ baseUrl: coreAddr, interceptors: [authenticateCorePrincipal] }),
  );
}
```

**Implementation Notes**:
- The operator-identity headers (`x-patchbay-operator-id`, `x-patchbay-operator-session-id`) are added by an auth interceptor populated from the CLI credential store. The exact shape of that store is the open Blocker below.
- Fail fast: refuse to run if `PATCHBAY_CORE_SECRET` is unset; refuse state-changing commands if no operator identity is established.

**Acceptance Criteria**:
- [ ] `patchbay-cli` runs, parses args, dispatches to a command handler, prints output, exits with the right code
- [ ] The core client reaches a running `patchbay-core-server` (health probe)
- [ ] Refuses to run without `PATCHBAY_CORE_SECRET`

### Unit 2: `setup` + `login` (operator bootstrap + enrollment)

**File**: `cli/src/commands/setup.ts`, `cli/src/commands/login.ts`

`setup` creates the first operator (SECURITY:77-78: first operator via CLI/local-console bootstrap, NOT an unauthenticated web page). It establishes the operator record (the `scrypt$<salt>$<hash>` password hash the web-server later verifies) + the operator's authority grant in the core, producing a one-time setup secret that expires (SECURITY:78). `login` enrolls a CLI endpoint via local setup credentials or an existing authenticated operator session (SECURITY:81), establishing the CLI credential store the auth interceptor reads.

**Implementation Notes**:
- `setup` is the bootstrap channel (SECURITY:208: local CLI/console/SSH/trusted device — distinct from routine web login; this channel distinction is load-bearing for lockdown exit).
- The credential store shape is the open Blocker. Whatever it is, `login` populates it and the auth interceptor reads it.

**Acceptance Criteria**:
- [ ] `setup` creates the first operator record + authority grant; the web-server can subsequently verify the password
- [ ] `login` establishes the CLI credential store for the auth interceptor
- [ ] A one-time setup secret expires after use or timeout (SECURITY:78)

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

1. Unit 1 (scaffold + core client + auth) — the foundation
2. Unit 2 (`setup` + `login`) — depends on Unit 1's auth posture
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
