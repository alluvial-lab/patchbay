//! Integration tests for session replay and target resolution.
//!
//! Verifies that `rebuild_from_log` reconstructs the registry from the durable
//! log (replay determinism + corruption rejection) and that the `TargetResolver`
//! impl binds targets per the design's existence + tombstone-only validation
//! depth (Q3): tombstoned generations are rejected as stale targets;
//! connectivity is a delivery concern and does NOT block resolution.
//!
//! Note: `SessionRegistry` has both an inherent `resolve(target_scope)` (the
//! Option-returning lookup) and the `TargetResolver::resolve` trait method
//! (the Result-returning acceptance seam). These tests exercise the trait impl,
//! so calls are fully qualified as `TargetResolver::resolve(&registry, ...)`.

use patchbay_contracts::patchbay::{
    AdapterId, AuthorityDomainId, Generation, OperationKind, RuntimeSessionId,
    SessionActivityState, SessionConnectivityState, TargetScope, TargetScopeKind,
};
use patchbay_core::acceptance::TargetResolver;
use patchbay_core::session::{ingest_session_report, rebuild_from_log, SessionReport};
use patchbay_core::storage::RusqliteStorage;

fn domain() -> AuthorityDomainId {
    AuthorityDomainId {
        value: "test-domain".to_string(),
    }
}

fn adapter() -> AdapterId {
    AdapterId {
        value: "pi".to_string(),
    }
}

fn runtime_session(id: &str) -> RuntimeSessionId {
    RuntimeSessionId {
        value: id.to_string(),
    }
}

fn generation(n: u64) -> Generation {
    Generation { value: n }
}

/// A live session report at the given generation.
fn report(gen: u64, connectivity: SessionConnectivityState) -> SessionReport {
    SessionReport {
        authority_domain_id: domain(),
        adapter_id: adapter(),
        deployment_scope: "local".to_string(),
        runtime_session_id: runtime_session("s-1"),
        session_generation: generation(gen),
        connectivity,
        activity: SessionActivityState::Idle,
        project: "proj".to_string(),
        cwd: "/cwd".to_string(),
        name: "name".to_string(),
        model: "provider/model".to_string(),
        spawn_origin: None,
    }
}

fn target_scope(gen: Option<u64>) -> TargetScope {
    TargetScope {
        kind: TargetScopeKind::RuntimeSession as i32,
        adapter_id: Some(adapter()),
        runtime_session_id: Some(runtime_session("s-1")),
        session_generation: gen.map(generation),
        deployment_scope: "local".to_string(),
        ..TargetScope::default()
    }
}

/// Rebuild the registry from the durable log after any prior writes.
async fn rebuild(storage: &RusqliteStorage) -> patchbay_core::session::SessionRegistry {
    rebuild_from_log(storage, &domain()).await.unwrap()
}

#[tokio::test]
async fn replay_reconstructs_a_live_registry() {
    let storage = RusqliteStorage::open_in_memory().unwrap();

    let mut registry = rebuild(&storage).await;
    ingest_session_report(
        &storage,
        &mut registry,
        report(1, SessionConnectivityState::Live),
    )
    .await
    .unwrap();

    let mut registry = rebuild(&storage).await;
    ingest_session_report(
        &storage,
        &mut registry,
        report(2, SessionConnectivityState::Live),
    )
    .await
    .unwrap();

    // Rebuild from the log: the live generation must be 2, generation 1 tombstoned.
    let registry = rebuild(&storage).await;
    let live = registry
        .get_live_session(&adapter(), "local", &runtime_session("s-1"))
        .expect("live session exists after replay");
    assert_eq!(live.identity.session_generation, generation(2));

    let tomb = registry
        .get_tombstone(&adapter(), "local", &runtime_session("s-1"), &generation(1))
        .expect("prior generation is tombstoned");
    assert_eq!(tomb.superseded_generation, generation(1));
}

#[tokio::test]
async fn resolve_binds_a_live_session() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = rebuild(&storage).await;
    ingest_session_report(
        &storage,
        &mut registry,
        report(1, SessionConnectivityState::Live),
    )
    .await
    .unwrap();

    let registry = rebuild(&storage).await;
    let binding = TargetResolver::resolve(
        &registry,
        &domain(),
        OperationKind::Instruct,
        &target_scope(Some(1)),
    )
    .await
    .unwrap();
    assert_eq!(
        binding,
        patchbay_core::acceptance::TargetBinding::RuntimeSession {
            adapter_id: adapter(),
            deployment_scope: "local".to_owned(),
            runtime_session_id: runtime_session("s-1"),
            session_generation: generation(1),
        }
    );
}

