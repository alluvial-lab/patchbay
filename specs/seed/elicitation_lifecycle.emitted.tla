# Usage statistics is OFF. We care about your privacy.
# If you want to help our project, consider enabling statistics with config --enable-stats=true.

Output directory: /home/agent/projects/patchbay/_apalache-out/server/2026-07-08T16-47-16_15800063716406170448
# APALACHE version: 0.56.1 | build: 70cdaf4                       I@16:47:16.554
Starting checker server on port 8822...                           I@16:47:16.563
The Apalache server is running on port 8822. Press Ctrl-C to stop.
PASS #0: SanyParser                                               I@16:47:19.395
------------------------- MODULE elicitation_lifecycle -------------------------

EXTENDS Integers, Sequences, FiniteSets, TLC, Apalache, Variants

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

(*
  @type: (() => Set(Str));
*)
NON_TERMINAL == { "opened", "pending" }

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

(*
  @type: (() => Set(Str));
*)
ELICITATION_IDS == {"e1"}

VARIABLE
  (*
    @type: (Str -> Bool);
  *)
  responseValid

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

(*
  @type: ((Str) => Bool);
*)
goStale(eid_1664) ==
  (state[eid_1664] = "pending"
      /\ targetGeneration[eid_1664] < 2
      /\ state' := [ state EXCEPT ![eid_1664] = "stale" ]
      /\ lsn' := (lsn + 1)
      /\ terminalLsn' := [ terminalLsn EXCEPT ![eid_1664] = lsn + 1 ]
      /\ sessionGeneration'
        := [
          sessionGeneration EXCEPT
            ![targetSession[eid_1664]] = targetGeneration[eid_1664] + 1
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
        := (IF terminalLsn[eid_1664] = 0
        THEN [ firstTerminalState EXCEPT ![eid_1664] = "stale" ]
        ELSE firstTerminalState)
      /\ firstAnsweredBy' := firstAnsweredBy
      /\ firstAnsweredResponseOp' := firstAnsweredResponseOp)
    \/ (state[eid_1664] \in TERMINAL
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
  @type: (() => Bool);
*)
elicitation_correlation_typed ==
  (((Cardinality((ELICITATION_IDS \intersect COMMAND_IDS)) = 0
          /\ Cardinality((ELICITATION_IDS \intersect MESSAGE_IDS)) = 0)
        /\ Cardinality((ELICITATION_IDS \intersect REPLY_IDS)) = 0)
      /\ Cardinality((ELICITATION_IDS \intersect EVENT_IDS)) = 0)
    /\ (\A ro_1881 \in RESPONSE_OP_IDS:
      responseValid[ro_1881]
        => (IF responseOpElicitation[ro_1881] \in ELICITATION_IDS
        THEN ((((((responseOpKind[ro_1881] \in RESPONSE_KINDS
                      /\ responseOpDomain[ro_1881]
                        = elicitationDomain[responseOpElicitation[ro_1881]])
                    /\ responseOpSession[ro_1881]
                      = targetSession[responseOpElicitation[ro_1881]])
                  /\ responseOpGeneration[ro_1881]
                    = targetGeneration[responseOpElicitation[ro_1881]])
                /\ responseOpGeneration[ro_1881]
                  = sessionGeneration[responseOpSession[ro_1881]])
              /\ responseOpActor[ro_1881]
                = responderActor[responseOpElicitation[ro_1881]])
            /\ responseOpEndpoint[ro_1881] \in ENDPOINTS)
          /\ answeredResponseOp[responseOpElicitation[ro_1881]] = ro_1881
        ELSE FALSE))

(*
  @type: (() => Bool);
*)
elicitation_timeout_neither_success_nor_denial ==
  \A eid_1916 \in ELICITATION_IDS:
    state[eid_1916] = "expired"
      => ((answeredBy[eid_1916] = "none"
            /\ answeredResponseOp[eid_1916] = "none")
          /\ state[eid_1916] /= "answered")
        /\ state[eid_1916] /= "declined"

(*
  @type: ((Str) => Bool);
*)
targetLive(eid_191) ==
  targetGeneration[eid_191] = sessionGeneration[targetSession[eid_191]]

(*
  @type: (() => Bool);
*)
elicitation_invalid_response_rejected ==
  ((\A ro_1935 \in RESPONSE_OP_IDS:
        ~(responseValid[ro_1935])
          => (\A eid_1932 \in ELICITATION_IDS:
            answeredResponseOp[eid_1932] /= ro_1935))
      /\ (\A ro_1957 \in RESPONSE_OP_IDS:
        responseDuplicate[ro_1957]
          => ~(responseValid[ro_1957])
            /\ (\A eid_1953 \in ELICITATION_IDS:
              answeredResponseOp[eid_1953] /= ro_1957)))
    /\ (\A eid_1979 \in ELICITATION_IDS:
      answeredResponseOp[eid_1979] /= "none"
        => answeredResponseOp[eid_1979] \in RESPONSE_OP_IDS
          /\ responseValid[answeredResponseOp[eid_1979]])

(*
  @type: (() => Bool);
*)
elicitation_stale_target_inert ==
  [](targetGeneration["e1"] < sessionGeneration[targetSession["e1"]]
    => ((state["e1"] = "stale" /\ firstTerminalState["e1"] = "stale")
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
committedResponseOp(responseOpId_306) ==
  \E eid_304 \in ELICITATION_IDS: answeredResponseOp[eid_304] = responseOpId_306

(*
  @type: (() => Set(Str));
*)
SUBMITTING_ACTORS == ACTORS \union {"mallory"}

(*
  @type: ((Str, Str) => Bool);
*)
duplicateAttempt(responseOpId_326, claimedEid_326) ==
  IF claimedEid_326 \in ELICITATION_IDS
  THEN state[claimedEid_326] = "answered"
    /\ answeredResponseOp[claimedEid_326] /= responseOpId_326
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
    /\ responseOpElicitation = [ ro_391 \in RESPONSE_OP_IDS |-> "none" ]
    /\ responseOpKind = [ ro_398 \in RESPONSE_OP_IDS |-> "none" ]
    /\ responseOpDomain = [ ro_405 \in RESPONSE_OP_IDS |-> "domain-main" ]
    /\ responseOpSession = [ ro_412 \in RESPONSE_OP_IDS |-> "s1" ]
    /\ responseOpGeneration = [ ro_419 \in RESPONSE_OP_IDS |-> 0 ]
    /\ responseOpActor = [ ro_426 \in RESPONSE_OP_IDS |-> "alice" ]
    /\ responseOpEndpoint = [ ro_433 \in RESPONSE_OP_IDS |-> "none" ]
    /\ responseValid = [ ro_440 \in RESPONSE_OP_IDS |-> FALSE ]
    /\ responseDuplicate = [ ro_447 \in RESPONSE_OP_IDS |-> FALSE ]
    /\ answeredResponseOp = SetAsFun({<<"e1", "none">>})
    /\ firstTerminalState = SetAsFun({<<"e1", "none">>})
    /\ firstAnsweredBy = SetAsFun({<<"e1", "none">>})
    /\ firstAnsweredResponseOp = SetAsFun({<<"e1", "none">>})

(*
  @type: ((Str, Str, Str, Str, Int) => Bool);
*)
openElicitation(eid_577, actor_577, contract_577, session_577, generation_577) ==
  state[eid_577] = "opened"
    /\ actor_577 = responderActor[eid_577]
    /\ contract_577 = contractKind[eid_577]
    /\ session_577 = targetSession[eid_577]
    /\ generation_577 = targetGeneration[eid_577]
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
makePending(eid_660) ==
  state[eid_660] = "opened"
    /\ state' := [ state EXCEPT ![eid_660] = "pending" ]
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
commitTerminal(eid_841, terminalState_841) ==
  (state[eid_841] = "pending"
      /\ terminalState_841 \in TERMINAL
      /\ state' := [ state EXCEPT ![eid_841] = terminalState_841 ]
      /\ lsn' := (lsn + 1)
      /\ terminalLsn' := [ terminalLsn EXCEPT ![eid_841] = lsn + 1 ]
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
        := (IF terminalLsn[eid_841] = 0
        THEN [ firstTerminalState EXCEPT ![eid_841] = terminalState_841 ]
        ELSE firstTerminalState)
      /\ firstAnsweredBy' := firstAnsweredBy
      /\ firstAnsweredResponseOp' := firstAnsweredResponseOp)
    \/ (state[eid_841] \in TERMINAL
      /\ terminalState_841 \in TERMINAL
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
lateAnswer(eid_1446, responseOpId_1446) ==
  state[eid_1446] \in TERMINAL
    /\ ~(committedResponseOp(responseOpId_1446))
    /\ responseOpElicitation[responseOpId_1446] = "none"
    /\ state' := state
    /\ lsn' := lsn
    /\ terminalLsn' := terminalLsn
    /\ answeredBy' := answeredBy
    /\ responseOpElicitation'
      := [ responseOpElicitation EXCEPT ![responseOpId_1446] = eid_1446 ]
    /\ responseOpKind'
      := [ responseOpKind EXCEPT ![responseOpId_1446] = "elicitation-response" ]
    /\ responseOpDomain'
      := [
        responseOpDomain EXCEPT
          ![responseOpId_1446] = elicitationDomain[eid_1446]
      ]
    /\ responseOpSession'
      := [
        responseOpSession EXCEPT
          ![responseOpId_1446] = targetSession[eid_1446]
      ]
    /\ responseOpGeneration'
      := [
        responseOpGeneration EXCEPT
          ![responseOpId_1446] = targetGeneration[eid_1446]
      ]
    /\ responseOpActor'
      := [
        responseOpActor EXCEPT
          ![responseOpId_1446] = responderActor[eid_1446]
      ]
    /\ responseOpEndpoint'
      := [ responseOpEndpoint EXCEPT ![responseOpId_1446] = "ep-b" ]
    /\ responseValid' := [ responseValid EXCEPT ![responseOpId_1446] = FALSE ]
    /\ responseDuplicate'
      := [
        responseDuplicate EXCEPT
          ![responseOpId_1446] = state[eid_1446] = "answered"
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
decline(eid_1452) == commitTerminal(eid_1452, "declined")

(*
  @type: ((Str) => Bool);
*)
expire(eid_1458) == commitTerminal(eid_1458, "expired")

(*
  @type: ((Str) => Bool);
*)
cancel(eid_1464) == commitTerminal(eid_1464, "cancelled")

(*
  @type: ((Str) => Bool);
*)
withdraw(eid_1470) == commitTerminal(eid_1470, "withdrawn")

(*
  @type: ((Str) => Bool);
*)
supersede(eid_1476) == commitTerminal(eid_1476, "superseded")

(*
  @type: ((Str, Str, Str, Str, Str, Int, Str) => Bool);
*)
responseMatchesTarget(eid_234, claimedEid_234, kind_234, domain_234, session_234,
generation_234, actor_234) ==
  (((((claimedEid_234 = eid_234 /\ kind_234 \in RESPONSE_KINDS)
            /\ domain_234 = elicitationDomain[eid_234])
          /\ session_234 = targetSession[eid_234])
        /\ generation_234 = targetGeneration[eid_234])
      /\ actor_234 = responderActor[eid_234])
    /\ targetLive(eid_234)

(*
  @type: (() => Bool);
*)
q_init == init

(*
  @type: ((Str, Str, Str, Str, Str, Str, Int, Str) => Bool);
*)
firstValidAnswerAllowed(eid_264, responseOpId_264, claimedEid_264, kind_264, domain_264,
session_264, generation_264, actor_264) ==
  (state[eid_264] = "pending"
      /\ responseOpElicitation[responseOpId_264] = "none")
    /\ responseMatchesTarget(eid_264, claimedEid_264, kind_264, domain_264, session_264,
    generation_264, actor_264)

(*
  @type: ((Str, Str, Str, Str, Str, Str, Int, Str) => Bool);
*)
idempotentResponseRetry(eid_294, responseOpId_294, claimedEid_294, kind_294, domain_294,
session_294, generation_294, actor_294) ==
  (state[eid_294] = "answered" /\ answeredResponseOp[eid_294] = responseOpId_294)
    /\ responseMatchesTarget(eid_294, claimedEid_294, kind_294, domain_294, session_294,
    generation_294, actor_294)

(*
  @type: ((Str, Str, Str, Str, Str, Str, Str, Int, Str) => Bool);
*)
attemptAnswer(eid_1320, responseOpId_1320, claimedEid_1320, kind_1320, endpoint_1320,
domain_1320, session_1320, generation_1320, actor_1320) ==
  (firstValidAnswerAllowed(eid_1320, responseOpId_1320, claimedEid_1320, kind_1320,
      domain_1320, session_1320, generation_1320, actor_1320)
      /\ state' := [ state EXCEPT ![eid_1320] = "answered" ]
      /\ lsn' := (lsn + 1)
      /\ terminalLsn' := [ terminalLsn EXCEPT ![eid_1320] = lsn + 1 ]
      /\ answeredBy' := [ answeredBy EXCEPT ![eid_1320] = endpoint_1320 ]
      /\ responseOpElicitation'
        := [
          responseOpElicitation EXCEPT
            ![responseOpId_1320] = claimedEid_1320
        ]
      /\ responseOpKind'
        := [ responseOpKind EXCEPT ![responseOpId_1320] = kind_1320 ]
      /\ responseOpDomain'
        := [ responseOpDomain EXCEPT ![responseOpId_1320] = domain_1320 ]
      /\ responseOpSession'
        := [ responseOpSession EXCEPT ![responseOpId_1320] = session_1320 ]
      /\ responseOpGeneration'
        := [
          responseOpGeneration EXCEPT
            ![responseOpId_1320] = generation_1320
        ]
      /\ responseOpActor'
        := [ responseOpActor EXCEPT ![responseOpId_1320] = actor_1320 ]
      /\ responseOpEndpoint'
        := [ responseOpEndpoint EXCEPT ![responseOpId_1320] = endpoint_1320 ]
      /\ responseValid' := [ responseValid EXCEPT ![responseOpId_1320] = TRUE ]
      /\ responseDuplicate'
        := [ responseDuplicate EXCEPT ![responseOpId_1320] = FALSE ]
      /\ answeredResponseOp'
        := [ answeredResponseOp EXCEPT ![eid_1320] = responseOpId_1320 ]
      /\ firstTerminalState'
        := (IF terminalLsn[eid_1320] = 0
        THEN [ firstTerminalState EXCEPT ![eid_1320] = "answered" ]
        ELSE firstTerminalState)
      /\ firstAnsweredBy'
        := (IF terminalLsn[eid_1320] = 0
        THEN [ firstAnsweredBy EXCEPT ![eid_1320] = endpoint_1320 ]
        ELSE firstAnsweredBy)
      /\ firstAnsweredResponseOp'
        := (IF terminalLsn[eid_1320] = 0
        THEN [ firstAnsweredResponseOp EXCEPT ![eid_1320] = responseOpId_1320 ]
        ELSE firstAnsweredResponseOp)
      /\ responderActor' := responderActor
      /\ contractKind' := contractKind
      /\ elicitationDomain' := elicitationDomain
      /\ targetSession' := targetSession
      /\ targetGeneration' := targetGeneration
      /\ sessionGeneration' := sessionGeneration)
    \/ (idempotentResponseRetry(eid_1320, responseOpId_1320, claimedEid_1320, kind_1320,
      domain_1320, session_1320, generation_1320, actor_1320)
      /\ state' := state
      /\ lsn' := lsn
      /\ terminalLsn' := terminalLsn
      /\ answeredBy' := answeredBy
      /\ responseOpElicitation'
        := [
          responseOpElicitation EXCEPT
            ![responseOpId_1320] = claimedEid_1320
        ]
      /\ responseOpKind'
        := [ responseOpKind EXCEPT ![responseOpId_1320] = kind_1320 ]
      /\ responseOpDomain'
        := [ responseOpDomain EXCEPT ![responseOpId_1320] = domain_1320 ]
      /\ responseOpSession'
        := [ responseOpSession EXCEPT ![responseOpId_1320] = session_1320 ]
      /\ responseOpGeneration'
        := [
          responseOpGeneration EXCEPT
            ![responseOpId_1320] = generation_1320
        ]
      /\ responseOpActor'
        := [ responseOpActor EXCEPT ![responseOpId_1320] = actor_1320 ]
      /\ responseOpEndpoint'
        := [ responseOpEndpoint EXCEPT ![responseOpId_1320] = endpoint_1320 ]
      /\ responseValid' := [ responseValid EXCEPT ![responseOpId_1320] = TRUE ]
      /\ responseDuplicate'
        := [ responseDuplicate EXCEPT ![responseOpId_1320] = FALSE ]
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
    \/ (committedResponseOp(responseOpId_1320)
      /\ ~(idempotentResponseRetry(eid_1320, responseOpId_1320, claimedEid_1320,
      kind_1320, domain_1320, session_1320, generation_1320, actor_1320))
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
    \/ (~(firstValidAnswerAllowed(eid_1320, responseOpId_1320, claimedEid_1320, kind_1320,
      domain_1320, session_1320, generation_1320, actor_1320))
      /\ ~(idempotentResponseRetry(eid_1320, responseOpId_1320, claimedEid_1320,
      kind_1320, domain_1320, session_1320, generation_1320, actor_1320))
      /\ ~(committedResponseOp(responseOpId_1320))
      /\ responseOpElicitation[responseOpId_1320] = "none"
      /\ state' := state
      /\ lsn' := lsn
      /\ terminalLsn' := terminalLsn
      /\ answeredBy' := answeredBy
      /\ responseOpElicitation'
        := [
          responseOpElicitation EXCEPT
            ![responseOpId_1320] = claimedEid_1320
        ]
      /\ responseOpKind'
        := [ responseOpKind EXCEPT ![responseOpId_1320] = kind_1320 ]
      /\ responseOpDomain'
        := [ responseOpDomain EXCEPT ![responseOpId_1320] = domain_1320 ]
      /\ responseOpSession'
        := [ responseOpSession EXCEPT ![responseOpId_1320] = session_1320 ]
      /\ responseOpGeneration'
        := [
          responseOpGeneration EXCEPT
            ![responseOpId_1320] = generation_1320
        ]
      /\ responseOpActor'
        := [ responseOpActor EXCEPT ![responseOpId_1320] = actor_1320 ]
      /\ responseOpEndpoint'
        := [ responseOpEndpoint EXCEPT ![responseOpId_1320] = endpoint_1320 ]
      /\ responseValid' := [ responseValid EXCEPT ![responseOpId_1320] = FALSE ]
      /\ responseDuplicate'
        := [
          responseDuplicate EXCEPT
            ![responseOpId_1320] =
              duplicateAttempt(responseOpId_1320, claimedEid_1320)
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

(*
  @type: (() => Bool);
*)
step ==
  makePending("e1")
    \/ attemptAnswer("e1", "ro1", "e1", "approval-response", "ep-a", "domain-main",
    "s1", 0, "alice")
    \/ attemptAnswer("e1", "ro2", "e1", "elicitation-response", "ep-b", "domain-main",
    "s1", 0, "alice")
    \/ attemptAnswer("e1", "ro2", "c1", "elicitation-response", "ep-b", "domain-forged",
    "s2", 1, "mallory")
    \/ lateAnswer("e1", "ro2")
    \/ (\E terminal \in TERMINAL: commitTerminal("e1", terminal))
    \/ goStale("e1")

(*
  @type: (() => Bool);
*)
q_step == step

================================================================================
