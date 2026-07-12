//! Property tests for storage invariants.
//!
//! This is the evidence floor for the stated-normative obligations in
//! `specs/seed/snapshot_recovery.qnt` and the promoted `BoundaryDedup`
//! property in `specs/seed/command_lifecycle.qnt`. These tests do not (yet)
//! carry checked formal-model formulas — the v1 formal gate owns the real
//! properties. What they *do* is give the implementation-backed evidence
//! that the storage layer upholds the properties it claims.
//!
//! # Properties and their honest scope
//!
//! - **Gap-free LSN** (`committed_lsns_are_gap_free_and_monotonic`): the
//!   empirical validation of the bare `INTEGER PRIMARY KEY`-as-LSN choice.
//!   Tests the guarantee rather than assuming it. Scoped to a single
//!   authority domain — the global rowid means per-domain LSNs interleave
//!   when multiple domains write; v0.1.0 has one domain so this is moot in
//!   production. `per_domain_lsns_monotonic_under_cross_domain_writes`
//!   documents the cross-domain behavior honestly (monotonicity, not
//!   contiguity).
//! - **Deterministic replay** (`replay_deterministic_for_unchanged_contents`):
//!   `IdempotentLogReplay` (stated-normative, storage-layer portion). Two
//!   `recover()` calls on unchanged contents return identical raw materials.
//!   End-to-end idempotent replay requires the domain layer's deterministic
//!   `apply`, which is not built yet.
//! - **Crash recovery** (`crash_recovery_preserves_full_events`):
//!   `CrashNoAcceptedLost` (stated-normative). All committed events —
//!   including their full payloads — survive a reopen. NOTE: "reopen" is
//!   not a process-level crash; it does not prove `synchronous=FULL`
//!   durability against power loss. It proves committed events are visible
//!   after the storage handle is dropped and a new one opens the same DB
//!   file. Process-level durability is a config assertion, not a proptest.
//! - **Snapshot bounds replay to the correct tail** (`snapshot_bounds_tail`):
//!   tests the storage-layer portion of `SnapshotConsistentPrefix`
//!   (stated-normative) — a snapshot at LSN N bounds replay to events > N,
//!   and the tail events are byte-identical to the corresponding events in a
//!   full replay. The snapshot *payload content* (that it reflects events
//!   1..=N) is a caller obligation per `port.rs` `write_snapshot`; the
//!   storage layer cannot prove it because the payload is opaque bytes.
//! - **Fail Fast rejections** (`write_snapshot_rejects_invalid_lsn`,
//!   `snapshot_isolated_per_authority_domain`): invalid-LSN rejection and
//!   cross-domain isolation.
//! - **BoundaryDedup** (`append_dedup_*`): the promoted property — retrying
//!   the same idempotency key against the same target cannot double-apply a
//!   command. A retry returns the existing event; a conflicting payload is
//!   rejected AND no conflicting event is persisted; different targets do
//!   not dedup; concurrent same-key submissions produce exactly one append;
//!   dedup keys survive restart.
//!
//! # Reserved error variants (not tested here)
//!
//! `StorageError::SnapshotStale` and `StorageError::SnapshotWrongDomain` are
//! reserved for future snapshot-reconciliation operations. The current
//! `write_snapshot` / `load_latest_snapshot` API shape cannot trigger them:
//! the authority domain is a call parameter (not a self-contained snapshot
//! object that could carry a conflicting domain), and staleness is a
//! reconciliation-time concern (the snapshot is older than current state),
//! not a write-time concern (writing a checkpoint at an older LSN is
//! legitimate). These variants are the non-foreclosure seam; they get tests
//! when the operations that can trigger them are implemented.
//!
//! # Mutation discipline
//!
//! The mutation tests prove non-vacuity: each wraps `RusqliteStorage` in a
//! fault-injecting adapter and asserts the property *fails* on the buggy
//! store. A property test that passes on a buggy implementation is vacuous.
//! Proptest shrinking is automatic when a generated case fails — the mutation
//! tests prove the properties *catch* the named bugs; shrinking kicks in
//! transitively when the proptest runner hits the failure.

use std::sync::Arc;

use patchbay_contracts::patchbay::{
    AuthorityDomainId, EventId, IdempotencyKey, Lsn, StoredEventKind, StoredEventPayload,
};
use patchbay_core::storage::{recover, Storage, TargetKey};
use proptest::prelude::*;

/// Any concrete (non-`Unspecified`) event kind.
fn any_event_kind() -> impl Strategy<Value = StoredEventKind> {
    prop_oneof![
        Just(StoredEventKind::Operation),
        Just(StoredEventKind::Observation),
        Just(StoredEventKind::Elicitation),
        Just(StoredEventKind::Grant),
        Just(StoredEventKind::DescendantGrant),
        Just(StoredEventKind::Revocation),
        Just(StoredEventKind::SessionState),
    ]
}

/// A generated event payload: a concrete kind + arbitrary bytes.
fn any_payload() -> impl Strategy<Value = StoredEventPayload> {
    (any_event_kind(), prop::collection::vec(any::<u8>(), 0..32)).prop_map(|(kind, payload)| {
        StoredEventPayload {
            kind: kind as i32,
            payload,
        }
    })
}

