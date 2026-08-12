---
id: gate-tests-v0.2.1-security-coverage-gaps
kind: story
stage: done
tags: [security, testing]
parent: null
depends_on: []
release_binding: null
gate_origin: tests
created: 2026-08-12
updated: 2026-08-12
---

# v0.2.1 security-coverage gaps (cross-adapter diagnostic ingest + IPv6 HTTP)

> Parked from the v0.2.1 gate-tests scan (2 medium gaps).

## Severity
Medium

## Gaps
1. **Cross-adapter credential substitution** is tested for observation and delivery subscription, but **not the independent diagnostic ingest RPC** (`server/src/adapter_service.rs:774-812`). Add a two-adapter test submitting `AdapterDiagnosticReport` with the other adapter's evidence/token → assert `UNAUTHENTICATED`, no source/audit append.
2. **Plaintext-HTTP rejection** lacks a non-loopback IPv6 + wildcard case; existing tests cover private IPv4 + DNS names only (`token-commune-adapter/tests/gateway_client.test.ts:40-53`, `resource_contract.test.ts:253-269`). Add `http://[2001:db8::1]` and `http://0.0.0.0` cases at both call sites.

Audit-attribution coverage for actor/endpoint/device is adequate (`server/tests/conformance_vectors.rs:1507-1564`).

## Implementation notes

- Execution capability: inline direct-read test hardening; all gaps were localized to existing security-boundary test tables and one two-adapter RPC fixture.
- Review weight: standard (project default); standalone-story review remained bounded and inline.
- Gap confirmation: repository search found cross-adapter evidence/token substitution coverage for observation ingest and delivery subscription, but no `ReportDiagnostics` call in the two-adapter test. Plaintext URL tables omitted wildcard IPv4 and non-loopback IPv6 at both construction/configuration call sites.
- Files changed: `server/src/adapter_service/tests.rs`, `token-commune-adapter/tests/gateway_client.test.ts`, `token-commune-adapter/tests/resource_contract.test.ts`.
- Tests added: extended `adapter_attachment_evidence_cannot_cross_adapter_identity` with an `AdapterDiagnosticReport` that claims the victim adapter while presenting the attacker adapter's evidence and current token. It asserts `UNAUTHENTICATED` and unchanged Observation/AuditRecord counts.
- URL coverage added: `http://0.0.0.0:8787/` and `http://[2001:db8::1]:8787/` now reject at both the HTTP gateway-client constructor and environment configuration loader.
- Verification: the focused Rust cross-adapter test passed; all 63 token-commune-adapter tests passed; `cargo test --workspace` passed; `cargo clippy --workspace --all-targets -- -D warnings` passed.
- Simplification: extended the existing two-adapter credential-substitution scenario and existing URL tables instead of creating duplicate fixtures.
- Discrepancies from design: none.
- Adjacent issues parked: none.

## Review record

- Verdict: approve; no blockers, important findings, or nits.
- Correctness/security: the diagnostic attempt uses both pieces of the attacker's current attachment evidence while claiming the other attached adapter, so it exercises the independent RPC's authentication boundary rather than only payload-owner validation. Source and audit append absence are checked independently.
- Tests: wildcard IPv4 and documentation-range IPv6 cases execute both public URL-validation call sites while preserving the explicit IPv4/hostname/IPv6 loopback allowlist cases.
- Design/breakage: no production change was needed; `report_diagnostics` already authenticates before report extraction, registration lookup, validation, or append. Weakening production behavior or adding duplicate validation was ruled out because the gap was coverage, not implementation.
- Foundation docs: no assertion changed.
- Reviewer path: bounded inline standalone-story review; no independent, fresh-context, or cross-model reviewer ran.
