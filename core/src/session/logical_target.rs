//! Stable logical-target identity and exact external-runtime ownership.
//!
//! This projection is deliberately limited to identity. It has no Operation,
//! claim, evidence, target-resolution, or authority dependency. Session event
//! replay and checkpoint recovery call the same transition methods used by the
//! hot fold, so the reverse ownership index cannot weaken after restart.

use std::collections::{BTreeMap, HashMap};

use patchbay_contracts::patchbay::{
    AdapterId, AuthorityDomainId, ExternalRuntimeRef, LogicalTargetId,
    LogicalTargetProjectionRecord, LogicalTargetTombstone as WireLogicalTargetTombstone, Lsn,
    RuntimeGenerationRef,
};

const MAX_DEPLOYMENT_SCOPE_BYTES: usize = 256;

/// Exact authority-domain-qualified external runtime identity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExternalRuntimeKey {
    pub authority_domain_id: String,
    pub adapter_id: String,
    pub deployment_scope: String,
    pub runtime_session_id: String,
    pub generation: u64,
}

/// Audit-retained identity of a superseded or retired runtime generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalTargetTombstone {
    pub external_runtime_ref: ExternalRuntimeRef,
    pub superseded_at_lsn: u64,
}

/// Stable logical identity plus its mutually constrained runtime slots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalTargetRecord {
    pub logical_target_id: LogicalTargetId,
    pub adapter_id: AdapterId,
    pub deployment_scope: String,
    pub current: Option<RuntimeGenerationRef>,
    pub reserved_candidate: Option<ExternalRuntimeRef>,
    pub tombstones: BTreeMap<ExternalRuntimeKey, LogicalTargetTombstone>,
}

/// Read-only external-runtime ownership lookup.
pub trait ExternalRuntimeOwnership {
    fn owner_of(&self, external: &ExternalRuntimeRef) -> Option<&LogicalTargetId>;
}

/// Identity/projection validation failures.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LogicalTargetError {
    #[error("logical target authority_domain_id must not be empty")]
    EmptyAuthorityDomain,
    #[error("logical_target_id must not be empty")]
    EmptyLogicalTargetId,
    #[error("adapter_id must not be empty")]
    EmptyAdapterId,
    #[error("runtime_session_id must not be empty")]
    EmptyRuntimeSessionId,
    #[error("deployment_scope must be 1..={MAX_DEPLOYMENT_SCOPE_BYTES} printable ASCII bytes")]
    MalformedDeploymentScope,
    #[error("external runtime generation must be positive")]
    NonPositiveGeneration,
    #[error("logical target {0:?} already exists")]
    DuplicateLogicalTarget(LogicalTargetId),
    #[error("logical target {0:?} does not exist")]
    UnknownLogicalTarget(LogicalTargetId),
    #[error("logical target mutation changes its adapter or deployment scope")]
    TargetScopeMutation,
    #[error(
        "runtime-generation reference does not match the logical target or exact current runtime"
    )]
    RuntimeRefMismatch,
    #[error("logical target already has a current runtime")]
    CurrentAlreadyAssigned,
    #[error("logical target has no current runtime")]
    NoCurrentRuntime,
    #[error("logical target already has a reserved candidate")]
    CandidateAlreadyReserved,
    #[error("logical target has no matching reserved candidate")]
    ReservedCandidateMismatch,
    #[error("retired logical target cannot reserve another candidate")]
    RetiredTarget,
    #[error("superseded_at_lsn must be positive")]
    NonPositiveTombstoneLsn,
    #[error(
        "duplicate-native-reference: exact external runtime is owned by {owner:?}, not {attempted_owner:?}"
    )]
    DuplicateNativeReference {
        owner: LogicalTargetId,
        attempted_owner: LogicalTargetId,
    },
    #[error("logical-target checkpoint is inconsistent: {0}")]
    CorruptCheckpoint(String),
}

/// One authority-domain projection of stable logical targets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalTargetRegistry {
    authority_domain_id: AuthorityDomainId,
    records: HashMap<LogicalTargetId, LogicalTargetRecord>,
    external_owners: HashMap<ExternalRuntimeKey, LogicalTargetId>,
}

