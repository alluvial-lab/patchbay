// Patchbay seed model: authority safety (all properties stated-normative)
//
// All authority properties are currently stated-normative (draft): the spawn-
// authority and responder-authority invariants were demoted because their
// formulas inspect state recorded by the accepting action rather than
// independent attempted evidence (trace-fidelity defect), and the descendant-
// grant property is not mutation-survivable. The real independent-actor/
// endpoint/scope verification and the genuine non-cascade formula are v1
// formal-gate work (epic-public-product-contract-executable-release-assurance).
// Grant records are represented as explicit scalar tuples for the bounded
// grant ids; descendant grants are real grant records, not boolean authority
// side-channels.
module authority {
  pure val ACTORS = Set("alice", "bob", "svc")
  pure val ENDPOINTS = Set("ep-web", "ep-cli", "ep-grpc")
  pure val SESSIONS = Set("os1", "os2", "os3")
  pure val RESPONSE_OP_IDS = Set("ro1", "ro2")
  pure val CLAIMED_ACTORS = ACTORS.union(Set("eve"))
  pure val FLEET_SCOPES = Set("scope-fleet")
  pure val SPAWN_GRANT_IDS = Set("g-spawn", "g-session-spawn")
  pure val DESCENDANT_SCOPE = "scope-session"

  var gSpawnStatus: str
  var gSessionSpawnStatus: str
  var gDescOs3Live: str
  var gDescOs3Status: str
  var gDescOs3Issuer: str
  var gDescOs3Subject: str
  var gDescOs3Target: str
  var os3Exists: str
  var os3Spawned: str
  var os3SpawnedBy: str
  var LastSpawnActor: str
  var LastSpawnTarget: str
  var SpawnAttempted: str
  var SpawnAccepted: str

  var ResponseEndpointRo1: str
  var ResponseEndpointRo2: str
  var ResponseAuthenticatedActorRo1: str
  var ResponseAuthenticatedActorRo2: str
  var ResponseClaimedActorRo1: str
  var ResponseClaimedActorRo2: str
  var ResponseAcceptedRo1: str
  var ResponseAcceptedRo2: str

  var phase: int

  def descendantGrantId(session) = if (session == "os3") "g-desc-os3" else "none"
  def sessionExists(session) = if (session == "os3") os3Exists == "yes" else true
  def spawnedSession(session) = if (session == "os3") os3Spawned == "yes" else false
  def grantLive(g) = if (g == "g-spawn") true else if (g == "g-session-spawn") true else if (g == "g-desc-os3") gDescOs3Live == "yes" else false
  def grantStatus(g) = if (g == "g-spawn") gSpawnStatus else if (g == "g-session-spawn") gSessionSpawnStatus else if (g == "g-desc-os3") gDescOs3Status else "revoked"
  def grantIssuer(g) = if (g == "g-desc-os3") gDescOs3Issuer else "alice"
  def grantSubject(g) = if (g == "g-desc-os3") gDescOs3Subject else "alice"
  def grantScope(g) = if (g == "g-spawn") "scope-fleet" else "scope-session"
  def grantEndpoint(g) = "any"
  def grantTarget(g) = if (g == "g-spawn") "fleet" else if (g == "g-session-spawn") "os1" else if (g == "g-desc-os3") gDescOs3Target else "none"
  def grantAllows(g, kind) =
    if ((g == "g-spawn") or (g == "g-session-spawn"))
      kind == "spawn"
    else if (g == "g-desc-os3")
      kind.in(Set("reboot", "snapshot", "stop_session"))
    else
      false

  def endpointActor(endpoint) = if (endpoint == "ep-web") "alice" else if (endpoint == "ep-cli") "bob" else "svc"
  def expectedResponder(ro) = if (ro == "ro1") "alice" else "bob"
  def responseEndpoint(ro) = if (ro == "ro1") ResponseEndpointRo1 else ResponseEndpointRo2
  def responseAuthenticatedActor(ro) = if (ro == "ro1") ResponseAuthenticatedActorRo1 else ResponseAuthenticatedActorRo2
  def responseClaimedActor(ro) = if (ro == "ro1") ResponseClaimedActorRo1 else ResponseClaimedActorRo2
  def responseAccepted(ro) = if (ro == "ro1") ResponseAcceptedRo1 else ResponseAcceptedRo2