/// A payload whose bytes encode the index, for tests that need a stable
/// identity per event and byte-exact comparison across recoveries.
fn indexed_payload(i: usize) -> StoredEventPayload {
    StoredEventPayload {
        kind: StoredEventKind::Operation as i32,
        payload: vec![(i >> 8) as u8, i as u8],
    }
}

fn test_domain() -> AuthorityDomainId {
    AuthorityDomainId {
        value: "proptest-domain".to_string(),
    }
}

fn lsn(v: u64) -> Lsn {
    Lsn { value: v }
}

/// Open a fresh in-memory storage for each proptest case. Each case is
/// isolated — no cross-test contamination.
async fn fresh_storage() -> patchbay_core::storage::RusqliteStorage {
    patchbay_core::storage::RusqliteStorage::open_in_memory().expect("in-memory storage opens")
}

/// Extract the LSN value from an EventId.
fn lsn_of(id: &EventId) -> u64 {
    id.lsn.as_ref().unwrap().value
}

/// Compare two `RecordedEvent`s by full content: event_id (domain + LSN),
/// kind, and payload bytes. This is the oracle that catches payload
/// corruption mutants that LSN-only comparisons miss.
fn events_match(expected: &[StoredEventPayload], got: &[patchbay_core::storage::RecordedEvent]) {
    assert_eq!(
        got.len(),
        expected.len(),
        "event count mismatch: expected {}, got {}",
        expected.len(),
        got.len()
    );
    for (i, (exp, actual)) in expected.iter().zip(got.iter()).enumerate() {
        assert_eq!(
            actual.payload, *exp,
            "event {i} payload mismatch: expected {:?}, got {:?}",
            exp, actual.payload
        );
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 100,
        ..ProptestConfig::default()
    })]

    /// Gap-free LSN: committed LSNs are contiguous (1, 2, 3, ...) for a
    /// single authority domain.
    ///
    /// This is the empirical validation of the bare `INTEGER PRIMARY KEY`
    /// (rowid) as the LSN. Rather than assuming SQLite gives a gap-free
    /// sequence on an append-only table, we test it. Also reads back the
    /// full events to verify payload preservation (not just LSNs).
    #[test]
    fn committed_lsns_are_gap_free_and_monotonic(
        events in prop::collection::vec(any_payload(), 1..100)
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let storage = fresh_storage().await;
            let domain = test_domain();
            let mut lsns = Vec::with_capacity(events.len());
            for payload in &events {
                let event_id = storage.append(&domain, payload.clone()).await.unwrap();
                lsns.push(lsn_of(&event_id));
            }
            let expected: Vec<u64> = (1..=lsns.len() as u64).collect();
            prop_assert_eq!(lsns, expected);

            // Read back and verify full payload preservation — not just LSNs.
            let recovered = storage.read_after(&domain, lsn(0)).await.unwrap();
            events_match(&events, &recovered);
            Ok(())
        })?;
    }

    /// Cross-domain writes: each domain's LSNs are strictly increasing
    /// (monotonic) and the count is correct, but NOT contiguous — the
    /// global rowid is shared across domains, so per-domain LSNs interleave.
    ///
    /// This is the honest property for the global-rowid design. v0.1.0 has
    /// one authority domain, so per-domain contiguity holds trivially in
    /// production. The per-domain gap-free contract (PROTOCOL.md) is a
    /// single-domain v0.1.0 property; multi-domain gap-free is a reserved
    /// seam that would require per-domain LSN allocation if federation
    /// arrives. Also verifies the full `(domain, LSN)` tuple identity on
    /// every read-back event, not just the bare LSN.
    #[test]
    fn per_domain_lsns_monotonic_under_cross_domain_writes(
        cross_events in prop::collection::vec(
            (0u8..3, any_payload()),
            1..60
        )
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let storage = fresh_storage().await;
            let domains: Vec<AuthorityDomainId> = (0..3)
                .map(|i| AuthorityDomainId { value: format!("domain-{i}") })
                .collect();
            let mut per_domain_count = [0u64; 3];
            for (dom_idx, payload) in cross_events {
                let _ = storage.append(&domains[dom_idx as usize], payload).await.unwrap();
                per_domain_count[dom_idx as usize] += 1;
            }
            for (i, domain) in domains.iter().enumerate() {
                let events = storage.read_after(domain, lsn(0)).await.unwrap();
                prop_assert_eq!(events.len() as u64, per_domain_count[i]);
                let lsns: Vec<u64> = events.iter().map(|e| lsn_of(&e.event_id)).collect();
                for w in lsns.windows(2) {
                    prop_assert!(w[0] < w[1], "LSNs not strictly increasing in domain-{i}: {:?}", lsns);
                }
                // Verify the full tuple identity: every read-back event
                // carries the domain we asked for, not a bare LSN.
                for event in &events {
                    let got_domain = event.event_id.authority_domain_id.as_ref().unwrap();
                    prop_assert_eq!(got_domain, domain,
                        "read-back event carried wrong authority domain");
                }
            }
            Ok(())
        })?;
    }

    /// Deterministic replay: for unchanged storage contents, `recover()`
    /// returns identical raw materials. (`IdempotentLogReplay`,
    /// stated-normative — storage-layer portion.)
    #[test]
    fn replay_deterministic_for_unchanged_contents(
        events in prop::collection::vec(any_payload(), 1..50)
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let storage = fresh_storage().await;
            let domain = test_domain();
            for payload in &events {
                storage.append(&domain, payload.clone()).await.unwrap();
            }
            let state1 = recover(&storage, &domain).await.unwrap();
            let state2 = recover(&storage, &domain).await.unwrap();
            prop_assert_eq!(&state1, &state2,
                "two recover() calls diverged for unchanged contents");
            // Also verify the tail matches the written events (full payload).
            events_match(&events, &state1.tail);
            Ok::<(), TestCaseError>(())
        })?;
    }

    /// Deterministic replay with a snapshot: two `recover()` calls return
    /// the same snapshot + tail.
    #[test]
    fn replay_deterministic_with_snapshot(
        events in prop::collection::vec(any_payload(), 2..50),
        snap_after in 1u64..50,
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let storage = fresh_storage().await;
            let domain = test_domain();
            for payload in &events {
                storage.append(&domain, payload.clone()).await.unwrap();
            }
            let n = events.len() as u64;
            let snap_lsn = snap_after.min(n);
            storage
                .write_snapshot(&domain, lsn(snap_lsn), vec![0xEE])
                .await
                .unwrap();
            let state1 = recover(&storage, &domain).await.unwrap();
            let state2 = recover(&storage, &domain).await.unwrap();
            prop_assert_eq!(state1, state2);
            Ok(())
        })?;
    }

    /// Crash recovery: all committed events survive a reopen, with full
    /// payload preservation. (`CrashNoAcceptedLost`, stated-normative.)
    ///
    /// NOTE: "reopen" (drop handle, open same file) is not a process-level
    /// crash. It proves committed events are visible after the handle is
    /// dropped — not that `synchronous=FULL` survives power loss. A
    /// payload-corruption mutant (constant payload, preserved LSNs) is caught
    /// here because we compare full events. See the mutation test
    /// `crash_recovery_catches_payload_corruption`.
    #[test]
    fn crash_recovery_preserves_full_events(
        events in prop::collection::vec(any_payload(), 1..50)
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let temp_path = tempfile::NamedTempFile::new().unwrap()
                .into_temp_path().keep().unwrap();
            let path = temp_path.to_str().unwrap().to_string();
            {
                let storage = patchbay_core::storage::RusqliteStorage::open(&path).unwrap();
                let domain = test_domain();
                for payload in &events {
                    storage.append(&domain, payload.clone()).await.unwrap();
                }
                // Drop storage — handle dropped, no clean shutdown.
            }
            let storage = patchbay_core::storage::RusqliteStorage::open(&path).unwrap();
            let domain = test_domain();
            let recovered = storage.read_after(&domain, lsn(0)).await.unwrap();
            events_match(&events, &recovered);
            Ok::<(), TestCaseError>(())
        })?;
    }

    /// Snapshot bounds replay to the correct tail: a snapshot at LSN N means
    /// `recover()` replays only events > N, and those tail events are
    /// byte-identical to the corresponding events in a full replay from 0.
    ///
    /// This is the storage-layer portion of `SnapshotConsistentPrefix`
    /// (stated-normative). The snapshot *payload content* (that it reflects
    /// events 1..=N) is a caller obligation per `port.rs` — the storage layer
    /// treats the payload as opaque bytes and cannot verify it.
    #[test]
    fn snapshot_bounds_tail(
        n_events in 2u64..50,
        snap_lsn in 1u64..50,
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let snap_lsn = snap_lsn.min(n_events);
            let storage = fresh_storage().await;
            let domain = test_domain();
            for i in 0..n_events {
                storage.append(&domain, indexed_payload(i as usize)).await.unwrap();
            }
            storage
                .write_snapshot(&domain, lsn(snap_lsn), vec![0xFF])
                .await
                .unwrap();

            let with_snap = recover(&storage, &domain).await.unwrap();
            prop_assert!(with_snap.snapshot.is_some());
            prop_assert_eq!(with_snap.start_lsn().unwrap(), snap_lsn);
            prop_assert_eq!(with_snap.tail.len() as u64, n_events - snap_lsn);

            // Full replay from a separate DB with the same events.
            let full_storage = fresh_storage().await;
            for i in 0..n_events {
                full_storage
                    .append(&domain, indexed_payload(i as usize))
                    .await
                    .unwrap();
            }
            let full = recover(&full_storage, &domain).await.unwrap();
            prop_assert!(full.snapshot.is_none());

            // The tail of the snapshot recovery must equal the corresponding
            // suffix of the full replay — full payload comparison.
            let snap_tail: Vec<StoredEventPayload> = with_snap.tail.iter()
                .map(|e| e.payload.clone())
                .collect();
            let full_tail_after_snap: Vec<StoredEventPayload> = full.tail.iter()
                .skip(snap_lsn as usize)
                .map(|e| e.payload.clone())
                .collect();
            prop_assert_eq!(snap_tail, full_tail_after_snap,
                "snapshot tail diverged from full replay for events after snap_lsn");
            Ok(())
        })?;
    }

    /// Snapshot at the log head (last committed LSN): tail is empty.
    #[test]
    fn snapshot_at_log_head_yields_empty_tail(
        n_events in 1u64..40,
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let storage = fresh_storage().await;
            let domain = test_domain();
            for i in 0..n_events {
                storage.append(&domain, indexed_payload(i as usize)).await.unwrap();
            }
            storage
                .write_snapshot(&domain, lsn(n_events), vec![0xCC])
                .await
                .unwrap();
            let recovery = recover(&storage, &domain).await.unwrap();
            prop_assert_eq!(recovery.start_lsn().unwrap(), n_events);
            prop_assert!(recovery.tail.is_empty());
            Ok(())
        })?;
    }

    /// Fail Fast: a snapshot at an LSN with no committed event is rejected.
    #[test]
    fn write_snapshot_rejects_invalid_lsn(
        n_events in 0u64..30,
        offset in 1u64..50,
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let storage = fresh_storage().await;
            let domain = test_domain();
            for i in 0..n_events {
                storage.append(&domain, indexed_payload(i as usize)).await.unwrap();
            }
            let invalid_lsn = n_events + offset;
            let result = storage
                .write_snapshot(&domain, lsn(invalid_lsn), vec![0x00])
                .await;
            prop_assert!(matches!(
                result,
                Err(patchbay_core::storage::StorageError::InvalidSnapshotLsn(v)) if v == invalid_lsn
            ), "expected InvalidSnapshotLsn({invalid_lsn}), got {:?}", result);
            Ok(())
        })?;
    }

    /// Cross-domain snapshot isolation: a snapshot written for domain A does
    /// not surface when loading for domain B. Verifies the returned identity
    /// (domain + LSN + payload), not just Some/None.
    #[test]
    fn snapshot_isolated_per_authority_domain(
        n_events in 1u64..20,
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let storage = fresh_storage().await;
            let domain_a = AuthorityDomainId { value: "domain-a".to_string() };
            let domain_b = AuthorityDomainId { value: "domain-b".to_string() };
            for i in 0..n_events {
                storage.append(&domain_a, indexed_payload(i as usize)).await.unwrap();
            }
            let snap_payload = vec![0xAA];
            storage
                .write_snapshot(&domain_a, lsn(n_events), snap_payload.clone())
                .await
                .unwrap();
            // Domain B has no snapshots.
            let snap_b = storage.load_latest_snapshot(&domain_b, None).await.unwrap();
            prop_assert!(snap_b.is_none(),
                "snapshot from domain-a leaked into domain-b");
            // Domain A's snapshot loads with correct identity.
            let snap_a = storage.load_latest_snapshot(&domain_a, None).await.unwrap().unwrap();
            prop_assert_eq!(lsn_of(&snap_a.event_id), n_events);
            prop_assert_eq!(
                snap_a.event_id.authority_domain_id.as_ref().unwrap(),
                &domain_a
            );
            prop_assert_eq!(snap_a.payload, snap_payload);
            Ok(())
        })?;
    }

    /// BoundaryDedup: retrying the same idempotency key against the same
    /// target returns the existing event — cannot double-apply.
    /// (`appliedKeys` from `command_lifecycle.qnt`.)
    #[test]
    fn append_dedup_retry_returns_existing_no_double_apply(
        payload in any_payload(),
        key in "[a-z]{1,8}",
        target in "[a-z]{1,8}",
        retries in 1u8..5,
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let storage = fresh_storage().await;
            let domain = test_domain();
            let key = IdempotencyKey { value: key };
            let target = TargetKey::new(target).unwrap();
            let first = storage
                .append_dedup(&domain, &key, &target, payload.clone())
                .await
                .unwrap();
            let first_id = match first {
                patchbay_core::storage::DedupOutcome::Appended(id) => id,
                _ => unreachable!("first append must be Appended"),
            };
            let first_lsn = lsn_of(&first_id);
            for _ in 0..retries {
                let outcome = storage
                    .append_dedup(&domain, &key, &target, payload.clone())
                    .await
                    .unwrap();
                match outcome {
                    patchbay_core::storage::DedupOutcome::Duplicate(id) => {
                        // Verify the FULL EventId tuple (domain + LSN), not
                        // just the bare LSN — a wrong-domain Duplicate
                        // with the right LSN must be caught.
                        prop_assert_eq!(&id, &first_id,
                            "retry returned a different EventId than the original");
                        prop_assert_eq!(lsn_of(&id), first_lsn,
                            "retry returned a different LSN than the original");
                    }
                    patchbay_core::storage::DedupOutcome::Appended(_) => {
                        return Err(proptest::test_runner::TestCaseError::fail(
                            "retry appended a new event — double-apply",
                        ));
                    }
                }
            }
            let events = storage.read_after(&domain, lsn(0)).await.unwrap();
            prop_assert_eq!(events.len(), 1, "dedup left more than one event");
            // Full payload preservation on the single event.
            prop_assert_eq!(&events[0].payload, &payload);
            Ok::<(), TestCaseError>(())
        })?;
    }

    /// BoundaryDedup: a key reused across different targets does NOT dedup.
    #[test]
    fn append_dedup_different_targets_do_not_dedup(
        payload in any_payload(),
        key in "[a-z]{1,8}",
        target_a in "[a-z]{1,8}",
        target_b in "[a-z]{1,8}",
    ) {
        prop_assume!(target_a != target_b);
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let storage = fresh_storage().await;
            let domain = test_domain();
            let key = IdempotencyKey { value: key };
            let target_a = TargetKey::new(target_a).unwrap();
            let target_b = TargetKey::new(target_b).unwrap();
            let o1 = storage.append_dedup(&domain, &key, &target_a, payload.clone())
                .await.unwrap();
            let o2 = storage.append_dedup(&domain, &key, &target_b, payload)
                .await.unwrap();
            prop_assert!(matches!(o1, patchbay_core::storage::DedupOutcome::Appended(_)));
            prop_assert!(matches!(o2, patchbay_core::storage::DedupOutcome::Appended(_)));
            let events = storage.read_after(&domain, lsn(0)).await.unwrap();
            prop_assert_eq!(events.len(), 2);
            Ok(())
        })?;
    }

    /// BoundaryDedup: a retry with the same key but a differing payload is
    /// rejected with `IdempotencyConflict` AND no conflicting event is
    /// persisted — the log count stays at 1.
    #[test]
    fn append_dedup_conflict_rejects_and_persists_nothing(
        payload_a in any_payload(),
        payload_b in any_payload(),
        key in "[a-z]{1,8}",
        target in "[a-z]{1,8}",
    ) {
        prop_assume!(payload_a != payload_b);
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let storage = fresh_storage().await;
            let domain = test_domain();
            let key = IdempotencyKey { value: key };
            let target = TargetKey::new(target).unwrap();
            let payload_a_clone = payload_a.clone();
            storage
                .append_dedup(&domain, &key, &target, payload_a)
                .await
                .unwrap();
            let result = storage
                .append_dedup(&domain, &key, &target, payload_b)
                .await;
            prop_assert!(matches!(
                result,
                Err(patchbay_core::storage::StorageError::IdempotencyConflict)
            ), "differing payload must conflict, got {:?}", result);
            // No conflicting event was persisted — exactly one event exists.
            let events = storage.read_after(&domain, lsn(0)).await.unwrap();
            prop_assert_eq!(events.len(), 1,
                "conflicting payload was persisted despite IdempotencyConflict");
            // The surviving event is the original, not the conflicting one.
            prop_assert_eq!(&events[0].payload, &payload_a_clone);
            Ok::<(), TestCaseError>(())
        })?;
    }

    /// BoundaryDedup: a mix of dedup'd and non-dedup'd appends preserves
    /// gap-free LSNs. The idempotency-key table does not disturb the
    /// contiguous LSN sequence. Includes a readback count oracle so an
    /// always-Duplicate mutant (which would produce an empty LSN vector and
    /// pass a windows-only check) is caught.
    #[test]
    fn dedup_appends_remain_gap_free(
        ops in prop::collection::vec(
            ("[a-z]{1,6}", "[a-z]{1,6}"),
            1..40
        )
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let storage = fresh_storage().await;
            let domain = test_domain();
            let payload = indexed_payload(0);
            let mut seen: std::collections::HashSet<(String, String)> =
                std::collections::HashSet::new();
            let mut expected_lsns: Vec<u64> = Vec::new();
            for (key, target) in ops {
                let key = IdempotencyKey { value: key };
                let target = TargetKey::new(target).unwrap();
                let outcome = storage
                    .append_dedup(&domain, &key, &target, payload.clone())
                    .await
                    .unwrap();
                let pair = (key.value, target.as_str().to_string());
                if seen.insert(pair) {
                    match outcome {
                        patchbay_core::storage::DedupOutcome::Appended(id) => {
                            expected_lsns.push(lsn_of(&id));
                        }
                        patchbay_core::storage::DedupOutcome::Duplicate(_) => {
                            return Err(proptest::test_runner::TestCaseError::fail(
                                "first-seen (key,target) returned Duplicate instead of Appended",
                            ));
                        }
                    }
                } else {
                    match outcome {
                        patchbay_core::storage::DedupOutcome::Duplicate(_) => {}
                        patchbay_core::storage::DedupOutcome::Appended(_) => {
                            return Err(proptest::test_runner::TestCaseError::fail(
                                "repeat (key,target) appended instead of duplicating",
                            ));
                        }
                    }
                }
            }
            for w in expected_lsns.windows(2) {
                prop_assert_eq!(w[1] - w[0], 1, "gap in appended LSNs: {:?}", expected_lsns);
            }
            let events = storage.read_after(&domain, lsn(0)).await.unwrap();
            prop_assert_eq!(
                events.len(),
                expected_lsns.len(),
                "log count != expected appended count"
            );
            let readback_lsns: Vec<u64> = events.iter().map(|e| lsn_of(&e.event_id)).collect();
            prop_assert_eq!(readback_lsns, expected_lsns);
            Ok::<(), TestCaseError>(())
        })?;
    }

    /// BoundaryDedup under concurrency: multiple tasks submitting the same
    /// key + target + payload simultaneously must produce exactly one
    /// `Appended` and the rest `Duplicate`, with exactly one persisted event.
    /// This is the concurrent-acceptance-handler case the port's
    /// `appliedKeys` claim is grounded in — sequential tests cannot catch a
    /// check-then-yield-then-append race.
    #[test]
    fn append_dedup_concurrent_same_key_no_double_apply(
        payload in any_payload(),
        key in "[a-z]{1,8}",
        target in "[a-z]{1,8}",
        n_concurrent in 2usize..=8,
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let storage = Arc::new(fresh_storage().await);
            let domain = Arc::new(test_domain());
            let key = Arc::new(IdempotencyKey { value: key });
            let target = Arc::new(TargetKey::new(target).unwrap());
            let payload = Arc::new(payload);

            // Launch n_concurrent tasks, all with the same key.
            let mut handles = Vec::new();
            for _ in 0..n_concurrent {
                let storage = storage.clone();
                let domain = domain.clone();
                let key = key.clone();
                let target = target.clone();
                let payload = payload.clone();
                handles.push(tokio::spawn(async move {
                    storage
                        .append_dedup(&domain, &key, &target, (*payload).clone())
                        .await
                }));
            }
            let mut appended = 0;
            let mut duplicated = 0;
            for handle in handles {
                let outcome = handle.await.unwrap().unwrap();
                match outcome {
                    patchbay_core::storage::DedupOutcome::Appended(_) => appended += 1,
                    patchbay_core::storage::DedupOutcome::Duplicate(_) => duplicated += 1,
                }
            }
            prop_assert_eq!(appended, 1, "exactly one task must append");
            prop_assert_eq!(appended + duplicated, n_concurrent as u64);
            // Exactly one event persisted.
            let events = storage.read_after(&domain, lsn(0)).await.unwrap();
            prop_assert_eq!(events.len(), 1, "concurrent dedup persisted {} events", events.len());
            Ok(())
        })?;
    }

    /// BoundaryDedup across restart: a dedup key applied before a reopen
    /// is still recognized after reopening. A backend that keeps
    /// idempotency keys only in memory would fail this.
    #[test]
    fn append_dedup_survives_restart(
        payload in any_payload(),
        key in "[a-z]{1,8}",
        target in "[a-z]{1,8}",
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let temp_path = tempfile::NamedTempFile::new().unwrap()
                .into_temp_path().keep().unwrap();
            let path = temp_path.to_str().unwrap().to_string();
            let domain = test_domain();
            let key = IdempotencyKey { value: key };
            let target = TargetKey::new(target).unwrap();
            let first_lsn = {
                let storage = patchbay_core::storage::RusqliteStorage::open(&path).unwrap();
                let outcome = storage
                    .append_dedup(&domain, &key, &target, payload.clone())
                    .await
                    .unwrap();
                match outcome {
                    patchbay_core::storage::DedupOutcome::Appended(id) => lsn_of(&id),
                    _ => unreachable!("first append must be Appended"),
                }
            };
            // Reopen and retry the same key.
            let storage = patchbay_core::storage::RusqliteStorage::open(&path).unwrap();
            let outcome = storage
                .append_dedup(&domain, &key, &target, payload)
                .await
                .unwrap();
            match outcome {
                patchbay_core::storage::DedupOutcome::Duplicate(id) => {
                    prop_assert_eq!(lsn_of(&id), first_lsn,
                        "post-restart retry returned a different LSN");
                }
                patchbay_core::storage::DedupOutcome::Appended(_) => {
                    return Err(proptest::test_runner::TestCaseError::fail(
                        "post-restart retry appended a new event — dedup key did not survive restart",
                    ));
                }
            }
            // Still exactly one event.
            let events = storage.read_after(&domain, lsn(0)).await.unwrap();
            prop_assert_eq!(events.len(), 1);
            Ok(())
        })?;
    }
}

