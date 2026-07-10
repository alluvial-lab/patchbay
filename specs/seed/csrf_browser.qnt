// Patchbay seed model: browser session and CSRF server-side boundary
//
// This model covers the SERVER-SIDE EFFECTS of browser session/CSRF evidence. It does
// not model browser cookie mechanics, SameSite behavior, token generation strength, or
// cryptographic secrecy. Instead, it models what docs/VERIFICATION.md calls the
// server-side effects of valid, missing, expired, and revoked session/CSRF evidence.
//
// Grounding: docs/SECURITY.md §CSRF says every state-changing web route must require
// an authenticated operator session cookie and a CSRF token tied to that operator
// session. It also treats browser-local state as non-authoritative: local UI claims
// cannot submit commands, grant authority, or keep a session live without server-side
// evidence.
//
// Genuine-checking discipline (load-bearing): submitStateChangingRequest is
// PERMISSIVE. It attempts requests from the full finite request space, including
// missing sessions, missing proofs, mismatched proofs, revoked sessions, expired
// sessions, and optimistic browser-local UI claims. The real server acceptance rule is
// applied inside the action; invariants prove that any accepted request met the
// server-side session and CSRF preconditions. The action guard never assumes the
// request is already valid.
module csrf_browser {
  // --- finite request evidence space ---
  pure val SESSION_IDS = Set("s_active", "s_revoked", "s_expired", "missing_session")
  pure val PROOFS = Set("proof_active", "proof_revoked", "proof_expired", "wrong_proof", "missing_proof")
  pure val DEAD_STATUSES = Set("revoked", "expired")

  // --- state required by the feature design ---
  var operatorSessions: Set[str]  // server-recognized authenticated operator-session ids
  var csrfProofs: str -> str      // session id -> server-bound CSRF proof
  var sessionStatus: str -> str   // session id -> "active" | "revoked" | "expired"

  // --- outcome state for the most recent state-changing request attempt ---
  var accepted: bool
  var lastSession: str
  var lastProof: str

  // --- environment pre-state: the RAW submitted evidence (set by arriveRequest, NOT by submit) ---
  // This is the genuine-checking fix for the trace-fidelity gap (idea-csrf-trace-fidelity / story-fix-csrf-trace).
  // The accepting action (submitStateChangingRequest) READS these but CANNOT rewrite them — so it
  // cannot lie about what was submitted. The invariants consult these pre-state values, not the
  // action-recorded lastSession/lastProof. A combined mutation (drop the proof check AND set
  // attemptedProof to the bound proof) is now impossible: submit doesn't assign attemptedProof.
  var attemptedSession: str
  var attemptedProof: str
  // tracks whether a request is pending arrival (so submit only fires after arrive)
  var requestPending: bool

  // Browser-local claims are deliberately modeled and deliberately ignored by the
  // acceptance rule. They represent UI optimism such as "this session still looks live"
  // or "the button looked granted". They are evidence in the request environment, not
  // server authority.
  var browserLocalSessionLive: bool
  var browserLocalGrantClaim: bool

  def authenticated(session) = operatorSessions.contains(session)
  def active(session) = sessionStatus.get(session) == "active"
  def validCsrfProof(session, proof) =
    csrfProofs.keys().contains(session) and proof == csrfProofs.get(session)
  def serverAccepts(session, proof) =
    authenticated(session) and active(session) and validCsrfProof(session, proof)