impl LogicalTargetRegistry {
    pub fn new(authority_domain_id: AuthorityDomainId) -> Result<Self, LogicalTargetError> {
        validate_authority_domain(&authority_domain_id)?;
        Ok(Self {
            authority_domain_id,
            records: HashMap::new(),
            external_owners: HashMap::new(),
        })
    }

    /// Rebuild the projection and reverse index from private checkpoint records.
    pub fn from_checkpoint(
        authority_domain_id: AuthorityDomainId,
        checkpoint_lsn: u64,
        records: Vec<LogicalTargetProjectionRecord>,
    ) -> Result<Self, LogicalTargetError> {
        if checkpoint_lsn == 0 {
            return Err(corrupt("checkpoint LSN is zero"));
        }
        let mut registry = Self::new(authority_domain_id)?;
        for wire in records {
            let record = registry.decode_checkpoint_record(checkpoint_lsn, wire)?;
            if registry.records.contains_key(&record.logical_target_id) {
                return Err(LogicalTargetError::CorruptCheckpoint(
                    "duplicate logical_target_id".to_owned(),
                ));
            }
            for external in record_external_refs(&record) {
                registry.reserve_external(&record.logical_target_id, external)?;
            }
            registry
                .records
                .insert(record.logical_target_id.clone(), record);
        }
        Ok(registry)
    }

    #[must_use]
    pub fn authority_domain_id(&self) -> &AuthorityDomainId {
        &self.authority_domain_id
    }

    pub fn records(&self) -> impl Iterator<Item = &LogicalTargetRecord> {
        self.records.values()
    }

    #[must_use]
    pub fn get(&self, logical_target_id: &LogicalTargetId) -> Option<&LogicalTargetRecord> {
        self.records.get(logical_target_id)
    }

    /// Create an empty stable logical target. Runtime identity is assigned by a
    /// separate exact transition and never inferred from labels or paths.
    pub fn create(
        &mut self,
        logical_target_id: LogicalTargetId,
        adapter_id: AdapterId,
        deployment_scope: String,
    ) -> Result<(), LogicalTargetError> {
        validate_logical_target_id(&logical_target_id)?;
        validate_adapter_id(&adapter_id)?;
        validate_deployment_scope(&deployment_scope)?;
        if self.records.contains_key(&logical_target_id) {
            return Err(LogicalTargetError::DuplicateLogicalTarget(
                logical_target_id,
            ));
        }
        self.records.insert(
            logical_target_id.clone(),
            LogicalTargetRecord {
                logical_target_id,
                adapter_id,
                deployment_scope,
                current: None,
                reserved_candidate: None,
                tombstones: BTreeMap::new(),
            },
        );
        Ok(())
    }

    /// Assign a discovered/pre-provisioned runtime as the first current slot.
    pub fn assign_initial_current(
        &mut self,
        logical_target_id: &LogicalTargetId,
        external: ExternalRuntimeRef,
    ) -> Result<(), LogicalTargetError> {
        self.validate_external_for_target(logical_target_id, &external)?;
        let record = self.require_record(logical_target_id)?;
        if record.current.is_some() {
            return Err(LogicalTargetError::CurrentAlreadyAssigned);
        }
        if record.reserved_candidate.is_some() || !record.tombstones.is_empty() {
            return Err(LogicalTargetError::RuntimeRefMismatch);
        }
        self.reserve_external(logical_target_id, &external)?;
        let record = self.require_record_mut(logical_target_id)?;
        record.current = Some(runtime_generation_ref(logical_target_id, external));
        Ok(())
    }

    /// Reserve one exact staged runtime for this target. A second logical owner
    /// is rejected before either the slot or reverse index mutates.
    pub fn reserve_candidate(
        &mut self,
        logical_target_id: &LogicalTargetId,
        external: ExternalRuntimeRef,
    ) -> Result<(), LogicalTargetError> {
        self.validate_external_for_target(logical_target_id, &external)?;
        let record = self.require_record(logical_target_id)?;
        if record.reserved_candidate.is_some() {
            return Err(LogicalTargetError::CandidateAlreadyReserved);
        }
        if record.current.is_none() && !record.tombstones.is_empty() {
            return Err(LogicalTargetError::RetiredTarget);
        }
        if record
            .current
            .as_ref()
            .is_some_and(|current| current.external_runtime.as_ref() == Some(&external))
        {
            return Err(LogicalTargetError::RuntimeRefMismatch);
        }
        self.reserve_external(logical_target_id, &external)?;
        self.require_record_mut(logical_target_id)?
            .reserved_candidate = Some(external);
        Ok(())
    }

