# Usage statistics is OFF. We care about your privacy.
# If you want to help our project, consider enabling statistics with config --enable-stats=true.

Output directory: /home/agent/projects/patchbay/_apalache-out/server/2026-07-08T19-05-00_4228965375540662680
# APALACHE version: 0.56.1 | build: 70cdaf4                       I@19:05:00.731
Starting checker server on port 8822...                           I@19:05:00.741
The Apalache server is running on port 8822. Press Ctrl-C to stop.
PASS #0: SanyParser                                               I@19:05:03.537
------------------------ MODULE subscription_authority ------------------------

EXTENDS Integers, Sequences, FiniteSets, TLC, Apalache, Variants

(*
  @type: (() => Set(Str));
*)
FILTERS == { "all", "ops-only" }

(*
  @type: (() => Set(Int));
*)
CURSORS == 0 .. 4

(*
  @type: (() => Set(Str));
*)
GRANTS == { "g-sub-ops-all", "g-sub-elicitations-ops" }

(*
  @type: (() => Set(Int));
*)
EVENT_LSNS == 1 .. 5

VARIABLE
  (*
    @type: Str;
  *)
  gSubOpsAllStatus

VARIABLE
  (*
    @type: Str;
  *)
  gSubElicitationsOpsStatus

VARIABLE
  (*
    @type: Str;
  *)
  subscriptionId

VARIABLE
  (*
    @type: Str;
  *)
  subscriptionGrantId

VARIABLE
  (*
    @type: Int;
  *)
  subscriptionCursor

VARIABLE
  (*
    @type: Str;
  *)
  subscriptionStream

VARIABLE
  (*
    @type: Str;
  *)
  subscriptionFilter

VARIABLE
  (*
    @type: Int;
  *)
  auditRecords

VARIABLE
  (*
    @type: Int;
  *)
  operationRecordsCreated

VARIABLE
  (*
    @type: Int;
  *)
  eventLsn

VARIABLE
  (*
    @type: (Int -> Str);
  *)
  eventStream

(*
  @type: (() => Set(Str));
*)
ACTORS == { "alice", "bob", "svc" }

VARIABLE
  (*
    @type: (Int -> Str);
  *)
  eventFilter

VARIABLE
  (*
    @type: Int;
  *)
  replayedEvents

VARIABLE
  (*
    @type: Str;
  *)
  LastSubscriptionActor

VARIABLE
  (*
    @type: Str;
  *)
  LastSubscriptionScope

VARIABLE
  (*
    @type: Str;
  *)
  SubscriptionAttempted

VARIABLE
  (*
    @type: Str;
  *)
  SubscriptionAccepted

VARIABLE
  (*
    @type: Int;
  *)
  SubscriptionEstablishAttempts

VARIABLE
  (*
    @type: Int;
  *)
  phase

(*
  @type: (() => Set(Str));
*)
STREAMS == { "stream-ops", "stream-elicitations" }

(*
  @type: ((Str) => Str);
*)
grantSubject(g_115) ==
  IF g_115 = "g-sub-elicitations-ops" THEN "bob" ELSE "alice"

(*
  @type: ((Str, Str) => Str);
*)
streamScope(stream_86, filt_86) ==
  IF stream_86 = "stream-ops"
  THEN IF filt_86 = "all"
  THEN "scope-stream-ops-all"
  ELSE "scope-stream-ops-ops-only"
  ELSE IF filt_86 = "all"
  THEN "scope-stream-elicitations-all"
  ELSE "scope-stream-elicitations-ops-only"

(*
  @type: ((Str) => Str);
*)
grantStatus(g_106) ==
  IF g_106 = "g-sub-ops-all"
  THEN gSubOpsAllStatus
  ELSE IF g_106 = "g-sub-elicitations-ops"
  THEN gSubElicitationsOpsStatus
  ELSE "revoked"

(*
  @type: ((Str, Str) => Bool);
*)
grantAllows(g_144, kind_144) == g_144 \in GRANTS /\ kind_144 = "subscribe"

(*
  @type: ((Int) => Bool);
*)
eventMatchesSubscription(lsn_261) ==
  ((lsn_261 \in EVENT_LSNS /\ lsn_261 <= eventLsn)
      /\ eventStream[lsn_261] = subscriptionStream)
    /\ eventFilter[lsn_261] = subscriptionFilter

