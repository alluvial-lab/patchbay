// Patchbay seed model: Elicitation lifecycle (first durable terminal commit wins)
//
// This model covers the promoted ElicitationState lifecycle properties from
// docs/PROTOCOL.md § ElicitationState lifecycle. It models opened/pending
// Elicitations, terminal races, response-Operation typed correlation, stale target
// generations, invalid response rejection, and duplicate response behavior.
//
// Rich-model note (formal-model-realignment Unit EL): response Operations carry
// their OWN domain/session/generation/actor/endpoint context. The Elicitation-side
// context and response-Operation-side context are both modeled so forged or stale
// response Operations are rejected by independent oracles rather than hidden by a
// thin lifecycle-only state machine.
//
// Genuine-checking discipline (load-bearing): actions are PERMISSIVE. The driver
// attempts valid and invalid response Operations, forged ids, wrong domains,
// mismatched sessions/generations, wrong actors, stale target generations,
// duplicate answers, and competing terminal candidates. Properties check raw state
// facts and response-Operation fields; they do not call the helper predicates that
// gate action branches.
module elicitation_lifecycle {
  // --- ElicitationState registry (docs/PROTOCOL.md canonical table) ---
  pure val TERMINAL = Set("answered", "declined", "expired", "cancelled", "withdrawn", "superseded", "stale")
  pure val NON_ANSWER_TERMINAL = Set("declined", "expired", "cancelled", "withdrawn", "superseded", "stale")
  pure val NON_TERMINAL = Set("opened", "pending")

  // --- finite bounds ---
  pure val ELICITATION_IDS = Set("e1")
  pure val RESPONSE_OP_IDS = Set("ro1", "ro2")
  pure val ENDPOINTS = Set("ep-a", "ep-b")
  pure val ACTORS = Set("alice")
  pure val SUBMITTING_ACTORS = ACTORS.union(Set("mallory"))
  pure val AUTHORITY_DOMAINS = Set("domain-main")
  pure val RESPONSE_DOMAINS = AUTHORITY_DOMAINS.union(Set("domain-forged"))
  pure val SESSIONS = Set("s1", "s2")
  pure val GENERATIONS = 0.to(2)
  pure val CONTRACT_KINDS = Set("approval", "question")
  pure val RESPONSE_KINDS = Set("approval-response", "elicitation-response")
  pure val ATTEMPTED_RESPONSE_KINDS = RESPONSE_KINDS.union(Set("spawn"))

  // Separate id spaces used by the typed-correlation oracle. These are intentionally
  // disjoint from ELICITATION_IDS in the promoted invariant.
  pure val COMMAND_IDS = Set("c1", "c2")
  pure val MESSAGE_IDS = Set("m1", "m2")
  pure val REPLY_IDS = Set("r1", "r2")
  pure val EVENT_IDS = Set("ev1", "ev2")
  pure val CLAIMED_ELICITATION_IDS = ELICITATION_IDS.union(COMMAND_IDS).union(MESSAGE_IDS).union(REPLY_IDS).union(EVENT_IDS).union(Set("unknown"))

  // --- state ---
  var state: str -> str                  // ElicitationId -> ElicitationState
  var terminalLsn: str -> int            // ElicitationId -> LSN at terminal commit (0 = not terminal)
  var lsn: int                           // monotonic gap-free log sequence number
  var responderActor: str -> str         // ElicitationId -> expected responder actor (operator in v0)
  var answeredBy: str -> str             // ElicitationId -> endpoint that answered (audit; "none" if not answered)
  var contractKind: str -> str           // ElicitationId -> response_contract.contract_kind

  // --- Elicitation-side context ---
  var elicitationDomain: str -> str       // ElicitationId -> authority domain it was opened in
  var targetSession: str -> str           // ElicitationId -> target session id
  var targetGeneration: str -> int        // ElicitationId -> target session generation
  var sessionGeneration: str -> int       // live session generation per session

  // --- response-Operation-side context ---
  var responseOpElicitation: str -> str   // responseOpId -> the ElicitationId it claims to answer (or forged id)
  var responseOpKind: str -> str          // responseOpId -> approval-response | elicitation-response
  var responseOpDomain: str -> str        // responseOpId -> authority domain of the submitting Operation
  var responseOpSession: str -> str       // responseOpId -> target session it binds to
  var responseOpGeneration: str -> int    // responseOpId -> session generation it binds to
  var responseOpActor: str -> str         // responseOpId -> authenticated actor submitting it
  var responseOpEndpoint: str -> str      // responseOpId -> authenticated endpoint submitting it (audit)
  var responseValid: str -> bool          // responseOpId -> whether it satisfied the contract
  var responseDuplicate: str -> bool      // responseOpId -> whether a prior response already committed

