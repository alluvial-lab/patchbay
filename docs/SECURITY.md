# Patchbay Security

Patchbay is a high-authority control plane: a browser or CLI action can mutate remote/headless agent sessions. v0.1.0 therefore treats security as part of protocol semantics, not as a UI add-on.

This document defines Patchbay's security posture. The committed **v0.1.0** posture is session-only: one human operator, one authoritative coordination core, a responsive web cockpit, a CLI, and the first Pi adapter. The **post-v0.1 agent-operations direction** (per `docs/SPEC.md`'s "Post-v0.1 agent-operations direction") extends the posture to operational-resource targets; resource objectives and the resource-report boundary below are labeled post-v0.1 and are **not** v0.1.0 release obligations. It should be read with `docs/PROTOCOL.md` for command/grant state and `docs/VERIFICATION.md` for model obligations.

Research grounding: `.research/analysis/briefs/web-control-security.md`.

## Security objectives

v0.1.0 security must ensure:

- only authenticated operator endpoints can submit control actions;
- every Operation is authorized before durable acceptance;
- accepted Operations bind to the intended runtime-session generation;
- retries cannot double-apply intent at the Patchbay boundary;
- browser sessions can be revoked promptly;
- stale or late events cannot mutate newer session state;
- audit records preserve security-relevant decisions without storing secrets.

Post-v0.1 operational-resource security must additionally ensure:

- accepted Operations bind to the exact operational-resource `(adapter_id, resource_kind, resource_id)` tuple;
- resource-id collisions across adapters or adapter-owned kinds cannot widen a resource grant or route to the wrong adapter.

Patchbay does not prove cryptographic primitives, operating-system isolation, browser correctness, network latency bounds, or third-party harness internals. Those are deployment and adapter assumptions.

## v0.1.0 authority domain

v0.1.0 has one human operator and one authority domain. This is a product scope decision, not a reason to omit authority modeling.

The model keeps these concepts explicit:

- **Operator** — the single human who controls Patchbay in v0.1.0.
- **Actor** — represented participant: operator, agent, adapter, daemon, service, or control surface.
- **Device** — a physical or virtual host that can run one or more endpoints, such as a browser on a laptop, a CLI on a VM, or an adapter process near a runtime.
- **Endpoint** — a concrete browser, CLI, adapter process, or other connection-bearing instance for an actor on a device.
- **Operator session** — an authenticated browser or CLI session for the operator, backed by a server-side session record.
- **Runtime session** — an adapter-reported control target, such as a Pi session.
- **Grant** — an authority relationship permitting a subject (an actor, optionally narrowed to an endpoint or endpoint class) to perform OperationKinds against a target scope.
- **Authority domain** — the single core-owned context in which grants, revocation, routing authority, and audit are evaluated.

Future multi-operator coordination remains a reserved extension seam. v0.1.0 data structures should not assume there can only ever be one operator, but v0.1.0 UX and provisioning do not implement multi-human administration, handoffs, or shared authority domains.

## Threat model

### In scope

v0.1.0 is designed against:

- unauthenticated browser or CLI access;
- unauthorized browser, CLI, or adapter endpoint enrollment;
- CSRF attempts against the web cockpit;
- replayed Operation submissions at the Patchbay boundary;
- Operation submission from a revoked browser/device endpoint;
- Operations aimed at the wrong session generation;
- stale adapter events or late replies mutating newer state;
- confused-deputy routing where UI labels or payload fields override verified identity;
- unsupported adapter Operations being presented as available;
- accidental logging of secret material — the canonical no-log/redaction list lives in the Audit events section below; this threat is designed against that list;
- deployment mistakes that expose an unauthenticated or HTTP-only non-localhost core;
- adapter-to-core channels that bypass Patchbay identity, capability, or grant checks.

### Out of scope

v0.1.0 does not attempt to solve:

