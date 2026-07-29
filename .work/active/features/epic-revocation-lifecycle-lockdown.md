---
id: epic-revocation-lifecycle-lockdown
kind: feature
stage: review
tags: [security, foundation, ui]
parent: epic-revocation-lifecycle
depends_on: [epic-revocation-lifecycle-grant-lifecycle]
release_binding: null
gate_origin: null
created: 2026-07-27
updated: 2026-07-30
---

# Security lockdown & bootstrap-channel exit

## Brief

SECURITY.md commits a durable **security lockdown** posture: reject new
Operations, mark affected runtime sessions stale, require fresh login, and
record the reason — durable across core restart (crash recovery replays the
log and lockdown remains in effect). Exit requires re-establishing bootstrap
trust **via the bootstrap channel** (local CLI/console, distinct from routine
web login — the channel distinction is load-bearing). None of this exists:
there is no lockdown decision surface at all.

This feature delivers lockdown end to end: the operator-facing lockdown
trigger (control-surface action), the core posture (durable event, rejection
gate on new Operations, session staleness marking, login invalidation), the
bootstrap-channel exit (admin-service path, since bootstrap already lives on
the loopback admin listener), and the **lockdown entry/exit audit producers**
— discharging the deferred obligation recorded in
`epic-observability-dogfooding-core-diagnostics` (the `AuditEventKind`
vocabulary already names them; the feature review deferred producers because
no decision surface existed — now it does).

It also owns the **cockpit emergency-controls UX**: the lockdown trigger and
the lockdown-state banner are net-new, safety-critical surfaces. **Mockups
REQUIRED at feature design** (`/ux-ui-design:screens
epic-revocation-lifecycle-lockdown`) — deferred from epic tier to keep
decomposition moving; this is the epic's one net-new UI surface, everything
else composes into existing views.

Does NOT cover: the revocation actions themselves (sibling features);
multi-operator lockdown scope (reserved).

## Epic context

- Parent epic: `epic-revocation-lifecycle`
- Position: sequences after grant-lifecycle (shared submission-authorization
  touchpoints; grant-lifecycle establishes the current enforcement pattern).

## Simplification opportunity

- The lockdown rejection gate should ride the same submission-authorization
  path as grant checks, not a parallel gate.
- Lockdown exit reuses the existing loopback admin/bootstrap channel rather
  than a new channel.

## Foundation references

- `docs/SECURITY.md` — revocation model (#5), Lockdown exit (durability,
  bootstrap-channel requirement, channel-distinction rationale)
- `docs/PROTOCOL.md` — Snapshots and streams (staleness), failure vocabulary
- `docs/UX.md` — emergency-control presentation (to be extended via mockups)

## Mockups

- Screens: `.mockups/screens/epic-revocation-lifecycle-lockdown/index.html`
- Selected: **option-hybrid** (operator sign-off "good MVP", 2026-07-29), after four initial options + iterative revision.
- **UI authority**: `.mockups/screens/epic-revocation-lifecycle-lockdown/option-hybrid.html` — reference this signed-off hybrid; do not re-mock.
- **Navigation architecture** (locked, applies cockpit-wide; grounded in
  `.research/analysis/briefs/cockpit-navigation-architecture.md`):
  icon-only left rail as the canonical desktop form (VS Code activity-bar
  model), left-accent highlighter for the active destination, destinations
  punch out contextual panels (Sessions ↔ session list; future Files/Git
  panels during chat), bottom tab bar on mobile (equal-width icon+label
  items, top-accent indicator, hamburger "More" overflow), drill-in with
  back affordance on mobile, inspector material as sheets/subroutes.
- **Security screen**: single-column flow — lockdown hero (two-step
  arm-then-confirm ritual), operator sessions, endpoints/devices, grants.
- **Lockdown state**: inline persistent banner over a read-only cockpit
  (reason, timestamp, bootstrap-exit instructions) — NOT a takeover
  interstitial; all actions disabled with lock reasons; server-side
  enforcement is authoritative (UI disabling is presentation).
- **Sessions/chat pane**: production `session-detail` structure and
  `shell.css` ported verbatim (msg/delivery/composer parity, attach button,
  Enter-to-send, auto-grow input, timeline activity indicator).
- Collapse discipline: one control per region — rail destinations drive
  panel punch-out; no separate panel chevrons.
- Implementer flags recorded: overlay/stale/motion tokens to add to
  tokens.css; promote session-row + stale treatment into components.css;
  persist rail/panel collapse state per user; lockdown exit is
  bootstrap-channel only.

## Design decisions

