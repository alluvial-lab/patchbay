// Patchbay seed model: subscription authority (draft obligations; no promoted properties)
//
// Unit SUB is split out of authority.qnt because composing subscription cursor/event
// state with the spawn temporal property caused Apalache's temporal backend to fail
// before checking Unit SA's former temporal property. The semantics are unchanged:
// subscription authorization queries real bounded Grant tuples, establishment is
// permissive and audited, and replay state is available for future independent-oracle work.
module subscription_authority {
  pure val ACTORS = Set("alice", "bob", "svc")
  pure val STREAMS = Set("stream-ops", "stream-elicitations")
  pure val FILTERS = Set("all", "ops-only")
  pure val CURSORS = 0.to(4)
  pure val GRANTS = Set("g-sub-ops-all", "g-sub-elicitations-ops")
  pure val EVENT_LSNS = 1.to(5)

  var gSubOpsAllStatus: str
  var gSubElicitationsOpsStatus: str
  var subscriptionId: str
  var subscriptionGrantId: str
  var subscriptionCursor: int
  var subscriptionStream: str
  var subscriptionFilter: str
  var auditRecords: int
  var operationRecordsCreated: int
  var eventLsn: int
  var eventStream: int -> str
  var eventFilter: int -> str
  var replayedEvents: int
  var LastSubscriptionActor: str
  var LastSubscriptionScope: str
  var SubscriptionAttempted: str
  var SubscriptionAccepted: str
  var SubscriptionEstablishAttempts: int
  var phase: int

  def streamScope(stream, filt) =
    if (stream == "stream-ops")
      if (filt == "all") "scope-stream-ops-all" else "scope-stream-ops-ops-only"
    else
      if (filt == "all") "scope-stream-elicitations-all" else "scope-stream-elicitations-ops-only"

  // Bounded Grant tuple accessors. These are the model's raw Grant records; there
  // is no subscriptionGrantLive boolean side-channel.
  def grantLive(g) = GRANTS.contains(g)
  def grantStatus(g) = if (g == "g-sub-ops-all") gSubOpsAllStatus else if (g == "g-sub-elicitations-ops") gSubElicitationsOpsStatus else "revoked"
  def grantSubject(g) = if (g == "g-sub-elicitations-ops") "bob" else "alice"
  def grantScope(g) = if (g == "g-sub-ops-all") streamScope("stream-ops", "all") else if (g == "g-sub-elicitations-ops") streamScope("stream-elicitations", "ops-only") else "none"
  def grantAllows(g, kind) = GRANTS.contains(g) and kind == "subscribe"

  // Action-side helper. Future promoted invariants must not reuse this acceptance predicate.
  def actionGrantAuthorizesSubscription(g, actor, stream, filt) =
    if (grantLive(g))
      if (grantStatus(g) == "active")
        if (grantScope(g) == streamScope(stream, filt))
          if (grantSubject(g) == actor)
            grantAllows(g, "subscribe")
          else false
        else false
      else false
    else false

  def liveSubscriptionGrantId(actor, stream, filt) =
    if (actionGrantAuthorizesSubscription("g-sub-ops-all", actor, stream, filt))
      "g-sub-ops-all"
    else if (actionGrantAuthorizesSubscription("g-sub-elicitations-ops", actor, stream, filt))
      "g-sub-elicitations-ops"
    else
      "none"

  def subscriptionAllowed(actor, stream, filt) = liveSubscriptionGrantId(actor, stream, filt) != "none"

  // Recorded-state consistency helper. It does not call actionGrantAuthorizesSubscription,
  // liveSubscriptionGrantId, or subscriptionAllowed, but its inputs are still written by
  // the accepting action; it is therefore not an independent attempted-evidence oracle.
  def acceptedSubscriptionHasRawGrant =
    if (GRANTS.contains(subscriptionGrantId))
      if (grantLive(subscriptionGrantId))
        if (grantStatus(subscriptionGrantId) == "active")
          if (grantSubject(subscriptionGrantId) == LastSubscriptionActor)
            if (grantScope(subscriptionGrantId) == LastSubscriptionScope)
              grantAllows(subscriptionGrantId, "subscribe")
            else false
          else false
        else false
      else false
    else false

  def eventMatchesSubscription(lsn) =
    EVENT_LSNS.contains(lsn)
      and lsn <= eventLsn
      and eventStream.get(lsn) == subscriptionStream
      and eventFilter.get(lsn) == subscriptionFilter

