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
    /\ browserLocalSessionLive = FALSE
    /\ browserLocalGrantClaim = FALSE

(*
  @type: (() => Bool);
*)
csrf_rejects_unauthenticated == accepted => lastSession \in operatorSessions

(*
  @type: (() => Bool);
*)
revoked_session_cannot_command ==
  accepted => ~(sessionStatus[lastSession] \in DEAD_STATUSES)

(*
  @type: ((Str) => Bool);
*)
authenticated(session_43) == session_43 \in operatorSessions

(*
  @type: ((Str) => Bool);
*)
active(session_51) == sessionStatus[session_51] = "active"

(*
  @type: ((Str, Str) => Bool);
*)
validCsrfProof(session_65, proof_65) ==
  session_65 \in DOMAIN csrfProofs /\ proof_65 = csrfProofs[session_65]

(*
  @type: (() => Bool);
*)
csrf_rejects_missing_proof == accepted => validCsrfProof(lastSession, lastProof)

(*
  @type: ((Str, Str) => Bool);
*)
serverAccepts(session_78, proof_78) ==
  (authenticated(session_78) /\ active(session_78))
    /\ validCsrfProof(session_78, proof_78)

(*
  @type: (() => Bool);
*)
q_init == init

(*
  @type: ((Str, Str, Bool, Bool) => Bool);
*)
submitStateChangingRequest(session_170, proof_170, uiSaysSessionLive_170, uiSaysGrantPresent_170) ==
  session_170 \in SESSION_IDS
    /\ proof_170 \in PROOFS
    /\ accepted' := (serverAccepts(session_170, proof_170))
    /\ lastSession' := session_170
    /\ lastProof' := proof_170
    /\ browserLocalSessionLive' := uiSaysSessionLive_170
    /\ browserLocalGrantClaim' := uiSaysGrantPresent_170
    /\ operatorSessions' := operatorSessions
    /\ csrfProofs' := csrfProofs
    /\ sessionStatus' := sessionStatus

(*
  @type: (() => Bool);
*)
browser_local_state_not_authority ==
  accepted => serverAccepts(lastSession, lastProof)

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
