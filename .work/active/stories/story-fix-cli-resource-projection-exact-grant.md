---
id: story-fix-cli-resource-projection-exact-grant
kind: story
stage: done
tags: [bug]
parent: null
depends_on: []
release_binding: v0.2.0
gate_origin: null
created: 2026-08-08
updated: 2026-08-08
---

# Let exact-resource operators read the CLI resource projection

## Symptom

`resource-query` and `resource-inspect` always call `LoadSecuritySnapshot` even when event replay is disabled. The real handler authorizes that inventory read as `query` against authority-domain scope, so an operator holding only an exact resource `query` grant is denied before the CLI can project the resource snapshot.

## Root cause

The cockpit-panel design added `LoadSecuritySnapshot` solely for CLI-local grant filtering as defense in depth. That duplicated the core snapshot authorization boundary and accidentally widened the authority prerequisite from the requested resource to the whole authority domain. The pass-1 regression used a fake client that returned the security inventory despite the exact-only fixture, so it never exercised the real handler's scope check. Investigation also found that the current resource `LoadSnapshot` implementation materializes the complete projection rather than applying the intended per-resource server-side grant filter, so merely dropping the CLI-local inventory call would expose unauthorized siblings.

## Fix approach

Make `LoadSnapshot(RESOURCE)` enforce the existing `query` grant matcher independently for every resource and return only authorized records/view revisions. Then remove `LoadSecuritySnapshot` and local grant filtering from the CLI projection: the CLI consumes the already-scoped authoritative snapshot and does not require authority-domain inventory authority. Keep `--replay-events` unchanged; subscription replay still explicitly requires authority-domain query authority.

## Regression test

`cli/tests/real-core-resource-projection.mjs` starts the real Rust server, bootstraps an operator, reports one pool plus one draw resource through the real adapter service, replaces the bootstrap fixture grant with only an exact pool `query` grant, restarts/replays the core, proves `LoadSecuritySnapshot` is denied, and then proves the real CLI `resource-query --json` succeeds while the unauthorized draw remains absent.

## Implementation notes

- **Execution capability:** direct host-context repair on `openai-codex/gpt-5.6-sol` at high reasoning. The write set is one narrow CLI/server authority path plus its real-process regression; a separate implementation worker would add handoff risk without useful isolation.
- **Files changed:** `server/src/service.rs`, `server/tests/conformance_vectors.rs`, `cli/src/commands/token-commune-projection.ts`, `cli/src/commands/resources.ts`, `cli/src/main.ts`, `cli/package.json`, `cli/tests/resource-projection.test.ts`, `cli/tests/real-core-resource-projection.mjs`, and the token-commune observer substrate records.
- **Regression capture:** before the fix, the real-process test failed with `[permission_denied] security snapshot is not authorized` and CLI exit `1`, reproducing the reported boundary rather than simulating it.
- **Chosen repair:** remove the security inventory from the resource projection and filter `LoadSnapshot(RESOURCE)` through the existing canonical `GrantCheck::check_at` for each exact resource using one decision-time sample. Remove view revisions for collections with no visible resource. `--replay-events` retains its separately documented authority-domain subscription prerequisite.
- **Focused confirmation:** the real-process regression now proves security-snapshot denial, projection success, and absence of the ungranted sibling draw. The core conformance snapshot fixture now carries the exact resource `query` grant required by the real boundary.
- **Full confirmation:** `cargo test --workspace` 347/347; Pi adapter 25/25; token-commune adapter 60/60; web cockpit 117/117; operator domain 9/9; CLI 46/46 plus the real-core process regression; vectors twice stable (52 vectors, 15 promoted, 20 implementation checks, 37 mutation kills, 103 proto references); models 8 checked-model/0 checked-normative/60 stated-normative; presentation 5 registries plus axe-core; generated drift clean; `git diff --check` clean.
- **Original reproduction:** gone against the real server. Exact-resource authority no longer implies authority-domain inventory authority, and the unauthorized draw never reaches the CLI.
- **Adjacent issues parked:** none. Session-snapshot filtering is outside this reported resource-only boundary and was not changed.

## Review

- **Mode:** standalone-story bounded inline review; no independent, fresh-context, or cross-model reviewer ran, as required for this lane.
- **Verdict:** approve; no blockers, important findings, or nits.
- **Authority boundary:** exact, adapter, fleet, and authority-domain resource visibility all reuse the canonical grant matcher; unauthorized resources and orphaned view metadata are omitted before wire encoding. The CLI performs no second authority decision and cannot require the broader security inventory accidentally.
- **Regression quality:** the test uses the real Rust binary, real bootstrap/login, real adapter attach/report ingress, durable restart/replay, real `LoadSecuritySnapshot` denial, real `LoadSnapshot` filtering, and the actual CLI process. It would fail both the original unconditional security call and a drop-only fix that leaked the ungranted draw.
- **Compatibility/invariants:** optional event replay retains its explicit authority-domain prerequisite; delivery-rejected→`REJECTED`, declared-share validation, redaction, PARTIAL/stale honesty, and generated contracts are untouched and covered by the full green gates.
- **Closure reason:** the reported symptom is reproduced before the fix, the minimal production path is corrected at its actual authority boundary, real-process evidence passes, and the complete required verification matrix is green.
