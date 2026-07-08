# Usage statistics is OFF. We care about your privacy.
# If you want to help our project, consider enabling statistics with config --enable-stats=true.

Output directory: /home/agent/projects/patchbay/specs/seed/_apalache-out/server/2026-07-08T14-14-35_14833624217107003331
# APALACHE version: 0.56.1 | build: 70cdaf4                       I@14:14:35.477
Starting checker server on port 8822...                           I@14:14:35.488
The Apalache server is running on port 8822. Press Ctrl-C to stop.
PASS #0: SanyParser                                               I@14:14:38.251
--------------------------- MODULE command_lifecycle ---------------------------

EXTENDS Integers, Sequences, FiniteSets, TLC, Apalache, Variants

(*
  @type: (() => Set(Str));
*)
NON_TERMINAL == { "accepted", "delivered", "running" }

(*
  @type: (() => Set(Str));
*)
CMD_IDS == { "c1", "c2", "c3" }

(*
  @type: (() => Set(Str));
*)
IDEMPOTENCY_KEYS == { "k1", "k2", "k3" }

VARIABLE
  (*
    @type: (Str -> Str);
  *)
  state

VARIABLE
  (*
    @type: (Str -> Str);
  *)
  idemKey

VARIABLE
  (*
    @type: Set(Str);
  *)
  appliedKeys

(*
  @type: (() => Set(Str));
*)
TERMINAL ==
  { "completed", "rejected", "failed", "expired", "cancelled", "superseded" }

VARIABLE
  (*
    @type: (Str -> Int);
  *)
  applyCount

VARIABLE
  (*
    @type: Int;
  *)
  lsn

VARIABLE
  (*
    @type: (Str -> Int);
  *)
  terminalLsn

(*
  @type: ((Str, Str) => Bool);
*)
allowedTransition(from_66, candidateState_66) ==
  IF from_66 = "accepted"
  THEN candidateState_66
    \in { "delivered",
      "rejected",
      "failed",
      "expired",
      "cancelled",
      "superseded" }
  ELSE IF from_66 = "delivered"
  THEN candidateState_66
    \in { "running",
      "completed",
      "rejected",
      "failed",
      "expired",
      "cancelled",
      "superseded" }
  ELSE IF from_66 = "running"
  THEN candidateState_66
    \in { "completed", "failed", "expired", "cancelled", "superseded" }
  ELSE FALSE

(*
  @type: (() => Bool);
*)
init ==
  state
      = SetAsFun({ <<"c1", "accepted">>,
        <<"c2", "accepted">>,
        <<"c3", "accepted">> })
    /\ idemKey = SetAsFun({ <<"c1", "k1">>, <<"c2", "k2">>, <<"c3", "k3">> })
    /\ appliedKeys = { "k1", "k2", "k3" }
    /\ applyCount = SetAsFun({ <<"k1", 1>>, <<"k2", 1>>, <<"k3", 1>> })
    /\ lsn = 3
    /\ terminalLsn = SetAsFun({ <<"c1", 0>>, <<"c2", 0>>, <<"c3", 0>> })

(*
  @type: ((Str, Str) => Bool);
*)
lateTerminalCandidate(cmd_264, candidate_264) ==
  state[cmd_264] \in TERMINAL
    /\ candidate_264 \in TERMINAL
    /\ state' := state
    /\ lsn' := lsn
    /\ terminalLsn' := terminalLsn
    /\ idemKey' := idemKey
    /\ appliedKeys' := appliedKeys
    /\ applyCount' := applyCount

