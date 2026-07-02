// Patchbay seed model: typed reply correlation across separate id spaces
//
// This model covers the checked-normative TypedCorrelation property from
// docs/PROTOCOL.md "Messages, commands, and replies". It models command ids,
// message ids, reply ids, and the event id space (implicitly: event id = LSN,
// never accepted as a reply correlation target) as separate spaces.
//
// Genuine-checking discipline (load-bearing): createReply(replyId, corrId,
// corrType) is PERMISSIVE in its inputs. The driver attempts every bounded id as
// a reply id, every bounded id as a correlation target, and valid plus invalid
// correlation types. The model records a reply only when the typed reference
// resolves to an existing prior command/message in the same authority/session
// context; invalid attempts are explicit no-ops. The invariant checks all
// recorded replies, so a transition that let an invalid reply into state would be
// found as a counterexample.
module reply_correlation {
  // --- finite id spaces (event id is the implicit LSN space, never a corr target) ---
  pure val COMMAND_ID_SPACE = Set("c1", "c2")
  pure val MESSAGE_ID_SPACE = Set("m1", "m2")
  pure val REPLY_ID_SPACE = Set("r1", "r2")
  pure val ALL_ID_ATTEMPTS = COMMAND_ID_SPACE.union(MESSAGE_ID_SPACE).union(REPLY_ID_SPACE).union(Set("x"))

  // Valid and adversarial attempted correlation-type tags. "event" exercises the
  // fourth id space: event ids exist as LSNs, but replies never correlate to them.
  pure val CORRELATION_TYPE_ATTEMPTS = Set("command", "message", "reply", "event")

  // Authority/session context is fixed metadata for bounded candidate ids. This
  // keeps the state small while still checking that correlation cannot cross
  // contexts. c1/m1/r1 are in ctxA; c2/m2/r2 are in ctxB.
  pure val commandContext = Map("c1" -> "ctxA", "c2" -> "ctxB")
  pure val messageContext = Map("m1" -> "ctxA", "m2" -> "ctxB")
  pure val replyContext = Map("r1" -> "ctxA", "r2" -> "ctxB")

  // --- state ---
  var commandIds: Set[str]                 // known prior command ids
  var messageIds: Set[str]                 // known prior message ids
  var replyIds: Set[str]                   // recorded reply ids
  var replyCorrelatesTo: str -> str        // ReplyId -> correlated command/message id
  var replyCorrelationType: str -> str     // ReplyId -> "command" | "message"

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

  // INDEPENDENT property oracle: checks raw id-space/context facts WITHOUT calling
  // typedReferenceOk (the helper the action's replyRecordable filter uses). This is the
  // genuine-checking discipline from the design: the invariant must not re-use the
  // predicate that gates the action, or it can never catch a broken predicate.
  // Mutation test: if typedReferenceOk is broken to `true`, this oracle still asserts the
  // raw facts (corrId is a known prior command/message, types match, contexts match, id
  // spaces disjoint) and a real forgery would fail it.
  def recordedReplyIndependentOk(replyId) =
    REPLY_ID_SPACE.contains(replyId)
      and replyCorrelatesTo.keys().contains(replyId)
      and replyCorrelationType.keys().contains(replyId)
      and not(commandIds.contains(replyId))
      and not(messageIds.contains(replyId))
      and (
        (replyCorrelationType.get(replyId) == "command"
          and commandIds.contains(replyCorrelatesTo.get(replyId))
          and replyContext.get(replyId) == commandContext.get(replyCorrelatesTo.get(replyId)))
        or (replyCorrelationType.get(replyId) == "message"
          and messageIds.contains(replyCorrelatesTo.get(replyId))
          and replyContext.get(replyId) == messageContext.get(replyCorrelatesTo.get(replyId)))
      )

  // ---------------------------------------------------------------------------
  // Actions
  // ---------------------------------------------------------------------------

  // The bounded prior command/message ids have already been accepted before any
  // reply is attempted. That keeps this seed model focused on reply validation:
  // "prior" is represented by membership in these known-id sets at init.
  action init = all {
    commandIds' = COMMAND_ID_SPACE,
    messageIds' = MESSAGE_ID_SPACE,
    replyIds' = Set(),
    replyCorrelatesTo' = REPLY_ID_SPACE.mapBy(r => "none"),
    replyCorrelationType' = REPLY_ID_SPACE.mapBy(r => "none"),
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
      replyIds' = replyIds.union(Set(replyId)),
      replyCorrelatesTo' = replyCorrelatesTo.set(replyId, corrId),
      replyCorrelationType' = replyCorrelationType.set(replyId, corrType),
    },
    all {
      not(replyRecordable(replyId, corrId, corrType)),
      commandIds' = commandIds,
      messageIds' = messageIds,
      replyIds' = replyIds,
      replyCorrelatesTo' = replyCorrelatesTo,
      replyCorrelationType' = replyCorrelationType,
    },
  }

  // The driver exercises adversarial reply creation attempts, including wrong
  // type, unknown id, reply-id-as-correlation, event-id-as-correlation-type,
  // duplicate reply id, and wrong-context cases.
  action step = {
    nondet replyId = ALL_ID_ATTEMPTS.oneOf(); nondet corrId = ALL_ID_ATTEMPTS.oneOf(); nondet corrType = CORRELATION_TYPE_ATTEMPTS.oneOf(); createReply(replyId, corrId, corrType)
  }

  // ---------------------------------------------------------------------------
  // Checked property
  // ---------------------------------------------------------------------------

  // @promotion {
  //   property:    TypedCorrelation
  //   tier:        checked-normative
  //   status:      promoted
  //   model:       specs/seed/reply_correlation.qnt
  //   language:    quint
  //   backend:     apalache
  //   invocation:  quint verify reply_correlation.qnt --invariant typed_correlation --max-steps 12
  //   bounds:      { command_ids: 2, message_ids: 2, reply_ids: 2, max_steps: 12 }
  //   expected:    pass
  //   proto_fields: [none]
  //   semantics:   replies use typed same-context references to known prior commands/messages and cannot masquerade across id spaces
  // }
  val typed_correlation =
    commandIds.subseteq(COMMAND_ID_SPACE)
      and messageIds.subseteq(MESSAGE_ID_SPACE)
      and replyIds.subseteq(REPLY_ID_SPACE)
      and commandIds.intersect(messageIds).size() == 0
      and commandIds.intersect(replyIds).size() == 0
      and messageIds.intersect(replyIds).size() == 0
      and replyIds.forall(r => recordedReplyIndependentOk(r))
}