- **Operation rejection scope**: while active, reject every `ControlService.Submit` and `QueryDiagnostics` submission for every committed `OperationKind`, including exact retries, with `SubmissionOutcome::Rejected`, `FailureCode::AuthorizationDenied`, and reason code `security_lockdown_active`; no command is appended. Already-accepted Operations and adapter terminal reports continue under their existing lifecycle. A retry is still an authenticated submission attempt and is refused rather than allowed to probe old command state; snapshots remain the read path.
- **Non-Operation reads and subscriptions**: `Subscribe`, `LoadSnapshot`, and the new `LoadSecuritySnapshot` remain available after a fresh login because PROTOCOL explicitly classifies Subscription and snapshots outside Operation lifecycle, and the signed-off cockpit must remain readable. `QueryDiagnostics` is an Operation and therefore rejects. Logout/current-session revocation and required audit ingress remain available; grant/session/principal/enrollment mutations return `FAILED_PRECONDITION` while locked.
- **Fresh login**: lockdown entry raises the durable operator-session generation floor through the lockdown event and invalidates every existing browser and CLI session. `VerifyOperatorPassword` remains available and issues a higher-generation session, which may inspect the read-only cockpit but cannot clear lockdown or submit Operations. This reconciles “require fresh login” with the signed-off read-only cockpit.
- **Trigger authorization**: `EnterSecurityLockdown` is a non-Operation administrative RPC but requires a current compound issuer plus a live `session-management` grant at authority-domain scope. The existing bootstrap grant satisfies this; a session-target descendant grant does not. This reuses the generated authority registry without inventing a lockdown OperationKind or treating UI confirmation as authority.
- **Bootstrap-only exit**: `ExitSecurityLockdown` exists only on `AdminService`, whose listener is separately configured, loopback-only, and absent from the web-server bridge. It requires no now-invalid operator credential; possession of the configured bootstrap channel is the authorization boundary. Routine password reauthentication can create a read-only session but has no RPC path that exits lockdown.
- **Reason and redaction**: persist only required `reason_code: [a-z0-9_]{1,64}` values, never arbitrary free text. The cockpit humanizes the code for display; audit and durable state retain the exact safe code. This records an actionable reason while structurally excluding secrets, prompt content, paths, tokens, and attachments from the event.
- **Session staleness**: one durable lockdown-entry event is folded by the session projection: every current runtime session becomes `stale` at that event LSN, and session reports received while locked are normalized to `stale`. Exit does not fabricate liveness; a post-exit adapter report or authoritative snapshot must move a session back to `live`/`offline`/`failed`.
- **Entry idempotency and exit idempotency**: repeated entry while active and repeated exit while inactive return the current posture without another posture source event. A first state change atomically appends exactly one posture event and the existing `LOCKDOWN_ENTERED` or `LOCKDOWN_EXITED` audit kind. Denied attempts use `AUTHORIZATION_FAILED`; storage/audit failure leaves the prior posture authoritative.
- **Single-operator scope**: lockdown is authority-domain keyed even though v0.1.0 has one domain/operator. Per-operator or partial-target lockdown is reserved for multi-operator authority design; no global process boolean without a domain key is introduced.
- **Autonomous checkpoint**: the operator was unavailable by instruction. These choices prefer the deny-by-default, least-irreversible path and preserve the load-bearing bootstrap-channel distinction rather than blocking for questions.

## Codebase mapping

Direct reading covered the landed grant-lifecycle and session/principal-revocation implementations, generated contracts, acceptance pipeline, `CoreDecisionGate`, audited storage transactions, independent control/adapter session projections, loopback `AdminService`, web-server gRPC-Web bridge, CLI clients, and the production cockpit shell. The feature is broad, but this delegated environment exposes no exploratory or advisory subagent tool; direct mapping and the normal feature review are the available path.

## Architectural choice

### Options considered

1. **Event-sourced authority-domain posture with a domain-owned acceptance port (chosen).** Add one generated lockdown event family and replay it into a `SecurityPostureProjection`, `SessionRegistry`, and `OperatorSessionRegistry`. Acceptance receives a narrow `OperationPosture` port adjacent to `GrantCheck`; server writers share `CoreDecisionGate`. This gives restart durability, atomic audit, deterministic stale-session/login effects, and one enforcement point for Submit and QueryDiagnostics.
2. **Fan out revoke-all sessions and grants.** Reuse existing revocation RPCs and infer lockdown from “nothing remains live.” This loses an explicit durable posture, cannot distinguish incident containment from ordinary attenuation, makes exit an unsafe grant-reconstruction exercise, and cannot keep newly issued sessions read-only.
3. **Server middleware flag or dedicated lockdown table.** Gate each RPC against a server-owned boolean/row. This is locally small but creates a second writer outside the event log, misses direct core acceptance callers and adapter/session replay, and makes restart/snapshot convergence backend-shaped.

The chosen event-native posture is the only option that preserves Patchbay's one-log authority and the sibling durable-projection pattern. The event is the source; audit, snapshots, web presentation, and login/session effects are derived folds. The trickiest unit is the **self-lockout/re-entry path**: entry must invalidate the caller only after the durable posture/audit commit, while the admin exit must remain callable without those credentials after restart. That path is designed and tested before cockpit polish.

## Implementation Units

### Unit 1: Generated lockdown, snapshot, and RPC contracts

**Files**: `contracts/proto/patchbay/security.proto` (new), `contracts/proto/patchbay/common.proto`, `contracts/proto/patchbay/sessions.proto`, `contracts/proto/patchbay/control.proto`, `contracts/proto/patchbay/admin.proto`, `contracts/rust/src/gen/patchbay/patchbay.rs`, `contracts/ts/src/gen/patchbay/security_pb.ts`, `contracts/ts/src/gen/patchbay/common_pb.ts`, `contracts/ts/src/gen/patchbay/sessions_pb.ts`, `contracts/ts/src/gen/patchbay/control_pb.ts`, `contracts/ts/src/gen/patchbay/admin_pb.ts`

**Story**: `epic-revocation-lifecycle-lockdown-core-posture`

```proto
// security.proto
enum BootstrapChannelKind {
  BOOTSTRAP_CHANNEL_KIND_UNSPECIFIED = 0;
  BOOTSTRAP_CHANNEL_KIND_LOOPBACK_ADMIN = 1;
}

message SecurityLockdownEvent {
  AuthorityDomainId authority_domain_id = 1;
  oneof transition {
    SecurityLockdownEntered entered = 2;
    SecurityLockdownExited exited = 3;
  }
}
message SecurityLockdownEntered {
  string reason_code = 1;
  google.protobuf.Timestamp occurred_at = 2;
  ActorEndpointRef entered_by = 3;
  Generation invalidated_through_operator_session_generation = 4;
  uint32 affected_runtime_session_count = 5;
}
message SecurityLockdownExited {
  string reason_code = 1;
  google.protobuf.Timestamp occurred_at = 2;
  EventId entered_event_id = 3;
  BootstrapChannelKind bootstrap_channel = 4;
}
message SecurityLockdownState {
  bool active = 1;
  string reason_code = 2;
  google.protobuf.Timestamp entered_at = 3;
  ActorEndpointRef entered_by = 4;
  EventId entered_event_id = 5;
}

message SecuritySnapshot {
  AuthorityDomainId authority_domain_id = 1;
  Lsn snapshot_lsn = 2;
  SecurityLockdownState lockdown = 3;
  repeated OperatorSessionSummary operator_sessions = 4;
  repeated ControlSurfaceSummary control_surfaces = 5;
  repeated GrantSummary grants = 6;
}
```

