---
id: epic-v0-core
kind: epic
stage: done
tags: [protocol, verification, foundation]
parent: epic-v0-1-0-implementation
depends_on: []
release_binding: v0.1.0
gate_origin: null
created: 2026-07-11
updated: 2026-07-14
---

# Epic: Rust coordination core

## Brief

Build the single authoritative coordination core in Rust. This is the root of the v0.1.0 build-out — every other layer depends on it. The core owns the durable event log, operation acceptance with idempotency, authority checks, snapshots, and crash recovery. It is the only writer to the durable log; the web server, CLI, and adapters are clients.

The core implements the protocol semantics defined in `docs/PROTOCOL.md`: the `CommandState` lifecycle (accepted → delivered → ... → terminal), idempotent retry by command id + idempotency key, session connectivity × activity axes, generation monotonicity, snapshot/cursor reconciliation, and the failure vocabulary. It reads and writes through a storage port (Ports & Adapters) so domain semantics remain independent of the backend choice; the first backend may be embedded (file or embedded database).

## Why this is epic-sized

The core contains four feature-sized sub-arcs, each with its own formal-model backing and distinct design surface. `epic-design` decomposes this into child features (each with its own design gate, alternatives evaluation, and formal-model evaluation), not just child stories. This mirrors how `epic-foundation-hardening` decomposed the *design* of these same concerns into separate features — the implementation deserves the same treatment.

## Epic context

- Parent epic: `epic-v0-1-0-implementation`
- Position in epic: root of the critical path. Nothing else starts until the core exists. The protocol seam, Pi adapter, web server, web cockpit, and CLI all depend on it.

## Formal-model backing

The 8 promoted (genuinely checked) properties directly constrain core internals:

- `command_lifecycle.qnt`: `TerminalFinality`, `BoundaryDedup`, `NoAcceptedToCompleted` — terminal transition, boundary deduplication, no accepted→completed adjacency
- `session_generation.qnt`: `GenerationMonotonic` — session generation never decreases
- `csrf_browser.qnt`: `CsrfRejectsMissingProof`, `CsrfRejectsUnauthenticated`, `RevokedSessionCannotCommand`, `browser_local_state_not_authority` — browser-session/CSRF boundary

The 39 stated-normative properties (demoted during verification-claim-correction) are obligations the core must eventually satisfy but that do not yet have checked formulas; they inform the design without carrying checked-model status.

## Decomposition

Split by capability, not by layer. The four sub-arcs are independent enough to parallelize after the shared persistence feature lands: acceptance, authority, and sessions interact through Ports & Adapters (grant-check port, target-identity port) rather than direct coupling, so they can proceed simultaneously once the event log and storage port exist.

### Child features

- `feature-v0-core-persistence` — durable event log, storage port, embedded backend, snapshots, crash recovery — depends on: `[]`
- `feature-v0-core-acceptance` — CommandState machine, operation submission, idempotency, retry, terminal races, observation/elicitation ingestion — depends on: `[feature-v0-core-persistence]`
- `feature-v0-core-authority` — grants, revocation, spawn authority, descendant grants, audit — depends on: `[feature-v0-core-persistence]`
- `feature-v0-core-sessions` — session registry, connectivity × activity axes, generation monotonicity, replacement, stale/offline/unknown — depends on: `[feature-v0-core-persistence]`

### Decomposition risks

- **Persistence is the root and the riskiest feature.** Acceptance, authority, and sessions all read and write through the storage port and depend on the event log. Backend choice affects crash recovery correctness and the qualitative responsiveness floor. The persistence feature's port interface must be designed (not necessarily fully implemented) before the other three can proceed in parallel.
- **Cross-cutting formal properties.** `BoundaryDedup` spans persistence (the `appliedKeys` set and `lsn` live in the event log) and acceptance (the dedup boundary is enforced at acceptance). The persistence feature owns the state; the acceptance feature owns the enforcement. `NoAcceptedToCompleted` touches acceptance + persistence (the transition guard reads command state that persistence durably records). Each child feature's `feature-design` pass should clarify which properties it owns vs. which are integration properties verified at the epic boundary.
- **Authority has the weakest formal backing.** All `authority.qnt` properties are stated-normative (draft). The one promoted property touching authority (`RevokedSessionCannotCommand`) lives in `csrf_browser.qnt` and models the browser/CSRF boundary — it is web-server-facing, not core-internal. The authority feature's obligations are real but not yet checked; the v1 formal gate owns the real authority properties.
- **CSRF/browser properties are out of scope for this epic.** The four `csrf_browser.qnt` promoted properties (`CsrfRejectsMissingProof`, `CsrfRejectsUnauthenticated`, `RevokedSessionCannotCommand`, `browser_local_state_not_authority`) are web-server-facing and belong to `feature-v0-web-server`, not this epic.
- **Elicitation scope.** Elicitation lifecycle handling folds into the acceptance feature as part of the operation/observation/elicitations plane. If the scope is too large, `feature-design` may spawn a child story for elicitation specifically.

