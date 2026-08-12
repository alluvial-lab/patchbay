---
id: release-v0.2.1
kind: release
stage: released
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

## Gate runs (top-level scanner fan-out)
- **gate-security** (2026-08-12): Critical=0 High=0 Medium=1 — loopback-HTTP proxy bleed (parked `gate-security-v0.2.1-loopback-http-proxy-bleed`)
- **gate-tests** (2026-08-12): 2 medium coverage gaps — cross-adapter diagnostic-ingest RPC + non-loopback IPv6/`0.0.0.0` HTTP cases (parked `gate-tests-v0.2.1-security-coverage-gaps`)
- **gate-cruft** (2026-08-12): 0
- **gate-docs** (2026-08-12): 5 drift — CHANGELOG entry + RUNBOOK migration handled at ship; README/SECURITY/PROTOCOL auth-posture staleness parked (`gate-docs-v0.2.1-auth-posture-staleness`)
- **gate-patterns** (2026-08-12): 0

Park policy: park low/medium; **0 critical / 0 high → ship unblocked.** Unlike v0.2.0, these gates ran the real top-level scanner fan-out (not a delegated inline fallback).

## Shipped

Bodies live in git history (delete-refs); `git show 3100627:<path>` recovers any pruned body.

| id | kind | archived_atop | git ref |
|----|------|---------------|---------|
| gate-security-adapter-attachment-secret-scoping | story | — | 3100627 |
| gate-security-token-commune-http-credentials | story | — | 3100627 |
| gate-security-forged-audit-sender | story | — | 3100627 |

- **Shipped:** 2026-08-12
- **Mapping:** tag-based (annotated tag `v0.2.1` pushed)
- **Items shipped:** 3
- **Gate totals:** security 0C/0H/1M(parked) · tests 2M(parked) · cruft 0 · docs 5 (2 fixed at ship, 3 parked) · patterns 0
- **Breaking change:** `PATCHBAY_ADAPTER_ATTACHMENT_CREDENTIALS` (JSON per-adapter map) replaces the core's shared `PATCHBAY_ADAPTER_ATTACHMENT_SECRET` — see RUNBOOK migration note.