(*
  @type: ((Str, Str) => Bool);
*)
emitEvent(stream_554, filt_554) ==
  stream_554 \in STREAMS
    /\ filt_554 \in FILTERS
    /\ eventLsn < 5
    /\ gSubOpsAllStatus' := gSubOpsAllStatus
    /\ gSubElicitationsOpsStatus' := gSubElicitationsOpsStatus
    /\ subscriptionId' := subscriptionId
    /\ subscriptionGrantId' := subscriptionGrantId
    /\ subscriptionCursor' := subscriptionCursor
    /\ subscriptionStream' := subscriptionStream
    /\ subscriptionFilter' := subscriptionFilter
    /\ auditRecords' := auditRecords
    /\ operationRecordsCreated' := operationRecordsCreated
    /\ eventLsn' := (eventLsn + 1)
    /\ eventStream' := [ eventStream EXCEPT ![eventLsn + 1] = stream_554 ]
    /\ eventFilter' := [ eventFilter EXCEPT ![eventLsn + 1] = filt_554 ]
    /\ replayedEvents' := replayedEvents
    /\ LastSubscriptionActor' := LastSubscriptionActor
    /\ LastSubscriptionScope' := LastSubscriptionScope
    /\ SubscriptionAttempted' := SubscriptionAttempted
    /\ SubscriptionAccepted' := SubscriptionAccepted
    /\ SubscriptionEstablishAttempts' := SubscriptionEstablishAttempts
    /\ phase' := 2

(*
  @type: (() => Bool);
*)
subscription_audited ==
  (operationRecordsCreated = 0 /\ auditRecords = SubscriptionEstablishAttempts)
    /\ SubscriptionEstablishAttempts <= 1

(*
  @type: ((Str) => Bool);
*)
grantLive(g_92) == g_92 \in GRANTS

(*
  @type: ((Str) => Str);
*)
grantScope(g_133) ==
  IF g_133 = "g-sub-ops-all"
  THEN streamScope("stream-ops", "all")
  ELSE IF g_133 = "g-sub-elicitations-ops"
  THEN streamScope("stream-elicitations", "ops-only")
  ELSE "none"

(*
  @type: (() => Bool);
*)
init ==
  gSubOpsAllStatus = "active"
    /\ gSubElicitationsOpsStatus = "revoked"
    /\ subscriptionId = "none"
    /\ subscriptionGrantId = "none"
    /\ subscriptionCursor = 0
    /\ subscriptionStream = "stream-ops"
    /\ subscriptionFilter = "all"
    /\ auditRecords = 0
    /\ operationRecordsCreated = 0
    /\ eventLsn = 0
    /\ eventStream = [ id__340 \in EVENT_LSNS |-> "none" ]
    /\ eventFilter = [ id__347 \in EVENT_LSNS |-> "none" ]
    /\ replayedEvents = 0
    /\ LastSubscriptionActor = "alice"
    /\ LastSubscriptionScope = streamScope("stream-ops", "all")
    /\ SubscriptionAttempted = "no"
    /\ SubscriptionAccepted = "no"
    /\ SubscriptionEstablishAttempts = 0
    /\ phase = 0

(*
  @type: ((Int) => Int);
*)
replayEventFor(cursor_277) ==
  IF (SubscriptionAccepted = "yes" /\ eventLsn > cursor_277)
    /\ eventMatchesSubscription(eventLsn)
  THEN eventLsn
  ELSE 0

(*
  @type: ((Str, Str, Str, Str) => Bool);
*)
actionGrantAuthorizesSubscription(g_177, actor_177, stream_177, filt_177) ==
  IF grantLive(g_177)
  THEN IF grantStatus(g_177) = "active"
  THEN IF grantScope(g_177) = streamScope(stream_177, filt_177)
  THEN IF grantSubject(g_177) = actor_177
  THEN grantAllows(g_177, "subscribe")
  ELSE FALSE
  ELSE FALSE
  ELSE FALSE
  ELSE FALSE

(*
  @type: (() => Bool);
*)
acceptedSubscriptionHasRawGrant ==
  IF subscriptionGrantId \in GRANTS
  THEN IF grantLive(subscriptionGrantId)
  THEN IF grantStatus(subscriptionGrantId) = "active"
  THEN IF grantSubject(subscriptionGrantId) = LastSubscriptionActor
  THEN IF grantScope(subscriptionGrantId) = LastSubscriptionScope
  THEN grantAllows(subscriptionGrantId, "subscribe")
  ELSE FALSE
  ELSE FALSE
  ELSE FALSE
  ELSE FALSE
  ELSE FALSE

(*
  @type: (() => Bool);
*)
q_init == init

