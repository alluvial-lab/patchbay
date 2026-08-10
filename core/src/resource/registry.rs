use std::collections::{HashMap, HashSet};

use patchbay_contracts::patchbay::{
    resource_state_mutation, AdapterSnapshotSupport, AuthorityDomainId, PayloadContentType,
    ResourceFreshnessState, ResourceFreshnessChanged, ResourceStateEvent, ResourceStateMutation,
    ResourceStateTombstone, ResourceStateUnknown, ResourceStateUpsert, StoredEventKind,
};
use prost::Message;
use prost_types::Timestamp;

use crate::storage::RecordedEvent;

use super::{
    ResourceError, ResourceIdentity, ResourceRecord, ResourceViewKey, ResourceViewRecord,
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct AppliedPrefix {
    authority_domain_id: Option<AuthorityDomainId>,
    applied_through_lsn: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PrefixPosition {
    Covered,
    Next,
}

impl AppliedPrefix {
    fn classify(
        &self,
        authority_domain_id: &AuthorityDomainId,
        event_lsn: u64,
    ) -> Result<PrefixPosition, ResourceError> {
        if authority_domain_id.value.is_empty() {
            return Err(ResourceError::CorruptRecord(
                "resource event has empty authority domain".into(),
            ));
        }
        if event_lsn == 0 {
            return Err(ResourceError::CorruptRecord("resource event has zero LSN".into()));
        }
        if self
            .authority_domain_id
            .as_ref()
            .is_some_and(|domain| domain != authority_domain_id)
        {
            return Err(ResourceError::CorruptLog(format!(
                "resource projection belongs to authority domain {:?}, event belongs to {:?}",
                self.authority_domain_id, authority_domain_id
            )));
        }
        if event_lsn <= self.applied_through_lsn {
            return Ok(PrefixPosition::Covered);
        }
        let next_lsn = self.applied_through_lsn.checked_add(1).ok_or_else(|| {
            ResourceError::CorruptLog("resource applied prefix LSN overflow".into())
        })?;
        if event_lsn != next_lsn {
            return Err(ResourceError::CorruptLog(format!(
                "resource event LSN {event_lsn} leaves a gap after applied LSN {}",
                self.applied_through_lsn
            )));
        }
        Ok(PrefixPosition::Next)
    }

    fn advance(&mut self, authority_domain_id: &AuthorityDomainId, event_lsn: u64) {
        debug_assert!(matches!(
            self.classify(authority_domain_id, event_lsn),
            Ok(PrefixPosition::Next)
        ));
        self.authority_domain_id = Some(authority_domain_id.clone());
        self.applied_through_lsn = event_lsn;
    }
}

/// Canonical operational-resource projection for one authority-domain log.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ResourceRegistry {
    resources: HashMap<ResourceIdentity, ResourceRecord>,
    views: HashMap<ResourceViewKey, ResourceViewRecord>,
    applied_prefix: AppliedPrefix,
}

impl ResourceRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Observe one committed event against the validated authority-domain
    /// prefix, folding owned resource state atomically and recording sibling
    /// events as prefix coverage only.
    pub fn observe(&mut self, event: &RecordedEvent) -> Result<(), ResourceError> {
        let kind = StoredEventKind::try_from(event.payload.kind).map_err(|_| {
            ResourceError::CorruptRecord(format!(
                "unknown stored event kind {}",
                event.payload.kind
            ))
        })?;
        if kind == StoredEventKind::Unspecified {
            return Err(ResourceError::CorruptRecord(
                "stored event kind is unspecified".into(),
            ));
        }

        let (event_domain, event_lsn) = event_identity(event)?;
        let state = if kind == StoredEventKind::ResourceState {
            let state = ResourceStateEvent::decode(event.payload.payload.as_slice()).map_err(|error| {
                ResourceError::CorruptRecord(format!(
                    "cannot decode resource state event at LSN {event_lsn}: {error}"
                ))
            })?;
            validate_event(&state, event_domain, event_lsn)?;
            Some(state)
        } else {
            None
        };

        match self.applied_prefix.classify(event_domain, event_lsn)? {
            PrefixPosition::Covered => Ok(()),
            PrefixPosition::Next => {
                let Some(state) = state else {
                    self.applied_prefix.advance(event_domain, event_lsn);
                    return Ok(());
                };

                // A malformed committed event must not leave a partially folded
                // projection or advance its applied prefix.
                let mut next = self.clone();
                next.apply_validated(&state, event_lsn)?;
                next.applied_prefix.advance(event_domain, event_lsn);
                *self = next;
                Ok(())
            }
        }
    }

    #[must_use]
    pub(crate) fn applied_lsn(&self) -> u64 {
        self.applied_prefix.applied_through_lsn
    }

    pub(crate) fn require_authority_domain(
        &self,
        authority_domain_id: &AuthorityDomainId,
    ) -> Result<(), ResourceError> {
        if authority_domain_id.value.is_empty() {
            return Err(ResourceError::CorruptRecord(
                "resource projection requires a non-empty authority domain".into(),
            ));
        }
        if self
            .applied_prefix
            .authority_domain_id
            .as_ref()
            .is_some_and(|domain| domain != authority_domain_id)
        {
            return Err(ResourceError::CorruptLog(format!(
                "resource projection belongs to authority domain {:?}, requested {:?}",
                self.applied_prefix.authority_domain_id, authority_domain_id
            )));
        }
        Ok(())
    }

    #[must_use]
    pub fn contains(&self, identity: &ResourceIdentity) -> bool {
        self.resources
            .get(identity)
            .is_some_and(|record| !record.tombstoned())
    }

    #[must_use]
    pub fn get(&self, identity: &ResourceIdentity) -> Option<&ResourceRecord> {
        self.resources.get(identity)
    }

    pub fn resources(&self) -> impl Iterator<Item = &ResourceRecord> {
        self.resources.values()
    }

    pub fn views(&self) -> impl Iterator<Item = &ResourceViewRecord> {
        self.views.values()
    }

    pub fn active_in_view<'a>(
        &'a self,
        key: &'a ResourceViewKey,
    ) -> impl Iterator<Item = &'a ResourceRecord> + 'a {
        self.resources.values().filter(move |record| {
            !record.tombstoned()
                && record.identity.adapter_id() == &key.adapter_id
                && record.identity.resource_kind() == &key.resource_kind
        })
    }

    fn apply_validated(
        &mut self,
        state: &ResourceStateEvent,
        event_lsn: u64,
    ) -> Result<(), ResourceError> {
        let adapter_id = state.source_adapter_id.as_ref().expect("validated adapter id");
        let generation = state
            .source_adapter_generation
            .expect("validated adapter generation");
        let observed_at = state.observed_at.expect("validated observed_at");
        let projected_generation = self
            .views
            .values()
            .filter(|view| view.key.adapter_id == *adapter_id)
            .map(|view| view.source_adapter_generation.value)
            .max()
            .unwrap_or(0);
        if generation.value < projected_generation {
            return Err(ResourceError::CorruptLog(format!(
                "resource state event at LSN {event_lsn} lowers adapter generation from {projected_generation} to {}",
                generation.value
            )));
        }

        for view in &state.views {
            let key = ResourceViewKey {
                adapter_id: adapter_id.clone(),
                resource_kind: view.resource_kind.clone().expect("validated resource kind"),
            };
            self.views.insert(
                key.clone(),
                ResourceViewRecord {
                    key,
                    completeness: AdapterSnapshotSupport::try_from(view.completeness)
                        .expect("validated completeness"),
                    source_adapter_generation: generation,
                    revision_lsn: event_lsn,
                    observed_at,
                },
            );
        }

        for mutation in &state.mutations {
            let identity = ResourceIdentity::try_from_wire(
                mutation.identity.as_ref().expect("validated identity"),
            )?;
            let prior = self.resources.get(&identity).cloned();
            validate_from_revision(prior.as_ref(), mutation, event_lsn)?;

            let next = match mutation.mutation.as_ref().expect("validated mutation") {
                resource_state_mutation::Mutation::Upsert(upsert) => {
                    apply_upsert(identity.clone(), prior, upsert, generation, event_lsn, observed_at)?
                }
                resource_state_mutation::Mutation::Unknown(_unknown) => {
                    apply_unknown(identity.clone(), prior, generation, event_lsn, observed_at)?
                }
                resource_state_mutation::Mutation::Tombstone(tombstone) => apply_tombstone(
                    identity.clone(),
                    prior,
                    tombstone,
                    generation,
                    event_lsn,
                    observed_at,
                )?,
                resource_state_mutation::Mutation::FreshnessChanged(change) => {
                    apply_freshness_change(
                        identity.clone(),
                        prior,
                        change,
                        generation,
                        event_lsn,
                        observed_at,
                    )?
                }
            };
            self.resources.insert(identity, next);
        }
        Ok(())
    }
}

