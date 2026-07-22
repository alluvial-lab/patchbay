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

A real transport-principal model where each control surface (web-server, CLI) is a distinct, core-verifiable principal with its own endpoint/device/generation, plus a bootstrap RPC that creates the first operator + authority grant via the local-console channel, and a shared operator record (password hash + actor id) that both the web-server and the CLI verify against. The core's `IssuerContext` becomes a real verifier that distinguishes principals and rejects unverified identity, not a metadata-passthrough.

The good news from grounding: the core already has a validated, durable `ingest_grant` function (`core/src/authority/ingest.rs:20`) that validates + appends + projects a grant. It is simply not exposed via a control-service RPC. So the bootstrap unit is largely "expose `ingest_grant` via a bootstrap RPC, gated by the local-console/setup-secret channel" — not a from-scratch grant engine. Similarly, `MetadataIssuerContext::from_request` (`server/src/issuer.rs:21`) already extracts operator-id + operator-session-id from metadata; it just doesn't *verify* them or distinguish principals. The verifier unit is "make that extraction a real verification + add principal identity," not a from-scratch auth system.

## Design decisions (to resolve — semantic 50/50s, operator input required)

Two security-bearing choices the foundation docs do not pin. Per the harness rule, these surface as questions rather than being resolved with judgment.

### D1 — Operator-record sharing mechanism

The bootstrap creates an operator record (actor id + `scrypt$<salt>$<hash>` password hash). The web-server must verify operator passwords against it; the CLI must verify against the same record (option-1 auth). How is the record shared?

1. **Core as source-of-truth, read via RPC.** The operator record lives in the core's durable store; the web-server and CLI call a read RPC at login to verify the password (the core does the scrypt check, or returns the hash for the surface to check). Single source of truth; no shared file; the core owns all operator state. Cost: a new read RPC + the core doing (or serving) password verification.
2. **Shared config artifact (file).** Bootstrap writes the operator record to a known file path (e.g. `~/.patchbay/operator.json`, 0600) that both the web-server and CLI read. Simple; no new RPC; matches the current env-only posture generalized to a file. Cost: a shared file is a new trust artifact; file-permission hygiene is load-bearing; the record can drift between the file and the core's grant.
3. **Hybrid:** the core owns the durable operator record (the grant subject); the password hash is a shared config artifact the surfaces read. Splits "who the operator is" (core) from "how they authenticate" (shared file).

**This is a semantic 50/50** (it affects the security model: who is authority for the operator record, and whether password verification is centralized or distributed). The docs commit the *requirement* (the operator record is verified) but not the mechanism.

### D2 — Bootstrap channel shape

The bootstrap RPC (create first operator + grant) must be reachable only via the local-console channel (SECURITY:77-78, 208 — distinct from routine web login, load-bearing for lockdown exit). What shape?

1. **Dedicated local-console RPC, local-listener only.** The core binds a separate local-only listener (loopback/unix socket) for bootstrap; the setup secret is presented there. Routinely unreachable from the network. Strongest channel separation.
2. **Setup-secret-gated RPC on the existing listener.** The existing `ControlService` (or a new admin service on the same listener) gains a bootstrap RPC gated by a one-time setup secret; the secret expires after use/timeout (SECURITY:78). Simpler (no second listener); the channel distinction is the setup-secret lifecycle, not a separate socket. The docs say "CLI, local console output, or a one-time bootstrap secret" (SECURITY:122) — a setup-secret-gated RPC is a defensible reading; a separate local listener is the stronger reading.

**This is a semantic 50/50** (it affects the lockdown-exit channel's strength: a separate local listener is a stronger channel distinction than a secret-gated RPC on the network listener). SECURITY:208 warns that "if a future deployment ever makes bootstrap trust equivalent to routine web login (same factor, same remote channel), lockdown would provide no protection" — so the channel distinction's *strength* is load-bearing.

## Implementation Units

(Detailed once D1/D2 are resolved. Likely units:)

### Unit 1: Bootstrap + grant-admin RPC + setup-secret lifecycle
Expose the core's existing `ingest_grant` via a bootstrap RPC gated by the local-console channel (per D2). Creates the first operator + authority grant. The setup secret expires after use or timeout (SECURITY:78). Establishes the operator record (per D1).

### Unit 2: Real transport-principal verifier
`server/src/issuer.rs`'s `MetadataIssuerContext` becomes a real verifier: each control surface presents a distinct, verifiable principal identity (endpoint/device/generation); the core distinguishes the web-server principal from the CLI principal; self-asserted operator identity is rejected (bound to verified principal evidence). The deferred "real operator-session and transport-principal verifier" from the protocol-seam decision lands here. Must be genuinely enforced (testable, not asserted).

### Unit 3: Operator-record sharing + web-server/CLI consumption
The operator record (per D1) flows to the web-server and CLI; both verify operator passwords against it. The web-server's env-only posture (`PATCHBAY_OPERATOR_ID`/`PATCHBAY_OPERATOR_PASSWORD_HASH`) is replaced or backed by the shared record.

### Unit 4: CLI-principal enrollment (may fold into feature-v0-cli)
The CLI enrolls as a transport principal (its own endpoint/device/generation) via the bootstrap/session channel, establishing the credential store `feature-v0-cli`'s option-1 auth posture requires. This may live in the CLI feature itself once Units 1-3 land; the boundary is decided after D1/D2.

## Risks

- This is a security-bearing cross-cutting change. The compound-issuer verification must be genuinely enforced (testable, not asserted — the standard the cockpit/component-layer arcs set). A verifier that accepts any input is not a verifier; the current `MetadataIssuerContext` accepts any non-empty operator-id, which is exactly the failure mode to eliminate.
- The bootstrap channel must remain distinct from routine web login (SECURITY:208 — load-bearing for lockdown exit). D2's strength determines whether lockdown-exit is real.
- Crosses four packages; the write scope is unusual for a single feature. Will likely warrant child stories per package boundary (contracts/core/server/web-server).
- The web-server's current env-only operator record is a working (if unergonomic) posture; replacing it must not regress the web-server's 4 csrf_browser.qnt properties (those are load-bearing, done, and tested).

## Implementation discovery (origin)

This feature was scoped in direct response to `feature-v0-cli`'s implementation-discovery blocker (commit `4da38dd`, 2026-07-21). The CLI worker correctly stopped when it found the resolved option-1 auth posture (CLI as a full transport principal with its own operator-session bootstrap) could not be realized against the shipped core boundary. The verified findings are reproduced in `## Grounding` above. Rather than weaken the CLI to the rejected option 2, the operator chose to scope this prerequisite feature (option 1 at the epic level): build the real trust boundary the docs promise, then build the CLI against it.