  // Action-side helpers.
  def actionGrantAuthorizesSpawn(g, actor) =
    if (grantLive(g))
      if (grantStatus(g) == "active")
        if (FLEET_SCOPES.contains(grantScope(g)))
          if (grantSubject(g) == actor)
            grantAllows(g, "spawn")
          else false
        else false
      else false
    else false

  def liveFleetSpawnGrant(actor) = SPAWN_GRANT_IDS.exists(g => actionGrantAuthorizesSpawn(g, actor))

  def spawnAllowed(actor, targetSession) =
    if (ACTORS.contains(actor))
      if (SESSIONS.contains(targetSession))
        if (not(sessionExists(targetSession)))
          liveFleetSpawnGrant(actor)
        else false
      else false
    else false

  def responseAuthorityOk(responseOpId, endpoint, claimedActor) =
    if (RESPONSE_OP_IDS.contains(responseOpId))
      if (ENDPOINTS.contains(endpoint))
        if (CLAIMED_ACTORS.contains(claimedActor))
          if (endpointActor(endpoint) == expectedResponder(responseOpId))
            claimedActor == expectedResponder(responseOpId)
          else false
        else false
      else false
    else false

  // Independent oracles: do not call action-side guard helpers.
  def acceptedSpawnHasRawFleetGrant =
    if (grantLive("g-spawn"))
      if (grantStatus("g-spawn") == "active")
        if (FLEET_SCOPES.contains(grantScope("g-spawn")))
          if (grantAllows("g-spawn", "spawn"))
            if (grantSubject("g-spawn") == LastSpawnActor)
              ACTORS.contains(grantIssuer("g-spawn"))
            else false
          else false
        else false
      else false
    else false

  def descendantGrantIndependentOk(session) =
    if (session == "os3")
      if (grantLive("g-desc-os3"))
        if (grantIssuer("g-desc-os3") == os3SpawnedBy)
          if (grantSubject("g-desc-os3") == os3SpawnedBy)
            if (grantTarget("g-desc-os3") == "os3")
              if (grantScope("g-desc-os3") == DESCENDANT_SCOPE)
                if (grantEndpoint("g-desc-os3") == "any")
                  if (grantStatus("g-desc-os3").in(Set("active", "revoked")))
                    if (not(grantAllows("g-desc-os3", "spawn")))
                      grantAllows("g-desc-os3", "reboot") and grantAllows("g-desc-os3", "snapshot") and grantAllows("g-desc-os3", "stop_session")
                    else false
                  else false
                else false
              else false
            else false
          else false
        else false
      else false
    else false

  def acceptedResponseIndependentOk(responseOpId) =
    if (ENDPOINTS.contains(responseEndpoint(responseOpId)))
      if (responseAuthenticatedActor(responseOpId) == endpointActor(responseEndpoint(responseOpId)))
        if (responseAuthenticatedActor(responseOpId) == expectedResponder(responseOpId))
          if (responseClaimedActor(responseOpId) == expectedResponder(responseOpId))
            ACTORS.contains(expectedResponder(responseOpId))
          else false
        else false
      else false
    else false

  def spawnRevocationNoCascadeState =
    if (gSpawnStatus == "revoked")
      (if (gDescOs3Live == "yes") (gDescOs3Status == "active") else true) and (SpawnAccepted == "no")
    else
      true

  action init = all {
    gSpawnStatus' = "active",
    gSessionSpawnStatus' = "active",
    gDescOs3Live' = "no",
    gDescOs3Status' = "revoked",
    gDescOs3Issuer' = "none",
    gDescOs3Subject' = "none",
    gDescOs3Target' = "os3",
    os3Exists' = "no",
    os3Spawned' = "no",
    os3SpawnedBy' = "none",
    LastSpawnActor' = "alice",
    LastSpawnTarget' = "os1",
    SpawnAttempted' = "no",
    SpawnAccepted' = "no",
    ResponseEndpointRo1' = "none",
    ResponseEndpointRo2' = "none",
    ResponseAuthenticatedActorRo1' = "none",
    ResponseAuthenticatedActorRo2' = "none",
    ResponseClaimedActorRo1' = "none",
    ResponseClaimedActorRo2' = "none",
    ResponseAcceptedRo1' = "no",
    ResponseAcceptedRo2' = "no",
    phase' = 0,
  }

