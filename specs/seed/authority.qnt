// Patchbay seed model: authority safety (DRAFT)
//
// Unit 6 of `feature-formal-model-seed`.
//
// DEFERRAL REASON:
// `CompoundIssuer` involves the transport-endpoint-vs-operator-actor verification interaction, which is the most complex authority property; warrants its own item. The Anti-spoofing caveat from the Alloy brief (the binding of an authenticated identity to transport/session is a dynamic property, not a relational shape) means part of authority-safety lives in this dynamic model, not the Alloy model.
//
// This model is intentionally DRAFT: it compiles and carries the property IDs, but
// the properties are not yet promoted or verified in this unit.
module authority {
  // --- finite bounds (2-3 atoms per set) ---
  pure val ACTORS = Set("alice", "bob", "svc")
  pure val DEVICES = Set("dev-terminal", "dev-cli")
  pure val ENDPOINTS = Set("ep-web", "ep-cli", "ep-grpc")
  pure val OPERATOR_SESSIONS = Set("os1", "os2", "os3")
  pure val SESSION_ACTORS = Map("os1" -> "alice", "os2" -> "alice", "os3" -> "bob")
  pure val GRANTS = Set("g1", "g2", "g3")
  pure val COMMAND_IDS = Set("c1", "c2", "c3")
  pure val COMMAND_KINDS_REGISTRY = Set("reboot", "snapshot", "stop_session")
  pure val ATTEMPTED_KINDS = COMMAND_KINDS_REGISTRY.union(Set("unsupported"))
  pure val CLAIMED_ACTORS = Set("alice", "bob", "eve")
  pure val SCOPES = Set("scope-device", "scope-adapter")
  pure val TARGETS = Set("target-device", "target-adapter")

  // Endpoint->command-kind capability declarations are advisory UX hints.
  pure val ENDPOINT_KIND_SUPPORT = Set(
    (("ep-web", "reboot"), true),
    (("ep-web", "snapshot"), true),
    (("ep-web", "stop_session"), false),
    (("ep-cli", "reboot"), true),
    (("ep-cli", "snapshot"), false),
    (("ep-cli", "stop_session"), true),
    (("ep-grpc", "reboot"), false),
    (("ep-grpc", "snapshot"), false),
    (("ep-grpc", "stop_session"), false)
  )

  // --- state variables ---
  var Actor: Set[str]
  var Device: Set[str]
  var Endpoint: Set[str]
  var OperatorSession: Set[str]
  var Grant: Set[str]
  var GrantScope: Set[str]

  // Grant represented as parallel maps (id -> field)
  var GrantIssuer: str -> str
  var GrantSubject: str -> str
  var GrantScopeById: str -> str
  var GrantEndpoint: str -> str
  var GrantCommandKinds: str -> Set[str]
  var GrantStatus: str -> str

  var CommandKind: Set[str]
  var Target: Set[str]
  var TargetGeneration: str -> int
  var RevocationGeneration: str -> int

  var CommandIssuer: str -> str        // claimed actor in payload (self-asserted)
  var AuthorityDomain: str

  // Abstract session/acceptance surface modeled independently of the dedicated session model
  var SessionGeneration: str -> int
  var SessionActor: str -> str
  var AdapterGeneration: str -> int
  var CorrelationRef: str -> str

  var CommandSession: str -> str
  var CommandEndpoint: str -> str
  var CommandTarget: str -> str
  var CommandScope: str -> str
  var CommandKindRequest: str -> str
  var CommandTargetGeneration: str -> int
  var CommandVerifiedActor: str -> str  // actor asserted by transport session verification
  var CommandAccepted: str -> bool
  var CommandAcceptedByGrant: str -> bool
  var CommandDelivered: str -> bool
  var CommandState: str -> str
  var CommandRevocationReason: str -> str

  var AdapterSupport: (str, str) -> bool

  var LastAttemptedCommand: str
  var AcceptanceGateOpen: bool

  // ---------------------------------------------------------------------------
  // Helpers
  // ---------------------------------------------------------------------------

  def sessionKnown(session) = OperatorSession.contains(session) and SessionActor.keys().contains(session)

  def sessionActor(session) = SessionActor.get(session)

  def matchScope(scope) = GrantScope.contains(scope)

