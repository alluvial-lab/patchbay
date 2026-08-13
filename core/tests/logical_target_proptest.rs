use std::collections::HashSet;

use patchbay_contracts::patchbay::{
    AdapterId, AuthorityDomainId, ExternalRuntimeRef, Generation, LogicalTargetId, RuntimeSessionId,
};
use patchbay_core::session::{
    external_runtime_key, ExternalRuntimeOwnership, LogicalTargetError, LogicalTargetRegistry,
};
use proptest::prelude::*;

fn domain(value: &str) -> AuthorityDomainId {
    AuthorityDomainId {
        value: value.to_owned(),
    }
}

fn target(value: &str) -> LogicalTargetId {
    LogicalTargetId {
        value: value.to_owned(),
    }
}

fn external(adapter: &str, scope: &str, runtime: &str, generation: u64) -> ExternalRuntimeRef {
    ExternalRuntimeRef {
        adapter_id: Some(AdapterId {
            value: adapter.to_owned(),
        }),
        deployment_scope: scope.to_owned(),
        runtime_session_id: Some(RuntimeSessionId {
            value: runtime.to_owned(),
        }),
        generation: Some(Generation { value: generation }),
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 100, ..ProptestConfig::default() })]

    #[test]
    fn exact_external_runtime_has_one_owner_across_checkpoint_recovery(
        authority in "[a-z][a-z0-9-]{0,15}",
        adapter in "[a-z][a-z0-9-]{0,15}",
        scope in "[a-z][a-z0-9._/-]{0,15}",
        runtime in "[a-z][a-z0-9-]{0,15}",
        generation in 1u64..=u64::MAX,
    ) {
        let authority = domain(&authority);
        let exact = external(&adapter, &scope, &runtime, generation);
        let first = target("target-a");
        let second = target("target-b");
        let mut registry = LogicalTargetRegistry::new(authority.clone()).unwrap();
        registry
            .create(first.clone(), AdapterId { value: adapter.clone() }, scope.clone())
            .unwrap();
        registry
            .create(second.clone(), AdapterId { value: adapter }, scope)
            .unwrap();
        registry.reserve_candidate(&first, exact.clone()).unwrap();
        prop_assert_eq!(registry.owner_of(&exact), Some(&first));
        let hot_duplicate_rejected = matches!(
            registry.reserve_candidate(&second, exact.clone()),
            Err(LogicalTargetError::DuplicateNativeReference { .. })
        );
        prop_assert!(hot_duplicate_rejected);

        let mut recovered = LogicalTargetRegistry::from_checkpoint(
            authority,
            1,
            registry.checkpoint_records(),
        )
        .unwrap();
        prop_assert_eq!(recovered.owner_of(&exact), Some(&first));
        let recovered_duplicate_rejected = matches!(
            recovered.reserve_candidate(&second, exact),
            Err(LogicalTargetError::DuplicateNativeReference { .. })
        );
        prop_assert!(recovered_duplicate_rejected);
    }

    #[test]
    fn generation_zero_never_reserves(
        adapter in "[a-z][a-z0-9-]{0,15}",
        scope in "[a-z][a-z0-9._/-]{0,15}",
        runtime in "[a-z][a-z0-9-]{0,15}",
    ) {
        let logical = target("target-a");
        let mut registry = LogicalTargetRegistry::new(domain("authority-main")).unwrap();
        registry
            .create(logical.clone(), AdapterId { value: adapter.clone() }, scope.clone())
            .unwrap();
        let original = registry.clone();
        prop_assert_eq!(
            registry.reserve_candidate(&logical, external(&adapter, &scope, &runtime, 0)),
            Err(LogicalTargetError::NonPositiveGeneration)
        );
        prop_assert_eq!(registry, original);
    }
}

#[derive(Clone, Copy)]
enum OmittedDimension {
    AuthorityDomain,
    Adapter,
    DeploymentScope,
    RuntimeSession,
    Generation,
}

fn mutant_key(
    key: &patchbay_core::session::ExternalRuntimeKey,
    omitted: OmittedDimension,
) -> (String, String, String, String, u64) {
    (
        if matches!(omitted, OmittedDimension::AuthorityDomain) {
            String::new()
        } else {
            key.authority_domain_id.clone()
        },
        if matches!(omitted, OmittedDimension::Adapter) {
            String::new()
        } else {
            key.adapter_id.clone()
        },
        if matches!(omitted, OmittedDimension::DeploymentScope) {
            String::new()
        } else {
            key.deployment_scope.clone()
        },
        if matches!(omitted, OmittedDimension::RuntimeSession) {
            String::new()
        } else {
            key.runtime_session_id.clone()
        },
        if matches!(omitted, OmittedDimension::Generation) {
            0
        } else {
            key.generation
        },
    )
}

#[test]
fn independent_oracle_kills_every_reverse_index_dimension_omission() {
    let refs = [
        (
            domain("domain-a"),
            external("pi", "scope-a", "runtime-a", 1),
        ),
        (
            domain("domain-b"),
            external("pi", "scope-a", "runtime-a", 1),
        ),
        (
            domain("domain-a"),
            external("other", "scope-a", "runtime-a", 1),
        ),
        (
            domain("domain-a"),
            external("pi", "scope-b", "runtime-a", 1),
        ),
        (
            domain("domain-a"),
            external("pi", "scope-a", "runtime-b", 1),
        ),
        (
            domain("domain-a"),
            external("pi", "scope-a", "runtime-a", 2),
        ),
    ];
    let keys: Vec<_> = refs
        .iter()
        .map(|(authority, external)| external_runtime_key(authority, external).unwrap())
        .collect();
    assert_eq!(keys.iter().collect::<HashSet<_>>().len(), keys.len());

    for (omitted, collision_pair) in [
        (OmittedDimension::AuthorityDomain, (0, 1)),
        (OmittedDimension::Adapter, (0, 2)),
        (OmittedDimension::DeploymentScope, (0, 3)),
        (OmittedDimension::RuntimeSession, (0, 4)),
        (OmittedDimension::Generation, (0, 5)),
    ] {
        assert_eq!(
            mutant_key(&keys[collision_pair.0], omitted),
            mutant_key(&keys[collision_pair.1], omitted),
            "independent oracle did not expose an omitted reverse-index dimension"
        );
    }
}