  action attemptSpawn(actor, targetSession) = all {
    actor.in(ACTORS), targetSession.in(SESSIONS),
    gSpawnStatus' = gSpawnStatus,
    gSessionSpawnStatus' = gSessionSpawnStatus,
    gDescOs3Live' = if (spawnAllowed(actor, targetSession) and (targetSession == "os3")) "yes" else gDescOs3Live,
    gDescOs3Status' = if (spawnAllowed(actor, targetSession) and (targetSession == "os3")) "active" else gDescOs3Status,
    gDescOs3Issuer' = if (spawnAllowed(actor, targetSession) and (targetSession == "os3")) actor else gDescOs3Issuer,
    gDescOs3Subject' = if (spawnAllowed(actor, targetSession) and (targetSession == "os3")) actor else gDescOs3Subject,
    gDescOs3Target' = gDescOs3Target,
    os3Exists' = if (spawnAllowed(actor, targetSession) and (targetSession == "os3")) "yes" else os3Exists,
    os3Spawned' = if (spawnAllowed(actor, targetSession) and (targetSession == "os3")) "yes" else os3Spawned,
    os3SpawnedBy' = if (spawnAllowed(actor, targetSession) and (targetSession == "os3")) actor else os3SpawnedBy,
    LastSpawnActor' = actor,
    LastSpawnTarget' = targetSession,
    SpawnAttempted' = "yes",
    SpawnAccepted' = if (spawnAllowed(actor, targetSession)) "yes" else "no",
    ResponseEndpointRo1' = ResponseEndpointRo1,
    ResponseEndpointRo2' = ResponseEndpointRo2,
    ResponseAuthenticatedActorRo1' = ResponseAuthenticatedActorRo1,
    ResponseAuthenticatedActorRo2' = ResponseAuthenticatedActorRo2,
    ResponseClaimedActorRo1' = ResponseClaimedActorRo1,
    ResponseClaimedActorRo2' = ResponseClaimedActorRo2,
    ResponseAcceptedRo1' = ResponseAcceptedRo1,
    ResponseAcceptedRo2' = ResponseAcceptedRo2,
    phase' = phase + 1,
  }

  action revokeSpawnGrant = all {
    gSpawnStatus' = "revoked",
    gSessionSpawnStatus' = gSessionSpawnStatus,
    gDescOs3Live' = gDescOs3Live,
    gDescOs3Status' = gDescOs3Status,
    gDescOs3Issuer' = gDescOs3Issuer,
    gDescOs3Subject' = gDescOs3Subject,
    gDescOs3Target' = gDescOs3Target,
    os3Exists' = os3Exists,
    os3Spawned' = os3Spawned,
    os3SpawnedBy' = os3SpawnedBy,
    LastSpawnActor' = LastSpawnActor,
    LastSpawnTarget' = LastSpawnTarget,
    SpawnAttempted' = "no",
    SpawnAccepted' = "no",
    ResponseEndpointRo1' = ResponseEndpointRo1,
    ResponseEndpointRo2' = ResponseEndpointRo2,
    ResponseAuthenticatedActorRo1' = ResponseAuthenticatedActorRo1,
    ResponseAuthenticatedActorRo2' = ResponseAuthenticatedActorRo2,
    ResponseClaimedActorRo1' = ResponseClaimedActorRo1,
    ResponseClaimedActorRo2' = ResponseClaimedActorRo2,
    ResponseAcceptedRo1' = ResponseAcceptedRo1,
    ResponseAcceptedRo2' = ResponseAcceptedRo2,
    phase' = phase + 1,
  }

  action revokeDescendantGrant(targetSession) = all {
    targetSession == "os3", gDescOs3Live == "yes",
    gSpawnStatus' = gSpawnStatus,
    gSessionSpawnStatus' = gSessionSpawnStatus,
    gDescOs3Live' = gDescOs3Live,
    gDescOs3Status' = "revoked",
    gDescOs3Issuer' = gDescOs3Issuer,
    gDescOs3Subject' = gDescOs3Subject,
    gDescOs3Target' = gDescOs3Target,
    os3Exists' = os3Exists,
    os3Spawned' = os3Spawned,
    os3SpawnedBy' = os3SpawnedBy,
    LastSpawnActor' = LastSpawnActor,
    LastSpawnTarget' = LastSpawnTarget,
    SpawnAttempted' = "no",
    SpawnAccepted' = "no",
    ResponseEndpointRo1' = ResponseEndpointRo1,
    ResponseEndpointRo2' = ResponseEndpointRo2,
    ResponseAuthenticatedActorRo1' = ResponseAuthenticatedActorRo1,
    ResponseAuthenticatedActorRo2' = ResponseAuthenticatedActorRo2,
    ResponseClaimedActorRo1' = ResponseClaimedActorRo1,
    ResponseClaimedActorRo2' = ResponseClaimedActorRo2,
    ResponseAcceptedRo1' = ResponseAcceptedRo1,
    ResponseAcceptedRo2' = ResponseAcceptedRo2,
    phase' = phase + 1,
  }

