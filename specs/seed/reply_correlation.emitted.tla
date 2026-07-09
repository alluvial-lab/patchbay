// GENERATED ARTIFACT — do not hand-edit. Regenerate via: quint compile reply_correlation.qnt --target tlaplus
// Source: reply_correlation.qnt. Inspection artifact, NOT an independent re-check lane (see feature-formal-model-seed Q4).

# Usage statistics is OFF. We care about your privacy.
# If you want to help our project, consider enabling statistics with config --enable-stats=true.

Output directory: /home/agent/projects/patchbay/specs/seed/_apalache-out/server/2026-07-08T19-16-10_5075870231213335309
# APALACHE version: 0.56.1 | build: 70cdaf4                       I@19:16:10.797
Starting checker server on port 8822...                           I@19:16:10.807
The Apalache server is running on port 8822. Press Ctrl-C to stop.
PASS #0: SanyParser                                               I@19:16:13.607
--------------------------- MODULE reply_correlation ---------------------------

EXTENDS Integers, Sequences, FiniteSets, TLC, Apalache, Variants

(*
  @type: (() => (Str -> Str));
*)
elicitationResponder == SetAsFun({ <<"e1", "alice">>, <<"e2", "alice">> })

(*
  @type: (() => (Str -> Str));
*)
responseOperationResponder ==
  SetAsFun({ <<"ro1", "alice">>, <<"ro2", "alice">> })

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

(*
  @type: (() => Set(Str));
*)
REPLY_ID_SPACE == { "r1", "r2" }

VARIABLE
  (*
    @type: Set(Str);
  *)
  elicitationIds

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

VARIABLE
  (*
    @type: Set(Str);
  *)
  responseOperationIds

VARIABLE
  (*
    @type: (Str -> Str);
  *)
  responseOperationKind

VARIABLE
  (*
    @type: (Str -> Str);
  *)
  responseCorrelatesTo

VARIABLE
  (*
    @type: (Str -> Str);
  *)
  responseCorrelationType

(*
  @type: (() => Set(Str));
*)
EVENT_ID_SPACE == { "ev1", "ev2" }

(*
  @type: (() => Set(Str));
*)
ELICITATION_ID_SPACE == { "e1", "e2" }

(*
  @type: (() => Set(Str));
*)
RESPONSE_OP_ID_SPACE == { "ro1", "ro2" }

(*
  @type: (() => Set(Str));
*)
COMMAND_ID_SPACE == { "c1", "c2" }

(*
  @type: (() => Set(Str));
*)
CORRELATION_TYPE_ATTEMPTS ==
  { "command", "message", "reply", "event", "elicitation" }

(*
  @type: (() => Set(Str));
*)
RESPONSE_OPERATION_KINDS == { "approval-response", "elicitation-response" }

(*
  @type: (() => (Str -> Str));
*)
commandContext == SetAsFun({ <<"c1", "ctxA">>, <<"c2", "ctxB">> })

(*
  @type: (() => (Str -> Str));
*)
messageContext == SetAsFun({ <<"m1", "ctxA">>, <<"m2", "ctxB">> })

(*
  @type: (() => (Str -> Str));
*)
replyContext == SetAsFun({ <<"r1", "ctxA">>, <<"r2", "ctxB">> })

(*
  @type: (() => Set(Str));
*)
MESSAGE_ID_SPACE == { "m1", "m2" }

(*
  @type: (() => (Str -> Str));
*)
elicitationContext == SetAsFun({ <<"e1", "ctxA">>, <<"e2", "ctxB">> })

(*
  @type: (() => (Str -> Str));
*)
responseOperationContext == SetAsFun({ <<"ro1", "ctxA">>, <<"ro2", "ctxB">> })

(*
  @type: ((Str, Str) => Bool);
*)
commandCorrelationOk(replyId_161, corrId_161) ==
  IF corrId_161 \in commandIds
  THEN (replyContext)[replyId_161] = (commandContext)[corrId_161]
  ELSE FALSE

(*
  @type: ((Str, Str) => Bool);
*)
messageCorrelationOk(replyId_177, corrId_177) ==
  IF corrId_177 \in messageIds
  THEN (replyContext)[replyId_177] = (messageContext)[corrId_177]
  ELSE FALSE

(*
  @type: ((Str, Str) => Bool);
*)
elicitationCorrelationOk(responseOpId_240, corrId_240) ==
  IF corrId_240 \in elicitationIds
  THEN (responseOperationContext)[responseOpId_240]
      = (elicitationContext)[corrId_240]
    /\ (responseOperationResponder)[responseOpId_240]
      = (elicitationResponder)[corrId_240]
  ELSE FALSE

