# Usage statistics is OFF. We care about your privacy.
# If you want to help our project, consider enabling statistics with config --enable-stats=true.

Output directory: /home/agent/projects/patchbay/_apalache-out/server/2026-07-08T17-39-35_17276014182664681011
# APALACHE version: 0.56.1 | build: 70cdaf4                       I@17:39:35.369
Starting checker server on port 8822...                           I@17:39:35.383
The Apalache server is running on port 8822. Press Ctrl-C to stop.
PASS #0: SanyParser                                               I@17:39:38.176
------------------------- MODULE elicitation_lifecycle -------------------------

EXTENDS Integers, Sequences, FiniteSets, TLC, Apalache, Variants

VARIABLE
  (*
    @type: (Str -> Str);
  *)
  state

VARIABLE
  (*
    @type: (Str -> Int);
  *)
  terminalLsn

VARIABLE
  (*
    @type: Int;
  *)
  lsn

VARIABLE
  (*
    @type: (Str -> Str);
  *)
  responderActor

VARIABLE
  (*
    @type: (Str -> Str);
  *)
  answeredBy

VARIABLE
  (*
    @type: (Str -> Str);
  *)
  contractKind

VARIABLE
  (*
    @type: (Str -> Str);
  *)
  elicitationDomain

VARIABLE
  (*
    @type: (Str -> Str);
  *)
  targetSession

VARIABLE
  (*
    @type: (Str -> Int);
  *)
  targetGeneration

VARIABLE
  (*
    @type: (Str -> Int);
  *)
  sessionGeneration

VARIABLE
  (*
    @type: (Str -> Str);
  *)
  responseOpElicitation

VARIABLE
  (*
    @type: (Str -> Str);
  *)
  responseOpKind

VARIABLE
  (*
    @type: (Str -> Str);
  *)
  responseOpDomain

VARIABLE
  (*
    @type: (Str -> Str);
  *)
  responseOpSession

VARIABLE
  (*
    @type: (Str -> Int);
  *)
  responseOpGeneration

VARIABLE
  (*
    @type: (Str -> Str);
  *)
  responseOpActor

VARIABLE
  (*
    @type: (Str -> Str);
  *)
  responseOpEndpoint

VARIABLE
  (*
    @type: (Str -> Bool);
  *)
  responseValid

(*
  @type: (() => Set(Str));
*)
NON_ANSWER_TERMINAL ==
  { "declined", "expired", "cancelled", "withdrawn", "superseded", "stale" }

VARIABLE
  (*
    @type: (Str -> Bool);
  *)
  responseDuplicate

VARIABLE
  (*
    @type: (Str -> Str);
  *)
  answeredResponseOp

VARIABLE
  (*
    @type: (Str -> Str);
  *)
  firstTerminalState

VARIABLE
  (*
    @type: (Str -> Str);
  *)
  firstAnsweredBy

VARIABLE
  (*
    @type: (Str -> Str);
  *)
  firstAnsweredResponseOp

(*
  @type: (() => Set(Str));
*)
NON_TERMINAL == { "opened", "pending" }

(*
  @type: (() => Set(Str));
*)
ELICITATION_IDS == {"e1"}

(*
  @type: (() => Set(Str));
*)
RESPONSE_OP_IDS == { "ro1", "ro2" }

(*
  @type: (() => Set(Str));
*)
ENDPOINTS == { "ep-a", "ep-b" }

(*
  @type: (() => Set(Str));
*)
ACTORS == {"alice"}

(*
  @type: (() => Set(Str));
*)
AUTHORITY_DOMAINS == {"domain-main"}

(*
  @type: (() => Set(Str));
*)
SESSIONS == { "s1", "s2" }

(*
  @type: (() => Set(Int));
*)
GENERATIONS == 0 .. 2

(*
  @type: (() => Set(Str));
*)
CONTRACT_KINDS == { "approval", "question" }

(*
  @type: (() => Set(Str));
*)
RESPONSE_KINDS == { "approval-response", "elicitation-response" }

(*
  @type: (() => Set(Str));
*)
COMMAND_IDS == { "c1", "c2" }

(*
  @type: (() => Set(Str));
*)
MESSAGE_IDS == { "m1", "m2" }

(*
  @type: (() => Set(Str));
*)
REPLY_IDS == { "r1", "r2" }

(*
  @type: (() => Set(Str));
*)
EVENT_IDS == { "ev1", "ev2" }

(*
  @type: (() => Set(Str));
*)
TERMINAL ==
  { "answered",
    "declined",
    "expired",
    "cancelled",
    "withdrawn",
    "superseded",
    "stale" }

