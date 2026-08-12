---
id: gate-security-upgrade-dompurify
kind: story
stage: done
tags: [security, dependencies]
parent: null
depends_on: []
release_binding: null
gate_origin: security
created: 2026-08-11
updated: 2026-08-12
---

# Upgrade DOMPurify past the detached-subtree XSS advisory

## Severity
Medium

## Domain
Dependencies & Supply Chain

## Location
`web-cockpit/package-lock.json:773`

## Evidence

`npm --prefix web-cockpit audit --omit=dev` reports `dompurify` 3.4.12 affected by `GHSA-55q2-fjhq-7xh7`: removing an `IN_PLACE` hook can leave a detached executable subtree. Patchbay does not configure this hook today, reducing immediate exploitability, but cockpit markdown sanitation should not remain on an affected release.

## Remediation direction

Update DOMPurify to a fixed release, regenerate the lockfile, and run markdown sanitization, conformance-mutation, and cockpit tests.

## Release disposition

Parked unbound under the operator's v0.2.0 gate policy (medium risk); it does not block shipment.

## Implementation notes

- Execution capability: inline direct-read dependency repair; one package and its lockfile were sufficient, with no implementation fan-out.
- Review weight: standard (project default); standalone-story review remained bounded and inline.
- Root cause confirmed: `npm --prefix web-cockpit audit --omit=dev` reported GHSA-55q2-fjhq-7xh7 against installed DOMPurify 3.4.12 (`<=3.4.12`).
- Files changed: `web-cockpit/package.json`, `web-cockpit/package-lock.json`.
- Fix: raised the declared range to `^3.4.13` and reinstalled DOMPurify 3.4.13, the fixed same-major release. The reinstall also reconciled stale root/local-workspace package versions in the lockfile with their existing 0.2.1 manifests.
- Tests added/removed: none; this is a dependency implementation fix and the existing markdown XSS and representative-render tests exercise the sanitization boundary.
- Four-step confirmation: `npm ls` resolved DOMPurify 3.4.13; `npm audit --omit=dev` reported zero vulnerabilities; all 128 web-cockpit tests passed; the original advisory reproduction no longer reports.
- Simplification: none.
- Discrepancies from design: none.
- Adjacent issues parked: none.

## Review record

- Verdict: approve; no blockers, important findings, or nits.
- Correctness/security: the installed version is strictly past the affected `<=3.4.12` range and the package audit is clean.
- Tests: existing sanitizer tests cover dangerous HTML, URL, and event-handler removal and remained green after the upgrade.
- Design/breakage: 3.4.13 is a non-breaking same-major update; the sanitizer API and sole call site compile unchanged. No `IN_PLACE` option or hook configuration exists in Patchbay, so no call-site adaptation was needed.
- Foundation docs: no current assertion changed.
- Reviewer path: bounded inline standalone-story review; no independent, fresh-context, or cross-model reviewer ran.
