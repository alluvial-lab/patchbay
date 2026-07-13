//! Canonical session identity and state-axis transitions.

use patchbay_contracts::patchbay::{
    AdapterId, Generation, RuntimeSessionId, SessionActivityState, SessionConnectivityState,
    SessionState,
};

/// The canonical session identity tuple from `docs/PROTOCOL.md` § "Sessions".
///
/// Project, working-directory, and display-name labels are deliberately absent:
/// metadata changes cannot alter identity or redirect an operation to another
/// session.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionIdentity {
    pub adapter_id: AdapterId,
    pub deployment_scope: String,
    pub runtime_session_id: RuntimeSessionId,
    pub session_generation: Generation,
}

/// Return whether the protocol permits a connectivity observation from `from`
/// to `to`.
///
/// This match is the implementation single source of truth for the adjacency
/// in `docs/PROTOCOL.md` § "Session state axes". `Unspecified` represents the
/// pre-observation state and may move to any protocol connectivity state. No
/// observed state may transition back to `Unspecified`.
#[must_use]
pub const fn allowed_connectivity_transition(
    from: SessionConnectivityState,
    to: SessionConnectivityState,
) -> bool {
    use SessionConnectivityState::{Failed, Live, Offline, Stale, Unknown, Unspecified};

    match from {
        Unspecified => matches!(to, Live | Stale | Offline | Unknown | Failed),
        Unknown => matches!(to, Live | Stale | Offline | Failed),
        Live => matches!(to, Stale | Offline | Failed),
        Stale => matches!(to, Live | Offline | Unknown | Failed),
        Offline => matches!(to, Live | Stale | Unknown | Failed),
        Failed => matches!(to, Live | Stale | Offline | Unknown),
    }
}

/// Return whether the protocol permits an activity observation from `from` to
/// `to`.
///
/// This match is the implementation single source of truth for the adjacency
/// in `docs/PROTOCOL.md` § "Session state axes". `Unspecified` represents the
/// pre-observation state and may move to any protocol activity state. No
/// observed state may transition back to `Unspecified`.
#[must_use]
pub const fn allowed_activity_transition(
    from: SessionActivityState,
    to: SessionActivityState,
) -> bool {
    use SessionActivityState::{Idle, Unknown, Unspecified, Working};

    match from {
        Unspecified => matches!(to, Idle | Working | Unknown),
        Unknown => matches!(to, Idle | Working),
        Idle => matches!(to, Working | Unknown),
        Working => matches!(to, Idle | Unknown),
    }
}

