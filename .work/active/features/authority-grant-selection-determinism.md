---
id: authority-grant-selection-determinism
kind: feature
stage: done
tags: [security, foundation]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-09
updated: 2026-08-10
---

# Authority grant-selection determinism (stable rule + regression)

## Brief
Close the grant-selection determinism gap split out of `authority-provenance-hardening`. Absorbs:

- `backlog-authority-grant-selection-determinism` — **PARTIAL**: overlapping matching grants need a stable selection rule so the returned `grant_id` (and downstream `spawning_grant_id` provenance + revocation policy) is replay-stable. Candidates are now sorted by `grant_id` before selection (`core/src/authority/check.rs:47-58`), giving a stable rule; **but no overlapping-grants before/after-replay regression exists**. *Src:* authority review Phase 1+2.

## Direction
Ratify the selection rule explicitly (most-specific-scope-first / sort-by-`grant_id` / reject-ambiguity — the current sort-by-`grant_id` is the implemented candidate), document it, and add the missing regression: overlapping matching grants return a stable `grant_id` before and after replay rebuild. Latent in v0.1.0 single-operator; becomes real with multiple/narrower grants or delegation.

## Foundation references
- `docs/PROTOCOL.md` — grant lifecycle; provenance
- Code: `core/src/authority/check.rs` (`GrantCheck` selection)

## Scope boundaries

This feature ratifies and protects the existing grant-decision rule; it does not broaden authority or redesign grant matching.

- Applies after boundary validation has produced a verified issuer, canonical `OperationKind`, target scope, authority domain, and one sampled decision time.
- Preserves the existing matching predicate: exact domain/actor, optional endpoint narrowing, kind membership, and target-scope containment.
- Preserves liveness priority across otherwise-matching grants: a live grant authorizes; only when none is live does an expired grant explain denial; only when neither exists does a revoked grant explain denial.
- Defines the tie-break within each liveness class as ascending lexicographic order of the exact `GrantId.value` bytes. The first candidate in the highest available class supplies decision provenance.
- Does not add scope-specific precedence, reject overlapping grants, change revocation behavior, alter idempotency, add a protocol field, or change generated contracts.
- Adds implementation-level replay evidence only. It does not promote an authority model property or conformance vector.
- No UI surface changes; the existing grant/audit presentations consume the selected durable id.

## Design decisions

- **Ratify lexicographically lowest matching `GrantId`, not most-specific scope or ambiguity rejection**: This is the behavior already implemented in `AuthorityRegistry`'s `GrantCheck`, is a total rule across every current scope shape, and makes provenance independent of `HashMap` iteration. A specificity rule would require a new partial-order policy for incomparable adapter, project-group, actor, and endpoint dimensions; ambiguity rejection would turn valid overlapping defense-in-depth grants into an authorization denial.
- **Keep liveness precedence outside the id tie-break**: A lexicographically lower expired or revoked grant must never defeat a live matching grant. The existing class order remains live → expired → revoked, with `GrantId` ordering only inside a class.
- **Make the ordering representation explicit**: Compare the exact opaque string values bytewise/lexicographically with no case folding, locale collation, numeric interpretation, or Unicode normalization. Grant ids are identity, not labels.
- **Test the exact winner as well as live/replay equality**: The regression uses a lexicographically higher, narrower grant inserted before a lexicographically lower, broader grant, proves both match the same request, and requires the lower id from both the live projection and a fresh replay rebuild. This distinguishes the chosen policy from insertion order and most-specific-scope selection; merely comparing two arbitrary results would not.
- **Do not extract a selector framework**: The existing sorted candidate vector and class-priority searches are short and direct. Add a load-bearing contract comment rather than a new public helper, enum, rank registry, or configuration surface.
- **Execution posture**: Direct-read only. The code and replay surface are bounded to the authority registry, grant-check adapter, and two existing test modules; nested subagents and peer mechanisms were explicitly prohibited. Worker capability is `openai-codex/gpt-5.6-sol`, selected by the caller for this security/contract-bearing provenance rule.
- **Review policy**: Effective `review_weight` is `thorough` (source: explicit operator selection). Pass it unchanged to implementation, feature review, and final completion review; reviewer findings remain proposals for receiver adjudication.