  // Supplemental proof/audit variable: records which response Operation, if any,
  // committed the durable answer. `answeredBy` is endpoint-only audit data; this op id
  // lets duplicate/invalid-response invariants prove that rejected response Operations
  // never become the committed answer.
  var answeredResponseOp: str -> str      // ElicitationId -> committed responseOpId ("none" if not answered)
  var firstTerminalState: str -> str      // ElicitationId -> first terminal state selected ("none" if not terminal)
  var firstAnsweredBy: str -> str         // ElicitationId -> first terminal answer endpoint snapshot
  var firstAnsweredResponseOp: str -> str // ElicitationId -> first terminal answer responseOp snapshot

  // ---------------------------------------------------------------------------
  // Helpers used by actions only. Promoted invariants below intentionally check raw
  // facts inline instead of calling these helpers.
  // ---------------------------------------------------------------------------

  def targetLive(eid) = targetGeneration.get(eid) == sessionGeneration.get(targetSession.get(eid))

  def responseMatchesTarget(eid, claimedEid, kind, domain, session, generation, actor) =
    claimedEid == eid
      and kind.in(RESPONSE_KINDS)
      and domain == elicitationDomain.get(eid)
      and session == targetSession.get(eid)
      and generation == targetGeneration.get(eid)
      and actor == responderActor.get(eid)
      and targetLive(eid)

  def responseOpUnused(responseOpId) = responseOpElicitation.get(responseOpId) == "none"

  def firstValidAnswerAllowed(eid, responseOpId, claimedEid, kind, domain, session, generation, actor) =
    state.get(eid) == "pending"
      and responseOpUnused(responseOpId)
      and responseMatchesTarget(eid, claimedEid, kind, domain, session, generation, actor)

  def idempotentResponseRetry(eid, responseOpId, claimedEid, kind, domain, session, generation, actor) =
    state.get(eid) == "answered"
      and answeredResponseOp.get(eid) == responseOpId
      and responseMatchesTarget(eid, claimedEid, kind, domain, session, generation, actor)

  def committedResponseOp(responseOpId) =
    ELICITATION_IDS.exists(eid => answeredResponseOp.get(eid) == responseOpId)

  def duplicateAttempt(responseOpId, claimedEid) =
    if (ELICITATION_IDS.contains(claimedEid))
      state.get(claimedEid) == "answered" and answeredResponseOp.get(claimedEid) != responseOpId
    else
      false

  // INDEPENDENT property oracle: checks raw response-Operation facts WITHOUT
  // calling responseMatchesTarget or firstValidAnswerAllowed. Mutation tests break
  // the action-side helper; this oracle still checks the recorded id-space,
  // authority-domain, session/generation, actor, endpoint, and committed-op facts.
  def recordedResponseIndependentOk(responseOpId) =
    if (ELICITATION_IDS.contains(responseOpElicitation.get(responseOpId)))
      responseOpKind.get(responseOpId).in(RESPONSE_KINDS)
        and responseOpDomain.get(responseOpId) == elicitationDomain.get(responseOpElicitation.get(responseOpId))
        and responseOpSession.get(responseOpId) == targetSession.get(responseOpElicitation.get(responseOpId))
        and responseOpGeneration.get(responseOpId) == targetGeneration.get(responseOpElicitation.get(responseOpId))
        and responseOpGeneration.get(responseOpId) == sessionGeneration.get(responseOpSession.get(responseOpId))
        and responseOpActor.get(responseOpId) == responderActor.get(responseOpElicitation.get(responseOpId))
        and ENDPOINTS.contains(responseOpEndpoint.get(responseOpId))
        and answeredResponseOp.get(responseOpElicitation.get(responseOpId)) == responseOpId
    else
      false

  // ---------------------------------------------------------------------------
  // Actions (permissive: races, stale attempts, forgeries, and late answers arrive)
  // ---------------------------------------------------------------------------