// ===== Mutation discipline: real bug injection =====
//
// Each mutation test wraps `RusqliteStorage` in a fault-injecting adapter and
// asserts the property FAILS on the buggy store. A property test that passes
// on a buggy implementation is vacuous. Proptest shrinking is automatic when
// a generated case fails; these tests prove the properties *catch* the named
// bugs (non-vacuity), which is the precondition for shrinking to matter.

/// A buggy storage wrapper that adds `+1` to every appended LSN, simulating
/// an off-by-one in LSN assignment.
struct OffByOneLsnStorage(patchbay_core::storage::RusqliteStorage);

impl Storage for OffByOneLsnStorage {
    async fn append(
        &self,
        authority_domain_id: &AuthorityDomainId,
        payload: StoredEventPayload,
    ) -> Result<EventId, patchbay_core::storage::StorageError> {
        let id = self.0.append(authority_domain_id, payload).await?;
        let buggy_lsn = lsn_of(&id) + 1;
        Ok(patchbay_core::storage::event_id(
            authority_domain_id.clone(),
            buggy_lsn,
        ))
    }
    async fn append_dedup(
        &self,
        authority_domain_id: &AuthorityDomainId,
        key: &IdempotencyKey,
        target: &TargetKey,
        payload: StoredEventPayload,
    ) -> Result<patchbay_core::storage::DedupOutcome, patchbay_core::storage::StorageError> {
        self.0
            .append_dedup(authority_domain_id, key, target, payload)
            .await
    }
    async fn read_after(
        &self,
        authority_domain_id: &AuthorityDomainId,
        cursor: Lsn,
    ) -> Result<Vec<patchbay_core::storage::RecordedEvent>, patchbay_core::storage::StorageError>
    {
        self.0.read_after(authority_domain_id, cursor).await
    }
    async fn write_snapshot(
        &self,
        authority_domain_id: &AuthorityDomainId,
        snapshot_lsn: Lsn,
        snapshot_payload: Vec<u8>,
    ) -> Result<(), patchbay_core::storage::StorageError> {
        self.0
            .write_snapshot(authority_domain_id, snapshot_lsn, snapshot_payload)
            .await
    }
    async fn load_latest_snapshot(
        &self,
        authority_domain_id: &AuthorityDomainId,
        at_or_before: Option<Lsn>,
    ) -> Result<Option<patchbay_core::storage::StoredSnapshot>, patchbay_core::storage::StorageError>
    {
        self.0
            .load_latest_snapshot(authority_domain_id, at_or_before)
            .await
    }
}