## Extension pressure classification

- **Committed v0.1.0**: Among grants that match the verified issuer, canonical kind, and target, decision classes are considered live before expired before revoked; the lexicographically lowest exact grant id inside the highest available class supplies the accepted or denial decision provenance. The selected id is durably retained for accepted Operations.
- **Reserved seams**: Future delegation or multi-operator policy may introduce an explicitly specified lineage/specificity policy through a protocol-change ceremony. Existing authority-domain ids, typed scopes, and opaque grant ids preserve the inputs needed for that future decision without implementing it now.
- **Explicitly rejected for v0.1.0**: Projection iteration order, locale/case/numeric id collation, an implicit “most specific” rank with no total scope relation, and rejecting a request solely because multiple valid grants overlap.
- **Parked-idea pressure test**: Multi-human coordination is not foreclosed because the rule consumes typed domain/actor/endpoint/scope facts rather than assuming one hard-coded operator. Desktop, mesh, and skin ideas are unaffected.

## Architectural choice

### Option A — most-specific scope, then a tie-break

Prefer a runtime-session/resource grant over adapter/project/fleet/domain grants. This sounds least-privilege-oriented, but current scope containment is not one total hierarchy: adapter and project-group scopes can both contain a session along different dimensions, endpoint narrowing is a subject dimension, and future delegation adds lineage. Choosing this option would create a new policy registry and still require another tie-break.

### Option B — reject more than one live match

Fail closed whenever grants overlap. This makes provenance unambiguous, but treats legitimate defense-in-depth grants as invalid configuration, introduces a new authorization-denial mode, and can create avoidable availability failures when grants are added or rotated.

### Option C — liveness class, then lexicographic `GrantId` (chosen)

Retain live → expired → revoked precedence and choose the lowest exact grant id inside the first non-empty class. This is total, deterministic, already implemented, independent of projection iteration and replay reconstruction, and does not pretend opaque ids express privilege. Its cost is that the winner is intentionally mechanical rather than “most specific,” so the rule must be documented.

**Choice**: Option C. It is the least irreversible sound option because it ratifies current behavior without inventing a scope lattice or turning overlap into a new error contract.

## Trickiest unit first

The riskiest unit is the regression fixture, not the sort statement. A test that only compares the live and rebuilt registries can pass while both select the same wrong grant, and a fixture with two identical scopes does not distinguish the chosen rule from a future specificity policy. The test must independently establish that both broad and narrow grants match one request, reverse creation/specificity pressure relative to id order, and assert the exact canonical winner on both sides of replay.

## Implementation units

### Unit 1: Ratify the canonical decision rule

**Files**: `docs/PROTOCOL.md`, `docs/SECURITY.md`, `core/src/authority/check.rs`

```rust
// Existing behavior retained in core/src/authority/check.rs.
let mut candidates: Vec<_> = self
    .grants()
    .filter(|grant| {
        grant_matches_request(grant, &issuer_ref, operation_kind, target_scope)
    })
    .collect();

candidates.sort_unstable_by(|left, right| {
    left.grant_id.value.cmp(&right.grant_id.value)
});

// Evaluate the ordered candidates by class: Live, then Expired, then Revoked.
```

**Implementation notes**:

- Add the normative rule to `docs/PROTOCOL.md` under **Authority grants**, adjacent to the matching and deny-by-default statements. Define matching independently from liveness, the live/expired/revoked class priority, exact `GrantId.value` ordering, and that the chosen id is the durable accepted provenance or typed denial correlation.
- In `docs/SECURITY.md` under **Operation authorization and replay resistance**, reference the canonical protocol rule and state that projection/container iteration order is never authority. Keep `PROTOCOL.md` as the product-semantics source rather than copying an alternate algorithm into Security.
- In `core/src/authority/check.rs`, retain the current implementation and add a concise load-bearing comment naming the class priority and bytewise id tie-break. Do not change `GrantCheck`, `GrantRecord`, generated DTOs, or error vocabulary.
- Preserve the single injected `evaluated_at` sample. Sorting and all liveness decisions occur against the same projection snapshot and timestamp already supplied to `check_at`.