  action init = all {
    state' = Map("e1" -> "opened"),
    terminalLsn' = Map("e1" -> 0),
    lsn' = 2,
    responderActor' = Map("e1" -> "alice"),
    answeredBy' = Map("e1" -> "none"),
    contractKind' = Map("e1" -> "approval"),
    elicitationDomain' = Map("e1" -> "domain-main"),
    targetSession' = Map("e1" -> "s1"),
    targetGeneration' = Map("e1" -> 0),
    sessionGeneration' = Map("s1" -> 0, "s2" -> 0),
    responseOpElicitation' = RESPONSE_OP_IDS.mapBy(ro => "none"),
    responseOpKind' = RESPONSE_OP_IDS.mapBy(ro => "none"),
    responseOpDomain' = RESPONSE_OP_IDS.mapBy(ro => "domain-main"),
    responseOpSession' = RESPONSE_OP_IDS.mapBy(ro => "s1"),
    responseOpGeneration' = RESPONSE_OP_IDS.mapBy(ro => 0),
    responseOpActor' = RESPONSE_OP_IDS.mapBy(ro => "alice"),
    responseOpEndpoint' = RESPONSE_OP_IDS.mapBy(ro => "none"),
    responseValid' = RESPONSE_OP_IDS.mapBy(ro => false),
    responseDuplicate' = RESPONSE_OP_IDS.mapBy(ro => false),
    answeredResponseOp' = Map("e1" -> "none"),
    firstTerminalState' = Map("e1" -> "none"),
    firstAnsweredBy' = Map("e1" -> "none"),
    firstAnsweredResponseOp' = Map("e1" -> "none"),
  }

  // Adapter/core durable open event. The bounded model starts with both Elicitations
  // opened; this action preserves the named protocol action without re-opening terminal
  // Elicitations or mutating context after init.
  action openElicitation(eid, actor, contract, session, generation) = all {
    state.get(eid) == "opened",
    actor == responderActor.get(eid),
    contract == contractKind.get(eid),
    session == targetSession.get(eid),
    generation == targetGeneration.get(eid),
    state' = state,
    terminalLsn' = terminalLsn,
    lsn' = lsn,
    responderActor' = responderActor,
    answeredBy' = answeredBy,
    contractKind' = contractKind,
    elicitationDomain' = elicitationDomain,
    targetSession' = targetSession,
    targetGeneration' = targetGeneration,
    sessionGeneration' = sessionGeneration,
    responseOpElicitation' = responseOpElicitation,
    responseOpKind' = responseOpKind,
    responseOpDomain' = responseOpDomain,
    responseOpSession' = responseOpSession,
    responseOpGeneration' = responseOpGeneration,
    responseOpActor' = responseOpActor,
    responseOpEndpoint' = responseOpEndpoint,
    responseValid' = responseValid,
    responseDuplicate' = responseDuplicate,
    answeredResponseOp' = answeredResponseOp,
    firstTerminalState' = firstTerminalState,
    firstAnsweredBy' = firstAnsweredBy,
    firstAnsweredResponseOp' = firstAnsweredResponseOp,
  }

  action makePending(eid) = all {
    state.get(eid) == "opened",
    state' = state.set(eid, "pending"),
    lsn' = lsn + 1,
    terminalLsn' = terminalLsn,
    responderActor' = responderActor,
    answeredBy' = answeredBy,
    contractKind' = contractKind,
    elicitationDomain' = elicitationDomain,
    targetSession' = targetSession,
    targetGeneration' = targetGeneration,
    sessionGeneration' = sessionGeneration,
    responseOpElicitation' = responseOpElicitation,
    responseOpKind' = responseOpKind,
    responseOpDomain' = responseOpDomain,
    responseOpSession' = responseOpSession,
    responseOpGeneration' = responseOpGeneration,
    responseOpActor' = responseOpActor,
    responseOpEndpoint' = responseOpEndpoint,
    responseValid' = responseValid,
    responseDuplicate' = responseDuplicate,
    answeredResponseOp' = answeredResponseOp,
    firstTerminalState' = firstTerminalState,
    firstAnsweredBy' = firstAnsweredBy,
    firstAnsweredResponseOp' = firstAnsweredResponseOp,
  }