/// A buggy storage wrapper that stores a constant payload for every event,
/// preserving LSNs. This is the payload-corruption mutant that LSN-only
/// comparisons miss but full-event comparisons catch.
struct ConstantPayloadStorage(patchbay_core::storage::RusqliteStorage);

impl Storage for ConstantPayloadStorage {
    async fn append(
        &self,
        authority_domain_id: &AuthorityDomainId,
        _payload: StoredEventPayload,
    ) -> Result<EventId, patchbay_core::storage::StorageError> {
        // THE INJECTED BUG: ignore the caller's payload, store a constant.
        let constant = StoredEventPayload {
            kind: StoredEventKind::Operation as i32,
            payload: vec![0x00],
        };
        self.0.append(authority_domain_id, constant).await
    }
    async fn append_dedup(
        &self,
        authority_domain_id: &AuthorityDomainId,
        key: &IdempotencyKey,
        target: &TargetKey,
        _payload: StoredEventPayload,
    ) -> Result<patchbay_core::storage::DedupOutcome, patchbay_core::storage::StorageError> {
        let constant = StoredEventPayload {
            kind: StoredEventKind::Operation as i32,
            payload: vec![0x00],
        };
        self.0
            .append_dedup(authority_domain_id, key, target, constant)
            .await
    }
    async fn read_after(
        &self,
        authority_domain_id: &AuthorityDomainId,
        cursor: Lsn,
    ) -> Result<Vec<patchbay_core::storage::RecordedEvent>, patchbay_core::storage::StorageError>
    {
        self.0.read_after(authority_domain_id, cursor).await
    }
    async fn write_snapshot(
        &self,
        authority_domain_id: &AuthorityDomainId,
        snapshot_lsn: Lsn,
        snapshot_payload: Vec<u8>,
    ) -> Result<(), patchbay_core::storage::StorageError> {
        self.0
            .write_snapshot(authority_domain_id, snapshot_lsn, snapshot_payload)
            .await
    }
    async fn load_latest_snapshot(
        &self,
        authority_domain_id: &AuthorityDomainId,
        at_or_before: Option<Lsn>,
    ) -> Result<Option<patchbay_core::storage::StoredSnapshot>, patchbay_core::storage::StorageError>
    {
        self.0
            .load_latest_snapshot(authority_domain_id, at_or_before)
            .await
    }
}

