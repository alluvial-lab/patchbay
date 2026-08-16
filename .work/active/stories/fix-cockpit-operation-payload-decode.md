---
id: fix-cockpit-operation-payload-decode
kind: story
stage: review
tags: [verification]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-16
updated: 2026-08-16
---

# Fix: cockpit replay decodes OPERATION events with the wrong schema

## Reproduction (live UAT, 2026-08-16)

Fresh dev stack (core+pi-adapter+web-server), one CLI diagnostic call submits a
`query` command (accepted at LSN 17, transitions at 19/22). Browser cockpit
first login → permanent "Reconnecting — The projection is unreconciled" banner,
retry storm (LoadSnapshot ×312), "+" spawn actions accepted by core but never
rendered. Reproduced headlessly by folding the real operator stream through the
cockpit's own compiled model:

```
THROW at kind=8 lsn=19: command transition references unknown command cli-ead4e7bf-…
```

## Root cause (diagnosed to the line)

`StoredEventKind::OPERATION` (kind 1) events carry a serialized
**`AcceptedOperation`** envelope (command id at `.operation.commandId`, plus
`authorizingGrantId`). `web-cockpit/src/domain/model.ts` `fold()` decodes the
payload as a bare **`Operation`** — protobuf field misalignment yields a
garbage non-empty command key, the real transition then references an
"unknown" command, `replaceFromSnapshots` throws, `reconcile()` never
completes, and the model stays `reconciled: false` forever.

The cockpit's unit tests pass because their fixtures encode kind-1 payloads as
bare `Operation` — stale against the core's actual stored shape.

## Fix

- `fold()` (and any other kind-1 decode site in web-cockpit) decodes
  `AcceptedOperation` and folds `accepted.operation` (the authorizing grant is
  available for presentation if needed). Match the decode used by
  cli/operator-domain/pi-adapter.
- Fix the fixtures/tests to encode `AcceptedOperation`.
- Add a regression that folds a REAL-shaped replay prefix (accepted operation +
  transitions) — the exact UAT shape.

## Acceptance

- [x] Folding a replay prefix containing a kind-1 event + its transitions
      registers the command and applies transitions without throwing.
- [ ] Live-stack retest: cockpit reconciles (banner clears, session list
      renders).
- [x] Full four verification groups + web-cockpit suite green.

## Implementation notes

- Execution capability: `openai-codex/gpt-5.6-sol`; direct-read implementation because the diagnosed wire-shape bug was confined to one fold branch and its fixtures.
- Review weight: `standard` (project default); caller requested the implementation stop at `stage: review` for the autopilot/review boundary.
- Files changed: `web-cockpit/src/domain/model.ts`, `web-cockpit/tests/model.test.ts`, `web-cockpit/tests/reconcile.test.ts`.
- Tests added/removed: added the exact UAT-shaped LSN 17/19/22 replay-prefix regression (accepted query Operation, delivered transition, completed transition); updated every web-cockpit kind-1 fixture to encode `AcceptedOperation` with an authorizing Grant id. No tests removed.
- Simplification: consolidated model-test kind-1 fixture construction in `acceptedOperationEvent`, eliminating repeated stale bare-`Operation` encoding.
- Discrepancies from design: the named CLI comparator currently also decodes kind-1 bytes as bare `Operation` in `cli/src/commands/operations.ts`; this bounded story changes only the web-cockpit decode sites requested by the caller.
- Mutation evidence: reverting the production branch to bare `OperationSchema` decoding made the exact regression fail with `command transition references unknown command cli-ead4e7bf-replay`; `git restore` reinstated the fix and the focused test passed.
- Verification: `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`; `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`; `cd operator-domain && npm run build && npm test`; `cd pi-adapter && npm test`; and `cd web-cockpit && npm test` all passed. Web cockpit: 149/149; operator domain: 32/32; Pi adapter: 128/128.
- Adjacent issues parked: none.
