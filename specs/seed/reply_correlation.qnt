// Patchbay seed model: typed reply correlation across separate id spaces
//
// This model reserves the stated-normative TypedCorrelation obligation from
// docs/PROTOCOL.md "Messages, commands, and replies". It models command ids,
// message ids, reply ids, event ids (LSN/event references are never accepted as
// typed correlation targets), Elicitation ids, and response-Operation ids as
// separate bounded spaces.
//
// Genuine-checking requirement (load-bearing): createReply(replyId, corrId,
// corrType) and createResponseOperation(responseOpId, opKind, corrId, corrType)
// are PERMISSIVE in their inputs. The driver attempts every bounded id as a
// reply/response Operation id, every bounded id as a correlation target, and
// valid plus invalid kind/type tags. The current TypedCorrelation property is
// draft because recorded-state helpers are not independent of actions that can
// accept arbitrary input while writing canonical evidence. A replacement formula
// must inspect attempted correlation evidence those actions cannot rewrite.
module reply_correlation {
  // --- finite id spaces (event id is the explicit LSN/event space, never a corr target) ---
  pure val COMMAND_ID_SPACE = Set("c1", "c2")
  pure val MESSAGE_ID_SPACE = Set("m1", "m2")
  pure val REPLY_ID_SPACE = Set("r1", "r2")
  pure val EVENT_ID_SPACE = Set("ev1", "ev2")
  pure val ELICITATION_ID_SPACE = Set("e1", "e2")
  pure val RESPONSE_OP_ID_SPACE = Set("ro1", "ro2")
  pure val ALL_ID_ATTEMPTS = COMMAND_ID_SPACE.union(MESSAGE_ID_SPACE).union(REPLY_ID_SPACE).union(EVENT_ID_SPACE).union(ELICITATION_ID_SPACE).union(RESPONSE_OP_ID_SPACE).union(Set("x"))

  // Valid and adversarial attempted correlation-type tags. "event" exercises the
  // event id space: event ids exist, but replies/response Operations never
  // correlate to them. Response Operations use the "elicitation" typed reference.
  pure val CORRELATION_TYPE_ATTEMPTS = Set("command", "message", "reply", "event", "elicitation")
  pure val RESPONSE_OPERATION_KINDS = Set("approval-response", "elicitation-response")
  pure val RESPONSE_OPERATION_KIND_ATTEMPTS = RESPONSE_OPERATION_KINDS.union(Set("spawn"))

  // Authority/session/responder context is fixed metadata for bounded candidate ids.
  // The compact `ctx*` atom represents authority-domain + target-session/generation;
  // responder maps represent the expected responder actor. c1/m1/r1/e1/ro1 are in
  // ctxA; c2/m2/r2/e2/ro2 are in ctxB.
  pure val commandContext = Map("c1" -> "ctxA", "c2" -> "ctxB")
  pure val messageContext = Map("m1" -> "ctxA", "m2" -> "ctxB")
  pure val replyContext = Map("r1" -> "ctxA", "r2" -> "ctxB")
  pure val elicitationContext = Map("e1" -> "ctxA", "e2" -> "ctxB")
  pure val responseOperationContext = Map("ro1" -> "ctxA", "ro2" -> "ctxB")
  pure val elicitationResponder = Map("e1" -> "alice", "e2" -> "alice")
  pure val responseOperationResponder = Map("ro1" -> "alice", "ro2" -> "alice")

  // --- state ---
  var commandIds: Set[str]                 // known prior command ids
  var messageIds: Set[str]                 // known prior message ids
  var elicitationIds: Set[str]             // known prior Elicitation ids
  var replyIds: Set[str]                   // recorded reply ids
  var replyCorrelatesTo: str -> str        // ReplyId -> correlated command/message id
  var replyCorrelationType: str -> str     // ReplyId -> "command" | "message"
  var responseOperationIds: Set[str]       // recorded response Operation ids
  var responseOperationKind: str -> str    // ResponseOperationId -> approval-response | elicitation-response
  var responseCorrelatesTo: str -> str     // ResponseOperationId -> correlated ElicitationId
  var responseCorrelationType: str -> str  // ResponseOperationId -> "elicitation"

  // ---------------------------------------------------------------------------
  // Helpers
  // ---------------------------------------------------------------------------

  def commandCorrelationOk(replyId, corrId) =
    if (commandIds.contains(corrId))
      replyContext.get(replyId) == commandContext.get(corrId)
    else
      false

  def messageCorrelationOk(replyId, corrId) =
    if (messageIds.contains(corrId))
      replyContext.get(replyId) == messageContext.get(corrId)
    else
      false

  def typedReferenceOk(replyId, corrId, corrType) =
    if (corrType == "command")
      commandCorrelationOk(replyId, corrId)
    else if (corrType == "message")
      messageCorrelationOk(replyId, corrId)
    else
      false

  def replyRecordable(replyId, corrId, corrType) =
    if (REPLY_ID_SPACE.contains(replyId) and not(replyIds.contains(replyId)))
      typedReferenceOk(replyId, corrId, corrType)
    else
      false

