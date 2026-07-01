// GENERATED ARTIFACT — do not hand-edit. Regenerate via:
//   quint compile command_lifecycle.qnt --target tlaplus
// Source: command_lifecycle.qnt (the single source of truth)
// This is an inspection artifact, NOT an independent re-check lane (see feature-formal-model-seed Q4).

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
  @type: (() => Set(Str));
*)
TERMINAL ==
  { "completed", "rejected", "failed", "expired", "cancelled", "superseded" }

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
commitTerminal(cmd_143, candidate_143) ==
  state[cmd_143] \in NON_TERMINAL
    /\ candidate_143 \in TERMINAL
    /\ state' := [ state EXCEPT ![cmd_143] = candidate_143 ]
    /\ lsn' := (lsn + 1)
    /\ terminalLsn' := [ terminalLsn EXCEPT ![cmd_143] = lsn + 1 ]
    /\ idemKey' := idemKey
    /\ appliedKeys' := appliedKeys
    /\ applyCount' := applyCount

(*
  @type: ((Str, Str) => Bool);
*)
lateTerminalCandidate(cmd_174, candidate_174) ==
  state[cmd_174] \in TERMINAL
    /\ candidate_174 \in TERMINAL
    /\ state' := state
    /\ lsn' := lsn
    /\ terminalLsn' := terminalLsn
    /\ idemKey' := idemKey
    /\ appliedKeys' := appliedKeys
    /\ applyCount' := applyCount

(*
  @type: ((Str) => Bool);
*)
receive(key_229) ==
  (key_229 \in appliedKeys
      /\ appliedKeys' := appliedKeys
      /\ applyCount' := applyCount
      /\ state' := state
      /\ lsn' := lsn
      /\ terminalLsn' := terminalLsn
      /\ idemKey' := idemKey)
    \/ (~(key_229 \in appliedKeys)
      /\ appliedKeys' := (appliedKeys \union {key_229})
      /\ applyCount' := [ applyCount EXCEPT ![key_229] = 1 ]
      /\ state' := state
      /\ lsn' := lsn
      /\ terminalLsn' := terminalLsn
      /\ idemKey' := idemKey)

(*
  @type: ((Str, Str) => Bool);
*)
retry(cmd_260, key_260) ==
  idemKey[cmd_260] = key_260
    /\ key_260 \in appliedKeys
    /\ state' := state
    /\ appliedKeys' := appliedKeys
    /\ applyCount' := applyCount
    /\ lsn' := lsn
    /\ terminalLsn' := terminalLsn
    /\ idemKey' := idemKey

(*
  @type: (() => Bool);
*)
command_durability == \A c_309 \in CMD_IDS: c_309 \in DOMAIN state

(*
  @type: (() => Bool);
*)
terminal_finality ==
  [](\A cmd_328 \in CMD_IDS:
    state[cmd_328] \in TERMINAL => state[cmd_328]' = state[cmd_328])

(*
  @type: (() => Bool);
*)
pre_append_terminal_choice ==
  [](\A cmd_353 \in CMD_IDS:
    terminalLsn[cmd_353] = 0 /\ state[cmd_353]' \in TERMINAL
      => terminalLsn[cmd_353]' > 0)

(*
  @type: (() => Bool);
*)
lsn_determines_terminal_winner ==
  [](\A cmd_370 \in CMD_IDS:
    state[cmd_370] \in TERMINAL => terminalLsn[cmd_370] > 0)

(*
  @type: (() => Bool);
*)
boundary_dedup == \A k_381 \in IDEMPOTENCY_KEYS: applyCount[k_381] <= 1

(*
  @type: (() => Bool);
*)
retry_reuses_id_and_key ==
  [](\A cmd_394 \in CMD_IDS: idemKey[cmd_394]' = idemKey[cmd_394])

(*
  @type: (() => Bool);
*)
retry_after_terminal_returns_existing ==
  [](\A cmd_414 \in CMD_IDS:
    state[cmd_414] \in TERMINAL => state[cmd_414]' = state[cmd_414])

(*
  @type: (() => Bool);
*)
step ==
  (\E cmd \in CMD_IDS: \E cand \in TERMINAL: commitTerminal(cmd, cand))
    \/ (\E cmd \in CMD_IDS:
      \E cand \in TERMINAL: lateTerminalCandidate(cmd, cand))
    \/ (\E cmd \in CMD_IDS: \E key \in IDEMPOTENCY_KEYS: retry(cmd, key))
    \/ (\E key \in IDEMPOTENCY_KEYS: receive(key))

(*
  @type: (() => Bool);
*)
q_init == init

(*
  @type: (() => Bool);
*)
q_step == step

================================================================================