- malicious code execution inside an already-controlled host;
- compromise of the operator's browser, device, password manager, or OS account;
- malicious or backdoored agent harnesses beyond adapter-declared behavior;
- correctness of cryptographic libraries;
- multi-operator social workflows or organization-level policy;
- high availability, replicated authority domains, or split-brain recovery;
- public multi-tenant internet service hardening.

Out-of-scope does not mean irrelevant. These risks belong to deployment guidance, adapter documentation, future features, or operator operational discipline.

## Enrollment and authentication

v0.1.0 enrollment is intentionally narrow:

- The first operator is created through CLI/local-console bootstrap, not through an unauthenticated network setup page.
- Bootstrap produces a one-time setup secret that expires after use or timeout.
- Setup establishes the operator's primary authenticator: password/passphrase for v0.1.0, with passkeys or MFA reserved as an extension seam.
- A browser endpoint enrolls only after successful operator authentication and creation of a server-side operator session.
- A CLI endpoint enrolls only through local setup credentials or an existing authenticated operator session.
- An adapter endpoint enrolls only through configured adapter attachment material or an adapter-specific trust root; an adapter cannot self-assert core authority by display name or payload field.
- Device labels are operator-facing metadata. The authority boundary is the endpoint/operator-session/grant tuple, not the label.

Interactive login must be rate-limited, track failed attempts against the operator account as well as useful network metadata, and audit both success and failure. Rate limiting and lockout policy must avoid making denial of service easier than authentication.

## Browser session model

The web cockpit uses server-side sessions. The browser receives only an opaque session cookie; Operation authority, endpoint metadata, grants, and session state remain server-side.

v0.1.0 browser-session requirements:

- session identifiers are high-entropy, meaningless client-side values;
- session records include operator id, endpoint id, device id, created time, last-used time, expiration, revoked time, and **operator-session generation**;
- session cookies use `HttpOnly`, `Secure` outside localhost, `SameSite=Strict` by default, `Path=/`, and no `Domain` attribute;
- non-localhost deployments require direct HTTPS before browser sessions are accepted; the only supported proxy mode is an explicitly enabled loopback proxy that overwrites `X-Forwarded-Proto` and attests `https` (see `docs/RUNBOOK.md`);
- localhost development may use the browser's localhost secure-cookie exception, but that exception must not be generalized to LAN/IP/container deployments;
- session secrets are never stored in localStorage;
- login, logout, session renewal, and session revocation are audit records;
- reauthentication is required after suspicious session signals or before future high-risk operations when those operations are introduced.

Default cookie shape:

```http
Set-Cookie: __Host-patchbay_session=<opaque>; Path=/; Secure; HttpOnly; SameSite=Strict
```

`SameSite=Lax` is a reserved fallback only for a concrete operator flow that requires top-level cross-site navigation into Patchbay. `SameSite=None` is not a v0.1.0 default.

## CSRF and browser request protection

SameSite is defense-in-depth, not the whole CSRF model.

Every state-changing web route must require:

- an authenticated operator session cookie;
- a CSRF token tied to that operator session;
- a custom request header for API/XHR/fetch mutations;
- Origin and/or Fetch Metadata checks where the browser provides them;
- a non-GET method for mutations.

Unauthenticated setup pages must not expose state-changing bootstrap flows on a network listener. First-run setup must use the CLI, local console output, or a one-time bootstrap secret that becomes invalid after setup.

## Operation authorization and replay resistance

An Operation is accepted only after Patchbay validates:

1. payload shape and `OperationKind` (the kind must be a known Patchbay OperationKind; an unknown or reserved-but-not-validatable kind like `agent-send` or `adapter-utility-exec` is `validation_failed` at submission, before a grant is evaluated);
2. authenticated issuer session or endpoint;
3. target actor/runtime-session identity and generation, exact typed operational-resource identity, or fleet/supervisor scope for spawn Operations whose target does not yet exist;
4. idempotency key or command id;
5. Operation expiration window;
6. a live, unrevoked grant permitting that issuer to perform that OperationKind on that target scope.

