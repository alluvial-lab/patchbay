---
id: completion-promotion-driver-review-2026-08-15
kind: story
stage: done
tags: [review, spawn]
parent: research-handoff-spawn-completion-promotion-driver
created: 2026-08-15
updated: 2026-08-15
---

# Thorough review — Unit 7 atomic promotion completion driver

## Verdict

**CLEAN** — advance `research-handoff-spawn-completion-promotion-driver` to `done`.

Commit `f48cfea` cleanly establishes the managed promotion driver as the sole completion/descendant-authority owner while retaining a command-scoped, one-way legacy repair lane. No authority bypass, duplicate grant or terminal, legacy misclassification, partial publication, head-of-line poisoning, vacuous oracle, replay divergence, or nit survived this pass.

Review mode: independent fresh-context story review, effective weight `thorough`, implementation range `ecf6ddc..f48cfea`.

## Findings

No material findings or nits.

### Checklist disposition

- **Sole owner — PASS.** `SpawnDescendantTail::managed_promotion_action` returns only `CommitPromotion` after consulting the folded authority registry (`core/src/authority/spawn_tail.rs:240-275`). Every accepted fresh or continuation `SpawnClaim` is marked managed per exact domain/command and has any same-command Operation-shaped compatibility progress removed (`core/src/authority/spawn_tail.rs:546-592`). Managed completion audits before promotion, separate descendant Grants, and generic completed transitions fail closed (`core/src/authority/spawn_tail.rs:864-879,952-960,1032-1040`). The server drives the resulting action through the dedicated atomic writer (`server/src/spawn_completion.rs:148-198,281-356`); successful spawn Results remain evidence-only at generic ingestion (`core/src/acceptance/observation.rs:201-237`).
- **Evidence gating — PASS.** The producer requires a managed accepted claim, delivered/running lifecycle recorded before the Result, one successful exact-target Result, and one exact staged successor before constructing a candidate (`core/src/session/runtime_evidence.rs:99-393`). The ordered fold consumes the shared envelope/result-order validators and checks the exact active/poisoned claim, reserved external-runtime owner, and immediate `∅→1` or `N→N+1` pre-state before staged clone publication in authority → session → claim → command order (`core/src/session/runtime_evidence.rs:445-665`). The driver consults these derived facts; it does not recreate the Leaf-6/Unit-4 transition validators.
- **Legacy one-way migration — PASS.** Ownership is keyed by `(AuthorityDomainId, CommandId)`, not by domain-global presence of any claim. Managed claims remove only their own legacy progress, while unrelated evidence-only, audit-only, audit+grant, completed, and migrated duplicate-descendant prefixes remain eligible for suffix-only repair. Same-command mixed managed/legacy writes fail rather than reacquire the compatibility reactor. The command-scope mutant and legacy-Operation suppression mutant were both killed.
- **Suppressed promotion — PASS.** Expired or revoked accepted provenance maps to `None`, never to a storage call or descendant publication. The driver records that exact command in `suppressed_promotions`, loops, and asks `next_spawn_promotion_excluding` to omit it while sorting all remaining candidates (`server/src/spawn_completion.rs:148-198`; `core/src/session/runtime_evidence.rs:89-99,272-276`). Thus one fenced claim cannot block another ready claim. The staged claim remains unpromoted for explicit reconciliation. Both unsafe-publication and exclusion-removal mutants were killed; the production revocation test observed zero promotion, audit, and descendant authority.
- **Crash/replay and ordering — PASS.** SQLite stamps the promotion id, immediate successor audit id, and nested Grant audit link; validates the candidate against the complete transactional prefix; reserves the Grant identity; and inserts source plus audit in one transaction (`core/src/storage/rusqlite.rs:3852-4001`). The injected crash-before prefix contains neither fact; lost acknowledgement after commit contains the complete pair and restart replays exactly one promotion. Result-first and report-first production orders each stay non-terminal after the first fact and converge to one promotion.

## Mutation matrix

Each mutant was applied alone on the main tree, run with one focused oracle, reverted with `git restore`, and followed by a clean `git status --short`. All seven were killed.

| Mutant | Focused oracle | Result |
|---|---|---|
| Worker 1: replace missing-staged-successor deferral with a default staged candidate | `managed_evidence_retries_complete_once_and_restart_as_a_replayable_prefix` | **KILLED**, exit 101 — bootstrap rejected `staged reference has no envelope` instead of accepting premature promotion |
| Worker 2: hide all legacy `Operation` events from the compatibility tail | `crash_prefixes_repair_to_one_audit_grant_and_terminal_transition` | **KILLED**, exit 101 — repair counts remained `(0,0,0)` instead of exactly one audit/Grant/terminal |
| Worker 3: convert suppressed expired/revoked authority into `CommitPromotion` | `managed_completion_decision_suppresses_expired_or_revoked_exact_prior_authority` | **KILLED**, exit 101 — exact suppression assertion observed `CommitPromotion` |
| Worker 4: disable the successful-spawn deferred Result branch and admit it to generic completion | `managed_evidence_retries_complete_once_and_restart_as_a_replayable_prefix` | **KILLED**, exit 101 — generic route failed before the required staged successor path |
| Fresh: clear every legacy command's progress when observing one managed claim | `managed_claim_does_not_hide_an_unrelated_legacy_repair_prefix` | **KILLED**, exit 101 — unrelated legacy repair disappeared |
| Fresh: force promotion authority readiness to `Ready`, allowing revoked provenance to publish N+1 | `managed_driver_suppresses_promotion_after_accepted_authority_revocation` | **KILLED**, exit 101 — oracle observed `(1,1,1)` instead of no promotion/audit/Grant |
| Fresh: ignore `excluded_commands` in the promotion producer | `promotion_producer_can_skip_a_permanently_suppressed_claim` | **KILLED**, exit 101 — suppressed candidate remained selected, exposing the head-of-line regression |

Clean focused baselines also passed the authority-tail suite, promotion-time continuation liveness, atomic crash-prefix replay, all legacy crash-prefix repairs, and both live Result/report orders.

## Full clean-tree suite

1. `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`: **PASS** — including 14 authority-tail, 34 runtime-evidence/promotion, 39 spawn-claim, 82 server-unit, and 12 server spawn-completion tests; doctests and warnings-denied clippy passed.
2. `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`: **PASS** — 55 vectors, 17 promoted vectors, 22 implementation checks, 38 killed registered mutation witnesses, and 54 model-promotion blocks; generated paths remained clean.
3. `cd operator-domain && npm run build && npm test`: **PASS** — 23/23.
4. `cd pi-adapter && npm test`: **PASS** — 38/38, including the real core/adapter generation-bump, reconnect, and core-restart e2e.

The tracked tree was clean before mutation work, after every `git restore`, before the full suite, and before this review file was written. `git diff --check` passed. Disk discipline was observed without a temporary worktree; `/` retained 60G free.

## Recommendation

**Advance to `done`.** The atomic promotion source is the only managed path that can publish descendant authority, N+1, claim consumption, and command completion; promotion-time liveness, command-scoped legacy normalization, suppressed-claim isolation, crash idempotence, and both evidence orders are mutation-sensitive and green.