fn validate_event(
    state: &ResourceStateEvent,
    event_domain: &AuthorityDomainId,
    event_lsn: u64,
) -> Result<(), ResourceError> {
    let domain = state.authority_domain_id.as_ref().ok_or_else(|| {
        ResourceError::CorruptRecord(format!(
            "resource state event at LSN {event_lsn} is missing authority_domain_id"
        ))
    })?;
    if domain.value.is_empty() || domain != event_domain {
        return Err(ResourceError::CorruptLog(format!(
            "resource state event domain {:?} does not match {:?} at LSN {event_lsn}",
            domain, event_domain
        )));
    }
    let adapter_id = state.source_adapter_id.as_ref().ok_or_else(|| {
        ResourceError::CorruptRecord(format!(
            "resource state event at LSN {event_lsn} is missing source_adapter_id"
        ))
    })?;
    if adapter_id.value.is_empty() || state.source_adapter_generation.is_none() {
        return Err(ResourceError::CorruptRecord(format!(
            "resource state event at LSN {event_lsn} has incomplete source identity"
        )));
    }
    let observed_at = state.observed_at.as_ref().ok_or_else(|| {
        ResourceError::CorruptRecord(format!(
            "resource state event at LSN {event_lsn} is missing observed_at"
        ))
    })?;
    validate_timestamp(observed_at, event_lsn)?;

    let mut view_keys = HashSet::new();
    for view in &state.views {
        let kind = view.resource_kind.as_ref().ok_or_else(|| {
            ResourceError::CorruptRecord(format!(
                "resource view update at LSN {event_lsn} is missing resource_kind"
            ))
        })?;
        if kind.value.is_empty() || !view_keys.insert(kind.clone()) {
            return Err(ResourceError::CorruptRecord(format!(
                "resource view update at LSN {event_lsn} has empty or duplicate resource_kind"
            )));
        }
        let completeness = AdapterSnapshotSupport::try_from(view.completeness).map_err(|_| {
            ResourceError::CorruptRecord(format!(
                "resource view update at LSN {event_lsn} has unknown completeness {}",
                view.completeness
            ))
        })?;
        if completeness == AdapterSnapshotSupport::Unspecified {
            return Err(ResourceError::CorruptRecord(format!(
                "resource view update at LSN {event_lsn} has unspecified completeness"
            )));
        }
    }

    let mut identities = HashSet::new();
    for mutation in &state.mutations {
        let identity = ResourceIdentity::try_from_wire(mutation.identity.as_ref().ok_or_else(|| {
            ResourceError::CorruptRecord(format!(
                "resource mutation at LSN {event_lsn} is missing identity"
            ))
        })?)?;
        if identity.adapter_id() != adapter_id {
            return Err(ResourceError::CorruptLog(format!(
                "resource mutation at LSN {event_lsn} does not match source adapter"
            )));
        }
        if !view_keys.contains(identity.resource_kind()) {
            return Err(ResourceError::CorruptLog(format!(
                "resource mutation at LSN {event_lsn} has no matching view update"
            )));
        }
        if !identities.insert(identity.clone()) {
            return Err(ResourceError::CorruptRecord(format!(
                "resource event at LSN {event_lsn} mutates one identity more than once"
            )));
        }
        match mutation.mutation.as_ref().ok_or_else(|| {
            ResourceError::CorruptRecord(format!(
                "resource mutation at LSN {event_lsn} has no mutation variant"
            ))
        })? {
            resource_state_mutation::Mutation::Upsert(upsert) => {
                validate_upsert(upsert, event_lsn)?;
            }
            resource_state_mutation::Mutation::Unknown(ResourceStateUnknown {}) => {}
            resource_state_mutation::Mutation::Tombstone(tombstone) => {
                if let Some(replacement) = tombstone.replaced_by.as_ref() {
                    let replacement = ResourceIdentity::try_from_wire(replacement)?;
                    if replacement == identity || replacement.adapter_id() != adapter_id {
                        return Err(ResourceError::CorruptLog(format!(
                            "resource tombstone at LSN {event_lsn} has invalid replacement identity"
                        )));
                    }
                    let replacement_is_upsert = state.mutations.iter().any(|candidate| {
                        candidate
                            .identity
                            .as_ref()
                            .and_then(|wire| ResourceIdentity::try_from_wire(wire).ok())
                            .as_ref()
                            == Some(&replacement)
                            && matches!(
                                candidate.mutation,
                                Some(resource_state_mutation::Mutation::Upsert(_))
                            )
                    });
                    if !replacement_is_upsert {
                        return Err(ResourceError::CorruptLog(format!(
                            "resource tombstone at LSN {event_lsn} names a replacement without a matching upsert"
                        )));
                    }
                }
            }
            resource_state_mutation::Mutation::FreshnessChanged(change) => {
                validate_freshness_change(change, event_lsn)?;
            }
        }
    }
    Ok(())
}