Authorization is deny-by-default. Missing, expired, revoked, target-mismatched, or kind-mismatched grants produce `SubmissionOutcome = rejected` with `authorization_denied` or the narrower applicable failure term from `docs/PROTOCOL.md`. When multiple grants match, decision provenance follows the canonical deterministic selection rule in `docs/PROTOCOL.md` § Authority grants; projection, storage, and container iteration order is never authority.

Retries with the same idempotency key return the existing command record. A new intentional action requires a new command id/key.

Sender identity comes from the verified connection/session context. Payload display names, human labels, project names, cwd values, and adapter-reported friendly names are never routing authority. v0.1.0 Operations are operator-originated; non-operator Operation senders (agent→agent, adapter→operator service Operations) are a reserved seam, not v0.1.0 mediated behavior.

### Operational-resource report boundary (post-v0.1)

A typed resource report is accepted only on the current authenticated adapter
attachment. The core replaces source authority with the verified adapter id,
requires the report generation to equal the current attachment generation,
selects the exact manifest declaration by `(adapter_id, ResourceKind)`, caps the
reported completeness tier to that declaration, and matches both resource and
domain-projection envelope descriptors before durable append. Payload-supplied
identity cannot override this binding; mixed kind, cross-adapter, stale-token,
stale-generation, undeclared, overclaimed, and schema-mismatched reports fail
closed without a resource-state append.

Resource and projection envelopes are metadata/control-plane state only. They
must not contain provider credentials, onboarding secrets, access tokens,
model prompts or responses, tool data-plane content, or other LLM traffic.
Schema-reference matching establishes declared format identity but does not make
opaque bytes trusted or semantically valid; local typed decoders remain
fail-closed. The canonical no-log/redaction list below still applies, and a
resource report must never create a diagnostic or audit path around it.

Terminal exact-identity tombstones and same-event distinct replacement prevent
late or reordered evidence from redirecting an Operation to a retired resource.
Abnormal disconnect and newer-generation reconciliation may reduce cache
freshness but never manufacture adapter domain health or authority.

### Compound issuer

When an Operation arrives at the core through a control surface (the v0.1.0 path is browser → web server → core), the core verifies a compound issuer: the operator actor is the grant subject and is verified against operator-session evidence, and the transport endpoint (the web server, or a CLI endpoint) is verified as a principal. The core must not trust a self-asserted operator identity. The implemented web↔core wire shape carries the operator-session evidence and control-surface principal evidence separately, and the core independently verifies both before accepting an Operation.

## Grant shape

A v0.1.0 grant has at least:

- grant id;
- authority domain id;
- subject actor id;
- optional subject endpoint id or endpoint class;
- target scope: actor, adapter, runtime session, project/session group, fleet/supervisor scope, or other modeled resource;
- allowed OperationKinds;
- created time and provenance;
- optional expiration;
- revocation generation or revoked time;
- revocation policy for already accepted commands.

Delegation is a reserved future direction, not a v0.1.0 field; a `parent grant id / delegated-by` field is intentionally absent from v0.1.0. Device is part of the identity model (for audit and revocation grouping) but is not a grant-matching field. Adapter capability sets are not grant authority; they are advisory UX declarations, and the adapter is the authority on its own support at delivery time. Operational-resource identity is the exact `(adapter_id, resource_kind, resource_id)` tuple: a resource Grant matches only that tuple and a requested resource target. A local id collision under another adapter or kind is denied; adapter, fleet, and authority-domain Grants are the explicit wider scopes. The legacy Protobuf tag-8 audit target cannot satisfy an operational resource Grant.

### Spawn authority

Spawn is fleet-level by default in v0.1.0: a spawn grant authorizes spawning across any adapter/supervisor the operator can reach, before a target session exists. Adapter-level spawn grants remain expressible through the existing target-scope flexibility when narrower authority is desired; no schema change is needed. Per-spawn-variant authority is reserved.