  def grantMatches(session, endpoint, commandKind, scope) =
    Grant.exists(g =>
      GrantStatus.get(g) == "active" and
      GrantScopeById.get(g) == scope and
      GrantSubject.get(g) == sessionActor(session) and
      GrantCommandKinds.get(g).contains(commandKind) and
      (GrantEndpoint.get(g) == "any" or GrantEndpoint.get(g) == endpoint) and
      GrantIssuer.get(g).in(Actor)
    )

  def claimMatchesVerified(session, claimedActor) =
    sessionKnown(session) and
    sessionActor(session) == claimedActor

  def targetNotRevoked(target) = TargetGeneration.get(target) >= RevocationGeneration.get(target)

  def canAcceptCommand(session, endpoint, claimedActor, kind, target, scope) =
    AcceptanceGateOpen and
    kind.in(CommandKind) and
    sessionKnown(session) and
    Endpoint.contains(endpoint) and
    Target.contains(target) and
    matchScope(scope) and
    targetNotRevoked(target) and
    grantMatches(session, endpoint, kind, scope) and
    claimMatchesVerified(session, claimedActor)

  def canDeliver(session, endpoint, kind, target, scope, claimedActor) =
    canAcceptCommand(session, endpoint, claimedActor, kind, target, scope) and
    AdapterSupport.get((endpoint, kind))

  def commandAcceptanceReason(session, endpoint, claimedActor, kind, target, scope) =
    if (not(sessionKnown(session)))
      "no_session"
    else if (not(scope.in(SCOPES)))
      "bad_scope"
    else if (not(claimMatchesVerified(session, claimedActor)))
      "mismatched_actor"
    else if (not(commandAcceptanceGrantKind(session, endpoint, kind, scope)))
      "no_grant"
    else if (not(targetNotRevoked(target)))
      "revoked_target"
    else if (not(kind.in(CommandKind)))
      "unknown_kind"
    else
      "accepted"

  def commandAcceptanceGrantKind(session, endpoint, kind, scope) =
    Grant.exists(g =>
      GrantStatus.get(g) == "active" and
      GrantScopeById.get(g) == scope and
      GrantSubject.get(g) == sessionActor(session) and
      GrantCommandKinds.get(g).contains(kind) and
      (GrantEndpoint.get(g) == "any" or GrantEndpoint.get(g) == endpoint)
    )

  // ---------------------------------------------------------------------------
  // Actions
  // ---------------------------------------------------------------------------

