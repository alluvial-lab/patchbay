---------------------------- MODULE counterexample ----------------------------

EXTENDS elicitation_lifecycle

(* Constant initialization state *)
ConstInit == TRUE

(* Initial state [_transition(0)] *)
(* State0 ==
  __InLoop = FALSE
    /\ __q::temporalProps_init = FALSE
    /\ __saved_☐((terminalLsn['e1'] > 0) ⇒ (((state['e1'] = firstTerminalState['e1']) ∧ (answeredBy['e1'] = firstAnsweredBy['e1'])) ∧ (answeredResponseOp['e1'] = firstAnsweredResponseOp['e1'])))
      = FALSE
    /\ __saved_☐((terminalLsn['e1'] > 0) ⇒ (((state['e1'] = firstTerminalState['e1']) ∧ (answeredBy['e1'] = firstAnsweredBy['e1'])) ∧ (answeredResponseOp['e1'] = firstAnsweredResponseOp['e1'])))
      = FALSE
    /\ __saved_answeredBy = SetAsFun({<<"e1", "none">>})
    /\ __saved_answeredResponseOp = SetAsFun({<<"e1", "none">>})
    /\ __saved_contractKind = SetAsFun({<<"e1", "approval">>})
    /\ __saved_elicitationDomain = SetAsFun({<<"e1", "domain-main">>})
    /\ __saved_firstAnsweredBy = SetAsFun({<<"e1", "none">>})
    /\ __saved_firstAnsweredResponseOp = SetAsFun({<<"e1", "none">>})
    /\ __saved_firstTerminalState = SetAsFun({<<"e1", "none">>})
    /\ __saved_lsn = 2
    /\ __saved_responderActor = SetAsFun({<<"e1", "alice">>})
    /\ __saved_responseDuplicate
      = SetAsFun({ <<"ro1", FALSE>>, <<"ro2", FALSE>> })
    /\ __saved_responseOpActor
      = SetAsFun({ <<"ro1", "alice">>, <<"ro2", "alice">> })
    /\ __saved_responseOpDomain
      = SetAsFun({ <<"ro1", "domain-main">>, <<"ro2", "domain-main">> })
    /\ __saved_responseOpElicitation
      = SetAsFun({ <<"ro1", "none">>, <<"ro2", "none">> })
    /\ __saved_responseOpEndpoint
      = SetAsFun({ <<"ro1", "none">>, <<"ro2", "none">> })
    /\ __saved_responseOpGeneration = SetAsFun({ <<"ro1", 0>>, <<"ro2", 0>> })
    /\ __saved_responseOpKind
      = SetAsFun({ <<"ro1", "none">>, <<"ro2", "none">> })
    /\ __saved_responseOpSession
      = SetAsFun({ <<"ro1", "s1">>, <<"ro2", "s1">> })
    /\ __saved_responseValid = SetAsFun({ <<"ro1", FALSE>>, <<"ro2", FALSE>> })
    /\ __saved_sessionGeneration = SetAsFun({ <<"s1", 0>>, <<"s2", 0>> })
    /\ __saved_state = SetAsFun({<<"e1", "opened">>})
    /\ __saved_targetGeneration = SetAsFun({<<"e1", 0>>})
    /\ __saved_targetSession = SetAsFun({<<"e1", "s1">>})
    /\ __saved_terminalLsn = SetAsFun({<<"e1", 0>>})
    /\ ☐((terminalLsn['e1'] > 0) ⇒ (((state['e1'] = firstTerminalState['e1']) ∧ (answeredBy['e1'] = firstAnsweredBy['e1'])) ∧ (answeredResponseOp['e1'] = firstAnsweredResponseOp['e1'])))
      = FALSE
    /\ ☐((terminalLsn['e1'] > 0) ⇒ (((state['e1'] = firstTerminalState['e1']) ∧ (answeredBy['e1'] = firstAnsweredBy['e1'])) ∧ (answeredResponseOp['e1'] = firstAnsweredResponseOp['e1'])))
      = FALSE
    /\ __temporal_t_2_unroll = TRUE
    /\ __temporal_t_2_unroll_prev = TRUE
    /\ answeredBy = SetAsFun({<<"e1", "none">>})
    /\ answeredResponseOp = SetAsFun({<<"e1", "none">>})
    /\ contractKind = SetAsFun({<<"e1", "approval">>})
    /\ elicitationDomain = SetAsFun({<<"e1", "domain-main">>})
    /\ firstAnsweredBy = SetAsFun({<<"e1", "none">>})
    /\ firstAnsweredResponseOp = SetAsFun({<<"e1", "none">>})
    /\ firstTerminalState = SetAsFun({<<"e1", "none">>})
    /\ lsn = 2
    /\ responderActor = SetAsFun({<<"e1", "alice">>})
    /\ responseDuplicate = SetAsFun({ <<"ro1", FALSE>>, <<"ro2", FALSE>> })
    /\ responseOpActor = SetAsFun({ <<"ro1", "alice">>, <<"ro2", "alice">> })
    /\ responseOpDomain
      = SetAsFun({ <<"ro1", "domain-main">>, <<"ro2", "domain-main">> })
    /\ responseOpElicitation
      = SetAsFun({ <<"ro1", "none">>, <<"ro2", "none">> })
    /\ responseOpEndpoint = SetAsFun({ <<"ro1", "none">>, <<"ro2", "none">> })
    /\ responseOpGeneration = SetAsFun({ <<"ro1", 0>>, <<"ro2", 0>> })
    /\ responseOpKind = SetAsFun({ <<"ro1", "none">>, <<"ro2", "none">> })
    /\ responseOpSession = SetAsFun({ <<"ro1", "s1">>, <<"ro2", "s1">> })
    /\ responseValid = SetAsFun({ <<"ro1", FALSE>>, <<"ro2", FALSE>> })
    /\ sessionGeneration = SetAsFun({ <<"s1", 0>>, <<"s2", 0>> })
    /\ state = SetAsFun({<<"e1", "opened">>})
    /\ targetGeneration = SetAsFun({<<"e1", 0>>})
    /\ targetSession = SetAsFun({<<"e1", "s1">>})
    /\ terminalLsn = SetAsFun({<<"e1", 0>>}) *)
