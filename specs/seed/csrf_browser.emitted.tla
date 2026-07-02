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
    /\ browserLocalSessionLive = FALSE
    /\ browserLocalGrantClaim = FALSE

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
authenticated(session_47) == session_47 \in operatorSessions

(*
  @type: ((Str) => Bool);
*)
active(session_55) == sessionStatus[session_55] = "active"

(*
  @type: ((Str, Str) => Bool);
*)
validCsrfProof(session_69, proof_69) ==
  session_69 \in DOMAIN csrfProofs /\ proof_69 = csrfProofs[session_69]

(*
  @type: ((Str, Str) => Bool);
*)
serverAccepts(session_82, proof_82) ==
  (authenticated(session_82) /\ active(session_82))
    /\ validCsrfProof(session_82, proof_82)

(*
  @type: (() => Bool);
*)
q_init == init

(*
  @type: ((Str, Str, Bool, Bool) => Bool);
*)
submitStateChangingRequest(session_186, proof_186, uiSaysSessionLive_186, uiSaysGrantPresent_186) ==
  session_186 \in SESSION_IDS
    /\ proof_186 \in PROOFS
    /\ accepted' := (serverAccepts(session_186, proof_186))
    /\ lastSession' := session_186
    /\ lastProof' := proof_186
    /\ attemptedSession' := session_186
    /\ attemptedProof' := proof_186
    /\ browserLocalSessionLive' := uiSaysSessionLive_186
    /\ browserLocalGrantClaim' := uiSaysGrantPresent_186
    /\ operatorSessions' := operatorSessions
    /\ csrfProofs' := csrfProofs
    /\ sessionStatus' := sessionStatus

(*
  @type: (() => Bool);
*)
step ==
  \E session \in SESSION_IDS:
    \E proof \in PROOFS:
      \E uiSaysSessionLive \in { TRUE, FALSE }:
        \E uiSaysGrantPresent \in { TRUE, FALSE }:
          submitStateChangingRequest(session, proof, uiSaysSessionLive, uiSaysGrantPresent)

(*
  @type: (() => Bool);
*)
q_step == step

================================================================================
