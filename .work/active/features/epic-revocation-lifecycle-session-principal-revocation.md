---
id: epic-revocation-lifecycle-session-principal-revocation
kind: feature
stage: review
tags: [security, foundation]
parent: epic-revocation-lifecycle
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-27
updated: 2026-07-27
---

# Session & principal revocation

## Brief

v0.1.0 ships current-session revocation plus actor-scoped **revoke all
operator sessions** (including CLI sessions) and **principal/endpoint/device
revocation**. Revoke-all invalidates operator-session generations without a
signing-secret rotation layer; principal/endpoint/device revocation marks a
credential scope revoked and rejects future Operations and same-id enrollment.

This feature delivers the session/principal plane across the web-server session
store, CLI credential controls, and core contract. Both durable scope families
write audited revocation events; the core remains the authority and each
control surface maintains a fail-closed local projection.

The feature also absorbs the session-record fields gap
(`backlog-session-record-fields-gap`): endpoint revocation requires records
that identify endpoints — endpoint id, revoked-at, and (per SECURITY.md:94)
session generation are added where missing rather than descoping the doc.

Does NOT cover: grant revocation (sibling feature) or lockdown (sibling
feature). Cockpit emergency-controls UX lives with the lockdown feature's
mockup pass; this feature owns the generated RPCs and scriptable CLI controls.

## Epic context

- Parent epic: `epic-revocation-lifecycle`
- Position: independent — parallel with grant-lifecycle; lockdown sequences
  after grant-lifecycle, not this.

## Simplification opportunity

- One revocation path pattern (`RevokeOperatorSession` + audit) generalizes
  to the new scopes instead of three bespoke flows.
- Session-record fields land once, serving both endpoint revocation and the
  SECURITY.md:94 record contract.

## Foundation references