```proto
// common.proto
STORED_EVENT_KIND_SECURITY_LOCKDOWN = 14;

// control.proto
rpc EnterSecurityLockdown(EnterSecurityLockdownRequest)
    returns (EnterSecurityLockdownResult);
rpc LoadSecuritySnapshot(LoadSecuritySnapshotRequest)
    returns (LoadSecuritySnapshotResponse);

message EnterSecurityLockdownRequest {
  AuthorityDomainId authority_domain_id = 1;
  string reason_code = 2;
}
message EnterSecurityLockdownResult {
  SecurityLockdownState lockdown = 1;
  EventId lockdown_event_id = 2;
  bool already_active = 3;
  uint32 affected_runtime_session_count = 4;
  Generation invalidated_through_operator_session_generation = 5;
}
message LoadSecuritySnapshotRequest { AuthorityDomainId authority_domain_id = 1; }
message LoadSecuritySnapshotResponse { SecuritySnapshot snapshot = 1; }

// admin.proto / AdminService
rpc ExitSecurityLockdown(ExitSecurityLockdownRequest)
    returns (ExitSecurityLockdownResult);
message ExitSecurityLockdownRequest {
  AuthorityDomainId authority_domain_id = 1;
  string reason_code = 2;
}
message ExitSecurityLockdownResult {
  SecurityLockdownState lockdown = 1;
  EventId lockdown_event_id = 2;
  bool already_inactive = 3;
  EventId entered_event_id = 4;
}
```

**Implementation notes**:

- `OperatorSessionSummary` exposes actor/endpoint/device/operator-session generation and active/revoked/expired status, never the opaque session id/hash. `ControlSurfaceSummary` exposes safe ids/generation/revoked status, never credential hashes. `GrantSummary` exposes id, subject, target, generated OperationKinds, expiry/revocation/policy, never provenance free text.
- Add the lockdown event to every exhaustive `StoredEventKind` consumer. It is operator-facing through `Subscribe`; source event payload is safe by construction.
- Keep `AuditEventKind::{LockdownEntered,LockdownExited}` unchanged: the generated vocabulary already exists. Generate Rust and TypeScript from proto; never hand-edit artifacts.
- `SessionSnapshot` also carries `SecurityLockdownState lockdown = 7` so the ordinary reconnect snapshot cannot present live sessions without the posture that made them stale. `SecuritySnapshot` supplies the dedicated security-screen inventory.

**Acceptance criteria**:

- [ ] One schema owns posture transitions, bootstrap channel, stored-event identity, snapshot state, and all three RPCs.
- [ ] No lockdown wire message has arbitrary text, secret, cookie/session id, credential hash, prompt, attachment, or generic metadata fields.
- [ ] Generated Rust/TypeScript and drift checks pass; unknown transition/channel values fail closed.

### Unit 2: Durable posture projection, submission gate, session clamp, and model evidence

**Files**: `core/src/security/mod.rs` (new), `core/src/security/events.rs` (new), `core/src/security/projection.rs` (new), `core/src/security/replay.rs` (new), `core/src/lib.rs`, `core/src/acceptance/ports.rs`, `core/src/acceptance/pipeline.rs`, `core/src/session/registry.rs`, `core/src/session/ingest.rs`, `core/src/session/replay.rs`, `server/src/operator_session.rs`, `server/src/state.rs`, `specs/seed/security_lockdown.qnt` (new), `contracts/vectors/lockdown-rejects-operation.json` (new), `contracts/vectors/lockdown-replay-persists.json` (new), `contracts/vectors/lockdown-stales-sessions.json` (new), `contracts/vectors/lockdown-bootstrap-only-exit.json` (new)

**Story**: `epic-revocation-lifecycle-lockdown-core-posture`

```rust
// core/src/acceptance/ports.rs
pub trait OperationPosture: Send + Sync {
    fn check(
        &self,
        authority_domain_id: &AuthorityDomainId,
    ) -> impl Future<Output = Result<(), OperationPostureDenied>> + Send;
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OperationPostureDenied {
    #[error("security lockdown is active: {reason_code}")]
    SecurityLockdown {
        reason_code: String,
        entered_event_id: EventId,
    },
}

// core/src/security/projection.rs
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SecurityPostureProjection {
    active: Option<ActiveSecurityLockdown>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveSecurityLockdown {
    pub authority_domain_id: AuthorityDomainId,
    pub reason_code: String,
    pub entered_at: prost_types::Timestamp,
    pub entered_by: ActorEndpointRef,
    pub entered_event_id: EventId,
    pub invalidated_through_operator_session_generation: Generation,
}

impl SecurityPostureProjection {
    pub fn observe(&mut self, event: &RecordedEvent) -> Result<(), SecurityError>;
    pub fn active(&self) -> Option<&ActiveSecurityLockdown>;
    pub fn state(&self) -> SecurityLockdownState;
}

pub async fn rebuild_from_log<S: Storage>(
    storage: &S,
    authority_domain_id: &AuthorityDomainId,
) -> Result<SecurityPostureProjection, SecurityError>;
```

**Implementation notes**:

