---
source_handle: owasp-session-management
fetched: 2026-07-07
source_url: https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html
provenance: source-direct
---

# OWASP Session Management Cheat Sheet

## Structural metadata

- Source type: OWASP Cheat Sheet Series security guidance.
- Fetched representation: HTML rendered to text with `lynx`.
- Local fetched copy: `.research/fetched/v0-stack-tooling/ts-web-and-browser/owasp-session-management.txt`.

## Paraphrased source summary

The cheat sheet gives requirements and recommendations for web session identifiers and cookie handling. It covers entropy, meaningless session identifier contents, server-side session state, transport and cookie attributes, cookie prefixes, `Domain`/`Path` scope, and browser storage risks.

## Key passages

1. OWASP says session identifiers must have at least 64 bits of entropy to prevent brute-force guessing attacks, and that a strong CSPRNG must be used to generate them.

2. OWASP says a session ID's content must be meaningless and that the meaning and application logic associated with the session ID must be stored server-side in session objects or a session-management database or repository.

3. OWASP recommends using the session ID created by the language or framework; if creating a custom session ID, OWASP says to use a CSPRNG with at least 128 bits and ensure uniqueness.

4. OWASP says the `Secure` cookie attribute must be used so session IDs are only exchanged over encrypted channels.

5. OWASP says the `HttpOnly` cookie attribute instructs browsers not to allow scripts to access cookies via `document.cookie`; it calls this mandatory to prevent session ID stealing through XSS, while noting that XSS+CSRF can still send requests that include the cookie.

6. OWASP says `SameSite` prevents cookies from being sent on cross-site requests, mitigates cross-origin leakage, and provides CSRF defense; it says session cookies must explicitly set `SameSite=Strict` preferred or `SameSite=Lax`, and never use `SameSite=None` without `Secure`.

7. OWASP describes `__Host-` cookies as requiring `Secure`, no `Domain`, and `Path=/`, says this prevents subdomain forgery and HTTPS downgrade attacks, and recommends `__Host-` for session IDs.

8. OWASP recommends not setting the `Domain` attribute for session cookies, thereby restricting the cookie to the origin server, and warns that permissive domain attributes allow cross-subdomain cookie attacks and session fixation.

9. OWASP warns not to store session IDs, refresh tokens, or credentials in localStorage or sessionStorage because JavaScript in the origin can access those APIs; it recommends `HttpOnly; Secure; SameSite=Strict` cookies or a backend-for-frontend pattern.