  action spawnAfterRevoke(actor, targetSession) = all { (gSpawnStatus == "revoked"), attemptSpawn(actor, targetSession) }

  action attemptElicitationResponse(responseOpId, endpoint, claimedActor) = all {
    responseOpId.in(RESPONSE_OP_IDS), endpoint.in(ENDPOINTS), claimedActor.in(CLAIMED_ACTORS),
    gSpawnStatus' = gSpawnStatus,
    gSessionSpawnStatus' = gSessionSpawnStatus,
    gDescOs3Live' = gDescOs3Live,
    gDescOs3Status' = gDescOs3Status,
    gDescOs3Issuer' = gDescOs3Issuer,
    gDescOs3Subject' = gDescOs3Subject,
    gDescOs3Target' = gDescOs3Target,
    os3Exists' = os3Exists,
    os3Spawned' = os3Spawned,
    os3SpawnedBy' = os3SpawnedBy,
    LastSpawnActor' = LastSpawnActor,
    LastSpawnTarget' = LastSpawnTarget,
    SpawnAttempted' = "no",
    SpawnAccepted' = "no",
    ResponseEndpointRo1' = if (responseOpId == "ro1") endpoint else ResponseEndpointRo1,
    ResponseEndpointRo2' = if (responseOpId == "ro2") endpoint else ResponseEndpointRo2,
    ResponseAuthenticatedActorRo1' = if (responseOpId == "ro1") endpointActor(endpoint) else ResponseAuthenticatedActorRo1,
    ResponseAuthenticatedActorRo2' = if (responseOpId == "ro2") endpointActor(endpoint) else ResponseAuthenticatedActorRo2,
    ResponseClaimedActorRo1' = if (responseOpId == "ro1") claimedActor else ResponseClaimedActorRo1,
    ResponseClaimedActorRo2' = if (responseOpId == "ro2") claimedActor else ResponseClaimedActorRo2,
    ResponseAcceptedRo1' = if (responseOpId == "ro1") (if (responseAuthorityOk(responseOpId, endpoint, claimedActor)) "yes" else "no") else ResponseAcceptedRo1,
    ResponseAcceptedRo2' = if (responseOpId == "ro2") (if (responseAuthorityOk(responseOpId, endpoint, claimedActor)) "yes" else "no") else ResponseAcceptedRo2,
    phase' = 0,
  }

  action step = any {
    nondet actor = ACTORS.oneOf(); nondet targetSession = SESSIONS.oneOf(); all { (phase == 0), attemptSpawn(actor, targetSession) },
    all { (phase == 1), revokeSpawnGrant },
    nondet actorAfterRevoke = ACTORS.oneOf(); nondet targetAfterRevoke = SESSIONS.oneOf(); all { (phase == 2), spawnAfterRevoke(actorAfterRevoke, targetAfterRevoke) },
    nondet responseOpId = RESPONSE_OP_IDS.oneOf(); nondet endpoint = ENDPOINTS.oneOf(); nondet claimedActor = CLAIMED_ACTORS.oneOf(); all { (phase == 3), attemptElicitationResponse(responseOpId, endpoint, claimedActor) },
  }