fn validate_upsert(upsert: &ResourceStateUpsert, event_lsn: u64) -> Result<(), ResourceError> {
    for (role, envelope) in [
        ("resource", upsert.resource_payload.as_ref()),
        ("projection", upsert.projection_payload.as_ref()),
    ] {
        let envelope = envelope.ok_or_else(|| {
            ResourceError::CorruptRecord(format!(
                "resource upsert at LSN {event_lsn} is missing {role} payload"
            ))
        })?;
        let content_type = PayloadContentType::try_from(envelope.content_type).map_err(|_| {
            ResourceError::CorruptRecord(format!(
                "resource upsert at LSN {event_lsn} has unknown {role} content type"
            ))
        })?;
        if content_type == PayloadContentType::Unspecified || envelope.schema_ref.is_empty() {
            return Err(ResourceError::CorruptRecord(format!(
                "resource upsert at LSN {event_lsn} has incomplete {role} envelope"
            )));
        }
    }
    Ok(())
}

fn validate_freshness_change(
    change: &ResourceFreshnessChanged,
    event_lsn: u64,
) -> Result<(), ResourceError> {
    let from = freshness(change.from, event_lsn)?;
    let to = freshness(change.to, event_lsn)?;
    if from == to {
        return Err(ResourceError::CorruptLog(format!(
            "resource freshness change at LSN {event_lsn} is a no-op"
        )));
    }
    Ok(())
}