    /// Release the exact reserved slot. This transition is identity-only; the
    /// downstream claim/effect contract decides when release is permitted.
    pub fn release_candidate(
        &mut self,
        logical_target_id: &LogicalTargetId,
        expected: &ExternalRuntimeRef,
    ) -> Result<(), LogicalTargetError> {
        self.validate_external_for_target(logical_target_id, expected)?;
        if self
            .require_record(logical_target_id)?
            .reserved_candidate
            .as_ref()
            != Some(expected)
        {
            return Err(LogicalTargetError::ReservedCandidateMismatch);
        }
        let key = self.external_key(expected)?;
        self.require_record_mut(logical_target_id)?
            .reserved_candidate = None;
        self.external_owners.remove(&key);
        Ok(())
    }

    /// Install the exact reserved candidate as current. For replacement, the
    /// prior exact runtime becomes a retained tombstone at `event_lsn`.
    pub fn commit_reserved_candidate(
        &mut self,
        logical_target_id: &LogicalTargetId,
        expected_current: Option<&RuntimeGenerationRef>,
        candidate: &ExternalRuntimeRef,
        event_lsn: u64,
    ) -> Result<(), LogicalTargetError> {
        if event_lsn == 0 {
            return Err(LogicalTargetError::NonPositiveTombstoneLsn);
        }
        self.validate_external_for_target(logical_target_id, candidate)?;
        let record = self.require_record(logical_target_id)?;
        if record.reserved_candidate.as_ref() != Some(candidate) {
            return Err(LogicalTargetError::ReservedCandidateMismatch);
        }
        validate_expected_current(logical_target_id, record.current.as_ref(), expected_current)?;

        let prior = record.current.clone();
        let prior_tombstone = prior.as_ref().map(|current| {
            let external = current
                .external_runtime
                .clone()
                .expect("validated current runtime reference");
            let key = self
                .external_key(&external)
                .expect("validated current runtime reference");
            (
                key,
                LogicalTargetTombstone {
                    external_runtime_ref: external,
                    superseded_at_lsn: event_lsn,
                },
            )
        });

        let record = self.require_record_mut(logical_target_id)?;
        record.current = Some(runtime_generation_ref(logical_target_id, candidate.clone()));
        record.reserved_candidate = None;
        if let Some((key, tombstone)) = prior_tombstone {
            record.tombstones.insert(key, tombstone);
        }
        Ok(())
    }

    /// Retire the exact current runtime while retaining its ownership
    /// reservation for audit and late-event correlation.
    pub fn tombstone_current(
        &mut self,
        expected_current: &RuntimeGenerationRef,
        event_lsn: u64,
    ) -> Result<(), LogicalTargetError> {
        if event_lsn == 0 {
            return Err(LogicalTargetError::NonPositiveTombstoneLsn);
        }
        let logical_target_id = expected_current
            .logical_target_id
            .as_ref()
            .ok_or(LogicalTargetError::RuntimeRefMismatch)?;
        validate_logical_target_id(logical_target_id)?;
        let record = self.require_record(logical_target_id)?;
        validate_expected_current(
            logical_target_id,
            record.current.as_ref(),
            Some(expected_current),
        )?;
        if record.reserved_candidate.is_some() {
            return Err(LogicalTargetError::CandidateAlreadyReserved);
        }
        let external = expected_current
            .external_runtime
            .clone()
            .ok_or(LogicalTargetError::RuntimeRefMismatch)?;
        let key = self.external_key(&external)?;
        let record = self.require_record_mut(logical_target_id)?;
        record.current = None;
        record.tombstones.insert(
            key,
            LogicalTargetTombstone {
                external_runtime_ref: external,
                superseded_at_lsn: event_lsn,
            },
        );
        Ok(())
    }