State0 ==
  __InLoop = FALSE
    /\ __q_temporalProps_init = FALSE
    /\ __saved___temporal_t_1 = FALSE
    /\ __saved___temporal_t_2 = FALSE
    /\ __saved_answeredBy = SetAsFun({<<"e1", "none">>})
    /\ __saved_answeredResponseOp = SetAsFun({<<"e1", "none">>})
    /\ __saved_contractKind = SetAsFun({<<"e1", "approval">>})
    /\ __saved_elicitationDomain = SetAsFun({<<"e1", "domain-main">>})
    /\ __saved_firstAnsweredBy = SetAsFun({<<"e1", "none">>})
    /\ __saved_firstAnsweredResponseOp = SetAsFun({<<"e1", "none">>})
    /\ __saved_firstTerminalState = SetAsFun({<<"e1", "none">>})
    /\ __saved_lsn = 2
    /\ __saved_responderActor = SetAsFun({<<"e1", "alice">>})
    /\ __saved_responseDuplicate
      = SetAsFun({ <<"ro1", FALSE>>, <<"ro2", FALSE>> })
    /\ __saved_responseOpActor
      = SetAsFun({ <<"ro1", "alice">>, <<"ro2", "alice">> })
    /\ __saved_responseOpDomain
      = SetAsFun({ <<"ro1", "domain-main">>, <<"ro2", "domain-main">> })
    /\ __saved_responseOpElicitation
      = SetAsFun({ <<"ro1", "none">>, <<"ro2", "none">> })
    /\ __saved_responseOpEndpoint
      = SetAsFun({ <<"ro1", "none">>, <<"ro2", "none">> })
    /\ __saved_responseOpGeneration = SetAsFun({ <<"ro1", 0>>, <<"ro2", 0>> })
    /\ __saved_responseOpKind
      = SetAsFun({ <<"ro1", "none">>, <<"ro2", "none">> })
    /\ __saved_responseOpSession
      = SetAsFun({ <<"ro1", "s1">>, <<"ro2", "s1">> })
    /\ __saved_responseValid = SetAsFun({ <<"ro1", FALSE>>, <<"ro2", FALSE>> })
    /\ __saved_sessionGeneration = SetAsFun({ <<"s1", 0>>, <<"s2", 0>> })
    /\ __saved_state = SetAsFun({<<"e1", "opened">>})
    /\ __saved_targetGeneration = SetAsFun({<<"e1", 0>>})
    /\ __saved_targetSession = SetAsFun({<<"e1", "s1">>})
    /\ __saved_terminalLsn = SetAsFun({<<"e1", 0>>})
    /\ __temporal_t_1 = FALSE
    /\ __temporal_t_2 = FALSE
    /\ __temporal_t_2_unroll = TRUE
    /\ __temporal_t_2_unroll_prev = TRUE
    /\ answeredBy = SetAsFun({<<"e1", "none">>})
    /\ answeredResponseOp = SetAsFun({<<"e1", "none">>})
    /\ contractKind = SetAsFun({<<"e1", "approval">>})
    /\ elicitationDomain = SetAsFun({<<"e1", "domain-main">>})
    /\ firstAnsweredBy = SetAsFun({<<"e1", "none">>})
    /\ firstAnsweredResponseOp = SetAsFun({<<"e1", "none">>})
    /\ firstTerminalState = SetAsFun({<<"e1", "none">>})
    /\ lsn = 2
    /\ responderActor = SetAsFun({<<"e1", "alice">>})
    /\ responseDuplicate = SetAsFun({ <<"ro1", FALSE>>, <<"ro2", FALSE>> })
    /\ responseOpActor = SetAsFun({ <<"ro1", "alice">>, <<"ro2", "alice">> })
    /\ responseOpDomain
      = SetAsFun({ <<"ro1", "domain-main">>, <<"ro2", "domain-main">> })
    /\ responseOpElicitation
      = SetAsFun({ <<"ro1", "none">>, <<"ro2", "none">> })
    /\ responseOpEndpoint = SetAsFun({ <<"ro1", "none">>, <<"ro2", "none">> })
    /\ responseOpGeneration = SetAsFun({ <<"ro1", 0>>, <<"ro2", 0>> })
    /\ responseOpKind = SetAsFun({ <<"ro1", "none">>, <<"ro2", "none">> })
    /\ responseOpSession = SetAsFun({ <<"ro1", "s1">>, <<"ro2", "s1">> })
    /\ responseValid = SetAsFun({ <<"ro1", FALSE>>, <<"ro2", FALSE>> })
    /\ sessionGeneration = SetAsFun({ <<"s1", 0>>, <<"s2", 0>> })
    /\ state = SetAsFun({<<"e1", "opened">>})
    /\ targetGeneration = SetAsFun({<<"e1", 0>>})
    /\ targetSession = SetAsFun({<<"e1", "s1">>})
    /\ terminalLsn = SetAsFun({<<"e1", 0>>})

(* State1 [_transition(0)] *)
(* State1 ==
  __InLoop = FALSE
    /\ __q::temporalProps_init = FALSE
    /\ __saved_☐((terminalLsn['e1'] > 0) ⇒ (((state['e1'] = firstTerminalState['e1']) ∧ (answeredBy['e1'] = firstAnsweredBy['e1'])) ∧ (answeredResponseOp['e1'] = firstAnsweredResponseOp['e1'])))
      = FALSE
    /\ __saved_☐((terminalLsn['e1'] > 0) ⇒ (((state['e1'] = firstTerminalState['e1']) ∧ (answeredBy['e1'] = firstAnsweredBy['e1'])) ∧ (answeredResponseOp['e1'] = firstAnsweredResponseOp['e1'])))
      = FALSE
    /\ __saved_answeredBy = SetAsFun({<<"e1", "none">>})
    /\ __saved_answeredResponseOp = SetAsFun({<<"e1", "none">>})
    /\ __saved_contractKind = SetAsFun({<<"e1", "approval">>})
    /\ __saved_elicitationDomain = SetAsFun({<<"e1", "domain-main">>})
    /\ __saved_firstAnsweredBy = SetAsFun({<<"e1", "none">>})
    /\ __saved_firstAnsweredResponseOp = SetAsFun({<<"e1", "none">>})
    /\ __saved_firstTerminalState = SetAsFun({<<"e1", "none">>})
    /\ __saved_lsn = 2
    /\ __saved_responderActor = SetAsFun({<<"e1", "alice">>})
    /\ __saved_responseDuplicate
      = SetAsFun({ <<"ro1", FALSE>>, <<"ro2", FALSE>> })
    /\ __saved_responseOpActor
      = SetAsFun({ <<"ro1", "alice">>, <<"ro2", "alice">> })
    /\ __saved_responseOpDomain
      = SetAsFun({ <<"ro1", "domain-main">>, <<"ro2", "domain-main">> })
    /\ __saved_responseOpElicitation
      = SetAsFun({ <<"ro1", "none">>, <<"ro2", "none">> })
    /\ __saved_responseOpEndpoint
      = SetAsFun({ <<"ro1", "none">>, <<"ro2", "none">> })
    /\ __saved_responseOpGeneration = SetAsFun({ <<"ro1", 0>>, <<"ro2", 0>> })
    /\ __saved_responseOpKind
      = SetAsFun({ <<"ro1", "none">>, <<"ro2", "none">> })
    /\ __saved_responseOpSession
      = SetAsFun({ <<"ro1", "s1">>, <<"ro2", "s1">> })
    /\ __saved_responseValid = SetAsFun({ <<"ro1", FALSE>>, <<"ro2", FALSE>> })
    /\ __saved_sessionGeneration = SetAsFun({ <<"s1", 0>>, <<"s2", 0>> })
    /\ __saved_state = SetAsFun({<<"e1", "opened">>})
    /\ __saved_targetGeneration = SetAsFun({<<"e1", 0>>})
    /\ __saved_targetSession = SetAsFun({<<"e1", "s1">>})
    /\ __saved_terminalLsn = SetAsFun({<<"e1", 0>>})
    /\ ☐((terminalLsn['e1'] > 0) ⇒ (((state['e1'] = firstTerminalState['e1']) ∧ (answeredBy['e1'] = firstAnsweredBy['e1'])) ∧ (answeredResponseOp['e1'] = firstAnsweredResponseOp['e1'])))
      = FALSE
    /\ ☐((terminalLsn['e1'] > 0) ⇒ (((state['e1'] = firstTerminalState['e1']) ∧ (answeredBy['e1'] = firstAnsweredBy['e1'])) ∧ (answeredResponseOp['e1'] = firstAnsweredResponseOp['e1'])))
      = FALSE
    /\ __temporal_t_2_unroll = TRUE
    /\ __temporal_t_2_unroll_prev = TRUE
    /\ answeredBy = SetAsFun({<<"e1", "none">>})
    /\ answeredResponseOp = SetAsFun({<<"e1", "none">>})
    /\ contractKind = SetAsFun({<<"e1", "approval">>})
    /\ elicitationDomain = SetAsFun({<<"e1", "domain-main">>})
    /\ firstAnsweredBy = SetAsFun({<<"e1", "none">>})
    /\ firstAnsweredResponseOp = SetAsFun({<<"e1", "none">>})
    /\ firstTerminalState = SetAsFun({<<"e1", "none">>})
    /\ lsn = 3
    /\ responderActor = SetAsFun({<<"e1", "alice">>})
    /\ responseDuplicate = SetAsFun({ <<"ro1", FALSE>>, <<"ro2", FALSE>> })
    /\ responseOpActor = SetAsFun({ <<"ro1", "alice">>, <<"ro2", "alice">> })
    /\ responseOpDomain
      = SetAsFun({ <<"ro1", "domain-main">>, <<"ro2", "domain-main">> })
    /\ responseOpElicitation
      = SetAsFun({ <<"ro1", "none">>, <<"ro2", "none">> })
    /\ responseOpEndpoint = SetAsFun({ <<"ro1", "none">>, <<"ro2", "none">> })
    /\ responseOpGeneration = SetAsFun({ <<"ro1", 0>>, <<"ro2", 0>> })
    /\ responseOpKind = SetAsFun({ <<"ro1", "none">>, <<"ro2", "none">> })
    /\ responseOpSession = SetAsFun({ <<"ro1", "s1">>, <<"ro2", "s1">> })
    /\ responseValid = SetAsFun({ <<"ro1", FALSE>>, <<"ro2", FALSE>> })
    /\ sessionGeneration = SetAsFun({ <<"s1", 0>>, <<"s2", 0>> })
    /\ state = SetAsFun({<<"e1", "pending">>})
    /\ targetGeneration = SetAsFun({<<"e1", 0>>})
    /\ targetSession = SetAsFun({<<"e1", "s1">>})
    /\ terminalLsn = SetAsFun({<<"e1", 0>>}) *)
