// Patchbay bounded model: operator-session generation and control-surface fences.
//
// The four properties below deliberately record raw attempted evidence separately from
// the acceptance result. A guard-removal mutation that accepts an invalidated generation
// or revoked identity therefore produces a counterexample instead of rewriting the
// evidence that the invariant reads. All properties remain stated-normative until the
// independent checker, non-vacuity, mutation, and promoted-vector gates are completed.
module session_principal_revocation {
  pure val ACTORS = Set("operator-a", "operator-b")
  pure val PRINCIPALS = Set("principal-a", "principal-b")
  pure val ENDPOINTS = Set("endpoint-a", "endpoint-b")
  pure val DEVICES = Set("device-a", "device-b")
  pure val GENERATIONS = 0.to(3)
  pure val SCOPES = Set("session", "principal", "endpoint", "device")

  var sessionFloor: str -> int
  var principalRevoked: str -> bool
  var endpointRevoked: str -> bool
  var deviceRevoked: str -> bool

  // Independent attempted evidence: never derive these from accepted.
  var attemptedScope: str
  var attemptedTarget: str
  var attemptedGeneration: int
  var attemptedActor: str
  var accepted: bool
  var acceptanceReachable: bool

  action init = all {
    sessionFloor' = ACTORS.mapBy(actor => 0),
    principalRevoked' = PRINCIPALS.mapBy(principal => false),
    endpointRevoked' = ENDPOINTS.mapBy(endpoint => false),
    deviceRevoked' = DEVICES.mapBy(device => false),
    attemptedScope' = "none",
    attemptedTarget' = "none",
    attemptedGeneration' = 0,
    attemptedActor' = "none",
    accepted' = false,
    acceptanceReachable' = false,
  }

  action revokeAll(actor, generation) = all {
    actor.in(ACTORS), generation.in(GENERATIONS),
    sessionFloor' = sessionFloor.set(actor, generation),
    principalRevoked' = principalRevoked,
    endpointRevoked' = endpointRevoked,
    deviceRevoked' = deviceRevoked,
    attemptedScope' = "revoke-all",
    attemptedTarget' = actor,
    attemptedGeneration' = generation,
    attemptedActor' = actor,
    accepted' = false,
    acceptanceReachable' = acceptanceReachable,
  }

  action revokePrincipal(principal) = all {
    principal.in(PRINCIPALS),
    sessionFloor' = sessionFloor,
    principalRevoked' = principalRevoked.set(principal, true),
    endpointRevoked' = endpointRevoked,
    deviceRevoked' = deviceRevoked,
    attemptedScope' = "revoke-principal",
    attemptedTarget' = principal,
    attemptedGeneration' = 0,
    attemptedActor' = "operator-a",
    accepted' = false,
    acceptanceReachable' = acceptanceReachable,
  }

  action revokeEndpoint(endpoint) = all {
    endpoint.in(ENDPOINTS),
    sessionFloor' = sessionFloor,
    principalRevoked' = principalRevoked,
    endpointRevoked' = endpointRevoked.set(endpoint, true),
    deviceRevoked' = deviceRevoked,
    attemptedScope' = "revoke-endpoint",
    attemptedTarget' = endpoint,
    attemptedGeneration' = 0,
    attemptedActor' = "operator-a",
    accepted' = false,
    acceptanceReachable' = acceptanceReachable,
  }

  action revokeDevice(device) = all {
    device.in(DEVICES),
    sessionFloor' = sessionFloor,
    principalRevoked' = principalRevoked,
    endpointRevoked' = endpointRevoked,
    deviceRevoked' = deviceRevoked.set(device, true),
    attemptedScope' = "revoke-device",
    attemptedTarget' = device,
    attemptedGeneration' = 0,
    attemptedActor' = "operator-a",
    accepted' = false,
    acceptanceReachable' = acceptanceReachable,
  }

  // This action is intentionally permissive with respect to incoming evidence. The
  // acceptance decision is the implementation rule under test, while the attempted
  // target/generation values remain the independent oracle inputs.
  action operationAttempt = {
    nondet principal = PRINCIPALS.oneOf()
    nondet actor = ACTORS.oneOf()
    nondet generation = GENERATIONS.oneOf()
    all {
      sessionFloor' = sessionFloor,
      principalRevoked' = principalRevoked,
      endpointRevoked' = endpointRevoked,
      deviceRevoked' = deviceRevoked,
      attemptedScope' = "operation",
      attemptedTarget' = principal,
      attemptedGeneration' = generation,
      attemptedActor' = actor,
      accepted' =
        actor == "operator-a"
        and generation > sessionFloor.get(actor)
        and not(principalRevoked.get(principal))
        and not(endpointRevoked.get(if (principal == "principal-a") "endpoint-a" else "endpoint-b"))
        and not(deviceRevoked.get(if (principal == "principal-a") "device-a" else "device-b")),
      acceptanceReachable' = true,
    }
  }