(*
  @type: ((Str) => Bool);
*)
receive(key_319) ==
  (key_319 \in appliedKeys
      /\ appliedKeys' := appliedKeys
      /\ applyCount' := applyCount
      /\ state' := state
      /\ lsn' := lsn
      /\ terminalLsn' := terminalLsn
      /\ idemKey' := idemKey)
    \/ (~(key_319 \in appliedKeys)
      /\ appliedKeys' := (appliedKeys \union {key_319})
      /\ applyCount' := [ applyCount EXCEPT ![key_319] = 1 ]
      /\ state' := state
      /\ lsn' := lsn
      /\ terminalLsn' := terminalLsn
      /\ idemKey' := idemKey)

(*
  @type: ((Str, Str) => Bool);
*)
retry(cmd_350, key_350) ==
  idemKey[cmd_350] = key_350
    /\ key_350 \in appliedKeys
    /\ state' := state
    /\ appliedKeys' := appliedKeys
    /\ applyCount' := applyCount
    /\ lsn' := lsn
    /\ terminalLsn' := terminalLsn
    /\ idemKey' := idemKey

(*
  @type: (() => Bool);
*)
completed_reachable == \E cmd_416 \in CMD_IDS: state[cmd_416] = "completed"

(*
  @type: (() => Bool);
*)
command_durability == \A c_425 \in CMD_IDS: c_425 \in DOMAIN state

(*
  @type: (() => Bool);
*)
terminal_finality ==
  [](\A cmd_444 \in CMD_IDS:
    state[cmd_444] \in TERMINAL => state[cmd_444]' = state[cmd_444])

(*
  @type: (() => Bool);
*)
pre_append_terminal_choice ==
  [](\A cmd_469 \in CMD_IDS:
    terminalLsn[cmd_469] = 0 /\ state[cmd_469]' \in TERMINAL
      => terminalLsn[cmd_469]' > 0)

(*
  @type: (() => Bool);
*)
lsn_determines_terminal_winner ==
  [](\A cmd_486 \in CMD_IDS:
    state[cmd_486] \in TERMINAL => terminalLsn[cmd_486] > 0)

(*
  @type: (() => Bool);
*)
boundary_dedup == \A k_497 \in IDEMPOTENCY_KEYS: applyCount[k_497] <= 1

(*
  @type: (() => Bool);
*)
retry_reuses_id_and_key ==
  [](\A cmd_510 \in CMD_IDS: idemKey[cmd_510]' = idemKey[cmd_510])

(*
  @type: (() => Bool);
*)
retry_after_terminal_returns_existing ==
  [](\A cmd_530 \in CMD_IDS:
    state[cmd_530] \in TERMINAL => state[cmd_530]' = state[cmd_530])

(*
  @type: (() => Bool);
*)
no_accepted_to_completed ==
  [](\A cmd_556 \in CMD_IDS:
    state[cmd_556] /= "completed" /\ state[cmd_556]' = "completed"
      => state[cmd_556] \in { "delivered", "running" })

(*
  @type: ((Str, Str) => Bool);
*)
commitTerminal(cmd_192, candidate_192) ==
  state[cmd_192] \in NON_TERMINAL
    /\ candidate_192 \in TERMINAL
    /\ allowedTransition(state[cmd_192], candidate_192)
    /\ state' := [ state EXCEPT ![cmd_192] = candidate_192 ]
    /\ lsn' := (lsn + 1)
    /\ terminalLsn' := [ terminalLsn EXCEPT ![cmd_192] = lsn + 1 ]
    /\ idemKey' := idemKey
    /\ appliedKeys' := appliedKeys
    /\ applyCount' := applyCount

(*
  @type: ((Str, Str) => Bool);
*)
advance(cmd_233, candidate_233) ==
  state[cmd_233] \in NON_TERMINAL
    /\ candidate_233 \in NON_TERMINAL
    /\ allowedTransition(state[cmd_233], candidate_233)
    /\ state' := [ state EXCEPT ![cmd_233] = candidate_233 ]
    /\ lsn' := (lsn + 1)
    /\ terminalLsn' := terminalLsn
    /\ idemKey' := idemKey
    /\ appliedKeys' := appliedKeys
    /\ applyCount' := applyCount

(*
  @type: (() => Bool);
*)
completeViaAdvanceWitness ==
  (state["c1"] = "accepted" /\ advance("c1", "delivered"))
    \/ (state["c1"] = "delivered" /\ commitTerminal("c1", "completed"))
    \/ (state["c1"] = "completed" /\ lateTerminalCandidate("c1", "failed"))

(*
  @type: (() => Bool);
*)
step ==
  (\E cand \in TERMINAL: commitTerminal("c1", cand))
    \/ (advance("c1", "delivered") \/ advance("c1", "running"))
    \/ (\E cand \in TERMINAL: lateTerminalCandidate("c1", cand))
    \/ retry("c1", "k1")
    \/ receive("k1")

(*
  @type: (() => Bool);
*)
q_init == init

(*
  @type: (() => Bool);
*)
q_step == step

================================================================================
