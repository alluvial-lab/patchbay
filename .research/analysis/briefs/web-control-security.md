---
provenance: agent-synthesis
updated: 2026-06-28
research_item: feature-research-web-control-security
intent: inform-architecture-decision
output_kind: synthesis-brief
---

# Web-first control plane security patterns for Patchbay v0

## Recommendation

For v0, Patchbay should treat the web cockpit as a **single-operator, high-authority browser control surface** backed by server-side sessions, explicit command authorization, CSRF protection on every state-changing request, and durable audit events for control actions.

The recommended v0 posture is:

1. **Use server-side browser sessions**, not long-lived browser bearer tokens. Issue a high-entropy meaningless session identifier in a `__Host-` cookie with `HttpOnly`, `Secure` outside localhost, `SameSite=Strict` by default, `Path=/`, and no `Domain` attribute. OWASP requires session identifiers to have at least 64 bits of entropy and be generated with a CSPRNG [owasp-session-management]{1}; OWASP also says session IDs should be meaningless client-side identifiers with application meaning stored server-side [owasp-session-management]{2}. OWASP and MDN both describe `HttpOnly`, `SameSite`, `Secure`, and `__Host-` cookie protections relevant to this shape [owasp-session-management]{3} [owasp-session-management]{4} [owasp-session-management]{5} [mdn-set-cookie]{1} [mdn-set-cookie]{2} [mdn-set-cookie]{4} [mdn-set-cookie]{5}.
2. **Authenticate once, then bind continuity to a session secret**. NIST describes authenticated session continuity as possession of a session secret issued at authentication, including web browser sessions with session cookies [nist-session-management]{2} [nist-session-management]{3}. NIST also requires such secrets to be random, at least 64 bits, invalidated on logout, timeout-bound, unavailable to intermediaries, and transferred over authenticated protected channels or derived from them [nist-session-management]{4} [nist-session-management]{5}.
3. **Do not rely on SameSite alone for CSRF.** Use framework CSRF support if available; otherwise use synchronizer tokens for server-side sessions, place the token in a custom request header for API requests, verify Origin / Fetch Metadata where practical, and reject state changes through GET [owasp-csrf]{1} [owasp-csrf]{2} [owasp-csrf]{3} [owasp-csrf]{5} [owasp-csrf]{7}. NIST also states that POST/PUT content should contain a session identifier verified for CSRF protection [nist-session-management]{6}.
4. **Make authority checks command-local and deny-by-default.** Even with one human operator, each command submission should pass through a central grant check. OWASP recommends least privilege [owasp-authorization]{1}, deny-by-default authorization [owasp-authorization]{2} [owasp-authorization]{3}, and permission validation on every request [owasp-authorization]{4}.
5. **Make emergency revocation boring and immediate.** V0 should support “revoke this browser/device session,” “revoke all sessions,” “disable a target adapter/session grant,” and, as a Patchbay implementation control, “rotate the server session-signing/encryption secret” when the deployment needs to invalidate every browser session at once. Session IDs should be renewed or regenerated after privilege changes [owasp-session-management]{6}; session secrets should be invalidated on logout and timeout-bound [nist-session-management]{4} [nist-session-management]{5}.
6. **Audit control actions, not secrets.** Log authentication successes/failures, authorization failures, session-management failures, command acceptance/delivery/terminal state, grant changes, revocation, adapter attach/detach, and target changes. OWASP says application logging should include security events [owasp-logging]{1}, be risk-proportionate rather than checklist-only [owasp-logging]{2}, and include authentication, authorization, and session-management failures where possible [owasp-logging]{3}. OWASP also says session IDs, access tokens, passwords, encryption keys, and sensitive personal data should not be recorded directly in logs [owasp-logging]{4}.

## V0 authentication and device/session model

### Bootstrap and login

Patchbay v0 can start with one operator account and a local bootstrap secret or CLI-created password/passphrase. That keeps the operator model simple while preserving future multi-operator seams through explicit actor, endpoint, session, and grant records.

Recommended v0 behavior:

- First run creates or prints a one-time bootstrap secret through the CLI or local console, not through an unauthenticated web page exposed to the network.
- The operator completes setup and establishes a password or passkey-capable authenticator for the single account.
- Interactive login is rate-limited and records failed attempts. OWASP describes login throttling as a control to prevent too many password guesses and notes maximum-attempt controls [owasp-authentication]{5}; it also recommends associating failed-login counters with the account rather than only source IP [owasp-authentication]{6}.
- MFA/passkeys should be a reserved seam, and a strong recommendation for internet-exposed deployments, but not a mandatory v0 dependency. OWASP recommends MFA wherever possible while acknowledging it may not be practical or feasible to enforce for every application audience [owasp-authentication]{4}.

