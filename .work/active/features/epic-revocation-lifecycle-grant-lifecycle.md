---
id: epic-revocation-lifecycle-grant-lifecycle
kind: feature
stage: implementing
tags: [security, foundation, protocol]
parent: epic-revocation-lifecycle
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-27
updated: 2026-07-27
---

# Grant lifecycle: revocation, expiry enforcement, Subscribe check

## Brief

Grants are durable and near-permanent in v0.1.0: there is no public
grant-admin path (a grant cannot be revoked), `Subscribe` authenticates the
compound issuer but performs no grant check, and expiry has a correctness
debt — `GrantRecord.is_expired()` evaluates `expires_at` against
`SystemTime::now()` directly (no clock port, and the field comment at
`core/src/authority/state.rs:45` still falsely claims expiry is "intentionally
not evaluated").

This feature delivers the grant lifecycle contract: a **public
grant-revocation path** (control-service RPC with its own grant
authorization; revocation is durable and audited, per-command
`GrantRevocationPolicy` for already-accepted work), **expiry enforcement
done right** (an injected clock port per Ports & Adapters, `is_live` honoring
expiry, expired grants rejecting with the committed failure semantics and an
audited rejection), and the **`Subscribe` grant check** (subscription
requests authorize against the issuer's grant like other Operations).

Does NOT cover: session/principal revocation or lockdown (sibling features);
cascade-revoke over grant provenance (explicitly out per SECURITY.md's
revocation model); descendant allowed-kinds inheritance (reserved seam).

## Epic context

- Parent epic: `epic-revocation-lifecycle`
- Position: independent start; the lockdown feature sequences after it
  (both touch the submission-authorization path and this feature establishes
  the current enforcement pattern there).

## Simplification opportunity

- The stale "intentionally not evaluated" comment and the direct `SystemTime`
  call collapse into one injected-clock design shared with the future
  session-staleness consumer (parked separately).
- Grant expiry enforcement deletes the "stored but lying" `expires_at` dead
  weight.

## Foundation references

- `docs/SECURITY.md` — revocation model (#4), grant rejection contract
  ("Missing, expired, revoked, target-mismatched, or kind-mismatched grants
  produce SubmissionOutcome = rejected")
- `docs/PROTOCOL.md` — authority, GrantRevocationPolicy, Revocation events
- `contracts/proto/patchbay/authority.proto` — grant/revocation anchors
- `core/src/authority/state.rs` — `is_live`/`is_expired`/`grant_authorizes`

## Design decisions

- **Clock ownership**: move the existing `Clock`/`SystemClock` out of acceptance into the shared core domain at `core/src/time.rs`, add a deterministic mutable `TestClock`, and pass one sampled `Timestamp` into pure validity/grant predicates. The clock is a core-owned port wired by the server composition root, not a server-only helper; this removes the authority→acceptance dependency and leaves one seam for future staleness timers.
- **Grant-revocation authority**: v0.1.0 permits self-attenuation only: the verified actor may revoke a grant whose subject actor matches, and an endpoint-narrowed grant additionally requires that endpoint. The grant itself must be live for the first revocation; exact repeat by the same subject is idempotent. Cross-subject/authority-domain administration is reserved for multi-operator/RBAC design rather than invented as an overloaded OperationKind.
- **Accepted-work policy**: persist the authorizing grant with every accepted Operation and make the Revocation event the durable policy boundary. `continue` leaves non-terminal work unchanged; `cancel` terminalizes accepted/delivered/running work as `Cancelled`; `require_reauthorization` rejects only still-`Accepted` (not yet delivered) work with `AuthorizationDenied`, while delivered/running work continues. Reauthorization means submitting a new Operation under a fresh grant; v0.1.0 does not add an unbounded held state.
- **Subscribe authority**: each `Subscribe` establishment uses `OperationKind::Query` against `TargetScopeKind::AuthorityDomain`. Every reconnect/resume RPC, including a nonzero cursor, re-checks the current grant. v0.1.0 does not continuously reauthorize an already-established stream; PROTOCOL commits establish-time checking and future filter-scoped subscriptions remain reserved.
- **Expiry vocabulary**: an otherwise-matching expired grant returns `SubmissionOutcome::Rejected` with existing `FailureCode::Expired`, reason `grant_expired`, and existing `AuditEventKind::GrantExpired`. Missing/mismatched/revoked grants remain `AuthorizationDenied` with narrower reason codes; no new failure enum is added.
- **Grant discovery/admin surface**: ship `grant-revoke` in the CLI and carry `grant_id` in redacted audit records/query output. Do not add a parallel grant-list store/RPC: `audit-query --kind grant_created,grant_revoked` is the existing durable projection for discovering grant ids, and bootstrap already prints its grant id.
- **Verification tier**: add implementation property tests and draft vectors against the existing `RevocationPreventsFuture`, `NoCommandWithoutGrant`, and `SubscriptionGrantChecked` stated-normative ids. Do not claim formal promotion; the current authority/subscription models explicitly lack mutation-survivable independent attempted evidence, and their v1 promotion gate remains intact.
- **Autonomous resolution**: the operator was unavailable by instruction. These protocol/security choices use the least-irreversible deny-by-default option and are logged here instead of blocking.

## UI surface

No visual UI surface: this feature exposes core RPCs and CLI administration only, so Phase 4.6 is skipped.

## Codebase mapping

Direct-read only (requested medium scope): authority state/ingest/replay, acceptance, storage/audit, ControlService/AdapterControlService, generated contracts, CLI command patterns, tests, and the existing draft authority/subscription models were inspected. No exploratory fanout was needed.

## Architectural choice

### Options considered

1. **Event-native revocation with durable acceptance provenance (chosen).** Wrap each durable accepted Operation with its authorizing grant, and let one Revocation event carry exact command-policy effects. Live and replay projections fold the same event; its LSN participates naturally in first-durable-terminal ordering. This costs a generated-contract/storage touch but gives crash-safe, auditable semantics.
2. **Imperative server loop over commands.** After writing a Revocation, append ordinary `CommandTransition` events one by one. This is smaller locally but can partially apply on failure, races adapter transitions through stale `from_state`, and needs repair logic after restart. It does not fit Patchbay's event-log authority posture.
3. **Add a grant-administration OperationKind.** Route revocation through normal Submit lifecycle. This would expand the canonical OperationKind registry and descendant/capability mappings solely to authorize attenuation, despite v0.1.0 having no cross-actor administrator. It is more irreversible and was rejected for this feature.

The chosen architecture keeps revocation in the existing authority event family, makes command policy effects deterministic under replay, and adds only the transport/admin surface needed by the single operator. A shared server decision gate serializes production command-transition planning with revocation-effect capture; storage then appends the Revocation and all typed audit records in one transaction.

## Implementation units

### Unit 1: Shared core clock and pure grant liveness

**Files**: `core/src/time.rs` (new), `core/src/lib.rs`, `core/src/acceptance/ports.rs`, `core/src/acceptance/pipeline.rs`, `core/src/authority/state.rs`, `core/src/authority/check.rs`, `core/src/authority/ingest.rs`, `core/src/storage/audited.rs`

**Story**: `epic-revocation-lifecycle-grant-lifecycle-clock-expiry`

```rust
// core/src/time.rs
pub trait Clock: Send + Sync {
    fn now(&self) -> prost_types::Timestamp;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

#[derive(Debug, Clone)]
pub struct TestClock {
    now: std::sync::Arc<std::sync::RwLock<prost_types::Timestamp>>,
}

impl TestClock {
    pub fn new(now: prost_types::Timestamp) -> Self;
    pub fn set(&self, now: prost_types::Timestamp);
}

// core/src/authority/state.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrantLiveness { Live, Expired, Revoked }

impl GrantRecord {
    pub fn liveness_at(&self, now: &prost_types::Timestamp) -> GrantLiveness;
    pub fn is_live_at(&self, now: &prost_types::Timestamp) -> bool;
    pub fn is_expired_at(&self, now: &prost_types::Timestamp) -> bool;
}

pub fn grant_authorizes_at(
    grant: &GrantRecord,
    issuer: &IssuerRef<'_>,
    operation_kind: OperationKind,
    target_scope: &TargetScope,
    now: &prost_types::Timestamp,
) -> bool;
```

**Implementation notes**:

- Move `timestamp_from_system_time` with `Clock`; re-export only where existing internal imports benefit. Remove `SystemTime` from authority state and the stale “intentionally not evaluated” comments.
- Revocation takes precedence over expiry when classifying a single grant. Across otherwise-matching candidates, a live grant wins; absent a live grant, return an expired candidate before a revoked candidate, with stable `grant_id` ordering so HashMap iteration never selects the public reason.
- `submit_with_clock` samples once, validates the Operation window with that value, and passes the same value to `GrantCheck::check`. Do not re-read time for the audit classification.
- `ControlServiceImpl` stores `Arc<dyn Clock>`; production constructors install `SystemClock`, while `new_with_clock`/test fixtures inject `TestClock`. Core functions receive timestamps/ports explicitly.

**Acceptance criteria**:

- [ ] `expires_at > now` is live and `expires_at <= now` is expired, including nanosecond boundaries.
- [ ] Expiry can be tested without sleep or wall-clock dependence.
- [ ] Operation-window and grant-expiry decisions use one sampled core instant.
- [ ] Authority matching contains no direct wall-clock read.

### Unit 2: Generated acceptance and revocation contracts

**Files**: `contracts/proto/patchbay/operations.proto`, `contracts/proto/patchbay/authority.proto`, `contracts/proto/patchbay/common.proto`, `contracts/proto/patchbay/control.proto`, `contracts/proto/patchbay/diagnostics.proto`, generated `contracts/rust/src/gen/patchbay/patchbay.rs`, generated `contracts/ts/src/gen/patchbay/*_pb.ts`

**Stories**: `epic-revocation-lifecycle-grant-lifecycle-clock-expiry`, `epic-revocation-lifecycle-grant-lifecycle-revocation-decision`, `epic-revocation-lifecycle-grant-lifecycle-subscribe-authorization`

```proto
// operations.proto
message AcceptedOperation {
  Operation operation = 1;
  GrantId authorizing_grant_id = 2;
}

message SubmissionResult {
  // existing fields 1..7
  GrantId decision_grant_id = 8;
  string reason_code = 9;
}

// authority.proto
message GrantRevocationEffect {
  CommandId command_id = 1;
  OperationState from_state = 2;
  OperationState to_state = 3;
  FailureCode failure_code = 4;
}

message Revocation {
  // existing fields 1..8
  repeated GrantRevocationEffect command_effects = 9;
}

// control.proto
service ControlService {
  rpc RevokeGrant(RevokeGrantRequest) returns (RevokeGrantResult);
}
message RevokeGrantRequest {
  AuthorityDomainId authority_domain_id = 1;
  GrantId grant_id = 2;
  string reason = 3;
}
message RevokeGrantResult {
  bool changed = 1;
  bool already_revoked = 2;
  EventId revocation_event_id = 3;
  GrantRevocationPolicy applied_policy = 4;
  repeated GrantRevocationEffect command_effects = 5;
}
```

```proto
// diagnostics.proto
// Add after current AuditEventKind values.
AUDIT_EVENT_KIND_SUBSCRIPTION_ESTABLISHED = 39;
AUDIT_EVENT_KIND_SUBSCRIPTION_DENIED = 40;

message AuditRecord {
  // existing fields 1..15
  GrantId grant_id = 16;
}
message AuditQuery {
  // existing fields 1..11
  GrantId grant_id = 12;
}
```

**Implementation notes**:

- `STORED_EVENT_KIND_OPERATION` keeps its discriminator but its payload becomes `AcceptedOperation`; all decoders must consume the wrapper. The ingress request remains plain `Operation`, so callers cannot self-assert the authorizing grant.
- Extend the dedup port with caller-supplied byte-exact logical-Operation equivalence bytes. The durable wrapper may contain a different currently selected grant on retry; storage remains opaque and compares the submitted Operation bytes, not core-owned acceptance metadata. The existing `idempotency_keys.payload_bytes` column can retain this value without a schema fork.
- `decision_grant_id` identifies the live grant that accepted an Operation or the matching expired/revoked grant that explains a denial. `reason_code` is bounded lower-snake-case audit vocabulary (`grant_expired`, `grant_revoked`, `authorization_denied`, `operation_expired`, etc.), not a second failure enum.
- Run Buf generation; never hand-edit generated output.

**Acceptance criteria**:

- [ ] Durable accepted work has replayable authorizing-grant provenance without trusting a caller field.
- [ ] Retry payload equivalence ignores only core-owned acceptance metadata and still rejects any client Operation mismatch.
- [ ] RPC and audit field identities are generated in Rust and TypeScript with drift checks green.
- [ ] No new OperationKind or FailureCode is introduced.

### Unit 3: Typed grant decisions and expiry audit propagation

**Files**: `core/src/acceptance/ports.rs`, `core/src/acceptance/pipeline.rs`, `core/src/acceptance/state.rs`, `core/src/acceptance/index.rs`, `core/src/acceptance/replay.rs`, `core/src/authority/check.rs`, `core/src/authority/registry.rs`, `server/src/state.rs`, `server/src/service.rs`

**Story**: `epic-revocation-lifecycle-grant-lifecycle-clock-expiry`

```rust
pub trait GrantCheck: Send + Sync {
    fn check(
        &self,
        authority_domain_id: &AuthorityDomainId,
        issuer: &dyn IssuerContext,
        operation_kind: OperationKind,
        target_scope: &TargetScope,
        evaluated_at: &prost_types::Timestamp,
    ) -> impl Future<Output = Result<Authorized, GrantDenied>> + Send;
}

pub enum GrantDenied {
    Expired { grant_id: GrantId },
    Revoked { grant_id: GrantId },
    NoMatchingGrant { actor: String, kind: OperationKind, target: String },
}

pub struct CommandRecord {
    // existing fields
    pub grant_id: GrantId,
}
```

**Implementation notes**:

- Replace the current post-hoc `ProjectionState::has_expired_grant`/second authority scan. The core rejection already carries failure, reason, and related grant; the server writes the correct audit directly.
- For an expired matching grant, return `FailureCode::Expired`, `reason_code = "grant_expired"`, and audit `GrantExpired` with `grant_id`. Operation validity-window expiry remains `FailureCode::Expired`, `reason_code = "operation_expired"`, and `CommandSubmissionRejected`.
- Revoked/missing/mismatched grants return `AuthorizationDenied`; use `grant_revoked` only when an otherwise matching revoked grant exists, otherwise `authorization_denied`.
- `CommandRecord::new` accepts/decodes `AcceptedOperation` and requires a non-empty grant id. Change in place rather than supporting a dual legacy payload path; Patchbay has no verified external event-log consumer or production-data compatibility obligation.

**Acceptance criteria**:

- [ ] Failure code, reason, audit kind, and grant correlation agree without inspecting diagnostic strings.
- [ ] Expired/revoked/missing distinctions cannot create command state.
- [ ] Command replay fails fast on accepted records missing grant provenance.

### Unit 4: Event-native revocation policy and atomic audit

**Files**: `core/src/authority/state.rs`, `core/src/authority/ingest.rs`, `core/src/authority/registry.rs`, `core/src/acceptance/index.rs`, `core/src/acceptance/transitions.rs`, `core/src/diagnostics/mod.rs`, `core/src/storage/port.rs`, `core/src/storage/audited.rs`, `core/src/storage/rusqlite.rs`, `server/src/state.rs`, `server/src/decision_gate.rs` (new), `server/src/service.rs`, `server/src/adapter_service.rs`, `server/src/main.rs`

**Story**: `epic-revocation-lifecycle-grant-lifecycle-revocation-decision`

```rust
// core/src/authority/state.rs
pub enum GrantAdministrationDenied {
    MissingOrForeign,
    EndpointMismatch,
    Expired { grant_id: GrantId },
}

pub fn authorize_self_revocation_at(
    grant: &GrantRecord,
    issuer: &IssuerRef<'_>,
    now: &prost_types::Timestamp,
) -> Result<(), GrantAdministrationDenied>;

// core/src/acceptance/transitions.rs
pub fn apply_grant_revocation_effect(
    record: &mut CommandRecord,
    effect: &GrantRevocationEffect,
    revocation_lsn: u64,
) -> Result<bool, AcceptanceError>;

// core/src/storage/port.rs
pub struct AuditedDecisionAppend {
    pub source_event_id: EventId,
    pub audit_event_ids: Vec<EventId>,
}

pub trait Storage {
    fn append_decision_audited_many(
        &self,
        authority_domain_id: &AuthorityDomainId,
        source: StoredEventPayload,
        audits: Vec<AuditRecordDraft>,
    ) -> impl Future<Output = Result<AuditedDecisionAppend, StorageError>> + Send;
}

// server/src/decision_gate.rs
#[derive(Clone, Default)]
pub struct CoreDecisionGate(std::sync::Arc<tokio::sync::Mutex<()>>);
```

**Implementation notes**:

- Under `CoreDecisionGate`, catch projections up, validate self-scope, sample the clock, and derive effects from commands accepted under the target grant:
  - `Continue`: no effects.
  - `Cancel`: each `Accepted|Delivered|Running` record gets `to_state=Cancelled`, `failure_code=Cancelled`.
  - `RequireReauthorization`: each `Accepted` record gets `to_state=Rejected`, `failure_code=AuthorizationDenied`; delivered/running records continue because delivery cannot be retroactively held.
  - Existing terminal records never receive an effect.
- Build `Revocation` fields in the core: exact domain/grant, verified `revoked_by`, injected `revoked_at`, generation `1` for the first terminal revocation, the grant's stored policy (request cannot override it), bounded reason, and exact effects.
- `ingest_revocation` validates the grant/effects, atomically appends one source plus `GrantRevoked` and per-effect `CommandCancelled`/`CommandRejected` audit records, then warms authority. Every audit points to the Revocation source event and includes `grant_id`; effect audits include `command_id`.
- Command and diagnostics projections fold `Revocation` in LSN order using one helper. `from_state`, grant provenance, allowed adjacency, policy/result mapping, domain, and command existence are replay validations. Later command transitions see terminal finality and become stale candidates/no-ops under existing rules.
- Production `ControlServiceImpl` and `AdapterControlServiceImpl` share one `CoreDecisionGate`. Acquire it around Submit/query transitions, grant revocation, adapter delivery acknowledgement/Observation command transitions, and disconnect command reconciliation. Never hold it for the lifetime of a Subscribe or delivery stream.
- `RevokeGrant` validates required fields/reason before the gate; missing/foreign/endpoint-mismatched grants all return `PermissionDenied` to avoid an existence oracle. Expired self grants return `PermissionDenied` plus `GrantExpired` audit. An exact already-revoked self grant returns `already_revoked=true` without another source event. Storage/audit failure returns `Unavailable` and the atomic transaction leaves no partial policy.
- Revocation remains non-cascading: only the named grant changes. Descendant provenance is queryable but never traversed.

**Acceptance criteria**:

- [ ] Revocation cannot be authorized by a self-asserted request actor or by adapter capabilities.
- [ ] Every policy effect is durable, replay-identical, terminal-final, and paired with typed audit.
- [ ] A crash cannot leave a Revocation source without its required audit records or half of its listed effects.
- [ ] Revoking a spawn grant does not touch descendant grants.
- [ ] A fresh grant plus a new command id/key is required after a reauthorization rejection.

### Unit 5: Subscribe establishment authorization

**Files**: `server/src/service.rs`, `server/src/state.rs`, `core/src/authority/check.rs`, `contracts/proto/patchbay/control.proto`

**Story**: `epic-revocation-lifecycle-grant-lifecycle-subscribe-authorization`

```rust
async fn authorize_subscription(
    &self,
    issuer: &dyn IssuerContext,
    authority_domain_id: &AuthorityDomainId,
    evaluated_at: &prost_types::Timestamp,
) -> Result<Authorized, GrantDenied>;

fn subscription_scope() -> TargetScope {
    TargetScope {
        kind: TargetScopeKind::AuthorityDomain as i32,
        ..TargetScope::default()
    }
}
```

**Implementation notes**:

- `subscribe` keeps boundary order: validate domain/cursor → verify compound issuer → sample injected clock once → `GrantCheck(Query, AuthorityDomain)` → durable audit → read/filter events → return stream.
- Audit success as `SubscriptionEstablished` with actor/endpoint/device, `decision_grant_id`, authority-domain target, and `subscription_established`. Audit ordinary denial as `SubscriptionDenied`/`AuthorizationDenied`; an expired matching grant uses `GrantExpired`/`Expired` with reason `subscription_grant_expired`.
- If required audit append fails, fail closed with `Unavailable` before replay. Do not create an Operation/Command record.
- A nonzero cursor is not trusted proof of an earlier authorization. Each resume calls the same path. Grant revocation after a finite batch was established does not rewrite already-returned events; the next resume denies.
- v0.1.0 has no subscription filter in `SubscribeRequest`; all operator-facing events are authority-domain scoped. Do not invent a global-public rule for future multi-operator deployments.

**Acceptance criteria**:

- [ ] No live Query grant means no event replay, including cursor resume.
- [ ] Successful/denied establishment is durably audited without Operation state.
- [ ] The existing operator-facing event allowlist and LSN gaps remain unchanged.

### Unit 6: CLI administration and audit discovery

**Files**: `cli/src/commands/grant-revoke.ts` (new), `cli/src/commands/audit-query.ts`, `cli/src/core-client.ts`, `cli/src/main.ts`, `cli/src/output.ts`, `cli/tests/auth-commands.test.ts`, `cli/tests/output-diagnostics.test.ts`, `cli/tests/scripting-commands.test.ts`

**Story**: `epic-revocation-lifecycle-grant-lifecycle-cli-conformance`

```ts
export async function grantRevokeCommand(
  client: Pick<ControlClient, "revokeGrant">,
  authorityDomainId: string,
  options: { grantId: string; reason?: string; json: boolean },
  output: CliOutput,
): Promise<number>;
```

**Implementation notes**:

- Command grammar: `grant-revoke <grant-id> [--reason TEXT] [--json]`; default reason is `operator_requested`. Reject empty ids/reasons and unsafe/oversized reason text locally.
- Human output names grant, changed/already-revoked status, applied policy, Revocation event id, and affected command count. JSON emits generated enum names as canonical lower-snake-case and 64-bit LSNs as decimal strings.
- Exit codes: `0` changed or already revoked; `2` authenticated denial; `1` local validation, transport, unavailable, or protocol-shape failure.
- Extend audit query/output with `--grant-id`; `GrantCreated`, `GrantChanged`, `GrantExpired`, and `GrantRevoked` rows show safe grant ids. No raw authority event payload is exposed.

**Acceptance criteria**:

- [ ] The operator can discover grant ids via audit query and revoke one without a browser surface.
- [ ] JSON and human output distinguish first revocation from idempotent repeat.
- [ ] Denial never clears credentials or claims the grant changed.

### Unit 7: Executable evidence and rolling foundation

**Files**: `core/tests/authority_grant_check.rs`, `core/tests/authority_ingest.rs`, `core/tests/authority_registry.rs`, `core/tests/authority_replay.rs`, `core/tests/authority_proptest.rs`, `core/tests/acceptance_pipeline.rs`, `core/tests/acceptance_replay.rs`, `core/tests/audit_records.rs`, `core/tests/rusqlite_storage.rs`, `server/tests/grpc_smoke.rs`, `server/tests/trust_boundary.rs`, `server/src/adapter_service/tests.rs`, `contracts/vectors/grant-expiry-rejected.json` (new), `contracts/vectors/grant-revocation-prevents-future.json` (new), `contracts/vectors/grant-revocation-policy-effects.json` (new), `contracts/vectors/subscription-grant-checked.json` (new), `contracts/vectors/subscription-resume-rechecked.json` (new), `docs/SECURITY.md`, `docs/PROTOCOL.md`, `docs/VERIFICATION.md`, `docs/UX.md`, `docs/GLOSSARY.md`

**Story**: `epic-revocation-lifecycle-grant-lifecycle-cli-conformance`

**Implementation notes**:

- Keep new vectors `promotion_status: draft`; trace them to existing stated-normative property ids and generated fields. Do not edit generated VERIFICATION traceability blocks by hand.
- Add property tests over time boundaries, grant candidate combinations, policy/state cross-products, replay, non-cascade, and concurrency ordering. Tests assert independent inputs and observable source/audit/state outputs; do not encode the implementation helper as the oracle.
- Roll SECURITY's implementation-status sentence forward for action #4, specify self-scope and accepted-work behavior, and preserve cross-subject administration as reserved. PROTOCOL records Query/AuthorityDomain Subscribe semantics and the Revocation-event terminal boundary. UX adds the CLI command. VERIFICATION stays honest about stated-normative formal status.

**Acceptance criteria**:

- [ ] A concurrent adapter terminal candidate and grant revocation always replay to the lowest-LSN valid terminal winner.
- [ ] Mutation-oriented tests fail if expiry is ignored, revoke permits a foreign actor, policy effects are omitted, Subscribe skips grant check, resume trusts its cursor, or audit/source atomicity splits.
- [ ] Foundation docs describe the intended/implemented contract without a contradictory “grant cannot be revoked” or “expiry ignored” assertion.

## Implementation order

1. `epic-revocation-lifecycle-grant-lifecycle-clock-expiry` — shared Clock, generated accepted-operation/rejection fields, deterministic expiry enforcement.
2. In parallel after 1:
   - `epic-revocation-lifecycle-grant-lifecycle-revocation-decision` — acceptance provenance, event-native revocation policies, atomic audits, RPC.
   - `epic-revocation-lifecycle-grant-lifecycle-subscribe-authorization` — Query/AuthorityDomain establishment check and audit.
3. `epic-revocation-lifecycle-grant-lifecycle-cli-conformance` — regenerated consumers, CLI/admin discovery, vectors, integration/property tests, rolling docs.

The feature remains one cohesive implementation/review bundle; stories are durable design checkpoints, not one-worker-per-story assignments.

## Simplification

- Delete direct authority `SystemTime::now()`, the stale “expiry intentionally not evaluated” comments, and duplicated acceptance-owned time conversion after moving them to `core/src/time.rs`.
- Delete `ProjectionState::has_expired_grant` and the server's post-hoc expired-grant recheck; the first grant decision carries typed failure/reason/grant provenance.
- Consolidate live and diagnostics revocation effect application behind one pure transition helper; do not maintain a server-only policy interpretation.
- Reuse existing `FailureCode::Expired`, `AuditEventKind::{GrantExpired,GrantRevoked,CommandCancelled,CommandRejected}`, Revocation event kind, Query OperationKind, authority-domain target, audit projection, and CLI auth client.
- Intentionally do not add cascade traversal, a grant-admin OperationKind, a grant-list database/projection, a held command state, or a visual control surface.

## Testing

- **Interface contracts**: gRPC smoke/trust-boundary tests protect self-revocation, typed errors, Subscribe establish/resume checks, audit-before-replay, and generated wire shape.
- **Regression**: exact expiry boundary and one-clock-sample tests prevent the current raw-wall-clock/stale-comment debt from returning.
- **Property tests**: liveness/candidate ordering and policy × command-state matrices protect deny-by-default and replay equivalence; non-cascade remains independently observed.
- **Storage tests**: injected transaction failure and restart prove source/audit atomicity and authorizing-grant recovery.
- **Concurrency test**: barrier-controlled adapter transition versus revocation protects decision-gate ordering and first durable terminal semantics.
- **CLI tests**: command grammar, reason validation, human/JSON output, grant-id audit discovery, and exit mapping protect the only operator-facing surface.
- **Test removal**: update existing fixtures/decoders to `AcceptedOperation`; remove tests that assert raw `Operation` is the stored payload or that Subscribe authenticates without a grant. Do not retain dual-format compatibility tests.

## Risks

- **Highest risk — revocation/adapter terminal races**: without one decision gate and event-native effects, a stale `from_state` can corrupt replay or partial storage can split policy. Fallback is to retain event-native effects and serialize all production command writers before exposing the RPC; do not fall back to sequential best-effort transitions.
- **Accepted-operation wire migration**: every decoder of `StoredEventKind::Operation` must move to `AcceptedOperation`; a missed decoder will fail replay or delivery. Generated types plus grep-backed inventory and full workspace tests are the mitigation. There is no verified external durable-log consumer, so a dual reader would add cost without an earned compatibility obligation.
- **Self-lockout**: self-revocation can remove the operator's last broad grant. This is deliberate attenuation in the single-operator model; the command must print the affected grant/policy clearly. Cross-subject admin and recovery/grant issuance remain bootstrap/admin concerns, not an implicit bypass in this RPC.
- **`require_reauthorization` semantics**: rejecting only undelivered work is less stateful than a hold but irreversible for that command id. The operator can submit a new intent under a fresh grant; a resumable held state is a reserved future policy, not silently approximated.
- **Formal assurance gap**: implementation evidence does not promote the existing draft authority/subscription properties. Foundation prose and vector metadata must continue to say stated-normative until the independent-evidence models clear their separate gate.
- **Design-time advisory**: this delegated environment exposes no subagent/reviewer tool. Independent design review was unavailable and is non-blocking per the skill; the feature's normal standard implementation review remains required.

## Extension pressure classification

- **Committed v0.1.0**: injected core clock; grant expiry at authorization; self-scoped, durable, non-cascading grant revocation; all three existing `GrantRevocationPolicy` values with the exact state effects above; Query/authority-domain Subscribe establishment and resume checks; typed audit/CLI administration.
- **Reserved seam**: cross-subject authority-domain/RBAC administration, filtered subscriptions for multi-operator privacy, continuous stream reauthorization, resumable held commands, delegation/cascade queries, and future session-staleness use of the clock.
- **Explicitly rejected for v0.1.0**: implicit cascade revocation, UI-only authorization, capability-based grant authority, and adding a grant-admin OperationKind solely for this attenuation RPC.
