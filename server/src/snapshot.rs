use patchbay_contracts::patchbay::{AuthorityDomainId, Generation, SessionSnapshot};
use patchbay_core::storage::StoredSnapshot;
use prost::Message;

#[derive(Debug, PartialEq, Eq)]
pub enum SessionCheckpointRejection {
    Decode,
    AuthorityDomain,
    CoreGeneration,
    Lsn,
}

impl std::fmt::Display for SessionCheckpointRejection {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Decode => "session checkpoint payload is not decodable",
            Self::AuthorityDomain => "session checkpoint has an invalid authority-domain anchor",
            Self::CoreGeneration => "session checkpoint has an invalid core-generation anchor",
            Self::Lsn => "session checkpoint LSN does not match its storage anchor",
        })
    }
}

impl std::error::Error for SessionCheckpointRejection {}

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
        .ok_or(SessionCheckpointRejection::Lsn)?;
    let snapshot = SessionSnapshot::decode(stored.payload.as_slice())
        .map_err(|_| SessionCheckpointRejection::Decode)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use patchbay_contracts::patchbay::{EventId, Lsn};

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
            payload: snapshot.encode_to_vec(),
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
    fn corrupt_payload_is_rejected() {
        let mut stored = checkpoint(valid_snapshot());
        stored.payload = vec![0xff];
        assert_eq!(
            decode_compatible_session_checkpoint(
                &stored,
                &domain("main"),
                &Generation { value: 11 },
            ),
            Err(SessionCheckpointRejection::Decode)
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
    fn missing_or_mismatched_lsn_is_rejected() {
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
        let mut stored = checkpoint(valid_snapshot());
        stored.event_id.lsn = None;
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
