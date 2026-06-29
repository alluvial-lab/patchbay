---
source_handle: owasp-csrf
fetched: 2026-06-28
source_url: https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html
provenance: source-direct
substrate_confidence: source-direct
---

# Attestation: OWASP CSRF Prevention Cheat Sheet

## Summary

The OWASP CSRF Prevention Cheat Sheet recommends using framework-provided CSRF protections where available, adding CSRF tokens to all state-changing requests, using the synchronizer token pattern for stateful software, using signed double-submit cookies for stateless software, considering custom request headers for API-driven sites, using SameSite as one defense, verifying Origin headers as defense-in-depth, and avoiding state changes through GET.

## Key passages

1. From the introductory guidance list:

> First, check if your framework has built-in CSRF protection and use it. If the framework does not have built-in CSRF protection, add CSRF tokens to all state-changing requests (requests that cause actions on the site) and validate them on the backend.

2. From the introductory guidance list:

> Stateful software should use the synchronizer token pattern. Stateless software should use double submit cookies. If an API-driven site can't use `<form>` tags, consider using custom request headers.

3. From the introductory guidance list:

> SameSite Cookie Attribute can be used for session cookies but be careful to NOT set a cookie specifically for a domain. ... Consider verifying the origin with standard headers. Do not use GET requests for state changing operations.

4. From "Synchronizer Token Pattern":

> CSRF tokens should be generated on the server-side and they should be generated only once per user session or each request.

5. From the custom-header discussion:

> Since requests with custom headers are automatically subject to the same-origin policy, it is more secure to insert the CSRF token in a custom HTTP request header via JavaScript than adding a CSRF token in the hidden field form parameter.

6. From "Signed Double-Submit Cookie (RECOMMENDED)":

> The most secure implementation of the Double Submit Cookie pattern is the Signed Double-Submit Cookie, which explicitly ties tokens to the user's authenticated session ... Always bind the CSRF token explicitly to session-specific data.

7. From the introductory guidance list:

> If your software targets only modern browsers, you may rely on Fetch Metadata headers together with the fallback options described below to block cross-site state-changing requests.
