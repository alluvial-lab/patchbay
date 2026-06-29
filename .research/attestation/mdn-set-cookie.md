---
source_handle: mdn-set-cookie
fetched: 2026-06-28
source_url: https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Set-Cookie
provenance: source-direct
substrate_confidence: source-direct
---

# Attestation: MDN Set-Cookie reference

## Summary

MDN documents cookie attributes relevant to browser session binding: `HttpOnly` blocks JavaScript access while still allowing cookies to be sent on JavaScript-initiated requests, `SameSite` controls whether cookies are sent with cross-site requests and can protect against some CSRF attacks, `Secure` sends cookies only over HTTPS except localhost, and `__Host-` cookies must be Secure, host-only, and `Path=/`.

## Key passages

1. From `HttpOnly`:

> Forbids JavaScript from accessing the cookie, for example, through the `Document.cookie` property. Note that a cookie that has been created with `HttpOnly` will still be sent with JavaScript-initiated requests ... This mitigates attacks against cross-site scripting (XSS).

2. From `SameSite=<samesite-value>`:

> Controls whether or not a cookie is sent with cross-site requests ... This provides some protection against certain cross-site attacks, including cross-site request forgery (CSRF) attacks.

3. From `SameSite` values:

> `Strict`: Send the cookie only for requests originating from the same site that set the cookie. `Lax`: Send the cookie only for requests originating from the same site that set the cookie, and for cross-site requests that meet [top-level navigation and safe method] criteria.

4. From `Secure`:

> Indicates that the cookie is sent to the server only when a request is made with the `https:` scheme (except on localhost), and therefore, is more resistant to man-in-the-middle attacks. ... Insecure sites (`http:`) cannot set cookies with the `Secure` attribute. The `https:` requirements are ignored when the `Secure` attribute is set by localhost.

5. From `Cookie prefixes`:

> `__Host-`: Cookies with names starting with `__Host-` must be set with the `Secure` attribute by a secure page (HTTPS). In addition, they must not have a `Domain` attribute specified, and the `Path` attribute must be set to `/`. This guarantees that such cookies are only sent to the host that set them, and not to any other host on the domain.
