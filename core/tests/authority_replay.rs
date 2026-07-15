use patchbay_contracts::patchbay::{
    ActorId, AuthorityDomainId, EventId, Generation, Grant, GrantId, GrantProvenance,
    GrantRevocationPolicy, IdempotencyKey, Lsn, OperationKind, Revocation, StoredEventPayload,
    TargetScope, TargetScopeKind,
};
use patchbay_core::{
    authority::{
        ingest_grant, ingest_revocation, rebuild_from_log, AuthorityError, AuthorityRegistry,
    },
    storage::{
        DedupOutcome, RecordedEvent, RusqliteStorage, Storage, StorageError, StoredSnapshot,
        TargetKey,
    },
};

fn domain(value: &str) -> AuthorityDomainId {
    AuthorityDomainId {
        value: value.to_owned(),
    }
}

fn grant_id(value: &str) -> GrantId {
    GrantId {
        value: value.to_owned(),
    }
}

fn grant(id: &str, actor: &str) -> Grant {
    Grant {
        grant_id: Some(grant_id(id)),
        authority_domain_id: Some(domain("authority-main")),
        subject_actor_id: Some(ActorId {
            value: actor.to_owned(),
        }),
        target_scope: Some(TargetScope {
            kind: TargetScopeKind::FleetSupervisor as i32,
            ..TargetScope::default()
        }),
        allowed_operation_kinds: vec![OperationKind::Spawn as i32],
        provenance: Some(GrantProvenance {
            reason: "replay fixture".to_owned(),
            ..GrantProvenance::default()
        }),
        revocation_policy: GrantRevocationPolicy::Continue as i32,
        ..Grant::default()
    }
}

fn revocation(id: &str) -> Revocation {
    Revocation {
        authority_domain_id: Some(domain("authority-main")),
        grant_id: Some(grant_id(id)),
        revocation_generation: Some(Generation { value: 1 }),
        accepted_operation_policy: GrantRevocationPolicy::Cancel as i32,
        reason: "replay fixture".to_owned(),
        ..Revocation::default()
    }
}

#[tokio::test]
async fn replay_reconstructs_the_live_authority_registry() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut live = AuthorityRegistry::new();

    ingest_grant(
        &storage,
        &mut live,
        &domain("authority-main"),
        grant("grant-live", "operator"),
    )
    .await
    .unwrap();
    ingest_grant(
        &storage,
        &mut live,
        &domain("authority-main"),
        grant("grant-revoked", "automation"),
    )
    .await
    .unwrap();
    ingest_revocation(
        &storage,
        &mut live,
        &domain("authority-main"),
        revocation("grant-revoked"),
    )
    .await
    .unwrap();

    let rebuilt = rebuild_from_log(&storage, &domain("authority-main"))
        .await
        .expect("the committed authority log must replay");

    assert_eq!(rebuilt, live);
    assert_eq!(rebuilt.live_grants().count(), 1);
    assert!(rebuilt
        .get_grant(&grant_id("grant-live"))
        .expect("the live grant must be recovered")
        .is_live());
    assert!(rebuilt
        .get_grant(&grant_id("grant-revoked"))
        .expect("the revoked grant must be retained")
        .is_revoked());
}

/// A deliberately faulty storage adapter that returns one domain's records for
/// every read. Recovery must reject this port-contract violation rather than
/// accepting cross-domain state.
struct DomainMismatchingStorage {
    inner: RusqliteStorage,
    source_domain: AuthorityDomainId,
}

impl Storage for DomainMismatchingStorage {
    async fn append(
        &self,
        authority_domain_id: &AuthorityDomainId,
        payload: StoredEventPayload,
    ) -> Result<EventId, StorageError> {
        self.inner.append(authority_domain_id, payload).await
    }

    async fn append_dedup(
        &self,
        authority_domain_id: &AuthorityDomainId,
        key: &IdempotencyKey,
        target: &TargetKey,
        payload: StoredEventPayload,
    ) -> Result<DedupOutcome, StorageError> {
        self.inner
            .append_dedup(authority_domain_id, key, target, payload)
            .await
    }

    async fn read_after(
        &self,
        _authority_domain_id: &AuthorityDomainId,
        cursor: Lsn,
    ) -> Result<Vec<RecordedEvent>, StorageError> {
        self.inner.read_after(&self.source_domain, cursor).await
    }

    async fn write_snapshot(
        &self,
        authority_domain_id: &AuthorityDomainId,
        snapshot_lsn: Lsn,
        snapshot_payload: Vec<u8>,
    ) -> Result<(), StorageError> {
        self.inner
            .write_snapshot(authority_domain_id, snapshot_lsn, snapshot_payload)
            .await
    }

    async fn load_latest_snapshot(
        &self,
        authority_domain_id: &AuthorityDomainId,
        at_or_before: Option<Lsn>,
    ) -> Result<Option<StoredSnapshot>, StorageError> {
        self.inner
            .load_latest_snapshot(authority_domain_id, at_or_before)
            .await
    }
}

#[tokio::test]
async fn replay_rejects_cross_domain_events_returned_by_storage() {
    let storage = DomainMismatchingStorage {
        inner: RusqliteStorage::open_in_memory().unwrap(),
        source_domain: domain("authority-main"),
    };
    let mut live = AuthorityRegistry::new();
    ingest_grant(
        &storage,
        &mut live,
        &domain("authority-main"),
        grant("grant-1", "operator"),
    )
    .await
    .unwrap();

    assert!(matches!(
        rebuild_from_log(&storage, &domain("authority-other")).await,
        Err(AuthorityError::CorruptLog(message))
            if message.contains("belongs to authority domain")
    ));
}