  def elicitationCorrelationOk(responseOpId, corrId) =
    if (elicitationIds.contains(corrId))
      responseOperationContext.get(responseOpId) == elicitationContext.get(corrId)
        and responseOperationResponder.get(responseOpId) == elicitationResponder.get(corrId)
    else
      false

  def typedResponseReferenceOk(responseOpId, corrId, corrType) =
    if (corrType == "elicitation")
      elicitationCorrelationOk(responseOpId, corrId)
    else
      false

  def responseOperationRecordable(responseOpId, opKind, corrId, corrType) =
    if (RESPONSE_OP_ID_SPACE.contains(responseOpId) and not(responseOperationIds.contains(responseOpId)) and opKind.in(RESPONSE_OPERATION_KINDS))
      typedResponseReferenceOk(responseOpId, corrId, corrType)
    else
      false

  // Recorded-reply consistency helper: checks id-space/context facts without calling
  // typedReferenceOk (the helper the action's replyRecordable filter uses). This catches
  // some isolated guard mutations, but it is not an independent attempted-evidence oracle:
  // createReply can accept arbitrary input while recording canonical evidence.
  def recordedReplyIndependentOk(replyId) =
    REPLY_ID_SPACE.contains(replyId)
      and replyCorrelatesTo.keys().contains(replyId)
      and replyCorrelationType.keys().contains(replyId)
      and not(commandIds.contains(replyId))
      and not(messageIds.contains(replyId))
      and not(elicitationIds.contains(replyId))
      and not(EVENT_ID_SPACE.contains(replyId))
      and (
        (replyCorrelationType.get(replyId) == "command"
          and commandIds.contains(replyCorrelatesTo.get(replyId))
          and replyContext.get(replyId) == commandContext.get(replyCorrelatesTo.get(replyId)))
        or (replyCorrelationType.get(replyId) == "message"
          and messageIds.contains(replyCorrelatesTo.get(replyId))
          and replyContext.get(replyId) == messageContext.get(replyCorrelatesTo.get(replyId)))
      )

  // Recorded-response consistency helper: checks response Operation facts without
  // calling typedResponseReferenceOk, elicitationCorrelationOk, or
  // responseOperationRecordable. It is independent of those guard helpers but not
  // of createResponseOperation's writes; a coordinated mutation can record canonical
  // evidence regardless of attempted input.
  def recordedResponseOpIndependentOk(responseOpId) =
    RESPONSE_OP_ID_SPACE.contains(responseOpId)
      and responseCorrelatesTo.keys().contains(responseOpId)
      and responseCorrelationType.keys().contains(responseOpId)
      and responseOperationKind.keys().contains(responseOpId)
      and responseOperationKind.get(responseOpId).in(RESPONSE_OPERATION_KINDS)
      and responseCorrelationType.get(responseOpId) == "elicitation"
      and elicitationIds.contains(responseCorrelatesTo.get(responseOpId))
      and not(commandIds.contains(responseCorrelatesTo.get(responseOpId)))
      and not(messageIds.contains(responseCorrelatesTo.get(responseOpId)))
      and not(replyIds.contains(responseCorrelatesTo.get(responseOpId)))
      and not(EVENT_ID_SPACE.contains(responseCorrelatesTo.get(responseOpId)))
      and responseOperationContext.get(responseOpId) == elicitationContext.get(responseCorrelatesTo.get(responseOpId))
      and responseOperationResponder.get(responseOpId) == elicitationResponder.get(responseCorrelatesTo.get(responseOpId))

  // ---------------------------------------------------------------------------
  // Actions
  // ---------------------------------------------------------------------------

  // The bounded prior command/message/Elicitation ids have already been accepted
  // before any reply or response Operation is attempted. That keeps this seed
  // model focused on typed-correlation validation: "prior" is represented by
  // membership in these known-id sets at init.
  action init = all {
    commandIds' = COMMAND_ID_SPACE,
    messageIds' = MESSAGE_ID_SPACE,
    elicitationIds' = ELICITATION_ID_SPACE,
    replyIds' = Set(),
    replyCorrelatesTo' = REPLY_ID_SPACE.mapBy(r => "none"),
    replyCorrelationType' = REPLY_ID_SPACE.mapBy(r => "none"),
    responseOperationIds' = Set(),
    responseOperationKind' = RESPONSE_OP_ID_SPACE.mapBy(ro => "none"),
    responseCorrelatesTo' = RESPONSE_OP_ID_SPACE.mapBy(ro => "none"),
    responseCorrelationType' = RESPONSE_OP_ID_SPACE.mapBy(ro => "none"),
  }