- `submit_with_clock` accepts `OperationPosture`. Order is: generated envelope/kind/time validation → verified issuer → posture check → response-contract/grant/target/dedup/append. An active posture returns the committed pre-acceptance rejection without target/grant existence leakage. All production Submit and QueryDiagnostics paths pass the replayed projection; no server-only shortcut may be the sole gate.
- `SessionRegistry::observe(SecurityLockdownEntered)` verifies `affected_runtime_session_count == current live-session count`, changes each connectivity axis to `Stale`, and advances each record's `last_authoritative_lsn` to the lockdown LSN. While active, `ingest_session_report` normalizes every registration or connectivity report to `Stale`; exit clears the clamp but leaves existing records stale until later authority arrives.
- `OperatorSessionRegistry::observe` applies the entered event's generation floor exactly as revoke-all does. Issuance remains monotonic above the floor; opaque pre-entry ids are not restored on restart.
- `ProjectionState` holds a locked `SecurityPostureProjection`, folds it in `rebuild`/`catch_up`, exposes it as `OperationPosture`, and includes its state in both snapshots. The adapter service folds the same event into its independent `SessionRegistry` under the shared decision gate before accepting reports.
- The Quint model uses independent attempted-operation, attempted-exit-channel, pre-entry sessions, committed-log, and replay variables. Properties are `LockdownRejectsNewOperations`, `LockdownReplayPersists`, `LockdownEntryStalesSessions`, `LockdownInvalidatesExistingOperatorSessions`, and `BootstrapOnlyExit`. Guard-removal mutations must fail. Promote only with reviewed executable vectors; a green self-defining formula is not evidence.

**Acceptance criteria**:

- [ ] Active lockdown rejects every OperationKind and exact retry before acceptance with `authorization_denied/security_lockdown_active`; zero command events are created.
- [ ] Restart replay restores active posture, stale runtime sessions, and the operator-session generation floor.
- [ ] Adapter reports cannot make a session render live while locked; exit alone also cannot fabricate live state.
- [ ] Already-accepted command transitions remain valid and terminal-final.
- [ ] Model checks, mutation checks, and traced executable vectors establish the named boundaries without overclaiming unrelated behavior.

### Unit 3: Authorized entry, loopback-only exit, atomic audit, and security snapshot

**Files**: `core/src/security/ingest.rs` (new), `core/src/storage/port.rs`, `core/src/storage/audited.rs`, `core/src/storage/rusqlite.rs`, `server/src/state.rs`, `server/src/service.rs`, `server/src/admin_service.rs`, `server/src/adapter_service.rs`, `server/src/main.rs`, `server/tests/grpc_smoke.rs`, `server/tests/trust_boundary.rs`, `server/tests/lockdown_recovery.rs` (new)

**Story**: `epic-revocation-lifecycle-lockdown-trigger-exit-rpcs`

```rust
// core/src/security/ingest.rs
pub async fn ingest_lockdown_transition<S: Storage, P: SecurityPostureProjectionPort>(
    storage: &S,
    projection: &mut P,
    authority_domain_id: &AuthorityDomainId,
    transition: SecurityLockdownEvent,
    audit: AuditRecordDraft,
) -> Result<EventId, SecurityError>;

// server/src/state.rs
impl ProjectionState {
    pub async fn lockdown_state(&self) -> SecurityLockdownState;
    pub async fn require_operations_open(&self) -> Result<(), ActiveSecurityLockdown>;
    pub async fn materialize_security_snapshot(
        &self,
        authority_domain_id: AuthorityDomainId,
    ) -> SecuritySnapshot;
    pub async fn current_runtime_session_count(&self) -> u32;
}
```

**Implementation notes**:

- `EnterSecurityLockdown`: validate domain/reason, pre-verify issuer, acquire the composition-root `CoreDecisionGate`, catch up every projection, re-verify issuer, authorize `OperationKind::SessionManagement` against authority-domain scope at one clock sample, then derive generation/session counts and append. Build `entered_by` solely from verified issuer evidence.
- Use existing `Storage::append_decision`/audited decorator so the posture source and `LockdownEntered` audit commit atomically before hot projection/session invalidation. Audit includes verified actor/endpoint/device, authority-domain target, safe reason code, and source event. A failed append returns `UNAVAILABLE` and leaves sessions usable because no posture committed.
- The entry response is constructed from the committed fold, then web/CLI clients clear their local sessions. Repeated entry returns the existing state/event and `already_active=true`; a denied trigger creates `AuthorizationFailed` audit and no posture event.
- Under lockdown, `RevokeGrant`, revoke-all/principal/endpoint/device, and principal enrollment fail with `FAILED_PRECONDITION` plus `AuthorizationFailed/security_lockdown_active`. `Submit`/`QueryDiagnostics` use the acceptance rejection. `Subscribe`, `LoadSnapshot`, `LoadSecuritySnapshot`, `VerifyOperatorPassword`, current-session revoke/logout, and safe audit ingress remain available to a freshly authenticated session.
- `ExitSecurityLockdown` is registered only on `AdminServiceServer` at `PATCHBAY_ADMIN_BIND_ADDR`. The CLI's `makeAdminClient` loopback assertion remains mandatory; the web server gets no bridge route or client method. Under `CoreDecisionGate`, replay current posture, require an active entered event, append `SecurityLockdownExited` with `BootstrapChannelKind::LoopbackAdmin` and atomic `LockdownExited` audit, catch up, and return inactive state. Storage failure remains locked. Already inactive is an idempotent no-op.
- Adapter attach/terminal reporting remains available so accepted work and recovery evidence are not lost, but session reports fold through the active stale clamp. New operator Operations never reach delivery.
- `LoadSecuritySnapshot` is a snapshot read, not a no-lifecycle query Operation. It requires fresh compound issuer and a live Query/authority-domain grant, reads under the decision gate at one LSN, and returns only the redacted summaries defined above; it remains available while locked.

**Acceptance criteria**:

- [ ] A weak/session-scoped grant cannot trigger authority-domain lockdown; bootstrap broad authority can.
- [ ] Entry source/audit/session invalidation are all-or-nothing from the caller's observable boundary.
- [ ] There is no ControlService or web route that exits lockdown; password reauthentication never changes posture.
- [ ] `patchbay-cli lockdown-exit` can call the loopback admin RPC after entry and after an ungraceful core restart without reading an operator credential file.
- [ ] Exit source/audit commit before Operations reopen; a failed exit leaves the posture active.
- [ ] Security snapshots expose no raw bearer/session/audit-sensitive data.

### Unit 4: Web-server lockdown bridge and local fail-closed sessions