(*
  @type: ((Str) => Bool);
*)
recordedReplyIndependentOk(replyId_357) ==
  ((((((replyId_357 \in REPLY_ID_SPACE
                /\ replyId_357 \in DOMAIN replyCorrelatesTo)
              /\ replyId_357 \in DOMAIN replyCorrelationType)
            /\ ~(replyId_357 \in commandIds))
          /\ ~(replyId_357 \in messageIds))
        /\ ~(replyId_357 \in elicitationIds))
      /\ ~(replyId_357 \in EVENT_ID_SPACE))
    /\ (((replyCorrelationType[replyId_357] = "command"
          /\ replyCorrelatesTo[replyId_357] \in commandIds)
        /\ (replyContext)[replyId_357]
          = (commandContext)[replyCorrelatesTo[replyId_357]])
      \/ ((replyCorrelationType[replyId_357] = "message"
          /\ replyCorrelatesTo[replyId_357] \in messageIds)
        /\ (replyContext)[replyId_357]
          = (messageContext)[replyCorrelatesTo[replyId_357]]))

(*
  @type: (() => Set(Str));
*)
ALL_ID_ATTEMPTS ==
  (((((COMMAND_ID_SPACE \union MESSAGE_ID_SPACE) \union REPLY_ID_SPACE)
    \union EVENT_ID_SPACE)
    \union ELICITATION_ID_SPACE)
    \union RESPONSE_OP_ID_SPACE)
    \union {"x"}

(*
  @type: ((Str) => Bool);
*)
recordedResponseOpIndependentOk(responseOpId_444) ==
  (((((((((((responseOpId_444 \in RESPONSE_OP_ID_SPACE
                          /\ responseOpId_444 \in DOMAIN responseCorrelatesTo)
                        /\ responseOpId_444 \in DOMAIN responseCorrelationType)
                      /\ responseOpId_444 \in DOMAIN responseOperationKind)
                    /\ responseOperationKind[responseOpId_444]
                      \in RESPONSE_OPERATION_KINDS)
                  /\ responseCorrelationType[responseOpId_444] = "elicitation")
                /\ responseCorrelatesTo[responseOpId_444] \in elicitationIds)
              /\ ~(responseCorrelatesTo[responseOpId_444] \in commandIds))
            /\ ~(responseCorrelatesTo[responseOpId_444] \in messageIds))
          /\ ~(responseCorrelatesTo[responseOpId_444] \in replyIds))
        /\ ~(responseCorrelatesTo[responseOpId_444] \in EVENT_ID_SPACE))
      /\ (responseOperationContext)[responseOpId_444]
        = (elicitationContext)[responseCorrelatesTo[responseOpId_444]])
    /\ (responseOperationResponder)[responseOpId_444]
      = (elicitationResponder)[responseCorrelatesTo[responseOpId_444]]

(*
  @type: (() => Bool);
*)
init ==
  commandIds = COMMAND_ID_SPACE
    /\ messageIds = MESSAGE_ID_SPACE
    /\ elicitationIds = ELICITATION_ID_SPACE
    /\ replyIds = {}
    /\ replyCorrelatesTo = [ r_461 \in REPLY_ID_SPACE |-> "none" ]
    /\ replyCorrelationType = [ r_468 \in REPLY_ID_SPACE |-> "none" ]
    /\ responseOperationIds = {}
    /\ responseOperationKind = [ ro_478 \in RESPONSE_OP_ID_SPACE |-> "none" ]
    /\ responseCorrelatesTo = [ ro_485 \in RESPONSE_OP_ID_SPACE |-> "none" ]
    /\ responseCorrelationType = [ ro_492 \in RESPONSE_OP_ID_SPACE |-> "none" ]

(*
  @type: (() => Set(Str));
*)
RESPONSE_OPERATION_KIND_ATTEMPTS == RESPONSE_OPERATION_KINDS \union {"spawn"}

(*
  @type: ((Str, Str, Str) => Bool);
*)
typedReferenceOk(replyId_197, corrId_197, corrType_197) ==
  IF corrType_197 = "command"
  THEN commandCorrelationOk(replyId_197, corrId_197)
  ELSE IF corrType_197 = "message"
  THEN messageCorrelationOk(replyId_197, corrId_197)
  ELSE FALSE

