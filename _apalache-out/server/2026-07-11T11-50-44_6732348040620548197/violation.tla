---------------------------- MODULE counterexample ----------------------------

EXTENDS command_lifecycle

(* Constant initialization state *)
ConstInit == TRUE

(* Initial state [_transition(0)] *)
(* State0 ==
  __InLoop = FALSE
    /\ __q::temporalProps_init = FALSE
    /\ __saved_☐(∀cmd_438$2 ∈ {'c1', 'c2', 'c3'}: (state[cmd_438$2] ∈ {'completed', 'rejected', 'failed', 'expired', 'cancelled', 'superseded'} ⇒ (state[cmd_438$2]' = state[cmd_438$2])))
      = FALSE
    /\ __saved_☐(∀cmd_438$2 ∈ {'c1', 'c2', 'c3'}: (state[cmd_438$2] ∈ {'completed', 'rejected', 'failed', 'expired', 'cancelled', 'superseded'} ⇒ (state[cmd_438$2]' = state[cmd_438$2])))
      = FALSE
    /\ __saved_appliedKeys = { "k1", "k2", "k3" }
    /\ __saved_applyCount = SetAsFun({ <<"k1", 1>>, <<"k2", 1>>, <<"k3", 1>> })
    /\ __saved_idemKey
      = SetAsFun({ <<"c1", "k1">>, <<"c2", "k2">>, <<"c3", "k3">> })
    /\ __saved_lsn = 3
    /\ __saved_state
      = SetAsFun({ <<"c1", "accepted">>,
        <<"c2", "accepted">>,
        <<"c3", "accepted">> })
    /\ __saved_terminalLsn = SetAsFun({ <<"c1", 0>>, <<"c2", 0>>, <<"c3", 0>> })
    /\ ☐(∀cmd_438$2 ∈ {'c1', 'c2', 'c3'}: (state[cmd_438$2] ∈ {'completed', 'rejected', 'failed', 'expired', 'cancelled', 'superseded'} ⇒ (state[cmd_438$2]' = state[cmd_438$2])))
      = FALSE
    /\ ☐(∀cmd_438$2 ∈ {'c1', 'c2', 'c3'}: (state[cmd_438$2] ∈ {'completed', 'rejected', 'failed', 'expired', 'cancelled', 'superseded'} ⇒ (state[cmd_438$2]' = state[cmd_438$2])))
      = FALSE
    /\ __temporal_t_2_unroll = TRUE
    /\ __temporal_t_2_unroll_prev = TRUE
    /\ appliedKeys = { "k1", "k2", "k3" }
    /\ applyCount = SetAsFun({ <<"k1", 1>>, <<"k2", 1>>, <<"k3", 1>> })
    /\ idemKey = SetAsFun({ <<"c1", "k1">>, <<"c2", "k2">>, <<"c3", "k3">> })
    /\ lsn = 3
    /\ state
      = SetAsFun({ <<"c1", "accepted">>,
        <<"c2", "accepted">>,
        <<"c3", "accepted">> })
    /\ terminalLsn = SetAsFun({ <<"c1", 0>>, <<"c2", 0>>, <<"c3", 0>> }) *)
State0 ==
  __InLoop = FALSE
    /\ __q_temporalProps_init = FALSE
    /\ __saved___temporal_t_1 = FALSE
    /\ __saved___temporal_t_2 = FALSE
    /\ __saved_appliedKeys = { "k1", "k2", "k3" }
    /\ __saved_applyCount = SetAsFun({ <<"k1", 1>>, <<"k2", 1>>, <<"k3", 1>> })
    /\ __saved_idemKey
      = SetAsFun({ <<"c1", "k1">>, <<"c2", "k2">>, <<"c3", "k3">> })
    /\ __saved_lsn = 3
    /\ __saved_state
      = SetAsFun({ <<"c1", "accepted">>,
        <<"c2", "accepted">>,
        <<"c3", "accepted">> })
    /\ __saved_terminalLsn = SetAsFun({ <<"c1", 0>>, <<"c2", 0>>, <<"c3", 0>> })
    /\ __temporal_t_1 = FALSE
    /\ __temporal_t_2 = FALSE
    /\ __temporal_t_2_unroll = TRUE
    /\ __temporal_t_2_unroll_prev = TRUE
    /\ appliedKeys = { "k1", "k2", "k3" }
    /\ applyCount = SetAsFun({ <<"k1", 1>>, <<"k2", 1>>, <<"k3", 1>> })
    /\ idemKey = SetAsFun({ <<"c1", "k1">>, <<"c2", "k2">>, <<"c3", "k3">> })
    /\ lsn = 3
    /\ state
      = SetAsFun({ <<"c1", "accepted">>,
        <<"c2", "accepted">>,
        <<"c3", "accepted">> })
    /\ terminalLsn = SetAsFun({ <<"c1", 0>>, <<"c2", 0>>, <<"c3", 0>> })

(* State1 [_transition(0)] *)
(* State1 ==
  __InLoop = FALSE
    /\ __q::temporalProps_init = FALSE
    /\ __saved_☐(∀cmd_438$2 ∈ {'c1', 'c2', 'c3'}: (state[cmd_438$2] ∈ {'completed', 'rejected', 'failed', 'expired', 'cancelled', 'superseded'} ⇒ (state[cmd_438$2]' = state[cmd_438$2])))
      = FALSE
    /\ __saved_☐(∀cmd_438$2 ∈ {'c1', 'c2', 'c3'}: (state[cmd_438$2] ∈ {'completed', 'rejected', 'failed', 'expired', 'cancelled', 'superseded'} ⇒ (state[cmd_438$2]' = state[cmd_438$2])))
      = FALSE
    /\ __saved_appliedKeys = { "k1", "k2", "k3" }
    /\ __saved_applyCount = SetAsFun({ <<"k1", 1>>, <<"k2", 1>>, <<"k3", 1>> })
    /\ __saved_idemKey
      = SetAsFun({ <<"c1", "k1">>, <<"c2", "k2">>, <<"c3", "k3">> })
    /\ __saved_lsn = 3
    /\ __saved_state
      = SetAsFun({ <<"c1", "accepted">>,
        <<"c2", "accepted">>,
        <<"c3", "accepted">> })
    /\ __saved_terminalLsn = SetAsFun({ <<"c1", 0>>, <<"c2", 0>>, <<"c3", 0>> })
    /\ ☐(∀cmd_438$2 ∈ {'c1', 'c2', 'c3'}: (state[cmd_438$2] ∈ {'completed', 'rejected', 'failed', 'expired', 'cancelled', 'superseded'} ⇒ (state[cmd_438$2]' = state[cmd_438$2])))
      = FALSE
    /\ ☐(∀cmd_438$2 ∈ {'c1', 'c2', 'c3'}: (state[cmd_438$2] ∈ {'completed', 'rejected', 'failed', 'expired', 'cancelled', 'superseded'} ⇒ (state[cmd_438$2]' = state[cmd_438$2])))
      = FALSE
    /\ __temporal_t_2_unroll = FALSE
    /\ __temporal_t_2_unroll_prev = TRUE
    /\ appliedKeys = { "k1", "k2", "k3" }
    /\ applyCount = SetAsFun({ <<"k1", 1>>, <<"k2", 1>>, <<"k3", 1>> })
    /\ idemKey = SetAsFun({ <<"c1", "k1">>, <<"c2", "k2">>, <<"c3", "k3">> })
    /\ lsn = 4
    /\ state
      = SetAsFun({ <<"c1", "cancelled">>,
        <<"c2", "accepted">>,
        <<"c3", "accepted">> })
    /\ terminalLsn = SetAsFun({ <<"c1", 4>>, <<"c2", 0>>, <<"c3", 0>> }) *)
State1 ==
  __InLoop = FALSE
    /\ __q_temporalProps_init = FALSE
    /\ __saved___temporal_t_1 = FALSE
    /\ __saved___temporal_t_2 = FALSE
    /\ __saved_appliedKeys = { "k1", "k2", "k3" }
    /\ __saved_applyCount = SetAsFun({ <<"k1", 1>>, <<"k2", 1>>, <<"k3", 1>> })
    /\ __saved_idemKey
      = SetAsFun({ <<"c1", "k1">>, <<"c2", "k2">>, <<"c3", "k3">> })
    /\ __saved_lsn = 3
    /\ __saved_state
      = SetAsFun({ <<"c1", "accepted">>,
        <<"c2", "accepted">>,
        <<"c3", "accepted">> })
    /\ __saved_terminalLsn = SetAsFun({ <<"c1", 0>>, <<"c2", 0>>, <<"c3", 0>> })
    /\ __temporal_t_1 = FALSE
    /\ __temporal_t_2 = FALSE
    /\ __temporal_t_2_unroll = FALSE
    /\ __temporal_t_2_unroll_prev = TRUE
    /\ appliedKeys = { "k1", "k2", "k3" }
    /\ applyCount = SetAsFun({ <<"k1", 1>>, <<"k2", 1>>, <<"k3", 1>> })
    /\ idemKey = SetAsFun({ <<"c1", "k1">>, <<"c2", "k2">>, <<"c3", "k3">> })
    /\ lsn = 4
    /\ state
      = SetAsFun({ <<"c1", "cancelled">>,
        <<"c2", "accepted">>,
        <<"c3", "accepted">> })
    /\ terminalLsn = SetAsFun({ <<"c1", 4>>, <<"c2", 0>>, <<"c3", 0>> })

(* State2 [_transition(7)] *)
(* State2 ==
  __InLoop = TRUE
    /\ __q::temporalProps_init = FALSE
    /\ __saved_☐(∀cmd_438$2 ∈ {'c1', 'c2', 'c3'}: (state[cmd_438$2] ∈ {'completed', 'rejected', 'failed', 'expired', 'cancelled', 'superseded'} ⇒ (state[cmd_438$2]' = state[cmd_438$2])))
      = FALSE
    /\ __saved_☐(∀cmd_438$2 ∈ {'c1', 'c2', 'c3'}: (state[cmd_438$2] ∈ {'completed', 'rejected', 'failed', 'expired', 'cancelled', 'superseded'} ⇒ (state[cmd_438$2]' = state[cmd_438$2])))
      = FALSE
    /\ __saved_appliedKeys = { "k1", "k2", "k3" }
    /\ __saved_applyCount = SetAsFun({ <<"k1", 1>>, <<"k2", 1>>, <<"k3", 1>> })
    /\ __saved_idemKey
      = SetAsFun({ <<"c1", "k1">>, <<"c2", "k2">>, <<"c3", "k3">> })
    /\ __saved_lsn = 4
    /\ __saved_state
      = SetAsFun({ <<"c1", "cancelled">>,
        <<"c2", "accepted">>,
        <<"c3", "accepted">> })
    /\ __saved_terminalLsn = SetAsFun({ <<"c1", 4>>, <<"c2", 0>>, <<"c3", 0>> })
    /\ ☐(∀cmd_438$2 ∈ {'c1', 'c2', 'c3'}: (state[cmd_438$2] ∈ {'completed', 'rejected', 'failed', 'expired', 'cancelled', 'superseded'} ⇒ (state[cmd_438$2]' = state[cmd_438$2])))
      = FALSE
    /\ ☐(∀cmd_438$2 ∈ {'c1', 'c2', 'c3'}: (state[cmd_438$2] ∈ {'completed', 'rejected', 'failed', 'expired', 'cancelled', 'superseded'} ⇒ (state[cmd_438$2]' = state[cmd_438$2])))
      = FALSE
    /\ __temporal_t_2_unroll = FALSE
    /\ __temporal_t_2_unroll_prev = FALSE
    /\ appliedKeys = { "k1", "k2", "k3" }
    /\ applyCount = SetAsFun({ <<"k1", 1>>, <<"k2", 1>>, <<"k3", 1>> })
    /\ idemKey = SetAsFun({ <<"c1", "k1">>, <<"c2", "k2">>, <<"c3", "k3">> })
    /\ lsn = 4
    /\ state
      = SetAsFun({ <<"c1", "superseded">>,
        <<"c2", "accepted">>,
        <<"c3", "accepted">> })
    /\ terminalLsn = SetAsFun({ <<"c1", 4>>, <<"c2", 0>>, <<"c3", 0>> }) *)
State2 ==
  __InLoop = TRUE
    /\ __q_temporalProps_init = FALSE
    /\ __saved___temporal_t_1 = FALSE
    /\ __saved___temporal_t_2 = FALSE
    /\ __saved_appliedKeys = { "k1", "k2", "k3" }
    /\ __saved_applyCount = SetAsFun({ <<"k1", 1>>, <<"k2", 1>>, <<"k3", 1>> })
    /\ __saved_idemKey
      = SetAsFun({ <<"c1", "k1">>, <<"c2", "k2">>, <<"c3", "k3">> })
    /\ __saved_lsn = 4
    /\ __saved_state
      = SetAsFun({ <<"c1", "cancelled">>,
        <<"c2", "accepted">>,
        <<"c3", "accepted">> })
    /\ __saved_terminalLsn = SetAsFun({ <<"c1", 4>>, <<"c2", 0>>, <<"c3", 0>> })
    /\ __temporal_t_1 = FALSE
    /\ __temporal_t_2 = FALSE
    /\ __temporal_t_2_unroll = FALSE
    /\ __temporal_t_2_unroll_prev = FALSE
    /\ appliedKeys = { "k1", "k2", "k3" }
    /\ applyCount = SetAsFun({ <<"k1", 1>>, <<"k2", 1>>, <<"k3", 1>> })
    /\ idemKey = SetAsFun({ <<"c1", "k1">>, <<"c2", "k2">>, <<"c3", "k3">> })
    /\ lsn = 4
    /\ state
      = SetAsFun({ <<"c1", "superseded">>,
        <<"c2", "accepted">>,
        <<"c3", "accepted">> })
    /\ terminalLsn = SetAsFun({ <<"c1", 4>>, <<"c2", 0>>, <<"c3", 0>> })

(* State3 [_transition(6)] *)
(* State3 ==
  __InLoop = TRUE
    /\ __q::temporalProps_init = FALSE
    /\ __saved_☐(∀cmd_438$2 ∈ {'c1', 'c2', 'c3'}: (state[cmd_438$2] ∈ {'completed', 'rejected', 'failed', 'expired', 'cancelled', 'superseded'} ⇒ (state[cmd_438$2]' = state[cmd_438$2])))
      = FALSE
    /\ __saved_☐(∀cmd_438$2 ∈ {'c1', 'c2', 'c3'}: (state[cmd_438$2] ∈ {'completed', 'rejected', 'failed', 'expired', 'cancelled', 'superseded'} ⇒ (state[cmd_438$2]' = state[cmd_438$2])))
      = FALSE
    /\ __saved_appliedKeys = { "k1", "k2", "k3" }
    /\ __saved_applyCount = SetAsFun({ <<"k1", 1>>, <<"k2", 1>>, <<"k3", 1>> })
    /\ __saved_idemKey
      = SetAsFun({ <<"c1", "k1">>, <<"c2", "k2">>, <<"c3", "k3">> })
    /\ __saved_lsn = 4
    /\ __saved_state
      = SetAsFun({ <<"c1", "cancelled">>,
        <<"c2", "accepted">>,
        <<"c3", "accepted">> })
    /\ __saved_terminalLsn = SetAsFun({ <<"c1", 4>>, <<"c2", 0>>, <<"c3", 0>> })
    /\ ☐(∀cmd_438$2 ∈ {'c1', 'c2', 'c3'}: (state[cmd_438$2] ∈ {'completed', 'rejected', 'failed', 'expired', 'cancelled', 'superseded'} ⇒ (state[cmd_438$2]' = state[cmd_438$2])))
      = FALSE
    /\ ☐(∀cmd_438$2 ∈ {'c1', 'c2', 'c3'}: (state[cmd_438$2] ∈ {'completed', 'rejected', 'failed', 'expired', 'cancelled', 'superseded'} ⇒ (state[cmd_438$2]' = state[cmd_438$2])))
      = FALSE
    /\ __temporal_t_2_unroll = FALSE
    /\ __temporal_t_2_unroll_prev = FALSE
    /\ appliedKeys = { "k1", "k2", "k3" }
    /\ applyCount = SetAsFun({ <<"k1", 1>>, <<"k2", 1>>, <<"k3", 1>> })
    /\ idemKey = SetAsFun({ <<"c1", "k1">>, <<"c2", "k2">>, <<"c3", "k3">> })
    /\ lsn = 4
    /\ state
      = SetAsFun({ <<"c1", "cancelled">>,
        <<"c2", "accepted">>,
        <<"c3", "accepted">> })
    /\ terminalLsn = SetAsFun({ <<"c1", 4>>, <<"c2", 0>>, <<"c3", 0>> }) *)
State3 ==
  __InLoop = TRUE
    /\ __q_temporalProps_init = FALSE
    /\ __saved___temporal_t_1 = FALSE
    /\ __saved___temporal_t_2 = FALSE
    /\ __saved_appliedKeys = { "k1", "k2", "k3" }
    /\ __saved_applyCount = SetAsFun({ <<"k1", 1>>, <<"k2", 1>>, <<"k3", 1>> })
    /\ __saved_idemKey
      = SetAsFun({ <<"c1", "k1">>, <<"c2", "k2">>, <<"c3", "k3">> })
    /\ __saved_lsn = 4
    /\ __saved_state
      = SetAsFun({ <<"c1", "cancelled">>,
        <<"c2", "accepted">>,
        <<"c3", "accepted">> })
    /\ __saved_terminalLsn = SetAsFun({ <<"c1", 4>>, <<"c2", 0>>, <<"c3", 0>> })
    /\ __temporal_t_1 = FALSE
    /\ __temporal_t_2 = FALSE
    /\ __temporal_t_2_unroll = FALSE
    /\ __temporal_t_2_unroll_prev = FALSE
    /\ appliedKeys = { "k1", "k2", "k3" }
    /\ applyCount = SetAsFun({ <<"k1", 1>>, <<"k2", 1>>, <<"k3", 1>> })
    /\ idemKey = SetAsFun({ <<"c1", "k1">>, <<"c2", "k2">>, <<"c3", "k3">> })
    /\ lsn = 4
    /\ state
      = SetAsFun({ <<"c1", "cancelled">>,
        <<"c2", "accepted">>,
        <<"c3", "accepted">> })
    /\ terminalLsn = SetAsFun({ <<"c1", 4>>, <<"c2", 0>>, <<"c3", 0>> })

(* The following formula holds true in the last state and violates the invariant *)
(* InvariantViolation ==
  (__InLoop
      /\ state = __saved_state
      /\ idemKey = __saved_idemKey
      /\ appliedKeys = __saved_appliedKeys
      /\ applyCount = __saved_applyCount
      /\ lsn = __saved_lsn
      /\ terminalLsn = __saved_terminalLsn
      /\ ☐(∀cmd_438$2 ∈ {'c1', 'c2', 'c3'}: (state[cmd_438$2] ∈ {'completed', 'rejected', 'failed', 'expired', 'cancelled', 'superseded'} ⇒ (state[cmd_438$2]' = state[cmd_438$2])))
        = __saved_☐(∀cmd_438$2 ∈ {'c1', 'c2', 'c3'}: (state[cmd_438$2] ∈ {'completed', 'rejected', 'failed', 'expired', 'cancelled', 'superseded'} ⇒ (state[cmd_438$2]' = state[cmd_438$2])))
      /\ ☐(∀cmd_438$2 ∈ {'c1', 'c2', 'c3'}: (state[cmd_438$2] ∈ {'completed', 'rejected', 'failed', 'expired', 'cancelled', 'superseded'} ⇒ (state[cmd_438$2]' = state[cmd_438$2])))
        = __saved_☐(∀cmd_438$2 ∈ {'c1', 'c2', 'c3'}: (state[cmd_438$2] ∈ {'completed', 'rejected', 'failed', 'expired', 'cancelled', 'superseded'} ⇒ (state[cmd_438$2]' = state[cmd_438$2])))
      /\ (~__temporal_t_2_unroll
        \/ ☐(∀cmd_438$2 ∈ {'c1', 'c2', 'c3'}: (state[cmd_438$2] ∈ {'completed', 'rejected', 'failed', 'expired', 'cancelled', 'superseded'} ⇒ (state[cmd_438$2]' = state[cmd_438$2]))))
      /\ __temporal_t_2_unroll_prev = __temporal_t_2_unroll)
    /\ ~__q::temporalProps_init *)
InvariantViolation ==
  (__InLoop
      /\ state = __saved_state
      /\ idemKey = __saved_idemKey
      /\ appliedKeys = __saved_appliedKeys
      /\ applyCount = __saved_applyCount
      /\ lsn = __saved_lsn
      /\ terminalLsn = __saved_terminalLsn
      /\ __temporal_t_1 = __saved___temporal_t_1
      /\ __temporal_t_2 = __saved___temporal_t_2
      /\ (~__temporal_t_2_unroll \/ __temporal_t_2)
      /\ __temporal_t_2_unroll_prev = __temporal_t_2_unroll)
    /\ ~__q_temporalProps_init

================================================================================
(* Created by Apalache on Sat Jul 11 11:50:51 MDT 2026 *)
(* https://github.com/apalache-mc/apalache *)