    /// Deterministically encode the complete private checkpoint projection.
    #[must_use]
    pub fn checkpoint_records(&self) -> Vec<LogicalTargetProjectionRecord> {
        let mut records: Vec<_> = self.records.values().collect();
        records.sort_by(|left, right| {
            left.logical_target_id
                .value
                .cmp(&right.logical_target_id.value)
        });
        records
            .into_iter()
            .map(|record| LogicalTargetProjectionRecord {
                logical_target_id: Some(record.logical_target_id.clone()),
                adapter_id: Some(record.adapter_id.clone()),
                deployment_scope: record.deployment_scope.clone(),
                current: record.current.clone(),
                reserved_candidate: record.reserved_candidate.clone(),
                tombstones: record
                    .tombstones
                    .values()
                    .map(|tombstone| WireLogicalTargetTombstone {
                        external_runtime_ref: Some(tombstone.external_runtime_ref.clone()),
                        superseded_at_lsn: Some(Lsn {
                            value: tombstone.superseded_at_lsn,
                        }),
                    })
                    .collect(),
            })
            .collect()
    }

    fn decode_checkpoint_record(
        &self,
        checkpoint_lsn: u64,
        wire: LogicalTargetProjectionRecord,
    ) -> Result<LogicalTargetRecord, LogicalTargetError> {
        let logical_target_id = wire
            .logical_target_id
            .ok_or_else(|| corrupt("missing logical_target_id"))?;
        let adapter_id = wire
            .adapter_id
            .ok_or_else(|| corrupt("missing adapter_id"))?;
        validate_logical_target_id(&logical_target_id)?;
        validate_adapter_id(&adapter_id)?;
        validate_deployment_scope(&wire.deployment_scope)?;

        let mut record = LogicalTargetRecord {
            logical_target_id: logical_target_id.clone(),
            adapter_id,
            deployment_scope: wire.deployment_scope,
            current: wire.current,
            reserved_candidate: wire.reserved_candidate,
            tombstones: BTreeMap::new(),
        };
        if let Some(current) = record.current.as_ref() {
            validate_runtime_generation_ref(&logical_target_id, current)?;
            let external = current
                .external_runtime
                .as_ref()
                .expect("validated runtime-generation reference");
            validate_external_against_record(&record, external)?;
        }
        if let Some(candidate) = record.reserved_candidate.as_ref() {
            validate_external_ref(candidate)?;
            validate_external_against_record(&record, candidate)?;
            if record
                .current
                .as_ref()
                .is_some_and(|current| current.external_runtime.as_ref() == Some(candidate))
            {
                return Err(corrupt("current and reserved candidate are identical"));
            }
        }
        for tombstone in wire.tombstones {
            let external = tombstone
                .external_runtime_ref
                .ok_or_else(|| corrupt("tombstone is missing external_runtime_ref"))?;
            validate_external_ref(&external)?;
            validate_external_against_record(&record, &external)?;
            let superseded_at_lsn = tombstone
                .superseded_at_lsn
                .filter(|lsn| lsn.value > 0 && lsn.value <= checkpoint_lsn)
                .ok_or_else(|| corrupt("tombstone LSN is outside the checkpoint's durable prefix"))?
                .value;
            if record
                .current
                .as_ref()
                .is_some_and(|current| current.external_runtime.as_ref() == Some(&external))
                || record.reserved_candidate.as_ref() == Some(&external)
            {
                return Err(corrupt("tombstone overlaps a live identity slot"));
            }
            let key = self.external_key(&external)?;
            if record
                .tombstones
                .insert(
                    key,
                    LogicalTargetTombstone {
                        external_runtime_ref: external,
                        superseded_at_lsn,
                    },
                )
                .is_some()
            {
                return Err(corrupt("duplicate tombstone external runtime"));
            }
        }
        let mut lineage: Vec<_> = record
            .tombstones
            .values()
            .map(|tombstone| {
                (
                    tombstone
                        .external_runtime_ref
                        .generation
                        .expect("checkpoint tombstone generation validated")
                        .value,
                    tombstone.superseded_at_lsn,
                )
            })
            .collect();
        lineage.sort_unstable();
        if lineage
            .windows(2)
            .any(|pair| pair[0].0 == pair[1].0 || pair[0].1 >= pair[1].1)
        {
            return Err(corrupt(
                "tombstone lineage has duplicate generations or non-increasing promotion LSNs",
            ));
        }
        if let Some(current_generation) = record
            .current
            .as_ref()
            .and_then(|current| current.external_runtime.as_ref())
            .and_then(|external| external.generation)
        {
            if lineage
                .last()
                .is_some_and(|(generation, _)| *generation >= current_generation.value)
            {
                return Err(corrupt(
                    "current runtime generation does not advance retained tombstones",
                ));
            }
        }
        if record.current.is_none()
            && record.reserved_candidate.is_some()
            && !record.tombstones.is_empty()
        {
            return Err(corrupt("retired target retains a reserved candidate"));
        }
        Ok(record)
    }

