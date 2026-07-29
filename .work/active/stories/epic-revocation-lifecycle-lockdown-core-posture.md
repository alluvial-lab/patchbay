---
id: epic-revocation-lifecycle-lockdown-core-posture
kind: story
stage: done
tags: [security, protocol, verification]
parent: epic-revocation-lifecycle-lockdown
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-29
updated: 2026-07-30
---

# Build the durable lockdown posture and acceptance fence

## Checkpoint

Land Units 1–2 from the parent design: generated lockdown/snapshot/RPC wire
shapes, the replayed authority-domain `SecurityPostureProjection`, the
domain-owned `OperationPosture` acceptance port, durable session stale clamp,
and operator-session generation invalidation. Add the independent Quint model,
guard-removal mutations, and traced vectors before downstream RPC surfaces rely
on the safety claim.

The parent feature is authoritative for exact proto fields, Rust signatures,
rejection ordering, retry behavior, reason-code redaction, and the distinction
between Operations and snapshot/subscription reads.

## Acceptance evidence

- Active posture rejects every generated committed `OperationKind`, including
  exact retry, as pre-acceptance `AuthorizationDenied` with
  `security_lockdown_active`; no command event is created.
- Replay after restart restores active posture, every current runtime session as
  stale at the entry LSN, and the operator-session invalidated-through floor.
- Session reports while active cannot restore live connectivity; exit clears the
  clamp but does not fabricate a live state.
- Already-accepted command transitions remain legal and terminal-final.
- Quint properties use independent attempted evidence; each acceptance,
  replay, stale, generation, and bootstrap-channel guard-removal mutation fails.
- Generated Rust/TypeScript, vector, model, and drift checks pass without
  hand-edited generated output.

## Ordering constraints

This is the foundation checkpoint. Trigger/exit RPCs, cockpit, and CLI all
consume its generated contract and replay semantics. Keep the lockdown event
keyed by authority domain and fold it through every exhaustive stored-event
consumer before advancing this story.

## Implementation notes

- Added the schema-owned `SecurityLockdownEvent` family, bootstrap channel enum,
  security snapshot summaries, ControlService entry/read RPCs, AdminService exit
  RPC, and `SessionSnapshot.lockdown`; Rust and TypeScript bindings were regenerated
  with `buf generate`.
- Added event-log replay for `SecurityPostureProjection`, an acceptance-owned
  `OperationPosture` port, stale session clamp, and operator-session generation-floor
  replay. All exhaustive `StoredEventKind` consumers fail closed on the new event.
- Added the independent Quint seed model and four draft executable vectors. The
  property tier remains stated-normative/checked-model as appropriate; no stronger
  checked-normative claim is made.

## Verification

- `cargo test --workspace` passed.
- `cd contracts/ts && npm run check:vectors` passed; `npm run check:models` passed
  after its generated `docs/VERIFICATION.md` traceability refresh.