/// Return the connectivity that presentation must render for `state`.
///
/// Connectivity is authoritative over activity: stale working remains stale,
/// and unknown working remains unknown. Prost stores enum fields as raw `i32`;
/// an unrecognized wire value receives prost's normal enum-field fallback of
/// `Unspecified` and must be rejected by boundary validation before this pure
/// presentation projection is used.
#[must_use]
pub fn effective_connectivity(state: SessionState) -> SessionConnectivityState {
    let connectivity = SessionConnectivityState::try_from(state.connectivity)
        .unwrap_or(SessionConnectivityState::Unspecified);

    match connectivity {
        SessionConnectivityState::Stale | SessionConnectivityState::Unknown => connectivity,
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONNECTIVITY_STATES: [SessionConnectivityState; 6] = [
        SessionConnectivityState::Unspecified,
        SessionConnectivityState::Live,
        SessionConnectivityState::Stale,
        SessionConnectivityState::Offline,
        SessionConnectivityState::Unknown,
        SessionConnectivityState::Failed,
    ];

    const CONNECTIVITY_TRANSITIONS: [(SessionConnectivityState, SessionConnectivityState); 24] = [
        (
            SessionConnectivityState::Unspecified,
            SessionConnectivityState::Live,
        ),
        (
            SessionConnectivityState::Unspecified,
            SessionConnectivityState::Stale,
        ),
        (
            SessionConnectivityState::Unspecified,
            SessionConnectivityState::Offline,
        ),
        (
            SessionConnectivityState::Unspecified,
            SessionConnectivityState::Unknown,
        ),
        (
            SessionConnectivityState::Unspecified,
            SessionConnectivityState::Failed,
        ),
        (
            SessionConnectivityState::Unknown,
            SessionConnectivityState::Live,
        ),
        (
            SessionConnectivityState::Unknown,
            SessionConnectivityState::Stale,
        ),
        (
            SessionConnectivityState::Unknown,
            SessionConnectivityState::Offline,
        ),
        (
            SessionConnectivityState::Unknown,
            SessionConnectivityState::Failed,
        ),
        (
            SessionConnectivityState::Live,
            SessionConnectivityState::Stale,
        ),
        (
            SessionConnectivityState::Live,
            SessionConnectivityState::Offline,
        ),
        (
            SessionConnectivityState::Live,
            SessionConnectivityState::Failed,
        ),
        (
            SessionConnectivityState::Stale,
            SessionConnectivityState::Live,
        ),
        (
            SessionConnectivityState::Stale,
            SessionConnectivityState::Offline,
        ),
        (
            SessionConnectivityState::Stale,
            SessionConnectivityState::Unknown,
        ),
        (
            SessionConnectivityState::Stale,
            SessionConnectivityState::Failed,
        ),
        (
            SessionConnectivityState::Offline,
            SessionConnectivityState::Live,
        ),
        (
            SessionConnectivityState::Offline,
            SessionConnectivityState::Stale,
        ),
        (
            SessionConnectivityState::Offline,
            SessionConnectivityState::Unknown,
        ),
        (
            SessionConnectivityState::Offline,
            SessionConnectivityState::Failed,
        ),
        (
            SessionConnectivityState::Failed,
            SessionConnectivityState::Live,
        ),
        (
            SessionConnectivityState::Failed,
            SessionConnectivityState::Stale,
        ),
        (
            SessionConnectivityState::Failed,
            SessionConnectivityState::Offline,
        ),
        (
            SessionConnectivityState::Failed,
            SessionConnectivityState::Unknown,
        ),
    ];

    const ACTIVITY_STATES: [SessionActivityState; 4] = [
        SessionActivityState::Unspecified,
        SessionActivityState::Idle,
        SessionActivityState::Working,
        SessionActivityState::Unknown,
    ];

    const ACTIVITY_TRANSITIONS: [(SessionActivityState, SessionActivityState); 9] = [
        (
            SessionActivityState::Unspecified,
            SessionActivityState::Idle,
        ),
        (
            SessionActivityState::Unspecified,
            SessionActivityState::Working,
        ),
        (
            SessionActivityState::Unspecified,
            SessionActivityState::Unknown,
        ),
        (SessionActivityState::Unknown, SessionActivityState::Idle),
        (SessionActivityState::Unknown, SessionActivityState::Working),
        (SessionActivityState::Idle, SessionActivityState::Working),
        (SessionActivityState::Idle, SessionActivityState::Unknown),
        (SessionActivityState::Working, SessionActivityState::Idle),
        (SessionActivityState::Working, SessionActivityState::Unknown),
    ];

    #[test]
    fn connectivity_adjacency_matches_every_protocol_table_cell() {
        for from in CONNECTIVITY_STATES {
            for to in CONNECTIVITY_STATES {
                let expected = CONNECTIVITY_TRANSITIONS.contains(&(from, to));
                assert_eq!(
                    allowed_connectivity_transition(from, to),
                    expected,
                    "unexpected protocol adjacency result for {from:?} -> {to:?}"
                );
            }
        }
    }

    #[test]
    fn activity_adjacency_matches_every_protocol_table_cell() {
        for from in ACTIVITY_STATES {
            for to in ACTIVITY_STATES {
                let expected = ACTIVITY_TRANSITIONS.contains(&(from, to));
                assert_eq!(
                    allowed_activity_transition(from, to),
                    expected,
                    "unexpected protocol adjacency result for {from:?} -> {to:?}"
                );
            }
        }
    }

    #[test]
    fn session_identity_contains_only_canonical_identity_fields() {
        let left = SessionIdentity {
            adapter_id: AdapterId {
                value: "pi".to_owned(),
            },
            deployment_scope: "machine-a".to_owned(),
            runtime_session_id: RuntimeSessionId {
                value: "runtime-1".to_owned(),
            },
            session_generation: Generation { value: 7 },
        };
        let right = left.clone();

        let left_labels = ("project-a", "/work/a", "display-a");
        let right_labels = ("project-b", "/work/b", "display-b");
        assert_ne!(left_labels, right_labels);
        assert_eq!(left, right);
    }

    #[test]
    fn stale_and_unknown_connectivity_dominate_every_activity() {
        for connectivity in [
            SessionConnectivityState::Stale,
            SessionConnectivityState::Unknown,
        ] {
            for activity in ACTIVITY_STATES {
                let state = SessionState {
                    connectivity: connectivity as i32,
                    activity: activity as i32,
                };
                assert_eq!(effective_connectivity(state), connectivity);
            }
        }
    }

    #[test]
    fn effective_connectivity_preserves_other_connectivity_states() {
        for connectivity in [
            SessionConnectivityState::Unspecified,
            SessionConnectivityState::Live,
            SessionConnectivityState::Offline,
            SessionConnectivityState::Failed,
        ] {
            let state = SessionState {
                connectivity: connectivity as i32,
                activity: SessionActivityState::Working as i32,
            };
            assert_eq!(effective_connectivity(state), connectivity);
        }
    }
}