(*
  @type: ((Str) => Bool);
*)
advanceSessionGeneration(eid_1657) ==
  state[eid_1657] = "pending"
    /\ targetGeneration[eid_1657] < 2
    /\ state' := state
    /\ lsn' := (lsn + 1)
    /\ terminalLsn' := terminalLsn
    /\ sessionGeneration'
      := [
        sessionGeneration EXCEPT
          ![targetSession[eid_1657]] = targetGeneration[eid_1657] + 1
      ]
    /\ responderActor' := responderActor
    /\ answeredBy' := answeredBy
    /\ contractKind' := contractKind
    /\ elicitationDomain' := elicitationDomain
    /\ targetSession' := targetSession
    /\ targetGeneration' := targetGeneration
    /\ responseOpElicitation' := responseOpElicitation
    /\ responseOpKind' := responseOpKind
    /\ responseOpDomain' := responseOpDomain
    /\ responseOpSession' := responseOpSession
    /\ responseOpGeneration' := responseOpGeneration
    /\ responseOpActor' := responseOpActor
    /\ responseOpEndpoint' := responseOpEndpoint
    /\ responseValid' := responseValid
    /\ responseDuplicate' := responseDuplicate
    /\ answeredResponseOp' := answeredResponseOp
    /\ firstTerminalState' := firstTerminalState
    /\ firstAnsweredBy' := firstAnsweredBy
    /\ firstAnsweredResponseOp' := firstAnsweredResponseOp

