---
source_handle: nist-session-management
fetched: 2026-06-28
source_url: https://pages.nist.gov/800-63-4/sp800-63b/session/
provenance: source-direct
substrate_confidence: source-direct
---

# Attestation: NIST SP 800-63B session management

## Summary

NIST SP 800-63B describes authenticated session management. It says sessions may be started after authentication and terminated by inactivity timeout or logout. A session secret binds subscriber software to the service; continuity of an authenticated session is based on possession of a session secret issued at authentication and optionally refreshed. Session secrets must be established using an approved random bit generator, be at least 64 bits, be invalidated on logout, be transferred over authenticated protected channels or derived from such channels, time out, and not fall back to insecure transport. NIST also says POST/PUT content must contain a session identifier verified for CSRF protection.

## Key passages

1. From the session-management introduction:

> A session may be started in response to an authentication event and continue until it is terminated. The session may be terminated for any number of reasons, including an inactivity timeout or an explicit logout event. The session may be extended through a reauthentication event.

2. From "Session Bindings":

> A session secret shall be shared between the subscriber's software and the accessed service. This secret binds the two ends of the session and allows the subscriber to continue using the service over time.

3. From "Session Bindings":

> The continuity of authenticated sessions shall be based on the possession of a session secret that is issued by the session host at the time of authentication and optionally refreshed during the session. The nature of a session depends on the application, such as: A web browser session with a "session" cookie.

4. From the session-secret requirements list:

> Secrets are established during or immediately following authentication. Secrets are established using input from an approved random bit generator ... and are at least 64 bits in length. Secrets are erased or invalidated by the session subject when the subscriber logs out.

5. From the session-secret requirements list:

> Secrets are either transferred ... via an authenticated protected channel or derived from keys that are established as part of establishing a valid, mutually authenticated protected channel. Secrets will time out ... Secrets are unavailable to intermediaries between the host and the subscriber's endpoint.

6. From the session-security requirements after the list:

> Following authentication, authenticated sessions shall not fall back to an insecure transport (e.g., from https to http). POST/PUT content shall contain a session identifier that the RP shall verify to protect against cross-site request forgery (CSRF).

7. From the endpoint storage guidance:

> [Session secrets] should not be placed in insecure locations (e.g., HTML5 Local Storage) due to the potential exposure of local storage to cross-site scripting (XSS) attacks.
