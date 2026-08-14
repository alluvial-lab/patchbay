use std::collections::HashSet;

use patchbay_contracts::patchbay::{
    AdapterId, Operation, ResourceId, ResourceKind, TargetScope, TargetScopeKind,
};
use patchbay_core::{
    acceptance::target_key_for,
    resource::{ResourceIdentity, ResourceIdentityError},
};
use prost::Message;

fn identity(adapter: &str, kind: &str, id: &str) -> ResourceIdentity {
    ResourceIdentity::new(
        AdapterId {
            value: adapter.to_owned(),
        },
        ResourceKind {
            value: kind.to_owned(),
        },
        ResourceId {
            value: id.to_owned(),
        },
    )
    .unwrap()
}

#[test]
fn canonical_identity_round_trips_and_each_tuple_dimension_is_distinct() {
    let base = identity("adapter-a", "pool", "shared");
    let variants = [
        base.clone(),
        identity("adapter-b", "pool", "shared"),
        identity("adapter-a", "window", "shared"),
        identity("adapter-a", "pool", "other"),
    ];

    let set: HashSet<_> = variants.iter().cloned().collect();
    assert_eq!(set.len(), variants.len());
    for value in &variants {
        assert_eq!(
            ResourceIdentity::try_from_scope(&value.to_scope()).unwrap(),
            *value
        );
    }

    let encoded: HashSet<_> = variants
        .iter()
        .map(|value| value.to_scope().encode_to_vec())
        .collect();
    assert_eq!(encoded.len(), variants.len());
    let target_keys: HashSet<_> = variants
        .iter()
        .map(|value| {
            target_key_for(&Operation {
                target_scope: Some(value.to_scope()),
                ..Operation::default()
            })
            .unwrap()
        })
        .collect();
    assert_eq!(target_keys.len(), variants.len());
}

#[test]
fn empty_partial_mixed_legacy_and_dual_shapes_are_rejected() {
    for (adapter, kind, id, field) in [
        ("", "pool", "one", "adapter_id"),
        ("a", "", "one", "resource_kind"),
        ("a", "pool", "", "resource_id"),
    ] {
        assert_eq!(
            ResourceIdentity::new(
                AdapterId {
                    value: adapter.to_owned()
                },
                ResourceKind {
                    value: kind.to_owned()
                },
                ResourceId {
                    value: id.to_owned()
                },
            ),
            Err(ResourceIdentityError::Missing { field })
        );
    }

    let mut partial = identity("a", "pool", "one").to_scope();
    partial.resource.as_mut().unwrap().resource_kind = None;
    assert_eq!(
        ResourceIdentity::try_from_scope(&partial),
        Err(ResourceIdentityError::Missing {
            field: "resource_kind"
        })
    );

    let mut mixed = identity("a", "pool", "one").to_scope();
    mixed.adapter_id = Some(AdapterId {
        value: "a".to_owned(),
    });
    assert_eq!(
        ResourceIdentity::try_from_scope(&mixed),
        Err(ResourceIdentityError::MixedTargetFields)
    );

    let legacy = TargetScope {
        kind: TargetScopeKind::Resource as i32,
        legacy_audit_resource_id: "old-audit-target".to_owned(),
        ..TargetScope::default()
    };
    assert_eq!(
        ResourceIdentity::try_from_scope(&legacy),
        Err(ResourceIdentityError::LegacyAuditOnly)
    );
    let mut dual = identity("a", "pool", "one").to_scope();
    dual.legacy_audit_resource_id = "old-audit-target".to_owned();
    assert_eq!(
        ResourceIdentity::try_from_scope(&dual),
        Err(ResourceIdentityError::LegacyAuditOnly)
    );
}

#[test]
fn protobuf_tag_eight_still_decodes_as_audit_only_data() {
    // kind=RESOURCE (tag 1) followed by legacy string tag 8.
    let bytes = [
        0x08,
        TargetScopeKind::Resource as u8,
        0x42,
        0x06,
        b'l',
        b'e',
        b'g',
        b'a',
        b'c',
        b'y',
    ];
    let decoded = TargetScope::decode(bytes.as_slice()).unwrap();
    assert_eq!(decoded.legacy_audit_resource_id, "legacy");
    assert!(decoded.resource.is_none());
    assert_eq!(
        ResourceIdentity::try_from_scope(&decoded),
        Err(ResourceIdentityError::LegacyAuditOnly)
    );
}
