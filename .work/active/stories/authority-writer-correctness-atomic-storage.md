---
id: authority-writer-correctness-atomic-storage
kind: story
stage: done
tags: [security, foundation]
parent: authority-writer-correctness
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-10
updated: 2026-08-10
---

# Atomic grant-identity storage and audit transaction

## Checkpoint
Establish the backend-neutral atomic absent/exact/conflict primitive that both normal and descendant grant creation will consume. This checkpoint owns the storage contract, SQLite v5 identity index and checked backfill, generic audited-path bypass rejection, and focused storage evidence. It does not change authority ingress yet.

## Design contract

**Files:**
- `core/src/storage/port.rs`
- `core/src/storage/mod.rs`
- `core/src/storage/audited.rs`
- `core/src/storage/rusqlite.rs`
- `core/tests/rusqlite_storage.rs`
- `core/tests/audit_records.rs`
- focused `Storage` test doubles only where the new method is consumed

Add:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GrantIdentityKey(String);

impl GrantIdentityKey {
    pub fn new(value: String) -> Option<Self>;
    pub fn as_str(&self) -> &str;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrantAppendOutcome {
    Appended(AuditedAppend),
    Existing(EventId),
}

pub trait Storage: Send + Sync {
    fn append_grant_audited(
        &self,
        authority_domain_id: &AuthorityDomainId,
        identity: &GrantIdentityKey,
        source: StoredEventPayload,
        audit: AuditRecordDraft,
    ) -> impl Future<Output = Result<GrantAppendOutcome, StorageError>> + Send;
}
```

Add `StorageError::GrantIdentityConflict { grant_id, existing_lsn }`. The default method is `UnsupportedOperation`; there is no read-then-append fallback.

## Required behavior
- Advance SQLite to schema v5 with a `grant_identities(authority_domain_id, grant_id, source_lsn)` key→source-LSN constraint. Event content remains authoritative in `events`.
- Backfill v4 grant/descendant sources in LSN order. Preserve the earliest source for exact historical duplicates; reject different content or kind under the same domain/id. Migration consumes no LSN and emits no audit.
- Validate the index against the authoritative grant event set on every open: missing, extra, foreign, substituted, or conflicting rows are corruption.
- In one writer transaction, decode only the identity boundary: require a `Grant`/`DescendantGrant` source, non-empty embedded id, exact embedded/requested domain, embedded id equal to `GrantIdentityKey`, and the matching truthful creation audit (`GrantCreated/grant_created` or `GrantCreated/descendant_grant_created`). Then encode the candidate and join any identity row to its immutable source event: return `Existing` without writes for equal bytes, conflict without writes for unequal bytes, or append source + identity row + linked audit and commit for absent identity. Replace any input `audit.source_event_id` with the committed source. Storage validates framing but does not construct or interpret grant policy.
- The same key space covers `Grant` and `DescendantGrant`; source kind participates in content comparison.
- `AuditedStorage` delegates the new primitive. Its generic append/audited/decision/batch routes reject grant-creation kinds so production cannot bypass the identity primitive. Remove generic Grant/Descendant audit inference after the dedicated writer supplies the truthful draft.
- Bare raw SQLite append remains available only as a trusted fixture/corruption seam.

## Acceptance evidence
- [ ] First creation with matching key/domain/audit framing returns `Appended`, one source, and one linked creation audit; mismatched framing writes nothing.
- [ ] Exact retry returns the original source `EventId` and leaves the full prefix unchanged.
- [ ] Different same-id content, including normal-vs-descendant, conflicts before append.
- [ ] Barrier-controlled concurrent exact attempts converge on one source/audit/id; conflicting attempts leave one valid winner and one conflict.
- [ ] v4 migration/open preserves earliest exact duplicates and rejects conflicting/index-corrupt history.
- [ ] Production generic storage routes cannot write a grant creation.

## Ordering
No sibling prerequisite. This checkpoint must finish before `authority-writer-correctness-ingest-contract`, which consumes the new fail-closed storage API.

## Verification

```bash
cargo test -p patchbay-core --test rusqlite_storage --test audit_records
cargo clippy -p patchbay-core --all-targets -- -D warnings
```

## Implementation run contract
- Capability: Sol xhigh for the security/durability contract.
- Effective review weight inherited from parent: thorough.
- Findings are proposals; the receiver adjudicates materiality from evidence.
- Do not edit docs, backlog, other items, models, vectors, or generated contracts for this checkpoint.

## Implementation notes
- Added the fail-closed `GrantIdentityKey` / `GrantAppendOutcome` storage contract and backend-neutral `GrantIdentityConflict` context. The server's exhaustive storage-error mapper was updated as a required interface reconciliation.
- SQLite schema v5 adds the domain-qualified `grant_identities` key→source-LSN constraint. v4 migration decodes grant envelopes in LSN order, retains the earliest byte-exact duplicate, and rolls back on conflicting history. Every open validates the exact table definition and complete identity/log correspondence.
- The dedicated writer validates embedded domain/id/kind plus `GrantCreated` reason and matching audit `grant_id`, then commits source, identity, and linked audit in one transaction. Exact retries return the earliest source `EventId` without writes; changed or cross-kind content conflicts before append.
- `AuditedStorage` delegates the dedicated primitive and rejects grant creation through every generic append, audited, dedup, decision, and batch route. Bare `RusqliteStorage::append` remains the intentional trusted corruption-fixture seam.
- Added focused storage/audit evidence for malformed framing, exact retry prefix stability, changed/cross-kind conflicts, barrier-controlled exact/conflicting races, audit cardinality/linkage, checked legacy migration, inconsistent-index rejection, and generic production bypass rejection.
- Verification: `cargo test -p patchbay-core --test rusqlite_storage --test audit_records`; `cargo clippy -p patchbay-core --all-targets -- -D warnings`; `cargo check --workspace`; `git diff --check` — passed.
- Formatting discrepancy: repository-wide `cargo fmt --all -- --check` remains red on pre-existing unrelated Rust drift. No unrelated whole-file reformat was applied.
- Design discrepancies/blockers: none material. Audit framing requires the draft's `grant_id` to equal the immutable identity so the durable creation audit is truthfully queryable; this is the concrete fail-closed interpretation of the designed “truthful creation audit” requirement.