Successful spawn completion records an explicit, auditable **descendant grant** for the spawned session. This is an explicit grant record generated as part of spawn, not an implicit grant-matching rule. The descendant grant shape matches `docs/PROTOCOL.md` and is a normal grant instance with:

- `grant id` — standard grant id (core-assigned).
- `authority domain id` — same domain as the spawning grant.
- `subject actor id` — the spawner (operator actor in v0.1.0).
- `optional subject endpoint id or endpoint class` — the spawning endpoint, if applicable.
- `target scope` — the spawned session/generation (an existing-session scope, now that the session exists).
- `allowed OperationKinds` — the full set of committed kinds applicable to an existing session, enumerated explicitly (not a wildcard `all`): `instruct`, `cancel`, `interrupt`, `query`, `approval-response`, `elicitation-response`, `reconfigure`, `session-management`. `spawn` is excluded because recursive spawning requires a separate fleet-level spawn grant; `attach` is excluded because the spawned session is already attached to its spawner's control plane.
- `creation time and provenance` — `provenance = { spawn_operation_id, spawning_grant_id }` (explicit link to the spawn Operation and the grant that authorized it).
- `optional expiration` — none by default (the descendant grant lives until revoked or the session is retired).
- `revocation generation or revoked time` — standard; revocable independently of the spawn grant (two-lever rule, no cascade).
- `revocation policy for already accepted commands` — standard.
- `audit id` — links to the spawn-completion audit event.

The auto-issued descendant grant is same actor (operator), new target (spawned session), not cross-actor delegation. No delegation lineage field is present in the v0.1.0 descendant grant. The reserved future direction is to inherit descendant allowed kinds from the spawning grant for delegation-aware authority; that future work must be designed with multi-operator / federated-authority semantics before use.

Spawn completion is exposed only after verified provenance is durable. Generic Observation ingestion records a successful spawn result without terminalizing the command. A single core owner correlates that evidence with the exact registered/replacement session target, writes a `CommandCompleted` audit with reason `spawn_completion` using the verified accepted actor/endpoint/device and authorizing grant, stores its exact event id on the descendant grant, and appends the completed transition last while holding the shared decision gate. Restart repair derives progress only from the durable log and finishes before listeners open; self-asserted Observation sender fields do not become descendant authority.

Revocation uses two independent levers: revoking the spawn grant prevents future spawns, but already-spawned sessions keep operating under their auto-issued descendant grant until that grant is separately revoked. No cascade-revoke is v0.1.0 behavior; future cascade is a query over grant provenance and needs no schema change.

### Elicitation responder authorization

v0.1.0 Elicitations bind to the operator actor (the `expected_responder_actor`), not a specific endpoint. Any authenticated operator endpoint may answer; the responding endpoint is captured in the response Operation audit at response time, not pre-bound in the Elicitation. First valid answer terminalizes the Elicitation for all subscribed surfaces; later attempts from other surfaces are rejected as already-terminal/stale and audited. Tighter responder binding (endpoint, endpoint class, fallback chain) and responder-actor distinction for multi-operator sessions are reserved seams.

Response Operations and spawn are state-changing and subject to the same CSRF and authority requirements as other Operations. Secret response-contract kinds (reserved, not validatable in v0.1.0) carry redaction/no-log obligations: a `secret` contract response must never be persisted in plaintext or logged raw; redaction policy is enforced at the boundary before any audit or snapshot materializes.

Grant checks are centralized in the coordination core. Control surfaces may hide unavailable actions, but UI availability is never authoritative.

## Revocation model

Revocation prevents future authority. Already accepted Operations follow the policy attached to their grant and OperationKind:

- **continue** — preserve already accepted work, but reject future Operations;
- **cancel** — submit or record cancellation for accepted non-terminal Operations when supported;
- **require reauthorization** — hold or reject delivery until a fresh grant/session is established.

v0.1.0 must support these operator-facing revocation actions (the current-session, all-session, principal/endpoint/device, grant, and security-lockdown controls are implemented):

