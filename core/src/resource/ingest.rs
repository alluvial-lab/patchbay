use std::collections::{HashMap, HashSet};

use patchbay_contracts::patchbay::{
    resource_report_mutation, resource_state_mutation, AdapterId, AdapterSnapshotSupport,
    AuthorityDomainId, EventId, Generation, Lsn, ResourceFreshnessChanged,
    ResourceFreshnessState, ResourceReportMutation, ResourceStateEvent, ResourceStateMutation,
    ResourceStateTombstone, ResourceStateUnknown, ResourceViewReport, ResourceViewStateUpdate,
};
use prost_types::Timestamp;

use crate::storage::{RecordedEvent, Storage};

use super::{events, replay, ResourceError, ResourceIdentity, ResourceRegistry, ResourceViewKey};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceReportMode {
    Snapshot,
    Delta,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedResourceReport {
    pub authority_domain_id: AuthorityDomainId,
    pub adapter_id: AdapterId,
    pub adapter_generation: Generation,
    pub mode: ResourceReportMode,
    pub views: Vec<ResourceViewReport>,
    pub observed_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceIngestResult {
    pub event_id: EventId,
    pub touched_resources: usize,
    pub touched_views: usize,
}

/// Normalize one authenticated report, append exactly one durable source
/// event, then fold only the committed event into the hot projection.
pub async fn ingest_resource_report<S: Storage>(
    storage: &S,
    registry: &mut ResourceRegistry,
    report: ValidatedResourceReport,
) -> Result<ResourceIngestResult, ResourceError> {
    let event = normalize_report(registry, report)?;
    let authority_domain_id = event
        .authority_domain_id
        .clone()
        .expect("normalized resource event has domain");
    let touched_resources = event.mutations.len();
    let touched_views = event.views.len();
    let payload = events::encode(&event);
    let event_id = storage.append(&authority_domain_id, payload.clone()).await?;
    validate_event_id(&event_id, &authority_domain_id)?;
    let recorded = RecordedEvent {
        event_id: event_id.clone(),
        payload,
    };
    if let Err(error) = registry.observe(&recorded) {
        // The append is already authoritative. Never continue with a hot
        // projection that rejected a committed prefix.
        *registry = replay::rebuild_from_log(storage, &authority_domain_id).await?;
        return Err(error);
    }
    Ok(ResourceIngestResult {
        event_id,
        touched_resources,
        touched_views,
    })
}

/// Build the normalized stale-degradation event for one detached adapter.
pub fn adapter_stale_event(
    registry: &ResourceRegistry,
    authority_domain_id: &AuthorityDomainId,
    adapter_id: &AdapterId,
    adapter_generation: Generation,
    observed_at: Timestamp,
) -> Result<Option<patchbay_contracts::patchbay::StoredEventPayload>, ResourceError> {
    if authority_domain_id.value.is_empty() || adapter_id.value.is_empty() {
        return Err(ResourceError::InvalidReport(
            "adapter stale event requires domain and adapter identity".into(),
        ));
    }
    validate_observed_at(&observed_at)?;
    let mut views: Vec<_> = registry
        .views()
        .filter(|view| view.key.adapter_id == *adapter_id)
        .map(|view| ResourceViewStateUpdate {
            resource_kind: Some(view.key.resource_kind.clone()),
            completeness: view.completeness as i32,
        })
        .collect();
    views.sort_by(|left, right| {
        left.resource_kind
            .as_ref()
            .map(|kind| &kind.value)
            .cmp(&right.resource_kind.as_ref().map(|kind| &kind.value))
    });
    let mut records: Vec<_> = registry
        .resources()
        .filter(|record| {
            record.identity.adapter_id() == adapter_id
                && !record.tombstoned()
                && record.freshness != ResourceFreshnessState::Stale
        })
        .collect();
    records.sort_by(resource_record_order);
    let mutations = records
        .into_iter()
        .map(|record| ResourceStateMutation {
            identity: Some(record.identity.to_scope().resource.expect("canonical resource")),
            from_revision_lsn: Some(Lsn {
                value: record.revision_lsn,
            }),
            mutation: Some(resource_state_mutation::Mutation::FreshnessChanged(
                ResourceFreshnessChanged {
                    from: record.freshness as i32,
                    to: ResourceFreshnessState::Stale as i32,
                },
            )),
        })
        .collect::<Vec<_>>();
    if mutations.is_empty() {
        return Ok(None);
    }
    Ok(Some(events::encode(&ResourceStateEvent {
        authority_domain_id: Some(authority_domain_id.clone()),
        source_adapter_id: Some(adapter_id.clone()),
        source_adapter_generation: Some(adapter_generation),
        views,
        mutations,
        observed_at: Some(observed_at),
    })))
}

fn normalize_report(
    registry: &ResourceRegistry,
    report: ValidatedResourceReport,
) -> Result<ResourceStateEvent, ResourceError> {
    validate_report_shape(&report)?;
    let mut reported: HashMap<ResourceViewKey, &ResourceViewReport> = HashMap::new();
    let mut explicit: HashMap<ResourceIdentity, &ResourceReportMutation> = HashMap::new();
    for view in &report.views {
        let key = ResourceViewKey {
            adapter_id: report.adapter_id.clone(),
            resource_kind: view.resource_kind.clone().expect("validated kind"),
        };
        reported.insert(key, view);
        for mutation in &view.mutations {
            let identity = ResourceIdentity::try_from_wire(
                mutation.identity.as_ref().expect("validated identity"),
            )?;
            explicit.insert(identity, mutation);
        }
    }

    let newest_projected_generation = registry
        .views()
        .filter(|view| view.key.adapter_id == report.adapter_id)
        .map(|view| view.source_adapter_generation.value)
        .max()
        .unwrap_or(0);
    if report.adapter_generation.value < newest_projected_generation {
        return Err(ResourceError::StaleAdapterGeneration {
            live: newest_projected_generation,
            reported: report.adapter_generation.value,
        });
    }
    let newer_generation = report.adapter_generation.value > newest_projected_generation;

    let mut mutation_by_identity: HashMap<ResourceIdentity, ResourceStateMutation> = HashMap::new();
    for (identity, wire) in explicit {
        let prior = registry.get(&identity);
        let mutation = match wire.mutation.as_ref().expect("validated mutation") {
            resource_report_mutation::Mutation::Upsert(upsert) => ResourceStateMutation {
                identity: wire.identity.clone(),
                from_revision_lsn: prior.map(|record| Lsn {
                    value: record.revision_lsn,
                }),
                mutation: Some(resource_state_mutation::Mutation::Upsert(upsert.clone())),
            },
            resource_report_mutation::Mutation::Unknown(_) => ResourceStateMutation {
                identity: wire.identity.clone(),
                from_revision_lsn: prior.map(|record| Lsn {
                    value: record.revision_lsn,
                }),
                mutation: Some(resource_state_mutation::Mutation::Unknown(
                    ResourceStateUnknown {},
                )),
            },
            resource_report_mutation::Mutation::Tombstone(tombstone) => {
                if prior.is_none_or(|record| record.tombstoned()) {
                    return Err(ResourceError::InvalidReport(format!(
                        "tombstone targets unknown or retired resource {identity:?}"
                    )));
                }
                ResourceStateMutation {
                    identity: wire.identity.clone(),
                    from_revision_lsn: prior.map(|record| Lsn {
                        value: record.revision_lsn,
                    }),
                    mutation: Some(resource_state_mutation::Mutation::Tombstone(
                        tombstone.clone(),
                    )),
                }
            }
        };
        if prior.is_some_and(|record| record.tombstoned()) {
            return Err(ResourceError::TerminalTombstone(identity));
        }
        mutation_by_identity.insert(identity, mutation);
    }

    for record in registry.resources().filter(|record| {
        record.identity.adapter_id() == &report.adapter_id && !record.tombstoned()
    }) {
        if mutation_by_identity.contains_key(&record.identity) {
            continue;
        }
        let key = ResourceViewKey {
            adapter_id: report.adapter_id.clone(),
            resource_kind: record.identity.resource_kind().clone(),
        };
        let implied = if let Some(view) = reported.get(&key) {
            if report.mode == ResourceReportMode::Snapshot {
                match AdapterSnapshotSupport::try_from(view.completeness)
                    .expect("validated completeness")
                {
                    AdapterSnapshotSupport::Authoritative => Some(
                        resource_state_mutation::Mutation::Tombstone(ResourceStateTombstone {
                            replaced_by: None,
                        }),
                    ),
                    AdapterSnapshotSupport::Partial | AdapterSnapshotSupport::None => {
                        stale_change(record.freshness)
                    }
                    AdapterSnapshotSupport::Unspecified => unreachable!(),
                }
            } else if newer_generation {
                stale_change(record.freshness)
            } else {
                None
            }
        } else if newer_generation {
            stale_change(record.freshness)
        } else {
            None
        };
        if let Some(mutation) = implied {
            mutation_by_identity.insert(
                record.identity.clone(),
                ResourceStateMutation {
                    identity: Some(
                        record
                            .identity
                            .to_scope()
                            .resource
                            .expect("canonical resource identity"),
                    ),
                    from_revision_lsn: Some(Lsn {
                        value: record.revision_lsn,
                    }),
                    mutation: Some(mutation),
                },
            );
        }
    }

    validate_replacements(&report.adapter_id, &mutation_by_identity)?;
    let mut mutations: Vec<_> = mutation_by_identity.into_values().collect();
    mutations.sort_by(resource_mutation_order);
    let mut views = report
        .views
        .iter()
        .map(|view| ResourceViewStateUpdate {
            resource_kind: view.resource_kind.clone(),
            completeness: view.completeness,
        })
        .collect::<Vec<_>>();
    if newer_generation {
        for existing in registry.views().filter(|view| {
            view.key.adapter_id == report.adapter_id && !reported.contains_key(&view.key)
        }) {
            views.push(ResourceViewStateUpdate {
                resource_kind: Some(existing.key.resource_kind.clone()),
                completeness: existing.completeness as i32,
            });
        }
    }
    views.sort_by(|left, right| {
        left.resource_kind
            .as_ref()
            .map(|kind| &kind.value)
            .cmp(&right.resource_kind.as_ref().map(|kind| &kind.value))
    });

    Ok(ResourceStateEvent {
        authority_domain_id: Some(report.authority_domain_id),
        source_adapter_id: Some(report.adapter_id),
        source_adapter_generation: Some(report.adapter_generation),
        views,
        mutations,
        observed_at: Some(report.observed_at),
    })
}

fn validate_report_shape(report: &ValidatedResourceReport) -> Result<(), ResourceError> {
    if report.authority_domain_id.value.is_empty() || report.adapter_id.value.is_empty() {
        return Err(ResourceError::InvalidReport(
            "resource report requires domain and adapter identity".into(),
        ));
    }
    validate_observed_at(&report.observed_at)?;
    if report.views.is_empty() {
        return Err(ResourceError::InvalidReport(
            "resource report requires at least one view".into(),
        ));
    }
    let mut kinds = HashSet::new();
    let mut identities = HashSet::new();
    for view in &report.views {
        let kind = view
            .resource_kind
            .as_ref()
            .filter(|kind| !kind.value.is_empty())
            .ok_or_else(|| ResourceError::InvalidReport("resource view is missing kind".into()))?;
        if !kinds.insert(kind.clone()) {
            return Err(ResourceError::InvalidReport(
                "resource report contains duplicate view kind".into(),
            ));
        }
        let tier = AdapterSnapshotSupport::try_from(view.completeness).map_err(|_| {
            ResourceError::InvalidReport("resource view has unknown completeness".into())
        })?;
        if tier == AdapterSnapshotSupport::Unspecified {
            return Err(ResourceError::InvalidReport(
                "resource view completeness is unspecified".into(),
            ));
        }
        if tier == AdapterSnapshotSupport::None && !view.mutations.is_empty() {
            return Err(ResourceError::InvalidReport(
                "none-completeness view cannot carry reconstructed mutations".into(),
            ));
        }
        for mutation in &view.mutations {
            let identity = ResourceIdentity::try_from_wire(
                mutation.identity.as_ref().ok_or_else(|| {
                    ResourceError::InvalidReport("resource mutation is missing identity".into())
                })?,
            )?;
            if identity.adapter_id() != &report.adapter_id || identity.resource_kind() != kind {
                return Err(ResourceError::InvalidReport(
                    "resource mutation identity does not match authenticated view".into(),
                ));
            }
            if !identities.insert(identity) {
                return Err(ResourceError::InvalidReport(
                    "resource report contains duplicate mutation identity".into(),
                ));
            }
            let mutation = mutation.mutation.as_ref().ok_or_else(|| {
                ResourceError::InvalidReport("resource mutation has no variant".into())
            })?;
            if let resource_report_mutation::Mutation::Upsert(upsert) = mutation {
                // Projection schema admission is owned by AdapterRegistry at
                // server ingress. Core still rejects structurally incomplete
                // envelopes before normalization.
                for envelope in [
                    upsert.resource_payload.as_ref(),
                    upsert.projection_payload.as_ref(),
                ] {
                    let envelope = envelope.ok_or_else(|| {
                        ResourceError::InvalidReport(
                            "resource upsert requires resource and projection payloads".into(),
                        )
                    })?;
                    if envelope.schema_ref.is_empty()
                        || patchbay_contracts::patchbay::PayloadContentType::try_from(
                            envelope.content_type,
                        )
                        .ok()
                        .is_none_or(|kind| {
                            kind == patchbay_contracts::patchbay::PayloadContentType::Unspecified
                        })
                    {
                        return Err(ResourceError::InvalidReport(
                            "resource upsert has malformed envelope".into(),
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

fn validate_replacements(
    adapter_id: &AdapterId,
    mutations: &HashMap<ResourceIdentity, ResourceStateMutation>,
) -> Result<(), ResourceError> {
    for (identity, mutation) in mutations {
        let Some(resource_state_mutation::Mutation::Tombstone(tombstone)) =
            mutation.mutation.as_ref()
        else {
            continue;
        };
        let Some(replacement) = tombstone.replaced_by.as_ref() else {
            continue;
        };
        let replacement = ResourceIdentity::try_from_wire(replacement)?;
        if replacement.adapter_id() != adapter_id || replacement == *identity {
            return Err(ResourceError::InvalidReport(
                "replacement must be distinct and owned by the authenticated adapter".into(),
            ));
        }
        if !mutations.get(&replacement).is_some_and(|candidate| {
            matches!(
                candidate.mutation,
                Some(resource_state_mutation::Mutation::Upsert(_))
            )
        }) {
            return Err(ResourceError::InvalidReport(
                "replacement requires a matching upsert in the same report".into(),
            ));
        }
    }
    Ok(())
}

fn stale_change(
    from: ResourceFreshnessState,
) -> Option<resource_state_mutation::Mutation> {
    (from != ResourceFreshnessState::Stale).then_some(
        resource_state_mutation::Mutation::FreshnessChanged(ResourceFreshnessChanged {
            from: from as i32,
            to: ResourceFreshnessState::Stale as i32,
        }),
    )
}

fn validate_observed_at(timestamp: &Timestamp) -> Result<(), ResourceError> {
    if !(-62_135_596_800..=253_402_300_799).contains(&timestamp.seconds)
        || !(0..1_000_000_000).contains(&timestamp.nanos)
    {
        return Err(ResourceError::InvalidReport(
            "observed_at is not a valid protobuf Timestamp".into(),
        ));
    }
    Ok(())
}

fn validate_event_id(
    event_id: &EventId,
    authority_domain_id: &AuthorityDomainId,
) -> Result<(), ResourceError> {
    if event_id.authority_domain_id.as_ref() != Some(authority_domain_id)
        || event_id.lsn.is_none()
    {
        return Err(ResourceError::CorruptRecord(
            "storage returned resource event with invalid identity".into(),
        ));
    }
    Ok(())
}

fn resource_mutation_order(
    left: &ResourceStateMutation,
    right: &ResourceStateMutation,
) -> std::cmp::Ordering {
    wire_identity_key(left.identity.as_ref()).cmp(&wire_identity_key(right.identity.as_ref()))
}

fn resource_record_order(
    left: &&super::ResourceRecord,
    right: &&super::ResourceRecord,
) -> std::cmp::Ordering {
    (
        &left.identity.resource_kind().value,
        &left.identity.resource_id().value,
    )
        .cmp(&(
            &right.identity.resource_kind().value,
            &right.identity.resource_id().value,
        ))
}

fn wire_identity_key(
    identity: Option<&patchbay_contracts::patchbay::ResourceIdentity>,
) -> (&str, &str, &str) {
    identity.map_or(("", "", ""), |identity| {
        (
            identity.adapter_id.as_ref().map_or("", |id| id.value.as_str()),
            identity
                .resource_kind
                .as_ref()
                .map_or("", |kind| kind.value.as_str()),
            identity
                .resource_id
                .as_ref()
                .map_or("", |id| id.value.as_str()),
        )
    })
}
