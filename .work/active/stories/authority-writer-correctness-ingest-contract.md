---
id: authority-writer-correctness-ingest-contract
kind: story
stage: implementing
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
