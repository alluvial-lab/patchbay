---
id: authority-descendant-grant-completion-contract-fold
kind: story
stage: done
tags: [security, protocol]
parent: authority-descendant-grant-completion
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-10
updated: 2026-08-10
---

# Complete the descendant-completion contract and durable fold

## Checkpoint
Land the schema and pure-domain half of the parent design. Carry `spawn_origin` on `SessionGenerationBumped`, replace the ephemeral three-fact issuance latch with the durable `SpawnCompletionAction` fold, and make descendant-grant audit provenance required and replay-validated.

This is a design checkpoint inside one feature-owning implementation bundle, not a separate worker assignment. Follow the exact interfaces, matching rules, and file list in the parent feature's Unit 1.

## Acceptance evidence
- Generated Rust/TypeScript bindings preserve existing tags and add only `SessionGenerationBumped.spawn_origin = 11`.
- Registration and generation-bump facts produce the same exact spawned-session target.
- Accepted spawn, successful result, session fact, qualifying completion audit, descendant grant, and terminal events fold in arbitrary delivery order without using an in-memory `issued` truth.
- `next_action()` is deterministic and orders audit → grant → completion for new spawns.
- Descendant grants require the deterministic id, verified actor/optional endpoint, non-empty spawning grant and operation ids, canonical allowed kinds, and a real prior matching same-domain completion audit.
- Invalid/forged/cross-domain audit links and conflicting facts fail before durable descendant append.
- Obsolete tests expecting `audit_id == None` are replaced, not retained as compatibility behavior.

## Ordering constraint
No sibling prerequisite. This checkpoint must finish before `authority-descendant-grant-completion-crash-safe-writer`, which consumes the new generated field and action API.

## Verification

```bash
PATH="$HOME/.npm-global/bin:$PATH" npm --prefix contracts/ts run gen
npm --prefix contracts/ts run build
# After staging/committing the intended generated outputs:
PATH="$HOME/.npm-global/bin:$PATH" npm --prefix contracts/ts run check:drift
cargo test -p patchbay-core --test authority_spawn_tail --test authority_ingest --test authority_registry --test sessions_ingest
```

## Implementation notes
- Execution capability: Sol xhigh (explicit autopilot caller selection for security/provenance and durable-log correctness); direct-read only, with no nested agents or peers per the endpoint constraint.
- Review weight: thorough (explicit caller selection; parent feature stops at review for a fresh pass).
- Files changed: `contracts/proto/patchbay/sessions.proto`; generated Rust/TypeScript session bindings; `core/src/session/ingest.rs`; authority action fold, registry, state, ingestion, and exports; focused authority/session tests.
- Tests added/updated: mutation-sensitive ordered-action, registration/generation-bump equivalence, durable redelivery, legacy repair, competing-terminal, verified-attribution, exact audit-link, forged-link, deterministic-id, replay, and generated-contract coverage. `cargo test -p patchbay-core`, TypeScript contract build, and core Clippy passed.
- Simplification: removed the ephemeral `issued` latch and the old three-fact immediate issuance API; durable audit/grant/terminal facts now determine progress and replay.
- Discrepancies from design: `SessionGenerationBumped.model` already occupied tag 10 in current repo reality, so `spawn_origin` was generated at unique tag 11 exactly as settled; no existing tag was renumbered. Production `AuditedStorage` continues to supply the ordinary descendant-grant creation audit while the required spawn-completion audit remains the grant's explicit `audit_id`.
- Adjacent issues parked: none (generic concurrent authority-writer correctness remains owned by the excluded downstream feature).
