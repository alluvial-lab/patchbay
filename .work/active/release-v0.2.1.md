---
id: release-v0.2.1
kind: release
stage: quality-gate
tags: []
parent: null
depends_on: []
release_binding: v0.2.1
gate_origin: null
created: 2026-08-12
updated: 2026-08-12
---

# Release v0.2.1

Security patch following the retroactive deep scan of v0.2.0. Closes the 1 High + 2 Medium findings the inline release gate missed.

## Bound items
- `gate-security-adapter-attachment-secret-scoping` (High) — per-adapter attachment credentials replace the shared secret; cross-adapter attach/replace/subscribe/ingest blocked. **Breaking config change:** `PATCHBAY_ADAPTER_ATTACHMENT_CREDENTIALS` now required (was `PATCHBAY_ADAPTER_ATTACHMENT_SECRET`).
- `gate-security-token-commune-http-credentials` (Medium) — HTTPS required for non-loopback token-commune gateways; plaintext HTTP restricted to verified loopback.
- `gate-security-forged-audit-sender` (Medium) — observation audit attribution derived from authenticated attachment context; forged actor/endpoint/device claims rejected.

## Gate runs
<populated as gates run>