    fn validate_external_for_target(
        &self,
        logical_target_id: &LogicalTargetId,
        external: &ExternalRuntimeRef,
    ) -> Result<(), LogicalTargetError> {
        validate_logical_target_id(logical_target_id)?;
        validate_external_ref(external)?;
        validate_external_against_record(self.require_record(logical_target_id)?, external)
    }

    fn reserve_external(
        &mut self,
        logical_target_id: &LogicalTargetId,
        external: &ExternalRuntimeRef,
    ) -> Result<(), LogicalTargetError> {
        let key = self.external_key(external)?;
        if let Some(owner) = self.external_owners.get(&key) {
            return if owner == logical_target_id {
                Err(LogicalTargetError::RuntimeRefMismatch)
            } else {
                Err(LogicalTargetError::DuplicateNativeReference {
                    owner: owner.clone(),
                    attempted_owner: logical_target_id.clone(),
                })
            };
        }
        self.external_owners.insert(key, logical_target_id.clone());
        Ok(())
    }

    fn external_key(
        &self,
        external: &ExternalRuntimeRef,
    ) -> Result<ExternalRuntimeKey, LogicalTargetError> {
        external_runtime_key(&self.authority_domain_id, external)
    }

    fn require_record(
        &self,
        logical_target_id: &LogicalTargetId,
    ) -> Result<&LogicalTargetRecord, LogicalTargetError> {
        self.records
            .get(logical_target_id)
            .ok_or_else(|| LogicalTargetError::UnknownLogicalTarget(logical_target_id.clone()))
    }

    fn require_record_mut(
        &mut self,
        logical_target_id: &LogicalTargetId,
    ) -> Result<&mut LogicalTargetRecord, LogicalTargetError> {
        self.records
            .get_mut(logical_target_id)
            .ok_or_else(|| LogicalTargetError::UnknownLogicalTarget(logical_target_id.clone()))
    }
}

impl ExternalRuntimeOwnership for LogicalTargetRegistry {
    fn owner_of(&self, external: &ExternalRuntimeRef) -> Option<&LogicalTargetId> {
        self.external_key(external)
            .ok()
            .and_then(|key| self.external_owners.get(&key))
    }
}

fn record_external_refs(record: &LogicalTargetRecord) -> Vec<&ExternalRuntimeRef> {
    let mut refs = Vec::with_capacity(2 + record.tombstones.len());
    if let Some(current) = record.current.as_ref() {
        refs.push(
            current
                .external_runtime
                .as_ref()
                .expect("checkpoint current validated before indexing"),
        );
    }
    if let Some(candidate) = record.reserved_candidate.as_ref() {
        refs.push(candidate);
    }
    refs.extend(
        record
            .tombstones
            .values()
            .map(|tombstone| &tombstone.external_runtime_ref),
    );
    refs
}

fn validate_expected_current(
    logical_target_id: &LogicalTargetId,
    projected: Option<&RuntimeGenerationRef>,
    expected: Option<&RuntimeGenerationRef>,
) -> Result<(), LogicalTargetError> {
    if let Some(expected) = expected {
        validate_runtime_generation_ref(logical_target_id, expected)?;
    }
    if projected == expected {
        Ok(())
    } else {
        Err(LogicalTargetError::RuntimeRefMismatch)
    }
}