### Browser session binding

Patchbay should represent each authenticated browser as a server-side `operator_session` record:

- opaque session id / secret;
- operator id;
- endpoint/device label;
- created, last-used, expires-at, revoked-at;
- soft metadata such as user agent and coarse network origin;
- session generation for revocation and stale-event rejection.

The browser receives only the session cookie. Keep command authority, target grants, and device metadata server-side because OWASP says the business/application meaning of the session ID belongs server-side [owasp-session-management]{2}. Do not store the session secret in localStorage because NIST warns session secrets should not be placed in insecure locations such as HTML5 Local Storage due to XSS exposure [nist-session-management]{7}.

Hard IP binding is not recommended for v0. It can break ordinary remote/container/reverse-proxy use and is not required by the attested session-binding sources. Use IP and user-agent changes as audit/risk signals that can trigger reauthentication rather than as primary proof of possession. OWASP lists unusual login patterns, IP address changes, and device enrollments as examples of suspicious activity that can trigger reauthentication [owasp-authentication]{3}.

## Browser session, CSRF, replay, and transport protection

### Cookie policy

Use this default cookie shape for the web cockpit:

```http
Set-Cookie: __Host-patchbay_session=<opaque>; Path=/; Secure; HttpOnly; SameSite=Strict
```

For localhost development, the `Secure` attribute can still be used in modern browser behavior because MDN notes the HTTPS requirement for `Secure` is ignored when set by localhost [mdn-set-cookie]{4}. For non-localhost container, LAN, VM, or cloud access, require HTTPS or a trusted reverse proxy that terminates HTTPS before accepting browser sessions; NIST says authenticated sessions must not fall back from HTTPS to HTTP [nist-session-management]{6}.

`SameSite=Strict` is the default because the cockpit is first-party and high authority. Use `SameSite=Lax` only if a concrete operator flow requires top-level cross-site navigation into Patchbay; OWASP says `Strict` is preferred and `Lax` is the fallback for session cookies [owasp-session-management]{4}, while MDN defines the request behavior for both values [mdn-set-cookie]{3}.

### CSRF policy

Every state-changing route must require:

- authenticated session cookie;
- CSRF token tied to the session;
- custom request header for API/XHR/fetch calls;
- Origin / Fetch Metadata allow-check where available [owasp-csrf]{3} [owasp-csrf]{7};
- non-GET method for state changes.

This combines OWASP's guidance to add CSRF tokens to state-changing requests, use synchronizer tokens for stateful software, consider custom request headers for API-driven sites, verify origin with standard headers, and avoid GET for state changes [owasp-csrf]{1} [owasp-csrf]{2} [owasp-csrf]{3} [owasp-csrf]{5}. If Patchbay later needs stateless web sessions, use a signed double-submit cookie explicitly bound to session-specific data, because OWASP says the signed double-submit pattern should bind CSRF tokens to the authenticated session [owasp-csrf]{6}.

### Replay and command submission

Transport-level replay protection is not enough for Patchbay commands because a command can mutate an external agent session. V0 command submission should require:

- active authenticated browser session;
- valid CSRF token;
- command id or idempotency key;
- target session generation;
- short command validity window;
- server-side deduplication before delivery;
- authorization check immediately before acceptance.

The external support for this pattern is session-secret freshness and CSRF protection: NIST requires session secrets to time out and be unavailable to intermediaries [nist-session-management]{5}, and OWASP requires every request to pass authorization checks [owasp-authorization]{4}. Patchbay's own protocol should carry the command idempotency and target-generation rules.

## Remote control safety

### Grants and command authority

Model v0 as one operator but not as “no authorization.” The operator still acts through browser endpoints, CLI endpoints, adapter endpoints, and runtime sessions. Each command should be authorized against a grant that names:

- actor / endpoint;
- target session or adapter scope;
- command kind;
- expiration or revocation generation;
- reason / provenance when created by setup.

This keeps least privilege and deny-by-default in the core from the first implementation [owasp-authorization]{1} [owasp-authorization]{2}. It also gives future multi-operator support somewhere to grow without changing command semantics.

### Emergency revocation

V0 should include these operator controls:

1. **Revoke current browser session** — delete server-side session record and clear cookie.
2. **Revoke all browser sessions** — increment session-secret generation and invalidate all session records.
3. **Revoke endpoint/device** — mark a browser/CLI endpoint revoked and reject future commands from it.
4. **Revoke adapter/session grant** — stop command acceptance for a target while preserving audit history.
5. **Security lockdown** — reject new commands, mark remote sessions stale, and require fresh login before control resumes.