**Files**: `web-server/src/routes/rpc.ts`, `web-server/src/sessions.ts`, `web-server/src/main.ts`, `web-server/tests/integration.test.ts`, `web-server/tests/sessions.test.ts`

**Story**: `epic-revocation-lifecycle-lockdown-trigger-exit-rpcs`

```ts
// gRPC-Web routes
POST /patchbay.ControlService/EnterSecurityLockdown
POST /patchbay.ControlService/LoadSecuritySnapshot
POST /patchbay.ControlService/RevokeGrant
// Existing sibling revocation routes remain composed into Security.
```

**Implementation notes**:

- `EnterSecurityLockdown` and every security mutation uses the existing authenticated session + CSRF + custom-header + Origin/Fetch Metadata guard. `LoadSecuritySnapshot` is authenticated but read-only and does not require CSRF.
- On a confirmed entry, revoke every local browser session for the operator after encoding the response. On transport/unknown failure, revoke the caller's local browser session fail-closed and report that core posture must be reconciled by fresh login; never claim entry succeeded.
- Add the existing `RevokeGrant` unary bridge because the signed-off Security view composes the landed sibling control. Do not add an `ExitSecurityLockdown` bridge, generic admin proxy, or browser access to `PATCHBAY_CORE_ADMIN_ADDR`.
- Map `FAILED_PRECONDITION/security lockdown active` without invalidating a fresh read-only session. `UNAUTHENTICATED` still invalidates dead bridge sessions.

**Acceptance criteria**:

- [ ] Browser entry requires CSRF and cannot self-assert actor/endpoint attribution.
- [ ] Confirmed entry invalidates all local browser sessions and the next cockpit access takes the login path.
- [ ] Fresh login can load read-only session/security snapshots and subscribe; every mutation stays server-rejected.
- [ ] Route inventory proves no browser-reachable bootstrap exit exists.

### Unit 5: Cockpit destination shell and security screen

**Files**: `web-cockpit/src/domain/model.ts`, `web-cockpit/src/domain/reconcile.ts`, `web-cockpit/src/domain/protocol-client.ts`, `web-cockpit/src/ui/icons.ts`, `web-cockpit/src/ui/shell.ts`, `web-cockpit/src/ui/shell.css`, `web-cockpit/src/ui/security-view.ts` (new), `web-cockpit/src/main.ts`, `web-cockpit/tests/model.test.ts`, `web-cockpit/tests/reconcile.test.ts`, `web-cockpit/tests/shell.test.ts`, `web-cockpit/tests/security-view.test.ts` (new), `web-cockpit/tests/main.test.ts`, `.mockups/design-system/tokens.css`, `.mockups/design-system/components.css`

**Story**: `epic-revocation-lifecycle-lockdown-cockpit-shell-ui`

```ts
export type CockpitDestination =
  | "sessions" | "security" | "diagnostics" | "files" | "git" | "settings";

export interface LockdownView {
  active: boolean;
  reasonCode?: string;
  enteredAt?: Date;
  enteredEventLsn?: bigint;
}

export interface CockpitShellPreferences {
  sessionsPanelCollapsed: boolean;
}
export interface CockpitShellPreferenceStore {
  load(authorityDomainId: string): CockpitShellPreferences;
  save(authorityDomainId: string, value: CockpitShellPreferences): void;
}

export interface SecurityViewActions {
  enterLockdown(reasonCode: string): Promise<void>;
  revokeAllSessions(): Promise<void>;
  revokePrincipal(principalId: string): Promise<void>;
  revokeEndpoint(endpointId: string): Promise<void>;
  revokeDevice(deviceId: string): Promise<void>;
  revokeGrant(grantId: string): Promise<void>;
}
```

**Implementation notes**:

- Port the signed-off hybrid topology, not its preview JS: desktop icon-only activity rail with left active accent; Sessions destination punches the session-list panel in/out; detail remains the existing `renderSessionDetail`; Security is a single-column destination; planned destinations show honest unavailable panels. Clicking the active Sessions destination is the sole panel-collapse control. Persist panel state in namespaced local storage through the preference port; narrow layouts ignore a restored desktop split.
- Mobile uses equal-width Sessions/Security/More bottom tabs with top active accent, safe-area padding, list→detail drill-in/back, and planned destinations in More. Existing Elicitation inspector remains a sheet; no three-column mobile layout is introduced.
- Fold `SecurityLockdownEvent` and snapshot posture into `PresentationModel.lockdown`. Render one persistent inline danger banner above the workspace with humanized reason code, entered timestamp, authority domain, and literal `patchbay-cli lockdown-exit` instruction. It never becomes a takeover interstitial.
- Active posture disables composer, cancel/interrupt, Elicitation responses, spawn/attach, revocation buttons, and diagnostics query refresh with a visible “read-only during lockdown” reason. `main.ts` also refuses to construct/dispatch Operations while active. This is defense-in-depth only; server rejection is tested separately.
- Security view follows the mock exactly: lockdown hero, operator sessions, endpoints/devices, grants. Entry is two deliberate steps: Arm dialog, then confirmation dialog requiring exact `LOCKDOWN`; reason code is selected/entered as safe lower-snake-case and displayed humanized. There is no exit button.
- Promote stale session-row treatment and new overlay/stale/motion values into the design-system token/component source rather than maintaining cockpit-local protocol-state CSS. Keep production message/delivery/composer markup unchanged.
- Add CSRF interception for `EnterSecurityLockdown` and all security mutations. After confirmed entry, set the local posture immediately, render banner/read-only state, then accept the expected session-expiry/login transition.

**Acceptance criteria**:

- [ ] Desktop rail/punch-out, mobile bottom tabs/More, and drill-in/back match the signed-off mock at content-driven 760px behavior.
- [ ] Lockdown banner is persistent and inline; the cockpit remains readable and every action explains why it is disabled.
- [ ] Exact `LOCKDOWN` confirmation is required before the network call; cancelling either step makes no call.
- [ ] Presentation folds never render a locked session live and never invent a protocol state.
- [ ] No browser UI or transport code can invoke lockdown exit.
- [ ] Accessibility tests cover rail/tab names, `aria-current`, modal focus/labels, alert announcement, keyboard operation, reduced motion, and disabled-control explanations.