  action commitTerminal(eid, terminalState) = any {
    all {
      state.get(eid) == "pending",
      terminalState.in(TERMINAL),
      state' = state.set(eid, terminalState),
      lsn' = lsn + 1,
      terminalLsn' = terminalLsn.set(eid, lsn + 1),
      responderActor' = responderActor,
      answeredBy' = answeredBy,
      contractKind' = contractKind,
      elicitationDomain' = elicitationDomain,
      targetSession' = targetSession,
      targetGeneration' = targetGeneration,
      sessionGeneration' = sessionGeneration,
      responseOpElicitation' = responseOpElicitation,
      responseOpKind' = responseOpKind,
      responseOpDomain' = responseOpDomain,
      responseOpSession' = responseOpSession,
      responseOpGeneration' = responseOpGeneration,
      responseOpActor' = responseOpActor,
      responseOpEndpoint' = responseOpEndpoint,
      responseValid' = responseValid,
      responseDuplicate' = responseDuplicate,
      answeredResponseOp' = answeredResponseOp,
      firstTerminalState' = if (terminalLsn.get(eid) == 0) firstTerminalState.set(eid, terminalState) else firstTerminalState,
      firstAnsweredBy' = firstAnsweredBy,
      firstAnsweredResponseOp' = firstAnsweredResponseOp,
    },
    all {
      state.get(eid).in(TERMINAL),
      terminalState.in(TERMINAL),
      state' = state,
      lsn' = lsn,
      terminalLsn' = terminalLsn,
      responderActor' = responderActor,
      answeredBy' = answeredBy,
      contractKind' = contractKind,
      elicitationDomain' = elicitationDomain,
      targetSession' = targetSession,
      targetGeneration' = targetGeneration,
      sessionGeneration' = sessionGeneration,
      responseOpElicitation' = responseOpElicitation,
      responseOpKind' = responseOpKind,
      responseOpDomain' = responseOpDomain,
      responseOpSession' = responseOpSession,
      responseOpGeneration' = responseOpGeneration,
      responseOpActor' = responseOpActor,
      responseOpEndpoint' = responseOpEndpoint,
      responseValid' = responseValid,
      responseDuplicate' = responseDuplicate,
      answeredResponseOp' = answeredResponseOp,
      firstTerminalState' = firstTerminalState,
      firstAnsweredBy' = firstAnsweredBy,
      firstAnsweredResponseOp' = firstAnsweredResponseOp,
    },
  }

