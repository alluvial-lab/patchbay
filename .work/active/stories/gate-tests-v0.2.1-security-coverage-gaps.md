---
id: gate-tests-v0.2.1-security-coverage-gaps
kind: story
stage: drafting
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