(*
  @type: ((Str, Str, Str) => Bool);
*)
typedResponseReferenceOk(responseOpId_253, corrId_253, corrType_253) ==
  IF corrType_253 = "elicitation"
  THEN elicitationCorrelationOk(responseOpId_253, corrId_253)
  ELSE FALSE

(*
  @type: (() => Bool);
*)
typed_correlation ==
  ((((((((((((((((((((((((((((((commandIds \subseteq COMMAND_ID_SPACE
                                                                /\ messageIds
                                                                  \subseteq MESSAGE_ID_SPACE)
                                                              /\ elicitationIds
                                                                \subseteq ELICITATION_ID_SPACE)
                                                            /\ replyIds
                                                              \subseteq REPLY_ID_SPACE)
                                                          /\ responseOperationIds
                                                            \subseteq RESPONSE_OP_ID_SPACE)
                                                        /\ Cardinality((COMMAND_ID_SPACE
                                                          \intersect MESSAGE_ID_SPACE))
                                                          = 0)
                                                      /\ Cardinality((COMMAND_ID_SPACE
                                                        \intersect REPLY_ID_SPACE))
                                                        = 0)
                                                    /\ Cardinality((COMMAND_ID_SPACE
                                                      \intersect EVENT_ID_SPACE))
                                                      = 0)
                                                  /\ Cardinality((COMMAND_ID_SPACE
                                                    \intersect ELICITATION_ID_SPACE))
                                                    = 0)
                                                /\ Cardinality((MESSAGE_ID_SPACE
                                                  \intersect REPLY_ID_SPACE))
                                                  = 0)
                                              /\ Cardinality((MESSAGE_ID_SPACE
                                                \intersect EVENT_ID_SPACE))
                                                = 0)
                                            /\ Cardinality((MESSAGE_ID_SPACE
                                              \intersect ELICITATION_ID_SPACE))
                                              = 0)
                                          /\ Cardinality((REPLY_ID_SPACE
                                            \intersect EVENT_ID_SPACE))
                                            = 0)
                                        /\ Cardinality((REPLY_ID_SPACE
                                          \intersect ELICITATION_ID_SPACE))
                                          = 0)
                                      /\ Cardinality((EVENT_ID_SPACE
                                        \intersect ELICITATION_ID_SPACE))
                                        = 0)
                                    /\ Cardinality((commandIds
                                      \intersect messageIds))
                                      = 0)
                                  /\ Cardinality((commandIds \intersect replyIds))
                                    = 0)
                                /\ Cardinality((commandIds
                                  \intersect EVENT_ID_SPACE))
                                  = 0)
                              /\ Cardinality((commandIds
                                \intersect elicitationIds))
                                = 0)
                            /\ Cardinality((messageIds \intersect replyIds)) = 0)
                          /\ Cardinality((messageIds \intersect EVENT_ID_SPACE))
                            = 0)
                        /\ Cardinality((messageIds \intersect elicitationIds))
                          = 0)
                      /\ Cardinality((replyIds \intersect EVENT_ID_SPACE)) = 0)
                    /\ Cardinality((replyIds \intersect elicitationIds)) = 0)
                  /\ Cardinality((EVENT_ID_SPACE \intersect elicitationIds)) = 0)
                /\ Cardinality((responseOperationIds \intersect commandIds)) = 0)
              /\ Cardinality((responseOperationIds \intersect messageIds)) = 0)
            /\ Cardinality((responseOperationIds \intersect replyIds)) = 0)
          /\ Cardinality((responseOperationIds \intersect EVENT_ID_SPACE)) = 0)
        /\ Cardinality((responseOperationIds \intersect elicitationIds)) = 0)
      /\ (\A r_917 \in replyIds: recordedReplyIndependentOk(r_917)))
    /\ (\A ro_924 \in responseOperationIds:
      recordedResponseOpIndependentOk(ro_924))

(*
  @type: (() => Bool);
*)
q_init == init

(*
  @type: ((Str, Str, Str) => Bool);
*)
replyRecordable(replyId_216, corrId_216, corrType_216) ==
  IF replyId_216 \in REPLY_ID_SPACE /\ ~(replyId_216 \in replyIds)
  THEN typedReferenceOk(replyId_216, corrId_216, corrType_216)
  ELSE FALSE