  action attemptAnswer(eid, responseOpId, claimedEid, kind, endpoint, domain, session, generation, actor) = any {
    all {
      firstValidAnswerAllowed(eid, responseOpId, claimedEid, kind, domain, session, generation, actor),
      state' = state.set(eid, "answered"),
      lsn' = lsn + 1,
      terminalLsn' = terminalLsn.set(eid, lsn + 1),
      answeredBy' = answeredBy.set(eid, endpoint),
      responseOpElicitation' = responseOpElicitation.set(responseOpId, claimedEid),
      responseOpKind' = responseOpKind.set(responseOpId, kind),
      responseOpDomain' = responseOpDomain.set(responseOpId, domain),
      responseOpSession' = responseOpSession.set(responseOpId, session),
      responseOpGeneration' = responseOpGeneration.set(responseOpId, generation),
      responseOpActor' = responseOpActor.set(responseOpId, actor),
      responseOpEndpoint' = responseOpEndpoint.set(responseOpId, endpoint),
      responseValid' = responseValid.set(responseOpId, true),
      responseDuplicate' = responseDuplicate.set(responseOpId, false),
      answeredResponseOp' = answeredResponseOp.set(eid, responseOpId),
      firstTerminalState' = if (terminalLsn.get(eid) == 0) firstTerminalState.set(eid, "answered") else firstTerminalState,
      firstAnsweredBy' = if (terminalLsn.get(eid) == 0) firstAnsweredBy.set(eid, endpoint) else firstAnsweredBy,
      firstAnsweredResponseOp' = if (terminalLsn.get(eid) == 0) firstAnsweredResponseOp.set(eid, responseOpId) else firstAnsweredResponseOp,
      responderActor' = responderActor,
      contractKind' = contractKind,
      elicitationDomain' = elicitationDomain,
      targetSession' = targetSession,
      targetGeneration' = targetGeneration,
      sessionGeneration' = sessionGeneration,
    },
    all {
      idempotentResponseRetry(eid, responseOpId, claimedEid, kind, domain, session, generation, actor),
      state' = state,
      lsn' = lsn,
      terminalLsn' = terminalLsn,
      answeredBy' = answeredBy,
      responseOpElicitation' = responseOpElicitation.set(responseOpId, claimedEid),
      responseOpKind' = responseOpKind.set(responseOpId, kind),
      responseOpDomain' = responseOpDomain.set(responseOpId, domain),
      responseOpSession' = responseOpSession.set(responseOpId, session),
      responseOpGeneration' = responseOpGeneration.set(responseOpId, generation),
      responseOpActor' = responseOpActor.set(responseOpId, actor),
      responseOpEndpoint' = responseOpEndpoint.set(responseOpId, endpoint),
      responseValid' = responseValid.set(responseOpId, true),
      responseDuplicate' = responseDuplicate.set(responseOpId, false),
      answeredResponseOp' = answeredResponseOp,
      firstTerminalState' = firstTerminalState,
      firstAnsweredBy' = firstAnsweredBy,
      firstAnsweredResponseOp' = firstAnsweredResponseOp,
      responderActor' = responderActor,
      contractKind' = contractKind,
      elicitationDomain' = elicitationDomain,
      targetSession' = targetSession,
      targetGeneration' = targetGeneration,
      sessionGeneration' = sessionGeneration,
    },
    all {
      not(firstValidAnswerAllowed(eid, responseOpId, claimedEid, kind, domain, session, generation, actor)),
      not(idempotentResponseRetry(eid, responseOpId, claimedEid, kind, domain, session, generation, actor)),
      responseOpUnused(responseOpId),
      state' = state,
      lsn' = lsn,
      terminalLsn' = terminalLsn,
      answeredBy' = answeredBy,
      responseOpElicitation' = responseOpElicitation.set(responseOpId, claimedEid),
      responseOpKind' = responseOpKind.set(responseOpId, kind),
      responseOpDomain' = responseOpDomain.set(responseOpId, domain),
      responseOpSession' = responseOpSession.set(responseOpId, session),
      responseOpGeneration' = responseOpGeneration.set(responseOpId, generation),
      responseOpActor' = responseOpActor.set(responseOpId, actor),
      responseOpEndpoint' = responseOpEndpoint.set(responseOpId, endpoint),
      responseValid' = responseValid.set(responseOpId, false),
      responseDuplicate' = responseDuplicate.set(responseOpId, duplicateAttempt(responseOpId, claimedEid)),
      answeredResponseOp' = answeredResponseOp,
      firstTerminalState' = firstTerminalState,
      firstAnsweredBy' = firstAnsweredBy,
      firstAnsweredResponseOp' = firstAnsweredResponseOp,
      responderActor' = responderActor,
      contractKind' = contractKind,
      elicitationDomain' = elicitationDomain,
      targetSession' = targetSession,
      targetGeneration' = targetGeneration,
      sessionGeneration' = sessionGeneration,
    },
    all {
      not(idempotentResponseRetry(eid, responseOpId, claimedEid, kind, domain, session, generation, actor)),
      not(responseOpUnused(responseOpId)),
      state' = state,
      lsn' = lsn,
      terminalLsn' = terminalLsn,
      answeredBy' = answeredBy,
      responseOpElicitation' = responseOpElicitation,
      responseOpKind' = responseOpKind,
      responseOpDomain' = responseOpDomain,
      responseOpSession' = responseOpSession,
      responseOpGeneration' = responseOpGeneration,
      responseOpActor' = responseOpActor,
      responseOpEndpoint' = responseOpEndpoint,
      responseValid' = responseValid,
      responseDuplicate' = responseDuplicate,
      answeredResponseOp' = answeredResponseOp,
      firstTerminalState' = firstTerminalState,
      firstAnsweredBy' = firstAnsweredBy,
      firstAnsweredResponseOp' = firstAnsweredResponseOp,
      responderActor' = responderActor,
      contractKind' = contractKind,
      elicitationDomain' = elicitationDomain,
      targetSession' = targetSession,
      targetGeneration' = targetGeneration,
      sessionGeneration' = sessionGeneration,
    },
  }

  action lateAnswer(eid, responseOpId) = all {
    state.get(eid).in(TERMINAL),
    not(committedResponseOp(responseOpId)),
    responseOpElicitation.get(responseOpId) == "none",
    state' = state,
    lsn' = lsn,
    terminalLsn' = terminalLsn,
    answeredBy' = answeredBy,
    responseOpElicitation' = responseOpElicitation.set(responseOpId, eid),
    responseOpKind' = responseOpKind.set(responseOpId, "elicitation-response"),
    responseOpDomain' = responseOpDomain.set(responseOpId, elicitationDomain.get(eid)),
    responseOpSession' = responseOpSession.set(responseOpId, targetSession.get(eid)),
    responseOpGeneration' = responseOpGeneration.set(responseOpId, targetGeneration.get(eid)),
    responseOpActor' = responseOpActor.set(responseOpId, responderActor.get(eid)),
    responseOpEndpoint' = responseOpEndpoint.set(responseOpId, "ep-b"),
    responseValid' = responseValid.set(responseOpId, false),
    responseDuplicate' = responseDuplicate.set(responseOpId, state.get(eid) == "answered"),
    answeredResponseOp' = answeredResponseOp,
    firstTerminalState' = firstTerminalState,
    firstAnsweredBy' = firstAnsweredBy,
    firstAnsweredResponseOp' = firstAnsweredResponseOp,
    responderActor' = responderActor,
    contractKind' = contractKind,
    elicitationDomain' = elicitationDomain,
    targetSession' = targetSession,
    targetGeneration' = targetGeneration,
    sessionGeneration' = sessionGeneration,
  }

