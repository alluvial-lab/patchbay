---
id: feature-v0-cli
kind: feature
stage: implementing
tags: [ux, protocol]
parent: epic-v0-1-0-implementation
depends_on: [feature-v0-protocol-seam]
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

### Unit 3: Diagnostic commands (`audit-query`, `inspect-command`, `session-health`, `adapter-status`)

**File**: `cli/src/commands/audit-query.ts`, `cli/src/commands/inspect-command.ts`, `cli/src/commands/session-health.ts`, `cli/src/commands/adapter-status.ts`

The four read-only diagnostic commands. Each issues a `query` Operation via `Submit` (or calls `LoadSnapshot` for `session-health`), awaits the result, and prints filtered/structured output. All read-only; no new storage/write path. Redaction is enforced at the core boundary (the CLI inherits it; `adapter-status` never shows raw `attachment_method.descriptor`).

```typescript
// cli/src/commands/audit-query.ts — shape of a diagnostic command
import { create, toBinary } from "@bufbuild/protobuf";
import { OperationSchema, OperationKindSchema, /* ... */ } from "@patchbay/contracts";

export async function auditQuery(client: CoreClient, filters: AuditFilters): Promise<AuditRecord[]> {
  const operation = create(OperationSchema, {
    kind: OperationKind.QUERY,
    // payload: a query-spec envelope describing the audit filter
    // (the exact query-payload schema is a design detail resolved in Unit 3)
    // ...
  });
  const result = await client.submit({ operation });
  // decode + return the projected audit records
}
```

**Implementation Notes**:
- The `query` Operation's payload envelope carries the query specification (filter by actor/command/target/time/outcome for `audit-query`; by command id/correlation id for `inspect-command`; session-id or all for `session-health`; adapter-id or all for `adapter-status`). The exact query-payload schema (a typed `QuerySpec` message vs. a JSON envelope) is a design detail resolved in Unit 3 — prefer a typed proto message if the core already projects one, else a bounded `PayloadContentType.JSON` envelope.
- `inspect-command` surfaces lifecycle state + timestamps + LSNs + audit-trail entries — NOT prompt bodies or sensitive payload (SECURITY:240). The delivery trace is a projection, not authoritative `CommandState` (UX.md).
- `session-health` prints the full canonical registries: `SessionConnectivityState` × `SessionActivityState` (live/stale/offline/unknown/failed × idle/working/unknown).
- Output: human-readable tables by default; `--json` for scripting. Exit 0 with results; non-zero for no-results / error.

**Acceptance Criteria**:
- [ ] `audit-query --actor=X --outcome=denied` filters audit records and prints them
- [ ] `inspect-command <id>` prints the full lifecycle + audit trail with timestamps + LSNs
- [ ] `session-health [session-id]` prints the connectivity × activity axes
- [ ] `adapter-status` prints adapters + capability manifests, excluding raw `attachment_method.descriptor`
- [ ] All four are read-only (no storage write); `--json` output is machine-readable
- [ ] `inspect-command` never surfaces prompt bodies or sensitive payload content

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

1. Unit 1 (scaffold + core client + auth) — once the auth Blocker is resolved
2. Unit 2 (`setup` + `login`) — depends on Unit 1's auth posture
3. Unit 3 (diagnostic commands) — depends on Unit 1
4. Unit 4 (scripting commands + output) — depends on Unit 1; can parallelize with 3

## Testing

- **Interface tests:** the core client reaches the core (smoke); each command builds a valid `Operation` and decodes the result.
- **Regression tests:** redaction (`adapter-status` excludes `attachment_method.descriptor`; `inspect-command` excludes prompt bodies); identity-before-intent on `instruct`.
- **Unit tests:** output formatting (human + `--json`); exit-code mapping for `SubmissionOutcome`.
- **Test data:** a fixture operator record + grant for the core-client smoke test.

## Risks

- **Auth posture (the Blocker):** see below — this must be resolved before Unit 1/2.
- **Query-payload schema:** the `query` Operation's payload shape for the four diagnostic commands. Prefer a typed proto message if the core already projects query results; else a bounded JSON envelope. If a proto extension is needed (a new `QuerySpec`/`QueryResult` message), that crosses into `contracts/` (the CLI's forbidden write scope) — surface as a blocker, do not invent an ad-hoc convention.
- **CLI credential store security:** whatever shape the auth posture takes, the credential store must be file-permission-locked (0600) and never log the secret. SECURITY:93 (session ids high-entropy, meaningless client-side).

## Simplification

- No browser presentation model / reconnect state machine — the CLI is synchronous command-response.
- No heavy command framework — small fixed command set, hand-rolled dispatch.
- No new storage/write path — all diagnostics are read-only `query` projections.

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