## Foundation references

- `docs/PROTOCOL.md` — Command lifecycle state, OperationKind registry, acceptance semantics, idempotency and retry, snapshots and streams, persistence and recovery, authority grants
- `docs/ARCHITECTURE.md` — v0.1.0 component slice, process topology, persistence topology
- `docs/SECURITY.md` — threat model, grants, revocation, audit
- `docs/VERIFICATION.md` — property-graded assurance, promoted vs stated-normative tiers
- `contracts/proto/patchbay/*.proto` — generated contract source (operations, sessions, observations, authority, elicitations, common, adapter)
- `contracts/rust/` — generated Rust bindings (the starting contract for the core's types)
- Formal models in `contracts/` — `command_lifecycle.qnt`, `session_generation.qnt`, `csrf_browser.qnt`, `elicitation_lifecycle.qnt`, `authority.qnt`

## Epic review (2026-07-14)

**Verdict**: Approve — advance to `stage: done`.

All four child features are at `stage: done`:
- `feature-v0-core-persistence` — done
- `feature-v0-core-acceptance` — done
- `feature-v0-core-authority` — done (this session: implemented, deep-reviewed, 3 findings closed via convergence loop)
- `feature-v0-core-sessions` — done (B1-B5 fix arc closed earlier; spawn_origin prerequisite re-reviewed)

### Aggregate alignment (epic-lens, not per-line)
- **Decomposition matches brief**: four capability-split features (persistence → acceptance/authority/sessions in parallel), each with its own design gate and formal-model evaluation. The Ports & Adapters seams the decomposition promised are realized: `authority::AuthorityRegistry` impls `acceptance::GrantCheck`; `session::SessionRegistry` impls `acceptance::TargetResolver`; all three siblings depend on the `storage::Storage` port, not on rusqlite. No adapter leak into the domain (rusqlite confined to `storage/rusqlite.rs` + tests).
- **Cross-cutting formal properties** (the epic flagged these as "verified at the epic boundary"): confirmed covered at the feature boundaries with mutation evidence — `BoundaryDedup` + `NoAcceptedToCompleted` in `acceptance_proptest.rs` (each with a `_catches_injected_*` mutation test); `GenerationMonotonic` in `sessions_proptest.rs` (`generation_monotonic_catches_injected_decrease`). The CSRF/browser properties are correctly out of scope (belong to `feature-v0-web-server`).
- **Capability completeness**: the v0.1.0 core delivers the durable event log, operation acceptance with idempotency/retry/terminal-races, authority (deny-by-default grants, verified issuer, non-cascade revocation, descendant-grant-on-spawn), and sessions (identity, state axes, generation monotonicity, replacement). The epic's "root of the critical path" position is met — the protocol seam, Pi adapter, web server, and CLI can now build against it.
- **Authority's weakest-formal-backing caveat** (epic decomposition risk): honored. All `authority.qnt` properties are stated-normative; the authority feature ships 7 property oracles + 2 mutation tests + 1 documented gap (#8), honestly scoped as component-complete-not-live. The v1 formal gate owns the real authority properties.

### Verification
189 tests green across `patchbay-core` (persistence + acceptance + authority + sessions + storage), clippy clean (`-D warnings`), `cargo fmt --all --check` clean. (Mid-review hit a transient `/tmp` disk-full from accumulated SQLite tempfiles — environment, not code; cleaned and re-verified green.)

### Notes
- The authority feature's deep-review convergence loop (2 blockers + 1 verification gap found, fixed, re-reviewed, one incomplete-fix caught and closed) is the strongest evidence in this epic that the two-phase deep-review discipline works — it caught a real incomplete fix that green tests blessed.
- Backlog items track the live-path follow-on (authority durable acceptance metadata, live composition, payload-actor trust, grant-selection determinism, ingest pre-append conflict check, replay gap detection, fleet target resolution, expiry enforcement, elicitation responder authority, failed-auth audit). None block v0.1.0 component-complete; several become blocking when the live path lands.