fn runtime_generation_ref(
    logical_target_id: &LogicalTargetId,
    external: ExternalRuntimeRef,
) -> RuntimeGenerationRef {
    RuntimeGenerationRef {
        logical_target_id: Some(logical_target_id.clone()),
        external_runtime: Some(external),
    }
}

/// Validate and qualify one exact external runtime for reverse-index use.
pub fn external_runtime_key(
    authority_domain_id: &AuthorityDomainId,
    external: &ExternalRuntimeRef,
) -> Result<ExternalRuntimeKey, LogicalTargetError> {
    validate_authority_domain(authority_domain_id)?;
    validate_external_ref(external)?;
    Ok(ExternalRuntimeKey {
        authority_domain_id: authority_domain_id.value.clone(),
        adapter_id: external
            .adapter_id
            .as_ref()
            .expect("validated external adapter id")
            .value
            .clone(),
        deployment_scope: external.deployment_scope.clone(),
        runtime_session_id: external
            .runtime_session_id
            .as_ref()
            .expect("validated external runtime id")
            .value
            .clone(),
        generation: external
            .generation
            .expect("validated external generation")
            .value,
    })
}

fn validate_runtime_generation_ref(
    logical_target_id: &LogicalTargetId,
    runtime: &RuntimeGenerationRef,
) -> Result<(), LogicalTargetError> {
    if runtime.logical_target_id.as_ref() != Some(logical_target_id) {
        return Err(LogicalTargetError::RuntimeRefMismatch);
    }
    validate_external_ref(
        runtime
            .external_runtime
            .as_ref()
            .ok_or(LogicalTargetError::RuntimeRefMismatch)?,
    )
}

fn validate_external_against_record(
    record: &LogicalTargetRecord,
    external: &ExternalRuntimeRef,
) -> Result<(), LogicalTargetError> {
    if external.adapter_id.as_ref() != Some(&record.adapter_id)
        || external.deployment_scope != record.deployment_scope
    {
        Err(LogicalTargetError::TargetScopeMutation)
    } else {
        Ok(())
    }
}

fn validate_external_ref(external: &ExternalRuntimeRef) -> Result<(), LogicalTargetError> {
    validate_adapter_id(
        external
            .adapter_id
            .as_ref()
            .ok_or(LogicalTargetError::EmptyAdapterId)?,
    )?;
    validate_deployment_scope(&external.deployment_scope)?;
    let runtime_session_id = external
        .runtime_session_id
        .as_ref()
        .ok_or(LogicalTargetError::EmptyRuntimeSessionId)?;
    if runtime_session_id.value.is_empty() {
        return Err(LogicalTargetError::EmptyRuntimeSessionId);
    }
    if external
        .generation
        .is_none_or(|generation| generation.value == 0)
    {
        return Err(LogicalTargetError::NonPositiveGeneration);
    }
    Ok(())
}

fn validate_authority_domain(
    authority_domain_id: &AuthorityDomainId,
) -> Result<(), LogicalTargetError> {
    if authority_domain_id.value.is_empty() {
        Err(LogicalTargetError::EmptyAuthorityDomain)
    } else {
        Ok(())
    }
}

fn validate_logical_target_id(
    logical_target_id: &LogicalTargetId,
) -> Result<(), LogicalTargetError> {
    if logical_target_id.value.is_empty() {
        Err(LogicalTargetError::EmptyLogicalTargetId)
    } else {
        Ok(())
    }
}

fn validate_adapter_id(adapter_id: &AdapterId) -> Result<(), LogicalTargetError> {
    if adapter_id.value.is_empty() {
        Err(LogicalTargetError::EmptyAdapterId)
    } else {
        Ok(())
    }
}

fn validate_deployment_scope(scope: &str) -> Result<(), LogicalTargetError> {
    if scope.is_empty()
        || scope.len() > MAX_DEPLOYMENT_SCOPE_BYTES
        || !scope.bytes().all(|byte| byte.is_ascii_graphic())
    {
        Err(LogicalTargetError::MalformedDeploymentScope)
    } else {
        Ok(())
    }
}

fn corrupt(message: &str) -> LogicalTargetError {
    LogicalTargetError::CorruptCheckpoint(message.to_owned())
}