State1 ==
  __InLoop = FALSE
    /\ __q_temporalProps_init = FALSE
    /\ __saved___temporal_t_1 = FALSE
    /\ __saved___temporal_t_2 = FALSE
    /\ __saved_answeredBy = SetAsFun({<<"e1", "none">>})
    /\ __saved_answeredResponseOp = SetAsFun({<<"e1", "none">>})
    /\ __saved_contractKind = SetAsFun({<<"e1", "approval">>})
    /\ __saved_elicitationDomain = SetAsFun({<<"e1", "domain-main">>})
    /\ __saved_firstAnsweredBy = SetAsFun({<<"e1", "none">>})
    /\ __saved_firstAnsweredResponseOp = SetAsFun({<<"e1", "none">>})
    /\ __saved_firstTerminalState = SetAsFun({<<"e1", "none">>})
    /\ __saved_lsn = 2
    /\ __saved_responderActor = SetAsFun({<<"e1", "alice">>})
    /\ __saved_responseDuplicate
      = SetAsFun({ <<"ro1", FALSE>>, <<"ro2", FALSE>> })
    /\ __saved_responseOpActor
      = SetAsFun({ <<"ro1", "alice">>, <<"ro2", "alice">> })
    /\ __saved_responseOpDomain
      = SetAsFun({ <<"ro1", "domain-main">>, <<"ro2", "domain-main">> })
    /\ __saved_responseOpElicitation
      = SetAsFun({ <<"ro1", "none">>, <<"ro2", "none">> })
    /\ __saved_responseOpEndpoint
      = SetAsFun({ <<"ro1", "none">>, <<"ro2", "none">> })
    /\ __saved_responseOpGeneration = SetAsFun({ <<"ro1", 0>>, <<"ro2", 0>> })
    /\ __saved_responseOpKind
      = SetAsFun({ <<"ro1", "none">>, <<"ro2", "none">> })
    /\ __saved_responseOpSession
      = SetAsFun({ <<"ro1", "s1">>, <<"ro2", "s1">> })
    /\ __saved_responseValid = SetAsFun({ <<"ro1", FALSE>>, <<"ro2", FALSE>> })
    /\ __saved_sessionGeneration = SetAsFun({ <<"s1", 0>>, <<"s2", 0>> })
    /\ __saved_state = SetAsFun({<<"e1", "opened">>})
    /\ __saved_targetGeneration = SetAsFun({<<"e1", 0>>})
    /\ __saved_targetSession = SetAsFun({<<"e1", "s1">>})
    /\ __saved_terminalLsn = SetAsFun({<<"e1", 0>>})
    /\ __temporal_t_1 = FALSE
    /\ __temporal_t_2 = FALSE
    /\ __temporal_t_2_unroll = TRUE
    /\ __temporal_t_2_unroll_prev = TRUE
    /\ answeredBy = SetAsFun({<<"e1", "none">>})
    /\ answeredResponseOp = SetAsFun({<<"e1", "none">>})
    /\ contractKind = SetAsFun({<<"e1", "approval">>})
    /\ elicitationDomain = SetAsFun({<<"e1", "domain-main">>})
    /\ firstAnsweredBy = SetAsFun({<<"e1", "none">>})
    /\ firstAnsweredResponseOp = SetAsFun({<<"e1", "none">>})
    /\ firstTerminalState = SetAsFun({<<"e1", "none">>})
    /\ lsn = 3
    /\ responderActor = SetAsFun({<<"e1", "alice">>})
    /\ responseDuplicate = SetAsFun({ <<"ro1", FALSE>>, <<"ro2", FALSE>> })
    /\ responseOpActor = SetAsFun({ <<"ro1", "alice">>, <<"ro2", "alice">> })
    /\ responseOpDomain
      = SetAsFun({ <<"ro1", "domain-main">>, <<"ro2", "domain-main">> })
    /\ responseOpElicitation
      = SetAsFun({ <<"ro1", "none">>, <<"ro2", "none">> })
    /\ responseOpEndpoint = SetAsFun({ <<"ro1", "none">>, <<"ro2", "none">> })
    /\ responseOpGeneration = SetAsFun({ <<"ro1", 0>>, <<"ro2", 0>> })
    /\ responseOpKind = SetAsFun({ <<"ro1", "none">>, <<"ro2", "none">> })
    /\ responseOpSession = SetAsFun({ <<"ro1", "s1">>, <<"ro2", "s1">> })
    /\ responseValid = SetAsFun({ <<"ro1", FALSE>>, <<"ro2", FALSE>> })
    /\ sessionGeneration = SetAsFun({ <<"s1", 0>>, <<"s2", 0>> })
    /\ state = SetAsFun({<<"e1", "pending">>})
    /\ targetGeneration = SetAsFun({<<"e1", 0>>})
    /\ targetSession = SetAsFun({<<"e1", "s1">>})
    /\ terminalLsn = SetAsFun({<<"e1", 0>>})

