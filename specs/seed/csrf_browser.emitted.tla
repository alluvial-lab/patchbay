// GENERATED ARTIFACT — do not hand-edit. Regenerate via: quint compile csrf_browser.qnt --target tlaplus
// Source: csrf_browser.qnt. Inspection artifact, NOT an independent re-check lane (see feature-formal-model-seed Q4).

----------------------------- MODULE csrf_browser -----------------------------

EXTENDS Integers, Sequences, FiniteSets, TLC, Apalache, Variants

(*
  @type: (() => Set(Str));
*)
PROOFS ==
  { "proof_active",
    "proof_revoked",
    "proof_expired",
    "wrong_proof",
    "missing_proof" }

(*
  @type: (() => Set(Str));
*)
DEAD_STATUSES == { "revoked", "expired" }

VARIABLE
  (*
    @type: Set(Str);
  *)
  operatorSessions

VARIABLE
  (*
    @type: (Str -> Str);
  *)
  csrfProofs

VARIABLE
  (*
    @type: (Str -> Str);
  *)
  sessionStatus

VARIABLE
  (*
    @type: Bool;
  *)
  accepted

VARIABLE
  (*
    @type: Str;
  *)
  lastSession

VARIABLE
  (*
    @type: Str;
  *)
  lastProof

VARIABLE
  (*
    @type: Str;
  *)
  attemptedSession

VARIABLE
  (*
    @type: Str;
  *)
  attemptedProof

VARIABLE
  (*
    @type: Bool;
  *)
  requestPending

VARIABLE
  (*
    @type: Bool;
  *)
  browserLocalSessionLive

VARIABLE
  (*
    @type: Bool;
  *)
  browserLocalGrantClaim

(*
  @type: (() => Set(Str));
*)
SESSION_IDS == { "s_active", "s_revoked", "s_expired", "missing_session" }

(*
  @type: (() => Bool);
*)
init ==
  operatorSessions = { "s_active", "s_revoked", "s_expired" }
    /\ csrfProofs
      = SetAsFun({ <<"s_active", "proof_active">>,
        <<"s_revoked", "proof_revoked">>,
        <<"s_expired", "proof_expired">>,
        <<"missing_session", "wrong_proof">> })
    /\ sessionStatus
      = SetAsFun({ <<"s_active", "active">>,
        <<"s_revoked", "revoked">>,
        <<"s_expired", "expired">>,
        <<"missing_session", "active">> })
    /\ accepted = FALSE
    /\ lastSession = "missing_session"
    /\ lastProof = "missing_proof"
    /\ attemptedSession = "missing_session"
    /\ attemptedProof = "missing_proof"
    /\ requestPending = FALSE
    /\ browserLocalSessionLive = FALSE
    /\ browserLocalGrantClaim = FALSE

(*
  @type: ((Str, Str, Bool, Bool) => Bool);
*)
arriveRequest(session_192, proof_192, uiSaysSessionLive_192, uiSaysGrantPresent_192) ==
  session_192 \in SESSION_IDS
    /\ proof_192 \in PROOFS
    /\ attemptedSession' := session_192
    /\ attemptedProof' := proof_192
    /\ browserLocalSessionLive' := uiSaysSessionLive_192
    /\ browserLocalGrantClaim' := uiSaysGrantPresent_192
    /\ requestPending' := TRUE
    /\ accepted' := FALSE
    /\ lastSession' := lastSession
    /\ lastProof' := lastProof
    /\ operatorSessions' := operatorSessions
    /\ csrfProofs' := csrfProofs
    /\ sessionStatus' := sessionStatus

(*
  @type: (() => Bool);
*)
csrf_rejects_unauthenticated ==
  accepted => attemptedSession \in operatorSessions

(*
  @type: (() => Bool);
*)
csrf_rejects_missing_proof ==
  accepted
    => attemptedSession \in DOMAIN csrfProofs
      /\ attemptedProof = csrfProofs[attemptedSession]

(*
  @type: (() => Bool);
*)
revoked_session_cannot_command ==
  accepted => ~(sessionStatus[attemptedSession] \in DEAD_STATUSES)

(*
  @type: (() => Bool);
*)
browser_local_state_not_authority ==
  accepted
    => ((attemptedSession \in operatorSessions
          /\ sessionStatus[attemptedSession] = "active")
        /\ attemptedSession \in DOMAIN csrfProofs)
      /\ attemptedProof = csrfProofs[attemptedSession]

(*
  @type: ((Str) => Bool);
*)
authenticated(session_49) == session_49 \in operatorSessions

(*
  @type: ((Str) => Bool);
*)
active(session_57) == sessionStatus[session_57] = "active"

(*
  @type: ((Str, Str) => Bool);
*)
validCsrfProof(session_71, proof_71) ==
  session_71 \in DOMAIN csrfProofs /\ proof_71 = csrfProofs[session_71]

(*
  @type: ((Str, Str) => Bool);
*)
serverAccepts(session_84, proof_84) ==
  (authenticated(session_84) /\ active(session_84))
    /\ validCsrfProof(session_84, proof_84)

(*
  @type: (() => Bool);
*)
q_init == init

(*
  @type: (() => Bool);
*)
submitStateChangingRequest ==
  requestPending
    /\ accepted' := (serverAccepts(attemptedSession, attemptedProof))
    /\ lastSession' := attemptedSession
    /\ lastProof' := attemptedProof
    /\ requestPending' := FALSE
    /\ attemptedSession' := attemptedSession
    /\ attemptedProof' := attemptedProof
    /\ browserLocalSessionLive' := browserLocalSessionLive
    /\ browserLocalGrantClaim' := browserLocalGrantClaim
    /\ operatorSessions' := operatorSessions
    /\ csrfProofs' := csrfProofs
    /\ sessionStatus' := sessionStatus

(*
  @type: (() => Bool);
*)
step ==
  (\E session \in SESSION_IDS:
      \E proof \in PROOFS:
        \E uiSaysSessionLive \in { TRUE, FALSE }:
          \E uiSaysGrantPresent \in { TRUE, FALSE }:
            arriveRequest(session, proof, uiSaysSessionLive, uiSaysGrantPresent))
    \/ submitStateChangingRequest

(*
  @type: (() => Bool);
*)
q_step == step

================================================================================