### Unit 6: CLI entry and bootstrap-channel exit

**Files**: `cli/src/commands/lockdown.ts` (new), `cli/src/main.ts`, `cli/src/core-client.ts`, `cli/src/credentials.ts`, `cli/src/output.ts`, `cli/tests/auth-commands.test.ts`, `cli/tests/scripting-commands.test.ts`, `cli/tests/output-diagnostics.test.ts`

**Story**: `epic-revocation-lifecycle-lockdown-cli-conformance`

```ts
export async function lockdownEnterCommand(
  client: Pick<ControlClient, "enterSecurityLockdown">,
  store: CredentialStore,
  authorityDomainId: string,
  options: { reasonCode: string; confirm: string; json: boolean },
  output: CliOutput,
): Promise<number>;

export async function lockdownExitCommand(
  client: Pick<AdminClient, "exitSecurityLockdown">,
  authorityDomainId: string,
  options: { reasonCode: string; json: boolean },
  output: CliOutput,
): Promise<number>;
```

**Implementation notes**:

- Grammar: `lockdown-enter --reason-code CODE --confirm LOCKDOWN [--json]` and `lockdown-exit [--reason-code CODE] [--json]`. Entry requires explicit confirmation and current credentials; exit intentionally reads no credential store and works as the literal mock instruction through `makeAdminClient`'s loopback-only address check.
- Confirmed entry clears the local credential file after a valid committed/already-active result and instructs `patchbay-cli login` for read-only inspection or `patchbay-cli lockdown-exit` for recovery. Unknown transport outcome never claims success and warns that credentials may already be invalid.
- Exit prints the source event, prior entered event, bootstrap channel, and active=false, then directs the operator to `patchbay-cli login`. It never accepts setup secret/password flags and never silently falls back to the routine ControlService address.
- Human and JSON output use canonical lower-snake-case reason/channel values and decimal-string LSN/generation values. Exit codes: `0` committed/idempotent success; `2` authenticated entry denial; `1` local validation, transport, unavailable, malformed result, or exit failure.

**Acceptance criteria**:

- [ ] `lockdown-enter` cannot run without exact confirmation and never prints secret/session material.
- [ ] `lockdown-exit` succeeds with no credential file against the configured loopback admin listener and fails locally for a non-loopback admin address.
- [ ] Malformed/contradictory responses and unknown outcomes never print success or reopen local Operation commands.
- [ ] CLI guidance never advertises the consumed one-time setup secret as recovery.

### Unit 7: Cross-boundary recovery, conformance, and rolling foundation

**Files**: `core/tests/security_lockdown.rs` (new), `core/tests/acceptance_pipeline.rs`, `core/tests/session_registry.rs`, `core/tests/session_replay.rs`, `core/tests/rusqlite_storage.rs`, `server/tests/lockdown_recovery.rs` (new), `server/tests/grpc_smoke.rs`, `server/tests/trust_boundary.rs`, `web-server/tests/integration.test.ts`, `web-cockpit/tests/security-view.test.ts` (new), `cli/tests/auth-commands.test.ts`, `contracts/vectors/lockdown-*.json`, `contracts/scripts/check-models.mjs`, `docs/SECURITY.md`, `docs/PROTOCOL.md`, `docs/VERIFICATION.md`, `docs/UX.md`, `docs/GLOSSARY.md`, `docs/RUNBOOK.md`

**Story**: `epic-revocation-lifecycle-lockdown-cli-conformance`

**Implementation notes**:

- Build one real-process recovery test: authenticate web/CLI, accept an Operation, enter lockdown, prove existing session evidence and every new Operation reject, prove accepted work may terminalize, kill/restart core, prove posture/session staleness persists, call admin exit with no credential file, log in at a higher generation, report fresh adapter state, and accept a new Operation.
- Add barrier-controlled entry versus Submit and entry versus adapter-live-report tests under the shared decision gate. Whichever event commits first determines behavior; no post-entry Operation acceptance or live session projection is allowed.
- Inject transaction failure for entry and exit source/audit writes. Entry failure leaves the old sessions/authority usable; exit failure remains locked. Audit queries must show one typed producer per committed transition with safe attribution/channel and no raw reason text.
- Roll foundation docs in place: SECURITY specifies exact coverage/channel/reason semantics and removes the “owned by feature” status; PROTOCOL registers posture rejection/read exceptions; VERIFICATION records property/vector tier honestly; UX registers shell/banner/security/CLI behavior; RUNBOOK documents local recovery without setup-secret reuse.

**Acceptance criteria**:

- [ ] The self-lockout/restart/bootstrap-exit path passes against the real listeners and persistent store.
- [ ] Mutations that remove acceptance gating, session clamp, generation invalidation, replay fold, bootstrap-channel check, or source/audit atomicity fail independent tests.
- [ ] Foundation and generated traceability describe the landed behavior without claiming a stronger model/vector tier than evidence supports.

## Implementation Order

1. `epic-revocation-lifecycle-lockdown-core-posture` — generated contracts, durable projections, acceptance/session/login semantics, formal model and seed vectors.
2. `epic-revocation-lifecycle-lockdown-trigger-exit-rpcs` — authorized entry, admin-only exit, atomic producers, security snapshot, web bridge.
3. In parallel after 2:
   - `epic-revocation-lifecycle-lockdown-cockpit-shell-ui` — first production consumer of the signed-off navigation/security shell.
   - `epic-revocation-lifecycle-lockdown-cli-conformance` — CLI recovery path, real-process/conformance evidence, and rolling foundation.
4. Feature-level integrated review after all four checkpoints; child stories verify directly to done.

The feature remains one cohesive ownership/review bundle despite heterogeneous checkpoints. The two final stories may proceed in parallel because their write sets are cockpit/web presentation versus CLI/tests/docs; coordinate any generated-contract imports from the already-landed RPC checkpoint.