(* State2 [_transition(4)] *)
(* State2 ==
  __InLoop = FALSE
    /\ __q::temporalProps_init = FALSE
    /\ __saved_☐((terminalLsn['e1'] > 0) ⇒ (((state['e1'] = firstTerminalState['e1']) ∧ (answeredBy['e1'] = firstAnsweredBy['e1'])) ∧ (answeredResponseOp['e1'] = firstAnsweredResponseOp['e1'])))
      = FALSE
    /\ __saved_☐((terminalLsn['e1'] > 0) ⇒ (((state['e1'] = firstTerminalState['e1']) ∧ (answeredBy['e1'] = firstAnsweredBy['e1'])) ∧ (answeredResponseOp['e1'] = firstAnsweredResponseOp['e1'])))
      = FALSE
    /\ __saved_answeredBy = SetAsFun({<<"e1", "none">>})
    /\ __saved_answeredResponseOp = SetAsFun({<<"e1", "none">>})
    /\ __saved_contractKind = SetAsFun({<<"e1", "approval">>})
    /\ __saved_elicitationDomain = SetAsFun({<<"e1", "domain-main">>})
    /\ __saved_firstAnsweredBy = SetAsFun({<<"e1", "none">>})
    /\ __saved_firstAnsweredResponseOp = SetAsFun({<<"e1", "none">>})
    /\ __saved_firstTerminalState = SetAsFun({<<"e1", "none">>})
    /\ __saved_lsn = 2
    /\ __saved_responderActor = SetAsFun({<<"e1", "alice">>})
    /\ __saved_responseDuplicate
      = SetAsFun({ <<"ro1", FALSE>>, <<"ro2", FALSE>> })
    /\ __saved_responseOpActor
      = SetAsFun({ <<"ro1", "alice">>, <<"ro2", "alice">> })
    /\ __saved_responseOpDomain
      = SetAsFun({ <<"ro1", "domain-main">>, <<"ro2", "domain-main">> })
    /\ __saved_responseOpElicitation
      = SetAsFun({ <<"ro1", "none">>, <<"ro2", "none">> })
    /\ __saved_responseOpEndpoint
      = SetAsFun({ <<"ro1", "none">>, <<"ro2", "none">> })
    /\ __saved_responseOpGeneration = SetAsFun({ <<"ro1", 0>>, <<"ro2", 0>> })
    /\ __saved_responseOpKind
      = SetAsFun({ <<"ro1", "none">>, <<"ro2", "none">> })
    /\ __saved_responseOpSession
      = SetAsFun({ <<"ro1", "s1">>, <<"ro2", "s1">> })
    /\ __saved_responseValid = SetAsFun({ <<"ro1", FALSE>>, <<"ro2", FALSE>> })
    /\ __saved_sessionGeneration = SetAsFun({ <<"s1", 0>>, <<"s2", 0>> })
    /\ __saved_state = SetAsFun({<<"e1", "opened">>})
    /\ __saved_targetGeneration = SetAsFun({<<"e1", 0>>})
    /\ __saved_targetSession = SetAsFun({<<"e1", "s1">>})
    /\ __saved_terminalLsn = SetAsFun({<<"e1", 0>>})
    /\ ☐((terminalLsn['e1'] > 0) ⇒ (((state['e1'] = firstTerminalState['e1']) ∧ (answeredBy['e1'] = firstAnsweredBy['e1'])) ∧ (answeredResponseOp['e1'] = firstAnsweredResponseOp['e1'])))
      = FALSE
    /\ ☐((terminalLsn['e1'] > 0) ⇒ (((state['e1'] = firstTerminalState['e1']) ∧ (answeredBy['e1'] = firstAnsweredBy['e1'])) ∧ (answeredResponseOp['e1'] = firstAnsweredResponseOp['e1'])))
      = FALSE
    /\ __temporal_t_2_unroll = TRUE
    /\ __temporal_t_2_unroll_prev = TRUE
    /\ answeredBy = SetAsFun({<<"e1", "ep-a">>})
    /\ answeredResponseOp = SetAsFun({<<"e1", "ro1">>})
    /\ contractKind = SetAsFun({<<"e1", "approval">>})
    /\ elicitationDomain = SetAsFun({<<"e1", "domain-main">>})
    /\ firstAnsweredBy = SetAsFun({<<"e1", "ep-a">>})
    /\ firstAnsweredResponseOp = SetAsFun({<<"e1", "ro1">>})
    /\ firstTerminalState = SetAsFun({<<"e1", "answered">>})
    /\ lsn = 4
    /\ responderActor = SetAsFun({<<"e1", "alice">>})
    /\ responseDuplicate = SetAsFun({ <<"ro1", FALSE>>, <<"ro2", FALSE>> })
    /\ responseOpActor = SetAsFun({ <<"ro1", "alice">>, <<"ro2", "alice">> })
    /\ responseOpDomain
      = SetAsFun({ <<"ro1", "domain-main">>, <<"ro2", "domain-main">> })
    /\ responseOpElicitation = SetAsFun({ <<"ro1", "e1">>, <<"ro2", "none">> })
    /\ responseOpEndpoint = SetAsFun({ <<"ro1", "ep-a">>, <<"ro2", "none">> })
    /\ responseOpGeneration = SetAsFun({ <<"ro1", 0>>, <<"ro2", 0>> })
    /\ responseOpKind
      = SetAsFun({ <<"ro1", "approval-response">>, <<"ro2", "none">> })
    /\ responseOpSession = SetAsFun({ <<"ro1", "s1">>, <<"ro2", "s1">> })
    /\ responseValid = SetAsFun({ <<"ro1", TRUE>>, <<"ro2", FALSE>> })
    /\ sessionGeneration = SetAsFun({ <<"s1", 0>>, <<"s2", 0>> })
    /\ state = SetAsFun({<<"e1", "answered">>})
    /\ targetGeneration = SetAsFun({<<"e1", 0>>})
    /\ targetSession = SetAsFun({<<"e1", "s1">>})
    /\ terminalLsn = SetAsFun({<<"e1", 4>>}) *)
