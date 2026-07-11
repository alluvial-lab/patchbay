---------------------------- MODULE counterexample ----------------------------

EXTENDS session_generation

(* Constant initialization state *)
ConstInit == TRUE

(* Initial state [_transition(0)] *)
(* State0 ==
  __InLoop = FALSE
    /\ __q::temporalProps_init = FALSE
    /\ __saved_☐(∀sid_279$2 ∈ {'s1', 's2'}: (generation[sid_279$2]' ≥ generation[sid_279$2]))
      = FALSE
    /\ __saved_☐(∀sid_279$2 ∈ {'s1', 's2'}: (generation[sid_279$2]' ≥ generation[sid_279$2]))
      = FALSE
    /\ __saved_attemptedGen = 0
    /\ __saved_attemptedKind = "relabel"
    /\ __saved_attemptedSid = "s1"
    /\ __saved_generation = SetAsFun({ <<"s1", 0>>, <<"s2", 0>> })
    /\ __saved_identityGeneration = SetAsFun({ <<"s1", 0>>, <<"s2", 0>> })
    /\ __saved_label = SetAsFun({ <<"s1", "proj-A">>, <<"s2", "proj-A">> })
    /\ __saved_lsn = 0
    /\ __saved_tombstoneLsn
      = SetAsFun({ <<<<"s1", 0>>, 0>>,
        <<<<"s1", 1>>, 0>>,
        <<<<"s1", 2>>, 0>>,
        <<<<"s1", 3>>, 0>>,
        <<<<"s2", 0>>, 0>>,
        <<<<"s2", 1>>, 0>>,
        <<<<"s2", 2>>, 0>>,
        <<<<"s2", 3>>, 0>> })
    /\ __saved_tombstoned
      = SetAsFun({ <<<<"s1", 0>>, FALSE>>,
        <<<<"s1", 1>>, FALSE>>,
        <<<<"s1", 2>>, FALSE>>,
        <<<<"s1", 3>>, FALSE>>,
        <<<<"s2", 0>>, FALSE>>,
        <<<<"s2", 1>>, FALSE>>,
        <<<<"s2", 2>>, FALSE>>,
        <<<<"s2", 3>>, FALSE>> })
    /\ ☐(∀sid_279$2 ∈ {'s1', 's2'}: (generation[sid_279$2]' ≥ generation[sid_279$2]))
      = FALSE
    /\ ☐(∀sid_279$2 ∈ {'s1', 's2'}: (generation[sid_279$2]' ≥ generation[sid_279$2]))
      = FALSE
    /\ __temporal_t_2_unroll = TRUE
    /\ __temporal_t_2_unroll_prev = TRUE
    /\ attemptedGen = 0
    /\ attemptedKind = "relabel"
    /\ attemptedSid = "s1"
    /\ generation = SetAsFun({ <<"s1", 0>>, <<"s2", 0>> })
    /\ identityGeneration = SetAsFun({ <<"s1", 0>>, <<"s2", 0>> })
    /\ label = SetAsFun({ <<"s1", "proj-A">>, <<"s2", "proj-A">> })
    /\ lsn = 0
    /\ tombstoneLsn
      = SetAsFun({ <<<<"s1", 0>>, 0>>,
        <<<<"s1", 1>>, 0>>,
        <<<<"s1", 2>>, 0>>,
        <<<<"s1", 3>>, 0>>,
        <<<<"s2", 0>>, 0>>,
        <<<<"s2", 1>>, 0>>,
        <<<<"s2", 2>>, 0>>,
        <<<<"s2", 3>>, 0>> })
    /\ tombstoned
      = SetAsFun({ <<<<"s1", 0>>, FALSE>>,
        <<<<"s1", 1>>, FALSE>>,
        <<<<"s1", 2>>, FALSE>>,
        <<<<"s1", 3>>, FALSE>>,
        <<<<"s2", 0>>, FALSE>>,
        <<<<"s2", 1>>, FALSE>>,
        <<<<"s2", 2>>, FALSE>>,
        <<<<"s2", 3>>, FALSE>> }) *)
State0 ==
  __InLoop = FALSE
    /\ __q_temporalProps_init = FALSE
    /\ __saved___temporal_t_1 = FALSE
    /\ __saved___temporal_t_2 = FALSE
    /\ __saved_attemptedGen = 0
    /\ __saved_attemptedKind = "relabel"
    /\ __saved_attemptedSid = "s1"
    /\ __saved_generation = SetAsFun({ <<"s1", 0>>, <<"s2", 0>> })
    /\ __saved_identityGeneration = SetAsFun({ <<"s1", 0>>, <<"s2", 0>> })
    /\ __saved_label = SetAsFun({ <<"s1", "proj-A">>, <<"s2", "proj-A">> })
    /\ __saved_lsn = 0
    /\ __saved_tombstoneLsn
      = SetAsFun({ <<<<"s1", 0>>, 0>>,
        <<<<"s1", 1>>, 0>>,
        <<<<"s1", 2>>, 0>>,
        <<<<"s1", 3>>, 0>>,
        <<<<"s2", 0>>, 0>>,
        <<<<"s2", 1>>, 0>>,
        <<<<"s2", 2>>, 0>>,
        <<<<"s2", 3>>, 0>> })
    /\ __saved_tombstoned
      = SetAsFun({ <<<<"s1", 0>>, FALSE>>,
        <<<<"s1", 1>>, FALSE>>,
        <<<<"s1", 2>>, FALSE>>,
        <<<<"s1", 3>>, FALSE>>,
        <<<<"s2", 0>>, FALSE>>,
        <<<<"s2", 1>>, FALSE>>,
        <<<<"s2", 2>>, FALSE>>,
        <<<<"s2", 3>>, FALSE>> })
    /\ __temporal_t_1 = FALSE
    /\ __temporal_t_2 = FALSE
    /\ __temporal_t_2_unroll = TRUE
    /\ __temporal_t_2_unroll_prev = TRUE
    /\ attemptedGen = 0
    /\ attemptedKind = "relabel"
    /\ attemptedSid = "s1"
    /\ generation = SetAsFun({ <<"s1", 0>>, <<"s2", 0>> })
    /\ identityGeneration = SetAsFun({ <<"s1", 0>>, <<"s2", 0>> })
    /\ label = SetAsFun({ <<"s1", "proj-A">>, <<"s2", "proj-A">> })
    /\ lsn = 0
    /\ tombstoneLsn
      = SetAsFun({ <<<<"s1", 0>>, 0>>,
        <<<<"s1", 1>>, 0>>,
        <<<<"s1", 2>>, 0>>,
        <<<<"s1", 3>>, 0>>,
        <<<<"s2", 0>>, 0>>,
        <<<<"s2", 1>>, 0>>,
        <<<<"s2", 2>>, 0>>,
        <<<<"s2", 3>>, 0>> })
    /\ tombstoned
      = SetAsFun({ <<<<"s1", 0>>, FALSE>>,
        <<<<"s1", 1>>, FALSE>>,
        <<<<"s1", 2>>, FALSE>>,
        <<<<"s1", 3>>, FALSE>>,
        <<<<"s2", 0>>, FALSE>>,
        <<<<"s2", 1>>, FALSE>>,
        <<<<"s2", 2>>, FALSE>>,
        <<<<"s2", 3>>, FALSE>> })

(* State1 [_transition(0)] *)
(* State1 ==
  __InLoop = FALSE
    /\ __q::temporalProps_init = FALSE
    /\ __saved_☐(∀sid_279$2 ∈ {'s1', 's2'}: (generation[sid_279$2]' ≥ generation[sid_279$2]))
      = FALSE
    /\ __saved_☐(∀sid_279$2 ∈ {'s1', 's2'}: (generation[sid_279$2]' ≥ generation[sid_279$2]))
      = FALSE
    /\ __saved_attemptedGen = 0
    /\ __saved_attemptedKind = "relabel"
    /\ __saved_attemptedSid = "s1"
    /\ __saved_generation = SetAsFun({ <<"s1", 0>>, <<"s2", 0>> })
    /\ __saved_identityGeneration = SetAsFun({ <<"s1", 0>>, <<"s2", 0>> })
    /\ __saved_label = SetAsFun({ <<"s1", "proj-A">>, <<"s2", "proj-A">> })
    /\ __saved_lsn = 0
    /\ __saved_tombstoneLsn
      = SetAsFun({ <<<<"s1", 0>>, 0>>,
        <<<<"s1", 1>>, 0>>,
        <<<<"s1", 2>>, 0>>,
        <<<<"s1", 3>>, 0>>,
        <<<<"s2", 0>>, 0>>,
        <<<<"s2", 1>>, 0>>,
        <<<<"s2", 2>>, 0>>,
        <<<<"s2", 3>>, 0>> })
    /\ __saved_tombstoned
      = SetAsFun({ <<<<"s1", 0>>, FALSE>>,
        <<<<"s1", 1>>, FALSE>>,
        <<<<"s1", 2>>, FALSE>>,
        <<<<"s1", 3>>, FALSE>>,
        <<<<"s2", 0>>, FALSE>>,
        <<<<"s2", 1>>, FALSE>>,
        <<<<"s2", 2>>, FALSE>>,
        <<<<"s2", 3>>, FALSE>> })
    /\ ☐(∀sid_279$2 ∈ {'s1', 's2'}: (generation[sid_279$2]' ≥ generation[sid_279$2]))
      = FALSE
    /\ ☐(∀sid_279$2 ∈ {'s1', 's2'}: (generation[sid_279$2]' ≥ generation[sid_279$2]))
      = FALSE
    /\ __temporal_t_2_unroll = TRUE
    /\ __temporal_t_2_unroll_prev = TRUE
    /\ attemptedGen = 2
    /\ attemptedKind = "report"
    /\ attemptedSid = "s1"
    /\ generation = SetAsFun({ <<"s1", 2>>, <<"s2", 0>> })
    /\ identityGeneration = SetAsFun({ <<"s1", 2>>, <<"s2", 0>> })
    /\ label = SetAsFun({ <<"s1", "proj-A">>, <<"s2", "proj-A">> })
    /\ lsn = 1
    /\ tombstoneLsn
      = SetAsFun({ <<<<"s1", 0>>, 1>>,
        <<<<"s1", 1>>, 0>>,
        <<<<"s1", 2>>, 0>>,
        <<<<"s1", 3>>, 0>>,
        <<<<"s2", 0>>, 0>>,
        <<<<"s2", 1>>, 0>>,
        <<<<"s2", 2>>, 0>>,
        <<<<"s2", 3>>, 0>> })
    /\ tombstoned
      = SetAsFun({ <<<<"s1", 0>>, TRUE>>,
        <<<<"s1", 1>>, FALSE>>,
        <<<<"s1", 2>>, FALSE>>,
        <<<<"s1", 3>>, FALSE>>,
        <<<<"s2", 0>>, FALSE>>,
        <<<<"s2", 1>>, FALSE>>,
        <<<<"s2", 2>>, FALSE>>,
        <<<<"s2", 3>>, FALSE>> }) *)
State1 ==
  __InLoop = FALSE
    /\ __q_temporalProps_init = FALSE
    /\ __saved___temporal_t_1 = FALSE
    /\ __saved___temporal_t_2 = FALSE
    /\ __saved_attemptedGen = 0
    /\ __saved_attemptedKind = "relabel"
    /\ __saved_attemptedSid = "s1"
    /\ __saved_generation = SetAsFun({ <<"s1", 0>>, <<"s2", 0>> })
    /\ __saved_identityGeneration = SetAsFun({ <<"s1", 0>>, <<"s2", 0>> })
    /\ __saved_label = SetAsFun({ <<"s1", "proj-A">>, <<"s2", "proj-A">> })
    /\ __saved_lsn = 0
    /\ __saved_tombstoneLsn
      = SetAsFun({ <<<<"s1", 0>>, 0>>,
        <<<<"s1", 1>>, 0>>,
        <<<<"s1", 2>>, 0>>,
        <<<<"s1", 3>>, 0>>,
        <<<<"s2", 0>>, 0>>,
        <<<<"s2", 1>>, 0>>,
        <<<<"s2", 2>>, 0>>,
        <<<<"s2", 3>>, 0>> })
    /\ __saved_tombstoned
      = SetAsFun({ <<<<"s1", 0>>, FALSE>>,
        <<<<"s1", 1>>, FALSE>>,
        <<<<"s1", 2>>, FALSE>>,
        <<<<"s1", 3>>, FALSE>>,
        <<<<"s2", 0>>, FALSE>>,
        <<<<"s2", 1>>, FALSE>>,
        <<<<"s2", 2>>, FALSE>>,
        <<<<"s2", 3>>, FALSE>> })
    /\ __temporal_t_1 = FALSE
    /\ __temporal_t_2 = FALSE
    /\ __temporal_t_2_unroll = TRUE
    /\ __temporal_t_2_unroll_prev = TRUE
    /\ attemptedGen = 2
    /\ attemptedKind = "report"
    /\ attemptedSid = "s1"
    /\ generation = SetAsFun({ <<"s1", 2>>, <<"s2", 0>> })
    /\ identityGeneration = SetAsFun({ <<"s1", 2>>, <<"s2", 0>> })
    /\ label = SetAsFun({ <<"s1", "proj-A">>, <<"s2", "proj-A">> })
    /\ lsn = 1
    /\ tombstoneLsn
      = SetAsFun({ <<<<"s1", 0>>, 1>>,
        <<<<"s1", 1>>, 0>>,
        <<<<"s1", 2>>, 0>>,
        <<<<"s1", 3>>, 0>>,
        <<<<"s2", 0>>, 0>>,
        <<<<"s2", 1>>, 0>>,
        <<<<"s2", 2>>, 0>>,
        <<<<"s2", 3>>, 0>> })
    /\ tombstoned
      = SetAsFun({ <<<<"s1", 0>>, TRUE>>,
        <<<<"s1", 1>>, FALSE>>,
        <<<<"s1", 2>>, FALSE>>,
        <<<<"s1", 3>>, FALSE>>,
        <<<<"s2", 0>>, FALSE>>,
        <<<<"s2", 1>>, FALSE>>,
        <<<<"s2", 2>>, FALSE>>,
        <<<<"s2", 3>>, FALSE>> })

(* State2 [_transition(0)] *)
(* State2 ==
  __InLoop = FALSE
    /\ __q::temporalProps_init = FALSE
    /\ __saved_☐(∀sid_279$2 ∈ {'s1', 's2'}: (generation[sid_279$2]' ≥ generation[sid_279$2]))
      = FALSE
    /\ __saved_☐(∀sid_279$2 ∈ {'s1', 's2'}: (generation[sid_279$2]' ≥ generation[sid_279$2]))
      = FALSE
    /\ __saved_attemptedGen = 0
    /\ __saved_attemptedKind = "relabel"
    /\ __saved_attemptedSid = "s1"
    /\ __saved_generation = SetAsFun({ <<"s1", 0>>, <<"s2", 0>> })
    /\ __saved_identityGeneration = SetAsFun({ <<"s1", 0>>, <<"s2", 0>> })
    /\ __saved_label = SetAsFun({ <<"s1", "proj-A">>, <<"s2", "proj-A">> })
    /\ __saved_lsn = 0
    /\ __saved_tombstoneLsn
      = SetAsFun({ <<<<"s1", 0>>, 0>>,
        <<<<"s1", 1>>, 0>>,
        <<<<"s1", 2>>, 0>>,
        <<<<"s1", 3>>, 0>>,
        <<<<"s2", 0>>, 0>>,
        <<<<"s2", 1>>, 0>>,
        <<<<"s2", 2>>, 0>>,
        <<<<"s2", 3>>, 0>> })
    /\ __saved_tombstoned
      = SetAsFun({ <<<<"s1", 0>>, FALSE>>,
        <<<<"s1", 1>>, FALSE>>,
        <<<<"s1", 2>>, FALSE>>,
        <<<<"s1", 3>>, FALSE>>,
        <<<<"s2", 0>>, FALSE>>,
        <<<<"s2", 1>>, FALSE>>,
        <<<<"s2", 2>>, FALSE>>,
        <<<<"s2", 3>>, FALSE>> })
    /\ ☐(∀sid_279$2 ∈ {'s1', 's2'}: (generation[sid_279$2]' ≥ generation[sid_279$2]))
      = FALSE
    /\ ☐(∀sid_279$2 ∈ {'s1', 's2'}: (generation[sid_279$2]' ≥ generation[sid_279$2]))
      = TRUE
    /\ __temporal_t_2_unroll = TRUE
    /\ __temporal_t_2_unroll_prev = TRUE
    /\ attemptedGen = 1
    /\ attemptedKind = "report"
    /\ attemptedSid = "s1"
    /\ generation = SetAsFun({ <<"s1", 1>>, <<"s2", 0>> })
    /\ identityGeneration = SetAsFun({ <<"s1", 2>>, <<"s2", 0>> })
    /\ label = SetAsFun({ <<"s1", "proj-A">>, <<"s2", "proj-A">> })
    /\ lsn = 1
    /\ tombstoneLsn
      = SetAsFun({ <<<<"s1", 0>>, 1>>,
        <<<<"s1", 1>>, 0>>,
        <<<<"s1", 2>>, 0>>,
        <<<<"s1", 3>>, 0>>,
        <<<<"s2", 0>>, 0>>,
        <<<<"s2", 1>>, 0>>,
        <<<<"s2", 2>>, 0>>,
        <<<<"s2", 3>>, 0>> })
    /\ tombstoned
      = SetAsFun({ <<<<"s1", 0>>, TRUE>>,
        <<<<"s1", 1>>, FALSE>>,
        <<<<"s1", 2>>, FALSE>>,
        <<<<"s1", 3>>, FALSE>>,
        <<<<"s2", 0>>, FALSE>>,
        <<<<"s2", 1>>, FALSE>>,
        <<<<"s2", 2>>, FALSE>>,
        <<<<"s2", 3>>, FALSE>> }) *)
State2 ==
  __InLoop = FALSE
    /\ __q_temporalProps_init = FALSE
    /\ __saved___temporal_t_1 = FALSE
    /\ __saved___temporal_t_2 = FALSE
    /\ __saved_attemptedGen = 0
    /\ __saved_attemptedKind = "relabel"
    /\ __saved_attemptedSid = "s1"
    /\ __saved_generation = SetAsFun({ <<"s1", 0>>, <<"s2", 0>> })
    /\ __saved_identityGeneration = SetAsFun({ <<"s1", 0>>, <<"s2", 0>> })
    /\ __saved_label = SetAsFun({ <<"s1", "proj-A">>, <<"s2", "proj-A">> })
    /\ __saved_lsn = 0
    /\ __saved_tombstoneLsn
      = SetAsFun({ <<<<"s1", 0>>, 0>>,
        <<<<"s1", 1>>, 0>>,
        <<<<"s1", 2>>, 0>>,
        <<<<"s1", 3>>, 0>>,
        <<<<"s2", 0>>, 0>>,
        <<<<"s2", 1>>, 0>>,
        <<<<"s2", 2>>, 0>>,
        <<<<"s2", 3>>, 0>> })
    /\ __saved_tombstoned
      = SetAsFun({ <<<<"s1", 0>>, FALSE>>,
        <<<<"s1", 1>>, FALSE>>,
        <<<<"s1", 2>>, FALSE>>,
        <<<<"s1", 3>>, FALSE>>,
        <<<<"s2", 0>>, FALSE>>,
        <<<<"s2", 1>>, FALSE>>,
        <<<<"s2", 2>>, FALSE>>,
        <<<<"s2", 3>>, FALSE>> })
    /\ __temporal_t_1 = FALSE
    /\ __temporal_t_2 = TRUE
    /\ __temporal_t_2_unroll = TRUE
    /\ __temporal_t_2_unroll_prev = TRUE
    /\ attemptedGen = 1
    /\ attemptedKind = "report"
    /\ attemptedSid = "s1"
    /\ generation = SetAsFun({ <<"s1", 1>>, <<"s2", 0>> })
    /\ identityGeneration = SetAsFun({ <<"s1", 2>>, <<"s2", 0>> })
    /\ label = SetAsFun({ <<"s1", "proj-A">>, <<"s2", "proj-A">> })
    /\ lsn = 1
    /\ tombstoneLsn
      = SetAsFun({ <<<<"s1", 0>>, 1>>,
        <<<<"s1", 1>>, 0>>,
        <<<<"s1", 2>>, 0>>,
        <<<<"s1", 3>>, 0>>,
        <<<<"s2", 0>>, 0>>,
        <<<<"s2", 1>>, 0>>,
        <<<<"s2", 2>>, 0>>,
        <<<<"s2", 3>>, 0>> })
    /\ tombstoned
      = SetAsFun({ <<<<"s1", 0>>, TRUE>>,
        <<<<"s1", 1>>, FALSE>>,
        <<<<"s1", 2>>, FALSE>>,
        <<<<"s1", 3>>, FALSE>>,
        <<<<"s2", 0>>, FALSE>>,
        <<<<"s2", 1>>, FALSE>>,
        <<<<"s2", 2>>, FALSE>>,
        <<<<"s2", 3>>, FALSE>> })

(* State3 [_transition(0)] *)
(* State3 ==
  __InLoop = FALSE
    /\ __q::temporalProps_init = FALSE
    /\ __saved_☐(∀sid_279$2 ∈ {'s1', 's2'}: (generation[sid_279$2]' ≥ generation[sid_279$2]))
      = FALSE
    /\ __saved_☐(∀sid_279$2 ∈ {'s1', 's2'}: (generation[sid_279$2]' ≥ generation[sid_279$2]))
      = FALSE
    /\ __saved_attemptedGen = 0
    /\ __saved_attemptedKind = "relabel"
    /\ __saved_attemptedSid = "s1"
    /\ __saved_generation = SetAsFun({ <<"s1", 0>>, <<"s2", 0>> })
    /\ __saved_identityGeneration = SetAsFun({ <<"s1", 0>>, <<"s2", 0>> })
    /\ __saved_label = SetAsFun({ <<"s1", "proj-A">>, <<"s2", "proj-A">> })
    /\ __saved_lsn = 0
    /\ __saved_tombstoneLsn
      = SetAsFun({ <<<<"s1", 0>>, 0>>,
        <<<<"s1", 1>>, 0>>,
        <<<<"s1", 2>>, 0>>,
        <<<<"s1", 3>>, 0>>,
        <<<<"s2", 0>>, 0>>,
        <<<<"s2", 1>>, 0>>,
        <<<<"s2", 2>>, 0>>,
        <<<<"s2", 3>>, 0>> })
    /\ __saved_tombstoned
      = SetAsFun({ <<<<"s1", 0>>, FALSE>>,
        <<<<"s1", 1>>, FALSE>>,
        <<<<"s1", 2>>, FALSE>>,
        <<<<"s1", 3>>, FALSE>>,
        <<<<"s2", 0>>, FALSE>>,
        <<<<"s2", 1>>, FALSE>>,
        <<<<"s2", 2>>, FALSE>>,
        <<<<"s2", 3>>, FALSE>> })
    /\ ☐(∀sid_279$2 ∈ {'s1', 's2'}: (generation[sid_279$2]' ≥ generation[sid_279$2]))
      = TRUE
    /\ ☐(∀sid_279$2 ∈ {'s1', 's2'}: (generation[sid_279$2]' ≥ generation[sid_279$2]))
      = TRUE
    /\ __temporal_t_2_unroll = TRUE
    /\ __temporal_t_2_unroll_prev = TRUE
    /\ attemptedGen = 2
    /\ attemptedKind = "report"
    /\ attemptedSid = "s1"
    /\ generation = SetAsFun({ <<"s1", 2>>, <<"s2", 0>> })
    /\ identityGeneration = SetAsFun({ <<"s1", 2>>, <<"s2", 0>> })
    /\ label = SetAsFun({ <<"s1", "proj-A">>, <<"s2", "proj-A">> })
    /\ lsn = 2
    /\ tombstoneLsn
      = SetAsFun({ <<<<"s1", 0>>, 1>>,
        <<<<"s1", 1>>, 2>>,
        <<<<"s1", 2>>, 0>>,
        <<<<"s1", 3>>, 0>>,
        <<<<"s2", 0>>, 0>>,
        <<<<"s2", 1>>, 0>>,
        <<<<"s2", 2>>, 0>>,
        <<<<"s2", 3>>, 0>> })
    /\ tombstoned
      = SetAsFun({ <<<<"s1", 0>>, TRUE>>,
        <<<<"s1", 1>>, TRUE>>,
        <<<<"s1", 2>>, FALSE>>,
        <<<<"s1", 3>>, FALSE>>,
        <<<<"s2", 0>>, FALSE>>,
        <<<<"s2", 1>>, FALSE>>,
        <<<<"s2", 2>>, FALSE>>,
        <<<<"s2", 3>>, FALSE>> }) *)
State3 ==
  __InLoop = FALSE
    /\ __q_temporalProps_init = FALSE
    /\ __saved___temporal_t_1 = FALSE
    /\ __saved___temporal_t_2 = FALSE
    /\ __saved_attemptedGen = 0
    /\ __saved_attemptedKind = "relabel"
    /\ __saved_attemptedSid = "s1"
    /\ __saved_generation = SetAsFun({ <<"s1", 0>>, <<"s2", 0>> })
    /\ __saved_identityGeneration = SetAsFun({ <<"s1", 0>>, <<"s2", 0>> })
    /\ __saved_label = SetAsFun({ <<"s1", "proj-A">>, <<"s2", "proj-A">> })
    /\ __saved_lsn = 0
    /\ __saved_tombstoneLsn
      = SetAsFun({ <<<<"s1", 0>>, 0>>,
        <<<<"s1", 1>>, 0>>,
        <<<<"s1", 2>>, 0>>,
        <<<<"s1", 3>>, 0>>,
        <<<<"s2", 0>>, 0>>,
        <<<<"s2", 1>>, 0>>,
        <<<<"s2", 2>>, 0>>,
        <<<<"s2", 3>>, 0>> })
    /\ __saved_tombstoned
      = SetAsFun({ <<<<"s1", 0>>, FALSE>>,
        <<<<"s1", 1>>, FALSE>>,
        <<<<"s1", 2>>, FALSE>>,
        <<<<"s1", 3>>, FALSE>>,
        <<<<"s2", 0>>, FALSE>>,
        <<<<"s2", 1>>, FALSE>>,
        <<<<"s2", 2>>, FALSE>>,
        <<<<"s2", 3>>, FALSE>> })
    /\ __temporal_t_1 = TRUE
    /\ __temporal_t_2 = TRUE
    /\ __temporal_t_2_unroll = TRUE
    /\ __temporal_t_2_unroll_prev = TRUE
    /\ attemptedGen = 2
    /\ attemptedKind = "report"
    /\ attemptedSid = "s1"
    /\ generation = SetAsFun({ <<"s1", 2>>, <<"s2", 0>> })
    /\ identityGeneration = SetAsFun({ <<"s1", 2>>, <<"s2", 0>> })
    /\ label = SetAsFun({ <<"s1", "proj-A">>, <<"s2", "proj-A">> })
    /\ lsn = 2
    /\ tombstoneLsn
      = SetAsFun({ <<<<"s1", 0>>, 1>>,
        <<<<"s1", 1>>, 2>>,
        <<<<"s1", 2>>, 0>>,
        <<<<"s1", 3>>, 0>>,
        <<<<"s2", 0>>, 0>>,
        <<<<"s2", 1>>, 0>>,
        <<<<"s2", 2>>, 0>>,
        <<<<"s2", 3>>, 0>> })
    /\ tombstoned
      = SetAsFun({ <<<<"s1", 0>>, TRUE>>,
        <<<<"s1", 1>>, TRUE>>,
        <<<<"s1", 2>>, FALSE>>,
        <<<<"s1", 3>>, FALSE>>,
        <<<<"s2", 0>>, FALSE>>,
        <<<<"s2", 1>>, FALSE>>,
        <<<<"s2", 2>>, FALSE>>,
        <<<<"s2", 3>>, FALSE>> })

(* State4 [_transition(1)] *)
(* State4 ==
  __InLoop = TRUE
    /\ __q::temporalProps_init = FALSE
    /\ __saved_☐(∀sid_279$2 ∈ {'s1', 's2'}: (generation[sid_279$2]' ≥ generation[sid_279$2]))
      = TRUE
    /\ __saved_☐(∀sid_279$2 ∈ {'s1', 's2'}: (generation[sid_279$2]' ≥ generation[sid_279$2]))
      = TRUE
    /\ __saved_attemptedGen = 2
    /\ __saved_attemptedKind = "report"
    /\ __saved_attemptedSid = "s1"
    /\ __saved_generation = SetAsFun({ <<"s1", 2>>, <<"s2", 0>> })
    /\ __saved_identityGeneration = SetAsFun({ <<"s1", 2>>, <<"s2", 0>> })
    /\ __saved_label = SetAsFun({ <<"s1", "proj-A">>, <<"s2", "proj-A">> })
    /\ __saved_lsn = 2
    /\ __saved_tombstoneLsn
      = SetAsFun({ <<<<"s1", 0>>, 1>>,
        <<<<"s1", 1>>, 2>>,
        <<<<"s1", 2>>, 0>>,
        <<<<"s1", 3>>, 0>>,
        <<<<"s2", 0>>, 0>>,
        <<<<"s2", 1>>, 0>>,
        <<<<"s2", 2>>, 0>>,
        <<<<"s2", 3>>, 0>> })
    /\ __saved_tombstoned
      = SetAsFun({ <<<<"s1", 0>>, TRUE>>,
        <<<<"s1", 1>>, TRUE>>,
        <<<<"s1", 2>>, FALSE>>,
        <<<<"s1", 3>>, FALSE>>,
        <<<<"s2", 0>>, FALSE>>,
        <<<<"s2", 1>>, FALSE>>,
        <<<<"s2", 2>>, FALSE>>,
        <<<<"s2", 3>>, FALSE>> })
    /\ ☐(∀sid_279$2 ∈ {'s1', 's2'}: (generation[sid_279$2]' ≥ generation[sid_279$2]))
      = TRUE
    /\ ☐(∀sid_279$2 ∈ {'s1', 's2'}: (generation[sid_279$2]' ≥ generation[sid_279$2]))
      = TRUE
    /\ __temporal_t_2_unroll = TRUE
    /\ __temporal_t_2_unroll_prev = TRUE
    /\ attemptedGen = 2
    /\ attemptedKind = "report"
    /\ attemptedSid = "s1"
    /\ generation = SetAsFun({ <<"s1", 2>>, <<"s2", 0>> })
    /\ identityGeneration = SetAsFun({ <<"s1", 2>>, <<"s2", 0>> })
    /\ label = SetAsFun({ <<"s1", "proj-A">>, <<"s2", "proj-A">> })
    /\ lsn = 2
    /\ tombstoneLsn
      = SetAsFun({ <<<<"s1", 0>>, 1>>,
        <<<<"s1", 1>>, 2>>,
        <<<<"s1", 2>>, 0>>,
        <<<<"s1", 3>>, 0>>,
        <<<<"s2", 0>>, 0>>,
        <<<<"s2", 1>>, 0>>,
        <<<<"s2", 2>>, 0>>,
        <<<<"s2", 3>>, 0>> })
    /\ tombstoned
      = SetAsFun({ <<<<"s1", 0>>, TRUE>>,
        <<<<"s1", 1>>, TRUE>>,
        <<<<"s1", 2>>, FALSE>>,
        <<<<"s1", 3>>, FALSE>>,
        <<<<"s2", 0>>, FALSE>>,
        <<<<"s2", 1>>, FALSE>>,
        <<<<"s2", 2>>, FALSE>>,
        <<<<"s2", 3>>, FALSE>> }) *)
State4 ==
  __InLoop = TRUE
    /\ __q_temporalProps_init = FALSE
    /\ __saved___temporal_t_1 = TRUE
    /\ __saved___temporal_t_2 = TRUE
    /\ __saved_attemptedGen = 2
    /\ __saved_attemptedKind = "report"
    /\ __saved_attemptedSid = "s1"
    /\ __saved_generation = SetAsFun({ <<"s1", 2>>, <<"s2", 0>> })
    /\ __saved_identityGeneration = SetAsFun({ <<"s1", 2>>, <<"s2", 0>> })
    /\ __saved_label = SetAsFun({ <<"s1", "proj-A">>, <<"s2", "proj-A">> })
    /\ __saved_lsn = 2
    /\ __saved_tombstoneLsn
      = SetAsFun({ <<<<"s1", 0>>, 1>>,
        <<<<"s1", 1>>, 2>>,
        <<<<"s1", 2>>, 0>>,
        <<<<"s1", 3>>, 0>>,
        <<<<"s2", 0>>, 0>>,
        <<<<"s2", 1>>, 0>>,
        <<<<"s2", 2>>, 0>>,
        <<<<"s2", 3>>, 0>> })
    /\ __saved_tombstoned
      = SetAsFun({ <<<<"s1", 0>>, TRUE>>,
        <<<<"s1", 1>>, TRUE>>,
        <<<<"s1", 2>>, FALSE>>,
        <<<<"s1", 3>>, FALSE>>,
        <<<<"s2", 0>>, FALSE>>,
        <<<<"s2", 1>>, FALSE>>,
        <<<<"s2", 2>>, FALSE>>,
        <<<<"s2", 3>>, FALSE>> })
    /\ __temporal_t_1 = TRUE
    /\ __temporal_t_2 = TRUE
    /\ __temporal_t_2_unroll = TRUE
    /\ __temporal_t_2_unroll_prev = TRUE
    /\ attemptedGen = 2
    /\ attemptedKind = "report"
    /\ attemptedSid = "s1"
    /\ generation = SetAsFun({ <<"s1", 2>>, <<"s2", 0>> })
    /\ identityGeneration = SetAsFun({ <<"s1", 2>>, <<"s2", 0>> })
    /\ label = SetAsFun({ <<"s1", "proj-A">>, <<"s2", "proj-A">> })
    /\ lsn = 2
    /\ tombstoneLsn
      = SetAsFun({ <<<<"s1", 0>>, 1>>,
        <<<<"s1", 1>>, 2>>,
        <<<<"s1", 2>>, 0>>,
        <<<<"s1", 3>>, 0>>,
        <<<<"s2", 0>>, 0>>,
        <<<<"s2", 1>>, 0>>,
        <<<<"s2", 2>>, 0>>,
        <<<<"s2", 3>>, 0>> })
    /\ tombstoned
      = SetAsFun({ <<<<"s1", 0>>, TRUE>>,
        <<<<"s1", 1>>, TRUE>>,
        <<<<"s1", 2>>, FALSE>>,
        <<<<"s1", 3>>, FALSE>>,
        <<<<"s2", 0>>, FALSE>>,
        <<<<"s2", 1>>, FALSE>>,
        <<<<"s2", 2>>, FALSE>>,
        <<<<"s2", 3>>, FALSE>> })

(* The following formula holds true in the last state and violates the invariant *)
(* InvariantViolation ==
  (__InLoop
      /\ generation = __saved_generation
      /\ tombstoned = __saved_tombstoned
      /\ tombstoneLsn = __saved_tombstoneLsn
      /\ lsn = __saved_lsn
      /\ label = __saved_label
      /\ identityGeneration = __saved_identityGeneration
      /\ attemptedKind = __saved_attemptedKind
      /\ attemptedSid = __saved_attemptedSid
      /\ attemptedGen = __saved_attemptedGen
      /\ ☐(∀sid_279$2 ∈ {'s1', 's2'}: (generation[sid_279$2]' ≥ generation[sid_279$2]))
        = __saved_☐(∀sid_279$2 ∈ {'s1', 's2'}: (generation[sid_279$2]' ≥ generation[sid_279$2]))
      /\ ☐(∀sid_279$2 ∈ {'s1', 's2'}: (generation[sid_279$2]' ≥ generation[sid_279$2]))
        = __saved_☐(∀sid_279$2 ∈ {'s1', 's2'}: (generation[sid_279$2]' ≥ generation[sid_279$2]))
      /\ (~__temporal_t_2_unroll
        \/ ☐(∀sid_279$2 ∈ {'s1', 's2'}: (generation[sid_279$2]' ≥ generation[sid_279$2])))
      /\ __temporal_t_2_unroll_prev = __temporal_t_2_unroll)
    /\ ~__q::temporalProps_init *)
InvariantViolation ==
  (__InLoop
      /\ generation = __saved_generation
      /\ tombstoned = __saved_tombstoned
      /\ tombstoneLsn = __saved_tombstoneLsn
      /\ lsn = __saved_lsn
      /\ label = __saved_label
      /\ identityGeneration = __saved_identityGeneration
      /\ attemptedKind = __saved_attemptedKind
      /\ attemptedSid = __saved_attemptedSid
      /\ attemptedGen = __saved_attemptedGen
      /\ __temporal_t_1 = __saved___temporal_t_1
      /\ __temporal_t_2 = __saved___temporal_t_2
      /\ (~__temporal_t_2_unroll \/ __temporal_t_2)
      /\ __temporal_t_2_unroll_prev = __temporal_t_2_unroll)
    /\ ~__q_temporalProps_init

================================================================================
(* Created by Apalache on Sat Jul 11 11:51:13 MDT 2026 *)
(* https://github.com/apalache-mc/apalache *)
