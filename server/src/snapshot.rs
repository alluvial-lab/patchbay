use patchbay_contracts::patchbay::{AuthorityDomainId, Generation, SessionSnapshot};
use patchbay_core::storage::StoredSnapshot;
use prost::Message;

const CHECKPOINT_MAGIC: &[u8] = b"\x89PATCHBAY-CHECKPOINT\r\n\x1a\n";
const CHECKPOINT_FORMAT_VERSION: u32 = 1;
const CHECKPOINT_VERSION_BYTES: usize = std::mem::size_of::<u32>();
const CHECKPOINT_KIND_BYTES: usize = std::mem::size_of::<u8>();
const CHECKPOINT_HEADER_BYTES: usize =
    CHECKPOINT_MAGIC.len() + CHECKPOINT_VERSION_BYTES + CHECKPOINT_KIND_BYTES;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum CheckpointKind {
    Session = 1,
    #[cfg(test)]
    Resource = 2,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SessionCheckpointRejection {
    Undiscriminated,
    UnsupportedVersion,
    WrongType,
    Decode,
    AuthorityDomain,
    CoreGeneration,
    Lsn,
}

impl std::fmt::Display for SessionCheckpointRejection {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Undiscriminated => {
                "session checkpoint is missing the typed checkpoint discriminator"
            }
            Self::UnsupportedVersion => "session checkpoint envelope version is unsupported",
            Self::WrongType => "checkpoint envelope does not contain a session snapshot",
            Self::Decode => "session checkpoint payload is not decodable",
            Self::AuthorityDomain => "session checkpoint has an invalid authority-domain anchor",
            Self::CoreGeneration => "session checkpoint has an invalid core-generation anchor",
            Self::Lsn => "session checkpoint LSN does not match its storage anchor",
        })
    }
}

impl std::error::Error for SessionCheckpointRejection {}

/// Encode a session snapshot for the durable checkpoint slot.
///
/// The storage payload is private checkpoint framing, not the public
/// `LoadSnapshotResponse.snapshot_payload`: the service removes this envelope
/// before returning the generated `SessionSnapshot` bytes. Legacy raw snapshot
/// bytes are intentionally not upgraded or dual-read; they are disposable
/// derived data and fall back to log materialization.
#[must_use]
pub fn encode_session_checkpoint(snapshot: &SessionSnapshot) -> Vec<u8> {
    encode_checkpoint(CheckpointKind::Session, &snapshot.encode_to_vec())
}

pub fn decode_compatible_session_checkpoint(
    stored: &StoredSnapshot,
    expected_domain: &AuthorityDomainId,
    expected_core_generation: &Generation,
) -> Result<SessionSnapshot, SessionCheckpointRejection> {
    if stored.event_id.authority_domain_id.as_ref() != Some(expected_domain) {
        return Err(SessionCheckpointRejection::AuthorityDomain);
    }
    let stored_lsn = stored
        .event_id
        .lsn
        .as_ref()
        .filter(|lsn| lsn.value > 0)
        .ok_or(SessionCheckpointRejection::Lsn)?;
    let payload = decode_session_checkpoint_envelope(&stored.payload)?;
    let snapshot =
        SessionSnapshot::decode(payload).map_err(|_| SessionCheckpointRejection::Decode)?;
    if snapshot.authority_domain_id.as_ref() != Some(expected_domain) {
        return Err(SessionCheckpointRejection::AuthorityDomain);
    }
    if expected_core_generation.value == 0
        || snapshot.core_generation.as_ref().is_none_or(|generation| {
            generation.value == 0 || generation != expected_core_generation
        })
    {
        return Err(SessionCheckpointRejection::CoreGeneration);
    }
    if snapshot.snapshot_lsn.as_ref() != Some(stored_lsn) {
        return Err(SessionCheckpointRejection::Lsn);
    }
    Ok(snapshot)
}

fn encode_checkpoint(kind: CheckpointKind, payload: &[u8]) -> Vec<u8> {
    let mut encoded = Vec::with_capacity(CHECKPOINT_HEADER_BYTES + payload.len());
    encoded.extend_from_slice(CHECKPOINT_MAGIC);
    encoded.extend_from_slice(&CHECKPOINT_FORMAT_VERSION.to_be_bytes());
    encoded.push(kind as u8);
    encoded.extend_from_slice(payload);
    encoded
}

fn decode_session_checkpoint_envelope(encoded: &[u8]) -> Result<&[u8], SessionCheckpointRejection> {
    if encoded.len() < CHECKPOINT_HEADER_BYTES || !encoded.starts_with(CHECKPOINT_MAGIC) {
        return Err(SessionCheckpointRejection::Undiscriminated);
    }
    let version_offset = CHECKPOINT_MAGIC.len();
    let version_end = version_offset + CHECKPOINT_VERSION_BYTES;
    let version = u32::from_be_bytes(
        encoded[version_offset..version_end]
            .try_into()
            .expect("checkpoint version slice has a fixed width"),
    );
    if version != CHECKPOINT_FORMAT_VERSION {
        return Err(SessionCheckpointRejection::UnsupportedVersion);
    }
    if encoded[version_end] != CheckpointKind::Session as u8 {
        return Err(SessionCheckpointRejection::WrongType);
    }
    Ok(&encoded[CHECKPOINT_HEADER_BYTES..])
}

#[cfg(test)]
mod tests {
    use super::*;
    use patchbay_contracts::patchbay::{EventId, Lsn, ResourceSnapshot};

    fn domain(value: &str) -> AuthorityDomainId {
        AuthorityDomainId {
            value: value.to_owned(),
        }
    }