These actions align with the attested requirement that sessions terminate through logout/timeout [nist-session-management]{1}, session secrets be invalidated on logout [nist-session-management]{4}, and session IDs be renewed after privilege changes [owasp-session-management]{6}. In Patchbay-specific terms, revocation should not erase accepted commands; it should produce auditable terminal outcomes or require reauthorization according to the command policy.

### Audit logging

Patchbay's audit log should be durable and queryable even before a full product UI exists. Minimum v0 events:

- login success/failure;
- logout and session revocation;
- failed CSRF/origin check;
- failed authorization;
- command submission accepted/rejected/failed/unknown;
- command delivery/running/terminal events;
- target session generation mismatch or stale event;
- grant creation/revocation;
- adapter attach/detach/failure;
- emergency lockdown.

Log correlation IDs, actor/endpoint IDs, target IDs, command IDs, state transitions, timestamps, and result codes. Do not log raw session cookies, CSRF tokens, access tokens, passwords, bootstrap secrets, command prompt bodies by default, or sensitive payload attachments. OWASP recommends logging security events and auth/authz/session failures [owasp-logging]{1} [owasp-logging]{3}, while excluding or protecting session IDs, access tokens, passwords, keys, and sensitive personal data [owasp-logging]{4}.

## V0 vs reserved-future security

### Necessary for v0

- Server-side sessions with hardened cookies.
- Login throttling and audit of auth failures.
- CSRF token/header/origin defenses for every state-changing route.
- HTTPS requirement for non-localhost browser access.
- Deny-by-default command authorization against explicit grants.
- Idempotency keys, target generation, command expiry, and durable command lifecycle.
- Operator-visible session/device list with revoke controls.
- Security audit log with secret redaction.

### Reserve but do not require for v0

- Multi-operator organization model.
- Full OAuth/OIDC provider mode.
- Mandatory MFA/passkeys for all deployments.
- WebAuthn-only or hardware-backed device-bound sessions.
- Mutual TLS for browser clients.
- Fine-grained RBAC administration UI.
- SIEM integrations and long-retention compliance archive.
- Public internet multi-tenant hardening.

These reserved items are not bad directions; they are broader than the v0 single-operator slice. The important v0 move is to keep their seams explicit: actor, endpoint, grant, session generation, revocation generation, audit event, and command authority should already exist.

## Disconfirming analysis

One counterargument to cookie sessions is the appeal of bearer tokens in SPA architectures. The attested sources push against putting session secrets where JavaScript can read them: MDN says `HttpOnly` forbids JavaScript access to cookies and mitigates XSS theft [mdn-set-cookie]{1}, while NIST warns against placing session secrets in insecure locations such as localStorage due to XSS exposure [nist-session-management]{7}. For Patchbay's high-authority browser cockpit, this favors server-side sessions with `HttpOnly` cookies.

One counterargument to requiring MFA/passkeys in v0 is scope. OWASP recommends MFA wherever possible but says it may not be practical or feasible to enforce depending on the application audience [owasp-authentication]{4}. For Patchbay, the compromise is to reserve passkey/MFA seams and recommend them for remote exposure, while not blocking the walking skeleton on them.

One counterargument to SameSite+CSRF tokens is that a self-hosted single-operator deployment might be “local only.” That does not hold for Patchbay's target shape: it may run in containers, VMs, home servers, or cloud hosts, and it controls remote/headless agent sessions. OWASP recommends CSRF tokens for state-changing requests and defense-in-depth mitigations [owasp-csrf]{1} [owasp-csrf]{3}; NIST also requires POST/PUT content to contain a session identifier verified for CSRF protection [nist-session-management]{6}. Treat locality as a deployment hardening layer, not as the application security model.

## Contradictions and tensions

No direct source contradiction surfaced. The main tensions are:

- `SameSite=Strict` improves isolation but can break legitimate cross-site entry flows; `Lax` is an attested acceptable fallback for session cookies [owasp-session-management]{4} [mdn-set-cookie]{3}.
- MFA is recommended wherever possible, but OWASP explicitly acknowledges feasibility and audience constraints [owasp-authentication]{4}.
- Audit logs need enough data for incident reconstruction, but OWASP warns against directly logging session IDs, access tokens, passwords, keys, and sensitive personal data [owasp-logging]{4}.

## Revisit if

- Patchbay is exposed directly to the public internet rather than localhost, LAN, VPN, or reverse-proxy-authenticated environments.
- Multi-operator coordination enters v0 scope.
- Native mobile or Expo moves from future work into the executable slice.
- Commands gain destructive host-level actions beyond agent-session control.
- Browser push notifications or third-party identity providers become required.
- Adapter sessions require unattended control from non-browser endpoints.

## Acquisition candidates

None. No load-bearing source was blocked during this engagement.