State2 ==
  __InLoop = FALSE
    /\ __q_temporalProps_init = FALSE
    /\ __saved___temporal_t_1 = FALSE
    /\ __saved___temporal_t_2 = FALSE
    /\ __saved_answeredBy = SetAsFun({<<"e1", "none">>})
    /\ __saved_answeredResponseOp = SetAsFun({<<"e1", "none">>})
    /\ __saved_contractKind = SetAsFun({<<"e1", "approval">>})
    /\ __saved_elicitationDomain = SetAsFun({<<"e1", "domain-main">>})
    /\ __saved_firstAnsweredBy = SetAsFun({<<"e1", "none">>})
    /\ __saved_firstAnsweredResponseOp = SetAsFun({<<"e1", "none">>})
    /\ __saved_firstTerminalState = SetAsFun({<<"e1", "none">>})
    /\ __saved_lsn = 2
    /\ __saved_responderActor = SetAsFun({<<"e1", "alice">>})
    /\ __saved_responseDuplicate
      = SetAsFun({ <<"ro1", FALSE>>, <<"ro2", FALSE>> })
    /\ __saved_responseOpActor
      = SetAsFun({ <<"ro1", "alice">>, <<"ro2", "alice">> })
    /\ __saved_responseOpDomain
      = SetAsFun({ <<"ro1", "domain-main">>, <<"ro2", "domain-main">> })
    /\ __saved_responseOpElicitation
      = SetAsFun({ <<"ro1", "none">>, <<"ro2", "none">> })
    /\ __saved_responseOpEndpoint
      = SetAsFun({ <<"ro1", "none">>, <<"ro2", "none">> })
    /\ __saved_responseOpGeneration = SetAsFun({ <<"ro1", 0>>, <<"ro2", 0>> })
    /\ __saved_responseOpKind
      = SetAsFun({ <<"ro1", "none">>, <<"ro2", "none">> })
    /\ __saved_responseOpSession
      = SetAsFun({ <<"ro1", "s1">>, <<"ro2", "s1">> })
    /\ __saved_responseValid = SetAsFun({ <<"ro1", FALSE>>, <<"ro2", FALSE>> })
    /\ __saved_sessionGeneration = SetAsFun({ <<"s1", 0>>, <<"s2", 0>> })
    /\ __saved_state = SetAsFun({<<"e1", "opened">>})
    /\ __saved_targetGeneration = SetAsFun({<<"e1", 0>>})
    /\ __saved_targetSession = SetAsFun({<<"e1", "s1">>})
    /\ __saved_terminalLsn = SetAsFun({<<"e1", 0>>})
    /\ __temporal_t_1 = FALSE
    /\ __temporal_t_2 = FALSE
    /\ __temporal_t_2_unroll = TRUE
    /\ __temporal_t_2_unroll_prev = TRUE
    /\ answeredBy = SetAsFun({<<"e1", "ep-a">>})
    /\ answeredResponseOp = SetAsFun({<<"e1", "ro1">>})
    /\ contractKind = SetAsFun({<<"e1", "approval">>})
    /\ elicitationDomain = SetAsFun({<<"e1", "domain-main">>})
    /\ firstAnsweredBy = SetAsFun({<<"e1", "ep-a">>})
    /\ firstAnsweredResponseOp = SetAsFun({<<"e1", "ro1">>})
    /\ firstTerminalState = SetAsFun({<<"e1", "answered">>})
    /\ lsn = 4
    /\ responderActor = SetAsFun({<<"e1", "alice">>})
    /\ responseDuplicate = SetAsFun({ <<"ro1", FALSE>>, <<"ro2", FALSE>> })
    /\ responseOpActor = SetAsFun({ <<"ro1", "alice">>, <<"ro2", "alice">> })
    /\ responseOpDomain
      = SetAsFun({ <<"ro1", "domain-main">>, <<"ro2", "domain-main">> })
    /\ responseOpElicitation = SetAsFun({ <<"ro1", "e1">>, <<"ro2", "none">> })
    /\ responseOpEndpoint = SetAsFun({ <<"ro1", "ep-a">>, <<"ro2", "none">> })
    /\ responseOpGeneration = SetAsFun({ <<"ro1", 0>>, <<"ro2", 0>> })
    /\ responseOpKind
      = SetAsFun({ <<"ro1", "approval-response">>, <<"ro2", "none">> })
    /\ responseOpSession = SetAsFun({ <<"ro1", "s1">>, <<"ro2", "s1">> })
    /\ responseValid = SetAsFun({ <<"ro1", TRUE>>, <<"ro2", FALSE>> })
    /\ sessionGeneration = SetAsFun({ <<"s1", 0>>, <<"s2", 0>> })
    /\ state = SetAsFun({<<"e1", "answered">>})
    /\ targetGeneration = SetAsFun({<<"e1", 0>>})
    /\ targetSession = SetAsFun({<<"e1", "s1">>})
    /\ terminalLsn = SetAsFun({<<"e1", 4>>})

(* State3 [_transition(16)] *)
(* State3 ==
  __InLoop = FALSE
    /\ __q::temporalProps_init = FALSE
    /\ __saved_☐((terminalLsn['e1'] > 0) ⇒ (((state['e1'] = firstTerminalState['e1']) ∧ (answeredBy['e1'] = firstAnsweredBy['e1'])) ∧ (answeredResponseOp['e1'] = firstAnsweredResponseOp['e1'])))
      = FALSE
    /\ __saved_☐((terminalLsn['e1'] > 0) ⇒ (((state['e1'] = firstTerminalState['e1']) ∧ (answeredBy['e1'] = firstAnsweredBy['e1'])) ∧ (answeredResponseOp['e1'] = firstAnsweredResponseOp['e1'])))
      = FALSE
    /\ __saved_answeredBy = SetAsFun({<<"e1", "none">>})
    /\ __saved_answeredResponseOp = SetAsFun({<<"e1", "none">>})
    /\ __saved_contractKind = SetAsFun({<<"e1", "approval">>})
    /\ __saved_elicitationDomain = SetAsFun({<<"e1", "domain-main">>})
    /\ __saved_firstAnsweredBy = SetAsFun({<<"e1", "none">>})
    /\ __saved_firstAnsweredResponseOp = SetAsFun({<<"e1", "none">>})
    /\ __saved_firstTerminalState = SetAsFun({<<"e1", "none">>})
    /\ __saved_lsn = 2
    /\ __saved_responderActor = SetAsFun({<<"e1", "alice">>})
    /\ __saved_responseDuplicate
      = SetAsFun({ <<"ro1", FALSE>>, <<"ro2", FALSE>> })
    /\ __saved_responseOpActor
      = SetAsFun({ <<"ro1", "alice">>, <<"ro2", "alice">> })
    /\ __saved_responseOpDomain
      = SetAsFun({ <<"ro1", "domain-main">>, <<"ro2", "domain-main">> })
    /\ __saved_responseOpElicitation
      = SetAsFun({ <<"ro1", "none">>, <<"ro2", "none">> })
    /\ __saved_responseOpEndpoint
      = SetAsFun({ <<"ro1", "none">>, <<"ro2", "none">> })
    /\ __saved_responseOpGeneration = SetAsFun({ <<"ro1", 0>>, <<"ro2", 0>> })
    /\ __saved_responseOpKind
      = SetAsFun({ <<"ro1", "none">>, <<"ro2", "none">> })
    /\ __saved_responseOpSession
      = SetAsFun({ <<"ro1", "s1">>, <<"ro2", "s1">> })
    /\ __saved_responseValid = SetAsFun({ <<"ro1", FALSE>>, <<"ro2", FALSE>> })
    /\ __saved_sessionGeneration = SetAsFun({ <<"s1", 0>>, <<"s2", 0>> })
    /\ __saved_state = SetAsFun({<<"e1", "opened">>})
    /\ __saved_targetGeneration = SetAsFun({<<"e1", 0>>})
    /\ __saved_targetSession = SetAsFun({<<"e1", "s1">>})
    /\ __saved_terminalLsn = SetAsFun({<<"e1", 0>>})
    /\ ☐((terminalLsn['e1'] > 0) ⇒ (((state['e1'] = firstTerminalState['e1']) ∧ (answeredBy['e1'] = firstAnsweredBy['e1'])) ∧ (answeredResponseOp['e1'] = firstAnsweredResponseOp['e1'])))
      = FALSE
    /\ ☐((terminalLsn['e1'] > 0) ⇒ (((state['e1'] = firstTerminalState['e1']) ∧ (answeredBy['e1'] = firstAnsweredBy['e1'])) ∧ (answeredResponseOp['e1'] = firstAnsweredResponseOp['e1'])))
      = FALSE
    /\ __temporal_t_2_unroll = FALSE
    /\ __temporal_t_2_unroll_prev = TRUE
    /\ answeredBy = SetAsFun({<<"e1", "ep-a">>})
    /\ answeredResponseOp = SetAsFun({<<"e1", "ro1">>})
    /\ contractKind = SetAsFun({<<"e1", "approval">>})
    /\ elicitationDomain = SetAsFun({<<"e1", "domain-main">>})
    /\ firstAnsweredBy = SetAsFun({<<"e1", "ep-a">>})
    /\ firstAnsweredResponseOp = SetAsFun({<<"e1", "ro1">>})
    /\ firstTerminalState = SetAsFun({<<"e1", "answered">>})
    /\ lsn = 4
    /\ responderActor = SetAsFun({<<"e1", "alice">>})
    /\ responseDuplicate = SetAsFun({ <<"ro1", FALSE>>, <<"ro2", FALSE>> })
    /\ responseOpActor = SetAsFun({ <<"ro1", "alice">>, <<"ro2", "alice">> })
    /\ responseOpDomain
      = SetAsFun({ <<"ro1", "domain-main">>, <<"ro2", "domain-main">> })
    /\ responseOpElicitation = SetAsFun({ <<"ro1", "e1">>, <<"ro2", "none">> })
    /\ responseOpEndpoint = SetAsFun({ <<"ro1", "ep-a">>, <<"ro2", "none">> })
    /\ responseOpGeneration = SetAsFun({ <<"ro1", 0>>, <<"ro2", 0>> })
    /\ responseOpKind
      = SetAsFun({ <<"ro1", "approval-response">>, <<"ro2", "none">> })
    /\ responseOpSession = SetAsFun({ <<"ro1", "s1">>, <<"ro2", "s1">> })
    /\ responseValid = SetAsFun({ <<"ro1", TRUE>>, <<"ro2", FALSE>> })
    /\ sessionGeneration = SetAsFun({ <<"s1", 0>>, <<"s2", 0>> })
    /\ state = SetAsFun({<<"e1", "expired">>})
    /\ targetGeneration = SetAsFun({<<"e1", 0>>})
    /\ targetSession = SetAsFun({<<"e1", "s1">>})
    /\ terminalLsn = SetAsFun({<<"e1", 4>>}) *)
