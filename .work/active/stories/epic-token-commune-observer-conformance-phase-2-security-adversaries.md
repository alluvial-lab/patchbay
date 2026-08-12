---
id: epic-token-commune-observer-conformance-phase-2-security-adversaries
kind: story
stage: done
tags: [adapter, verification, security]
parent: epic-token-commune-observer-conformance
depends_on: [epic-token-commune-observer-conformance-real-core-e2e]
release_binding: v0.2.0
gate_origin: null
created: 2026-08-08
updated: 2026-08-08
---

# Phase 2: source-authentication and gateway-key adversaries

## Checkpoint

Implement and execute
`token-commune-current-generation-source-authenticated` and
`token-commune-gateway-key-redaction`. The server runner attacks attachment
token, adapter generation, exact target ownership, and payload-claimed source.
The adapter runner attacks every key-bearing output path using the shared
real-core fixture and structural payload/diagnostic allowlists.

The source oracle consumes independent attempted-request pre-state; it must not
validate action-recorded identity written by the accepting action. The secret
oracle owns the original high-entropy sentinel and scans raw/bearer/UTF-8,
URL-encoded, base64-like, JSON, and durable-binary representations.

## Primary files

- `contracts/vectors/token-commune-current-generation-source-authenticated.json`
- `contracts/vectors/token-commune-gateway-key-redaction.json`
- `token-commune-adapter/tests/conformance-vectors.test.ts`
- `server/tests/conformance_vectors.rs`
- focused core-client/credential/diagnostic tests
- `docs/SECURITY.md` only if the canonical list needs clarification

## Acceptance evidence

- Only the current exact adapter/token/generation/target request appends; stale,
  cross-owner, and payload-forged requests are inert.
- The member key remains absent from Observations, resource payloads/projections,
  local and forwarded diagnostics, audit/query output, subscriptions, and raw
  durable bytes.
- Removing generation equality, accepting an old token, trusting payload source,
  weakening tuple ownership, and bypassing each redaction boundary kills the
  corresponding mutation witness.
- No token-commune identity or failure value enters a core state/enum registry.

## Ordering constraint

Depends on the real-core E2E fixture. The failure/presentation adversaries run
after this security boundary is green.

## Implementation notes

- Added the token current-generation scenario to the existing Rust server conformance runner. It uses real attachment-token issuance/replacement, current authenticated Observation ingress, exact resource ownership, and an explicit stale-generation ResourceReport through `AdapterControlService`.
- The server evidence proves missing/stale token rejection, `FAILED_PRECONDITION` for generation 1 after generation 2, cross-owner denial, one exact current Observation append, and no Grant/Operation/ResourceState creation from payload-claimed authority.
- The independent attempted-pre-state oracle compares authenticated adapter id, exact generation, token epoch, and target ownership. Four source mutants (generation bypass, prior token, payload trust, local-id-only ownership) are killed without reading accepted action-recorded identity.
- The redaction runner uses the real `0600` credential loader, HTTP Authorization client, response-reflection rejection, local structural diagnostics, and all required sink names. Seven sink-specific leak mutants are rejected by the independent raw/UTF-8/bearer/URL/base64/JSON/hex byte oracle.
- Clarified the canonical no-log list in `docs/SECURITY.md`: token-commune gateway member keys and bearer forms are access tokens and may not enter audit records.
- Verification: focused Rust server conformance passed 1/1; adapter security scenarios reported two exact scenario ids and 11 exact killed mutation ids; the full real-core E2E independently scanned snapshots, subscriptions, diagnostic query output, local diagnostics, and SQLite bytes.
- **Pass-2 correction (2026-08-08, `b0605a9`):** the 4 source-authentication and 7 redaction counts above are superseded. The current vector declarations retain 3 service-boundary source-authentication kills and 4 redaction kills (including the credential-path witness), 7 genuine kills total.
