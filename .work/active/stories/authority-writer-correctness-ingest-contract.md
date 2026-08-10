---
id: authority-writer-correctness-ingest-contract
kind: story
stage: done
tags: [security, foundation]
parent: authority-writer-correctness
depends_on: [authority-writer-correctness-atomic-storage]
release_binding: null
gate_origin: null
created: 2026-08-10
updated: 2026-08-10
---

# Normal and descendant authority writer contract

## Checkpoint
Route both immutable grant-creation writers through the atomic storage identity primitive while preserving boundary validation, exact descendant audit linkage, stable public signatures, and replay-equivalent projection warming.

## Design contract

**Files:**
- `core/src/authority/ingest.rs`
- `core/src/authority/projection.rs` only for comments/simplification caused by removing creation lookup
- `core/src/authority/mod.rs` only for required exports
- `core/tests/authority_ingest.rs`
- `core/tests/authority_replay.rs`
- `core/tests/authority_proptest.rs`

Keep public signatures unchanged:

```rust
pub async fn ingest_grant<S, L>(
    storage: &S,
    projection: &mut L,
    authority_domain_id: &AuthorityDomainId,
    grant: Grant,
) -> Result<EventId, AuthorityError>
where
    S: Storage,
    L: GrantProjection;

pub async fn ingest_descendant_grant<S, L>(
    storage: &S,
    projection: &mut L,
    authority_domain_id: &AuthorityDomainId,
    grant: DescendantGrant,
) -> Result<EventId, AuthorityError>
where
    S: Storage,
    L: GrantProjection;
```

Use one private helper around `GrantIdentityKey` / `GrantAppendOutcome`; add no second public authority service.

## Required behavior
- Preserve all validation before the atomic call: requested/message domain, non-empty grant id, complete target, normal registry preflight, exact descendant kinds, deterministic descendant id, lifecycle/provenance, and exact prior same-domain completion audit/source linkage.
- Build the normalized `StoredEventPayload` first. Derive the identity key directly from validated `GrantId`; descendants use existing `descendant_grant_id`.
- First normal creation supplies `GrantCreated/grant_created`; first descendant creation supplies `GrantCreated/descendant_grant_created`.
- Delete projection-driven `GrantCreated` versus `GrantChanged` selection. Exact creation retry writes no audit; same-id changed content maps from `StorageError::GrantIdentityConflict` to `AuthorityError::CorruptLog`.
- For both `Appended` and `Existing`, read the exact immutable returned source event, require envelope equality with the candidate, and fold it into the projection before returning. Do not construct a synthetic warm event from request bytes.
- Descendant creation must fold its exact completion source and audit prerequisites before the grant so `AuthorityRegistry` remains the provenance authority.
- Leave revocation behavior and its real `current_grant` lookup unchanged.

## Acceptance evidence
- [ ] Normal and descendant exact writer retries return the original source `EventId`, add no source/audit, and leave fresh/warm projections replay-equivalent.
- [ ] Different-content retries fail as `AuthorityError::CorruptLog` before the prefix or projection changes.
- [ ] A normal/descendant cross-kind collision under one id fails before append.
- [ ] Malformed descendant audit/provenance/kind/lifecycle inputs still fail before identity/log mutation.
- [ ] Immutable creation never emits `GrantChanged`; revocation and replay semantics remain green.

## Ordering
Depends on `authority-writer-correctness-atomic-storage`. This checkpoint must finish before `authority-writer-correctness-retry-evidence`, which exercises ambiguous commits and the production driver.

## Verification

```bash
cargo test -p patchbay-core --test authority_ingest --test authority_replay --test authority_proptest
cargo clippy -p patchbay-core --all-targets -- -D warnings
```

## Implementation run contract
- Capability: Sol xhigh for the security/durability contract.
- Effective review weight inherited from parent: thorough.
- Findings are proposals; the receiver adjudicates materiality from evidence.
- Do not edit docs, backlog, other items, models, vectors, or generated contracts for this checkpoint.

## Implementation notes
- Both normal and descendant creation now build a canonical stored envelope and truthful `GrantCreated` audit, then call the atomic domain-qualified identity primitive. The projection-driven `GrantChanged` branch and generic append path were removed; revocation lookup/writer behavior is unchanged.
- The shared private helper maps same-id changed content to `AuthorityError::CorruptLog`, accepts `Appended` or `Existing`, reads the exact returned source `EventId` back from storage, requires envelope equality, and only then folds that durable record. No `CoreDecisionGate` participates in this correctness path.
- Descendant ingress still folds and validates the complete gap-free spawn prefix before storage. The completed spawn-tail interface was reconciled so an already-committed, context-valid descendant reconstructs the same canonical issuance for exact retry instead of being rejected before the identity transaction.
- Creation audits retain subject/target attribution, carry the exact grant id, and occur only on the first atomic append. Exact normal/descendant retries return the original source id and make a fresh projection replay-equivalent; changed normal content and normal/descendant cross-kind collisions preserve the durable prefix and first identity.
- Updated the one focused storage test adapter consumed by authority creation to delegate the new atomic primitive; the default remains fail-closed for unrelated fakes.
- Verification: `cargo test -p patchbay-core --test authority_ingest --test authority_replay --test authority_proptest`; `cargo clippy -p patchbay-core --all-targets -- -D warnings`; `cargo check --workspace`; `git diff --check` — passed.
- Formatting discrepancy: repository-wide rustfmt remains on the pre-existing red baseline; unrelated whole-file formatting was preserved.
- Design discrepancies/blockers: no material flaw. The designed stable public writer signatures were preserved. `SpawnDescendantTail::descendant_issuance_for` required the narrow retry reconciliation above because the hardened descendant-completion implementation previously returned no issuance once the source already existed; leaving that interface unchanged would have contradicted the explicit descendant retry contract.
