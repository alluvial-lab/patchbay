---
source_handle: mdn-set-cookie
fetched: 2026-07-07
source_url: https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Set-Cookie
provenance: source-direct
---

# MDN Set-Cookie

## Structural metadata

- Source type: MDN Web Docs HTTP header reference.
- Fetched representation: browser-readable HTML rendered to text with `lynx`.
- Local fetched copy: `.research/fetched/v0-stack-tooling/ts-web-and-browser/mdn-set-cookie.txt`.

## Paraphrased source summary

MDN defines the `Set-Cookie` response header syntax and cookie attributes, including `Domain`, `HttpOnly`, `Path`, `Secure`, and `SameSite`. The page explains how `HttpOnly` affects JavaScript access, how `Secure` is tied to HTTPS except for localhost, how `SameSite` changes cross-site sending behavior, and how cookie name prefixes such as `__Host-` impose browser-enforced constraints.

## Key passages

1. `HttpOnly` "forbids JavaScript from accessing the cookie" via `Document.cookie`, while the cookie is still sent with JavaScript-initiated requests such as `XMLHttpRequest.send()` or `fetch()`; MDN says this mitigates cross-site scripting attacks against the cookie value.

2. `SameSite` "controls whether or not a cookie is sent with cross-site requests" and provides "some protection" against attacks including CSRF. MDN lists `Strict`, `Lax`, and `None`; `None` sends the cookie with same-site and cross-site requests and requires `Secure`.

3. `Secure` means the cookie is sent only over `https:` requests except on localhost. MDN warns not to treat `Secure` alone as full protection because cookies can still be read or modified if `HttpOnly` is not set, and notes that insecure sites cannot set `Secure` cookies except for localhost's ignored HTTPS requirement.

4. For `__Host-` cookies, MDN says the cookie must be set with `Secure` from a secure page, must not specify `Domain`, and must set `Path=/`; MDN says this guarantees the cookie is sent only to the host that set it and not other hosts on the domain.

5. MDN's cookie-prefix examples show accepted `__Host-ID=123; Secure; Path=/` and rejected variants missing `Secure`, missing `Path=/`, or setting a `Domain` attribute.