  def replayEventFor(cursor) =
    if (SubscriptionAccepted == "yes" and eventLsn > cursor and eventMatchesSubscription(eventLsn))
      eventLsn
    else
      0

  def replayedEventIndependentOk(lsn) =
    EVENT_LSNS.contains(lsn)
      and lsn > subscriptionCursor
      and lsn <= eventLsn
      and eventStream.get(lsn) == subscriptionStream
      and eventFilter.get(lsn) == subscriptionFilter
      and acceptedSubscriptionHasRawGrant

  action init = all {
    gSubOpsAllStatus' = "active",
    gSubElicitationsOpsStatus' = "revoked",
    subscriptionId' = "none",
    subscriptionGrantId' = "none",
    subscriptionCursor' = 0,
    subscriptionStream' = "stream-ops",
    subscriptionFilter' = "all",
    auditRecords' = 0,
    operationRecordsCreated' = 0,
    eventLsn' = 0,
    eventStream' = EVENT_LSNS.mapBy(_ => "none"),
    eventFilter' = EVENT_LSNS.mapBy(_ => "none"),
    replayedEvents' = 0,
    LastSubscriptionActor' = "alice",
    LastSubscriptionScope' = streamScope("stream-ops", "all"),
    SubscriptionAttempted' = "no",
    SubscriptionAccepted' = "no",
    SubscriptionEstablishAttempts' = 0,
    phase' = 0,
  }

  // PERMISSIVE establishment: every bounded actor/stream/filter/cursor attempt is
  // recorded, allowed or denied. The action consults Grant tuples to decide the
  // acceptance flag and writes the state available to future trace-faithful formulas.
  action attemptEstablish(actor, stream, filt, cursor) = all {
    actor.in(ACTORS), stream.in(STREAMS), filt.in(FILTERS), cursor.in(CURSORS),
    gSubOpsAllStatus' = gSubOpsAllStatus,
    gSubElicitationsOpsStatus' = gSubElicitationsOpsStatus,
    subscriptionId' = if (subscriptionAllowed(actor, stream, filt)) "sub1" else "none",
    subscriptionGrantId' = liveSubscriptionGrantId(actor, stream, filt),
    subscriptionCursor' = cursor,
    subscriptionStream' = stream,
    subscriptionFilter' = filt,
    auditRecords' = auditRecords + 1,
    operationRecordsCreated' = operationRecordsCreated,
    eventLsn' = eventLsn,
    eventStream' = eventStream,
    eventFilter' = eventFilter,
    replayedEvents' = replayedEvents,
    LastSubscriptionActor' = actor,
    LastSubscriptionScope' = streamScope(stream, filt),
    SubscriptionAttempted' = "yes",
    SubscriptionAccepted' = if (subscriptionAllowed(actor, stream, filt)) "yes" else "no",
    SubscriptionEstablishAttempts' = SubscriptionEstablishAttempts + 1,
    phase' = 1,
  }

  action emitEvent(stream, filt) = all {
    stream.in(STREAMS), filt.in(FILTERS), eventLsn < 5,
    gSubOpsAllStatus' = gSubOpsAllStatus,
    gSubElicitationsOpsStatus' = gSubElicitationsOpsStatus,
    subscriptionId' = subscriptionId,
    subscriptionGrantId' = subscriptionGrantId,
    subscriptionCursor' = subscriptionCursor,
    subscriptionStream' = subscriptionStream,
    subscriptionFilter' = subscriptionFilter,
    auditRecords' = auditRecords,
    operationRecordsCreated' = operationRecordsCreated,
    eventLsn' = eventLsn + 1,
    eventStream' = eventStream.set(eventLsn + 1, stream),
    eventFilter' = eventFilter.set(eventLsn + 1, filt),
    replayedEvents' = replayedEvents,
    LastSubscriptionActor' = LastSubscriptionActor,
    LastSubscriptionScope' = LastSubscriptionScope,
    SubscriptionAttempted' = SubscriptionAttempted,
    SubscriptionAccepted' = SubscriptionAccepted,
    SubscriptionEstablishAttempts' = SubscriptionEstablishAttempts,
    phase' = 2,
  }