fn validate_from_revision(
    prior: Option<&ResourceRecord>,
    mutation: &ResourceStateMutation,
    event_lsn: u64,
) -> Result<(), ResourceError> {
    match (prior, mutation.from_revision_lsn.as_ref()) {
        (None, None) => Ok(()),
        (Some(record), Some(from)) if from.value == record.revision_lsn => Ok(()),
        (None, Some(from)) => Err(ResourceError::CorruptLog(format!(
            "resource mutation at LSN {event_lsn} expects unknown prior revision {}",
            from.value
        ))),
        (Some(record), Some(from)) => Err(ResourceError::CorruptLog(format!(
            "resource mutation at LSN {event_lsn} expects revision {}, projected revision is {}",
            from.value, record.revision_lsn
        ))),
        (Some(record), None) => Err(ResourceError::CorruptLog(format!(
            "resource mutation at LSN {event_lsn} omits prior revision {}",
            record.revision_lsn
        ))),
    }
}

fn apply_upsert(
    identity: ResourceIdentity,
    prior: Option<ResourceRecord>,
    upsert: &ResourceStateUpsert,
    generation: patchbay_contracts::patchbay::Generation,
    event_lsn: u64,
    observed_at: Timestamp,
) -> Result<ResourceRecord, ResourceError> {
    if prior.as_ref().is_some_and(ResourceRecord::tombstoned) {
        return Err(ResourceError::TerminalTombstone(identity));
    }
    Ok(ResourceRecord {
        identity,
        resource_payload: upsert.resource_payload.clone(),
        projection_payload: upsert.projection_payload.clone(),
        freshness: ResourceFreshnessState::Current,
        source_adapter_generation: generation,
        revision_lsn: event_lsn,
        observed_at,
        tombstoned_at_lsn: None,
        replaced_by: None,
    })
}

fn apply_unknown(
    identity: ResourceIdentity,
    prior: Option<ResourceRecord>,
    generation: patchbay_contracts::patchbay::Generation,
    event_lsn: u64,
    observed_at: Timestamp,
) -> Result<ResourceRecord, ResourceError> {
    if prior.as_ref().is_some_and(ResourceRecord::tombstoned) {
        return Err(ResourceError::TerminalTombstone(identity));
    }
    Ok(ResourceRecord {
        identity,
        resource_payload: None,
        projection_payload: None,
        freshness: ResourceFreshnessState::Unknown,
        source_adapter_generation: generation,
        revision_lsn: event_lsn,
        observed_at,
        tombstoned_at_lsn: None,
        replaced_by: None,
    })
}