  // @promotion {
  //   property:    NoCommandWithoutGrant
  //   status:      draft
  //   model:       specs/seed/authority.qnt
  //   language:    quint
  //   backend:     apalache
  //   invocation:  <TBD — not yet checked; promote in a follow-on item>
  //   bounds:      { TBD }
  //   expected:    pass
  //   proto_fields: [none]
  //   semantics:   commands that reach accepted state do so only with a live matching grant
  // }
  // @promotion {
  //   property:    CompoundIssuer
  //   status:      draft
  //   model:       specs/seed/authority.qnt
  //   language:    quint
  //   backend:     apalache
  //   invocation:  <TBD — not yet checked; promote in a follow-on item>
  //   bounds:      { TBD }
  //   expected:    pass
  //   proto_fields: [none]
  //   semantics:   accepted commands use verified session-derived actor identity, not self-asserted payload actor
  // }
  // @promotion {
  //   property:    GrantAuthorityIsCommandKinds
  //   status:      draft
  //   model:       specs/seed/authority.qnt
  //   language:    quint
  //   backend:     apalache
  //   invocation:  <TBD — not yet checked; promote in a follow-on item>
  //   bounds:      { TBD }
  //   expected:    pass
  //   proto_fields: [none]
  //   semantics:   grant checks constrain authority by canonical command kinds, not adapter capability declarations
  // }
  // @promotion {
  //   property:    RevocationPreventsFuture
  //   status:      draft
  //   model:       specs/seed/authority.qnt
  //   language:    quint
  //   backend:     apalache-temporal
  //   invocation:  <TBD — not yet checked; promote in a follow-on item>
  //   bounds:      { TBD }
  //   expected:    pass
  //   proto_fields: [none]
  //   semantics:   a command cannot become accepted in the transition if it is being submitted at or below a revoked generation
  // }
  // @promotion {
  //   property:    FleetAuthorityForSpawn
  //   status:      draft
  //   model:       specs/seed/authority.qnt
  //   language:    quint
  //   backend:     apalache
  //   invocation:  <TBD — demoted; formula does not independently establish the claimed behavior; v1 formal gate owns the real property>
  //   bounds:      { actors: 3, sessions: 3, spawn_grant_scopes: 2, grant_statuses: 2, max_steps: 12 }
  //   expected:    pass
  //   proto_fields: [none]
  //   demotion_reason: invariant inspects LastSpawnActor written by the accepting action, so it proves recorded-actor consistency rather than independently checking the submitting actor against the fleet grant; accepting arbitrary input while recording the grant subject can pass
  //   semantics:   spawn acceptance requires a live fleet-scope spawn Grant whose subject matches the submitting actor; per-session grants alone cannot authorize spawning a not-yet-existing session
  // }

  // @promotion {
  //   property:    SpawnCreatesDescendantGrant
  //   status:      draft
  //   model:       specs/seed/authority.qnt
  //   language:    quint
  //   backend:     apalache
  //   invocation:  <TBD — demoted; model uses invented kind names (reboot/snapshot/stop_session) contradicting PROTOCOL.md:181; allowed-kind set is a hard-coded pure function, not action-created state; v1 formal gate owns the real property>
  //   bounds:      { actors: 3, sessions: 3, spawn_grant_scopes: 2, grant_statuses: 2, max_steps: 12 }
  //   expected:    pass
  //   proto_fields: [none]
  //   demotion_reason: uses invented kind names contradicting canonical descendant-grant allowed-kind set; allowed-kind set is hard-coded, not mutation-survivable
  //   semantics:   successful spawn inserts an explicit descendant Grant record for the spawned session with non-spawn OperationKinds
  // }

  // @promotion {
  //   property:    SpawnRevocationDoesNotCascade
  //   status:      draft
  //   model:       specs/seed/authority.qnt
  //   language:    quint
  //   backend:     apalache-temporal
  //   invocation:  <TBD — demoted; formula does not independently establish the claimed behavior; v1 formal gate owns the real property>
  //   bounds:      { actors: 3, sessions: 3, spawn_grant_scopes: 2, grant_statuses: 2, max_steps: 10 }
  //   expected:    pass
  //   proto_fields: [none]
  //   demotion_reason: formula makes the descendant condition true after deletion; mutating fleet revocation to set gDescOs3Live' = "no" still passes, so preservation is action-encoded rather than a mutation-survivable non-cascade oracle
  //   semantics:   revoking the fleet spawn grant blocks future spawns and, when a descendant grant exists, does not revoke it
  // }

  // @promotion {
  //   property:    ElicitationResponderAuthority
  //   status:      draft
  //   model:       specs/seed/authority.qnt
  //   language:    quint
  //   backend:     apalache
  //   invocation:  <TBD — demoted; formula does not independently establish the claimed behavior; v1 formal gate owns the real property>
  //   bounds:      { actors: 3, endpoints: 3, response_operations: 2, max_steps: 12 }
  //   expected:    pass
  //   proto_fields: [none]
  //   demotion_reason: invariant inspects ResponseEndpoint and ResponseClaimedActor fields written by the accepting action, so it proves recorded endpoint/actor consistency rather than independently checking submitted responder evidence; accepting arbitrary input while recording expected values can pass
  //   semantics:   response Operations are accepted only when the modeled submitting endpoint maps to the expected responder actor and the claimed actor matches that responder
  // }
}