(*
  @type: ((Int) => Bool);
*)
replayByCursor(cursor_619) ==
  cursor_619 \in CURSORS
    /\ gSubOpsAllStatus' := gSubOpsAllStatus
    /\ gSubElicitationsOpsStatus' := gSubElicitationsOpsStatus
    /\ subscriptionId' := subscriptionId
    /\ subscriptionGrantId' := subscriptionGrantId
    /\ subscriptionCursor' := cursor_619
    /\ subscriptionStream' := subscriptionStream
    /\ subscriptionFilter' := subscriptionFilter
    /\ auditRecords' := auditRecords
    /\ operationRecordsCreated' := operationRecordsCreated
    /\ eventLsn' := eventLsn
    /\ eventStream' := eventStream
    /\ eventFilter' := eventFilter
    /\ replayedEvents' := (replayEventFor(cursor_619))
    /\ LastSubscriptionActor' := LastSubscriptionActor
    /\ LastSubscriptionScope' := LastSubscriptionScope
    /\ SubscriptionAttempted' := SubscriptionAttempted
    /\ SubscriptionAccepted' := SubscriptionAccepted
    /\ SubscriptionEstablishAttempts' := SubscriptionEstablishAttempts
    /\ phase' := 2

(*
  @type: ((Str, Str, Str) => Str);
*)
liveSubscriptionGrantId(actor_197, stream_197, filt_197) ==
  IF actionGrantAuthorizesSubscription("g-sub-ops-all", actor_197, stream_197, filt_197)
  THEN "g-sub-ops-all"
  ELSE IF actionGrantAuthorizesSubscription("g-sub-elicitations-ops", actor_197,
  stream_197, filt_197)
  THEN "g-sub-elicitations-ops"
  ELSE "none"

(*
  @type: ((Int) => Bool);
*)
replayedEventIndependentOk(lsn_305) ==
  ((((lsn_305 \in EVENT_LSNS /\ lsn_305 > subscriptionCursor)
          /\ lsn_305 <= eventLsn)
        /\ eventStream[lsn_305] = subscriptionStream)
      /\ eventFilter[lsn_305] = subscriptionFilter)
    /\ acceptedSubscriptionHasRawGrant

(*
  @type: (() => Bool);
*)
subscription_grant_checked ==
  (SubscriptionAccepted = "no" /\ subscriptionId = "none")
    \/ ((SubscriptionAccepted = "yes" /\ subscriptionId = "sub1")
      /\ acceptedSubscriptionHasRawGrant)

(*
  @type: ((Str, Str, Str) => Bool);
*)
subscriptionAllowed(actor_208, stream_208, filt_208) ==
  liveSubscriptionGrantId(actor_208, stream_208, filt_208) /= "none"

(*
  @type: (() => Bool);
*)
subscription_cursor_replay_authorized ==
  replayedEvents = 0 \/ replayedEventIndependentOk(replayedEvents)

(*
  @type: ((Str, Str, Str, Int) => Bool);
*)
attemptEstablish(actor_471, stream_471, filt_471, cursor_471) ==
  actor_471 \in ACTORS
    /\ stream_471 \in STREAMS
    /\ filt_471 \in FILTERS
    /\ cursor_471 \in CURSORS
    /\ gSubOpsAllStatus' := gSubOpsAllStatus
    /\ gSubElicitationsOpsStatus' := gSubElicitationsOpsStatus
    /\ subscriptionId'
      := (IF subscriptionAllowed(actor_471, stream_471, filt_471)
      THEN "sub1"
      ELSE "none")
    /\ subscriptionGrantId'
      := (liveSubscriptionGrantId(actor_471, stream_471, filt_471))
    /\ subscriptionCursor' := cursor_471
    /\ subscriptionStream' := stream_471
    /\ subscriptionFilter' := filt_471
    /\ auditRecords' := (auditRecords + 1)
    /\ operationRecordsCreated' := operationRecordsCreated
    /\ eventLsn' := eventLsn
    /\ eventStream' := eventStream
    /\ eventFilter' := eventFilter
    /\ replayedEvents' := replayedEvents
    /\ LastSubscriptionActor' := actor_471
    /\ LastSubscriptionScope' := (streamScope(stream_471, filt_471))
    /\ SubscriptionAttempted' := "yes"
    /\ SubscriptionAccepted'
      := (IF subscriptionAllowed(actor_471, stream_471, filt_471)
      THEN "yes"
      ELSE "no")
    /\ SubscriptionEstablishAttempts' := (SubscriptionEstablishAttempts + 1)
    /\ phase' := 1

(*
  @type: (() => Bool);
*)
step ==
  (\E actor \in ACTORS:
      \E stream \in STREAMS:
        \E filt \in FILTERS:
          \E cursor \in CURSORS:
            phase = 0 /\ attemptEstablish(actor, stream, filt, cursor))
    \/ (\E eventStreamCandidate \in STREAMS:
      \E eventFilterCandidate \in FILTERS:
        phase = 1 /\ emitEvent(eventStreamCandidate, eventFilterCandidate))
    \/ (\E replayCursor \in CURSORS: phase = 2 /\ replayByCursor(replayCursor))

(*
  @type: (() => Bool);
*)
q_step == step

================================================================================
