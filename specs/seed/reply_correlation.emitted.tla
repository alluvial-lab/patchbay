// GENERATED ARTIFACT — do not hand-edit. Regenerate via: quint compile reply_correlation.qnt --target tlaplus
// Source: reply_correlation.qnt. Inspection artifact, NOT an independent re-check lane (see feature-formal-model-seed Q4).

MODULE reply_correlation ---------------------------

EXTENDS Integers, Sequences, FiniteSets, TLC, Apalache, Variants

(*
  @type: (() => Set(Str));
*)
REPLY_ID_SPACE == { "r1", "r2" }

(*
  @type: (() => Set(Str));
*)
CORRELATION_TYPE_ATTEMPTS == { "command", "message", "reply", "event" }

(*
  @type: (() => (Str -> Str));
*)
commandContext == SetAsFun({ <<"c1", "ctxA">>, <<"c2", "ctxB">> })

(*
  @type: (() => Set(Str));
*)
COMMAND_ID_SPACE == { "c1", "c2" }

(*
  @type: (() => (Str -> Str));
*)
messageContext == SetAsFun({ <<"m1", "ctxA">>, <<"m2", "ctxB">> })

(*
  @type: (() => (Str -> Str));
*)
replyContext == SetAsFun({ <<"r1", "ctxA">>, <<"r2", "ctxB">> })

VARIABLE
  (*
    @type: Set(Str);
  *)
  commandIds

VARIABLE
  (*
    @type: Set(Str);
  *)
  messageIds

VARIABLE
  (*
    @type: Set(Str);
  *)
  replyIds

VARIABLE
  (*
    @type: (Str -> Str);
  *)
  replyCorrelatesTo

VARIABLE
  (*
    @type: (Str -> Str);
  *)
  replyCorrelationType

(*
  @type: (() => Set(Str));
*)
MESSAGE_ID_SPACE == { "m1", "m2" }

(*
  @type: ((Str, Str) => Bool);
*)
messageCorrelationOk(replyId_99, corrId_99) ==
  IF corrId_99 \in messageIds
  THEN (replyContext)[replyId_99] = (messageContext)[corrId_99]
  ELSE FALSE

(*
  @type: (() => Bool);
*)
init ==
  commandIds = COMMAND_ID_SPACE
    /\ messageIds = MESSAGE_ID_SPACE
    /\ replyIds = {}
    /\ replyCorrelatesTo = [ r_188 \in REPLY_ID_SPACE |-> "none" ]
    /\ replyCorrelationType = [ r_195 \in REPLY_ID_SPACE |-> "none" ]

(*
  @type: (() => Set(Str));
*)
ALL_ID_ATTEMPTS ==
  ((COMMAND_ID_SPACE \union MESSAGE_ID_SPACE) \union REPLY_ID_SPACE)
    \union {"x"}

(*
  @type: ((Str, Str) => Bool);
*)
commandCorrelationOk(replyId_83, corrId_83) ==
  IF corrId_83 \in commandIds
  THEN (replyContext)[replyId_83] = (commandContext)[corrId_83]
  ELSE FALSE

(*
  @type: ((Str, Str, Str) => Bool);
*)
typedReferenceOk(replyId_119, corrId_119, corrType_119) ==
  IF corrType_119 = "command"
  THEN commandCorrelationOk(replyId_119, corrId_119)
  ELSE IF corrType_119 = "message"
  THEN messageCorrelationOk(replyId_119, corrId_119)
  ELSE FALSE

(*
  @type: (() => Bool);
*)
q_init == init

(*
  @type: ((Str, Str, Str) => Bool);
*)
replyRecordable(replyId_138, corrId_138, corrType_138) ==
  IF replyId_138 \in REPLY_ID_SPACE /\ ~(replyId_138 \in replyIds)
  THEN typedReferenceOk(replyId_138, corrId_138, corrType_138)
  ELSE FALSE

(*
  @type: ((Str) => Bool);
*)
recordedReplyOk(replyId_174) ==
  IF (replyId_174 \in REPLY_ID_SPACE /\ replyId_174 \in DOMAIN replyCorrelatesTo)
    /\ replyId_174 \in DOMAIN replyCorrelationType
  THEN (~(replyId_174 \in commandIds) /\ ~(replyId_174 \in messageIds))
    /\ typedReferenceOk(replyId_174, replyCorrelatesTo[replyId_174], replyCorrelationType[
      replyId_174
    ])
  ELSE FALSE

(*
  @type: ((Str, Str, Str) => Bool);
*)
createReply(replyId_255, corrId_255, corrType_255) ==
  (replyRecordable(replyId_255, corrId_255, corrType_255)
      /\ commandIds' := commandIds
      /\ messageIds' := messageIds
      /\ replyIds' := (replyIds \union {replyId_255})
      /\ replyCorrelatesTo'
        := [ replyCorrelatesTo EXCEPT ![replyId_255] = corrId_255 ]
      /\ replyCorrelationType'
        := [ replyCorrelationType EXCEPT ![replyId_255] = corrType_255 ])
    \/ (~(replyRecordable(replyId_255, corrId_255, corrType_255))
      /\ commandIds' := commandIds
      /\ messageIds' := messageIds
      /\ replyIds' := replyIds
      /\ replyCorrelatesTo' := replyCorrelatesTo
      /\ replyCorrelationType' := replyCorrelationType)

(*
  @type: (() => Bool);
*)
typed_correlation ==
  (((((commandIds \subseteq COMMAND_ID_SPACE
              /\ messageIds \subseteq MESSAGE_ID_SPACE)
            /\ replyIds \subseteq REPLY_ID_SPACE)
          /\ Cardinality((commandIds \intersect messageIds)) = 0)
        /\ Cardinality((commandIds \intersect replyIds)) = 0)
      /\ Cardinality((messageIds \intersect replyIds)) = 0)
    /\ (\A r_310 \in replyIds: recordedReplyOk(r_310))

(*
  @type: (() => Bool);
*)
step ==
  \E replyId \in ALL_ID_ATTEMPTS:
    \E corrId \in ALL_ID_ATTEMPTS:
      \E corrType \in CORRELATION_TYPE_ATTEMPTS:
        createReply(replyId, corrId, corrType)

(*
  @type: (() => Bool);
*)
q_step == step

================================================================================
