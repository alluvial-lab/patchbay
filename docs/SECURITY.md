# Patchbay Security

Patchbay is a high-authority control plane: a browser or CLI action can mutate remote/headless agent sessions. V0 therefore treats security as part of protocol semantics, not as a UI add-on.

This document defines the v0 security posture for one human operator, one authoritative coordination core, a responsive web cockpit, a CLI, and the first Pi adapter. It should be read with `docs/PROTOCOL.md` for command/grant state and `docs/VERIFICATION.md` for model obligations.

Research grounding: `.research/analysis/briefs/web-control-security.md`.

## Security objectives

V0 security must ensure:

- only authenticated operator endpoints can submit control actions;
- every command is authorized before durable acceptance;
- accepted commands bind to the intended target session and generation;
- retries cannot double-apply intent at the Patchbay boundary;
- browser sessions can be revoked promptly;
- stale or late events cannot mutate newer session state;
- audit records preserve security-relevant decisions without storing secrets.

Patchbay does not prove cryptographic primitives, operating-system isolation, browser correctness, network latency bounds, or third-party harness internals. Those are deployment and adapter assumptions.

## V0 authority domain

V0 has one human operator and one authority domain. This is a product scope decision, not a reason to omit authority modeling.

The model keeps these concepts explicit:

- **Operator** — the single human who controls Patchbay in v0.
- **Actor** — represented participant: operator, agent, adapter, daemon, service, or control surface.
- **Device** — a physical or virtual host that can run one or more endpoints, such as a browser on a laptop, a CLI on a VM, or an adapter process near a runtime.
- **Endpoint** — a concrete browser, CLI, adapter process, or other connection-bearing instance for an actor on a device.
- **Operator session** — an authenticated browser or CLI session for the operator, backed by a server-side session record.
- **Runtime session** — an adapter-reported control target, such as a Pi session.
- **Grant** — an authority relationship permitting an actor or endpoint to perform command kinds against a target scope.
- **Authority domain** — the single core-owned context in which grants, revocation, routing authority, and audit are evaluated.

Future multi-operator coordination remains a reserved extension seam. V0 data structures should not assume there can only ever be one operator, but v0 UX and provisioning do not implement multi-human administration, handoffs, or shared authority domains.

## Threat model

### In scope

V0 is designed against:

- unauthenticated browser or CLI access;
- unauthorized browser, CLI, or adapter endpoint enrollment;
- CSRF attempts against the web cockpit;
- replayed command submissions at the Patchbay boundary;
- command submission from a revoked browser/device endpoint;
- commands aimed at the wrong session generation;
- stale adapter events or late replies mutating newer state;
- confused-deputy routing where UI labels or payload fields override verified identity;
- unsupported adapter commands being presented as available;
- accidental logging of session cookies, CSRF tokens, passwords, bootstrap secrets, or prompt bodies;
- deployment mistakes that expose an unauthenticated or HTTP-only non-localhost core;
- adapter-to-core channels that bypass Patchbay identity, capability, or grant checks.

### Out of scope

V0 does not attempt to solve:

- malicious code execution inside an already-controlled host;
- compromise of the operator's browser, device, password manager, or OS account;
- malicious or backdoored agent harnesses beyond adapter-declared behavior;
- correctness of cryptographic libraries;
- multi-operator social workflows or organization-level policy;
- high availability, replicated authority domains, or split-brain recovery;
- public multi-tenant internet service hardening.

Out-of-scope does not mean irrelevant. These risks belong to deployment guidance, adapter documentation, future features, or operator operational discipline.

## Enrollment and authentication

> **Provisional** (`story-review-provisional-semantics` candidate 2): the enrollment posture below came from research and was adopted without an alternatives pass. Under review.

V0 enrollment is intentionally narrow:

- The first operator is created through CLI/local-console bootstrap, not through an unauthenticated network setup page.
- Bootstrap produces a one-time setup secret that expires after use or timeout.
- Setup establishes the operator's primary authenticator: password/passphrase for v0, with passkeys or MFA reserved as an extension seam.
- A browser endpoint enrolls only after successful operator authentication and creation of a server-side operator session.
- A CLI endpoint enrolls only through local setup credentials or an existing authenticated operator session.
- An adapter endpoint enrolls only through configured adapter attachment material or an adapter-specific trust root; an adapter cannot self-assert core authority by display name or payload field.
- Device labels are operator-facing metadata. The authority boundary is the endpoint/operator-session/grant tuple, not the label.

Interactive login must be rate-limited, track failed attempts against the operator account as well as useful network metadata, and audit both success and failure. Rate limiting and lockout policy must avoid making denial of service easier than authentication.

## Browser session model

The web cockpit uses server-side sessions. The browser receives only an opaque session cookie; command authority, endpoint metadata, grants, and session state remain server-side.

V0 browser-session requirements:

- session identifiers are high-entropy, meaningless client-side values;
- session records include operator id, endpoint id, created time, last-used time, expiration, revoked time, and session generation;
- session cookies use `HttpOnly`, `Secure` outside localhost, `SameSite=Strict` by default, `Path=/`, and no `Domain` attribute;
- non-localhost deployments require HTTPS or a trusted HTTPS-terminating reverse proxy before browser sessions are accepted;
- localhost development may use the browser's localhost secure-cookie exception, but that exception must not be generalized to LAN/IP/container deployments;
- session secrets are never stored in localStorage;
- login, logout, session renewal, and session revocation are audit records;
- reauthentication is required after suspicious session signals or before future high-risk operations when those operations are introduced.

Default cookie shape:

```http
Set-Cookie: __Host-patchbay_session=<opaque>; Path=/; Secure; HttpOnly; SameSite=Strict
```

`SameSite=Lax` is a reserved fallback only for a concrete operator flow that requires top-level cross-site navigation into Patchbay. `SameSite=None` is not a v0 default.

## CSRF and browser request protection

SameSite is defense-in-depth, not the whole CSRF model.

Every state-changing web route must require:

- an authenticated operator session cookie;
- a CSRF token tied to that operator session;
- a custom request header for API/XHR/fetch mutations;
- Origin and/or Fetch Metadata checks where the browser provides them;
- a non-GET method for mutations.

Unauthenticated setup pages must not expose state-changing bootstrap flows on a network listener. First-run setup must use the CLI, local console output, or a one-time bootstrap secret that becomes invalid after setup.

## Command authorization and replay resistance

A command is accepted only after Patchbay validates:

1. payload shape and command kind (the kind must be a known Patchbay command kind);
2. authenticated issuer session or endpoint;
3. target actor/session identity and generation;
4. idempotency key or command id;
5. command expiration window;
6. a live, unrevoked grant permitting that issuer to perform that command kind on that target scope.

Authorization is deny-by-default. Missing, expired, revoked, target-mismatched, or command-kind-mismatched grants produce `SubmissionOutcome = rejected` with `authorization_denied` or the narrower applicable failure term from `docs/PROTOCOL.md`.

Retries with the same idempotency key return the existing command record. A new intentional action requires a new command id/key.

Sender identity comes from the verified connection/session context. Payload display names, human labels, project names, cwd values, and adapter-reported friendly names are never routing authority.

### Compound issuer

When a command arrives at the core through a control surface (the v0 path is browser → web server → core), the core verifies a compound issuer: the operator actor is the grant subject and is verified against operator-session evidence, and the transport endpoint (the web server, or a CLI endpoint) is verified as a principal. The core must not trust a self-asserted operator identity. The exact wire/evidence shape for how operator-session evidence crosses the web↔core seam is deferred to `feature-web-core-protocol-seam`; this document commits only to the requirement that the core independently verify both the transport principal and the operator identity.

## Grant shape

A v0 grant has at least:

- grant id;
- authority domain id;
- subject actor id;
- optional subject endpoint id or endpoint class;
- target scope: actor, adapter, runtime session, project/session group, or other modeled resource;
- allowed command kinds;
- created time and provenance;
- optional expiration;
- revocation generation or revoked time;
- revocation policy for already accepted commands.

Delegation is a reserved future direction, not a v0 field; a `parent grant id / delegated-by` field is intentionally absent from v0. Device is part of the identity model (for audit and revocation grouping) but is not a grant-matching field. Adapter capability sets are not grant authority; they are advisory UX declarations, and the adapter is the authority on its own support at delivery time.

Grant checks are centralized in the coordination core. Control surfaces may hide unavailable actions, but UI availability is never authoritative.

## Revocation model

> **Provisional** (`story-review-provisional-semantics` candidate 3): the five revocation actions below (especially "security lockdown" as a named posture) were invented without a design pass. Under review.

Revocation prevents future authority. Already accepted commands follow the policy attached to their grant and command kind:

- **continue** — preserve already accepted work, but reject future commands;
- **cancel** — submit or record cancellation for accepted non-terminal commands when supported;
- **require reauthorization** — hold or reject delivery until a fresh grant/session is established.

V0 must support these operator-facing revocation actions:

1. **Revoke current browser session** — delete or mark the session revoked and clear its cookie.
2. **Revoke all browser sessions** — invalidate all operator-session generations, optionally by rotating the server-side session-signing/encryption secret.
3. **Revoke endpoint/device** — mark a browser or CLI endpoint revoked and reject future commands from it.
4. **Revoke adapter/session grant** — stop command acceptance for a target scope while preserving audit history.
5. **Security lockdown** — reject new commands, mark affected runtime sessions stale, require fresh login, and record the reason.

Revocation never deletes command history. Late events after revocation are audit/reconciliation events unless they are valid transitions for commands already accepted under the relevant policy.

## Audit events

Security audit is part of v0. The audit log should be durable and queryable even before a full audit UI exists.

Audit records are distinct from durable command/session state-transition events. They may record rejected attempts, failed checks, and security decisions that do not create command records.

Minimum audit records:

- bootstrap started/completed/expired;
- login success/failure;
- logout;
- operator-session created, renewed, expired, revoked;
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

Audit records must not directly store raw session cookies, CSRF tokens, access tokens, passwords, bootstrap secrets, encryption keys, command prompt bodies by default, or sensitive attachments. If command content logging is later added, it must be an explicit policy with redaction and operator-visible controls.

## Deployment posture

Allowed v0 deployments:

- localhost development;
- local workstation service;
- VM or container behind local access controls, with HTTPS required for non-localhost browser sessions;
- LAN, VPN, or reverse-proxy deployment with HTTPS and authenticated browser sessions;
- split deployment where adapters run near runtimes and the core remains the single authority.

Forbidden v0 deployments:

- internet-exposed unauthenticated core;
- non-localhost HTTP browser access carrying authenticated sessions;
- deployments that bypass Patchbay authority checks by sending browser commands directly to adapters;
- deployments that treat adapter-reported display names as routing authority;
- deployments that disable audit for command acceptance, authorization, or revocation.

## Extension pressure classification

Committed v0 behavior:

- one operator and one authority domain;
- explicit actor, device, endpoint, operator-session, runtime-session, grant, revocation, and audit concepts;
- server-side browser sessions with hardened cookies;
- CSRF protection for state-changing web requests;
- deny-by-default command authorization;
- idempotent command retry at the Patchbay boundary;
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

Rejected v0 directions:

- unauthenticated web control;
- UI-only authorization;
- long-lived JavaScript-readable browser bearer tokens as the primary session model;
- Pi-specific principals or Pi-specific permission names in core protocol;
- best-effort hidden delivery when a grant or adapter capability is absent;
- logging raw secrets or prompt bodies by default.