## Simplification

- Reuse the shared `CoreDecisionGate`, core `Clock`, authority-domain Query/SessionManagement grant checks, operator-session generation floor, `append_decision` atomic source+audit transaction, `SessionRegistry` replay fold, loopback `AdminService`, and existing lockdown audit enum values.
- Add one `OperationPosture` port and one event family; do not scatter `if lockdown` across OperationKind handlers, create a second lockdown table, revoke/reissue every grant, add a lockdown OperationKind/CommandState, or model lockdown as an Elicitation.
- Consolidate session staleness under the lockdown event fold instead of appending a best-effort per-session batch. The event count is replay-validated and newly discovered sessions use the same clamp.
- Consolidate the cockpit's current sidebar/detail layout into the signed-off destination shell rather than layering a second navigation system around it. Preserve `session-detail` and its tests.
- Add only safe summary DTOs needed by the Security screen; never expose full operator-session/principal records or infer inventory from audit prose.
- Intentionally retain distinct control and bootstrap services/listeners: merging them would delete the load-bearing channel distinction, not simplify it.

## Testing

- **Interface tests**: generated gRPC and gRPC-Web tests protect trigger authorization, reason validation, exact status mapping, fresh-login read access, mutation denial, and the absence of a web exit route.
- **Recovery regression**: the real-process enter → restart → admin exit → higher-generation login test protects the critical self-lockout path and is a release blocker for this feature.
- **Property/model evidence**: independent attempted inputs and guard-removal mutations protect rejection, replay durability, session stale dominance, generation invalidation, and bootstrap-only exit. Vectors trace wire fields to executable outcomes.
- **Concurrency tests**: deterministic barriers protect entry versus Submit/QueryDiagnostics and entry versus adapter live reporting under the shared gate.
- **Storage tests**: fault-injected atomic source/audit writes prove no unaudited posture and no reopen-before-exit-commit.
- **Presentation tests**: DOM/property tests protect destination topology, responsive drill-in, inline banner, exact confirmation ritual, lockdown action-disable coverage, and stale-never-live behavior.
- **CLI tests**: grammar, no-credential exit, loopback enforcement, credential cleanup, malformed responses, human/JSON output, and exit codes protect the recovery surface.
- **Test removal**: replace sidebar-only topology assertions with destination-shell assertions; do not retain duplicate mock-shape tests or implementation-bound per-button snapshots. Retain existing session-detail, cookie/CSRF, and registry presentation tests because they protect separate boundaries.

## Risks

- **Critical — self-lockout exit does not actually work**: entry intentionally destroys the only routine sessions, so any hidden credential dependency or missing admin-listener wiring strands the operator. Mitigation is to land/test the admin exit before UI, require no credential store, and run the persistent real-process restart scenario. No routine-web bypass is an acceptable fallback; if exit fails, do not ship entry.
- **Race between posture and writers**: an Operation or live adapter report could commit after an entry plan read. The shared `CoreDecisionGate`, catch-up/re-verify sequence, and barrier tests establish one durable order. A service-local lock is insufficient.
- **Projection disagreement**: control and adapter services currently own separate session registries. Every registry must fold the same lockdown event, and adapter ingress must catch up under the shared gate; otherwise one snapshot may show stale while another report restores live.
- **Read-only exception creep**: allowing Subscribe/snapshots is necessary for the signed-off cockpit, but QueryDiagnostics or a future “read” Operation must not bypass the posture. The generated `OperationPosture` gate applies to all OperationKinds; only named non-Operation snapshot/subscription RPCs are exceptions.
- **Reason leakage**: a free-text incident reason could capture secrets. The schema structurally permits only a bounded safe reason code; humanization is presentation-only.
- **Exit-channel erosion**: adding an admin proxy route, using routine password proof for exit, or merging listeners destroys lockdown's protection. Route-inventory tests and the explicit bootstrap-channel enum make that regression visible.
- **Formal overclaim**: a green model can still be self-defining. Mutation/non-vacuity evidence and executable vectors are required; otherwise properties remain stated-normative and the implementation review must not label them checked-normative.
- **Design-time advisory**: no subagent/reviewer mechanism is available in this delegated context. Independent design advisory was therefore unavailable and non-blocking; implementation remains subject to the project-standard feature review, with the verification-tagged core checkpoint taking the deep review lane.

## Acceptance mapping and integrated evidence

| Acceptance area | Landed evidence |
|---|---|
| Durable all-Operation lockdown fence | Generated `SecurityLockdownEvent`/state/RPC contracts; replay-backed `SecurityPostureProjection`; domain-owned `OperationPosture`; `Submit` and `QueryDiagnostics` reject before append with `authorization_denied/security_lockdown_active`; accepted lifecycle and adapter reports remain reconciliable. |
| Session and login containment | Entry folds stale runtime sessions, clamps incoming reports, and raises the operator-session generation floor; fresh password login remains available for read-only inspection; stale sessions cannot render live. |
| Authorized entry and bootstrap-only exit | `CoreDecisionGate` orders catch-up, authorization, source/audit append, and projection refresh; entry requires authority-domain SessionManagement grant; exit is AdminService-only, loopback constrained, credential-independent, restart-safe, and no web route exists. |
| Atomicity and redaction | Source posture event and typed lockdown audit append atomically; injected append failure leaves the prior posture authoritative; state/snapshot/CLI projections expose bounded reason codes and decimal event/generation values without bearer/session material. |
| Cockpit contract | Signed-off hybrid shell is landed with desktop rail/punch-out, mobile tabs/More, persisted namespaced collapse, inline alert/read-only controls, stale-dominant presentation, and Security Arm → safe-reason → literal `LOCKDOWN` ritual. No browser exit affordance or transport exists. |
| CLI contract | `lockdown-enter` validates exact confirmation and clears credentials only after a valid response; `lockdown-exit` uses `makeAdminClient` with no credential read; human/JSON output is safe and failure paths never claim success. |