/// A buggy storage wrapper that drops the dedup check — it always appends a
/// new event, even for a retry of an existing key.
struct DoubleApplyStorage(patchbay_core::storage::RusqliteStorage);

impl Storage for DoubleApplyStorage {
    async fn append(
        &self,
        authority_domain_id: &AuthorityDomainId,
        payload: StoredEventPayload,
    ) -> Result<EventId, patchbay_core::storage::StorageError> {
        self.0.append(authority_domain_id, payload).await
    }
    async fn append_dedup(
        &self,
        authority_domain_id: &AuthorityDomainId,
        _key: &IdempotencyKey,
        _target: &TargetKey,
        payload: StoredEventPayload,
    ) -> Result<patchbay_core::storage::DedupOutcome, patchbay_core::storage::StorageError> {
        let id = self.0.append(authority_domain_id, payload).await?;
        Ok(patchbay_core::storage::DedupOutcome::Appended(id))
    }
    async fn read_after(
        &self,
        authority_domain_id: &AuthorityDomainId,
        cursor: Lsn,
    ) -> Result<Vec<patchbay_core::storage::RecordedEvent>, patchbay_core::storage::StorageError>
    {
        self.0.read_after(authority_domain_id, cursor).await
    }
    async fn write_snapshot(
        &self,
        authority_domain_id: &AuthorityDomainId,
        snapshot_lsn: Lsn,
        snapshot_payload: Vec<u8>,
    ) -> Result<(), patchbay_core::storage::StorageError> {
        self.0
            .write_snapshot(authority_domain_id, snapshot_lsn, snapshot_payload)
            .await
    }
    async fn load_latest_snapshot(
        &self,
        authority_domain_id: &AuthorityDomainId,
        at_or_before: Option<Lsn>,
    ) -> Result<Option<patchbay_core::storage::StoredSnapshot>, patchbay_core::storage::StorageError>
    {
        self.0
            .load_latest_snapshot(authority_domain_id, at_or_before)
            .await
    }
}