  action init = all {
    Actor' = ACTORS,
    Device' = DEVICES,
    Endpoint' = ENDPOINTS,
    OperatorSession' = OPERATOR_SESSIONS,
    Grant' = GRANTS,
    GrantScope' = SCOPES,

    GrantIssuer' = Map(
      "g1" -> "alice",
      "g2" -> "alice",
      "g3" -> "bob"
    ),
    GrantSubject' = Map(
      "g1" -> "alice",
      "g2" -> "alice",
      "g3" -> "bob"
    ),
    GrantScopeById' = Map(
      "g1" -> "scope-device",
      "g2" -> "scope-adapter",
      "g3" -> "scope-device"
    ),
    GrantEndpoint' = Map(
      "g1" -> "ep-web",
      "g2" -> "any",
      "g3" -> "ep-cli"
    ),
    GrantCommandKinds' = Map(
      "g1" -> Set("reboot", "snapshot"),
      "g2" -> Set("reboot", "stop_session"),
      "g3" -> Set("snapshot")
    ),
    GrantStatus' = Map("g1" -> "active", "g2" -> "active", "g3" -> "active"),

    CommandKind' = COMMAND_KINDS_REGISTRY,
    Target' = TARGETS,
    TargetGeneration' = TARGETS.mapBy(t => 0),
    RevocationGeneration' = TARGETS.mapBy(t => 0),

    CommandIssuer' = Map("c1" -> "alice", "c2" -> "alice", "c3" -> "bob"),
    AuthorityDomain' = "domain-main",

    SessionGeneration' = OPERATOR_SESSIONS.mapBy(s => 0),
    SessionActor' = SESSION_ACTORS,
    AdapterGeneration' = ENDPOINTS.mapBy(e => 0),
    CorrelationRef' = Map("c1" -> "corr-c1", "c2" -> "corr-c2", "c3" -> "corr-c3"),

    CommandSession' = Map("c1" -> "os1", "c2" -> "os2", "c3" -> "os1"),
    CommandEndpoint' = Map("c1" -> "ep-web", "c2" -> "ep-cli", "c3" -> "ep-web"),
    CommandTarget' = Map("c1" -> "target-device", "c2" -> "target-device", "c3" -> "target-adapter"),
    CommandScope' = Map("c1" -> "scope-device", "c2" -> "scope-device", "c3" -> "scope-adapter"),
    CommandKindRequest' = Map("c1" -> "reboot", "c2" -> "snapshot", "c3" -> "snapshot"),
    CommandTargetGeneration' = Map("c1" -> 0, "c2" -> 0, "c3" -> 0),
    CommandVerifiedActor' = Map("c1" -> "alice", "c2" -> "alice", "c3" -> "bob"),
    CommandAccepted' = Map("c1" -> false, "c2" -> false, "c3" -> false),
    CommandAcceptedByGrant' = Map("c1" -> false, "c2" -> false, "c3" -> false),
    CommandDelivered' = Map("c1" -> false, "c2" -> false, "c3" -> false),
    CommandState' = Map("c1" -> "none", "c2" -> "none", "c3" -> "none"),
    CommandRevocationReason' = Map("c1" -> "none", "c2" -> "none", "c3" -> "none"),

    AdapterSupport' = ENDPOINT_KIND_SUPPORT.setToMap(),
    LastAttemptedCommand' = "c1",
    AcceptanceGateOpen' = true,
  }

  action attemptSubmit(cmd, session, endpoint, claimedActor, target, scope, kind) = all {
    cmd.in(COMMAND_IDS),
    session.in(OPERATOR_SESSIONS),
    endpoint.in(ENDPOINTS),
    claimedActor.in(CLAIMED_ACTORS),
    target.in(TARGETS),
    scope.in(SCOPES),
    kind.in(ATTEMPTED_KINDS),

    Actor' = Actor,
    Device' = Device,
    Endpoint' = Endpoint,
    OperatorSession' = OperatorSession,
    Grant' = Grant,
    GrantScope' = GrantScope,
    GrantIssuer' = GrantIssuer,
    GrantSubject' = GrantSubject,
    GrantScopeById' = GrantScopeById,
    GrantEndpoint' = GrantEndpoint,
    GrantCommandKinds' = GrantCommandKinds,
    GrantStatus' = GrantStatus,

    CommandKind' = CommandKind,
    Target' = Target,
    TargetGeneration' = TargetGeneration,
    RevocationGeneration' = RevocationGeneration,

    CommandIssuer' = CommandIssuer.put(cmd, claimedActor),
    CommandSession' = CommandSession.put(cmd, session),
    CommandEndpoint' = CommandEndpoint.put(cmd, endpoint),
    CommandTarget' = CommandTarget.put(cmd, target),
    CommandScope' = CommandScope.put(cmd, scope),
    CommandKindRequest' = CommandKindRequest.put(cmd, kind),
    CommandTargetGeneration' = CommandTargetGeneration.put(cmd, TargetGeneration.get(target)),
    CommandVerifiedActor' = CommandVerifiedActor.put(cmd, sessionActor(session)),
    CommandAcceptedByGrant' = CommandAcceptedByGrant.put(
      cmd,
      commandAcceptanceGrantKind(session, endpoint, kind, scope)
    ),
    CommandAccepted' = CommandAccepted.put(
      cmd,
      canAcceptCommand(session, endpoint, claimedActor, kind, target, scope)
    ),
    CommandDelivered' = CommandDelivered.put(
      cmd,
      canDeliver(session, endpoint, kind, target, scope, claimedActor)
    ),
    CommandState' = CommandState.put(
      cmd,
      commandAcceptanceReason(session, endpoint, claimedActor, kind, target, scope)
    ),
    CommandRevocationReason' = CommandRevocationReason.put(
      cmd,
      if (targetNotRevoked(target))
        "none"
      else
        "revoked"
    ),

    SessionGeneration' = SessionGeneration,
    SessionActor' = SessionActor,
    AdapterGeneration' = AdapterGeneration,
    CorrelationRef' = CorrelationRef,
    AuthorityDomain' = AuthorityDomain,
    AdapterSupport' = AdapterSupport,
    LastAttemptedCommand' = cmd,
    AcceptanceGateOpen' = AcceptanceGateOpen,
  }

  // Note: this action intentionally updates all variables, even those not logically
  // changed, to satisfy the action-mode shape checks and avoid branch-coverage
  // mismatches.
  action rotateSession(session) = all {
    session.in(OPERATOR_SESSIONS),
    Actor' = Actor,
    Device' = Device,
    Endpoint' = Endpoint,
    OperatorSession' = OperatorSession,
    Grant' = Grant,
    GrantScope' = GrantScope,
    GrantIssuer' = GrantIssuer,
    GrantSubject' = GrantSubject,
    GrantScopeById' = GrantScopeById,
    GrantEndpoint' = GrantEndpoint,
    GrantCommandKinds' = GrantCommandKinds,
    GrantStatus' = GrantStatus,
    CommandKind' = CommandKind,
    Target' = Target,
    TargetGeneration' = TargetGeneration,
    RevocationGeneration' = RevocationGeneration,
    CommandIssuer' = CommandIssuer,
    CommandSession' = CommandSession,
    CommandEndpoint' = CommandEndpoint,
    CommandTarget' = CommandTarget,
    CommandScope' = CommandScope,
    CommandKindRequest' = CommandKindRequest,
    CommandTargetGeneration' = CommandTargetGeneration,
    CommandVerifiedActor' = CommandVerifiedActor,
    CommandAccepted' = CommandAccepted,
    CommandAcceptedByGrant' = CommandAcceptedByGrant,
    CommandDelivered' = CommandDelivered,
    CommandState' = CommandState,
    CommandRevocationReason' = CommandRevocationReason,
    AuthorityDomain' = AuthorityDomain,
    SessionGeneration' = SessionGeneration.put(session, SessionGeneration.get(session) + 1),
    SessionActor' = SessionActor,
    AdapterGeneration' = AdapterGeneration,
    CorrelationRef' = CorrelationRef,
    AdapterSupport' = AdapterSupport,
    LastAttemptedCommand' = LastAttemptedCommand,
    AcceptanceGateOpen' = AcceptanceGateOpen,
  }

  action revokeTarget(target) = all {
    target.in(TARGETS),
    Actor' = Actor,
    Device' = Device,
    Endpoint' = Endpoint,
    OperatorSession' = OperatorSession,
    Grant' = Grant,
    GrantScope' = GrantScope,
    GrantIssuer' = GrantIssuer,
    GrantSubject' = GrantSubject,
    GrantScopeById' = GrantScopeById,
    GrantEndpoint' = GrantEndpoint,
    GrantCommandKinds' = GrantCommandKinds,
    GrantStatus' = GrantStatus,
    CommandKind' = CommandKind,
    Target' = Target,
    TargetGeneration' = TargetGeneration,
    RevocationGeneration' = RevocationGeneration.put(target, RevocationGeneration.get(target) + 1),
    CommandIssuer' = CommandIssuer,
    CommandSession' = CommandSession,
    CommandEndpoint' = CommandEndpoint,
    CommandTarget' = CommandTarget,
    CommandScope' = CommandScope,
    CommandKindRequest' = CommandKindRequest,
    CommandTargetGeneration' = CommandTargetGeneration,
    CommandVerifiedActor' = CommandVerifiedActor,
    CommandAccepted' = CommandAccepted,
    CommandAcceptedByGrant' = CommandAcceptedByGrant,
    CommandDelivered' = CommandDelivered,
    CommandState' = CommandState,
    CommandRevocationReason' = CommandRevocationReason,
    AuthorityDomain' = AuthorityDomain,
    SessionGeneration' = SessionGeneration,
    SessionActor' = SessionActor,
    AdapterGeneration' = AdapterGeneration,
    CorrelationRef' = CorrelationRef,
    AdapterSupport' = AdapterSupport,
    LastAttemptedCommand' = LastAttemptedCommand,
    AcceptanceGateOpen' = AcceptanceGateOpen,
  }

  // Reusable one-step driver.
  action step = {
    nondet cmd = COMMAND_IDS.oneOf()
    nondet session = OPERATOR_SESSIONS.oneOf()
    nondet endpoint = ENDPOINTS.oneOf()
    nondet claimedActor = CLAIMED_ACTORS.oneOf()
    nondet target = TARGETS.oneOf()
    nondet scope = SCOPES.oneOf()
    nondet kind = ATTEMPTED_KINDS.oneOf()
    attemptSubmit(cmd, session, endpoint, claimedActor, target, scope, kind)
  }

  // ---------------------------------------------------------------------------
  // Checked properties (DRAFT — reserved IDs are intentionally draft-status)
  // ---------------------------------------------------------------------------

  // @promotion {
  //   property:    NoCommandWithoutGrant
  //   tier:        stated-normative
  //   status:      draft
  //   model:       specs/seed/authority.qnt
  //   language:    quint
  //   backend:     apalache
  //   invocation:  <TBD — not yet checked; promote in a follow-on item>
  //   bounds:      { TBD }
  //   expected:    pass
  //   proto_fields: [none]
  //   semantics:   commands that reach accepted state do so only with a live matching grant
  //   deferral_reason: `CompoundIssuer` involves the transport-endpoint-vs-operator-actor verification interaction, which is the most complex authority property; warrants its own item. The Anti-spoofing caveat from the Alloy brief (the binding of an authenticated identity to transport/session is a dynamic property, not a relational shape) means part of authority-safety lives in this dynamic model, not the Alloy model.
  // }
  val NoCommandWithoutGrant = COMMAND_IDS.forall(cmd =>
    not(CommandAccepted.get(cmd)) or
      (CommandAccepted.get(cmd) implies CommandAcceptedByGrant.get(cmd)))

  // @promotion {
  //   property:    CompoundIssuer
  //   tier:        stated-normative
  //   status:      draft
  //   model:       specs/seed/authority.qnt
  //   language:    quint
  //   backend:     apalache
  //   invocation:  <TBD — not yet checked; promote in a follow-on item>
  //   bounds:      { TBD }
  //   expected:    pass
  //   proto_fields: [none]
  //   semantics:   accepted commands use verified session-derived actor identity, not self-asserted payload actor
  //   deferral_reason: `CompoundIssuer` involves the transport-endpoint-vs-operator-actor verification interaction, which is the most complex authority property; warrants its own item. The Anti-spoofing caveat from the Alloy brief (the binding of an authenticated identity to transport/session is a dynamic property, not a relational shape) means part of authority-safety lives in this dynamic model, not the Alloy model.
  // }
  val CompoundIssuer = COMMAND_IDS.forall(cmd =>
    not(CommandAccepted.get(cmd)) or
      (CommandIssuer.get(cmd) == CommandVerifiedActor.get(cmd) and
       CommandVerifiedActor.get(cmd).in(Actor) and
       CommandSession.get(cmd).in(OperatorSession)))

  // @promotion {
  //   property:    GrantAuthorityIsCommandKinds
  //   tier:        stated-normative
  //   status:      draft
  //   model:       specs/seed/authority.qnt
  //   language:    quint
  //   backend:     apalache
  //   invocation:  <TBD — not yet checked; promote in a follow-on item>
  //   bounds:      { TBD }
  //   expected:    pass
  //   proto_fields: [none]
  //   semantics:   grant checks constrain authority by canonical command kinds, not adapter capability declarations
  //   deferral_reason: `CompoundIssuer` involves the transport-endpoint-vs-operator-actor verification interaction, which is the most complex authority property; warrants its own item. The Anti-spoofing caveat from the Alloy brief (the binding of an authenticated identity to transport/session is a dynamic property, not a relational shape) means part of authority-safety lives in this dynamic model, not the Alloy model.
  // }
  val GrantAuthorityIsCommandKinds = COMMAND_IDS.forall(cmd =>
    not(CommandAccepted.get(cmd)) or
      (CommandKindRequest.get(cmd).in(CommandKind) and
       CommandAcceptedByGrant.get(cmd)))

  // @promotion {
  //   property:    RevocationPreventsFuture
  //   tier:        stated-normative
  //   status:      draft
  //   model:       specs/seed/authority.qnt
  //   language:    quint
  //   backend:     apalache-temporal
  //   invocation:  <TBD — not yet checked; promote in a follow-on item>
  //   bounds:      { TBD }
  //   expected:    pass
  //   proto_fields: [none]
  //   semantics:   a command cannot become accepted in the transition if it is being submitted at or below a revoked generation
  //   deferral_reason: `CompoundIssuer` involves the transport-endpoint-vs-operator-actor verification interaction, which is the most complex authority property; warrants its own item. The Anti-spoofing caveat from the Alloy brief (the binding of an authenticated identity to transport/session is a dynamic property, not a relational shape) means part of authority-safety lives in this dynamic model, not the Alloy model.
  // }
  temporal RevocationPreventsFuture = always(COMMAND_IDS.forall(cmd =>
    (not(CommandAccepted.get(cmd)) and next(CommandAccepted.get(cmd)))
      .implies(next(CommandTargetGeneration.get(cmd)) >= RevocationGeneration.get(CommandTarget.get(cmd)))
  ))
}