State3 ==
  __InLoop = FALSE
    /\ __q_temporalProps_init = FALSE
    /\ __saved___temporal_t_1 = FALSE
    /\ __saved___temporal_t_2 = FALSE
    /\ __saved_answeredBy = SetAsFun({<<"e1", "none">>})
    /\ __saved_answeredResponseOp = SetAsFun({<<"e1", "none">>})
    /\ __saved_contractKind = SetAsFun({<<"e1", "approval">>})
    /\ __saved_elicitationDomain = SetAsFun({<<"e1", "domain-main">>})
    /\ __saved_firstAnsweredBy = SetAsFun({<<"e1", "none">>})
    /\ __saved_firstAnsweredResponseOp = SetAsFun({<<"e1", "none">>})
    /\ __saved_firstTerminalState = SetAsFun({<<"e1", "none">>})
    /\ __saved_lsn = 2
    /\ __saved_responderActor = SetAsFun({<<"e1", "alice">>})
    /\ __saved_responseDuplicate
      = SetAsFun({ <<"ro1", FALSE>>, <<"ro2", FALSE>> })
    /\ __saved_responseOpActor
      = SetAsFun({ <<"ro1", "alice">>, <<"ro2", "alice">> })
    /\ __saved_responseOpDomain
      = SetAsFun({ <<"ro1", "domain-main">>, <<"ro2", "domain-main">> })
    /\ __saved_responseOpElicitation
      = SetAsFun({ <<"ro1", "none">>, <<"ro2", "none">> })
    /\ __saved_responseOpEndpoint
      = SetAsFun({ <<"ro1", "none">>, <<"ro2", "none">> })
    /\ __saved_responseOpGeneration = SetAsFun({ <<"ro1", 0>>, <<"ro2", 0>> })
    /\ __saved_responseOpKind
      = SetAsFun({ <<"ro1", "none">>, <<"ro2", "none">> })
    /\ __saved_responseOpSession
      = SetAsFun({ <<"ro1", "s1">>, <<"ro2", "s1">> })
    /\ __saved_responseValid = SetAsFun({ <<"ro1", FALSE>>, <<"ro2", FALSE>> })
    /\ __saved_sessionGeneration = SetAsFun({ <<"s1", 0>>, <<"s2", 0>> })
    /\ __saved_state = SetAsFun({<<"e1", "opened">>})
    /\ __saved_targetGeneration = SetAsFun({<<"e1", 0>>})
    /\ __saved_targetSession = SetAsFun({<<"e1", "s1">>})
    /\ __saved_terminalLsn = SetAsFun({<<"e1", 0>>})
    /\ __temporal_t_1 = FALSE
    /\ __temporal_t_2 = FALSE
    /\ __temporal_t_2_unroll = FALSE
    /\ __temporal_t_2_unroll_prev = TRUE
    /\ answeredBy = SetAsFun({<<"e1", "ep-a">>})
    /\ answeredResponseOp = SetAsFun({<<"e1", "ro1">>})
    /\ contractKind = SetAsFun({<<"e1", "approval">>})
    /\ elicitationDomain = SetAsFun({<<"e1", "domain-main">>})
    /\ firstAnsweredBy = SetAsFun({<<"e1", "ep-a">>})
    /\ firstAnsweredResponseOp = SetAsFun({<<"e1", "ro1">>})
    /\ firstTerminalState = SetAsFun({<<"e1", "answered">>})
    /\ lsn = 4
    /\ responderActor = SetAsFun({<<"e1", "alice">>})
    /\ responseDuplicate = SetAsFun({ <<"ro1", FALSE>>, <<"ro2", FALSE>> })
    /\ responseOpActor = SetAsFun({ <<"ro1", "alice">>, <<"ro2", "alice">> })
    /\ responseOpDomain
      = SetAsFun({ <<"ro1", "domain-main">>, <<"ro2", "domain-main">> })
    /\ responseOpElicitation = SetAsFun({ <<"ro1", "e1">>, <<"ro2", "none">> })
    /\ responseOpEndpoint = SetAsFun({ <<"ro1", "ep-a">>, <<"ro2", "none">> })
    /\ responseOpGeneration = SetAsFun({ <<"ro1", 0>>, <<"ro2", 0>> })
    /\ responseOpKind
      = SetAsFun({ <<"ro1", "approval-response">>, <<"ro2", "none">> })
    /\ responseOpSession = SetAsFun({ <<"ro1", "s1">>, <<"ro2", "s1">> })
    /\ responseValid = SetAsFun({ <<"ro1", TRUE>>, <<"ro2", FALSE>> })
    /\ sessionGeneration = SetAsFun({ <<"s1", 0>>, <<"s2", 0>> })
    /\ state = SetAsFun({<<"e1", "expired">>})
    /\ targetGeneration = SetAsFun({<<"e1", 0>>})
    /\ targetSession = SetAsFun({<<"e1", "s1">>})
    /\ terminalLsn = SetAsFun({<<"e1", 4>>})