  // PERMISSIVE attempted reply creation. The attempted reply id, correlation id,
  // and correlation type may be anything in the bounded adversarial domains. Only
  // typed, same-context command/message correlations are recorded; all other
  // attempts leave no reply record.
  action createReply(replyId, corrId, corrType) = any {
    all {
      replyRecordable(replyId, corrId, corrType),
      commandIds' = commandIds,
      messageIds' = messageIds,
      elicitationIds' = elicitationIds,
      replyIds' = replyIds.union(Set(replyId)),
      replyCorrelatesTo' = replyCorrelatesTo.set(replyId, corrId),
      replyCorrelationType' = replyCorrelationType.set(replyId, corrType),
      responseOperationIds' = responseOperationIds,
      responseOperationKind' = responseOperationKind,
      responseCorrelatesTo' = responseCorrelatesTo,
      responseCorrelationType' = responseCorrelationType,
    },
    all {
      not(replyRecordable(replyId, corrId, corrType)),
      commandIds' = commandIds,
      messageIds' = messageIds,
      elicitationIds' = elicitationIds,
      replyIds' = replyIds,
      replyCorrelatesTo' = replyCorrelatesTo,
      replyCorrelationType' = replyCorrelationType,
      responseOperationIds' = responseOperationIds,
      responseOperationKind' = responseOperationKind,
      responseCorrelatesTo' = responseCorrelatesTo,
      responseCorrelationType' = responseCorrelationType,
    },
  }

  // PERMISSIVE attempted response Operation creation. The attempted response id,
  // Operation kind, correlation id, and correlation type may be anything in the
  // bounded adversarial domains. Only response Operations with a typed, same
  // authority/session/responder-context Elicitation correlation are recorded; all
  // other attempts leave no response Operation record. This action also writes the
  // evidence consumed by the recorded-state helper, which is why TypedCorrelation is draft.
  action createResponseOperation(responseOpId, opKind, corrId, corrType) = any {
    all {
      responseOperationRecordable(responseOpId, opKind, corrId, corrType),
      commandIds' = commandIds,
      messageIds' = messageIds,
      elicitationIds' = elicitationIds,
      replyIds' = replyIds,
      replyCorrelatesTo' = replyCorrelatesTo,
      replyCorrelationType' = replyCorrelationType,
      responseOperationIds' = responseOperationIds.union(Set(responseOpId)),
      responseOperationKind' = responseOperationKind.set(responseOpId, opKind),
      responseCorrelatesTo' = responseCorrelatesTo.set(responseOpId, corrId),
      responseCorrelationType' = responseCorrelationType.set(responseOpId, corrType),
    },
    all {
      not(responseOperationRecordable(responseOpId, opKind, corrId, corrType)),
      commandIds' = commandIds,
      messageIds' = messageIds,
      elicitationIds' = elicitationIds,
      replyIds' = replyIds,
      replyCorrelatesTo' = replyCorrelatesTo,
      replyCorrelationType' = replyCorrelationType,
      responseOperationIds' = responseOperationIds,
      responseOperationKind' = responseOperationKind,
      responseCorrelatesTo' = responseCorrelatesTo,
      responseCorrelationType' = responseCorrelationType,
    },
  }

  // The driver exercises adversarial reply and response Operation attempts,
  // including wrong type, unknown id, wrong id-space-as-correlation, duplicate
  // record id, and wrong-context cases.
  action attemptReply = {
    nondet replyId = ALL_ID_ATTEMPTS.oneOf(); nondet corrId = ALL_ID_ATTEMPTS.oneOf(); nondet corrType = CORRELATION_TYPE_ATTEMPTS.oneOf(); createReply(replyId, corrId, corrType)
  }

  action attemptResponseOperation = {
    nondet responseOpId = ALL_ID_ATTEMPTS.oneOf(); nondet opKind = RESPONSE_OPERATION_KIND_ATTEMPTS.oneOf(); nondet corrId = ALL_ID_ATTEMPTS.oneOf(); nondet corrType = CORRELATION_TYPE_ATTEMPTS.oneOf(); createResponseOperation(responseOpId, opKind, corrId, corrType)
  }

  action step = any {
    attemptReply,
    attemptResponseOperation,
  }

  // ---------------------------------------------------------------------------
  // Draft model obligation
  // ---------------------------------------------------------------------------

  // @promotion {
  //   property:    TypedCorrelation
  //   status:      draft
  //   model:       specs/seed/reply_correlation.qnt
  //   language:    quint
  //   backend:     apalache
  //   invocation:  <TBD — demoted; formula inspects state recorded by the accepting action, not independent attempted evidence; not a mutation-survivable oracle; v1 formal gate owns the real property>
  //   demotion_reason: `createResponseOperation` records correlation evidence in the same action that decides recordability. A mutation accepting arbitrary input while recording canonical correlation data passes, so the formula cannot independently detect forged attempted correlation evidence.
  //   bounds:      { command_ids: 2, message_ids: 2, reply_ids: 2, event_ids: 2, elicitation_ids: 2, response_op_ids: 2, max_steps: 12 }
  //   expected:    pass
  //   proto_fields: [none]
  //   semantics:   replies use typed same-context references to known prior commands/messages, response Operations use typed same authority/session/responder-context references to known prior Elicitations, and neither can masquerade across CommandId/MessageId/ReplyId/EventId/ElicitationId spaces
  // }
}