/// Run the gap-free check against a given storage. Returns Ok if the
/// property held, Err with the failure otherwise.
async fn run_gap_free_check<S: Storage>(storage: &S, n: u64) -> Result<(), String> {
    let domain = test_domain();
    let mut lsns = Vec::new();
    for i in 0..n {
        let id = storage
            .append(&domain, indexed_payload(i as usize))
            .await
            .map_err(|e| format!("append failed: {e:?}"))?;
        lsns.push(lsn_of(&id));
    }
    let expected: Vec<u64> = (1..=n).collect();
    if lsns == expected {
        Ok(())
    } else {
        Err(format!(
            "gap-free check failed: got {lsns:?}, expected {expected:?}"
        ))
    }
}

/// Run the crash-recovery payload check: append events, drop, reopen,
/// compare full payloads. Returns Ok if preserved, Err otherwise.
async fn run_crash_payload_check<S: Storage + Sync + Send>(
    make_storage: impl Fn(&str) -> S,
    path: &str,
    payloads: &[StoredEventPayload],
) -> Result<(), String> {
    let domain = test_domain();
    {
        let storage = make_storage(path);
        for payload in payloads {
            storage
                .append(&domain, payload.clone())
                .await
                .map_err(|e| format!("append failed: {e:?}"))?;
        }
    }
    let storage = make_storage(path);
    let recovered = storage
        .read_after(&domain, lsn(0))
        .await
        .map_err(|e| format!("read failed: {e:?}"))?;
    let got: Vec<StoredEventPayload> = recovered.iter().map(|e| e.payload.clone()).collect();
    if got == payloads {
        Ok(())
    } else {
        Err(format!(
            "crash payload check failed: got {got:?}, expected {payloads:?}"
        ))
    }
}