  action replayByCursor(cursor) = all {
    cursor.in(CURSORS),
    gSubOpsAllStatus' = gSubOpsAllStatus,
    gSubElicitationsOpsStatus' = gSubElicitationsOpsStatus,
    subscriptionId' = subscriptionId,
    subscriptionGrantId' = subscriptionGrantId,
    subscriptionCursor' = cursor,
    subscriptionStream' = subscriptionStream,
    subscriptionFilter' = subscriptionFilter,
    auditRecords' = auditRecords,
    operationRecordsCreated' = operationRecordsCreated,
    eventLsn' = eventLsn,
    eventStream' = eventStream,
    eventFilter' = eventFilter,
    replayedEvents' = replayEventFor(cursor),
    LastSubscriptionActor' = LastSubscriptionActor,
    LastSubscriptionScope' = LastSubscriptionScope,
    SubscriptionAttempted' = SubscriptionAttempted,
    SubscriptionAccepted' = SubscriptionAccepted,
    SubscriptionEstablishAttempts' = SubscriptionEstablishAttempts,
    phase' = 2,
  }

  action step = any {
    nondet actor = ACTORS.oneOf(); nondet stream = STREAMS.oneOf(); nondet filt = FILTERS.oneOf(); nondet cursor = CURSORS.oneOf(); all { phase == 0, attemptEstablish(actor, stream, filt, cursor) },
    nondet eventStreamCandidate = STREAMS.oneOf(); nondet eventFilterCandidate = FILTERS.oneOf(); all { phase == 1, emitEvent(eventStreamCandidate, eventFilterCandidate) },
    nondet replayCursor = CURSORS.oneOf(); all { phase == 2, replayByCursor(replayCursor) },
  }

  // @promotion {
  //   property:    SubscriptionGrantChecked
  //   status:      draft
  //   model:       specs/seed/subscription_authority.qnt
  //   language:    quint
  //   backend:     apalache
  //   invocation:  <TBD — demoted; formula does not independently establish the claimed behavior; v1 formal gate owns the real property>
  //   bounds:      { actors: 3, streams: 2, filters: 2, grant_statuses: 2, cursors: 5, max_steps: 12 }
  //   expected:    pass
  //   proto_fields: [none]
  //   demotion_reason: invariant inspects LastSubscriptionActor and LastSubscriptionScope written by the establishing action, so it proves recorded actor/scope consistency rather than independently checking submitted subscription evidence; accepting arbitrary input while recording the grant subject and scope can pass
  //   semantics:   subscription establishment succeeds only with a live subscribe-kind Grant record whose subject matches the submitting actor and stream/filter scope
  // }

  // @promotion {
  //   property:    SubscriptionAudited
  //   status:      draft
  //   model:       specs/seed/subscription_authority.qnt
  //   language:    quint
  //   backend:     apalache
  //   invocation:  <TBD — demoted; formula inspects state recorded by the accepting action, not independent attempted evidence; not a mutation-survivable oracle; v1 formal gate owns the real property>
  //   demotion_reason: invariant compares `auditRecords == SubscriptionEstablishAttempts`, but the establishment action writes both counters. A mutation setting `auditRecords' = SubscriptionEstablishAttempts + 1` can lie about the audit and still pass, so this is not an independent audit oracle.
  //   bounds:      { actors: 3, streams: 2, filters: 2, grant_statuses: 2, cursors: 5, max_steps: 12 }
  //   expected:    pass
  //   proto_fields: [none]
  //   semantics:   subscription allow/deny decisions create audit records without creating OperationState records
  // }
  // @promotion {
  //   property:    SubscriptionCursorReplayAuthorized
  //   status:      draft
  //   model:       specs/seed/subscription_authority.qnt
  //   language:    quint
  //   backend:     apalache
  //   invocation:  <TBD — demoted; formula inspects state recorded by the accepting action, not independent attempted evidence; not a mutation-survivable oracle; v1 formal gate owns the real property>
  //   demotion_reason: invariant calls `replayedEventIndependentOk`, which reads grant, scope, stream, and filter state recorded by the accepting actions. Forcing the helper to `true` passes, so replay authorization is not checked against independent attempted evidence.
  //   bounds:      { actors: 3, streams: 2, filters: 2, grant_statuses: 2, cursors: 5, max_steps: 12 }
  //   expected:    pass
  //   proto_fields: [none]
  //   semantics:   cursor replay returns only events with LSN greater than the requested cursor and inside the authorized subscription stream/filter
  // }
}