(*
  @type: ((Str, Str, Str, Str) => Bool);
*)
responseOperationRecordable(responseOpId_277, opKind_277, corrId_277, corrType_277) ==
  IF (responseOpId_277 \in RESPONSE_OP_ID_SPACE
      /\ ~(responseOpId_277 \in responseOperationIds))
    /\ opKind_277 \in RESPONSE_OPERATION_KINDS
  THEN typedResponseReferenceOk(responseOpId_277, corrId_277, corrType_277)
  ELSE FALSE

(*
  @type: ((Str, Str, Str) => Bool);
*)
createReply(replyId_582, corrId_582, corrType_582) ==
  (replyRecordable(replyId_582, corrId_582, corrType_582)
      /\ commandIds' := commandIds
      /\ messageIds' := messageIds
      /\ elicitationIds' := elicitationIds
      /\ replyIds' := (replyIds \union {replyId_582})
      /\ replyCorrelatesTo'
        := [ replyCorrelatesTo EXCEPT ![replyId_582] = corrId_582 ]
      /\ replyCorrelationType'
        := [ replyCorrelationType EXCEPT ![replyId_582] = corrType_582 ]
      /\ responseOperationIds' := responseOperationIds
      /\ responseOperationKind' := responseOperationKind
      /\ responseCorrelatesTo' := responseCorrelatesTo
      /\ responseCorrelationType' := responseCorrelationType)
    \/ (~(replyRecordable(replyId_582, corrId_582, corrType_582))
      /\ commandIds' := commandIds
      /\ messageIds' := messageIds
      /\ elicitationIds' := elicitationIds
      /\ replyIds' := replyIds
      /\ replyCorrelatesTo' := replyCorrelatesTo
      /\ replyCorrelationType' := replyCorrelationType
      /\ responseOperationIds' := responseOperationIds
      /\ responseOperationKind' := responseOperationKind
      /\ responseCorrelatesTo' := responseCorrelatesTo
      /\ responseCorrelationType' := responseCorrelationType)

(*
  @type: ((Str, Str, Str, Str) => Bool);
*)
createResponseOperation(responseOpId_674, opKind_674, corrId_674, corrType_674) ==
  (responseOperationRecordable(responseOpId_674, opKind_674, corrId_674, corrType_674)
      /\ commandIds' := commandIds
      /\ messageIds' := messageIds
      /\ elicitationIds' := elicitationIds
      /\ replyIds' := replyIds
      /\ replyCorrelatesTo' := replyCorrelatesTo
      /\ replyCorrelationType' := replyCorrelationType
      /\ responseOperationIds'
        := (responseOperationIds \union {responseOpId_674})
      /\ responseOperationKind'
        := [ responseOperationKind EXCEPT ![responseOpId_674] = opKind_674 ]
      /\ responseCorrelatesTo'
        := [ responseCorrelatesTo EXCEPT ![responseOpId_674] = corrId_674 ]
      /\ responseCorrelationType'
        := [ responseCorrelationType EXCEPT ![responseOpId_674] = corrType_674 ])
    \/ (~(responseOperationRecordable(responseOpId_674, opKind_674, corrId_674, corrType_674))
      /\ commandIds' := commandIds
      /\ messageIds' := messageIds
      /\ elicitationIds' := elicitationIds
      /\ replyIds' := replyIds
      /\ replyCorrelatesTo' := replyCorrelatesTo
      /\ replyCorrelationType' := replyCorrelationType
      /\ responseOperationIds' := responseOperationIds
      /\ responseOperationKind' := responseOperationKind
      /\ responseCorrelatesTo' := responseCorrelatesTo
      /\ responseCorrelationType' := responseCorrelationType)

(*
  @type: (() => Bool);
*)
attemptReply ==
  \E replyId \in ALL_ID_ATTEMPTS:
    \E corrId \in ALL_ID_ATTEMPTS:
      \E corrType \in CORRELATION_TYPE_ATTEMPTS:
        createReply(replyId, corrId, corrType)

(*
  @type: (() => Bool);
*)
attemptResponseOperation ==
  \E responseOpId \in ALL_ID_ATTEMPTS:
    \E opKind \in RESPONSE_OPERATION_KIND_ATTEMPTS:
      \E corrId \in ALL_ID_ATTEMPTS:
        \E corrType \in CORRELATION_TYPE_ATTEMPTS:
          createResponseOperation(responseOpId, opKind, corrId, corrType)

(*
  @type: (() => Bool);
*)
step == attemptReply \/ attemptResponseOperation

(*
  @type: (() => Bool);
*)
q_step == step

================================================================================
