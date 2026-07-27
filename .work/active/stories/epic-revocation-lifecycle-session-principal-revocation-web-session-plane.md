---
id: epic-revocation-lifecycle-session-principal-revocation-web-session-plane
kind: story
stage: done
tags: [security]
parent: epic-revocation-lifecycle-session-principal-revocation
depends_on: [epic-revocation-lifecycle-session-principal-revocation-core-state]
release_binding: null
gate_origin: null
created: 2026-07-27
updated: 2026-07-27
---

# Web browser-session revocation projection

## Checkpoint

Complete browser session records with verified endpoint/device identity,
core-issued operator-session generation, and `revokedAt`; expose the generated
revocation RPCs through CSRF-protected gRPC-Web routes; and keep the local
browser projection fail-closed when revoke-all is attempted.

This checkpoint owns Unit 3 in the parent feature. The parent design is
authoritative for the TypeScript record/signature shapes, local/core asymmetry,
route behavior, and the decision not to mock the lockdown-owned cockpit safety
surface.

## Acceptance evidence

- Every browser record carries the fields committed by `SECURITY.md:94`, with
  endpoint/device/generation copied only from core-issued authentication
  evidence.
- Local revoke/current/all/endpoint/device paths set `revokedAt` once and retain
  recognized records; expiration leaves `revokedAt` null.
- Revoke-all from a browser invalidates caller and siblings locally as well as
  invoking the core. If the core is unavailable, local records still fail
  closed and the response does not claim core-wide success.
- Principal/endpoint/device self-revocation invalidates matching local browser
  sessions after confirmed core success; later dead-core-session errors retain
  the existing bridge-record invalidation behavior.
- CSRF, Origin, Fetch Metadata, cookie, and integration tests remain green with
  no new browser-supplied identity authority.

## Ordering constraints

Consumes the stable core RPCs. Do not add cockpit controls or mockups here; the
parent epic assigns that composition and warning/confirmation UX to the
lockdown sibling.

## Implementation notes
- Execution capability: inline implementation; the web session projection and gRPC-Web bridge share one boundary.
- Review weight: standard (project default).
- Files changed: `web-server/src/sessions.ts`, `src/main.ts`, `src/routes/login.ts`, `src/routes/rpc.ts`, and session/integration tests.
- Tests added/removed: session identity/fence, retained revocation timestamps, revoke-all fail-closed behavior, and confirmed self principal/endpoint invalidation tests added.
- Simplification: local invalidation is expressed through `SessionStore` matching helpers; the bridge routes only project local records after core confirmation except revoke-all, which always fences locally.
- Discrepancies from design: compatibility overloads retain legacy local test callers while production login uses the full verified `SessionIdentity` shape.
- Adjacent issues parked: none.
- Verification: `cd web-server && npm test` passed (29 tests).