- `docs/SECURITY.md` — revocation model (actions #2, #3), browser session
  model (record fields)
- `docs/PROTOCOL.md` — authority, Revocation events
- `contracts/proto/patchbay/control.proto` — existing revocation RPC pattern

## Design decisions

- **Revoke-all scope**: `RevokeAllOperatorSessions` invalidates every current
  core-issued operator session for the verified actor, including the caller and
  CLI sessions; the web proxy additionally marks every matching browser record
  revoked. The brief says “invalidate all operator sessions,” and a weaker
  browser-only core action would leave stolen CLI/session evidence live.
- **Durable revocation shape**: use append-only generation-fence and
  principal/endpoint/device revocation events, then rebuild hot projections by
  replay. Mutable in-memory flags alone would make principal credentials live
  again after restart and would violate the durable-audit commitment.
- **Endpoint and device recovery**: a revoked endpoint id or device id cannot be
  silently re-enrolled. Recovery uses a distinct endpoint/device identity from
  the trusted local CLI/SSH channel; an explicit restore RPC is reserved rather
  than treating login as implicit unrevocation.
- **Already-accepted Operations**: session/principal/endpoint revocation uses
  `continue`. It blocks new acceptance and subscription establishment but does
  not rewrite or bulk-cancel accepted command state. Cancellation and
  reauthorization policy belong to the grant-revocation sibling; after
  re-entry the operator may cancel work explicitly.
- **Session secret rotation**: do not add a signing/encryption-secret layer.
  Browser cookies are already opaque identifiers for server-side records, so a
  durable operator-session generation fence gives immediate invalidation with
  fewer secrets and without pretending the current cookie is signed.
- **No-interactive checkpoint**: the operator explicitly requested autonomous
  judgment. The choices above are the least-irreversible sound options: events
  preserve history, restore remains additive, and no accepted work is erased.
- **Codebase mapping**: direct-read only. The source area and established
  current-session/principal-enrollment patterns were bounded enough that an
  Explore dispatch would add handoff cost without resolving a named unknown.

## UI alignment

No mockups. This feature exposes generated RPC routes and scriptable CLI
commands, but its brief and parent epic assign the cockpit emergency-controls
composition to the lockdown sibling. Mocking here would duplicate that safety
surface before its owning feature designs it.

## Architectural choice

Three approaches were considered:

1. **Local mutable revocation only.** Extend `SessionStore` and the core maps
   with booleans. This is small, but endpoint credentials become valid after a
   core restart and audit can drift from the mutation.
2. **Event-sourced revocation fences with process-local session records
   (chosen).** Persist non-secret generation/scope events atomically with audit,
   replay them into the operator/principal projections, and keep opaque session
   tokens process-local. This matches the existing durable-log projection and
   audited-decision patterns while preserving restart fail-closed behavior.
3. **Rotate the shared core/session secret.** This invalidates too much,
   couples unrelated principals, provides no endpoint granularity, and adds key
   lifecycle machinery around cookies that are not currently signed.

The chosen approach makes the core the authority for revocation, with the web
store as a fail-closed browser-session projection. It adds one generated
revocation family rather than three bespoke state paths.

## Implementation Units

### Unit 1: Generated revocation and audit contract
**Files**: `contracts/proto/patchbay/control.proto`,
`contracts/proto/patchbay/admin.proto`, `contracts/proto/patchbay/common.proto`,
`contracts/proto/patchbay/diagnostics.proto`, generated artifacts under
`contracts/rust/src/gen/patchbay/` and `contracts/ts/src/gen/patchbay/`
**Story**: `epic-revocation-lifecycle-session-principal-revocation-contract-model`

```proto
service ControlService {
  rpc RevokeAllOperatorSessions(RevokeAllOperatorSessionsRequest)
      returns (RevokeAllOperatorSessionsResult);
  rpc RevokeControlSurfacePrincipal(RevokeControlSurfacePrincipalRequest)
      returns (RevokeControlSurfaceResult);
  rpc RevokeControlSurfaceEndpoint(RevokeControlSurfaceEndpointRequest)
      returns (RevokeControlSurfaceResult);
}

message RevokeAllOperatorSessionsRequest { string reason_code = 1; }
message RevokeAllOperatorSessionsResult {
  uint32 revoked_session_count = 1;
  Generation invalidated_through_generation = 2;
  EventId revocation_event_id = 3;
}
message RevokeControlSurfacePrincipalRequest {
  string principal_id = 1;
  string reason_code = 2;
}
message RevokeControlSurfaceEndpointRequest {
  oneof target {
    EndpointId endpoint_id = 1;
    DeviceId device_id = 2;
  }
  string reason_code = 3;
}
message RevokeControlSurfaceResult {
  bool newly_revoked = 1;
  uint32 revoked_principal_count = 2;
  uint32 revoked_session_count = 3;
  EventId revocation_event_id = 4;
}
```

`admin.proto` adds non-secret durable `OperatorSessionRevocation` (actor,
`invalidated_through_generation`, verified revoker, timestamp, reason) and
`ControlSurfaceRevocation` (authority domain, oneof principal id / endpoint id /
device id, verified revoker, timestamp, reason). `StoredEventKind` gets one
variant per concrete message. `VerifyOperatorPasswordResult` and
`BootstrapResult` return `operator_session_generation` alongside the opaque id.
`AuditEventKind::OperatorSessionRevoked` remains canonical for current/all
session revocation; add outcome-bearing `ControlSurfacePrincipalRevoked`,
`ControlSurfaceEndpointRevoked`, and `ControlSurfaceDeviceRevoked` kinds. No new
`FailureCode` is needed because these administrative unary RPCs do not create a
`SubmissionOutcome`.

**Implementation Notes**:
- Update the `.proto` first and regenerate both languages; never edit generated
  Rust/TypeScript by hand.
- Require `reason_code` to match `[a-z0-9_]{1,64}`. Missing targets/ids are
  `INVALID_ARGUMENT`; unknown or foreign-actor targets are `NOT_FOUND`;
  inactive compound issuer evidence is `UNAUTHENTICATED`; durable-write
  failure is `UNAVAILABLE`.
- Add `specs/seed/session_principal_revocation.qnt` with independent attempted
  evidence for `RevokeAllInvalidatesPriorSessionGeneration`,
  `PrincipalRevocationPreventsFuture`, `EndpointRevocationPreventsFuture`, and
  `DeviceRevocationPreventsFuture`. Promotion requires checker commands,
  non-vacuity runs, and mutations that remove each acceptance guard and must
  fail. Add traced vectors under `contracts/vectors/`; do not call a property
  checked-normative until its model and promoted vector both clear the gates.

**Acceptance Criteria**:
- [ ] One schema owns every RPC, durable source event, stored-event discriminator,
  session-generation field, and audit kind; Rust/TS drift checks pass.
- [ ] Generated audit values distinguish principal, endpoint, and device scope
  without encoding a principal id into free-form audit text.
- [ ] The model catches acceptance using an invalidated session generation or a
  revoked principal/endpoint/device under guard-removal mutations.

### Unit 2: Replayable core session and principal revocation
**Files**: `server/src/operator_session.rs`, `server/src/state.rs`,
`server/src/issuer.rs`, `server/src/identity.rs`, `server/src/admin_service.rs`,
`server/src/service.rs`, `core/src/authority/operator.rs`,
`core/src/storage/audited.rs`, plus exhaustive `StoredEventKind` consumers under
`core/src/`
**Story**: `epic-revocation-lifecycle-session-principal-revocation-core-state`

```rust
pub struct OperatorSessionBinding {
    pub actor_id: ActorId,
    pub endpoint_id: EndpointId,
    pub device_id: DeviceId,
    pub endpoint_generation: Generation,
}

pub struct IssuedOperatorSession {
    pub id: OperatorSessionId,
    pub session_generation: Generation,
}

struct OperatorSessionRecord {
    binding: OperatorSessionBinding,
    session_generation: Generation,
    created_at: Instant,
    last_used_at: Instant,
    expires_at: Instant,
    revoked_at: Option<Instant>,
}

impl OperatorSessionRegistry {
    pub async fn issue(&self, binding: OperatorSessionBinding)
        -> IssuedOperatorSession;
    pub async fn verify(&self, id: &OperatorSessionId,
        binding: &OperatorSessionBinding) -> bool;
    pub async fn revoke_current(&self, id: &OperatorSessionId,
        binding: &OperatorSessionBinding) -> bool;
    pub async fn observe(&self, event: &RecordedEvent)
        -> Result<(), OperatorSessionError>;
}

impl OperatorRegistry {
    pub fn verify_principal(&self, principal_id: &str, credential: &str)
        -> Option<ControlSurfacePrincipalRecord>;
    pub fn revocation_for_principal(&self, principal_id: &str)
        -> Option<&RecordedControlSurfaceRevocation>;
    pub fn revocation_for_endpoint(&self, endpoint_id: &EndpointId)
        -> Option<&RecordedControlSurfaceRevocation>;
    pub fn revocation_for_device(&self, device_id: &DeviceId)
        -> Option<&RecordedControlSurfaceRevocation>;
}
```

**Implementation Notes**:
- Core session generations are core-assigned, monotonic per operator. Issuance
  uses the next value above the replayed revoke-all floor. Verification updates
  `last_used_at` and requires actor + endpoint + device + endpoint generation to
  match the verified principal, closing the current cross-endpoint session
  evidence gap.
- `OperatorRegistry::observe` folds `ControlSurfaceRevocation`; exact principal
  revocation blocks one credential, while endpoint/device revocation blocks all
  matching current and future same-id principals. Enrollment at a revoked
  endpoint/device fails closed instead of implicitly restoring it.
- Under the existing submit guard, append each durable revocation source and
  its typed audit record in one storage transaction, then warm both the operator
  and operator-session projections. Replay is authoritative if warming fails.
  Duplicate scope revocation returns the existing event/result without another
  source event.
- Revoke matching process-local operator sessions when principal,
  endpoint/device, or revoke-all events fold. Raw session ids, cookies,
  credentials, and CSRF secrets never enter source or audit events.
- Extend every exhaustive `StoredEventKind` fold/filter so authority records are
  replayed but excluded from operator subscription payloads.

**Acceptance Criteria**:
- [ ] Restart replay preserves principal/endpoint/device revocation and the
  operator-session generation floor; opaque pre-restart tokens remain invalid.
- [ ] A session cannot be combined with a principal from another endpoint,
  device, or endpoint generation.
- [ ] Repeated revocation is idempotent, concurrent issuance is serialized, and
  unrelated principals/endpoints remain usable.
- [ ] A command accepted before revocation may still reach its existing valid
  terminal transition; no revocation path rewrites command history.

### Unit 3: Web browser-session projection and generated routes
**Files**: `web-server/src/sessions.ts`,
`web-server/src/middleware/csrf-auth.ts`, `web-server/src/routes/login.ts`,
`web-server/src/routes/rpc.ts`, `web-server/src/main.ts`,
`web-server/tests/sessions.test.ts`, `web-server/tests/integration.test.ts`
**Story**: `epic-revocation-lifecycle-session-principal-revocation-web-session-plane`

```ts
export interface OperatorSession {
  sessionId: string;
  operatorActorId: string;
  endpointId: string;
  deviceId: string;
  sessionGeneration: bigint;
  coreSessionId?: string;
  status: "active" | "revoked" | "expired";
  csrfSecret: string;
  createdAt: number;
  lastUsedAt: number;
  expiresAt: number;
  revokedAt: number | null;
}

export interface SessionIdentity {
  operatorActorId: string;
  endpointId: string;
  deviceId: string;
  sessionGeneration: bigint;
  coreSessionId?: string;
}

create(identity: SessionIdentity): OperatorSession;
revokeAllForOperator(operatorActorId: string): number;
revokeForEndpoint(endpointId: string): number;
revokeForDevice(deviceId: string): number;
```

**Implementation Notes**:
- `coreOperatorAuthenticator` copies endpoint/device/session generation only
  from the generated core result; browser input never supplies these fields.
- Add CSRF-protected gRPC-Web proxy routes for all three new RPCs. A successful
  revoke-all marks every local operator record revoked. Revoke-all also marks
  local records revoked in a `finally` path when the core call fails, so the
  browser plane fails closed; the error remains visible because core-wide
  invalidation is not claimed. Principal/endpoint RPCs apply local invalidation
  only when the current web principal/endpoint/device is the target.
- Every local revoke sets `revokedAt` once and retains the recognized record;
  expiry leaves `revokedAt = null`. Existing unauthenticated-core handling
  continues to delete a dead local bridge session on its next RPC.

**Acceptance Criteria**:
- [ ] Browser records contain every field promised by `SECURITY.md:94`, and
  middleware rejects revoked/expired records before a core call.
- [ ] Revoke-all from the browser invalidates the caller and every sibling
  browser session, clears no audit/history, and requires fresh login.
- [ ] Core unavailability cannot leave the browser records active after a local
  revoke-all attempt; the response does not falsely claim core success.
- [ ] Endpoint/device self-revocation makes subsequent browser RPCs fail closed.

### Unit 4: CLI emergency-control commands and honest re-entry
**Files**: `cli/src/commands/revocation.ts`, `cli/src/main.ts`,
`cli/src/credentials.ts`, `cli/tests/auth-commands.test.ts`,
`cli/tests/output-diagnostics.test.ts`
**Story**: `epic-revocation-lifecycle-session-principal-revocation-cli-controls`

```ts
export async function revokeAllSessionsCommand(
  client: Pick<ControlClient, "revokeAllOperatorSessions">,
  store: CredentialStore,
  options: { reasonCode: string; json: boolean },
  output: CliOutput,
): Promise<number>;

export async function revokePrincipalCommand(
  client: Pick<ControlClient, "revokeControlSurfacePrincipal">,
  store: CredentialStore,
  options: { principalId: string; reasonCode: string; json: boolean },
  output: CliOutput,
): Promise<number>;

export async function revokeEndpointCommand(
  client: Pick<ControlClient, "revokeControlSurfaceEndpoint">,
  store: CredentialStore,
  options: { endpointId?: string; deviceId?: string;
             reasonCode: string; json: boolean },
  output: CliOutput,
): Promise<number>;
```

Commands are `revoke-all-sessions`, `revoke-principal <principal-id>`,
`revoke-endpoint <endpoint-id>`, and `revoke-device <device-id>`, each with
`--reason-code` and `--json`.

**Implementation Notes**:
- Confirmed revoke-all always clears the local credential file because the
  caller's session is invalidated. Principal/endpoint/device commands clear it
  only when the stored credential is inside the confirmed target scope.
- On transport/unknown failure, do not claim success; print that credentials
  may already be invalid and that `patchbay-cli login` is the reconciliation
  path. Never print principal secrets or raw session ids.
- Re-entry is honest: revoke-all leaves the principal live, so a trusted-host
  `patchbay-cli login` with core secret + operator password creates a
  higher-generation session. Self-revoking a principal/endpoint/device requires
  login from a distinct unrevoked identity (or new web endpoint/device config).
  The consumed one-time `setup` secret is not reusable and must not be
  advertised as recovery.

**Acceptance Criteria**:
- [ ] Commands validate exactly one endpoint/device target, bounded reason code,
  and positional/flag grammar before network access.
- [ ] Self-lockout commands remove confirmed-dead local credentials and provide
  an actionable, truthful login/re-entry message.
- [ ] JSON output contains ids/counts/generation only; no credential or session
  secret reaches output or argv guidance.

### Unit 5: Cross-boundary conformance and rolling foundation
**Files**: `server/tests/trust_boundary.rs`, `server/tests/grpc_smoke.rs`,
`core/tests/`, `contracts/vectors/`, `contracts/scripts/check-models.mjs`,
`docs/SECURITY.md`, `docs/PROTOCOL.md`, `docs/VERIFICATION.md`, `docs/UX.md`,
`docs/GLOSSARY.md`, `docs/RUNBOOK.md`
**Story**: `epic-revocation-lifecycle-session-principal-revocation-conformance-foundation`

**Implementation Notes**:
- Add real-process and replay evidence for old-session generation rejection,
  principal/endpoint/device rejection before acceptance, unaffected-principal
  continuity, idempotent repeated scope revocation, and accepted-work
  continuation.
- Roll foundation assertions in place: remove the stale “only #1 implemented”
  status; define operator-session generation separately from runtime-session
  generation; register durable revocation/audit vocabulary and CLI recovery;
  record `continue` as this plane's accepted-work policy. Update generated
  model/vector traceability rather than hand-editing generated blocks.
- Mark `backlog-session-record-fields-gap` absorbed when implementation lands;
  do not create a second follow-up for the same fields.

**Acceptance Criteria**:
- [x] Unit/integration/restart tests prove future Operations and subscriptions
  reject before acceptance after every revocation scope.
- [x] Model, vector, generated-contract, clippy, Rust workspace, TS workspace,
  and generated-drift checks are green with traceable property ids.
- [x] Foundation docs describe the implemented contract and the self-lockout
  recovery boundary without historical migration prose.

## Completion evidence

- Acceptance mapping: contract/model registries and four draft vectors are in `contracts/` and `specs/seed/session_principal_revocation.qnt`; replayable core scope fences and atomic source/audit writes are covered by core/server tests; web records and all three CSRF-protected gRPC-Web routes are covered by 29 web tests; CLI controls, safe output, selective credential cleanup, and recovery guidance are covered by 33 CLI tests; real gRPC scope, subscription, and restart tests are in `server/tests/grpc_smoke.rs`.
- Verification: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, contracts build/vector/model/drift/presentation checks, `cd web-server && npm test`, `cd cli && npm test`, `cd web-cockpit && npm test`, and `cd pi-adapter && npm test` pass. Revocation model properties remain honestly stated-normative draft properties; current checker and mutation/non-vacuity evidence do not promote them without reviewed vectors.
- Deviations: a duplicate endpoint decision-gate acquisition was removed when integrated gRPC tests exercised endpoint/device RPCs; no semantic deviation from the design.
- Parked issues: none. `backlog-session-record-fields-gap` is absorbed and retained as `.work/archive/backlog-session-record-fields-gap.md`; no duplicate follow-up was created.

## Implementation Order

1. `...-contract-model` — pin wire/audit/model registries first.
2. `...-core-state` — implement replayable authority and generation semantics.
3. `...-web-session-plane` and `...-cli-controls` — parallel consumers of the
   stable core contract.
4. `...-conformance-foundation` — integrated restart evidence and rolling docs.

## Simplification

- Generalize the existing `RevokeOperatorSession` service/audit construction
  into bounded revocation helpers; do not add three independent metadata/error
  parsers.
- Keep one `ControlSurfaceRevocation` durable family and one operator registry
  projection for principal/endpoint/device scope.
- Replace boolean-only revocation with `revokedAt` in both session registries
  while retaining the existing status API; remove duplicated “active” tests
  that become subsets of the generation/scope boundary tests.
- Do not add cookie signing, a second revocation database, a hand-written DTO,
  or a separate endpoint denylist outside the replayed operator projection.

## Testing

- **Interface**: generated RPC/trust-boundary tests protect compound-issuer
  authorization, exact gRPC status mapping, self-revocation response delivery,
  and rejection before durable Operation acceptance.
- **Regression**: web tests protect local fail-closed revoke-all and the
  `SECURITY.md:94` record fields; core restart tests protect durable credential
  revocation and generation floors.
- **Property/model**: independent-attempt Quint properties plus guard-removal
  mutations protect against self-defining revocation claims.
- **Conformance**: traced vectors exercise old vs fresh generation, exact
  principal, endpoint/device grouping, and unaffected endpoint cases.
- **Removal**: consolidate duplicate revoked-session happy-path tests after the
  stronger cross-boundary table covers them; retain cookie/CSRF tests because
  they protect a separate browser boundary.

## Risks

- **Self-lockout is intentional and high-impact.** Revoke-all invalidates the
  caller; endpoint/device self-revocation may also strand the current web
  process. The fallback is trusted-host CLI login with a distinct endpoint;
  one-time setup is not a reset mechanism. Implementation and runbook must say
  this before the operator confirms the command.
- **Riskiest assumption — atomic source/audit replay.** Every durable scope
  mutation must use the existing paired writer transaction. A projection-warm
  failure falls back to replay; any path that mutates first and audits later is
  a blocker.
- **Browser/core split can be temporarily asymmetric.** CLI-initiated revoke-all
  cannot directly mutate web-server memory. Core authority is still removed;
  the next browser RPC receives `UNAUTHENTICATED` and deletes its bridge record.
  The web-initiated path revokes local records immediately.
- **Permanent identity fences can surprise recovery.** Same-id enrollment is
  deliberately refused; the operator needs a distinct endpoint/device id. An
  explicit audited restore operation is reserved and can be added without
  changing existing event meaning.
- **Model quality risk.** A green checker is insufficient. Guard-removal
  mutations and independent pre-state attempted evidence are mandatory; if the
  model cannot express that genuinely, keep the properties stated-normative and
  rely on executable tests rather than overclaim promotion.
- **Design-time advisory review unavailable.** This delegated feature-design
  context has no subagent adapter. Implementation remains subject to the
  project-standard fresh-context feature review, and verification-tagged
  checkpoints use the deep lane.

## Extension pressure classification

- **Committed v0.1.0**: actor-scoped revoke-all with core-assigned operator
  session generations; durable principal, endpoint, and device revocation;
  existing `continue` policy for already-accepted work; generated RPC/audit
  registries and executable rejection evidence.
- **Reserved seam**: multi-operator/foreign-authority administration, endpoint
  classes, explicit restore/unrevoke, and session-signing-secret rotation. The
  durable keys retain `authority_domain_id`, actor, endpoint, device, and
  generation demarcators so promotion is additive.
- **Explicitly rejected for this feature**: rotating the shared core secret as
  the ordinary revocation mechanism, deleting command/audit history, and
  silently reactivating a revoked endpoint/device on login.

## Returned to review (2026-07-27, orchestrator)

The implementing worker advanced this feature straight to `done`, skipping
the mandatory independent review pass (explicitly instructed otherwise).
Orchestrator wave verification is green across all suites (cargo 30 + clippy,
cli 33, web-server 29, web-cockpit 67, pi-adapter 24, e2e, drift). Feature
returned to `review` for the standard independent pass before closure.

## Review findings (standard pass 1, 2026-07-27 — independent reviewer: gpt-5.6-sol)

Verdict: blockers-found. Receiver-confirmed blockers (fix before `done`):

1. **Stale-issuer race** — compound issuer verification happens BEFORE
   acquiring CoreDecisionGate; a request can verify, wait out a revocation
   commit, then submit with the cached issuer. Fix: under the gate, catch up
   projections then RE-VERIFY the issuer before every principal-gated
   decision; deterministic race test (revocation between arrival and
   acceptance).
2. **Audit incomplete + misattribution** — valid-but-denied target decisions
   and authentication failures bypass durable audit; idempotent repeats
   return old events unaudited; endpoint-revocation audits overwrite
   `endpoint_id` with the TARGET endpoint (misattributing the action to the
   revoked endpoint instead of the verified revoker). Fix: audit every
   allowed/repeated/denied attempt; verified actor/endpoint/device in
   attribution fields, revoked target in target_scope; audit-query tests for
   denial/repeat/third-endpoint cases.
3. **`revoked_session_count` always 0** — state ingestion already revokes
   matching sessions, handlers invoke revocation again, responses report 0.
   Fix: return the first count (or remove one mutation site); gRPC
   assertions for nonzero + idempotent counts.

Parked notes: Quint model assigns (not monotonic) and doesn't model replay —
parkable, no promotion claimed; CLI device-revocation output could name the
required --device-id more concretely; web-memory/core revoke-all desync is
honest once blocker 1 lands.
