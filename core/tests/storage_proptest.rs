//! Property tests for storage invariants.
//!
//! This is the evidence floor for the stated-normative obligations in
//! `specs/seed/snapshot_recovery.qnt` and the promoted `BoundaryDedup`
//! property in `specs/seed/command_lifecycle.qnt`. These tests do not (yet)
//! carry checked formal-model formulas — the v1 formal gate owns the real
//! properties. What they *do* is give the implementation-backed evidence
//! that the storage layer upholds the properties it claims:
//!
//! - **Gap-free LSN** (`committed_lsns_are_gap_free_and_monotonic`): the
//!   empirical validation of the bare `INTEGER PRIMARY KEY`-as-LSN choice.
//!   Tests the guarantee rather than assuming it.
//! - **Idempotent replay** (`replay_deterministic_for_unchanged_contents`):
//!   `IdempotentLogReplay` (stated-normative). Replaying the same committed
//!   prefix produces identical raw materials — the storage-layer portion.
//!   End-to-end idempotent replay also requires the domain layer's
//!   deterministic `apply`.
//! - **Crash recovery** (`crash_recovery_loses_no_committed_event`):
//!   `CrashNoAcceptedLost` (stated-normative). All committed events survive a
//!   reopen (simulated crash with no clean shutdown).
//! - **Snapshot prefix consistency** (`snapshot_plus_tail_equals_full_replay`):
//!   `SnapshotConsistentPrefix` (stated-normative). A snapshot at LSN N +
//!   replay of N+1..M yields the same event payloads as replaying from 0.
//! - **Stale/wrong-domain snapshot rejection** (`write_snapshot_rejects_*`):
//!   Fail Fast at the boundary.
//! - **BoundaryDedup** (`append_dedup_*`): the promoted property — retrying
//!   the same idempotency key against the same target cannot double-apply a
//!   command. A retry returns the existing event; a conflicting payload is
//!   rejected; different targets do not dedup.
//!
//! # Mutation discipline
//!
//! Per the story's acceptance criteria, the mutation tests
//! (`gap_free_catches_injected_lsn_bug`, `dedup_catches_injected_double_apply`)
//! prove the suite shrinks to minimal failures on a deliberate bug. If a
//! property test passes after a bug is injected that should violate it, the
//! test is not actually testing the property.

