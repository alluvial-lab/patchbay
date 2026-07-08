---
source_handle: owasp-csrf
fetched: 2026-07-07
source_url: https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html
provenance: source-direct
---

# OWASP Cross-Site Request Forgery Prevention Cheat Sheet

## Structural metadata

- Source type: OWASP Cheat Sheet Series security guidance.
- Fetched representation: HTML rendered to text with `lynx`.
- Local fetched copy: `.research/fetched/v0-stack-tooling/ts-web-and-browser/owasp-csrf.txt`.

## Paraphrased source summary

The cheat sheet describes CSRF as an attack where a browser automatically includes cookies and causes unwanted state-changing actions on an authenticated site. It recommends framework CSRF support when available, synchronizer tokens for stateful software, session-bound signed double-submit tokens for stateless software, custom headers for API/AJAX requests, Fetch Metadata and Origin/Referer checks as mitigations, SameSite as defense in depth, and avoiding GET for state changes.

## Key passages

1. OWASP states that browser requests automatically include cookies, including session cookies, which is why a target site needs mechanisms that verify requester identity and authority for CSRF-sensitive actions.

2. OWASP says that if a framework lacks built-in CSRF protection, applications should add CSRF tokens to all state-changing requests and validate them on the backend.

3. OWASP says stateful software should use the synchronizer token pattern, and stateless software should use double-submit cookies.

4. For the synchronizer token pattern, OWASP says CSRF tokens should be generated server-side once per user session or per request; per-session implementations store the token in the session and use it until the session expires.

5. OWASP says the server-side component must verify that the request token exists and is valid, compare it to the token in the user session, and reject the request when the token is missing or does not match.

6. OWASP says CSRF tokens should be unique per user session, secret, and unpredictable. It says a token may be sent as a hidden form field, custom AJAX header, or JSON payload, but should not be transmitted in a cookie for synchronized-token patterns and must not leak in URLs or logs.

7. OWASP says custom headers are more secure than hidden fields for AJAX because requests with custom headers are automatically subject to the same-origin policy.

8. OWASP says the signed double-submit cookie pattern should explicitly tie tokens to the authenticated session, and that simply signing tokens without session binding gives minimal protection and remains vulnerable to cookie injection.

9. OWASP recommends considering Fetch Metadata headers, verifying Origin with standard headers, using SameSite for session cookies while avoiding broad domain cookies, and not using GET for state-changing operations.
