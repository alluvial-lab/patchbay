---
id: gate-docs-runbook-web-operator-config
kind: story
stage: implementing
tags: [documentation]
parent: null
depends_on: []
release_binding: v0.1.0
gate_origin: docs
created: 2026-07-24
updated: 2026-07-24
---

# Runbook marks required web operator identity optional

## Drift category
foundation-doc-assertion

## Location
- Doc: `docs/RUNBOOK.md:33`
- Contradicting source: `web-server/src/main.ts:57`

## Current doc text
> | `PATCHBAY_OPERATOR_ID` / `PATCHBAY_OPERATOR_PASSWORD_HASH` | web-server | no | First-run fallback only; the core's operator record is the source of truth after bootstrap. |

## Contradiction
`loadConfig` calls `requireNonEmpty(env.PATCHBAY_OPERATOR_ID, "PATCHBAY_OPERATOR_ID")`, so the web server refuses startup when that variable is absent. The core remains the password-verification authority after bootstrap (`web-server/src/main.ts:226-241`), but the web process still needs the configured operator id to issue that verification request.

## Required edit
Mark `PATCHBAY_OPERATOR_ID` as required for the web server, distinguish it from the optional password-hash fallback, and state the post-bootstrap core verification behavior accurately.