    fn checkpoint(snapshot: SessionSnapshot) -> StoredSnapshot {
        StoredSnapshot {
            event_id: EventId {
                authority_domain_id: Some(domain("main")),
                lsn: Some(Lsn { value: 7 }),
            },
            payload: encode_session_checkpoint(&snapshot),
        }
    }

    fn valid_snapshot() -> SessionSnapshot {
        SessionSnapshot {
            authority_domain_id: Some(domain("main")),
            snapshot_lsn: Some(Lsn { value: 7 }),
            core_generation: Some(Generation { value: 11 }),
            ..SessionSnapshot::default()
        }
    }

    #[test]
    fn compatible_checkpoint_requires_exact_domain_generation_and_lsn() {
        let stored = checkpoint(valid_snapshot());
        assert_eq!(
            decode_compatible_session_checkpoint(
                &stored,
                &domain("main"),
                &Generation { value: 11 },
            )
            .unwrap(),
            valid_snapshot()
        );
    }

    #[test]
    fn legacy_undiscriminated_snapshot_is_disposable() {
        let mut stored = checkpoint(valid_snapshot());
        stored.payload = valid_snapshot().encode_to_vec();
        assert_eq!(
            decode_compatible_session_checkpoint(
                &stored,
                &domain("main"),
                &Generation { value: 11 },
            ),
            Err(SessionCheckpointRejection::Undiscriminated)
        );
    }

    #[test]
    fn resource_snapshot_bytes_cannot_decode_as_a_session_checkpoint() {
        let resource = ResourceSnapshot {
            authority_domain_id: Some(domain("main")),
            snapshot_lsn: Some(Lsn { value: 7 }),
            core_generation: Some(Generation { value: 11 }),
            ..ResourceSnapshot::default()
        };
        let mut stored = checkpoint(valid_snapshot());

        stored.payload = resource.encode_to_vec();
        assert_eq!(
            decode_compatible_session_checkpoint(
                &stored,
                &domain("main"),
                &Generation { value: 11 },
            ),
            Err(SessionCheckpointRejection::Undiscriminated)
        );

        stored.payload = encode_checkpoint(CheckpointKind::Resource, &resource.encode_to_vec());
        assert_eq!(
            decode_compatible_session_checkpoint(
                &stored,
                &domain("main"),
                &Generation { value: 11 },
            ),
            Err(SessionCheckpointRejection::WrongType)
        );
    }

    #[test]
    fn corrupt_or_unsupported_envelope_is_rejected() {
        let mut stored = checkpoint(valid_snapshot());
        stored.payload = encode_checkpoint(CheckpointKind::Session, &[0xff]);
        assert_eq!(
            decode_compatible_session_checkpoint(
                &stored,
                &domain("main"),
                &Generation { value: 11 },
            ),
            Err(SessionCheckpointRejection::Decode)
        );

        stored = checkpoint(valid_snapshot());
        let version_offset = CHECKPOINT_MAGIC.len();
        stored.payload[version_offset..version_offset + CHECKPOINT_VERSION_BYTES]
            .copy_from_slice(&(CHECKPOINT_FORMAT_VERSION + 1).to_be_bytes());
        assert_eq!(
            decode_compatible_session_checkpoint(
                &stored,
                &domain("main"),
                &Generation { value: 11 },
            ),
            Err(SessionCheckpointRejection::UnsupportedVersion)
        );
    }

    #[test]
    fn missing_or_wrong_domain_is_rejected() {
        for embedded_domain in [None, Some(domain("other"))] {
            let mut snapshot = valid_snapshot();
            snapshot.authority_domain_id = embedded_domain;
            assert_eq!(
                decode_compatible_session_checkpoint(
                    &checkpoint(snapshot),
                    &domain("main"),
                    &Generation { value: 11 },
                ),
                Err(SessionCheckpointRejection::AuthorityDomain)
            );
        }
        let mut stored = checkpoint(valid_snapshot());
        stored.event_id.authority_domain_id = Some(domain("other"));
        assert_eq!(
            decode_compatible_session_checkpoint(
                &stored,
                &domain("main"),
                &Generation { value: 11 },
            ),
            Err(SessionCheckpointRejection::AuthorityDomain)
        );
        stored.event_id.authority_domain_id = None;
        assert_eq!(
            decode_compatible_session_checkpoint(
                &stored,
                &domain("main"),
                &Generation { value: 11 },
            ),
            Err(SessionCheckpointRejection::AuthorityDomain)
        );
    }

    #[test]
    fn missing_zero_or_different_generation_is_rejected() {
        for generation in [
            None,
            Some(Generation { value: 0 }),
            Some(Generation { value: 12 }),
        ] {
            let mut snapshot = valid_snapshot();
            snapshot.core_generation = generation;
            assert_eq!(
                decode_compatible_session_checkpoint(
                    &checkpoint(snapshot),
                    &domain("main"),
                    &Generation { value: 11 },
                ),
                Err(SessionCheckpointRejection::CoreGeneration)
            );
        }
    }

    #[test]
    fn missing_zero_or_mismatched_lsn_is_rejected() {
        for lsn in [None, Some(Lsn { value: 8 })] {
            let mut snapshot = valid_snapshot();
            snapshot.snapshot_lsn = lsn;
            assert_eq!(
                decode_compatible_session_checkpoint(
                    &checkpoint(snapshot),
                    &domain("main"),
                    &Generation { value: 11 },
                ),
                Err(SessionCheckpointRejection::Lsn)
            );
        }
        for stored_lsn in [None, Some(Lsn { value: 0 })] {
            let mut stored = checkpoint(valid_snapshot());
            stored.event_id.lsn = stored_lsn;
            assert_eq!(
                decode_compatible_session_checkpoint(
                    &stored,
                    &domain("main"),
                    &Generation { value: 11 },
                ),
                Err(SessionCheckpointRejection::Lsn)
            );
        }
    }
}