  action init = all {
    // The revoked/expired sessions remain in the recognized-id set on purpose: this
    // separates "the cookie names a known operator session" from "that session is still
    // live" and makes RevokedSessionCannotCommand non-tautological.
    operatorSessions' = Set("s_active", "s_revoked", "s_expired"),
    csrfProofs' = Map(
      "s_active" -> "proof_active",
      "s_revoked" -> "proof_revoked",
      "s_expired" -> "proof_expired",
      "missing_session" -> "wrong_proof"
    ),
    sessionStatus' = Map(
      "s_active" -> "active",
      "s_revoked" -> "revoked",
      "s_expired" -> "expired",
      "missing_session" -> "active"
    ),
    accepted' = false,
    lastSession' = "missing_session",
    lastProof' = "missing_proof",
    attemptedSession' = "missing_session",
    attemptedProof' = "missing_proof",
    requestPending' = false,
    browserLocalSessionLive' = false,
    browserLocalGrantClaim' = false,
  }

  // Environment action: a request ARRIVES with raw submitted evidence. This action sets the
  // pre-state attemptedSession/attemptedProof (the submitted values) and marks a request pending.
  // The accepting action below reads these but does NOT rewrite them — so the accepting action
  // cannot lie about what was submitted. Browser-local UI claims also arrive here as environment
  // input the server cannot forge.
  action arriveRequest(session, proof, uiSaysSessionLive, uiSaysGrantPresent) = all {
    session.in(SESSION_IDS),
    proof.in(PROOFS),
    attemptedSession' = session,
    attemptedProof' = proof,
    browserLocalSessionLive' = uiSaysSessionLive,
    browserLocalGrantClaim' = uiSaysGrantPresent,
    requestPending' = true,
    // a new request arriving means the prior acceptance is no longer the current outcome —
    // reset accepted so the invariants reason about THIS request, not a stale prior one
    accepted' = false,
    lastSession' = lastSession,
    lastProof' = lastProof,
    operatorSessions' = operatorSessions,
    csrfProofs' = csrfProofs,
    sessionStatus' = sessionStatus,
  }

  // Server processing: reads the pending request's RAW evidence (attemptedSession/attemptedProof
  // — set by arriveRequest, NOT rewritable here) and applies the acceptance rule. It records the
  // outcome in lastSession/lastProof/accepted (the recorded trace) but the INVARIANTS consult the
  // pre-state attempted* values, not these. A buggy acceptance rule (e.g. dropping the proof check)
  // is caught because attemptedProof != csrfProofs.get(attemptedSession) for a wrong-proof request.
  action submitStateChangingRequest = all {
    requestPending,
    accepted' = serverAccepts(attemptedSession, attemptedProof),
    lastSession' = attemptedSession,
    lastProof' = attemptedProof,
    requestPending' = false,
    // submit does NOT rewrite attemptedSession/attemptedProof (the genuine-checking invariant)
    attemptedSession' = attemptedSession,
    attemptedProof' = attemptedProof,
    browserLocalSessionLive' = browserLocalSessionLive,
    browserLocalGrantClaim' = browserLocalGrantClaim,
    operatorSessions' = operatorSessions,
    csrfProofs' = csrfProofs,
    sessionStatus' = sessionStatus,
  }

  action step = any {
    nondet session = SESSION_IDS.oneOf(); nondet proof = PROOFS.oneOf(); nondet uiSaysSessionLive = Set(true, false).oneOf(); nondet uiSaysGrantPresent = Set(true, false).oneOf(); arriveRequest(session, proof, uiSaysSessionLive, uiSaysGrantPresent),
    submitStateChangingRequest,
  }

  // ---------------------------------------------------------------------------
  // Promoted model properties (each with an inline @promotion block)
  // ---------------------------------------------------------------------------

  // @promotion {
  //   property:    CsrfRejectsUnauthenticated
  //   status:      promoted
  //   model:       specs/seed/csrf_browser.qnt
  //   language:    quint
  //   backend:     apalache
  //   invocation:  quint verify csrf_browser.qnt --invariant csrf_rejects_unauthenticated --max-steps 12
  //   bounds:      { sessions: 4, proofs: 5, max_steps: 12 }
  //   expected:    pass
  //   proto_fields: [none]
  //   semantics:   SECURITY.md §CSRF requires an authenticated operator session cookie before accepting a state-changing web command
  // }
  // INDEPENDENT oracle: checks the RAW attempted evidence (attemptedSession/attemptedProof),
  // not lastSession/lastProof (which the action records and could lie about). This is the
  // genuine-checking discipline extended to the evidence layer (idea-csrf-trace-fidelity):
  // the invariant consults what was SUBMITTED, not what the action claims was submitted.
  val csrf_rejects_unauthenticated =
    accepted.implies(operatorSessions.contains(attemptedSession))

  // @promotion {
  //   property:    CsrfRejectsMissingProof
  //   status:      promoted
  //   model:       specs/seed/csrf_browser.qnt
  //   language:    quint
  //   backend:     apalache
  //   invocation:  quint verify csrf_browser.qnt --invariant csrf_rejects_missing_proof --max-steps 12
  //   bounds:      { sessions: 4, proofs: 5, max_steps: 12 }
  //   expected:    pass
  //   proto_fields: [none]
  //   semantics:   SECURITY.md §CSRF requires a CSRF token tied to the authenticated operator session before command acceptance
  // }
  // INDEPENDENT oracle: asserts raw csrfProofs map facts against the ATTEMPTED evidence
  // (attemptedSession/attemptedProof), NOT the validCsrfProof helper that serverAccepts uses,
  // and NOT lastSession/lastProof (which the action records). Mutation tests: (1) breaking
  // validCsrfProof to `true` is caught; (2) an action that lies about the recorded trace
  // (setting lastProof to the bound proof regardless of input) is caught because the invariant
  // reads attemptedProof, the raw submitted value.
  val csrf_rejects_missing_proof =
    accepted.implies(
      csrfProofs.keys().contains(attemptedSession)
        and attemptedProof == csrfProofs.get(attemptedSession))

  // @promotion {
  //   property:    RevokedSessionCannotCommand
  //   status:      promoted
  //   model:       specs/seed/csrf_browser.qnt
  //   language:    quint
  //   backend:     apalache
  //   invocation:  quint verify csrf_browser.qnt --invariant revoked_session_cannot_command --max-steps 12
  //   bounds:      { sessions: 4, proofs: 5, max_steps: 12 }
  //   expected:    pass
  //   proto_fields: [none]
  //   semantics:   SECURITY.md and VERIFICATION.md require revoked or expired operator sessions to be rejected before issuing new commands
  // }
  val revoked_session_cannot_command =
    accepted.implies(not(sessionStatus.get(attemptedSession).in(DEAD_STATUSES)))

  // @promotion {
  //   property:    browser_local_state_not_authority
  //   status:      promoted
  //   model:       specs/seed/csrf_browser.qnt
  //   language:    quint
  //   backend:     apalache
  //   invocation:  quint verify csrf_browser.qnt --invariant browser_local_state_not_authority --max-steps 12
  //   bounds:      { sessions: 4, proofs: 5, max_steps: 12 }
  //   expected:    pass
  //   proto_fields: [none]
  //   semantics:   browser-local UI claims cannot grant authority or override server-side session/CSRF checks
  // }
  // Promoted local-UI authority invariant from docs/VERIFICATION.md. INDEPENDENT oracle:
  // asserts raw operatorSessions + csrfProofs + sessionStatus facts against the ATTEMPTED
  // evidence, not the serverAccepts helper or the recorded trace. Mutation test: breaking
  // serverAccepts' sub-helpers, or an action lying about the recorded trace, is caught here.
  val browser_local_state_not_authority =
    accepted.implies(
      operatorSessions.contains(attemptedSession)
        and sessionStatus.get(attemptedSession) == "active"
        and csrfProofs.keys().contains(attemptedSession)
        and attemptedProof == csrfProofs.get(attemptedSession))
}
