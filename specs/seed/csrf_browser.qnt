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
    browserLocalSessionLive' = false,
    browserLocalGrantClaim' = false,
  }

  // Permissive request attempt: every session/proof in the finite request space may be
  // submitted, and browser-local UI may optimistically claim the session/grant is good.
  // Acceptance ignores those local claims and depends only on server-side evidence.
  action submitStateChangingRequest(session, proof, uiSaysSessionLive, uiSaysGrantPresent) = all {
    session.in(SESSION_IDS),
    proof.in(PROOFS),
    accepted' = serverAccepts(session, proof),
    lastSession' = session,
    lastProof' = proof,
    browserLocalSessionLive' = uiSaysSessionLive,
    browserLocalGrantClaim' = uiSaysGrantPresent,
    operatorSessions' = operatorSessions,
    csrfProofs' = csrfProofs,
    sessionStatus' = sessionStatus,
  }

  action step = {
    nondet session = SESSION_IDS.oneOf()
    nondet proof = PROOFS.oneOf()
    nondet uiSaysSessionLive = Set(true, false).oneOf()
    nondet uiSaysGrantPresent = Set(true, false).oneOf()
    submitStateChangingRequest(session, proof, uiSaysSessionLive, uiSaysGrantPresent)
  }

  // ---------------------------------------------------------------------------
  // Checked properties (each with an inline @promotion block)
  // ---------------------------------------------------------------------------

  // @promotion {
  //   property:    CsrfRejectsUnauthenticated
  //   tier:        checked-normative
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
  val csrf_rejects_unauthenticated =
    accepted.implies(operatorSessions.contains(lastSession))

  // @promotion {
  //   property:    CsrfRejectsMissingProof
  //   tier:        checked-normative
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
  val csrf_rejects_missing_proof =
    accepted.implies(validCsrfProof(lastSession, lastProof))

  // @promotion {
  //   property:    RevokedSessionCannotCommand
  //   tier:        checked-normative
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
    accepted.implies(not(sessionStatus.get(lastSession).in(DEAD_STATUSES)))

  // Non-promoted documentation invariant for the local-UI authority rule from
  // docs/VERIFICATION.md. It is equivalent to saying browserLocalSessionLive and
  // browserLocalGrantClaim never widen the server acceptance predicate.
  val browser_local_state_not_authority =
    accepted.implies(serverAccepts(lastSession, lastProof))
}