Verification evidence:

- `cargo test --workspace` — passed.
- `node contracts/scripts/check-models.mjs` and
  `node contracts/scripts/check-vectors.mjs` — passed; 40 vectors remain draft,
  with no checked-normative promotion claimed.
- `npx --yes @informalsystems/quint@0.32.0 compile specs/seed/security_lockdown.qnt`
  — passed; five named mutation/conformance runs passed 10,000 simulations
  each (`entry_then_operation`, `entry_then_restart`, `entry_stales_all`,
  `admin_exit_succeeds`, `web_exit_denied`).
- `cd contracts/ts && npm run check:presentation` — passed.
- `cd web-server && npm test` — 31 passed; `cd web-cockpit && npm test` — 72
  passed; `cd cli && npm test` — 36 passed; `cd e2e && npm test` — passed.
- Final cockpit bundle build passed and contained lockdown, banner, exact
  confirmation, bootstrap-exit guidance, CSRF, and destination-shell strings.

Deviations and parked issues:

- No behavior deviation from the signed-off design or the four story
  checkpoints. The QNT seed and design-system token/component updates are
  intentional foundation artifacts required by Units 2 and 5, not production
  contract forks.
- Formal lockdown properties and vectors are deliberately still
  **stated-normative/draft**; this checkpoint does not promote them to
  checked-normative semantics. Multi-operator/partial lockdown, additional
  bootstrap channels, automatic liveness restoration, and browser/admin proxy
  exit remain reserved seams, not silently implemented requirements.
- No additional parked implementation issue was created. The feature is left
  at `stage: review` for the feature-level integrated review.

## Extension pressure classification

- **Committed v0.1.0**: authority-domain durable lockdown; all-Operation pre-acceptance rejection; existing-session generation invalidation; stale runtime-session projection; fresh-login read-only snapshots/subscriptions; SessionManagement/authority-domain trigger; loopback-admin bootstrap exit; existing entry/exit audit vocabulary; signed-off web/CLI surfaces.
- **Reserved seam**: multiple authority domains/operators, scoped/partial lockdown, additional configured bootstrap-channel variants, continuous subscription reauthorization, richer incident taxonomy, and future native/desktop surfaces. Domain ids and `BootstrapChannelKind` keep these promotions additive.
- **Explicitly rejected for v0.1.0**: routine web reauthentication as exit proof, browser/admin proxy exit, arbitrary free-text durable reasons, grant-revocation fanout as posture, automatic liveness restoration on exit, a lockdown OperationKind/CommandState, and a takeover interstitial.

## Review findings (standard pass 1, 2026-07-29 — independent reviewer: gpt-5.6-sol)

Verdict: blockers-found. Receiver-confirmed blockers (fix before `done`):

1. **Adapter projection stale at session-report decisions** — after lockdown
   commits, the adapter service holds the shared gate but evaluates reports
   against its pre-lockdown SessionRegistry; a live report can append a live
   registration/transition that replay then rejects as corruption. Fix:
   catch up the adapter projection under the gate before deriving/appending;
   regression: entry → live session report → successful replay.
2. **Malformed QueryDiagnostics bypasses the canonical lockdown outcome** —
   query validation runs before gate/posture enforcement, so a malformed
   query gets validation_failed instead of the required
   authorization_denied/security_lockdown_active. Fix: query-specific
   validation under the gate after catch-up + posture check; test
   malformed/valid/exact-retry during lockdown.
3. **Cockpit doesn't match the signed-off security inventory** — never calls
   LoadSecuritySnapshot; operator-sessions section reports a runtime-session
   count; endpoint/device/grant sections are static cards without inventory
   rows or controls; rail shows visible labels (not icon-only). Confirmed in
   the built bundle. Fix: load + reconcile SecuritySnapshot into the
   presentation model, render signed-off rows/actions, hide rail labels
   visually (keep accessible names).
4. **Required security test evidence absent** — only one recovery test
   exists; missing: OperationKind matrix + exact retry during lockdown,
   QueryDiagnostics lockdown outcomes, adapter-report race, mutation race,
   transaction-failure atomicity, attributed entry/exit audit queries. Fix:
   add the designed deterministic tests alongside the code fixes.

## Review resolution

1. **Adapter projection stale at session-report decisions** — fixed by rebuilding
   the adapter-owned `SessionRegistry` from the authority-domain log under the
   shared `CoreDecisionGate` before deriving a session report delta. Added
   `lockdown_entry_then_live_report_catches_up_adapter_projection_before_derivation`;
   the entry → live report path now replays with a stale session rather than
   producing a lockdown-invalid live transition.
2. **Malformed QueryDiagnostics lockdown ordering** — fixed by validating the
   shared operation envelope/time boundary first, then acquiring the gate,
   catching up, enforcing `OperationPosture`, and only then decoding the typed
   diagnostics query. Added malformed, valid exact-retry, and all committed
   `OperationKind` lockdown cases; each returns
   `authorization_denied/security_lockdown_active` without a command event.
3. **Cockpit security inventory and rail** — fixed by adding the redacted
   `SecuritySnapshot` projection to `PresentationModel`, loading it through
   `Reconciler` at startup and reconnect, rendering operator-session,
   endpoint/device, and grant rows/actions from that snapshot, and visually
   hiding desktop rail labels while retaining `aria-label` names. The built
   bundle contains `loadSecuritySnapshot`, `operatorSessions`,
   `controlSurfaces`, and grant inventory rendering.
4. **Security test evidence** — added deterministic adapter-report replay,
   lockdown/Submit ordering race, entry/exit transaction-failure atomicity,
   OperationKind matrix and exact QueryDiagnostics retry coverage, and durable
   entry/exit audit-query attribution checks. Cockpit inventory rendering is
   covered by a DOM test.

Resolution verification: `cargo test -p patchbay-core-server --lib`, the
lockdown recovery and gRPC smoke suites, and `cd web-cockpit && npm test` pass;
full required verification is recorded in the implementation report.