**Acceptance criteria**:

- [ ] Foundation prose names one unambiguous total decision rule and does not imply that a lower id can outrank a live grant from another liveness class.
- [ ] The implementation remains deny-by-default and uses only verified issuer evidence plus the canonical matching predicate.
- [ ] Accepted `Authorized.grant_id`, `AcceptedOperation.authorizing_grant_id`, and downstream `spawning_grant_id` continue to refer to the selected durable grant.
- [ ] No schema, generated artifact, state registry, failure code, or grant-scope rank is added.

### Unit 2: Overlapping-grant live/replay regression

**File**: `core/tests/authority_replay.rs`

```rust
async fn selected_grant_id(
    registry: &AuthorityRegistry,
    issuer: &dyn IssuerContext,
    target: &TargetScope,
) -> GrantId {
    registry
        .check(
            &domain("authority-main"),
            issuer,
            OperationKind::Instruct,
            target,
        )
        .await
        .expect("one of the overlapping live grants must authorize")
        .grant_id
        .expect("authorization must retain grant provenance")
}

#[tokio::test]
async fn overlapping_grants_select_the_same_lowest_id_before_and_after_replay() {
    // Ingest grant-z-adapter first and grant-a-domain second.
    // Both match one adapter-targeted instruct request; the broader grant has
    // the lower id so the fixture distinguishes id ordering from specificity.
}
```

**Implementation notes**:

- Extend the existing in-memory `RusqliteStorage` replay test setup rather than creating a fake replay path. Use normal `ingest_grant` for both records and `rebuild_from_log` for the fresh projection.
- Add a minimal verified `IssuerContext` fixture with the same domain/actor used by both grants. Endpoint narrowing is absent so the test isolates target overlap and id selection.
- Build one authority-domain-scope grant with id `grant-a-domain` and one adapter-scope grant with id `grant-z-adapter`; both allow `Instruct` for the same actor, and ingest the narrower/higher id first.
- Before checking the winner, assert through `grant_matches_request` that each fixture grant independently matches the verified issuer, kind, and request target. This prevents a malformed fixture from turning the test into a one-candidate case.
- Assert the exact id `grant-a-domain` from the live registry and from a newly rebuilt registry, not only equality between the two results. Also assert the rebuilt registry equals the live projection as the existing replay contract requires.
- Keep expiry/revocation tests where they already live; this feature does not duplicate the grant-lifecycle suite merely to enumerate every class.

**Acceptance criteria**:

- [ ] Two genuinely overlapping live grants produce `grant-a-domain` despite reverse ingestion order and the competing grant's narrower scope.
- [ ] A fresh log replay returns the same exact selected id and projection state.
- [ ] Removing id ordering, changing to insertion/specificity ordering, or selecting from a different liveness class makes the focused evidence fail.
- [ ] The test uses production grant ingestion, projection rebuild, and `GrantCheck`; it does not assert a test-only selector.

## Implementation order

1. Unit 1 — write the canonical protocol/security wording and align the load-bearing source comment with the already-running rule.
2. Unit 2 — add the overlapping broad/narrow fixture and assert exact live plus replayed provenance.
3. Run focused authority tests, workspace checks, and the caller-selected `thorough` integrated feature review.

## Child-story decision

No child stories are spawned. The feature is one cohesive ratification slice: one existing decision algorithm, its canonical prose, and one replay regression. There is no useful intermediate checkpoint, separate write-ownership bundle, or heterogeneous acceptance surface.

## Simplification

- Keep the current sorted candidate vector and existing `GrantCheck` port; add no selector service, scope-specificity registry, grant-priority field, configuration option, or compatibility path.
- Reuse `ingest_grant`, `AuthorityRegistry`, `rebuild_from_log`, `grant_matches_request`, `IssuerContext`, and the in-memory SQLite fixture.
- Do not create a conformance vector or formal-model property solely to relabel this implementation regression as stronger assurance. Promotion remains a separately reviewed verification act.
- No valuable existing test is removed. Avoid duplicate one-test-per-liveness-class coverage already owned by grant-lifecycle tests.

## Testing

