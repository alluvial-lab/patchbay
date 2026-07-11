---------------------------- MODULE counterexample ----------------------------

EXTENDS command_lifecycle

(* Constant initialization state *)
ConstInit == TRUE

(* Initial state [_transition(0)] *)
State0 ==
  appliedKeys = { "k1", "k2", "k3" }
    /\ applyCount = SetAsFun({ <<"k1", 1>>, <<"k2", 1>>, <<"k3", 1>> })
    /\ idemKey = SetAsFun({ <<"c1", "k1">>, <<"c2", "k2">>, <<"c3", "k3">> })
    /\ lsn = 3
    /\ state
      = SetAsFun({ <<"c1", "accepted">>,
        <<"c2", "accepted">>,
        <<"c3", "accepted">> })
    /\ terminalLsn = SetAsFun({ <<"c1", 0>>, <<"c2", 0>>, <<"c3", 0>> })

(* State1 [_transition(3)] *)
State1 ==
  appliedKeys = { "k1", "k2", "k3" }
    /\ applyCount = SetAsFun({ <<"k1", 2>>, <<"k2", 1>>, <<"k3", 1>> })
    /\ idemKey = SetAsFun({ <<"c1", "k1">>, <<"c2", "k2">>, <<"c3", "k3">> })
    /\ lsn = 3
    /\ state
      = SetAsFun({ <<"c1", "accepted">>,
        <<"c2", "accepted">>,
        <<"c3", "accepted">> })
    /\ terminalLsn = SetAsFun({ <<"c1", 0>>, <<"c2", 0>>, <<"c3", 0>> })

(* The following formula holds true in the last state and violates the invariant *)
InvariantViolation ==
  Skolem((\E k_453_2 \in { "k1", "k2", "k3" }: applyCount[k_453_2] > 1))

================================================================================
(* Created by Apalache on Sat Jul 11 11:50:57 MDT 2026 *)
(* https://github.com/apalache-mc/apalache *)