  action decline(eid) = commitTerminal(eid, "declined")
  action expire(eid) = commitTerminal(eid, "expired")
  action cancel(eid) = commitTerminal(eid, "cancelled")
  action withdraw(eid) = commitTerminal(eid, "withdrawn")
  action supersede(eid) = commitTerminal(eid, "superseded")

  action advanceSessionGeneration(eid) = all {
    state.get(eid) == "pending",
    targetGeneration.get(eid) < 2,
    state' = state,
    lsn' = lsn + 1,
    terminalLsn' = terminalLsn,
    sessionGeneration' = sessionGeneration.set(targetSession.get(eid), targetGeneration.get(eid) + 1),
    responderActor' = responderActor,
    answeredBy' = answeredBy,
    contractKind' = contractKind,
    elicitationDomain' = elicitationDomain,
    targetSession' = targetSession,
    targetGeneration' = targetGeneration,
    responseOpElicitation' = responseOpElicitation,
    responseOpKind' = responseOpKind,
    responseOpDomain' = responseOpDomain,
    responseOpSession' = responseOpSession,
    responseOpGeneration' = responseOpGeneration,
    responseOpActor' = responseOpActor,
    responseOpEndpoint' = responseOpEndpoint,
    responseValid' = responseValid,
    responseDuplicate' = responseDuplicate,
    answeredResponseOp' = answeredResponseOp,
    firstTerminalState' = firstTerminalState,
    firstAnsweredBy' = firstAnsweredBy,
    firstAnsweredResponseOp' = firstAnsweredResponseOp,
  }

  action goStale(eid) = any {
    all {
      state.get(eid) == "pending",
      targetGeneration.get(eid) < 2,
      state' = state.set(eid, "stale"),
      lsn' = lsn + 1,
      terminalLsn' = terminalLsn.set(eid, lsn + 1),
      sessionGeneration' = sessionGeneration.set(targetSession.get(eid), targetGeneration.get(eid) + 1),
      responderActor' = responderActor,
      answeredBy' = answeredBy,
      contractKind' = contractKind,
      elicitationDomain' = elicitationDomain,
      targetSession' = targetSession,
      targetGeneration' = targetGeneration,
      responseOpElicitation' = responseOpElicitation,
      responseOpKind' = responseOpKind,
      responseOpDomain' = responseOpDomain,
      responseOpSession' = responseOpSession,
      responseOpGeneration' = responseOpGeneration,
      responseOpActor' = responseOpActor,
      responseOpEndpoint' = responseOpEndpoint,
      responseValid' = responseValid,
      responseDuplicate' = responseDuplicate,
      answeredResponseOp' = answeredResponseOp,
      firstTerminalState' = if (terminalLsn.get(eid) == 0) firstTerminalState.set(eid, "stale") else firstTerminalState,
      firstAnsweredBy' = firstAnsweredBy,
      firstAnsweredResponseOp' = firstAnsweredResponseOp,
    },
    all {
      state.get(eid).in(TERMINAL),
      state' = state,
      lsn' = lsn,
      terminalLsn' = terminalLsn,
      sessionGeneration' = sessionGeneration,
      responderActor' = responderActor,
      answeredBy' = answeredBy,
      contractKind' = contractKind,
      elicitationDomain' = elicitationDomain,
      targetSession' = targetSession,
      targetGeneration' = targetGeneration,
      responseOpElicitation' = responseOpElicitation,
      responseOpKind' = responseOpKind,
      responseOpDomain' = responseOpDomain,
      responseOpSession' = responseOpSession,
      responseOpGeneration' = responseOpGeneration,
      responseOpActor' = responseOpActor,
      responseOpEndpoint' = responseOpEndpoint,
      responseValid' = responseValid,
      responseDuplicate' = responseDuplicate,
      answeredResponseOp' = answeredResponseOp,
      firstTerminalState' = firstTerminalState,
      firstAnsweredBy' = firstAnsweredBy,
      firstAnsweredResponseOp' = firstAnsweredResponseOp,
    },
  }