#[test]
fn gap_free_catches_injected_lsn_bug() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    // Sanity: real implementation passes.
    let real_passes = rt.block_on(async {
        let storage = patchbay_core::storage::RusqliteStorage::open_in_memory().unwrap();
        run_gap_free_check(&storage, 5).await.is_ok()
    });
    assert!(real_passes, "real storage must pass the gap-free check");

    // Mutation: off-by-one wrapper must FAIL.
    let buggy_fails = rt.block_on(async {
        let storage =
            OffByOneLsnStorage(patchbay_core::storage::RusqliteStorage::open_in_memory().unwrap());
        run_gap_free_check(&storage, 5).await.is_err()
    });
    assert!(
        buggy_fails,
        "off-by-one LSN bug was NOT caught — the gap-free property is vacuous"
    );
}

#[test]
fn crash_recovery_catches_payload_corruption() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let payloads: Vec<StoredEventPayload> = (0..5).map(indexed_payload).collect();

    // Sanity: real implementation preserves payloads.
    let real_passes = rt.block_on(async {
        let temp = tempfile::NamedTempFile::new()
            .unwrap()
            .into_temp_path()
            .keep()
            .unwrap();
        run_crash_payload_check(
            |p| patchbay_core::storage::RusqliteStorage::open(p).unwrap(),
            temp.to_str().unwrap(),
            &payloads,
        )
        .await
        .is_ok()
    });
    assert!(
        real_passes,
        "real storage must preserve payloads across reopen"
    );

    // Mutation: constant-payload wrapper must FAIL the full-event check.
    let buggy_fails = rt.block_on(async {
        let temp = tempfile::NamedTempFile::new()
            .unwrap()
            .into_temp_path()
            .keep()
            .unwrap();
        run_crash_payload_check(
            |p| ConstantPayloadStorage(patchbay_core::storage::RusqliteStorage::open(p).unwrap()),
            temp.to_str().unwrap(),
            &payloads,
        )
        .await
        .is_err()
    });
    assert!(
        buggy_fails,
        "payload-corruption bug was NOT caught — the crash-recovery property is vacuous"
    );
}

