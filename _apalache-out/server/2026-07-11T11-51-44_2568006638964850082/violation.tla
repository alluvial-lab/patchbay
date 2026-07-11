---------------------------- MODULE counterexample ----------------------------

EXTENDS csrf_browser

(* Constant initialization state *)
ConstInit == TRUE

(* Initial state [_transition(0)] *)
State0 ==
  accepted = FALSE
    /\ attemptedProof = "missing_proof"
    /\ attemptedSession = "missing_session"
    /\ browserLocalGrantClaim = FALSE
    /\ browserLocalSessionLive = FALSE
    /\ csrfProofs
      = SetAsFun({ <<"missing_session", "wrong_proof">>,
        <<"s_active", "proof_active">>,
        <<"s_expired", "proof_expired">>,
        <<"s_revoked", "proof_revoked">> })
    /\ lastProof = "missing_proof"
    /\ lastSession = "missing_session"
    /\ operatorSessions = { "s_active", "s_expired", "s_revoked" }
    /\ requestPending = FALSE
    /\ sessionStatus
      = SetAsFun({ <<"missing_session", "active">>,
        <<"s_active", "active">>,
        <<"s_expired", "expired">>,
        <<"s_revoked", "revoked">> })

(* State1 [_transition(1)] *)
State1 ==
  accepted = FALSE
    /\ attemptedProof = "proof_revoked"
    /\ attemptedSession = "s_active"
    /\ browserLocalGrantClaim = TRUE
    /\ browserLocalSessionLive = TRUE
    /\ csrfProofs
      = SetAsFun({ <<"missing_session", "wrong_proof">>,
        <<"s_active", "proof_active">>,
        <<"s_expired", "proof_expired">>,
        <<"s_revoked", "proof_revoked">> })
    /\ lastProof = "missing_proof"
    /\ lastSession = "missing_session"
    /\ operatorSessions = { "s_active", "s_expired", "s_revoked" }
    /\ requestPending = TRUE
    /\ sessionStatus
      = SetAsFun({ <<"missing_session", "active">>,
        <<"s_active", "active">>,
        <<"s_expired", "expired">>,
        <<"s_revoked", "revoked">> })

(* State2 [_transition(0)] *)
State2 ==
  accepted = TRUE
    /\ attemptedProof = "proof_revoked"
    /\ attemptedSession = "s_active"
    /\ browserLocalGrantClaim = TRUE
    /\ browserLocalSessionLive = TRUE
    /\ csrfProofs
      = SetAsFun({ <<"missing_session", "wrong_proof">>,
        <<"s_active", "proof_active">>,
        <<"s_expired", "proof_expired">>,
        <<"s_revoked", "proof_revoked">> })
    /\ lastProof = "proof_active"
    /\ lastSession = "s_active"
    /\ operatorSessions = { "s_active", "s_expired", "s_revoked" }
    /\ requestPending = FALSE
    /\ sessionStatus
      = SetAsFun({ <<"missing_session", "active">>,
        <<"s_active", "active">>,
        <<"s_expired", "expired">>,
        <<"s_revoked", "revoked">> })

(* The following formula holds true in the last state and violates the invariant *)
InvariantViolation ==
  accepted
    /\ (((~(attemptedSession \in operatorSessions)
          \/ ~(sessionStatus[attemptedSession] = "active"))
        \/ ~(attemptedSession \in DOMAIN csrfProofs))
      \/ ~(attemptedProof = csrfProofs[attemptedSession]))

================================================================================
(* Created by Apalache on Sat Jul 11 11:51:48 MDT 2026 *)
(* https://github.com/apalache-mc/apalache *)
