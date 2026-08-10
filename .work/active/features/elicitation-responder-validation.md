---
id: elicitation-responder-validation
kind: feature
stage: review
tags: [security, protocol]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-09
updated: 2026-08-10
---

# Elicitation responder authority validation

## Brief
Close the responder-authority gap split out of `authority-provenance-hardening`. Absorbs:

- `backlog-elicitation-responder-authority` — **OPEN** (highest-risk silent-check-drop in the set): response Operations (`approval-response`, `elicitation-response`) must be accepted only when the verified issuer maps to `Elicitation.expected_responder_actor`; but the projection retains `expected_responder_actor` (`elicitation.rs:50-56`) while the `ActiveElicitation` port omits it (`ports.rs:110-119`), so `validate_response_payload` cannot compare (`pipeline.rs:247-263`). *Src:* authority review #2(G)+#3(R6).

## Direction
Add an Elicitation lookup/validation port to response-Operation acceptance: on a response Operation, look up the correlated Elicitation, require the verified issuer actor to equal `expected_responder_actor`, deny-by-default on mismatch. This is an acceptance-owned change (acceptance owns the response-Operation path). **Keep it a distinct fail-fast acceptance/Elicitation check** — do NOT fold it into a shared grant primitive (a valid grant is not authority to answer an Elicitation intended for another actor; the review's explicit warning).

## Foundation references
- `docs/PROTOCOL.md` — Elicitation responder matching (`:329-332`); response-Operation acceptance
- `docs/VERIFICATION.md` — `ElicitationResponderAuthority` (stated-normative, untested)
- Code: `core/src/acceptance/elicitation.rs`, `core/src/acceptance/ports.rs`, `core/src/acceptance/pipeline.rs`

## Scope boundaries

This feature closes the existing acceptance implementation gap; it does not change wire shape or protocol semantics.

- Applies to both committed response kinds: `approval-response` and `elicitation-response`.
- Compares only the authenticated issuer's verified actor with the Elicitation's `expected_responder_actor`. Any authenticated endpoint for that actor remains eligible; no endpoint, endpoint-class, or fallback-chain binding is added.
- Retains the ordinary grant check. Expected-responder authority is an additional necessary condition, not a substitute for a live matching grant.
- Does not move the check into `GrantCheck`, add a shared grant primitive, change `ElicitationState`, change first-answer-wins, promote a response-contract kind, or alter `.proto`.
- Does not promote `ElicitationResponderAuthority` beyond stated-normative. The tests below are implementation evidence; formal-property and conformance-vector promotion remain separate work under `docs/VERIFICATION.md`.
- No UI surface changes. The actor-level multi-surface behavior already specified by `docs/UX.md` is preserved.

## Design decisions

- **Use the existing lookup once, but broaden its response-validation context**: Add `expected_responder_actor` to `ActiveElicitation` and update the existing `ElicitationContractLookup` documentation instead of adding a second lookup or renaming the trait. The existing projection is already reconciled under the core decision gate; one snapshot must feed both payload and responder checks.
- **Keep responder validation distinct from grant authorization**: Implement a separate pure `validate_response_responder` check in the Elicitation-response boundary and call it before `GrantCheck`. A valid grant cannot authorize one actor to consume another actor's pending response slot.
- **Use verified ingress identity only**: The validator receives `&dyn IssuerContext` and reads `verified_actor()`; it never reads `Operation.sender`, labels, or endpoint metadata. This preserves the compound-issuer boundary and actor-level fan-out semantics.
- **Fail closed on absent or empty responder evidence**: An active Elicitation whose `expected_responder_actor` is absent or empty is denied exactly like a mismatched actor. The server adapter must return the `ActiveElicitation` with `expected_responder_actor: None`, not collapse it to a missing lookup, so absence is explicit and testable.
- **Preserve existing unknown-Elicitation semantics**: A missing typed correlation or lookup miss remains `validation_failed`. For a known active Elicitation, missing/mismatched responder authority returns pre-acceptance `authorization_denied`; no command record is appended. Diagnostics remain generic and do not echo actor identifiers.
- **Responder authority precedes payload diagnostics for known Elicitations**: After structural validation, verified-issuer establishment, lockdown posture, and lookup, run responder validation before contract-payload validation. This avoids giving a known-but-unauthorized actor contract-detail diagnostics. A lookup miss still flows through the existing payload validator for `validation_failed`.
- **Preserve exact terminal retry behavior**: The winning response may reach storage deduplication only when the current verified actor still matches the Elicitation's expected actor. A different actor cannot replay the winning Operation to obtain its existing command record.
- **Execution posture**: Direct-read only. The feature is bounded to the acceptance response path and its existing server projection; nested exploration was unnecessary and prohibited by the delegated endpoint contract.
- **Review policy**: Effective `review_weight` is `thorough`, source: explicit operator selection. Pass it unchanged to feature review and final completion review.

## Extension pressure classification

- **Committed v0.1.0**: Both response Operation kinds require verified issuer actor equality with the already wire-present `Elicitation.expected_responder_actor`, in addition to ordinary grant authorization. This implements the existing stated-normative `ElicitationResponderAuthority` obligation; it does not introduce a new registry member.
- **Reserved seams**: Specific-endpoint, endpoint-class, service-role, fallback-chain, delegated-responder, responder escalation, and multi-operator distinctions remain reserved exactly as documented. The actor field remains the future-relevant demarcator.
- **Explicitly rejected for this feature**: Treating a grant as sufficient responder authority, trusting `Operation.sender`, binding v0.1.0 Elicitations to one endpoint, or performing a second independent Elicitation lookup.
- **Parked-idea pressure test**: Multi-human coordination remains possible because comparison uses the stored actor id rather than assuming the sole v0.1.0 operator or hard-coding an operator constant. No desktop, mesh, or skin seam is affected.

## Architectural choice

### Option A — enrich the existing active-Elicitation context and add a distinct validator (chosen)

Project `expected_responder_actor` through the existing `ElicitationContractLookup`, then run a pure acceptance-owned actor check before payload/grant/target/durable work. This uses one gate-reconciled projection snapshot, keeps the security rule adjacent to the response path, and adds no wire or persistence concept.

### Option B — add a separate responder lookup port

A dedicated port would make the authority concern visually obvious, but it would read the same `ElicitationSlotLayer` twice and allow contract state and responder state to be observed from different snapshots if a future adapter diverged. It adds a concept without a second data source or consumer need.

### Option C — fold expected-responder matching into `GrantCheck`

This would centralize denials, but it conflates target/kind grant authority with ownership of one Elicitation response slot. It is explicitly rejected by the review-vetted direction: a valid session grant is not permission to answer a slot intended for another actor.

**Choice**: Option A. It follows the domain-owned-port and fail-fast-boundary patterns while remaining the smallest change that cannot silently degrade into grant-only authorization.

## Trickiest unit first

The riskiest unit is the acceptance ordering in `pipeline.rs`. The lookup must occur once; a known Elicitation must reject a wrong or missing responder before payload diagnostics, grant evaluation, target resolution, idempotency lookup, or append; and an unknown Elicitation must retain its existing `validation_failed` result. Exact terminal retries must still reach deduplication for the expected actor. The interface and mutation-oriented tests are designed around this ordering rather than around private helper coverage alone.

## Implementation Units

### Unit 1: Carry responder evidence through the acceptance-owned port

**Files**: `core/src/acceptance/ports.rs`, `core/src/acceptance/elicitation_response.rs`, `core/src/acceptance/mod.rs`

```rust
// core/src/acceptance/ports.rs
#[derive(Debug, Clone, PartialEq)]
pub struct ActiveElicitation {
    pub contract: ResponseContract,
    pub expected_responder_actor: Option<ActorId>,
    pub is_terminal: bool,
    pub winning_response: Option<Operation>,
}

// core/src/acceptance/elicitation_response.rs
pub fn validate_response_responder(
    active: &ActiveElicitation,
    issuer: &dyn IssuerContext,
) -> Result<(), String>;
```

**Implementation notes**:

- Update the `ElicitationContractLookup` comment to describe one side-effect-free active-Elicitation response-validation lookup (contract, lifecycle/dedup context, and responder authority), while retaining its current name to avoid a repository-wide semantic rename with no behavioral value.
- `validate_response_responder` succeeds only when expected and verified actor ids are both present, non-empty, and exactly equal. Missing/empty expected evidence, incomplete/empty verified actor evidence, and inequality all return generic denial diagnostics without actor values.
- Keep the validator independent from `validate_response_payload`; the two functions protect different contracts and map to different failure codes.
- Re-export `validate_response_responder` from `acceptance::mod` for interface/property tests.

**Acceptance criteria**:

- [ ] `ActiveElicitation` exposes the projected optional expected actor without changing generated contracts or `ElicitationRecord`.
- [ ] Exact non-empty verified-actor equality succeeds; mismatch and absent/empty expected or verified actor evidence fail closed.
- [ ] The check never consults caller-supplied `Operation.sender` or endpoint identity.

### Unit 2: Enforce responder authority in response-Operation acceptance

**File**: `core/src/acceptance/pipeline.rs`

```rust
if matches!(
    validated.operation_kind,
    OperationKind::ApprovalResponse | OperationKind::ElicitationResponse
) {
    let active = match correlation_to_elicitation(&operation.correlations) {
        Some(id) => contract_lookup.active_contract(&id).await,
        None => None,
    };

    if let Some(active) = active.as_ref() {
        if let Err(diagnostic) = validate_response_responder(active, issuer) {
            return Ok(rejected_result(
                Some(validated.command_id.clone()),
                FailureCode::AuthorizationDenied,
                "authorization_denied".to_owned(),
                None,
                diagnostic,
            ));
        }
    }

    if let Err(diagnostic) = validate_response_payload(&operation, active.as_ref()) {
        // Existing validation_failed mapping.
    }
}
```

**Implementation notes**:

- Reuse `acceptance::elicitation::correlation_to_elicitation` instead of retaining the pipeline's second inline typed-correlation scanner. `validate_response_payload` may retain its own diagnostic-oriented check.
- Ordering remains: structural/time validation → verified compound issuer → security posture → one Elicitation lookup → responder authority for a known slot → response payload contract → ordinary grant → target resolution → deduplicating append.
- A known missing/mismatched responder maps to `SubmissionOutcome::Rejected`, `FailureCode::AuthorizationDenied`, reason `authorization_denied`, no decision grant id, and no command state. The existing server rejection path records the corresponding authorization-failure audit.
- Both response kinds use the same actor check. Non-response Operations do not consult or depend on Elicitation responder state.

**Acceptance criteria**:

- [ ] A matching expected actor plus a live ordinary grant follows the unchanged accepted path for both response kinds.
- [ ] A mismatched actor or missing/empty `expected_responder_actor` rejects both response kinds with `authorization_denied` before grant, target, dedup, or append.
- [ ] A missing correlation/unknown Elicitation remains `validation_failed`, while a matching actor with malformed payload reaches the existing payload validation result.
- [ ] A valid grant never masks a responder mismatch.
- [ ] An exact terminal retry is admitted only for the expected actor and continues to return the existing deduplicated command record.

### Unit 3: Preserve responder evidence in the production projection adapter

**File**: `server/src/state.rs`

```rust
impl ElicitationContractLookup for LockedElicitationContractLookup {
    async fn active_contract(&self, elicitation_id: &ElicitationId) -> Option<ActiveElicitation> {
        let layer = self.inner.lock().await;
        let record = layer.get_slot(elicitation_id)?;
        Some(ActiveElicitation {
            contract: record.contract.clone()?,
            expected_responder_actor: record.expected_responder_actor.clone(),
            is_terminal: is_terminal_state(record.state),
            winning_response: record.winning_response.clone(),
        })
    }
}
```

**Implementation notes**:

- Copy the optional actor exactly; do not use `?` on it. A malformed opening record with no responder must reach the explicit deny-by-default check.
- Keep the current `CoreDecisionGate`/`catch_up` topology. No direct storage read, new lock, or second projection is introduced.
- Extend the existing fold-lag test so an actor on an appended Elicitation is visible after catch-up and after `ProjectionState::rebuild`.

**Acceptance criteria**:

- [ ] Live catch-up and restart rebuild return the same `expected_responder_actor` that the durable Elicitation carries.
- [ ] An absent expected actor remains an explicit `None` in `ActiveElicitation`, rather than becoming a lookup miss or default operator.

### Unit 4: Mutation-sensitive implementation evidence

**Files**: `core/src/acceptance/elicitation_response.rs`, `core/tests/acceptance_pipeline.rs`, `core/tests/authority_proptest.rs`, `server/src/state.rs`

**Implementation notes**:

- Add focused pure-validator cases for non-empty equality, mismatch, and missing/empty expected or verified actor evidence.
- In `acceptance_pipeline.rs`, exercise both response Operation kinds with an otherwise-authorizing `GrantCheck`. The wrong-actor and missing-expected cases must assert `AuthorizationDenied`, `authorization_denied`, zero grant calls, zero target-resolver calls, and an empty durable event list. This is the load-bearing mutation witness: deleting the responder check would accept under the deliberately valid grant.
- Add a matching-actor case that proves the ordinary grant/target/append path still runs, plus expected-actor and wrong-actor exact-terminal-retry cases so dedup behavior is not accidentally weakened.
- Replace the stale “ElicitationResponderAuthority: NOT TESTED HERE” gap in `authority_proptest.rs` with an actor-equality property over generated distinct/equal actor ids and the production validator. Keep the assurance wording honest: this is implementation evidence, not a promoted formal property.
- Extend `server::state::tests::fold_lag_invariant_exposes_contract_only_after_storage_catch_up` to assert responder carriage through catch-up and restart.

**Acceptance criteria**:

- [ ] The real acceptance path kills a grant-only mutation for both response kinds.
- [ ] Equality behavior is property-tested across actor values without deriving the oracle from action-recorded state.
- [ ] Production projection wiring cannot silently drop `expected_responder_actor`.
- [ ] No test claims checked-model, checked-normative, or promoted-vector status.

## Implementation Order

1. Unit 1 — extend `ActiveElicitation`, add/export the pure responder validator, and update its focused unit cases.
2. Unit 3 — immediately update the production projection adapter and its live/rebuild evidence so the new required context is wired before broader compilation.
3. Unit 2 — insert the distinct pre-grant acceptance check and consolidate typed-correlation extraction.
4. Unit 4 — complete interface, mutation, retry, property, and production-wiring evidence; run the full workspace verification.

## Child-story decision

No child stories are spawned. The new field, production adapter, pipeline check, and mutation witness form one tightly cohesive security boundary; splitting code from its evidence would create non-green intermediate checkpoints and duplicated ownership. One feature-owning implementation worker should deliver and verify the bundle in a single stride.

## Simplification

- Reuse the existing `ElicitationSlotLayer`, `ElicitationContractLookup`, `ActiveElicitation`, generated `expected_responder_actor`, and core decision gate. Add no new service, projection, storage read, or wire type.
- Replace the pipeline's inline correlation scan with the existing `correlation_to_elicitation` helper.
- Do not rename `ElicitationContractLookup`/`LockedElicitationContractLookup`; update their comments to reflect the broadened validation context. A rename would touch numerous test adapters without removing runtime complexity.
- Retain payload validation, grant validation, and responder validation as three explicit checks because consolidating them would weaken failure semantics or recreate the silent-check-drop risk.
- No existing valuable test is removed. Avoid one test per initializer or branch; use table/property cases around the authority boundary.

## Testing

- **Pure boundary tests** (`core/src/acceptance/elicitation_response.rs`): protect non-empty actor equality and fail-closed missing/empty evidence.
- **Acceptance interface tests** (`core/tests/acceptance_pipeline.rs`): protect failure mapping, ordering, both response kinds, no pre-acceptance side effects, and terminal-retry behavior. These are the primary regression evidence.
- **Property/mutation evidence** (`core/tests/authority_proptest.rs`): protect the `ElicitationResponderAuthority` implementation against actor mismatch and against removing the distinct check while a grant remains valid.
- **Production adapter test** (`server/src/state.rs`): protects the exact defect that motivated the feature—the projection record retaining the actor while the acceptance port drops it—across live catch-up and restart.
- Existing generic server tests already establish that pre-acceptance `AuthorizationDenied` results are audited as authorization failures; do not duplicate the entire gRPC fixture solely for this reason.
- No conformance vector or formal-model promotion is part of this feature.

Implementation verification commands:

```bash
cargo fmt --all -- --check
cargo test -p patchbay-core --lib elicitation_response
cargo test -p patchbay-core --test acceptance_pipeline
cargo test -p patchbay-core --test authority_proptest
cargo test -p patchbay-core-server state::tests::fold_lag_invariant_exposes_contract_only_after_storage_catch_up
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
node contracts/scripts/check-vectors.mjs
```

## Risks

- **Silent projection drop recurs**: `ElicitationRecord` already stores the expected actor, but the adapter can omit it again. Mitigation: make it an explicit `ActiveElicitation` field and test live plus rebuilt production projection output.
- **Grant-only regression**: a refactor could delete or move the responder check into `GrantCheck`. Mitigation: interface tests deliberately use a valid grant for the wrong actor and require zero grant invocations.
- **Payload identity confusion**: using `Operation.sender` would reintroduce self-asserted authority. Mitigation: validator signature accepts `IssuerContext`, and tests use contradictory payload/verified identities where useful.
- **Retry regression**: terminal exact-retry support is equality-sensitive and already subtle. Mitigation: test expected and unexpected verified actors against the same winning response before relying on storage dedup.
- **Failure-code drift**: treating mismatches as malformed payload would hide an authority decision. Mitigation: assert canonical `AuthorizationDenied`/`authorization_denied`; keep lookup misses as `ValidationFailed`.
- **TOCTOU or lock growth**: a second lookup/port could observe different slot state or nest locks. Mitigation: one gate-reconciled lookup result feeds both checks; no new lock or storage access.
- **Future multi-operator foreclosure**: hard-coding the sole v0.1.0 operator would make later promotion invasive. Mitigation: compare typed actor ids already carried by the Elicitation; do not compare endpoints or constants.

Fallback if enriching the active context proves unexpectedly invasive: keep the same single `ElicitationContractLookup` trait and introduce a renamed response-validation DTO returned by `active_contract`; do not fall back to a second lookup or grant folding. The uncertainty is naming/compile fan-out, not protocol feasibility.

## Other agent review

- **Invoked because**: responder authority is a security-critical pre-acceptance rule with a documented silent-check-drop history.
- **Skipped/degraded**: the delegated endpoint contract explicitly prohibited nested subagents and peeragent. Design-time advisory review therefore could not run; this is non-blocking under the risk-driven policy. The feature's effective `thorough` implementation review remains required.
- **Fixed/active blockers**: none found during direct evidence review.
- **Parked**: none.
- **Rejected**: shared-grant folding and a second lookup, for the reasons in Architectural choice.

## Implementation notes
- Execution capability: `openai-codex/gpt-5.6-sol` — explicit autopilot caller selection for a security-critical acceptance authority boundary.
- Review weight: `thorough`, source: explicit operator selection; implementation stops at `stage: review` for the required separate fresh review.
- Files changed: `core/src/acceptance/ports.rs`, `core/src/acceptance/elicitation_response.rs`, `core/src/acceptance/mod.rs`, `core/src/acceptance/pipeline.rs`, `core/tests/acceptance_pipeline.rs`, `core/tests/authority_proptest.rs`, `server/src/state.rs`, and this feature item.
- Tests added/removed: added pure fail-closed responder-equality cases; real acceptance-path coverage for both response kinds, grant-only wrong-actor mutation witnesses, failure ordering, unknown/malformed behavior, and expected/wrong-actor terminal retries; added generated-actor equality property evidence; extended production projection catch-up/restart and absent-evidence carriage coverage. Removed none.
- Simplification: reused the existing `correlation_to_elicitation` helper and the single `ElicitationContractLookup` snapshot; no second port, lookup, lock, storage read, grant primitive, or wire type was introduced.
- Discrepancies from design: none. The implementation kept responder authority acceptance-owned, verified-issuer-only, pre-grant, and distinct from payload and grant validation.
- Adjacent issues parked: none.

## Integrated verification
- `cargo test -p patchbay-core --lib elicitation_response` — pass (7 tests).
- `cargo test -p patchbay-core --test acceptance_pipeline` — pass (23 tests).
- `cargo test -p patchbay-core --test authority_proptest` — pass (14 tests).
- `cargo test -p patchbay-core-server state::tests::fold_lag_invariant_exposes_contract_only_after_storage_catch_up` — pass.
- `cargo test --workspace` — pass.
- `cargo clippy --workspace --all-targets -- -D warnings` — pass.
- `node contracts/scripts/check-vectors.mjs` — pass after installing/building the repository's declared TypeScript dependencies; 52 vectors read, 15 promoted vectors, 20 implementation checks, and 37 mutation witnesses killed. Dependency installation changed no tracked files.
- Grant-only mutation check: temporarily deleting the distinct responder-validation block made `responder_mismatch_or_missing_expected_actor_denies_before_grant_target_and_append` fail; restoring production code made it pass.
- Projection-drop mutation check: temporarily replacing projected responder carriage with `None` made `fold_lag_invariant_exposes_contract_only_after_storage_catch_up` fail; restoring production code made it pass.
- `cargo fmt --all -- --check` — repository baseline remains non-green because of pre-existing repo-wide rustfmt drift beginning in untouched files such as `core/src/acceptance/elicitation.rs` and `core/src/acceptance/index.rs`; no unrelated formatting was applied. Changed code compiles cleanly and passes Clippy with warnings denied.
- Acceptance criteria walk-through: exact non-empty verified actor equality is required for both response kinds; mismatched/missing/empty responder evidence returns generic pre-acceptance `authorization_denied` with zero grant/target/append work; matching actors still require and traverse the ordinary grant/target/dedup path; unknown Elicitations remain `validation_failed`; responder denial precedes known-slot payload diagnostics; only the expected actor reaches exact-terminal dedup; and the production projection preserves present and absent responder evidence across live catch-up and restart.

## Review (2026-08-10) — thorough pass 1

**Verdict**: Request changes — receiver-accepted blocker fixed; feature remains at `stage: review` for the required thorough convergence pass.

**Blockers**:
- **Fixed — terminal retry compared untrusted sender with normalized winner**: `validate_response_payload` compared the caller-supplied `Operation.sender` byte-for-byte with the production-normalized winning response before storage deduplication. An exact retry of an accepted response whose original sender claim was forged therefore failed as already terminal even though the verified actor was the expected responder. The pre-dedup terminal comparison now clears only `sender` on cloned Operations; all other fields remain under exact equality, and the original logical Operation bytes still reach storage unchanged for dedup/conflict protection.

**Important**: none.

**Nits**: none.

**Rejected**: none.

**Notes**:
- Effective review weight: `thorough`, source: explicit operator selection. This is the receiver fix for pass 1, not closure; the corrected snapshot remains in review for the next independent pass.
- Regression first failed against the uncorrected code: the production-shaped `ApprovalResponse` case returned `Rejected` instead of reaching deduplication.
- Production-shaped regression now covers both committed response kinds. It submits an original Operation with a forged sender, decodes the durably recorded normalized winning response, proves an exact original retry from the expected verified actor deduplicates with one Operation event, and proves a different verified actor is denied before grant/target/dedup with no second Operation.
- Focused verification: `cargo test -p patchbay-core --lib elicitation_response` (7 passed), `cargo test -p patchbay-core --test acceptance_pipeline` (23 passed), and `cargo test -p patchbay-core --test authority_proptest` (14 passed).
- Integrated verification: `cargo test --workspace` passed; `cargo clippy --workspace --all-targets -- -D warnings` passed; `node contracts/scripts/check-vectors.mjs` passed (53 vectors, 16 promoted, 21 implementation checks, 37 mutation witnesses killed); `git diff --check` passed.
- Repository-wide `cargo fmt --all -- --check` retains the pre-existing baseline drift already recorded above, beginning in untouched acceptance files; no unrelated formatting was bundled.