(*
  @type: ((Str) => Bool);
*)
goStale(eid_1845) ==
  (state[eid_1845] = "pending"
      /\ targetGeneration[eid_1845] < 2
      /\ state' := [ state EXCEPT ![eid_1845] = "stale" ]
      /\ lsn' := (lsn + 1)
      /\ terminalLsn' := [ terminalLsn EXCEPT ![eid_1845] = lsn + 1 ]
      /\ sessionGeneration'
        := [
          sessionGeneration EXCEPT
            ![targetSession[eid_1845]] = targetGeneration[eid_1845] + 1
        ]
      /\ responderActor' := responderActor
      /\ answeredBy' := answeredBy
      /\ contractKind' := contractKind
      /\ elicitationDomain' := elicitationDomain
      /\ targetSession' := targetSession
      /\ targetGeneration' := targetGeneration
      /\ responseOpElicitation' := responseOpElicitation
      /\ responseOpKind' := responseOpKind
      /\ responseOpDomain' := responseOpDomain
      /\ responseOpSession' := responseOpSession
      /\ responseOpGeneration' := responseOpGeneration
      /\ responseOpActor' := responseOpActor
      /\ responseOpEndpoint' := responseOpEndpoint
      /\ responseValid' := responseValid
      /\ responseDuplicate' := responseDuplicate
      /\ answeredResponseOp' := answeredResponseOp
      /\ firstTerminalState'
        := (IF terminalLsn[eid_1845] = 0
        THEN [ firstTerminalState EXCEPT ![eid_1845] = "stale" ]
        ELSE firstTerminalState)
      /\ firstAnsweredBy' := firstAnsweredBy
      /\ firstAnsweredResponseOp' := firstAnsweredResponseOp)
    \/ (state[eid_1845] \in TERMINAL
      /\ state' := state
      /\ lsn' := lsn
      /\ terminalLsn' := terminalLsn
      /\ sessionGeneration' := sessionGeneration
      /\ responderActor' := responderActor
      /\ answeredBy' := answeredBy
      /\ contractKind' := contractKind
      /\ elicitationDomain' := elicitationDomain
      /\ targetSession' := targetSession
      /\ targetGeneration' := targetGeneration
      /\ responseOpElicitation' := responseOpElicitation
      /\ responseOpKind' := responseOpKind
      /\ responseOpDomain' := responseOpDomain
      /\ responseOpSession' := responseOpSession
      /\ responseOpGeneration' := responseOpGeneration
      /\ responseOpActor' := responseOpActor
      /\ responseOpEndpoint' := responseOpEndpoint
      /\ responseValid' := responseValid
      /\ responseDuplicate' := responseDuplicate
      /\ answeredResponseOp' := answeredResponseOp
      /\ firstTerminalState' := firstTerminalState
      /\ firstAnsweredBy' := firstAnsweredBy
      /\ firstAnsweredResponseOp' := firstAnsweredResponseOp)

(*
  @type: (() => Bool);
*)
elicitation_pending_finality ==
  [](terminalLsn["e1"] > 0
    => (state["e1"] = firstTerminalState["e1"]
        /\ answeredBy["e1"] = firstAnsweredBy["e1"])
      /\ answeredResponseOp["e1"] = firstAnsweredResponseOp["e1"])

(*
  @type: (() => Bool);
*)
elicitation_first_answer_wins ==
  [](firstTerminalState["e1"] = "answered"
    => (state["e1"] = "answered" /\ answeredBy["e1"] = firstAnsweredBy["e1"])
      /\ answeredResponseOp["e1"] = firstAnsweredResponseOp["e1"])

(*
  @type: ((Str) => Bool);
*)
targetLive(eid_199) ==
  targetGeneration[eid_199] = sessionGeneration[targetSession[eid_199]]

(*
  @type: (() => Bool);
*)
elicitation_timeout_neither_success_nor_denial ==
  \A eid_2039 \in ELICITATION_IDS:
    firstTerminalState[eid_2039] = "expired"
      => ((answeredBy[eid_2039] = "none"
            /\ answeredResponseOp[eid_2039] = "none")
          /\ state[eid_2039] /= "answered")
        /\ state[eid_2039] /= "declined"

(*
  @type: (() => Bool);
*)
elicitation_stale_target_inert ==
  [](targetGeneration["e1"] < sessionGeneration[targetSession["e1"]]
    => ((state["e1"] /= "answered" /\ firstTerminalState["e1"] /= "answered")
        /\ answeredBy["e1"] = "none")
      /\ answeredResponseOp["e1"] = "none")

(*
  @type: (() => Bool);
*)
elicitation_withdrawal_finality ==
  [](firstTerminalState["e1"] = "withdrawn"
    => (state["e1"] = "withdrawn" /\ answeredBy["e1"] = firstAnsweredBy["e1"])
      /\ answeredResponseOp["e1"] = firstAnsweredResponseOp["e1"])

(*
  @type: ((Str) => Bool);
*)
responseOpUnused(responseOpId_250) ==
  responseOpElicitation[responseOpId_250] = "none"

(*
  @type: ((Str) => Bool);
*)
committedResponseOp(responseOpId_319) ==
  \E eid_317 \in ELICITATION_IDS: answeredResponseOp[eid_317] = responseOpId_319

(*
  @type: ((Str, Str) => Bool);
*)
duplicateAttempt(responseOpId_339, claimedEid_339) ==
  IF claimedEid_339 \in ELICITATION_IDS
  THEN state[claimedEid_339] = "answered"
    /\ answeredResponseOp[claimedEid_339] /= responseOpId_339
  ELSE FALSE

(*
  @type: (() => Set(Str));
*)
SUBMITTING_ACTORS == ACTORS \union {"mallory"}

(*
  @type: ((Str) => Bool);
*)
recordedResponseIndependentOk(responseOpId_418) ==
  IF responseOpElicitation[responseOpId_418] \in ELICITATION_IDS
  THEN ((((((responseOpKind[responseOpId_418] \in RESPONSE_KINDS
                /\ responseOpDomain[responseOpId_418]
                  = elicitationDomain[responseOpElicitation[responseOpId_418]])
              /\ responseOpSession[responseOpId_418]
                = targetSession[responseOpElicitation[responseOpId_418]])
            /\ responseOpGeneration[responseOpId_418]
              = targetGeneration[responseOpElicitation[responseOpId_418]])
          /\ responseOpGeneration[responseOpId_418]
            = sessionGeneration[responseOpSession[responseOpId_418]])
        /\ responseOpActor[responseOpId_418]
          = responderActor[responseOpElicitation[responseOpId_418]])
      /\ responseOpEndpoint[responseOpId_418] \in ENDPOINTS)
    /\ answeredResponseOp[responseOpElicitation[responseOpId_418]]
      = responseOpId_418
  ELSE FALSE

(*
  @type: (() => Set(Str));
*)
RESPONSE_DOMAINS == AUTHORITY_DOMAINS \union {"domain-forged"}

(*
  @type: (() => Bool);
*)
init ==
  state = SetAsFun({<<"e1", "opened">>})
    /\ terminalLsn = SetAsFun({<<"e1", 0>>})
    /\ lsn = 2
    /\ responderActor = SetAsFun({<<"e1", "alice">>})
    /\ answeredBy = SetAsFun({<<"e1", "none">>})
    /\ contractKind = SetAsFun({<<"e1", "approval">>})
    /\ elicitationDomain = SetAsFun({<<"e1", "domain-main">>})
    /\ targetSession = SetAsFun({<<"e1", "s1">>})
    /\ targetGeneration = SetAsFun({<<"e1", 0>>})
    /\ sessionGeneration = SetAsFun({ <<"s1", 0>>, <<"s2", 0>> })
    /\ responseOpElicitation = [ ro_483 \in RESPONSE_OP_IDS |-> "none" ]
    /\ responseOpKind = [ ro_490 \in RESPONSE_OP_IDS |-> "none" ]
    /\ responseOpDomain = [ ro_497 \in RESPONSE_OP_IDS |-> "domain-main" ]
    /\ responseOpSession = [ ro_504 \in RESPONSE_OP_IDS |-> "s1" ]
    /\ responseOpGeneration = [ ro_511 \in RESPONSE_OP_IDS |-> 0 ]
    /\ responseOpActor = [ ro_518 \in RESPONSE_OP_IDS |-> "alice" ]
    /\ responseOpEndpoint = [ ro_525 \in RESPONSE_OP_IDS |-> "none" ]
    /\ responseValid = [ ro_532 \in RESPONSE_OP_IDS |-> FALSE ]
    /\ responseDuplicate = [ ro_539 \in RESPONSE_OP_IDS |-> FALSE ]
    /\ answeredResponseOp = SetAsFun({<<"e1", "none">>})
    /\ firstTerminalState = SetAsFun({<<"e1", "none">>})
    /\ firstAnsweredBy = SetAsFun({<<"e1", "none">>})
    /\ firstAnsweredResponseOp = SetAsFun({<<"e1", "none">>})

(*
  @type: ((Str, Str, Str, Str, Int) => Bool);
*)
openElicitation(eid_669, actor_669, contract_669, session_669, generation_669) ==
  state[eid_669] = "opened"
    /\ actor_669 = responderActor[eid_669]
    /\ contract_669 = contractKind[eid_669]
    /\ session_669 = targetSession[eid_669]
    /\ generation_669 = targetGeneration[eid_669]
    /\ state' := state
    /\ terminalLsn' := terminalLsn
    /\ lsn' := lsn
    /\ responderActor' := responderActor
    /\ answeredBy' := answeredBy
    /\ contractKind' := contractKind
    /\ elicitationDomain' := elicitationDomain
    /\ targetSession' := targetSession
    /\ targetGeneration' := targetGeneration
    /\ sessionGeneration' := sessionGeneration
    /\ responseOpElicitation' := responseOpElicitation
    /\ responseOpKind' := responseOpKind
    /\ responseOpDomain' := responseOpDomain
    /\ responseOpSession' := responseOpSession
    /\ responseOpGeneration' := responseOpGeneration
    /\ responseOpActor' := responseOpActor
    /\ responseOpEndpoint' := responseOpEndpoint
    /\ responseValid' := responseValid
    /\ responseDuplicate' := responseDuplicate
    /\ answeredResponseOp' := answeredResponseOp
    /\ firstTerminalState' := firstTerminalState
    /\ firstAnsweredBy' := firstAnsweredBy
    /\ firstAnsweredResponseOp' := firstAnsweredResponseOp

(*
  @type: (() => Set(Str));
*)
ATTEMPTED_RESPONSE_KINDS == RESPONSE_KINDS \union {"spawn"}

(*
  @type: ((Str) => Bool);
*)
makePending(eid_752) ==
  state[eid_752] = "opened"
    /\ state' := [ state EXCEPT ![eid_752] = "pending" ]
    /\ lsn' := (lsn + 1)
    /\ terminalLsn' := terminalLsn
    /\ responderActor' := responderActor
    /\ answeredBy' := answeredBy
    /\ contractKind' := contractKind
    /\ elicitationDomain' := elicitationDomain
    /\ targetSession' := targetSession
    /\ targetGeneration' := targetGeneration
    /\ sessionGeneration' := sessionGeneration
    /\ responseOpElicitation' := responseOpElicitation
    /\ responseOpKind' := responseOpKind
    /\ responseOpDomain' := responseOpDomain
    /\ responseOpSession' := responseOpSession
    /\ responseOpGeneration' := responseOpGeneration
    /\ responseOpActor' := responseOpActor
    /\ responseOpEndpoint' := responseOpEndpoint
    /\ responseValid' := responseValid
    /\ responseDuplicate' := responseDuplicate
    /\ answeredResponseOp' := answeredResponseOp
    /\ firstTerminalState' := firstTerminalState
    /\ firstAnsweredBy' := firstAnsweredBy
    /\ firstAnsweredResponseOp' := firstAnsweredResponseOp

(*
  @type: ((Str, Str) => Bool);
*)
commitTerminal(eid_933, terminalState_933) ==
  (state[eid_933] = "pending"
      /\ terminalState_933 \in TERMINAL
      /\ state' := [ state EXCEPT ![eid_933] = terminalState_933 ]
      /\ lsn' := (lsn + 1)
      /\ terminalLsn' := [ terminalLsn EXCEPT ![eid_933] = lsn + 1 ]
      /\ responderActor' := responderActor
      /\ answeredBy' := answeredBy
      /\ contractKind' := contractKind
      /\ elicitationDomain' := elicitationDomain
      /\ targetSession' := targetSession
      /\ targetGeneration' := targetGeneration
      /\ sessionGeneration' := sessionGeneration
      /\ responseOpElicitation' := responseOpElicitation
      /\ responseOpKind' := responseOpKind
      /\ responseOpDomain' := responseOpDomain
      /\ responseOpSession' := responseOpSession
      /\ responseOpGeneration' := responseOpGeneration
      /\ responseOpActor' := responseOpActor
      /\ responseOpEndpoint' := responseOpEndpoint
      /\ responseValid' := responseValid
      /\ responseDuplicate' := responseDuplicate
      /\ answeredResponseOp' := answeredResponseOp
      /\ firstTerminalState'
        := (IF terminalLsn[eid_933] = 0
        THEN [ firstTerminalState EXCEPT ![eid_933] = terminalState_933 ]
        ELSE firstTerminalState)
      /\ firstAnsweredBy' := firstAnsweredBy
      /\ firstAnsweredResponseOp' := firstAnsweredResponseOp)
    \/ (state[eid_933] \in TERMINAL
      /\ terminalState_933 \in TERMINAL
      /\ state' := state
      /\ lsn' := lsn
      /\ terminalLsn' := terminalLsn
      /\ responderActor' := responderActor
      /\ answeredBy' := answeredBy
      /\ contractKind' := contractKind
      /\ elicitationDomain' := elicitationDomain
      /\ targetSession' := targetSession
      /\ targetGeneration' := targetGeneration
      /\ sessionGeneration' := sessionGeneration
      /\ responseOpElicitation' := responseOpElicitation
      /\ responseOpKind' := responseOpKind
      /\ responseOpDomain' := responseOpDomain
      /\ responseOpSession' := responseOpSession
      /\ responseOpGeneration' := responseOpGeneration
      /\ responseOpActor' := responseOpActor
      /\ responseOpEndpoint' := responseOpEndpoint
      /\ responseValid' := responseValid
      /\ responseDuplicate' := responseDuplicate
      /\ answeredResponseOp' := answeredResponseOp
      /\ firstTerminalState' := firstTerminalState
      /\ firstAnsweredBy' := firstAnsweredBy
      /\ firstAnsweredResponseOp' := firstAnsweredResponseOp)

(*
  @type: (() => Set(Str));
*)
CLAIMED_ELICITATION_IDS ==
  ((((ELICITATION_IDS \union COMMAND_IDS) \union MESSAGE_IDS) \union REPLY_IDS)
    \union EVENT_IDS)
    \union {"unknown"}

(*
  @type: ((Str, Str) => Bool);
*)
lateAnswer(eid_1533, responseOpId_1533) ==
  state[eid_1533] \in TERMINAL
    /\ ~(committedResponseOp(responseOpId_1533))
    /\ responseOpElicitation[responseOpId_1533] = "none"
    /\ state' := state
    /\ lsn' := lsn
    /\ terminalLsn' := terminalLsn
    /\ answeredBy' := answeredBy
    /\ responseOpElicitation'
      := [ responseOpElicitation EXCEPT ![responseOpId_1533] = eid_1533 ]
    /\ responseOpKind'
      := [ responseOpKind EXCEPT ![responseOpId_1533] = "elicitation-response" ]
    /\ responseOpDomain'
      := [
        responseOpDomain EXCEPT
          ![responseOpId_1533] = elicitationDomain[eid_1533]
      ]
    /\ responseOpSession'
      := [
        responseOpSession EXCEPT
          ![responseOpId_1533] = targetSession[eid_1533]
      ]
    /\ responseOpGeneration'
      := [
        responseOpGeneration EXCEPT
          ![responseOpId_1533] = targetGeneration[eid_1533]
      ]
    /\ responseOpActor'
      := [
        responseOpActor EXCEPT
          ![responseOpId_1533] = responderActor[eid_1533]
      ]
    /\ responseOpEndpoint'
      := [ responseOpEndpoint EXCEPT ![responseOpId_1533] = "ep-b" ]
    /\ responseValid' := [ responseValid EXCEPT ![responseOpId_1533] = FALSE ]
    /\ responseDuplicate'
      := [
        responseDuplicate EXCEPT
          ![responseOpId_1533] = state[eid_1533] = "answered"
      ]
    /\ answeredResponseOp' := answeredResponseOp
    /\ firstTerminalState' := firstTerminalState
    /\ firstAnsweredBy' := firstAnsweredBy
    /\ firstAnsweredResponseOp' := firstAnsweredResponseOp
    /\ responderActor' := responderActor
    /\ contractKind' := contractKind
    /\ elicitationDomain' := elicitationDomain
    /\ targetSession' := targetSession
    /\ targetGeneration' := targetGeneration
    /\ sessionGeneration' := sessionGeneration

(*
  @type: ((Str) => Bool);
*)
decline(eid_1539) == commitTerminal(eid_1539, "declined")

(*
  @type: ((Str) => Bool);
*)
expire(eid_1545) == commitTerminal(eid_1545, "expired")

(*
  @type: ((Str) => Bool);
*)
cancel(eid_1551) == commitTerminal(eid_1551, "cancelled")

(*
  @type: ((Str) => Bool);
*)
withdraw(eid_1557) == commitTerminal(eid_1557, "withdrawn")

(*
  @type: ((Str) => Bool);
*)
supersede(eid_1563) == commitTerminal(eid_1563, "superseded")

(*
  @type: (() => Bool);
*)
elicitation_correlation_typed ==
  (((Cardinality((ELICITATION_IDS \intersect COMMAND_IDS)) = 0
          /\ Cardinality((ELICITATION_IDS \intersect MESSAGE_IDS)) = 0)
        /\ Cardinality((ELICITATION_IDS \intersect REPLY_IDS)) = 0)
      /\ Cardinality((ELICITATION_IDS \intersect EVENT_IDS)) = 0)
    /\ (\A ro_2004 \in RESPONSE_OP_IDS:
      responseValid[ro_2004] => recordedResponseIndependentOk(ro_2004))

(*
  @type: (() => Bool);
*)
elicitation_invalid_response_rejected ==
  (((\A ro_2062 \in RESPONSE_OP_IDS:
          ~(recordedResponseIndependentOk(ro_2062))
            => ~(responseValid[ro_2062])
              /\ (\A eid_2058 \in ELICITATION_IDS:
                answeredResponseOp[eid_2058] /= ro_2062))
        /\ (\A ro_2084 \in RESPONSE_OP_IDS:
          responseDuplicate[ro_2084]
            => ~(responseValid[ro_2084])
              /\ (\A eid_2080 \in ELICITATION_IDS:
                answeredResponseOp[eid_2080] /= ro_2084)))
      /\ (\A eid_2111 \in ELICITATION_IDS:
        answeredResponseOp[eid_2111] /= "none"
          => (answeredResponseOp[eid_2111] \in RESPONSE_OP_IDS
              /\ responseValid[answeredResponseOp[eid_2111]])
            /\ recordedResponseIndependentOk(answeredResponseOp[eid_2111])))
    /\ (\A eid_2137 \in ELICITATION_IDS:
      firstTerminalState[eid_2137] = "answered"
        => answeredBy[eid_2137] = firstAnsweredBy[eid_2137]
          /\ answeredResponseOp[eid_2137] = firstAnsweredResponseOp[eid_2137])

(*
  @type: ((Str, Str, Str, Str, Str, Int, Str) => Bool);
*)
responseMatchesTarget(eid_242, claimedEid_242, kind_242, domain_242, session_242,
generation_242, actor_242) ==
  (((((claimedEid_242 = eid_242 /\ kind_242 \in RESPONSE_KINDS)
            /\ domain_242 = elicitationDomain[eid_242])
          /\ session_242 = targetSession[eid_242])
        /\ generation_242 = targetGeneration[eid_242])
      /\ actor_242 = responderActor[eid_242])
    /\ targetLive(eid_242)

(*
  @type: (() => Bool);
*)
q_init == init

(*
  @type: ((Str, Str, Str, Str, Str, Str, Int, Str) => Bool);
*)
firstValidAnswerAllowed(eid_277, responseOpId_277, claimedEid_277, kind_277, domain_277,
session_277, generation_277, actor_277) ==
  (state[eid_277] = "pending" /\ responseOpUnused(responseOpId_277))
    /\ responseMatchesTarget(eid_277, claimedEid_277, kind_277, domain_277, session_277,
    generation_277, actor_277)

(*
  @type: ((Str, Str, Str, Str, Str, Str, Int, Str) => Bool);
*)
idempotentResponseRetry(eid_307, responseOpId_307, claimedEid_307, kind_307, domain_307,
session_307, generation_307, actor_307) ==
  (state[eid_307] = "answered" /\ answeredResponseOp[eid_307] = responseOpId_307)
    /\ responseMatchesTarget(eid_307, claimedEid_307, kind_307, domain_307, session_307,
    generation_307, actor_307)

(*
  @type: ((Str, Str, Str, Str, Str, Str, Str, Int, Str) => Bool);
*)
attemptAnswer(eid_1407, responseOpId_1407, claimedEid_1407, kind_1407, endpoint_1407,
domain_1407, session_1407, generation_1407, actor_1407) ==
  (firstValidAnswerAllowed(eid_1407, responseOpId_1407, claimedEid_1407, kind_1407,
      domain_1407, session_1407, generation_1407, actor_1407)
      /\ state' := [ state EXCEPT ![eid_1407] = "answered" ]
      /\ lsn' := (lsn + 1)
      /\ terminalLsn' := [ terminalLsn EXCEPT ![eid_1407] = lsn + 1 ]
      /\ answeredBy' := [ answeredBy EXCEPT ![eid_1407] = endpoint_1407 ]
      /\ responseOpElicitation'
        := [
          responseOpElicitation EXCEPT
            ![responseOpId_1407] = claimedEid_1407
        ]
      /\ responseOpKind'
        := [ responseOpKind EXCEPT ![responseOpId_1407] = kind_1407 ]
      /\ responseOpDomain'
        := [ responseOpDomain EXCEPT ![responseOpId_1407] = domain_1407 ]
      /\ responseOpSession'
        := [ responseOpSession EXCEPT ![responseOpId_1407] = session_1407 ]
      /\ responseOpGeneration'
        := [
          responseOpGeneration EXCEPT
            ![responseOpId_1407] = generation_1407
        ]
      /\ responseOpActor'
        := [ responseOpActor EXCEPT ![responseOpId_1407] = actor_1407 ]
      /\ responseOpEndpoint'
        := [ responseOpEndpoint EXCEPT ![responseOpId_1407] = endpoint_1407 ]
      /\ responseValid' := [ responseValid EXCEPT ![responseOpId_1407] = TRUE ]
      /\ responseDuplicate'
        := [ responseDuplicate EXCEPT ![responseOpId_1407] = FALSE ]
      /\ answeredResponseOp'
        := [ answeredResponseOp EXCEPT ![eid_1407] = responseOpId_1407 ]
      /\ firstTerminalState'
        := (IF terminalLsn[eid_1407] = 0
        THEN [ firstTerminalState EXCEPT ![eid_1407] = "answered" ]
        ELSE firstTerminalState)
      /\ firstAnsweredBy'
        := (IF terminalLsn[eid_1407] = 0
        THEN [ firstAnsweredBy EXCEPT ![eid_1407] = endpoint_1407 ]
        ELSE firstAnsweredBy)
      /\ firstAnsweredResponseOp'
        := (IF terminalLsn[eid_1407] = 0
        THEN [ firstAnsweredResponseOp EXCEPT ![eid_1407] = responseOpId_1407 ]
        ELSE firstAnsweredResponseOp)
      /\ responderActor' := responderActor
      /\ contractKind' := contractKind
      /\ elicitationDomain' := elicitationDomain
      /\ targetSession' := targetSession
      /\ targetGeneration' := targetGeneration
      /\ sessionGeneration' := sessionGeneration)
    \/ (idempotentResponseRetry(eid_1407, responseOpId_1407, claimedEid_1407, kind_1407,
      domain_1407, session_1407, generation_1407, actor_1407)
      /\ state' := state
      /\ lsn' := lsn
      /\ terminalLsn' := terminalLsn
      /\ answeredBy' := answeredBy
      /\ responseOpElicitation'
        := [
          responseOpElicitation EXCEPT
            ![responseOpId_1407] = claimedEid_1407
        ]
      /\ responseOpKind'
        := [ responseOpKind EXCEPT ![responseOpId_1407] = kind_1407 ]
      /\ responseOpDomain'
        := [ responseOpDomain EXCEPT ![responseOpId_1407] = domain_1407 ]
      /\ responseOpSession'
        := [ responseOpSession EXCEPT ![responseOpId_1407] = session_1407 ]
      /\ responseOpGeneration'
        := [
          responseOpGeneration EXCEPT
            ![responseOpId_1407] = generation_1407
        ]
      /\ responseOpActor'
        := [ responseOpActor EXCEPT ![responseOpId_1407] = actor_1407 ]
      /\ responseOpEndpoint'
        := [ responseOpEndpoint EXCEPT ![responseOpId_1407] = endpoint_1407 ]
      /\ responseValid' := [ responseValid EXCEPT ![responseOpId_1407] = TRUE ]
      /\ responseDuplicate'
        := [ responseDuplicate EXCEPT ![responseOpId_1407] = FALSE ]
      /\ answeredResponseOp' := answeredResponseOp
      /\ firstTerminalState' := firstTerminalState
      /\ firstAnsweredBy' := firstAnsweredBy
      /\ firstAnsweredResponseOp' := firstAnsweredResponseOp
      /\ responderActor' := responderActor
      /\ contractKind' := contractKind
      /\ elicitationDomain' := elicitationDomain
      /\ targetSession' := targetSession
      /\ targetGeneration' := targetGeneration
      /\ sessionGeneration' := sessionGeneration)
    \/ (~(firstValidAnswerAllowed(eid_1407, responseOpId_1407, claimedEid_1407, kind_1407,
      domain_1407, session_1407, generation_1407, actor_1407))
      /\ ~(idempotentResponseRetry(eid_1407, responseOpId_1407, claimedEid_1407,
      kind_1407, domain_1407, session_1407, generation_1407, actor_1407))
      /\ responseOpUnused(responseOpId_1407)
      /\ state' := state
      /\ lsn' := lsn
      /\ terminalLsn' := terminalLsn
      /\ answeredBy' := answeredBy
      /\ responseOpElicitation'
        := [
          responseOpElicitation EXCEPT
            ![responseOpId_1407] = claimedEid_1407
        ]
      /\ responseOpKind'
        := [ responseOpKind EXCEPT ![responseOpId_1407] = kind_1407 ]
      /\ responseOpDomain'
        := [ responseOpDomain EXCEPT ![responseOpId_1407] = domain_1407 ]
      /\ responseOpSession'
        := [ responseOpSession EXCEPT ![responseOpId_1407] = session_1407 ]
      /\ responseOpGeneration'
        := [
          responseOpGeneration EXCEPT
            ![responseOpId_1407] = generation_1407
        ]
      /\ responseOpActor'
        := [ responseOpActor EXCEPT ![responseOpId_1407] = actor_1407 ]
      /\ responseOpEndpoint'
        := [ responseOpEndpoint EXCEPT ![responseOpId_1407] = endpoint_1407 ]
      /\ responseValid' := [ responseValid EXCEPT ![responseOpId_1407] = FALSE ]
      /\ responseDuplicate'
        := [
          responseDuplicate EXCEPT
            ![responseOpId_1407] =
              duplicateAttempt(responseOpId_1407, claimedEid_1407)
        ]
      /\ answeredResponseOp' := answeredResponseOp
      /\ firstTerminalState' := firstTerminalState
      /\ firstAnsweredBy' := firstAnsweredBy
      /\ firstAnsweredResponseOp' := firstAnsweredResponseOp
      /\ responderActor' := responderActor
      /\ contractKind' := contractKind
      /\ elicitationDomain' := elicitationDomain
      /\ targetSession' := targetSession
      /\ targetGeneration' := targetGeneration
      /\ sessionGeneration' := sessionGeneration)
    \/ (~(idempotentResponseRetry(eid_1407, responseOpId_1407, claimedEid_1407, kind_1407,
      domain_1407, session_1407, generation_1407, actor_1407))
      /\ ~(responseOpUnused(responseOpId_1407))
      /\ state' := state
      /\ lsn' := lsn
      /\ terminalLsn' := terminalLsn
      /\ answeredBy' := answeredBy
      /\ responseOpElicitation' := responseOpElicitation
      /\ responseOpKind' := responseOpKind
      /\ responseOpDomain' := responseOpDomain
      /\ responseOpSession' := responseOpSession
      /\ responseOpGeneration' := responseOpGeneration
      /\ responseOpActor' := responseOpActor
      /\ responseOpEndpoint' := responseOpEndpoint
      /\ responseValid' := responseValid
      /\ responseDuplicate' := responseDuplicate
      /\ answeredResponseOp' := answeredResponseOp
      /\ firstTerminalState' := firstTerminalState
      /\ firstAnsweredBy' := firstAnsweredBy
      /\ firstAnsweredResponseOp' := firstAnsweredResponseOp
      /\ responderActor' := responderActor
      /\ contractKind' := contractKind
      /\ elicitationDomain' := elicitationDomain
      /\ targetSession' := targetSession
      /\ targetGeneration' := targetGeneration
      /\ sessionGeneration' := sessionGeneration)

(*
  @type: (() => Bool);
*)
attemptAnyAnswer ==
  \E responseOpId \in RESPONSE_OP_IDS:
    \E claimedEid \in CLAIMED_ELICITATION_IDS:
      \E kind \in ATTEMPTED_RESPONSE_KINDS:
        \E endpoint \in ENDPOINTS:
          \E domain \in RESPONSE_DOMAINS:
            \E session \in SESSIONS:
              \E generation \in GENERATIONS:
                \E actor \in SUBMITTING_ACTORS:
                  attemptAnswer("e1", responseOpId, claimedEid, kind, endpoint, domain,
                  session, generation, actor)

(*
  @type: (() => Bool);
*)
step ==
  makePending("e1")
    \/ advanceSessionGeneration("e1")
    \/ attemptAnyAnswer
    \/ lateAnswer("e1", "ro2")
    \/ (\E terminal \in NON_ANSWER_TERMINAL: commitTerminal("e1", terminal))
    \/ goStale("e1")

(*
  @type: (() => Bool);
*)
q_step == step

================================================================================