1. **Revoke current browser session** — delete or mark the session revoked and clear its cookie.
2. **Revoke all operator sessions** — invalidate every current operator-session generation for the verified actor, including CLI sessions. v0.1.0 uses opaque server-side records and a durable generation fence; it does not add a signing-secret rotation layer.
3. **Revoke principal/endpoint/device** — mark the matching browser or CLI credential scope revoked and reject future Operations and same-id enrollment from it.
4. **Revoke adapter/session grant** — stop Operation acceptance for a target scope while preserving audit history.
5. **Security lockdown** — reject new Operations, mark affected runtime sessions stale, require fresh login, and record the reason.

**Lockdown exit.** Lockdown is a durable posture (an audited, persisted event). Restarting the core does not clear it: crash recovery replays the log and lockdown remains in effect. Exit requires re-establishing the bootstrap trust level **via the bootstrap channel** (v0.1.0's loopback `AdminService`, invoked by `patchbay-cli lockdown-exit`), not routine web re-authentication. This self-scales with the operator's configured security posture. The protection depends on the enrollment channel being distinct from routine web login: if a future deployment ever makes bootstrap trust equivalent to routine web login (same factor, same remote channel), lockdown would provide no protection, because an attacker holding the routine credential could clear it. That channel distinction is load-bearing, not incidental.

While active, every `ControlService.Submit` and `QueryDiagnostics` Operation is rejected before acceptance with `authorization_denied/security_lockdown_active`; no command record is created. Already accepted Operations may finish under their existing policy, and adapter reports, `Subscribe`, `LoadSnapshot`, `LoadSecuritySnapshot`, fresh `VerifyOperatorPassword` login, current-session logout/revocation, and required audit ingress remain available. Fresh login is read-only for Operations and security mutations; grant, enrollment, and scope-revocation mutations fail closed until bootstrap exit. Entry and exit persist only bounded lower-snake-case reason codes and atomically pair their source event with the corresponding audit record.

Revocation never deletes command history. Late events after revocation are audit/reconciliation events unless they are valid transitions for commands already accepted under the relevant policy. This session/principal/endpoint/device plane uses `continue`: accepted work may finish, while future acceptance and subscription establishment require a fresh valid authority.

CLI recovery is explicit and truthful: confirmed `patchbay-cli revoke-all-sessions` clears the local credential file, then `patchbay-cli login` from a trusted host obtains fresh credentials and a higher operator-session generation. Self-revoked principal/endpoint/device credentials require a distinct unrevoked identity or new endpoint/device configuration. The one-time `setup` secret is not a recovery mechanism and must not be advertised as one.

## Audit events

Security audit is part of v0.1.0. The walking skeleton emits redacted security audit lines to process stderr/stdout. The implemented post-v0.1.0 core-diagnostics capability adds a durable, queryable redacted audit index behind the core storage port; process stderr remains diagnostic-only.

Audit records are distinct from durable command/session state-transition events. They may record rejected attempts, failed checks, and security decisions that do not create command records.

Minimum audit records:

- bootstrap started/completed/expired;
- login success/failure;
- logout;
- operator-session created, renewed, expired, revoked;
- control-surface principal, endpoint, and device revocation;
- failed CSRF, Origin, or Fetch Metadata check;
- failed authorization;
- grant created, changed, expired, revoked;
- command submission accepted/rejected/failed/unknown;
- command delivered/running/completed/rejected/failed/expired/cancelled/superseded;
- target session generation mismatch;
- stale event or late reply ignored for mutation;
- adapter attach/detach/failure;
- emergency lockdown entered/exited.

Audit records should include event id, timestamp, actor id, endpoint id when known, operator-session id or hash when applicable, command id when applicable, target id, target generation, outcome, reason/failure vocabulary, and correlation id.

Audit records must not directly store raw session cookies, CSRF tokens, access tokens (including token-commune gateway member keys and their bearer forms), passwords, bootstrap secrets, encryption keys, command prompt bodies by default, or sensitive attachments. Adapter attachment material — the `attachment_method.descriptor` bytes an adapter presents at enrollment (mTLS material, configured local material, OAuth tokens, future trust-root proofs) — is secret-bearing and must not be stored in audit records or surfaced raw by diagnostic commands; it is redacted/excluded at the boundary before any audit or diagnostic projection materializes, the same as other secrets above. If command content logging is later added, it must be an explicit policy with redaction and operator-visible controls.

**This is the canonical no-log/redaction list for Patchbay.** Other docs (PROTOCOL, UX, ARCHITECTURE) summarize or point here; they do not maintain competing lists. Add new redacted fields to this list, not to a doc-local copy.

The committed v0.1.0 `session-health` CLI projection reads canonical session state and does not create a raw-payload exposure path. The committed post-v0.1.0 `audit-query`, `inspect-command`, and `adapter-status` CLI commands consume the core's principal-gated `QueryDiagnostics` RPC. These projections inherit the redaction boundary above: `inspect-command` excludes prompt bodies and sensitive payloads, and `adapter-status` excludes raw `attachment_method.descriptor`. Any future diagnostic command that would surface a field not covered by the rules above must extend this section before shipping.

The committed post-v0.1.0 adapter-reporting extension has a structural allowlist: the generated diagnostic payload contains only adapter-declared `code`, severity, adapter generation, optional OperationKind, and bounded count. Target scope, at most one typed command correlation, observed time, and canonical failure code are separate generated fields. It has no message, stack, cause, prompt, transcript, tool result, attachment, descriptor, path, model, token, credential, or arbitrary metadata field. The core replaces adapter identity, endpoint, authority domain, and generation from the authenticated attachment and atomically appends the safe source Observation with its `ADAPTER_DIAGNOSTIC_REPORTED` audit record. Forwarding is best effort and non-retrying; a report cannot establish liveness or endanger the adapter control loop.

## Deployment posture

Allowed v0.1.0 deployments:

- a loopback/colocated deployment on a local workstation, VM, or container: the core's general and admin listeners bind to loopback, and the web server, CLI, and Pi adapter run on that host and reach the core through its loopback listener;
- a browser connected directly to that colocated web server over loopback or TLS. Direct TLS browser access does not make the core network-reachable.

Reserved deployment seams:

- exposing the core on a LAN or VPN, or separating the web server, CLI, or adapters from the core, requires a future transport/TLS design;
- reverse-proxy TLS termination for the web server requires an explicit trusted-proxy design. v0.1.0 accepts direct TLS or loopback browser connections only.

Forbidden v0.1.0 deployments:

- internet-exposed unauthenticated core;
- non-localhost HTTP browser access carrying authenticated sessions;
- deployments that bypass Patchbay authority checks by sending browser Operations directly to adapters;
- deployments that treat adapter-reported display names as routing authority;
- deployments that disable audit for Operation acceptance, authorization, or revocation.

## Extension pressure classification

Committed v0.1.0 behavior:

- one operator and one authority domain;
- explicit actor, device, endpoint, operator-session, runtime-session, grant, revocation, and audit concepts;
- server-side browser sessions with hardened cookies;
- CSRF protection for state-changing web requests;
- deny-by-default Operation authorization;
- idempotent Operation retry at the Patchbay boundary;
- target session generation checks;
- emergency revocation controls;
- security event audit with secret redaction.

Reserved extension seams:

- multiple human operators and shared authority domains;
- passkeys, MFA, or WebAuthn-only authentication;
- OAuth/OIDC integration;
- mutual TLS for browser or adapter endpoints;
- fine-grained RBAC administration;
- third-party control surfaces;
- SIEM export and long-retention compliance archives;
- lease-backed exclusive coordination.

Rejected v0.1.0 directions:

- unauthenticated web control;
- UI-only authorization;
- long-lived JavaScript-readable browser bearer tokens as the primary session model;
- Pi-specific principals or Pi-specific permission names in core protocol;
- best-effort hidden delivery when a grant is absent;
- logging raw secrets or prompt bodies by default.