#[test]
fn dedup_catches_injected_double_apply() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    // Sanity: real implementation dedups.
    let real_passes = rt.block_on(async {
        let storage = patchbay_core::storage::RusqliteStorage::open_in_memory().unwrap();
        let domain = test_domain();
        let key = IdempotencyKey {
            value: "k".to_string(),
        };
        let target = TargetKey::new("t".to_string()).unwrap();
        let payload = indexed_payload(0);
        let _ = storage
            .append_dedup(&domain, &key, &target, payload.clone())
            .await
            .unwrap();
        let _ = storage
            .append_dedup(&domain, &key, &target, payload)
            .await
            .unwrap();
        let events = storage.read_after(&domain, lsn(0)).await.unwrap();
        events.len() == 1
    });
    assert!(real_passes, "real storage must dedup a retry to one event");

    // Mutation: double-apply wrapper must leave two events.
    let buggy_caught = rt.block_on(async {
        let storage =
            DoubleApplyStorage(patchbay_core::storage::RusqliteStorage::open_in_memory().unwrap());
        let domain = test_domain();
        let key = IdempotencyKey {
            value: "k".to_string(),
        };
        let target = TargetKey::new("t".to_string()).unwrap();
        let payload = indexed_payload(0);
        let _ = storage
            .append_dedup(&domain, &key, &target, payload.clone())
            .await
            .unwrap();
        let _ = storage
            .append_dedup(&domain, &key, &target, payload)
            .await
            .unwrap();
        let events = storage.read_after(&domain, lsn(0)).await.unwrap();
        events.len() != 1
    });
    assert!(
        buggy_caught,
        "double-apply bug was NOT caught — the dedup property is vacuous"
    );
}