  // The driver pins lifecycle changes to e1 for symmetry reduction. Response
  // attempts range over valid and adversarial fields so each action guard conjunct
  // can be mutation-tested independently without duplicating action branches.
  action attemptAnyAnswer = {
    nondet responseOpId = RESPONSE_OP_IDS.oneOf()
    nondet claimedEid = CLAIMED_ELICITATION_IDS.oneOf()
    nondet kind = ATTEMPTED_RESPONSE_KINDS.oneOf()
    nondet endpoint = ENDPOINTS.oneOf()
    nondet domain = RESPONSE_DOMAINS.oneOf()
    nondet session = SESSIONS.oneOf()
    nondet generation = GENERATIONS.oneOf()
    nondet actor = SUBMITTING_ACTORS.oneOf()
    attemptAnswer("e1", responseOpId, claimedEid, kind, endpoint, domain, session, generation, actor)
  }

  action step = any {
    makePending("e1"),
    advanceSessionGeneration("e1"),
    attemptAnyAnswer,
    lateAnswer("e1", "ro2"),
    nondet terminal = NON_ANSWER_TERMINAL.oneOf(); commitTerminal("e1", terminal),
    goStale("e1"),
  }

  // ---------------------------------------------------------------------------
  // Model properties (each with an inline @promotion block; mixed promoted/draft status)
  // ---------------------------------------------------------------------------

  // @promotion {
  //   property:    ElicitationPendingFinality
  //   status:      promoted
  //   model:       specs/seed/elicitation_lifecycle.qnt
  //   language:    quint
  //   backend:     apalache-temporal
  //   invocation:  echo y | quint verify elicitation_lifecycle.qnt --temporal elicitation_pending_finality --max-steps 10
  //   bounds:      { elicitation_ids: 1, response_ops: 2, endpoints: 2, contract_kinds: 2, max_steps: 10 }
  //   expected:    pass
  //   proto_fields: [none]
  //   semantics:   once an Elicitation reaches a terminal state, later answer/cancel/expire/withdraw/stale candidates do not mutate it
  // }
  temporal elicitation_pending_finality =
    always((terminalLsn.get("e1") > 0).implies(
      state.get("e1") == firstTerminalState.get("e1")
        and answeredBy.get("e1") == firstAnsweredBy.get("e1")
        and answeredResponseOp.get("e1") == firstAnsweredResponseOp.get("e1")))

  // @promotion {
  //   property:    ElicitationFirstAnswerWins
  //   status:      promoted
  //   model:       specs/seed/elicitation_lifecycle.qnt
  //   language:    quint
  //   backend:     apalache-temporal
  //   invocation:  echo y | quint verify elicitation_lifecycle.qnt --temporal elicitation_first_answer_wins --max-steps 10
  //   bounds:      { elicitation_ids: 1, response_ops: 2, endpoints: 2, contract_kinds: 2, max_steps: 10 }
  //   expected:    pass
  //   proto_fields: [none]
  //   semantics:   for single-answer contracts, the first durably committed valid answer wins and later answers are no-ops
  // }
  temporal elicitation_first_answer_wins =
    always((firstTerminalState.get("e1") == "answered").implies(
      state.get("e1") == "answered"
        and answeredBy.get("e1") == firstAnsweredBy.get("e1")
        and answeredResponseOp.get("e1") == firstAnsweredResponseOp.get("e1")))

  // @promotion {
  //   property:    ElicitationCorrelationTyped
  //   status:      promoted
  //   model:       specs/seed/elicitation_lifecycle.qnt
  //   language:    quint
  //   backend:     apalache
  //   invocation:  quint verify elicitation_lifecycle.qnt --invariant elicitation_correlation_typed --max-steps 12
  //   bounds:      { elicitation_ids: 1, response_ops: 2, id_spaces: 5, domains: 2, sessions: 2, generations: 3, max_steps: 12 }
  //   expected:    pass
  //   proto_fields: [none]
  //   semantics:   response Operations reference a known ElicitationId in the same authority/session/generation/responder context and cannot forge across id spaces
  // }
  val elicitation_correlation_typed =
    ELICITATION_IDS.intersect(COMMAND_IDS).size() == 0
      and ELICITATION_IDS.intersect(MESSAGE_IDS).size() == 0
      and ELICITATION_IDS.intersect(REPLY_IDS).size() == 0
      and ELICITATION_IDS.intersect(EVENT_IDS).size() == 0
      and RESPONSE_OP_IDS.forall(ro =>
        responseValid.get(ro).implies(recordedResponseIndependentOk(ro)))