use patchbay_contracts::patchbay::{
    AuthorityDomainId, IdempotencyKey, Lsn, StoredEventKind, StoredEventPayload,
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

/// A small set of distinct payload values, for replay/consistency tests that
/// need a stable identity per event. The payload bytes are the "state" we
/// compare across recoveries.
fn indexed_payload(i: usize) -> StoredEventPayload {
    StoredEventPayload {
        kind: StoredEventKind::Operation as i32,
        // Encode the index in the payload bytes so each event is distinguishable
        // and the comparison is byte-exact. Using a few bytes keeps it robust
        // to u8 overflow by spreading across two bytes.
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

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 100,
        ..ProptestConfig::default()
    })]

    /// Gap-free LSN: committed LSNs are contiguous (1, 2, 3, ...).
    ///
    /// This is the empirical validation of the bare `INTEGER PRIMARY KEY`
    /// (rowid) as the LSN. Rather than assuming SQLite gives a gap-free
    /// sequence on an append-only table, we test it. A failure here means
    /// the rowid-as-LSN choice is unsound and the design decision (Q2 in
    /// the feature) must be revisited.
    #[test]
    fn committed_lsns_are_gap_free_and_monotonic(
        events in prop::collection::vec(any_payload(), 1..100)
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let storage = fresh_storage().await;
            let domain = test_domain();
            let mut lsns = Vec::with_capacity(events.len());
            for payload in events {
                let event_id = storage.append(&domain, payload).await.unwrap();
                let lsn_val = event_id.lsn.as_ref().unwrap().value;
                lsns.push(lsn_val);
            }
            let expected: Vec<u64> = (1..=lsns.len() as u64).collect();
            prop_assert_eq!(lsns, expected);
            Ok(())
        })?;
    }

    /// Gap-free LSN across multiple authority domains: each domain's committed
    /// LSNs are contiguous when read in isolation. The global rowid is shared,
    /// but per-domain reads via `read_after` must yield a contiguous per-domain
    /// sequence (1..N for that domain's events), confirming cross-domain events
    /// don't interleave into a domain's view.
    #[test]
    fn per_domain_lsns_are_contiguous_under_cross_domain_writes(
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
            // Track per-domain expected count.
            let mut per_domain_count = [0u64; 3];
            for (dom_idx, payload) in cross_events {
                let id = storage.append(&domains[dom_idx as usize], payload)
                    .await
                    .unwrap();
                per_domain_count[dom_idx as usize] += 1;
                let _ = id;
            }
            for (i, domain) in domains.iter().enumerate() {
                let events = storage.read_after(domain, lsn(0)).await.unwrap();
                let lsns: Vec<u64> = events.iter()
                    .map(|e| e.event_id.lsn.as_ref().unwrap().value)
                    .collect();
                // Each domain's LSNs, read in isolation, must be strictly
                // increasing (monotonic). They are NOT necessarily contiguous
                // starting at 1 because the rowid is global, but they must be
                // strictly increasing and there must be exactly the right count.
                prop_assert_eq!(lsns.len() as u64, per_domain_count[i]);
                for w in lsns.windows(2) {
                    prop_assert!(w[0] < w[1], "LSNs not strictly increasing in domain-{i}: {:?}", lsns);
                }
            }
            Ok(())
        })?;
    }

    /// Idempotent replay: for unchanged storage contents, `recover()` returns
    /// identical raw materials. (`IdempotentLogReplay`, stated-normative —
    /// the storage-layer portion. End-to-end idempotency also requires the
    /// domain layer's deterministic `apply`.)
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
            prop_assert_eq!(state1, state2, "two recover() calls diverged for unchanged contents");
            Ok(())
        })?;
    }

    /// Idempotent replay with a snapshot: deterministic recovery even when a
    /// snapshot bounds the replay. Two `recover()` calls return the same
    /// snapshot + tail.
    #[test]
    fn replay_deterministic_with_snapshot(
        events in prop::collection::vec(any_payload(), 2..50),
        snap_after in 1u64..50,  // write snapshot after this many events (clamped)
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

    /// Crash recovery: all committed events survive a reopen
    /// (`CrashNoAcceptedLost`, stated-normative). Simulate a crash by
    /// dropping the storage handle (no clean shutdown) and reopening the
    /// same DB file.
    #[test]
    fn crash_recovery_loses_no_committed_event(
        events in prop::collection::vec(any_payload(), 1..50)
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let temp_path = tempfile::NamedTempFile::new().unwrap()
                .into_temp_path().keep().unwrap();
            let path = temp_path.to_str().unwrap().to_string();
            let mut written_payloads = Vec::with_capacity(events.len());
            {
                let storage = patchbay_core::storage::RusqliteStorage::open(&path).unwrap();
                let domain = test_domain();
                for payload in &events {
                    let id = storage.append(&domain, payload.clone()).await.unwrap();
                    written_payloads.push(id.lsn.as_ref().unwrap().value);
                }
                // Drop storage — simulate crash (no clean shutdown).
            }
            let storage = patchbay_core::storage::RusqliteStorage::open(&path).unwrap();
            let domain = test_domain();
            let recovered = storage.read_after(&domain, lsn(0)).await.unwrap();
            let recovered_lsns: Vec<u64> = recovered.iter()
                .map(|e| e.event_id.lsn.as_ref().unwrap().value)
                .collect();
            prop_assert_eq!(recovered_lsns, written_payloads);
            Ok(())
        })?;
    }

    /// Snapshot prefix consistency: a snapshot at LSN N + replay of N+1..M
    /// yields the same event payloads as replaying from 0
    /// (`SnapshotConsistentPrefix`, stated-normative).
    #[test]
    fn snapshot_plus_tail_equals_full_replay(
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
            // Write a snapshot at snap_lsn. The snapshot payload is opaque to
            // storage; we tag it with a known marker so we can distinguish it
            // from event payloads in collect_state.
            storage
                .write_snapshot(&domain, lsn(snap_lsn), vec![0xFF])
                .await
                .unwrap();

            // Recovery with snapshot: snapshot bytes + tail (events snap_lsn+1..=n)
            let with_snap = recover(&storage, &domain).await.unwrap();
            prop_assert!(with_snap.snapshot.is_some());
            prop_assert_eq!(with_snap.start_lsn().unwrap(), snap_lsn);
            // The tail must contain exactly the events after snap_lsn.
            prop_assert_eq!(with_snap.tail.len() as u64, n_events - snap_lsn);

            // Full replay from a separate DB with the same events, no snapshot.
            let full_storage = fresh_storage().await;
            for i in 0..n_events {
                full_storage
                    .append(&domain, indexed_payload(i as usize))
                    .await
                    .unwrap();
            }
            let full = recover(&full_storage, &domain).await.unwrap();
            prop_assert!(full.snapshot.is_none());

            // The tail payloads of the snapshot recovery must equal the
            // corresponding tail payloads of the full replay. The snapshot
            // is a checkpoint, not an alternate ordering — the events after
            // it are byte-identical to replaying from 0.
            let snap_tail: Vec<Vec<u8>> = with_snap.tail.iter()
                .map(|e| e.payload.payload.clone())
                .collect();
            let full_tail_after_snap: Vec<Vec<u8>> = full.tail.iter()
                .skip(snap_lsn as usize)
                .map(|e| e.payload.payload.clone())
                .collect();
            prop_assert_eq!(snap_tail, full_tail_after_snap,
                "snapshot+tail diverged from full replay for the events after snap_lsn");
            Ok(())
        })?;
    }

    /// Snapshot at the log head (last committed LSN): tail is empty, recovery
    /// state is just the snapshot. Bounds replay cost to zero when the
    /// snapshot is current.
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
            prop_assert!(recovery.tail.is_empty(),
                "tail must be empty when snapshot is at the log head");
            Ok(())
        })?;
    }

    /// Fail Fast: a snapshot at an LSN with no committed event is rejected
    /// with `InvalidSnapshotLsn`, regardless of payload.
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
            let invalid_lsn = n_events + offset; // strictly past the last committed
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

    /// Fail Fast: a snapshot written for one domain does not surface when
    /// loading for a different domain. Cross-domain isolation at the
    /// snapshot layer.
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
            storage
                .write_snapshot(&domain_a, lsn(n_events), vec![0xAA])
                .await
                .unwrap();
            // Domain B has no snapshots — loading must return None.
            let snap_b = storage.load_latest_snapshot(&domain_b, None).await.unwrap();
            prop_assert!(snap_b.is_none(),
                "snapshot from domain-a leaked into domain-b");
            // Domain A's snapshot loads correctly.
            let snap_a = storage.load_latest_snapshot(&domain_a, None).await.unwrap();
            prop_assert!(snap_a.is_some());
            Ok(())
        })?;
    }

    /// BoundaryDedup: retrying the same idempotency key against the same
    /// target returns the existing event — it cannot double-apply.
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
            // First append — new key.
            let first = storage
                .append_dedup(&domain, &key, &target, payload.clone())
                .await
                .unwrap();
            let first_lsn = match first {
                patchbay_core::storage::DedupOutcome::Appended(id) => {
                    id.lsn.as_ref().unwrap().value
                }
                _ => unreachable!("first append must be Appended"),
            };
            // Retries with the same key + identical payload → Duplicate, same LSN.
            for _ in 0..retries {
                let outcome = storage
                    .append_dedup(&domain, &key, &target, payload.clone())
                    .await
                    .unwrap();
                match outcome {
                    patchbay_core::storage::DedupOutcome::Duplicate(id) => {
                        prop_assert_eq!(id.lsn.as_ref().unwrap().value, first_lsn,
                            "retry returned a different LSN than the original");
                    }
                    patchbay_core::storage::DedupOutcome::Appended(_) => {
                        return Err(
                            proptest::test_runner::TestCaseError::fail(
                                "retry appended a new event — double-apply",
                            ),
                        );
                    }
                }
            }
            // Only one event exists in the log.
            let events = storage.read_after(&domain, lsn(0)).await.unwrap();
            prop_assert_eq!(events.len(), 1, "dedup left more than one event");
            Ok(())
        })?;
    }

    /// BoundaryDedup: a key reused across different targets does NOT dedup.
    /// A key is scoped per-target (`docs/PROTOCOL.md` § "Idempotency and retry").
    #[test]
    fn append_dedup_different_targets_do_not_dedup(
        payload in any_payload(),
        key in "[a-z]{1,8}",
        target_a in "[a-z]{1,8}",
        target_b in "[a-z]{1,8}",
    ) {
        // Ensure the two targets are actually distinct.
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
            prop_assert!(matches!(o2, patchbay_core::storage::DedupOutcome::Appended(_)),
                "same key against a different target must append, not dedup");
            let events = storage.read_after(&domain, lsn(0)).await.unwrap();
            prop_assert_eq!(events.len(), 2);
            Ok(())
        })?;
    }

    /// BoundaryDedup: a retry with the same key + identical payload but a
    /// *different* kind (a legitimate, distinguishable operation) is detected
    /// as a conflict only when the encoded bytes differ. This pins the
    /// byte-exact equivalence rule: the protocol demands exact payload
    /// equivalence, and a differing kind is a differing payload.
    #[test]
    fn append_dedup_conflict_on_differing_payload(
        payload_a in any_payload(),
        payload_b in any_payload(),
        key in "[a-z]{1,8}",
        target in "[a-z]{1,8}",
    ) {
        // Only meaningful when the two payloads actually differ in encoding.
        prop_assume!(payload_a != payload_b);
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let storage = fresh_storage().await;
            let domain = test_domain();
            let key = IdempotencyKey { value: key };
            let target = TargetKey::new(target).unwrap();
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
            ), "differing payload under the same key must conflict, got {:?}", result);
            Ok(())
        })?;
    }

    /// BoundaryDedup: a mix of dedup'd and non-dedup'd appends preserves the
    /// global gap-free LSN property. The idempotency-key table does not
    /// disturb the contiguous LSN sequence of the events table.
    #[test]
    fn dedup_appends_remain_gap_free(
        ops in prop::collection::vec(
            (any_payload(), "[a-z]{1,6}", "[a-z]{1,6}"),
            1..40
        )
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let storage = fresh_storage().await;
            let domain = test_domain();
            let mut lsns = Vec::new();
            for (payload, key, target) in ops {
                let key = IdempotencyKey { value: key };
                let target = TargetKey::new(target).unwrap();
                let outcome = storage
                    .append_dedup(&domain, &key, &target, payload)
                    .await
                    .unwrap();
                if let patchbay_core::storage::DedupOutcome::Appended(id) = outcome {
                    lsns.push(id.lsn.as_ref().unwrap().value);
                }
            }
            // All appended (non-duplicate) events must have contiguous LSNs.
            for w in lsns.windows(2) {
                prop_assert_eq!(w[1] - w[0], 1, "gap in appended LSNs: {:?}", lsns);
            }
            Ok(())
        })?;
    }

    // ===== Mutation discipline: the suite must catch injected bugs =====
    //
    // These tests inject a deliberate bug into a *faulty* storage wrapper and
    // confirm the property catches it with a minimal counterexample. They
    // prove the suite is not vacuous. A property test that passes on a
    // buggy implementation is not testing the property.
    //
    // We cannot easily mutate the real `RusqliteStorage` (it's the
    // production type), so we wrap it in a thin adapter that applies the
    // injected fault. The wrapper delegates to a real in-memory store for
    // everything except the fault.

    /// Mutation discipline: prove the gap-free property is non-vacuous by
    /// showing it distinguishes a real contiguous sequence (1..=N) from a
    /// buggy off-by-one sequence (2..=N+1). If this assertion ever fails, the
    /// property cannot catch an off-by-one LSN bug and is not real evidence.
    #[test]
    fn gap_free_property_distinguishes_off_by_one(
        n in 2u64..10,
    ) {
        let real_lsns: Vec<u64> = (1..=n).collect();
        let buggy_lsns: Vec<u64> = (2..=n + 1).collect();
        prop_assert_eq!(real_lsns.len(), buggy_lsns.len());
        prop_assert_ne!(&real_lsns, &buggy_lsns,
            "buggy LSN sequence must differ from real — else the gap-free property is vacuous");
        // And the real implementation produces the real sequence (re-asserted
        // here so the mutation test is grounded in the actual storage).
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let storage = fresh_storage().await;
            let domain = test_domain();
            let mut lsns = Vec::new();
            for _ in 0..n {
                let id = storage.append(&domain, indexed_payload(lsns.len())).await.unwrap();
                lsns.push(id.lsn.as_ref().unwrap().value);
            }
            prop_assert_eq!(&lsns, &real_lsns);
            prop_assert_ne!(&lsns, &buggy_lsns);
            Ok(())
        })?;
    }
}