  action step = any {
    nondet actor = ACTORS.oneOf(); nondet generation = GENERATIONS.oneOf(); revokeAll(actor, generation),
    nondet principal = PRINCIPALS.oneOf(); revokePrincipal(principal),
    nondet endpoint = ENDPOINTS.oneOf(); revokeEndpoint(endpoint),
    nondet device = DEVICES.oneOf(); revokeDevice(device),
    operationAttempt,
  }

  // These invariants inspect only the independent attempted evidence and the
  // state fences. They are intentionally not promoted to checked-normative.
  // @promotion {
  //   property:    RevokeAllInvalidatesPriorSessionGeneration
  //   status:      draft
  //   model:       specs/seed/session_principal_revocation.qnt
  //   language:    quint
  //   backend:     apalache
  //   invocation:  quint verify session_principal_revocation.qnt --invariant revoke_all_invalidates_prior_session_generation --max-steps 8
  //   bounds:      { actors: 2, generations: 4, max_steps: 8 }
  //   expected:    pass
  //   proto_fields: [patchbay.RevokeAllOperatorSessionsRequest.reason_code, patchbay.RevokeAllOperatorSessionsResult.invalidated_through_generation, patchbay.VerifyOperatorPasswordResult.operator_session_generation]
  //   semantics:   a session generation at or below the durable revoke-all floor cannot be accepted
  // }
  val revoke_all_invalidates_prior_session_generation =
    attemptedScope == "operation" and accepted
      implies attemptedGeneration > sessionFloor.get(attemptedActor)

  // @promotion {
  //   property:    PrincipalRevocationPreventsFuture
  //   status:      draft
  //   model:       specs/seed/session_principal_revocation.qnt
  //   language:    quint
  //   backend:     apalache
  //   invocation:  quint verify session_principal_revocation.qnt --invariant principal_revocation_prevents_future --max-steps 8
  //   bounds:      { principals: 2, max_steps: 8 }
  //   expected:    pass
  //   proto_fields: [patchbay.RevokeControlSurfacePrincipalRequest.principal_id, patchbay.RevokeControlSurfaceResult.newly_revoked]
  //   semantics:   a principal fence rejects future Operations from that exact credential principal
  // }
  val principal_revocation_prevents_future =
    attemptedScope == "operation" and principalRevoked.get(attemptedTarget)
      implies not(accepted)

  // @promotion {
  //   property:    EndpointRevocationPreventsFuture
  //   status:      draft
  //   model:       specs/seed/session_principal_revocation.qnt
  //   language:    quint
  //   backend:     apalache
  //   invocation:  quint verify session_principal_revocation.qnt --invariant endpoint_revocation_prevents_future --max-steps 8
  //   bounds:      { endpoints: 2, max_steps: 8 }
  //   expected:    pass
  //   proto_fields: [patchbay.RevokeControlSurfaceEndpointRequest.endpoint_id, patchbay.RevokeControlSurfaceResult.revoked_principal_count]
  //   semantics:   an endpoint fence rejects future Operations from principals bound to that endpoint
  // }
  val endpoint_revocation_prevents_future =
    attemptedScope == "operation" and endpointRevoked.get(if (attemptedTarget == "principal-a") "endpoint-a" else "endpoint-b")
      implies not(accepted)

  // @promotion {
  //   property:    DeviceRevocationPreventsFuture
  //   status:      draft
  //   model:       specs/seed/session_principal_revocation.qnt
  //   language:    quint
  //   backend:     apalache
  //   invocation:  quint verify session_principal_revocation.qnt --invariant device_revocation_prevents_future --max-steps 8
  //   bounds:      { devices: 2, max_steps: 8 }
  //   expected:    pass
  //   proto_fields: [patchbay.RevokeControlSurfaceEndpointRequest.device_id, patchbay.RevokeControlSurfaceResult.revoked_principal_count]
  //   semantics:   a device fence rejects future Operations from principals bound to that device
  // }
  val device_revocation_prevents_future =
    attemptedScope == "operation" and deviceRevoked.get(if (attemptedTarget == "principal-a") "device-a" else "device-b")
      implies not(accepted)
}