#[tokio::test]
async fn resolve_binds_the_live_generation_when_unspecified() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = rebuild(&storage).await;
    ingest_session_report(
        &storage,
        &mut registry,
        report(7, SessionConnectivityState::Live),
    )
    .await
    .unwrap();

    let registry = rebuild(&storage).await;
    // No session_generation in the scope: bind the live one.
    let binding = TargetResolver::resolve(
        &registry,
        &domain(),
        OperationKind::Instruct,
        &target_scope(None),
    )
    .await
    .unwrap();
    assert!(matches!(
        binding,
        patchbay_core::acceptance::TargetBinding::RuntimeSession {
            session_generation,
            ..
        } if session_generation == generation(7)
    ));
}

#[tokio::test]
async fn resolve_rejects_a_tombstoned_generation() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = rebuild(&storage).await;
    ingest_session_report(
        &storage,
        &mut registry,
        report(1, SessionConnectivityState::Live),
    )
    .await
    .unwrap();
    let mut registry = rebuild(&storage).await;
    // Bump to generation 2 — generation 1 is now tombstoned.
    ingest_session_report(
        &storage,
        &mut registry,
        report(2, SessionConnectivityState::Live),
    )
    .await
    .unwrap();

    let registry = rebuild(&storage).await;
    let result = TargetResolver::resolve(
        &registry,
        &domain(),
        OperationKind::Instruct,
        &target_scope(Some(1)),
    )
    .await;
    assert!(result.is_err(), "tombstoned generation must not resolve");
}

#[tokio::test]
async fn resolve_rejects_a_generation_that_is_neither_live_nor_tombstoned() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = rebuild(&storage).await;
    ingest_session_report(
        &storage,
        &mut registry,
        report(1, SessionConnectivityState::Live),
    )
    .await
    .unwrap();

    let registry = rebuild(&storage).await;
    // Generation 99 was never registered and is not tombstoned.
    let result = TargetResolver::resolve(
        &registry,
        &domain(),
        OperationKind::Instruct,
        &target_scope(Some(99)),
    )
    .await;
    assert!(result.is_err(), "unknown generation must not resolve");
}

#[tokio::test]
async fn resolve_rejects_an_unknown_session() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let registry = rebuild(&storage).await;

    let scope = TargetScope {
        kind: TargetScopeKind::RuntimeSession as i32,
        adapter_id: Some(adapter()),
        runtime_session_id: Some(runtime_session("never-registered")),
        session_generation: Some(generation(1)),
        deployment_scope: "local".to_string(),
        ..TargetScope::default()
    };
    let result =
        TargetResolver::resolve(&registry, &domain(), OperationKind::Instruct, &scope).await;
    assert!(result.is_err(), "unknown session must not resolve");
}

#[tokio::test]
async fn resolve_allows_an_offline_session() {
    // Q3 load-bearing assertion: connectivity is a delivery concern, not an
    // identity/existence concern. An offline session is still a valid target.
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = rebuild(&storage).await;
    ingest_session_report(
        &storage,
        &mut registry,
        report(1, SessionConnectivityState::Offline),
    )
    .await
    .unwrap();

    let registry = rebuild(&storage).await;
    let binding = TargetResolver::resolve(
        &registry,
        &domain(),
        OperationKind::Instruct,
        &target_scope(Some(1)),
    )
    .await;
    assert!(binding.is_ok(), "offline session must still resolve");
}

#[tokio::test]
async fn resolve_allows_a_failed_session() {
    // A failed session is likewise a valid (if degraded) delivery target.
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = rebuild(&storage).await;
    ingest_session_report(
        &storage,
        &mut registry,
        report(1, SessionConnectivityState::Failed),
    )
    .await
    .unwrap();

    let registry = rebuild(&storage).await;
    let binding = TargetResolver::resolve(
        &registry,
        &domain(),
        OperationKind::Instruct,
        &target_scope(Some(1)),
    )
    .await;
    assert!(binding.is_ok(), "failed session must still resolve");
}