- **Replay interface regression** (`core/tests/authority_replay.rs`) protects the observable selected `GrantId` before and after durable reconstruction and distinguishes the exact policy from creation order and specificity.
- **Existing grant-check tests** (`core/tests/authority_grant_check.rs`) continue to protect verified issuer, domain, endpoint/kind/target matching, revocation, and deny-by-default behavior.
- **Existing authority properties** (`core/tests/authority_proptest.rs`) remain implementation evidence for grant safety and replay projection equality; this feature makes no formal-promotion claim.
- **Foundation consistency** is checked by reviewing `PROTOCOL.md` as the semantic source and `SECURITY.md` as a reference, with no competing algorithm in generated contracts or other docs.

Implementation verification commands:

```bash
cargo fmt --all -- --check
cargo test -p patchbay-core --test authority_grant_check --test authority_replay
cargo test -p patchbay-core --test authority_proptest
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
node contracts/scripts/check-models.mjs
node contracts/scripts/check-vectors.mjs
git diff --check
```

## Risks

- **The mechanical winner may surprise readers expecting least-specific or most-specific authority.** Mitigation: state explicitly that ids are an opaque total tie-break and that overlap does not imply a privilege ranking.
- **A lower-id expired/revoked record could accidentally defeat a live grant if sorting and liveness precedence are conflated.** Mitigation: document and retain class-first selection, and keep existing liveness tests green.
- **A weak replay test could prove only projection equality, not the policy.** Mitigation: assert both grants match and require one exact id chosen against insertion and specificity pressure.
- **Opaque id comparison could drift across implementations.** Mitigation: define exact bytewise lexicographic `GrantId.value` ordering with no normalization; selection remains core-owned rather than duplicated in clients.
- **Future delegation may need lineage-aware precedence.** Mitigation: delegation remains reserved and must explicitly replace or extend this committed rule through the protocol-change ceremony; do not silently reinterpret “specificity.”

Fallback if implementation mapping shows the current sort has moved is to preserve the same class-first/minimum-id semantics at the new single `GrantCheck` decision point and keep the regression at the public port. Do not fall back to container iteration order, scope ranking, or ambiguity rejection.

## Other agent review

- **Invoked because**: selected grant provenance controls accepted-operation attribution, downstream descendant provenance, and revocation-policy correlation.
- **Skipped/degraded**: the delegated endpoint explicitly prohibited nested subagents and peeragent, so no independent design-time pass ran. This is non-blocking under the risk-driven design policy; the effective `thorough` implementation/feature/final review path remains mandatory.
- **Fixed/active blockers**: none found during direct foundation and code review.
- **Parked**: formal/conformance promotion and any future delegation-specific precedence require separate scope and review.
- **Rejected**: unordered container selection, most-specific ranking, and ambiguity denial for the current contract, for the reasons in Architectural choice.

## Implementation notes

- **Execution capability:** `openai-codex/gpt-5.6-sol`, selected by the explicit autopilot caller for the security/provenance contract. Direct-read implementation only; no nested agent or peer mechanism was used.
- **Review weight:** `thorough` (source: explicit operator selection). The feature stops at `review` for the required fresh reviewer.
- **Files changed:** `docs/PROTOCOL.md`, `docs/SECURITY.md`, `core/src/authority/check.rs`, `core/tests/authority_replay.rs`, and this feature item.
- **Tests added/removed:** added `overlapping_grants_select_the_same_lowest_id_before_and_after_replay`; it proves that broad and narrow live grants independently match, that reverse ingestion and narrower-scope pressure do not change the exact `grant-a-domain` winner, and that production replay plus `GrantCheck` returns the same provenance. No tests were removed.
- **Simplification:** retained the existing sorted candidate vector and live → expired → revoked searches; added only the load-bearing contract comment, canonical prose, and one replay regression. No selector framework, scope rank, schema field, generated artifact, model, vector, or compatibility path was added.
- **Discrepancies from design:** none. The existing implementation already matched the ratified rule.
- **Adjacent issues parked:** none; the workspace-wide rustfmt baseline described below is outside this feature's allowed write set and was not changed.

## Implementation verification