(* State4 [_transition(11)] *)
(* State4 ==
  __InLoop = TRUE
    /\ __q::temporalProps_init = FALSE
    /\ __saved_☐((terminalLsn['e1'] > 0) ⇒ (((state['e1'] = firstTerminalState['e1']) ∧ (answeredBy['e1'] = firstAnsweredBy['e1'])) ∧ (answeredResponseOp['e1'] = firstAnsweredResponseOp['e1'])))
      = FALSE
    /\ __saved_☐((terminalLsn['e1'] > 0) ⇒ (((state['e1'] = firstTerminalState['e1']) ∧ (answeredBy['e1'] = firstAnsweredBy['e1'])) ∧ (answeredResponseOp['e1'] = firstAnsweredResponseOp['e1'])))
      = FALSE
    /\ __saved_answeredBy = SetAsFun({<<"e1", "ep-a">>})
    /\ __saved_answeredResponseOp = SetAsFun({<<"e1", "ro1">>})
    /\ __saved_contractKind = SetAsFun({<<"e1", "approval">>})
    /\ __saved_elicitationDomain = SetAsFun({<<"e1", "domain-main">>})
    /\ __saved_firstAnsweredBy = SetAsFun({<<"e1", "ep-a">>})
    /\ __saved_firstAnsweredResponseOp = SetAsFun({<<"e1", "ro1">>})
    /\ __saved_firstTerminalState = SetAsFun({<<"e1", "answered">>})
    /\ __saved_lsn = 4
    /\ __saved_responderActor = SetAsFun({<<"e1", "alice">>})
    /\ __saved_responseDuplicate
      = SetAsFun({ <<"ro1", FALSE>>, <<"ro2", FALSE>> })
    /\ __saved_responseOpActor
      = SetAsFun({ <<"ro1", "alice">>, <<"ro2", "alice">> })
    /\ __saved_responseOpDomain
      = SetAsFun({ <<"ro1", "domain-main">>, <<"ro2", "domain-main">> })
    /\ __saved_responseOpElicitation
      = SetAsFun({ <<"ro1", "e1">>, <<"ro2", "none">> })
    /\ __saved_responseOpEndpoint
      = SetAsFun({ <<"ro1", "ep-a">>, <<"ro2", "none">> })
    /\ __saved_responseOpGeneration = SetAsFun({ <<"ro1", 0>>, <<"ro2", 0>> })
    /\ __saved_responseOpKind
      = SetAsFun({ <<"ro1", "approval-response">>, <<"ro2", "none">> })
    /\ __saved_responseOpSession
      = SetAsFun({ <<"ro1", "s1">>, <<"ro2", "s1">> })
    /\ __saved_responseValid = SetAsFun({ <<"ro1", TRUE>>, <<"ro2", FALSE>> })
    /\ __saved_sessionGeneration = SetAsFun({ <<"s1", 0>>, <<"s2", 0>> })
    /\ __saved_state = SetAsFun({<<"e1", "expired">>})
    /\ __saved_targetGeneration = SetAsFun({<<"e1", 0>>})
    /\ __saved_targetSession = SetAsFun({<<"e1", "s1">>})
    /\ __saved_terminalLsn = SetAsFun({<<"e1", 4>>})
    /\ ☐((terminalLsn['e1'] > 0) ⇒ (((state['e1'] = firstTerminalState['e1']) ∧ (answeredBy['e1'] = firstAnsweredBy['e1'])) ∧ (answeredResponseOp['e1'] = firstAnsweredResponseOp['e1'])))
      = FALSE
    /\ ☐((terminalLsn['e1'] > 0) ⇒ (((state['e1'] = firstTerminalState['e1']) ∧ (answeredBy['e1'] = firstAnsweredBy['e1'])) ∧ (answeredResponseOp['e1'] = firstAnsweredResponseOp['e1'])))
      = FALSE
    /\ __temporal_t_2_unroll = FALSE
    /\ __temporal_t_2_unroll_prev = FALSE
    /\ answeredBy = SetAsFun({<<"e1", "ep-a">>})
    /\ answeredResponseOp = SetAsFun({<<"e1", "ro1">>})
    /\ contractKind = SetAsFun({<<"e1", "approval">>})
    /\ elicitationDomain = SetAsFun({<<"e1", "domain-main">>})
    /\ firstAnsweredBy = SetAsFun({<<"e1", "ep-a">>})
    /\ firstAnsweredResponseOp = SetAsFun({<<"e1", "ro1">>})
    /\ firstTerminalState = SetAsFun({<<"e1", "answered">>})
    /\ lsn = 4
    /\ responderActor = SetAsFun({<<"e1", "alice">>})
    /\ responseDuplicate = SetAsFun({ <<"ro1", FALSE>>, <<"ro2", FALSE>> })
    /\ responseOpActor = SetAsFun({ <<"ro1", "alice">>, <<"ro2", "alice">> })
    /\ responseOpDomain
      = SetAsFun({ <<"ro1", "domain-main">>, <<"ro2", "domain-main">> })
    /\ responseOpElicitation = SetAsFun({ <<"ro1", "e1">>, <<"ro2", "none">> })
    /\ responseOpEndpoint = SetAsFun({ <<"ro1", "ep-a">>, <<"ro2", "none">> })
    /\ responseOpGeneration = SetAsFun({ <<"ro1", 0>>, <<"ro2", 0>> })
    /\ responseOpKind
      = SetAsFun({ <<"ro1", "approval-response">>, <<"ro2", "none">> })
    /\ responseOpSession = SetAsFun({ <<"ro1", "s1">>, <<"ro2", "s1">> })
    /\ responseValid = SetAsFun({ <<"ro1", TRUE>>, <<"ro2", FALSE>> })
    /\ sessionGeneration = SetAsFun({ <<"s1", 0>>, <<"s2", 0>> })
    /\ state = SetAsFun({<<"e1", "expired">>})
    /\ targetGeneration = SetAsFun({<<"e1", 0>>})
    /\ targetSession = SetAsFun({<<"e1", "s1">>})
    /\ terminalLsn = SetAsFun({<<"e1", 4>>}) *)
