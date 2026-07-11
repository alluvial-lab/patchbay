---------------------------- MODULE counterexample ----------------------------

EXTENDS subscription_authority

(* Constant initialization state *)
ConstInit == TRUE

(* Initial state [_transition(0)] *)
State0 ==
  LastSubscriptionActor = "alice"
    /\ LastSubscriptionScope = "scope-stream-ops-all"
    /\ SubscriptionAccepted = "no"
    /\ SubscriptionAttempted = "no"
    /\ SubscriptionEstablishAttempts = 0
    /\ auditRecords = 0
    /\ eventFilter
      = SetAsFun({ <<1, "none">>,
        <<2, "none">>,
        <<3, "none">>,
        <<4, "none">>,
        <<5, "none">> })
    /\ eventLsn = 0
    /\ eventStream
      = SetAsFun({ <<1, "none">>,
        <<2, "none">>,
        <<3, "none">>,
        <<4, "none">>,
        <<5, "none">> })
    /\ gSubElicitationsOpsStatus = "revoked"
    /\ gSubOpsAllStatus = "active"
    /\ operationRecordsCreated = 0
    /\ phase = 0
    /\ replayedEvents = 0
    /\ subscriptionCursor = 0
    /\ subscriptionFilter = "all"
    /\ subscriptionGrantId = "none"
    /\ subscriptionId = "none"
    /\ subscriptionStream = "stream-ops"

(* State1 [_transition(0)] *)
State1 ==
  LastSubscriptionActor = "svc"
    /\ LastSubscriptionScope = "scope-stream-elicitations-all"
    /\ SubscriptionAccepted = "no"
    /\ SubscriptionAttempted = "yes"
    /\ SubscriptionEstablishAttempts = 1
    /\ auditRecords = 0
    /\ eventFilter
      = SetAsFun({ <<1, "none">>,
        <<2, "none">>,
        <<3, "none">>,
        <<4, "none">>,
        <<5, "none">> })
    /\ eventLsn = 0
    /\ eventStream
      = SetAsFun({ <<1, "none">>,
        <<2, "none">>,
        <<3, "none">>,
        <<4, "none">>,
        <<5, "none">> })
    /\ gSubElicitationsOpsStatus = "revoked"
    /\ gSubOpsAllStatus = "active"
    /\ operationRecordsCreated = 0
    /\ phase = 1
    /\ replayedEvents = 0
    /\ subscriptionCursor = 0
    /\ subscriptionFilter = "all"
    /\ subscriptionGrantId = "none"
    /\ subscriptionId = "none"
    /\ subscriptionStream = "stream-elicitations"

(* The following formula holds true in the last state and violates the invariant *)
InvariantViolation == ~(auditRecords = SubscriptionEstablishAttempts)

================================================================================
(* Created by Apalache on Sat Jul 11 10:36:11 MDT 2026 *)
(* https://github.com/apalache-mc/apalache *)