  // @promotion {
  //   property:    ElicitationTimeoutNeitherSuccessNorDenial
  //   status:      draft
  //   model:       specs/seed/elicitation_lifecycle.qnt
  //   language:    quint
  //   backend:     apalache
  //   invocation:  <TBD — demoted; formula does not model the claimed failure boundary; v1 formal gate owns the real property>
  //   demotion_reason: metadata claims timeout never implies grant, but the model has no grant state. The formula checks only answer and decline fields.
  //   bounds:      { elicitation_ids: 1, response_ops: 2, terminal_candidates: 7, max_steps: 12 }
  //   expected:    pass
  //   proto_fields: [none]
  //   semantics:   timeout terminalizes as expired; timeout never implies answer, denial, or grant
  // }
  val elicitation_timeout_neither_success_nor_denial =
    ELICITATION_IDS.forall(eid =>
      (firstTerminalState.get(eid) == "expired").implies(
        answeredBy.get(eid) == "none"
          and answeredResponseOp.get(eid) == "none"
          and state.get(eid) != "answered"
          and state.get(eid) != "declined"))

  // @promotion {
  //   property:    ElicitationInvalidResponseRejected
  //   status:      promoted
  //   model:       specs/seed/elicitation_lifecycle.qnt
  //   language:    quint
  //   backend:     apalache
  //   invocation:  quint verify elicitation_lifecycle.qnt --invariant elicitation_invalid_response_rejected --max-steps 12
  //   bounds:      { elicitation_ids: 1, response_ops: 2, endpoints: 2, contract_kinds: 2, max_steps: 12 }
  //   expected:    pass
  //   proto_fields: [none]
  //   semantics:   invalid or duplicate response Operations are rejected/idempotent and never mutate the terminal answer
  // }
  val elicitation_invalid_response_rejected =
    RESPONSE_OP_IDS.forall(ro =>
      (not(recordedResponseIndependentOk(ro))).implies(
        not(responseValid.get(ro))
          and ELICITATION_IDS.forall(eid => answeredResponseOp.get(eid) != ro)))
      and RESPONSE_OP_IDS.forall(ro =>
        responseDuplicate.get(ro).implies(
          not(responseValid.get(ro))
            and ELICITATION_IDS.forall(eid => answeredResponseOp.get(eid) != ro)))
      and ELICITATION_IDS.forall(eid =>
        (answeredResponseOp.get(eid) != "none").implies(
          RESPONSE_OP_IDS.contains(answeredResponseOp.get(eid))
            and responseValid.get(answeredResponseOp.get(eid))
            and recordedResponseIndependentOk(answeredResponseOp.get(eid))))
      and ELICITATION_IDS.forall(eid =>
        (firstTerminalState.get(eid) == "answered").implies(
          answeredBy.get(eid) == firstAnsweredBy.get(eid)
            and answeredResponseOp.get(eid) == firstAnsweredResponseOp.get(eid)))

  // @promotion {
  //   property:    ElicitationStaleTargetInert
  //   status:      promoted
  //   model:       specs/seed/elicitation_lifecycle.qnt
  //   language:    quint
  //   backend:     apalache-temporal
  //   invocation:  echo y | quint verify elicitation_lifecycle.qnt --temporal elicitation_stale_target_inert --max-steps 10
  //   bounds:      { elicitation_ids: 1, response_ops: 2, sessions: 2, generations: 3, max_steps: 10 }
  //   expected:    pass
  //   proto_fields: [none]
  //   semantics:   responses to stale target/session generations do not mutate live Elicitation state
  // }
  temporal elicitation_stale_target_inert =
    always((targetGeneration.get("e1") < sessionGeneration.get(targetSession.get("e1"))).implies(
      state.get("e1") != "answered"
        and firstTerminalState.get("e1") != "answered"
        and answeredBy.get("e1") == "none"
        and answeredResponseOp.get("e1") == "none"))

  // @promotion {
  //   property:    ElicitationWithdrawalFinality
  //   status:      promoted
  //   model:       specs/seed/elicitation_lifecycle.qnt
  //   language:    quint
  //   backend:     apalache-temporal
  //   invocation:  echo y | quint verify elicitation_lifecycle.qnt --temporal elicitation_withdrawal_finality --max-steps 10
  //   bounds:      { elicitation_ids: 1, response_ops: 2, terminal_candidates: 7, max_steps: 10 }
  //   expected:    pass
  //   proto_fields: [none]
  //   semantics:   opener withdrawal terminalizes the Elicitation without allowing later response mutation
  // }
  temporal elicitation_withdrawal_finality =
    always((firstTerminalState.get("e1") == "withdrawn").implies(
      state.get("e1") == "withdrawn"
        and answeredBy.get("e1") == firstAnsweredBy.get("e1")
        and answeredResponseOp.get("e1") == firstAnsweredResponseOp.get("e1")))
}