State4 ==
  __InLoop = TRUE
    /\ __q_temporalProps_init = FALSE
    /\ __saved___temporal_t_1 = FALSE
    /\ __saved___temporal_t_2 = FALSE
    /\ __saved_answeredBy = SetAsFun({<<"e1", "ep-a">>})
    /\ __saved_answeredResponseOp = SetAsFun({<<"e1", "ro1">>})
    /\ __saved_contractKind = SetAsFun({<<"e1", "approval">>})
    /\ __saved_elicitationDomain = SetAsFun({<<"e1", "domain-main">>})
    /\ __saved_firstAnsweredBy = SetAsFun({<<"e1", "ep-a">>})
    /\ __saved_firstAnsweredResponseOp = SetAsFun({<<"e1", "ro1">>})
    /\ __saved_firstTerminalState = SetAsFun({<<"e1", "answered">>})
    /\ __saved_lsn = 4
    /\ __saved_responderActor = SetAsFun({<<"e1", "alice">>})
    /\ __saved_responseDuplicate
      = SetAsFun({ <<"ro1", FALSE>>, <<"ro2", FALSE>> })
    /\ __saved_responseOpActor
      = SetAsFun({ <<"ro1", "alice">>, <<"ro2", "alice">> })
    /\ __saved_responseOpDomain
      = SetAsFun({ <<"ro1", "domain-main">>, <<"ro2", "domain-main">> })
    /\ __saved_responseOpElicitation
      = SetAsFun({ <<"ro1", "e1">>, <<"ro2", "none">> })
    /\ __saved_responseOpEndpoint
      = SetAsFun({ <<"ro1", "ep-a">>, <<"ro2", "none">> })
    /\ __saved_responseOpGeneration = SetAsFun({ <<"ro1", 0>>, <<"ro2", 0>> })
    /\ __saved_responseOpKind
      = SetAsFun({ <<"ro1", "approval-response">>, <<"ro2", "none">> })
    /\ __saved_responseOpSession
      = SetAsFun({ <<"ro1", "s1">>, <<"ro2", "s1">> })
    /\ __saved_responseValid = SetAsFun({ <<"ro1", TRUE>>, <<"ro2", FALSE>> })
    /\ __saved_sessionGeneration = SetAsFun({ <<"s1", 0>>, <<"s2", 0>> })
    /\ __saved_state = SetAsFun({<<"e1", "expired">>})
    /\ __saved_targetGeneration = SetAsFun({<<"e1", 0>>})
    /\ __saved_targetSession = SetAsFun({<<"e1", "s1">>})
    /\ __saved_terminalLsn = SetAsFun({<<"e1", 4>>})
    /\ __temporal_t_1 = FALSE
    /\ __temporal_t_2 = FALSE
    /\ __temporal_t_2_unroll = FALSE
    /\ __temporal_t_2_unroll_prev = FALSE
    /\ answeredBy = SetAsFun({<<"e1", "ep-a">>})
    /\ answeredResponseOp = SetAsFun({<<"e1", "ro1">>})
    /\ contractKind = SetAsFun({<<"e1", "approval">>})
    /\ elicitationDomain = SetAsFun({<<"e1", "domain-main">>})
    /\ firstAnsweredBy = SetAsFun({<<"e1", "ep-a">>})
    /\ firstAnsweredResponseOp = SetAsFun({<<"e1", "ro1">>})
    /\ firstTerminalState = SetAsFun({<<"e1", "answered">>})
    /\ lsn = 4
    /\ responderActor = SetAsFun({<<"e1", "alice">>})
    /\ responseDuplicate = SetAsFun({ <<"ro1", FALSE>>, <<"ro2", FALSE>> })
    /\ responseOpActor = SetAsFun({ <<"ro1", "alice">>, <<"ro2", "alice">> })
    /\ responseOpDomain
      = SetAsFun({ <<"ro1", "domain-main">>, <<"ro2", "domain-main">> })
    /\ responseOpElicitation = SetAsFun({ <<"ro1", "e1">>, <<"ro2", "none">> })
    /\ responseOpEndpoint = SetAsFun({ <<"ro1", "ep-a">>, <<"ro2", "none">> })
    /\ responseOpGeneration = SetAsFun({ <<"ro1", 0>>, <<"ro2", 0>> })
    /\ responseOpKind
      = SetAsFun({ <<"ro1", "approval-response">>, <<"ro2", "none">> })
    /\ responseOpSession = SetAsFun({ <<"ro1", "s1">>, <<"ro2", "s1">> })
    /\ responseValid = SetAsFun({ <<"ro1", TRUE>>, <<"ro2", FALSE>> })
    /\ sessionGeneration = SetAsFun({ <<"s1", 0>>, <<"s2", 0>> })
    /\ state = SetAsFun({<<"e1", "expired">>})
    /\ targetGeneration = SetAsFun({<<"e1", 0>>})
    /\ targetSession = SetAsFun({<<"e1", "s1">>})
    /\ terminalLsn = SetAsFun({<<"e1", 4>>})

(* The following formula holds true in the last state and violates the invariant *)
(* InvariantViolation ==
  (__InLoop
      /\ state = __saved_state
      /\ terminalLsn = __saved_terminalLsn
      /\ lsn = __saved_lsn
      /\ responderActor = __saved_responderActor
      /\ answeredBy = __saved_answeredBy
      /\ contractKind = __saved_contractKind
      /\ elicitationDomain = __saved_elicitationDomain
      /\ targetSession = __saved_targetSession
      /\ targetGeneration = __saved_targetGeneration
      /\ sessionGeneration = __saved_sessionGeneration
      /\ responseOpElicitation = __saved_responseOpElicitation
      /\ responseOpKind = __saved_responseOpKind
      /\ responseOpDomain = __saved_responseOpDomain
      /\ responseOpSession = __saved_responseOpSession
      /\ responseOpGeneration = __saved_responseOpGeneration
      /\ responseOpActor = __saved_responseOpActor
      /\ responseOpEndpoint = __saved_responseOpEndpoint
      /\ responseValid = __saved_responseValid
      /\ responseDuplicate = __saved_responseDuplicate
      /\ answeredResponseOp = __saved_answeredResponseOp
      /\ firstTerminalState = __saved_firstTerminalState
      /\ firstAnsweredBy = __saved_firstAnsweredBy
      /\ firstAnsweredResponseOp = __saved_firstAnsweredResponseOp
      /\ ☐((terminalLsn['e1'] > 0) ⇒ (((state['e1'] = firstTerminalState['e1']) ∧ (answeredBy['e1'] = firstAnsweredBy['e1'])) ∧ (answeredResponseOp['e1'] = firstAnsweredResponseOp['e1'])))
        = __saved_☐((terminalLsn['e1'] > 0) ⇒ (((state['e1'] = firstTerminalState['e1']) ∧ (answeredBy['e1'] = firstAnsweredBy['e1'])) ∧ (answeredResponseOp['e1'] = firstAnsweredResponseOp['e1'])))
      /\ ☐((terminalLsn['e1'] > 0) ⇒ (((state['e1'] = firstTerminalState['e1']) ∧ (answeredBy['e1'] = firstAnsweredBy['e1'])) ∧ (answeredResponseOp['e1'] = firstAnsweredResponseOp['e1'])))
        = __saved_☐((terminalLsn['e1'] > 0) ⇒ (((state['e1'] = firstTerminalState['e1']) ∧ (answeredBy['e1'] = firstAnsweredBy['e1'])) ∧ (answeredResponseOp['e1'] = firstAnsweredResponseOp['e1'])))
      /\ (~__temporal_t_2_unroll
        \/ ☐((terminalLsn['e1'] > 0) ⇒ (((state['e1'] = firstTerminalState['e1']) ∧ (answeredBy['e1'] = firstAnsweredBy['e1'])) ∧ (answeredResponseOp['e1'] = firstAnsweredResponseOp['e1']))))
      /\ __temporal_t_2_unroll_prev = __temporal_t_2_unroll)
    /\ ~__q::temporalProps_init *)
InvariantViolation ==
  (__InLoop
      /\ state = __saved_state
      /\ terminalLsn = __saved_terminalLsn
      /\ lsn = __saved_lsn
      /\ responderActor = __saved_responderActor
      /\ answeredBy = __saved_answeredBy
      /\ contractKind = __saved_contractKind
      /\ elicitationDomain = __saved_elicitationDomain
      /\ targetSession = __saved_targetSession
      /\ targetGeneration = __saved_targetGeneration
      /\ sessionGeneration = __saved_sessionGeneration
      /\ responseOpElicitation = __saved_responseOpElicitation
      /\ responseOpKind = __saved_responseOpKind
      /\ responseOpDomain = __saved_responseOpDomain
      /\ responseOpSession = __saved_responseOpSession
      /\ responseOpGeneration = __saved_responseOpGeneration
      /\ responseOpActor = __saved_responseOpActor
      /\ responseOpEndpoint = __saved_responseOpEndpoint
      /\ responseValid = __saved_responseValid
      /\ responseDuplicate = __saved_responseDuplicate
      /\ answeredResponseOp = __saved_answeredResponseOp
      /\ firstTerminalState = __saved_firstTerminalState
      /\ firstAnsweredBy = __saved_firstAnsweredBy
      /\ firstAnsweredResponseOp = __saved_firstAnsweredResponseOp
      /\ __temporal_t_1 = __saved___temporal_t_1
      /\ __temporal_t_2 = __saved___temporal_t_2
      /\ (~__temporal_t_2_unroll \/ __temporal_t_2)
      /\ __temporal_t_2_unroll_prev = __temporal_t_2_unroll)
    /\ ~__q_temporalProps_init

================================================================================
(* Created by Apalache on Sat Jul 11 10:28:15 MDT 2026 *)
(* https://github.com/apalache-mc/apalache *)
