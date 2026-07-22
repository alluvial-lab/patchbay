---
id: feature-v0-control-surface-trust-boundary
kind: feature
stage: drafting
tags: [security, protocol]
parent: epic-v0-1-0-implementation
depends_on: [feature-v0-protocol-seam, feature-v0-core-authority, feature-v0-web-server]
release_binding: null
gate_origin: null
created: 2026-07-21
updated: 2026-07-21
---

# Feature: v0.1.0 control-surface trust boundary (real transport principals + bootstrap)

## Brief

Build the real control-surface security boundary that `docs/SECURITY.md` commits but that the shipped v0.1.0 components deferred. The protocol-seam feature settled the **compound-issuer** *requirement* (the core independently verifies both the transport principal and the operator identity — SECURITY:143) and shipped the web-server *half* (the web-server verifies the operator session at its boundary and forwards `x-patchbay-operator-id` + `x-patchbay-operator-session-id`), but it explicitly deferred "the real operator-session and transport-principal verifier" (`core/src/authority/issuer.rs`: "v0.1.0 tests supply a test context; the real operator-session and transport-principal verifier lands with the protocol seam and web server"). That deferred work — plus the bootstrap channel that creates the first operator/grant, and the shared operator record — never landed. This feature builds it, so the v0.1.0 control-surface security posture the docs promise is actually real rather than asserted.

This is the forcing-function discovery from `feature-v0-cli`: the CLI cannot be a real transport principal (its resolved auth posture, option 1) against the shipped core, because (a) there is no bootstrap/grant-admin RPC on `ControlService`, (b) the web-server reads the operator record only from env at startup, and (c) the core's `MetadataIssuerContext` hard-codes the endpoint to `patchbay-web-server` and accepts any non-empty operator-id without verification. See `## Implementation discovery (origin)` below for the verified findings.

Scope spans the four packages the boundary crosses: `contracts/` (the bootstrap + principal-identity schema), `core/` (the real transport-principal verifier + grant-admin ingestion), `server/` (the verified-issuer context that distinguishes principals), and the operator-record contract `web-server/` consumes. The CLI (`feature-v0-cli`) depends on this feature; its resolved option-1 auth posture becomes realizable once this lands.

## Epic context

- Parent epic: `epic-v0-1-0-implementation`
- Position in epic: a cross-cutting security-boundary feature that the protocol-seam, core-authority, and web-server features each partially delivered but none completed. It unblocks the CLI (the last v0.1.0 surface) and makes the compound-issuer verification the docs promise genuinely enforced rather than trusted-on-input.

## Foundation references

- `docs/SECURITY.md` — Compound issuer (§143), Enrollment and authentication (first operator via CLI/local-console bootstrap, §77-81; one-time setup secret that expires, §78; lockdown-exit channel distinct from routine web login, §208)
- `docs/PROTOCOL.md` — OperationKind registry, authority grants, audit
- `docs/ARCHITECTURE.md` — v0.1.0 process topology (web server, CLI, core, adapter may run on different machines; the core is the network-reachable fixed point + single durable writer)
- `feature-v0-protocol-seam` (done) — settled the compound-issuer wire-evidence shape (forwarded verified session record evidence); deferred the real verifier
- `feature-v0-cli` (drafting, blocked on this feature) — the forcing function

## Grounding (verified against shipped code, 2026-07-21)

- `ControlService` exposes only `Submit`/`Subscribe`/`LoadSnapshot` (`contracts/proto/patchbay/control.proto`). No bootstrap, operator-session enrollment, setup-secret, or grant-administration RPC.
- The core has an internal `ingest_grant` function (`core/src/authority/ingest.rs:20`) but no control-service method exposes it. Every `Submit` passes through the live-grant check before acceptance — so the first grant cannot be created by submitting an Operation (chicken-and-egg).
- The web-server reads `PATCHBAY_OPERATOR_ID` + `PATCHBAY_OPERATOR_PASSWORD_HASH` at startup into an in-memory `SessionStore` (`web-server/src/main.ts:43-46`, `web-server/src/sessions.ts`). A CLI-created password record would not be consumed.
- No shipped component stores, expires, or consumes the one-time setup secret (SECURITY:78).
- `server/src/issuer.rs:9` hard-codes `WEB_SERVER_ENDPOINT_ID = "patchbay-web-server"`; `MetadataIssuerContext::from_request` accepts any non-empty operator-id/session-id from metadata without verifying them, returns `None` for device + endpoint generation, and stamps the endpoint as the web-server regardless of caller. A direct CLI request cannot be represented as its own full transport principal.

## Architectural choice

(Detailed in the `feature-design` pass — this section is the seed.)

A real transport-principal model where each control surface (web-server, CLI) is a distinct, core-verifiable principal with its own endpoint/device/generation, plus a bootstrap RPC that creates the first operator + authority grant via the local-console channel, and a shared operator record (password hash + actor id) that both the web-server and the CLI verify against. The core's `IssuerContext` becomes a real verifier that distinguishes principals and rejects unverified identity, not a metadata-passthrough.

## Implementation Units

(Detailed in the `feature-design` pass. Likely units, to be refined against the shipped code:)

### Unit 1: Bootstrap + grant-admin RPC + setup-secret lifecycle
The `ControlService` (or a new admin service) gains a bootstrap RPC that creates the first operator + authority grant. It is reachable only via the local-console / setup-secret channel (SECURITY:77-78, 208), not the routine network path. The setup secret expires after use or timeout. The operator record (actor id + `scrypt$<salt>$<hash>` password hash) becomes a shared artifact the web-server consumes (not env-only).

### Unit 2: Real transport-principal verifier
`server/src/issuer.rs`'s `MetadataIssuerContext` becomes a real verifier: each control surface presents a distinct, verifiable principal identity (endpoint/device/generation), the core distinguishes the web-server principal from the CLI principal, and self-asserted operator identity is rejected (or bound to verified principal evidence). The deferred "real operator-session and transport-principal verifier" from the protocol-seam decision lands here.

### Unit 3: Operator-record sharing + web-server consumption
The operator record created by bootstrap flows to the web-server (not env-only). The web-server verifies operator passwords against the shared record. The CLI verifies the same record. (Whether this is a durable store in the core, a shared config artifact, or an RPC the web-server calls at startup is a design decision for the pass.)

### Unit 4: CLI-principal enrollment
The CLI enrolls as a transport principal (its own endpoint/device/generation) via the bootstrap/session channel, establishing the credential store `feature-v0-cli`'s option-1 auth posture requires. (This may live in the CLI feature itself once Units 1-3 land; the boundary is decided in the design pass.)

## Risks

- This is a security-bearing cross-cutting change. The compound-issuer verification must be genuinely enforced (testable, not asserted — the standard the cockpit/component-layer arcs set). A verifier that accepts any input is not a verifier.
- The bootstrap channel must remain distinct from routine web login (SECURITY:208 — load-bearing for lockdown exit). Do not collapse them.
- Crosses four packages; the write scope is unusual for a single feature. May warrant child stories per package boundary.

## Implementation discovery (origin)

This feature was scoped in direct response to `feature-v0-cli`'s implementation-discovery blocker (commit `4da38dd`, 2026-07-21). The CLI worker correctly stopped when it found the resolved option-1 auth posture (CLI as a full transport principal with its own operator-session bootstrap) could not be realized against the shipped core boundary. The verified findings are reproduced in `## Grounding` above. Rather than weaken the CLI to the rejected option 2, the operator chose to scope this prerequisite feature (option 1 at the epic level): build the real trust boundary the docs promise, then build the CLI against it.