- `cargo test -p patchbay-core --test authority_grant_check --test authority_replay` — passed (8/8).
- `cargo test -p patchbay-core --test authority_proptest` — passed (13/13).
- `cargo test --workspace` — passed.
- `cargo clippy --workspace --all-targets -- -D warnings` — passed.
- `node contracts/scripts/check-models.mjs` — passed.
- `node contracts/scripts/check-vectors.mjs` — passed after installing the repository's locked Node dependencies and building the existing local TypeScript packages; 20 implementation checks executed and 37 mutation witnesses killed. No generated source changed.
- `rustfmt --edition 2021 --check core/src/authority/check.rs core/tests/authority_replay.rs` — passed. `cargo fmt --all -- --check` remains red on 70 pre-existing, out-of-scope Rust files; neither feature-owned Rust file appears in that diff, and the scope boundary forbids formatting unrelated files.
- Focused reverse-order mutation — killed: changing the comparator to select `grant-z-adapter` made the new exact-winner regression fail against expected `grant-a-domain`; the production comparator was restored and the focused suite rerun green.
- `git diff --check` — passed.

All acceptance criteria are satisfied. This is implementation-level replay evidence only; no formal-model or conformance-vector promotion is claimed.

## Review findings — pass 1 (2026-08-10)

**Receiver-accepted**:

- Canonical protocol prose must make the mutually exclusive liveness classification explicit: a grant that is both revoked and expired classifies as revoked, while selection still considers the resulting classes live → expired → revoked.
- Fixed-time regressions must prove lower-id expired/revoked matches cannot defeat a higher-id live match, expired denial provenance outranks revoked provenance, revocation-first classification governs a revoked+expired match, and fresh replay returns each exact decision.
- Add the cheap exact-byte edge: canonically equivalent composed/decomposed Unicode ids must retain raw UTF-8 ordering without normalization.

**Rejected**: no formal model or conformance vector is added. The property-graded baseline permits this authority obligation to remain stated-normative, and this feature ratifies production behavior rather than promoting assurance tier.

**Out of scope**: lower-risk endpoint-class enforcement remains outside this review fix; no backlog or sibling item was touched.

**Closure policy**: explicit `thorough`; keep the feature at `review` after the fix and verification so the caller can run convergence pass 2.

## Review fix verification — pass 1

- `docs/PROTOCOL.md` now states revocation-first classification before the separate live → expired → revoked selection order; the production `GrantLiveness::liveness_at` behavior was already correct, and its selection-site comment now mirrors the contract.
- `core/tests/authority_replay.rs` adds fixed-time live/expired/revoked and expired-vs-revoked cases, asserts the revoked+expired class directly, checks exact accepted/denied provenance before and after production-log replay, and covers composed/decomposed UTF-8 ids under reverse insertion pressure.
- `cargo test -p patchbay-core --test authority_grant_check --test authority_replay` — passed (11/11).
- `cargo test -p patchbay-core --test authority_proptest` — passed (14/14).
- `cargo test --workspace` — passed.
- `cargo clippy --workspace --all-targets -- -D warnings` — passed.
- `node contracts/scripts/check-models.mjs` — passed.
- `node contracts/scripts/check-vectors.mjs` — passed (21 implementation checks; 37 mutation witnesses killed); no model, vector, generated source, or traceability table changed.
- `rustfmt --edition 2021 --check core/src/authority/check.rs core/tests/authority_replay.rs` and `git diff --check` — passed. Repository-wide `cargo fmt --all -- --check` remains red on 71 pre-existing, out-of-scope Rust files; neither owned Rust file appears in that diff.

The corrected feature remained at `review` for thorough convergence pass 2.

## Review closure — pass 2

- Fresh-context adversarial pass 2 found no material current-cycle blockers.
- Confirmed revocation-first liveness classification, live → expired → revoked
  class selection, raw UTF-8 grant-id ordering, and exact hot/replay provenance.
- Recurring findings: none. The pass-1 evidence gap did not recur.
- Lower-risk endpoint-class matching remains outside this feature and is parked
  separately; it does not affect current producers, which leave the class empty.
- Effective weight: `thorough` (explicit operator). Verdict: approved.