fn apply_tombstone(
    identity: ResourceIdentity,
    prior: Option<ResourceRecord>,
    tombstone: &ResourceStateTombstone,
    generation: patchbay_contracts::patchbay::Generation,
    event_lsn: u64,
    observed_at: Timestamp,
) -> Result<ResourceRecord, ResourceError> {
    let prior = prior.ok_or_else(|| {
        ResourceError::CorruptLog(format!(
            "resource tombstone at LSN {event_lsn} targets an unknown identity"
        ))
    })?;
    if prior.tombstoned() {
        return Err(ResourceError::TerminalTombstone(identity));
    }
    let freshness = match (
        prior.resource_payload.as_ref(),
        prior.projection_payload.as_ref(),
    ) {
        (Some(_), Some(_)) => ResourceFreshnessState::Stale,
        (None, None) => ResourceFreshnessState::Unknown,
        _ => {
            return Err(ResourceError::CorruptLog(format!(
                "resource tombstone at LSN {event_lsn} has only one cached envelope"
            )))
        }
    };
    Ok(ResourceRecord {
        identity,
        resource_payload: prior.resource_payload,
        projection_payload: prior.projection_payload,
        freshness,
        source_adapter_generation: generation,
        revision_lsn: event_lsn,
        observed_at,
        tombstoned_at_lsn: Some(event_lsn),
        replaced_by: tombstone
            .replaced_by
            .as_ref()
            .map(ResourceIdentity::try_from_wire)
            .transpose()?,
    })
}

fn apply_freshness_change(
    identity: ResourceIdentity,
    prior: Option<ResourceRecord>,
    change: &ResourceFreshnessChanged,
    generation: patchbay_contracts::patchbay::Generation,
    event_lsn: u64,
    observed_at: Timestamp,
) -> Result<ResourceRecord, ResourceError> {
    let mut prior = prior.ok_or_else(|| {
        ResourceError::CorruptLog(format!(
            "resource freshness change at LSN {event_lsn} targets an unknown identity"
        ))
    })?;
    if prior.tombstoned() {
        return Err(ResourceError::TerminalTombstone(identity));
    }
    let from = freshness(change.from, event_lsn)?;
    if prior.freshness != from {
        return Err(ResourceError::CorruptLog(format!(
            "resource freshness change at LSN {event_lsn} expects {from:?}, projected freshness is {:?}",
            prior.freshness
        )));
    }
    let to = freshness(change.to, event_lsn)?;
    match to {
        ResourceFreshnessState::Unknown => {
            // UNKNOWN means Patchbay has no payload it can honestly classify;
            // use the explicit unknown semantics even for a normalized
            // freshness-only transition.
            prior.resource_payload = None;
            prior.projection_payload = None;
        }
        ResourceFreshnessState::Current | ResourceFreshnessState::Stale
            if prior.resource_payload.is_none() || prior.projection_payload.is_none() =>
        {
            return Err(ResourceError::CorruptLog(format!(
                "resource freshness change at LSN {event_lsn} would mark an empty payload {to:?}"
            )));
        }
        ResourceFreshnessState::Current | ResourceFreshnessState::Stale => {}
        ResourceFreshnessState::Unspecified => unreachable!("freshness() rejects unspecified"),
    }
    prior.freshness = to;
    prior.source_adapter_generation = generation;
    prior.revision_lsn = event_lsn;
    prior.observed_at = observed_at;
    Ok(prior)
}

fn freshness(raw: i32, event_lsn: u64) -> Result<ResourceFreshnessState, ResourceError> {
    let state = ResourceFreshnessState::try_from(raw).map_err(|_| {
        ResourceError::CorruptRecord(format!(
            "resource event at LSN {event_lsn} has unknown freshness {raw}"
        ))
    })?;
    if state == ResourceFreshnessState::Unspecified {
        return Err(ResourceError::CorruptRecord(format!(
            "resource event at LSN {event_lsn} has unspecified freshness"
        )));
    }
    Ok(state)
}

fn validate_timestamp(timestamp: &Timestamp, event_lsn: u64) -> Result<(), ResourceError> {
    if !(-62_135_596_800..=253_402_300_799).contains(&timestamp.seconds)
        || !(0..1_000_000_000).contains(&timestamp.nanos)
    {
        return Err(ResourceError::CorruptRecord(format!(
            "resource event at LSN {event_lsn} has invalid observed_at"
        )));
    }
    Ok(())
}

fn event_identity(event: &RecordedEvent) -> Result<(&AuthorityDomainId, u64), ResourceError> {
    let domain = event.event_id.authority_domain_id.as_ref().ok_or_else(|| {
        ResourceError::CorruptRecord("resource event has no authority domain".into())
    })?;
    if domain.value.is_empty() {
        return Err(ResourceError::CorruptRecord(
            "resource event has empty authority domain".into(),
        ));
    }
    let lsn = event
        .event_id
        .lsn
        .as_ref()
        .ok_or_else(|| ResourceError::CorruptRecord("resource event has no LSN".into()))?;
    Ok((domain, lsn.value))
}
