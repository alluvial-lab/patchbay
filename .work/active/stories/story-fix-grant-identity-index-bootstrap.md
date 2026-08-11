---
id: story-fix-grant-identity-index-bootstrap
kind: story
stage: drafting
tags: [bug, storage, authority, ci]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-11
updated: 2026-08-11
---

# Bug: core binary refuses to boot — grant identity index rejects the bootstrap grant

## Symptom (CI)
`typescript-suites` → **"Test CLI"** fails (run `31540050502`, job `93941385761`). The CLI test spawns the core binary, which exits before it can listen:

```
Error: core exited before listening (1): Error: CorruptRecord("grant identity index has extra row default/bootstrap-operator-operator-exact-resource")
```

This is the **last remaining red CI job**. `rust` and `contracts-and-conformance` are green. It only surfaced now because `typescript-suites` `needs: [contracts-and-conformance, rust]`, both of which were red for weeks — so this job hadn't run since well before this session.

## Repro
- Locally: the core binary fails to start against the bootstrap/seed database. The CLI's core-smoke harness (`cli` → spawns core → waits for "listening") catches the early exit.
- CI: `typescript-suites` job, "Test CLI" step. (web-server, web-server-against-core, and web-cockpit tests all pass now; the failure is specifically the core binary booting with bootstrap grant data.)

## Root cause (located)
`core/src/storage/rusqlite.rs:468` — a recovery/startup consistency check iterates the **grant identity index** (a derived projection of `(authority_domain_id, grant_id, source_lsn)`) and, for each row, looks it up in an `expected` map computed from the authoritative event log. A row present in the index but absent from `expected` is treated as `CorruptRecord` ("grant identity index has extra row").

Concretely, the bootstrap flow produces a grant identity index row for `default / bootstrap-operator-operator-exact-resource` that the recovery's `expected`-set does not include → the index/log invariant is violated on a clean boot.

## Likely source
This session's authority/grant arc — `authority-writer-correctness`, `authority-descendant-grant-completion`, `authority-grant-selection-determinism`, and the `authority-provenance-hardening` split — tightened the grant identity index ↔ event-log invariant. The bootstrap path (and/or the `expected`-set computation in recovery) was not reconciled to the new invariant, so a freshly-booted core rejects its own bootstrap grant. (The `rust` job's `cargo test --workspace` did not catch this because the bootstrap-against-seed-DB path is exercised by the TS smoke harness, not a Rust unit/integration test.)

## Fix direction (for the `fix` lane)
Determine which side is authoritative and reconcile:
1. **Is the bootstrap wrongly populating the index?** The bootstrap may write a grant identity index row that the event log doesn't back (index write not preceded/covered by the corresponding grant event).
2. **Or is the `expected`-set computation missing the bootstrap grant?** Recovery may not be deriving the bootstrap grant as `expected` (e.g. it's seeded outside the event log, or the grant-kind/scope it uses isn't recognized by the new invariant).
3. Add a Rust-level regression test that boots core against the bootstrap/seed DB and asserts it reaches "listening" — so `cargo test --workspace` catches this class of failure, not just the TS smoke harness.

The honest invariant to assert and test: **on a clean boot, every grant identity index row is backed by an `expected` source derived from the authoritative log (including the bootstrap grant).**

## Why filed, not patched inline
This is a real storage/authority invariant regression (not CI config — the CI-config layers are all fixed). It deserves a proper diagnose → root-cause → minimal-fix → verify pass via the `fix` lane rather than a hurried patch to the consistency check.

## Blocks
Green CI. `typescript-suites` is the only failing job; resolving this makes CI fully green.