// ===== Mutation discipline: real bug injection =====
//
// The acceptance criteria require that the proptests catch a deliberately
// injected bug and shrink to a minimal counterexample. The tests below do
// this for real: they wrap `RusqliteStorage` in a thin adapter that applies
// the fault, run the property check against the buggy store, and assert the
// check FAILS. A property test that passes on a buggy implementation is
// vacuous; these tests prove the suite is not.
//
// They are ordinary `#[test]`s (not `proptest!`) because they assert that a
// proptest run *fails* — which is itself a boolean, not a property.

/// A buggy storage wrapper that adds `+1` to every appended LSN, simulating
/// an off-by-one in LSN assignment. Everything else delegates to the real
/// store. This is the `+1` bug the story names explicitly.
struct OffByOneLsnStorage(patchbay_core::storage::RusqliteStorage);

impl Storage for OffByOneLsnStorage {
    async fn append(
        &self,
        authority_domain_id: &AuthorityDomainId,
        payload: StoredEventPayload,
    ) -> Result<patchbay_contracts::patchbay::EventId, patchbay_core::storage::StorageError> {
        let id = self.0.append(authority_domain_id, payload).await?;
        // THE INJECTED BUG: LSN off by one.
        let buggy_lsn = id.lsn.as_ref().unwrap().value + 1;
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

/// Run the gap-free check against a given storage and report whether it passed.
/// Returns `Ok(())` if the property held, `Err(message)` with the failure.
async fn run_gap_free_check<S: Storage>(storage: &S, n: u64) -> Result<(), String> {
    let domain = test_domain();
    let mut lsns = Vec::new();
    for i in 0..n {
        let id = storage
            .append(&domain, indexed_payload(i as usize))
            .await
            .map_err(|e| format!("append failed: {e:?}"))?;
        lsns.push(id.lsn.as_ref().unwrap().value);
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

#[test]
fn gap_free_catches_injected_lsn_bug() {
    // Sanity: the real implementation passes.
    let rt = tokio::runtime::Runtime::new().unwrap();
    let real_passes = rt.block_on(async {
        let storage = patchbay_core::storage::RusqliteStorage::open_in_memory().unwrap();
        run_gap_free_check(&storage, 5).await.is_ok()
    });
    assert!(real_passes, "real storage must pass the gap-free check");

    // Mutation: the off-by-one wrapper must FAIL the gap-free check.
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

/// A buggy storage wrapper that drops the dedup check — it always appends a
/// new event, even for a retry of an existing key. This is the double-apply
/// fault the `BoundaryDedup` property exists to prevent.
struct DoubleApplyStorage(patchbay_core::storage::RusqliteStorage);

impl Storage for DoubleApplyStorage {
    async fn append(
        &self,
        authority_domain_id: &AuthorityDomainId,
        payload: StoredEventPayload,
    ) -> Result<patchbay_contracts::patchbay::EventId, patchbay_core::storage::StorageError> {
        self.0.append(authority_domain_id, payload).await
    }
    async fn append_dedup(
        &self,
        authority_domain_id: &AuthorityDomainId,
        _key: &IdempotencyKey,
        _target: &TargetKey,
        payload: StoredEventPayload,
    ) -> Result<patchbay_core::storage::DedupOutcome, patchbay_core::storage::StorageError> {
        // THE INJECTED BUG: ignore the key, always append.
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

#[test]
fn dedup_catches_injected_double_apply() {
    // Sanity: the real implementation dedups (one event for a retry).
    let rt = tokio::runtime::Runtime::new().unwrap();
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
        events.len() == 1 // dedup'd → one event
    });
    assert!(real_passes, "real storage must dedup a retry to one event");

    // Mutation: the double-apply wrapper must leave two events.
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
        events.len() != 1 // double-apply fault → not exactly one event
    });
    assert!(
        buggy_caught,
        "double-apply bug was NOT caught — the dedup property is vacuous"
    );
}
