---
id: story-fix-grant-identity-index-bootstrap
kind: story
stage: review
tags: [bug, storage, authority, ci]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-11
updated: 2026-08-11
---

# Bug: resource-projection seed rewrite leaves the grant identity index stale

## Symptom (CI)
`typescript-suites` → **"Test CLI"** fails (run `31540050502`, job `93941385761`). The CLI test spawns the core binary, which exits before it can listen:

```
Error: core exited before listening (1): Error: CorruptRecord("grant identity index has extra row default/bootstrap-operator-operator-exact-resource")
```

This is the **last remaining red CI job**. `rust` and `contracts-and-conformance` are green. It only surfaced now because `typescript-suites` `needs: [contracts-and-conformance, rust]`, both of which were red for weeks — so this job hadn't run since well before this session.

## Repro
- Local: `npm --prefix cli test` reproduced the exact startup failure in `cli/tests/real-core-resource-projection.mjs`: `CorruptRecord("grant identity index has extra row default/bootstrap-operator-operator-exact-resource")`.
- CI: `typescript-suites` job, "Test CLI" step. (web-server, web-server-against-core, and web-cockpit tests all pass now; the failure is specifically the real-core resource projection restart.)

## Root cause
`core/src/storage/rusqlite.rs:468` correctly detects a stale derived row: startup computes grant identities from the authoritative event log, while `grant_identities` still names the bootstrap grant at the same source LSN.

The production bootstrap writer is not divergent. It appends the bootstrap Grant event and its `(authority_domain_id, grant_id, source_lsn)` identity row atomically through `append_grant_audited`; a new file-backed Rust restart test confirms that a clean bootstrap store reopens and rebuilds successfully.

The divergence is created by `cli/tests/real-core-resource-projection.mjs`. Its exact-resource authority fixture stops the core and directly rewrites the bootstrap Grant event payload in SQLite from `bootstrap-operator-operator-exact-resource` to `exact-pool-query`, but did not update the `grant_identities` projection. The later startup check therefore rejects the intentionally edited seed database. The event log remains authoritative; loosening or repairing the startup check would hide genuine corruption.

## Fix approach
Update the stopped-core fixture's event rewrite and identity-row rewrite in one SQLite transaction, asserting that each update affects exactly one row. Preserve the strict startup invariant unchanged.

## Regression test
`server/tests/lockdown_recovery.rs::file_backed_bootstrap_grant_survives_storage_reopen` bootstraps through `AdminService`, closes the file-backed store, reopens it, rebuilds the control service, and asserts the bootstrap state and grant are healthy. It passed before the fixture correction, which is diagnostic evidence that production bootstrap was not the broken writer; the pre-fix failing regression remains the real-core CLI test that performs the seed rewrite and restart.

## Preserved invariant
On every startup, each grant identity index row must be backed by the earliest matching Grant or DescendantGrant source derived from the authoritative log. Test fixtures that intentionally replace a grant source must keep that projection consistent; production recovery must not silently normalize divergent durable state.

## Blocks
Green CI. `typescript-suites` is the only failing job; resolving this makes CI fully green.

## Implementation notes

- **Execution capability:** highest (`gpt-5.6-sol`), selected by the caller because this touches a durable authority index and a fail-closed startup invariant. Direct-read implementation was used because the defect was confined to one seed fixture, one storage invariant, and one restart test; no independent reviewer was dispatched for this standalone fix.
- **Files changed:** `cli/tests/real-core-resource-projection.mjs` keeps its intentional Grant event replacement and the corresponding `grant_identities` row in one stopped-core SQLite transaction; `server/tests/lockdown_recovery.rs` adds the file-backed bootstrap/reopen guard.
- **Regression evidence:** before the fix, `npm --prefix cli test` reproduced the exact `CorruptRecord` during the resource-projection restart. The new Rust test passed before the fixture edit, proving the clean production bootstrap writer was already consistent. After the fixture edit, both the focused real-core test and the full CLI suite pass.
- **Four-step confirmation:** (1) `cargo test -p patchbay-core-server --test lockdown_recovery file_backed_bootstrap_grant_survives_storage_reopen -- --exact --nocapture` passes; (2) `cargo test --workspace` passes; (3) `npm --prefix cli test` re-runs the original core-start reproduction and passes all 46 unit cases plus the real-core resource projection; (4) `cargo clippy --workspace --all-targets -- -D warnings` passes, and startup no longer reports the grant-identity extra row while exact-resource query authority and authority-domain denial remain intact.
- **Adjacent issues parked:** none. The strict recovery check remains unchanged; broadening recovery into silent index repair would weaken the intended corruption boundary and was intentionally rejected.
