use patchbay_contracts::patchbay::{
    ActorId, AdapterId, AuthorityDomainId, DeviceId, EndpointId, EventId, Generation, Grant,
    GrantId, GrantProvenance, GrantRevocationPolicy, IdempotencyKey, Lsn, OperationKind,
    Revocation, StoredEventPayload, TargetScope, TargetScopeKind,
};
use patchbay_core::{
    acceptance::{Authorized, GrantCheck, GrantDenied},
    authority::{
        grant_matches_request, ingest_grant, ingest_revocation, rebuild_from_log, AuthorityError,
        AuthorityRegistry, GrantLiveness, IssuerContext, IssuerRef,
    },
    storage::{
        AuditRecordDraft, DedupOutcome, GrantAppendOutcome, GrantIdentityKey, RecordedEvent,
        RusqliteStorage, Storage, StorageError, StoredSnapshot, TargetKey,
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

fn overlapping_grant(id: &str, target_scope: TargetScope) -> Grant {
    Grant {
        grant_id: Some(grant_id(id)),
        authority_domain_id: Some(domain("authority-main")),
        subject_actor_id: Some(ActorId {
            value: "operator".to_owned(),
        }),
        target_scope: Some(target_scope),
        allowed_operation_kinds: vec![OperationKind::Instruct as i32],
        provenance: Some(GrantProvenance {
            reason: "overlapping replay fixture".to_owned(),
            ..GrantProvenance::default()
        }),
        revocation_policy: GrantRevocationPolicy::Continue as i32,
        ..Grant::default()
    }
}

fn adapter_target() -> TargetScope {
    TargetScope {
        kind: TargetScopeKind::Adapter as i32,
        adapter_id: Some(AdapterId {
            value: "pi".to_owned(),
        }),
        ..TargetScope::default()
    }
}

fn timestamp(seconds: i64) -> prost_types::Timestamp {
    prost_types::Timestamp { seconds, nanos: 0 }
}

async fn ingest_adapter_grant(
    storage: &RusqliteStorage,
    registry: &mut AuthorityRegistry,
    id: &str,
    expires_at: Option<prost_types::Timestamp>,
) {
    let mut grant = overlapping_grant(id, adapter_target());
    grant.expires_at = expires_at;
    ingest_grant(storage, registry, &domain("authority-main"), grant)
        .await
        .expect("the matching grant fixture must be ingested");
}

struct VerifiedIssuer {
    actor: ActorId,
    authority_domain_id: AuthorityDomainId,
}

impl VerifiedIssuer {
    fn operator() -> Self {
        Self {
            actor: ActorId {
                value: "operator".to_owned(),
            },
            authority_domain_id: domain("authority-main"),
        }
    }
}

impl IssuerContext for VerifiedIssuer {
    fn verified_actor(&self) -> Option<&ActorId> {
        Some(&self.actor)
    }

    fn verified_endpoint(&self) -> Option<&EndpointId> {
        None
    }

    fn verified_device(&self) -> Option<&DeviceId> {
        None
    }

    fn endpoint_generation(&self) -> Option<Generation> {
        None
    }

    fn authority_domain_id(&self) -> &AuthorityDomainId {
        &self.authority_domain_id
    }
}

async fn grant_decision_at(
    registry: &AuthorityRegistry,
    issuer: &dyn IssuerContext,
    target: &TargetScope,
    evaluated_at: &prost_types::Timestamp,
) -> Result<Authorized, GrantDenied> {
    registry
        .check_at(
            &domain("authority-main"),
            issuer,
            OperationKind::Instruct,
            target,
            evaluated_at,
        )
        .await
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

fn revocation_at(id: &str, seconds: i64) -> Revocation {
    Revocation {
        revoked_at: Some(timestamp(seconds)),
        ..revocation(id)
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

#[tokio::test]
async fn overlapping_grants_select_the_same_lowest_id_before_and_after_replay() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut live = AuthorityRegistry::new();
    let request_target = TargetScope {
        kind: TargetScopeKind::Adapter as i32,
        adapter_id: Some(AdapterId {
            value: "pi".to_owned(),
        }),
        ..TargetScope::default()
    };

    // Ingest the narrower, higher id first so neither creation order nor
    // scope specificity can explain the selected provenance.
    ingest_grant(
        &storage,
        &mut live,
        &domain("authority-main"),
        overlapping_grant(
            "grant-z-adapter",
            TargetScope {
                kind: TargetScopeKind::Adapter as i32,
                adapter_id: Some(AdapterId {
                    value: "pi".to_owned(),
                }),
                ..TargetScope::default()
            },
        ),
    )
    .await
    .unwrap();
    ingest_grant(
        &storage,
        &mut live,
        &domain("authority-main"),
        overlapping_grant(
            "grant-a-domain",
            TargetScope {
                kind: TargetScopeKind::AuthorityDomain as i32,
                ..TargetScope::default()
            },
        ),
    )
    .await
    .unwrap();

    let issuer = VerifiedIssuer::operator();
    let issuer_ref = IssuerRef {
        actor: &issuer.actor,
        endpoint: None,
        authority_domain_id: &issuer.authority_domain_id,
    };
    for id in ["grant-z-adapter", "grant-a-domain"] {
        let candidate = live
            .get_grant(&grant_id(id))
            .expect("the overlapping grant must be projected");
        assert!(
            grant_matches_request(
                candidate,
                &issuer_ref,
                OperationKind::Instruct,
                &request_target,
            ),
            "{id} must independently match the request",
        );
    }

    let evaluated_at = timestamp(100);
    let expected = Ok(Authorized {
        grant_id: Some(grant_id("grant-a-domain")),
    });
    assert_eq!(
        grant_decision_at(&live, &issuer, &request_target, &evaluated_at).await,
        expected.clone(),
    );

    let rebuilt = rebuild_from_log(&storage, &domain("authority-main"))
        .await
        .expect("the committed overlapping grants must replay");
    assert_eq!(rebuilt, live);
    assert_eq!(
        grant_decision_at(&rebuilt, &issuer, &request_target, &evaluated_at).await,
        expected,
    );
}

#[tokio::test]
async fn lower_id_expired_and_revoked_grants_cannot_defeat_a_live_grant_after_replay() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut live = AuthorityRegistry::new();
    let evaluated_at = timestamp(100);

    ingest_adapter_grant(&storage, &mut live, "grant-a-expired", Some(evaluated_at)).await;
    ingest_adapter_grant(&storage, &mut live, "grant-b-revoked", None).await;
    ingest_revocation(
        &storage,
        &mut live,
        &domain("authority-main"),
        revocation_at("grant-b-revoked", 99),
    )
    .await
    .unwrap();
    ingest_adapter_grant(&storage, &mut live, "grant-z-live", None).await;

    for (id, expected_liveness) in [
        ("grant-a-expired", GrantLiveness::Expired),
        ("grant-b-revoked", GrantLiveness::Revoked),
        ("grant-z-live", GrantLiveness::Live),
    ] {
        assert_eq!(
            live.get_grant(&grant_id(id))
                .expect("the mixed-class grant must be projected")
                .liveness_at(&evaluated_at),
            expected_liveness,
        );
    }

    let issuer = VerifiedIssuer::operator();
    let request_target = adapter_target();
    let expected = Ok(Authorized {
        grant_id: Some(grant_id("grant-z-live")),
    });
    assert_eq!(
        grant_decision_at(&live, &issuer, &request_target, &evaluated_at).await,
        expected.clone(),
    );

    let rebuilt = rebuild_from_log(&storage, &domain("authority-main"))
        .await
        .expect("the mixed-class grants must replay");
    assert_eq!(rebuilt, live);
    assert_eq!(
        grant_decision_at(&rebuilt, &issuer, &request_target, &evaluated_at).await,
        expected,
    );
}

#[tokio::test]
async fn expired_denial_precedes_revoked_and_revoked_expired_classifies_as_revoked() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut live = AuthorityRegistry::new();
    let evaluated_at = timestamp(100);

    ingest_adapter_grant(
        &storage,
        &mut live,
        "grant-a-revoked-expired",
        Some(timestamp(90)),
    )
    .await;
    ingest_revocation(
        &storage,
        &mut live,
        &domain("authority-main"),
        revocation_at("grant-a-revoked-expired", 95),
    )
    .await
    .unwrap();
    ingest_adapter_grant(&storage, &mut live, "grant-z-expired", Some(evaluated_at)).await;

    assert_eq!(
        live.get_grant(&grant_id("grant-a-revoked-expired"))
            .unwrap()
            .liveness_at(&evaluated_at),
        GrantLiveness::Revoked,
    );
    assert_eq!(
        live.get_grant(&grant_id("grant-z-expired"))
            .unwrap()
            .liveness_at(&evaluated_at),
        GrantLiveness::Expired,
    );

    let issuer = VerifiedIssuer::operator();
    let request_target = adapter_target();
    let expected = Err(GrantDenied::NoGrant {
        actor: "grant_expired:grant-z-expired".to_owned(),
        kind: OperationKind::Instruct,
        target: format!("{request_target:?}"),
    });
    assert_eq!(
        grant_decision_at(&live, &issuer, &request_target, &evaluated_at).await,
        expected.clone(),
    );

    let rebuilt = rebuild_from_log(&storage, &domain("authority-main"))
        .await
        .expect("the denial-provenance grants must replay");
    assert_eq!(rebuilt, live);
    assert_eq!(
        grant_decision_at(&rebuilt, &issuer, &request_target, &evaluated_at).await,
        expected,
    );
}

#[tokio::test]
async fn grant_id_collation_uses_exact_utf8_bytes_without_normalization() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut live = AuthorityRegistry::new();
    let evaluated_at = timestamp(100);
    let decomposed = "grant-e\u{301}";
    let composed = "grant-\u{e9}";
    assert!(decomposed.as_bytes() < composed.as_bytes());

    // Reverse insertion pressure: exact UTF-8 ordering must still choose the
    // decomposed id rather than normalizing the visually equivalent strings.
    ingest_adapter_grant(&storage, &mut live, composed, None).await;
    ingest_adapter_grant(&storage, &mut live, decomposed, None).await;

    let issuer = VerifiedIssuer::operator();
    let request_target = adapter_target();
    let expected = Ok(Authorized {
        grant_id: Some(grant_id(decomposed)),
    });
    assert_eq!(
        grant_decision_at(&live, &issuer, &request_target, &evaluated_at).await,
        expected.clone(),
    );

    let rebuilt = rebuild_from_log(&storage, &domain("authority-main"))
        .await
        .expect("the exact-byte grant ids must replay");
    assert_eq!(rebuilt, live);
    assert_eq!(
        grant_decision_at(&rebuilt, &issuer, &request_target, &evaluated_at).await,
        expected,
    );
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

    async fn append_grant_audited(
        &self,
        authority_domain_id: &AuthorityDomainId,
        identity: &GrantIdentityKey,
        source: StoredEventPayload,
        audit: AuditRecordDraft,
    ) -> Result<GrantAppendOutcome, StorageError> {
        self.inner
            .append_grant_audited(authority_domain_id, identity, source, audit)
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
